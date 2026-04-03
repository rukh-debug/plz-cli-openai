use colored::Colorize;
use serde::Deserialize;
use std::{env, fs, io::Write, path::PathBuf, process::exit};

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub think: Option<String>,
}

impl ConfigFile {
    pub fn from_path(path: &PathBuf) -> Option<Self> {
        if !path.exists() {
            return None;
        }
        match fs::read_to_string(path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => Some(config),
                Err(e) => {
                    eprintln!(
                        "{}",
                        format!(
                            "Warning: Failed to parse config file {}: {}",
                            path.display(),
                            e
                        )
                        .yellow()
                    );
                    None
                }
            },
            Err(e) => {
                eprintln!(
                    "{}",
                    format!(
                        "Warning: Failed to read config file {}: {}",
                        path.display(),
                        e
                    )
                    .yellow()
                );
                None
            }
        }
    }

    pub fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("plz");
        path.push("config.toml");
        path
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub provider: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub shell: String,
    pub think: Option<String>,
}

impl Config {
    pub fn new(cli: &crate::CliArgs) -> Self {
        let config_path = cli.config.clone().unwrap_or_else(ConfigFile::default_path);
        let file_config = ConfigFile::from_path(&config_path).unwrap_or(ConfigFile {
            base_url: None,
            api_key: None,
            model: None,
            provider: None,
            temperature: None,
            max_tokens: None,
            think: None,
        });

        let base_url = cli
            .base_url
            .clone()
            .or_else(|| env::var("PLZ_BASE_URL").ok())
            .or_else(|| file_config.base_url)
            .unwrap_or_else(|| {
                println!(
                    "{}",
                    "Warning: No base URL specified. Using default: https://api.openai.com"
                        .yellow()
                );
                "https://api.openai.com".to_string()
            });

        let api_key = cli
            .api_key
            .clone()
            .or_else(|| env::var("PLZ_API_KEY").ok())
            .or_else(|| file_config.api_key);

        let model = cli
            .model
            .clone()
            .or_else(|| env::var("PLZ_MODEL").ok())
            .or_else(|| file_config.model)
            .unwrap_or_else(|| {
                println!(
                    "{}",
                    "Error: No model specified. Set via --model, PLZ_MODEL env, or config file."
                        .red()
                );
                exit(1);
            });

        let provider = cli
            .provider
            .clone()
            .or_else(|| env::var("PLZ_PROVIDER").ok())
            .or_else(|| file_config.provider)
            .unwrap_or_else(|| Self::infer_provider(&base_url));

        let temperature = cli
            .temperature
            .or_else(|| {
                env::var("PLZ_TEMPERATURE")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .or(file_config.temperature)
            .unwrap_or(0.7);

        let max_tokens = cli
            .max_tokens
            .or_else(|| env::var("PLZ_MAX_TOKENS").ok().and_then(|v| v.parse().ok()))
            .or(file_config.max_tokens)
            .unwrap_or(4096);

        let think = if cli.no_think {
            None
        } else {
            cli.think
                .clone()
                .or_else(|| env::var("PLZ_THINK").ok())
                .or(file_config.think)
        };

        let shell = env::var("SHELL").unwrap_or_default();

        if api_key.is_none() && !base_url.contains("localhost") && !base_url.contains("127.0.0.1") {
            println!(
                "{}",
                "Warning: No API key specified. Requests may fail if authentication is required."
                    .yellow()
            );
        }

        Self {
            base_url,
            api_key,
            model,
            provider,
            temperature,
            max_tokens,
            shell,
            think,
        }
    }

    fn infer_provider(base_url: &str) -> String {
        if base_url.contains("openai.com") {
            "openai".to_string()
        } else if base_url.contains("anthropic.com") {
            "anthropic".to_string()
        } else if base_url.contains("groq.com") {
            "groq".to_string()
        } else if base_url.contains("localhost") || base_url.contains("127.0.0.1") {
            "local".to_string()
        } else {
            "custom".to_string()
        }
    }

    pub fn write_to_history(&self, code: &str) {
        let history_file = match self.shell.as_str() {
            "/bin/bash" => env::var("HOME").unwrap_or_default() + "/.bash_history",
            "/bin/zsh" => env::var("HOME").unwrap_or_default() + "/.zsh_history",
            _ => return,
        };

        if let Ok(mut file) = fs::OpenOptions::new().append(true).open(history_file) {
            let _ = file.write_all(format!("{code}\n").as_bytes());
        }
    }
}
