# RustySNES `v1.30.1` "Imprint"

**Released:** 2026-08-04 · **Previous release:** [`v1.30.0` "Threshold"](https://github.com/doublegate/RustySNES/releases/tag/v1.30.0)

> A provenance, licensing and attribution release. **Zero emulation-core behaviour change** — across
> 13 changed source files the only non-comment edit is a test *rename*. The AccuracySNES battery is
> 56/56 and every golden is unmoved, by construction.
>
> A printer's *imprint* is the statement of who made a thing and under what terms. That is what this
> release adds, and in several places it is the first time the project has stated it correctly.

---

## Executive summary

| | |
|---|---|
| Emulation behaviour | **unchanged** — comment/doc/packaging only |
| Non-comment source edits | **1** (a test rename) |
| AccuracySNES battery | 56/56 · scenes 2/2 · 0 failed |
| Copyleft source-path citations removed | **10** |
| Copyleft private-symbol cross-references removed | **29** |
| "Ported from" doc claims corrected | **15** |
| New files | `NOTICE` (rewritten), `LICENSES-THIRD-PARTY-FONTS.txt`, `tests/roms/LICENSES.md`, `docs/originality-and-provenance.md`, `docs/provenance.md` |

---

## 1. The finding that mattered most: a false licence annotation

`crates/rustysnes-cart/src/coproc/armv3/mod.rs` described the ST018 coprocessor core as:

> *"Clean-room port of Mesen2's `ArmV3Cpu` (**MIT**, …)"*

**Mesen2 is GPLv3.** That was verified against Mesen2's own `LICENSE` file and README rather than
assumed. "Clean-room port" is also self-contradictory — a clean-room implementation is by definition
not a port.

The governing rule for fixing something like this is that **you may not reword a real copy into a
claim of independence.** So the question was settled by inspection, not by preference:

| Evidence | Reading |
|---|---|
| Instruction decomposition matches the reference | **Nothing** — those are ARM's own instruction-class names (data processing, branch, PSR transfer, single/block data transfer, multiply, swap) |
| Multiply cycle counting | **Diverges** — implements the documented ARM early-termination rule directly, where the reference delegates to a helper from a different console's CPU |
| Board scheduling model | **Diverges** — steps every master tick here, versus a catch-up burst before each register access |
| The reference's distinctive marginalia | **Absent** — none of its PSR-bit notes, empty-register-list glitch commentary, or non-sequential-access annotations appear here. A transcription carries the original's "why" comments |

So this is hardware-documentation work that had been *described* as a port, and the description is
corrected. **The residual uncertainty is recorded rather than resolved favourably:** this pass
sampled the module; it did not line-by-line re-derive all ~3,000 lines. The correction of the false
MIT annotation stands regardless, because it was wrong on its own terms.

One consequence of oracle use is now recorded instead of hidden: where the ARM architecture reserves
an encoding as undefined that this chip appears to decode normally, RustySNES follows the *observed*
behaviour — a fact about the part, adopted from cross-checking.

## 2. Copyleft citations removed throughout

- **10** `ref-proj/` source-file paths into bsnes, Mesen2 and MesenCE.
- **29** private-symbol / field-name cross-references into them — **20 in the ST018 board alone**.
- **15** further "ported from" claims across `README.md`, `docs/` and `to-dos/`.

Each is replaced by the documented hardware source plus, where a reference was consulted, an
explicit *"cross-checked against reference emulators as behavioural oracles; no third-party emulator
code is incorporated"*.

**The two naming bsnes (GPLv3) were the sharpest**, and both turned out to be public facts:

- **The Game Genie decoder.** The SNES code format — a 16-symbol hex-digit substitution followed by
  a fixed bit transposition — has been published since the 1990s and is catalogued publicly today.
  Confirmed against that published record rather than assumed. Pro Action Replay is plain
  `AAAAAADD` hex, documented in fullsnes.
- **The CPU's direct-page arithmetic**, which is the documented 65C816 addressing behaviour (WDC
  W65C816S datasheet; emulation-mode page-lock per anomie).

## 3. `NOTICE` rewritten — 5 lines to 7 sections

The old file said, in full, that the project is dual-licensed and that *"vendored reference code (if
any) retains its own license — see `ref-proj/`."* That is **doubly wrong**: `ref-proj/` is gitignored
and never distributed, while the tree that genuinely *is* vendored — `rcheevos` 12.3.0 (MIT) — went
unmentioned entirely.

The new `NOTICE` carries hardware documentation; reference emulators **with their licences** and the
flat statement that no GPL-licensed emulator's code is incorporated; incorporated permissive
components with full permission text; bundled fonts; visual influences; and test ROMs.

The reference set, read from the actual licence files:

| Reference | Licence | Role |
|---|---|---|
| ares | **ISC** | cross-validation reference; third battery reference since `v1.29.0` |
| bsnes | **GPLv3** (its libco/nall/ruby/hiro are ISC, none used) | oracle only |
| Mesen2 / MesenCE | **GPLv3** | oracle only |
| Snes9x | **custom, non-commercial** | oracle only; explicitly do-not-incorporate |

**ares' ISC notice is now reproduced deliberately.** Implementing hardware behaviour does not
require it, but ares' model was studied closely for several coprocessors, and satisfying a
permissive notice costs nothing while leaving no ambiguity.

## 4. A real compliance gap, closed

egui's `epaint_default_fonts` crate embeds **Ubuntu-Light, NotoEmoji, Hack and emoji-icon-font
directly into the binary**. SIL OFL-1.1 and the Ubuntu Font Licence both require their text to
accompany the font wherever it is distributed — and **every RustySNES release archive shipped the
binary with `README`, `LICENSE-MIT` and `LICENSE-APACHE` and none of those font licences.**

New `LICENSES-THIRD-PARTY-FONTS.txt` now ships:

- in **both** `release.yml` packaging steps (the Unix `tar.gz` and the Windows `zip`), and
- in the **wasm dist**, because a hosted WebAssembly build is a distribution too — verified by
  actually running `trunk build` and confirming the file lands in `dist/`, rather than assuming a
  parent-relative `copy-file` href resolves.

Release archives from `v1.30.1` onward therefore contain: the binary, `README.md`, `LICENSE-MIT`,
`LICENSE-APACHE`, **`NOTICE`**, and **`LICENSES-THIRD-PARTY-FONTS.txt`**.

## 5. Two documents that did not exist

- **`tests/roms/LICENSES.md`** — the authoritative per-corpus inventory. Every count is derived from
  `git ls-files`, never `find`, so the gitignored `external/` tree cannot inflate it. 347 committed
  files, 36 `.sfc` images, across four corpora (AccuracySNES MIT-OR-Apache-2.0; undisbeliever MIT
  *and* zlib — both texts retained deliberately, since upstream relicensed after these copies were
  taken; gilyon MIT; spc700-singlestep MIT). No commercial ROM is committed, ever.
- **`docs/originality-and-provenance.md`** — the independence account: the three-category
  oracle-versus-port classification, an **AI-assistance disclosure**, an explicit *not a superiority
  claim* note, and the ST018 case stated in full including its residual uncertainty.

## 6. README corrections

- **The AI-assistance disclosure is now at the top**, as a `Development note` blockquote immediately
  after the opening paragraph, matching the sibling project's placement. A disclosure a reader has
  to scroll 800 lines to find is a disclosure in name only.
- **The emulator capability-comparison graphic is removed**, with its now-orphaned asset. It ranked
  this project against Mesen2, bsnes, Snes9x and ares in a hand-drawn diagram that nothing verified
  and nothing regenerated — its own caption already disclaimed it as "illustrative" and pointed
  elsewhere for real numbers. A comparison that has to disclaim itself is not carrying its weight.
- **A false licence claim fixed.** Test ROMs were described as "individually CC0, MIT, or Zlib
  licensed". **There is no CC0 corpus** — they are MIT, zlib, and this project's own
  MIT-OR-Apache-2.0.
- **Stale accuracy figures corrected** against the generated coverage report: the battery is **346
  tests at 100% on-cart covering 304 of 443** assertions, not "124 / 124 scoring, plus 10 golden
  vectors"; rendered scenes cover **55**, not 3; the host tier covers 2; **361 of 443 in total**,
  with the standing rule that the three tiers are never summed made explicit. The cross-validation
  line still named only Mesen2 and snes9x after ares landed in `v1.29.0`.
- **Acknowledgments synced to `NOTICE`**, and now credit the sources this emulator is actually
  implemented *from* — fullsnes / Martin Korth, anomie, undisbeliever, the WDC datasheet — rather
  than only the emulators it is checked *against*.
- BibTeX citation `1.8.0` → `1.30.1`.

## Compatibility and upgrade notes

- **Emulation behaviour: unchanged.** No chip model, timing constant, or golden vector moved.
- **Save states:** format version unchanged.
- **Public API:** unchanged.
- **Release archives gain two files** (`NOTICE`, `LICENSES-THIRD-PARTY-FONTS.txt`). Packagers who
  enumerate archive contents should expect them.

## Verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p rustysnes-test-harness --all-targets --features test-roms -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features
cargo test --workspace                          # 68 suites, 892 passed, 0 failed
cargo test --workspace --features test-roms     # AccuracySNES 56/56, scenes 2/2
cd crates/rustysnes-frontend/web && trunk build  # font licences land in dist/
```

Plus per-feature frontend clippy across all eight gated feature sets, and markdownlint (pinned
`v0.39.0`, changed files only). **CI green on all jobs** including the three-OS `full-test` matrix.

The strongest single piece of evidence that this release changes no behaviour is structural: across
all 13 changed source files, the only non-comment edit is one test **rename**.

---

Full per-entry detail: [`CHANGELOG.md` → `[1.30.1]`](../../CHANGELOG.md). The audit playbook this
pass followed is [`docs/provenance.md`](../provenance.md).
