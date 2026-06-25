# SPC700 (S-SMP) + S-DSP + ARAM — RustySNES

**References:** `ref-docs/2026-06-24-apu.md` (the primary source for this doc);
`ref-docs/research-report.md` §3; `docs/scheduler.md` §async-resync; `docs/adr/0001`,
`docs/adr/0004`. Cited inline: SNESdev S-SMP, undisbeliever S-SMP clock measurements,
byuu.net / bsnes.org scheduler articles.

This doc is the SPEC, not history — update it in the same PR as the code. Pin behavior
against the test ROMs first.

## Purpose

The SNES audio subsystem is a **decoupled audio computer**: the Sony **SPC700 (S-SMP)** 8-bit
CPU, the **S-DSP** wavetable synthesizer, and **64 KiB of ARAM**, all on a separate
24.576 MHz resonator **asynchronous** to the 21.477 MHz master clock. Keeping this second
timeline coherent with the main CPU at the four communication ports is *the* SNES accuracy
crux (`ref-docs/research-report.md` "Executive summary"). `rustysnes-apu` owns all of it; the
S-DSP sees a narrow `DspBus`/ARAM trait.

## Hardware facts

Per `ref-docs/2026-06-24-apu.md` §1 (SNESdev S-SMP, undisbeliever):

- **SPC700 clock = 1.024 MHz**, independent of the rest of the SNES, generated from a
  **24.576 MHz ceramic resonator** (which also clocks the S-DSP internally at 3.072 MHz).
- **Resonator tolerance ±0.5% (5000 ppm).** Measured per-console S-DSP sample rates span
  **32036–32152 Hz** (nominal 32000 Hz). This drift + the variable S-CPU↔S-SMP handshake delay
  is a documented cause of **TAS desyncs on real hardware** — and the reason drift is excluded
  from the deterministic core (`docs/adr/0004`).
- **S-DSP:** 8 voices, **BRR-compressed samples** decoded with **4-point Gaussian
  interpolation**, 32 kHz 16-bit stereo, **one stereo sample every 768 resonator cycles**.
- **64 KiB ARAM** (two 32K×8 PSRAM), time-shared "1 S-SMP access for every 2nd S-DSP access."
- **Three timers:** two @ 8 kHz, one @ 64 kHz.
- **Four communication ports:** SMP side `$F4–$F7` ↔ CPU side `$2140–$2143`. Each port is two
  one-way latches (CPU→SMP and SMP→CPU), readable only from the other side.
- **IPL boot ROM:** the SMP starts in a 64-byte IPL that handshakes over the ports to receive
  the audio program the main CPU uploads at boot — a timing-sensitive handshake the emulator
  must reproduce exactly.

## The async-resync model (the accuracy crux)

Because the SPC700 / S-DSP run on a separate crystal, RustySNES runs them as a **separate
timeline** and forces lockstep with the main CPU **only when the two communicate** — a
read/write of `$2140–$2143` (or `$F4–$F7`). The mechanism (full derivation in
`docs/scheduler.md` §async-resync, `ref-docs/2026-06-24-apu.md` §2):

- A single **signed integer relative-time accumulator** tracks "how far ahead is the CPU vs
  the SMP." CPU step of N clocks → subtract N × 24,576,000; SMP step of N → add N × 21,477,272
  (the two source frequencies as integer scaling factors). Exact, no floating point.
- The bus resyncs the SMP up to "now" on **(1) every port access** and **(2) once per
  scanline** (latency bound). Between syncs the SMP may run arbitrarily far ahead.

This is the higan/bsnes cooperative-threaded technique implemented **single-threaded** — Rust
coroutines fit awkwardly and would complicate bit-deterministic save-states / netplay
(`docs/architecture.md` §rejected-alternatives). The accumulator approach gets the same
accuracy deterministically.

### Determinism caveat

Resonator drift (±0.5%) is **intentionally not modeled** in the deterministic core — the SPC
domain uses a fixed nominal 1.024 MHz. Offer drift only as a separate non-deterministic
"hardware-accurate audio" toggle in the frontend, never in the bit-identical path
(`docs/adr/0004`). RTC chips (S-RTC, RTC-4513) are likewise seeded / frozen (`docs/cart.md`).

## S-DSP voice pipeline

8 voices, each: a BRR sample-source pointer, ADSR/GAIN envelope, pitch (with the Gaussian
4-tap interpolation), and L/R volume; mixed into the 32 kHz stereo output. BRR blocks are
9 bytes (1 header + 8 data → 16 samples) with per-block shift + filter. Echo (FIR) and
pitch-modulation feed off ARAM. The DSP is streaming/stateful (no instruction-discrete JSON
oracle exists — see Test plan).

## Interfaces (sketch)

```rust
// rustysnes-apu
pub struct Smp { /* SPC700 regs + 64 KiB ARAM + 3 timers + IPL ROM */ }
pub struct Dsp { /* 8 voices, echo, master vol; reads ARAM */ }

pub trait ApuPorts {            // the $2140-$2143 / $F4-$F7 latches
    fn cpu_read_port(&mut self, n: u8) -> u8;   // resyncs SMP first
    fn cpu_write_port(&mut self, n: u8, v: u8); // resyncs SMP first
}
```

## Edge cases and gotchas

1. **The boot handshake** (IPL upload) is the first thing to get right — it stresses the
   resync (`ref-docs/2026-06-24-apu.md` §1).
2. **Port latch direction:** each of the four ports is two independent latches; a read returns
   what the *other* side last wrote, not an echo of your own write.
3. **Forced per-scanline sync** is required even when no port access happens, or audio latency
   grows unbounded (`docs/scheduler.md` §async-resync).
4. **Timer dividers** must tick on the SMP timebase, not the master clock.
5. **DSP $STP/SLEEP and timer-edge behaviors** settle empirically against blargg + SST/SPC700
   (`ref-docs/research-report.md` "Open questions" #2).

## Test plan

Per `ref-docs/2026-06-24-apu.md` §3 and `ref-docs/research-report.md` "Standards":

- **Primary SPC700 CPU oracle:** SingleStepTests/spc700 — per-instruction JSON + bus activity.
  **MIT** (cleanly licensed, committable / external per policy).
- **Committable layer:** gilyon/snes-tests (MIT) covers SPC-700 opcodes (all addr modes,
  except SLEEP/STOP) with golden `tests*.txt` tables.
- **Cycle-accurate SPC/DSP oracle:** blargg `spc_*` ROMs (`spc_dsp6` hardest,
  `spc_mem_access_times`, `spc_spc`, `spc_timer`) — license unstated → **external/reference
  tier**. **Mesen-S is the only emulator that passed all of them**, so matching this suite is
  the concrete accuracy bar.
- **DSP audio comparison:** there is **no JSON single-step oracle for the S-DSP** (it is
  streaming, not instruction-discrete); validate by `.spc`→`.wav` output comparison vs blargg
  `snes_spc` (LGPL-2.1, external comparison only, never vendored) or ares/bsnes.

## Open questions

- SPC700 `STP`/`SLEEP` and a few timer-edge cases — settle empirically (above).
- The S-DSP nominal internal clock is cited as ~7.6 vs 8 MHz across sources; correctness is
  gated by the test ROMs, not the nominal number (`ref-docs/research-report.md` "Open
  questions" #4).
