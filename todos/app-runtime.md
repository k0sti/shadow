# App Runtime Plan

Living plan. Revise it as we learn. Do not treat this as a fixed contract.

## Scope

- Finish the runtime app platform until the next serious app can reuse it.
- Keep the Nostr timeline app as the proving ground.
- Do not start the Bitcoin wallet tranche until viewport, input, lifecycle, and capability seams are solid enough.

## Current Position

- `counter` and `timeline` are real shell apps in the VM/home flow.
- `deno_core` remains the default runtime helper. `deno_runtime` is proven, but not promoted.
- Rooted Pixel runtime apps work. Rooted Pixel real-shell unification does not.
- `just runtime-app-host-smokes` is now the truthful host proof surface.
- The runtime viewport contract is now unified around the shell app viewport (`540x1106` today). Pixel fits that viewport into the real panel instead of using raw panel size as the app surface.
- Host proofs already exist for focus, keyboard input, selection metadata, relay sync, and restart/cache reload.
- Host wheel scroll already works through Blitz's native `UiEvent::Wheel` path; the runtime wrapper now suppresses drag / pan gestures from turning into synthetic runtime clicks.
- The VM shelve/reopen lane has a smoke recipe, but it needs recheck on the current machine because `just ui-vm-timeline-smoke` stalled in `shadowctl vm wait-ready` during this pass.

## Stable Bets

- TS / TSX app modules.
- Solid-style authoring.
- JS emits `{ html, css }` snapshots.
- Rust owns the outer frame and native integration.
- Events are routed by stable app-owned ids.
- App/runtime owns text mutation semantics.
- `deno_core` remains the pragmatic default until a real feature forces promotion.

## Approach

- Make the operator/docs/check surface truthful before widening the contract again.
- Attack one seam at a time: operator truth, viewport contract, scroll/input parity, Pixel lane decision, then capability shaping.
- Prefer proofs on real host/VM/Pixel lanes over stdio-only coverage when picking the next chunk.
- Keep app/runtime APIs pre-alpha: optimize for fast iteration and clean design, not backwards compatibility.

## Milestones

- [x] Make the runtime operator and doc surface truthful.
- [x] Unify runtime viewport sizing across shell, Blitz host window, compositor launch, VM, and Pixel.
- [~] Finish the remaining real-app input gap: host wheel / pan proof is in, but live VM/compositor recheck is still pending.
- [ ] Decide the near-term Pixel lane so device work stops splitting: either push the real shell on device, or intentionally focus on the runtime-app path for now.
- [ ] Keep the OS capability seam small, reusable, and easy to change while we iterate quickly toward a good app API.

## Near-Term Steps

- [x] Replace stale `just` and docs references to missing runtime host smoke scripts with the current consolidated host smokes.
- [x] Move the host window and Pixel runtime scripts off hardcoded `384x720` and raw panel sizing onto one shared viewport contract.
- [x] Prove the existing host wheel / pan path and stop drag gestures from collapsing into synthetic runtime clicks.
- [ ] Decide whether the next scroll/input seam is a live VM/compositor proof or a JS-facing wheel/scroll event shape for apps that need explicit handlers.
- [ ] Re-check the VM shelve/reopen lane after the viewport cleanup and decide whether it needs extra runtime-specific assertions.

## Implementation Notes

- 2026-04-07: `just runtime-app-keyboard-smoke` passed. That already covers focus, keyboard input, and selection metadata on the bundled host seam.
- 2026-04-07: `just runtime-app-nostr-timeline-smoke` passed. That proves relay sync, keyboard compose, restart behavior, and cache-backed timeline reload on the host runtime seam.
- 2026-04-07: The old split host commands (`runtime-app-document-smoke`, `runtime-app-click-smoke`, `runtime-app-input-smoke`, `runtime-app-focus-smoke`, `runtime-app-toggle-smoke`, `runtime-app-selection-smoke`, `runtime-app-host-smoke`, `runtime-app-compositor-smoke-gpu`) were removed from the live `just` surface because their scripts no longer exist.
- 2026-04-07: `just ui-vm-timeline-smoke` stalled in `shadowctl vm wait-ready`. Treat the VM lifecycle lane as recheck-needed, not green.
- 2026-04-07: The viewport contract is now unified around the shell app viewport from `shadow-ui-core` (`540x1106` today).
  - `shadow-blitz-demo` defaults to that viewport on the host.
  - `shadow-compositor` and `shadow-compositor-guest` both use the same viewport contract when no override is supplied.
  - Pixel runtime scripts fit that logical viewport into the real panel and pass the fitted size to both the guest compositor and the runtime client (`1080x2212` on a Pixel 4a panel).
- 2026-04-07: Host scroll did not need a new runtime JSON event shape for the current app lane.
  - Blitz already converts host wheel input into `UiEvent::Wheel` and scrolls overflow containers natively.
  - The real runtime-wrapper bug was that press-drag-release gestures could still synthesize a runtime click after a pan.
  - `RuntimeDocument` now cancels synthetic clicks after pointer movement crosses a small threshold, and unit coverage now proves host wheel scroll, finger-pan scroll, and tap-to-click separately.

## Current Runtime Contract

- JS -> Rust: `{ html, css? }`
- Rust -> JS events always include:
  - `type`
  - `targetId`
- Optional event payload:
  - `value`
  - `checked`
  - `selection`
  - `pointer`
  - `keyboard`
- Host wheel / pan scrolling still lives in the Blitz document layer; it is not yet forwarded through the JS runtime event schema.

## What Is Out Of Scope Right Now

- Adding more apps just to prove variety.
- Promoting `deno_runtime` by default without concrete feature pressure.
- Perfect browser compatibility.
- IME / composition correctness.
- Fine-grained Rust-side DOM patching.

## Related Plans

- `todos/gpu.md`: device rendering/perf work that still affects typing and interaction quality.
