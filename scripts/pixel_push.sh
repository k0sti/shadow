#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./pixel_common.sh
source "$SCRIPT_DIR/pixel_common.sh"
ensure_bootimg_shell "$@"

serial="$(pixel_resolve_serial)"

if ! pixel_require_runtime_artifacts; then
  "$SCRIPT_DIR/pixel_build.sh"
fi

printf 'Pushing device artifacts to %s\n' "$serial"
pixel_adb "$serial" push "$(pixel_session_artifact)" "$(pixel_session_dst)" >/dev/null
pixel_adb "$serial" push "$(pixel_compositor_artifact)" "$(pixel_compositor_dst)" >/dev/null
pixel_adb "$serial" push "$(pixel_counter_artifact)" "$(pixel_counter_dst)" >/dev/null
pixel_adb "$serial" shell chmod 0755 \
  "$(pixel_session_dst)" \
  "$(pixel_compositor_dst)" \
  "$(pixel_counter_dst)"

printf 'Pushed %s\n' "$(pixel_session_dst)"
printf 'Pushed %s\n' "$(pixel_compositor_dst)"
printf 'Pushed %s\n' "$(pixel_counter_dst)"
