# RustySNES — Version Plan (v0.1.0 to v1.0.0)

This document outlines the incremental builds and release milestones for RustySNES from the current state (v0.1.0) through the v1.0.0 completion. It matches the quality and execution standards of its predecessor, RustyNES.

## v0.1.0 — CPU Oracle & Foundation (Current)
- **Status:** Complete.
- **Features:** WDC 65C816 CPU passes SingleStepTests/65816 per-opcode oracle to 0-diff. Master-clock lockstep scheduler, dual-chip PPU, DMA/HDMA, and base cart mappers boot and run real ROMs.

## v0.2.0 — Audio Core
- **Goal:** Cycle-accurate SPC700 and S-DSP integration.
- **Features:**
  - SPC700 CPU reaching 0-diff against SingleStepTests/spc700.
  - S-DSP implementation (BRR decode, Gaussian interpolation, envelopes).
  - The asynchronous integer-accumulator resync model.
  - Passing `blargg` `spc_*` test ROMs.

## v0.3.0 — Memory Map & LLE Coprocessors
- **Goal:** Base map completeness and the NEC DSP family.
- **Features:**
  - LoROM, HiROM, and ExHiROM memory map models fully tested.
  - Internal header auto-detection heuristic.
  - LLE core for NEC µPD77C25 (DSP-1, DSP-2, DSP-3, DSP-4).

## v0.4.0 — Advanced Coprocessors (Super FX & SA-1)
- **Goal:** Emulation of the two most complex coprocessors.
- **Features:**
  - Super FX / GSU (Argonaut RISC) implementation.
  - SA-1 (second 65C816) integration with parallel stepping in the scheduler.
  - Verification against coprocessor test ROMs and commercial games (e.g., Yoshi's Island, Super Mario RPG).

## v0.5.0 — Desktop Frontend
- **Goal:** Playable native interface.
- **Features:**
  - `wgpu` + `winit` + `egui` shell.
  - Lock-free audio ring with dynamic rate control (DRC).
  - Controller support (keyboard + USB gamepads).
  - 8:7 pixel-aspect correction and basic video filters.

## v0.6.0 — Save States, Rewind, & Run-Ahead
- **Goal:** Core determinism applied to UX features.
- **Features:**
  - Core-wide deterministic snapshotting across the `Board` trait, Bus, and APU.
  - Ring buffer for instant rewind.
  - Run-ahead latency reduction.

## v0.7.0 — WebAssembly & Portability
- **Goal:** Run RustySNES seamlessly in the browser.
- **Features:**
  - `wasm32-unknown-unknown` compilation.
  - Canvas / WebGL integration.
  - Browser-based audio and input handling.

## v0.8.0 — Debugger & Developer Tools
- **Goal:** Mesen2-class debugging suite.
- **Features:**
  - Expression / conditional breakpoints and R/W/X watchpoints.
  - Hex editor, RAM watch, and tile/palette/OAM viewer.
  - Cycle trace logger and event viewer (IRQ/NMI).

## v0.9.0 — The "Reach" Features
- **Goal:** Additive, opt-in modern features.
- **Features:**
  - Lua 5.4 scripting sandbox.
  - Rollback netplay (UDP + WebRTC).
  - RetroAchievements native integration (`rcheevos`).
  - TAStudio piano-roll editor and TAS movie recording (`.rnm`).

## v1.0.0 — Production Cut & 100% Accuracy
- **Goal:** Reference-grade hardware accuracy and final polish.
- **Features:**
  - Composed two-layer accuracy battery reaching ≥90% pass rate (aiming for 100%).
  - Libretro core (`rustysnes_libretro.so`) integration.
  - Finalized documentation, release matrix (cross-platform binaries), and Pages deployment.
