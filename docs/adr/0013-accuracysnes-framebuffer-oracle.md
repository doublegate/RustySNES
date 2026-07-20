# ADR 0013 — AccuracySNES and the renderer-dependent tests: a host-side framebuffer oracle

## Status

Proposed. Blocks the remaining ~42 Group C assertions (`C5`, `C6`, `C8`, `C10`, `C12`, most of
`C9`, and `C13.01`–`C13.06`), which are at zero coverage and cannot move without it.

## Context

AccuracySNES scores itself. The cart runs with no input, decides pass/fail on-cart, and publishes
verdicts to WRAM; the host harness reads that block and supplies **no expected values of its own**.
That property is the reason the cartridge is worth having — the identical image runs unmodified on
ares, bsnes, Mesen2, snes9x and on real hardware, so a result means the same thing everywhere.

`docs/accuracysnes-coverage.md` reports **123 of 443** enumerated assertions covered. A large,
coherent block of the remainder is unreachable by construction:

| Sub-group | Assertions | What it tests |
|---|---:|---|
| `C5` | 15 | backgrounds and modes 0-7, priority ordering, tilemap layout |
| `C8` | 12 | colour math, windows, clipping |
| `C6` | 7 | offset-per-tile |
| `C10` | 5 | mosaic |
| `C12` | 3 | direct colour |
| most of `C9` | ~6 | hi-res, pseudo-hires, interlace output |
| `C13.01`-`C13.06` | 6 | INIDISP early-read artifacts, brightness ramp |

These decide **what appears on screen and nothing else**. There is no register to read back, no
counter that moves, no flag that changes. A cart cannot see its own framebuffer: the PPU offers no
path from rendered pixels back to the CPU.

So the on-cart, self-scoring approach has reached its ceiling. Everything reachable that way is
either done or scheduled; what remains needs the *host* to look at pixels.

## Decision

**Add a host-side framebuffer oracle as a second, clearly separated tier — and keep it out of the
cartridge's pass rate.**

1. **The cart renders; the host judges.** New `Kind::Rendered` tests set up a PPU state, render a
   known number of frames under a documented deterministic schedule, and write a **scene ID** to the
   results block instead of a verdict. The cart asserts nothing about pixels.

2. **The host compares against committed golden framebuffers**, hashed the way
   `undisbeliever_golden.rs` already does it — the mechanism exists and is proven across 29 ROMs.

3. **Rendered tests are reported separately and never enter the on-cart pass rate.** The headline
   figure stays "N of M scoring, cross-validated against Mesen2 and snes9x on the identical image".
   A rendered result is reported as its own line: *"R of S rendered scenes match committed
   goldens"*.

4. **A golden may be blessed only from a cross-validated render.** A scene's golden is committed
   only when it has been produced and compared across the available reference emulators, with the
   agreement recorded alongside it. A scene where the references disagree is committed as a
   **variant set** — every distinct rendering, each attributed — never as one arbitrary winner.

5. **The provenance tiers still apply.** A rendered test carries the same `Documented` /
   `Corroborated` / `Contested` / `Novel` tier as any other, and `Contested` scenes are recorded,
   not scored, exactly as `A7.04` and `A9.03` are.

## Consequences

### What this costs, stated plainly

- **These tests do not run on real hardware unaided.** That is the property being given up, and it
  is the whole reason for tier separation. A rendered scene on a flash cart displays a picture; only
  a host with the golden can say whether it is the right one.
- **A golden is a snapshot of agreement, not of truth.** `docs/scheduler.md` already records the
  hazard from the `hdmaen_latch_test` re-bless: a golden that tracks our own output proves
  regression-freedom, not correctness. Rule 4 exists to keep that distinction visible, and any
  re-bless must carry the same reasoning trail that re-bless did.
- **Scene setup is where the bugs will be.** This session's record is instructive: `C7.02` (wrong
  `OBJSEL` field), `A9.03` (a seed colliding with an open-bus value), `B2.06` (an uncontrolled
  field), `A5.S34` (a flag clobbered by the measurement harness). All were setup errors that looked
  like emulator bugs. A rendered test has strictly more setup surface and no on-cart assertion to
  catch a mistake early — the failure will be a wrong picture, which is much harder to read than a
  failure code.

### What it buys

- ~42 assertions that are otherwise permanently unreachable, including several `[ERRATA]` items
  (`C6.05`'s never-affected leftmost tile, `C8.01`'s palette-4-7 sprite colour math).
- A per-scene diff for emulator authors, which a pass/fail byte cannot give.
- Reuse of proven machinery rather than new invention.

## Alternatives rejected

- **Score rendered tests in the main pass rate.** Rejected: it would silently redefine the headline
  number from "runs anywhere" to "runs where we have goldens", and the two are not comparable.
- **Have the cart hash its own framebuffer.** Not possible — no CPU-visible read path from rendered
  output.
- **Skip these assertions permanently.** Rejected: it would leave `C5`/`C8` — backgrounds and colour
  math, the parts most games actually exercise — untested by the project's own accuracy cartridge.
- **Use a reference emulator's output directly as the oracle.** Rejected on the grounds established
  in `docs/accuracysnes-timing-oracle.md`: emulator output is a consensus, not a measurement, and
  `A5.08`/`A9.03` both showed the references disagreeing with each other.

## Open questions for ratification

1. **Scene count and scope.** One scene per assertion (~42) or grouped scenes testing several at
   once? Grouped is cheaper to maintain; per-assertion gives a sharper failure.
2. **Where goldens live.** Alongside `tests/golden/undisbeliever-framebuffer.tsv`, or a dedicated
   `tests/golden/accuracysnes/`? The latter keeps first-party and third-party goldens separable.
3. **Whether rendered tests gate CI at all**, or run as an informational job until a scene set has
   proven stable across several releases.
