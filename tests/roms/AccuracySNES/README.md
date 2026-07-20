# AccuracySNES

A first-party SNES hardware-accuracy test cartridge, in the spirit of
[`100thCoin/AccuracyCoin`](https://github.com/100thCoin/AccuracyCoin) for the NES.

**Dual-licensed MIT OR Apache-2.0.** All of it is original work — no third-party test ROM, font,
or reference source is vendored or derived from. That matters: every other SNES corpus in this
repo is either unlicensed (SingleStepTests/65816, PeterLemon), unstated (blargg `spc_*`), or
copyleft (240p Test Suite), which is why `tests/roms/README.md` has a two-tier committed/external
split at all. AccuracySNES has no such encumbrance, so any emulator project can vendor it.

## Why it exists

`docs/testing-strategy.md` states the gap plainly: *"The SNES has no single canonical battery — and
no Nintendulator-style textual golden CPU log exists for the 65816."* `docs/STATUS.md` recorded
ticket **T-04** — the AccuracyCoin-equivalent ROM — as *"never implemented and has since been
superseded — no publicly available SNES ROM plays the AccuracyCoin role."* This closes it.

## Running it

```bash
cargo run -p accuracysnes-gen                       # regenerate asm + catalog, assemble, link
cargo test -p rustysnes-test-harness --features test-roms --test accuracysnes -- --nocapture
```

Building requires `ca65`/`ld65` (cc65 2.19+). The linked `build/accuracysnes.sfc` is committed, so
running the battery does not.

On real hardware or another emulator, just boot it: the battery runs to completion **with no input
at all**, then draws a scrollable result list. D-Pad up/down scrolls.

## How scoring works

The cart decides pass/fail entirely on-cart and publishes a results block in WRAM. The host
harness supplies **no expected values** — it only reads the block. That is what lets the same
image run unmodified on ares, bsnes, Mesen2, and a real SNES.

Results block at `$7E:F000`:

| Offset | Field |
|---|---|
| `+$00` | magic `"ACSN"` |
| `+$04` | format version (u16) |
| `+$06` | test count (u16) |
| `+$08` | `$A5` once every test has run |
| `+$0A` | passed / failed / skipped / golden (four u16) |
| `+$20` | status array, one byte per test, aligned to `SOURCE_CATALOG.tsv` |

Status byte encoding:

| Byte | Meaning |
|---|---|
| `$00` | not run |
| `$01` | PASS |
| `(n<<1)\|1` | PASS, **variant `n`** — which legal hardware behaviour was observed |
| `n<<1` (non-zero) | FAIL, error code `n` identifying the exact sub-assertion |
| `$FF` | skipped (auto-gated by chip revision/region, or user-marked) |

`ERROR_CODES.md` is the generated dictionary mapping every code to the assertion it represents.
A bare "FAIL" is useless to an emulator author; the code tells you which sub-case broke.

**Variants** exist because the SNES has more legal-divergence axes than the NES — 5A22 v1/v2,
PPU2 v1/v2/v3, 1CHIP vs 3CHIP, NTSC vs PAL. A test that hard-fails a legitimate second console
revision is a bug in the test.

## Provenance tiers — the anti-circularity gate

A test we wrote, grading an emulator we wrote, proves nothing. Every test carries a tier in
`SOURCE_CATALOG.tsv`, and **only the top two may contribute to the pass rate**:

| Tier | Meaning | Scores? |
|---|---|---|
| **Documented** | a primary reference states the behaviour outright | yes |
| **Corroborated** | ares, bsnes, and Mesen2 independently agree in source | yes |
| **Contested** | references disagree, or one admits it is unexplained | no |
| **Novel** | our own hypothesis, no external backing | no |

There is also a **golden-vector** kind for behaviour hardware genuinely does not define — the
`$4203`/`$4206` overlap, decimal-mode `V`, the WRAM power-on fill, post-reset `ENDX`. Those record
what they observed and are never counted either way. `docs/accuracy-ledger.md` already classifies
the first of those as out-of-scope because *"inventing one would violate the determinism-contract
spirit."*

This mirrors `docs/adr/0003`'s coprocessor honesty gate, and the harness enforces it in
`provenance_gate_holds`.

## Layout

```text
gen/                      Rust: the DSL, the test definitions, the asm emitter, the build driver
  src/dsl.rs              Test/Provenance/Kind types + the assembly builder
  src/tests/cpu.rs        Group A — the 65816 CPU tests
  src/font.rs             the 8x8 font, generated
asm/                      runtime.s + header.s hand-written; tests_group_a.s + font.s GENERATED
lorom.cfg                 ld65 configuration
build/accuracysnes.sfc    the committed 128 KiB image
SOURCE_CATALOG.tsv        GENERATED machine-readable manifest (the harness include_str!s this)
ERROR_CODES.md            GENERATED failure-code dictionary
```

Each test is authored **once**, in Rust, and the assembly plus the catalog row are both emitted
from that single definition — so the on-cart behaviour and the host-side manifest cannot drift.

## Why 128 KiB

Several addressing tests must distinguish "the effective address wrapped inside bank `$00`" from
"it crossed into bank `$01`". Inside a 32 KiB image every bank mirrors the same bytes, so the two
outcomes are indistinguishable and the test proves nothing. Four distinct 32 KiB LoROM banks each
carry a signature byte at `$xx:8005`, making the difference observable.

## Cross-validation

`scripts/accuracysnes/crossval.sh` runs the same image on emulators we did not write:

| Reference | How | Result |
|---|---|---|
| **Mesen2** | headless `--testrunner` + `mesen_crossval.lua` reading `emu.memType.snesWorkRam` | 0 failures |
| **snes9x** | `libretro_crossval.c`, a small host reading `RETRO_MEMORY_SYSTEM_RAM` | 41/41 |
| bsnes / ares | source review only — bsnes' libretro target stubs out `retro_get_memory_data` and ares has no headless mode | 23/24 claims confirmed |

This is not ceremony. An early version of `A6.06` passed here and on snes9x but failed on Mesen2,
and the fault turned out to be the test's own: `REP` is ignored while `E=1`, so a 16-bit store
wrote one byte over stale memory that happened to match on two emulators.

## Design notes

- **Input is read manually through `$4016`**, not via auto-joypad. The auto-read has a documented
  start-window race, and RustySNES does not model `$4212` bit 0 at all — the usual "wait for busy
  to set, then clear" idiom would deadlock there.
- **VBlank is polled, not NMI-driven**, so no interrupt can fire in the middle of a test that is
  deliberately corrupting the stack or the `E` flag.
- **Tests may corrupt anything.** `test_restore` rebuilds `S`, `DP`, `DBR`, and the `E`/`M`/`X`
  flags from a saved snapshot, which is what makes the emulation-mode and stack-wrap groups safe
  to write without per-test cleanup.

## Scope

Phase A shipped **Group A (65816 CPU)**, 43 tests. Phase B adds **Group C sub-groups C1-C3** — the OAM, VRAM and CGRAM/counter *port mechanics*, 13 tests. 56 total: 55 scoring + 1 golden vector, all passing on RustySNES, Mesen2, and snes9x.

C1-C3 come first deliberately: port behaviour is pure register logic with no renderer dependency, so it establishes a passing baseline before the sub-groups that lean on parts of the PPU this project's own docs record as simplified. Groups B-G — PPU, DMA/HDMA, SPC700/S-DSP, input, power-on
— are enumerated per-test in `docs/accuracysnes-research-dossier.md` §5 and land in later phases.

Deliberately out of scope, matching AccuracyCoin's own NROM choice: coprocessors (this is a plain
LoROM cart, no SRAM) and audio *output* verification (the APU is exercised through its register
and timing side effects).
