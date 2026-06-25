# ADR 0001 — Master-clock lockstep scheduler + the SPC700 second clock domain

## Status

Accepted.

## Context

RustySNES must reach cycle accuracy (the Mesen-S / ares bar) without per-quirk patches. Per
`ref-docs/research-report.md` §§1–3 the SNES is harder than the NES on two axes: the main CPU
cycle is **variable** (6/8/12 master clocks per access, by region + the `$420D` FastROM bit),
and the audio subsystem (SPC700 + S-DSP) runs on a **separate 24.576 MHz resonator,
asynchronous** to the 21.477270 MHz master clock. The reference accuracy emulators
(higan/bsnes/ares) solve the async problem with a cooperative-threaded scheduler and a single
signed relative-time counter, resyncing only at the four communication ports $2140–$2143 plus
a forced once-per-scanline sync.

## Decision

1. **Single master clock, lockstep.** A scheduler in `rustysnes-core` advances the
   **21.477270 MHz** (PAL 21.281370 MHz) master clock; the CPU/PPU/DMA/HDMA step on their
   divisors **in lockstep, not catch-up**. The CPU bus returns each access's speed (6/8/12) and
   the scheduler advances that many master ticks, then re-derives the PPU dot / HDMA / IRQ-timer
   phase. This makes mid-instruction events (mid-scanline scroll / Mode-7 / CGRAM writes, HDMA
   at H≈$116, H/V-IRQ at an exact counter) visible to subsequent CPU code without per-quirk
   hacks.

2. **The SPC700 / S-DSP are a second clock domain, not a master-clock divisor.** They run on
   their own ~1.024 MHz timebase, tracked by a **single signed integer relative-time
   accumulator** (CPU step → subtract N × 24,576,000; SMP step → add N × 21,477,272 — the two
   source frequencies as exact integer scaling factors, no floating point). The bus resyncs the
   SMP up to "now" **on every port access ($2140–$2143 / $F4–$F7) and once per scanline**.

**Rationale for the integer accumulator over coroutines:** the higan/bsnes libco coroutine
model is the conceptual reference, but Rust coroutines fit awkwardly and complicate
bit-deterministic save-states / netplay. The single-threaded accumulator gets the same
accuracy while preserving the determinism contract (`docs/adr/0004`) — no host RNG or
wall-clock leaks into the core. (`ref-docs/2026-06-24-apu.md` §2.)

## Consequences

- (+) No per-quirk hacks for mid-frame events; the async audio is genuinely modeled, not
  faked.
- (+) Deterministic: the accumulator is exact integer arithmetic.
- (−) One global run loop; the divisor + access-speed tables must be exact (`docs/scheduler.md`).
- (−) The variable CPU cycle + the accumulator math need care — verified against the
  SingleStepTests per-cycle bus traces.
- This scheduler is "the master clock exists" — **not** the future fractional-timebase refactor
  (`docs/adr/0002`); they are different milestones.
