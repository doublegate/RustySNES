# AccuracySNES coverability audit — 2026-07-23

## Why this document exists

A prior working conclusion held that AccuracySNES was "architecturally blocked at ~341/443 —
the remaining rows are provably uncoverable or hard, not quick wins." A five-way parallel research
audit (each bucket cross-referenced against the dossier, all five reference emulators in
`ref-proj/` — ares, bsnes, Mesen2, MesenCE, snes9x — plus fullsnes/anomie, the test-ROM corpora,
and primary web sources) **overturned that conclusion for four of the five buckets examined.**

The honest revised picture: **443/443 is still not reachable — but the genuinely-impossible core is
~9 rows, not ~100.** Adding the ~12 rows that are expressible-but-unblessable (reference disagreement)
or oracle-thin brings the effective wall to ≈ **21 rows** (9 impossible + ~12 soft-ceiling), for a
practical ceiling of roughly **≈422/443** — not 341, and not 443. The prior claim conflated three distinct states — "provably
impossible", "reachable only as a golden / needs an oracle we lack", and "not yet implemented" — and
labelled all three "uncoverable". Only the first is a true wall; the second is a soft ceiling; the third
is just a backlog.

This audit is a dated supplement (per the module-40 immutable-reference discipline); it does not rewrite
the dossier. Factual dossier/plan corrections it surfaced are listed in the last section.

## The revised ceiling at a glance

| Class | Count (of the ~46 audited "hard" rows) | Meaning |
|---|---:|---|
| **Genuinely uncoverable** | ~9 | No path exists; proof given per row |
| **Golden-only / oracle-thin / very-hard** | ~12 | Expressible but references *disagree* (true-hires mainscreen, ~6), or no agreeing oracle (mouse micro-timing, Super Scope, ~3), needs major new machinery (interlace, ~2), or on-cart-unreachable (G1.18, ~1) |
| **Coverable with bounded work** | ~24 | RustySNES + references already agree, or a self-scoring / loader-tier / second-image path exists |

The other ~56 uncovered rows (A5/A6 opcode tail, B2/B4 timing, most of Group E APU, D1.12, …) were
never claimed uncoverable — they are Bucket 1/2 "not yet done", reachable with the primitives already
built. They are out of scope here.

## Bucket 1 — C11.08 (Mode-7 MPY during active display): **OVERTURNED → coverable-with-work**

Prior verdict "genuinely uncoverable" was correct only for the *exact-value* framing and wrong for the
self-scoring route in general. It conflated the golden route (needs an agreeing reference — genuinely
dead, every emulator returns the combinatorial CPU product with no render-time latch) with the battery
route (self-scoring, needs no reference).

The per-pixel sequence **is documented** — fullsnes `30-ppu.md:394-413`: during visible drawing
`$2134-$2136` receives per-pixel products (`M7A*(SCREEN.X&FF)/8`, `M7C*(SCREEN.X&FF)/8`, and the
line-start prologue terms); only in V-/forced-blank is it the CPU product `M7A*(M7B>>8)`.

**Non-vacuous structural assertion:** set `ORG.X=ORG.Y=0` (zeroes the prologue terms), so the only legal
visible-loop values form a magnitude-bounded set `S`; choose the matrix so the CPU product `P_cpu ∉ S`;
read `$2134` at a controlled dot during active display; assert `value ∈ S && value ≠ P_cpu`. Robust to
the unknown dot→phase offset (every visible-loop phase lands in `S`), fails all five references + today's
RustySNES, passes only a correct render-time model.

- **Work:** implement the render-time MPY latch in `crates/rustysnes-ppu/src/regs.rs:397-403` (currently
  returns the combinatorial product unconditionally — a real accuracy improvement per fullsnes; the
  per-dot compositor did *not* add it) + author the structural test.
- **Honesty caveat (must be recorded on the row):** the oracle is a *written spec*, not a hardware
  capture or agreeing reference, so it is `crossval.sh`-exempt and proves "valid render-product class,
  not the CPU product" — weaker than exact-value, but exactly the defect C11.08 names. The exact-value
  variant stays blocked-on-capture (same standing as C13). `C11.07` is unaffected — ordinary V-blank
  on-cart row.

## Bucket 2 — C13.01-06 (INIDISP sub-scanline): **CONFIRMED uncoverable — but the stated reason was wrong**

The plan's blocker (a) — "the whole-line compositor rules out sub-scanline effects" — is now **obsolete**
(the per-dot compositor is the sole renderer, mid-line brightness/force-blank is column-accurate). It was
never the binding constraint. The durable reasons:

1. **No reference models the INIDISP early-read glitch.** ares/bsnes/MesenCE/RustySNES all latch `$2100`
   atomically at the write cycle. There is no agreeing render to bless a golden from — the "agreement"
   would be on the glitch's *absence* (vacuous golden). Same standing as C11.08's exact-value route.
2. **Irreducible revision-dependence:** the early-read bug is 3-chip-only (absent on S-CPU-B/1CHIP);
   C13.05's ~72px ramp is a **1CHIP-only analog DAC settling** curve (2.5 dots on 3-chip, 72 on 1-chip,
   332 on an anti-ghosting-modded unit) — not a hashable digital pixel set at all.

All of C13.01-06 stay uncovered on purpose. **New coverage found alongside:** a *clean* mid-line
brightness-step / force-blank toggle (write during active display, new value from that column onward) is
now framebuffer-observable and RustySNES + ares + bsnes + MesenCE all agree → a blessable ADR-0013 scene.
That is a distinct new assertion about clean mid-line register timing, not a C13 errata row.

## Bucket 3 — F-group (input) F1.08-F1.22: **OVERTURNED → 8 of 12 coverable**

The blanket "a cartridge cannot press its own buttons" conflated bare-hardware portability with
in-harness scoreability. Every runner already holds the fixed `PAD_CONTRACT`/`PAD2_CONTRACT` masks and
the in-repo harness can already vary input mid-run (menu nav, `accuracysnes.rs:1644-1764`).

| Rows | Verdict | Note |
|---|---|---|
| **F1.08, F1.09, F1.10** | **COVERABLE NOW (golden), zero contract change** | Auto-read *timing* — no buttons involved; `bus.rs:838` admits the omission. Would expose a real RustySNES start-dot gap (starts window at vblank-entry; Mesen2 pins dot 32.5–95.5; F1.09 duration 4224 has 2/3 agreement). |
| **F1.13, F1.15, F1.16, F1.17, F1.18** | **COVERABLE with a bounded host-contract extension** | Time-varying input timeline (F1.13); multitap-attach + 4 sub-pads (F1.15/16/17); mouse-attach + deltas (F1.18). All peripherals modeled by RustySNES + all 3 refs. |
| **F1.19, F1.20, F1.21** | **Oracle-thin — golden-at-best** | Mouse micro-timing (170/336-cycle minima) and Super Scope 6-beam latch; references don't robustly model them, and F1.21 also needs a lit on-screen target. |
| **F1.22** | **GENUINELY UNCOVERABLE** | NTT Data Keypad: unmodeled in RustySNES; only ares (of 3 hosts) can attach it — no consensus possible. |

## Bucket 4 — hi-res / interlace / mid-line scenes: **OVERTURNED → 12 of 14 coverable**

Two conflated claims. The per-dot compositor genuinely unblocked the *mid-line-timing* rows; the *hi-res*
rows were never blocked on the compositor at all — RustySNES has rendered true 512-wide hi-res since
v0.7.0. The blocker was purely the scene host's hard-coded 256×224 hash region.

| Rows | Verdict |
|---|---|
| **B2.09, C10.03, C11.12, C11.03** | **Unblocked by the per-dot default** (256-wide, no host change). C11.03 never needed per-dot — just unwritten. The 3 mid-line-timing rows carry cross-val-split risk → may land as recorded goldens. |
| **C5.15, C10.04** (and the *subscreen* aspects of hi-res) | **Coverable via the bounded 512-widening** where the assertion does not depend on true-hires *mainscreen* compositing. |
| **C5.06, C5.07, C9.01, C9.02, C9.07, C9.08 (mainscreen halves)** | **GOLDEN-BLOCKED by reference DISAGREEMENT — worse than this agent estimated.** See the empirical caveat below. |
| **C9.06, C9.03 (interlace-V clause)** | **STILL BLOCKED** — need a doubled-height (448/478) region + field-parity pinning in `run_scenes` + still risk a 3-way reference split (the C7.12 precedent). C9.03's coarse-H-scroll half is coverable at 512. |

The bounded widening itself is real: add a per-scene width/geometry column to `build/scenes.tsv` (emitted
by `gen/src/scenes.rs`), parametrize the three hosts' hash region (`accuracysnes_scenes.rs:106`,
`libretro_crossval.c:117-147`, `mesen_scenes.lua:82-107`). Overscan (239-tall) needs no widening.

**EMPIRICAL CAVEAT (overrides this bucket's optimism).** The hi-res research relied on a *structural*
argument ("all three emit 512-wide dual-column output, so agreement is expected"). But a prior actual
experiment (2026-07-22, recorded in the `accuracysnes-remaining-work-map` note) **built the widened
oracle and pixel-diffed a real Mode-5 scene on all three references**: the even/**subscreen** columns
agree (~0.4–3% diff), but the odd/**mainscreen** columns diverge **~33–35% pairwise across all three**,
in a periodic tile-aligned (mod-16) pattern — a genuine hi-res mainscreen *compositing-convention*
difference between the reference emulators themselves. Per ADR 0013 (golden = agreement), **no
true-hires mainscreen golden can be blessed today.** So C5.06/C5.07 and the mainscreen halves of the
C9 hi-res rows are **golden-blocked by reference disagreement**, not cleanly coverable — the widening is
necessary but not sufficient. Only assertions confined to the agreeing subscreen columns, or to
non-hires-width behavior, are blessable. This is the one place the audit's per-bucket optimism must be
walked back.

## Bucket 5 — G-group conflict/reset + D2/D3 DMA: **OVERTURNED → only D3.01/D3.02 genuinely uncoverable**

| Row(s) | Verdict | Key fact |
|---|---|---|
| **G1.06** (PPU survives /RESET) | **COVERABLE-WITH-WORK** | The soft reset **already exists and already preserves PPU + WRAM** (`facade.rs:205-212`, `scheduler.rs:120-135` reset only CPU/region). Needs a runner reset-hook + a 2-phase WRAM-sequenced cart test. PPU-*memory* survival is Documented (all refs); PPU-*register* survival is honored only by Mesen2 + RustySNES → golden that exposes the other three. |
| **G1.13** (FastROM header bit) | **COVERABLE (second image / differential golden)** | The bit-position "CONFLICT" is already settled: **bit 4**, documented. No hardware reads the header speed bit (timing is gated by `$420D`), so a bare `$FFD5` read is vacuous — use a second image or a `$420D`-timing differential. |
| **G1.18** (copier header) | **on-cart GENUINELY-UNCOVERABLE (proof); loader-tier coverable** | Post-strip address space is byte-identical to the un-headered image; if unstripped the reset vector is misaligned and it never boots. Coverable at the loader tier (emit a 512-prepended variant, assert detect/strip in the harness). |
| **G1.01** (power-on reg values) | **PARTIALLY COVERABLE** | `$4202`/`$4204-05` already covered via B5.05/`capture_power_on`; `$420D`=$00 via the B1.01 timing probe; `$4207-0A`/`$4201` via IRQ position / latch line. Only `$4200` + the write-only-latch half are truly unobservable (that *is* G1.08). |
| **D2.08** | **COVERABLE NOW** | `$420C` mid-frame → channel starts next line; WRAM `$2180` trail begins one line later. |
| **D2.01, D2.02, D2.11-14, D2.15, D2.17** | **COVERABLE-WITH-WORK** | Observable via the `$2180` WRAM byte-trail (the mechanism covered D2.03/04 already use) and/or H-counter timing; all pinned to ≥2 references. |
| **D2.10, D2.16** | **Reported-variant / scene** | D2.10 as a reported probe (like D2.09); D2.16 (HDMA write takes effect next line) as a per-dot scene (contested per-title representation). |
| **D3.01, D3.02** | **GENUINELY UNCOVERABLE (scored)** | 5A22 v1/v2 chip-revision crash bugs; no reference models them; a self-scoring assertion is vacuous. Auto-skip on `$4210` version. |

## The genuinely-impossible core (the true wall)

- **C13.01-C13.06** (6) — no reference models the early-read glitch; irreducibly revision-dependent; C13.05 is analog.
- **F1.22** (1) — NTT keypad unmodeled + un-attachable on 2/3 hosts.
- **D3.01, D3.02** (2) — chip-revision crash bugs no reference models.

≈ **9 rows**. Add the soft-ceiling tier — true-hires *mainscreen* rows that references disagree on
(C5.06/C5.07 + the mainscreen halves of C9.01/02/07/08, ~6), interlace (C9.06 + C9.03-V, 2),
oracle-thin (F1.19/20/21, 3), G1.18 on-cart (1) — ≈ 12 rows, so the effective wall is ≈ **21 rows**.
**Everything else among the audited rows — and the entire un-audited Bucket-1/2 backlog — is
reachable.** Practical ceiling ≈ 422/443.

## Factual corrections this audit surfaced (applied in this PR)

1. `docs/accuracysnes-plan.md` C11.08 subsection: it is **coverable-with-work** (structural, spec-grounded),
   not uncoverable. The blocked-on-capture standing applies only to the exact-value variant. *(Applied.)*
2. `docs/accuracysnes-plan.md` C13 subsection: dropped the obsolete "whole-line compositor" blocker; the
   durable reason is "no reference models the early-read glitch + irreducible revision-dependence." *(Applied.)*
3. `docs/accuracysnes-research-dossier.md` D2.02: HDMA per-line dot is **276 (1104 master cycles)**, not 278
   (consensus of Mesen2/bsnes/ares — and it matches the cart's own dot-276 compositor). *(Applied.)*
4. `docs/accuracysnes-research-dossier.md` D2.01: HDMA init is **dot≈3 / 12 master cycles**, not "H≈6". *(Applied.)*
5. `docs/ppu.md`: "the per-line compositor is the simplification point" is **superseded** by
   ADR 0014 (per-dot is the sole renderer). *(Applied.)*
6. `docs/accuracysnes-plan.md`: "41 scenes blessed" → **53** (the regenerated coverage report is
   authoritative). *(Applied.)*

## Recommended next-action priority (highest value / lowest risk first)

1. **F1.08/F1.09/F1.10** — coverable now as goldens, zero contract change, and they expose a real
   auto-read start-dot accuracy gap. Best ROI.
2. **The 512-scene-region widening** — one bounded host change unblocks ~8 hi-res rows at once.
3. **D2.08 + the D2 WRAM-trail rows** — the HDMA mechanism is already proven by the covered D2.03/04.
4. **The 4b over-flag dot cursor** (already specced) → C7.05/C7.06.
5. **C11.08 structural** + **G1.06 reset test** — higher-effort, and C11.08 needs the spec-grounded
   caveat recorded.
