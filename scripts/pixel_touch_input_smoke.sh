#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./pixel_common.sh
source "$SCRIPT_DIR/pixel_common.sh"
ensure_bootimg_shell "$@"

serial="$(pixel_resolve_serial)"
run_dir="$(pixel_prepare_named_run_dir "$(pixel_touch_runs_dir)")"
descriptor_path="$run_dir/touchscreen-descriptor.txt"
capture_path="$run_dir/touch-events.txt"
root_id_path="$run_dir/root-id.txt"
status_path="$run_dir/status.json"
device_props_path="$run_dir/device-props.txt"
device_processes_before_path="$run_dir/processes-before.txt"
device_processes_after_path="$run_dir/processes-after.txt"
mode="${PIXEL_TOUCH_SMOKE_MODE:-inject}"
capture_timeout_secs="${PIXEL_TOUCH_SMOKE_DURATION_SECS:-6}"
device_log_path="/data/local/tmp/shadow-touch-smoke.log"
root_ok=false
device_detected=false
capture_ok=false
output_ok=false
failure_message=""

pixel_prepare_dirs
pixel_capture_props "$serial" "$device_props_path"
pixel_capture_processes "$serial" "$device_processes_before_path"

set +e
root_id="$(pixel_root_id "$serial")"
root_status="$?"
set -e
if [[ "$root_status" -ne 0 ]]; then
  echo "pixel-touch-input-smoke: root is required; run 'just pixel-root-check'" >&2
  exit 1
fi
root_ok=true
printf '%s\n' "$root_id" >"$root_id_path"

touch_device="$(pixel_touchscreen_event_device "$serial")"
device_detected=true

event_listing="$(pixel_adb "$serial" shell getevent -pl 2>/dev/null | tr -d '\r')"
touch_descriptor="$(
  printf '%s\n' "$event_listing" | awk -v target="$touch_device" '
    /^add device/ {
      if (capture) {
        exit
      }
      capture = ($4 == target)
    }
    capture {
      print
    }
  '
)"
if [[ -z "$touch_descriptor" ]]; then
  failure_message="failed to capture touchscreen descriptor for $touch_device"
else
  printf '%s\n' "$touch_descriptor" >"$descriptor_path"
fi

touch_name="$(
  printf '%s\n' "$touch_descriptor" \
    | sed -n 's/^  name:[[:space:]]*"\(.*\)"/\1/p' \
    | head -n 1
)"

max_x="$(
  printf '%s\n' "$touch_descriptor" | awk '
    /ABS_MT_POSITION_X/ {
      for (i = 1; i <= NF; i++) {
        if ($i == "max") {
          value = $(i + 1)
          gsub(/,/, "", value)
          print value
          exit
        }
      }
    }
  '
)"
max_y="$(
  printf '%s\n' "$touch_descriptor" | awk '
    /ABS_MT_POSITION_Y/ {
      for (i = 1; i <= NF; i++) {
        if ($i == "max") {
          value = $(i + 1)
          gsub(/,/, "", value)
          print value
          exit
        }
      }
    }
  '
)"

if [[ -z "${max_x:-}" || -z "${max_y:-}" ]]; then
  failure_message="${failure_message:-failed to parse touchscreen coordinate range from descriptor}"
fi

inject_x="${PIXEL_TOUCH_SMOKE_X:-}"
inject_y="${PIXEL_TOUCH_SMOKE_Y:-}"
if [[ -z "$inject_x" && -n "${max_x:-}" ]]; then
  inject_x="$((max_x / 2))"
fi
if [[ -z "$inject_y" && -n "${max_y:-}" ]]; then
  inject_y="$((max_y / 2))"
fi
tracking_id="${PIXEL_TOUCH_SMOKE_TRACKING_ID:-9001}"
pressure="${PIXEL_TOUCH_SMOKE_PRESSURE:-40}"
touch_major="${PIXEL_TOUCH_SMOKE_TOUCH_MAJOR:-5}"

case "$mode" in
  inject)
    ;;
  manual)
    printf 'Tap the Pixel touchscreen within %ss to satisfy the raw touch smoke.\n' "$capture_timeout_secs" >&2
    ;;
  *)
    echo "pixel-touch-input-smoke: unsupported PIXEL_TOUCH_SMOKE_MODE=$mode (expected inject or manual)" >&2
    exit 1
    ;;
esac

if [[ -z "$failure_message" ]]; then
  phone_script="$(
    cat <<EOF
rm -f '$device_log_path'
(timeout '$capture_timeout_secs' getevent -lt '$touch_device' >'$device_log_path' 2>&1) &
capture_pid=\$!
sleep 1
if [ '$mode' = inject ]; then
  sendevent '$touch_device' 3 47 0
  sendevent '$touch_device' 3 57 '$tracking_id'
  sendevent '$touch_device' 3 53 '$inject_x'
  sendevent '$touch_device' 3 54 '$inject_y'
  sendevent '$touch_device' 3 58 '$pressure'
  sendevent '$touch_device' 3 48 '$touch_major'
  sendevent '$touch_device' 1 330 1
  sendevent '$touch_device' 0 0 0
  sleep 0.1
  sendevent '$touch_device' 3 47 0
  sendevent '$touch_device' 3 57 -1
  sendevent '$touch_device' 1 330 0
  sendevent '$touch_device' 0 0 0
fi
wait "\$capture_pid" || true
cat '$device_log_path'
EOF
  )"

  if pixel_root_shell "$serial" "$phone_script" >"$capture_path"; then
    capture_ok=true
  else
    failure_message="touch capture command failed"
  fi
fi

if [[ "$capture_ok" == true ]]; then
  if grep -Fq 'ABS_MT_POSITION_X' "$capture_path" \
    && grep -Fq 'ABS_MT_POSITION_Y' "$capture_path" \
    && grep -Fq 'BTN_TOUCH' "$capture_path"; then
    output_ok=true
  else
    failure_message="touch capture did not contain the expected multitouch markers"
  fi
fi

pixel_capture_processes "$serial" "$device_processes_after_path"

pixel_write_status_json "$status_path" \
  run_dir="$run_dir" \
  serial="$serial" \
  mode="$mode" \
  root_ok="$root_ok" \
  device_detected="$device_detected" \
  capture_ok="$capture_ok" \
  output_ok="$output_ok" \
  touch_device="$touch_device" \
  touch_name="$touch_name" \
  touch_max_x="$max_x" \
  touch_max_y="$max_y" \
  inject_x="$inject_x" \
  inject_y="$inject_y" \
  capture_timeout_secs="$capture_timeout_secs" \
  failure_message="$failure_message" \
  success="$([[ "$root_ok" == true && "$device_detected" == true && "$capture_ok" == true && "$output_ok" == true ]] && echo true || echo false)"

cat "$status_path"

if [[ "$output_ok" != true ]]; then
  [[ -n "$failure_message" ]] && echo "pixel-touch-input-smoke: $failure_message" >&2
  echo "pixel-touch-input-smoke: raw touch seam failed; see $run_dir" >&2
  exit 1
fi

printf 'Pixel touch input smoke succeeded: %s\n' "$run_dir"
