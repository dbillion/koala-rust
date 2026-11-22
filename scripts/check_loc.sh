#!/usr/bin/env bash
set -euo pipefail

limit=99
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
allowlist="$script_dir/loc_allowlist.txt"
failed=0
base_ref="${1:-}"
tmp_files=()

cleanup() {
  for file in "${tmp_files[@]}"; do
    rm -f "$file"
  done
}
trap cleanup EXIT

if [[ ! -f "$allowlist" ]]; then
  echo "Missing LOC allowlist: $allowlist" >&2
  exit 2
fi

line_count() {
  awk 'END { print NR }' "$1"
}

if [[ -n "$base_ref" ]]; then
  if ! git -C "$repo_root" rev-parse --verify "${base_ref}^{commit}" >/dev/null 2>&1; then
    echo "Cannot resolve base revision: $base_ref" >&2
    exit 2
  fi
  if git -C "$repo_root" cat-file -e "${base_ref}:scripts/loc_allowlist.txt" 2>/dev/null; then
    base_allowlist="$(mktemp)"
    tmp_files+=("$base_allowlist")
    git -C "$repo_root" show "${base_ref}:scripts/loc_allowlist.txt" > "$base_allowlist"
    while IFS= read -r file; do
      [[ -n "$file" ]] || continue
      if ! grep -Fxq "$file" "$base_allowlist"; then
        echo "FAIL: allowlist entry added: $file (only removals are permitted)" >&2
        failed=1
      fi
    done < "$allowlist"
  fi
fi

while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  path="$repo_root/$file"
  [[ -f "$path" ]] || {
    echo "STALE allowlist entry: $file (file no longer exists)" >&2
    failed=1
    continue
  }
  lines="$(line_count "$path")"
  if (( lines <= limit )); then
    echo "STALE allowlist entry: $file has $lines lines (limit $limit)" >&2
    failed=1
  fi
done < "$allowlist"

rust_files="$(mktemp)"
tmp_files+=("$rust_files")
if ! find "$repo_root" -type f -name '*.rs' \
  -not -path '*/.git/*' \
  -not -path '*/target/*' \
  -not -path '*/.codegraph/*' -print0 > "$rust_files"; then
  echo "FAIL: Rust source discovery failed" >&2
  exit 2
fi

while IFS= read -r -d '' path; do
  file="${path#"$repo_root"/}"
  lines="$(line_count "$path")"
  if (( lines > limit )) && ! grep -Fxq "$file" "$allowlist"; then
    echo "FAIL: $file has $lines lines (limit $limit)" >&2
    failed=1
  fi
done < "$rust_files"

if (( failed != 0 )); then
  exit 1
fi

echo "PASS: every Rust file is <= $limit lines or explicitly allowlisted"
