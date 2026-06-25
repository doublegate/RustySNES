# ADR 0005 — The 65816 per-opcode oracle: self-generate, cross-validate, commit

## Status

Proposed. (Resolves the standing Phase-0 open question / ticket **T-01-005**; ratify before
Phase 1 gates its primary CPU oracle.)

## Context

Phase 1's primary CPU oracle is the **SingleStepTests/65816** JSON corpus — per-opcode, every
addressing mode, 8/16-bit, native + emulation, with the cycle-by-cycle bus-pin traces that no
textual Nintendulator-style 65816 log provides (`docs/testing-strategy.md` Layer 2;
`ref-docs/research-report.md` "Open questions" #1). The blocker, **verified live 2026-06-24**
(404 on the GitHub license API): the 65816 set ships **no LICENSE**. Its sibling
SingleStepTests/**spc700** is MIT and is fine to use in the external tier; the 65816 set is not.

Per the ADR 0003 / `docs/testing-strategy.md` licensing posture, **only permissively-licensed
corpora may be committed into the MIT/Apache tree**, and unlicensed data must not be vendored.
So the corpus cannot be committed, and "just gitignore it and fetch in CI" still has CI pulling
unlicensed third-party data from an upstream that could move or revoke it — a fragile, legally
murky dependency for the *primary* gate of the whole CPU phase.

Three options were on the table (T-01-005): (a) keep it gitignored in the external tier; (b)
secure explicit permission from upstream; (c) self-generate equivalent JSON from a validated
core. (a) leaves CI unable to gate cleanly; (b) is slow, uncertain, and a single point of
failure outside our control.

## Decision

**Adopt a hybrid: option (c) as the oracle of record, with option (a) as a local cross-check.**

1. **Self-generate the committed oracle.** The test-harness emits its own per-opcode JSON —
   `initial state → execute one opcode → final state`, **including the cycle-by-cycle bus-pin
   trace** (the upstream set's real value-add; the generator records the bus trace, not just
   register deltas). This is the bsnes/ares-style approach and is fully license-clean, so it
   **commits** into `tests/roms/` (or `crates/rustysnes-test-harness/oracle/`) and CI gates on it.

2. **Bootstrap-validate before trusting it.** Self-generated JSON is only as correct as the core
   that produced it — frozen blindly, it would "pass its own bugs." So the freeze is gated:
   before the self-gen set becomes the committed oracle of record, the 65816 core must agree,
   opcode-for-opcode and cycle-for-cycle, with **both** (i) the upstream SingleStepTests/65816
   set (fetched locally into the gitignored `tests/roms/external/`, never committed) **and** (ii)
   a second independent reference (the gilyon CPU ROMs, and spot-checks against ares). Only after
   that cross-validation passes is the self-gen JSON frozen + committed.

3. **Record provenance.** Each committed oracle file carries a header noting the generating core
   version, the date, and the references it was cross-validated against, so the oracle's
   trust basis is auditable and re-derivable.

4. **Thereafter, CI gates on the committed self-gen set only.** The upstream unlicensed set is a
   *local* cross-validation reference, fetched on demand into the external tier — never a CI
   dependency, never committed.

## Consequences

- (+) The primary CPU oracle becomes **license-clean and committable**; CI gates without
  fetching unlicensed data and without an external single point of failure.
- (+) The self-gen set doubles as a frozen regression snapshot of the 65816 core — any future
  drift shows up as a diff against committed expectations.
- (+) Removes the Phase-1 blocker without waiting on an upstream permission grant.
- (−) Trust bootstrapping is the hazard: a self-generated oracle validates the core against
  *itself* unless the cross-validation gate (§2) is taken seriously. The cross-validation step
  is **mandatory, not optional**, and its passing is the precondition for the freeze.
- (−) Standing cost: a JSON generator + a cross-validation harness + the provenance discipline.
  Larger up-front than "just gitignore it," but it is paid once and removes a permanent external
  dependency.
- (−) Until the freeze, Phase 1 runs against the gitignored upstream set locally, so the very
  first CPU work still needs a developer to fetch it — documented in the ROM-seeding runbook.

## Follow-ups

- `docs/testing-strategy.md` §licensing "Open questions" #1 → mark resolved, pointing here.
- `docs/STATUS.md` suite table → 65816 row posture becomes "self-gen (committed) + upstream
  cross-check (external)".
- New Phase-1 tickets: the generator, the cross-validation gate, the freeze + provenance.
