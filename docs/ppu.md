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
  end-of-line-sampling approach causes for HDMA-driven per-line register changes). BG modes 0–7
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
  writes at the exact dot. **Correction (researched this pass, superseding the prior claim
  here):** the per-scanline compositor does NOT correctly handle even single-split-per-line HDMA
  effects — see "Mid-scanline/HDMA-driven register timing" below for the confirmed off-by-one-line
  bug and why it isn't fixed in this pass.

## Mid-scanline/HDMA-driven register timing — researched, confirmed, deferred (v0.5.0)

**Status: a genuine off-by-one-line compositor bug is confirmed against ares' per-pixel reference
model. Not fixed this pass — the fix touches the single hottest code path in the engine (every
frame, all 29 currently-passing `undisbeliever` goldens, every commercial screenshot test) and
needs dedicated empirical verification against the full golden suite, not a rushed change.**

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

### Why it isn't fixed this pass

The conceptually correct fix is small in shape — composite line `V` using the register state as
it stood *before* line `V`'s own HDMA run (dot 276), not after — but the safe way to land it
(e.g. moving `Ppu::end_of_scanline`'s render trigger earlier, or snapshotting the relevant
registers at a boundary the scheduler and PPU currently have no shared concept of) touches:

- The single hottest, most heavily-tested code path in the whole engine — every one of the 29
  currently-passing `undisbeliever` golden-framebuffer hashes (`docs/STATUS.md`'s accuracy
  dashboard), every `*_oncart`/commercial-screenshot test, and the `playable_smoke` gate all
  render through this exact function.
- A cross-crate coordination gap: `rustysnes-ppu` has no visibility into `rustysnes-core`'s HDMA
  scheduling today (by design — the PPU only sees the narrow `VideoBus` trait,
  `docs/architecture.md` §2) — the fix needs a clean way to communicate "render now, before this
  line's HDMA" without leaking DMA/HDMA concerns into the PPU crate's dependency graph.
- No dedicated committed test ROM demonstrating this specific per-line-scroll-split pattern
  exists yet in `tests/roms/` (an Air-Strike-Patrol-equivalent ROM, homebrew or a permissively
  licensed `undisbeliever`/Krom/PeterLemon test, would make the fix independently verifiable
  rather than validated only against the existing (possibly HDMA-timing-insensitive) goldens).

Landing an unverified change against this surface area is a real regression risk to a currently
green suite — mirrors `docs/scheduler.md`'s DRAM-refresh and open-bus-via-HDMA-latch treatment:
research and confirm the mechanism, but do not force an implementation through without the
verification infrastructure to prove it's actually correct.

### What a future investigation/fix needs

1. A committed test ROM (or a hand-assembled 65C816 program in a new `#[test]`, mirroring the
   pattern already used for rewind/run-ahead's synthetic per-frame signal tests) that exercises
   exactly this case: HDMA-driven per-line BG scroll changing every line, verified against a
   known-correct per-line expected value sequence.
2. A design for how the scheduler communicates "line `V`'s HDMA run has completed" to the PPU
   without giving the PPU direct DMA/HDMA knowledge — e.g. a new narrow `VideoBus` (or a new
   `Ppu` method the scheduler calls at the right dot) that triggers `end_of_scanline`'s render
   step early, at (or just before) `HDMA_RUN_DOT`, rather than at dot 340.
3. A full re-run of `cargo test --workspace --features test-roms` (the complete golden/oracle
   suite, not a subset) after the change, checked line-by-line against the *reason* for any
   diff — since some or all of the 29 current goldens may shift by exactly one scanline's worth
   of register state for any ROM that already uses HDMA-driven per-line effects, and each such
   shift needs confirming as "now correct" rather than blindly re-baselined.

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
