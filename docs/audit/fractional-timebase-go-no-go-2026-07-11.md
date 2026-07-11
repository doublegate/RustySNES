# Fractional-timebase refactor — go/no-go assessment (`v1.1.0`, 2026-07-11)

**Status: assessed. Conclusion: NO — `docs/adr/0002`'s gate is not met. Do not start the
refactor.**

## Purpose

`docs/adr/0002-fractional-timebase-refactor.md`'s own Consequences section states the gate
explicitly: "do **not** start the refactor until the v1.0 accuracy battery shows residuals that
only sub-cycle resolution can close." This document applies that gate, once, with evidence,
against every residual currently named in `docs/STATUS.md`'s accuracy dashboard — the deliverable
the `v1.1.0` plan called for after the open-bus-via-DMA-latch and DRAM-refresh timing items
concluded (`docs/scheduler.md` §Open bus via DMA/HDMA, §DRAM refresh).

## Method

Enumerate every item in `docs/STATUS.md`'s "Named residuals, tracked not hidden" paragraph (and
the per-layer table above it), plus the two items this `v1.1.0` pass itself worked on, and classify
each as one of:

- **(A) Requires sub-cycle/fractional-clock resolution to close** — the only category ADR-0002's
  gate actually watches for.
- **(B) Requires a ROM dump, not a timing-model change** — a sourcing gap, orthogonal to timebase
  resolution entirely.
- **(C) Requires board/scope work unrelated to timing** — a coprocessor implementation gap.
- **(D) A whole-tick-model bug, fixable (or already fixed) without finer resolution** — the
  current architecture's own granularity is sufficient; the gap is in correctness, not precision.

## Classification

| Residual | Category | Reasoning |
|---|---|---|
| CPU oracle's `e1.e` (`SBC (dp,X)`, emulation) divergence | **(D)**, arguably not even a residual | Per `docs/STATUS.md:147-149` and `docs/cpu.md:134-136`, this is a documented **inter-reference divergence**: the SingleStepTests vector for this one case models bsnes' `readDirectX` `DL!=0` high-byte-wrap behavior, which the *rest* of the SingleStepTests set does not consistently model. This is an **address-computation** edge case (direct-page indexed addressing wraparound), not a bus-phase-timing question — closing it, if ever done, means picking which reference's address-wrap convention to follow, not resolving sub-cycle bus timing. Does not implicate ADR-0002 at all. |
| DSP-3 / ST011 no board wired | **(C)** | Pure board-implementation scope gap (`necdsp_variant.rs`) — no verified board/window entry exists to implement against yet, unrelated to clock resolution. |
| SPC7110 local dump is a fan-translation, not original cartridge | **(B)** | Purely a ROM-sourcing gap (`docs/audit/spc7110-boot-crash-2026-07-08.md`'s own conclusion: root cause #2 is closed, was never a RustySNES bug). The needed dump (sha256 `69d06a3f3a4f3ba769541fe94e92b42142e423e9f0924eab97865b2d826ec82d`) is tracked in `docs/rom-test-corpus.md`. |
| PAL / ExLoROM lack golden-ROM-boot proof | **(B)** | Both are "no properly-tagged/no known ROM exists" sourcing gaps (`docs/rom-test-corpus.md`), not timing-model gaps — the underlying mechanisms (region auto-detect, the ExLoROM decode formula) are already implemented and unit-verified. |
| Hi-res (Modes 5/6) real-title validation | **(D)**, a validation-methodology gap | The color-math precision mechanism itself is implemented and unit-verified (`v0.7.0`); the open item is that no available commercial dump has been observed actually *entering* hi-res mode in a headless run (`docs/ppu.md` §Hi-res). This is "we haven't proven it against real content," not "the current model lacks precision to represent it." |
| Open-bus-via-DMA-latch (this session, `docs/scheduler.md` §Open bus via DMA/HDMA) | **(D)** | The mechanism under investigation is *value propagation* (whether/when `Bus::open_bus` gets updated by a DMA-driven access, and how that value later surfaces), not *sub-cycle phase* — the current whole-master-clock-tick model already has enough resolution to represent "did this access happen before or after that one." This session found and fixed one real, independent whole-tick-model bug in `SuperFxBoard::map` along the way (zero regressions) and substantially narrowed the remaining mechanism to a concrete, reproducible CPU-control-flow divergence — still open, but not gated on finer-than-whole-tick resolution. |
| DRAM refresh (40 clocks/scanline) | **(D)**, and now further downgraded | This session's empirical measurement (`docs/scheduler.md` §DRAM refresh) found the current CPU-driven model *already* reproduces the correct 357,368-clock NTSC frame length to within natural whole-tick instruction-boundary quantization noise (±20-40 clocks, averaging to within a fraction of a clock of zero over 500 frames × 3 ROMs) — there is no gap for an explicit refresh stall to fill at all, let alone one that would need sub-cycle resolution to model correctly. If this residual is ever revisited, it's a whole-tick-level cost-reallocation question (per `docs/scheduler.md`'s conclusion), not a timebase-refactor candidate. |

## Result

**Zero of the currently-named residuals fall into category (A).** Every one is either a pure
ROM-sourcing gap (B), a coprocessor-board scope gap (C), or a bug/validation gap answerable within
the existing whole-master-clock-tick model (D) — several of which this very `v1.1.0` pass either
fixed outright (the `SuperFxBoard::map` RAM-ownership gap) or definitively ruled out as needing any
new modeling at all (DRAM refresh). None require the φ1/φ2 sub-cycle bus-phase split ADR-0002's
refactor exists to provide.

## Recommendation

**Do not start the fractional-timebase refactor.** This matches ADR-0002's own explicit "risk of
premature optimization" caution (Consequences, final bullet) applied honestly rather than as a
foregone conclusion — this assessment was undertaken as a genuine evidence-gathering exercise, and
the evidence came back negative. No change to ADR-0002's Decision or Consequences sections is
warranted; this document is the dated status check its own text anticipated ("only if residuals
warrant it").

## What would change this conclusion

Per `docs/scheduler.md`'s own closing note: if a *future* change to the per-opcode cost model or a
new hard-tier residual surfaces that is specifically a sub-cycle bus-phase question (e.g. a
verified real-hardware behavior that depends on which half of a single master-clock tick an access
lands in, which the current whole-tick model cannot represent even in principle — not just "a bug
in the current model's use of whole ticks"), re-run this same assessment against the new evidence.
Until then, this conclusion stands as the current baseline.
