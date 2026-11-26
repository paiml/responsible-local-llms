#!/usr/bin/env bash
# Derive course/li-c1-rllm/manifest.jsonl (one row per lesson) from the committed outline, the
# teleprompter pieces, the audio checksum list and the lesson-to-demo-bin map. Never hand-edited.
# A row names, for one lesson: its outline entry, its narrated pieces (teleprompter + wav sha256),
# its demo bins (each must exist in evidence/demo-bins.jsonl) and the plate, SRT and render state.
# Plates and SRTs are not in this repo: the manifest says `missing` until a PR adds them.
# Usage: scripts/course-manifest.sh          # rewrite course/li-c1-rllm/manifest.jsonl
#        scripts/course-manifest.sh --check  # exit 1 if stale or any cross-check fails
set -euo pipefail
cd "$(dirname "$0")/.."
dir=course/li-c1-rllm
out=$dir/manifest.jsonl

derive() {
  local lesson id piece sha pieces bins t l b
  while IFS= read -r lesson; do
    id=$(jq -r '.file | split("-")[0]' <<<"$lesson")
    pieces='[]'
    for t in "$dir"/teleprompter/"$id"-p*.teleprompter.md; do
      [ -e "$t" ] || { echo "course-manifest: no teleprompter pieces for lesson $id" >&2; return 1; }
      piece=$(basename "$t" .teleprompter.md)
      sha=$(awk -v p="$piece.wav" '$2 == p { print $1 }' "$dir/audio.sha256")
      [ -n "$sha" ] || { echo "course-manifest: no audio checksum for $piece" >&2; return 1; }
      pieces=$(jq -c --arg p "$piece" --arg t "$t" --arg s "$sha" '. + [{piece:$p,teleprompter:$t,wav_sha256:$s}]' <<<"$pieces")
    done
    bins='[]'
    while IFS=$'\t' read -r l b; do
      [ "$l" = "$id" ] || continue
      grep -q "\"id\":\"$b\"" evidence/demo-bins.jsonl || { echo "course-manifest: demo bin $b not in evidence/demo-bins.jsonl" >&2; return 1; }
      bins=$(jq -c --arg b "$b" '. + [$b]' <<<"$bins")
    done < "$dir/lesson-demos.tsv"
    jq -c --arg id "$id" --argjson pieces "$pieces" --argjson bins "$bins" \
      '{lesson:$id,index:.index,module:.module,title:.lesson,outline_file:.file,pieces:$pieces,demo_bins:$bins,plate:"missing",srt:"missing",rendered:false}' <<<"$lesson"
  done < "$dir/outline.jsonl"
}

check_counts() {
  local n p w
  n=$(wc -l < "$dir/outline.jsonl")
  [ "$n" -eq 35 ] || { echo "course-manifest: outline has $n lessons, expected 35" >&2; return 1; }
  p=$(find "$dir/teleprompter" -name '*.teleprompter.md' | wc -l)
  w=$(wc -l < "$dir/audio.sha256")
  [ "$p" -eq "$w" ] || { echo "course-manifest: $p teleprompter pieces but $w audio checksums" >&2; return 1; }
}

check_counts
if [ "${1:-}" = "--check" ]; then
  if ! diff -u "$out" <(derive); then
    echo "course-manifest: $out is stale; run scripts/course-manifest.sh and commit" >&2
    exit 1
  fi
  echo "course-manifest: $out matches the outline, pieces and demo bins"
else
  derive > "$out"
fi
