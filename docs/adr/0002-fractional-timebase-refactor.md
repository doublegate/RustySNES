# ADR 0002 — The fractional-timebase refactor (the future "v2.0")

## Status

Proposed. This console needs sub-cycle resolution in principle, so the model is designed in
from day one even though the refactor itself is deferred.

## Context

The `docs/adr/0001` scheduler advances the master clock in whole ticks and treats each CPU
access as an integer 6/8/12-tick block. That cannot represent **sub-cycle bus phases** — the
φ1/φ2 access split, sub-cycle read/modify/write brackets, and the exact intra-cycle moment a
read or write lands on the bus. Per `ref-docs/research-report.md` "Principal engineering
challenges" #1 and "Open questions", some hard-tier behaviors (rare addressing-mode bus
timing, exact DMA/HDMA preemption boundaries, edge-of-cycle PPU latching) may need that
resolution. The RustyNES line names the analogous future work its "Timebase" rewrite.

## Decision

The eventual refactor is a **fractional master clock** with an **every-cycle-bus-access**
model — the one-clock collapse with a φ1/φ2 access split and cycle-accurate reset, the same
shape as RustyNES's planned v2.0.0 "Timebase." Until then:

- The Phase-0 scheduler (`docs/adr/0001`) is built so the access-speed query and phase
  re-derivation are the only places that assume whole-tick blocks — i.e. the refactor is a
  localized change, not a rewrite of every chip.
- **Hard-tier residuals that genuinely need sub-cycle phase are deferred to this refactor —
  documented in `docs/STATUS.md`, NOT point-fixed** with per-quirk hacks.
- This is the **one milestone expected to break byte-identity / save-state compatibility**.

**Do NOT conflate** "the master clock already exists (the Phase-0 / v0.1 scheduler)" with
"this future fractional refactor." They are different milestones — the RustyNES versioning
trap. The forward path is Phase 0 → v1.0.0 → (only if residuals warrant) this refactor.

## Consequences

- (+) The accuracy ceiling is reachable without contorting the lockstep scheduler.
- (+) Keeping residuals deferred-not-hacked keeps the codebase honest and the accuracy number
  meaningful (`docs/adr/0003`).
- (−) Save-states and any cross-version determinism break at this milestone — a deliberate,
  one-time, well-signposted cost.
- (−) Risk of premature optimization: do **not** start the refactor until the v1.0 accuracy
  battery shows residuals that only sub-cycle resolution can close.

## Status update — 2026-07-11 (`v1.1.0`)

Assessed this gate explicitly against every residual named in `docs/STATUS.md`'s accuracy
dashboard: **the gate is not met.** Zero currently-named residuals require sub-cycle resolution to
close — each is a ROM-sourcing gap, a coprocessor-board scope gap, or a bug/validation question
answerable within the existing whole-master-clock-tick model. Full classification and reasoning:
`docs/audit/fractional-timebase-go-no-go-2026-07-11.md`. Decision and Consequences above are
unchanged; re-run that assessment if a future residual specifically implicates sub-cycle bus-phase
timing.
