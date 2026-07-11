# Architecture — RustySNES (load-bearing facts)

**References:** `ref-docs/research-report.md` §§1–7, plus `docs/scheduler.md`,
`docs/adr/0001`–`docs/adr/0004`.

These cross-cutting decisions span every crate. Read this before any chip doc — reading a
chip spec in isolation will mislead, because the timing model and bus ownership are global.

## Purpose

RustySNES is a cycle-accurate Super Nintendo / Super Famicom emulator in pure Rust. The
accuracy bar is Mesen-S / ares (per `ref-docs/research-report.md` "State of the art"): the
one reference core legally adaptable is **ares (ISC)**; bsnes / Mesen-S / higan are GPLv3
clone-only and Snes9x is non-commercial. We build clean-room from the test ROMs and the
`ref-docs/` corpus, not from copyleft source lines.

## The seven load-bearing facts

### 1. The master clock drives a lockstep scheduler

Per `ref-docs/research-report.md` §1, the whole main system derives from a single
**21.477270 MHz NTSC** (PAL 21.281370 MHz) master clock. The scheduler advances the master
clock and steps every other chip on its divisor — **lockstep, not catch-up**. This makes
mid-instruction events (a mid-scanline scroll / Mode-7 / CGRAM write landing at an exact dot,
HDMA firing at H≈$116, an H/V-IRQ at a precise counter position) visible to subsequent CPU
code without per-quirk patches. The SNES twist over the NES: the CPU cycle is **variable**
(6/8/12 master clocks per access) and the dot / scanline lengths vary
(1360/1364/1368 clocks). See `docs/scheduler.md` and `docs/adr/0001`.

### 2. The Bus owns everything mutable

`rustysnes-core::Bus` holds the PPU, APU/SMP, cart (with its coprocessor / board logic),
WRAM, the DMA/HDMA controller, controllers, and the open-bus latch. The CPU borrows
`&mut Bus` during `tick()`. As in RustyNES (the TetaNES-postmortem lesson), this single
choice avoids the borrow-checker fight that "CPU holds PPU, but PPU also needs the CPU bus"
creates. The PPU and SMP each see a **narrow trait** (`PpuBus`, `SmpBus` / `DspBus`) for only
what they need — VRAM/CGRAM/OAM access and ARAM access respectively.

### 3. The crate graph is one-directional

```text
rustysnes-cpu   (65C816 — no PPU/APU/cart dep)
rustysnes-ppu   (PPU1+PPU2 — VRAM/CGRAM/OAM only)
rustysnes-apu   (SPC700 + S-DSP + ARAM — independent)
rustysnes-cart  (memory map + coprocessor families — independent)
        \         |         /        /
         rustysnes-core   (ties them together, re-exports public types)
                 |
   rustysnes-{frontend, netplay, cheevos, script, test-harness}
```

No chip crate depends on another. `rustysnes-core` is the only crate that knows all four.
Result: each chip is fuzzable and benchmarkable in isolation. Adding a cross-chip dependency
breaks this invariant — don't. Downstream consumers depend on `rustysnes-core`, never the
chip crates directly.

### 4. Board / coprocessor logic lives in the cart crate

Per `ref-docs/2026-06-24-coprocessors.md`, each SNES coprocessor (DSP-1..4, Super FX/GSU,
SA-1, S-DD1, SPC7110, CX4, OBC1, ST01x/ST018, S-RTC) is a "mapper-equivalent" with its own
bus window and clock. All of it lives behind a `Cart` / `Coprocessor` trait in
`rustysnes-cart` with default-no-op hooks — the PPU and CPU never special-case a board. Six
of the chips share **one µPD77C25 / µPD96050 LLE core** (DSP-1/2/3/4 + ST010/011), so the
cart crate implements that engine once. See `docs/cart.md`.

### 5. Determinism is a hard contract

Same seed + ROM + input sequence ⇒ bit-identical framebuffer and audio. Power-on
CPU/PPU/SMP phase alignment comes from a **seeded PRNG**; reset preserves it. The async
SPC700 domain is tracked by an **integer relative-time accumulator** (no floating point, no
host wall-clock) — real-hardware resonator drift (±0.5%) and RTC chips are deliberately
frozen out of the deterministic path. This is required for save-state round-trip, regression
tests, TAS replay, and netplay rollback. See `docs/adr/0004` and `docs/apu.md` §2.

### 6. The frontend is an always-on egui shell

`rustysnes-frontend` is winit + wgpu + cpal + egui, and egui runs **every frame** — a
persistent menu bar + status bar + tabbed Settings, with toggleable debugger panels layered
on top. The shell never holds the emu lock inside the egui closure: menu interactions return
a `MenuAction` dispatched *after* the egui pass. On native the emulator runs on a dedicated
thread; the winit thread only does UI + present. Rate control and run-ahead live here, never
in the core synthesis (that is what keeps fact #5 intact). See `docs/frontend.md`.

### 7. Test ROMs are the spec

When the docs and a passing test ROM disagree, the ROM wins and the docs get updated. The
oracle is **two-layer** (`ref-docs/research-report.md` "Standards / test-ROM corpora"): the
SingleStepTests 65816 + spc700 JSON per-opcode suites, the committable
gilyon / undisbeliever ROMs, blargg's `spc_*` for audio, and the 240p Suite for video. See
`docs/testing-strategy.md`.

## Crate inventory

| Crate | Owns |
|---|---|
| `rustysnes-cpu` | WDC 65C816 (5A22 core): emulation / native modes, variable access cycles, vectors. |
| `rustysnes-ppu` | PPU1 (5C77) sprites / Mode-7-multiply + PPU2 (5C78) CGRAM / output / counters. |
| `rustysnes-apu` | SPC700 (S-SMP) + S-DSP + 64 KiB ARAM; the async domain + BRR + 8 voices. |
| `rustysnes-cart` | LoROM/HiROM/ExHiROM map + header detect + the coprocessor families. |
| `rustysnes-core` | `Bus`, the master-clock scheduler, DMA/HDMA, multiply / divide units, joypad auto-read; re-exports chip types. Also the `EmuCore` embedding facade (`facade` module, `std`-only, `v1.2.0`) — load/step/framebuffer/audio/save-state, for any headless embedder (a libretro core, `rustysnes-frontend`'s own thin wrapper). |
| `rustysnes-frontend` | The egui shell, audio ring, pacing, gamepads, save-states, rewind, wasm. |
| `rustysnes-netplay` | Rollback netplay (frontend-orchestrated; deterministic core required). |
| `rustysnes-cheevos` | RetroAchievements (opt-in, native FFI). |
| `rustysnes-script` | Lua scripting / TAS API. |
| `rustysnes-test-harness` | Golden-log differ, `run_until_complete`, JSON-oracle runner, screenshot baseline. |

The chip stack is `#![no_std]` + `extern crate alloc;`; `rustysnes-core` is conditionally so
(`#![cfg_attr(not(feature = "std"), no_std)]`, `v1.2.0`) — its default `std` feature enables the
`facade` module, and disabling it (the `thumbv7em` no_std CI gate) restores unconditional
`no_std`, proving the facade compiles out entirely rather than merely going unused. Only
`rustysnes-frontend` and `rustysnes-cheevos` (FFI) carry `unsafe` (each with a `// SAFETY:`
comment).

## Architectural alternatives (rejected)

Per `ref-docs/research-report.md` "Architecture options":

- **Cooperative-threaded coroutines (higan / bsnes libco model).** The proven accuracy
  model; used as the *conceptual* reference. Rejected as the *literal* implementation:
  coroutines fit Rust awkwardly and complicate bit-deterministic save-states / netplay. We
  borrow its relative-time-accumulator + sync-on-access technique for the SPC700 domain only.
- **Catch-up / lazy sync (Snes9x model).** Run the CPU freely, catch the PPU/APU up at sync
  points. Rejected: loses the sub-instruction accuracy the bar demands.

The chosen path is single-threaded master-clock lockstep (fact #1) with the SPC700 resync
borrowed from the coroutine model.

## Open questions

- Whether any hard-tier residual forces the fractional-timebase refactor before v1.0 (it is
  designed in from day one regardless — see `docs/adr/0002`).
- Per-board SRAM / coprocessor bus windows are not canonically tabulated; build them from the
  cartridge database + ares board definitions during Phase 4 (`ref-docs/research-report.md`
  "Open questions" #3).
