# Phase 1 — CPU + golden oracle

## Goal

The WDC 65C816 main-CPU core in `rustysnes-cpu` executes every opcode × addressing mode
correctly — in both 8- and 16-bit register widths and in both emulation and native modes — and
passes the per-opcode oracle to **0-diff** (state + cycle-by-cycle bus activity). The SNES has
no Nintendulator-style textual golden CPU log; the SingleStepTests/65816 JSON per-opcode suite
is the golden oracle (`docs/testing-strategy.md`).

## Exit criteria

- [x] All official 65C816 opcodes implemented across every addressing mode. *(all 256; oracle
      passes every opcode file.)*
- [x] `REP`/`SEP` width changes + `XCE` emulation/native transitions correct. *(native `.n` and
      emulation `.e` oracle files both pass.)*
- [x] The SingleStepTests/65816 oracle 0-diffs on state **and** per-cycle activity. *(100.00% —
      5,119,999 / 5,120,000 full passes; cycle count 100.00%. The 1 residual is a documented
      inter-reference `e1.e` divergence. Cross-checked against the gitignored external set per
      ADR 0005.)*
- [ ] gilyon/snes-tests CPU `.sfc` golden `tests*.txt` tables match. **DEFERRED → T-04:** booting
      a `.sfc` needs a working `System` (cart memory map + scheduler), which is Phase 2/4 scope,
      not the pure-CPU Phase 1. The runner contract (`run_until_complete`) is in place.
- [x] The per-opcode master-clock cost formula (variable cycle penalties) verified against the
      JSON traces. *(cycle-count column of the oracle is 100.00%.)*
- [x] All sprints in this phase complete; `docs/STATUS.md` CPU row updated. *(CPU row + 65816
      suite counts updated; one criterion deferred above with reason.)*

## Scope

In-scope:

- The 65C816 register file, status flags (incl. M/X/E), 24-bit addressing via PBR/DBR.
- Every addressing mode + the variable cycle penalties (m=0, direct-page misalignment,
  page-cross).
- Emulation/native modes; the separate vector tables.
- The `CpuBus` trait whose `read`/`write`/`io` carry the access speed back to the scheduler.

Out-of-scope:

- The scheduler / PPU phase derivation (Phase 2 consumes the `CpuBus` access speeds).
- DMA/HDMA halt behavior (Phase 2) — the CPU must merely be steppable at access granularity.
- The SPC700 (a separate core, Phase 3) and the SA-1 second-CPU instance (Phase 4 reuses this
  core).

## Sprints

- [Sprint 1 — Register file, addressing modes, official opcodes](sprint-1-core-opcodes.md) —
  the instruction core to first-pass completeness.
- Sprint 2 — Modes + cycle accuracy + the JSON oracle to 0-diff.
  **Status:** stub — refine when Sprint 1 is ~complete.

## Dependencies

Phase 0 complete (the harness skeleton + the oracle license decision).

## Risks

- **The 65816 oracle license** could block CI gating — mitigated by T-01-005's decision; if
  self-generating, that work lands here.
- **Variable-cycle subtlety** (width-after-flag-write, page-cross, direct-page misalignment) —
  the JSON per-cycle bus traces catch these; pin the failing opcode first.

## Reference docs

- [docs/cpu.md](../../docs/cpu.md) — the register/mode/timing spec.
- [docs/scheduler.md](../../docs/scheduler.md) — the access-speed map the `CpuBus` returns.
- [docs/testing-strategy.md](../../docs/testing-strategy.md) — the oracle + licensing.
