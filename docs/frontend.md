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

- Save-states serialize the deterministic core state (including the SPC relative-time
  accumulator and the seeded power-on phase).
- Rewind is a ring of keyframes + deltas; run-ahead re-simulates from a snapshot.

**Deferred (next sprint):** save-states / rewind / run-ahead are not yet implemented. They require
a core-wide deterministic snapshot — `Clone`/serialize across the `Board` trait (and its boards),
the `Apu`/`Dsp`, `Bus`, and `System` — which is a larger core/cart change than the Phase-5
frontend work and is sequenced as its own sprint so the determinism oracle is re-validated with it.
The menu items are present and report this honestly rather than corrupting state.

## wasm

A winit+wgpu build plus a lightweight canvas embed (mirrors RustyNES); the determinism path is
identical to native. Reuse the RustyNES / prior Rusty\* frontend shell where possible rather
than re-authoring.

## Reuse posture

Reuse the egui shell, the audio ring, the pacing matrix, and the debugger-panel scaffolding
from the RustyNES frontend; SNES-specific work is the second CPU/APU panel, the Mode-7 / HDMA
debug views, and the coprocessor status panel.

## Open questions

- Whether the second-CPU (SA-1 / Super FX) state warrants its own debugger panel from day one
  or a Phase 8 add — defer to when Phase 4 lands SA-1.
