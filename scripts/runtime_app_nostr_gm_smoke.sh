#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=./runtime_host_backend_common.sh
source "$SCRIPT_DIR/runtime_host_backend_common.sh"
INPUT_PATH="${SHADOW_RUNTIME_APP_INPUT_PATH:-runtime/app-nostr-gm/app.tsx}"
CACHE_DIR="${SHADOW_RUNTIME_APP_CACHE_DIR:-build/runtime/app-nostr-gm}"

cd "$REPO_ROOT"
runtime_host_backend_resolve

bundle_json="$(
  nix develop .#runtime -c deno run --quiet --allow-env --allow-read --allow-write --allow-run \
    scripts/runtime_prepare_app_bundle.ts \
    --input "$INPUT_PATH" \
    --cache-dir "$CACHE_DIR"
)"
printf '%s\n' "$bundle_json"

bundle_path="$(
  printf '%s\n' "$bundle_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["bundlePath"])'
)"

output="$(
  nix run --accept-flake-config ".#${SHADOW_RUNTIME_HOST_PACKAGE_ATTR}" -- "$bundle_path"
)"
printf '%s\n' "$output"

python3 -c '
import json
import re
import sys

payload = sys.stdin.read()
for line in reversed(payload.splitlines()):
    match = re.search(r"result=(\{.*\})$", line)
    if not match:
        continue
    document = json.loads(match.group(1))
    html = document.get("html", "")
    required = ["Shadow GM", "GM sent", "https://primal.net/e/note1"]
    missing = [item for item in required if item not in html]
    if missing:
        raise SystemExit(f"gm app html missing markers: {missing!r}")
    break
else:
    raise SystemExit("could not find gm app document payload")
' <<<"$output"

printf 'Runtime app nostr gm smoke succeeded: backend=%s bundle=%s\n' \
  "$SHADOW_RUNTIME_HOST_BACKEND" \
  "$bundle_path"
