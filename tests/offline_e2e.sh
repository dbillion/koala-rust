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
network_isolation="namespace"
firewall_rule="koala-offline-e2e"

cleanup() {
  if [[ "$network_isolation" == "firewall" ]]; then
    sudo -n iptables -D OUTPUT -m comment --comment "$firewall_rule" -j DROP || true
    sudo -n ip6tables -D OUTPUT -m comment --comment "$firewall_rule" -j DROP || true
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT

# GitHub-hosted runners may disable unprivileged user namespaces. Prefer a
# network namespace; fall back to a temporary OUTPUT firewall rule when sudo
# is available so this test still enforces an OS-level network boundary.
if ! unshare -Urn true >/dev/null 2>&1; then
  if command -v sudo >/dev/null 2>&1 && \
    sudo -n iptables -I OUTPUT -m comment --comment "$firewall_rule" -j DROP; then
    network_isolation="firewall"
    if ! sudo -n ip6tables -I OUTPUT -m comment --comment "$firewall_rule" -j DROP; then
      sudo -n iptables -D OUTPUT -m comment --comment "$firewall_rule" -j DROP || true
      network_isolation="namespace"
      echo "Cannot establish IPv6 firewall isolation" >&2
      exit 77
    fi
  else
    echo "Cannot establish network isolation (unshare and firewall unavailable)" >&2
    exit 77
  fi
fi
export KOALA_MODEL_DIR KOALA_VEC0_PATH
export KOALA_STATE_DIR="$tmp/state"
mkdir -p "$KOALA_STATE_DIR"

run_offline() {
  # --state-dir is explicit so the test never touches a developer's real state.
  # The selected namespace or firewall blocks all network traffic at OS level.
  if [[ "$network_isolation" == "namespace" ]]; then
    unshare -Urn "$KOALA_BIN" --offline --state-dir "$KOALA_STATE_DIR" --format json "$@"
  else
    "$KOALA_BIN" --offline --state-dir "$KOALA_STATE_DIR" --format json "$@"
  fi
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
