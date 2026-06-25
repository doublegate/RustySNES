# Phase 6 — Accuracy to target

## Goal

Drive the composed two-layer accuracy battery to **≥90% (100% the goal)** by pinning each
failing test ROM, implementing until it passes, and repeating. Identify the hard-tier residuals
and decide which defer to the fractional-timebase refactor (`docs/adr/0002`) — documented in
`docs/STATUS.md`, **not point-fixed** with per-quirk hacks.

## Exit criteria

- [ ] The accuracy battery (SingleStepTests 65816+spc700, gilyon, undisbeliever, blargg `spc_*`,
      240p Suite) at ≥90%; 100% the standing goal.
- [ ] Mesen-S-parity on the blargg SPC/DSP suite (the concrete audio bar) to the achievable
      bound.
- [ ] The hard-tier residuals enumerated in `docs/STATUS.md` with a deferral note pointing at
      `docs/adr/0002`.
- [ ] The determinism layer (save-state round-trip + replay) green.
- [ ] All sprints complete.

## Scope

In-scope:

- Closing accuracy gaps across CPU / PPU / APU / cart by pinning ROMs.
- Off-axis accuracy (overscan, interlace, hi-res, mid-scanline edge cases).
- The residual triage: which gaps are reachable now vs need sub-cycle resolution.

Out-of-scope:

- The fractional-timebase refactor itself (deferred, `docs/adr/0002`) — only *triaged* here.
- BestEffort coprocessor breadth (Phase 7).

## Sprints

- [Sprint 1 — Accuracy triage + the residual ledger](sprint-1-triage.md) — measure, pin, close,
  defer.
- Sprint 2 — Off-axis + mid-scanline edge cases.
  **Status:** stub — refine when Sprint 1 is ~complete.

## Dependencies

Phases 1–4 feature-complete enough to run the full battery; Phase 5 for the determinism layer.

## Risks

- **Point-fixing residuals** instead of deferring them — forbidden; it corrupts the accuracy
  number's meaning. Detect: a per-quirk hack in review. Mitigate: the residual ledger +
  `docs/adr/0002`.
- **The 65816 oracle license** still gating CI — resolved in Phase 0 / 1; revisit if unresolved.

## Reference docs

- [docs/testing-strategy.md](../../docs/testing-strategy.md) — the battery + the ≥90% gate.
- [docs/adr/0002](../../docs/adr/0002-fractional-timebase-refactor.md) — where residuals defer.
- [docs/STATUS.md](../../docs/STATUS.md) — the residual ledger lives here.
