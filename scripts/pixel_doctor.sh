#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./pixel_common.sh
source "$SCRIPT_DIR/pixel_common.sh"
ensure_bootimg_shell "$@"

serial="$(pixel_resolve_serial)"
model="$(pixel_prop "$serial" ro.product.model)"
device="$(pixel_prop "$serial" ro.product.device)"
fingerprint="$(pixel_prop "$serial" ro.build.fingerprint)"
cpu_abi="$(pixel_prop "$serial" ro.product.cpu.abi)"
boot_completed="$(pixel_prop "$serial" sys.boot_completed)"
slot_suffix="$(pixel_prop "$serial" ro.boot.slot_suffix)"
flash_locked="$(pixel_prop "$serial" ro.boot.flash.locked)"
verified_boot_state="$(pixel_prop "$serial" ro.boot.verifiedbootstate)"
debuggable="$(pixel_prop "$serial" ro.debuggable)"
shell_id="$(pixel_adb "$serial" shell id | tr -d '\r')"

set +e
su_id="$(pixel_adb "$serial" shell 'su 0 sh -c id' 2>/dev/null | tr -d '\r')"
su_status="$?"
set -e

cat <<EOF
serial: $serial
model: $model
device: $device
fingerprint: $fingerprint
cpu_abi: $cpu_abi
boot_completed: ${boot_completed:-0}
slot_suffix: ${slot_suffix:-unknown}
flash_locked: ${flash_locked:-unknown}
verified_boot_state: ${verified_boot_state:-unknown}
debuggable: ${debuggable:-unknown}
shell_user: $shell_id
su_available: $([[ "$su_status" -eq 0 ]] && echo yes || echo no)
post_boot_offscreen_loop: ready
display_takeover_loop: $([[ "$su_status" -eq 0 ]] && echo maybe || echo blocked_without_root)
boot_image_loop: $([[ "${flash_locked:-1}" == "0" ]] && echo maybe || echo blocked_without_bootloader_unlock)
EOF

if [[ "$su_status" -eq 0 ]]; then
  printf 'su_user: %s\n' "$su_id"
fi
