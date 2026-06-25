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

## Open questions

- Exact long-dot placement within the 1364-clock line under interlace transitions — resolve
  against the test ROMs (`ref-docs/2026-06-24-ppu.md` "Note on a flagged discrepancy").
