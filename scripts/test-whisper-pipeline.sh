#!/usr/bin/env bash
# Offline test for whisper-pipeline.sh with stub whisper binaries (no whisper.cpp, no model).
# Cases: dry-run output shape, order, empty transcript refused, whitespace-only refused,
# missing input refused, no-arg usage refused, missing WHISPER_MODEL refused, and the
# non-dry-run path against a stub curl. Each refusal case also asserts its exit code.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
touch "$tmp/a.wav" "$tmp/b.wav"
CONTRACT='contract: merged transcript has one non-empty line per file OK'

cat > "$tmp/whisper-stub" <<'STUB'
#!/usr/bin/env bash
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-f" ]; then printf 'hello from %s\nsecond line\n' "$(basename "$2")"; fi
  shift
done
STUB
cat > "$tmp/whisper-empty" <<'STUB'
#!/usr/bin/env bash
exit 0
STUB
cat > "$tmp/whisper-blank" <<'STUB'
#!/usr/bin/env bash
printf '   \n\n'
STUB
chmod +x "$tmp"/whisper-*

run() { WHISPER_BIN="$1" WHISPER_MODEL=x RLL_DRY_RUN=1 "$here/whisper-pipeline.sh" "${@:2}"; }

# Dry-run output shape: exactly N merged lines (a.wav then b.wav, newlines folded), then the contract line.
out="$(run "$tmp/whisper-stub" "$tmp/a.wav" "$tmp/b.wav")"
[ "$(printf '%s\n' "$out" | wc -l)" -eq 3 ] || fail "expected 2 transcript lines + contract line, got: $out"
[ "$(printf '%s\n' "$out" | sed -n 1p)" = "hello from a.wav second line" ] || fail "line 1 wrong: $out"
[ "$(printf '%s\n' "$out" | sed -n 2p)" = "hello from b.wav second line" ] || fail "line 2 wrong (order): $out"
[ "$(printf '%s\n' "$out" | sed -n 3p)" = "$CONTRACT" ] || fail "contract line wrong: $out"

# Empty transcript refused (exit 1, names the violation, no contract line).
rc=0; err="$(run "$tmp/whisper-empty" "$tmp/a.wav" 2>&1 >"$tmp/o")" || rc=$?
[ "$rc" -eq 1 ] || fail "empty transcript: expected exit 1, got $rc"
echo "$err" | grep -q 'contract violated: 1 lines, 0 non-empty, for 1 files' || fail "empty transcript: wrong message: $err"
! grep -q 'contract:' "$tmp/o" || fail "empty transcript printed a contract line"

# Whitespace-only transcript is empty too.
rc=0; run "$tmp/whisper-blank" "$tmp/a.wav" >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 1 ] || fail "blank transcript: expected exit 1, got $rc"

# One empty transcript among good ones breaks the contract (falsifier: contract is per file).
rc=0; run "$tmp/whisper-empty" "$tmp/a.wav" "$tmp/b.wav" >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 1 ] || fail "all-empty two files: expected exit 1, got $rc"

# Missing input refused before whisper runs (exit 2, names the file).
rc=0; err="$(run "$tmp/whisper-stub" "$tmp/a.wav" "$tmp/nope.wav" 2>&1 >"$tmp/o")" || rc=$?
[ "$rc" -eq 2 ] || fail "missing input: expected exit 2, got $rc"
echo "$err" | grep -q "missing input: $tmp/nope.wav" || fail "missing input: wrong message: $err"
[ ! -s "$tmp/o" ] || fail "missing input produced output"

# No arguments: usage, exit 2.
rc=0; "$here/whisper-pipeline.sh" >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 2 ] || fail "no-arg: expected exit 2, got $rc"

# WHISPER_MODEL unset: refused (non-zero).
rc=0; env -u WHISPER_MODEL WHISPER_BIN="$tmp/whisper-stub" "$here/whisper-pipeline.sh" "$tmp/a.wav" >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || fail "unset WHISPER_MODEL should fail"

# Non-dry-run path against stub jq/curl on PATH (recorded response, no live runtime).
mkdir "$tmp/bin"
cat > "$tmp/bin/curl" <<'STUB'
#!/usr/bin/env bash
cat >/dev/null
printf '%s' '{"choices":[{"message":{"content":"- one\n- two\n- three"}}]}'
STUB
chmod +x "$tmp/bin/curl"
out="$(PATH="$tmp/bin:$PATH" WHISPER_BIN="$tmp/whisper-stub" WHISPER_MODEL=x "$here/whisper-pipeline.sh" "$tmp/a.wav")"
printf '%s\n' "$out" | sed -n 1p | grep -q '^- one$' || fail "model reply not printed: $out"
printf '%s\n' "$out" | tail -n 1 | grep -q "^$CONTRACT\$" || fail "no contract line after model reply: $out"

echo "whisper-pipeline test OK"
