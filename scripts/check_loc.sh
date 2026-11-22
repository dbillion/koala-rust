#!/usr/bin/env bash
set -euo pipefail

limit=99
allowlist="$(dirname "$0")/loc_allowlist.txt"
failed=0

if [[ ! -f "$allowlist" ]]; then
  echo "Missing LOC allowlist: $allowlist" >&2
  exit 2
fi

while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  [[ -f "$file" ]] || {
    echo "STALE allowlist entry: $file (file no longer exists)" >&2
    failed=1
    continue
  }
  lines="$(wc -l < "$file")"
  if (( lines <= limit )); then
    echo "STALE allowlist entry: $file has $lines lines (limit $limit)" >&2
    failed=1
  fi
done < "$allowlist"

while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  lines="$(wc -l < "$file")"
  if (( lines > limit )) && ! grep -Fxq "$file" "$allowlist"; then
    echo "FAIL: $file has $lines lines (limit $limit)" >&2
    failed=1
  fi
done < <(find . -type f -name '*.rs' \
  -not -path './.git/*' \
  -not -path './target/*' \
  -not -path './.codegraph/*' | sed 's|^\./||' | sort)

if (( failed != 0 )); then
  exit 1
fi

echo "PASS: every Rust file is <= $limit lines or explicitly allowlisted"
