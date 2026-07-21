# AccuracySNES — Research Dossier

**Status:** research corpus, compiled 2026-07-19. This is the evidence base for the AccuracySNES
test cartridge (see `docs/accuracysnes.md` for the spec once it exists, and
`to-dos/ROADMAP.md` for the phase plan). It is a *findings* document — it records what was
established, by whom, and with what confidence. It is not itself a specification.

**Method.** Five parallel research agents ran to completion:

| # | Scope | Primary sources |
|---|---|---|
| 1 | AccuracyCoin's integration into RustyNES | `../RustyNES/` source, read directly |
| 2 | AccuracyCoin upstream | `github.com/100thCoin/AccuracyCoin` README + `.asm` + `.nes`, fetched live |
| 3 | RustySNES test infrastructure | this repo, read directly |
| 4 | SNES hardware behavior catalog | SNESdev Wiki (Errata, Timing, PPU/CPU/APU register pages), fullsnes, Anomie's docs, superfamicom.org, TASVideos — fetched live, with six sub-agents |
| 5 | Reference-emulator quirk corpus | `ref-proj/{ares,bsnes,Mesen2}` source comments, read directly |

Agent 4's full working notes are committed as
`ref-docs/2026-07-19-accuracysnes-hardware-test-design.md` (938 lines) — the immutable research
corpus this dossier distils. Read it when a claim here needs its original wording or its wider
context; per `ref-docs/README.md` it is frozen, so corrections go in a new dated file rather than
edits to it. This dossier is the layer that adds the confidence marking below; the corpus itself
carries none.

**Confidence marking.** Claims are marked where they are not settled:
**[ERRATA]** = SNESdev Errata page · **[CONFLICT]** = sources disagree, resolution stated ·
**[UNVERIFIED]** = upstream itself marks it untested · **[UNDEFINED]** = hardware genuinely does
not define it, so no assertion is possible.

---

## Part I — The model: AccuracyCoin

### 1.1 What it is

`github.com/100thCoin/AccuracyCoin` — "A large collection of NES accuracy tests on a single NROM
cartridge." Author **Chris Siebert** (handle **100thCoin**), also author of TriCNES, a C# NES
emulator "with a focus on test-driven accuracy". **MIT licensed**, © 2025. ~1.0k stars, 355
commits, 100% assembly, **no tagged releases** — consumers take `AccuracyCoin.nes` from `main`.

Hardware target is load-bearing and explicitly stated: *"This ROM was designed for an NTSC console
with an RP2A03G CPU and RP2C02G PPU. Some tests might be automatically skipped on hardware with a
different revision."*

Design philosophy, from the source header comments: the `.asm` is heavily commented as
documentation-of-record; **macros are deliberately avoided** "since they make the ASM code look
different than the compiled bytes, and thus harder to debug"; the NMI and IRQ vectors point into
**RAM** (`$0700`/`$0600`) so each test installs its own handler. There is an explicit
"don't hang the emulator" ethos — `VblSync_ABORT` bails out of a VBlank-sync loop when a pre-test
shows frame timing is too wrong to ever converge.

Known caveat from the README: the open-bus tests **fail on an EverDrive N8 Pro**. Some real-hardware
failures are the flash cart's fault, not the console's.

### 1.2 Build and structure

The entire repository is nine files:

| File | Size | Role |
|---|---|---|
| `AccuracyCoin.asm` | 679,622 B (18,968 lines) | the entire ROM, one file |
| `AccuracyCoin.nes` | 40,976 B | prebuilt ROM |
| `nesasm.exe` | 77,824 B | **the assembler, committed into the repo** |
| `Sprites.pcx` | 5,506 B | CHR pattern data (sprites) |
| `Tiles.pcx` | 9,631 B | CHR pattern data (background) |
| `README.md` | 53,368 B | the error-code dictionary |
| `LICENSE` | — | MIT |
| `.gitattributes` / `.gitignore` | — | `* text=auto`; ignores `AccuracyCoin.fns` |

- **Assembler: `nesasm`** — not ca65, not asm6. No Makefile, no batch script, no CI. The build is
  a single invocation: `nesasm AccuracyCoin.asm`.
- **CHR is assembled from PCX** via nesasm's importer (lines 18967-18968):
  `.incchr "Sprites.pcx"` / `.incchr "Tiles.pcx"`.
- Header directives: `.inesprg 2` / `.ineschr 1` / `.inesmap 0` / `.inesmir 0`.
- Verified header bytes `4E 45 53 1A 02 01 00 00...` → **NROM-256, 32 KiB PRG, 8 KiB CHR ROM,
  horizontal mirroring, no battery/trainer.** 16 + 32768 + 8192 = 40,976, exactly the file size.
- **CHR is ROM, not RAM** — one test (`CHR ROM is not writable`) depends on this.

**Takeaway for AccuracySNES:** a single self-contained cart with no external dependencies, built by
one command, is achievable and is the right shape. We improve on it by generating the source rather
than hand-maintaining an 18,968-line monolith, and by not committing a Windows binary.

### 1.3 The complete test list

Authoritative source is the `TestPages:` table at `AccuracyCoin.asm:531`, entries of the form
`table "Name", $FF, Result_Address, Test_Entry_Point`.
**20 pages, 146 entries = 141 scored + 5 info-only "DRAW" tests.**

| Page | Suite | n |
|---|---|---|
| 1 | CPU Behavior | 9 |
| 2 | Addressing mode wraparound | 6 |
| 3-9 | Unofficial Instructions: SLO / RLA / SRE / RRA / *AX / DCP / ISC | 7+7+7+7+10+7+7 |
| 10 | Unofficial Instructions: SH* | 6 |
| 11 | Unofficial Immediates | 8 |
| 12 | CPU Interrupts | 3 |
| 13 | APU Registers and DMA tests | 10 |
| 14 | APU Tests | 9 |
| 15 | Power On State | 5 (DRAW, unscored) |
| 16 | PPU Behavior | 8 |
| 17 | PPU VBlank Timing | 7 |
| 18 | Sprite Evaluation | 9 |
| 19 | PPU Misc. | 9 |
| 20 | CPU Behavior 2 | 5 |

Selected detail worth mirroring in structure:

- **Page 1** covers ROM-not-writable, RAM mirroring, PC wraparound, the decimal flag (inert on
  ADC/SBC but still pushed by PHP/BRK), the B flag across PHP/BRK/IRQ/NMI, dummy read cycles,
  dummy write cycles, open bus, and `All NOP instructions` — which individually exercises all 27
  unofficial NOP opcodes.
- **Pages 3-11** are **one test per opcode**, 73 tests total. Each checks operand byte count,
  target address, A/X/Y, P flags, and SP — plus, for SHA/SHX/SHY/SHS, the behavior **when RDY goes
  low 2 cycles before the write cycle**. SHA/SHS report a *variant code* for which ABH-corruption
  behavior occurred.
- **Page 13** is the DMC/OAM DMA suite: DMA + open bus / `$2002` / `$2007` read / `$2007` write /
  `$4015` / `$4016`, DMC DMA bus conflicts, DMC+OAM overlap, explicit and implicit DMA abort.
- **Page 14** includes `Length Table` (all 32 entries individually) and `Frame Counter IRQ`
  (15 sub-checks including the exact 29827/29828/29829/29830-cycle flag windows).
- **Page 15** prints values only — uninitialized CPU RAM, CPU registers at power-on, PPU RAM,
  palette RAM. These are the model for AccuracySNES's **golden-vector class**.
- **Page 20** `Implied Dummy Reads` verifies a cycle-2 dummy read for **all 25 implied/stack
  instructions individually**, plus RTS's cycle-6 dummy read.

Two tests are **commented out** in the source, and the reasoning is instructive:
`RMW $2007 Extra Write` (removed — several revision-G PPUs fail it, "more research is needed") and
`Palette Corruption` ("I did not write a test for this, because it relies on a specific cpu/ppu
clock alignment").

**Not tested:** no mapper tests (the ROM is NROM and never touches a mapper), and no audio *output*
tests — the APU is exercised purely through register/timing/IRQ side effects.

### 1.4 Result protocol

Results are written to fixed RAM addresses in **`$0400-$0492`**, one per test, named by
`result_*` constants at `AccuracyCoin.asm:148-320`. That constant block is effectively the
machine-readable manifest.

Encoding, from the summary routine at `AccuracyCoin.asm:948-966`:

| Byte | Meaning |
|---|---|
| `$FF` | skipped |
| `bit0 == 1` | **PASS**; `value & $FE` carries the **variant code** for multi-behavior tests |
| `bit0 == 0`, non-zero | **FAIL**; the byte *is* the hex error code identifying the sub-assertion |
| `$00` | not run |

Tallies live at zero-page `$37` (`PostAllTestTally`) and `$38` (`PostAllPassTally`).

Two oddities worth knowing: `result_UnOp_SRE_47 = $47F`, deliberately relocated so `$0421` stays
`$00` for the Implied-Dummy-Reads test; and all 5 DRAW tests sink to `result_DrawTest = $03FF`,
which is how the tally excludes them (it skips any entry whose result page is 3 rather than 4).

On screen: `PASS` or `FAIL` next to the name, with a **hex/alphanumeric error code (1-9, then
A-Z)** on failure. The README is the code dictionary — one entry per code per test. The summary is
a grid, one column per page, one cell per test: blue square `$FE` = pass, error-code character =
fail, `$C9` = skipped, with light-blue **variant numbers drawn as sprites over the cell**. Below:
`Tests passed: <pass> / <total>` plus `Tests skipped: N`.

**The critical gap:** there is **no magic header, no version field, no test count, no completion
sentinel**. A harness must hard-code the address map by transcribing it from the `.asm`. This is
the single biggest usability defect for emulator CI, and the thing AccuracySNES most clearly
improves on.

### 1.5 Menu and automation

- D-Pad moves the cursor. **A** = run the highlighted test. **B** = mark/unmark it *skipped* —
  explicitly "useful if any tests are crashing the console or emulator and you still want to see
  the table of results."
- With the cursor on the page-index header line: **Left/Right** change page, **A** runs the whole
  page, **B** skip-marks the page, **Start** runs **every test in the ROM** and draws the results
  table (`AutomaticallyRunEveryTestInROM`).
- **Select** after a test opens a debug view of `$20-$2F`, `$50-$6F`, and all of `$0500-$05FF`.

**No music, no sound engine.** Grepping the source for any playback code returns nothing; the APU
is written to only as a test instrument, and the auto-run routine does `LDA #0 / STA $4015` to
silence everything when finished. Expect incidental noise during APU tests.

**Visuals** are text-mode menus from `Tiles.pcx` (font/box-drawing) and `Sprites.pcx` (OAM), with
two palettes. Many PPU tests **self-verify via sprite-0 hit** rather than visible output —
`Attributes As Tiles`, `$2007 read w/ rendering`, `Suddenly Resize Sprite`, `t Register Quirks`,
`Stale BG/Sprite Shift Registers`, `BG Serial In`, `ALE + Read`, `Hybrid Addresses`, `INC $4014`
all state "results are tested via a sprite zero hit."

**On the name:** there is no coin scoring mechanic. Grepping all 19k lines, the only occurrence of
"coin" is the title banner. The name is a portmanteau of "accuracy" and the author's handle — the
same pattern as TriCNES. Scoring is a plain `passed / total`.

### 1.6 How other emulators score

Totals differ because the test count has grown over time (125 → 131 → 136 → 141):

| Emulator | Score | Source |
|---|---|---|
| AprNes (C#) | **136/136** (drops 5 with full-accuracy mode off) | nesdev forum t=26533 |
| ares | 106/131 | ares-emulator/ares#2275 |
| jgenesis | 89/125 | jsgroth/jgenesis#551 |
| Mesen 2 / MesenCE | actively tuned against it ("passes 11 more AccuracyCoin tests" in 2.2.1) | MesenCE release notes |

No official leaderboard exists; scores circulate as GitHub issues and forum posts. **ares' failure
list is the most useful calibration reference** for what a mature emulator still misses:
PC-executing-from-open-bus, SH* target addresses, DMC-DMA cycle counts and bus conflicts,
frame-counter get/put IRQ clearing, `$4015` DMC-timer write races, controller strobe on put→get,
6-bit palette reads with open-bus upper bits, arbitrary/misaligned sprite zero, `$2004` reads
during cycles 256-320, OAM corruption, stale BG shift registers, scanline-0 sprites via pre-render
fetch, implied dummy-read data-bus updates, and JSR leaving operand 2 on the bus.

---

## Part II — How RustyNES consumes AccuracyCoin

### 2.1 Where it lives

Two deliberately case-split sibling directories, both tracked in git:

| Path | Contents |
|---|---|
| `tests/roms/accuracycoin/` | `AccuracyCoin.nes` (40 KiB), `LICENSE` (MIT), `README.md` |
| `tests/roms/AccuracyCoin/` | `SOURCE_CATALOG.tsv` (146 rows), `sub-tests/*.nes` (27 derived ROMs), `README.md` |

Not a submodule (`.gitmodules` does not exist). No build script for the main ROM — the `.nes` is
committed verbatim from upstream. Committing is explicit in `.gitignore:82`
(`!/tests/roms/**/*.nes`, a whitelist punched through a broader ignore).

**Upstream source is NOT vendored.** `AccuracyCoin.asm` exists only at `ref-proj/AccuracyCoin/`,
which is gitignored. The only compile-time artifact derived from upstream is `SOURCE_CATALOG.tsv`,
extracted by the recipe documented at `tests/roms/AccuracyCoin/README.md:38-43`:

> walk each `Suite_*` block, and for every `table "name", $FF, result_symbol, TEST_addr` macro
> entry emit a `(suite, test-name, ram-addr)` triple, resolving `result_symbol` to its
> `result_X = $ADDR` definition.

### 2.2 The two decoders — and the calibration lesson

**This is the single most important finding in this dossier.**

RustyNES built a **framebuffer grid decoder first**
(`crates/rustynes-test-harness/src/accuracy_coin.rs`), sampling one pixel per cell centre with
exact-RGB nearest-neighbour classification:

```rust
pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
    match (r, g, b) {
        (0x64, 0xA0, 0xFF) => Self::Pass,
        (0x4F, 0x10, 0x00) => Self::Fail,
        (0xDC, 0x83, 0x4C) => Self::Partial,
        (0x4C, 0x4C, 0x4C) => Self::NotRun,
        _ => Self::Unknown,
    }
}
```

with geometry `GRID_ROWS = 10`, `GRID_COLS = 16`, `CELL_X_STRIDE = 10`, `CELL_Y_STRIDE = 8`.

**That geometry was wrong.** The ROM's layout is 20 columns at stride 8. From
`crates/rustynes-test-harness/tests/accuracycoin.rs:52-58`:

> The framebuffer decoder had a grid-stride bug (16 cols × stride 10 vs the ROM's 20 cols ×
> stride 8) and silently missed 31 cells — pages 5, 10, 15, 20 in the ROM's layout. **This is a
> calibration correction, not an accuracy regression.** The same emulator state that measured
> `75.93%` via framebuffer measures `64.03%` via RAM.

A screen-scraping oracle **under-samples silently and reads high** — it does not fail loudly, it
reports a better number than the truth. RustyNES did not delete the framebuffer decoder; it added
a **RAM-direct decoder as authoritative** (`accuracy_coin_catalog.rs`) and demoted the framebuffer
one to a divergence alarm that logs but does not fail.

The RAM decoder mirrors the ROM's own display routine exactly:

```rust
pub const fn from_byte(b: u8) -> Self {
    match b {
        0x00 => Self::NotRun,
        0x01 => Self::Pass,
        0xFF => Self::Skipped,
        _ => {
            let code = b >> 2;
            if b & 0x01 != 0 { Self::PassWithCode(code) }
            else if b & 0x02 != 0 { Self::Fail(code) }
            else { Self::Unknown(b) }
        }
    }
}
```

and the catalog is `include_str!`'d so the in-code table cannot drift from disk:

```rust
const RAW_TSV: &str = include_str!("../../../tests/roms/AccuracyCoin/SOURCE_CATALOG.tsv");
```

parsed lazily via `OnceLock`.

### 2.3 Scoring

The metric is derived from the ROM's own tally routine (`AccuracyCoin.asm:1042-1047`), not from
intuition:

```rust
pub fn pass_rate(&self) -> f64 {
    let num = self.pass + self.pass_with_code;
    let denom = num + self.fail + self.unknown;
    if denom == 0 { 0.0 } else { f64::from(num) / f64::from(denom) }
}
```

`NotRun` and `Skipped` are **excluded from the denominator** — which is why the catalog has 146
rows but 141 assigned tests (the 5 DRAW tests share the `$03FF` sentinel and always decode as
`NotRun`).

The gate carries a deliberately loose floor with a ratchet rule stated in its own rustdoc
(*"When the measured rate exceeds those thresholds in CI, raise this constant — don't lower it."*):

```rust
const MIN_PASS_RATE: f64 = 0.60;
```

plus a **liveness assert** (`summary.assigned() > 0`, "battery may have never started") so a total
non-boot fails loudly rather than scoring 0/0. A per-suite breakdown table and a per-failing-test
list with names and error codes are printed **unconditionally**, so CI logs form a time series.

The catalog is pinned by unit tests against silent drift: exact row count (146), all addresses
`< 0x0800`, all 20 suite names hardcoded, and index 0 / index 145 pinned by name.

RustyNES currently reports **141/141 = 100.00%** (RAM-authoritative). Note: the "82.73%" figure in
the workspace-level `CLAUDE.md` is a **RustyNES_v2 number and is stale for that repo.**

### 2.4 Menu automation and the source-patch escape hatch

The in-process harness needs exactly **one button press**, because upstream provides a run-all
entry point:

```rust
for _ in 0..300 { nes.run_frame(); }        // wait for splash + menu
nes.set_buttons(0, Buttons::START);
for _ in 0..6 { nes.run_frame(); }
nes.set_buttons(0, Buttons::empty());
```

Completion is a **stability poll, not a fixed frame count**: sample every 600 frames; when the
not-run count stops changing for 1800 frames *and* the grid is not still all-gray, stop. Budget
72,000 frames (~20 min NES time, ~3x the actual ~4200-frame completion) — bounding CI wall time,
not timing out a legitimate run.

Where menu-driving was genuinely infeasible — driving the **external** Mesen2 oracle, whose
`testRunner` under Xvfb stalls at the menu spin loop around frame 1589 at ~7 FPS — RustyNES did
not automate harder. It **rebuilt the ROM**: `scripts/accuracycoin-build/build_sub_test_rom.py`
patches upstream `AccuracyCoin.asm` with two regex surgeries to boot straight into one test,
reaching the target by **frame ~30** instead of ~3000. 27 such ROMs are committed.

Two correctness fixes baked into that wrapper are exactly the class of bug an SNES equivalent would
hit:

- **Controller pre-drain** — in the full battery the menu's NMI handler calls `ReadController1`
  every frame, leaving the shift register fully shifted before each test. Without replicating that,
  `TEST_ControllerStrobing` test 1 fails spuriously.
- **Cross-test prerequisite pre-seeding** — some tests read zero-page bytes set by *earlier* tests;
  in single-test mode those never ran, so the script pre-seeds them. Comment: *"better to over-seed
  than to silently degrade the diagnostic."*

### 2.5 Design lessons adopted

1. Screen-scraping is a trap. Find the in-RAM result protocol and decode that; keep the visual
   decoder only as a divergence alarm.
2. `include_str!` the catalog; pin its size, suite names, and endpoint indices with unit tests.
3. Derive the metric from the ROM's own tally routine, not from your own notion of what counts.
4. Ratchet the floor upward only, and state that rule in the constant's rustdoc.
5. Print the per-suite table and every failing test name+code unconditionally.
6. Poll for stability; keep a generous budget that bounds CI, not the run.
7. Replicate the battery's implicit side effects (input shift state, cross-test prerequisites) or
   chase phantom failures.
8. Gate behind a marker feature at module level and run `--release`.

---

## Part III — RustySNES test infrastructure today

### 3.1 The corpus

```text
tests/roms/
  README.md                 the corpus manifest (two-tier licensing table)
  commercial-corpus.json    metadata + SHA-256 ONLY, never ROM bytes
  gilyon/                   COMMITTED (MIT)      3 .sfc + golden .txt tables
  undisbeliever/            COMMITTED (MIT/Zlib) 29 .sfc (hdma-*, inidisp_*, scpu-a-dma-bug-*)
  spc700-singlestep/        COMMITTED (MIT)      256 JSON, 50-per-opcode sample = 12,800 cases
  external/                 GITIGNORED (~3.3 GB)
```

External tier, measured: `65816-singlestep` 2.7 GB (**no license**), `krom` 398 MB (**no
license**), `commercial` 252 MB, `spc700-singlestep-full` 81 MB, `240p` 6.5 MB (GPL-2.0+),
`blargg-spc` 812 KB (unstated), `firmware` 292 KB.

Gitignore mechanics (`.gitignore:81-85`) — a broad exclude with explicit re-includes:

```text
!/tests/roms/**/*.sfc
!/tests/roms/gilyon/**
!/tests/roms/undisbeliever/**
/tests/roms/external/
```

**AccuracySNES follows this exact pattern**, adding `!/tests/roms/AccuracySNES/**`.

### 3.2 The harness — skeletons vs. real tests

**`src/` is a set of unimplemented skeletons; every real oracle lives in `tests/`.** From
`src/lib.rs:1-8`: *"Each piece is a SKELETON with a clear surface + `TODO(T-PS-NNN)` markers."*

| Skeleton | State |
|---|---|
| `src/runner.rs::run_until_complete` | breaks after 1 step, always returns `Timeout` |
| `src/accuracy_battery.rs::score_accuracy_battery` | returns `AccuracyReport::default()` (zeros) |
| `src/golden_log.rs::parse_golden_line` | returns `None` |
| `src/visual.rs::hash_frame` | real FNV-1a, but unused by any test |

`AccuracyReport { passed, failed, partial }` with `total()`, `pass_rate()`, and `meets()` **is**
implemented and unit-tested — that part is reusable as-is. `docs/STATUS.md:161-166` declares the
rest dead:

> An early skeleton for exactly that approach exists (`rustysnes-test-harness::accuracy_battery`,
> ticket T-04) but was never implemented and has since been superseded — no publicly available
> SNES ROM plays the AccuracyCoin role … that skeleton is tracked as dead code to remove in a
> follow-up, not a competing source of truth.

The 14 real integration tests are standalone files in `tests/`.

### 3.3 The five result-detection mechanisms in use

There is no blargg-`$6000`-style status protocol anywhere in the SNES corpus.

**(a) WRAM counter + VRAM tilemap character** — `gilyon_oncart.rs`. Reads `test_num` at WRAM
`$0010` and the result tile at VRAM word `$32`, asserting `tile & 0xFF == 0x53` ('S' for
"Success"). Completion is detected by **PC stability across 60 consecutive frames**.

**(b) Tilemap ASCII OCR** — `blargg_spc.rs`. blargg exposes no status byte, so the harness reads
the BG tilemap where `tile & 0xFF == ASCII` at VRAM `$0400` (header) and `$0800` (result grid),
accumulating across 12,000 frames because the grid scrolls, then decides on `contains("passed")` /
`contains("failed")`.

**(c) Golden framebuffer FNV-1a hash** — `undisbeliever_golden.rs`. Boots 60 frames, hashes the
`&[u16]` BGR555 framebuffer, compares against a committed `name\t0xHEX` TSV. Every ROM is booted
**twice** and the hashes asserted equal before the golden comparison — the determinism contract.

**(d) JSON per-opcode oracle** — `cpu_oracle.rs` / `spc700_oracle.rs`. No ROM involved; a flat
24-bit `HashMap` bus, seed → `cpu.step()` → diff registers, RAM, and cycle count.

**(e) Coprocessor liveness counters** — `dsp1_oncart.rs`, `superfx_oncart.rs`, `sa1_oncart.rs`.
Access counts, plot-pipeline byte counts, and a firmware-differential (booting without firmware
must produce a *different* framebuffer hash).

**(f) Honesty gate** — `mapper_tier_honesty.rs` asserts no BestEffort board backs the oracle. This
is the structural template for AccuracySNES's provenance-tier gate.

### 3.4 The established pattern

Every real test does this, **bypassing `EmuCore` entirely**:

```rust
let rom = std::fs::read(path)?;
let cart = Cart::from_rom(&rom)?;
let mut sys = System::new(0);
sys.bus.cart = Some(cart);
sys.reset();
for _ in 0..FRAMES { sys.run_frame(); }
// read via sys.bus.peek_wram() / sys.bus.ppu.vram_word() / sys.bus.framebuffer()
```

`Bus::peek_wram(addr24)` (`bus.rs:398`) is the results-block reader — its contract is explicitly
*"does NOT advance the clock, touch open bus, or trip register side effects"*, and it covers banks
`$7E-$7F` plus the low-8 KiB mirrors. `Bus::poke_wram` is the write counterpart.

`EmuCore` (`facade.rs`) offers `set_pad`/`run_frame`/`framebuffer` (RGBA8) but **no memory or VRAM
peek** — deliberately, per its own doc: *"Debugger-only concerns … stay OUTSIDE this facade."* The
escape hatch is `EmuCore::system_mut()`.

### 3.5 Feature gates and CI

```toml
[features]
default = []
test-roms = []          # the committed suites + integration tests
commercial-roms = []    # the local-dump oracle (ROMs gitignored)
```

Every real test file opens with `#![cfg(feature = "test-roms")]` at **module** level. CI runs
exactly one line (`ci.yml:187`): `cargo test --workspace --features test-roms`. Nothing in CI ever
sets `commercial-roms`. Every `test-roms` test **self-skips with `eprintln!("SKIP …")` and
returns** when its corpus is absent, so a fresh clone stays green.

### 3.6 There is no ROM build tooling

**Zero.** No Makefile, no assembler invocation, no `build.rs` that assembles anything. Every `.sfc`
in the tree is a prebuilt upstream binary. The only assembler references are upstream READMEs
describing how *their* authors built the ROMs (gilyon: ca65/ld65 + Python; krom: bass v14; 240p:
ca65/PVSnesLib). The frontend's inline 65816 assembler is explicitly *not built*
(`debugger/mod.rs:12`), though a disassembler ships.

AccuracySNES introduces the first in-repo ROM build pipeline.

---

## Part IV — Toolchain validation (performed 2026-07-19)

`ca65` / `ld65` **v2.19** are already installed at `~/.local/bin/`. A probe ROM was built this
session to validate the approach rather than assume it.

A hand-written LoROM linker config plus a `.p816` source assembled and linked cleanly to an
**exactly 32,768-byte `.sfc`**:

```text
00000000: 7818 fbc2 30a9 3412 8f00 007e 80fe
          SEI CLC XCE  REP #$30  LDA #$1234  STA $7E0000  BRA -2
00007ffc: 0080                    <- emulation RESET vector = $8000, correct
```

Working config shape (sizes matter — `ROM0` must be `$7FB0` so the header lands at `$FFC0`):

```text
MEMORY {
  ROM0: start = $008000, size = $7FB0, type = ro, fill = yes, file = %O;
  HDR:  start = $00FFC0, size = $0030, type = ro, fill = yes, file = %O;
  VEC:  start = $00FFE0, size = $0020, type = ro, fill = yes, file = %O;
}
```

**One gap identified.** `ld65` cannot compute the SNES header checksum, and RustySNES's own header
detection scores it as the **strongest signal**. From `crates/rustysnes-cart/src/header.rs:326-357`:

```rust
// 1. Checksum + complement sum to $FFFF — the strongest signal.
if checksum != 0 && complement != 0 && checksum ^ complement == 0xFFFF { score += 8; }
if mode & 0xE0 == 0x20 && mode & 0x0F == expected_mode_nibble(map_mode) { score += 4; }
if reset >= 0x8000 { score += 2; }
if /* printable 21-byte title */ { score += 1; }
```

Maximum score 15. The generator must therefore emit the header and patch the checksum/complement
post-link — a solved problem, assigned to the Rust generator crate.

---

## Part V — SNES hardware behavior catalog

Numbering `<GROUP><page>.<test>`. Each row is one menu entry with a hex error code per
sub-assertion. Target total ~320 tests.

### 5.0 The SNESdev Errata page — the master quirk list

`snes.nesdev.org/wiki/Errata` is effectively a pre-made test list.

**Video** — offset-per-tile never affects the leftmost tile · color math on sprites only for
palettes 4-7 · enabling NMI while RDNMI is already set fires an immediate NMI · sprite overflow
drops **high**-priority slivers · Time Over false positive (first sprite 16x16+ at X=0-255 with
others at negative X) · sprite X=-256 still consumes range and sliver slots · INIDISP brightness
fades over 72+ px on 1CHIP · INIDISP early-read bug · 16x32/32x64 broken under OBJ interlace, with
V-flip flipping halves independently.

**S-SMP** — 16-bit writes to `$2140/41` may also write `$2143` · simultaneous CPU/SPC access to
`$2140-43` corrupts data · `$F0` TEST writes can crash the SPC700.

**S-DSP** — fixed release rate · ADSR/GAIN mid-note race (write ADSR2/GAIN before ADSR1) · noise is
highpass-filtered · `EDL=0` overwrites 4 bytes at the buffer start · EDL takes effect only at
buffer end (up to 7680 samples / 240 ms) · ESA delayed one sample · echo wraps at a 16-bit
boundary, corrupting page zero · **KON/KOFF polled every second sample** · three overflow bugs
(BRR clip, FIR clip on the first 7 taps, Gaussian overflow on three consecutive max-negative
samples).

**SPC700** — `TSET1`/`TCLR1` are equality tests not bit tests · `MUL` flags from Y only · `DIV`
valid only for quotient ≤ 511 · `DIV` flags from A only.

**Mode 7** — MPY corrupted if an interrupt or HDMA writes BG1 scroll or an M7 matrix register
**between the two M7A writes** (shared latch).

**65C816** — SEP/PLP/XCE to 8-bit index **clears XH/YH** · `JMP (a)`/`JMP [a]` read bank $00 ·
`JMP (a,X)`/`JSR (a,X)` read the program bank · MVN/MVP overwrite DB with the destination bank.

**5A22** — overlapping `$4203`/`$4206` yields **[UNDEFINED]** RDDIV/RDMPY · invalid DMA A-bus
addresses (`$21xx`, `$4000-41FF`, `$4200-421F`, `$4300-437F`, WRAM-with-`$2180`) · **v1 crashes if
DMA finishes just before HDMA** · **v2: recent HDMA to INIDISP blocks the next DMA** · HDMA fails
for the whole frame if a DMA ends at scanline 0 start with previous read value 0 · enabling HDMA
outside vblank causes erroneous PPU writes.

**Input** — auto-read begins between H=32.5 and H=95.5 of the first vblank line · results may
change during lag frames.

**Hardware** — **cartridge /RESET does not reset the PPU** (only S-CPU, APU, S-WRAM).

### 5.1 Behavior-to-game motivation

From `snes.nesdev.org/wiki/Tricky-to-emulate_games`. Every row has proven real-world stakes.

| Behavior | Titles |
|---|---|
| CPU/PPU open bus | Captain America and the Avengers, The Combatribes, Home Alone, Rock n' Roll Racing, Super 3D Noah's Ark |
| BRK/COP implementation | Actraiser, Cybernator, Illusion of Gaia, Soul Blazer, Kamaitachi no Yoru, Dekitate High School, Sailor Moon Another Story |
| `ORA [d]` | Super Mario World |
| OAM priority rotation | Super Mario World |
| VRAM write during rendering | Hook |
| VRAM address increment during rendering | Kick Off |
| VRAM read behavior | Breath of Fire |
| Offset-per-tile | Axelay, Chrono Trigger; wraparound: Super Famista 5 |
| Mode 7 window logic | The Atlas: Renaissance Voyager, MechWarrior |
| Mode 7 scroll offset latch timing | NHL '94 |
| Mode 7 direct color | Aerobiz |
| Color window + DMA B-bus access | Krusty's Super Fun House |
| Color math on subscreen in hi-res | Jurassic Park |
| **OAM write timing during rendering** | **Uniracers** |
| OAM fetch/render timing | Mega lo Mania |
| DMA power-on state | Heian Fuuunden |
| CPU cycle timing before DMA start | MMPR: The Fighting Edition |
| DMA/HDMA timing | Circuit USA, Jumbo Ozaki no Hole in One |
| HDMA fixed-transfer flag | Batman Forever, The Lost Vikings |
| HDMA decrement flag | The Adventures of Kid Kleets |
| HDMA direction flag + NMI enable timing | Pocky & Rocky |
| HDMA transfer-flag state | Aladdin, Super Ghouls'n Ghosts |
| DMA suspension during HDMA | Dekitate High School |
| V-IRQ trigger conditions | RoboCop versus The Terminator |
| NMI during vblank | Alien vs Predator |
| NMI vector execution timing | Jaki Crush |
| SPC cycle-level timing | ActRaiser 2, Hiouden, Tales of Phantasia, Illusion of Gaia |
| CPU read effect timing | Rendering Ranger R2 |
| DSP `KOF` register init | Chester Cheetah, King of Dragons |
| SRAM mapping | Fire Emblem: Thracia 776, Ys III |
| RAM power-on state | Death Brade, Power Drive |
| SRAM power-on state | Super Keiba 2 |
| Super FX `RPIX` | Yoshi's Island |
| **HDMA x open bus** | **Speedy Gonzales: Los Gatos Bandidos** |

Uncommon-mode coverage targets: Mode 0 (Super Mario Kart driver select, FFIV/FFV menus), Mode 3
8bpp (DKC logo, SimCity 2000), Mode 4 8bpp (Bust-a-Move), Mode 7 EXTBG (Contra 3 L2, Super
Turrican 2), Direct Color (Actraiser 2, Secret of Mana world map), hi-res (Jurassic Park, Chrono
Trigger, DKC), Mode 5 hi-res Japanese text (Seiken Densetsu 3, Rudra no Hihou, Marvelous),
horizontal OPT (Chrono Trigger Black Omen), vertical OPT (Axelay L2, Star Fox, Tetris Attack,
Yoshi's Island), overscan/239-line (Dragon Quest I&II, Rendering Ranger R2, PAL SMW).

### 5.A Group A — 65C816 CPU (~55 tests)

#### A1. Emulation vs native mode (9)

| # | Assertion |
|---|---|
| A1.01 | `SEC; XCE` forces m=1, x=1, SH=$01, XH=$00, YH=$00 |
| A1.02 | `CLC; XCE` changes nothing except E |
| A1.03 | `REP #$30` while E=1 leaves m=x=1 |
| A1.04 | **Index narrowing destroys XH/YH**: `LDX #$1234; SEP #$10; REP #$10` → XH=$00 **[ERRATA]** |
| A1.05 | Same via PLP and via XCE — all three paths identical |
| A1.06 | `TXS` in emulation: X=$FF → S=$01FF, not $00FF |
| A1.07 | `TCS` in emulation: A=$1234 → S=$0134 (SH forced $01) |
| A1.08 | `TCD`/`TSC`/`TDC` always 16-bit regardless of m |
| A1.09 | `TCS`/`TXS` set no flags; all other transfers set N/Z |

#### A2. Direct-page wrapping (10)

| # | Assertion |
|---|---|
| A2.01 | **`d,X` never crosses bank**: `D=$8001, X=$FFFE, LDA $01,X` → `$00:8000`, not `$01:8000` |
| A2.02 | E=1, DL=$00 → page wrap: `D=$0000, X=$01, LDA $FF,X` → `$00:0000` |
| A2.03 | E=1, DL≠$00 → no wrap: `D=$0010, X=$01, LDA $FF,X` → `$00:0110` |
| A2.04 | Native always carries: `E=0, D=$0000, X=$01, LDA $FF,X` → `$00:0100` |
| A2.05 | 16-bit `d` read across page boundary **[UNVERIFIED — superfamicom.org says "theoretically"]** |
| A2.06 | `[dp]` is a "new" mode — never page-wraps even at E=1/DL=$00 |
| A2.07 | `(dp),Y` always bank-carries after the pointer load |
| A2.08 | `[dp],Y` same |
| A2.09 | `(dp,X)` inherits the d,X rules |
| A2.10 | `PEI (dp)` never page-wraps |

#### A3. Stack wrapping (10)

| # | Assertion |
|---|---|
| A3.01 | E=1, S=$01FF, `PLA` → pulls `$00:0100` |
| A3.02 | **`PEA` escapes `$01xx`**: E=1, S=$0100 → writes $00:0100 and $00:00FF, S=$01FE, $01FF untouched |
| A3.03 | **`PLD` escapes**: E=1, S=$01FF → reads `$00:0200/0201` |
| A3.04 | **`PLY` does NOT escape**: same S → reads `$00:0100`. A3.03 vs A3.04 is the old-vs-new discriminator |
| A3.05 | `d,S` escapes: E=1, S=$01FF, `LDA $02,S` → `$00:0201` |
| A3.06 | `(d,S),Y` escapes and bank-carries |
| A3.07 | `JSL`/`RTL` escape |
| A3.08 | `JSR (a,X)` escapes |
| A3.09 | `PHD`/`PER` escape |
| A3.10 | Stack confined to bank $00: native S=$0000, `PHA` → `$00:0000`, next push wraps to `$00:FFFF` |

WDC's list of instructions escaping `$01xx` even in emulation mode: `JSL`, `JSR (a,X)`, `PEA`,
`PEI`, `PER`, `PHD`, `PLD`, `RTL`, `d,S`, `(d,S),Y`. Hardware-confirmed for PEA, PLD, d,S, (d,S),Y.

#### A4. Absolute / jump wrapping (10)

| # | Assertion |
|---|---|
| A4.01 | NMOS `JMP ($12FF)` page bug is **FIXED** — high byte from `$00:1300` |
| A4.02 | `JMP (a)` pointer read from **bank $00** **[ERRATA]** |
| A4.03 | `JML [a]` pointer from bank $00, full 24-bit destination |
| A4.04 | `JMP (a,X)` pointer from the **program bank**, wrapping in-bank: PBR=$05, X=$04, `JMP ($FFFE,X)` → `$05:0002` **[ERRATA]** |
| A4.05 | `JSR (a,X)` same rule |
| A4.06 | `abs,X` bank carry: DBR=$00, X=$80, `LDA $FFC0,X` → `$01:0040` |
| A4.07 | 16-bit abs read carries: m=0, `LDA $FFFF` → low `$00:FFFF`, high `$01:0000` |
| A4.08 | `long,X` bank carry |
| A4.09 | PC wraps within bank on operand fetch |
| A4.10 | Branch target wrap **[UNVERIFIED — upstream literally marks `r`/`rl` "XXX: untested"]** |

#### A5. Cycle counts (22 assertions in 15 rows)

| # | Assertion |
|---|---|
| A5.01-08 | Base sweep, all 256 opcodes at m=1,x=1,e=0,DL=$00, no page cross |
| A5.09 | `+1 m` sweep under `REP #$20` |
| A5.10 | `+1 x` sweep under `REP #$10` |
| A5.11 | **`+1 w` (DL≠0) sweep** — D=$0000 vs D=$0001, exactly +1. The most commonly mis-implemented penalty |
| A5.12 | `+1 p` on reads: `LDA $00FF,X` X=$00 vs X=$01 |
| A5.13 | **Stores have NO p penalty** — `STA a,X`, `STA a,Y`, `STZ a,X`, `STA (d),Y` always pay the higher count |
| A5.14 | RMW `abs,X` flat (e.g. `ASL $1234,X` = 7) |
| A5.15 | Branch = `2 + t + t*e*p`; page-cross penalty **only when E=1** |
| A5.16 | `BRL` flat 4, never penalized |
| A5.17 | **16-bit RMW is +2, not +1.** Not a transcription error since corrected, as this table previously said — undisbeliever's published table is **internally inconsistent to this day**: `ASL`/`INC`/`DEC`/`TRB`/`TSB` say `+2 if m=0` while `LSR`/`ROL`/`ROR` say `+1`, twelve rows disagreeing with the rest of their own instruction class. Verified 2026-07-20 against `ref-docs/2026-07-20-undisbeliever-65816-timing.md`. Commit `de84e932` (2021-03-13) fixed a *different* RMW bug (a bogus page-cross penalty on absolute-indexed forms) and did not touch this |
| A5.18 | `BRK` 8 native / 7 emulation |
| A5.19 | `RTI` 7 native / 6 emulation |
| A5.20 | `MVN`/`MVP` 7 cycles per byte **[NOT CART-MEASURABLE — the sources do not decompose the 7 cycles into bus vs internal, and a cartridge reads dots rather than clocks; see `docs/accuracysnes-plan.md` §A5.20 and ticket T-06-A]** |
| A5.21 | **Decimal mode adds ZERO cycles** (unlike 65C02) |
| A5.22 | Spot checks: PHD=4, PLD=5, PEA=5, PEI=6+w, PER=6, REP/SEP=3, XBA=3 |

#### A6. Interrupts (15)

| # | Assertion |
|---|---|
| A6.01 | Native vectors COP `$FFE4`, BRK `$FFE6`, ABORT `$FFE8`, NMI `$FFEA`, IRQ `$FFEE` — read from **and jumped to in bank $00** |
| A6.02 | Emulation vectors COP `$FFF4`, ABORT `$FFF8`, NMI `$FFFA`, RESET `$FFFC`, IRQ/BRK `$FFFE` |
| A6.03 | Native pushes 4 bytes (PBR, PCH, PCL, P) |
| A6.04 | Emulation pushes 3 (no PBR) |
| A6.05 | B flag discriminates BRK from hardware IRQ in emulation |
| A6.06 | **D cleared on ALL interrupts** — `SED; BRK` → handler sees D=0 |
| A6.07 | I set; m/x unchanged |
| A6.08 | BRK/COP are 2-byte; pushed PC = PC+2 |
| A6.09 | PBR = $00 in the handler |
| A6.10 | RTI must match mode (native pulls PBR, emulation does not) |
| A6.11 | `WAI` + IRQ with I=1 resumes without vectoring |
| A6.12 | `WAI` wake latency = 1 cycle |
| A6.13 | `STP` halts until reset **[NOT CART-MEASURABLE — a self-scoring cart must keep running to report; see `docs/accuracysnes-plan.md` §A6.13]** |
| A6.14 | `WDM ($42)` = 2-byte NOP |
| A6.15 | All 256 opcodes defined; only STP hangs |

**Note:** ABORT is not wired on the 5A22 — its vectors are unused. Reset always forces E=1, so the
"native RESET" slot `$FFEC` some references list does not exist in practice.

#### A7. Decimal mode (5)

| # | Assertion |
|---|---|
| A7.01 | `SED; CLC; LDA #$09; ADC #$01` → A=$10, C=0 |
| A7.02 | 16-bit `LDA #$0999; ADC #$0001` → A=$1000 |
| A7.03 | 8/16-bit `SBC` |
| A7.04 | **N/Z valid in decimal** (65C02-like) |
| A7.05 | **V is meaningless → golden vector, never asserted** **[UNDEFINED]** |

#### A8. Block move (6)

| # | Assertion |
|---|---|
| A8.01 | Machine encoding is `$54 <dest> <src>` — **destination bank byte FIRST**, opposite of assembly syntax |
| A8.02 | Terminal state: A=$FFFF, DB = destination bank **permanently** |
| A8.03 | MVN: X=X0+N, Y=Y0+N; MVP: X=X0-N, Y=Y0-N |
| A8.04 | X wraps within srcBank, Y within destBank, independently |
| A8.05 | E=1 confines offsets to `$00xx` |
| A8.06 | **Interruptible mid-block** — NMI + RTI mid-MVN resumes correctly **[UNVERIFIED — undocumented upstream]** |

#### A9. Misc (3)

| # | Assertion |
|---|---|
| A9.01 | `BIT #imm` affects Z only |
| A9.02 | `BIT abs` sets N=bit15/7, V=bit14/6 |
| A9.03 | `ORA [d]` (the Super Mario World case) |

### 5.B Group B — 5A22 bus, clock, timing (~30 tests)

#### B1. Memory access speed (5)

| Range | Master clocks |
|---|---|
| `$00-$3F:$0000-$1FFF` WRAM mirror | 8 |
| `$00-$3F:$2000-$3FFF` B-bus | 6 |
| `$00-$3F:$4000-$41FF` JOYSER | **12** |
| `$00-$3F:$4200-$5FFF` CPU MMIO | 6 |
| `$00-$3F:$6000-$FFFF` | 8 |
| `$40-$7F` | 8 |
| `$80-$BF:$8000-$FFFF`, `$C0-$FF` | **6 if MEMSEL=1 else 8** |
| Internal cycles | 6 |

| # | Assertion |
|---|---|
| B1.01 | FastROM toggle observable |
| B1.02 | joypad port measurably slower |
| B1.03 | internal cycles always 6 |
| B1.04 | **DMA is 8 clocks/byte, region-independent** |
| B1.05 | Derived rates: /6 = 3.579545 MHz, /8 = 2.684658 MHz, /12 = 1.789772 MHz |

#### B2. Scanline / frame geometry (10 assertions in 9 rows)

| # | Assertion |
|---|---|
| B2.01 | 1364 clocks per normal line = **340 dots, with dots 323 and 327 taking 6 master cycles**, all others 4 |
| B2.02 | **Short scanline**: line $F0 (240) on alternating non-interlace frames = 1360 clocks |
| B2.03 | Long scanline: PAL interlace field=1, V=311 = 1368 clocks / 341 dots |
| B2.04 | NTSC frame = 262 lines / 357,368 clocks (alternating 357,364) |
| B2.05 | PAL frame = 312 lines / 425,568 clocks |
| B2.06 | Interlace adds a line when `$213F.7 = 0` |
| B2.07 | NTSC 60.0988 Hz · B2.08 PAL 50.00698 Hz |
| B2.09 | Picture window: left edge clock 88, right edge clock 1112 — *not CPU-observable directly; reachable through the framebuffer oracle (`docs/adr/0013`) once the dot-resolution compositor lands, since locating a mid-line register change in the rendered picture is what maps a dot to a pixel column. See `docs/ppu.md` §Mid-scanline/HDMA-driven register timing.* |
| B2.10 | **`$213F` bit 4 = 50/60 Hz [CONFLICT — SNESdev PPU_registers says bit 3; bits 3-0 are the version field, so bit 4 is correct] — SETTLED 2026-07-20 by measurement: the battery ships an NTSC and a PAL image differing in one header byte, and **bit 4** is the bit that moves between them while bit 3 does not. fullsnes is right; the wiki is wrong.** |

Master clocks: NTSC 945/44 MHz = 21,477,272.7 Hz; PAL 21,281,370 Hz. VBlank budgets:
NTSC/224 = 37 lines / 48,988 clocks / 6,123 DMA bytes; NTSC/239 = 22 / 29,128 / 3,641;
PAL/224 = 87 / 115,188 / 14,398; PAL/239 = 72 / 95,328 / 11,916.

#### B3. DRAM refresh (3)

| # | Assertion |
|---|---|
| B3.01 | 40-clock CPU stall per scanline, leaving 1324 active |
| B3.02 | begins at cycle **538** on the first line of the first frame, thereafter at the multiple of 8 closest to 536 after the previous pause |
| B3.03 | observable via a tight H-counter-timed loop |

**Note the local conflict:** `docs/accuracy-ledger.md` classifies DRAM refresh **Out-of-scope
(empirically)** — measured across 500 frames × 3 ROMs, RustySNES's CPU-driven model already
reproduces the correct ≈357,368-clock NTSC frame, so an additive stall would be a regression.
ares independently notes its own refresh pattern is *"technically"* wrong but averages out
(`sfc/cpu/timing.cpp:23`, the "5-3, 5-3" logic-analyzer note). B3 tests should therefore probe
**position**, not aggregate frame length.

#### B4. Interrupt timing (14)

| # | Assertion |
|---|---|
| B4.01 | /NMI asserts at **H = 0.5** at vblank start |
| B4.02 | VBlank at V=$E1 (225), or V=$F0 (240) with overscan |
| B4.03 | RDNMI bit 7 set at V=225/240, HC=2 |
| B4.04 | RDNMI read-to-clear; a second read in the same vblank returns 0 |
| B4.05 | RDNMI auto-clears at vblank end |
| B4.06 | **Enabling `$4200.7` while RDNMI is already set fires an NMI immediately**, possibly outside vblank **[ERRATA]** |
| B4.07 | H-IRQ at H = HTIME + ~3.5 |
| B4.08 | V-IRQ at V=VTIME, H ≈ 2.5 |
| B4.09 | HV-IRQ at both |
| B4.10 | **No IRQ at dot 153 on the short scanline** |
| B4.11 | **No IRQ at dot 153 on the last scanline of any frame** |
| B4.12 | `$4211` read releases IRQ; so does disabling via `$4200` |
| B4.13 | HTIME range 0-339, VTIME 0-261/311 |
| B4.14 | Interrupt poll occurs **just before the final CPU cycle** → handler entry ≥6-12 master cycles after assertion — *the sub-cycle poll point is not CPU-observable (the finest readable clock is the H counter at 4 clocks/dot, and reading it costs more than the interval). Its **consequence** is measured instead: handler entry is timed with the CPU spinning on `NOP`s versus on `JSL`/`RTL`. The three references split on the sign — RustySNES +3 dots, snes9x +2, Mesen2 **−2** — so this is recorded as a golden vector, not scored.* |

`$4200 NMITIMEN` = `n-yx ---a`: n = VBlank NMI, yx = IRQ mode (00 none / 01 V / 10 H / 11 both),
a = auto-joypad enable. `$4212 HVBJOY` = `vh.. ...a`: v = VBlank, **h = HBlank (bit 6)
[CONFLICT — Mesen prose says bit 4; its own diagram and fullsnes say bit 6]**, a = auto-read busy.

#### B5. Multiply / divide (4)

| # | Assertion |
|---|---|
| B5.01 | `$4202`/`$4203` unsigned 8×8→16 into RDMPY `$4216/17` |
| B5.02 | `$4204/05`/`$4206` 16/8 → RDDIV `$4214/15` with remainder in RDMPY |
| B5.03 | **overlapping operation is [UNDEFINED] per Errata — report the observed value as a golden vector, never assert** |
| B5.04 | power-on `$4202`=$FF, `$4204/05`=$FFFF |

### 5.C Group C — S-PPU1 / S-PPU2 (~85 tests, the largest group)

#### C1. OAM port mechanics (9)

| # | Assertion |
|---|---|
| C1.01 | Writing `$2102` **or** `$2103` copies the whole 9-bit reload with bit0 forced 0. Anomie's example: set $104, write 4 bytes, write $1 to `$2103` → address is word **4**, not 6 |
| C1.02 | **Low-table write-twice latch**: addr 0, write $01, $02, read `$2138`, write $03 → OAM = `01 02 01 03` |
| C1.03 | High table (addr > $1FF) commits per-byte, no pairing |
| C1.04 | `$220-$3FF` mirror `$200-$21F` |
| C1.05 | `$2138` reads and `$2104` writes share one address counter |
| C1.06 | **Address reloads at H=10 on line 225 (or 240), only when force-blank is off** |
| C1.07 | Any **1→0 transition of `$2100` bit 7** also triggers the reload |
| C1.08 | Address destroyed during render — `$2138` mid-frame returns a position ≠ programmed |
| C1.09 | Priority rotation: `$2103` bit 7 → first sprite index = `(OAMAddr & $FE) >> 1` **[CONFLICT — fullsnes says register bits 6-1; unresolved]** |

#### C2. VRAM port mechanics (12 assertions in 10 rows)

| # | Assertion |
|---|---|
| C2.01 | Increments +1, +32, +128, **+128** (both `10` and `11` are 128) |
| C2.02 | VMAIN bit 7 selects the trigger register, symmetric for read and write |
| C2.03-05 | All three address-translation rotations produce the documented permutations |
| C2.06 | **Remap affects the bus, not the register**: `$2116/7=$0003` + remap 1 → access at word `$0018` while the register increments to `$0004` |
| C2.07 | Bit 15 unconnected — `$8000-$FFFF` alias `$0000-$7FFF` |
| C2.08 | **Writing `$2116/17` prefetches** — the first `$2139/$213A` read returns stale data |
| C2.09 | Read order: return latch → refill latch → increment |
| C2.10 | **Out-of-window write is dropped but the address STILL increments** |
| C2.11 | VRAM accessible only in vblank/force-blank — **H-Blank does not work** |
| C2.12 | Force-blank 1→0 mid-frame closes the window immediately |

VMAIN remap permutations:

```text
 8-bit : aaaaaaaa YYYxxxxx  -> aaaaaaaa xxxxxYYY    (4-colour,   1 word/plane)
 9-bit : aaaaaaa YYYxxxxxP  -> aaaaaaa xxxxxPYYY    (16-colour,  2 words/plane)
10-bit : aaaaaa YYYxxxxxPP  -> aaaaaa xxxxxxPPYYY   (256-colour, 4 words/plane)
```

#### C3. CGRAM and counters (10)

| # | Assertion |
|---|---|
| C3.01 | CGRAM inherits the OAM low-table two-write latch |
| C3.02 | `$2121` resets the flipflop |
| C3.03 | `$213B` 2nd read bit 7 = PPU2 open bus |
| C3.04 | **CGRAM access during active display hits the color currently being drawn** |
| C3.05 | `$2137` latches only when `$4201.7` is set and **returns open bus** |
| C3.06 | `$213C`/`$213D` 2nd read = counter bit 8 in bit 0, **bits 7-1 PPU2 open bus** |
| C3.07 | the two counter flipflops are independent |
| C3.08 | **`$213F` read resets BOTH flipflops and clears the latch flag** |
| C3.09 | `$213F.7` toggles at V=0, H=1 |
| C3.10 | Super Scope latches ≈ dot X+40, line Y+1 |

#### C4. Scroll registers (5)

| # | Assertion |
|---|---|
| C4.01 | `BGnHOFS = (Cur<<8) \| (Prev & ~7) \| ((Reg>>8) & 7)` |
| C4.02 | `BGnVOFS = (Cur<<8) \| Prev` |
| C4.03 | **one shared `Prev` latch across BG1-BG4, H and V** |
| C4.04 | Mode 7 latch separate |
| C4.05 | `$210D` writes both BG1HOFS and M7HOFS |

#### C5. Backgrounds and modes (15)

| # | Assertion |
|---|---|
| C5.01 | Mode 0 — BG1-4 all 2bpp, no hires, priority `S3 1H 2H S2 1L 2L S1 3H 4H S0 3L 4L` |
| C5.02 | Mode 1 — BG1/BG2 4bpp, BG3 2bpp, priority `S3 1H 2H S2 1L 2L S1 3H S0 3L` (BGMODE.3=1 → BG3H to front) |
| C5.03 | Mode 2 — BG1/BG2 4bpp, BG3 = OPT, priority `S3 1H S2 2H S1 1L S0 2L` |
| C5.04 | Mode 3 — BG1 8bpp, BG2 4bpp, priority `S3 1H S2 2H S1 1L S0 2L` |
| C5.05 | Mode 4 — BG1 8bpp, BG2 2bpp, BG3 = OPT, priority `S3 1H S2 2H S1 1L S0 2L` |
| C5.06 | Mode 5 — BG1 4bpp, BG2 2bpp, **hires**, priority `S3 1H S2 2H S1 1L S0 2L` |
| C5.07 | Mode 6 — BG1 4bpp, BG2 = OPT, **hires**, priority `S3 1H S2 S1 1L S0` |
| C5.08 | Mode 7 — BG1 8bpp, BG2 = EXTBG, priority `S3 S2 2H S1 1L S0 2L` |
| C5.09 | Mode 0 palette segregation (BG1 0-31, BG2 32-63, BG3 64-95, BG4 96-127) |
| C5.10 | tilemap entry `vhopppcc cccccccc` |
| C5.11 | 16x16 assembly uses +1/+16/+17 |
| C5.12 | BGnSC sizes place extra maps right/below/both |
| C5.13 | BGnSC/BGnNBA ignored in Mode 7 |
| C5.14 | 2/4/8bpp bitplane layouts |
| C5.15 | modes 5/6 use 16-px-wide tiles |

#### C6. Offset-per-tile (7)

| # | Assertion |
|---|---|
| C6.01 | Bit 13 → BG1, bit 14 → BG2 |
| C6.02 | Mode 4 bit 15 selects H vs V |
| C6.03 | H offsets keep the BG's low 3 HOFS bits |
| C6.04 | V offsets replace VOFS entirely |
| C6.05 | **the leftmost tile is NEVER affected [ERRATA]** — the first entry controls the *second* visible column |
| C6.06 | each entry affects a whole column |
| C6.07 | wraparound (the Super Famista 5 case) |

#### C7. Sprites (16)

| # | Assertion |
|---|---|
| C7.01 | 32-sprite range limit — highest OAM index drops |
| C7.02 | **34-sliver limit evaluated in REVERSE OAM order**, so the lowest-index (highest-priority) slivers drop first **[ERRATA]** |
| C7.03 | Slivers consumed left-to-right on screen even when H-flipped |
| C7.04 | **X = $100 (-256)**: fully offscreen yet consumes a range slot and all its slivers **[ERRATA]** |
| C7.05 | Range Over set at `V = OBJ.YLOC, H = OAM.INDEX*2` |
| C7.06 | Time Over set at `V = OBJ.YLOC+1, H = 0` |
| C7.07 | **Time Over false positive** — first sprite 16x16+ at X=0-255 with others at negative X **[ERRATA]** |
| C7.08 | Flags set regardless of `$212C` OBJ enable |
| C7.09 | **Flags clear at vblank end but NOT during forced blank** |
| C7.10 | OBJSEL sizes 6/7 undocumented (16x32/32x64, 16x32/32x32) |
| C7.11 | Tile addr = `((Base<<13) + (tile<<4) + (N ? ((Name+1)<<12) : 0)) & $7FFF` |
| C7.12 | **16x32 under OBJ interlace** renders as 16x16, bottom half ignored, top squished to 16x8 **[ERRATA]** |
| C7.13 | **V-flip on tall sizes flips each half independently [ERRATA]** |
| C7.14 | 64-px sprites wrap bottom→top in 224-line mode |
| C7.15 | Lower OAM index always on top |
| C7.16 | OAM write timing during render (the Uniracers case) |

#### C8. Color math and windows (12)

| # | Assertion |
|---|---|
| C8.01 | **Sprite color math only on palettes 4-7 [ERRATA]** |
| C8.02 | clamp to 0/31, no wrap |
| C8.03 | **half/div2 ignored when the subscreen is the fixed backdrop** |
| C8.04 | window bounds inclusive |
| C8.05 | `left > right` → empty |
| C8.06 | inverted + `left > right` → full |
| C8.07 | **both windows disabled → empty, not full** |
| C8.08 | OR/AND/XOR/XNOR window combination |
| C8.09 | CGWSEL force-black (bits 7-6) and prevent-math (bits 5-4) independent |
| C8.10 | subtract mode `$2131.7` |
| C8.11 | COLDATA per-channel select bits |
| C8.12 | color window + DMA B-bus (the Krusty's case) |

#### C9. Hi-res, interlace, overscan (8)

| # | Assertion |
|---|---|
| C9.01 | Pseudo-hires: even columns = subscreen (the **left** of each pair), odd = main |
| C9.02 | **Pseudo-hires color math copies the previous main-screen pixel's operation, using that pixel's PRE-math value as the operand** |
| C9.03 | Hi-res H scroll is coarse 2-px; V gets 1/480 when interlaced |
| C9.04 | Overscan `$2133.2` → 224 vs 239; vblank start moves $E1 → $F0 |
| C9.05 | **Mid-frame overscan toggle between $E0 and $F0 defers vblank events; setting too late leaves VRAM locked as if still rendering.** Repro: `STA $2118 / LDA $2133 / STA $2133 / STA $2118` → only one byte lands |
| C9.06 | Screen interlace `$2133.0` doubles effective height in modes 5/6 |
| C9.07 | Modes 5/6 color math restriction |
| C9.08 | Subscreen color math in hi-res (the Jurassic Park case) |

#### C10. Mosaic (5)

| # | Assertion |
|---|---|
| C10.01 | Applied after scrolling, before window/color math |
| C10.02 | anchored to screen top-left, not the scroll origin |
| C10.03 | **mid-frame `$2106` write re-anchors the start line to the current scanline** |
| C10.04 | 1x1 = 2x1 half-pixels in true hi-res |
| C10.05 | Mode 7 BG2 uses bits A and B separately for V and H |

#### C11. Mode 7 (12)

| # | Assertion |
|---|---|
| C11.01 | `[Tx,Ty] = M * [Sx+HOFS-X, Sy+VOFS-Y] + [X,Y]` |
| C11.02 | **13-bit sign handling**: `ORG.X = (M7HOFS - M7X) AND NOT $1C00; if < 0 then OR $1C00` |
| C11.03 | **Each `M7x * ORG` product has its low 6 bits masked (`AND NOT $3F`) before accumulation** |
| C11.04 | Screen-over: bit7=0 clamps to 0..1023; bits 7+6 set → out-of-range uses low 3 bits of char 0 |
| C11.05 | Tilemap in VRAM low bytes, tiles in high bytes, first 16 KB, fixed |
| C11.06 | MPY = signed16(`$211B`) × signed8(`$211C`) |
| C11.07 | **MPY latch corruption** — an interrupt or HDMA writing BG1 scroll or an M7 matrix register **between the two M7A writes** **[ERRATA]** |
| C11.08 | MPY during active display holds intermediate per-pixel rotation results |
| C11.09 | EXTBG splits BG1 by the pixel high bit into two priority layers |
| C11.10 | **Direct color unavailable on EXTBG BG2**, always available on Mode 7 BG1 |
| C11.11 | Mode 7 window logic (the Atlas / MechWarrior case) |
| C11.12 | Scroll latch timing (the NHL '94 case) |

#### C12. Direct color (3)

| # | Assertion |
|---|---|
| C12.01 | `RRRr0 GGGg0 BBb00` — pixel bits supply the high bits, tilemap attribute bits one extra per channel (blue gets 2+1) |
| C12.02 | **pure black is unreachable** (pixel value 0 is always transparent) |
| C12.03 | available on Mode 3/4 BG1 and Mode 7 BG1 only |

#### C13. INIDISP and open bus (10)

| # | Assertion |
|---|---|
| C13.01 | **INIDISP early-read: object tile corruption** — write outside vblank with prior bus bit 7 set, **3-chip only** · *blocked twice: sub-scanline (the compositor paints a whole line from one register snapshot) and chip-revision-dependent, so a golden would commit to one revision as though it were the behaviour. See `docs/accuracysnes-plan.md`. Applies to `C13.01`-`C13.06`.* |
| C13.02 | **Display flash** — force-blank on + prior bus bit 7 clear → display on for one dot |
| C13.03 | **Brightness glitch** — one-dot brightness step |
| C13.04 | `STA $8F2100` vs `STA $0F2100` produce different artifacts |
| C13.05 | **Brightness ramp ~72+ pixels on 1CHIP** — write `$8F`, not `$80` |
| C13.06 | **SETINI has an analogous early-read bug** |
| C13.07 | PPU1 open bus refreshed by `$2134-36`, `$2138-3A`, `$213E` |
| C13.08 | PPU2 open bus refreshed by `$213B-3D`, `$213F` |
| C13.09 | **PPU1 and PPU2 open bus are SEPARATE latches** |
| C13.10 | `$213E` bit 4 = PPU1 open bus; `$213F` bit 5 = PPU2 open bus |

#### C14. Version detection (3)

| # | Assertion |
|---|---|
| C14.01 | `$213E` bits 3-0 = PPU1 version (only 1 known) |
| C14.02 | `$213F` bits 3-0 = PPU2 version (1/2/3) — **gates C13.01** |
| C14.03 | `$213E` bit 5 = master/slave |

### 5.D Group D — DMA / HDMA (~35 tests)

#### D1. General-purpose DMA (15)

`$4300` DMAP = `da-ttttt`: bit7 direction, bit6 HDMA indirect, bit4 A-bus fixed, bit3 A-bus
decrement, bits 2-0 mode.

| # | Assertion |
|---|---|
| D1.01 | Transfer modes 0-7, one test each |
| D1.02 | **8 master cycles per byte, region-independent** |
| D1.03 | 8-cycle startup overhead plus channel-start alignment |
| D1.04 | lower channel number first |
| D1.05 | `$4302-04` A-bus, `$4305/06` size (0 = $10000), `$4307` indirect bank |
| D1.06 | size decrements to 0 during transfer |
| D1.07 | A-bus fixed/increment/decrement |
| D1.08 | **invalid A-bus addresses [ERRATA]**: `$21xx`, `$4000-$41FF`, `$4200-$421F`, `$4300-$437F` |
| D1.09 | **WRAM→WRAM via `$2180` prohibited** |
| D1.10 | `$43x0-$43xF` mirroring, with `$43xB`/`$43xF` an undocumented readable scratch latch (both ares and bsnes model it and serialize it into save states) |
| D1.11 | DMA power-on state (Heian Fuuunden) |
| D1.12 | CPU timing before DMA start (MMPR) |
| D1.13 | **DMA reads update open bus; DMA writes never do** |
| D1.14 | **`$2180` asymmetry, B→A**: `$2180`→WRAM *"does cause a write to occur (but no read), but the value written is invalid"* (+4 clocks) |
| D1.15 | **`$2180` asymmetry, A→B**: WRAM→`$2180` *"does not cause a write to occur"* (+8 clocks, no access) |

The `$2180` asymmetry is Mesen2's `SnesDmaController.cpp:53,62`.

#### D2. HDMA (17 assertions in 14 rows)

| # | Assertion |
|---|---|
| D2.01 | Init at V=0, H≈6 |
| D2.02 | **Per-line transfer at dot 278**, ~18 cycles overhead + 8-24 per channel |
| D2.03 | Line-count byte: bit7 repeat, bits 6-0 count, `$00` terminates |
| D2.04 | Repeat transfers every line; non-repeat once then counts down |
| D2.05 | Indirect mode via `$4306/07` |
| D2.06 | `$4308/09` counter, `$430A` NLTR |
| D2.07 | **HDMA preempts GP-DMA, which pauses and resumes** |
| D2.08 | `$420C` set mid-frame → channel starts next line |
| D2.09 | **Enabling HDMA outside vblank → erroneous writes from uninitialized A2An/NLTRn [ERRATA]** |
| D2.10 | **Whole-frame HDMA failure if a DMA ends at scanline 0 start with previous read value 0 [ERRATA]** |
| D2.11-14 | Fixed / decrement / direction+NMI-enable / transfer-flag-state (Batman Forever, Kid Kleets, Pocky & Rocky, Aladdin+SGnG) |
| D2.15 | DMA suspension during HDMA (Dekitate High School) |
| D2.16 | **HDMA-driven register writes take effect the FOLLOWING line** (the Air Strike Patrol BG3 case) |
| D2.17 | **Open bus via HDMA latch** (Speedy Gonzales 6-1) |

Additional Mesen2-sourced detail: **HDMA last-channel oddity** — if `$43xA` is 0 and this is the
last active HDMA channel for the scanline, only one byte is loaded for Address with `$00` as the
low byte, so Address ends up incremented one less than expected and **one fewer CPU cycle is
used** (`SnesDmaController.cpp:296-298`). Indirect address reload costs 16 master cycles. A
terminated HDMA channel does **not** clear its `$420C` bit, so it can auto-restart next frame.
`DoTransfer` must not be reset per-frame beyond the documented points — *"not resetting this causes
graphical glitches in some games (Aladdin, Super Ghouls and Ghosts)."*

**Unresolved in the lineage:** the HDMA indirect high-byte on a partial write carries an identical
`//todo: should 0x00 be indirectAddress >> 8 ?` in **both** ares (`sfc/cpu/dma.cpp:164`) and bsnes
(`:163`).

#### D3. Revision-gated (auto-skip by `$4210.3-0`) (2)

| # | Assertion |
|---|---|
| D3.01 | **5A22 v1**: crash if a DMA finishes just before HDMA |
| D3.02 | **5A22 v2**: a recent HDMA to INIDISP prevents the next DMA from completing (workaround `BBADn=$FF` with transfer pattern 1) |

`docs/accuracy-ledger.md` classifies these **Out-of-scope** for RustySNES — chip-revision defects
compliant commercial ROMs avoid, not reproduced by mainstream reference emulators. AccuracySNES
should therefore report them as **variants**, not failures.

### 5.E Group E — SPC700 + S-DSP (~75 tests)

#### E1. Arithmetic quirks (15)

| # | Assertion |
|---|---|
| E1.01 | **`MUL YA` flags from Y only** — `Y=$10, A=$10` → YA=$0100, Z **clear** despite A==0 **[ERRATA]** |
| E1.02 | **`DIV YA,X` normal branch** (`Y < (X<<1)`): A = YA/X, Y = YA%X |
| E1.03 | **`DIV` overflow branch**: `A = 255 - (YA-(X<<9))/(256-X)`, `Y = X + (YA-(X<<9))%(256-X)` |
| E1.04 | **`DIV` H flag = nibble compare** `(Y&15) >= (X&15)` — not a real half-carry |
| E1.05 | `DIV` V = quotient bit 8 |
| E1.06 | `DIV` N/Z from the quotient only **[ERRATA]** |
| E1.07 | `DIV` valid only for quotient ≤ 511 **[ERRATA]** |
| E1.08 | **`DAA`**: `if (C \|\| A>$99) {A+=$60; C=1;} if (H \|\| (A&15)>9) A+=6;` — the second test uses the **post-adjustment** value |
| E1.09 | **`DAS`**: the mirror form |
| E1.10 | **`TSET1`/`TCLR1` are equality tests** — N/Z reflect `CMP A,[addr]` **before** modification **[ERRATA]** |
| E1.11 | **They read the target TWICE** — a read-sensitive `$FD-$FF` target gets cleared twice |
| E1.12 | **`CLRV` clears H as well as V** |
| E1.13 | `ADDW`/`SUBW` H = bit11→12 carry, Z = true 16-bit zero |
| E1.14 | `XCN` is 5 cycles |
| E1.15 | `MOVW YA,aa` sets N/Z on the 16-bit value |

#### E2. Memory-access side effects (10)

| # | Assertion |
|---|---|
| E2.01 | **Store opcodes issue a dummy read** — `MOV $FD,A` **clears Timer 0's counter** |
| E2.02 | exemptions `$FA` (`MOV aa,bb`) and `$AF` (`MOV (X)+,A`) |
| E2.03 | `MOVW aa,YA` dummy-reads the **LSB only** |
| E2.04 | `DBNZ aa` is an RMW |
| E2.05 | DP index wraps within the page |
| E2.06 | `PSW.P` selects `$00xx`/`$01xx` everywhere including bit ops and `[aa]+Y` pointer fetches |
| E2.07 | **calls push the exact return address, not retaddr-1** |
| E2.08 | `TCALL n` → `[$FFDE - n*2]` (n=15 reads inside the IPL ROM when mapped) |
| E2.09 | `BRK` shares the `TCALL 0` vector |
| E2.10 | full 256-opcode cycle sweep |

#### E3. I/O registers (14)

Map: `$F0` TEST (reset $0A) · `$F1` CONTROL · `$F2` DSPADDR · `$F3` DSPDATA · `$F4-$F7` CPUIO0-3 ·
`$F8/$F9` AUXIO4/5 · `$FA-$FC` TnDIV · `$FD-$FF` TnOUT.

| # | Assertion |
|---|---|
| E3.01 | **Reading `$FD`/`$FE`/`$FF` returns 4 bits and zeroes the counter** |
| E3.02 | `$F1` bits 0-2 0→1 reset that timer's stage2 **and** stage3 |
| E3.03 | `$F1` bits 4/5 clear the CPUIO input latches, non-persistent |
| E3.04 | `$F1` bit 7 unmaps the IPL ROM; **writes to `$FFC0+` always hit RAM regardless** |
| E3.05 | `TnDIV = $00` means divide-by-**256** |
| E3.06 | T0/T1 8 kHz (128 cycles), T2 64 kHz (16 cycles) |
| E3.07 | **timers advance on DSP cycles T1 and T17** |
| E3.08 | TEST bits 0/3 halt timers |
| E3.09 | **TEST wait-states 2 and 3 cost the CPU 10/20 clocks but the timers only 8/16** |
| E3.10 | TEST bit 1 = RAM write enable; clearing it blocks SPC700 *and* S-DSP writes |
| E3.11 | `$F2` bit 7 discards writes through `$F3` |
| E3.12 | **CPUIO bus conflict: the S-CPU reads the OR of old and new** |
| E3.13 | writes to `$00F0-$00FF` also land in the RAM shadow |
| E3.14 | `$F8`/`$F9` behave as plain RAM |

**The 10/20-vs-8/16 divergence is the SMP wait-state glitch**, documented identically in ares and
bsnes (`sfc/smp/timing.cpp:1-8`): *"due to an unknown hardware issue, clock dividers of 8 and 16
are glitchy; the SMP ends up consuming 10 and 20 clocks per opcode cycle instead… sometimes the SMP
will run far slower than expected, other times… deadlock until the system is reset. **The timers
are not affected.**"* Hence two tables: `cycleWaitStates[4] = {2,4,10,20}` vs
`timerWaitStates[4] = {2,4,8,16}`.

#### E4. IPL boot and handshake (11)

| # | Assertion |
|---|---|
| E4.01 | IPL ROM byte-identical to the canonical 64-byte listing, reset vector `$FFC0` |
| E4.02 | handoff state `A=0, X=0, Y=0, SP=$EF, PSW=$02` |
| E4.03 | zerofills `$0000-$00EF` |
| E4.04 | ready = `Word[$2140] == $BBAA` |
| E4.05 | first kick `$CC`; subsequent `((index+2) & $FF) \| 1`, strictly > last index+1 and non-zero |
| E4.06 | `$2141 == 0` execute, non-zero transfer |
| E4.07 | **the final data-byte ack window is only a few cycles wide** |
| E4.08 | transfer address `$00F2` lets the IPL loop poke DSP registers as (reg#, value) pairs |
| E4.09 | **16-bit writes to `$2140/41` can corrupt `$2143` [ERRATA] — write 8-bit only** |
| E4.10 | simultaneous CPU/SPC access to `$2140-43` produces incorrect data **[ERRATA]** |
| E4.11 | APU RAM power-on pattern repeating **32×$00 then 32×$FF** (chip-dependent — informational) |

#### E5. BRR decoding (13)

| # | Assertion |
|---|---|
| E5.01 | Header `ssssffle` |
| E5.02 | high nibble first, signed -8..+7 |
| E5.03 | `(nibble << shift) >> 1` arithmetic |
| E5.04 | **invalid shift 13/14/15 collapse to `$0000` / `$F800`** |
| E5.05 | the four filters as exact integer formulas (below) |
| E5.06 | **15-bit wrap: clamp to 16 bits, then `+4000h..+7FFFh` → `-4000h..-1` and `-8000h..-4001h` → `0..3FFFh`, sign lost** |
| E5.07 | End+Mute forces Release with envelope 0 immediately |
| E5.08 | code 2 behaves as code 0 |
| E5.09 | **ENDX sets at the START of decoding the end block** |
| E5.10 | **BRR decoding continues even for released voices** |
| E5.11 | DIR entry = `DIR*$100 + SRCN*4` |
| E5.12 | **mid-playback SRCN change: not yet looped → start address; already looped → loop address** |
| E5.13 | three consecutive max-negative samples cause an overflow pop **[ERRATA]** |

```text
Filter 0: new = sample
Filter 1: new = sample + old*1 + ((-old*1)  SAR 4)
Filter 2: new = sample + old*2 + ((-old*3)  SAR 5) - older + ((older*1) SAR 4)
Filter 3: new = sample + old*2 + ((-old*13) SAR 6) - older + ((older*3) SAR 4)
```

> "Games depend on these exact formulas; simplifying will break sound effects."

#### E6. Pitch and gaussian interpolation (11)

| # | Assertion |
|---|---|
| E6.01 | Counter bits 15-12 select the sample, 11-4 the gaussian index |
| E6.02 | `$1000` = 1:1 (32 kHz), `$2000` = +1 octave |
| E6.03 | PMON factor `(OUTX[x-1] SAR 4) + $400`, then `(Step*Factor) SAR 10` |
| E6.04 | **PMON never affects voice 0** |
| E6.05 | **PMON does not modulate noise** |
| E6.06 | counter clamped to `$7FFF` |
| E6.07 | all 512 gaussian table entries byte-exact |
| E6.08 | **the `$801` overflow bug**: `nibbles=77778888, shift=12, filter=0` → `+3FF8h` instead of `-4000h` |
| E6.09 | **partial overflow rules: 1st addition can't overflow, 2nd WRAPS (i=$00-$1F), 3rd SATURATES (i=$20-$FF)** |
| E6.10 | gaussian bypassed for noise |
| E6.11 | golden waveform vectors `79797979`, `77997799`, `77779999`, `7777CC44` |

#### E7. Envelopes (18)

| # | Assertion |
|---|---|
| E7.01 | 32-entry rate table `{0,2048,1536,…,2,1}`, rate 0 never fires |
| E7.02 | **counter offset table `{0,0,1040,536,…}`** — two voices at different rates reveal the implied phase |
| E7.03 | attack index `a*2+1`, step +32 |
| E7.04 | **`a==$F` → every sample, step +1024** |
| E7.05 | decay index `d*2+16`, step `E -= 1; E -= E>>8` |
| E7.06 | sustain rate index = `r` verbatim |
| E7.07 | sustain boundary `$100*(l+1)`, compare `(E>>8) == SL` |
| E7.08 | release forced every-sample, step -8, ~0.008 s |
| E7.09 | **release rate is fixed — custom release requires GAIN [ERRATA]** |
| E7.10 | direct gain `E = G<<4` |
| E7.11 | four custom-gain modes (linear dec -32, exp dec, linear inc +32, bent inc +32 below $600 else +8) |
| E7.12 | **GAIN-mode sustain-boundary bug: the Decay→Sustain compare reads the boundary from `VxGAIN` bits 7-5, not `VxADSR2`** |
| E7.13 | **bent-increase uses the CLIPPED previous envelope, so an underflowed negative reads as ≥ $600** |
| E7.14 | linear-decrease underflow clamps, never wraps |
| E7.15 | `VxENVX = E>>4`, bit 7 always 0 |
| E7.16 | `VxOUTX` is post-envelope, pre-volume, high byte |
| E7.17 | **ENVX/OUTX writes 1-2 clocks before the DSP writeback are lost** |
| E7.18 | **ADSR/GAIN mode-change race — write ADSR2/GAIN before ADSR1 [ERRATA]** |

ares corroborates the GAIN mode-7 quirk directly: `i32 _envelope; //used by GAIN mode 7, very
obscure quirk` (`sfc/dsp/dsp.hpp:128`). RustySNES already fixed the corresponding `spc_dsp6`
"Envelope/gain $E0 threshold" case.

#### E8. Key on/off (11)

| # | Assertion |
|---|---|
| E8.01 | **KON/KOFF polled every SECOND sample (16 kHz) [ERRATA]** |
| E8.02 | **5-sample key-on delay** |
| E8.03 | KON restarts even if playing, zeroing the envelope, and clears ENDX even when suppressed |
| E8.04 | KON is write-triggered/non-persistent; **KOFF and FLG.7 exert influence continuously** |
| E8.05 | collapse case `KOFF=$FF / KON=$01 / KOFF=$00` → voice 1 usually keys on |
| E8.06 | collapse case `KON=$01 / KON=$02` → usually only voice 2, and if both, **voice 1 is 2 samples ahead, proving the 16 kHz rate** |
| E8.07 | collapse case `KOFF=$FF / KOFF=$00` → voices keep playing |
| E8.08 | **63-cycle KOFF window** (64-127 probabilistic) |
| E8.09 | **FLG.7 polled EVERY sample, per-voice** |
| E8.10 | KOFF+KON together silences faster than KOFF alone |
| E8.11 | DSP KOF init (Chester Cheetah / King of Dragons) |

#### E9. Noise, echo, mixer (20)

| # | Assertion |
|---|---|
| E9.01 | LFSR taps bit0 XOR bit1 → bit14, initial `$4000` |
| E9.02 | **noise output is highpass-filtered [ERRATA]** |
| E9.03 | VxPITCH does not affect noise frequency |
| E9.04 | **noise voices still decode BRR, so End+Mute kills the noise** |
| E9.05 | echo 4 bytes/entry, low 7 bits in bits 1-7 |
| E9.06 | **`EDL=0` gives a 4-byte (1-sample) buffer and continuously overwrites 4 bytes at the buffer start [ERRATA]** |
| E9.07 | **EDL change latency up to 0.25 s** |
| E9.08 | **ESA change delayed 1-2 samples** |
| E9.09 | **echo buffer wraps at a 16-bit boundary, corrupting page zero and the `$FFC0+` IPL shadow [ERRATA]** |
| E9.10 | **FLG bit 5 disables echo WRITES but not READS — the buffer becomes a static forever-loop** |
| E9.11 | **only the final FIR7 addition saturates; the first seven WRAP [ERRATA]** |
| E9.12 | echo write masked `& $FFFE` |
| E9.13 | L/R FIR independent, no crosstalk |
| E9.14 | **MVOL/EVOL/EFB/FIRx = `$80` overflows; VxVOL = `$80` does not** |
| E9.15 | per-voice mix saturates after each addition |
| E9.16 | **final output XORed with `$FFFF`** by the post-amp (phase inversion — matters for any bit-exact audio hash) |
| E9.17 | FLG.MUTE zeroes output but echo RAM writes continue |
| E9.18 | FLG.RESET behaves as `KOFF=$FF` + envelopes 0, but echo keeps sounding |
| E9.19 | **ENDX: any write clears ALL bits regardless of value** |
| E9.20 | the official Nintendo FIR preset `FF 08 17 24 24 17 08 FF` is **bugged** (positive taps exceed +$7F) yet games rely on it |

ares and bsnes both flag global muting as unresolved: `//todo: global muting isn't this simple`
(`sfc/dsp/echo.cpp:96`).

#### E10. Cycle-level pipeline (6)

| # | Assertion |
|---|---|
| E10.01 | 32 SPC cycles per output sample |
| E10.02 | the T0-T31 register-access schedule (ENDX.n at T1/T4/T7…, FLG.5 at T29/T30, KON at T31) |
| E10.03 | ENDX/OUTX/ENVX written on three separate cycles |
| E10.04 | SPC and DSP share `/RESET` and the 2.048 MHz clock |
| E10.05 | **post-reset FLG behaves as `$E0` regardless of what reads back**; **[CONFLICT — nocash says ENDX=$FF, Anomie says 0 → golden vector]** |
| E10.06 | SPC cycle timing (the ActRaiser 2 / Tales of Phantasia case) |

Clocks: S-DSP 24.576 MHz, DSP internal 3.072 MHz, SPC700 1.024 MHz, output 32 kHz nominal.
**Real-hardware DSP rate varies 31,965-32,349 Hz and rises ~8 Hz cold→warm — audio timing must
never be a determinism input** (consistent with `docs/adr/0004`).

### 5.F Group F — Input (~22 tests)

Serial strings: joypad `byetUDLRaxlr0000` · mouse `00000000rlss0001 YyyyyyyyXxxxxxxx` ·
Super Scope `fctp00on` · Justifier `0000000000001110 01010101TtSsl000 1111111111111111`.

| # | Assertion |
|---|---|
| F1.01 | Manual read order: B,Y,Select,Start,Up,Down,Left,Right,A,X,L,R,0,0,0,0 |
| F1.02 | Reads 17-32 return **1** on official pads |
| F1.03 | **The latch is shared** — writing `$4016.0` latches both ports |
| F1.04 | `$4016`/`$4017` bits 7-2 return **CPU open bus** |
| F1.05 | Auto-read signature nibble `0000` for a standard pad |
| F1.06 | Bit 15 of `$4219` = first bit clocked = B |
| F1.07 | With `$4200.0 = 0`, `$4218-$421F` do not update |
| F1.08 | **Auto-read starts between dot 32.5 and 95.5** of the first vblank line (74.5 on the first frame; thereafter multiples of 256 cycles) **[ERRATA]** |
| F1.09 | **Duration exactly 4224 master cycles** (≈3.097 scanlines) |
| F1.10 | **The race**: reading `$4212` at NMI entry can see busy=0 *before* auto-read starts → a naive wait-loop returns **stale data** **[ERRATA]** |
| F1.11 | **`$4016.0` must stay 0 during auto-read** or `$4218-$421F` corrupt |
| F1.12 | Results valid by V=$E3 |
| F1.13 | **Results may change during lag frames [ERRATA]** |
| F1.14 | `$4201` power-on `$FF`; `$4213` open-collector wired-AND |
| F1.15 | Multitap detect: `$4016.0=1` → eight `$4017` D1 reads give $FF; `=0` → not $FF |
| F1.16 | **Multitap port-pair select is `$4201` bit 7**, not `$4016` bit 1 |
| F1.17 | Multitap pads supply a **17th bit** = controller-connected |
| F1.18 | Mouse: signature `0001`; **sign-magnitude, not two's complement**; **zero magnitude repeats the previous sign** |
| F1.19 | Mouse timing: ≥170 master cycles between bit reads; **≥336 between the byte-2 and byte-3 reads** |
| F1.20 | Mouse sensitivity cycling fails during an active auto-read |
| F1.21 | Super Scope: port 2 only; latches OPHCT/OPVCT after the sensor sees the beam **6 times** |
| F1.22 | NTT Data Keypad bits 12-15 = `0100` |

Corroborating quirks from the reference emulators: the SNES Mouse has a *"hardware quirk that the
real mouse does"* (Mesen2 `SnesMouse.h:88`); the D-pad physically cannot register up+down or
left+right (ares/bsnes); Super Scope works only in port 2 because iobit there drives the PPU H/V
latch, and **no commercial game** uses the port-1 manual-polling alternative.

### 5.G Group G — Power-on / reset / cartridge (~18 tests)

| # | Assertion |
|---|---|
| G1.01 | Documented power-on: `$4200=$00, $4201=$FF, $4202=$FF, $4204/05=$FFFF, $4207/08=$1FF, $4209/0A=$1FF, $420D=$00` |
| G1.02 | `$4210`/`$4211` bit 7 clear on power-on and reset |
| G1.03 | **Everything else indeterminate** — APUIOn, WMDATA, WMADD*, JOYSER, HDMAEN, MDMAEN, JOY1-4. Report, never assert **[UNDEFINED]** |
| G1.04 | CPU enters emulation mode, vectors through `$00FFFC` |
| G1.05 | **No boot ROM; most PPU registers start unknown** |
| G1.06 | **Cartridge /RESET does not reset the PPU [ERRATA]** — PPU state survives soft reset |
| G1.07 | **No canonical WRAM fill exists — model-dependent [UNDEFINED]**. Golden vector only |
| G1.08 | Write-only MMIO reads return **CPU open bus**, not $00/$FF |
| G1.09 | `$4210` bits 3-0 = 5A22 version — gates D3 |
| G1.10 | Header invariant `checksum XOR complement == $FFFF` |
| G1.11 | Checksum algorithm: sum all bytes with `$FFDC`=$FFFF and `$FFDE`=$0000; non-power-of-2 uses largest-prefix + mirrored-remainder |
| G1.12 | Header at `$007FC0` (LoROM), `$00FFC0` (HiROM), `$40FFC0` (ExHiROM) |
| G1.13 | **`$FFD5` bit 4 = FastROM [CONFLICT — SNESdev prose says bit 7; its own `001smmmm` diagram and fullsnes say bit 4]** |
| G1.14 | LoROM decode `((bank & $7F) << 15) \| (addr & $7FFF)`; cart A15 unconnected |
| G1.15 | HiROM decode `((bank & $3F) << 16) \| addr`; banks $40-$7D mirror $C0-$FD |
| G1.16 | ExHiROM: A23 inverted into cart A22 |
| G1.17 | SRAM mapping (Thracia 776 / Ys III) |
| G1.18 | Copier header when `filesize % 1024 == 512` |

Note bsnes' `Hacks/Entropy` setting (default `"Low"`) controls WRAM randomization, and it
special-cases **Dirt Racer (Europe)** by forcing WRAM to `0xFF` — *"the game itself is broken and
will fail to run sometimes on real hardware"* (`sfc/cpu/cpu.cpp:97-103`). This is direct evidence
that G1.07 must be a golden vector, not an assertion.

---

## Part VI — Reference-emulator quirk corpus

88 quirks were harvested from `ref-proj/{ares,bsnes,Mesen2}` source comments. The full list is in
agent 4's and agent 5's raw notes; this section records the ones that are **contested or admitted
unexplained**, because those are precisely the tests that must be tiered **Contested** and excluded
from AccuracySNES's pass rate.

### Behaviors the reference emulators admit they do not understand

| Behavior | Admission |
|---|---|
| IRQ fires 2 ticks / 8 master clocks after the H/V match | Mesen2 `InternalRegisters.h`: *"What's causing the 2 ticks/8 master clock delay…?"* |
| H=0 IRQs appear delayed by an extra tick | Mesen2, unexplained |
| NMI signal has a 1-tick/4-clock delay | Mesen2: *"why does the CPU behave like it was set on H=6 instead of H=2?"* |
| IRQ at V=261 H=339 cannot trigger | Mesen2 speculates V=0/H=339 fires on both scanline 0 and 1 — marked **(unverified)** |
| VRAM reads during rendering | Mesen2: *"Unknown: does it read from the address the ppu is currently reading from (like oam/cgram)?"* |
| OAM address reset timing at vblank | Mesen2: *"TODO, the timing of this may be slightly off? should happen at H=10 based on anomie's docs"* |
| `$4212` set/clear timing | Mesen2 `//TODO TIMING` |
| HDMA indirect high-byte on partial write | identical `//todo` in **both** ares and bsnes |
| `$4203`/`$4206` write-side `irqPoll()` | bsnes marks `//unverified` ×4 |
| Reading `$4016/$4017/$4218-$421F` during auto-joypad polling | bsnes ×4: *"it is not known what happens"* |
| SA-1 `$230x` read returns `0xff` | `//unverified` in both ares and bsnes; *"reset timing is unknown"* |
| Auto-joypad latch cycle | ares `//TODO: this may need to happen one cycle earlier` |
| ST018/ARMv3 timings | ares & bsnes: *"completely unverified; due to the ST018 chip design (on-die ROM), testing is nearly impossible"* |
| Global DSP muting | ares & bsnes: *"todo: global muting isn't this simple"* |

#### Behaviors the emulators model as deliberate approximations

- **OAM read/write during rendering** — Mesen2 approximates via `_oamRenderAddress`, *"not cycle
  accurate — needed for Uniracers"*; bsnes hard-codes the constant: *"0x0218: Uniracers (2-player
  mode) hack; requires cycle timing for latch.oamAddress to be correct."* **This is the canonical
  contested SNES PPU behavior.**
- **CGRAM access during rendering** — bsnes gives a precise, testable window:
  `hcounter() >= 88 && < 1096` and `vcounter() > 0 && < vdisp()`.
- **bsnes' entire `Hacks/PPU` tree** is a catalogue of what it considers approximable: `fast`,
  `deinterlace`, `noSpriteLimit`, `noVRAMBlocking`, **`renderCycle = 512`**, and the Mode 7
  scale/perspective/supersample/mosaic knobs. `renderCycle` is the direct analogue of RustySNES's
  own `RENDER_DOT`.
- **bsnes' `Hacks/CPU/FastMath`** gates exact ALU timing — mid-multiply reads are a known-hard,
  optional path. Mesen2 admits its ALU peek is *"not completely accurate."*
- **`Hacks/Coprocessor/DelayedSync` defaults to `true`** in bsnes — coprocessors are *not* synced
  per-access by default.

#### Three-emulator-agreed quirks (cheap, unambiguous, **Corroborated** tier)

- **Interlace + `baseSize >= 6` forces sprite height 16** — flagged `//hardware quirk` in three
  separate files across ares and bsnes.
- **Rectangular sprites break vertical mirroring** — top and bottom halves mirror separately and do
  not swap positions.
- **Sprites at X = -256 count for Time/Range but are never drawn.**
- **Sprite Y wraps after 256 scanlines.**
- **Access-speed map** — Mesen2 builds a 0x800-entry `_masterClockTable`; ares encodes the identical
  map arithmetically in `CPU::wait()`. Independent agreement.
- **`$43xB`/`$43xF` undocumented scratch register** — both model it as a plain readable latch and
  both serialize it into save states.

#### Structural notes worth mirroring

- **ares** splits each access as `step(clockCount - 4)` → `bus.read()` → `step(4)`, i.e. **the bus
  access lands in the final 4 clocks of the cycle**. Directly testable, and it explains the ALU
  read-vs-write one-cycle offset.
- **Mesen2** runs a small event queue (`HdmaInit`, `DramRefresh`, `HdmaStart`, `EndOfScanline`)
  with a precomputed speed table and specialized `IncMasterClock4/6/8/40/Startup()` entry points.
- **bsnes constants worth pinning:** `dramRefreshPosition` 530/538 **by CPU revision**,
  `hdmaSetupPosition` `12 ± 8 ∓ dmaCounter()`, `hdmaPosition` 1104, PPU `renderCycle` 512.

---

## Part VII — Existing SNES test ROMs and the gap

| Suite | License | Covers | Granularity |
|---|---|---|---|
| SingleStepTests/**65816** | **NONE** | per-opcode, all modes, 8/16-bit, native+emulation, cycle-by-cycle bus traces | JSON, host-side |
| SingleStepTests/**spc700** | MIT | same for SPC700 | JSON, host-side |
| gilyon/snes-tests | MIT | on-cart CPU + SPC, 1107 assertions | pass/fail `.sfc` + golden `.txt` |
| undisbeliever/snes-test-roms | Zlib | PPU/DMA/HDMA + `hardware-glitch-tests/` (INIDISP/SETINI early-read) | visual / golden |
| blargg `spc_*` | unstated | `spc_smp`, `spc_timer`, `spc_mem_access_times`, `spc_dsp6` | literal PASS/FAIL |
| 240p Test Suite (SNES) | GPL-2.0+ | video / overscan patterns | visual |
| PeterLemon/SNES (Krom) | **NONE** | broad CPU/PPU/SPC/DSP/GSU + reference PNGs | screenshot diff |
| Nintendo Aging / Test / Controller Program | not redistributable | RAM/DRAM/VRAM, DMA, mul/div, timers, EXT Latch, HV Timer, VH Flag | official diagnostic |
| ctrltest / mset (rainwarrior) | — | controller / mouse | visual |
| gradient-test (NovaSquirrel) | — | CGWSEL | visual |
| Two Ship / Elasticity / PPU bus activity | — | Mode 5 + interlace, Mode 3, modes 0-6 | visual |

Historical TASVideos pass/fail data: ZSNES and early snes9x fail blargg's ADC/SBC and OAM tests,
the Nintendo Test Program's electronics and color tests, and the Aging Test's "EXT Latch",
"HV Timer", and "VH Flag" components; higan v094, lsnes, and BizHawk pass. snes9x v1.43 uniquely
fails the Cx4 LDMAC test.

### Why a new ROM is warranted

**1. No canonical battery exists.** This repo's own `docs/testing-strategy.md` states it plainly.
Accuracy today is a *composed* oracle across five heterogeneous corpora.

**2. Licensing is the binding constraint, not coverage.** The single best CPU oracle ships **no
license**; Krom ships none; blargg's are unstated; 240p is GPL-2.0+. A permissively-licensed,
self-contained battery *is itself the deliverable* — it lets any emulator project gate CI on a
vendored artifact rather than a gitignored external tier.

**3. No machine-readable result surface** exists in any on-cart suite, including AccuracyCoin.

### Behaviors with no public test-ROM coverage found

`r`/`rl` PC-relative wrap (upstream literally says "XXX: untested") · emulation-mode `d`/`d,X`
**word-read** wrap (documented only "theoretically") · MVN/MVP mid-instruction interrupt behavior ·
MVN/MVP 8-bit index in native mode · `$4203`/`$4206` overlapping race · 5A22 v1/v2 DMA-HDMA crash
quirks · HDMA scanline-0 whole-frame failure · DRAM refresh position · the two no-IRQ-at-dot-153
exceptions · GAIN-mode sustain-boundary bug · bent-increase clipped-value interaction · 63-cycle
KON/KOFF window · gaussian `$801` overflow · FIR wrap-vs-saturate asymmetry · EDL/ESA change
latency · PPU1-vs-PPU2 separate open-bus latches · CGRAM write during active display · OAM address
destruction during render · overscan mid-frame VRAM lock · auto-read start-window race · mouse
170/336-cycle minimums · multitap 17th connected bit.

---

## Part VIII — Live documentation conflicts

Ten conflicts surfaced where sources disagree today. AccuracySNES can settle several; the rest
become golden vectors.

| Conflict | Sources | Working answer |
|---|---|---|
| `$213F` 50/60 Hz flag bit | SNESdev PPU_registers says bit 3; fullsnes says bit 4 | **bit 4** (bits 3-0 are the version field) |
| `$FFD5` speed bit | SNESdev prose says bit 7; its own `001smmmm` diagram and fullsnes say bit 4 | **bit 4** |
| `$4212` HBlank flag bit | Mesen prose says bit 4; its own diagram and fullsnes say bit 6 | **bit 6** |
| HBlank set/clear dot | superfamicom says H=274/H=1; Mesen says H≈$121/$12 | **unresolved** (~15-dot gap) |
| Dots per scanline | SNESdev says 341; Anomie/superfamicom/Mesen say 340 with dots 323/327 stretched | **340 + 2 stretched** (reconciles to 1364) |
| `$2103` priority-rotation source | fullsnes says register bits 6-1; Anomie says `(internal OAMAddr & $FE) >> 1` | **unresolved** |
| WRIO bit→port mapping | connector page vs Mesen wiki disagree on bit 6/7 | bit 7 → port 2 (matches bsnes/ares) |
| Key-on delay | nocash/blargg/ares say 5 samples; Anomie says ~8 | **5** |
| Post-reset ENDX | nocash says $FF; Anomie says 0 | **unresolved** — golden vector |
| CGRAM during HBlank | SNESdev says yes; fullsnes says "doesn't work too well" | **unresolved** |

Also noted: undisbeliever's opcode table has two transcription errors (16-bit `LSR`/`ROL`/`ROR`
memory forms listed as +1 rather than +2; the branch penalty stated as bare "+1 if e=1" without the
required page-cross condition). fullsnes has essentially no 65816 core documentation — it is
authoritative only for I/O registers, MEMSEL, and clock rates.

---

## Part IX — RustySNES's own admitted approximations

These are the highest-value AccuracySNES targets, because RustySNES's own docs already concede
them. They are catalogued here so the initial pass rate is understood as **information, not
regression**.

| Gap | Source | Disposition today |
|---|---|---|
| Offset-per-tile (Modes 2/4/6) and interlace field doubling not wired to dot resolution | `docs/ppu.md:185-186` | acknowledged simplification |
| Per-scanline compositor — *"bit-identical to a per-dot renderer only when no register a line's rendering reads is changed mid-line — but it does NOT always hold"* | `docs/ppu.md:169-190` | acknowledged |
| 65816 access **order** never validated against SingleStepTests pin traces | `docs/cpu.md:193` | acknowledged |
| ABORT + sub-instruction interrupt injection mid-RMW not modelled | `docs/cpu.md:195` | acknowledged |
| `STP`/`WAI` exact wake-edge timing approximate | `docs/cpu.md:194` | acknowledged |
| S-DSP 32-tick interleave batched once per output sample | `docs/apu.md:333` | expandable if `spc_dsp6` demands |
| SMP glitchy `{2,4,10,20}` wait-state divider collapsed to `SMP_WAIT = 2` | `crates/rustysnes-apu/src/lib.rs:53-55` | acknowledged in the constant's own doc |
| `$4212` bit 0 (auto-joypad busy) unimplemented | `crates/rustysnes-core/src/bus.rs:722-725` | verified 2026-07-19; only bits 7/6 modelled |
| `$4210` lacks the 4-cycle held-flag (Terranigma) quirk and open-bus bits 4-6 | `bus.rs:710-715` | verified 2026-07-19 |
| Auto-joypad read timing window entirely unmodeled | `bus.rs:730-739` | verified 2026-07-19; no `auto_joypad` symbol exists in the core |
| Open-bus-via-HDMA-latch | `docs/accuracy-ledger.md` | **Deferred** — correct fix breaks 24 GSU goldens, root cause unknown |
| Hi-res Modes 5/6 real-title validation | `docs/ppu.md` | **No-stricter-oracle-available** |
| `frame_hires` cached per-frame; `compose_dac` never consults it live | `docs/ppu.md:409` | acknowledged |

And the behaviors RustySNES has already ruled **Out-of-scope with evidence** — AccuracySNES should
report these as golden vectors or variants, never as failures:

- `$4203`/`$4206` overlapping multiply/divide — SNESdev errata says genuinely undefined; *"inventing
  one would violate the determinism-contract spirit."*
- The DMA/HDMA-collision crash quirk — two of three sub-behaviors are 5A22 v1/v2 chip defects
  compliant ROMs avoid and no mainstream emulator reproduces; the third has no known title or ROM.
- DRAM refresh as an additive stall — measured across 500 frames × 3 ROMs; the current model already
  yields the correct ≈357,368-clock NTSC frame, so adding a stall would be a regression.

---

## Part X — Sources

**Primary hardware references (fetched live 2026-07-19):** SNESdev Wiki —
`Errata`, `Timing`, `Tricky-to-emulate_games`, `Uncommon_graphics_mode_games`, `Emulator_tests`,
`PPU_registers`, `CPU_registers`, `Memory_map`, `Tests` · fullsnes (nocash) · Anomie's SNES
documents · superfamicom.org 65816 reference · 6502.org 65c816opcodes · WDC 65C816 datasheet ·
TASVideos SNES accuracy pages.

**AccuracyCoin:** `github.com/100thCoin/AccuracyCoin` (README, `AccuracyCoin.asm`,
`AccuracyCoin.nes`, LICENSE) · `github.com/100thCoin/TriCNES` · ares issue #2275 ·
jgenesis issue #551 · nesdev forum t=26533 · MesenCE 2.2.1 release notes · the author's
"Auto-Test" TAS.

**RustyNES (read directly):** `crates/rustynes-test-harness/src/accuracy_coin.rs`,
`src/accuracy_coin_catalog.rs`, `tests/accuracycoin.rs`,
`scripts/accuracycoin-build/build_sub_test_rom.py`, `tests/roms/AccuracyCoin/README.md`,
`tests/roms/LICENSES.md`, `docs/STATUS.md`,
`docs/audit/accuracycoin-readme-analysis-2026-05-17.md`.

**Reference emulators (`ref-proj/`, study-only):** ares (ISC, commit `6dc3f33`) — `ares/sfc/`;
bsnes (GPLv3, commit `7d5aa1e`) — `bsnes/sfc/`; Mesen2 (GPLv3, commit `b9fa69d`) — `Core/SNES/`,
`Core/Shared/RecordedRomTest.h`, `UI/Utilities/TestRunner.cs`. **GPLv3 sources are clone-only: study
behavior and re-implement clean-room; never copy.**

**This repository:** `docs/testing-strategy.md`, `docs/STATUS.md`, `docs/accuracy-ledger.md`,
`docs/cpu.md`, `docs/ppu.md`, `docs/apu.md`, `docs/scheduler.md`, `docs/cart.md`,
`docs/rom-test-corpus.md`, `docs/adr/0002-0005`, `ref-docs/research-report.md`,
`ref-docs/2026-06-24-{apu,ppu,coprocessors}.md`, `tests/roms/README.md`,
`crates/rustysnes-test-harness/**`, `crates/rustysnes-core/src/{bus.rs,facade.rs,scheduler.rs}`,
`crates/rustysnes-cart/src/header.rs`, `crates/rustysnes-apu/src/lib.rs`.

**Research completeness note.** Agent 4's DMA/HDMA and test-ROM-survey sub-agents did not return;
Parts V.D and VII were assembled from the Errata page, the SNESdev Timing page, the
Tricky-to-emulate-games table, the Emulator_tests and TASVideos pages, agent 5's direct reads of
ares/bsnes/Mesen2 DMA source, and this repository's own docs. Those two sections are solid but less
exhaustively sourced than the rest — a follow-up pass specifically on HDMA per-channel cycle
overheads and a fuller GitHub sweep for SNES test ROMs is warranted before Phase C begins.
