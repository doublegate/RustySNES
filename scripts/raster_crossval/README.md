# Mid-line-raster cross-check (T-CA-10 Phase 4c)

The external reference cross-check for the per-dot compositor's fetch-vs-composite cursor split
(`docs/adr/0014`). It renders a synthetic **mid-line raster** ROM in both RustySNES (the per-dot
compositor) and a headless **MesenCE** (the cycle-accurate reference) and compares the raster
boundary — the column at which a mid-scanline register write takes effect.

No committed corpus ROM does a mid-line register write (that is why 4c's fetch-ahead was untestable
against the corpus), so this authors one.

## The ROM (`raster.s`)

BG1 is a solid colour A over a blue backdrop B. An **HDMA** channel rewrites the target register to
its "BG1 fully shown" value at the *start* of every scanline; an **H-IRQ** at `RASTER_DOT` (no V
match, so it fires every line) writes it again *mid-line*. Each line therefore shows colour A from
column 0 up to the cursor at the write dot, then the post-write colour — a per-scanline raster split
whose boundary the framebuffer encodes.

Two variants (ca65 `-D`):

- **DRAW** (default): the mid-line write targets `TM` (`$212C`, a *composite* register) — boundary at
  the **draw cursor**. Post-boundary colour = the backdrop (B, blue).
- **FETCH** (`-DFETCH_RASTER`): the write targets `BGnNBA` (`$210B` char base, a *BG-data* register),
  switching BG1 to a second solid tile — boundary at the **fetch cursor**. Post-boundary colour C
  (green).

Build: `./build.sh [RASTER_DOT]` (default 128). Run the whole comparison: `./raster_crossval.sh`
(needs `ca65`/`ld65` and a MesenCE binary; set `MESEN=/path/to/Mesen`). The RustySNES side is the
`raster_crossval` harness test (`crates/rustysnes-test-harness/tests/raster_crossval.rs`,
self-skips without the built ROM); the MesenCE side is `mce_boundary.lua`.

## Why the OFFSET, not the absolute boundary

The absolute boundary confounds three things: when the H-IRQ is recognised, the ISR's CPU cycles to
the write, and the compositor's cursor position at the write dot. Only the third is the 4c subject.

The **FETCH-minus-DRAW offset** cancels the first two: both variants fire the same H-IRQ, and the
CPU-cycle difference between the two ISRs is identical on any accurate core, so
`offset = fetch_boundary − draw_boundary = BG_FETCH_AHEAD + (isr-cycle-diff)` is a purely
compositor-side quantity. It should agree between RustySNES and MesenCE regardless of any H-IRQ
timing difference.

## Findings (2026-07-25)

| dot | DRAW  r / mce | FETCH r / mce | offset r / mce |
|----:|--------------:|--------------:|---------------:|
| 100 |    124 / 138  |    150 / 160  |     26 / 22     |
| 128 |    152 / 166  |    179 / 192  |     27 / 26     |
| 160 |    184 / 188  |    210 / 208  |     26 / 20     |

- **The compositor's fetch-vs-draw offset agrees.** RustySNES is a stable **26–27** (= the 22-column
  `BG_FETCH_AHEAD` plus the ~5-cycle-longer FETCH ISR); MesenCE gives the same offset within its
  sub-dot-phase measurement noise (its boundary alternates ~2:1 between two adjacent columns row to
  row, so its modal offset scatters 20–26). The two-cursor split — BG-data at the fetch cursor,
  composite registers ~`BG_FETCH_AHEAD` columns behind at the draw cursor — is confirmed against the
  reference. This is what `BG_FETCH_AHEAD = 22` (itself read from MesenCE's
  `_fetchBgEnd`/`_drawEndX`) encodes.
- **The absolute boundary differs by up to ~14 dots** (e.g. draw 152 vs 166 at dot 128). This
  cancels in the offset and is *not* a compositor difference: RustySNES's H-IRQ→write latency is a
  clean constant (boundary = `RASTER_DOT + 24`), MesenCE's varies with the write dot. That is an
  **H-IRQ-recognition / ISR-latency modelling** difference (the CPU/IRQ side, separately validated by
  AccuracySNES Group B against Mesen2/snes9x), left as a follow-up — it is outside the compositor's
  fetch-ahead subject.
- Near the right edge (`RASTER_DOT` ≳ 200) the FETCH boundary saturates at 256, so use mid-range
  dots.
