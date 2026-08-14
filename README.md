# whytop

`whytop` is a read-only, cross-platform process monitor with local AI explanations. It never kills, pauses, or modifies a process. Snapshots, questions, and answers live only in memory for the duration of the session.

## Build and run

```sh
cargo run --release
```

Select a process, type a question, and press Enter. Press `n` for name, `c` for CPU, or `m` for memory sorting. The first numeric sort is largest-first and the first name sort is A–Z; press the same key again for the opposite direction, then once more to return to normal PID order. `q` exits. A local Rapid-MLX or llama-server must be running for explanations.

## Local runtimes

Both adapters use an OpenAI-compatible local HTTP API.

On Apple Silicon, whytop uses the existing Rapid-MLX model `nail-qwen3.6-35b-a3b` at `http://127.0.0.1:8000/v1` by default. You can override this with `WHYTOP_MLX_URL` and `WHYTOP_MLX_MODEL`.

On Windows, download `openbmb/MiniCPM5-1B-GGUF`, start `llama-server` with its OpenAI-compatible API on port 8080, or set `WHYTOP_LLAMA_URL` and `WHYTOP_LLAMA_MODEL`. The default model name is `openbmb/MiniCPM5-1B-GGUF`.

The adapter checks `/v1/models` before generating. If the server is absent, the process table remains usable and the inspector reports the connection error. A configured model name is preferred; if a server exposes a different single model, that model is used so local aliases do not make the UI unusable.

## Evidence and portability

The shared collector uses `sysinfo` for portable PID/start-time/resource/parent data. Evidence sent to the model is bounded, excludes environment variables and secrets, limits command lines and activity summaries, and includes explicit unavailable fields. Rich file/network/signature enrichment is intentionally represented as unavailable until a native platform provider is added; no macOS-only tools are invoked on Windows.

Run shared checks with:

```sh
cargo test
cargo check --target x86_64-pc-windows-gnu
```

The Windows target check requires that target to be installed. Real Rapid-MLX/llama-server generation, protected-process behavior, and the 250-case benchmark must be run on their respective target machines.
