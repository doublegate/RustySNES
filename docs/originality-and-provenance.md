# Originality and provenance

An honest account of what RustySNES is, where it came from, and what it borrows. Written to be
checked, not admired. `NOTICE` is the formal attribution record; this document is the reasoning
behind it.

## 1. Thesis

**RustySNES is an independent implementation of documented SNES hardware, with a small number of
transparently attributed borrowings from permissively-licensed sources, cross-validated against
reference emulators used as behavioural oracles.**

Three disclosures belong at the top, before anything else.

**This project is developed with substantial AI assistance.** The architecture, the accuracy
methodology, and every judgement call are directed by the maintainer; a large fraction of the code
and prose is written by an AI coding agent under that direction. The git history makes the velocity
obvious — see §3 — and pretending otherwise would be the kind of omission that destroys trust when
someone works it out. It is disclosed here, in `NOTICE`, and in the README.

**Nothing here is a claim of superiority over any other emulator.** This document names ares, bsnes,
Mesen2, MesenCE and Snes9x repeatedly, because measuring against them is how RustySNES knows whether
it is right. Where a comparison appears, it is a comparison. Those projects are older, broader,
more battle-tested, and are the reason an accurate independent implementation is even possible.
RustySNES has found exactly one defect in a reference emulator (§2), and the honest framing of that
is *"a third opinion caught something"*, not *"we are better."*

**Copyleft emulators were oracles, never sources.** No source code from any GPL-licensed emulator is
incorporated into RustySNES. §4 explains how that claim is structured and where it was previously
described wrongly.

## 2. Where this project advances, diverges, or independently re-derives

The genuinely distinctive work is in *methodology* more than in any individual chip model — chip
models converge, because the hardware is the hardware.

### AccuracySNES: a first-party, self-scoring accuracy cartridge

The largest original artefact here. No publicly available SNES ROM offered a single self-scoring
battery across CPU, PPU, APU, DMA, controllers and cartridge behaviour, so this project generated
one: `accuracysnes-gen` emits 65816 source, assembles it with `ca65`/`ld65`, and produces the ROM
plus its own coverage report.

Two properties are the point:

- **The cart decides pass/fail on-cart.** The host supplies no expected values. A result therefore
  means the same thing on any emulator *and on a real flash cart* — which is what separates a
  portable accuracy claim from "our output matches our golden."
- **Three coverage tiers that are never summed.** On-cart (a verdict the cart itself reaches),
  rendered scene (needs a host holding a golden), and host-side (this project testing its own code,
  admitted *only* where the cart physically cannot observe the assertion). Adding them into one
  figure would quietly change what the number claims, so the report keeps three columns and the
  generator regenerates it with the ROM, so it cannot drift.

Current standing: **361 of 443 enumerated assertions** covered (304 + 55 + 2), a 346-test battery at
100% on-cart, cross-validated against three independent references.

### Non-vacuity as a gate, not an aspiration

Every scored row must fail when its own named bug is injected **at the site the row names**. This is
enforced practice, and it repeatedly caught tests that passed for the wrong reason: an assertion that
could not fail; a guard that arithmetically subsumed the assertion it protected; a whole subsystem
unobservable because no controller input was held; and one row that passed *harder* under its own
injection, because a neighbouring interpolator — not the decoder under test — was supplying the sign.

The general rule that fell out of it: **if the injection at the named site does not move the verdict,
the attribution is wrong even when the test passes.**

### Retraction discipline

Several published findings in this project's own CHANGELOG are retractions of earlier published
findings, kept rather than deleted. A defect claim against reference emulators was withdrawn when a
third reference made it 2-vs-2. A "1-vs-3" verdict was withdrawn when an unrelated change moved it,
proving it encoded an uncontrolled timing phase. A claimed oracle failure turned out to be one extra
line in this project's own test harness.

Keeping the retractions where the claims were made is deliberate. A changelog that silently deletes
its wrong answers cannot be audited backwards.

### An honesty gate on accuracy tiering

Boards and coprocessors are tiered Core / Curated / BestEffort (`docs/adr/0003`), and a CI gate
prevents an unverified BestEffort board from backing an accuracy claim. Known gaps are enumerated in
`docs/accuracy-ledger.md` rather than omitted.

### Determinism as an enforced contract

Same seed + ROM + input ⇒ bit-identical framebuffer and audio (`docs/adr/0004`). The practical test
of whether a contract like that is real is what happens when something needs a clock: netplay peer
liveness genuinely requires wall-clock time, and rather than admit `Instant` into the session, it is
confined to a `Transport` decorator outside the deterministic core, with an injected clock so its
tests need no sleeps. The core needed no change.

### Measured, and sometimes rejected

Run-ahead remains off by default because the measurement said so: the save/load round trip is 2.4%
of the NTSC frame budget, while one extra emulated frame costs 79% of it. The number is published
with the decision, including the case against.

### One reference-emulator defect

AccuracySNES found an inverted timer-2 counter reset in one reference emulator's `$F1` handler — the
first time this project has found a defect in a reference rather than in itself. It required a third
independent reference to be visible at all: with two, the affected rows passed everywhere and there
was nothing to investigate. That is the honest lesson, and it is about method, not merit.

## 3. How it was built, and how long it took

**The public timestamps are short, and this document is not going to imply otherwise.** The first
commit is `325f272`, dated **2026-06-25**. As of **2026-08-03** the repository holds **477 commits,
42 release tags, 17 workspace crates, and ~120,600 lines of Rust.**

That is roughly **six weeks**. A pace like that is not a claim of individual heroics — it is what
heavy AI assistance under experienced direction produces, which is exactly why §1 discloses it.
Anyone is free to divide those numbers and draw their own conclusion; the point of stating them is
that the conclusion should be available without having to dig.

What the pace does *not* excuse is unverified work, which is why the method is inverted from
"write, then test":

1. **Research first.** Hardware behaviour is read out of `ref-docs/` and public documentation before
   code is written; the research corpus is treated as immutable, with corrections landing as new
   dated files rather than in-place rewrites.
2. **Test as spec.** The failing test ROM or oracle vector is pinned *first*; implementation
   proceeds only until it passes. Where documentation and a test disagree, the test wins and the
   documentation is corrected.
3. **Verify last, against something that is not us.** Cross-validation against three independent
   references, with every divergence adjudicated per-row rather than counted.

## 4. Independence: oracle versus port

Every reference this project touched falls into exactly one category, and the source comments now
say which.

| Category | What it means | Applies to |
|---|---|---|
| **1 — implemented from public documentation** | Hardware behaviour written from fullsnes, anomie, the WDC datasheet, the ARM architecture, published data formats. Not a derivative of any emulator. | The overwhelming majority of the codebase |
| **2 — incorporated, permissively licensed** | Real third-party code, attributed in-source and in `NOTICE`. | `rcheevos` (MIT), vendored; egui's bundled fonts, via dependency |
| **3 — behavioural oracle only** | A reference emulator run to observe and cross-check documented behaviour. No code incorporated. | ares, bsnes, Mesen2, MesenCE, Snes9x |

**Why hardware behaviour is category 1.** Copyright protects expression, not facts. A CPU opcode's
effect, a register map, a documented signal model — every accurate emulator converges on these
because they describe the same silicon. Implementing them from public documentation is not a
derivative work, and two emulators agreeing about them is evidence that both are correct, not that
one copied the other.

### The sharpest honest example: the ST018 ARMv3 coprocessor

This is the case where the old comments were most wrong, so it is the one worth stating fully.

The ST018 is an ARMv3/ARM6-class CPU on a cartridge. Its module doc used to read *"Clean-room port of
Mesen2's `ArmV3Cpu` (MIT, ...)"*, and the board doc *"Board/bus protocol ported from Mesen2's
`St018`"*, with roughly twenty cross-references to Mesen2's private symbols scattered through the
file.

**Two things were wrong with that.** "Clean-room port" is self-contradictory — a clean-room
implementation is by definition not a port. And **the licence was simply false: Mesen2 is GPLv3, not
MIT**, which was verified against Mesen2's own `LICENSE` and README rather than assumed.

What the code actually is, established by inspection rather than by preference:

- The instruction decomposition (data processing, branch, PSR transfer, single and block data
  transfer, multiply, swap) matches Mesen2's — **because those are the names ARM's own architecture
  gives them.** Naming overlap here is evidence of nothing.
- Where implementations genuinely diverge, ours diverges. Multiply cycle counting implements the
  documented ARM early-termination rule directly, where the reference delegates to a shared helper
  from a different console's CPU. The board's scheduling model is deliberately different — per
  master tick here, versus a catch-up burst before each register access.
- **None of the reference's distinctive marginalia appears in our source** — not its notes about
  which PSR bit is always set, not its empty-register-list glitch commentary, not its
  non-sequential-access annotations. A transcription carries the original's "why" comments; this
  does not.

So it is category 1 with category-2 wording, and the wording is corrected. **One consequence of
oracle use is recorded rather than hidden:** where the ARM architecture reserves an encoding as
undefined that this chip appears to decode normally, RustySNES follows the *observed* behaviour. That
is a fact about the part, adopted from cross-checking — and the source comment now says exactly that
instead of describing itself as matching someone's decode table.

**Residual uncertainty, stated rather than smoothed over:** this assessment sampled the
implementation; it did not line-by-line re-derive all ~3,000 lines of the ARMv3 module. The evidence
above supports category 1, and the maintainer may wish to confirm it at line level. The correction
of the false MIT annotation stands regardless of that review, because it was wrong on its own terms.

### What was corrected, in summary

- Every source-file path and line citation into a **copyleft** reference removed.
- Every private-symbol / field-name cross-reference into a copyleft reference removed.
- "Ported from" / "clean-room port of" replaced with an accurate statement of the documented source
  plus, where a reference was consulted, an explicit oracle disclaimer.
- The false "Mesen2 ... (MIT)" licence annotation corrected to GPLv3.
- `NOTICE` rewritten from five lines that mentioned none of this.
- Bundled-font licence texts now ship with the binary; they previously did not.

**No comment was reworded from an accurate description of copying into a false claim of
independence.** That transformation is falsification, not remediation, and where evidence was mixed
the uncertainty is recorded above instead of resolved in this project's favour.

## 5. Licence compliance

**RustySNES is dual-licensed MIT OR Apache-2.0.**

- **Reference emulators** (§4 category 3) are not distributed. `ref-proj/` is gitignored; a
  developer clones the references locally to run cross-validation.
- **ares is ISC**, which asks that its notice appear in copies. `NOTICE` reproduces it — deliberately,
  even though this project implements hardware behaviour rather than copying ares, because ares'
  model was studied closely for several coprocessors and satisfying a permissive notice costs
  nothing while leaving no ambiguity.
- **Snes9x is a custom non-commercial licence** and is explicitly do-not-incorporate. It is used only
  as a cross-validation host.
- **Vendored code** is `rcheevos` 12.3.0 (MIT), carrying its upstream `LICENSE` in-tree and
  reproduced in `NOTICE`. It is reached only through an opt-in feature.
- **Cargo dependencies** are gated by `deny.toml`, whose allow-list admits no GPL, LGPL or AGPL
  crate. Reproduce with `cargo deny check licenses`.
- **Bundled fonts** ship inside the frontend binary via egui. Their licences — OFL-1.1 and the
  Ubuntu Font Licence among them — require the text to travel with the distribution, and now do, in
  `LICENSES-THIRD-PARTY-FONTS.txt`, packaged in every release archive and in the wasm build.
- **Test ROMs** are permissive-only and inventoried in `tests/roms/LICENSES.md`. No commercial ROM is
  committed, ever.
- **Creative expression** (shaders, NTSC filter) is this project's own WGSL, reproducing a *look*
  rather than incorporating anyone's shader source. No preset is bundled.

## 6. Conclusion

The strongest claim this project can honestly make is not that it is the most accurate SNES
emulator. It is that **its accuracy claims are checkable by someone who does not trust it** — an
on-cart battery whose verdicts do not depend on this project's own goldens, three independent
references, a coverage report that regenerates with the artefact it describes, and a changelog that
keeps its own retractions.

This provenance pass applies the same standard to the written record. Where the code said something
about itself that was not true, the code now says the true thing; where the truth is uncertain, the
uncertainty is written down instead of resolved favourably.

---

**See also:** `NOTICE` · `docs/provenance.md` (the audit playbook this pass followed) ·
`tests/roms/LICENSES.md` · `docs/STATUS.md` · `docs/accuracy-ledger.md` · `CHANGELOG.md`
