# Sprint 1 — Accuracy triage + the residual ledger

**Phase:** Phase 6 — Accuracy to target
**Sprint goal:** run the full battery, pin and close the reachable gaps, and triage the
hard-tier residuals into a deferral ledger (not point-fixes).
**Estimated duration:** 2 weeks

## Tickets

### T-61-001 — Run the full battery + baseline the pass rate

**Description:** run the composed battery (65816+spc700 JSON, gilyon, undisbeliever, blargg
`spc_*`, 240p Suite) and record a per-suite pass count in `docs/STATUS.md`.

**Acceptance criteria:**

- [ ] Every suite runs in CI (the 65816 set per its license posture).
- [ ] `docs/STATUS.md` shows real per-suite counts (no longer all-zero).
- [ ] The honesty gate confirms no BestEffort board inflates the number.

**Dependencies:** T-41-004; T-51-003
**Reference:** `docs/testing-strategy.md`; `docs/STATUS.md`
**Estimated complexity:** M

---

### T-61-002 — Pin + close the reachable gaps to ≥90%

**Description:** for each failing test ROM that does *not* need sub-cycle resolution, pin the
expectation first, implement the fix, and re-run. Drive the battery to ≥90%.

**Acceptance criteria:**

- [ ] The battery reaches ≥90% (100% the standing goal).
- [ ] Each fix references the test ROM that closes it.
- [ ] No per-quirk hack closes a residual that actually needs sub-cycle resolution.

**Dependencies:** T-61-001
**Reference:** `docs/testing-strategy.md` Layer 4
**Estimated complexity:** L

---

### T-61-003 — The hard-tier residual ledger

**Description:** enumerate the residuals that need sub-cycle / fractional-timebase resolution,
record them in `docs/STATUS.md` with a deferral note pointing at `docs/adr/0002`.

**Acceptance criteria:**

- [ ] Each residual has a one-line cause + why it needs the refactor.
- [ ] `docs/STATUS.md` carries the ledger; none are point-fixed.
- [ ] `docs/adr/0002` is referenced as the closure path.

**Dependencies:** T-61-002
**Reference:** `docs/adr/0002`; `docs/STATUS.md`
**Estimated complexity:** S

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] The battery is at target; residuals are deferred, not hacked.
- [ ] CHANGELOG.md updated.
