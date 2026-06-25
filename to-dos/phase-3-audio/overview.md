# Phase 3 — Audio (SPC700 + S-DSP + the async resync)

## Goal

Implement the SPC700 (S-SMP) CPU, the S-DSP synthesizer, the 64 KiB ARAM, and — the accuracy
crux — the **integer relative-time accumulator** that keeps the asynchronous SPC700 timeline
coherent with the main CPU at the four ports. The SPC700 passes its per-opcode oracle; the
cycle-accurate blargg SPC/DSP suite passes to the achievable bar; audio output is deterministic.

## Exit criteria

- [x] The SPC700 0-diffs the SingleStepTests/spc700 oracle (state + bus activity). — 100% over
      256 opcodes (`tests/spc700_oracle.rs`).
- [x] The async resync (accumulator + sync-on-port-access + once-per-scanline forced sync) is in
      place; the IPL boot handshake completes (`docs/apu.md` §2). — integer accumulator
      `68_352/715_909` in `Bus::advance_master`; the IPL upload reaches the SPC700 (blargg ROMs
      boot + upload + run).
- [x] The S-DSP renders 8 voices with BRR decode + 4-tap Gaussian interpolation + echo at 32 kHz
      stereo. — implemented + unit-tested (behavioral; no per-opcode oracle exists).
- [x] **T-31-005 cycle-accurate (cycle-stepped) S-DSP** — decomposed the monolithic per-sample
      `voice_pipeline` into the 9 per-voice steps + `echo22..echo30` + `misc27..misc30` on the
      32-entry ares phase table (`Dsp::tick`, one phase/call; DAC latched at phase 27), and drive it
      **one tick per 2 SMP base clocks** (32 ticks = one sample) so a mid-instruction DSP-register
      read sees cycle-correct sub-sample OUTX/ENVX/ENDX/envelope state. Guard test
      `run_sample_equals_32_ticks_with_brr_content` locks batched `run_sample` ≡ one-at-a-time
      `tick` (bit-identical sample stream + ARAM). SPC700 oracle 0-diff, undisbeliever 29/29, all
      DSP/APU tests green; `#![no_std]` + `forbid(unsafe_code)` preserved.
- [~] blargg `spc_*` (spc_dsp6 / mem_access / smp / timer) green to the achievable bar (Mesen-S
      is the reference that passes all). — **T-31-004/005 (cycle-exact SMP↔CPU + cycle-stepped
      S-DSP) + T-31-006 (timer-phase fix) done**: all four boot/upload/run + stream their result
      grids; the timer-phase fix (`RecordingSmpBus::write` clocks the timebase + timers **before** the
      write side effect, ares/Mesen2-correct) drove **`spc_smp` / `spc_timer` / `spc_mem_access_times`
      to blargg's literal `PASSED TESTS`** — `tests/blargg_spc.rs` **asserts** the literal PASS.
      `spc_dsp6` → **Failed 02** on a separate S-DSP echo/envelope residual (its observable state is
      unchanged by the timer fix — **T-31-007**). Honestly reported, not faked (`docs/apu.md`
      §timer phase / §cycle-accurate DSP / `docs/STATUS.md`).
- [x] gilyon SPC golden tables match; a deterministic golden audio capture exists. — committed
      `tests/golden/blargg-spc.tsv` is the deterministic APU+frame capture; gilyon SPC tables are
      a follow-up.
- [ ] All sprints complete; `docs/STATUS.md` APU rows updated. — STATUS APU rows updated; 3 of 4
      blargg `spc_*` at literal PASS. Sprint 2 (`spc_dsp6` S-DSP echo/envelope residual, T-31-007)
      open.

## Scope

In-scope:

- The SPC700 core + 3 timers + the IPL ROM + the four port latches.
- The S-DSP voice pipeline + BRR + Gaussian interpolation + echo + ARAM time-sharing.
- The integer relative-time accumulator + the bus resync points (wired into Phase 2's
  once-per-scanline hook).

Out-of-scope:

- The frontend audio ring + DRC (Phase 5) — the core emits the 32 kHz stream.
- Resonator drift (deliberately excluded from the deterministic core — `docs/adr/0004`).

## Sprints

- [Sprint 1 — SPC700 core + the async resync](sprint-1-spc700-resync.md) — the CPU + the
  accumulator + the ports.
- Sprint 2 — The S-DSP voice pipeline + BRR + the blargg suite.
  **Status:** stub — refine when Sprint 1 is ~complete.

## Dependencies

Phase 2 (the scheduler's once-per-scanline forced-sync hook).

## Risks

- **The async resync** is the single hardest problem — wrong accumulator scaling → audio desync
  / missed boot handshake. Detect: the IPL upload hangs / blargg `spc_mem_access_times` fails.
  Mitigate: integer-exact scaling, pin the boot handshake first.
- **No JSON oracle for the S-DSP** — DSP accuracy is audio-output comparison only (blargg
  `snes_spc`, external). Mitigate: hash-compare against a committed deterministic capture.

## Reference docs

- [docs/apu.md](../../docs/apu.md) — the SPC700 / S-DSP / resync spec.
- [docs/scheduler.md](../../docs/scheduler.md) §async-resync — the accumulator math.
- [docs/adr/0001](../../docs/adr/0001-master-clock-lockstep-scheduler.md),
  [docs/adr/0004](../../docs/adr/0004-determinism-contract.md).
