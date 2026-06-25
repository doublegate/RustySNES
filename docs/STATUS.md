# RustySNES — STATUS (single source of truth)

This file is authoritative for per-suite pass counts, the board / coprocessor matrix, and
version policy. Everything else defers to it.

**Current release:** v0.1.0 (scaffold). **Phase 1 (CPU + golden oracle) is functionally
complete** — the 65C816 core passes the SingleStepTests/65816 per-opcode oracle to 0-diff on
both architectural state and per-instruction cycle count. The remaining subsystems are not
started; their counts stay **0** until their phase lands.

## Subsystem progress

| Crate | Chip | State |
|---|---|---|
| `rustysnes-cpu` | WDC 65C816 (5A22) | **Phase 1 complete — 65816 oracle 0-diff (state+cycles), all 256 opcodes × modes, native+emulation, REP/SEP/XCE** |
| `rustysnes-ppu` | PPU1 (5C77) + PPU2 (5C78) | not started |
| `rustysnes-apu` | SPC700 (S-SMP) + S-DSP + ARAM | not started |
| `rustysnes-cart` | LoROM/HiROM/ExHiROM + coprocessors | not started |
| `rustysnes-core` | Bus + master-clock scheduler + DMA/HDMA | not started |
| `rustysnes-frontend` | egui shell + audio ring + pacing | not started |
| `rustysnes-netplay` | rollback netplay | not started |
| `rustysnes-cheevos` | RetroAchievements (opt-in FFI) | not started |
| `rustysnes-script` | Lua scripting / TAS API | not started |
| `rustysnes-test-harness` | golden-log + JSON-oracle + screenshot baseline | not started |

## Accuracy — per-suite pass counts

| Suite | Layer | License posture | Pass | Total |
|---|---|---|---|---|
| SingleStepTests/65816 (JSON) | CPU per-opcode oracle | **self-gen committed + upstream cross-check (external)** — ADR 0005 | **5,119,999** | 5,120,000 |
| SingleStepTests/spc700 (JSON) | SPC per-opcode oracle | MIT (committable) | 0 | TBD |
| gilyon/snes-tests (.sfc + golden tables) | CPU + SPC on-cart | MIT (committed) | 0 | TBD (blocked: needs `System` boot — Phase 2/4) |
| undisbeliever/snes-test-roms (.sfc) | PPU/DMA/HDMA hardware | Zlib (committed) | 0 | TBD |
| blargg `spc_*` (spc_dsp6 / mem_access / spc / timer) | cycle-accurate SPC/DSP | unstated (external) | 0 | TBD |
| 240p Test Suite (SNES) | video / overscan | GPLv2 (run-only) | 0 | TBD |
| Visual golden corpus (`tests/golden/`) | framebuffer / audio hashes | own (committed) | 0 | 0 |

- **CPU 65816 oracle (0-diff):** **100.00%** — 5,119,999 / 5,120,000 full passes
  (state + RAM + cycle count) across all 512 opcode files × 10,000 tests each, native +
  emulation. The one
  residual is a single `e1.e` (`SBC (dp,X)`, emulation) test exercising the bsnes `readDirectX`
  `DL!=0` high-byte wrap that the rest of the SingleStepTests set does **not** model — a
  documented inter-reference divergence (`docs/adr/0002` posture), not point-fixed. Measured via
  `tests/cpu_oracle.rs` against the gitignored external set (ADR 0005: cross-check only, never a
  CI dependency). No textual Nintendulator-style 65816 log exists — this JSON oracle replaces it
  (`docs/testing-strategy.md`).
- **Accuracy battery (the composed two-layer oracle):** **0%.** Target ≥90% by v1.0, 100% the
  goal; hard residuals defer to the fractional-timebase refactor (`docs/adr/0002`).
- **Determinism contract:** not yet exercised (save-state round-trip + replay must be
  bit-identical).

## Coprocessor / board tier matrix (honesty gate: `docs/adr/0003`)

`boards_tiered = true` — the honesty gate is real: **no BestEffort board backs the oracle.**

| Chip | Core | Tier | Shared LLE core | State |
|---|---|---|---|---|
| DSP-1/1A/1B | µPD77C25 | **Core/Curated** | µPD77C25 (shared, 6 chips) | not started |
| Super FX / GSU-1/2 | Argonaut RISC | **Core/Curated** | — (cart ROM) | not started |
| SA-1 | 65C816 @ 10.74 MHz | **Core/Curated** | (reuses CPU core) | not started |
| DSP-2 / DSP-3 / DSP-4 | µPD77C25 | BestEffort | µPD77C25 (shared) | not started |
| ST010 / ST011 | µPD96050 | BestEffort | µPD96050 (shared) | not started |
| S-DD1 | Nintendo ASIC | BestEffort | — | not started |
| SPC7110 (+RTC-4513) | Hudson ASIC | BestEffort | — (frozen RTC) | not started |
| CX4 | Hitachi HG51B169 | BestEffort/Curated | — | not started |
| OBC1 | simple ASIC | BestEffort | — (HLE) | not started |
| ST018 | ARMv3 | BestEffort | — (separate ARM LLE) | not started |
| S-RTC | Epson RTC | BestEffort | — (frozen time) | not started |

One **µPD77C25 / µPD96050 LLE engine** covers DSP-1/2/3/4 + ST010/011 (six chips, one engine).

## Memory-map model support

| Model | Header offset | State |
|---|---|---|
| LoROM ($20) | $007FC0 | not started |
| HiROM ($21) | $00FFC0 | not started |
| ExHiROM ($25) | $40FFC0 | not started |
| ExLoROM | — | deferred |

## Region timing

| Region | Master clock | Lines/frame | State |
|---|---|---|---|
| NTSC | 21.477270 MHz | 262 (+1 interlaced) | not started |
| PAL | 21.281370 MHz | 312 (+1 interlaced) | not started |

Region is **data, not a build fork** (`docs/scheduler.md`).

## Version policy

- Start at **v0.1.0**. Additive features ship behind **default-off feature flags** so the
  shipped / native / `no_std` / wasm builds stay byte-identical with the flags off.
- The **fractional-timebase refactor** is the one milestone expected to break byte-identity /
  save-state compatibility (`docs/adr/0002`) — and only happens if hard residuals warrant it.
- **Do NOT** import RustyNES engine-lineage "v2.0" anchors as RustySNES releases. The forward
  path is Phase 0 → v1.0.0 → (only then) the fractional-timebase refactor; see
  `to-dos/ROADMAP.md`.
