# Changelog

All notable changes to RustySNES will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **RustySNES integrates a cycle-accurate emulation engine.** Modeled after its predecessor `RustyNES`, this emulator is built on a master-clock-precise, lockstep-scheduled core targeting the Mesen2/ares accuracy bar. The entries below document the engine-internal milestones as this core is built and hardened.

## [Unreleased]

### Added

- **Phase 5 — Playable native frontend (`rustysnes-frontend`).** The always-on egui shell is now a
  working SNES emulator: a real commercial ROM boots in a window with picture, sound, and control.
  - **Video:** `EmuCore` decodes the PPU's 256×(224|239) 15-bit BGR555 framebuffer to RGBA8 each
    frame and uploads it to the wgpu streaming texture; the blit now samples only the live sub-rect
    and letterboxes to the 4:3 SNES display aspect via a small uniform (the prior skeleton sampled
    the whole oversized texture). The stale "PPU produces no pixels / cleared frame" path in
    `emu.rs` is replaced with the real present path.
  - **Audio:** a new additive S-DSP output FIFO (`Apu::drain_audio`, captured at the DAC-latch point
    in `dsp::echo27`) feeds a producer-side linear resampler (32 kHz → cpal device rate, DRC-paced)
    into the lock-free ring; the cpal callback now emits true stereo. The FIFO is pure
    instrumentation over already-emitted samples, so the deterministic audio contract is unchanged.
  - **Input:** keyboard (default SNES map) + gilrs gamepad late-latch into `Bus::set_joypad` for P1
    and P2.
  - **Cartridge UX:** ROM load resolves coprocessor firmware (DSP-1.. / CX4) from beside the ROM /
    a `firmware/` dir and auto-loads a `<rom>.srm` battery save; **Reset**, **Power-Cycle**, and
    **Pause** are wired to the core; a missing firmware dump surfaces a clear "supply it" message
    (the `docs/adr/0003` honesty posture).
  - **Dependency stack refreshed to the latest mutually-compatible tier:** egui / egui-wgpu /
    egui-winit **0.35**, wgpu **29**, winit **0.30** (winit 0.31 is beta-only and egui-winit 0.35
    pins to 0.30 — winit is the gating crate), directories **6**, wasm-bindgen **0.2.126** /
    web-sys · js-sys **0.3.103** / wasm-bindgen-futures **0.4.76**. Native **and**
    `wasm32-unknown-unknown` both build.
  - **Validation:** a `playable_smoke` integration test drives a staged commercial ROM through the
    same `EmuCore` path the GUI uses and asserts a structured (non-blank) frame **and** a non-silent
    audio stream (Super Mario World: 256×224 picture + 63,975 samples over 120 frames); it skips
    cleanly when no ROM is staged. The native binary was also launched headless under xvfb (clean
    init + run, no panic).
  - **Deferred:** save-states / rewind / run-ahead (need a core-wide deterministic snapshot across
    the `Board` trait + APU/Bus/System) and the full wasm browser frontend (the wasm entry point is
    a compiling bootstrap scaffold).

- **Phase 4 — SA-1 (second 65C816 + ASIC) coprocessor:** a clean-room
  `no_std`/`forbid(unsafe_code)` port of the SA-1 system (`rustysnes-cart::coproc::sa1::Sa1Board`,
  from ares' `sfc/coprocessor/sa1`, ISC) — the `$2200–$23FF` register file (SA-1 control/reset, the
  bidirectional S-CPU↔SA-1 IRQ/NMI/message lines + the S-CPU NMI/IRQ vector redirect), the Super-MMC
  ROM banking (CXB/DXB/EXB/FXB), BW-RAM (the shared battery RAM with the `$2224` 8 KiB S-CPU window,
  the `$40–$4F` linear image, the SA-1 2/4 bpp bitmap + linear projections, and the SWEN/CWEN/BWPA
  write-protect), 2 KiB I-RAM (SIWP/CIWP protect), the arithmetic unit (signed multiply / unsigned
  divide / cumulative-sum sigma), the variable-length bit unit, the H/V timer, and the normal +
  type-1/type-2 character-conversion DMA. The **second 65C816** is instantiated and stepped in
  `rustysnes-core` (the one-directional crate graph keeps the CPU core out of the cart crate): the
  scheduler owns an optional `sa1_cpu`, wires a `Sa1Bus` adapter to the new `Board` second-CPU hooks
  (`has_second_cpu` / `second_cpu_read|write` / `second_cpu_running` / `second_cpu_take_reset` /
  `second_cpu_poll_nmi|irq` / `second_cpu_tick`), and advances it in deterministic master-clock
  catch-up — so the SA-1 runs in parallel **without perturbing the main CPU** (the `cpu_oracle`
  stays 0-diff; SA-1 stepping is gated to SA-1 carts). `Board::irq_pending()` is now ORed into the
  bus IRQ line (the documented wiring), so the SA-1→S-CPU IRQ reaches the main CPU. `board::select`
  routes `Coprocessor::Sa1` (no chip dump — the SA-1 program is in cart ROM). Tier stays
  **Curated** and joins the honesty oracle set. Validated by the new `sa1_oncart` harness gate (18
  commercial SA-1 carts: detection + S-CPU↔SA-1 register traffic for all 18, an aggregate
  "the SA-1 CPU executed millions of cycles" liveness floor — Super Mario RPG, both Kirby titles,
  PGA Tour 96, Power Rangers Zeo, … — and a deterministic golden framebuffer) plus board unit tests.
- **Phase 4 — Super FX / GSU (Argonaut RISC) coprocessor:** a clean-room
  `no_std`/`forbid(unsafe_code)` port of the GSU core (`rustysnes-cart::coproc::gsu`) — the full
  Argonaut RISC: R0–R15 (R15 = PC) with the FROM/TO/WITH register-select prefixes, the
  ALT1/ALT2/ALT3 composite-mode machine, the ALU + signed/unsigned `mult`/`umult` and the
  `fmult`/`lmult` multiplier, the ROM buffer (ROMBR:R14 + busy/latency) and RAM buffer
  (RAMBR/RAMADDR + deferred-write latency), the 256-byte/32-line opcode cache, the 1-instruction
  pipeline that gives the GSU its branch delay slot, and the PLOT/RPIX pixel-plot pipeline (the
  two-deep pixel cache, the color/cmode logic with dither/freeze-high/high-nibble/transparent, and
  the SCBR/SCMR screen-base + 2/4/8 bpp character-format addressing) + the SFR status flags. Added
  `SuperFxBoard` (`coproc::superfx`): it owns the cart ROM + the Game Pak RAM (the GSU plot
  bitmap), decodes the LoROM Super FX CPU map (the `$3000–$32FF` register/cache window, the LoROM +
  linear ROM windows, the `$70–$71` + `$6000–$7FFF` RAM windows), and arbitrates the shared
  ROM/RAM bus (the snooze-vector / open-bus model while the GSU owns the bus). Unlike the DSP
  family there is **no chip-ROM dump** — the GSU program lives in the cartridge ROM — so the board
  is functional the moment the cart loads. Host↔GSU sync reuses the DSP-1 idea: the board runs the
  GSU **to completion the instant the CPU sets the Go flag** (`Gsu::run_until_stopped`, capped),
  byte-exact and deterministic with **no free-running core-scheduler tick**. `board::select` routes
  `Coprocessor::SuperFx` (the base board is never built — Super FX re-decodes its own map). New
  harness gate `superfx_oncart` (feature `test-roms`, self-skips when ROMs absent): boots the 58
  Krom GSU test ROMs (2/4/8 bpp PlotPixel/PlotLine/FillPoly + the per-instruction `GSUTest` suite)
  on the full System and asserts SuperFx detection, that the GSU actually executed its program out
  of cart ROM, that the `FillPoly` suites plot a substantial bitmap into the Game Pak RAM (the
  whole plot pipeline end-to-end at the cart boundary, PPU-independent), and a committed
  deterministic golden framebuffer. `mapper_tier_honesty` adds `SuperFx` to the oracle set and
  stays green (Super FX is the second `Core/Curated` coprocessor backing the oracle). Engine unit
  tests cover a hand-assembled `ibt`/`stop` program through the full host-sync path + the board
  ROM/RAM/register decode.
- **Phase 4 — DSP-1 (NEC µPD77C25) + the shared NEC DSP engine:** a clean-room
  `no_std`/`forbid(unsafe_code)` port of the NEC µPD77C25 / µPD96050 LLE core
  (`rustysnes-cart::coproc::upd77c25`) — the full DSP instruction set (OP/RT/JP/LD, the K×L
  signed-multiplier pipeline, dual accumulators + 6-flag condition sets, the 16-deep stack,
  program/data ROM + data RAM, and the DR/SR/DP host ports), revision-parameterized so one engine
  backs DSP-1/2/3/4 + ST010/011 (six chips). Added the `Dsp1Board` (`coproc::dsp1`) wrapping a base
  LoROM/HiROM board and intercepting the DR/SR window, with the snes9x/ares-equivalent window
  selection (HiROM `$00–$1F:$6000–$7FFF`; LoROM ≤1 MiB `$30–$3F:$8000–$FFFF`; LoROM >1 MiB
  `$60–$6F:$0000–$7FFF`). Header detection now decodes the coprocessor from the `$FFD6` chipset
  byte, and `board::select` routes `Coprocessor::Dsp` to the DSP-1 board. New
  `Cart::install_coprocessor_firmware` + `Board::load_firmware` hook: the µPD77C25 is **inert until
  the user supplies the `dsp1.rom` / `dsp1b.rom` chip dump** (gitignored, never committed —
  `docs/adr/0003`), never silently degraded. Host↔chip sync is the RQM-handshake catch-up
  (`run_until_rqm`) — byte-exact at the bus boundary and fully deterministic, no core-scheduler
  hook. New harness gate `dsp1_oncart` (feature `test-roms`, self-skips when ROMs/firmware absent):
  boots Super Mario Kart / Pilotwings / Super Bases Loaded 2 / Aim for the Ace on the full System,
  asserts DSP detection, the RQM-handshake access count on both the LoROM and HiROM windows, a
  committed deterministic golden framebuffer, and the firmware-differential (the Mode-7 titles
  render differently with the chip installed). Engine unit tests cover the decode/ALU/multiplier
  via a hand-assembled synthetic firmware. The `mapper_tier_honesty` gate stays green (DSP-1 is the
  first real `Core/Curated` coprocessor backing the oracle).
- **Phase 3 — Audio (SPC700 + S-DSP + ARAM):** a clean-room SPC700 (S-SMP) core driving the
  SingleStepTests/spc700 oracle to **0-diff — 100% state + cycle count over all 256 opcodes**
  (12,800 committed-sample tests in-tree; 256,000 in the full external tier). Full 256-opcode
  set with every addressing mode, `MUL`/`DIV`, the word ops, `DAA`/`DAS`, the bit-manipulation
  family, and `STP`/`SLEEP`. Added the S-DSP (8 voices, BRR decode, 4-point Gaussian
  interpolation, ADSR/GAIN envelopes, noise + PMON, KON/KOFF/ENDX edges, the 8-tap echo FIR +
  feedback, MVOL/EVOL, 32 kHz stereo mix), the 64 KiB ARAM, the three timers, the four
  `$2140-$2143` communication-port latches, and the IPL boot ROM. New `Apu` API
  (`tick`/`step_instruction`/`run_cycles`/`cpu_read_port`/`cpu_write_port`/`sample`) for the
  core to wire the bus ports + async resync. New oracle `tests/spc700_oracle.rs` (gated behind
  `test-roms`, self-skips when data absent). `#![no_std]` + `forbid(unsafe_code)`; bare-metal
  `thumbv7em-none-eabihf` build green. See `docs/apu.md` §Implementation status.
- **Phase 3 — APU↔machine integration + the deterministic async resync (T-31-002/T-31-003):**
  wired the four `$2140-$2143` CPU↔APU ports through the real `Apu` (`cpu_read_port` /
  `cpu_write_port` — the one-way latches; removed the dead `apu_ports` latch array), so the CPU's
  IPL upload handshake now reaches the SPC700. The SPC700/S-DSP advance in **integer-accumulator
  lockstep** with the 21.477 MHz master clock at the exact reduced ratio `68_352 / 715_909`
  (= `(apuFrequency/12) / NTSC-master`, no floating point — determinism, `docs/adr/0004`), so a
  CPU port read observes every SMP write up to that master instant. Corrected the SMP internal
  timebase to the ares base clock (`apuFrequency/12 ≈ 2.05 MHz`, `SMP_WAIT = 2` base clocks per
  access matching ares `cycleWaitStates[0]`, S-DSP one 32 kHz sample every 64 base clocks),
  replacing the earlier 1-unit/access approximation that ran the timers + DSP off-rate. New
  `Apu::advance_smp_cycle` (port-preserving lockstep advance) + `smp_pc`/`smp_stopped` debug
  accessors. Added `tests/blargg_spc.rs` (gated behind `test-roms`, self-skips when the
  external-tier ROMs are absent): the four blargg `spc_*` ROMs **boot, drive the IPL upload
  handshake to completion, and execute the uploaded SPC700 program bit-deterministically**
  (framebuffer + ARAM + ports hashed identical across runs) against the committed baseline
  `tests/golden/blargg-spc.tsv`.
- **Phase 3 — cycle-exact SMP↔CPU lockstep (T-31-004):** `Apu::advance_smp_cycle` now releases
  **exactly one SMP base clock per call** by draining a recorded micro-op timeline of the in-flight
  SPC700 instruction (one entry per bus access, with each SMP→CPU port write **deferred to the
  precise base cycle** its access completes). The new `RecordingSmpBus` runs the *unchanged*
  `Spc700::step` and applies every side effect byte-for-byte as the per-instruction `SmpBus` does —
  so the SPC700 oracle stays **0-diff (100%)** — while emitting the timeline. This is the
  ares/bsnes cooperative-thread interleaving achieved single-threaded (no coroutines, so save-state
  / netplay stay bit-deterministic). With it, **all four blargg `spc_*` ROMs now boot, upload, run,
  and stream their detailed result grids** (previously all stalled at "Running tests:");
  `tests/blargg_spc.rs` was upgraded to **decode and report the real on-screen verdict** (blargg's
  BG-tilemap header at `$0400` + result grid at `$0800`), keeping — not weakening — the
  deterministic + baseline-hash assertion (baselines re-blessed for the new timing). A literal
  blargg PASS is **still not earned**: every ROM streams its grid and reports **Failed 02**
  (`spc_smp` after the CPU-Instructions + CPU-Timing opcode grid; `spc_dsp6` after the Echo +
  Envelope list; `spc_timer` / `spc_mem_access_times` likewise). The residual is a sub-cycle
  interleave skew intrinsic to the **CPU-leading** clock model: ares/bsnes use a *symmetric*
  cooperative-thread model (either chip may lead, the other catches up at its port access), which
  would require a CPU↔SMP bus-master inversion out of scope for an APU-only change. Documented
  honestly in `docs/apu.md` §cycle-exact / `docs/scheduler.md` / `docs/STATUS.md` — reported, not
  faked. **(Superseded — see "Fixed: SPC700 timer clocking phase" below: the `spc_smp`/`spc_timer`/
  `spc_mem_access_times` residual was the recording-bus write phase, not a clock-model asymmetry, and
  all three now reach a literal PASS.)**
- **Phase 3 — cycle-accurate (cycle-stepped) S-DSP (T-31-005):** decomposed the S-DSP's monolithic
  per-sample `voice_pipeline` into the nine per-voice steps (`voice1..voice9`, with `voice3a/b/c`),
  the echo path (`echo22..echo30`), and `misc27..misc30`, scheduled on the **32-entry ares phase
  table** (`sfc/dsp/dsp.cpp::main`, ISC) via a new `Dsp::tick` (one phase per call; voices
  interleaved, voice 0 wrapping the sample boundary, DAC latched at phase 27 / `echo27`). The
  integration (`Apu` `step_instruction` + `RecordingSmpBus::record`) now drives the DSP **one tick
  per 2 SMP base clocks** (32 ticks = one 32 kHz sample) instead of a whole sample per 64 clocks, so
  an SMP instruction that reads a DSP register (`$F3`) mid-execution sees **cycle-correct
  sub-sample** OUTX/ENVX/ENDX/envelope state. `Dsp::run_sample` is retained as the batched
  `32 × tick` wrapper; a new guard test (`run_sample_equals_32_ticks_with_brr_content`) asserts the
  batched vs one-at-a-time drives are bit-identical (sample stream + ARAM) on real BRR/echo content.
  **Empirical outcome:** the cycle-accurate DSP *isolated* the residual blargg gap — `spc_smp` /
  `spc_timer` / `spc_mem_access_times` are now **byte-for-byte identical** to the per-sample build
  (DSP granularity was provably **not** their blocker; their residual is the CPU-leading clock-model
  asymmetry above), while `spc_dsp6` (the DSP-register-reading member) changed — more Echo/Envelope
  timing resolves — but still reports **Failed 02**. SPC700
  oracle stays **0-diff**, undisbeliever framebuffer golden **29/29**, all DSP unit + APU
  integration tests green, `#![no_std]` + `forbid(unsafe_code)` preserved. See `docs/apu.md`
  §cycle-accurate DSP. **(Superseded for the three timer-mechanism ROMs — see "Fixed: SPC700 timer
  clocking phase" below: `spc_smp`/`spc_timer`/`spc_mem_access_times` now reach a literal PASS;
  `spc_dsp6` remains Failed 02 on the S-DSP residual.)**
- Initial workspace scaffold (cycle-accurate emulator architecture, ported from RustyNES).
- Seeded `tests/roms/` with the permissive corpora — gilyon (MIT), undisbeliever (MIT/Zlib),
  and a deterministic SingleStepTests/spc700 (MIT) sample — plus the gitignored `external/`
  tier (65816, full spc700, 240p, Krom, blargg-spc). Curated the commercial-ROM coverage
  manifest (`tests/roms/commercial-corpus.json`) to popularity-weighted, genre-diverse beloved
  titles (metadata + SHA-256 only; no ROM bytes committed).
- **Phase 2 — cartridge base-mapper memory model: real SNES internal-header detection
  (copier-prefix strip + scored `$7FC0`/`$FFC0`/`$40FFC0` candidate selection on
  checksum+complement, map-mode, reset-vector, and printable-title heuristics) and working
  LoROM / HiROM / ExHiROM address decode backed by owned `rom`/`sram` storage.** `Cart::load`
  builds the board with the stripped ROM bytes + a zeroed header-sized SRAM; `read24`/`write24`
  route through the decode over real memory (ROM read-only, SRAM read/write, hardware-accurate
  non-power-of-two ROM mirroring). Added `save_sram`/`load_sram` battery accessors. Coprocessors
  remain stubs (Phase 4).
- **Phase 2 — dual-chip PPU (PPU1 5C77 + PPU2 5C78):** VRAM/CGRAM/OAM + the full `$2100-$213F`
  register file (with the BG-offset / Mode-7 / scroll write latches, VMAIN remap + increment,
  CGRAM/OAM/VRAM read-prefetch quirks, MPYL/M/H multiply, SLHV/OPHCT/OPVCT, STAT77/78); BG modes
  0-7 tile fetch (2/4/8 bpp, per-mode priority, 16×16 tiles, mosaic); Mode 7 affine
  (matrix/center/wrap/flip, EXTBG); the 128-sprite OAM pipeline (32-sprite range / 34-tile time
  limits, reverse-order fetch); color math (add/sub/half, fixed/sub addend, direct color); and
  windows (OR/AND/XOR/XNOR). Per-scanline compositor; mid-line raster + hi-res 512 deferred.
- **Phase 2 — master-clock lockstep scheduler + bus + DMA/HDMA:** the master clock advances
  through the CPU's bus accesses on the **6/8/12 region access-speed map** (ares `CPU::wait`,
  `$420D` MEMSEL FastROM), stepping the PPU dot clock (4 master/dot) + the SPC accumulator in
  lockstep. Full 24-bit memory decode (WRAM + low-mirror, PPU/APU B-bus, controllers, the
  CPU registers `$4200-$421F` incl. the multiply/divide unit, the DMA registers, cart routing).
  The 8-channel **GP-DMA** (CPU-halt, 8 transfer modes) and **HDMA** (per-line budget, indirect
  tables, mode lengths `{1,2,2,4,4,4,2,4}`) clean-room from ares `dma.cpp`. NMI + the RDNMI
  VBlank flag + the H/V-IRQ comparator. The `System` boots a cart from its reset vector and runs
  deterministic frames (an NTSC frame ≈357,374 master clocks).
- **Phase 2 — verified on-cart:** gilyon `cputest-basic.sfc` boots and reports "Success" (all
  1107 65C816 tests; `tests/gilyon_oncart.rs`) — closing Phase 1's deferred on-cart criterion;
  the 29 undisbeliever PPU/DMA/HDMA ROMs render bit-deterministic golden framebuffers matching
  `tests/golden/undisbeliever-framebuffer.tsv` (`tests/undisbeliever_golden.rs`).
- **Phase 1 — CPU + golden oracle: the WDC 65C816 core passes the SingleStepTests/65816
  per-opcode oracle to 0-diff** on architectural state **and** per-instruction cycle count
  (5,119,999 / 5,120,000 = 100.00% across all 512 opcode files × 10,000 tests, native +
  emulation). All 256 opcodes × addressing modes, `REP`/`SEP` width changes, and `XCE`
  emulation/native transitions verified.

### Fixed

- **65C816 memory-access timing is now cycle-exact (ares `CPU::read`/`write` phasing).** A CPU cycle
  used to perform its bus access *first* and advance the master clock *after*, so a register write
  landed a full cycle (6/8/12 master clocks) too early relative to the PPU/HDMA. The CPU now asks the
  bus for the access cost (`Bus::access_cycles`, ares `wait`) and sequences the advance
  (`Bus::advance`, ares `step`) around the access: a **write** advances the whole cycle then stores
  (lands at the cycle end), a **read** advances cost−4, reads, then advances 4 (lands four clocks
  before the end). Instruction cycle *counts* are unchanged (the CPU-timing tables still pass); only
  the sub-cycle phase at which each access becomes visible to the PPU/HDMA moves to the hardware-exact
  instant. `superfx-framebuffer.tsv` re-blessed for the re-phased GSU concurrency (Star Fox fly-in
  ship + planet verified still rendering); undisbeliever stays 29/29. Note: the `hdmaen_latch` band
  pattern still differs from ares because HDMA is serviced at the scanline boundary rather than at
  its dot-accurate hcounter — a separate limitation tracked independently.
- **Star Fox fly-in now renders correctly (Super FX) — ship and planet.** Four coupled fixes across
  the DMA/HDMA, PPU, cart, and CPU paths, all validated against ares:
  - **HDMA during GP-DMA (missing ship segment).** `Bus::run_gp_dma` takes the `Dma` out of the bus,
    so while a framebuffer GP-DMA ran, HDMA (Star Fox's per-line force-blank) was dormant and the
    DMA's tail lines dropped. The taken `Dma` now drives HDMA itself at scanline crossings via
    `Dma::service_hdma_line` / `service_hdma_during_gp` (new `DmaBus` scanline hooks); a
    frame-crossing framebuffer DMA no longer drops writes.
  - **HDMA setup/reset faithfulness.** `hdma_setup` sets `hdma_do_transfer` for every channel before
    the enable-check and `service_hdma_line` runs `hdma_reset` unconditionally at frame start
    (matching ares `Channel::hdmaSetup` / `timing.cpp`), so a channel enabled mid-frame reactivates.
  - **Mode-2 offset-per-tile (missing planet).** The planet is a mode-2 OPT BG2 layer, not a GSU
    object; `render_bg` ignored OPT so its columns never scrolled in. Implemented mode-2/4/6 OPT
    (`bg3_opt_tile` + per-column `world_x`/`world_y` override), transcribed from ares
    `background.cpp` — a general accuracy improvement for any OPT-using game.
  - **Super FX CPU→Game Pak RAM writes are unconditional.** `SuperFxBoard::write24` no longer gates
    RAM writes behind GSU ownership (reads still return open bus), matching ares `CPURAM::write`.
  - **65C816 `WAI` wakes on any asserted interrupt line** regardless of the `I` flag (WDC datasheet);
    a masked-IRQ `SEI; WAI` sync primitive no longer hangs.
  - Goldens re-blessed for the intentional behavior change: `superfx-framebuffer.tsv` (Super FX
    corpus now plots structured framebuffers) and the two `hdmaen_latch` entries in
    `undisbeliever-framebuffer.tsv` (HDMA now executes instead of a blank screen). undisbeliever
    stays 29/29; `superfx_oncart` passes. Exact `hdmaen_latch` band-parity with ares additionally
    needs cycle-exact 65C816 write timing and is tracked separately.
- **PPU color math — subscreen-backdrop addend is the fixed color (washed/black backgrounds).**
  When "add subscreen" (CGWSEL $2130 bit 1) is enabled but the subscreen pixel at a column is the
  backdrop (no opaque sub-layer wrote it), the color-math addend must be **COLDATA's fixed color**,
  not CGRAM[0]. `compose_dac` (`crates/rustysnes-ppu/src/render.rs`) used `layer_color(&bp)`, which
  returns CGRAM[0] (black) for a transparent subscreen pixel, so SMW's blue title sky (painted by
  the fixed color over a black main backdrop) rendered **black**. Now the addend falls back to the
  fixed color when the subscreen is transparent, and the half is suppressed for that pixel —
  matching ares `DAC::above` (`io.blendMode && math.transparent ⇒ addend = fixedColor()`). Verified
  by an ares (ISC) framebuffer pixel-diff on Super Mario World: the title sky now reads the fixed
  color (BGR555 `0x7393` ⇒ light blue) instead of `0x0000`.
- **PPU background palette-group offset (washed multi-palette BG art).** A BG tilemap entry's 3-bit
  palette group (bits 12–10) was fetched but dropped from the CGRAM index, collapsing every BG tile
  onto palette group 0 — the SMW logo and brick border rendered as flat grey/cream instead of their
  per-letter colors. `render_bg` now computes `paletteBase + (group << bpp) + color` (masked to a
  byte; 8bpp ignores the group; `paletteBase = id<<5` only in Mode 0), per ares `background.cpp`.
  The ares pixel-diff confirms the title logo/border colors now match. **undisbeliever golden stays
  29/29** (no re-bless — none of those ROMs exercise a hash-affecting non-zero BG palette group or
  subscreen-backdrop math); the PPU stays `#![no_std]` + `forbid(unsafe_code)`.
- **Frontend pacing — emulation ran at the display refresh, not the region rate (~2–3× too fast).**
  The synchronous (default) drive stepped exactly one emulated frame per winit `RedrawRequested`,
  i.e. once per display vsync, so a 144 Hz monitor ran the emulator 2.4× too fast. The present path
  now drives emulation from a wall-clock **fixed-timestep accumulator** (`app::Pacer`): `run_frame`
  runs only once `1 / region.frame_rate()` seconds of real time have accrued (NTSC 60.0988 /
  PAL 50.0070 Hz), the latest framebuffer is presented in between, catch-up after a stall is capped
  to avoid a spiral of death, and the present mode now governs **only** vsync/tearing. Unit-tested
  to hold ~60 fps across 30/60/75/144/240 Hz present rates (`pacing_tracks_region_rate_not_present_rate`).
- **Frontend FPS counter always read `0.0`.** `ShellInfo::fps` was hardcoded to `0.0`; the new
  `Pacer` measures the emulated-frame rate over a 0.5 s window and feeds the status bar.
- **Frontend Settings → Video present-mode toggle did nothing.** The radio wrote
  `config.video.present_mode` but the wgpu surface was only configured once at startup. The present
  path now detects the change and calls the new `Gfx::set_present_mode`, which re-validates against
  the surface's supported modes (falling back to `Fifo`) and reconfigures the live surface.
- **S-DSP GAIN mode-7 threshold (blargg `spc_dsp6` literal PASS, T-31-007):** `Dsp::envelope_run`
  compared the voice's internal envelope latch (`env_internal`) against the bent/two-slope
  `GAIN`-increase threshold `0x600` with a **signed** test, where blargg `SPC_DSP`
  (`(unsigned) hidden_env >= 0x600`) and ares (`(u32) _envelope >= 0x600`) use an **unsigned** one.
  The latch is the pre-clamp envelope; a preceding `GAIN` *decrease* mode (4 linear / 5 exponential)
  can leave it **negative**, and the unsigned reinterpretation makes that trip the reduced `+0x08`
  slope — a signed compare misses it and over-increments by `+0x20`. This was the sole divergence
  behind `spc_dsp6`'s `Envelope/gain $E0 threshold` → **"Failed 02"**. Cast the latch to `u32` for
  the comparison (`crates/rustysnes-apu/src/dsp.rs`), matching both references; the rest of the
  envelope path was already bit-identical to ares (verified by an all-`GAIN`-value differential).
  **`spc_dsp6` now reaches blargg's literal `PASSED TESTS`** (rendered at `$0800` row 30 near frame
  8.8k), so **all four blargg `spc_*` ROMs are now asserted to PASS** in `tests/blargg_spc.rs`
  (`screen_text` widened to the full 32×32 nametable, `VERDICT_FRAMES` raised to 12000). The quirk
  fires only deep in the Envelope suite, so no ROM's 120-frame boot hash moves (baseline TSV
  unchanged). undisbeliever golden stays 29/29, SPC700 oracle **0-diff**; `#![no_std]` +
  `forbid(unsafe_code)` preserved. See `docs/apu.md` §DSP GAIN mode-7 threshold.
- **SPC700 timer clocking phase (blargg `spc_*` literal PASS, T-31-006):** `RecordingSmpBus::write`
  — the bus the integrated machine drives through `Apu::advance_smp_cycle` — applied the write side
  effect (`$F0` global-enable / `$F1` enable / `$FA-$FC` target / the store) **before** advancing the
  SMP timebase and clocking the three timers. ares (`SMP::step`) and Mesen2 (`Spc::Write` →
  `IncCycleCount` first) clock the timers **before** the store, and our own per-instruction
  `SmpBus::write` already did so — but the recording bus was reversed, shifting the timer phase by
  **one access** on every timer-register write (e.g. arming `target` was observed *before* the
  arming cycle's own clock instead of after, so `TnOUT` lagged hardware by an off-by-one in the stage
  accumulation). Reordered `record()` (timebase + timer clock) to run first, then the store + IO
  decode (the deferred SMP→CPU port latch still rides that access's micro-op, so the CPU↔SMP
  handshake timing is unchanged). With the phase corrected, **`spc_smp`, `spc_timer`, and
  `spc_mem_access_times` reach blargg's literal `PASSED TESTS`** — `tests/blargg_spc.rs` now
  **asserts** the literal PASS (no longer determinism-only reporting); their re-blessed baselines are
  in `tests/golden/blargg-spc.tsv`. `spc_dsp6` is **unchanged** by the fix (its observable state is
  byte-identical) and still reports **Failed 02** on a separate S-DSP echo/envelope residual,
  reported honestly. This supersedes the earlier "literal PASS pending a CPU↔SMP bus-master
  inversion" conclusion above — the residual was the recording-bus write phase, not a clock-model
  asymmetry. SPC700 oracle stays **0-diff** (it replays against a flat, timer-less bus);
  `#![no_std]` + `forbid(unsafe_code)` preserved. See `docs/apu.md` §timer phase.
- `MVN`/`MVP` (block move): address now uses the full 16-bit `X`/`Y` regardless of index width
  and the increment respects the `X` flag (8-bit keeps the high byte); the `A.w` loop test is a
  post-decrement (ares `instructionBlockMove`). The oracle harness re-steps these looping
  instructions to the recorded cycle budget.
- `JSL`/`RTL`: stack access uses the full-16-bit `S` "new" push/pull (`pushN`/`pullN`) so it no
  longer corrupts `S` on an emulation-mode page wrap; the page-1 confinement is re-applied at
  the instruction boundary (ares `CallLong`/`instructionReturnLong`).
