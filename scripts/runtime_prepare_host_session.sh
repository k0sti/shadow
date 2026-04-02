#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INPUT_PATH="${SHADOW_RUNTIME_APP_INPUT_PATH:-runtime/app-compile-smoke/app.tsx}"
CACHE_DIR="${SHADOW_RUNTIME_APP_CACHE_DIR:-build/runtime/app-host}"

cd "$REPO_ROOT"

bundle_json="$(
  nix develop .#runtime -c deno run --quiet \
    --allow-env --allow-read --allow-write --allow-run \
    scripts/runtime_prepare_app_bundle.ts \
    --input "$INPUT_PATH" \
    --cache-dir "$CACHE_DIR"
)"

bundle_path="$(
  printf '%s\n' "$bundle_json" | python3 -c '
import json
import os
import sys

data = json.load(sys.stdin)
print(os.path.abspath(data["bundlePath"]))
'
)"

runtime_host_prefix="$(
  nix build --accept-flake-config .#deno-core-smoke --no-link --print-out-paths
)"
runtime_host_binary_path="${runtime_host_prefix}/bin/deno-core-smoke"

python3 - "$bundle_path" "$runtime_host_binary_path" "$INPUT_PATH" "$CACHE_DIR" <<'PY'
import json
import os
import sys

bundle_path, runtime_host_binary_path, input_path, cache_dir = sys.argv[1:5]
print(json.dumps({
    "bundlePath": bundle_path,
    "cacheDir": cache_dir,
    "inputPath": input_path,
    "runtimeHostBinaryPath": runtime_host_binary_path,
}, indent=2))
PY
