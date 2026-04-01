# Deno / JS Runtime Plan

Living note. Revise it as we learn. Do not treat this as a fixed contract.

## Ground Rules

- Android is the current bootstrap host, not the target runtime.
- Aim at Linux-style payloads for the phone.
- Do not force `gnu` vs `musl` too early.
- Start below the full Deno CLI.

## Current Ladder

1. Package `rusty_v8` under Nix.
   First prove the build on normal Linux targets.
2. Build a tiny `rusty_v8` embedder.
   Eval JS, print result, no Deno yet.
3. Cross-build that embedder for `aarch64` Linux.
4. Pick the phone execution envelope.
   Most likely `gnu` userspace-on-device or a later `musl` retarget.
5. Build a tiny `deno_core` embedder.
6. Run JS on the real phone.
7. Add `deno_runtime` only after the lower seams are stable.
8. Feed Blitz documents from the runtime seam.

## Open Questions

- Can we rely on upstream `rusty_v8` archives for the first seams?
- When do we switch from `gnu` convenience to a more self-contained payload?
- Which minimum runtime features does Blitz actually need first?
