# whytop

> A read-only, cross-platform terminal process monitor with local AI explanations.

`whytop` is a privacy-first alternative to `top` and `htop` for understanding what is running on your computer. Inspect a process, ask a plain-language question, and get an explanation from a local OpenAI-compatible model—without giving process evidence to a hosted service.

[![CI](https://github.com/IFAKA/whytop/actions/workflows/ci.yml/badge.svg)](https://github.com/IFAKA/whytop/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/whytop.svg)](https://crates.io/crates/whytop)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Why whytop?

- Read-only by design: it never kills, pauses, or modifies a process.
- Familiar terminal UI: sort by process name, CPU, or memory and keep the live table usable when AI is offline.
- Local explanations: connect Rapid-MLX on Apple Silicon or `llama-server` on Windows through an OpenAI-compatible HTTP API.
- Bounded evidence: environment variables and secrets are excluded; command lines and activity summaries are limited.
- Cross-platform core: shared process collection uses `sysinfo` and avoids macOS-only commands on Windows.

## Quick start

Install Rust through [rustup](https://rustup.rs/), then run:

```sh
git clone https://github.com/IFAKA/whytop.git
cd whytop
cargo run --release
```

The process table works without an AI runtime. Start a supported local model server to ask for explanations.

## Controls

| Key | Action |
| --- | --- |
| `Enter` | Open the selected process and ask a question |
| `n` | Sort by process name |
| `c` | Sort by CPU usage |
| `m` | Sort by memory usage |
| `Esc` | Return from process chat to the monitor |
| `q` | Quit |

The first numeric sort is largest-first and the first name sort is A–Z. Press the same key again for the opposite direction, then once more to return to normal PID order.

## Local AI runtimes

Both adapters use an OpenAI-compatible local HTTP API.

### Apple Silicon: Rapid-MLX

whytop defaults to the Rapid-MLX model `nail-qwen3.6-35b-a3b` at `http://127.0.0.1:8000/v1`. Override the endpoint or model with:

```sh
export WHYTOP_MLX_URL=http://127.0.0.1:8000/v1
export WHYTOP_MLX_MODEL=nail-qwen3.6-35b-a3b
```

### Windows: llama-server

Download `openbmb/MiniCPM5-1B-GGUF` and start `llama-server` with its OpenAI-compatible API on port 8080. You can override the defaults with `WHYTOP_LLAMA_URL` and `WHYTOP_LLAMA_MODEL`.

Before generating an answer, whytop checks `/v1/models`. If the server is unavailable, the process table remains usable and the inspector reports the connection error. A configured model is preferred; when a server exposes one different model, whytop uses it automatically.

## Privacy and safety

Snapshots, questions, and answers live only in memory for the current session. Evidence sent to the local model is bounded and excludes environment variables and secrets. Rich file, network, and signature enrichment is currently represented as unavailable until a native provider is added.

## Development

Format, test, and check the project with:

```sh
cargo fmt --all -- --check
cargo test
cargo check
```

For Windows portability, install the target and run `cargo check --target x86_64-pc-windows-gnu`. Real Rapid-MLX/llama-server generation, protected-process behavior, and the 250-case benchmark must be run on their respective target machines.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow. Found a security issue? Read [SECURITY.md](SECURITY.md).

## Limitations and roadmap

The current release focuses on safe, portable process evidence and local chat. Native file/network/signature enrichment, packaged binaries, and broader local runtime setup are future work. See the [issue tracker](https://github.com/IFAKA/whytop/issues) for current discussion.
