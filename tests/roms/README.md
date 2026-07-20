# `tests/roms/` — Test ROM Corpora (RustySNES)

The SNES test-ROM oracle. Two tiers, governed by `docs/testing-strategy.md`
(the authoritative licensing table) and `to-dos/phase-0-foundation/rom-seeding-runbook.md`:

- **Committed tier** (this dir, minus `external/`): only **permissively-licensed**
  corpora — MIT / Zlib. Every committed `.sfc` / `.json` ships its upstream `LICENSE`
  verbatim. The root `.gitignore` broad-excludes all ROM extensions and re-includes
  ONLY `tests/roms/gilyon/**` and `tests/roms/undisbeliever/**`.
- **External tier** (`external/`): **gitignored, never committed.** Unlicensed /
  copyleft / Nintendo corpora live here for local cross-check, run-only verification,
  and reference screenshots. Confirm with
  `git check-ignore tests/roms/external/anything` → returns the matched ignore line.

Provenance + licensing was verified **live on 2026-06-25** by reading each upstream's
actual `LICENSE` (or confirming its absence via `gh api .../license` → 404).

## Committed corpora (permissive — in the git tree)

| Corpus | Upstream | License (verified) | Tier | Committed contents | Footprint |
|---|---|---|---|---|---|
| **gilyon** | [github.com/gilyon/snes-tests](https://github.com/gilyon/snes-tests) (release `v1.4`) | **MIT** (`LICENSE`, "Copyright (c) 2023 gilyon") | committed | 3 prebuilt `.sfc` (`cputest/cputest-basic.sfc`, `cputest/cputest-full.sfc`, `spctest/spctest.sfc`) + their golden result tables (`cputest/tests-basic.txt`, `cputest/tests-full.txt`, `spctest/tests.txt`) + per-dir READMEs + `LICENSE` | 1.3 MB / 10 files |
| **undisbeliever** | [github.com/undisbeliever/snes-test-roms](https://github.com/undisbeliever/snes-test-roms) (release `v20210217`) | **MIT** as-shipped in the release ROM zip (`LICENSE`, "Copyright (c) 2019"); current repo HEAD is **Zlib** ("Copyright © 2016") — kept as `LICENSE.zlib-repo-current`. Both permissive/committable. | committed | 29 prebuilt PPU/DMA/HDMA hardware-glitch `.sfc` (`hdma-*`, `inidisp_*`, `scpu-a-dma-bug-*`) + `LICENSE` (MIT) + `LICENSE.zlib-repo-current` | 3.7 MB / 31 files |
| **AccuracySNES** | **first-party** (`tests/roms/AccuracySNES/`) | **MIT OR Apache-2.0** (own work) | committed | Cartridge source (`gen/` Rust definitions + `asm/`), the generated `SOURCE_CATALOG.tsv` / `ERROR_CODES.md`, and the linked `build/accuracysnes.sfc` (128 KiB). Assembler intermediates are gitignored. | 1.1 MB |
| **spc700-singlestep** | [github.com/SingleStepTests/spc700](https://github.com/SingleStepTests/spc700) (`v1/`) | **MIT** (`gh api .../license` → MIT; `LICENSE`, "Copyright (c) 2024 SingleStepTests") | committed | **Deterministic sampled subset**: 256 opcode files × first 50 tests = 12,800 tests (`v1/00.json`..`v1/ff.json`) + `LICENSE` + `SAMPLING.md`. Full 256k-test set lives in `external/spc700-singlestep-full/`. | 4.5 MB / 258 files |

**Committed total: ~10.6 MB.**

## External corpora (gitignored — local only, `external/`)

| Corpus | Upstream | License (verified) | Why external | Local contents | Footprint |
|---|---|---|---|---|---|
| **65816-singlestep** | [github.com/SingleStepTests/65816](https://github.com/SingleStepTests/65816) (`v1/`) | **NONE** (`gh api .../license` → 404; no `LICENSE` in repo) | Unlicensed → ADR 0005: local CPU-oracle cross-check only, the committed oracle of record is self-generated. Never committed, never a CI dep. | 512 JSON (`XX.e.json`/`XX.n.json`, emulation+native per opcode, ~5k tests each) + `README.md` + `NO-LICENSE.txt` | 2.7 GB / 514 files |
| **spc700-singlestep-full** | (same as committed spc700) | **MIT** | Full 256k-test set is too large for the source tree; committed tier carries the 50-per-opcode sample. | 256 opcode JSON (1000 tests each) + `LICENSE` | 81 MB / 257 files |
| **blargg-spc** | blargg `spc_*` (SNESdev Wiki "Tests") | **unstated** | Unstated license → reference tier. **NOT auto-fetched** — see below. | `MISSING-fetch-manually.txt` (acquisition instructions; no ROMs) | <1 KB / 1 file |
| **240p** | [github.com/ArtemioUrbina/240pTestSuite](https://github.com/ArtemioUrbina/240pTestSuite) (`240psuite/SNES/`) | **GPL-2.0-or-later** (`LICENSE`) | Copyleft → run-only, never vendored into MIT/Apache tree. | SNES **source** subtree (PVSnesLib C/asm; no prebuilt `.sfc` on GitHub — build, or grab the official build from itch.io) + `LICENSE` + `NOTES.txt` | 6.5 MB / 97 files |
| **krom** | [github.com/PeterLemon/SNES](https://github.com/PeterLemon/SNES) | **NONE** (`gh api .../license` → 404) | Unlicensed → reference-only (broad CPU/PPU/SPC/DSP/GSU ROMs + reference PNGs for screenshot-diff). | 164 `.sfc` + 255 reference `.png` (`.git` stripped) + `NO-LICENSE.txt` | 398 MB / 2051 files |
| **commercial** | end-user legal dump (No-Intro) | Nintendo | Not redistributable. | (developer-staged locally; only the derived `commercial-corpus.json` manifest + screenshots/`.snap` are committed) | 99 MB / 48 files |

**External total: ~3.3 GB (gitignored).**

## blargg `spc_*` — manual acquisition (the one gap)

blargg's standalone SNES SPC/DSP test suite (`spc`, `spc_dsp6`, `mem_access`, `timer`)
was historically distributed as `spc_tests.zip` on `blargg.parodius.com` and
`slack.net/~ant/old/spc-tests/` — both hosts are now dead, and no stable direct
mirror was found (zophar / superfamicom wiki / archive.org all serve HTML landing
pages, not the zip). Acquire it manually from the SNESdev Wiki "Tests" page
(<https://snes.nesdev.org/wiki/Tests>) or a higan/ares/Mesen2 test-rom bundle, and
drop the extracted ROMs in `external/blargg-spc/` (gitignored). See that dir's
`MISSING-fetch-manually.txt`. Interim SPC700 per-opcode coverage is already provided
by `spc700-singlestep/` (committed) and `external/krom/CPUTest/SPC700/*`.

## AccuracySNES — the first-party battery

The one corpus here that is **ours**. `docs/testing-strategy.md` records that the SNES has no
single canonical accuracy battery, and `docs/STATUS.md` tracked that gap as ticket **T-04**;
AccuracySNES closes it. Because it is original work it carries no licence encumbrance, so unlike
every corpus in the external tier it can simply be committed — and any other emulator project can
vendor it too.

The cart is self-scoring: it runs the whole battery with no input and publishes a results block in
WRAM, so the same image runs unmodified on ares, bsnes, Mesen2, and real hardware. Every test
carries a **provenance tier**, and only `Documented`/`Corroborated` tests may contribute to the
pass rate — the anti-circularity gate, since a test we wrote grading an emulator we wrote proves
nothing on its own. See `tests/roms/AccuracySNES/README.md`.

```bash
cargo run -p accuracysnes-gen        # rebuild the cart (needs ca65/ld65)
cargo test -p rustysnes-test-harness --features test-roms --test accuracysnes -- --nocapture
```

## How to run

```bash
cargo test --workspace --features test-roms            # enumerates committed gilyon + undisbeliever + spc700 sample
cargo test --workspace --features test-roms,commercial-roms   # adds external/ boards where staged locally
```

The `commercial-roms` feature depends on `external/` ROMs not in the git tree;
absent ROMs report **skipped**, never failed (the `/accuracy-battery` skill enforces
"skipped ≠ pass").

## See also

- `docs/testing-strategy.md` — authoritative licensing table + the six testing layers.
- `to-dos/phase-0-foundation/rom-seeding-runbook.md` — target layout + per-corpus steps + loader contract.
- `tests/roms/spc700-singlestep/SAMPLING.md` — the deterministic sampling rule.
- `docs/adr/0003` (commit posture) · `docs/adr/0005` (self-generated 65816 oracle).
- `docs/STATUS.md` — single source of truth for test count + accuracy pass rate.
