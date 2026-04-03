# plz

Generate shell scripts from natural language using any OpenAI-compatible API.

## Usage

```bash
plz "list all files in current directory"
plz "find all rust files and count lines of code"
plz -y "compress this folder into a tar.gz archive"
```

## Configuration

Config priority: **CLI args > env vars > config file > defaults**

### CLI Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--base-url` | `-u` | API base URL |
| `--api-key` | `-k` | API key |
| `--model` | `-m` | Model name (required) |
| `--provider` | `-p` | Provider name (display only) |
| `--temperature` | `-t` | Sampling temperature (0.0-2.0) |
| `--max-tokens` | | Max tokens in response |
| `--think` | | Reasoning effort (low/medium/high) for reasoning models |
| `--no-think` | `-x` | Disable reasoning even if configured |
| `--config` | | Custom config file path |
| `--force` | `-y` | Skip confirmation prompt |

### Environment Variables

`PLZ_BASE_URL`, `PLZ_API_KEY`, `PLZ_MODEL`, `PLZ_PROVIDER`, `PLZ_TEMPERATURE`, `PLZ_MAX_TOKENS`, `PLZ_THINK`

### Config File

Location: `~/.config/plz/config.toml`

```toml
base_url = "https://api.openai.com"
api_key = "sk-..."
model = "gpt-4o"
temperature = 0.7
max_tokens = 4096
think = "high"
```

## Examples

```bash
# OpenAI
plz -m gpt-4o -k sk-... "show disk usage"

# Ollama (local)
plz -u http://localhost:11434 -m qwen2.5-coder:7b "list all files"

# Groq
plz -u https://api.groq.com/openai/v1 -k gsk_... -m llama-3.1-70b "find largest files"

# With reasoning/thinking (o1, o3, and other reasoning models)
plz --think high "debug this complex issue"

# Disable thinking even if configured
plz -x "simple task"
```

## Reasoning / Thinking Mode

For OpenAI reasoning models (o1, o3, etc.), you can enable extended thinking to improve output quality on complex tasks.

### Enabling thinking

```bash
# CLI flag
plz --think high "complex task"

# Environment variable
export PLZ_THINK=high

# Config file (~/.config/plz/config.toml)
think = "medium"
```

Accepted values: `low`, `medium`, `high`

### Disabling thinking

If thinking is enabled in your config or env, you can override it for a single run:

```bash
plz -x "simple task"
plz --no-think "quick task"
```

## Develop

Requires Rust. Build with `cargo build`, run with `cargo run`.

## License

MIT
