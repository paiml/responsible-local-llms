#!/usr/bin/env bash
# Live suite: start real local runtimes, then run the same demo-bin tests against each
# one through RLL_LIVE=1 / RLL_RUNTIME / RLL_BASE_URL / RLL_MODEL.
#
# Usage: scripts/live-suite.sh [runtime ...]     (default: $RLL_LIVE_RUNTIMES, else "llamacpp ollama")
#   llamacpp, ollama  run from pinned container images (needs docker; nothing is installed on the host)
#   apr               runs scripts/serve-apr.sh (needs `apr` on PATH)
# Env: RLL_QUANT (set to q4_k_m here: the cached model) labels the lesson3_quality score.
#      RLL_MODEL_DIR  where the test model is cached (default .cache/models)
#
# A runtime that cannot start fails the run with a message naming what is missing.
set -euo pipefail
cd "$(dirname "$0")/.."

LLAMACPP_IMAGE=ghcr.io/ggml-org/llama.cpp:server-b12110
OLLAMA_IMAGE=ollama/ollama:0.40.13
MODEL_URL=https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q4_K_M.gguf
MODEL_SHA256=ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a
MODEL_NAME=qwen3-0.6b

die() { echo "live-suite: FAIL: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is not on PATH ($2); this job never installs it"; }

read -r -a runtimes <<< "${*:-${RLL_LIVE_RUNTIMES:-llamacpp ollama}}"
need curl "used to fetch the test model and probe health"
need sha256sum "used to verify the test model"

model_dir=${RLL_MODEL_DIR:-.cache/models}
gguf=$model_dir/Qwen3-0.6B-Q4_K_M.gguf
mkdir -p "$model_dir"
if ! echo "$MODEL_SHA256  $gguf" | sha256sum -c --status 2>/dev/null; then
  echo "live-suite: fetching test model"
  curl -fsSL --retry 3 -o "$gguf.part" "$MODEL_URL"
  mv "$gguf.part" "$gguf"
  echo "$MODEL_SHA256  $gguf" | sha256sum -c --status || die "test model sha256 mismatch"
fi

containers=()
pids=()
cleanup() {
  for c in "${containers[@]}"; do docker rm -f "$c" >/dev/null 2>&1 || true; done
  for p in "${pids[@]}"; do kill "$p" 2>/dev/null || true; done
}
trap cleanup EXIT

# Start a container with the model copied in (no host bind mount), echo its bridge IP.
start_container() { # name image args...
  local name=$1 image=$2; shift 2
  docker create --name "$name" "$image" "$@" >/dev/null
  containers+=("$name")
  docker cp "$gguf" "$name:/tmp/model.gguf" >/dev/null
  docker start "$name" >/dev/null
  docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "$name"
}

wait_up() { # url what
  for _ in $(seq 1 120); do
    curl -fsS -o /dev/null "$1" 2>/dev/null && return 0
    sleep 1
  done
  die "$2 did not answer $1 within 120 s"
}

base_url() { # runtime -> sets BASE
  local rt=$1 ip
  case $rt in
    llamacpp)
      need docker "llama.cpp runs from $LLAMACPP_IMAGE"
      ip=$(start_container "rll-llamacpp-$$" "$LLAMACPP_IMAGE" -m /tmp/model.gguf --host 0.0.0.0 --port 8080)
      BASE=http://$ip:8080
      wait_up "$BASE/health" "llama.cpp" ;;
    ollama)
      need docker "Ollama runs from $OLLAMA_IMAGE"
      ip=$(start_container "rll-ollama-$$" "$OLLAMA_IMAGE")
      BASE=http://$ip:11434
      wait_up "$BASE/api/tags" "ollama"
      docker exec "rll-ollama-$$" sh -c "printf 'FROM /tmp/model.gguf\n' > /tmp/Modelfile && ollama create $MODEL_NAME -f /tmp/Modelfile" >/dev/null ;;
    apr)
      need apr "the apr leg runs scripts/serve-apr.sh"
      GGUF=$gguf RLL_APR_PORT=18081 scripts/serve-apr.sh >/dev/null 2>&1 &
      pids+=("$!")
      BASE=http://127.0.0.1:18081
      wait_up "$BASE/v1/models" "apr" ;;
    *) die "unknown runtime '$rt' (apr, ollama, llamacpp)" ;;
  esac
}

declare -A urls=()
for rt in "${runtimes[@]}"; do
  base_url "$rt"
  urls[$rt]=$BASE
  echo "live-suite: $rt up at $BASE"
  RLL_LIVE=1 RLL_QUANT=q4_k_m RLL_RUNTIME=$rt RLL_BASE_URL=$BASE RLL_MODEL=$MODEL_NAME \
    cargo test -q -p ch1-sovereign --test configured_runtime -- --nocapture --test-threads 1
  RLL_LIVE=1 RLL_RUNTIME=$rt RLL_BASE_URL=$BASE RLL_MODEL=$MODEL_NAME \
    cargo test -q -p ch2-evals --test configured_runtime -- --nocapture --test-threads 1
  RLL_LIVE=1 RLL_RUNTIME=$rt RLL_BASE_URL=$BASE RLL_MODEL=$MODEL_NAME \
    cargo test -q -p ch4-agents --test configured_runtime -- --nocapture --test-threads 1
  echo "live-suite: $rt PASS"
done
# Lesson 1.2 asks all three runtimes in one run, so it needs all three up at once. The
# servers started above stay up until this script exits.
if [ -n "${urls[apr]:-}" ] && [ -n "${urls[ollama]:-}" ] && [ -n "${urls[llamacpp]:-}" ]; then
  RLL_LIVE=1 RLL_MODEL=$MODEL_NAME RLL_BASE_URL_APR=${urls[apr]} RLL_BASE_URL_OLLAMA=${urls[ollama]} \
    RLL_BASE_URL_LLAMACPP=${urls[llamacpp]} \
    cargo test -q -p ch1-sovereign --test configured_runtime lesson2_three_runtimes -- --nocapture --test-threads 1
  echo "live-suite: lesson 1.2 PASS (all three runtimes at once)"
else
  echo "live-suite: lesson 1.2 skipped (needs apr, ollama and llamacpp in one run)"
fi
echo "live-suite: PASS (${runtimes[*]})"
