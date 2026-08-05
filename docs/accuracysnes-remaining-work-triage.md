# AccuracySNES — triage of the 82 uncovered rows

**Coverage: 362 of 443** as of `D1.12` landing (was 361 when this was written) (304 on-cart + 55 scene + 2 host). This document sorts
the remaining **82** into what can actually be done, so the next attempt spends its effort on the
55 that are reachable instead of rediscovering why the other 27 are not.

**It is a map, not a plan.** Every verdict below is sourced from `docs/accuracysnes-plan.md`, the
[coverability audit](accuracysnes-coverability-audit-2026-07-23.md), or the CHANGELOG entry that
withdrew the row — not from a fresh judgement. Where a row has no recorded note, it says so rather
than inventing one.

> **Do not read "82 remaining" as "82 achievable."** The coverability audit measured a **soft
> ceiling of ~422 of 443**. This triage lands close to it: **~55 reachable**, 27 not.

## Summary

| Tier | Rows | Meaning |
|---|---:|---|
| **1 — Genuinely uncoverable** | **10** | No test can exist. Enumerated and scored as such; do not attempt. |
| **2 — Attempted and withdrawn** | **10** | Tried, failed for a *recorded* reason. Do not blind-retry; the reason is the spec for any new attempt. |
| **3 — Reference disagreement** | **8** | The references do not agree, so ADR 0013 forbids blessing a golden. Variant sets, not gaps. |
| **4 — Reachable** | **54** (1 landed; `C7.07` moved to tier 2) | The real target. Ordered by what machinery each needs. |
| | **82** | |

---

## Tier 1 — genuinely uncoverable (10)

Do not attempt. Each is enumerated in the dossier precisely so the coverage figure stays honest
about them.

| Rows | Why |
|---|---|
| `C13.01`–`C13.06` (6) | The INIDISP early-read glitch. **Chip-revision-gated** (3-chip vs 1CHIP — `C13.05` is 1CHIP-specific), and no reference emulator models it, so there is nothing to cross-validate against. `C13.06` is recorded as *"genuinely uncoverable, but not for the reason once written"* — read that note before touching it. |
| `D3.01`, `D3.02` (2) | Revision crash bugs the accuracy ledger classifies out of scope. Report as variants. |
| `F1.22` (1) | NTT keypad — unmodelled, and un-attachable on 2 of 3 hosts. |
| `C11.12` (1) | **Verified** unauthorable in `v1.29.0` (PR #328), not assumed: the subject appears exactly twice in `ref-docs/`, both times as a game-compatibility table row, with no behavioural statement anywhere; and no reference models a distinct timing. Writing it would mean inventing an expectation and checking our own arithmetic against itself. |

## Tier 2 — attempted and withdrawn, with a recorded reason (9)

**Each of these already cost a session.** The recorded reason is the specification for any new
attempt — a retry that does not address it will fail the same way.

| Row | Recorded reason |
|---|---|
| `A5.20` | Needs a clock-domain instrument the cart lacks. |
| `B1.05` | *"Attempted twice, withdrawn twice; the region map is right and the measurement is not."* Same missing instrument as `A5.20` — **one obstacle, not two.** |
| `E6.04` | **Vacuous.** Pitch modulation changes *when* a sample is read; against a constant waveform there is no *when* to observe. |
| `E6.08` | The `$801` overflow. Interpolation sits between the decoder and every cart-reachable observable — the sign came from the gaussian interpolator, not the decoder. |
| `E8.06` | Two voices cannot be compared at one sample: the minimum three-read window is 21 of a sample's 32 cycles, and the straddle error equals the signal. **21 is the instruction set's floor** — a tighter loop does not exist. |
| `E9.11` | Separating the models means computing the expected answer from our own arithmetic and asserting it. |
| `C7.12` | Written as a scene; produced three hashes on three emulators. Root cause is **host-side**: Mesen2's headless runner does not deterministically associate a rendered frame with the `R_SCENE` value it read. The cart-side field gate landed; the missing half is not ours. |
| `C10.04` | Blocked by the same pixel-exclusion rule that *unblocked* `C5.15`. |
| `C7.07` | Attempt 1 authored and **reverted the same day**. Built as "20 in-range 16x16 sprites, only sprite 0 on-screen, rest at X = -128 (fully off-screen)" versus a 4-sprite control, expecting the errata to raise Time Over from off-screen tiles. **RustySNES failed code 2 — and so did snes9x, identically**, which is this project's broken-test signature rather than a bug found. Both cull fully-off-screen sprites before counting the budget (`render.rs`'s `compute_over_flag_dots`, the `obj.x > 256 && obj.x + w - 1 < 512` cull). The dossier wording — *"**first** sprite 16x16+ at X=0-255 **with others** at negative X"* — describes an **interaction** that this configuration does not reproduce. A retry must start from what that interaction actually is, not from "more off-screen sprites". |
| `D2.08` | Attempt 1 authored, cross-validated, **reverted** — snes9x failed identically, which is the broken-test signature. Its timing is entangled with `D2.09`'s stale-table quirk; start fresh and debug the `A=0` anomaly first. |

## Tier 3 — reference disagreement (8)

Not gaps in RustySNES. The references disagree with **each other**, so ADR 0013's *"bless only from
a render the references agree on"* cannot be satisfied in either direction. Record as variant sets;
**do not pick a winner.**

| Rows | Disagreement |
|---|---|
| `C5.06`, `C5.07` (2) | Mode-5 hi-res **mainscreen**: even/subscreen columns agree within 0.4–3%, but odd/mainscreen columns diverge **33–35% pairwise across all three references**. |
| `C9.01`, `C9.02`, `C9.07`, `C9.08` (4) | The mainscreen halves of the `C9` rows — same blocker. |
| `C9.03`, `C9.06` (2) | Screen interlace carries the identical field-parity dependency as `C7.12`; they would land unblessed for the identical reason. |

## Tier 4 — reachable (55)

The real target, grouped by the machinery each cluster needs. **Order matters**: the clusters near
the top need nothing new.

### 4a — needs no new machinery (**1 landed; 1 genuinely left (`B4.10`), 5 reclassified, 1 suspect, 1 withdrawn**)

`B2.02`, `B2.03` — *"both landed in the emulator; the cart rows are the remaining half."* The
dot-model work shipped in `v1.28.0`; these are the on-cart assertions for it. Note the **cart-ID vs
dossier-ID** trap: these dossier rows are not the cart tests of the same name.

`B2.09` — but read its note first: *"`B2.09`'s window edges aren't CPU-observable."* That is a
hint the row may belong in tier 2 once someone looks properly; it is left here because no attempt
was ever recorded, and a row nobody tried is not a row that failed.

~~`D1.12`~~ — **LANDED**, golden-not-scored, cross-validated on two references. Its instrument
turned out **unable to separate anomie's alignment term from its own `nop` skew**; that is
recorded in the test's doc comment rather than papered over, and the row records the aggregate
exactly as the plan asked. Isolating the alignment term needs a sub-CPU-cycle skew source the
cart does not have — a separate row, not a fix to this one.

**CORRECTION (2026-08-05, after reading the dossier row-by-row): five of these were misfiled here
by the first pass of this document, which grouped by "has no recorded blocker" rather than by what
the row actually needs.** Checking each against the coverage table and the dossier text:

| Row | Actually belongs in | Why |
|---|---|---|
| `C6.07`, `C8.09`, `C10.03` | **scene tier** | `C6`, `C8` and `C10` have **zero** on-cart coverage — they are pure scene groups. These are framebuffer-oracle rows, not on-cart ones. |
| `C1.09` | **do not author yet** | The dossier row is itself an unresolved **`[CONFLICT]`** — *"fullsnes says register bits 6-1; unresolved."* Authoring against an unresolved conflict means picking a side the sources do not. |
| `C3.10` | **4d** (host peripheral contract) | Super Scope latch position — needs the peripheral attached in every runner, exactly like `F1.13`-`F1.18`. |
| `E3.07`, `E4.07`, `E6.03`, `E6.05` | **4c** (Group E emitters) | All four need an uploaded SPC700 program, so they carry 4c's machinery cost, not 4a's. |

**Genuinely 4a, on-cart, no new machinery:**

`B4.10` — *"No IRQ at dot 153 on the short scanline."* A negative assertion resting on the
short-line model that shipped in `v1.28.0`; the field-parity gate to reach a short line already
exists.

~~`C7.07`~~ — **ATTEMPTED 2026-08-05 AND WITHDRAWN. Now tier 2; see its row there.**

`B4.01` — *"/NMI asserts at **H = 0.5**"*. Half-dot precision against an instrument that reads
whole dots; likely needs the same clock-domain instrument `A5.20`/`B1.05` are parked on. Treat as
suspect until someone shows an observable, rather than as a quick row.

### 4a — starting analysis (done 2026-08-05, authoring not yet begun)

Three findings from reading the dossier and plan for the first candidates. **Each resolves the
cart-ID vs dossier-ID ambiguity explicitly**, because that has bitten here before.

**`D1.12` — best-specified candidate, and its blocker has cleared.** Dossier row is *"CPU timing
before DMA start (MMPR)"*. The plan records it as **GOLDEN, not scored** — *"record the measured
aggregate; assert nothing"* — because the delay is not a constant (one CPU cycle after the `$420B`
write at 6/8/12 clocks set by the *next* access's speed, then 2–8 aligning to a multiple of 8, then
8 whole-transfer, then 8 per channel). Crucially the plan also says it was *"blocked in practice
until `T-06-A`: the aggregate is a clock-domain quantity and the cart reads dots"* — **`T-06-A`
landed in `v1.29.0`**, so that blocker is gone.

Two cautions that are part of the row, not obstacles to it: Mesen2, ares and bsnes all implement
this from **the same anomie document**, so their agreement is **not independent corroboration** —
which is exactly why the plan says record-don't-assert. And there is no cart test mapped to `D1.12`
today (`grep '"D1.12"' gen/src/dossier.rs` → nothing), so this is genuinely new authoring.

**`B2.02`/`B2.03` — the emulator half shipped; the cart half is delicate.** Dossier `B2.02` is the
short scanline (line `$F0` = 1360 clocks on alternating non-interlace frames) and `B2.03` the long
one (PAL interlace field=1, `V=311` = 1368 clocks / 341 dots). Both are modelled in the emulator
since `v1.28.0`. The cart-side assertion has to observe a **4-master-clock difference inside a
357,366-clock frame** — far below what `B2.07`/`B2.08`'s ±2% APU-referenced frame-rate instrument
can see. The plausible shape is a *differential*: `$213F` bit 7 gives the frame parity, so latch the
raw H counter at a fixed `V` **after** line 240 on both parities and compare — the short line should
displace everything after it by exactly one dot. That lands squarely in the dot-domain and
sawtooth-phase trap territory this project has been caught by repeatedly, so it wants a fresh
session and its own injection design, not a tail-end attempt.

**`B2.09` — likely tier 2, not tier 4.** The dossier row itself says the picture-window edges are
*"not CPU-observable directly; reachable through the framebuffer oracle once the dot-resolution
compositor lands."* The compositor landed, so it is a **scene** row rather than an on-cart one —
and scene blessing needs the two-reference agreement that was only restored today.

### 4b — the WRAM-trail family (7)

`D2.01`, `D2.02`, `D2.10`, `D2.11-14`, `D2.15`, `D2.16`, `D2.17`.

`D2.01` is a **bracket, not an exact dot** — the references disagree on whether the transfer is done
before dot 274. Treat the whole family as one instrument built once, then reused.

### 4c — Group E, needs new SPC700 emitters (23)

`E1.11` (*"looks like the easy one and is not"* — read its note first), `E3.12`, `E4.05`, `E4.09`
(probabilistic — *"can corrupt"* needs a statistical framing), `E4.10`, `E6.01`, `E6.06`, `E6.07`,
`E6.10`, `E7.02` (needs two voices at different rates), `E8.08`, `E8.09`, `E8.11`, `E9.07`, `E9.08`,
`E9.14`, `E9.16`, `E9.20`, `E10.02`, `E10.03`, `E10.04`, `E10.06`, `A6.13`.

**`gen/src/spc.rs` carries only opcodes a committed test exercises**, so most of these need new
encodings — each verified against `rustysnes-apu/src/spc700_exec.rs`'s dispatch table, because an
unverified byte surfaces as an emulator disagreement rather than an assembler bug.

Two standing traps: every uploaded program must hand the APU back via `release_to_ipl` (which
re-maps the IPL ROM first — a test that writes `$F1` for its own reasons strands every *later*
upload while the battery still reports 100%), and every handshake wait must be bounded or it hangs
the whole battery.

`E10.02`/`E10.03` are the failing oracle that would mandate `T-CA-05`; `A6.13` (`STP` halts until
reset) is the one for `T-CA-06`. **Remediate only what turns red** — a green row is not a reason to
change the emulator.

### 4d — needs a host peripheral contract (8)

`F1.13`, `F1.15`, `F1.16`, `F1.17`, `F1.18` — lag-frame results, multitap detect/select/17th bit,
mouse sign-magnitude. These need **emulated peripherals attached in all three runners**, the same
shape as the input contract that already exists (`PAD_CONTRACT`/`PAD2_CONTRACT`) and for the same
reason: without one, every observable reads `$0000` and the tests are unassertable.

`F1.19`, `F1.20`, `F1.21` are oracle-thin — golden-at-best.

### 4e — second-image differential (2)

`G1.13` — `$FFD5` bit 4 FastROM. Recorded as a `[CONFLICT]`; resolve the conflict before authoring.

---

## Blockers on the whole programme (as of 2026-08-05)

| Blocker | State |
|---|---|
| **snes9x oracle** | ✅ **Working** via the installed libretro core. Battery OK, 55/55 scenes match, NTSC + PAL. |
| **Mesen2 scene runner** | ❌ Needs the real `Mesen.dll --testRunner`; a libretro core does not reach the scene loop. **Blocks blessing any new scene** under ADR 0013 (existing 55 remain verified by snes9x). |
| **ares third reference** | ❌ The headless host links ares' **static libraries**, i.e. it needs ares *source*. `/usr/bin/ares` is the GUI with no headless mode. Per `docs/ai-emulator-provenance-guardrails.md` that source must be placed **outside the tree by a human**; the agent must not fetch it in. |

**Consequence for sequencing:** tiers 4a–4d are on-cart rows and need only the battery oracle, so
they can proceed on snes9x alone — but with **one** reference, a divergence cannot be adjudicated
(this project's own heuristic is *"three failing identically = broken test; RustySNES failing alone
= real bug"*, which needs at least two). Restoring Mesen2 raises that to two and unblocks scenes.

## Working rules that apply to every row here

Each already cost a session; all are in `CLAUDE.md`, repeated because this is the document someone
will actually have open:

- Emit `.a8`/`.a16` from every `sep`/`rep`.
- **Never hand-write a verdict byte** — use the assertion helpers, so `ERROR_CODES.md` stays the
  complete account of failure bytes.
- **A guard must not subsume the assertion it protects.**
- **Inject at the site the row names.** If the injection does not move the verdict, the attribution
  is wrong *even when the test passes*.
- Rebuild after any `gen/`/`asm/` change; **never pipe `accuracysnes-gen` through `tail`** — it
  hides the panic.
- Groups A and B must not be relocated; the build gate rejects it.
- **Cart IDs and dossier IDs are different numbering schemes.** Read the mapping through
  `gen/src/dossier.rs`'s `MAP`/`SPLITS`/`UNENUMERATED`, never through the ID text.
