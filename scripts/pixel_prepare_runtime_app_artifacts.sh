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
input_path="${PIXEL_RUNTIME_APP_INPUT_PATH:-runtime/app-counter/app.tsx}"
cache_dir="${PIXEL_RUNTIME_APP_CACHE_DIR:-build/runtime/pixel-counter}"
bundle_artifact="$(pixel_runtime_app_bundle_artifact)"
host_bundle_dir="$(pixel_runtime_host_bundle_artifact_dir)"
host_bundle_out_link="$(pixel_dir)/shadow-runtime-host-aarch64-linux-gnu-result"
host_binary_name="shadow-runtime-host"
host_launcher_artifact="$host_bundle_dir/run-shadow-runtime-host"
package_ref="$repo#shadow-runtime-host-aarch64-linux-gnu"
bundle_json=""
bundle_source_path=""

bundle_json="$(
  nix develop "$repo"#runtime -c deno run --quiet \
    --allow-env --allow-read --allow-write --allow-run \
    "$repo/scripts/runtime_prepare_app_bundle.ts" \
    --input "$input_path" \
    --cache-dir "$cache_dir"
)"
printf '%s\n' "$bundle_json"

bundle_source_path="$(
  printf '%s\n' "$bundle_json" | python3 -c '
import json
import os
import sys

data = json.load(sys.stdin)
print(os.path.abspath(data["bundlePath"]))
'
)"

mkdir -p "$(dirname "$bundle_artifact")"
cp "$bundle_source_path" "$bundle_artifact"
chmod 0644 "$bundle_artifact"

stage_runtime_host_linux_bundle "$package_ref" "$host_bundle_out_link" "$host_bundle_dir" "$host_binary_name"

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

python3 - "$bundle_artifact" "$host_bundle_dir" "$input_path" <<'PY'
import json
import os
import sys

bundle_artifact, host_bundle_dir, input_path = sys.argv[1:4]
print(json.dumps({
    "inputPath": input_path,
    "runtimeAppBundleArtifact": os.path.abspath(bundle_artifact),
    "runtimeAppBundleDevicePath": "/data/local/tmp/shadow-runtime-gnu/runtime-app-bundle.js",
    "runtimeHostBundleArtifactDir": os.path.abspath(host_bundle_dir),
    "runtimeHostBinaryDevicePath": "/data/local/tmp/shadow-runtime-gnu/shadow-runtime-host",
    "runtimeHostLauncherDevicePath": "/data/local/tmp/shadow-runtime-gnu/run-shadow-runtime-host",
}, indent=2))
PY
