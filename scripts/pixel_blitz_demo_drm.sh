#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./pixel_common.sh
source "$SCRIPT_DIR/pixel_common.sh"
ensure_bootimg_shell "$@"

if [[ ! -f "$(pixel_blitz_demo_artifact)" ]]; then
  "$SCRIPT_DIR/pixel_build_blitz_demo.sh"
fi

PIXEL_GUEST_CLIENT_ARTIFACT="$(pixel_blitz_demo_artifact)" \
PIXEL_GUEST_CLIENT_DST="$(pixel_blitz_demo_dst)" \
PIXEL_COMPOSITOR_MARKER='[shadow-guest-compositor] presented-frame' \
PIXEL_CLIENT_MARKER='[shadow-blitz-demo] static-document-ready' \
  "$SCRIPT_DIR/pixel_guest_ui_drm.sh"
