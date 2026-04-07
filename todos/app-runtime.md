# App Runtime Plan

Living plan. Keep it short. Delete completed tracks instead of turning this into history.

## Current Position

- The Nostr timeline app is the proving ground for the app model.
- `counter` and `timeline` are real shell apps in the VM/home flow.
- `deno_core` stays the default runtime helper.
- `deno_runtime` is proven, but not promoted.
- Rooted Pixel runtime apps work; rooted Pixel real-shell unification does not.

## Goal

- Make the runtime app environment good enough that the next serious app can reuse it.
- Keep evolving the Nostr app until the shell/runtime/capability story feels solid.
- Do not start the Bitcoin wallet tranche until this platform work is good enough.

## Stable Bets

- TS / TSX app modules.
- Solid-style authoring.
- JS emits `{ html, css }` snapshots.
- Rust owns the outer frame and native integration.
- Events are routed by stable app-owned ids.
- App/runtime owns text mutation semantics.
- `deno_core` remains the pragmatic default until a real feature forces promotion.

## Active Work

- [ ] Unify runtime app sizing around one source of truth across VM, shell viewport, Blitz surface, and Pixel.
- [ ] Finish generic input support for real apps:
  - pointer
  - wheel / axis scroll
  - focus
  - keyboard
- [ ] Decide the Pixel shell story.
  Either make the real shell work on device or explicitly defer it and keep the current runtime-app path as the product lane.
- [ ] Unify VM and Pixel operator/debug commands where it improves iteration without hiding environment differences.
- [ ] Make app lifecycle semantics explicit and tested:
  - cold launch
  - warm shelve/reopen
  - restart
  - state persistence expectations
- [ ] Keep the runtime OS capability boundary small and reusable so the Nostr app and later wallet app can share it cleanly.

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

## What Is Out Of Scope Right Now

- Adding more apps just to prove variety.
- Promoting `deno_runtime` by default without concrete feature pressure.
- Perfect browser compatibility.
- IME / composition correctness.
- Fine-grained Rust-side DOM patching.

## Related Plans

- The Nostr timeline app remains the main app-level proving ground for this plan.
- `todos/gpu.md`: device rendering/perf work that still affects typing and interaction quality.
