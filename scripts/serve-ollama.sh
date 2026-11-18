#!/usr/bin/env bash
# Serve the same GGUF with Ollama on 127.0.0.1:11434 (OpenAI-compatible).
# Usage: GGUF=/path/model.gguf scripts/serve-ollama.sh
set -euo pipefail
: "${GGUF:?set GGUF to the model file}"
NAME="${RLL_MODEL:-qwen3-0.6b}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
printf 'FROM %s\n' "$GGUF" > "$tmp/Modelfile"
if ! ollama list >/dev/null 2>&1; then
  ollama serve >/dev/null 2>&1 &
  sleep 2
fi
ollama create "$NAME" -f "$tmp/Modelfile"
echo "ollama is serving $NAME on 127.0.0.1:11434; press Ctrl-C to stop"
wait
