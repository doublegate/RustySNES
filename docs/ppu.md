# PPU1 (5C77) + PPU2 (5C78) — RustySNES

**References:** `ref-docs/2026-06-24-ppu.md` (the primary source for this doc);
`ref-docs/research-report.md` §2; `docs/scheduler.md` (dot timeline, DMA/HDMA). Cited inline:
SNESdev PPU registers / Backgrounds / Sprites / Mode 7, Fullsnes.

This doc is the SPEC, not history — update it in the same PR as the code. Pin behavior
against the test ROMs first.

## Purpose

The SNES picture-processing unit is **two chips** on the CPU's B-bus ($2100–$21FF), sharing
a 16-bit data bus but with separate address buses (`ref-docs/2026-06-24-ppu.md` §1). They map
onto two sub-modules in `rustysnes-ppu`:

- **PPU1 = 5C77:** OAM / sprites + the background / sprite rendering pipeline, including the
  Mode 7 multiply result. `STAT77 $213E` = version + sprite over / time flags;
  MPYL/M/H ($2134–$2136) read from PPU1.
- **PPU2 = 5C78:** CGRAM (palette) access, color math / output, and the timing / status flags
  (H/V counters, VBlank/HBlank, the latch). `STAT78 $213F` = version + NTSC/PAL +
  counter-latch flag.

The PPU sees only a narrow `PpuBus` trait for VRAM/CGRAM/OAM (`docs/architecture.md` §2).

## Background modes 0–7

Per `ref-docs/2026-06-24-ppu.md` §2 (SNESdev Backgrounds, Fullsnes):

| Mode | BG1 | BG2 | BG3 | BG4 | Layers | Special |
|---|---|---|---|---|---|---|
| 0 | 2bpp | 2bpp | 2bpp | 2bpp | 4 | separate palette region per BG |
| 1 | 4bpp | 4bpp | 2bpp | — | 3 | BG3 priority-bit selectable (BGMODE bit 3) |
| 2 | 4bpp | 4bpp | (OPT) | — | 2 | offset-per-tile |
| 3 | 8bpp | 4bpp | — | — | 2 | high-color |
| 4 | 8bpp | 2bpp | (OPT) | — | 2 | offset-per-tile |
| 5 | 4bpp | 2bpp | — | — | 2 | forced 512-px hi-res (16×8 tiles) |
| 6 | 4bpp | — | (OPT) | — | 1 | hi-res + offset-per-tile |
| 7 | 8bpp | — | — | — | 1 | affine; +EXTBG → BG2 |

- **BGMODE $2105:** bits 0–2 = mode, bit 3 = BG3 priority (Mode 1), bits 4–7 = per-BG tile
  size (8×8 vs 16×16).
- **Offset-per-tile** is Modes 2/4/6 only. **Hi-res Modes 5/6** force 512-px horizontal.

### Mode 7 affine

Single **128×128-tile** map (256 distinct 8×8 tiles), 8bpp, affine 2×2 matrix
(`ref-docs/2026-06-24-ppu.md` §2):

- **M7A $211B / M7B $211C / M7C $211D / M7D $211E** — each written low-then-high, **16-bit
  signed 8.8 fixed-point**. Typical: M7A=+cos·SX, M7B=+sin·SX, M7C=−sin·SY, M7D=+cos·SY.
- **Center M7X $211F / M7Y $2120** — 13-bit signed, written twice. **M7SEL $211A** =
  wrap / flip.
- **Hardware-multiply reuse:** M7A × (M7B high byte) → signed 24-bit at MPYL/M/H
  ($2134–$2136), the general-purpose multiplier, no delay.
- **EXTBG (SETINI $2133 bit 6):** turns Mode 7's high color bit into a per-pixel priority bit
  (BG1 8bpp + BG2 7bpp).

## OAM / sprite model

Per `ref-docs/2026-06-24-ppu.md` §3 (SNESdev Sprites, Fullsnes):

- **128 sprites**, **OAM = 544 bytes** (low table 512 + high table 32).
- **Low table (4 B/sprite):** byte0 X-low, byte1 Y, byte2 tile-low, byte3 `VHPP CCCt` (name
  high bit, 3-bit palette, 2-bit priority, V/H flip). **High table (2 b/sprite):** X high bit
  plus a size-toggle.
- **OBSEL $2101** `SSS NN bBB`: name base, name select, size pair (0–7: 8/16, 8/32, 8/64,
  16/32, 16/64, 32/64, + two rectangular pairs).
- **Per-scanline limits:** **Range Over = 32 sprites/line** (STAT77 bit 6); **Time Over = 34
  sprite-tiles/line** (STAT77 bit 7); both reset at end of VBlank. **Lower OAM index = on
  top**; tile fetch is reverse-order, so **low-index sprites drop first** on overflow.

## Dot-clock timeline & H/V counters

Per `ref-docs/2026-06-24-ppu.md` §4 and `docs/scheduler.md` (which is binding for the
numbering convention — **341 dots / line, dots nominally 4 master clocks**):

- Normal line = 1364 clocks = 341 dots; short = 1360 (NTSC non-interlace, V=240 alt frames);
  long = 1368 (PAL interlace field=1 V=311). 262/312 lines (NTSC/PAL), +1 interlaced.
- Active output dots 22–277 on lines 1–224 (or 1–239 overscan). **VBlank** at V=225 (or V=240
  overscan). **HBlank** H=274→H=1.
- **H/V latch:** read **SLHV $2137** latches H/V; **OPHCT $213C / OPVCT $213D** read twice for
  the 9-bit values; **STAT78 $213F** read clears the latch + resets the toggle.

## CGRAM, VRAM, color math, windows

Per `ref-docs/2026-06-24-ppu.md` §6:

- **CGRAM:** 256 colors, **15-bit BGR** (R 0–4, G 5–9, B 10–14). **CGADD $2121** addr;
  **CGDATA $2122** write twice; `$213B` read twice.
- **VRAM:** **64 KiB = 32 K words**, word-addressed. **VMAIN $2115** = increment step
  (1/32/128 words) + remap + high/low toggle. **VMADDL/H $2116/7**, write `$2118/9`, read
  `$2139/A` (prefetch-latch). Tile sizes: 2bpp=16 B, 4bpp=32 B, 8bpp=64 B per 8×8.
- **Color math:** **CGWSEL $2130** (window black / transparent + addend select + direct
  color), **CGADSUB $2131** (add/sub, half, per-layer enable), **COLDATA $2132** (fixed
  color). **Windows $2123–$2129** (W1/W2, OR/AND/XOR/XNOR, positions). **TM $212C / TS $212D**
  main / sub layer enable.
  - **Subscreen-backdrop addend = the fixed color.** When "add subscreen" (CGWSEL bit 1) is set
    but the *subscreen* pixel at that column is the backdrop (no opaque sub-layer wrote it), the
    addend is **COLDATA's fixed color**, not CGRAM[0] — mirroring ares `DAC::above`
    (`io.blendMode && math.transparent ⇒ blendMode = false ⇒ addend = fixedColor()`), and the
    half is suppressed for that pixel. This is what paints SMW's blue sky (the fixed color) over
    the black main backdrop; treating the sub-backdrop as CGRAM[0] renders the sky black.
  - **BG palette-group offset.** A BG tilemap entry carries a 3-bit palette group (bits 12–10).
    The CGRAM index of a BG pixel is `paletteBase + (group << bpp) + color` (masked to a byte;
    `bpp` = 2/4/8, so 8bpp ignores the group), where `paletteBase = id<<5` only in Mode 0. Per
    ares `background.cpp`. Dropping the group collapses every tile onto palette group 0 and
    washes multi-palette art (the SMW logo/border).

## Frame structure / resolutions

Per `ref-docs/2026-06-24-ppu.md` §7: standard **256×224**; overscan **256×239** (SETINI
bit 2); forced hi-res **512** (Modes 5/6) or pseudo-hires (SETINI bit 3); interlace **448/478**
(SETINI bit 0). **SETINI $2133** `EX.. HOiI`: bit0 interlace, bit1 OBJ interlace, bit2
overscan, bit3 pseudo-hires, bit6 EXTBG, bit7 ext sync.

## DMA / HDMA into the PPU

The DMA/HDMA registers ($43n0–$43nA) live in `rustysnes-core`; the timing and cycle-steal
budget is specified in `docs/scheduler.md` §DMA/HDMA. The PPU is the typical *target*
(VRAM/CGRAM/OAM via the B-bus), and HDMA per-line writes to the PPU registers are the
mechanism behind raster effects — they must land at the exact dot (`ref-docs/2026-06-24-ppu.md`
§5).

## Edge cases and gotchas

1. **Mid-scanline writes** to scroll / Mode-7 matrix / CGRAM must take effect at the exact dot
   — the reason the PPU renders at dot resolution on the master-clock scheduler.
2. **The 32-sprite / 34-tile limits** with reverse-order fetch (low index drops first) drive
   the over/time flags games poll.
3. **VRAM access is forbidden mid-frame** except in force-blank / VBlank; mid-frame writes
   corrupt — undisbeliever's mid-scanline-VRAM ROM tests this.
4. **VRAM read prefetch latch** ($2139/A) returns the *previous* word on the first read after
   an address set.
5. **The 340-vs-341 dot convention** is resolved in `docs/scheduler.md`; do not reintroduce
   the alternate numbering here.

## Test plan

- **Committable PPU/DMA/HDMA layer:** undisbeliever/snes-test-roms (Zlib) — HDMA timing,
  force-blank mid-frame, mid-scanline VRAM, OAM dropout.
- **Video / integration:** 240p Test Suite (SNES) (GPLv2, run-only) patterns;
  Krom/PeterLemon Mode-7 / HDMA / window / blend ROMs (no-license, reference-only,
  screenshot-diff).
- **Visual golden:** deterministic framebuffer hashes for a canonical commercial set
  (`tests/golden/`); ROMs stay gitignored in `tests/roms/external/`.

## Implementation status (`rustysnes-ppu`, v0.1.0)

The crate is a working dual-chip model. Public API the scheduler/bus call:

- **Storage:** `vram: [u16; 0x8000]`, `cgram: [u16; 256]`, `oam: [u8; 544]` (all owned).
- **Registers:** `write_reg(&mut self, addr: u16, val: u8)` / `read_reg(&mut self, addr: u16) -> u8`
  cover `$2100`–`$213F`. Modeled quirks: shared BG-offset write latch (PPU1/PPU2 halves), the
  Mode-7 / scroll write-twice latch, `VMAIN` remap + increment-on-low/high, the CGRAM
  write-twice + `$213B` read-twice, the OAM even-byte latch, the `$2139/A` VRAM read prefetch
  latch, `MPYL/M/H` Mode-7 multiply, `SLHV $2137` latch, `OPHCT/OPVCT` read-twice 9-bit,
  `STAT77/78` (over-flags + version + NTSC/PAL + field; `$213F` read clears the latch). Reads
  of write-only/unused registers return the PPU MDR (open-bus) latch.
- **Timeline:** `tick_dot(&mut self, bus: &mut impl VideoBus)` advances H 0..=340 / V per region
  (262 NTSC / 312 PAL), sets VBlank at V=225 (V=240 overscan) and HBlank, fires
  `notify_scanline`/`notify_vblank`, raises NMI at VBlank start, and level-fires the HV-IRQ
  comparator (`set_hv_irq(enable_h, enable_v, h, v)` programs it). The horizontal match is
  asserted `HIRQ_TRIGGER_DELAY` (4) dots **after** the programmed `HTIME`, modelling the SNES
  counter→CPU interrupt communication delay (ares `hcounter(10) == (HTIME+1)<<2`; see
  `docs/scheduler.md` §H/V-IRQ). Without it an IRQ-gated register write lands a few dots early.
- **Polls (the scheduler reads these — no extra `VideoBus` methods were added):**
  `nmi_pending()`/`ack_nmi()`, `irq_pending()`/`ack_irq()`, `in_vblank()`/`in_hblank()`,
  `dot()`/`scanline()`, `frame_ready()`/`take_frame()`/`frame_count()`, `framebuffer() -> &[u16]`.
- **Rendering model:** **per-scanline** — the whole visible line is composited at the line's
  end into a `256×239` 15-bit framebuffer. This is far simpler than a per-dot renderer, and
  bit-identical to one **only when no register a line's rendering reads is changed mid-line**
  (the determinism contract only requires the finished frame be reproducible, so this is a valid
  simplification *when the equivalence holds* — but it does NOT always hold: see "Mid-scanline/
  HDMA-driven register timing" below for a confirmed off-by-one-line compositor bug this
  end-of-line-sampling approach causes for HDMA-driven per-line register changes, and a designed
  fix that is NOT yet landed pending a Super FX/GSU regression investigation). BG modes 0–7
  tile fetch (2/4/8 bpp), per-mode priority tables, 16×16 tiles,
  mosaic (vertical+horizontal block), Mode 7 affine (matrix + center + wrap/flip from M7SEL,
  EXTBG high-bit priority), the 128-sprite OAM pipeline with the 32-sprite range / 34-tile time
  limits (reverse-order fetch → low index survives → STAT77 bits 6/7), color math (add/sub/half,
  per-layer enable, fixed-color/subscreen addend, direct color), windows (W1/W2 OR/AND/XOR/XNOR
  + per-layer enable + the CGWSEL color-math regions), and INIDISP master brightness all work.
  Not yet wired to dot resolution: hi-res Modes 5/6 render at 256-wide (the 512-px doubling is
  deferred), offset-per-tile (Modes 2/4/6), pseudo-hires, and interlace field doubling — the
  per-line compositor is the simplification point for those, landing with HDMA/raster work.

## Open questions

- Exact long-dot placement within the 1364-clock line under interlace transitions — resolve
  against the test ROMs (`ref-docs/2026-06-24-ppu.md` "Note on a flagged discrepancy").
- Mid-scanline raster effects (HDMA palette/scroll splits) need the scheduler to drive register
  writes at the exact dot. **Correction (researched, superseding the prior claim here):** the
  per-scanline compositor does NOT correctly handle even single-split-per-line HDMA effects — see
  "Mid-scanline/HDMA-driven register timing" below for the confirmed off-by-one-line bug, a
  designed and SA-1-verified fix, and why it isn't landed (a real, separate Super FX/GSU
  regression the same change causes, not yet understood).

## Mid-scanline/HDMA-driven register timing — designed, prototyped, blocked (v0.6.0-era)

**Status: the off-by-one-line compositor bug confirmed in `v0.5.0` has a designed and prototyped
fix that is independently verified CORRECT for CPU/HDMA-driven register changes (SA-1's `SD F-1
Grand Prix`, pixel-exact confirmation, see "The SA-1 case" below) — but the SAME change causes a
second, genuinely different, NOT-YET-UNDERSTOOD regression across every Super FX/GSU golden test
(24/24), with a diff signature that does NOT fit the fix's own mechanism (see "The Super FX/GSU
regression" below). Because both effects come from one code change, the fix cannot be landed
piecemeal — it is NOT landed. The prototype (`rustysnes-ppu::RENDER_DOT`, `Ppu::tick_dot`
compositing at that dot instead of end-of-scanline) is fully designed and described below so a
future investigation can pick it up directly; the missing piece is understanding *why* GSU-driven
rendering specifically regresses.**

### The bug (the "Air Strike Patrol BG3 scroll" case)

RustySNES's compositor renders scanline `V` at the very end of that line's own dot loop
(`Ppu::end_of_scanline`, called when `h` wraps `DOTS_PER_LINE → 0`, i.e. after all 341 dots of
line `V` — including line `V`'s own HDMA run — have already been processed by
`Bus::advance_master`). HDMA's per-visible-line *run* fires at dot 276 (`HDMA_RUN_DOT`,
`docs/scheduler.md` §DMA/HDMA bus-steal), which is *before* dot 340 where `end_of_scanline` runs.
So when line `V`'s own HDMA write updates a scroll/CGRAM/window register, RustySNES's compositor
— reading live register state at dot 340 — observes that NEW value and paints it across the
*entirety* of line `V`'s already-rendered pixels.

Real hardware does not do this. ares' PPU is a genuine per-pixel renderer (`ref-proj/ares/ares/
sfc/ppu/main.cpp`): `PPU::main()`'s `cycle<Cycle>()` unrolling only calls `cycleRenderPixel()`
(the DAC/compositor output stage, `dac.cpp`'s `DAC::run()`) for `Cycle` in `[56, 1078]`
(`main.cpp:217-219`) — i.e. every active pixel of line `V` is drawn using live register state by
hcounter ~1078 (dot ~269.5), *before* HDMA services that same line at hcounter 1104 (dot 276,
`ref-proj/ares/ares/sfc/cpu/timing.cpp`'s `hdmaRun()` trigger). A per-line HDMA write during line
`V` therefore lands **after** line `V` has already finished drawing on real hardware — it can only
take effect starting line `V+1`. This is the entire mechanism games like Air Strike Patrol rely on
for a smooth per-line BG3 raster scroll: HDMA writes `S[V+1]` during line `V`'s late-active
period, ready in time for line `V+1`'s draw.

RustySNES's current end-of-line compositor instead applies `S[V+1]` to line `V` itself — every
HDMA-driven per-line register change lands **one scanline too early**. This is a real,
previously-undocumented accuracy bug (the prior claim in this doc, that "the per-scanline
compositor already samples end-of-line register state, so single-split-per-line effects work",
was wrong — corrected above). It is a systemic bug, not confined to Air Strike Patrol; any title
that HDMA-drives a per-line scroll/window/CGRAM change is affected. The `crates/rustysnes-ppu/
src/lib.rs` module doc's claim that the per-scanline compositor is "bit-identical in the final
framebuffer to a per-dot renderer" is likewise only true when no register HDMA-changes mid-line —
false in exactly this case.

### The prototype fix (designed, not landed)

`rustysnes-ppu` would gain a new public constant, `RENDER_DOT` (`= 276`), documented as the PPU's
own video-timing fact (not a DMA-specific one) that `rustysnes-core::bus`'s `HDMA_RUN_DOT` is
defined equal to — a regression-locking `#[test]` in `rustysnes-core` would assert the two never
drift apart, since `rustysnes-ppu` cannot depend on `rustysnes-core` (the crate graph is strictly
one-directional, `docs/architecture.md`), so the PPU-owned constant is the single source of truth.

`Ppu::tick_dot` would call `render_scanline` at `self.h == RENDER_DOT` (inside the main per-dot
loop), not inside `end_of_scanline` (which currently runs it at dot 340/`DOTS_PER_LINE`
wraparound). Because `Bus::advance_master` services line `V`'s own HDMA run at this same dot but
*after* `tick_ppu_dot` returns within the same master-clock tick (the HDMA check runs after the
PPU-dot call, `docs/scheduler.md` §DMA/HDMA bus-steal), line `V` would be composited using
register state as it stood strictly *before* that line's own HDMA write — matching ares'
per-pixel timing exactly, without giving the PPU crate any direct knowledge of DMA/HDMA.

### The SA-1 case (verified correct)

Prototyping this change and running the full `--features test-roms` golden/oracle suite: all 29
`undisbeliever` goldens held, every `*_oncart`/commercial-screenshot test held, with exactly
**one** exception — SA-1's `SD F-1 Grand Prix` golden framebuffer hash changed. That one change
was independently confirmed as a real accuracy improvement, not blindly re-blessed: dumping the
pre-prototype and post-prototype framebuffers and diffing them row-by-row found 159/239 rows
differ, and testing the specific hypothesis "does `fixed[row]` match `buggy[row-1]`" (the exact
signature this change's mechanism predicts — content that used to render one line early now
renders one line later) matched 232/237 candidate rows (97.9%), with **zero** rows differing in
any other pattern (every single differing row fully explained by a clean, uniform one-line-later
shift — no unexplained artifacts). `SD F-1 Grand Prix` is a racing game, consistent with a
per-line HDMA-driven road/gradient color effect (it drives `$2100`-adjacent CGRAM/brightness-class
registers per scanline) that was rendering one line early before this change and would render on
the hardware-correct line after it. This half of the change is real and correct — it just cannot
ship in isolation from the GSU regression below, since both come from the same code path.

### The Super FX/GSU regression (blocking, not understood)

The same prototype broke **all 24** Super FX/GSU golden framebuffer tests
(`crates/rustysnes-test-harness/tests/superfx_oncart.rs`) — every `GSU{2,4,8}BPP256x{128,160,192}
{FillPoly,PlotLine,PlotPixel}` combination in the Krom corpus. This is the identical failure
signature (all 24 GSU goldens, at once) an *earlier*, unrelated investigation this session
(open-bus-via-HDMA-latch, `docs/scheduler.md` §Open bus via DMA/HDMA) also hit and correctly did
not land — a second, independent confirmation that Super FX/GSU's interaction with changes near
this exact area of the master-clock tick sequence is fragile in a way not yet understood.

Unlike the SA-1 case, the GSU diffs do **not** fit the "shifted one line later" signature the
prototype's own mechanism predicts:

- `GSU2BPP256x128FillPoly`: only 3 rows differ (not the broad, whole-frame shift SA-1 showed), but
  the pattern is inconsistent with a clean shift — a solid color bar moved from row 219 to row
  215 (4 rows *earlier*, the opposite direction and a larger magnitude than the fix's own
  one-line-*later* mechanism), plus a subtle backdrop-color change at row 0 (`0x290000` →
  `0x000000`) that may be related to line `V=0`'s special-cased once-per-frame HDMA *setup* (as
  opposed to a normal per-line HDMA *run* — `docs/scheduler.md` §DMA/HDMA bus-steal) interacting
  with the new render trigger differently than a normal visible line does.
- `GSU2BPP256x128PlotPixel`: 10 rows differ, only 3 of which fit "unchanged or shifted-one-later"
  — 7 are genuine outliers matching neither hypothesis.

**Working hypothesis (not confirmed):** the GSU coprocessor is host-synced via
`Board::coprocessor_tick`, stepped once per master-clock unit from inside the same
`Bus::advance_master` loop the PPU's own dot-tick runs from (`docs/cart.md` §Super FX). Moving the
PPU's render trigger from dot 340 (very late in the line) to dot 276 (earlier) may sample GSU
VRAM/CGRAM writes at a point where the GSU's own per-tick progress differs from before — either
catching genuinely in-progress GSU output that used to be complete by dot 340, or missing GSU
writes that used to land before the old (later) sample point. This is a plausible, coherent
explanation for why GSU-driven rendering specifically regresses while SA-1 (a second CPU that
does not render directly into VRAM) improves from the identical change — but it has **not been
confirmed** by tracing actual GSU tick/write timing against the new render point, only inferred
from the shape of the pixel diffs above. A real investigation needs that trace, not more
inference from framebuffer diffs alone.

### What a future investigation needs

1. An access-level trace of GSU VRAM/CGRAM writes (via `coprocessor_tick`) correlated against the
   exact master-clock tick the new `RENDER_DOT` render call would fire on, for one of the
   currently-passing GSU golden ROMs — to confirm or refute the host-sync-timing hypothesis above
   (mirrors `docs/audit/spc7110-boot-crash-2026-07-08.md`'s access-level-trace approach for a
   different coprocessor-timing gap).
2. Once the mechanism is understood, either: (a) a GSU-aware adjustment to when/how the PPU
   samples VRAM for lines a host-synced coprocessor is actively rendering into, or (b) evidence
   that the current (pre-prototype) end-of-line render point is actually the *hardware-correct*
   one for GSU-authored content specifically (in which case the fix would need to special-case
   host-synced-coprocessor carts rather than apply universally).
3. Full re-verification against `cargo test --workspace --features test-roms` (all 29
   `undisbeliever` goldens + all 24 GSU goldens + SA-1's `SD F-1 Grand Prix`) before landing
   anything — the same discipline this investigation and the sibling open-bus one both applied.

### Regression-baseline test

`crates/rustysnes-core/tests/mid_scanline_hdma_baseline.rs` is a minimal, self-authored
hand-assembled 65C816 program (mirroring the pattern already used for rewind/run-ahead's
synthetic per-frame signal tests) that drives `$2100` (`INIDISP`, master brightness) via HDMA
mode 0, alternating full-brightness (backdrop renders white) and force-off (backdrop renders
black) at a single transition partway through the frame. No BG/OBJ layer is ever enabled, so
every pixel falls through to the backdrop color (`Ppu::layer_color`'s `!p.opaque` path) — this
isolates the exact compositor-vs-HDMA dot-timing bug with no tilemap/tileset setup at all, at the
cost of not being the specific "Air Strike Patrol BG3 scroll" scenario (a scroll-register variant
remains open future work if a title-accurate reproduction is ever wanted).

The test still **locks in the current (confirmed-buggy) transition position** —
`(last-white row 99, first-black row 100)` — since the fix is not landed (see above). When a real
fix eventually lands (and passes the Super FX/GSU investigation this section now requires), this
specific assertion is expected to flip to `(100, 101)` — a deliberate, reviewed Golden-Vector
update, cross-checked against the *full* `--features test-roms` suite including all 24 GSU
goldens this time, not just the 29 `undisbeliever` ones. A second test confirms the reproduction
stays deterministic across fresh runs.

## Hi-res (Modes 5/6) color-math precision — researched, deferred (v0.5.0)

**Status: researched; blocked entirely on 512-px hi-res output not existing yet (a real feature
gap, not a precision nuance to layer on top of an existing feature) — see "Implementation status"
above ("hi-res Modes 5/6 render at 256-wide, the 512-px doubling is deferred").**

Real hardware's hi-res color-math trick (the mechanism Bishoujo Janshi Suchie-Pai / Marvelous+SA-1
rely on for pseudo-transparency/anti-aliasing) is confirmed against ares' DAC (`ref-proj/ares/
ares/sfc/ppu/dac.cpp`, `DAC::run()`): in hi-res, **each PPU pixel clock emits two real output
columns** by alternating between two *different* compositor results, not by computing 512
independently-color-math'd columns —

```
*line++ = hires ? belowColor : aboveColor;  // the "even" 512-wide column
*line++ = aboveColor;                        // the "odd" 512-wide column (always the normal path)
```

`aboveColor` is the normal (256-wide) main-screen-composited-with-color-math pixel; `belowColor`
is the *sub*-screen pixel blended a second time against the fixed/subscreen color (`DAC::below`,
`dac.cpp:43-80`) — i.e. hi-res mode doesn't add new *spatial* detail, it doubles the horizontal
rate and interleaves "with color math" and "as if reading one pixel earlier, blended differently"
outputs to fake a smoother 512-wide gradient. This is architecturally a real feature (dual-output
DAC stage), not a numeric-precision tweak to the existing color-math formula.

Since `rustysnes-ppu` doesn't yet emit 512-wide output at all (Modes 5/6 currently render at the
normal 256-wide resolution, per "Implementation status" above), there is no existing hi-res output
path to add color-math precision *to* — the color-math mechanism described here can only be
modeled once genuine dual-half-pixel hi-res output is built. That's a real feature addition (a new
`DAC`-equivalent output stage emitting two columns per pixel clock, alternating `above`/`below`
compositor results), not a small targeted fix, and is explicitly out of scope for this pass per
the same regression-risk discipline as the mid-scanline item above. Tracked as: build 512-wide
hi-res output first (a separate, larger v0.5.0/v0.6.0-scope item), then revisit hi-res color-math
precision once that foundation exists.
