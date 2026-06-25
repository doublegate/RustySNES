# ADR 0003 — The coprocessor accuracy-tiering honesty gate

## Status

Accepted.

## Context

Per `ref-docs/2026-06-24-coprocessors.md`, the SNES has a dozen-plus in-cart coprocessors of
wildly varying verifiability: some are cycle-accurate and well-tested (DSP-1, Super FX, SA-1),
others are single-game, algorithm-modeled, or depend on chip-ROM dumps the user must supply
(DSP-2/3/4, S-DD1, SPC7110, CX4, OBC1, ST01x, ST018, S-RTC). Reporting one flat "N
coprocessors supported" number would be dishonest — it would let an unverified board inflate
the accuracy figure. RustyNES solved the same problem for mappers with a Core/Curated/BestEffort
tiering + a CI honesty gate.

## Decision

Tier every board / coprocessor **Core / Curated / BestEffort** (the matrix lives in
`docs/STATUS.md`):

- **Core / Curated:** verified to the accuracy bar (test ROMs / commercial-set screenshots);
  these back the oracle. DSP-1, Super FX/GSU, SA-1.
- **BestEffort:** functional but not held to the bar (single-game, algorithm-HLE, or
  chip-ROM-dump-dependent). DSP-2/3/4, S-DD1, SPC7110, CX4, OBC1, ST010/011, ST018, S-RTC.

A **CI test fails if any BestEffort board backs the accuracy oracle / pass-gate.** BestEffort
boards may carry reference screenshots, but they **never inflate the accuracy number**
(`docs/testing-strategy.md` §honesty-gate; `docs/STATUS.md` records `boards_tiered = true`).

**Leverage the shared NEC core:** one **µPD77C25 / µPD96050 LLE engine** covers DSP-1/2/3/4 +
ST010/011 — six chips, one engine (`ref-docs/2026-06-24-coprocessors.md` §C). Implementing it
once promotes DSP-1 to Core/Curated cheaply and makes the BestEffort DSP siblings near-free,
but they are still tiered BestEffort until each is independently verified.

## Consequences

- (+) The accuracy number stays meaningful and auditable.
- (+) The shared LLE core gives breadth without breadth-inflating the verified count.
- (−) Each board needs an explicit tier label and the gate must enumerate which suites it may
  contribute to — a small standing maintenance cost.
- (−) Chip-ROM-dump-dependent boards need a loud "supply the dump" honesty caveat; absent the
  dump they are non-functional, never silently degraded.
