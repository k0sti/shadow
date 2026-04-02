# Nostr Plan

Living note. Revise it as the OS/runtime boundary gets clearer.

## Goal

- OS-owned Nostr capability.
- Tiny apps call small system APIs.
- One client/cache/signer below apps, not duplicated per app.

## Current Bet

- App-facing JS API lives at `@shadow/app-runtime-os`.
- First slice is host-only and mocked.
- The app uses `listKind1` and `publishKind1`.
- The Nostr implementation is still a fake in-memory system service inside the runtime bootstrap.

## First Ladder

- [x] Host-only OS API seam.
  `just runtime-app-nostr-smoke` proves a runtime app can load kind 1 notes from a system API and publish a new kind 1 note without owning Nostr logic itself. `just runtime-app-nostr-smoke-deno-runtime` proves the same seam on the alternate backend.
- [ ] Move the fake system Nostr service below JS.
  Keep the same app-facing API, but back it from the runtime helper / Rust side instead of bootstrap JS.
- [ ] Add sqlite-backed local cache and feed queries.
- [ ] Add real relay fetch for kind 1 events.
- [ ] Add OS-owned signer boundary for publishing.
- [ ] Prove the same API on the rooted Pixel runtime app lane.
