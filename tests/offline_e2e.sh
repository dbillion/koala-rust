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
network_namespace=()

cleanup() {
  rm -rf "$tmp"
}
trap cleanup EXIT

# Never alter the host firewall to simulate an air gap. Prefer an unprivileged
# user+network namespace; if the runner disables user namespaces, use sudo only
# to create a dedicated network namespace and drop back to the invoking UID/GID
# before executing the Koala binary.
if unshare -Urn true >/dev/null 2>&1; then
  network_namespace=(unshare -Urn)
  echo "Using an unprivileged network namespace"
elif command -v sudo >/dev/null 2>&1 && command -v setpriv >/dev/null 2>&1 && sudo -n unshare -n true >/dev/null 2>&1; then
  network_namespace=(
    sudo -n unshare -n --
    setpriv --reuid="$(id -u)" --regid="$(id -g)" --clear-groups
  )
  echo "Using a privileged isolated network namespace"
else
  echo "Cannot establish network isolation; refusing to run the offline test" >&2
  exit 77
fi

export KOALA_MODEL_DIR KOALA_VEC0_PATH
export KOALA_STATE_DIR="$tmp/state"
mkdir -p "$KOALA_STATE_DIR"

run_offline() {
  # --state-dir is explicit so the test never touches a developer's real state.
  # The selected network namespace blocks all network traffic at OS level.
  "${network_namespace[@]}" "$KOALA_BIN" --offline --state-dir "$KOALA_STATE_DIR" --format json "$@"
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
