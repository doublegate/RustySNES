# DRAM Refresh

The S-CPU (5A22) stalls once per scanline to refresh work RAM. RustySNES models this as a fixed
per-line CPU stall. This document explains what the behaviour is, how it is modelled here, why the
project needs it, and — the question it most often raises — why turning it on shifts some games'
rendered frames by a few frames while leaving others pixel-identical.

The mechanism lives in `crates/rustysnes-core/src/bus.rs` (`DRAM_REFRESH_CLOCKS`, `DRAM_REFRESH_DOT`,
and the injection in `Bus::advance_master`). Timing constants and the scheduler contract are in
`docs/scheduler.md` §DRAM refresh; this file is the narrative rationale.

## What it is (the hardware)

The SNES's 128 KiB of work RAM is **dynamic** RAM: each bit is a charge on a capacitor that leaks and
must be periodically rewritten or lost. The 5A22 — which packages the WDC 65C816 core together with
the memory controller — services this by **stealing the bus once per scanline** for a refresh cycle.
While the refresh runs, the 65C816 core cannot execute: it is frozen exactly as if it were blocked on
a slow memory access.

The reference cores agree on the shape (ares `sfc/cpu/timing.cpp`, MesenCE
`SnesMemoryManager::IncMasterClock40`, fullsnes):

- the CPU is halted for **40 master clocks**,
- **once per scanline**, at **≈ master-clock 536** into the line (the "multiple of 8 nearest 536"),
- on **every** scanline — visible and vertical-blank alike.

40 master clocks is **10 dots** (a dot is 4 master clocks outside the two long dots at H 323/327).
Over an NTSC frame that is `40 × 262 = 10,480` master clocks the CPU never gets to use — about **2.9 %**
of the frame.

## How RustySNES models it — reallocation, not addition

The naive model — "charge an extra `advance_master(40)` per line" — was long believed to be wrong
because it would supposedly inflate every frame by ~10,480 clocks. That belief was itself wrong, and
understanding why is the key to the whole model.

**The master clock here is CPU-driven, and the frame length is PPU-fixed.** `Bus::advance_master`
increments the master clock and the PPU dot counter in lockstep (1:1), and `run_frame` runs until the
PPU rolls a full frame — which is *always* exactly 357,368 master clocks (262 lines × 1364). So the
frame's length in master clocks is fixed by the video counters, not by how much work the CPU does.

Injecting a 40-clock stall therefore does **not** lengthen the frame. Those 40 clocks advance the PPU
(and APU, and any coprocessor) toward the fixed 357,368-clock rollover, so `run_frame` simply
completes after the CPU has supplied ~10,480 **fewer** access-clocks. The frame stays 357,368 clocks;
the CPU just accomplishes less inside it. This is precisely what the real chip does — refresh does not
add time to a frame, it takes time *away from the CPU* out of the frame's already-fixed budget, like a
slow access.

Measured, this is exact: the steady-state per-frame master-clock delta is 357,368 ± instruction-
boundary quantization noise, **identical with the stall on or off** (see `docs/scheduler.md` §DRAM
refresh for the measurement). The earlier "adding it inflates the frame" objection measured frame
length — a quantity that is *insensitive* to refresh here — and read a null result as proof the stall
was unnecessary. It was measuring the wrong thing; so was the parallel claim that refresh had no
observable effect at all.

### The injection

In `Bus::advance_master`, on the single master-sub-tick that completes `DRAM_REFRESH_DOT - 1`
(dot 133, i.e. line-clock 536), the scheduler runs a nested `advance_master(DRAM_REFRESH_CLOCKS)`
guarded by an `in_refresh` re-entrancy flag. Because a scanline crosses dot 133 exactly once, the
stall fires exactly once per line with no per-line state to store — it is derived statelessly from the
dot position, so it needs no serialization and does not affect determinism or save-state format. Dot
134 sits well before `HDMA_RUN_DOT` (276) and both long dots (H ≥ 323), so the 10-dot injection never
crosses a scanline boundary or perturbs HDMA/long-dot alignment.

## Why the project needs it

Two reasons, the first of which is what surfaced the whole item:

1. **Dot-accurate mid-line raster timing.** A game that writes a PPU register from an H-IRQ handler
   partway down a scanline (raster bars, mid-frame palette/scroll tricks) lands that write ~10 dots
   later *when the interrupt-plus-ISR execution straddles line-clock 536*. Without refresh, RustySNES
   produced a flat H-IRQ→write latency; with it, the latency varies exactly as the reference
   (MesenCE) does. This is the behaviour the mid-line-raster cross-check (`scripts/raster_crossval/`,
   ADR 0014 Phase 4c) measured: the DRAW-cursor boundary moves by ~10 dots precisely when the ISR
   crosses the refresh point. AccuracySNES `B3.01` independently confirms the pause (its tight
   H-counter loop measures one interval ~10 dots longer than the rest).

2. **Hardware-faithful CPU pacing.** Without refresh the emulated CPU executed ~10,480 clocks/frame
   *more* work than a real SNES — it ran the game's code slightly too fast relative to the video
   clock. Modelling refresh brings the CPU's per-frame work budget into line with hardware.

## Why some games differ by a few frames, and others don't

This is the effect that a before/after screenshot pass makes visible, and it has a specific,
non-mysterious cause.

Most SNES games are **VBlank-paced**: each frame they do their work, wait for the VBlank NMI, then
advance exactly one logical step. If that were the whole story, a 2.9 %-slower CPU would change
*nothing* — the game still advances one step per VBlank, so at VBlank *N* it is in the identical
state either way. That is exactly why a **held or static screen renders pixel-for-pixel identically**
with refresh on or off: a screen that is not animating is independent of the CPU timeline entirely.

The differences arise only where game progress is **not** locked to the VBlank count:

- **Boot-time offset (the dominant cause).** At power-on a game does un-paced work — decompressing
  graphics, clearing RAM, timed init loops, waiting on DMA. With the CPU ~2.9 % slower, that boot
  work takes a slightly different *number of VBlanks* to finish, so the game reaches "title ready" or
  "attract demo starts" a few frames later. From then on both builds advance one step per VBlank in
  lockstep — but **permanently offset by those few frames.** A capture at a fixed frame number
  therefore shows the same animation a few frames apart.

- **Lag frames on CPU-heavy titles.** If a frame's work does not fit before VBlank, the game drops a
  frame (two VBlanks for one logical step). Refresh removes ~2.9 % of the CPU's budget, so a frame
  that *just* fit before can now spill over, adding a lag frame in one build but not the other and
  drifting the timelines further. Super-FX and other CPU-bound titles (Star Fox) accrue both this and
  the boot offset.

- **CPU-timed logic.** A game that seeds an RNG or times an effect from a cycle count rather than the
  VBlank count will diverge outright.

Every such difference is the **same animation at a slightly different instant**, reached because the
more-accurate (slower) CPU got there a few frames later — not different pixels on the same moment.
Determinism still holds absolutely: same seed + ROM + input ⇒ a bit-identical frame *within* each
build, so nothing here is random; the shift is pure timing phase.

### Which side is correct

The **refresh-on** timeline is the accurate one. A real SNES pays the same 10,480-clock/frame refresh
tax, so it reaches the attract demo at the same later instant refresh-on does; the refresh-*off*
frames were the ones running subtly ahead of hardware. This is why the affected framebuffer regression
goldens (the coprocessor boot captures, blargg, the timing test ROMs) are re-blessed rather than
treated as breakage: the shift is *toward* hardware, and the accuracy oracles that carry a
hardware/MesenCE-validated value (AccuracySNES scenes, the `inidisp_*` cross-checks) still agree.

## Known limitations

- **Position precision.** The stall fires at a fixed line-clock 536; hardware/ares vary the exact
  position by the CPU's 8-clock DMA-divider phase (`530 + 8 − dmaCounter`, i.e. 531–538). The residual
  is ≤ ~2 clocks (< 1 dot) and below the resolution of every gate that observes it.
- **Not the HDMAEN-latch-race gap.** The `undisbeliever` `hdmaen_latch_test` / `_2` ROMs render an
  84-row band where MesenCE renders 45 — a pre-existing RustySNES HDMAEN-vs-latch modelling gap that
  refresh nudges by a few rows but does not cause and does not fix. It is tracked as a separate
  follow-up.
- **Residual H-IRQ recognition offset.** After refresh, the mid-line raster boundary matches MesenCE's
  *variable* component (the 10-dot straddle jump) but retains a small constant ~4-dot offset — a
  separate H-IRQ-recognition/dispatch-latency modelling detail, not a refresh matter.
