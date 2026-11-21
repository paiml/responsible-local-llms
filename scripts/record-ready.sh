#!/usr/bin/env bash
# Verify every chapter, module, and lesson demo in responsible-local-llms offline.
# Prints one line per item: CHAPTER <name>: PASS|FAIL|SKIP(<reason>) <seconds>s
# Exits non-zero if any check fails or any chapter crate has no runnable demo.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

failures=0

run_check() {
  local name=$1
  shift
  local start end elapsed rc out_file
  out_file="$tmp_dir/out.log"
  start=$(date +%s)
  if "$@" >"$out_file" 2>&1; then
    rc=0
  else
    rc=$?
  fi
  end=$(date +%s)
  elapsed=$((end - start))
  if [ "$rc" -eq 0 ]; then
    printf 'CHAPTER %s: PASS %ss\n' "$name" "$elapsed"
  else
    printf 'CHAPTER %s: FAIL %ss\n' "$name" "$elapsed"
    sed 's/^/  /' "$out_file" >&2
    failures=$((failures + 1))
  fi
}

skip_check() {
  local name=$1
  local reason=$2
  printf 'CHAPTER %s: SKIP(%s) 0s\n' "$name" "$reason"
}

fail_no_demo() {
  local name=$1
  printf 'CHAPTER %s: FAIL(no demo) 0s\n' "$name"
  failures=$((failures + 1))
}

# Shared libraries and CI gates
run_check "rll-client-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p rll-client
run_check "rll-evals-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p rll-evals
run_check "one-gate-tests" cargo test -p one-gate
run_check "one-gate" cargo run --quiet -p one-gate -- .
run_check "evals-table" scripts/check-evals-table.sh
run_check "demo-bins-jsonl" scripts/demo-bins-jsonl.sh --check

# Chapter 1: Sovereign Local LLMs (ch1-sovereign + whisper + live suite)
run_check "ch1-sovereign-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p ch1-sovereign
run_check "ch1-lesson2_three_runtimes" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch1-sovereign --bin lesson2_three_runtimes
run_check "ch1-lesson3_quantize" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch1-sovereign --bin lesson3_quantize
run_check "ch1-lesson3_quality" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch1-sovereign --bin lesson3_quality
run_check "ch1-lesson4_economics" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch1-sovereign --bin lesson4_economics
run_check "ch1-lesson5_whisper-pipeline-offline" scripts/test-whisper-pipeline.sh
skip_check "ch1-lesson5_whisper-pipeline-live" "live"
run_check "ch1-live-suite-partial-guard" scripts/test-live-suite-partial.sh
skip_check "ch1-live-runtimes" "live"

# Chapter 2: Evals as Code (ch2-evals)
run_check "ch2-evals-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p ch2-evals
run_check "ch2-lesson1_test_set_spec" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch2-evals --bin lesson1_test_set_spec
run_check "ch2-lesson2_hallucination" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch2-evals --bin lesson2_hallucination
run_check "ch2-lesson3_structured" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch2-evals --bin lesson3_structured
run_check "ch2-lesson4_flakiness" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch2-evals --bin lesson4_flakiness
run_check "ch2-lesson5_elo" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch2-evals --bin lesson5_elo
run_check "ch2-lesson6_ci_gate" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch2-evals --bin lesson6_ci_gate

# Chapter 3: Safety, Guardrails & Red-Teaming (ch3-safety, ch3-guard, ch3-redteam)
run_check "ch3-safety-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p ch3-safety
run_check "ch3-lesson1_bias_probes" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch3-safety --bin lesson1_bias_probes
run_check "ch3-lesson2_prompt_injection" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch3-safety --bin lesson2_prompt_injection
run_check "ch3-guard-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p ch3-guard
run_check "ch3-lesson33_pii_redaction" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch3-guard --bin lesson33_pii_redaction
run_check "ch3-lesson34_guardrails" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch3-guard --bin lesson34_guardrails
run_check "ch3-redteam-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p ch3-redteam
run_check "ch3-lesson5_redteam" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch3-redteam --bin lesson5_redteam

# Chapter 4: Safe Local Agents (ch4-agents)
run_check "ch4-agents-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p ch4-agents
run_check "ch4-lesson1_typed_agent" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch4-agents --bin lesson1_typed_agent
run_check "ch4-lesson2_mcp_loopback" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch4-agents --bin lesson2_mcp_loopback
run_check "ch4-lesson3_agency_limits" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch4-agents --bin lesson3_agency_limits
run_check "ch4-lesson4_tool_abuse" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch4-agents --bin lesson4_tool_abuse
run_check "ch4-lesson5_approval_audit" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch4-agents --bin lesson5_approval_audit

# Chapter 5: Governance as Code (ch5-governance)
run_check "ch5-governance-tests" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo test -p ch5-governance
run_check "ch5-lesson1_licence_check" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch5-governance --bin lesson1_licence_check
run_check "ch5-lesson2_ai_act_docs" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch5-governance --bin lesson2_ai_act_docs
run_check "ch5-lesson3_sensitivity_router" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch5-governance --bin lesson3_sensitivity_router
run_check "ch5-lesson4_checklist" env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL cargo run -p ch5-governance --bin lesson4_checklist

# Guard: any chapter crate without a runnable demo bin must fail loudly
for crate_dir in crates/ch*; do
  [ -d "$crate_dir" ] || continue
  crate=$(basename "$crate_dir")
  has_bin=0
  for bin_file in "$crate_dir"/src/bin/*.rs; do
    if [ -f "$bin_file" ]; then
      has_bin=1
      break
    fi
  done
  if [ "$has_bin" -eq 0 ]; then
    fail_no_demo "$crate"
  fi
done

if [ "$failures" -ne 0 ]; then
  exit 1
fi
