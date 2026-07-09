# Frontend — RustySNES

**References:** `docs/architecture.md` §6; `ref-docs/research-report.md` "External
dependencies"; `docs/adr/0004` (the determinism boundary).

## Purpose

`rustysnes-frontend` is the desktop + wasm shell: **winit + wgpu + cpal + egui**, pure Rust
and permissive (mirrors RustyNES). It is an **always-on egui shell, not a bare window** —
egui runs every frame.

**Status (Phase 5): playable native.** A real commercial ROM boots in a window with picture
(PPU BGR555 → RGBA8, aspect-correct 4:3 sub-rect letterbox blit), sound (S-DSP 32 kHz FIFO →
producer-side DRC-paced linear resampler → lock-free ring → cpal stereo), and control (keyboard +
gilrs gamepad → `Bus::set_joypad`). ROM load auto-resolves coprocessor firmware + `.srm` SRAM;
Reset / Power-Cycle / Pause are wired. The dependency stack tracks the latest mutually-compatible
tier: egui/egui-wgpu/egui-winit **0.35**, wgpu **29**, winit **0.30** (winit 0.31 is beta-only and
egui-winit 0.35 pins to 0.30 — winit is the crate gating us off 0.31). Native + `wasm32` both
build; the `playable_smoke` test is the headless AV proof.

## The shell model (the load-bearing rule)

- egui draws a **persistent menu bar** (File / Emulation / Tools / View / Debug / Help) +
  **status bar** + **tabbed Settings**, with toggleable CPU/PPU/APU/memory **debugger panels**
  layered on top.
- **Never hold the emu lock inside the egui closure.** Menu interactions return a `MenuAction`
  that the app dispatches *after* the egui pass; the hidden render branch copies the
  framebuffer under a brief lock, drops it, then renders / presents.
- On native, the emulator runs on a **dedicated thread** (communicating via an
  `Arc<Mutex<EmuCore>>` handle + a lock-free `SharedInput`); the winit thread only does UI +
  present.

## The determinism boundary

Rate control (the dynamic-rate-control resampler) and run-ahead (snapshot/restore
orchestration) live **here, in the frontend, never in the core synthesis** — that is what
keeps the core's bit-identical contract intact (`docs/adr/0004`, `docs/architecture.md` §5).
Netplay rollback is likewise frontend-orchestrated against the deterministic core.

## Audio + pacing

- A **lock-free audio ring** fed by the core's 32 kHz stereo output, drained by cpal, with
  dynamic rate control to absorb pacing jitter.
- A display-sync pacing matrix targeting 60.0988 Hz (NTSC) / 50.0070 Hz (PAL).
- The optional non-deterministic "hardware-accurate audio" SPC-drift toggle (`docs/apu.md`
  §determinism-caveat) is a frontend setting, off by default, outside the deterministic path.

### Fixed-timestep wall-clock pacing (synchronous drive)

winit's `RedrawRequested` fires once per **display** vsync, so stepping exactly one emulated
frame per redraw runs the emulator at the *monitor's* refresh — e.g. 2.4× too fast on a 144 Hz
panel. The synchronous (default, non-`emu-thread`) path therefore drives emulation from a
**wall-clock fixed-timestep accumulator** (`app::Pacer`): each present accumulates the real
elapsed time and runs `run_frame` only once `1 / region.frame_rate()` seconds have accrued,
presenting the latest framebuffer in between. Catch-up after a stall is capped
(`MAX_CATCHUP_FRAMES`, with the leftover backlog dropped) to avoid a spiral of death, and the
delta is clamped. The **present mode then governs only vsync/tearing, never emulation speed.**
The pacer's math is unit-tested (`pacing_tracks_region_rate_not_present_rate`) to hold ~60 fps
across 30/60/75/144/240 Hz present rates.

### FPS meter

`Pacer` doubles as the FPS meter: it counts emulated frames produced per wall-second over a
0.5 s window and exposes the smoothed value as `ShellInfo::fps`, which the status bar renders.
(In the `emu-thread` build the meter counts presents instead, since frames are produced off the
winit thread.)

### Present-mode application

The Settings → Video present-mode radio writes `config.video.present_mode`; the present path
detects a change against the last-applied mode and calls `Gfx::set_present_mode`, which
re-validates the request against the surface's supported modes (falling back to `Fifo`) and
**reconfigures the live wgpu surface**. Previously the surface was only ever configured once at
startup, so the toggle had no effect.

## Input

- USB gamepads auto-bind to P1; keyboard fallback for P1/P2.
- Late-latched input (sampled as close to the frame as possible) for responsiveness without
  breaking determinism.
- SNES peripherals (multitap / mouse / Super Scope) are frontend-side feeds into the core's
  controller ports; niche ones are stubbed initially (`ref-docs/research-report.md` "Scope").

## Save-states, rewind, run-ahead

- **Save-states** (`v0.2.0 "Persistence"`, `docs/adr/0006`) serialize the deterministic core
  state (including the SPC relative-time accumulator and the seeded power-on phase) into one
  versioned envelope via `System::save_state`/`load_state`. `EmuCore::save_state`/`load_state`
  (`emu.rs`) wrap it for the frontend, additionally re-rendering the framebuffer and clearing the
  audio FIFO on load (a state load jumps time discontinuously). Emulation → Save State / Load
  State drives a single quick-save slot held in `Active::quick_save`.
- **Rewind** (`v0.3.0 "Continuum"`, `crate::rewind::RewindBuffer`) is a bounded ring buffer of
  FULL save-state snapshots, recorded every `config.rewind.interval_frames` real frames (default
  6, i.e. ~10 Hz) up to `config.rewind.capacity` entries, oldest evicted first. This is simpler
  than the originally-sketched "keyframes + deltas" design — delta-compression is a possible
  future memory optimization, not a correctness requirement. **`capacity: 0` is the shipped
  default**, making recording a permanent no-op — off by default until a Settings-UI toggle + a
  dedicated hotkey land; the Emulation → Rewind menu item and the mechanism itself are both live
  today, driven purely by config. A user (or future UI) enabling it might reasonably pick
  something like `capacity: 300` at the default 6-frame interval (≈30s of NTSC rewind) — that's
  an example config, not what ships. Recorded snapshots are discarded (`RewindBuffer::clear`) on
  ROM load/close (a new cart invalidates any prior snapshot), NOT on Reset/Power-Cycle (rewinding
  past an accidental reset is a legitimate use).
- **Run-ahead** (`v0.3.0 "Continuum"`, `crate::rewind::step_with_run_ahead`) peeks
  `config.run_ahead.frames` frames ahead using the currently-latched input each displayed frame,
  presents that peek's video, then rolls back and re-runs exactly ONE real frame — so the
  persisted state (and its audio, the continuous stream — peek audio is never played) only ever
  advances by one frame per call, regardless of the peek depth. `frames: 0` (the shipped default)
  degrades to a plain `run_frame` — off by default.
- Both are pure re-simulation of the SAME deterministic core (`docs/adr/0004`): no injected
  timing/RNG, just running the existing `run_frame`/`save_state`/`load_state` extra times. Proven
  by `rewind.rs`'s tests, which hand-assemble a tiny 65C816 program (NMI-driven WRAM counter →
  CGRAM backdrop write) to get a real, observable per-frame state signal rather than asserting
  against a synthetic fingerprint.

## wasm

A winit+wgpu build plus a lightweight canvas embed (mirrors RustyNES); the determinism path is
identical to native. Reuse the RustyNES / prior Rusty\* frontend shell where possible rather
than re-authoring.

## Reuse posture

Reuse the egui shell, the audio ring, the pacing matrix, and the debugger-panel scaffolding
from the RustyNES frontend; SNES-specific work is the second CPU/APU panel, the Mode-7 / HDMA
debug views, and the coprocessor status panel.

## Debugger overlay (`v0.8.0 "Instrumentation"`, T-81-001)

`ui_shell.rs`'s debugger window's 4 panels (65C816 / PPU1+2 / SPC700+S-DSP / Cart) render a
`DebugSnapshot` the app copies out under the same brief lock `ShellInfo` already uses — CPU
registers/flags, key PPU registers + the dot/scanline timeline + a scrollable VRAM window + full
CGRAM, SPC700 PC/halt state + all 8 S-DSP voices' key registers, and the active board name.
Gated behind the `debug-hooks` feature (default off) at the menu-entry level: without it,
`debugger_open` can never become `true`, so the app never builds a snapshot and the default
build's emulation output is unaffected. Disassembly + breakpoints/step controls, and read/write
watchpoints (needing a new `debug-hooks` feature on `rustysnes-core` itself + a `Bus`-level
hook), are follow-up tickets (T-81-006, T-81-001b) — not yet landed.

## Open questions

- ~~Whether the second-CPU (SA-1 / Super FX) state warrants its own debugger panel from day one
  or a Phase 8 add~~ — **resolved, `v0.8.0`:** yes, from day one. The Cart panel shows SA-1's
  second-CPU registers (`System::sa1_regs`) or the Super FX/GSU register file
  (`Board::debug_gsu_state`) when the loaded cart uses either.
