# Responsible Generative AI and Local LLMs: demo code

Companion code for course *Responsible Generative AI and Local LLMs*.
One Rust workspace, one test suite, and three local runtimes serving the same model:
[apr](https://github.com/paiml/aprender), [Ollama](https://ollama.com) and [llama.cpp](https://github.com/ggml-org/llama.cpp).

## Layout

| Path | What it is |
|------|------------|
| `crates/rll-client` | One OpenAI-compatible chat client. `RLL_RUNTIME=apr\|ollama\|llamacpp` picks the runtime, `RLL_BASE_URL` overrides where it listens. |
| `crates/ch1-sovereign` | Chapter 1 demos (lessons 2 to 4), each ending in an `assert!` and a `contract: ... OK` line. |
| `crates/ch5-governance` | Chapter 5 demos (videos 5.1 to 5.4): licence checker, EU AI Act documentation generator, sensitivity router, release checklist. Offline, on recorded illustrative fixtures. |
| `crates/ch4-agents` | Chapter 4 demos (videos 4.1 to 4.5): typed tool agent, local MCP server over loopback, agency limits, human approval with an audit log, tool-abuse evals. Offline, the model is a RECORDED, illustrative answer set; live legs check invariants only. |
| `crates/ch3-guard` | Chapter 3 demos 3.3 and 3.4: PII and secret redaction (`lesson33_pii_redaction`) and input/output guardrails (`lesson34_guardrails`). Pure text, no runtime; run with `cargo run -p ch3-guard --bin lesson33_pii_redaction`. |
| `scripts/` | `serve-apr.sh`, `serve-ollama.sh`, `serve-llamacpp.sh`, and the whisper.cpp pipeline for lesson 5. |
| `evals/` | Test sets as `*.jsonl`: a planned set for each of the six risks of lesson 1.7, and `quantization-quality` for lesson 1.3 (see `evals/README.md`). |
| `evals/` | Test sets as `*.jsonl` (filled in from chapter 2). `pii.jsonl` and `guardrails.jsonl` (chapter 3) hold hand-written synthetic strings, each labelled `"source":"illustrative"`: examples of the shapes, not measurements of any model. |

## Run it offline

Everything runs against recorded replies, no model needed:

```bash
make test            # unit tests, demo bins, shell test
make coverage        # fails under 100% line coverage
make lint            # rustfmt, clippy, shellcheck
make pv              # provable contracts and the pv ontology
make live            # the same demo tests against real local runtimes (needs docker)
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

`scripts/live-suite.sh` does all of this in one step: it fetches a pinned, sha256-checked
Qwen3-0.6B q4_k_m GGUF, starts llama.cpp (`ghcr.io/ggml-org/llama.cpp:server-b12110`) and
Ollama (`ollama/ollama:0.40.13`) from pinned images, and runs
`crates/ch1-sovereign/tests/configured_runtime.rs` against each one with `RLL_LIVE=1`.
`scripts/live-suite.sh apr` adds the apr leg through `scripts/serve-apr.sh` when `apr` is on
your PATH. The live suite is a nightly LAB workflow (`.github/workflows/live.yml`), never part of a pull
request: the `live` job runs llama.cpp and Ollama, and the `live-apr` job runs the apr leg on a
runner that already has `apr` on its PATH. That job only checks for `apr`; it never installs
it, so a runner without it fails the job red and the failure is the signal. apr is also checked
locally with `make live RUNTIMES=apr`. A runtime that cannot start fails the run.

## Provable contracts

Every demo bin states a `Provable contract:` in its docstring and checks it at run time.
If the contract fails the bin exits non-zero; if it holds it prints `contract: ... OK`.

The same contracts are written down as YAML under `contracts/` and checked by
[`pv`](https://github.com/paiml/aprender) (`aprender-contracts-cli`), the provable-contracts verifier:

| Contract | Demo |
|----------|------|
| `three-runtimes-agree-v1` | lesson 2: apr, Ollama and llama.cpp answer one request and each contain "Paris" |
| `quantize-size-speed-v1` | lesson 3: size grows with bits per weight; measured tokens/s is finite and positive |
| `quantization-quality-v1` | lesson 3 (quality): each quantization level is scored on one fixed case set; a case passes only with every required property |
| `local-economics-breakeven-v1` | lesson 4: costs are equal at break-even; with none, hosted is cheaper at every volume |
| `pii-redaction-v1` | lesson 3.3: redaction removes every detected value, is idempotent, leaves clean text alone; log lines hold counts and kinds, never values |
| `guardrails-v1` | lesson 3.4: each guardrail rule trips on its case and not on its near miss; a filter passes on exactly the redacted text; no log line holds the text |
| `typed-tool-calls-v1` | lesson 4.1: only output that parses as a typed tool call runs |
| `mcp-loopback-v1` | lesson 4.2: tools listed and called over loopback MCP; remote addresses refused |
| `agency-limits-v1` | lesson 4.3: allow-list, jail and dry-run; a refusal changes nothing |
| `approval-audit-v1` | lesson 4.4: a destructive call needs a yes; the audit log is hash-chained |
| `tool-output-is-data-v1` | lesson 4.5: injected tool output never widens agency; the benign case still passes |
| `licence-check-v1` | lesson 5.1: no licence, no model; Allow only for a permissive table row |
| `ai-act-doc-v1` | lesson 5.2: the generated document claims no evidence file that is absent |
| `sensitivity-routing-v1` | lesson 5.3: no sensitive prompt reaches the hosted side; a local failure never falls back |
| `responsible-checklist-v1` | lesson 5.4: a ticked box needs an evidence file that exists |
| `demo-bin-v1` | every bin: contract docstring, `assert!` before the `contract: ... OK` line |

`make pv` runs `pv lint` (bindings in `contracts/binding.yaml`), the ontology's sigma and shapes
gates, and `pv extract contracts --check` (the tracked `contracts.nt` / `shapes.ttl` are fresh).
The shapes read `evidence/demo-bins.jsonl` (from `scripts/demo-bins-jsonl.sh`) and
`evidence/binary/snapshot.jsonl` (from `scripts/bin-snapshot.sh`); rerun both and commit when a
bin is added or changed.

## Chapter 5: governance (`crates/ch5-governance`)

Four offline demos: `lesson1_licence_check`, `lesson2_ai_act_docs`, `lesson3_sensitivity_router`,
`lesson4_checklist`. Everything they read is recorded and labelled: the model cards in
`fixtures/cards/`, the sample repo in `fixtures/sample-repo/` and the hosted stand-in's reply are
ILLUSTRATIVE, not real vendor or measured data. The hosted side of lesson 3 is a recorder with no
network; `RLL_LIVE=1` sends only the local half to a real runtime. Run
`cargo run -p ch5-governance --bin lesson4_checklist` to check `RESPONSIBLE-AI.md`.

Sources, all accessed 2026-11-19 (the demo prints them with a `[read]` or `[NOT VERIFIED]` flag):

- Apache-2.0 text: <https://www.apache.org/licenses/LICENSE-2.0.txt>; SPDX: <https://spdx.org/licenses/Apache-2.0.html>
- Llama 3.1 community licence: <https://huggingface.co/meta-llama/Llama-3.1-8B/blob/main/LICENSE>
- Model cards: <https://huggingface.co/docs/hub/model-cards>
- EU AI Act Arts 50, 53, 99 (unofficial mirror): <https://artificialintelligenceact.eu/article/50/>, `/53/`, `/99/`
- NOT VERIFIED: MIT text (<https://spdx.org/licenses/MIT.html>), Art 113 application dates
  (<https://artificialintelligenceact.eu/article/113/>; the mirror shows conflicting amendments, so no
  dates are stated), and the Art 5 prohibited list, Annex III and Art 9-15 titles
  (<https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32024R1689>; the fetch returned recitals only).
  Check them against the official text before you rely on them. This is not legal advice.

## License

MIT
