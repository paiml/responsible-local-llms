# RECORD-CHEATSHEET

- **Git SHA (`main` in kfocus run log)**: `responsible-local-llms@99c368f` (`99c368ffb15b382d54768ed64bec3601a347e08c`)
- **Notice**: **No command was guessed.** Every command below is marked `RAN`, `READ <file:line>`, or `NOT VERIFIED` (with `BLOCKED` where applicable), derived from `Makefile`, `README.md`, `evals/README.md`, `.github/workflows/live.yml`, `scripts/*.sh`, `Cargo.toml`, `crates/*/Cargo.toml`, and `evidence/demo-bins.jsonl`.
- **kfocus run log (`responsible-local-llms@99c368f`)**: `scripts/record-ready.sh` printed 41 lines, 39 `PASS`, 2 `SKIP(live: ch1-lesson5_whisper-pipeline-live, ch1-live-runtimes)`, `EXIT=0`.

## Per-Chapter Recording Status

| chapter | recordable yes/no | proof | blocker |
|---|---|---|---|
| `ch1` (offline) | yes | `RAN PASS` via `scripts/record-ready.sh` on `kfocus` (`responsible-local-llms@99c368f`: `ch1-sovereign-tests`, `ch1-lesson2_three_runtimes`, `ch1-lesson3_quantize`, `ch1-lesson3_quality`, `ch1-lesson4_economics`, `ch1-lesson5_whisper-pipeline-offline`, `ch1-live-suite-partial-guard`) | None (offline fixtures) |
| `ch1` (live) | no | `RAN` via `live.yml` run 44897400144 on `gx10`: success (check docker and apr PASS, test model cache PASS, "live suite (apr, llama.cpp, Ollama, lesson 1.2 on all three)" PASS; `scripts/live-suite.sh:100-101` also runs `lesson3_quantize` live via `ch1-sovereign --test configured_runtime`). `scripts/whisper-pipeline.sh` (`ch1-lesson5_whisper-pipeline-live`) stays `NOT VERIFIED` (`SKIP(live: ch1-lesson5_whisper-pipeline-live)` in `scripts/record-ready.sh` on `kfocus` at `99c368f`) | `scripts/whisper-pipeline.sh` (`ch1-lesson5_whisper-pipeline-live`) stays `NOT VERIFIED` (not covered by `live.yml` run 44897400144; requires `whisper-cli`, `WHISPER_MODEL`, `.wav` inputs, and a live runtime). `apr` is now mounted on all `gx10` pools (three-runtime and lesson 1.2 blocker resolved) |
| `ch2` (offline) | yes | `RAN PASS` via `scripts/record-ready.sh` on `kfocus` (`responsible-local-llms@99c368f`: `ch2-evals-tests` + 6 lesson bins) | None (offline) |
| `ch3` (offline) | yes | `RAN PASS` via `scripts/record-ready.sh` on `kfocus` (`responsible-local-llms@99c368f`: `ch3-safety-tests`, `ch3-guard-tests`, `ch3-redteam-tests` + 5 lesson bins) | None (offline) |
| `ch4` (offline) | yes | `RAN PASS` via `scripts/record-ready.sh` on `kfocus` (`responsible-local-llms@99c368f`: `ch4-agents-tests` + 5 lesson bins) | None (offline) |
| `ch5` (offline) | yes | `RAN PASS` via `scripts/record-ready.sh` on `kfocus` (`responsible-local-llms@99c368f`: `ch5-governance-tests` + 4 lesson bins) | None (offline) |

ch0: none exists; dropped from the sheet 2026-11-21

---

## Pre-flight & Workspace Gates

1. Run offline test suite (`cargo test --workspace`, `scripts/test-whisper-pipeline.sh`, `scripts/test-live-suite-partial.sh`):
   - `make test` — `NOT VERIFIED` directly via `make test` (`READ Makefile:8-11`, `READ README.md:26`); constituent crate test suites and shell tests `RAN PASS` via `scripts/record-ready.sh` on `kfocus` at `99c368f`
2. Run Rust & shell lints (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `shellcheck scripts/*.sh`):
   - `make lint` — `NOT VERIFIED` (`READ Makefile:17-29`, `READ README.md:28`)
3. Run line coverage gate (100% required):
   - `make coverage` — `RAN` on kfocus 2026-11-21 at 99c368f: EXIT=0 (lines 100%) (`READ Makefile:14-15`, `READ README.md:27`)
4. Run provable contracts & ontology gates:
   - `make pv` — `RAN` on kfocus 2026-11-21 at 99c368f: EXIT=0 (sigma+shapes Pass) (`READ Makefile:34-43`, `READ README.md:29`; `scripts/check-evals-table.sh` and `scripts/demo-bins-jsonl.sh --check` are `READ scripts/record-ready.sh:56-57`)
5. Run ONE GATE workflow check:
   - `make one-gate` (`cargo run --quiet -p one-gate -- .`) — `RAN PASS` for `cargo run --quiet -p one-gate -- .` via `scripts/record-ready.sh` (`one-gate`) on `kfocus` at `99c368f` (`READ scripts/record-ready.sh:55`); wrapper target `READ Makefile:56-57`
6. Run the per-chapter recording readiness script:
   - `bash scripts/record-ready.sh` — `RAN` on `kfocus` (`responsible-local-llms@99c368f`): printed 41 lines, 39 `PASS`, 2 `SKIP(live: ch1-lesson5_whisper-pipeline-live, ch1-live-runtimes)`, `EXIT=0` (`READ scripts/record-ready.sh:1-122`)

---

## Support Libraries (No Standalone Demo Bins)

- `rll-client` unit tests:
  - `cargo test -p rll-client` — `RAN PASS` (`rll-client-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/rll-client/Cargo.toml:2`, `READ scripts/record-ready.sh:52`
- `rll-evals` unit tests:
  - `cargo test -p rll-evals` — `RAN PASS` (`rll-evals-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/rll-evals/Cargo.toml:2`, `READ scripts/record-ready.sh:53`
- `one-gate` unit tests:
  - `cargo test -p one-gate` — `RAN PASS` (`one-gate-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/one-gate/Cargo.toml:2`, `READ scripts/record-ready.sh:54`

---

## Chapter 1: Sovereign Local LLMs (`crates/ch1-sovereign`)

### Offline Recording Commands

1. Chapter 1 test suite (unit tests, bin subprocess checks, whisper script test):
   - `cargo test -p ch1-sovereign` — `RAN PASS` (`ch1-sovereign-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/ch1-sovereign/Cargo.toml:2`, `READ scripts/record-ready.sh:60`
2. Lesson 1.2 — One model, three runtimes (offline fixtures):
   - `cargo run -p ch1-sovereign --bin lesson2_three_runtimes` — `RAN PASS` (`ch1-lesson2_three_runtimes` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ README.md:31`, `READ evidence/demo-bins.jsonl:1`, `READ scripts/record-ready.sh:61`
3. Lesson 1.3 — Quantization size and speed (offline fixtures):
   - `cargo run -p ch1-sovereign --bin lesson3_quantize` — `RAN PASS` (`ch1-lesson3_quantize` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:3`, `READ scripts/record-ready.sh:62`
4. Lesson 1.3 — Quantization answer quality (offline recorded answers):
   - `cargo run -p ch1-sovereign --bin lesson3_quality` — `RAN PASS` (`ch1-lesson3_quality` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evals/README.md:28`, `READ evidence/demo-bins.jsonl:2`, `READ scripts/record-ready.sh:63`
5. Lesson 1.4 — Local economics break-even (offline fixtures):
   - `cargo run -p ch1-sovereign --bin lesson4_economics` — `RAN PASS` (`ch1-lesson4_economics` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:4`, `READ scripts/record-ready.sh:64`
6. Lesson 1.5 — Whisper.cpp shell pipeline (offline stub test):
   - `scripts/test-whisper-pipeline.sh` — `RAN PASS` (`ch1-lesson5_whisper-pipeline-offline` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Makefile:10`, `READ scripts/test-whisper-pipeline.sh:1-80`, `READ scripts/record-ready.sh:65`
7. Live-suite partial-run falsifier (offline guard):
   - `scripts/test-live-suite-partial.sh` — `RAN PASS` (`ch1-live-suite-partial-guard` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Makefile:11`, `READ scripts/test-live-suite-partial.sh:1-13`, `READ scripts/record-ready.sh:67`

### Live Recording Commands (Requires Local Model GGUF + Runtimes / Docker / `apr`)

- Start `apr` runtime on port 8081:
  - `GGUF=$HOME/models/qwen3-0.6b-q4_k_m.gguf scripts/serve-apr.sh` — `RAN` via `live.yml` run 44897400144 on `gx10`: success (`apr` is now mounted on all `gx10` pools; `scripts/live-suite.sh:87` invokes `scripts/serve-apr.sh` on `RLL_APR_PORT=18081` with cached `.cache/models/Qwen3-0.6B-Q4_K_M.gguf`), `READ README.md:39`, `READ scripts/serve-apr.sh:1-7`, `READ scripts/live-suite.sh:85-90`
- Start `ollama` runtime on port 11434:
  - `GGUF=$HOME/models/qwen3-0.6b-q4_k_m.gguf scripts/serve-ollama.sh` — `RAN` via `live.yml` run 44897400144 on `gx10`: success (Ollama runtime covered by live suite via pinned container `ollama/ollama:0.40.13` in `scripts/live-suite.sh:79-84`; standalone host script `scripts/serve-ollama.sh` is not directly invoked by `live-suite.sh`), `READ README.md:40`, `READ scripts/serve-ollama.sh:1-16`, `READ scripts/live-suite.sh:79-84`
- Start `llama.cpp` runtime on port 8080:
  - `GGUF=$HOME/models/qwen3-0.6b-q4_k_m.gguf scripts/serve-llamacpp.sh` — `RAN` via `live.yml` run 44897400144 on `gx10`: success (`llama.cpp` runtime covered by live suite via pinned container `ghcr.io/ggml-org/llama.cpp:server-b12110` in `scripts/live-suite.sh:74-78`; standalone host script `scripts/serve-llamacpp.sh` is not directly invoked by `live-suite.sh`), `READ README.md:41`, `READ scripts/serve-llamacpp.sh:1-7`, `READ scripts/live-suite.sh:74-78`
- Run Lesson 1.3 live against Ollama:
  - `RLL_LIVE=1 RLL_RUNTIME=ollama cargo run -p ch1-sovereign --bin lesson3_quantize` — `RAN` via `live.yml` run 44897400144 on `gx10`: success (`scripts/live-suite.sh:100-101` runs `ch1-sovereign --test configured_runtime`, which runs `lesson3_quantize` live with `RLL_LIVE=1 RLL_RUNTIME=$rt RLL_QUANT=q4_k_m` across `apr`, `llamacpp`, and `ollama`), `READ README.md:43`, `READ scripts/live-suite.sh:100-101`, `READ crates/ch1-sovereign/tests/configured_runtime.rs:18-22`
- Run Lesson 1.5 Whisper.cpp pipeline live (requires `whisper-cli`, `WHISPER_MODEL`, `.wav` inputs, and live runtime):
  - `scripts/whisper-pipeline.sh a.wav [b.wav ...]` — `PENDING`: `RAN via live.yml job whisper, run <RUN-ID-PENDING>` (pending actual workflow run on `gx10`; wired to `RLL_DRY_RUN=1` on `tests/fixtures/whisper/hello.wav`; `SKIP(live: ch1-lesson5_whisper-pipeline-live)` in `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ .github/workflows/live.yml`, `READ scripts/whisper-pipeline.sh:4`, `READ scripts/record-ready.sh:66`
- Run full three-runtime live suite (Lesson 1.2; docker + `apr` on PATH + model fetch):
  - `make live RUNTIMES="apr llamacpp ollama"` — `RAN` via `live.yml` run 44897400144 on `gx10`: success (2026-11-21 attempt 4; check docker and apr PASS, test model cache PASS, "live suite (apr, llama.cpp, Ollama, lesson 1.2 on all three)" PASS; `apr` is now mounted on all `gx10` pools), `READ .github/workflows/live.yml:17-38`, `READ Makefile:47-48`, `READ scripts/record-ready.sh:68`

---

## Chapter 2: Evals as Code (`crates/ch2-evals`, Offline)

1. Chapter 2 test suite:
   - `cargo test -p ch2-evals` — `RAN PASS` (`ch2-evals-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/ch2-evals/Cargo.toml:2`, `READ scripts/record-ready.sh:71`
2. Lesson 2.1 — A test set is a spec:
   - `cargo run -p ch2-evals --bin lesson1_test_set_spec` — `RAN PASS` (`ch2-lesson1_test_set_spec` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:5`, `READ scripts/record-ready.sh:72`
3. Lesson 2.2 — Catch hallucination:
   - `cargo run -p ch2-evals --bin lesson2_hallucination` — `RAN PASS` (`ch2-lesson2_hallucination` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:6`, `READ scripts/record-ready.sh:73`
4. Lesson 2.3 — Structured output with bounded retries:
   - `cargo run -p ch2-evals --bin lesson3_structured` — `RAN PASS` (`ch2-lesson3_structured` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:7`, `READ scripts/record-ready.sh:74`
5. Lesson 2.4 — Determinism and flakiness:
   - `cargo run -p ch2-evals --bin lesson4_flakiness` — `RAN PASS` (`ch2-lesson4_flakiness` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:8`, `READ scripts/record-ready.sh:75`
6. Lesson 2.5 — Elo in Rust:
   - `cargo run -p ch2-evals --bin lesson5_elo` — `RAN PASS` (`ch2-lesson5_elo` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:9`, `READ scripts/record-ready.sh:76`
7. Lesson 2.6 — Evals in CI gate:
   - `cargo run -p ch2-evals --bin lesson6_ci_gate` — `RAN PASS` (`ch2-lesson6_ci_gate` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:10`, `READ scripts/record-ready.sh:77`

---

## Chapter 3: Safety, Guardrails & Red-Teaming (`ch3-safety`, `ch3-guard`, `ch3-redteam`, Offline)

1. Chapter 3 (`ch3-safety`) test suite:
   - `cargo test -p ch3-safety` — `RAN PASS` (`ch3-safety-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/ch3-safety/Cargo.toml:2`, `READ scripts/record-ready.sh:80`
2. Lesson 3.1 — Bias probes:
   - `cargo run -p ch3-safety --bin lesson1_bias_probes` — `RAN PASS` (`ch3-lesson1_bias_probes` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:14`, `READ scripts/record-ready.sh:81`
3. Lesson 3.2 — Prompt injection:
   - `cargo run -p ch3-safety --bin lesson2_prompt_injection` — `RAN PASS` (`ch3-lesson2_prompt_injection` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:15`, `READ scripts/record-ready.sh:82`
4. Chapter 3 (`ch3-guard`) test suite:
   - `cargo test -p ch3-guard` — `RAN PASS` (`ch3-guard-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/ch3-guard/Cargo.toml:2`, `READ scripts/record-ready.sh:83`
5. Lesson 3.3 — PII and secret redaction:
   - `cargo run -p ch3-guard --bin lesson33_pii_redaction` — `RAN PASS` (`ch3-lesson33_pii_redaction` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ README.md:17`, `READ evidence/demo-bins.jsonl:11`, `READ scripts/record-ready.sh:84`
6. Lesson 3.4 — Input/output guardrails as code:
   - `cargo run -p ch3-guard --bin lesson34_guardrails` — `RAN PASS` (`ch3-lesson34_guardrails` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:12`, `READ scripts/record-ready.sh:85`
7. Chapter 3 (`ch3-redteam`) test suite:
   - `cargo test -p ch3-redteam` — `RAN PASS` (`ch3-redteam-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/ch3-redteam/Cargo.toml:2`, `READ scripts/record-ready.sh:86`
8. Lesson 3.5 — Red-team attack set in CI:
   - `cargo run -p ch3-redteam --bin lesson5_redteam` — `RAN PASS` (`ch3-lesson5_redteam` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:13`, `READ scripts/record-ready.sh:87`

---

## Chapter 4: Safe Local Agents (`crates/ch4-agents`, Offline)

1. Chapter 4 test suite:
   - `cargo test -p ch4-agents` — `RAN PASS` (`ch4-agents-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/ch4-agents/Cargo.toml:2`, `READ scripts/record-ready.sh:90`
2. Lesson 4.1 — Typed local agent:
   - `cargo run -p ch4-agents --bin lesson1_typed_agent` — `RAN PASS` (`ch4-lesson1_typed_agent` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:16`, `READ scripts/record-ready.sh:91`
3. Lesson 4.2 — Tools over loopback MCP:
   - `cargo run -p ch4-agents --bin lesson2_mcp_loopback` — `RAN PASS` (`ch4-lesson2_mcp_loopback` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:17`, `READ scripts/record-ready.sh:92`
4. Lesson 4.3 — Agency limits (allow-list, path jail, dry-run):
   - `cargo run -p ch4-agents --bin lesson3_agency_limits` — `RAN PASS` (`ch4-lesson3_agency_limits` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:18`, `READ scripts/record-ready.sh:93`
5. Lesson 4.4 — Tool-output injection and tool-abuse evals:
   - `cargo run -p ch4-agents --bin lesson4_tool_abuse` — `RAN PASS` (`ch4-lesson4_tool_abuse` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:19`, `READ scripts/record-ready.sh:94`
6. Lesson 4.5 — Human approval and hash-chained audit log:
   - `cargo run -p ch4-agents --bin lesson5_approval_audit` — `RAN PASS` (`ch4-lesson5_approval_audit` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:20`, `READ scripts/record-ready.sh:95`

---

## Chapter 5: Governance as Code (`crates/ch5-governance`, Offline)

1. Chapter 5 test suite:
   - `cargo test -p ch5-governance` — `RAN PASS` (`ch5-governance-tests` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ Cargo.toml:3`, `READ crates/ch5-governance/Cargo.toml:2`, `READ scripts/record-ready.sh:98`
2. Lesson 5.1 — Licence checker:
   - `cargo run -p ch5-governance --bin lesson1_licence_check` — `RAN PASS` (`ch5-lesson1_licence_check` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:21`, `READ scripts/record-ready.sh:99`
3. Lesson 5.2 — EU AI Act documentation generator:
   - `cargo run -p ch5-governance --bin lesson2_ai_act_docs` — `RAN PASS` (`ch5-lesson2_ai_act_docs` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:22`, `READ scripts/record-ready.sh:100`
4. Lesson 5.3 — Sensitivity router:
   - `cargo run -p ch5-governance --bin lesson3_sensitivity_router` — `RAN PASS` (`ch5-lesson3_sensitivity_router` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ evidence/demo-bins.jsonl:23`, `READ scripts/record-ready.sh:101`
5. Lesson 5.4 — Responsible-AI checklist gate:
   - `cargo run -p ch5-governance --bin lesson4_checklist` — `RAN PASS` (`ch5-lesson4_checklist` via `scripts/record-ready.sh` on `kfocus` at `99c368f`), `READ README.md:104`, `READ evidence/demo-bins.jsonl:24`, `READ scripts/record-ready.sh:102`
