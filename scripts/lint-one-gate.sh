#!/usr/bin/env bash
# lint-one-gate: ONE GATE (operator ruling 2026-11-19). One check set per repo: every job
# that runs on push to main also runs on pull requests under the same name and is a required
# check, and the committed required list names only jobs that exist and run on pull requests.
#
# pmat comply has no repo-local check hook today (`pmat comply github` judges issues and PRs
# only), so this script is the check, wired into the gate job through `make one-gate`.
#
# Env overrides (the falsifier uses them): WORKFLOW_DIR, REQUIRED_FILE.
set -euo pipefail

WORKFLOW_DIR="${WORKFLOW_DIR:-.github/workflows}"
REQUIRED_FILE="${REQUIRED_FILE:-.github/required-checks.txt}"
fail=0
red() { echo "lint-one-gate: FAIL: $*" >&2; fail=1; }

[ -f "$REQUIRED_FILE" ] || { echo "lint-one-gate: FAIL: $REQUIRED_FILE is missing" >&2; exit 1; }
required=$(awk '!/^[[:space:]]*#/ && NF { sub(/[[:space:]]*$/, ""); print }' "$REQUIRED_FILE" | sort -u)
[ -n "$required" ] || { echo "lint-one-gate: FAIL: $REQUIRED_FILE lists no checks; an empty list is not a gate" >&2; exit 1; }

shopt -s nullglob
files=("$WORKFLOW_DIR"/*.yml "$WORKFLOW_DIR"/*.yaml)
[ "${#files[@]}" -gt 0 ] || { echo "lint-one-gate: FAIL: no workflows in $WORKFLOW_DIR" >&2; exit 1; }

# Emits per workflow: "T <event>" per trigger, "J <check name>" per job, "B <job id>" per
# bad construct (job-level if on event/ref, continue-on-error). Handles the block style
# (`on:` then indented events), the inline list style and the inline scalar style.
scan() {
  awk '
    function trim(s) { gsub(/^[ \t]+|[ \t]+$/, "", s); gsub(/^["\x27]|["\x27]$/, "", s); return s }
    function flush() { if (id != "") { print "J " (nm != "" ? nm : id); id = ""; nm = "" } }
    /^[ \t]*#/ { next }
    /^on:/ || /^"on":/ {
      sect = "on"; rest = $0; sub(/^[^:]*:/, "", rest); rest = trim(rest)
      if (rest != "") { gsub(/[\[\],]/, " ", rest); n = split(rest, a, " "); for (i = 1; i <= n; i++) print "T " a[i] }
      next }
    /^jobs:/ { sect = "jobs"; next }
    /^[^ \t#]/ { sect = "other"; next }
    sect == "on" && /^  [A-Za-z_]+:/ { k = $1; sub(/:.*/, "", k); print "T " k; next }
    sect == "jobs" && /^  [A-Za-z0-9_-]+:[ \t]*$/ { flush(); id = $1; sub(/:$/, "", id); next }
    sect == "jobs" && /^    name:/ { v = $0; sub(/^    name:/, "", v); nm = trim(v); next }
    sect == "jobs" && /^    if:/ { if ($0 ~ /event_name|github\.ref|github\.event/) print "B " id " job-level if on the event or ref (a job that runs on only one side)"; next }
    sect == "jobs" && /continue-on-error/ { print "B " id " continue-on-error (a third state)"; next }
    END { flush() }
  ' "$1"
}

all_pr_jobs=""
for f in "${files[@]}"; do
  out=$(scan "$f")
  triggers=$(awk '$1=="T"{print $2}' <<<"$out")
  jobs=$(awk '$1=="J"{ $1=""; sub(/^ /,""); print }' <<<"$out")
  has() { grep -qx "$1" <<<"$triggers"; }
  main_side=0; pr_side=0
  { has push || has workflow_run || has merge_group; } && main_side=1
  { has pull_request || has pull_request_target; } && pr_side=1
  while IFS= read -r b; do
    [ -n "$b" ] && red "$f: ${b#B }"
  done < <(awk '$1=="B"' <<<"$out")
  if [ "$main_side" = 1 ] && [ "$pr_side" = 0 ]; then
    red "$f: has a main-side trigger (push, workflow_run or merge_group) but no pull_request trigger; its jobs cannot be required on PRs. Add pull_request, or move it to schedule or workflow_dispatch."
  fi
  if [ "$main_side" = 1 ]; then
    while IFS= read -r j; do
      [ -n "$j" ] || continue
      grep -qxF "$j" <<<"$required" || red "$f: job '$j' runs on push to main but is not in $REQUIRED_FILE"
    done <<<"$jobs"
  fi
  if [ "$pr_side" = 1 ]; then all_pr_jobs="$all_pr_jobs$jobs
"; fi
done

while IFS= read -r r; do
  grep -qxF "$r" <<<"$all_pr_jobs" || red "$REQUIRED_FILE lists '$r' but no workflow job of that name runs on pull_request"
done <<<"$required"

if [ "$fail" = 0 ]; then
  echo "lint-one-gate: PASS ($(wc -l <<<"$required" | tr -d ' ') required check(s): $(tr '\n' ' ' <<<"$required"))"
fi
exit "$fail"
