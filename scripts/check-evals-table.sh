#!/usr/bin/env bash
# Fails when the evals README table names a set that has no file, or a README calls a set "planned".
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0
table=$(awk '/^## The six risks/{t=1;next} /^## /{t=0} t&&/^\|/' evals/README.md | tail -n +3)
[ -n "$table" ] || { echo "evals/README.md: risk table not found" >&2; exit 1; }
while IFS= read -r row; do
  cell=$(printf '%s' "$row" | awk -F'|' '{print $3}')
  # shellcheck disable=SC2016
  for set in $(printf '%s' "$cell" | grep -o '`[^`]*`' | tr -d '`'); do
    [ -f "evals/$set.jsonl" ] || { echo "evals/README.md names set '$set' but evals/$set.jsonl does not exist" >&2; bad=1; }
  done
done <<<"$table"
if grep -n -i 'planned' README.md evals/README.md >&2; then
  echo "a README says a set is planned; write it or remove the mention" >&2
  bad=1
fi
[ "$bad" -eq 0 ] && echo "evals table: every named set exists"
exit "$bad"
