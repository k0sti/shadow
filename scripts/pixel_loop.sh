#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./pixel_common.sh
source "$SCRIPT_DIR/pixel_common.sh"
ensure_bootimg_shell "$@"

max_attempts="${PIXEL_LOOP_MAX_ATTEMPTS:-0}"
retry_secs="${PIXEL_LOOP_RETRY_SECS:-15}"
rebuild_each_attempt="${PIXEL_LOOP_REBUILD_EACH_ATTEMPT:-0}"
attempt=1

if [[ "$rebuild_each_attempt" != "1" ]]; then
  "$SCRIPT_DIR/pixel_build.sh"
fi

while true; do
  printf 'pixel-loop: attempt %s\n' "$attempt"

  if [[ "$rebuild_each_attempt" == "1" ]]; then
    "$SCRIPT_DIR/pixel_build.sh"
  fi

  if "$SCRIPT_DIR/pixel_run.sh"; then
    printf 'pixel-loop: success on attempt %s\n' "$attempt"
    exit 0
  fi

  if [[ "$max_attempts" -gt 0 && "$attempt" -ge "$max_attempts" ]]; then
    printf 'pixel-loop: hit max attempts (%s)\n' "$max_attempts" >&2
    exit 1
  fi

  attempt="$((attempt + 1))"
  sleep "$retry_secs"
done
