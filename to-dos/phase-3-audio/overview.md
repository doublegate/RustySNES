# Phase 3 — Audio (SPC700 + S-DSP + the async resync)

## Goal

Implement the SPC700 (S-SMP) CPU, the S-DSP synthesizer, the 64 KiB ARAM, and — the accuracy
crux — the **integer relative-time accumulator** that keeps the asynchronous SPC700 timeline
coherent with the main CPU at the four ports. The SPC700 passes its per-opcode oracle; the
cycle-accurate blargg SPC/DSP suite passes to the achievable bar; audio output is deterministic.

## Exit criteria

- [ ] The SPC700 0-diffs the SingleStepTests/spc700 oracle (state + bus activity).
- [ ] The async resync (accumulator + sync-on-port-access + once-per-scanline forced sync) is in
      place; the IPL boot handshake completes (`docs/apu.md` §2).
- [ ] The S-DSP renders 8 voices with BRR decode + 4-tap Gaussian interpolation + echo at 32 kHz
      stereo.
- [ ] blargg `spc_*` (spc_dsp6 / mem_access / spc / timer) green to the achievable bar (Mesen-S
      is the reference that passes all).
- [ ] gilyon SPC golden tables match; a deterministic golden audio capture exists.
- [ ] All sprints complete; `docs/STATUS.md` APU rows updated.

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
