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

### Cycle-accurate DSP (the 32-tick micro-sequence)

The S-DSP is a hard pipeline, **not** a per-voice-at-once mixer. Each 32 kHz output sample is
produced over **32 micro-ticks** (`Dsp::tick`), and the nine per-voice steps (`voice1..voice9`),
the echo path (`echo22..echo30`), and the housekeeping latches (`misc27..misc30`) are
**interleaved across the 8 voices** on a fixed 32-entry phase table — reproduced verbatim from
ares `sfc/dsp/dsp.cpp::main` (ISC). Voice 0 is split (`voice3a`/`3b`/`3c` at phases 22/25/30,
`voice4`/`voice1` at phase 31) because it wraps the sample boundary; the stereo DAC sample latches
at phase 27 (`echo27`). This interleave is what gives **sub-sample timing** to the OUTX/ENVX/ENDX
register writes and the BRR/envelope/pitch latches — the resolution blargg's DSP/mem-access timing
tests probe.

The integration drives the DSP **one tick per 2 SMP base clocks** (32 ticks × 2 = the 64 base
clocks of one sample; SMP base = `apuFrequency/12`). Driving it sub-sample — rather than a whole
64-clock sample at the instruction boundary — means an SMP instruction that reads a DSP register
(`$F3`) mid-execution observes the DSP advanced to exactly that base clock and no further (the
cycle-correct value). `Dsp::run_sample` is retained as the batched `32 × tick` wrapper (unit tests,
`.spc` rendering); a guard test (`run_sample_equals_32_ticks_with_brr_content`) asserts the batched
and one-at-a-time drives are bit-identical (sample stream + ARAM) on real BRR/echo content, which is
what protects the already-passing per-sample output path.

### Per-voice mute (`v1.0.1`, frontend/debug convenience — not real hardware)

Real S-DSP hardware has no per-voice mute register (only the whole-mix `FLG.6` bit,
`MainVol::mute`). `Dsp::set_voice_mutes([bool; 8])` is an additive, frontend-only knob: a muted
voice's contribution to both the main mix and the echo send is dropped entirely inside
`voice_output` (an early `return` before either accumulator is touched). Everything upstream —
BRR decode, the Gaussian interpolation buffer, ADSR/GAIN envelope timing, OUTX/ENVX/ENDX register
content — is completely unaffected, so muting is purely cosmetic: it changes what reaches the
speaker, never anything a ROM can observe via register reads, and un-muting mid-note resumes
exactly where the voice already was. Deliberately NOT part of `save_state`/`load_state` (like
cheats/watchpoints elsewhere in this codebase, it is host UI state, not emulated hardware state);
re-synced from the frontend once per real frame (`Bus::set_voice_mutes`, wired from
`config.audio.voice_mutes` via Settings → Audio's 8 checkboxes). All-`false` (unmuted) is the
default, byte-identical to every prior release.

#### blargg status (honest) — all four literal PASSes

The **timer-phase** fix closed the SPC700 timer suite: `RecordingSmpBus::write` now advances the SMP
timebase and clocks the three timers **before** the write side effect lands (matching ares `step()`
and Mesen2 `Spc::Write`, which run `IncCycleCount` first), instead of storing first and stepping the
timer after. See §timer phase. With it, **`spc_smp`, `spc_timer`, and `spc_mem_access_times` reach
blargg's literal `PASSED TESTS`** (asserted in `tests/blargg_spc.rs`, not a determinism proxy).

`spc_dsp6` now also reaches **`PASSED TESTS`** after the S-DSP **GAIN mode-7 threshold** fix (see
§DSP GAIN mode-7 threshold). It previously stalled at `Envelope/gain $E0 threshold` → "Failed 02":
the bent/two-slope GAIN increase compared its internal envelope latch against `0x600` with a
**signed** test, where blargg `SPC_DSP` (`(unsigned) hidden_env >= 0x600`) and ares
(`(u32) _envelope >= 0x600`) use an **unsigned** one — so a latch left negative by a prior GAIN
decrease underflow failed to trip the reduced slope and the envelope over-incremented. All four ROMs
are now in `EXPECT_PASS`.

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

## Implementation status (Phase 3 — Audio)

The SPC700 + S-DSP + ARAM subsystem is implemented in `crates/rustysnes-apu`, clean-room from
ares (`component/processor/spc700` + `sfc/dsp`, ISC) and pinned to the SingleStepTests/spc700
oracle. Modules: `psw` (status word), `spc700` (core + ALU, generic over `Spc700Bus`),
`spc700_exec` (256-opcode cycle-accurate dispatch), `dsp` (the S-DSP), `lib` (the `Apu`
integration surface: ARAM, IPL ROM, the `$00F0-$00FF` registers, the three timers, the four
ports). `#![no_std]` + `forbid(unsafe_code)`; bare-metal `thumbv7em-none-eabihf` build green.

**SPC700 oracle (the primary gate) — 0-diff.** `tests/spc700_oracle.rs` (gated behind
`test-roms`, self-skips when data absent) replays every SingleStepTests/spc700 case through
`Spc700::step` against flat RAM and diffs registers + RAM + cycle count:

| Tier | Files | Tests | State (regs+ram) | Cycle count | Full |
|---|---|---|---|---|---|
| Committed sample (`tests/roms/spc700-singlestep/v1`, MIT, in-tree) | 256 | 12,800 | **100.00%** | **100.00%** | **100.00%** |
| Full external (`tests/roms/external/spc700-singlestep-full`, gitignored) | 256 | 256,000 | **100.00%** | **100.00%** | **100.00%** |

All 256 opcodes pass state + cycle count, including `MUL`/`DIV`, the word ops
(`MOVW`/`ADDW`/`SUBW`/`CMPW`/`INCW`/`DECW`), `DAA`/`DAS`, the bit-manipulation family
(`SET1`/`CLR1`/`TSET`/`TCLR`/`AND1`/`OR1`/`EOR1`/…), and `STP`/`SLEEP` (the SingleStepTests
capture a fixed 3-iteration halt window → 7 cycles; reproduced exactly).

**S-DSP — behavioral, unit-tested (no per-opcode oracle exists; it is streaming).** Implemented:
8 voices, BRR decode (the four IIR filters + scale/shift), 4-point Gaussian interpolation (table
built from the ares formula), ADSR + GAIN envelopes with the exact 32-entry counter-rate/offset
tables, pitch + pitch-modulation (PMON), the noise LFSR + NON, KON/KOFF with the 5-sample setup
delay and the ENDX/envelope edge timing, the echo system (echo buffer in ARAM, the 8-tap FIR,
EON/EDL/ESA/EFB feedback), MVOL/EVOL, and the per-sample 32 kHz stereo mix to the DAC. The full
ares 32-clock voice/echo/misc micro-sequence is reproduced per output sample. Validation: in-module
`#[cfg(test)]` vectors derived from ares (Gaussian table entries, noise-LFSR steps, BRR filter-0/1
decode, Gaussian interpolation, envelope attack/release, KON setup, echo silence, mute) plus
determinism checks; the assembled `Apu` is covered by `tests/dsp_unit.rs`.

**`Apu` API (for the core to wire the bus ports + async resync):**

- `Apu::new() -> Apu` — power-on; SMP boots from the IPL reset vector (`$FFC0`).
- `Apu::tick(&mut self, bus: &mut impl AudioBus)` — the scheduler's per-SPC-cycle hook (kept
  for the existing `rustysnes-core` call site); instruction-grained internally, the DSP is
  caught up from cycles consumed, ports mirrored through `bus` at the boundary.
- `Apu::step_instruction(&mut self) -> u32` — run one SMP instruction; returns SMP-clock units
  consumed (the unit of `step`; the DSP emits one 32 kHz sample per 768 units).
- `Apu::run_cycles(&mut self, clocks: u32)` — run until ≥`clocks` units elapse.
- `Apu::cpu_read_port(&self, n) -> u8` / `Apu::cpu_write_port(&mut self, n, v)` — the four
  `$2140-$2143` latches (one-way: a CPU read returns the SMP's last write, not an echo).
- `Apu::sample(&self) -> (i16, i16)` — the most-recent 32 kHz stereo DAC sample.
- `Apu::dsp_read(&self, addr) -> u8`, `Apu::aram(&self) -> &[u8; 0x10000]` — debug/save-state.

**Integrated into the machine (Phase 3 — T-31-002 / T-31-003):**

- **CPU↔APU ports + IPL boot handshake (T-31-002, done):** `rustysnes-core::Bus` routes the four
  `$2140-$2143` reads/writes directly through `Apu::cpu_read_port` / `Apu::cpu_write_port` (the
  one-way latches — a CPU read returns the SMP's last write, not an echo). The dead `apu_ports`
  latch array is removed. The CPU's IPL upload handshake reaches the SPC700 end-to-end: every
  blargg `spc_*` ROM boots, the SNES uploads its SPC program through the ports, and the SPC700
  runs it (verified — ARAM fills with the program, ports carry the `AA/BB` IPL signature + the
  upload stream).
- **The async resync accumulator (T-31-003, done):** the SPC700/S-DSP advance in **integer**
  lockstep with the master clock from `Bus::advance_master` — `spc_accum += SPC_NUM` per master
  tick, releasing one SMP **base** clock (`Apu::advance_smp_cycle`) per `SPC_DEN`. The exact
  reduced ratio is `68_352 / 715_909` = `(apuFrequency/12) / 21_477_270` (no floating point —
  ADR 0004). Because the SMP advances at master-clock granularity, a CPU port access already
  observes every SMP write up to that instant, so the once-per-scanline forced sync is subsumed
  by the continuous lockstep. **Verified bit-deterministic**: a booted frame's framebuffer + ARAM
  + ports hash identically across runs (`tests/blargg_spc.rs`).

### The cycle-exact SMP step (T-31-004, done)

`Apu::advance_smp_cycle` is a **true one-base-clock pump**, not an instruction-grained catch-up.
The SMP instruction in flight is decomposed into a recorded **micro-op timeline** — one entry per
SPC700 bus access (`read`/`write`/`idle`), carrying that access's wait-state base-clock count and,
for a write to one of the four `$F4-$F7` ports, the SMP→CPU latch update **deferred** to the base
cycle the access completes on. The mechanism:

- `record_next_instruction` runs the *unchanged* `Spc700::step` through a `RecordingSmpBus` that
  applies every side effect byte-for-byte as the normal `SmpBus` does (ARAM, DSP registers, timers,
  the `$00F0-$00FF` control regs, the boundary DSP-sample catch-up) — so the architectural result
  and the SPC700 oracle stay 0-diff — while emitting the timeline.
- `drain_one_base_clock` then releases the instruction one base clock per `advance_smp_cycle` call,
  committing each deferred SMP→CPU port write at the exact base cycle of its access. Reads of the
  CPU→SMP latch see the value the CPU last wrote (stable for the whole `advance_master` window, so
  cycle-correct in the CPU-leading model). The DSP/timers are advanced at the same cumulative base
  clocks as before.

This is the ares/bsnes cooperative-thread **observable interleaving achieved single-threaded,
without coroutines** (which would break the bit-deterministic save-state/netplay contract,
`docs/architecture.md` §rejected-alternatives). It is the only thing the main CPU can observe
mid-instruction (the rest of the SMP is private to the `Apu`), so it is the only state given
per-cycle precision. Integer-only and order-deterministic.

**Result:** all four blargg `spc_*` ROMs boot, complete the IPL upload handshake, run the SPC
program, and **stream their result grids** — `tests/blargg_spc.rs` decodes the BG-tilemap header
(`$0400`) + result grid (`$0800`) and asserts the real verdict. After the **timer-phase** fix
(§timer phase) and the **DSP GAIN mode-7 threshold** fix (§DSP GAIN mode-7 threshold), **all four —
`spc_smp` / `spc_timer` / `spc_mem_access_times` / `spc_dsp6` — reach blargg's literal
`PASSED TESTS`** (the gate asserts each, not a determinism proxy).

### Timer phase (the `spc_*` timer-suite fix)

The SPC700 timer step/compare logic matches ares `SMP::Timer` and Mesen2 `SpcTimer` line-for-line
(three-stage divider → 4-bit output, clock on a 1→0 line edge, `stage2 == target` bumps the visible
counter, rising `$F1` enable resets `stage2`+output, `$F0` global-enable re-evaluates the edge). The
**phase** at which the timer is clocked relative to an SMP instruction's bus accesses is the load-
bearing detail blargg's `spc_timer` / `spc_smp` / `spc_mem_access_times` pin:

- ares (`SMP::step` before the store) and Mesen2 (`Spc::Write` → `IncCycleCount` first, then the
  store) **advance the timebase and clock the timers BEFORE the write side effect lands**. So a
  write to `$FA-$FC` (timer target), `$F1` (enable), or `$F0` (global enable) observes *this
  access's* timer clock as already-happened.
- Our per-instruction `SmpBus::write` always did this (it calls `step()` first). But the
  **`RecordingSmpBus`** — the bus the integrated `System` actually drives through
  `Apu::advance_smp_cycle` — used to apply `write_io` (the store / target / enable) **first** and
  `record()` (the timebase + timer clock) **after**. That shifted the timer phase by **one access**
  on every `$F0`/`$F1`/`$FA-$FC` write: e.g. arming `target = 1` was seen *before* the arming
  cycle's own clock instead of after, so `TnOUT` lagged hardware by an off-by-one in the stage
  accumulation — exactly the blargg divergence.

The fix reorders `RecordingSmpBus::write` to `record()` first, then store + decode IO (carrying the
deferred SMP→CPU port latch onto that access's micro-op so the CPU↔SMP handshake timing is
unchanged). Reads were already correct (`record()` precedes `read_io`, so a `TnOUT` read at
`$FD-$FF` clears *after* the access's clock). The SPC700 oracle is untouched (it replays against a
flat bus with no timers); the change is confined to the integrated recording bus.
- **SMP wait-state / clock model (corrected for `spc_*`):** the SMP runs on the ares base clock
  `apuFrequency/12 ≈ 2.05 MHz`; a normal bus access is `SMP_WAIT = 2` base clocks (ares
  `cycleWaitStates[0]`), the three timers tick on the same base, and the S-DSP emits one 32 kHz
  sample every **64** base clocks. (Earlier the model charged 1 unit/access with a 768-unit DSP
  divisor, which ran the timers + DSP at the wrong relative rate; the SingleStepTests/spc700
  oracle is unaffected — it measures access *count* against its own flat bus, still 0-diff.) The
  per-region external/internal wait-state divider (`$F0`'s `{2,4,10,20}` glitch table) stays
  collapsed to the reset default; no committed program reprograms it.

### DSP GAIN mode-7 threshold (the `spc_dsp6` fix)

The S-DSP `GAIN` register's **mode 7** (bent / two-slope linear increase) raises the envelope by
`+0x20` per step until it crosses `0x600`, then by `+0x08`. Hardware (and both references — blargg
`SPC_DSP` `(unsigned) hidden_env >= 0x600`, ares `(u32) _envelope >= 0x600`) evaluates that threshold
against the voice's **internal envelope latch** (`env_internal` / `hidden_env`) reinterpreted as
**unsigned**. The latch is the pre-clamp envelope and a preceding `GAIN` *decrease* mode (4 linear /
5 exponential) can leave it **negative** (e.g. `-0x20`, from underflowing past 0). The unsigned
reinterpretation makes that negative latch read as `>= 0x600`, so the reduced `+0x08` slope still
applies. Our `Dsp::envelope_run` compared it **signed**, so a negative latch read as `< 0x600` and
the full `+0x20` slope was used — the envelope over-incremented. That is the sole divergence behind
`spc_dsp6`'s `Envelope/gain $E0 threshold` → "Failed 02". The fix casts the latch to `u32` for the
comparison (`crates/rustysnes-apu/src/dsp.rs`), matching both references; the rest of the envelope
path was already bit-identical to ares (verified by an all-`GAIN`-value differential). The quirk only
fires deep in the Envelope suite, so no ROM's 120-frame boot hash moves (baseline TSV unchanged).

**Deferred / approximated:**

- **blargg `spc_*` literal text PASS** (`tests/blargg_spc.rs`, external/reference tier): all four
  ROMs boot, upload, run, **and stream their result grids** bit-deterministically;
  `tests/blargg_spc.rs` decodes the BG-tilemap header (`$0400`) + the per-opcode/result grid
  (`$0800`, full 32×32 nametable) and **asserts the real verdict**. After the timer-phase fix
  (§timer phase) and the DSP GAIN mode-7 fix (§DSP GAIN mode-7 threshold), **all four — `spc_smp`,
  `spc_timer`, `spc_mem_access_times`, and `spc_dsp6` — reach blargg's literal `PASSED TESTS`** (the
  gate asserts each, not a determinism proxy). `spc_dsp6` is the slowest: its full DSP suite (Echo ·
  Envelope · KON · Misc · Order · Random · Timing) renders `PASSED TESTS` at `$0800` row 30 near
  frame 8.8k (hence `VERDICT_FRAMES = 12000` and the 32-row scan). blargg exposes **no
  machine-readable status byte** (confirmed against Mesen2's `RecordedRomTest`, which compares
  rendered frames); the readable signal is the BG-tilemap text (`tile & 0xFF == ASCII`) at words
  `$0400` (header) and `$0800` (result grid).
- **S-DSP literal 32-tick interleave:** `Dsp::run_sample` runs the full ares `misc27/28` +
  per-voice + `echo22..30` sub-steps with the shadow latches, but batched once per output sample
  rather than interleaved across the literal 32 DSP ticks. The observable per-sample DAC output +
  state match ares; the literal interleave is expandable (the decomposed sub-step methods already
  exist) if a future `spc_dsp6` intra-sample edge demands it.
- **Resonator drift** intentionally not modeled (deterministic core uses fixed nominal clocks,
  `docs/adr/0004`).
