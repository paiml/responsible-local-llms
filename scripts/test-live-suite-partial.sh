#!/usr/bin/env bash
# Falsifier for scripts/live-suite.sh: a run without all three runtimes must FAIL, never print
# PASS with lesson 1.2 skipped (#56). It must refuse before starting anything.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
for subset in "apr" "llamacpp ollama" "apr ollama" "apr llamacpp"; do
  # shellcheck disable=SC2086
  out=$(scripts/live-suite.sh $subset 2>&1); rc=$?
  [ "$rc" -ne 0 ] || { echo "FAIL: live-suite.sh $subset exited 0"; exit 1; }
  case $out in *"live-suite: FAIL: lesson 1.2 needs apr, llamacpp and ollama"*) ;; *) echo "FAIL: no refusal for '$subset': $out"; exit 1;; esac
  case $out in *PASS*) echo "FAIL: '$subset' printed PASS"; exit 1;; esac
done
echo "live-suite partial-run falsifier: OK (4 subsets refused)"
