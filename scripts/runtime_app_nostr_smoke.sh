#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=./runtime_host_backend_common.sh
source "$SCRIPT_DIR/runtime_host_backend_common.sh"
INPUT_PATH="runtime/app-nostr-smoke/app.tsx"
CACHE_DIR="build/runtime/app-nostr-smoke"
RESULT_EXPR='JSON.stringify(globalThis.SHADOW_RUNTIME_APP.dispatch({type:"click",targetId:"publish"}))'
EXPECTED_INITIAL_HTML='<main class="nostr-feed"><header class="feed-header"><h1>Shadow Nostr</h1><button class="publish" data-shadow-id="publish">Post Kind 1</button></header><p class="feed-status">Loaded 3 notes</p><ol class="feed-list"><li class="feed-item"><article class="feed-note"><p class="feed-meta">shadow-note-3:npub-feed-b</p><p class="feed-content">local cache warmed from the system service</p></article></li><li class="feed-item"><article class="feed-note"><p class="feed-meta">shadow-note-2:npub-feed-a</p><p class="feed-content">relay subscriptions will live below app code</p></article></li><li class="feed-item"><article class="feed-note"><p class="feed-meta">shadow-note-1:npub-feed-a</p><p class="feed-content">shadow os owns nostr for tiny apps</p></article></li></ol></main>'
EXPECTED_PUBLISHED_HTML='<main class="nostr-feed"><header class="feed-header"><h1>Shadow Nostr</h1><button class="publish" data-shadow-id="publish">Post Kind 1</button></header><p class="feed-status">Posted shadow-note-4</p><ol class="feed-list"><li class="feed-item"><article class="feed-note"><p class="feed-meta">shadow-note-4:npub-shadow-os</p><p class="feed-content">shadow says hello from the os</p></article></li><li class="feed-item"><article class="feed-note"><p class="feed-meta">shadow-note-3:npub-feed-b</p><p class="feed-content">local cache warmed from the system service</p></article></li><li class="feed-item"><article class="feed-note"><p class="feed-meta">shadow-note-2:npub-feed-a</p><p class="feed-content">relay subscriptions will live below app code</p></article></li><li class="feed-item"><article class="feed-note"><p class="feed-meta">shadow-note-1:npub-feed-a</p><p class="feed-content">shadow os owns nostr for tiny apps</p></article></li></ol></main>'

cd "$REPO_ROOT"
runtime_host_backend_resolve

bundle_json="$(
  deno run --quiet --allow-env --allow-read --allow-write --allow-run \
    scripts/runtime_prepare_app_bundle.ts \
    --input "$INPUT_PATH" \
    --cache-dir "$CACHE_DIR"
)"
printf '%s\n' "$bundle_json"

bundle_path="$(
  printf '%s\n' "$bundle_json" | python3 -c 'import json, sys; print(json.load(sys.stdin)["bundlePath"])'
)"

initial_output="$(
  nix run --accept-flake-config ".#${SHADOW_RUNTIME_HOST_PACKAGE_ATTR}" -- "$bundle_path"
)"
printf '%s\n' "$initial_output"

printf '%s\n' "$initial_output" | python3 -c '
import json
import re
import sys

expected_html = sys.argv[1]
payload = sys.stdin.read()
for line in reversed(payload.splitlines()):
    match = re.search(r"result=(\{.*\})$", line)
    if not match:
        continue
    document = json.loads(match.group(1))
    if document.get("html") != expected_html:
        raise SystemExit("unexpected initial html payload: %r" % (document.get("html"),))
    if document.get("css", None) is not None:
        raise SystemExit("expected initial css to be null, got: %r" % (document.get("css"),))
    break
else:
    raise SystemExit("could not find initial document payload in runtime host output")
' "$EXPECTED_INITIAL_HTML"

publish_output="$(
  nix run --accept-flake-config ".#${SHADOW_RUNTIME_HOST_PACKAGE_ATTR}" -- \
    "$bundle_path" \
    --result-expr "$RESULT_EXPR"
)"
printf '%s\n' "$publish_output"

document_json="$(
  printf '%s\n' "$publish_output" | python3 -c '
import json
import re
import sys

expected_html = sys.argv[1]
payload = sys.stdin.read()
for line in reversed(payload.splitlines()):
    match = re.search(r"result=(\{.*\})$", line)
    if not match:
        continue
    document = json.loads(match.group(1))
    if document.get("html") != expected_html:
        raise SystemExit("unexpected published html payload: %r" % (document.get("html"),))
    if document.get("css", None) is not None:
        raise SystemExit("expected published css to be null, got: %r" % (document.get("css"),))
    print(json.dumps(document, indent=2))
    break
else:
    raise SystemExit("could not find published document payload in runtime host output")
' "$EXPECTED_PUBLISHED_HTML"
)"
printf '%s\n' "$document_json"

printf 'Runtime app nostr smoke succeeded: backend=%s bundle=%s\n' "$SHADOW_RUNTIME_HOST_BACKEND" "$bundle_path"
