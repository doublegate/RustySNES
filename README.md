<!-- markdownlint-disable MD033 MD041 -->
<div align="center">

# RustySNES

**A cycle-accurate Super Nintendo / Super Famicom emulator in Rust.**

</div>

<p align="center">
  <a href="https://github.com/doublegate/RustySNES/actions"><img src="https://github.com/doublegate/RustySNES/workflows/CI/badge.svg" alt="Build Status"></a> <a href="#license"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a> <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/rust-1.96-orange.svg" alt="Rust: 1.96"></a><br>
  <a href="https://doublegate.github.io/RustySNES/"><img src="https://img.shields.io/badge/pages-demo%20%2B%20rustdoc-success.svg" alt="GitHub Pages"></a><br>
  <a href="#platform-support"><img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS%20%7C%20Web-lightgrey.svg" alt="Platform"></a>
</p>

## Overview

**RustySNES is a cycle-accurate Super Nintendo Entertainment System (SNES) emulator written in pure Rust.** Following the lineage of its predecessor [`RustyNES`](https://github.com/doublegate/RustyNES), it targets the Mesen2 / ares / higan accuracy bar — a master-clock lockstep scheduler, a strictly-owned bus, and a deterministic audio resync model.

Beyond reference accuracy, RustySNES aims to be a complete, modern emulation platform. See [`to-dos/VERSION-PLAN.md`](to-dos/VERSION-PLAN.md) for the named, versioned release ladder from where the project stands today to `v1.0.0` and beyond — the table below distinguishes what's **shipped** from what's **planned**, so this README stays accurate as the project moves through that ladder rather than describing a future state as if it already exists.

---

## Why RustySNES?

RustySNES combines **accuracy-first emulation** with the **safety guarantees of Rust**, and is building toward the same modern-feature breadth as its sibling projects ([`RustyNES`](https://github.com/doublegate/RustyNES), [`Rusty2600`](https://github.com/doublegate/Rusty2600)).

**Key differentiators:**

- **Reference-grade accuracy** — A from-scratch core on a 21.477 MHz NTSC master clock with a lockstep scheduler for every chip. The 5A22 CPU's variable-cycle (6/8/12) instruction timings and dot-accurate PPU/HDMA behavior are cycle-exact, not approximated.
- **Determinism as a hard contract** — The asynchronous SPC700/S-DSP audio processor is kept perfectly coherent with the main CPU through an integer relative-time accumulator, with no floating point in the timing path. The same seed, ROM, and input sequence yield a bit-identical framebuffer and audio output — the foundation that save-states, rewind, and netplay rollback all build on.
- **Honest accuracy tiering** — Every coprocessor/board is tiered Core / Curated / BestEffort (see [`docs/adr/0003`](docs/adr/0003-accuracy-tiering-honesty-gate.md)); a CI honesty gate ensures no unverified BestEffort board ever backs the accuracy oracle. Nothing is silently degraded.
- **Safe, modular Rust** — The chip stack is `no_std + alloc` with a one-directional workspace graph, making each component independently testable.

---

## Feature status

| Feature | Status |
| --- | --- |
| **Cycle-accurate CPU/PPU/APU** | ✅ Shipped — 65816 + SPC700 oracle 0-diff, master-clock lockstep scheduler, dot-accurate HDMA/interrupts. |
| **Coprocessors** | ✅ Shipped (Core/Curated): DSP-1, Super FX/GSU, SA-1. ✅ Shipped (BestEffort, validated against real ROMs): DSP-2, DSP-4, ST010, CX4, OBC1, S-DD1. ✅ Shipped (BestEffort, `v0.4.0`, unit-tested only — no commercial dump in the local corpus): ST018 (full ARMv3 core), standalone S-RTC. 🚧 SPC7110 implemented, addressing bug fixed, still not booting. |
| **Native desktop frontend** | ✅ Shipped — `winit` + `wgpu` + `cpal` + `egui`; keyboard + gamepad input, ROM/firmware/SRAM loading (including zip-archived ROMs), Reset/Power-Cycle/Pause. |
| **WebAssembly build** | 🚧 Compiles; the in-browser UI (canvas surface, `requestAnimationFrame` loop, file loading) is a bootstrap scaffold, not yet a playable demo. |
| **Save states** | ✅ Shipped — `v0.2.0`, a versioned deterministic envelope, see [`docs/adr/0006`](docs/adr/0006-save-state-format.md). |
| **Rewind / run-ahead** | ✅ Shipped — `v0.3.0`; config-driven, off by default (`rewind.capacity: 0` / `run_ahead.frames: 0`). |
| **PAL region auto-detection / ExLoROM** | ✅ Shipped — `v0.3.0`; neither has golden-ROM-boot validation yet (no PAL or ExLoROM ROM in the local corpus). |
| **Lua scripting** | ⏳ Planned — post-`v1.0.0`. |
| **Debugger (breakpoints, memory viewer)** | ⏳ Planned — post-`v1.0.0`. |
| **Rollback netplay** | ⏳ Planned — post-`v1.0.0`, builds on save-states. |
| **RetroAchievements** | ⏳ Planned — post-`v1.0.0`, via `rcheevos`. |
| **TAS movie recording** | ⏳ Planned — post-`v1.0.0`. |
| **Video filters / shaders, Libretro core** | ⏳ Planned — post-`v1.0.0`, stretch scope. |

Legend: ✅ shipped and usable today · 🚧 in progress / partially working · ⏳ not started. See [`to-dos/ROADMAP.md`](to-dos/ROADMAP.md) (the phase spine) and [`to-dos/VERSION-PLAN.md`](to-dos/VERSION-PLAN.md) (the release ladder) for exactly which release each planned item lands in, and [`docs/STATUS.md`](docs/STATUS.md) for the authoritative, always-current per-subsystem detail.

---

## Crates & Architecture

The workspace strictly enforces a one-directional dependency graph to isolate emulation systems from one another, connected only through the core bus.

- `rustysnes-cpu` — WDC 65C816 (Ricoh 5A22)
- `rustysnes-ppu` — PPU1 (5C77) + PPU2 (5C78)
- `rustysnes-apu` — SPC700 + S-DSP
- `rustysnes-cart` — LoROM/HiROM/ExHiROM + coprocessor implementations
- `rustysnes-core` — The Bus + scheduler tie crate
- `rustysnes-frontend` — The `winit + wgpu + cpal + egui` desktop/web shell (binary `rustysnes`)
- `rustysnes-test-harness` — The accuracy oracle (SingleStepTests runners, golden-log suites, per-coprocessor commercial-ROM validation)
- `rustysnes-netplay`, `rustysnes-cheevos`, `rustysnes-script` — reserved crates for the post-`v1.0.0` Reach features (netplay, RetroAchievements, Lua scripting); currently scaffolds.

See [`docs/DOCUMENTATION_INDEX.md`](docs/DOCUMENTATION_INDEX.md) for the full documentation map (subsystem specs, ADRs, testing strategy, and more).

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
