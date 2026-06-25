# Phase 7 — Breadth

## Goal

Fill in the remaining **BestEffort** coprocessors and niche peripherals so the full
coprocessor / board matrix in `docs/STATUS.md` is complete — leaning on the shared µPD77C25 /
µPD96050 LLE engine to make the BestEffort DSP siblings near-free. Region timing remains data,
not a build fork. BestEffort boards never back the oracle (`docs/adr/0003`).

## Exit criteria

- [ ] DSP-2/3/4 + ST010/011 ride the shared NEC core (BestEffort, honesty-gated).
- [ ] S-DD1, SPC7110 (+ frozen RTC-4513), CX4, OBC1, ST018, S-RTC implemented to BestEffort.
- [ ] RTC chips read frozen / seeded time (`docs/adr/0004`); chip-ROM-dump boards carry the
      honesty caveat.
- [ ] Niche peripherals (multitap, mouse, Super Scope) past the stub stage where in scope.
- [ ] The full board matrix in `docs/STATUS.md` is complete; the honesty gate stays green.
- [ ] All sprints complete.

## Scope

In-scope:

- The BestEffort coprocessor family + the shared-core economy.
- Niche peripherals; region-timing-as-data completeness.

Out-of-scope:

- The Satellaview / Sufami Turbo / Super Game Boy pass-through (deferred per
  `ref-docs/research-report.md` "Scope").
- The fractional-timebase refactor (`docs/adr/0002`).

## Sprints

- [Sprint 1 — BestEffort coprocessors via the shared core](sprint-1-besteffort-coprocessors.md)
  — DSP-2/3/4 + ST010/011 + the simpler ASICs.
- Sprint 2 — SPC7110 / S-DD1 / CX4 / ST018 + peripherals.
  **Status:** stub — refine when Sprint 1 is ~complete.

## Dependencies

Phase 4 (the cart foundation + the shared NEC core + the honesty gate).

## Risks

- **Per-board bus windows** (no canonical table) — the long tail of board quirks. Detect: a
  board boots but mis-maps. Mitigate: per-board fixtures + the cartridge database.
- **RTC determinism** — a frozen-time regression. Mitigate: a determinism test per RTC board.

## Reference docs

- [docs/cart.md](../../docs/cart.md) — the coprocessor families + tiers.
- [docs/adr/0003](../../docs/adr/0003-accuracy-tiering-honesty-gate.md),
  [docs/adr/0004](../../docs/adr/0004-determinism-contract.md).
- [docs/STATUS.md](../../docs/STATUS.md) — the matrix this phase completes.
