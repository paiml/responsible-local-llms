#!/usr/bin/env bash
# Write evidence/binary/snapshot.jsonl (schema binary-snapshot/v1), the record pv's
# binary extractor reads: one line per demo bin, measured at HEAD. pv never runs a binary;
# this script does, offline (recorded fixtures), and records what it saw.
# Usage: cargo build --workspace --bins && scripts/bin-snapshot.sh
#        BIN_SNAPSHOT_ONLY=<target> scripts/bin-snapshot.sh   (add or refresh one row, keep the rest)
#        Bins are crates/*/src/bin/*.rs and crates/*/src/main.rs (target = the package name).
#        (rerun and commit when a bin is added or changed; honours CARGO_TARGET_DIR)
set -euo pipefail
cd "$(dirname "$0")/.."
out=evidence/binary/snapshot.jsonl
sha=$(git rev-parse HEAD)
sum() { sha256sum | cut -d' ' -f1; }
mkdir -p "$(dirname "$out")"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
for f in crates/*/src/bin/*.rs crates/*/src/main.rs; do
  pkg=${f#crates/}; pkg=${pkg%%/*}
  tgt=$(basename "$f" .rs); [ "$tgt" = main ] && tgt=$pkg
  [ -z "${BIN_SNAPSHOT_ONLY:-}" ] || [ "$tgt" = "$BIN_SNAPSHOT_ONLY" ] || continue
  exe=${CARGO_TARGET_DIR:-target}/debug/$tgt
  [ -x "$exe" ] || { echo "bin-snapshot: $exe not built; run cargo build --workspace --bins first" >&2; exit 1; }
  ver=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
  version="$tgt $ver (${sha:0:9})"
  set +e
  run=$(env -u RLL_LIVE -u RLL_RUNTIME -u RLL_BASE_URL -u RLL_MODEL "$exe")
  code=$?
  set -e
  printf '{"schema":"binary-snapshot/v1","package":"%s","target":"%s","name":"%s","src_path":"%s","git_sha":"%s","version":"%s","version_sha256":"%s","binary_sha256":"%s","help_sha256":"%s","help_exit":%s,"commands":[],"routes":[],"tools":[]}\n' \
    "$pkg" "$tgt" "$tgt" "$f" "$sha" "$version" \
    "$(printf '%s' "$version" | sum)" "$(sum < "$exe")" "$(printf '%s' "$run" | sum)" "$code" >> "$tmp"
done
if [ -n "${BIN_SNAPSHOT_ONLY:-}" ]; then grep -v "\"target\":\"$BIN_SNAPSHOT_ONLY\"" "$out" >> "$tmp" || true; fi
LC_ALL=C sort "$tmp" > "$out"
echo "bin-snapshot: $(grep -c . "$out") bin targets -> $out (HEAD ${sha:0:9})"
