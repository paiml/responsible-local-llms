#!/usr/bin/env bash
# Falsifier for scripts/course-manifest.sh. The check passes on the tree, and each planted defect
# is refused BY the check it targets: the refusal must print that check's own message. A removed
# check therefore fails here, whether the defect would then be accepted or refused for another reason.
# Each defect is planted in a temp copy of the gate's inputs; the tree is never touched.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
dir=course/li-c1-rllm
scripts/course-manifest.sh --check >/dev/null || { echo "FAIL: clean tree rejected"; exit 1; }

# Each defect is a function (not a quoted string), so the edit is real shell code that the
# lint sees. Functions run inside the temp copy, so $dir resolves there.
defect_stale_manifest() {
  sed -i '1s/"rendered":false/"rendered":true/' "$dir/manifest.jsonl"
}
defect_outline_count() {
  sed -i '$d' "$dir/outline.jsonl"
}
defect_piece_checksum_count() {
  sed -i '1,2d' "$dir/audio.sha256"
}
defect_lesson_without_pieces() {
  rm -f course/li-c1-rllm/teleprompter/0.0-p*.teleprompter.md
  sed -i '1,3d' "$dir/audio.sha256"
}
defect_checksum_lookup() {
  sed -i 's/ 0\.0-p01\.wav$/ 0.0-p99.wav/' "$dir/audio.sha256"
}
defect_unknown_demo_bin() {
  sed -i '1s/lesson2_three_runtimes/no_such_bin/' "$dir/lesson-demos.tsv"
}

# plant NAME WANT EDIT_FN: copy the gate's inputs to a temp tree, run the edit function there,
# and require the gate to exit non-zero with WANT (a message from the targeted check) on stderr.
plant() {
  local t err
  t=$(mktemp -d) || exit 1
  cp -r scripts course evidence "$t"/
  (cd "$t" && "$3")
  if err=$(cd "$t" && scripts/course-manifest.sh --check 2>&1 >/dev/null); then
    echo "FAIL: $1 accepted"; rm -rf "${t:?}"; exit 1
  fi
  if ! grep -qF -- "$2" <<<"$err"; then
    echo "FAIL: $1 refused, but not by its check (wanted: $2; got: $err)"; rm -rf "${t:?}"; exit 1
  fi
  rm -rf "${t:?}"
}

plant "stale manifest" "is stale" defect_stale_manifest
plant "outline count" "outline has 34 lessons, expected 35" defect_outline_count
plant "piece/checksum count" "teleprompter pieces but 135 audio checksums" defect_piece_checksum_count
plant "lesson without pieces" "no teleprompter pieces for lesson 0.0" defect_lesson_without_pieces
plant "checksum lookup" "no audio checksum for 0.0-p01" defect_checksum_lookup
plant "unknown demo bin" "demo bin no_such_bin not in" defect_unknown_demo_bin
echo "course-manifest falsifier: OK (6 planted defects, each refused by its own check)"
