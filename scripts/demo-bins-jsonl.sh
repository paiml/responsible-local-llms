#!/usr/bin/env bash
# Derive evidence/demo-bins.jsonl from crates/*/src/bin/*.rs: one row per demo bin, with the
# facts the demo-bin-v1 shape requires (a `Provable contract:` docstring line, an assert! in
# main, and a `contract: ... OK` line printed after it). The JSONL is never hand-edited.
# Usage: scripts/demo-bins-jsonl.sh          # rewrite evidence/demo-bins.jsonl
#        scripts/demo-bins-jsonl.sh --check  # exit 1 if the tracked file is stale
set -euo pipefail
cd "$(dirname "$0")/.."
out=evidence/demo-bins.jsonl

derive() {
  local f crate bin contract asserts prints ordered a p
  for f in crates/*/src/bin/*.rs; do
    crate=${f#crates/}; crate=${crate%%/*}
    bin=$(basename "$f" .rs)
    contract=false asserts=false prints=false ordered=false
    grep -q '^//! Provable contract: .\{10,\}' "$f" && contract=true
    a=$(grep -n 'assert\(_eq\)\?!(' "$f" | tail -n 1 | cut -d: -f1 || true)
    p=$(grep -n 'println!("contract: .* OK")' "$f" | tail -n 1 | cut -d: -f1 || true)
    [ -n "$a" ] && asserts=true
    [ -n "$p" ] && prints=true
    [ -n "$a" ] && [ -n "$p" ] && [ "$a" -lt "$p" ] && ordered=true
    printf '{"id":"%s","crate":"%s","source":"%s","has_contract_doc":%s,"asserts_contract":%s,"prints_contract_ok":%s,"assert_before_ok":%s}\n' \
      "$bin" "$crate" "$f" "$contract" "$asserts" "$prints" "$ordered"
  done
}

if [ "${1:-}" = "--check" ]; then
  if ! diff -u "$out" <(derive); then
    echo "demo-bins-jsonl: $out is stale; run scripts/demo-bins-jsonl.sh and commit" >&2
    exit 1
  fi
  echo "demo-bins-jsonl: $out matches the demo bins"
else
  mkdir -p evidence
  derive > "$out"
fi
