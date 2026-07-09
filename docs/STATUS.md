# RustySNES — STATUS (single source of truth)

This file is authoritative for per-suite pass counts, the board / coprocessor matrix, and
version policy. Everything else defers to it.

**Current release:** `v0.7.0 "Resolution"` (`v0.1.0 "Foundation"`, `v0.2.0 "Persistence"`,
`v0.3.0 "Continuum"`, `v0.4.0 "Completion"`, `v0.5.0 "Fidelity"`, and `v0.6.0 "Shippable"`
precede it; see `to-dos/VERSION-PLAN.md` for the full breadth-inclusive `v0.7.0`→`v1.0.0` ladder).
`v0.5.0` closed out the accuracy-pass-rate dashboard (see "Accuracy dashboard" below) and the
full named hardware-gotcha regression list — every item fixed, correctly reclassified as an
intentional non-goal, or honestly researched-and-deferred with a full mechanism write-up. `v0.6.0`
closed out release engineering and doc parity — `security.yml`, checksummed release assets,
automated release-cutting (`release-auto.yml`), the `lint` job's `cargo doc` gate,
`docs/DOCUMENTATION_INDEX.md`, `docs/benchmarks.md`, `docs/audit/`, and 9 total ADRs
(`to-dos/VERSION-PLAN.md`'s v0.6.0 section has the per-item detail). `v0.7.0` implements true
512-px hi-res (Modes 5/6) output — `docs/ppu.md` §Hi-res (Modes 5/6) color-math precision has the
full mechanism (a genuine one-pixel-clock-delayed DAC pipeline, verified against ares' primary
source) and honest verification status (unit-test-proven, non-regression-proven; real-title
validation against Marvelous/SA-1 attempted and not achieved — the title never entered hi-res in
a 1200-frame headless run, and no working GUI environment was available to drive an `ares`
reference-screenshot comparison, both honestly tracked as open, not claimed done). The save-state
`FORMAT_VERSION` bumped `1`→`2` for this — its first real bump — closing the `v1.0.0` gate's
backward-compat-fixture gap early (`docs/adr/0006-save-state-format.md`'s bump log).
**Phases 1 (CPU + golden oracle)
and 2 (scheduler + video) are functionally complete** — the 65C816 passes the
SingleStepTests/65816 oracle to 0-diff (state + cycles), and the machine **boots and runs real
ROMs**: the master-clock lockstep scheduler + bus memory map + DMA/HDMA + the dual-chip PPU
produce a deterministic framebuffer. gilyon's on-cart CPU suite reports "Success" (all 1107
tests), and the undisbeliever PPU/DMA/HDMA suite renders bit-deterministic golden framebuffers.
Audio (Phase 3) is complete. Coprocessors (Phase 4/7): Core/Curated (DSP-1, Super FX, SA-1) plus
the BestEffort DSP-2/DSP-4/ST010, S-DD1, CX4, and OBC1 are implemented and validated against
real commercial ROMs (see the coprocessor matrix below); standalone S-RTC is now implemented
(`v0.4.0`, unit-tested only — no commercial dump in the local corpus); SPC7110 is implemented
with a confirmed, fixed addressing bug (`v0.4.0`: DROM/PROM reads used a plain modulo instead of
real hardware's block-mirror fold) that materially improved but did not fully resolve its boot
crash — see the coprocessor matrix below for the precise remaining failure; ST018 is now
implemented (`v0.4.0`, unit-tested only — no commercial dump in the local corpus).
Save-states (`v0.2.0`), rewind, run-ahead, PAL region auto-detection, and ExLoROM (all `v0.3.0`)
are implemented and shipped — see the frontend and memory-map-model tables below.

## Accuracy dashboard

RustySNES doesn't have one monolithic all-in-one oracle ROM the way RustyNES's AccuracyCoin does.
An early skeleton for exactly that approach exists (`rustysnes-test-harness::accuracy_battery`,
ticket T-04) but was never implemented and has since been superseded — no publicly available
SNES ROM plays the AccuracyCoin role, and the composed multi-suite approach below is what
actually shipped; that skeleton is tracked as dead code to remove in a follow-up, not a competing
source of truth. The accuracy story here is instead a **composed multi-layer battery** across
independently-sourced suites (`docs/testing-strategy.md`). Rather than force these heterogeneous
suites into one artificial summed fraction (a 5.12M-case CPU oracle would swamp a 4-ROM audio
suite in any raw sum, which would be misleading, not informative — this project's honesty-gate
posture, `docs/adr/0003`, applies to how numbers are presented too), each layer's own status is
tracked here, always current, reaffirmed every release:

| Layer | Status | Detail |
|---|---|---|
| CPU (65C816) per-opcode oracle | ✅ **0-diff vs. reference** | 5,119,999 / 5,120,000 (SingleStepTests/65816; the one residual is a documented inter-reference divergence, `docs/adr/0002`, not a bug — not literally 0 of 5,120,000, but 0 against the chosen reference behavior every other test vector agrees on) |
| SPC700 per-opcode oracle | ✅ **0-diff, 100.00%** | 256,000 / 256,000 (SingleStepTests/spc700) |
| On-cart CPU (gilyon `cputest-basic`) | ✅ **green** | 1107 / 1107 "Success" |
| PPU/DMA/HDMA golden framebuffer (undisbeliever) | ✅ **green, deterministic** | 29 / 29 ROMs bit-identical across runs |
| Audio boot+run (blargg `spc_*`) | ✅ **literal PASS, all 4** | `spc_smp`, `spc_timer`, `spc_mem_access_times`, `spc_dsp6` — asserted, not a determinism proxy |
| Core/Curated coprocessors (oracle-gated) | ✅ **3 / 3, honesty gate green** | DSP-1 (4 commercial ROMs), Super FX/GSU (58 Krom ROMs + per-opcode suite), SA-1 (18 commercial carts) — `ORACLE_COPROCESSORS` |
| BestEffort coprocessors, real-title validated | ✅ **6 / 9** | DSP-2, DSP-4, ST010, S-DD1, CX4, OBC1 — each boots a real commercial title to real gameplay content |
| BestEffort coprocessors, unit-test only | ⚠️ **3 / 9** | SPC7110 (wired against its one available ROM but doesn't reach a bootable screen — the addressing-bug fix in `v0.4.0` improved but didn't close this), ST018, S-RTC (neither has a commercial dump in the local corpus) |
| Determinism contract | ✅ **proven** | bit-identical framebuffer/audio across runs; save-state round-trip proven across all three board tiers (no-coprocessor, Curated, BestEffort) |

**Named residuals, tracked not hidden:** the 65816 `e1.e` divergence (`docs/adr/0002`); DSP-3 and
ST011 have no board wired (no verified board/window entry to pin against, `necdsp_variant.rs`);
SPC7110's post-`RTI` WRAM-population gap (`docs/cart.md` §SPC7110); PAL and ExLoROM both lack
golden-ROM-boot proof (no ROM in the local corpus for either); hi-res (Modes 5/6) output is
implemented and unit-verified (`v0.7.0`) but has no real-title validation — neither named
motivating commercial title (Bishoujo Janshi Suchie-Pai: no local dump; Marvelous/SA-1: dumped
but never observed entering hi-res in a 1200-frame headless run) has confirmed the mechanism
against actual hi-res game content, `docs/ppu.md` §Hi-res (Modes 5/6) color-math precision.
`v0.5.0 "Fidelity"` is where the next layer — a named hardware-gotcha regression suite (DRAM
refresh, HDMA mid-scanline placement, the DMA/HDMA-collision crash quirk, open-bus-via-HDMA-latch,
true mid-dot writes, hi-res color-math precision, the `$4203` double-write edge case) — gets
added to this table; hi-res color-math precision itself closed in `v0.7.0 "Resolution"`.

## Subsystem progress

| Crate | Chip | State |
|---|---|---|
| `rustysnes-cpu` | WDC 65C816 (5A22) | **Phase 1 complete — 65816 oracle 0-diff (state+cycles), all 256 opcodes × modes, native+emulation, REP/SEP/XCE** |
| `rustysnes-ppu` | PPU1 (5C77) + PPU2 (5C78) | **Phase 2 — BG 0-7 + Mode 7 + 128-sprite OAM + color math + windows + dot/HV timeline; per-scanline compositor (mid-line raster deferred; a fix for the mid-line HDMA-driven case is designed + SA-1-verified but NOT landed — blocked on an unexplained Super FX/GSU regression the same change causes, `docs/ppu.md` §Mid-scanline/HDMA-driven register timing). True 512-px hi-res (Modes 5/6, pseudo-hires) output landed `v0.7.0` — a genuine one-pixel-clock-delayed dual-column DAC pass mirroring ares' `PPU::DAC`, unit-verified + non-regression-verified, real-title validation still open (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision). Color fixes (ares pixel-diff vs SMW): color-math subscreen-backdrop addend = the COLDATA fixed color (blue-sky/black-bg fix), and the BG tilemap palette-group offset folded into the CGRAM index (washed multi-palette art fix); undisbeliever golden stays 29/29** |
| `rustysnes-apu` | SPC700 (S-SMP) + S-DSP + ARAM | **Phase 3 — SPC700 oracle 0-diff; S-DSP behavioral; integrated into the machine: the 4 `$2140-$2143` ports route through the real `Apu`, the integer-accumulator async resync clocks the SMP in **cycle-exact sub-instruction lockstep** (`68_352/715_909`, ADR 0004), SMP base-clock + timer + DSP rates ares-correct; blargg `spc_*` boot+upload+run bit-deterministically; the **timer-phase fix** (timebase/timers clocked before the write side effect, ares/Mesen2-correct) + the **DSP GAIN mode-7 threshold fix** (unsigned `hidden_env >= 0x600`, blargg/ares-correct) drive **all four `spc_*` (`spc_smp`/`spc_timer`/`spc_mem_access_times`/`spc_dsp6`) to literal `PASSED TESTS`** (asserted)** |
| `rustysnes-cart` | LoROM/HiROM/ExHiROM + coprocessors | **Phase 2 base map modes + Phase 4 coprocessors: chipset-byte detection, the shared µPD77C25/µPD96050 LLE engine + DSP-1 board (real DSP-1 games with user-supplied firmware), and the Super FX/GSU — full Argonaut RISC core (`coproc::gsu`) + `SuperFxBoard` (`coproc::superfx`), host-synced on the Go flag, boots the Krom GSU suite (`superfx_oncart`). SA-1 next** |
| `rustysnes-core` | Bus + master-clock scheduler + DMA/HDMA | **Phase 2 — master-clock lockstep (6/8/12 access map), full memory decode, CPU regs + mul/div, GP-DMA + HDMA, NMI/HV-IRQ** |
| `rustysnes-frontend` | egui shell + audio ring + pacing | **Phase 5 — PLAYABLE: native winit 0.30 + wgpu 29 + egui 0.35 + cpal shell boots real commercial ROMs with picture, sound, and control. Video: PPU BGR555→RGBA8 decode, aspect-correct (4:3) sub-rect letterbox blit. Audio: S-DSP 32 kHz FIFO → producer-side linear resampler (DRC-paced) → lock-free ring → cpal stereo stream. Input: keyboard + gilrs gamepad → `bus.set_joypad`. ROM load (+ coprocessor-firmware + `.srm` SRAM auto-load), Reset / Power-Cycle / Pause wired. wasm32 target compiles (winit/wgpu canvas path is a bootstrap scaffold). Proven by the `playable_smoke` headless gate (Super Mario World: 256×224 structured frame + 63,975 non-silent samples over 120 frames) + an xvfb launch run. Save-states landed (`docs/adr/0006`: a versioned `System::save_state()`/`load_state()` envelope across every `Board` + `Cpu`/`Ppu`/`Apu`/`Bus`, proven by a round-trip determinism test), wired to a quick-save menu slot; rewind (a bounded ring buffer of full snapshots, `crate::rewind::RewindBuffer`) and run-ahead (N-frame peek-and-discard, `crate::rewind::step_with_run_ahead`) landed in `v0.3.0 "Continuum"` — both config-driven and off by default (capacity/frames `0`), proven by tests that hand-assemble a tiny 65C816 program for a real per-frame state signal. Pacing/present fixes: wall-clock fixed-timestep drive (emulation tracks the region rate, not the display refresh — fixes the ~2–3× over-speed on high-refresh panels), a real smoothed FPS meter (was hardcoded `0.0`), and a live present-mode reconfigure on the Settings → Video toggle. `v0.8.0` T-81-003: a Tools → Cheats… window (Game Genie / Pro Action Replay, decode in `rustysnes_core::cheat`, applied via per-frame WRAM poke) behind the `cheats` flag, native + `wasm32`** |
| `rustysnes-netplay` | rollback netplay | not started |
| `rustysnes-cheevos` | RetroAchievements (opt-in FFI) | not started |
| `rustysnes-script` | Lua scripting / TAS API | **T-81-002 — sandboxed `mlua` 5.4 scripting (WRAM read/write + per-frame callback, runaway-loop instruction budget, `io`/`os`/`require`/`debug` denied) + a `rustysnes_core::movie` TAS format (deterministic input log, power-on or embedded-save-state start, replay-verified bit-identical vs a real committed ROM); behind the `scripting` flag, wired into a Tools menu, native-only** |
| `rustysnes-test-harness` | golden-log + JSON-oracle + screenshot baseline | **implemented and in active use** — the accuracy oracle (65816/SPC700 SingleStepTests runners, gilyon/undisbeliever/blargg golden-log gates, the `*_oncart` per-coprocessor commercial-ROM validation harnesses, and `commercial_screenshots.rs` the boot-screenshot generator behind `test-roms`/`commercial-roms`) |

## Accuracy — per-suite pass counts

| Suite | Layer | License posture | Pass | Total |
|---|---|---|---|---|
| SingleStepTests/65816 (JSON) | CPU per-opcode oracle | **self-gen committed + upstream cross-check (external)** — ADR 0005 | **5,119,999** | 5,120,000 |
| SingleStepTests/spc700 (JSON) | SPC per-opcode oracle | MIT (committable) | **256,000** | 256,000 (0-diff) |
| gilyon/snes-tests (cputest-basic .sfc) | CPU on-cart (boots on `System`) | MIT (committed) | **1107** | 1107 (= "Success") |
| undisbeliever/snes-test-roms (.sfc) | PPU/DMA/HDMA hardware (golden framebuffer) | Zlib (committed) | **29** | 29 (deterministic) |
| blargg `spc_*` (spc_dsp6 / mem_access / smp / timer) | cycle-accurate SPC/DSP (cycle-stepped S-DSP + timer-phase + GAIN mode-7 fixes) | unstated (external) | **4 boot+run det.; 4 literal PASS** | 4 (all → literal `PASSED TESTS` asserted: `spc_smp`/`spc_timer`/`spc_mem_access_times` via timer-phase fix, `spc_dsp6` via DSP GAIN mode-7 unsigned-threshold fix) |
| DSP-1 commercial dumps (`dsp1_oncart`) | DSP-1 coprocessor (boots on `System` w/ user firmware) | ROMs+firmware gitignored (golden committed) | **4 boot+det.** | 4 (detection + RQM-access + golden + firmware-diff) |
| Krom GSU suite (`superfx_oncart`) | Super FX/GSU coprocessor (boots on `System`) | CC0/homebrew (gitignored; golden committed) | **58 boot+live+det.** | 58 (SuperFx detect + GSU-executed + FillPoly-into-RAM plot pipeline + golden) |
| Commercial SA-1 carts (`sa1_oncart`) | SA-1 second-65C816 coprocessor (boots on `System`) | ROMs gitignored (golden committed) | **18 boot+det.; 10 SA-1-live** | 18 (Sa1 detect + S-CPU↔SA-1 traffic, all 18; aggregate SA-1-executed floor ≥8 — observed 10, incl. Super Mario RPG / both Kirbys / PGA Tour 96 / Power Rangers Zeo at millions of SA-1 cycles; deterministic golden) |
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
- **Accuracy battery (the composed multi-layer oracle):** see the "Accuracy dashboard" section at
  the top of this document for the always-current, per-layer status table — the CPU, SPC700,
  on-cart CPU, PPU/DMA golden, and audio layers are all green; the coprocessor layer is 6/9
  BestEffort boards real-title validated plus all 3 Core/Curated boards honesty-gate green.
- **Determinism contract:** the framebuffer is verified bit-identical across runs (same seed +
  ROM ⇒ identical hash) for all 29 undisbeliever ROMs; the full save-state round-trip (save,
  restore onto a fresh `System`, continue both, compare framebuffer + audio) is proven for a
  no-coprocessor ROM, a `Curated` Super FX ROM, and a `BestEffort` coprocessor ROM
  (`docs/adr/0006`, `save_state_determinism.rs`).

## Coprocessor / board tier matrix (honesty gate: `docs/adr/0003`)

`boards_tiered = true` — the honesty gate is real: **no BestEffort board backs the oracle.**

| Chip | Core | Tier | Shared LLE core | State |
|---|---|---|---|---|
| DSP-1/1A/1B | µPD77C25 | **Core/Curated** | µPD77C25 (shared, 6 chips) | **implemented** — full µPD7725 LLE engine (`coproc::upd77c25`) + `Dsp1Board` (Lo/HiROM DR/SR windows). Boots Super Mario Kart / Pilotwings / Super Bases Loaded 2 / Aim for the Ace on the full System with user-supplied `dsp1*.rom`; deterministic golden + firmware-differential + RQM-handshake access gate (`dsp1_oncart`, 4 ROMs). Honesty gate green (`ORACLE_COPROCESSORS` ∋ DSP). Firmware gitignored, never committed |
| Super FX / GSU-1/2 | Argonaut RISC | **Core/Curated** | — (cart ROM) | **implemented** — full GSU core (`coproc::gsu`: complete Argonaut RISC instruction set + ALT-mode machine, the multiplier, ROM/RAM buffers, opcode cache, the branch-delay pipeline, and the PLOT/RPIX pixel-plot pipeline) + `SuperFxBoard` (`coproc::superfx`: LoROM Super FX map, GSU register window, CPU↔GSU ROM/RAM arbitration). No chip dump — the GSU program is in cart ROM; host-synced on the Go flag (`run_until_stopped`, the DSP-1 `run_until_rqm` analogue), no core tick. Validated by `superfx_oncart` (58 Krom GSU ROMs: SuperFx detection + GSU-executed liveness + a FillPoly-into-RAM plot-pipeline assertion + deterministic golden) + the per-opcode `GSUTest` suite + engine unit tests. Honesty gate green (`ORACLE_COPROCESSORS` ∋ SuperFx) |
| SA-1 | 65C816 @ 10.74 MHz | **Core/Curated** | (reuses CPU core) | **implemented** — the full SA-1 system (`coproc::sa1::Sa1Board`: the `$2200–$23FF` register file, Super-MMC ROM banking, BW-RAM with the 2/4 bpp bitmap + linear projections + write-protect, 2 KiB I-RAM, the arithmetic unit, var-len bit unit, H/V timer, and normal + type-1/2 character-conversion DMA) + the **second 65C816** instantiated and stepped in `rustysnes-core` (deterministic master-clock catch-up via the `Board` second-CPU hooks — the crate graph keeps the CPU core out of the cart crate). No chip dump — the SA-1 program is in cart ROM. Validated by `sa1_oncart` (18 commercial SA-1 carts: detection + S-CPU↔SA-1 traffic for all 18, an aggregate "SA-1 CPU executed" liveness floor ≥8 — observed 10 incl. Super Mario RPG / both Kirbys / PGA Tour 96 / Power Rangers Zeo — + deterministic golden) + board unit tests. The main CPU oracle stays 0-diff (SA-1 stepping is gated to SA-1 carts and bounded by the untouched master clock). Honesty gate green (`ORACLE_COPROCESSORS` ∋ Sa1) |
| DSP-2 / DSP-4 | µPD77C25 | BestEffort | µPD77C25 (shared) | **implemented** — `coproc::necdsp_variant::NecDspVariantBoard` reuses the DSP-1 µPD7725 LLE engine, title-detected (`Variant::detect`). DSP-2 uses the generic bit-0 DR/SR split; DSP-4 needed a DSP-1-style half-boundary split instead (found + fixed against a real Top Gear 3000 boot-time 16-bit hardware check). Validated against real Dungeon Master (DSP-2) and Top Gear 3000 (DSP-4) — real title + gameplay content |
| ST010 / ST011 | µPD96050 | BestEffort | µPD96050 (shared) | **implemented** — same `NecDspVariantBoard`, µPD96050 DR/SR bit-0 split + the DP battery data-RAM window. Validated against real F1 ROC II — real title + gameplay content |
| S-DD1 | Nintendo ASIC | BestEffort | — | **implemented** — `coproc::sdd1`: Golomb-code + adaptive-binary-probability decompressor (`Decompressor`, ports ares' constant tables verbatim) streamed during fixed-address DMA via a new `Board::notify_dma_channel` hook (`rustysnes-core` snoops `$43n2-$43n6` writes). No chip dump — decompresses the cart's own compressed ROM. Validated against real Star Ocean and Street Fighter Alpha 2 — real title + gameplay content, after fixing a `u8`-shift-by-8 overflow bug in the codeword reader (ares' `n8` implicitly widens through C++ int promotion; the Rust port needed an explicit `u32` widen) |
| SPC7110 (+RTC-4513) | Hudson ASIC | BestEffort | — (frozen RTC) | **implemented, addressing bug fixed, still not booting to real content** — `coproc::spc7110`: DCU (Hudson adaptive binary range coder over 1/2/4bpp planes), data-port unit, ALU (16×16 multiply, 32/16 divide), memory-control unit, plus a paired `coproc::epsonrtc::EpsonRtc` (RTC-4513, seeded to a fixed epoch — real wall-clock time would break the determinism contract). Wired against the one available ROM (Far East of Eden Zero / Tengai Makyou Zero): header detection fixed (title is "TENGAI MAKYO" not "…MAKYOU"; the `$F`-custom chipset-nibble gate excluded RTC carts' `$F9` byte), and a `$40-$7D` HiROM-style ROM mirror added (confirmed needed — the game's own boot code executes there). **`v0.4.0`:** found + fixed a real addressing bug — `datarom_read`/`mcurom_read`'s PROM/DROM lookups used a plain `offset % len` fold instead of ares' `Bus::mirror` block-mirror algorithm (only equivalent when the buffer size is a power of two; the 6 MiB DROM here is not), silently returning the wrong byte for any register-selected offset past the physical chip size. Fixing it (`spc7110::bus_mirror`) pushed the wild-PC excursion from ~20-30 frames into boot to ~90+ frames and it now self-recovers via a BRK/RTI loop instead of crashing outright. Root cause of the REMAINING failure is narrowed but not fixed: the CPU eventually `RTI`s (from genuine PROM code, traced to `$CD:D2B3`) into WRAM (`$20:0848` = `$7E:0848` mirror) that is confirmed completely zeroed — code that should be resident there (likely via a DCU decompress-to-WRAM step) either hasn't run or produced nothing. Needs a real disassembler + symbol trace to finish; tracked as future work, not silently claimed fixed |
| CX4 | Hitachi HG51B169 | BestEffort/Curated | — | **implemented** — clean-room `coproc::hg51b` (HG51B S169 core: sequential mask/value opcode decode transcribed from ares' `pattern(...)` strings, register file, cache, DMA, suspend/wait state machine) + `Cx4Board`. No chip dump — the CX4 program runs from cart ROM; only a small 3 KiB data-ROM constant table (`cx4.rom`) needs external supply. Validated against real Mega Man X2 and Mega Man X3 — real Capcom copyright screens + real gameplay, after fixing a real bug where DMA/cache work triggered while the chip was halted never ran |
| OBC1 | simple ASIC | BestEffort | — (HLE) | **implemented** — `coproc::obc1::Obc1Board`: dedicated 8 KiB RAM, a reprogrammable cursor (`$1FF5`/`$1FF6`) over 4-byte slots + a packed 2-bit-per-slot status byte (`$1FF4`). Validated against real Metal Combat: Falcon's Revenge — real gameplay cinematic |
| ST018 | ARMv3 | BestEffort | — (separate ARM LLE) | **implemented (`v0.4.0`)** — `coproc::armv3`: a full ARMv3 (ARM6-class, pre-Thumb) CPU core (barrel shifter/condition-codes/ALU, mode-banked register file, 3-stage pipeline, the complete instruction set) + `St018Board` (a single combined `0x28000`-byte firmware dump, the `$3800`/`$3802`/`$3804` handshake registers, 16 KiB work RAM), driven by `Board::coprocessor_tick` rather than the SA-1 second-CPU hooks since this ARM core is entirely self-contained in `rustysnes-cart`. Detected via a title match on the confirmed real cart, Hayazashi Nidan Morita Shogi 2 — an earlier investigation wrongly assumed Star Ocean (which uses S-DD1 only). No commercial dump in the local corpus — unit-test-level coverage only, not golden-framebuffer validated |
| S-RTC | Sharp S-RTC | BestEffort | — (frozen time) | **implemented (`v0.4.0`)** — `coproc::sharprtc::SharpRtcBoard`: a standalone Sharp S-RTC (Daikaijuu Monogatari II, ExHiROM), a DIFFERENT chip/protocol from the Epson RTC-4513 paired with SPC7110 above — a 2-register (`$2800`/`$2801`) handshake over a 13-slot decimal clock file (`Ready -> Command -> Read`/`Write` state machine), seeded to a fixed epoch, never wall-clock-advanced. Wraps a base `ExHiRom` board. No commercial dump in the local corpus — unit-test-level coverage only, not golden-framebuffer validated; header detection is a best-effort title match, matching the existing CX4/SPC7110 posture |

One **µPD77C25 / µPD96050 LLE engine** covers DSP-1/2/3/4 + ST010/011 (six chips, one engine).

## Memory-map model support

| Model | Header offset | State |
|---|---|---|
| LoROM ($20) | $007FC0 | **Phase 2 — score-based header detect + ROM/SRAM decode + mirroring** |
| HiROM ($21) | $00FFC0 | **Phase 2 — detect + $C0+ linear + $00-3F:$8000 window + SRAM** |
| ExHiROM ($25) | $40FFC0 | **Phase 2 — detect + A23-inverted extended-bank decode** |
| ExLoROM | $407FC0 (unofficial, no dedicated `$FFD5` value) | **v0.3.0 — detect + A23-inverted LoROM-windowed decode (sourced from bsnes's runtime board database); no real-ROM validation, no ExLoROM dump in the local corpus** |

## Region timing

| Region | Master clock | Lines/frame | State |
|---|---|---|---|
| NTSC | 21.477270 MHz | 262 (+1 interlaced) | **implemented + validated** — the default region; every oracle/golden/commercial ROM in the accuracy battery boots and runs at this timing |
| PAL | 21.281370 MHz | 312 (+1 interlaced) | **implemented + auto-detected** — `Bus::sync_region_from_cart` reads the cart header's destination-code byte at `System::reset()` and reconfigures the PPU's line count/status bit accordingly (`rustysnes-core::scheduler` tests `pal_cart_auto_detects_pal_region_on_reset`/`ntsc_cart_auto_detects_ntsc_region_on_reset` prove this end-to-end, including a full 312-line frame actually completing). No PAL ROM is available in the local test corpus yet, so golden-framebuffer validation against a real PAL cartridge boot (the accuracy-battery-equivalent proof NTSC already has) remains open — tracked, not silently claimed |

Region is **data, not a build fork** (`docs/scheduler.md`). The differing NTSC/PAL master-clock
*rate* (Hz) is a real-world audio/video pacing concern the frontend owns (`docs/adr/0004`); the
core's master-clock counter is a pure tick count, so nothing else in the core depends on which
oscillator frequency a real console would use — only the PPU's line-count/status-bit timeline
is region-dependent here.

## Version policy

- Start at **v0.1.0**. Additive features ship behind **default-off feature flags** so the
  shipped / native / `no_std` / wasm builds stay byte-identical with the flags off.
- The **fractional-timebase refactor** is the one milestone expected to break byte-identity /
  save-state compatibility (`docs/adr/0002`) — and only happens if hard residuals warrant it.
- **Do NOT** import RustyNES engine-lineage "v2.0" anchors as RustySNES releases. The forward
  path is Phase 0 → v1.0.0 → (only then) the fractional-timebase refactor; see
  `to-dos/ROADMAP.md`.
