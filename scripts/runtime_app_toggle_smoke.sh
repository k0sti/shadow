#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INPUT_PATH="runtime/app-toggle-smoke/app.tsx"
CACHE_DIR="build/runtime/app-toggle-smoke"
EXPECTED_ENABLED_HTML='<main class="compose"><label class="toggle"><input data-shadow-id="alerts" name="alerts" type="checkbox" checked><span>Alerts enabled</span></label><p class="status">Enabled: yes</p><p class="status">Last: change:alerts:alerts:checkbox:true</p></main>'
EXPECTED_DISABLED_HTML='<main class="compose"><label class="toggle"><input data-shadow-id="alerts" name="alerts" type="checkbox"><span>Alerts enabled</span></label><p class="status">Enabled: no</p><p class="status">Last: change:alerts:alerts:checkbox:false</p></main>'

cd "$REPO_ROOT"

session_json="$(
  SHADOW_RUNTIME_APP_INPUT_PATH="$INPUT_PATH" \
  SHADOW_RUNTIME_APP_CACHE_DIR="$CACHE_DIR" \
  scripts/runtime_prepare_host_session.sh
)"
printf '%s\n' "$session_json"

bundle_path="$(
  printf '%s\n' "$session_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["bundlePath"])'
)"
runtime_host_binary_path="$(
  printf '%s\n' "$session_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["runtimeHostBinaryPath"])'
)"
runtime_backend="$(
  printf '%s\n' "$session_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["runtimeHostBackend"])'
)"

python3 - "$runtime_host_binary_path" "$bundle_path" "$EXPECTED_ENABLED_HTML" "$EXPECTED_DISABLED_HTML" "$runtime_backend" <<'PY'
import json
import subprocess
import sys

binary_path, bundle_path, expected_enabled_html, expected_disabled_html, backend = sys.argv[1:6]

proc = subprocess.Popen(
    [binary_path, "--session", bundle_path],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=None,
    text=True,
)

assert proc.stdin is not None
assert proc.stdout is not None

def request(payload):
    proc.stdin.write(json.dumps(payload) + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    if not line:
        raise SystemExit("runtime session closed stdout")
    response = json.loads(line)
    if response.get("status") != "ok":
        raise SystemExit(f"runtime session returned error: {response}")
    return response["payload"]

try:
    enabled_payload = request({
        "op": "dispatch",
        "event": {"type": "change", "targetId": "alerts", "checked": True},
    })
    if enabled_payload.get("html") != expected_enabled_html:
        raise SystemExit(f"unexpected enabled payload: {enabled_payload.get('html')!r}")
    if enabled_payload.get("css", None) is not None:
        raise SystemExit(f"expected enabled css to be null, got: {enabled_payload.get('css')!r}")

    disabled_payload = request({
        "op": "dispatch",
        "event": {"type": "change", "targetId": "alerts", "checked": False},
    })
    if disabled_payload.get("html") != expected_disabled_html:
        raise SystemExit(f"unexpected disabled payload: {disabled_payload.get('html')!r}")
    if disabled_payload.get("css", None) is not None:
        raise SystemExit(f"expected disabled css to be null, got: {disabled_payload.get('css')!r}")

    print(json.dumps({
        "enabled": enabled_payload,
        "disabled": disabled_payload,
    }, indent=2))
    print(f"Runtime app toggle smoke succeeded: backend={backend} bundle={bundle_path}")
finally:
    proc.stdin.close()
    proc.stdout.close()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)
PY
