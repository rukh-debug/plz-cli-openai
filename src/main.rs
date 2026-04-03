#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use bat::PrettyPrinter;
use clap::Parser;
use colored::Colorize;
use config::Config;
use question::{Answer, Question};
use reqwest::blocking::Client;
use serde_json::json;
use spinners::{Spinner, Spinners};
use std::process::Command;
use std::time::Instant;

use std::fs::File;
use std::io::{self, BufRead};

mod config;

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate shell scripts from natural language using any OpenAI-compatible API", long_about = None)]
pub struct CliArgs {
    /// Description of the command to execute
    pub prompt: Vec<String>,

    /// Run the generated script without asking for confirmation
    #[clap(short = 'y', long)]
    pub force: bool,

    /// API base URL (e.g. https://api.openai.com, http://localhost:11434)
    #[clap(long = "base-url", short = 'u')]
    pub base_url: Option<String>,

    /// API key for authentication
    #[clap(long = "api-key", short = 'k')]
    pub api_key: Option<String>,

    /// Model name to use (e.g. gpt-4o, qwen2.5-coder:7b)
    #[clap(long = "model", short = 'm')]
    pub model: Option<String>,

    /// Provider name for display purposes (e.g. openai, ollama, groq)
    #[clap(long = "provider", short = 'p')]
    pub provider: Option<String>,

    /// Sampling temperature (0.0 - 2.0)
    #[clap(long = "temperature", short = 't')]
    pub temperature: Option<f32>,

    /// Maximum tokens in the response
    #[clap(long = "max-tokens")]
    pub max_tokens: Option<u32>,

    /// Path to config file (default: ~/.config/plz/config.toml)
    #[clap(long = "config")]
    pub config: Option<std::path::PathBuf>,

    /// Reasoning effort level (low, medium, high) for reasoning models
    #[clap(long = "think")]
    pub think: Option<String>,

    /// Disable reasoning/thinking even if configured
    #[clap(long = "no-think", short = 'x')]
    pub no_think: bool,
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs < 1.0 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{:.1}s", secs)
    }
}

fn print_header(prompt: &str) {
    let width = 50;
    let title = format!("plz {}", prompt);
    let truncated = if title.len() > width - 4 {
        format!("{}...", &title[..width - 7])
    } else {
        title
    };
    let padded = format!(" {} ", truncated);
    let line = "─".repeat(width);
    println!("\n{}", line.dimmed());
    println!("{}", padded.cyan().bold());
    println!("{}\n", line.dimmed());
}

fn print_section(title: &str) {
    println!("\n{}", format!("── {} ──", title).dimmed());
}

fn main() {
    let cli = CliArgs::parse();

    if cli.prompt.is_empty() {
        eprintln!(
            "{} Error: Command is not complete. Please provide a task description.",
            "✖".red()
        );
        eprintln!("\nUsage: plz <task description>");
        eprintln!("Example: plz list all files in the current directory\n");
        std::process::exit(1);
    }

    let prompt_text = cli.prompt.join(" ");

    print_header(&prompt_text);

    let config = Config::new(&cli);

    let client = Client::new();
    let api_addr = format!("{}/v1/chat/completions", config.base_url);
    let mut request_body = json!({
        "model": config.model,
        "messages": [
            {
                "role": "system",
                "content": "Generate a small Bash/Zsh script for the given task. Return ONLY the raw script without any formatting, markdown, or code block indicators. Dont include explanations if not necessary, else include as comments within the script."
            },
            {
                "role": "user",
                "content": build_prompt(&cli.prompt.join(" "))
            }
        ],
        "temperature": config.temperature,
        "max_tokens": config.max_tokens,
        "stream": false
    });

    if let Some(ref think) = config.think {
        request_body["reasoning"] = json!({ "effort": think });
    }

    let mut request = client.post(&api_addr).json(&request_body);

    if let Some(ref api_key) = config.api_key {
        request = request.bearer_auth(api_key);
    }

    let mut spinner = Spinner::new(
        Spinners::BouncingBar,
        format!("Generating with {} ({})...", config.provider, config.model),
    );

    let start = Instant::now();
    let response = request.send().unwrap();
    let elapsed = start.elapsed();

    let status_code = response.status();
    if status_code.is_client_error() {
        let response_body = response.json::<serde_json::Value>().unwrap();
        let error_message = response_body
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .or_else(|| response_body.get("error").and_then(|e| e.as_str()))
            .unwrap_or("Unknown client error");
        spinner.stop_and_persist(
            "✖".red().to_string().as_str(),
            format!(
                "API error: \"{}\" ({})",
                error_message,
                format_duration(elapsed)
            )
            .red()
            .to_string(),
        );
        std::process::exit(1);
    } else if status_code.is_server_error() {
        spinner.stop_and_persist(
            "✖".red().to_string().as_str(),
            format!(
                "Server error from {}: {} ({})",
                config.provider,
                status_code,
                format_duration(elapsed)
            )
            .red()
            .to_string(),
        );
        std::process::exit(1);
    }

    let response_json = response.json::<serde_json::Value>().unwrap();
    let code = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_else(|| {
            spinner.stop_and_persist(
                "✖".red().to_string().as_str(),
                "Failed to parse response from API".red().to_string(),
            );
            std::process::exit(1);
        })
        .trim()
        .to_string();

    spinner.stop_and_persist(
        "✔".green().to_string().as_str(),
        format!("Script generated in {}", format_duration(elapsed))
            .green()
            .to_string(),
    );

    print_section("generated script");

    PrettyPrinter::new()
        .input_from_bytes(code.as_bytes())
        .language("bash")
        .grid(true)
        .print()
        .unwrap();

    print_section("execute");

    let should_run = if cli.force {
        true
    } else {
        Question::new(
            &format!("{} Run this script?", "▶".yellow())
                .bright_black()
                .to_string(),
        )
        .yes_no()
        .until_acceptable()
        .default(Answer::YES)
        .ask()
        .expect("Couldn't ask question.")
            == Answer::YES
    };

    if should_run {
        config.write_to_history(code.as_str());
        let exec_start = Instant::now();
        spinner = Spinner::new(Spinners::BouncingBar, "Executing...".into());

        let output = Command::new("bash")
            .arg("-c")
            .arg(code.as_str())
            .output()
            .unwrap_or_else(|_| {
                spinner.stop_and_persist(
                    "✖".red().to_string().as_str(),
                    "Failed to execute the generated program.".red().to_string(),
                );
                std::process::exit(1);
            });

        let exec_elapsed = exec_start.elapsed();

        if !output.status.success() {
            spinner.stop_and_persist(
                "✖".red().to_string().as_str(),
                format!("Script failed ({})", format_duration(exec_elapsed))
                    .red()
                    .to_string(),
            );
            println!("\n{}", "── error output ──".dimmed());
            println!("{}", String::from_utf8_lossy(&output.stderr));
            std::process::exit(1);
        }

        spinner.stop_and_persist(
            "✔".green().to_string().as_str(),
            format!(
                "Script executed successfully in {}",
                format_duration(exec_elapsed)
            )
            .green()
            .to_string(),
        );

        println!("\n{}", "── output ──".dimmed());
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
}

fn get_linux_distro() -> Option<String> {
    if let Ok(file) = File::open("/etc/os-release") {
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.starts_with("ID=") {
                    let distro_id = line[3..].trim_matches('"').to_string();
                    return Some(distro_id);
                }
            }
        }
    }
    None
}

fn build_prompt(prompt: &str) -> String {
    let os_hint = if cfg!(target_os = "macos") {
        " (on macOS)".to_string()
    } else if cfg!(target_os = "linux") {
        if let Some(distro) = get_linux_distro() {
            format!(" (on {} Linux)", distro)
        } else {
            " (on Linux)".to_string()
        }
    } else {
        "".to_string()
    };

    format!(
        "{prompt}{os_hint}:\n\n#!/usr/bin/env zsh\n",
        prompt = prompt,
        os_hint = os_hint
    )
}
