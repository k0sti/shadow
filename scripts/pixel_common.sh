#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./cf_common.sh
source "$SCRIPT_DIR/cf_common.sh"

pixel_dir() {
  printf '%s/pixel\n' "$(build_dir)"
}

pixel_artifacts_dir() {
  printf '%s/artifacts\n' "$(pixel_dir)"
}

pixel_runs_dir() {
  printf '%s/runs\n' "$(pixel_dir)"
}

pixel_latest_run_link() {
  printf '%s/latest-run\n' "$(pixel_dir)"
}

pixel_timestamp() {
  date -u +%Y%m%dT%H%M%SZ
}

pixel_connected_serials() {
  adb devices | awk 'NR > 1 && $2 == "device" { print $1 }'
}

pixel_resolve_serial() {
  local requested
  requested="${PIXEL_SERIAL:-}"

  if [[ -n "$requested" ]]; then
    if pixel_connected_serials | grep -Fxq "$requested"; then
      printf '%s\n' "$requested"
      return 0
    fi
    echo "pixel: requested PIXEL_SERIAL is not connected and authorized: $requested" >&2
    return 1
  fi

  mapfile -t serials < <(pixel_connected_serials)
  case "${#serials[@]}" in
    0)
      echo "pixel: no authorized adb device detected" >&2
      return 1
      ;;
    1)
      printf '%s\n' "${serials[0]}"
      ;;
    *)
      echo "pixel: multiple adb devices detected; set PIXEL_SERIAL" >&2
      printf '  %s\n' "${serials[@]}" >&2
      return 1
      ;;
  esac
}

pixel_adb() {
  local serial
  serial="$1"
  shift
  adb -s "$serial" "$@"
}

pixel_prop() {
  local serial key
  serial="$1"
  key="$2"
  pixel_adb "$serial" shell getprop "$key" | tr -d '\r'
}

pixel_prepare_dirs() {
  mkdir -p "$(pixel_artifacts_dir)" "$(pixel_runs_dir)"
}

pixel_prepare_run_dir() {
  local run_dir
  pixel_prepare_dirs
  run_dir="$(pixel_runs_dir)/$(pixel_timestamp)"
  mkdir -p "$run_dir"
  ln -sfn "$run_dir" "$(pixel_latest_run_link)"
  printf '%s\n' "$run_dir"
}

pixel_latest_run_dir() {
  local link
  link="$(pixel_latest_run_link)"
  if [[ -L "$link" ]]; then
    readlink "$link"
  fi
}

pixel_selected_run_dir() {
  if [[ -n "${PIXEL_RUN_DIR:-}" ]]; then
    printf '%s\n' "$PIXEL_RUN_DIR"
    return 0
  fi
  pixel_latest_run_dir
}

pixel_artifact_path() {
  printf '%s/%s\n' "$(pixel_artifacts_dir)" "$1"
}

pixel_session_artifact() {
  pixel_artifact_path shadow-session
}

pixel_compositor_artifact() {
  pixel_artifact_path shadow-compositor-guest
}

pixel_counter_artifact() {
  pixel_artifact_path shadow-counter-guest
}

pixel_session_dst() {
  printf '%s\n' "${PIXEL_SESSION_DST:-/data/local/tmp/shadow-session}"
}

pixel_compositor_dst() {
  printf '%s\n' "${PIXEL_COMPOSITOR_DST:-/data/local/tmp/shadow-compositor-guest}"
}

pixel_counter_dst() {
  printf '%s\n' "${PIXEL_COUNTER_DST:-/data/local/tmp/shadow-counter-guest}"
}

pixel_runtime_dir() {
  printf '%s\n' "${PIXEL_RUNTIME_DIR:-/data/local/tmp/shadow-runtime}"
}

pixel_frame_path() {
  printf '%s\n' "${PIXEL_FRAME_PATH:-/data/local/tmp/shadow-frame.ppm}"
}

pixel_expected_checksum() {
  printf '%s\n' "${PIXEL_EXPECTED_CHECKSUM:-dd64a1693b87ade5}"
}

pixel_expected_size() {
  printf '%s\n' "${PIXEL_EXPECTED_SIZE:-220x120}"
}

pixel_compositor_marker() {
  printf '[shadow-guest-compositor] captured-frame checksum=%s size=%s\n' \
    "$(pixel_expected_checksum)" \
    "$(pixel_expected_size)"
}

pixel_client_marker() {
  printf '[shadow-guest-counter] frame-committed checksum=%s size=%s\n' \
    "$(pixel_expected_checksum)" \
    "$(pixel_expected_size)"
}

pixel_require_runtime_artifacts() {
  local path missing
  missing=0
  for path in \
    "$(pixel_session_artifact)" \
    "$(pixel_compositor_artifact)" \
    "$(pixel_counter_artifact)"; do
    if [[ ! -f "$path" ]]; then
      echo "pixel: missing built artifact: $path" >&2
      missing=1
    fi
  done
  return "$missing"
}

pixel_capture_props() {
  local serial output
  serial="$1"
  output="$2"
  pixel_adb "$serial" shell getprop >"$output"
}

pixel_capture_processes() {
  local serial output
  serial="$1"
  output="$2"
  pixel_adb "$serial" shell 'ps -A -o USER,PID,PPID,NAME,ARGS 2>/dev/null | grep -E "shadow|wayland" || true' >"$output"
}

pixel_write_status_json() {
  local output
  output="$1"
  shift
  python3 - "$output" "$@" <<'PY'
import json
import sys

output = sys.argv[1]
data = {}
for item in sys.argv[2:]:
    key, value = item.split("=", 1)
    if value == "true":
        data[key] = True
    elif value == "false":
        data[key] = False
    else:
        try:
            data[key] = int(value)
        except ValueError:
            data[key] = value

with open(output, "w", encoding="utf-8") as fh:
    json.dump(data, fh, indent=2, sort_keys=True)
    fh.write("\n")
PY
}
