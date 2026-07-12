# RustySNES Icon Pack

The single, consolidated icon & image set for **RustySNES** — ready-to-ship
assets for **Windows**, **macOS**, and **Linux**, plus a **web/PWA** favicon set
and a **branding** banner set. Everything is generated from the square/banner
master renders by `generate_icons.py`.

> **Provenance.** Modeled on RustyNES's own `RustyNES_Icon_Pack` (same generator
> shape, same per-platform size tables, same source-selection strategy) but with
> **original SNES-specific artwork** — not a recolor of RustyNES's NES-cartridge
> "RN" monogram. RustySNES ships real **vector masters** (`source/svg/`) that
> RustyNES's raster-only pack doesn't have, which is what makes this pack's
> `transparent/` variants exact renders rather than a colour-key approximation
> (see `transparent/README.md`).

## The art

A **landscape SNES cartridge** — light-gray shell, vertical rib-fin grips on
both edges, a black label window with the classic Nintendo-published purple
header bar, a lower grip-notch recess with two rivets — carrying an **"RS"
monogram** (cyan `R` / magenta `S`) in a pixel-font (Press Start 2P), wrapped in
bold glowing cyan circuit traces with magenta terminal nodes on a dark-navy
field (`#04050f`). A matching **"RustySNES"** wordmark banner
("Precise. Pure. Powerful."). Two square masters drive everything: the
**primary** app icon (full detail + circuit traces) and a **simplified**
emblem (cleaner, for small sizes). No third-party trademark text or logos —
the cartridge shape and label-bar color cue the real hardware without
reproducing Nintendo's actual branding.

The cartridge shape itself is deliberately **not** a recolor of RustyNES's NES
cartridge: a real North American SNES Game Pak is **landscape** (136mm wide ×
88mm tall, ≈1.55:1 — confirmed against reference photos of real cartridges),
with vertical rib fins on both side edges and a bottom-center grip notch, unlike
the NES cartridge's portrait shape with top-edge ZIF-socket ribbing.

## Source-selection strategy

| Target size | Source used            | Why                                                    |
|-------------|------------------------|--------------------------------------------------------|
| ≤ 48 px     | simplified emblem      | the dense circuit traces turn to mush below ~48 px     |
| ≥ 64 px     | primary app icon       | full detail stays legible                              |

All resamples are **downscales** (masters are 1254 px; the largest square target
is 1024 px), so there is no upscaling softness. Downsampling is Lanczos; sizes
≤ 48 px get a mild unsharp pass to recover edge detail. Every raster is RGBA.

---

## Directory layout

```text
RustySNES_Icon_Pack/
├── README.md
├── generate_icons.py                     ← the generator (regenerates everything)
├── make_transparent.py                   ← exact vector-rendered transparent variants
├── transparent/                          ← transparent-background logo/icon (see transparent/README.md)
├── source/                               ← the original supplied masters (inputs)
│   ├── rustysnes_primary_app_icon_1x1.png       (1254×1254)
│   ├── rustysnes_simplified_favicon_icon_1x1.png (1254×1254)
│   ├── rustysnes_primary_logo_banner_3x1.png    (2172×724)
│   └── svg/                                      ← the true vector masters (opaque + transparent)
├── master/                               ← hi-res derived masters
│   ├── rustysnes-app-icon-master.png            (1024)
│   ├── rustysnes-small-icon-master.png          (1024)
│   └── rustysnes-logo-banner-master.png         (native 2172×724)
├── windows/
│   ├── RustySNES.ico          ← multi-res: 16,20,24,32,40,48,64,128,256
│   ├── RustySNES-small.ico    ← small-icon-optimized: 16,24,32,48 (simplified art)
│   └── png/RustySNES-<n>.png  ← standalone PNGs (installers, store art)
├── macos/
│   ├── RustySNES.icns         ← Retina-complete container (16…1024, @1x/@2x)
│   └── RustySNES.iconset/     ← Apple-named PNGs (rebuild with iconutil)
├── linux/
│   ├── hicolor/<n>x<n>/apps/rustysnes.png   ← freedesktop theme tree (16…512, incl. 22 & 96)
│   ├── png/rustysnes-<n>x<n>.png            ← flat convenience copies
│   ├── rustysnes.png          ← 512 px master (/usr/share/pixmaps)
│   └── rustysnes.desktop      ← example launcher entry
├── web/
│   ├── favicon.ico           ← 16,32,48
│   ├── favicon-16x16.png · favicon-32x32.png · favicon-96x96.png
│   ├── apple-touch-icon.png  ← 180 px
│   ├── android-chrome-192x192.png · android-chrome-512x512.png
│   ├── mstile-150x150.png    ← Windows/Edge pinned tile
│   └── site.webmanifest
└── branding/
    └── rustysnes-banner-<w>x<h>.png   ← 900×300, 1200×400, 1500×500, 1800×600, 2172×724
```

---

## Platform usage

### Windows

Use `windows/RustySNES.ico` as the application/executable icon — it embeds every
resolution Explorer, the taskbar, and Alt-Tab request. `RustySNES-small.ico` is a
list/tray-optimized variant built from the simplified emblem. The standalone PNGs
suit installer UIs (NSIS/WiX/Inno) and Microsoft Store listings.

### macOS

`macos/RustySNES.icns` drops into an app bundle at `YourApp.app/Contents/Resources/`
with `CFBundleIconFile` pointing at it — it carries the full Retina set (16…1024).
To rebuild from the iconset on a Mac:

```bash
iconutil -c icns macos/RustySNES.iconset -o RustySNES.icns
```

> These are full-bleed square icons. For the Big Sur "squircle" with inset
> padding, apply that mask to the 1024 px master before rebuilding — the raw art
> is intentionally left unmasked so you can choose.

### Linux

```bash
sudo cp -r linux/hicolor/*   /usr/share/icons/hicolor/
sudo cp linux/rustysnes.png   /usr/share/pixmaps/
sudo cp linux/rustysnes.desktop /usr/share/applications/
sudo gtk-update-icon-cache /usr/share/icons/hicolor
sudo update-desktop-database
```

The `.desktop` references the icon by name (`Icon=rustysnes`) — the theme-correct
approach; edit `Exec=` to point at your binary. `linux/png/` holds flat copies for
tooling that prefers a single directory of sizes.

### Web / PWA

```html
<link rel="icon" href="/favicon.ico" sizes="any">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">
<meta name="msapplication-TileImage" content="/mstile-150x150.png">
<meta name="msapplication-TileColor" content="#04050f">
```

---

## Full size / format inventory

| Platform | File(s)                              | Sizes (px)                               | Format |
|----------|--------------------------------------|------------------------------------------|--------|
| Windows  | RustySNES.ico                        | 16,20,24,32,40,48,64,128,256 (one file)  | ICO    |
| Windows  | RustySNES-small.ico                  | 16,24,32,48 (one file)                   | ICO    |
| Windows  | png/RustySNES-*.png                  | 16,20,24,32,40,48,64,128,256             | PNG    |
| macOS    | RustySNES.icns                       | 16,32,128,256,512 (+@2x → 1024)          | ICNS   |
| macOS    | RustySNES.iconset/*.png              | 16…1024 (Apple-named)                    | PNG    |
| Linux    | hicolor/*/apps/rustysnes.png         | 16,22,24,32,48,64,96,128,256,512         | PNG    |
| Linux    | png/rustysnes-*.png                  | 16,24,32,48,64,96,128,256,512            | PNG    |
| Linux    | rustysnes.png                        | 512                                      | PNG    |
| Web      | favicon.ico                          | 16,32,48 (one file)                      | ICO    |
| Web      | favicon-*, chrome-*, apple-*, mstile | 16,32,96,150,180,192,512                 | PNG    |
| Branding | rustysnes-banner-*.png               | 900,1200,1500,1800,2172 wide (exact 3:1) | PNG    |

---

## Regenerating

All rasters are produced by `generate_icons.py` (requires **Pillow**; optionally
**icnsutil** for a fully-complete `.icns` — without it, Pillow writes a slightly
smaller container and the complete `.iconset/` is still emitted for `iconutil`).
A bare run inside the pack reads `source/` and rewrites everything:

```bash
python3 generate_icons.py
# or explicitly:
python3 generate_icons.py \
  --primary source/rustysnes_primary_app_icon_1x1.png \
  --favicon source/rustysnes_simplified_favicon_icon_1x1.png \
  --banner  source/rustysnes_primary_logo_banner_3x1.png \
  --out     .
```

`transparent/` is regenerated separately by `make_transparent.py` (requires the
`rsvg-convert` CLI — `librsvg`), which renders `source/svg/*_transparent.svg`
directly rather than color-keying the opaque rasters — see
[`transparent/README.md`](transparent/README.md).

If the source art itself changes, re-edit `source/svg/*.svg` (plain hand-authored
SVG, no design-tool project file) and re-render the three PNG masters before
running either generator:

```bash
rsvg-convert -w 1254 -h 1254 source/svg/rustysnes_primary_app_icon_1x1.svg -o source/rustysnes_primary_app_icon_1x1.png
rsvg-convert -w 1254 -h 1254 source/svg/rustysnes_simplified_favicon_icon_1x1.svg -o source/rustysnes_simplified_favicon_icon_1x1.png
rsvg-convert -w 2172 -h 724  source/svg/rustysnes_primary_logo_banner_3x1.svg -o source/rustysnes_primary_logo_banner_3x1.png
```

Tunables at the top of `generate_icons.py`: `SIMPLIFIED_MAX` (the emblem/primary
crossover), `UNSHARP_MAX`/`UNSHARP_PARAMS` (small-size sharpening), and the
per-platform size lists.
