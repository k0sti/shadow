#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WAIT_FOR="${CF_WAIT_FOR:-VIRTUAL_DEVICE_BOOT_COMPLETED|GUEST_BUILD_FINGERPRINT}"
TIMEOUT_SECS="${CF_WAIT_TIMEOUT:-180}"

exec "$SCRIPT_DIR/cf_launch.sh" \
  --wait-for "$WAIT_FOR" \
  --timeout "$TIMEOUT_SECS"
