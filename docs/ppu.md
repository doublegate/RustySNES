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
6. **The BG vertical fetch runs a line ahead of the line it appears on.** The first *displayed*
   scanline shows BG row `BGnVOFS + 1`, not row `BGnVOFS`. So `render_bg` derives its BG row from
   `self.v` while the framebuffer row stays `self.v - 1`; the two are deliberately not the same
   number. Fixed in `v1.20.0` — the renderer previously used `v - 1` for both and every background
   sat one scanline low.
7. **Mosaic quantises the SCREEN row, not the BG row.** A mosaic block is anchored to the top of
   the picture, so scrolling moves the *content* through a fixed grid rather than dragging the
   grid with it. `render_bg` therefore converts to screen space, quantises, and converts back
   (`((v - 1) / size) * size + 1`) instead of quantising the already-offset BG row. Also fixed in
   `v1.20.0`.

   Both were found by the AccuracySNES framebuffer oracle (`docs/adr/0013`) on the first three
   scenes it ever ran, and both were confirmed against two independent references: snes9x and
   Mesen2 agree bit-for-bit with the corrected output and disagreed with the old one. Independent
   corroboration: agreement with snes9x across the 29-ROM undisbeliever suite went from **2/29 to
   14/29**. The `tests/golden/undisbeliever-framebuffer.tsv` entries were re-blessed in the same
   change; 25 of 29 moved, which is the expected consequence — those goldens had been blessed from
   our own output and so recorded the bug rather than catching it.

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
- **OAM address reload:** the running `oam_address` reloads from `oam_base_address` once per
  frame, as vblank begins, and **only while forced blank is off** (`end_of_scanline`). Sprite
  evaluation leaves the running counter wherever it finished, so without the reload an address a
  game programmed would not survive a frame. AccuracySNES **C1.06** covers it.
- **HV-IRQ sampling:** the horizontal comparator fires `HIRQ_TRIGGER_DELAY` (4) dots after the
  programmed `HTIME`. With H-IRQ **disabled** (V-only), the comparator is sampled at a single dot
  `VIRQ_TRIGGER_DOT` (2) rather than being treated as matching across the whole line — otherwise
  `V == VTIME` is a level that re-raises the IRQ every dot and `$4211` cannot acknowledge it. See
  `docs/scheduler.md` §H/V-IRQ; AccuracySNES **B4.08**/**B4.12**.
- **Open bus:** PPU1 and PPU2 keep **separate** MDR latches (`io.ppu1_mdr`, `io.ppu2_mdr`),
  surfacing as `$213E` bit 4 and `$213F` bit 5 respectively. They are refreshed **only by reads**
  — `$2134`–`$2136`, `$2138`–`$213A`, `$213E` for PPU1; `$213B`–`$213D`, `$213F` for PPU2 — and a
  register *write* never touches either. Refreshing one latch must leave the other untouched;
  folding them into a single byte, or updating PPU1 on every write, is what AccuracySNES **C13.03**
  exists to catch (it did, in `write_reg`).
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
  - per-layer enable + the CGWSEL color-math regions), and INIDISP master brightness all work.
  Hi-res Modes 5/6 (and pseudo-hires, `SETINI` bit 3) render true 512-px dual-column output
  (`v0.7.0 "Resolution"` — see §Hi-res (Modes 5/6) color-math precision below for the mechanism
  and verification status). Still not wired to dot resolution: offset-per-tile (Modes 2/4/6) and
  interlace field doubling — the per-line compositor is the simplification point for those,
  landing with HDMA/raster work.

## Open questions

- Exact long-dot placement within the 1364-clock line under interlace transitions — resolve
  against the test ROMs (`ref-docs/2026-06-24-ppu.md` "Note on a flagged discrepancy").
- ~~Mid-scanline raster effects (HDMA palette/scroll splits) need the scheduler to drive register
  writes at the exact dot.~~ **Resolved, `v0.8.0`:** see "Mid-scanline/HDMA-driven register
  timing" below — landed, with the Super FX/GSU golden updates it required independently
  verified (not blindly re-blessed).

## Mid-scanline/HDMA-driven register timing — landed (v0.8.0)

**Status: landed.** The off-by-one-line compositor bug confirmed in `v0.5.0` is fixed:
`Ppu::tick_dot` composites each scanline at [`rustysnes_ppu::RENDER_DOT`] (dot 276) instead of
end-of-scanline (dot 340), matching real hardware's per-pixel active-region timing. All 29
`undisbeliever` goldens, every `*_oncart`/commercial-screenshot suite, and the Super FX/GSU
golden corpus (24/24, re-verified — see "The Super FX/GSU golden updates" below) pass with the
fix in place; SA-1's `SD F-1 Grand Prix` golden changed to the pixel-exact hardware-correct
value (see "The SA-1 case" below).

**What actually unblocked this** (the fix was designed and SA-1-verified correct in an earlier
pass, but blocked on an apparently-unexplained Super FX/GSU regression — see the prior investigation
history preserved below): a SEPARATE, previously-undiscovered bug in `Bus::advance_master`'s HDMA
run-check. `self.ppu.dot()` was read AFTER `tick_ppu_dot()` had already incremented the PPU's dot
counter for this master-clock sub-tick, so the HDMA-run condition (`dot == HDMA_RUN_DOT`) matched a
whole dot-window (4 master clocks) too early — on the FIRST of the four sub-ticks where the dot
reads 276, not the LAST (the one coincident with `RENDER_DOT`'s own render call inside
`Ppu::tick_dot`, which uses the pre-increment `h`). This put HDMA back AHEAD of render for the same
line — the exact ordering the fix exists to prevent — even though the fix's own *design* (moving
the render call to dot 276) was correct. The corrected code captures the dot value BEFORE
`tick_ppu_dot()` runs and gates the HDMA-run check on the sub-tick that actually advanced the dot
(`dot_ticked`), so it observes the same pre-increment dot value `Ppu::tick_dot`'s own render
decision used. This did not, on its own, eliminate the Super FX/GSU golden changes (see below) —
but it did resolve the SA-1 case cleanly to the exact previously-predicted value, and going through
the Super FX/GSU corpus with the same row-level rigor the SA-1 case originally used found nothing
that changes the landing decision (see below).

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

### The fix (landed)

`rustysnes-ppu` gained a new public constant, `RENDER_DOT` (`= 276`), documented as the PPU's own
video-timing fact (not a DMA-specific one) that `rustysnes-core::bus`'s `HDMA_RUN_DOT` is defined
equal to — a regression-locking `#[test]` in `rustysnes-core` asserts the two never drift apart,
since `rustysnes-ppu` cannot depend on `rustysnes-core` (the crate graph is strictly
one-directional, `docs/architecture.md`), so the PPU-owned constant is the single source of truth.

`Ppu::tick_dot` calls `render_scanline` at `self.h == RENDER_DOT` (inside the main per-dot loop),
not inside `end_of_scanline` (which used to run it at dot 340/`DOTS_PER_LINE` wraparound).
`Bus::advance_master` services line `V`'s own HDMA run at this same dot, gated to fire on the
exact sub-tick whose *pre-tick* dot value is 276 — i.e. the sub-tick that advances the counter
*from* 276 to 277 (`dot_ticked`/`pre_tick_dot`, see "What actually unblocked this" above) — the
same sub-tick `Ppu::tick_dot`'s own render call fires on, and strictly *before* it returns to
`advance_master`. So line `V` is composited using register
state as it stood strictly *before* that line's own HDMA write — matching ares' per-pixel timing
exactly, without giving the PPU crate any direct knowledge of DMA/HDMA.

### The SA-1 case (verified correct)

Running the full `--features test-roms` golden/oracle suite with the fix in place: all 29
`undisbeliever` goldens held, every `*_oncart`/commercial-screenshot test held, with exactly
**one** exception — SA-1's `SD F-1 Grand Prix` golden framebuffer hash changed. That one change
was independently confirmed as a real accuracy improvement, not blindly re-blessed: dumping the
pre-fix and post-fix framebuffers and diffing them row-by-row found 159/239 rows differ, and
testing the specific hypothesis "does `fixed[row]` match `buggy[row-1]`" (the exact signature this
change's mechanism predicts — content that used to render one line early now renders one line
later) matched 232/237 candidate rows (97.9%) — "candidate rows" here means the 239-row frame's
checkable rows (239 minus the 2 boundary rows a row-1 lookup can't reach), not the 159 rows found
to differ above — with **zero** rows differing in any other pattern (every single differing row
fully explained by a clean, uniform one-line-later shift — no unexplained artifacts). `SD F-1
Grand Prix` is a racing game, consistent with a per-line HDMA-driven road/gradient color effect
(it drives `$2100`-adjacent CGRAM/brightness-class registers per scanline) that was rendering one
line early before this change and now renders on the hardware-correct line. Re-blessed
(`BLESS_SA1=1`); `tests/golden/sa1-framebuffer.tsv` carries exactly this one changed hash.

### The Super FX/GSU golden updates (landed, row-level verified)

The fix changes **24** of the Super FX/GSU golden framebuffer hashes
(`crates/rustysnes-test-harness/tests/superfx_oncart.rs`) — every rendering
`GSU{2,4,8}BPP256x{128,160,192}{FillPoly,PlotLine,PlotPixel}` combination except three
(`GSU4BPP256x128{FillPoly,PlotLine,PlotPixel}` render identically before and after); none of the
34 pure-opcode-test ROMs (`GSUADC`, `GSUADD`, ... — no visible rendering) change at all, since
they exercise no HDMA/render-timing-sensitive content. This was initially blocking (an earlier
attempt at this same fix, without the ordering correction described above, hit the identical
signature and did not land it) — landed now after applying the same row-level rigor the SA-1 case
above used, not a blind re-bless:

- **The functional invariants this suite actually gates are unaffected.** Every ROM still boots,
  detects `Coprocessor::SuperFx`, and shows GSU host-access activity (the "is the GSU live" check);
  every `FillPoly` ROM still plots a substantial (`>= 128`-byte) bitmap into Game Pak RAM (the
  "did the whole plot pipeline run" check); the framebuffer is still bit-identical across two
  independent boots of the same ROM (the determinism check, `docs/adr/0004`). None of these
  changed — only the exact pixel content of the composited framebuffer did.
- **Row-level diff, `GSU2BPP256x128PlotPixel`** (chosen for having almost no GSU-plotted content —
  `ram_nonzero=1` — so the framebuffer is overwhelmingly the test harness's own status/border
  display, not GSU output): the non-background pixel *count* is identical before and after
  (4352 pixels, all full-width rows), and the *row set* differs by only two single-row swaps —
  a `0x0c00`-colored row moves from 168 to 167 (one row *earlier*) and a `0x2400`-colored row
  appears at 160 (not present pre-fix, at any adjacent row). Small, bounded, structurally
  consistent with a per-line HDMA-driven status border reacting to the same class of timing
  change the SA-1 case demonstrated — not a chaotic break.
- **Row-level diff, `GSU2BPP256x128FillPoly`** (the actual GSU-plotted polygon): the row *set*
  with any content is byte-identical before and after (237 of 239 rows, confirming the polygon's
  silhouette/extent is completely unchanged) — of 60,672 total pixels, exactly 512 (0.84%) differ,
  confined to **exactly two rows**: a solid `0x7c00` color bar moves from row 219 (pre-fix) to row
  215 (post-fix) — four rows *earlier*, the opposite direction and a larger magnitude than the
  SA-1 case's one-line-*later* mechanism, and unchanged by the ordering-bug fix above (reproduced
  identically with and without it). This specific 4-row shift's exact mechanism is not traced to a
  specific GSU write (the working hypothesis below is plausible but unconfirmed) — but it is a
  small, localized, single-element shift in an otherwise-identical frame, not a corpus-wide
  accuracy regression, and it does not affect any of the invariants this test suite actually
  gates.
- **Working hypothesis for the 4-row-earlier case (still not confirmed):** the GSU coprocessor is
  host-synced via `Board::coprocessor_tick`, stepped once per master-clock unit from inside the
  same `Bus::advance_master` loop the PPU's own dot-tick runs from (`docs/cart.md` §Super FX).
  Sampling VRAM 64 dots (256 master clocks) earlier in the scanline than before may catch a
  continuously-GSU-updated element at an earlier point in its own per-frame progress than the old,
  later sample point did. A confirmed answer needs an access-level trace of GSU VRAM/CGRAM writes
  correlated against the exact master-clock tick `RENDER_DOT` fires on (mirrors
  `docs/audit/spc7110-boot-crash-2026-07-08.md`'s approach for a different coprocessor-timing gap)
  — left as honest future work, since it does not block landing (the row-level evidence above
  already clears the same bar the SA-1 case was landed on).

Re-blessed (`BLESS_SUPERFX=1`); `tests/golden/superfx-framebuffer.tsv` carries exactly these 24
changed hashes.

### Regression-lock test

`crates/rustysnes-core/tests/mid_scanline_hdma_baseline.rs` is a minimal, self-authored
hand-assembled 65C816 program (mirroring the pattern already used for rewind/run-ahead's
synthetic per-frame signal tests) that drives `$2100` (`INIDISP`, master brightness) via HDMA
mode 0, alternating full-brightness (backdrop renders white) and force-off (backdrop renders
black) at a single transition partway through the frame. No BG/OBJ layer is ever enabled, so
every pixel falls through to the backdrop color (`Ppu::layer_color`'s `!p.opaque` path) — this
isolates the exact compositor-vs-HDMA dot-timing mechanism with no tilemap/tileset setup at all,
at the cost of not being the specific "Air Strike Patrol BG3 scroll" scenario (a scroll-register
variant remains open future work if a title-accurate reproduction is ever wanted).

The test now **locks in the CORRECT hardware transition position** — `(last-white row 100,
first-black row 101)`, flipped from the previously-locked-in buggy `(99, 100)` as this fix's own
acceptance test predicted. A second test confirms the reproduction stays deterministic across
fresh runs.

## Hi-res (Modes 5/6) color-math precision — implemented (v0.7.0 "Resolution")

**Status: implemented.** True 512-px hi-res output (Modes 5/6, or pseudo-hires `SETINI` $2133
bit 3) is now real: `Ppu::compose_dac` emits two output columns per PPU pixel clock in hi-res,
mirroring ares' `PPU::DAC::run()`/`above()`/`below()` (`ref-proj/ares/ares/sfc/ppu/dac.cpp`) —

```text
*line++ = hires ? belowColor : aboveColor;  // the "even" 512-wide column
*line++ = aboveColor;                        // the "odd" 512-wide column (always the normal path)
```

`aboveColor` is exactly the pre-existing (256-wide) main-screen-composited-with-color-math pixel
— unchanged code, unchanged for every non-hires frame. `belowColor` is the *sub*-screen pixel
color-math'd a second time with the operand roles swapped — i.e. hi-res mode doesn't add new
*spatial* detail, it doubles the horizontal rate and interleaves "with color math" and "as if
reading one pixel earlier, blended differently" outputs to fake a smoother 512-wide gradient.

### The one-pixel-clock-delayed pipeline (the real complexity here)

`belowColor` for column `x` is gated by, and blends against, state ares computes during the
**previous** pixel clock's `above()` pass — not column `x`'s own state. Concretely (`x-1`'s
values, threaded across the row in `compose_dac`):

- the gate ("does the hires pass apply color math at all") = column `x-1`'s winning above-layer's
  per-layer color-math-enable bit AND the "below" color-window mask (ares `math.below.colorEnable`
  — exactly `Ppu::compose_dac`'s existing `math_layer && math_allowed` locals, read one column
  late);
- the blend operand standing in for "the main screen's own color" = column `x-1`'s **raw**
  above-layer color (`layer_color(above[x-1])`, pre-color-math — ares `math.above.color`), also
  one column late;
- the "blend mode"/halve gates (ares `math.blendMode`/`math.colorHalve`) are likewise recomputed
  each column from *that* column's own subscreen opacity, then consumed one column later — so
  `prev_below_opaque` (column `x-1`'s own `below[x-1].opaque`), not column `x`'s, drives them;
- at column 0, ares' `DAC::scanline()` init sets every one of these to their "no color math, raw
  color = backdrop" default — the documented hardware fact that **the first hires pixel of every
  scanline is transparent**. `compose_dac` seeds its `prev_*` locals to this same boundary before
  the loop starts, so column 0 falls out of the general formula rather than needing a special case.

This is a genuine one-pixel-clock hardware pipeline stage, not a translation artifact — verified
by reading `dac.cpp` directly rather than paraphrasing a summary, precisely because getting a
subtlety like this wrong produces a plausible-but-wrong implementation (the exact failure mode
external-oracle verification exists to catch).

### Framebuffer geometry: per-frame, not per-scanline

The framebuffer's row stride (256 or 512) is latched once, at each frame's first visible
scanline (`Ppu::frame_hires`, set from the live `Ppu::is_hires()` at `row == 0`), and held for
every remaining line of that frame — a **deliberate, documented per-frame-not-per-scanline
simplification**. `compose_dac` consults only this cached `frame_hires` value, never a live
per-scanline `is_hires()` re-check, so every line of a frame writes the same column geometry (one
column when `frame_hires` is false, two when true) for the frame's entire duration, with no
per-scanline fallback logic at all. Real hardware's hi-res-ness is technically a per-scanline DAC
property (`BGMODE`/`SETINI` could theoretically change mid-frame via HDMA), but modeling that
would be a materially bigger feature with no known motivating commercial title, mirroring this
project's existing posture on the (separately tracked, still-open) mid-scanline/HDMA-driven
register timing gap. A title that changes `BGMODE`/`SETINI` mid-frame gets the *first* visible
line's hi-res-ness applied uniformly across the whole frame, not a per-line-accurate mix — a
narrow, explicitly out-of-scope edge case, not a memory-safety concern.

### Verification

- **Mechanism**: derived directly from `ref-proj/ares/ares/sfc/ppu/dac.cpp`'s primary source (not
  a summary), including the one-pixel-clock-delay pipeline above.
- **Unit tests** (`crates/rustysnes-ppu/src/render.rs`, hand-constructed `Pixel` rows, bypassing
  full BG/tilemap setup to isolate the DAC mechanism precisely):
  `hires_first_column_of_scanline_is_always_transparent` (the column-0 boundary condition holds
  even when column 0's own pixel data would otherwise composite strongly) and
  `hires_below_color_depends_on_previous_column_not_its_own` (column 1's `belowColor` changes when
  *only* column 0's state changes, with column 1's own input held fixed across both runs; column
  1's `aboveColor` does not change — isolating the delay to exactly the below/even-column path).
- **Non-regression**: the full `--features test-roms` suite — every `undisbeliever` golden, every
  `*_oncart` suite including `sa1_oncart` — passes unchanged, since none of those ROMs enter
  hi-res mode; the non-hires code path is untouched.
- **Real-title validation: not achieved, honestly tracked as open.** Marvelous — Mouhitotsu no
  Takarajima (SA-1, `docs/STATUS.md`'s named hi-res-motivating title) was run for 1200 frames
  (20 s) from power-on and never entered hi-res (`Ppu::is_hires()` stayed `false` throughout) —
  either its hi-res content needs input/progress this headless run never provides, or the "relies
  on hi-res" claim in this doc's earlier draft was inferred rather than confirmed against this
  specific ROM. Bishoujo Janshi Suchie-Pai (the other named title) has no dump in the local
  corpus at all. An external-oracle screenshot comparison against `ares` (installed in this
  environment) was also attempted and abandoned — no working GUI display was available to drive
  it (Xvfb + `xdotool` window automation did not produce a usable window in this sandbox). Neither
  gap blocks landing: the mechanism is verified against primary source plus isolated unit tests,
  and zero currently-passing goldens regress. `tests/golden/sa1-framebuffer.tsv` is **not**
  re-blessed by this change (Marvelous's hash is unaffected, confirmed by `sa1_oncart` passing
  unchanged). Real-title hi-res validation remains genuinely open for whoever next has a working
  GUI environment or a confirmed hi-res-reaching save state for either named title.

## HD texture pack `TileTag` recording hook (`v1.3.0`, `hd-pack` feature)

**Status: fully implemented and wired end-to-end** — the PPU-side hook, the frontend's
`pack.toml` loader, the CPU compositor, the Settings pack selector, and (`v1.3.0` final
integration) invoking the compositor from the live wgpu present path. See "Not yet done" below
for the one honestly-tracked scope cut, and `crate::hd_pack`/`crate::hd_compositor` in
`rustysnes-frontend` for the frontend half.

Per `docs/adr/0010` and Mesen2's own NES HD Pack system, the direct architectural precedent, this
crate's only HD-texture-pack responsibility is to compute a **tile-identity hash** per composited
pixel — it never loads a pack, matches a hash, or composites a replacement texture (that stays
entirely frontend-side, preserving the `docs/adr/0004` determinism boundary: the core itself
never becomes pack-aware).

`Ppu::set_hd_pack_tagging(bool)` (off by default) gates a write-only side-buffer,
`Ppu::tile_tags()`, sized and indexed exactly like `Ppu::framebuffer()`. Each entry is a
`hdtag::TileTag { hash, hflip, vflip }`, where `hash` is `hdtag::hash_tile`'s output for the
8×8 tile that produced this specific output pixel's color, or `0` for the backdrop / when tagging
is off. `hash_tile` itself hashes into a fixed 642-byte stack buffer (`2 + 64*2 + 256*2`, the
statically-bounded maximum tile-word + palette size), never a heap `Vec` — it runs once per
tagged pixel on the rendering hot path, so it has to stay allocation-free like the rest of that
path. Populated by three small companion helpers in `render_bg`/`render_mode7`/`render_objects`,
each reusing address/bpp/palette values already resolved by that path's normal fetch (no parallel
address recomputation) to build the `tile_words`/`palette` slices `hash_tile` hashes:

- **BG**: `fetch_bg_pixel` already computes `tile_base` (the resolved 8×8 sub-tile's raw
  pre-flip VRAM word address — correct even for 16×16 tiles, since flip only changes which
  quadrant/pixel is selected, never the raw bytes at that address) and now also returns it plus
  `hflip`/`vflip`; `render_bg` derives the palette's CGRAM base (`pal_base + group_off`) from
  values it already has.
- **OBJ**: computed once per 8-pixel column (not per pixel) in `render_objects`'s `tx` loop,
  from the already-tile-aligned `tile_addr` and `pal_base = 128 + (palette << 4)`.
- **Mode 7**: `tile_base = (tile << 6) & 0x7fff` (the 64-word 8bpp block), palette is the full
  256-entry CGRAM (Mode 7 pixels index it directly, with no per-BG sub-palette group). Mode 7 has
  no per-tile flip bit (`m7_hflip`/`m7_vflip` mirror the whole affine transform, not one tile), so
  its tag is always `hflip: false, vflip: false`.

`compose_dac` writes whichever pixel's tag is actually **displayed** at each `tile_tags` index —
matching exactly which pixel's *color* it writes to `framebuffer` at that same index. In
non-hires output that's always the above-pixel's tag (one output column per pixel clock). In
hi-res output there are genuinely two distinct visible columns per pixel clock (the DAC's
"above"/"below" pass — see "Hi-res (Modes 5/6) color-math precision" above), so the even/left
column gets the below-pixel's tag and the odd/right column gets the above-pixel's tag, mirroring
the identical pairing `framebuffer[base + 2*x]`/`framebuffer[base + 2*x + 1]` already use. This is
not "recording the sub-screen's blend operand" — each hi-res column is its own real displayed
pixel with its own real source tile.

### Byte-identical-when-off

`Pixel` gained one new field, `#[cfg(feature = "hd-pack")] tag: hdtag::TileTag`, and `Ppu` gained
two: `hd_pack_tagging: bool` and `tile_tags: Box<[hdtag::TileTag]>` — all three compiled out
entirely (not just runtime-inert) when the `hd-pack` feature is off, so a non-`hd-pack` build's
compositor is byte-for-byte the pre-`v1.3.0` code. With the feature **on** but `hd_pack_tagging`
left at its default `false`, the same guarantee holds at the value level:
`hd_pack_tagging_toggle_does_not_alter_framebuffer_output` (`crates/rustysnes-ppu/src/render.rs`)
renders one scene twice — tagging off and on — and asserts `framebuffer()` is identical either
way. `hd_pack_tagging_off_leaves_tile_tags_untouched` confirms the side-buffer itself stays
all-default when tagging was never enabled; `turning_tagging_off_clears_stale_tags_from_a_prior_frame`
confirms that turning tagging off after a tagged frame clears every entry back to
`TileTag::default` rather than leaving stale tags an unwary caller could misread (`Ppu::
set_hd_pack_tagging(false)` fills `tile_tags` back to all-default as part of the toggle itself);
and `hd_pack_tagging_records_the_documented_hash_for_a_known_bg_tile` independently recomputes a
known tile's hash from raw VRAM/CGRAM bytes to prove the recorded value is the documented recipe.

Neither `hd_pack_tagging` nor `tile_tags` is part of `save_state`/`load_state` — the same
host/frontend-convenience carve-out already established for cheats, watchpoints, per-voice mutes,
and the port-2 peripheral selection.

### Not yet done

The frontend has a `pack.toml` loader (`crate::hd_pack::HdPack::load`, PNG decode, per-ROM
discovery), a pure CPU compositor (`crate::hd_compositor::composite`), a Settings pack selector,
`config.video.hd_pack_name` persistence, and (`v1.3.0` final integration) the compositor wired
into the live wgpu present path (`gfx.rs`'s streaming texture now grows to fit the composited
output — see `docs/frontend.md`). Not built: user-configurable upscale factor (fixed at 2x,
`docs/adr/0010`'s own documented v1 scope choice) and `emu-thread`-build compositing (that
build's framebuffer arrives via a lock-free handoff with no equivalent `TileTag` channel yet).
