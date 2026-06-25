# ADR 0004 — The determinism contract

## Status

Accepted.

## Context

Save-states, regression tests, TAS replay, and netplay rollback all require exact
reproducibility. The SNES poses two specific hazards to this (`ref-docs/research-report.md`
"Principal engineering challenges" #6, `ref-docs/2026-06-24-apu.md` §2 "Determinism caveat"):
the **SPC700/S-DSP resonator drifts ±0.5%** on real hardware (a documented TAS-desync source),
and several coprocessors carry **real-time clocks** (S-RTC, SPC7110's RTC-4513) that would read
host wall-clock time.

## Decision

**Same seed + ROM + input sequence ⇒ bit-identical framebuffer + audio.** Concretely:

- Power-on CPU/PPU/SMP phase alignment comes from a **seeded PRNG**; **reset preserves the
  alignment**.
- The SPC700 async domain is tracked by the **integer relative-time accumulator** of
  `docs/adr/0001` — exact, no floating point.
- **No system time, thread scheduling, or OS RNG enters the core.** Resonator drift (±0.5%) is
  **excluded** from the deterministic path — the SPC domain uses a fixed nominal 1.024 MHz; any
  "hardware-accurate audio" drift toggle lives in the frontend, off by default, outside the
  bit-identical path (`docs/apu.md` §determinism-caveat).
- **RTC chips are seeded / frozen** — they read a fixed/seeded time, never host wall-clock
  (`docs/cart.md`).
- **Rate control and run-ahead live in the frontend** (a resampler stage / snapshot-restore
  orchestration), **never in the core synthesis** (`docs/frontend.md`). Netplay rollback is
  frontend-orchestrated against the deterministic core.

## Consequences

- (+) Save-state round-trip, replay, and netplay rollback are reliable; a regression harness
  can assert bit-identical AV.
- (+) The accumulator + seeded phase make the whole core a pure function of (seed, ROM, input).
- (−) A game relying on per-console resonator drift plays deterministically but will not
  bit-match a specific physical unit — an accepted, documented divergence.
- (−) Contributors must resist introducing hidden non-determinism (system time, thread
  scheduling, OS RNG, HashMap iteration order) into any core crate — enforced by review + the
  determinism test layer (`docs/testing-strategy.md` Layer 6).
