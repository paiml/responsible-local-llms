#!/usr/bin/env bash
# Falsifier for scripts/course-manifest.sh. The check passes on the tree, and each planted defect
# is refused BY the check it targets: the refusal must print that check's own message. A removed
# check therefore fails here, whether the defect would then be accepted or refused for another reason.
# Each defect is planted in a temp copy of the gate's inputs; the tree is never touched.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
dir=course/li-c1-rllm
scripts/course-manifest.sh --check >/dev/null || { echo "FAIL: clean tree rejected"; exit 1; }

# plant NAME WANT EDIT: copy the gate's inputs to a temp tree, run EDIT there, and require the gate
# to exit non-zero with WANT (a message from the targeted check) on stderr.
plant() {
  local t err
  t=$(mktemp -d) || exit 1
  cp -r scripts course evidence "$t"/
  (cd "$t" && eval "$3")
  if err=$(cd "$t" && scripts/course-manifest.sh --check 2>&1 >/dev/null); then
    echo "FAIL: $1 accepted"; rm -rf "${t:?}"; exit 1
  fi
  if ! grep -qF -- "$2" <<<"$err"; then
    echo "FAIL: $1 refused, but not by its check (wanted: $2; got: $err)"; rm -rf "${t:?}"; exit 1
  fi
  rm -rf "${t:?}"
}

plant "stale manifest" "is stale" \
  'sed -i "1s/\"rendered\":false/\"rendered\":true/" $dir/manifest.jsonl'
plant "outline count" "outline has 34 lessons, expected 35" \
  'sed -i "\$d" $dir/outline.jsonl'
plant "piece/checksum count" "teleprompter pieces but 135 audio checksums" \
  'sed -i "1,2d" $dir/audio.sha256'
plant "checksum lookup" "no audio checksum for 0.0-p01" \
  'sed -i "s/ 0\.0-p01\.wav\$/ 0.0-p99.wav/" $dir/audio.sha256'
plant "unknown demo bin" "demo bin no_such_bin not in" \
  'sed -i "1s/lesson2_three_runtimes/no_such_bin/" $dir/lesson-demos.tsv'
echo "course-manifest falsifier: OK (5 planted defects, each refused by its own check)"
