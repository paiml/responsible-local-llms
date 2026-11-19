#!/usr/bin/env bash
# Serve one GGUF with apr on 127.0.0.1:8081 (OpenAI-compatible).
# Usage: GGUF=/path/model.gguf scripts/serve-apr.sh
set -euo pipefail
: "${GGUF:?set GGUF to the model file}"
PORT="${RLL_APR_PORT:-8081}"
exec apr serve run "$GGUF" --port "$PORT"
