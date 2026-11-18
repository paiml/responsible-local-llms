#!/usr/bin/env bash
# Serve the same GGUF with llama.cpp's llama-server on 127.0.0.1:8080.
# Usage: GGUF=/path/model.gguf scripts/serve-llamacpp.sh
set -euo pipefail
: "${GGUF:?set GGUF to the model file}"
PORT="${RLL_LLAMACPP_PORT:-8080}"
exec llama-server -m "$GGUF" --host 127.0.0.1 --port "$PORT"
