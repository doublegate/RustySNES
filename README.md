<!-- markdownlint-disable MD033 MD041 -->
<div align="center">

# RustySNES

**A cycle-accurate Super Nintendo / Super Famicom emulator in Rust.**

</div>

<p align="center">
  <a href="https://github.com/doublegate/RustySNES/actions"><img src="https://github.com/doublegate/RustySNES/workflows/Deploy%20Pages%20(demo%20+%20docs)/badge.svg" alt="Build Status"></a> <a href="#license"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a> <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/rust-1.96-orange.svg" alt="Rust: 1.96"></a><br>
  <a href="https://doublegate.github.io/RustySNES/"><img src="https://img.shields.io/badge/play-in%20browser-success.svg" alt="Try in browser"></a><br>
  <a href="#platform-support"><img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS%20%7C%20Web-lightgrey.svg" alt="Platform"></a>
</p>

## Overview

**RustySNES is a cycle-accurate Super Nintendo Entertainment System (SNES) emulator written in pure Rust.** Following the lineage of its predecessor `RustyNES`, it targets the Mesen2 / ares / higan accuracy bar—featuring a master-clock lockstep scheduler, a strictly-owned bus, and a deterministic audio resync model.

Beyond reference accuracy, RustySNES aims to be a complete, modern emulation platform: providing advanced coprocessor support (DSP-1..4, Super FX, SA-1), a suite of debugging and development tools (breakpoints, watchpoints, hex editors), rollback netplay, RetroAchievements, and cross-platform native + WebAssembly support.

**[Try it in your browser](https://doublegate.github.io/RustySNES/)** — no install required.

---

## Why RustySNES?

RustySNES combines **accuracy-first emulation** with **modern features** and the **safety guarantees of Rust**. 

**Key differentiators:**

- **Reference-grade accuracy** — A from-scratch core on a 21.477 MHz NTSC master clock with run-to-timestamp catch-up for all chips. It flawlessly handles the 5A22 CPU's variable-cycle (6/8/12) instruction timings and exact PPU dot-clock sub-instruction behaviours.
- **Determinism as a hard contract** — The asynchronous SPC700/S-DSP audio processor is kept perfectly coherent with the main CPU through an integer relative-time accumulator. The same seed, ROM, and input sequence yield a bit-identical framebuffer and audio output.
- **Modern features** — A suite of tools extending far beyond traditional emulation, including Lua scripting, TAS piano-roll editing, rewind capabilities, and display-sync pacing.
- **Safe, modular Rust** — The chip stack is `no_std + alloc` with a one-directional workspace graph, making each component independently testable. 

---

## Highlights

| Feature                | Description                                                                                  |
| ---------------------- | -------------------------------------------------------------------------------------------- |
| **Cycle-Accurate**     | Master-clock-precise CPU / PPU / APU scheduling. |
| **Advanced Coprocessors** | First-class support for the DSP family (NEC µPD77C25 LLE core), Super FX (GSU-1/2), and the SA-1 dual-CPU ASIC. |
| **RetroAchievements**  | Native integration for leaderboards, achievements, rich presence, and hardcore mode. |
| **Rollback Netplay**   | GGPO-style rollback for multiplayer, over UDP or browser WebRTC. |
| **TAS Tools**          | Frame-perfect deterministic record/replay with branching save-states. |
| **Run-Ahead**          | Latency reduction that hides internal game lag. |
| **Video Filters & Shaders** | Composable shader stack, CRT/scanline passes, and native integer scaling. |
| **Lua Scripting**      | Sandboxed Lua 5.4 scripting for custom HUDs, state modification, and movie driving. |
| **Debugger**           | Mesen2-class debugger with expression breakpoints, read/write/execute watchpoints, a hex editor, and RAM watch. |
| **Libretro Core**      | Seamless integration into RetroArch (`rustysnes_libretro.so`) with dynamic sync and rollback support. |
| **Pure Rust**          | `winit` + `wgpu` + `cpal` + `egui` frontend; strictly safe `no_std + alloc` chip stack. |

---

## Crates & Architecture

The workspace strictly enforces a one-directional dependency graph to isolate emulation systems from one another, connected only through the core bus.

- `rustysnes-cpu` — WDC 65C816 (Ricoh 5A22)
- `rustysnes-ppu` — PPU1 (5C77) + PPU2 (5C78)
- `rustysnes-apu` — SPC700 + S-DSP
- `rustysnes-cart` — LoROM/HiROM/ExHiROM + coprocessor implementations
- `rustysnes-core` — The Bus + scheduler tie crate
- `rustysnes-frontend` — The `winit + wgpu + cpal + egui` desktop/web shell (binary `rustysnes`)
- `rustysnes-test-harness` — The AccuracyCoin-equivalent oracle

## Build / test

```bash
cargo check --workspace
cargo test --workspace
cargo test --workspace --features test-roms
cargo run --release -p rustysnes-frontend -- path/to/rom.sfc
```

### WebAssembly

To build the Wasm browser frontend, use `trunk`:

```bash
cd crates/rustysnes-frontend/web
trunk build --release
```

## License

RustySNES is dual-licensed under **MIT OR Apache-2.0**. See `LICENSE-MIT` and `LICENSE-APACHE`.
