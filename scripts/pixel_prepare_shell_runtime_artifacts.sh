#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./pixel_common.sh
source "$SCRIPT_DIR/pixel_common.sh"
# shellcheck source=./pixel_runtime_linux_bundle_common.sh
source "$SCRIPT_DIR/pixel_runtime_linux_bundle_common.sh"
ensure_bootimg_shell "$@"

pixel_prepare_dirs
repo="$(repo_root)"
host_bundle_dir="$(pixel_shell_runtime_host_bundle_artifact_dir)"
host_bundle_out_link="$(pixel_dir)/shadow-runtime-shell-host-aarch64-linux-gnu-result"
host_binary_name="shadow-runtime-host"
host_launcher_artifact="$host_bundle_dir/run-shadow-runtime-host"
host_bundle_manifest_path="$host_bundle_dir/.bundle-manifest.json"
runtime_manifest_path="$host_bundle_dir/.runtime-bundle-manifest.json"
package_ref="$repo#shadow-runtime-host-aarch64-linux-gnu"
extra_bundle_dir="${PIXEL_RUNTIME_EXTRA_BUNDLE_ARTIFACT_DIR-}"

counter_input_path="${PIXEL_SHELL_COUNTER_INPUT_PATH:-runtime/app-counter/app.tsx}"
counter_cache_dir="${PIXEL_SHELL_COUNTER_CACHE_DIR:-build/runtime/pixel-shell-counter}"
counter_bundle_artifact="$(pixel_runtime_counter_bundle_artifact)"

timeline_input_path="${PIXEL_SHELL_TIMELINE_INPUT_PATH:-runtime/app-nostr-timeline/app.tsx}"
timeline_cache_dir="${PIXEL_SHELL_TIMELINE_CACHE_DIR:-build/runtime/pixel-shell-timeline}"
timeline_bundle_artifact="$(pixel_runtime_timeline_bundle_artifact)"

timeline_config_json="${SHADOW_RUNTIME_APP_TIMELINE_CONFIG_JSON-}"
if [[ -z "$timeline_config_json" ]]; then
  timeline_config_json='{"limit":12,"syncOnStart":false}'
fi

prepare_bundle() {
  local input_path cache_dir
  input_path="$1"
  cache_dir="$2"
  shift 2

  nix develop "$repo"#runtime -c env "$@" deno run --quiet \
    --allow-env --allow-read --allow-write --allow-run \
    "$repo/scripts/runtime_prepare_app_bundle.ts" \
    --input "$input_path" \
    --cache-dir "$cache_dir"
}

bundle_source_path_from_json() {
  python3 -c '
import json
import os
import sys

data = json.load(sys.stdin)
print(os.path.abspath(data["bundlePath"]))
'
}

if [[ -n "$extra_bundle_dir" ]]; then
  extra_bundle_dir="$(normalize_runtime_bundle_input_path "$extra_bundle_dir")"
  if [[ ! -d "$extra_bundle_dir" ]]; then
    echo "pixel_prepare_shell_runtime_artifacts: extra bundle dir not found: $extra_bundle_dir" >&2
    exit 1
  fi
fi

counter_bundle_json="$(
  prepare_bundle \
    "$counter_input_path" \
    "$counter_cache_dir"
)"
timeline_bundle_json="$(
  prepare_bundle \
    "$timeline_input_path" \
    "$timeline_cache_dir" \
    SHADOW_RUNTIME_APP_CONFIG_JSON="$timeline_config_json"
)"

counter_bundle_source_path="$(printf '%s\n' "$counter_bundle_json" | bundle_source_path_from_json)"
timeline_bundle_source_path="$(printf '%s\n' "$timeline_bundle_json" | bundle_source_path_from_json)"

mkdir -p "$(dirname "$counter_bundle_artifact")"
cp "$counter_bundle_source_path" "$counter_bundle_artifact"
cp "$timeline_bundle_source_path" "$timeline_bundle_artifact"
chmod 0644 "$counter_bundle_artifact" "$timeline_bundle_artifact"

host_bundle_source_fingerprint="$(
  runtime_bundle_source_fingerprint \
    "pixel-shell-runtime $package_ref" \
    "$repo/flake.nix" \
    "$repo/flake.lock" \
    "$repo/rust/shadow-runtime-host" \
    "$repo/rust/shadow-runtime-host/Cargo.lock" \
    "$repo/rust/runtime-nostr-host" \
    "$repo/rust/vendor/temporal_rs" \
    "$SCRIPT_DIR/pixel_prepare_shell_runtime_artifacts.sh" \
    "$SCRIPT_DIR/pixel_runtime_linux_bundle_common.sh" \
    "$counter_bundle_source_path" \
    "$timeline_bundle_source_path" \
    "${extra_bundle_dir:-__no_extra_bundle__}"
)"

host_bundle_cache_hit=0
if [[ "${PIXEL_FORCE_LINUX_BUNDLE_REBUILD-}" != 1 ]] \
  && [[ -d "$host_bundle_dir" ]] \
  && [[ -x "$host_launcher_artifact" ]] \
  && [[ -f "$host_bundle_dir/$host_binary_name" ]] \
  && [[ -f "$host_bundle_dir/$(basename "$(pixel_runtime_counter_bundle_dst)")" ]] \
  && [[ -f "$host_bundle_dir/$(basename "$(pixel_runtime_timeline_bundle_dst)")" ]] \
  && runtime_bundle_manifest_matches "$host_bundle_manifest_path" "$host_bundle_source_fingerprint"; then
  host_bundle_cache_hit=1
  printf 'Shell runtime host bundle cacheHit -> %s\n' "$host_bundle_dir"
else
  stage_runtime_host_linux_bundle "$package_ref" "$host_bundle_out_link" "$host_bundle_dir" "$host_binary_name"
  fill_linux_bundle_runtime_deps "$host_bundle_dir"

  if [[ -n "$extra_bundle_dir" ]]; then
    chmod -R u+w "$host_bundle_dir" 2>/dev/null || true
    cp -R "$extra_bundle_dir"/. "$host_bundle_dir"/
  fi

  cat >"$host_launcher_artifact" <<EOF
#!/system/bin/sh
DIR=\$(cd "\$(dirname "\$0")" && pwd)
if [ "\$#" -ne 2 ] || [ "\$1" != "--session" ]; then
  echo "usage: $host_binary_name --session <bundle-path>" >&2
  exit 64
fi
if command -v chroot >/dev/null 2>&1; then
  case "\$2" in
    "\$DIR"/*) set -- "\$1" "/\${2#\$DIR/}" ;;
  esac
  exec chroot "\$DIR" "/lib/$PIXEL_RUNTIME_STAGE_LOADER_NAME" --library-path /lib "/$host_binary_name" "\$@"
fi
exec "\$DIR/lib/$PIXEL_RUNTIME_STAGE_LOADER_NAME" --library-path "\$DIR/lib" "\$DIR/$host_binary_name" "\$@"
EOF
  chmod 0755 "$host_launcher_artifact"

  write_runtime_bundle_manifest \
    "$host_bundle_manifest_path" \
    "$host_bundle_source_fingerprint" \
    "$package_ref"
fi

cp "$counter_bundle_artifact" "$host_bundle_dir/$(basename "$(pixel_runtime_counter_bundle_dst)")"
cp "$timeline_bundle_artifact" "$host_bundle_dir/$(basename "$(pixel_runtime_timeline_bundle_dst)")"
chmod 0644 \
  "$host_bundle_dir/$(basename "$(pixel_runtime_counter_bundle_dst)")" \
  "$host_bundle_dir/$(basename "$(pixel_runtime_timeline_bundle_dst)")"

runtime_helper_content_fingerprint="$(
  runtime_bundle_directory_fingerprint "$host_bundle_dir"
)"
python3 - "$runtime_manifest_path" "$runtime_helper_content_fingerprint" "$timeline_config_json" "$counter_input_path" "$timeline_input_path" "$extra_bundle_dir" <<'PY'
import json
import os
import sys
from datetime import datetime, timezone

manifest_path, content_fingerprint, timeline_config_json, counter_input_path, timeline_input_path, extra_bundle_dir = sys.argv[1:7]
manifest = {
    "contentFingerprint": content_fingerprint,
    "counterInputPath": counter_input_path,
    "generatedAt": datetime.now(timezone.utc).isoformat(),
    "mode": "pixel-shell-runtime",
    "runtimeExtraBundleArtifactDir": os.path.abspath(extra_bundle_dir) if extra_bundle_dir else None,
    "timelineConfigJson": timeline_config_json,
    "timelineInputPath": timeline_input_path,
}
with open(manifest_path, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2)
    handle.write("\n")
PY

python3 - "$host_bundle_dir" "$counter_bundle_artifact" "$timeline_bundle_artifact" "$host_bundle_cache_hit" <<'PY'
import json
import os
import sys

host_bundle_dir, counter_bundle_artifact, timeline_bundle_artifact, host_bundle_cache_hit = sys.argv[1:5]
print(json.dumps({
    "counterBundleArtifact": os.path.abspath(counter_bundle_artifact),
    "counterBundleDevicePath": "/data/local/tmp/shadow-runtime-gnu/runtime-app-counter-bundle.js",
    "mode": "pixel-shell-runtime",
    "runtimeHostBundleArtifactDir": os.path.abspath(host_bundle_dir),
    "runtimeHostBundleCacheHit": host_bundle_cache_hit == "1",
    "runtimeHostLauncherDevicePath": "/data/local/tmp/shadow-runtime-gnu/run-shadow-runtime-host",
    "timelineBundleArtifact": os.path.abspath(timeline_bundle_artifact),
    "timelineBundleDevicePath": "/data/local/tmp/shadow-runtime-gnu/runtime-app-timeline-bundle.js",
}, indent=2))
PY
