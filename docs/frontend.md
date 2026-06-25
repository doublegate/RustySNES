# Frontend — RustySNES

**References:** `docs/architecture.md` §6; `ref-docs/research-report.md` "External
dependencies"; `docs/adr/0004` (the determinism boundary).

## Purpose

`rustysnes-frontend` is the desktop + wasm shell: **winit + wgpu + cpal + egui**, pure Rust
and permissive (mirrors RustyNES). It is an **always-on egui shell, not a bare window** —
egui runs every frame.

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
