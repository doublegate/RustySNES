# Commercial-ROM corpus plan (the local hardware-coverage oracle)

Ports RustyNES's commercial-corpus methodology to SNES. **Goal:** a local, comprehensive set of
real cartridge dumps exercising every board / coprocessor / map model / region, used to produce
**committed emulator-output baselines** (framebuffer + audio hashes + screenshots) — never
committing a single ROM byte. Authoritative policy: ADR 0003 + `docs/testing-strategy.md`.

## The two-corpus split (the load-bearing pattern)

| Corpus | Lives in | Committed? |
|---|---|---|
| Permissive test ROMs (gilyon, undisbeliever, spc700) | `tests/roms/<suite>/` | **Yes** (see `rom-seeding-runbook.md`) |
| **Commercial dumps** (the No-Intro set) | `tests/roms/external/commercial/` | **Never** (gitignored) |
| Derived **manifest** (metadata + SHA-256, no bytes) | `tests/roms/commercial-corpus.json` | **Yes** |
| Derived **golden hashes** (`.snap`) + **screenshots** | `tests/golden/`, `screenshots/` | **Yes** |

A contributor without the ROMs still gets the manifest, the golden hashes, and the screenshots;
the `commercial-roms` harness **discovers zero ROMs and cleanly SKIPs** (never fails), so CI
never depends on non-redistributable data. The SHA-256 in each entry means a *different* dump of
the same game produces a clear "ROM mismatch" instead of a cryptic hash diff.

## The tool: `scripts/coverage/coverage.py` (stdlib-only)

Mirrors RustyNES's `coverage.py`. Classifies by parsing the SNES internal header (LoROM `$7FC0`
/ HiROM `$FFC0` / ExHiROM `$40FFC0`, chosen by checksum+title score), reading the cartridge-type
byte `$16` for the coprocessor family + the `$FFBF` subtype for Custom chips, then applying a
**curated title→chip override** (authoritative for the handful of custom-chip games and to split
DSP-1/2/3/4 and GSU-1/2, which the header cannot distinguish). Uses the exact `Coprocessor` enum
vocabulary from `crates/rustysnes-cart/src/header.rs`.

```bash
# read-only coverage table (board | tier | avail | target | gap); default target = 5 per bucket
python3 scripts/coverage/coverage.py --target 5 survey "<rom-library>"

# emit the committed manifest (metadata + sha256 ONLY, no bytes; golden=null until capture)
python3 scripts/coverage/coverage.py --target 5 catalog "<rom-library>" --out tests/roms/commercial-corpus.json
```

**The "≥4–5 each" rule** is the `--target 5` per (board, coprocessor) bucket — capped by real
cartridge population. Where a chip shipped in fewer games, the corpus takes the **entire
population** and the bucket is labelled population-limited (ADR 0003 BestEffort).

## Current coverage (survey over the local set, 995 ROMs, 2026-06-25)

Manifest: **48 ROMs across 16 buckets**. Well-covered (≥5): LoROM-plain (714), HiROM-plain
(222), SA-1 (15), DSP-1 (16), GSU-1 (7), GSU-2 (8), CX4 (5). Region NTSC=962 / PAL=33;
battery-backed=436.

Population-limited (took all available — games don't exist beyond this): S-DD1 (2), DSP-2 (1),
DSP-4 (1), SPC7110 (1), OBC1 (1), ST010 (1).

**Missing → see [`rom-acquisition-list.md`](rom-acquisition-list.md):** ExHiROM, DSP-3, ST011,
ST018, S-RTC (all Japan-only; 4 titles close all 5 gaps).

## Harness wiring to implement (Phase 4+, mirrors RustyNES)

These need a running emulator, so they land as the cart/PPU/APU work matures — captured here so
the contract is fixed now:

1. **`commercial-roms` feature** in `rustysnes-test-harness/Cargo.toml` →
   `commercial-roms = ["dep:zip", "dep:sha2", "dep:png"]` (zip-unwrap, ROM pinning, screenshot
   dump). Off by default; CI runs `--features test-roms` only.
2. **Auto-discovery harness** `tests/external_coverage.rs` (`#![cfg(feature = "commercial-roms")]`):
   walk `tests/roms/external/commercial/`, boot each ROM via `runner::run_until_complete`, run a
   uniform capture (warmup → checkpoints), apply two checks per ROM — (a) blank/few-colour health,
   (b) `insta::assert_snapshot!` of the framebuffer+audio FNV-1a hashes against the committed
   `.snap`. New ROMs need no code: drop file → re-bless. New ROMs verify against `commercial-corpus.json`'s SHA-256.
3. **Hand-curated layer** `tests/external_real_games.rs`: a few `#[test]`s with tuned input
   scripts so the captured frame lands on a meaningful gameplay/menu screen (per-coprocessor: a
   Star Fox frame for GSU, a Mario RPG frame for SA-1, etc.).
4. **Single lock-guarded bless** `scripts/coverage/bless.sh` (flock + single-threaded harness +
   `cargo insta accept`) to avoid Cargo target-lock races on bulk re-bless.
5. **Floor-only accuracy gate**: `MIN_PASS_RATE` const set below baseline, only ever raised; the
   harness prints per-suite + per-failing-test breakdown every run.
6. **Honesty gate** already enforced by `tests/mapper_tier_honesty.rs` — extend its
   `ORACLE_COPROCESSORS` set only as boards graduate to Core/Curated; BestEffort buckets
   (S-DD1/SPC7110/CX4/OBC1/ST01x/S-RTC) carry reference screenshots but never the accuracy number.

## Staging (when ready)

Staging copies the selected dumps into the gitignored tree — local-only, reversible (`rm -rf`
the dir), and an explicit step (not auto-run):

```bash
mkdir -p tests/roms/external/commercial      # gitignored
# copy the manifest-selected ROMs here, organized as <map>/<chip>/<title>.sfc
```

The capture harness reads from there under `--features commercial-roms`; absent ROMs → SKIP.
