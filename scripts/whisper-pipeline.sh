#!/usr/bin/env bash
# Lesson 1.5: transcribe audio with whisper.cpp, merge the transcripts and
# prompt a local model, all in shell.
# Usage: scripts/whisper-pipeline.sh a.wav [b.wav ...]
# Env:   WHISPER_BIN (default whisper-cli), WHISPER_MODEL (ggml model path),
#        RLL_BASE_URL (default http://127.0.0.1:8081), RLL_MODEL,
#        RLL_DRY_RUN=1 prints the merged transcript and skips the model call.
#
# Provable contract: the merged transcript holds one line per input file, in order.
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: $0 audio.wav [more.wav ...]" >&2
  exit 2
fi

WHISPER_BIN="${WHISPER_BIN:-whisper-cli}"
BASE_URL="${RLL_BASE_URL:-http://127.0.0.1:8081}"
MODEL="${RLL_MODEL:-qwen3-0.6b}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

: > "$tmp/merged.txt"
for wav in "$@"; do
  "$WHISPER_BIN" -m "${WHISPER_MODEL:?set WHISPER_MODEL to a ggml model}" -f "$wav" -nt -np \
    | tr '\n' ' ' | sed 's/[[:space:]]*$//' >> "$tmp/merged.txt"
  printf '\n' >> "$tmp/merged.txt"
done

lines="$(wc -l < "$tmp/merged.txt")"
if [ "$lines" -ne "$#" ]; then
  echo "contract violated: $lines transcript lines for $# files" >&2
  exit 1
fi

if [ "${RLL_DRY_RUN:-0}" = "1" ]; then
  cat "$tmp/merged.txt"
else
  prompt="Summarize these transcripts in three bullet points: $(tr '\n' ' ' < "$tmp/merged.txt")"
  jq -n --arg m "$MODEL" --arg p "$prompt" \
    '{model:$m, temperature:0, seed:7, stream:false, messages:[{role:"user", content:$p}]}' \
    | curl -fsS "$BASE_URL/v1/chat/completions" -H 'Content-Type: application/json' -d @- \
    | jq -r '.choices[0].message.content'
fi
echo "contract: merged transcript has one line per file OK"
