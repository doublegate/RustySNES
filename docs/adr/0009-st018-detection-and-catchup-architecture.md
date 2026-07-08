# ADR 0009 — ST018 detection method and the `coprocessor_tick` catch-up architecture

## Status

Accepted (`v0.4.0 "Completion"`).

## Context

Two independent architectural decisions were needed to land ST018 (a full ARMv3/ARM6-class CPU
coprocessor), neither of which followed an existing precedent in this codebase exactly:

1. **Detection.** ST018's real hardware signature is fully unambiguous from header bytes alone —
   both Mesen2 (`BaseCartridge::GetCoprocessorType`) and ares (`mia/medium/super-famicom.cpp`)
   detect it via the extended-header byte `$xFBF` (`RomType` high nibble `$F` + `CartridgeType ==
   0x02`), no title match needed. But this project's own `coprocessor_from_chipset` (`crate::
   header`) deliberately does **not** read `$xFBF` for the *other* `$F`-nibble customs
   (CX4/SPC7110/S-RTC) — an earlier investigation found that byte unreliable against a real Mega
   Man X2 dump and fell back to title-matching instead. Introducing a new, differently-sourced
   detection mechanism just for ST018 (reading `$xFBF` where every sibling coprocessor
   deliberately doesn't) would fragment the detection strategy across the same nibble family.
2. **Catch-up.** ST018 is architecturally SA-1-like — the ARM core must run in deterministic
   lockstep with the master clock, not host-synced on a Go/RQM flag the way GSU/DSP-1 are. SA-1's
   own second-CPU catch-up lives in `rustysnes-core`, driven through `Board::second_cpu_*` hooks,
   specifically because SA-1's second core is a real `rustysnes_cpu::Cpu` instance — and the
   one-directional crate graph forbids `rustysnes-cart` from depending on `rustysnes-cpu`. ST018's
   ARM core (`crate::coproc::armv3::Cpu`) has no such constraint: it's a from-scratch core that
   lives entirely inside `rustysnes-cart` already.

## Decision

1. **Detection: title-match, mirroring the existing `$F`-nibble convention, not a new `$xFBF`
   read.** ST018 is detected via a title match on the confirmed real cart, *Hayazashi Nidan
   Morita Shogi 2* (`NIDAN MORITASHOGI2` per an external database, not yet verified against a
   real dump — no commercial copy exists in this project's local corpus). This keeps every `$F`-
   nibble coprocessor's detection sourced the same way, rather than having ST018 alone read a
   header byte the project has explicit, dump-verified evidence not to trust for its siblings.
2. **Catch-up: `Board::coprocessor_tick`, not the SA-1 second-CPU hooks.** Since the ARM core is
   self-contained in `rustysnes-cart`, `St018Board` owns and steps it directly, driven by the
   existing `coprocessor_tick` host-sync hook (already used for GSU/Super FX) — which fires once
   per master-clock unit from inside `rustysnes-core`'s own per-tick loop. This achieves the same
   deterministic master-clock lockstep SA-1 has, with **zero `rustysnes-core` changes**: no new
   second-CPU instance, no new hook family, no crate-graph exception.

## Consequences

- (+) Detection stays uniform across the whole `$F`-nibble family — a future contributor doesn't
  have to remember "every custom coprocessor is title-matched except this one."
- (+) The catch-up architecture required touching only `rustysnes-cart` — SA-1's own second-CPU
  plumbing in `rustysnes-core` is completely unaffected, and no new pattern needed inventing:
  `coprocessor_tick` already existed for exactly this "runs concurrently with the CPU's own
  instruction stream, not drained atomically" class of problem.
- (−) ST018's title-match detection is **unverified** — no ROM exists locally to confirm the
  exact internal title string. A wrong string means silent non-detection (the cart falls back to
  its plain base board), not a crash — matching the honesty-gate posture, but still a real,
  open gap until a dump surfaces.
- (−) A future coprocessor that genuinely needs `$xFBF` (if one is ever found reliable for some
  other chip subtype) would introduce the exact fragmentation this decision avoided for ST018 —
  that tradeoff would need its own ADR at that time, not silently reopened here.
