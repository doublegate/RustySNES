# Testing strategy — RustySNES

**References:** `ref-docs/research-report.md` "Standards / test-ROM corpora" + "External
dependencies"; `ref-docs/2026-06-24-apu.md` §3; `docs/adr/0003`. **All test-ROM licenses below
were verified live (2026-06-24) by reading the actual repo LICENSE files** — the licensing is
heterogeneous and has real gotchas (especially the SingleStepTests 65816 set, which ships **no
license**).

## The governing rule

**Test ROMs are the spec.** When the docs and a passing test ROM disagree, the ROM wins and
the docs get updated. For any accuracy work: **pin the failing ROM expectation FIRST**, then
implement until it passes.

## The AccuracySNES battery (first-party, the AccuracyCoin-equivalent)

RustyNES uses a single 139-test AccuracyCoin battery. The SNES had **no single canonical
battery** — and there is still **no Nintendulator-style textual golden CPU log for the 65816** —
so this project now ships its own: **AccuracySNES** (`tests/roms/AccuracySNES/`), original work,
MIT OR Apache-2.0, closing what `docs/STATUS.md` tracked as ticket **T-04**.

It is self-scoring: the cart decides pass/fail on-cart and publishes a results block in WRAM, so
the same image runs unmodified on ares, bsnes, Mesen2, and real hardware, and the host harness
supplies no expected values of its own. Two rules govern what its number is allowed to mean:

- **Provenance tiering.** Every test declares where its expected value came from — `Documented`
  (a primary reference states it), `Corroborated` (ares + bsnes + Mesen2 agree in source),
  `Contested` (the references disagree, or one admits it is unexplained), `Novel` (our own
  hypothesis). **Only the first two may contribute to the pass rate.** A test we wrote grading an
  emulator we wrote proves nothing otherwise. Enforced by `tests/accuracysnes.rs`'s
  `provenance_gate_holds`, mirroring `docs/adr/0003`.
- **Golden vectors.** Behaviour hardware genuinely does not define (`$4203`/`$4206` overlap,
  decimal-mode `V`, the WRAM power-on fill, post-reset `ENDX`) is *recorded*, never scored.

**Real-hardware validation is the honest ceiling on its authority** and has not been done; that is
tracked, not claimed.

The composed multi-suite oracle below remains in force alongside it — AccuracySNES adds a layer,
it does not replace the per-opcode oracles:

1. **Primary per-opcode oracle (both CPUs):** SingleStepTests **65816** + **spc700** JSON —
   per-opcode, all addressing modes, 8/16-bit, native + emulation, with **cycle-by-cycle
   bus-pin traces**. The direct analog of the NES SingleStepTests RustyNES already trusts.
2. **Committable on-cart system layer:** **gilyon/snes-tests** (pass/fail `.sfc` for both CPUs
   with golden `tests*.txt`) + **undisbeliever/snes-test-roms** (PPU/DMA/HDMA + hardware
   glitches). Plus blargg `spc_*` for cycle-accurate audio and the 240p Suite for video.

**The concrete bar:** Mesen-S is the only emulator that passed *all* of blargg's SPC/DSP
tests, so matching that suite is the accuracy target. Model the in-repo harness on Mesen2's
`RecordedRomTest` (per-frame screenshot-hash baseline + replay — **study the design, don't
copy the GPLv3 code**).

## Licensing — the critical gate (`docs/adr/0003` posture)

Commit only the **permissively-licensed** corpora into the MIT/Apache tree; keep everything
unlicensed / copyleft / Nintendo in a **gitignored** `tests/roms/external/`. This mirrors
RustyNES's commercial-ROM policy.

| Corpus | License (verified) | Role | Commit? |
|---|---|---|---|
| **AccuracySNES** (this repo) | **MIT OR Apache-2.0** [OK] | first-party accuracy battery | **yes — committed** |
| SingleStepTests/**65816** | **NONE** (404 on `gh api .../license`) [GATE] | primary CPU oracle | **NO — gitignore or self-generate** |
| SingleStepTests/**spc700** | **MIT** [OK] | primary SPC oracle | yes (external tier OK) |
| gilyon/snes-tests | **MIT** [OK] | committable CPU+SPC `.sfc` + golden tables | **yes** |
| undisbeliever/snes-test-roms | **Zlib** [OK] | committable PPU/DMA/HDMA hardware behavior | **yes** |
| blargg `spc_*` ROMs | unstated [GATE] | cycle-accurate SPC/DSP oracle | external/reference tier |
| blargg `snes_spc` library | **LGPL-2.1** [GATE] | audio `.spc`→`.wav` comparison | external only, **never vendor** |
| 240p Test Suite (SNES) | **GPL-2.0-or-later** [GATE] | video / overscan patterns | clone / run-only, **don't vendor** |
| PeterLemon/SNES (Krom) | **NONE** (404) [GATE] | broad CPU/PPU/SPC/DSP/GSU + ref PNGs | reference-only, **don't commit** |
| TASVideos / Nintendo service ROMs | Nintendo [NO] | aging / Cx4 / SPC7110 checks | reference-only, **not redistributable** |

Legend: **[OK]** = permissive, committable. **[GATE]** = unstated / copyleft / no-license —
external/reference tier, do not vendor. **[NO]** = Nintendo-copyrighted, not redistributable.

**The 65816 snag, restated:** secure explicit permission, keep the JSON in the gitignored
external tier, or **generate equivalent JSON from a validated core** before relying on it in
CI (`ref-docs/research-report.md` "Open questions" #1).

## The testing layers

- **Layer 1 — unit** (per crate). Target >90% on the chip crates; each chip is fuzzable in
  isolation (`docs/architecture.md` §3).
- **Layer 2 — per-opcode oracle.** Run the SingleStepTests 65816 + spc700 JSON through the
  harness's JSON runner; first cycle-level mismatch fails. (Gated on the 65816 license.)
- **Layer 3 — on-cart test ROMs.** `run_until_complete()` + result-code / golden-table assert
  over gilyon (committed) and undisbeliever (committed); blargg `spc_*` (external).
- **Layer 4 — accuracy battery.** Pass-rate gate, **≥90% by v1.0, 100% the goal**. The hard
  residuals defer to the fractional-timebase refactor (`docs/adr/0002`), documented not
  point-fixed.
- **Layer 5 — visual golden + screenshots.** `tests/golden/` and `screenshots/` (committed);
  240p Suite + Krom screenshot-diff. Commercial ROMs stay gitignored in
  `tests/roms/external/`.
- **Layer 6 — determinism.** Save-state round-trip and seed+ROM+input replay must be
  bit-identical (framebuffer + audio) — `docs/adr/0004`.

## The honesty gate (`docs/adr/0003`)

Once boards are tiered (Core / Curated / BestEffort), a CI test **fails if any BestEffort
board backs the accuracy oracle / pass-gate**. BestEffort boards may carry reference
screenshots but never inflate the accuracy number.

## Never

- Never commit commercial Nintendo ROMs.
- Never vendor LGPL / GPL / unlicensed corpora into the MIT/Apache tree.

## Open questions

- ~~Whether to self-generate the 65816 JSON oracle (removes the license dependency entirely) or
  pursue permission.~~ **Resolved — `docs/adr/0005`:** self-generate the committed oracle of
  record, cross-validated against the gitignored upstream set + gilyon + ares before freeze; the
  unlicensed upstream set stays a local cross-check, never committed, never a CI dependency.
