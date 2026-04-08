#!/usr/bin/env bash
set -euo pipefail

font_dir="${1:?usage: desktop_prepare_fonts.sh <font-dir>}"
mkdir -p "$font_dir"

roboto_store="$(
  nix build --accept-flake-config --no-link --print-out-paths 'nixpkgs#roboto' | tail -n 1
)"
roboto_mono_store="$(
  nix build --accept-flake-config --no-link --print-out-paths 'nixpkgs#roboto-mono' | tail -n 1
)"
emoji_store="$(
  nix build --accept-flake-config --no-link --print-out-paths 'nixpkgs#noto-fonts-color-emoji' | tail -n 1
)"

cp -L "$roboto_store/share/fonts/truetype/Roboto-Regular.ttf" "$font_dir/DroidSans.ttf"
cp -L "$roboto_store/share/fonts/truetype/Roboto-Bold.ttf" "$font_dir/DroidSans-Bold.ttf"
cp -L "$roboto_mono_store/share/fonts/truetype/RobotoMono-Regular.ttf" "$font_dir/DroidSansMono.ttf"
cp -L "$emoji_store/share/fonts/noto/NotoColorEmoji.ttf" "$font_dir/NotoColorEmoji.ttf"
