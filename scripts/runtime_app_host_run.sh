#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

session_json="$(scripts/runtime_prepare_host_session.sh)"
printf '%s\n' "$session_json"

bundle_path="$(
  printf '%s\n' "$session_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["bundlePath"])'
)"
runtime_host_binary_path="$(
  printf '%s\n' "$session_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["runtimeHostBinaryPath"])'
)"

nix develop .#ui -c env \
  SHADOW_BLITZ_DEMO_MODE=runtime \
  SHADOW_RUNTIME_APP_BUNDLE_PATH="$bundle_path" \
  SHADOW_RUNTIME_HOST_BINARY_PATH="$runtime_host_binary_path" \
  cargo run --quiet --manifest-path ui/Cargo.toml -p shadow-blitz-demo
