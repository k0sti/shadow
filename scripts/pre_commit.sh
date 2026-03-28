#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./cf_common.sh
source "$SCRIPT_DIR/cf_common.sh"
ensure_bootimg_shell "$@"

cd "$(repo_root)"

bash -n scripts/*.sh
SHADOW_UI_VM_SOURCE="$PWD" nix flake check --no-build --impure --accept-flake-config
just ui-check
just artifacts-fetch
just init-boot-repack
scripts/assert_repacked_identity.sh
