#!/usr/bin/env bash
set -euo pipefail

: "$KOALA_BIN"
: "$KOALA_MODEL_DIR"
: "$KOALA_VEC0_PATH"

if [[ ! -d "$KOALA_MODEL_DIR" ]]; then
  echo "Model directory does not exist: $KOALA_MODEL_DIR" >&2
  exit 2
fi
if [[ ! -f "$KOALA_VEC0_PATH" ]]; then
  echo "sqlite-vec extension does not exist: $KOALA_VEC0_PATH" >&2
  exit 2
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export KOALA_MODEL_DIR KOALA_VEC0_PATH
export KOALA_STATE_DIR="$tmp/state"
mkdir -p "$KOALA_STATE_DIR"

run_offline() {
  # --state-dir is explicit so the test never touches a developer's real state.
  # The user+network namespace provides an OS-level no-network boundary; the
  # no-default-features binary also omits runtime HTTP clients.
  unshare -Urn "$KOALA_BIN" --offline --state-dir "$KOALA_STATE_DIR" --format json "$@"
}

run_offline remember \
  --name offline-e2e-proof \
  --text "A locally provisioned model can remember and recall this air-gapped verification phrase."

output="$(run_offline recall --query "air-gapped verification phrase" --k 3)"
printf '%s\n' "$output"
grep -q "offline-e2e-proof" <<<"$output"

if run_offline model fetch --to "$tmp/should-not-download" >/dev/null 2>&1; then
  echo "model fetch unexpectedly succeeded in offline mode" >&2
  exit 1
fi

echo "OFFLINE_E2E_OK"
