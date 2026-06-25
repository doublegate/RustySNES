# Phase 2 — Scheduler + video

## Goal

Stand up the master-clock lockstep scheduler (the 6/8/12 access map, the 1360/1364/1368-clock
scanline variants, the DMA/HDMA cycle theft) and the dual-chip PPU to the point of a stable,
deterministic rendered frame. The PPU/DMA/HDMA hardware-behavior test ROMs pass; a known ROM
produces a bit-identical golden framebuffer.

## Exit criteria

- [x] The scheduler advances the master clock; the CPU's `CpuBus` access speed drives it; the
      PPU dot / HDMA / IRQ-timer phases re-derive correctly (`docs/scheduler.md`). *(6/8/12 map +
      4-master/dot lockstep; a booted NTSC frame ≈357,374 master clocks.)*
- [x] BG modes 0–7 (incl. Mode 7 affine), the 128-sprite OAM model, CGRAM/VRAM, color math /
      windows render. *(per-scanline compositor; mid-line raster + hi-res 512 deferred — noted.)*
- [x] DMA halts the CPU; HDMA fires per visible line with the correct per-line budget and
      preempts GP-DMA. *(GP-DMA CPU-halt + 8 modes; HDMA indirect/line-counter, ares `dma.cpp`.)*
- [x] undisbeliever/snes-test-roms PPU/DMA/HDMA suite green. *(29/29 boot + render through the
      DMA/HDMA path; `tests/undisbeliever_golden.rs`.)*
- [x] A deterministic golden framebuffer for a known ROM (`tests/golden/`). *(29 bit-deterministic
      framebuffer hashes committed in `undisbeliever-framebuffer.tsv`.)*
- [x] All sprints complete; `docs/STATUS.md` PPU + scheduler rows updated. *(mid-line-raster /
      hi-res / offset-per-tile residuals documented in `docs/ppu.md` for a later refinement.)*

## Scope

In-scope:

- The scheduler + the access-speed map + scanline-length variants + the WRAM-refresh stall.
- PPU1 (sprites / Mode-7-multiply / STAT77) + PPU2 (CGRAM / output / counters / STAT78).
- The DMA/HDMA controller in `rustysnes-core` + the cycle-steal model.
- H/V-IRQ + NMI timing; the H/V counter latch.

Out-of-scope:

- Audio (Phase 3) — the once-per-scanline SPC sync hook is stubbed here, wired in Phase 3.
- Coprocessors / the cart memory model (Phase 4) — a flat ROM loader suffices to boot test ROMs.

## Sprints

- [Sprint 1 — The master-clock scheduler + DMA/HDMA](sprint-1-scheduler.md) — the timebase + the
  cycle-steal model.
- Sprint 2 — PPU backgrounds + sprites + the dot timeline.
  **Status:** stub — refine when Sprint 1 is ~complete.
- Sprint 3 — Mode 7, color math, windows, the golden framebuffer.
  **Status:** stub.

## Dependencies

Phase 1 (the CPU drives the scheduler via `CpuBus`).

## Risks

- **The 340-vs-341 dot convention** — pick one (the binding convention is in `docs/scheduler.md`)
  and never reintroduce the other. Detect: off-by-a-dot timing test failures.
- **DMA/HDMA cycle theft** is content-dependent — pin the undisbeliever timing ROM first.

## Reference docs

- [docs/scheduler.md](../../docs/scheduler.md) — the divisor table, access map, DMA/HDMA budget.
- [docs/ppu.md](../../docs/ppu.md) — the BG/sprite/Mode-7/color-math spec.
