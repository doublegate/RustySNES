# AccuracySNES — triage of the 82 uncovered rows

**Coverage at time of writing: 361 of 443** (304 on-cart + 55 scene + 2 host). This document sorts
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
| **2 — Attempted and withdrawn** | **9** | Tried, failed for a *recorded* reason. Do not blind-retry; the reason is the spec for any new attempt. |
| **3 — Reference disagreement** | **8** | The references do not agree, so ADR 0013 forbids blessing a golden. Variant sets, not gaps. |
| **4 — Reachable** | **55** | The real target. Ordered by what machinery each needs. |
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

### 4a — needs no new machinery (15)

`B2.02`, `B2.03` — *"both landed in the emulator; the cart rows are the remaining half."* The
dot-model work shipped in `v1.28.0`; these are the on-cart assertions for it. Note the **cart-ID vs
dossier-ID** trap: these dossier rows are not the cart tests of the same name.

`B2.09` — but read its note first: *"`B2.09`'s window edges aren't CPU-observable."* That is a
hint the row may belong in tier 2 once someone looks properly; it is left here because no attempt
was ever recorded, and a row nobody tried is not a row that failed.

`B4.01`, `B4.10`, `C1.09`, `C3.10`, `C6.07`, `C7.07`, `C8.09`, `C10.03`, `D1.12` (recorded as
*"GOLDEN, not scored"* — may only need promoting), `E3.07`, `E4.07`, `E6.03`, `E6.05`.

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
