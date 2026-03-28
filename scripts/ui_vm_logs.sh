#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

exec "$SCRIPT_DIR/ui_vm_ssh.sh" "bash -lc 'echo ==shadow-ui-session==; tail -n 200 /var/lib/shadow-ui/log/shadow-ui-session.log 2>/dev/null || true; echo; echo ==weston==; tail -n 200 /var/lib/shadow-ui/log/weston.log 2>/dev/null || true'"
