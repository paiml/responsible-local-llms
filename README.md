# Responsible Generative AI and Local LLMs: demo code

Companion code for the LinkedIn Learning course *Responsible Generative AI and Local LLMs*.
One Rust workspace, one test suite, and three local runtimes serving the same model:
[apr](https://github.com/paiml/aprender), [Ollama](https://ollama.com) and [llama.cpp](https://github.com/ggml-org/llama.cpp).

## Layout

| Path | What it is |
|------|------------|
| `crates/rll-client` | One OpenAI-compatible chat client. `RLL_RUNTIME=apr\|ollama\|llamacpp` picks the runtime, `RLL_BASE_URL` overrides where it listens. |
| `crates/ch1-sovereign` | Chapter 1 demos (lessons 2 to 4), each ending in an `assert!` and a `contract: ... OK` line. |
| `scripts/` | `serve-apr.sh`, `serve-ollama.sh`, `serve-llamacpp.sh`, and the whisper.cpp pipeline for lesson 5. |
| `evals/` | Test sets as `*.jsonl` (filled in from chapter 2). |

## Run it offline

Everything runs against recorded replies, no model needed:

```bash
make test            # unit tests, demo bins, shell test
make coverage        # fails under 100% line coverage
make lint            # rustfmt, clippy, shellcheck
cargo run -p ch1-sovereign --bin lesson2_three_runtimes
```

## Run it live

Start a runtime on one GGUF (a small Qwen3 quantized to q4_k_m works), then point the demos at it:

```bash
GGUF=$HOME/models/qwen3-0.6b-q4_k_m.gguf scripts/serve-apr.sh        # port 8081
GGUF=$HOME/models/qwen3-0.6b-q4_k_m.gguf scripts/serve-ollama.sh     # port 11434
GGUF=$HOME/models/qwen3-0.6b-q4_k_m.gguf scripts/serve-llamacpp.sh   # port 8080

RLL_LIVE=1 RLL_RUNTIME=ollama cargo run -p ch1-sovereign --bin lesson3_quantize
```

| Variable | Meaning | Default |
|----------|---------|---------|
| `RLL_LIVE` | `1` talks to real runtimes; anything else uses fixtures | unset |
| `RLL_RUNTIME` | `apr`, `ollama` or `llamacpp` | `apr` |
| `RLL_BASE_URL` | server address | the runtime's default port |
| `RLL_MODEL` | model name sent in the request | `qwen3-0.6b` |

## Provable contracts

Every demo bin states a `Provable contract:` in its docstring and checks it at run time.
If the contract fails the bin exits non-zero; if it holds it prints `contract: ... OK`.

## License

MIT
