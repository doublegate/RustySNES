# Phase 8 — Instrumentation + Community (additive, off-by-default)

## Goal

Add the breadth features RustyNES shipped in its own v1.0.0 rather than deferring —
a debugger overlay, Lua scripting + a TAS movie/replay API, cheat-code support, rollback
netplay, and RetroAchievements — each **behind a default-off feature flag** and each **proven
byte-identical with the feature off**, so the shipped / native / `no_std` / wasm builds stay
byte-identical and the accuracy number is unaffected. **This phase gates `v1.0.0`** (reversed
from an earlier post-1.0 framing — see "Second reversal" in `to-dos/VERSION-PLAN.md`'s intro):
RustyNES front-loaded this exact breadth into its own v1.0.0, and matching that bar means Phase
8 lands before the production cut, not after it. A shader/filter ecosystem and a Libretro core
stay genuinely post-`v1.0.0` Reach — polish items with no accuracy-adjacency, matching how
RustyNES itself treated them.

**Folded in after `v0.7.0`, per explicit direction:** the real wasm frontend build.
`crates/rustysnes-frontend/src/wasm.rs` has been a scaffold stub since `v0.1.0` — it never
builds the actual app, so the live Pages demo renders blank despite every prior "wasm demo is
live" claim being true only at the HTTP level. Found live by direct comparison against
RustyNES's own working wasm deployment, not by CI. Ported from RustyNES's proven two-stage
shape (`wasm-canvas` MVP, then `wasm-winit` unification) — see Sprint 1 below.

## Exit criteria

- [ ] Debugger overlay (65C816/PPU/APU/Cart panels, including SA-1/Super FX coprocessor state
      when active) filling in `ui_shell.rs`'s already-wired `"TODO(impl-phase)"` panels.
- [ ] Lua scripting / TAS API (`rustysnes-script`'s full stated scope: scripting + movie
      record/playback together, per its own `docs/STATUS.md` description).
- [ ] Cheat-code support (Game Genie / Pro Action Replay SNES format), a new `cheats` flag.
- [ ] Rollback netplay (frontend-orchestrated against the deterministic core, native + browser),
      preceded by a `System::save_state()`/`load_state()` cost benchmark to confirm the existing
      full-snapshot design is fast enough for a real rollback window.
- [ ] RetroAchievements (opt-in, native FFI, the `RustySNES/<ver> rcheevos/<ver>` User-Agent
      pattern).
- [ ] The real wasm frontend: a `wasm-canvas` MVP (canvas-2D blit, `requestAnimationFrame`,
      keyboard, ROM load) landed first for a fast working demo, then `wasm-winit` unification
      (the same `App` native uses, via `EventLoopExtWebSys::spawn_app`) — both ported from
      RustyNES's proven shape, verified by a real headless-browser render check, not just an
      HTTP status check.
- [ ] Every feature off by default; with all off, builds are byte-identical (a CI gate proves
      it, re-verified after each sprint below).
- [ ] All sprints complete; `v1.0.0` cut prerequisites met (`to-dos/VERSION-PLAN.md`'s v1.0.0
      gate).

## Scope

In-scope:

- The five breadth feature families above, each default-off and byte-identical-when-off.
- The real wasm frontend build (two stages — `wasm-canvas` MVP, then `wasm-winit`
  unification), which requires un-gating `app.rs`/`audio.rs` from their current
  `#[cfg(not(target_arch = "wasm32"))]` exclusion — a real, confirmed architectural gap, not
  just plumbing (see Sprint 1's T-81-005/T-81-006 for the full breakdown).
- The `Board: Send` fix `emu-thread` needs (a prerequisite for `v1.0.0`'s dedicated emulation
  thread, tracked here since it's discovered/fixed alongside this phase's instrumentation work).
  Unrelated to the wasm-winit unification above — `emu-thread` is a native-only, single-player
  dedicated-thread feature; wasm's own event loop is a separate concern.

Out-of-scope (post-`v1.0.0` Reach, `to-dos/VERSION-PLAN.md`):

- A composable shader/filter ecosystem (CRT/HQ2x) and a Libretro core.
- HD texture packs (the `hd-pack` flag exists in the manifest already, but RustyNES itself
  doesn't have this feature, so it sits outside the parity target).
- The fractional-timebase refactor (`docs/adr/0002`) — strictly beyond `v1.0.0`.

## Sprints

- [Sprint 1 — Instrumentation](sprint-1-instrumentation.md) — debugger, scripting/TAS, cheats,
  the real wasm frontend (two stages). Maps to `v0.8.0 "Instrumentation"`.
- [Sprint 2 — Community](sprint-2-community.md) — rollback netplay, RetroAchievements. Maps to
  `v0.9.0 "Community"`.

The desktop UX shell maturity pass (thumbnail save-state manager, themes, input rebinding, the
Performance panel, wiring `emu-thread`) and the production cut itself are tracked directly under
`v1.0.0` in `to-dos/VERSION-PLAN.md`, not as a Phase 8 sprint — that work is UX/release polish,
not a reach *feature*.

## Dependencies

Phase 5 (the frontend + the exercised determinism contract); Phases 1–4 feature-complete; Phase
6 accuracy at target.

## Risks

- **Byte-identity drift** — a default-off feature accidentally changing the shipped path. Detect:
  the byte-identical CI gate. Mitigate: gate every feature, run the gate per feature combo (the
  RustyNES `--all-features` trap — use explicit combos, never `--all-features`).
- **Netplay determinism** — any hidden non-determinism breaks rollback. Mitigate: the
  determinism contract (`docs/adr/0004`) is the precondition.
- **Netplay save-state cost, unmeasured** — `RewindBuffer` was designed for ~10 Hz capture;
  rollback netplay calls save/restore far more often, and nothing currently benchmarks
  `System::save_state()`/`load_state()` cost. If full-snapshot cost is too high for a real
  rollback window, delta/incremental snapshots become necessary — a real design change beyond
  `docs/adr/0006`'s "future memory optimization, not correctness requirement" framing (a call
  made for rewind's occasional-capture case, not netplay's every-frame one). Benchmark before
  committing to the existing design; write a new ADR if it triggers a redesign.
- **`emu-thread` vs. netplay control-model conflict** — `emu_thread.rs`'s pacing model is
  single-player-only by its own doc comment; netplay's rollback drive loop is a different,
  frontend-orchestrated resimulation model. Mitigate by keeping the two mutually exclusive by
  session type (a netplay session uses its own rollback-aware loop, never the generic
  `emu-thread` pacer) rather than trying to unify them.
- **The wasm frontend shipping unverified again** — the root cause of the original stub going
  unnoticed since `v0.1.0` was that every prior check (`pages.yml`, this project's own CHANGELOG
  claims) only asserted the build/deploy pipeline succeeds, never that the resulting page
  actually renders. Mitigate by requiring a real headless-browser render check (Playwright/
  Chromium: assert a canvas exists and receives non-zero pixel data) as an explicit acceptance
  criterion for both T-81-005 and T-81-006, not just a build-success check.

## Reference docs

- [docs/frontend.md](../../docs/frontend.md) — the determinism boundary the reach features rest
  on.
- [docs/adr/0004](../../docs/adr/0004-determinism-contract.md) — the rollback/replay precondition.
- [docs/adr/0006](../../docs/adr/0006-save-state-format.md) — the save-state envelope netplay's
  rollback and TAS's replay both build on.
- [docs/STATUS.md](../../docs/STATUS.md) — version policy + the v1.0.0 cut record.
- [to-dos/VERSION-PLAN.md](../VERSION-PLAN.md) — the full `v0.7.0`→`v1.0.0` ladder this phase's
  sprints map onto.
