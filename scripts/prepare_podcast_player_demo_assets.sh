#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ASSET_DIR="${SHADOW_PODCAST_PLAYER_ASSET_DIR:-$REPO_ROOT/build/runtime/app-podcast-player-assets}"
PODCAST_DIR="$ASSET_DIR/assets/podcast"
PODCAST_FEED_URL="${SHADOW_PODCAST_PLAYER_FEED_URL:-https://sovereignengineering.io/dialogues.xml}"
EPISODE_IDS="${SHADOW_PODCAST_PLAYER_EPISODE_IDS:-00,01,02,03,04}"

mkdir -p "$PODCAST_DIR"

episode_json="$(
  PODCAST_FEED_URL="$PODCAST_FEED_URL" EPISODE_IDS="$EPISODE_IDS" python3 - <<'PY'
import json
import os
import re
import urllib.request
import xml.etree.ElementTree as ET

feed_url = os.environ["PODCAST_FEED_URL"]
episode_ids = {
    part.strip() for part in os.environ["EPISODE_IDS"].split(",") if part.strip()
}
xml = urllib.request.urlopen(feed_url).read()
root = ET.fromstring(xml)
channel = root.find("channel")
if channel is None:
    raise SystemExit("prepare_podcast_player_demo_assets: missing channel in feed")

license_node = channel.find("{https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/1.0.md}license")
podcast_title = (channel.findtext("title") or "").strip() or "No Solutions"
podcast_page_url = (channel.findtext("link") or "").strip() or "https://sovereignengineering.io/podcast"
episodes = []

def slugify(title: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")
    return slug or "episode"

def parse_duration_ms(raw: str) -> int:
    raw = raw.strip()
    if raw.isdigit():
      return int(raw) * 1000
    parts = [int(part) for part in raw.split(":")]
    total = 0
    for part in parts:
      total = total * 60 + part
    return total * 1000

for item in channel.findall("item"):
    title = (item.findtext("title") or "").strip()
    match = re.match(r"#(?P<id>\d{2}):\s*(?P<rest>.+)$", title)
    if not match:
        continue
    episode_id = match.group("id")
    if episode_id not in episode_ids:
        continue
    enclosure = item.find("enclosure")
    if enclosure is None or not enclosure.get("url"):
        raise SystemExit(f"prepare_podcast_player_demo_assets: missing enclosure for {title}")
    source_url = enclosure.get("url")
    source_ext = os.path.splitext(source_url)[1].lower()
    output_basename = f"{episode_id}-{slugify(match.group('rest'))}.mp3"
    duration_raw = item.findtext("{http://www.itunes.com/dtds/podcast-1.0.dtd}duration") or ""
    episodes.append({
        "durationMs": parse_duration_ms(duration_raw),
        "id": episode_id,
        "outputBasename": output_basename,
        "path": f"assets/podcast/{output_basename}",
        "sourceExt": source_ext,
        "sourceUrl": source_url,
        "title": title,
    })

episodes.sort(key=lambda episode: episode["id"])
missing = sorted(episode_ids - {episode["id"] for episode in episodes})
if missing:
    raise SystemExit(
        "prepare_podcast_player_demo_assets: missing episodes in feed: "
        + ", ".join(missing)
    )

print(json.dumps({
    "assetDir": os.path.abspath(os.environ.get("ASSET_DIR_OVERRIDE", "")) or None,
    "episodes": episodes,
    "podcastLicense": license_node.text.strip() if license_node is not None and license_node.text else None,
    "podcastPageUrl": podcast_page_url,
    "podcastTitle": podcast_title,
}, indent=2))
PY
)"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

while IFS=$'\t' read -r episode_id source_url source_ext output_basename; do
  output_path="$PODCAST_DIR/$output_basename"
  if [[ -f "$output_path" ]]; then
    continue
  fi

  source_path="$tmp_dir/$episode_id${source_ext:-}"
  curl -fsSL "$source_url" -o "$source_path"
  if [[ "$source_ext" == ".mp3" ]]; then
    mv "$source_path" "$output_path"
    chmod 0644 "$output_path"
    continue
  fi

  nix shell --accept-flake-config --inputs-from "$REPO_ROOT" nixpkgs#ffmpeg -c \
    ffmpeg -hide_banner -loglevel error -y \
    -i "$source_path" \
    -vn -c:a libmp3lame -b:a 128k \
    "$output_path"
  chmod 0644 "$output_path"
done < <(
  EPISODE_JSON="$episode_json" python3 - <<'PY'
import json
import os

data = json.loads(os.environ["EPISODE_JSON"])
for episode in data["episodes"]:
    print(
        "\t".join([
            episode["id"],
            episode["sourceUrl"],
            episode["sourceExt"],
            episode["outputBasename"],
        ])
    )
PY
)

EPISODE_JSON="$episode_json" ASSET_DIR="$ASSET_DIR" python3 - <<'PY'
import json
import os

data = json.loads(os.environ["EPISODE_JSON"])
data["assetDir"] = os.path.abspath(os.environ["ASSET_DIR"])
print(json.dumps(data, indent=2))
PY
