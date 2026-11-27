#!/usr/bin/env bash
# Falsifier for scripts/live-suite.sh: every container it starts must be removed by its EXIT
# cleanup. Regression for the leak on gx10 (2026-11-27): base_url called start_container inside
# $(...), so the subshell's containers+= never reached cleanup and 40 rll-* containers stayed up.
# The test runs the script's own containers/cleanup block and start_container (extracted from
# the file, not copied) against a fake docker that logs every call.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

tmp=$(mktemp -d) || exit 1
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
export LOG="$tmp/docker.log"
cat > "$tmp/bin/docker" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$LOG"
case $1 in inspect) echo 10.0.0.5 ;; esac
EOF
chmod +x "$tmp/bin/docker"
export PATH="$tmp/bin:$PATH"
gguf="$tmp/model.gguf"; : > "$gguf"

# The script's containers=() / pids=() / cleanup block (everything up to the trap line),
# and its start_container function.
block=$(sed -n '/^containers=()/,/^trap cleanup EXIT/p' scripts/live-suite.sh | grep -v '^trap cleanup EXIT')
fn=$(sed -n '/^start_container()/,/^}/p' scripts/live-suite.sh)
if [ -z "$block" ] || [ -z "$fn" ]; then
  echo "FAIL: could not extract cleanup block or start_container"; exit 1
fi
eval "$block"
eval "$fn"

# The fixed call shape: no command substitution, so the name reaches containers[].
start_container rll-test-llamacpp img -m /tmp/model.gguf
start_container rll-test-ollama img
[ "$CONTAINER_IP" = 10.0.0.5 ] || { echo "FAIL: CONTAINER_IP is '$CONTAINER_IP', want 10.0.0.5"; exit 1; }
cleanup

for n in rll-test-llamacpp rll-test-ollama; do
  grep -qx "rm -f $n" "$tmp/docker.log" || { echo "FAIL: cleanup did not remove $n"; cat "$tmp/docker.log"; exit 1; }
done
echo "live-suite cleanup falsifier: OK (2 containers started, 2 removed)"
