#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

nix develop --accept-flake-config .#ui -c cargo fmt --all --manifest-path ui/Cargo.toml --check
nix develop --accept-flake-config .#ui -c cargo check --workspace --all-targets --manifest-path ui/Cargo.toml
