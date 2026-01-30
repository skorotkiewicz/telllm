# TELLLM

Telnet server for chatting with LLM using OpenAI-compatible API.

## Features

- **Telnet Interface**: Connect using any telnet client
- **LLM Integration**: Works with any OpenAI-compatible API
- **Persistent Logging**: Chat history saved per client IP
- **User Tracking**: Remember user names across sessions
- **Custom System Prompt**: Configure AI personality

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Basic usage (defaults to port 2323 and local API)
./target/release/telllm

# With custom settings
./target/release/telllm \
    --port 2323 \
    --endpoint "http://localhost:8080/v1" \
    --model "llama3" \
    --system-prompt "You are a helpful AI assistant. Be concise and friendly."

# Full options
./target/release/telllm --help
```

## Command Line Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--port` | `-p` | 2323 | Port to listen on |
| `--endpoint` | `-e` | http://localhost:8080/v1 | LLM API endpoint |
| `--model` | `-m` | default | Model name |
| `--api-key` | `-k` | (empty) | API key (optional) |
| `--system-prompt` | `-s` | "You are a helpful..." | Custom system prompt |
| `--logs-dir` | | logs | Logs directory |

## Connecting

```bash
telnet localhost 2323
```

## Chat Commands

| Command | Description |
|---------|-------------|
| `/name <name>` | Set your name (persisted across sessions) |
| `/clear` | Clear conversation history |
| `/help` | Show available commands |
| `/quit` | Disconnect |

## Log Structure

```
logs/
└── {CLIENT_IP}/
    ├── chats/
    │   └── {dd-mm-yy}.txt   # Daily chat logs
    └── summary.txt          # User info (name, last_seen)
```

### Chat Log Format

```
--- Session started at 30-01-2026 12:30:00 ---

[12:30:05] USER: Hello!
[12:30:08] AI: Hello! How can I help you today?

--- Session ended at 30-01-2026 12:45:00 ---
```

### Summary Format

```
name: John
last_seen: 30-01-2026 12:45:00
```

## License

MIT
