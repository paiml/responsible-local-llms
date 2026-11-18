#!/usr/bin/env bash
# Offline test for whisper-pipeline.sh with a stub whisper binary.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/whisper-stub" <<'STUB'
#!/usr/bin/env bash
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-f" ]; then printf 'hello from %s\n' "$2"; fi
  shift
done
STUB
chmod +x "$tmp/whisper-stub"
out="$(WHISPER_BIN="$tmp/whisper-stub" WHISPER_MODEL=x RLL_DRY_RUN=1 "$here/whisper-pipeline.sh" a.wav b.wav)"
echo "$out" | grep -q 'hello from a.wav' || { echo "missing a.wav line" >&2; exit 1; }
echo "$out" | grep -q 'hello from b.wav' || { echo "missing b.wav line" >&2; exit 1; }
echo "$out" | grep -q '^contract: merged transcript has one line per file OK$' || { echo "no contract line" >&2; exit 1; }
if "$here/whisper-pipeline.sh" 2>/dev/null; then echo "no-arg run should fail" >&2; exit 1; fi
echo "whisper-pipeline test OK"
