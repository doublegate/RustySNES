# Sprint 1 — Instrumentation

**Phase:** Phase 8 — Instrumentation + Community
**Sprint goal:** the debugger overlay, Lua scripting + TAS movie API, and cheat-code support
ship behind default-off feature flags, each byte-identical with the feature off; the wasm
frontend goes from a scaffold stub to a genuinely working demo.
**Estimated duration:** 3 weeks (native tooling) + 2-3 weeks (wasm frontend, two stages)
**Release mapping:** `v0.8.0 "Instrumentation"` (`to-dos/VERSION-PLAN.md`)

## Tickets

### T-81-001 — Debugger overlay (65C816/PPU/APU/Cart panels)

**Description:** fill in `ui_shell.rs`'s already-wired debugger window (currently
`"TODO(impl-phase)"` placeholders in each of its 4 panel selectors) with real breakpoint and
memory-viewer functionality, behind the existing `debug-hooks` flag. Include SA-1/Super FX
coprocessor state in the Cart panel from day one — resolving `docs/frontend.md`'s open question
in the breadth-inclusive direction this whole ladder takes, not deferring it further.

**Acceptance criteria:**

- [ ] 65C816 panel: register/flag view, breakpoints (PC + read/write watchpoints), step/
      step-over/step-into.
- [ ] PPU panel: VRAM/CGRAM/OAM viewer, current scanline/dot, register state.
- [ ] APU panel: SPC700 registers, DSP voice state.
- [ ] Cart panel: active board type + coprocessor register state (SA-1 second-CPU state and
      Super FX/GSU state included when the loaded cart uses either).
- [ ] With `debug-hooks` off, the build is byte-identical (CI gate).

**Dependencies:** T-51-001 (the shell itself, already landed)
**Reference:** `docs/frontend.md` §open questions; `crates/rustysnes-frontend/src/ui_shell.rs`
**Estimated complexity:** L

---

### T-81-002 — Lua scripting + TAS movie API

**Description:** implement `rustysnes-script`'s full stated scope in one pass — Lua scripting
(a memory-read/write + frame-callback API) and TAS movie record/playback (a deterministic input
log format + save-state-at-frame-0 seeding, replay-verified bit-identical) — behind the existing
`scripting` flag. Both build on the existing `Bus::set_joypad`/save-state envelope; no new
architectural work needed.

**Acceptance criteria:**

- [ ] Lua scripts can read/write emulated memory and hook a per-frame callback.
- [ ] TAS movies record a deterministic input log; replaying a recorded movie against the same
      ROM + save-state-at-frame-0 produces a bit-identical framebuffer/audio trace.
- [ ] With `scripting` off, the build is byte-identical (CI gate).

**Dependencies:** T-31-004 (determinism exercised), `v0.2.0`'s save-state envelope
**Reference:** `docs/architecture.md` (determinism-contract fact citing "TAS replay" as a
designed-for use case); `docs/STATUS.md`'s `rustysnes-script` subsystem entry
**Estimated complexity:** L

---

### T-81-003 — Cheat-code support (Game Genie / Pro Action Replay SNES format)

**Description:** implement SNES Game Genie and Pro Action Replay cheat-code parsing + a
per-frame memory-patch application, behind a new `cheats` flag (no existing scaffold — the first
new flag added on this ladder, matching the existing naming convention). Grouped in this sprint
rather than with netplay/RetroAchievements: cheats are memory-watch/poke tooling, the same
substrate as the debugger's memory panel.

**Acceptance criteria:**

- [ ] Game Genie code parsing + decode to a RAM-address/value patch.
- [ ] Pro Action Replay code parsing + decode.
- [ ] Patches apply every frame without breaking the determinism contract when the feature is
      off (a cheat is host-applied external input, not a hardware behavior — model it that way).
- [ ] With `cheats` off, the build is byte-identical (CI gate).

**Dependencies:** none beyond the base memory-access surface
**Reference:** RustyNES's cheat-code feature (parity target); `docs/adr/0004`
**Estimated complexity:** M

---

### T-81-004 — The byte-identical CI gate (feature-off), extended

**Description:** extend the existing byte-identical-with-flags-off CI gate to cover the three
new flags this sprint adds (`debug-hooks`, `scripting`, `cheats`); run clippy per explicit
feature combo (never `--all-features`).

**Acceptance criteria:**

- [ ] The byte-identical gate passes with all three flags off.
- [ ] clippy runs each new feature combo explicitly.
- [ ] The gate is wired into the standard CI run, ready to extend again in Sprint 2.

**Dependencies:** T-81-001; T-81-002; T-81-003
**Reference:** `docs/testing-strategy.md`; `docs/STATUS.md` §version-policy
**Estimated complexity:** S

---

### T-81-005 — The real wasm frontend, stage 1: `wasm-canvas` MVP

**Description:** replace `crates/rustysnes-frontend/src/wasm.rs`'s scaffold stub (panic hook +
one log line, never renders anything) with a working canvas-2D MVP behind the existing
`wasm-canvas` flag, porting RustyNES's proven shape
(`../RustyNES/crates/rustynes-frontend/src/wasm.rs`, not inventing a new approach): a
`CanvasRenderingContext2d.putImageData` blit of the existing RGBA8 framebuffer
(`emu.rs::framebuffer()` already produces this — no PPU/core changes needed), a
`requestAnimationFrame` loop, keyboard input via DOM `keydown`/`keyup` events, ROM loading via
`<input type="file">`, and audio via `AudioWorklet`/`ScriptProcessorNode` (ported from RustyNES's
`wasm_audio.rs`, reusing the native DRC/resampler core — see the acceptance criteria below). No
`wgpu`/`egui` — this stage proves a real, visible, playable demo exists fast, without needing
`app.rs`/`audio.rs` un-gated for wasm32 yet. Ships to the live Pages deployment as soon as it
lands, closing the actual user-facing gap (a blank demo page) even before stage 2 (T-81-006) is
ready.

**Acceptance criteria:**

- [ ] The live demo shows a real picture and accepts keyboard input for a loaded ROM.
- [ ] Audio plays via `AudioWorklet` (primary) with a `ScriptProcessorNode` fallback — no
      `SharedArrayBuffer` (GitHub Pages can't send COOP/COEP headers), reusing the same DRC/
      resampler logic `audio.rs` already has for native.
- [ ] Verified with a real headless-browser load (e.g. Playwright/Chromium), not just an HTTP
      200 check — assert a canvas element exists and receives non-zero pixel data after loading
      a ROM. `pages.yml`'s existing verification only checks the build/deploy steps succeed;
      this is the gap that let the stub ship unnoticed since `v0.1.0`.
- [ ] `pages.yml` updated if the trunk build target/feature selection needs to change.

**Dependencies:** none beyond what already exists (`emu.rs::framebuffer()`, existing keyboard/
ROM-load native code as the porting reference)
**Reference:** `../RustyNES/crates/rustynes-frontend/src/wasm.rs` (538 lines, the port source);
`docs/frontend.md` §wasm
**Estimated complexity:** L

---

### T-81-006 — The real wasm frontend, stage 2: `wasm-winit` unification

**Description:** route the wasm build through the *same* `App`/`ApplicationHandler` native
already uses, replacing the stage-1 canvas-2D path as the default (`wasm-winit` is already the
default feature in `Cargo.toml`), via `winit::platform::web::EventLoopExtWebSys::spawn_app` +
an `EventLoopProxy` delivering `RomLoaded`/`GfxReady`-style events in from JS — the exact shape
RustyNES's `wasm_winit.rs` (254 lines) already proves works, ported not invented. Requires
un-gating `app.rs` and `audio.rs` from their current `#[cfg(not(target_arch = "wasm32"))]`
exclusion and adapting them for wasm32 (swap `cpal` for Web Audio behind a conditional path,
gate out native-only deps — `gilrs`, `directories`, direct `std::fs`). This is the real work;
stage 1 (T-81-005) deliberately avoids needing it so a working demo ships sooner.

**Acceptance criteria:**

- [ ] The same `App`/egui UI (menu bar, Settings, debugger panels once T-81-001 lands) renders
      in the browser via wgpu, not just the stage-1 canvas blit.
- [ ] `app.rs`/`audio.rs` compile and run for both native and `wasm32` targets from the same
      source, matching RustyNES's own "`ApplicationHandler` impl serves both native and wasm32"
      pattern.
- [ ] Verified with the same real headless-browser check as T-81-005, re-run against the
      winit/wgpu path.
- [ ] With `wasm-canvas` (not `wasm-winit`) selected instead, the stage-1 MVP still builds and
      works — the two paths stay independently functional, matching the manifest's existing
      "exactly one is compiled" comment.

**Dependencies:** T-81-005 (ships the interim working demo first); `Board: Send` is NOT required
here (that's `v1.0.0`'s native `emu-thread` prerequisite, a separate, unrelated flag)
**Reference:** `../RustyNES/crates/rustynes-frontend/src/wasm_winit.rs` (254 lines, the port
source); `../RustyNES/crates/rustynes-frontend/src/app.rs`'s `ApplicationHandler` doc comment
**Estimated complexity:** L

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] Every instrumentation feature is off by default + byte-identical when off.
- [ ] The live wasm demo actually renders and is playable, verified by a real headless-browser
      check, not just an HTTP status check.
- [ ] CHANGELOG.md updated.
