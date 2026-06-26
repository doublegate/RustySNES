# RustySNES — STATUS (single source of truth)

This file is authoritative for per-suite pass counts, the board / coprocessor matrix, and
version policy. Everything else defers to it.

**Current release:** v0.1.0 (scaffold). **Phases 1 (CPU + golden oracle) and 2 (scheduler +
video) are functionally complete** — the 65C816 passes the SingleStepTests/65816 oracle to
0-diff (state + cycles), and the machine **boots and runs real ROMs**: the master-clock lockstep
scheduler + bus memory map + DMA/HDMA + the dual-chip PPU produce a deterministic framebuffer.
gilyon's on-cart CPU suite reports "Success" (all 1107 tests), and the undisbeliever PPU/DMA/HDMA
suite renders bit-deterministic golden framebuffers. Audio (Phase 3) and coprocessors (Phase 4)
are not started.

## Subsystem progress

| Crate | Chip | State |
|---|---|---|
| `rustysnes-cpu` | WDC 65C816 (5A22) | **Phase 1 complete — 65816 oracle 0-diff (state+cycles), all 256 opcodes × modes, native+emulation, REP/SEP/XCE** |
| `rustysnes-ppu` | PPU1 (5C77) + PPU2 (5C78) | **Phase 2 — BG 0-7 + Mode 7 + 128-sprite OAM + color math + windows + dot/HV timeline; per-scanline compositor (mid-line raster/hi-res deferred)** |
| `rustysnes-apu` | SPC700 (S-SMP) + S-DSP + ARAM | **Phase 3 — SPC700 oracle 0-diff; S-DSP behavioral; integrated into the machine: the 4 `$2140-$2143` ports route through the real `Apu`, the integer-accumulator async resync clocks the SMP in **cycle-exact sub-instruction lockstep** (`68_352/715_909`, ADR 0004), SMP base-clock + timer + DSP rates ares-correct; blargg `spc_*` boot+upload+run bit-deterministically; the **timer-phase fix** (timebase/timers clocked before the write side effect, ares/Mesen2-correct) + the **DSP GAIN mode-7 threshold fix** (unsigned `hidden_env >= 0x600`, blargg/ares-correct) drive **all four `spc_*` (`spc_smp`/`spc_timer`/`spc_mem_access_times`/`spc_dsp6`) to literal `PASSED TESTS`** (asserted)** |
| `rustysnes-cart` | LoROM/HiROM/ExHiROM + coprocessors | **Phase 2 base map modes + Phase 4 DSP-1: chipset-byte coprocessor detection, the shared µPD77C25/µPD96050 LLE engine, and the DSP-1 board (boots real DSP-1 games with user-supplied firmware). Super FX/GSU + SA-1 next** |
| `rustysnes-core` | Bus + master-clock scheduler + DMA/HDMA | **Phase 2 — master-clock lockstep (6/8/12 access map), full memory decode, CPU regs + mul/div, GP-DMA + HDMA, NMI/HV-IRQ** |
| `rustysnes-frontend` | egui shell + audio ring + pacing | not started |
| `rustysnes-netplay` | rollback netplay | not started |
| `rustysnes-cheevos` | RetroAchievements (opt-in FFI) | not started |
| `rustysnes-script` | Lua scripting / TAS API | not started |
| `rustysnes-test-harness` | golden-log + JSON-oracle + screenshot baseline | not started |

## Accuracy — per-suite pass counts

| Suite | Layer | License posture | Pass | Total |
|---|---|---|---|---|
| SingleStepTests/65816 (JSON) | CPU per-opcode oracle | **self-gen committed + upstream cross-check (external)** — ADR 0005 | **5,119,999** | 5,120,000 |
| SingleStepTests/spc700 (JSON) | SPC per-opcode oracle | MIT (committable) | **256,000** | 256,000 (0-diff) |
| gilyon/snes-tests (cputest-basic .sfc) | CPU on-cart (boots on `System`) | MIT (committed) | **1107** | 1107 (= "Success") |
| undisbeliever/snes-test-roms (.sfc) | PPU/DMA/HDMA hardware (golden framebuffer) | Zlib (committed) | **29** | 29 (deterministic) |
| blargg `spc_*` (spc_dsp6 / mem_access / smp / timer) | cycle-accurate SPC/DSP (cycle-stepped S-DSP + timer-phase + GAIN mode-7 fixes) | unstated (external) | **4 boot+run det.; 4 literal PASS** | 4 (all → literal `PASSED TESTS` asserted: `spc_smp`/`spc_timer`/`spc_mem_access_times` via timer-phase fix, `spc_dsp6` via DSP GAIN mode-7 unsigned-threshold fix) |
| 240p Test Suite (SNES) | video / overscan | GPLv2 (run-only) | 0 | TBD |
| Visual golden corpus (`tests/golden/`) | framebuffer / audio hashes | own (committed) | **29** | 29 |

- **CPU 65816 oracle (0-diff):** **100.00%** — 5,119,999 / 5,120,000 full passes
  (state + RAM + cycle count) across all 512 opcode files × 10,000 tests each, native +
  emulation. The one
  residual is a single `e1.e` (`SBC (dp,X)`, emulation) test exercising the bsnes `readDirectX`
  `DL!=0` high-byte wrap that the rest of the SingleStepTests set does **not** model — a
  documented inter-reference divergence (`docs/adr/0002` posture), not point-fixed. Measured via
  `tests/cpu_oracle.rs` against the gitignored external set (ADR 0005: cross-check only, never a
  CI dependency). No textual Nintendulator-style 65816 log exists — this JSON oracle replaces it
  (`docs/testing-strategy.md`).
- **gilyon on-cart CPU (Phase 1's deferred criterion, unblocked by the Phase-2 boot):**
  `cputest-basic.sfc` boots on the integrated `System` and renders **"Success"** with all 1107
  65C816 instruction/addressing-mode tests run (`tests/gilyon_oncart.rs`). `cputest-full.sfc`
  wedges at test 39 (`adc ($10,s),y` routed through the ROM's RAM-resident BRK handler under
  `DBR=$7E`) — a narrow ROM-dispatch edge documented as a residual; the op itself is oracle-clean.
- **undisbeliever PPU/DMA/HDMA + golden framebuffer:** all **29** ROMs boot through the
  CPU+scheduler+bus+DMA/HDMA+PPU path and produce **bit-deterministic** framebuffer hashes
  matching the committed baseline `tests/golden/undisbeliever-framebuffer.tsv`
  (`tests/undisbeliever_golden.rs`). This is the Phase-2 deterministic-golden-framebuffer gate.
- **SPC700 per-opcode oracle (0-diff):** **100.00%** state + cycle count over all 256 opcodes
  (`tests/spc700_oracle.rs`): 12,800 committed-sample tests in-tree, 256,000 in the full external
  tier. Unaffected by the Phase-3 SMP base-clock correction (the oracle replays against its own
  flat bus, measuring access count, not the integrated wait-state model).
- **blargg `spc_*` (Phase-3 audio integration — cycle-exact SMP↔CPU + cycle-stepped S-DSP):** the
  SMP advances in **cycle-exact sub-instruction lockstep** with the main CPU (`Apu::advance_smp_cycle`
  releases one SMP base clock at a time from a recorded micro-op timeline; each SMP→CPU port write
  becomes visible at the precise base cycle it lands on), **and the S-DSP is now itself
  cycle-stepped** — it runs its 32-step ares micro-sequence one `Dsp::tick` per 2 SMP base clocks
  (32 ticks = one 32 kHz sample) instead of a whole sample per 64 clocks, so a mid-instruction
  DSP-register read sees cycle-correct OUTX/ENVX/ENDX/envelope state (`docs/apu.md`
  §cycle-accurate DSP, §cycle-exact). All four ROMs (`spc_smp`, `spc_timer`, `spc_mem_access_times`,
  `spc_dsp6`) boot, complete the IPL upload handshake, run the SPC700 program, and stream their
  detailed result text. The gate (`tests/blargg_spc.rs`) **decodes and asserts the real on-screen
  verdict**, retaining the deterministic + baseline-hash assertion (`spc_timer` / `spc_mem_access_times`
  re-blessed in `tests/golden/blargg-spc.tsv` for the timer-phase timing change; `spc_smp` /
  `spc_dsp6` hashes unchanged); it is **not** weakened to determinism-only. The **timer-phase fix**
  (`RecordingSmpBus::write` now advances the SMP timebase + clocks the timers **before** the write
  side effect, matching ares `step()` / Mesen2 `Spc::Write` `IncCycleCount`-first; `docs/apu.md`
  §timer phase) drove `spc_smp`, `spc_timer`, `spc_mem_access_times` to blargg's literal
  `PASSED TESTS`; the **DSP GAIN mode-7 threshold fix** (the bent/two-slope GAIN increase compares
  its internal envelope latch against `0x600` **unsigned** — blargg `SPC_DSP` `(unsigned) hidden_env`
  / ares `(u32) _envelope` — so a latch left negative by a prior GAIN decrease still trips the
  reduced slope; `docs/apu.md` §DSP GAIN mode-7 threshold) drove `spc_dsp6` to `PASSED TESTS` too.
  **All four ROMs now reach blargg's literal `PASSED TESTS`** (the gate asserts each, not a
  determinism proxy). The per-opcode SPC700 (oracle 0-diff) + per-tick S-DSP math is correct.
- **Master-clock totals:** a booted NTSC frame advances ≈357,374 master clocks (spec ≈357,368),
  confirming the 6/8/12 access map + dot timeline.
- **Accuracy battery (the composed two-layer oracle):** the CPU layer is 0-diff; the on-cart
  layer is gilyon-basic green + undisbeliever golden. Audio + coprocessor layers pending.
- **Determinism contract:** the framebuffer is verified bit-identical across runs (same seed +
  ROM ⇒ identical hash) for all 29 undisbeliever ROMs; full save-state round-trip lands later.

## Coprocessor / board tier matrix (honesty gate: `docs/adr/0003`)

`boards_tiered = true` — the honesty gate is real: **no BestEffort board backs the oracle.**

| Chip | Core | Tier | Shared LLE core | State |
|---|---|---|---|---|
| DSP-1/1A/1B | µPD77C25 | **Core/Curated** | µPD77C25 (shared, 6 chips) | **implemented** — full µPD7725 LLE engine (`coproc::upd77c25`) + `Dsp1Board` (Lo/HiROM DR/SR windows). Boots Super Mario Kart / Pilotwings / Super Bases Loaded 2 / Aim for the Ace on the full System with user-supplied `dsp1*.rom`; deterministic golden + firmware-differential + RQM-handshake access gate (`dsp1_oncart`, 4 ROMs). Honesty gate green (`ORACLE_COPROCESSORS` ∋ DSP). Firmware gitignored, never committed |
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
| LoROM ($20) | $007FC0 | **Phase 2 — score-based header detect + ROM/SRAM decode + mirroring** |
| HiROM ($21) | $00FFC0 | **Phase 2 — detect + $C0+ linear + $00-3F:$8000 window + SRAM** |
| ExHiROM ($25) | $40FFC0 | **Phase 2 — detect + A23-inverted extended-bank decode** |
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
