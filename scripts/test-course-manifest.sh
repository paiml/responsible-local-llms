#!/usr/bin/env bash
# Falsifier for scripts/course-manifest.sh: the check passes on the tree, and each of four planted
# defects (stale manifest, missing checksum, unknown demo bin, short outline) makes it fail.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
dir=course/li-c1-rllm
scripts/course-manifest.sh --check >/dev/null || { echo "FAIL: clean tree rejected"; exit 1; }
bak=$(mktemp -d)
cp "$dir/manifest.jsonl" "$dir/audio.sha256" "$dir/lesson-demos.tsv" "$dir/outline.jsonl" "$bak"/
restore() { cp "$bak"/* "$dir"/; }
trap restore EXIT
expect_fail() {
  if scripts/course-manifest.sh --check >/dev/null 2>&1; then echo "FAIL: $1 accepted"; exit 1; fi
  restore
}
sed -i '1s/"rendered":false/"rendered":true/' "$dir/manifest.jsonl"; expect_fail "stale manifest"
sed -i '1,2d' "$dir/audio.sha256"; expect_fail "missing checksum"
sed -i '1s/lesson2_three_runtimes/no_such_bin/' "$dir/lesson-demos.tsv"; expect_fail "unknown demo bin"
sed -i '$d' "$dir/outline.jsonl"; expect_fail "short outline"
echo "course-manifest falsifier: OK (4 planted defects refused)"
