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

**Landed in two PRs, not one** (scoping found during implementation, not before): PR A ships
the live state viewers for all 4 panels (pure read-only plumbing, no core changes beyond small
new accessors). PR B adds a minimal 65C816 disassembler + PC breakpoints + step/step-over/
step-into (frontend-only, using the existing `System::step_instruction()`). Read/write
watchpoints need a new `debug-hooks` feature on `rustysnes-core` itself + a `Bus`-level hook —
deferred to a separate follow-up ticket, T-81-001b, since it touches the hottest path in the
engine and deserves its own focused review.

**Acceptance criteria:**

- [x] 65C816 panel: register/flag view (PR A). Breakpoints (PC), step/step-over/step-into: PR B.
      Read/write watchpoints: T-81-001b (not this ticket).
- [x] PPU panel: VRAM (scrollable window) / CGRAM viewer, current scanline/dot, register state
      (PR A). OAM viewer not yet landed — small follow-up, same shape as the VRAM/CGRAM viewers.
- [x] APU panel: SPC700 PC/halt state, DSP voice state (PR A).
- [x] Cart panel: active board type + coprocessor register state (SA-1 second-CPU state via
      `System::sa1_regs`, Super FX/GSU state via `Board::debug_gsu_state`) (PR A).
- [x] With `debug-hooks` off, the build is byte-identical — the Debug menu entry itself is
      feature-gated, so `debugger_open` can never become `true` and the app never builds a
      snapshot (PR A; verified `cargo check`/`clippy`/`fmt` clean in both configs, full
      `--features test-roms` suite passes unchanged).

**Dependencies:** T-51-001 (the shell itself, already landed)
**Reference:** `docs/frontend.md` §Debugger overlay; `crates/rustysnes-frontend/src/ui_shell.rs`,
`debug_snapshot.rs`
**Estimated complexity:** L

---

### T-81-002 — Lua scripting + TAS movie API

**Description:** implement `rustysnes-script`'s full stated scope in one pass — Lua scripting
(a memory-read/write + frame-callback API) and TAS movie record/playback (a deterministic input
log format + save-state-at-frame-0 seeding, replay-verified bit-identical) — behind the existing
`scripting` flag. Both build on the existing `Bus::set_joypad`/save-state envelope; no new
architectural work needed.

**Acceptance criteria:**

- [x] Lua scripts can read/write emulated memory and hook a per-frame callback — `ScriptEngine`
      (`rustysnes-script`, `mlua` 0.12 vendored Lua 5.4, sandboxed), `emu.read`/`emu.write`
      (WRAM, via `Lua::scope`-bound closures) and `emu.onFrame(fn)`; verified by 5 unit tests
      including a runaway-loop instruction-budget interruption and a sandbox-escape rejection
      test (`io.open`/`os.execute`/`require('os')` all fail).
- [x] TAS movies record a deterministic input log; replaying a recorded movie against the same
      ROM + save-state-at-frame-0 produces a bit-identical framebuffer/audio trace — a new
      `rustysnes_core::movie` module (`Movie`/`MovieRecorder`/`MoviePlayer`), verified by
      `movie_determinism.rs`: 40 frames of varying synthetic input recorded against the
      committed `cputest-basic.sfc` ROM, round-tripped through the real on-disk byte format,
      replayed against a fresh `System`, framebuffer + audio hashes byte-identical frame-for-frame.
- [x] With `scripting` off, the build is byte-identical (CI gate) — `rustysnes-script` is an
      optional native-only dependency (`dep:rustysnes-script`), compiled out entirely with the
      flag off; full default-feature workspace build/test/clippy/fmt verified unaffected.

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

- [x] Game Genie code parsing + decode to a RAM-address/value patch — `rustysnes_core::cheat::
      decode_game_genie`, ported from bsnes's `CheatEditor::decodeSNES` and cross-checked
      bit-for-bit against Mesen2's independent decoder; unit-tested against real commercial
      codes from Mesen2's shipped `CheatDb.Snes.json`.
- [x] Pro Action Replay code parsing + decode — `rustysnes_core::cheat::decode_pro_action_replay`
      (straight 8-hex-digit `AAAAAADD`, no scrambling), same external-oracle test vectors.
- [x] Patches apply every frame without breaking the determinism contract when the feature is
      off (a cheat is host-applied external input, not a hardware behavior — model it that way) —
      `crate::cheats::apply_all` pokes enabled entries into WRAM via `Bus::poke_wram` before each
      frame runs; nothing executes unless `cheats` is on and at least one entry is enabled.
- [x] With `cheats` off, the build is byte-identical (CI gate) — the frontend's cheat list/UI
      module is `#[cfg(feature = "cheats")]`-gated entirely (the decode module itself stays
      unconditional in `rustysnes-core`, same precedent as the `movie` module); full
      default-feature workspace build/test/clippy/fmt/doc verified unaffected.

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
`wasm-canvas` flag, porting RustyNES's proven shape (`../RustyNES/crates/rustynes-frontend/src/
wasm.rs`, not inventing a new approach): a `CanvasRenderingContext2d.putImageData` blit of the
existing RGBA8 framebuffer (`emu.rs::framebuffer()` already produces this — no PPU/core changes
needed), a `requestAnimationFrame` loop, keyboard input via DOM `keydown`/`keyup` events, ROM
loading via `<input type="file">`. No `wgpu`/`egui` — this stage proves a real, visible,
playable demo exists fast, without needing `app.rs`/`audio.rs` un-gated for wasm32 yet. Ships to
the live Pages deployment as soon as it lands, closing the actual user-facing gap (a blank demo
page) even before stage 2 (T-81-006) is ready.

**Acceptance criteria:**

- [x] The live demo shows a real picture and accepts keyboard input for a loaded ROM.
- [x] Audio plays via `AudioWorklet` (primary) with a `ScriptProcessorNode` fallback — no
      `SharedArrayBuffer` (GitHub Pages can't send COOP/COEP headers). The DRC/resampler core
      (`Resampler`/`AudioRing`/`drc_ratio`) was extracted out of `audio.rs` into a new
      target-agnostic `audio_core.rs` specifically so `wasm_audio.rs` reuses the SAME logic
      native does, not a reimplementation — `audio_core::Resampler::process_into` is the one new
      addition (appends to a `Vec<f32>` for the worklet's `postMessage` boundary instead of an
      `AudioRing`; a `process_and_process_into_agree` test proves the two paths stay identical).
- [x] Verified with a real headless-browser load (Playwright/Chromium, actually run — not just
      described): loaded a real committed test ROM
      (`tests/roms/undisbeliever/inidisp_brightness_delay.sfc`) through the live `#rom-input`,
      confirmed a `<canvas>` element exists, and read back its pixel data — 28672/57344 pixels
      non-black after one ROM load, zero console errors, `"RustySNES wasm-canvas: ready"` +
      `"RustySNES: ROM loaded"` logged. This caught a real, separate, pre-existing bug (see
      below) that a build/HTTP-status check alone would have missed entirely, exactly the gap
      this criterion exists to close.
- [x] `pages.yml` updated: two changes, both load-bearing, not cosmetic.
      1. `web/index.html`'s trunk directive was `data-bin="rustysnes" data-type="main"`, which
         built the `[[bin]]` (`main.rs`, whose wasm32 arm is an empty `fn main() {}` that never
         references the lib) instead of the `[lib]` cdylib — the actual
         `#[wasm_bindgen(start)]` entry point in `wasm.rs` got dead-code-eliminated entirely
         since nothing in the binary's reachable call graph touched it. The built `.wasm` was
         only ~14 KB (confirmed by direct inspection) with zero emulator code linked in — this,
         not just "`wasm.rs` is a stub," was the actual root cause of the blank demo page since
         `v0.1.0`; every prior release's stub would have produced the same empty artifact even
         with real code in it. Fixed to `data-target-name="rustysnes_frontend"`, the same
         pattern RustyNES's own working `index.html` uses (confirmed by reading its source).
      2. `pages.yml`'s `RUSTFLAGS="-C target-feature=-reference-types"` broke wasm-bindgen's
         externref table generation (`failed to find the __wbindgen_externref_table_dealloc
         function`) once the demo actually linked in real `Closure`-based code — it had been a
         silent no-op given bug 1 above (there was no real code to break). Removed.

**Honest gap not claimed as covered:** audio was verified to construct without throwing (the
`AudioContext`/`AudioWorkletNode`/`ScriptProcessorNode` graph builds cleanly, no console errors),
but headless Chromium automation cannot conclusively prove audible output through the browser's
real autoplay-gesture semantics — genuine manual verification in a real browser is still owed as
a follow-up, same honesty posture as `v0.7.0`'s hi-res real-title-validation gap.

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

- [x] The same `App`/egui UI (menu bar, Settings, debugger panels — T-81-001 landed first)
      renders in the browser via wgpu, not just the stage-1 canvas blit. `app.rs` is ONE
      `ApplicationHandler<AppEvent>` impl shared by native and `wasm32`, with internal
      `#[cfg(target_arch = "wasm32")]` branches for the genuinely-async `Gfx::new_async` init
      (delivered back via a new `AppEvent::GfxReady` through an `EventLoopProxy` — native drives
      the same core synchronously via `pollster::block_on`), browser ROM loading
      (`AppEvent::RomLoaded`, replacing `rfd`'s native file dialog), and audio (`wasm_audio`
      instead of `cpal`/`AudioOutput`) — not two parallel copies of the shell.
- [x] `app.rs`/`gfx.rs` compile and run for both native and `wasm32` targets from the same
      source, matching RustyNES's own "`ApplicationHandler` impl serves both native and wasm32"
      pattern. `audio.rs` itself stays native-only (the cpal device glue has no wasm32
      equivalent to share — its console-agnostic core was already extracted to `audio_core.rs`
      in T-81-005 and is what `wasm_audio.rs` actually reuses).
- [x] Verified with a real headless-browser check (Playwright/Chromium, actually run): the
      WebGL2 fallback path (`wgpu::Backends::GL`, chosen when `navigator.gpu` probes absent)
      renders correctly end-to-end — a full-page screenshot of the live demo (after loading
      `tests/roms/undisbeliever/inidisp_brightness_delay.sfc`) shows the real egui menu bar, the
      status bar reading `LoROM | Ntsc | 60.0 fps | ROM loaded`, and the actual emulated
      framebuffer, not a blank canvas. (`getImageData`-based pixel counting — the method
      T-81-005 used — reads back empty on a WebGL/WebGPU canvas whose drawing buffer isn't
      preserved across presents; `page.screenshot()`, which reads the browser's own compositor
      output, is the reliable method for these canvas types and is what actually proved this.)
      **Honest gap:** the WebGPU path itself (`wgpu::Backends::BROWSER_WEBGPU`) could not be
      screenshotted in this sandbox — headless Chromium here exposes `navigator.gpu` but returns
      "no compatible wgpu adapter" for a real request, and several software-Vulkan launch-flag
      combinations didn't produce a working adapter. The WebGPU path shares the same
      `Gfx::new_async` core the verified GL path uses and its backend-selection/color-space
      handling is grounded in real prior hardware testing (documented in `gfx.rs`'s own doc
      comments), but a live screenshot specifically on WebGPU is not claimed here — real-browser
      verification (a machine with actual WebGPU support) is still owed as a follow-up.
- [x] With `wasm-canvas` (not `wasm-winit`) selected instead
      (`--no-default-features --features wasm-canvas`), the stage-1 MVP still builds and works —
      re-verified end-to-end after this change (real trunk build, real headless-browser load,
      same non-zero-pixel-data check T-81-005 used) with no regression. The two paths stay
      independently functional, matching the manifest's existing "exactly one is compiled"
      comment; `wasm-winit` is now the default (`Cargo.toml`), since a real `wasm_winit.rs`
      module exists for it to select.

**Dependencies:** T-81-005 (ships the interim working demo first); `Board: Send` is NOT required
here (that's `v1.0.0`'s native `emu-thread` prerequisite, a separate, unrelated flag)
**Reference:** `../RustyNES/crates/rustynes-frontend/src/wasm_winit.rs` (254 lines, the port
source); `../RustyNES/crates/rustynes-frontend/src/app.rs`'s `ApplicationHandler` doc comment
**Estimated complexity:** L

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason). T-81-001 (PR A landed, PR B +
      T-81-001b deferred), T-81-002, T-81-003, T-81-005, T-81-006 done; T-81-004 remains.
- [ ] Every instrumentation feature is off by default + byte-identical when off — verified
      individually for `debug-hooks` (T-81-001), `scripting` (T-81-002), and `cheats` (T-81-003);
      the formal CI gate covering all of them together is T-81-004, not yet landed.
- [x] The live wasm demo actually renders and is playable, verified by a real headless-browser
      check, not just an HTTP status check — the default `wasm-winit` build (T-81-006), via a
      full-page screenshot showing the egui shell + a real emulated framebuffer, not just pixel
      counts.
- [ ] CHANGELOG.md updated.
