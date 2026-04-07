#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"
session_json="$("$SCRIPT_DIR/runtime_prepare_host_session.sh")"

SESSION_JSON="$session_json" python3 - <<'PY'
import json
import os
import subprocess
import time

session = json.loads(os.environ["SESSION_JSON"])
bundle_path = session["bundlePath"]
binary_path = session["runtimeHostBinaryPath"]

process = subprocess.Popen(
    [binary_path, "--session", bundle_path],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
)


def send_raw(request):
    assert process.stdin is not None
    process.stdin.write(json.dumps(request) + "\n")
    process.stdin.flush()
    assert process.stdout is not None
    line = process.stdout.readline()
    if not line:
        stderr = process.stderr.read() if process.stderr is not None else ""
        raise SystemExit(
            f"runtime-app-sound-smoke: runtime host closed stdout\n{stderr}",
        )
    return json.loads(line)


def send_ok(request):
    response = send_raw(request)
    if response.get("status") != "ok":
        raise SystemExit(
            f"runtime-app-sound-smoke: unexpected response: {json.dumps(response)}",
        )
    return response["payload"]


def wait_for_fragment(fragment, timeout_seconds=5):
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        response = send_raw({"op": "render_if_dirty"})
        if response.get("status") == "no_update":
            time.sleep(0.05)
            continue
        if response.get("status") != "ok":
            raise SystemExit(
                f"runtime-app-sound-smoke: unexpected response: {json.dumps(response)}",
            )
        payload = response["payload"]
        html = payload["html"]
        if fragment in html:
            return html
        time.sleep(0.05)
    raise SystemExit(
        f"runtime-app-sound-smoke: timed out waiting for fragment {fragment!r}",
    )


def dispatch_and_wait(target_id, fragment):
    payload = send_ok({"op": "dispatch", "event": {"targetId": target_id, "type": "click"}})
    html = payload["html"]
    if fragment in html:
        return html
    return wait_for_fragment(fragment)


initial = send_ok({"op": "render"})
prepared_html = dispatch_and_wait("prepare", "State:</span> idle")
playing_html = dispatch_and_wait("play", "State:</span> playing")
stopped_html = dispatch_and_wait("stop", "State:</span> stopped")
released_html = dispatch_and_wait("release", "State:</span> released")

assert process.stdin is not None
process.stdin.close()
stderr = process.stderr.read() if process.stderr is not None else ""
return_code = process.wait(timeout=10)
if return_code not in (0, None):
    raise SystemExit(
        f"runtime-app-sound-smoke: runtime host exited {return_code}\n{stderr}",
    )

initial_html = initial["html"]

if "Linux audio seam" not in initial_html:
    raise SystemExit("runtime-app-sound-smoke: initial render missing headline")
if "State:</span> missing" not in initial_html:
    raise SystemExit("runtime-app-sound-smoke: initial render missing unprepared state")
if "State:</span> idle" not in prepared_html:
    raise SystemExit("runtime-app-sound-smoke: prepare click did not surface idle state")
if "Backend:</span> memory" not in prepared_html:
    raise SystemExit("runtime-app-sound-smoke: host smoke should use memory backend")
if "State:</span> playing" not in playing_html:
    raise SystemExit("runtime-app-sound-smoke: play click did not surface playing state")
if "State:</span> stopped" not in stopped_html:
    raise SystemExit("runtime-app-sound-smoke: stop click did not surface stopped state")
if "State:</span> released" not in released_html:
    raise SystemExit("runtime-app-sound-smoke: release click did not surface released state")

print(json.dumps({
    "runtimeHostPackageAttr": session["runtimeHostPackageAttr"],
    "runtimeHostBinaryName": session["runtimeHostBinaryName"],
    "bundlePath": bundle_path,
    "result": "sound-audio-api-ok",
}, indent=2))
PY
