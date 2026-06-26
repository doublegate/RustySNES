# Scheduler — RustySNES

**References:** `ref-docs/research-report.md` §§1, 2, 3, 5; `ref-docs/2026-06-24-ppu.md` §§4–5;
`ref-docs/2026-06-24-apu.md` §2; `docs/adr/0001`, `docs/adr/0002`.

## Purpose

The scheduler is the timebase that every chip phase derives from. It owns the single
**21.477270 MHz NTSC** (PAL **21.281370 MHz**) master clock and advances the CPU, PPU, DMA,
and HDMA in lockstep on it, while the SPC700 / S-DSP run in a **second, asynchronous clock
domain** resynced on demand. This is the central architectural choice (`docs/adr/0001`) and
the reason mid-instruction events work without per-quirk patches.

## The master-clock model (not pure dot-lockstep)

The NES CPU is always PPU÷3, so RustyNES gets away with integer dot-lockstep. The SNES
**cannot**: per `ref-docs/research-report.md` §1, a CPU cycle is **6, 8, or 12 master clocks
depending on the address region accessed and the FastROM bit `$420D.0`**, and per
`ref-docs/2026-06-24-ppu.md` §4 the dot and scanline lengths vary. So we model the **master
clock directly** and re-derive every chip's phase from it — a fractional-master-clock
timebase, which is why `docs/adr/0002`'s refactor is designed in from day one rather than
retrofitted.

`tick()` advances the master clock; the CPU bus returns the access *speed* (6/8/12) for each
memory cycle, the scheduler advances that many master ticks, then re-evaluates the PPU dot,
HDMA, and IRQ-timer phases. This is lockstep, not catch-up.

## The memory-access-speed map (master clocks per CPU access)

Per `ref-docs/research-report.md` §1 (sources: Fullsnes, SNESdev Timing, Super Famicom Dev
wiki):

| Address region | Banks | Speed | Cycles | Notes |
|---|---|---|---|---|
| System / WRAM mirror `$0000–$1FFF` | $00–$3F, $80–$BF | Slow | **8** | low 8 KiB WRAM mirror |
| PPU/APU/B-bus I/O `$2100–$21FF` | $00–$3F, $80–$BF | Fast | **6** | PPU1/PPU2 + APU ports |
| Old-style joypad `$4016/$4017` | $00–$3F, $80–$BF | XSlow | **12** | controller serial ports |
| CPU/DMA registers `$4200–$5FFF` | $00–$3F, $80–$BF | Fast | **6** | DMA, NMITIMEN, MEMSEL |
| Expansion `$6000–$7FFF` | $00–$3F, $80–$BF | Slow | **8** | cart-dependent |
| ROM `$8000–$FFFF` (WS1) | $00–$3F | Slow | **8** | always 8 |
| ROM `$8000–$FFFF` (WS2) | $80–$BF, $C0–$FF | MEMSEL | **6 or 8** | 0=8 (Slow), 1=6 (Fast) |
| WRAM `$7E0000–$7FFFFF` | $7E–$7F | Slow | **8** | full 128 KiB work RAM |
| Internal cycle (no bus access) | — | Fast | **6** | always 6 |

Resulting effective CPU frequencies: 6 → **3.58 MHz**, 8 → **2.68 MHz**, 12 → **1.79 MHz**.
`MEMSEL $420D` bit 0: `0 = SlowROM (2.68 MHz)`, `1 = FastROM (3.58 MHz)` for WS2 ROM
($80–$FF).

## The divisor table (the scheduler's core constants)

Per `ref-docs/research-report.md` §1:

| Chip | Advances per | Rate | Notes |
|---|---|---|---|
| Master clock | 1 tick | 21.477270 MHz | the finest practical quantum |
| 65C816 (5A22) | 6 / 8 / 12 master clocks | 3.58 / 2.68 / 1.79 MHz | **variable per access** (map above) |
| PPU dot | **4 master clocks** (nominal) | ~5.37 MHz | with long-dot exceptions (below) |
| SPC700 (S-SMP) | **separate ~1.024 MHz** | ~1.024 MHz | **asynchronous** — own resonator (§async) |
| S-DSP | 24.576 MHz resonator ÷768 | 32000 Hz sample | 1 stereo sample / 768 resonator cycles |

The SPC700 is **not** a master-clock divisor — it is a second clock domain. The two source
frequencies the accumulator math needs are **24,576,000 Hz** (SMP resonator) and
**21,477,272 Hz** (main master).

## Video timing — dots, scanlines, frames

Per `ref-docs/research-report.md` §2 and `ref-docs/2026-06-24-ppu.md` §4:

- **Normal scanline = 1364 master clocks = 341 dots.** The invariant is
  **1364 = 336×4 + 4×5** (SNESdev equivalently counts 340 dots with dots 323 and 327 being
  6 clocks). **Pick one numbering convention and document it** (see "Convention" below).
- **Short scanline = 1360 clocks / 340 dots:** NTSC non-interlace, V=240 of alternate frames.
- **Long scanline = 1368 clocks / 341 dots:** PAL interlace, field=1, V=311.
- **Lines/frame: 262 (NTSC) / 312 (PAL)** non-interlace; +1 interlaced (263/313). Last
  VBlank line: 261 (NTSC) / 311 (PAL). Per-frame master clocks ≈ 357,368 (NTSC) / 425,568
  (PAL).
- **WRAM refresh:** the CPU is **paused for 40 master clocks** beginning ~536 clocks into
  each scanline (DRAM refresh) — model this as a fixed per-line CPU stall.

### Convention (binding)

RustySNES counts **341 dots of nominally 4 master clocks**, treating the long dots as the
remainder needed to reach 1364/1360/1368. Code, comments, and `docs/ppu.md` use this
convention exclusively. The two source descriptions are the same silicon
(`ref-docs/2026-06-24-ppu.md` "Note on a flagged discrepancy").

## DMA / HDMA bus-steal

Per `ref-docs/research-report.md` §5 and `ref-docs/2026-06-24-ppu.md` §5:

- **GP-DMA** (`MDMAEN $420B`): **8 master clocks per byte** (regardless of FastROM),
  +8 cycles / channel, +12–24 cycles whole-transfer alignment. **The CPU is fully halted**
  until all transfers finish; the transfer fires "in the middle of the following
  instruction." Cannot cross a bank. Model as a **CPU stall inserted at the MDMAEN write**.
- **HDMA** (`HDMAEN $420C`): fires at ~H=$116 each visible line; **~18 cycles overhead** when
  any channel is active, **+8 cycles per active direct channel per scanline**,
  **+8 cycles / byte**; **indirect channels cost 24 cycles** (16-cycle pointer load). Worst
  case ~466 cycles / scanline (8 channels, indirect, 4 bytes). **HDMA preempts GP-DMA.**
  Model as a per-line budget evaluated at H≈$116.

This precise, content-dependent cycle theft is the second reason (after the variable CPU
cycle) the scheduler must be master-clock resolution.

## H/V-IRQ and NMI

The 5A22 raises NMI at VBlank start (V=225, or V=240 in overscan) and an IRQ at a programmed
H and/or V counter position (`$4207–$420A`), enabling mid-frame raster effects
(`ref-docs/research-report.md` §2). The H/V counters are latched by reading SLHV `$2137` and
read back from `$213C`/`$213D`. These fire off the master-clock phase, not the CPU cycle.

## SA-1 — the second CPU (Phase 4)

The SA-1 coprocessor is a **second WDC 65C816** clocked at master / 2 (~10.74 MHz), so each SA-1
CPU cycle is **2 master clocks**. The crate graph forbids `rustysnes-cart` from depending on
`rustysnes-cpu`, so the SA-1 *system* state lives in `coproc::sa1::Sa1Board` (the cart) while the
scheduler (`rustysnes-core`) owns the second `rustysnes_cpu::Cpu` and drives it through the
`Board` second-CPU hooks (`docs/cart.md` §SA-1).

**The stepping model is deterministic catch-up, not a free-running thread.** `System` holds an
optional `sa1_cpu`, instantiated in `reset()` iff the installed cart `has_second_cpu()`. After every
main-CPU instruction (and the HDMA/DMA bus-steals inside the run loop), `run_sa1` measures the
master clock the *untouched* main CPU has elapsed since the last call and converts it to an SA-1
cycle budget (`Δmaster / 2`). It then steps the second CPU — against a thin `Sa1Bus` adapter that
routes `read24`/`write24` to `Board::second_cpu_{read,write}` — until the budget is spent, charging
each instruction's returned cycle count (×2) to the SA-1 H/V timer via `second_cpu_tick`. Because
the budget is a pure function of `bus.clock.master` (which is a pure function of the deterministic
main CPU), **installing and stepping the second CPU never perturbs the main CPU's behaviour or the
existing scheduler timing** — the `cpu_oracle` stays bit-identical, and the SA-1 only runs for SA-1
carts (gated by `sa1_cpu.is_some()`).

**Control + interrupts.** The SA-1 powers up held in reset (RESB). When the S-CPU clears RESB the
board latches a reset edge that `run_sa1` consumes to reset the second CPU (its reset/NMI/IRQ vector
fetches are redirected inside the board to the SA-1's own CRV/CNV/CIV vectors). While the SA-1 is
held in reset or asleep (`second_cpu_running()` is false) the budget drains into the timer only.
The SA-1→S-CPU IRQ is `Board::irq_pending()`, ORed into the main bus IRQ line in `poll_irq`; the
S-CPU→SA-1 IRQ/NMI feed the second CPU's `poll_irq`/`poll_nmi`. The SA-1 timing is approximate
catch-up (not sub-instruction lockstep with the main CPU's bus accesses), which is exact for the
register/arithmetic/DMA results games observe and keeps the contract fully deterministic.

## The SPC700 async resync (the accuracy crux)

Per `ref-docs/2026-06-24-apu.md` §2: the SPC700 / S-DSP run on their own ~1.024 MHz timebase
(24.576 MHz resonator). RustySNES tracks "how far ahead is the CPU vs the SMP" with a single
**signed integer relative-time accumulator**: when the CPU steps N of its clocks, subtract
N × 24,576,000; when the SMP steps N, add N × 21,477,272 (or the equivalent reduced rational
ratio). No floating point, so the counter is exact. The bus resyncs the SMP up to "now":

1. on **every CPU access to `$2140–$2143`** (and SMP access to `$F4–$F7`), and
2. **once per scanline** (to bound audio latency).

Between syncs the SMP may run arbitrarily far ahead as long as neither side touches the
ports. This is the higan/bsnes cooperative-threaded technique (`docs/adr/0001`,
`ref-docs/research-report.md` §3) implemented single-threaded so save-states / netplay stay
bit-deterministic. Resonator drift is **deliberately not modeled** in the deterministic core
(see `docs/adr/0004`).

### Implementation (Phase 3 — T-31-003)

The accumulator lives in `Bus::Clock::spc_accum` (a `u64`) and is stepped inside
`Bus::advance_master` — the same per-master-tick loop that drives the PPU dot clock — so the SMP
advances in **true lockstep**, not catch-up:

```text
spc_accum += SPC_NUM;                       // SPC_NUM = 68_352
while spc_accum >= SPC_DEN {                 // SPC_DEN = 715_909
    spc_accum -= SPC_DEN;
    apu.advance_smp_cycle();                 // release one SMP *base* clock
}
```

`68_352 / 715_909` is the exact rational `(apuFrequency / 12) / 21_477_270` reduced by gcd = 30,
where `apuFrequency = 32040 × 768 = 24_606_720` Hz (ares) and the SMP runs at `apuFrequency / 12 ≈
2.05 MHz` (a normal SMP access = `SMP_WAIT` = 2 base clocks → the ~1.025 MHz effective opcode rate;
`docs/apu.md`). Integer-only, so the SPC domain is bit-deterministic (`docs/adr/0004`).

Because the SMP is advanced at master-clock granularity, by the time the CPU reads `$2140-$2143`
the SMP has already been clocked up to that exact master instant — `Bus::b_read`/`b_write` route
those four ports straight through `Apu::cpu_read_port` / `Apu::cpu_write_port` (the dead
`apu_ports` latch array is gone). The "forced per-scanline sync" the model above describes is
therefore **subsumed by the continuous lockstep** (the SMP is never arbitrarily ahead), which is
stronger than the latency-bounded on-demand sync and stays fully deterministic.

**Cycle-exact SMP step (T-31-004):** `advance_smp_cycle` now releases **exactly one SMP base clock
per call** by draining a recorded micro-op timeline of the in-flight instruction (one entry per
SPC700 bus access), committing each SMP→CPU port write at the precise base cycle its access
completes — rather than running a whole instruction at the budget boundary. So a CPU read of
`$2140-$2143` observes the SMP exactly up to that master instant and no further, the cooperative-
thread interleaving achieved single-threaded (full derivation in `docs/apu.md` §cycle-exact). This
got all four blargg `spc_*` ROMs to **stream their result text** (decoded + asserted by
`tests/blargg_spc.rs`). The **timer-phase fix** (T-31-006) then drove `spc_smp` / `spc_timer` /
`spc_mem_access_times` to blargg's **literal PASS** — the residual was the recording bus clocking the
SPC700 timer *after* the write side effect instead of before (ares/Mesen2 clock it first), not a
CPU-leading-vs-symmetric clock-model asymmetry as earlier believed (`docs/apu.md` §timer phase).
`spc_dsp6` remains Failed 02 on a separate S-DSP echo/envelope residual.

## Test plan

- The variable-cycle map: verify against the SingleStepTests/65816 per-cycle bus traces
  (each opcode JSON carries cycle-by-cycle bus activity).
- DMA/HDMA timing: undisbeliever/snes-test-roms HDMA-timing and mid-frame ROMs; the cycle
  budget must match within the test's tolerance.
- The SPC resync: blargg `spc_mem_access_times` + the IPL-boot handshake; gilyon SPC tables.
- Scanline-length variants: a deterministic golden framebuffer for a known ROM at each region.

## Implementation status (Phase 2)

The scheduler lives in `rustysnes-core` as the `Bus` (the master-clock phase + memory decode +
DMA/HDMA) plus the `System` run loop (`scheduler.rs`):

- **The clock is CPU-driven.** Each `CpuBus::read24`/`write24` stashes the region access speed
  (`Bus::access_speed`, the ares `CPU::wait` map above), and the following `on_cpu_cycle` advances
  the master clock by it — internal CPU cycles default to 6. `advance_master` steps the PPU dot
  clock (4 master/dot) and the SPC accumulator in-line, so it is true lockstep. A booted NTSC
  frame measures ≈357,374 master clocks (spec ≈357,368).
- **DMA/HDMA** is `dma.rs` (clean-room from ares `dma.cpp`): GP-DMA halts the CPU and charges
  `8`/byte; HDMA runs per visible scanline with the per-mode lengths `{1,2,2,4,4,4,2,4}`, indirect
  pointers, and the line counter. The `System` fires HDMA at scanline boundaries.
- **NMI / IRQ:** the RDNMI (`$4210`) VBlank flag sets at VBlank **regardless** of the NMITIMEN
  enable (so VBlank-poll loops like gilyon's work); the NMI *interrupt* and the H/V-IRQ comparator
  (pushed to the PPU each dot) fire only when enabled.
- **Deferred refinements** (no committed ROM depends on them yet): the 40-clock DRAM-refresh CPU
  stall, the exact H=$116 HDMA dot phase (currently the scanline-boundary trigger), and the
  PAL-frame master-clock cycle-check.

## Open questions

- Exact per-opcode master-clock breakdown for rarer addressing modes — a verify-against-the-
  oracle item, gated on securing the 65816 JSON license (`ref-docs/research-report.md`
  "Open questions" #1; `docs/testing-strategy.md` §licensing).
