# `docs/audit/` — Decision-Rationale Audit Documents

This directory holds **post-hoc audit reports** that capture *why* a particular investigation
went the way it did, the hypotheses that were tried and ruled out, and the exact state an
unresolved bug was left in — detail too long-form for the per-subsystem spec docs. Modeled on
the same directory in `../../../RustyNES/docs/audit/` (a different project; only the
organizing convention is borrowed, not any NES-specific content). It is intentionally distinct
from its siblings:

| Directory | Captures | Audience |
|-----------|----------|----------|
| `docs/` (top level) | **WHAT** the system does — per-subsystem specs (`cpu.md`, `ppu.md`, `cart.md`, etc.). | Future maintainers building against the spec. |
| `docs/audit/` (this dir) | **WHY** an investigation reached its current state — root-cause trail, ruled-out hypotheses, exact reproduction steps for an open bug. | Future Claude / future maintainers resuming a stalled investigation. |
| `docs/adr/` | **DECIDED** cross-cutting architecture choices in Michael Nygard ADR form. | Same audience as this dir, but for settled decisions rather than open investigations. ADRs are short and decision-focused; audits are long and provenance-focused. |

## Contents

- **`spc7110-boot-crash-2026-07-08.md`** — the SPC7110 coprocessor's boot-crash investigation:
  the `v0.4.0`-landed `bus_mirror` addressing fix (root cause #1, confirmed and fixed) and the
  still-open gap (root cause #2, narrowed but not fixed) that keeps Far East of Eden Zero from
  booting to real content. See `docs/cart.md`'s SPC7110 entry for the current summary; this file
  is the full trail behind it.
