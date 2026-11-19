#!/usr/bin/env bash
# falsify-one-gate: proves scripts/lint-one-gate.sh goes RED, not only green.
# Plants, in a scratch copy: a main-only job, a main-push-only workflow, a missing required
# entry, a required entry with no PR job, continue-on-error, and an empty list.
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
lint="$here/scripts/lint-one-gate.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
bad=0

fresh() { rm -rf "$tmp/w"; mkdir -p "$tmp/w"; cp "$here"/.github/workflows/*.yml "$tmp/w/"; cp "$here/.github/required-checks.txt" "$tmp/req.txt"; }
run() { WORKFLOW_DIR="$tmp/w" REQUIRED_FILE="$tmp/req.txt" "$lint" >"$tmp/out" 2>&1; }
expect_green() { if run; then echo "ok   green: $1"; else echo "BAD  $1 should pass"; cat "$tmp/out"; bad=1; fi; }
expect_red() {
  if run; then echo "BAD  $1 was NOT refused"; bad=1
  elif grep -q "$2" "$tmp/out"; then echo "ok   red:   $1"
  else echo "BAD  $1 refused for the wrong reason"; cat "$tmp/out"; bad=1; fi
}

fresh; expect_green "unmodified repo"

fresh
cat >"$tmp/w/extra.yml" <<'Y'
name: extra
on:
  push:
    branches: [main]
jobs:
  deploy:
    name: deploy
    runs-on: ubuntu-latest
    steps:
      - run: true
Y
expect_red "main-push-only workflow" "no pull_request trigger"

fresh
cat >"$tmp/w/extra.yml" <<'Y'
name: extra
on:
  pull_request:
  push:
    branches: [main]
jobs:
  second:
    name: second
    runs-on: ubuntu-latest
    steps:
      - run: true
Y
expect_red "main-push job missing from the required list" "job 'second' runs on push to main but is not in"

fresh
cat >"$tmp/w/extra.yml" <<'Y'
name: extra
on: [push, pull_request]
jobs:
  only-main:
    name: only-main
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - run: true
Y
printf 'only-main\n' >>"$tmp/req.txt"
expect_red "job-level if that makes a job main-only" "job-level if"

fresh
printf 'ghost\n' >>"$tmp/req.txt"
expect_red "required entry with no PR job" "lists 'ghost'"

fresh
sed -i 's/^    timeout-minutes: 30/    timeout-minutes: 30\n    continue-on-error: true/' "$tmp/w/ci.yml"
expect_red "continue-on-error third state" "continue-on-error"

fresh
printf '# nothing\n' >"$tmp/req.txt"
expect_red "empty required list" "lists no checks"

fresh
cat >"$tmp/w/extra.yml" <<'Y'
name: extra
'on':
  push:
    branches: [main]
jobs:
  sneaky:
    name: sneaky
    runs-on: ubuntu-latest
    steps:
      - run: true
Y
expect_red "single-quoted 'on' key is read, not skipped" "no pull_request trigger"

fresh
cat >"$tmp/w/extra.yml" <<'Y'
name: extra
true:
  push:
jobs:
  odd:
    name: odd
    runs-on: ubuntu-latest
    steps:
      - run: true
Y
expect_red "workflow whose triggers parse empty" "triggers parse to empty"

fresh
sed -i '/^  pull_request:/,/^    branches: \[main\]/d' "$tmp/w/ci.yml"
expect_red "ci.yml loses its pull_request trigger" "no pull_request trigger"

exit "$bad"
