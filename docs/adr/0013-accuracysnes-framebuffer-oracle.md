# ADR 0013 — AccuracySNES and the renderer-dependent tests: a host-side framebuffer oracle

## Status

**Accepted** (2026-07-20). Unblocks the remaining ~42 Group C assertions (`C5`, `C6`, `C8`, `C10`, `C12`, most of
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

## Resolutions (settled at ratification)

1. **One scene per assertion.** Consistent with the opcode sweep, which emits one test per opcode
   "so a failure names the instruction rather than the batch" — the same argument applies with more
   force here, because a wrong picture is harder to read than a wrong number. The usual objection to
   per-assertion granularity is storage, and it does not apply: a golden is an 8-byte FNV-1a hash,
   not a framebuffer, so ~42 scenes cost a few hundred bytes.

2. **Goldens live in `tests/golden/accuracysnes-scenes.tsv`**, separate from
   `undisbeliever-framebuffer.tsv`. First-party and third-party goldens have different re-bless
   rules — ours may be regenerated when *we* intend a behaviour change, theirs may not — and mixing
   them invites applying the wrong rule.

3. **Rendered scenes gate CI from the start, but only scenes that have been cross-validated.** An
   informational job that cannot fail gets ignored, and an unfailable check is worse than no check.
   The safety comes from rule 4 instead: a scene's golden is only committed once the references
   agree on it, so an ungated scene simply is not in the set yet. Start small and grow.

## How a scene is captured

The cart runs the whole battery before anything renders, so a scene cannot be left on screen for the
host to find. Rather than have the host drive the cart — which would break the "runs unmodified
anywhere" property — the cart drives itself:

After the battery completes, the runtime enters a **scene loop**. For each scene it sets up the PPU
state, holds it for a fixed number of frames, and publishes the current scene ID to the results
block. The host steps frames, watches the scene marker, and hashes the framebuffer on the last frame
of each hold. Wholly deterministic, and on real hardware the same loop is simply a slideshow the
viewer can watch.

## Supplement, 2026-08-02 — hi-res needs an extraction rule, not a wider region

The `C5.15`, `C10.04` and `C9` hi-res rows were parked behind "widen the capture region past
256x224". **That framing is wrong**, and the measurement that shows why is worth recording before
anyone builds to it.

A minimal Mode 5 frame was rendered as a throwaway scene and each host asked what it emits:

| host | ordinary frame | Mode 5 hi-res frame |
|---|---|---|
| snes9x (libretro) | 256x224 | **512x224** — width doubled |
| Mesen2 (Lua) | 256x239 | **512x478** — width **and height** doubled |

The two references do not agree on the *shape* of a hi-res frame, never mind its pixels: Mesen2
line-doubles and snes9x does not. A capture contract that simply says "512 wide now" would be
comparing a 224-row picture against a 478-row one.

Combined with the earlier empirical finding — on a real Mode 5 scene the even/subscreen columns
agree within 0.4-3% across all three references while the odd/mainscreen columns diverge **33-35%
pairwise** — the shape of the answer is:

- take the **even columns** of the 512-wide picture (the subscreen half, the half the references
  agree on);
- from Mesen2, additionally take the **even rows**, since its 478 is a line-double of 239;
- which yields a 256x224 sample again.

So the region size does not change at all. What a hi-res scene needs is a **declared per-scene
extraction** — a rule for turning whatever the host emits into the canonical 256x224 sample.

Concretely, so three hosts cannot each invent their own: `Scene` gains an `extract` field of a small
closed enum, and the value is emitted as a **column in `build/scenes.tsv`** rather than compiled into
any host. Every host already parses that file to learn the scene list; making the rule travel with
the scene is what stops the C, Lua and Rust implementations drifting apart, which is the same reason
`FIRST_ROW` is a declared per-host constant rather than a guess.

| `extract` | meaning |
|---|---|
| `Direct` | today's behaviour — the frame is 256 wide, take `SCENE_H` rows from `FIRST_ROW`. Every existing scene, and the default, so no golden moves. |
| `HiResEven` | the frame is 512 wide: take even columns, and even rows as well on a host whose height also doubled. Yields the same canonical 256x224 sample. |

A host that meets an `extract` value it does not implement must **reject the scene**, exactly as it
now rejects an out-of-contract geometry — never fall back to `Direct`, which would silently hash the
left half of a hi-res picture and is the failure this whole supplement exists to prevent. The mainscreen halves of `C5.06`/`C5.07` and
`C9.01`/`.02`/`.07`/`.08` stay golden-blocked regardless, because rule 4 forbids blessing a golden
the references disagree about, and on those columns they do.

The prerequisite for any of this was that a host must *reject* an out-of-contract geometry rather
than silently hash it. Both hosts' checks were lower bounds until 2026-08-02 — they caught a frame
that was too small and passed one that was too large, hashing a diagonal slice (Mesen2's Lua, from a
256 stride over a 512-wide buffer) or the leftmost 256 columns (the libretro host, which uses the
real pitch). That is fixed in **PR #320** (`libretro_crossval.c`'s exact `w`/`h` test and
`mesen_scenes.lua`'s exact `SCENE_BUF_LEN`), which landed before this supplement; the loud rejection
it added is literally what produced the table above.

### Implemented 2026-08-02, and it found something immediately

`Scene::extract` landed as specified above — a closed enum, emitted as `build/scenes.tsv`'s fourth
column, honoured by all three hosts, each rejecting a rule it does not implement rather than falling
back to `Direct`.

The first hi-res scene (`c5-mode5-hires-16px-tiles`, `C5.15`) produced a divergence on its first run:

| host | frame emitted | hash of the extracted 256x224 sample |
|---|---|---|
| snes9x | 512x224 | `0xbcb8d1c2bec08325` |
| Mesen2 | 512x478 | `0xbcb8d1c2bec08325` |
| **RustySNES** | 512 wide | **`0xd8dad9b9cb91e325`** |

**The two references agree bit-for-bit despite emitting different geometries and running entirely
different extraction paths.** That is strong evidence the extraction is right and the divergence is
real — and it is this project's own signature for a genuine defect: three hosts failing identically
usually means a broken test, one host failing alone means a bug in that host.

The scene is left **unblessed**. Rule 4 would permit blessing at the reference value, since the
references agree — but that turns the scene gate red on a live finding, and an unblessed scene does
not fail the gate, so the finding is preserved without taking the tree red. The investigation is
tracked separately; blessing follows the fix.

### Correction, same day — that divergence is a 2-vs-2, not a RustySNES bug

The claim above ("the two references agree bit-for-bit ... one host failing alone means a bug in
that host") is **withdrawn**. It was made from two references without consulting the third.

Diffing the pixels rather than the hashes: the divergence is **exactly one column** — column 0 of
the extracted sample, the first pixel of the 512-wide picture — on all 224 rows, and nothing else.
224 differing pixels of 57,344; RustySNES `0x0000`, snes9x `0x0421` (the backdrop).

ares' source settles it against the original conclusion. `sfc/ppu/dac.cpp::scanline()`:

```cpp
//the first hires pixel of each scanline is transparent
//note: exact value initializations are not confirmed on hardware
math.above.colorEnable = false;
```

and `below()` returns `math.above.colorEnable ? math.below.color : (n15)0` — **black** at line
start. RustySNES seeds `above_enable: false` and does the same thing. So the split is **RustySNES +
ares against snes9x + Mesen2**, and ares explicitly flags its own initialisation as *not confirmed
on hardware*.

Consequences, recorded so the scene is not mistaken for a pending fix:

- There is **no defect here to fix**. Changing column 0 to emit the backdrop would move RustySNES
  off ares' behaviour to match the other two on a value ares says is unverified.
- `c5-mode5-hires-16px-tiles` is **not blessable** under rule 4 and stays permanently unblessed
  unless real hardware settles it — a variant set, not a pending golden.
- `C5.15` does not become coverage this way. A hi-res scene that avoids column 0, or a hash that
  excludes the first column, would make the rest of Mode 5 blessable; that is the follow-up.
- The extraction itself is **vindicated**: two hosts with different geometries produced identical
  hashes for the other 57,120 pixels.

The method note is the durable part. Two references agreeing is not "the references agree" — this
project counts ares and bsnes as **one** reference precisely because lineage matters, and here the
lineage that disagreed was the one not consulted. Read the third source *before* publishing a defect
claim, not after.

### `C10.04` is blocked by the same exclusion that unblocked `C5.15`

Attempted 2026-08-02 and **withdrawn**, because the attempt is worth more written down than the
scene would have been.

`C10.04` says a mosaic of **1** is the identity at 256 pixels but a **2x1 pair of half-pixels** in
true hi-res. A Mode 5 scene with `MOSAIC = $01` was written against exactly the control the working
rules ask for — the same canvas with mosaic off — and it hashed **identically to that control on
both references**: `0xf7ed8ab9ecd95d85` either way.

The reason is structural, not a bug in the scene. A 2x1 mosaic pair merges an *even* column with its
*odd* neighbour, and `HiResEven` samples **only the even columns**. The merge is therefore invisible
to the hash by construction: the extraction discards precisely the columns the assertion is about.

So `C10.04` sits with `C5.06`/`C5.07` — its subject lives in the **mainscreen** half, which is the
half the three references diverge on by 33-35% pairwise and which rule 4 therefore forbids blessing.
`HiResEven` does not unblock it, and no variant of `HiResEven` can: any rule that includes the odd
columns inherits the disagreement.

This is the *unshowable scene* trap (`CLAUDE.md`) in its exact documented form — "a scene can arrange
a state no picture can show; an unshowable scene hashes stably and every emulator agrees with it."
The control scene is what caught it, and its doc comment said in advance what an equal hash would
mean. **Write the control first; it is the only thing that distinguishes "the references agree" from
"nothing happened".**
