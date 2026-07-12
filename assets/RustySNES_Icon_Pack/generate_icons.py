#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
generate_icons.py
=================

RustySNES consolidated cross-platform icon & image generator.

Ported from RustyNES's own `generate_icons.py` (same pipeline, same per-platform
size tables, same source-selection strategy) — ADAPTED here for RustySNES's own
art (an SNES cartridge "RS" monogram, not RustyNES's NES cartridge "RN") and,
unlike RustyNES's pack, generated from real vector (SVG) masters rather than
flat raster-only source renders. See `source/svg/` for the masters this script's
raster `source/*.png` inputs are themselves rendered from (`rsvg-convert`).

PURPOSE
-------
Take the authoritative square source renders from the RustySNES logo/icon set and
emit every icon and image size/format required for standard application packaging
on **Windows**, **macOS**, and **Linux**, plus a **web/PWA** favicon set and a
**branding** banner set. Everything lands in one directory tree ready to zip/ship.

SOURCE SELECTION STRATEGY
-------------------------
The set ships two square masters:

    * rustysnes_primary_app_icon_1x1.png        -> detailed cartridge + circuit traces
    * rustysnes_simplified_favicon_icon_1x1.png -> simplified cartridge (fewer traces)

The dense circuit-trace detail of the primary art turns to mush below ~48 px, so
this generator switches sources by target size:

    * size <= SIMPLIFIED_MAX (48 px)  -> simplified emblem (crisp at small sizes)
    * size >  SIMPLIFIED_MAX (>=64)   -> primary app icon (full detail legible)

``SIMPLIFIED_MAX`` is a single tunable constant below; set it to 0 to force the
primary art at every size, or very large to force the simplified art everywhere.

QUALITY NOTES
-------------
* All resamples are downscales (masters are 1254 px; the largest square target is
  1024 px), so there is no upscaling softness. Downsampling uses Lanczos.
* The smallest icons (<= UNSHARP_MAX) receive a mild UnsharpMask pass to recover
  edge contrast lost to aggressive downscaling. Radii/percent are conservative to
  avoid ringing halos.
* Every raster is emitted as RGBA for maximum container compatibility (ICNS in
  particular expects an alpha channel).

OUTPUT LAYOUT (relative to --out)
---------------------------------
    README.md                          consolidated documentation
    generate_icons.py                  this script
    source/                            the original supplied masters (inputs)
    source/svg/                        the true vector masters (opaque + transparent)
    master/                            hi-res derived masters (1024 icons + native banner)
    windows/RustySNES.ico              multi-resolution ICO (16..256)
    windows/RustySNES-small.ico        small-icon-optimized ICO (16,24,32,48; simplified art)
    windows/png/RustySNES-<n>.png      individual PNGs (16,20,24,32,40,48,64,128,256)
    macos/RustySNES.icns               Retina-complete ICNS
    macos/RustySNES.iconset/           Apple-named PNGs (for `iconutil`)
    linux/hicolor/<n>x<n>/apps/rustysnes.png  freedesktop icon theme tree (16..512)
    linux/png/rustysnes-<n>x<n>.png    flat convenience copies at the same sizes
    linux/rustysnes.png                512 px master (/usr/share/pixmaps)
    linux/rustysnes.desktop            example launcher entry
    web/favicon.ico + png/manifest     browser + PWA icons (incl. mstile)
    branding/rustysnes-banner-<w>x<h>.png  horizontal logo at 5 exact 3:1 widths

USAGE
-----
    python3 generate_icons.py \
        --primary  source/rustysnes_primary_app_icon_1x1.png \
        --favicon  source/rustysnes_simplified_favicon_icon_1x1.png \
        --banner   source/rustysnes_primary_logo_banner_3x1.png \
        --out      .

All arguments default to the in-pack ``source/`` masters and the pack root, so a
bare ``python3 generate_icons.py`` run inside the pack regenerates everything.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path

from PIL import Image, ImageFilter

# --------------------------------------------------------------------------- #
# Tunable configuration                                                        #
# --------------------------------------------------------------------------- #

# Any target <= this size uses the simplified emblem; above it uses the primary
# (full-detail) app icon. 48 keeps the dense circuit traces from muddying at the
# small sizes while preserving full detail from 64 px up.
SIMPLIFIED_MAX = 48

# Targets <= this size receive a mild sharpening pass after downscaling.
UNSHARP_MAX = 48

# Conservative UnsharpMask parameters (radius px, percent strength, threshold).
UNSHARP_PARAMS = dict(radius=0.8, percent=60, threshold=2)

# Windows .ico embedded resolutions (Explorer/taskbar/alt-tab/store all covered).
WINDOWS_ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256]

# Small-icon-optimized .ico (list-view / tray) — simplified art, small sizes only.
WINDOWS_SMALL_ICO_SIZES = [16, 24, 32, 48]

# Standalone Windows PNG exports (installers / store listings).
WINDOWS_PNG_SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256]

# freedesktop.org hicolor theme sizes (Linux desktops read these directly).
LINUX_HICOLOR_SIZES = [16, 22, 24, 32, 48, 64, 96, 128, 256, 512]

# Flat convenience copies (some tooling prefers a single directory of sizes).
LINUX_FLAT_SIZES = [16, 24, 32, 48, 64, 96, 128, 256, 512]

# macOS .iconset requires these exact (logical, scale) pairs. Physical pixel size
# is logical * scale; the '@2x' suffix marks Retina variants.
MACOS_ICONSET = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]

# Web / PWA favicon set.
WEB_ICO_SIZES = [16, 32, 48]
WEB_PNG = {
    "favicon-16x16.png": 16,
    "favicon-32x32.png": 32,
    "favicon-96x96.png": 96,
    "apple-touch-icon.png": 180,          # iOS home-screen (opaque, no rounding)
    "android-chrome-192x192.png": 192,     # PWA standard
    "android-chrome-512x512.png": 512,     # PWA maskable/large
    "mstile-150x150.png": 150,             # Windows/Edge pinned tile
}

# Branding banner widths — exact 3:1 sizes (the master is 2172x724 ≈ 3:1).
BANNER_WIDTHS = [900, 1200, 1500, 1800, 2172]


# --------------------------------------------------------------------------- #
# Core helpers                                                                 #
# --------------------------------------------------------------------------- #

def load_rgba(path: Path) -> Image.Image:
    """Open an image and normalize it to RGBA for consistent downstream ops."""
    return Image.open(path).convert("RGBA")


def render_square(src: Image.Image, size: int) -> Image.Image:
    """
    Produce a ``size``x``size`` icon from a square source.

    Downscales with Lanczos, then applies a gentle sharpening pass to the very
    small sizes so fine emblem detail survives. If the source is not square it is
    center-cropped to the largest inscribed square first (defensive guard).
    """
    w, h = src.size
    if w != h:
        side = min(w, h)
        left = (w - side) // 2
        top = (h - side) // 2
        src = src.crop((left, top, left + side, top + side))

    out = src.resize((size, size), Image.LANCZOS)
    if size <= UNSHARP_MAX:
        out = out.filter(ImageFilter.UnsharpMask(**UNSHARP_PARAMS))
    return out


def pick_source(size: int, primary: Image.Image, favicon: Image.Image) -> Image.Image:
    """Return the appropriate master for a given target size (see module docs)."""
    return favicon if size <= SIMPLIFIED_MAX else primary


def save_png(img: Image.Image, path: Path) -> None:
    """Write an optimized PNG, creating parent directories as needed."""
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, format="PNG", optimize=True)


def save_ico(frames: list[Image.Image], sizes: list[int], path: Path) -> None:
    """Write a multi-resolution ICO from per-size, hand-tuned frames."""
    path.parent.mkdir(parents=True, exist_ok=True)
    frames[-1].save(
        path,
        format="ICO",
        sizes=[(s, s) for s in sizes],
        append_images=frames[:-1],
    )


# --------------------------------------------------------------------------- #
# Per-platform builders                                                        #
# --------------------------------------------------------------------------- #

def build_windows(out: Path, primary: Image.Image, favicon: Image.Image) -> None:
    """Emit the multi-resolution .ico, a small-icon .ico, and standalone PNGs."""
    win = out / "windows"

    for s in WINDOWS_PNG_SIZES:
        save_png(render_square(pick_source(s, primary, favicon), s), win / "png" / f"RustySNES-{s}.png")

    # Primary app .ico (per-size source-selected frames).
    save_ico([render_square(pick_source(s, primary, favicon), s) for s in WINDOWS_ICO_SIZES],
             WINDOWS_ICO_SIZES, win / "RustySNES.ico")

    # Small-icon-optimized .ico: always the simplified emblem, small sizes only.
    save_ico([render_square(favicon, s) for s in WINDOWS_SMALL_ICO_SIZES],
             WINDOWS_SMALL_ICO_SIZES, win / "RustySNES-small.ico")


def build_macos(out: Path, primary: Image.Image, favicon: Image.Image) -> None:
    """Emit the Apple .iconset folder and a Retina-complete .icns file."""
    mac = out / "macos"
    iconset = mac / "RustySNES.iconset"

    for name, size in MACOS_ICONSET:
        save_png(render_square(pick_source(size, primary, favicon), size), iconset / name)

    icns_path = mac / "RustySNES.icns"
    try:
        import icnsutil  # type: ignore

        icns = icnsutil.IcnsFile()
        for name, _ in MACOS_ICONSET:
            icns.add_media(file=str(iconset / name))
        icns.write(str(icns_path))
    except Exception:
        # Native Pillow fallback (no icnsutil): build the same per-size,
        # source-selected frames the .iconset uses — `pick_source` returns the
        # simplified emblem for small sizes so the dense circuit traces don't turn
        # to mush — and hand them to Pillow's ICNS saver via `append_images` (base =
        # the largest frame) instead of letting it downscale the 1024 primary for
        # every embedded size.
        sizes = sorted({size for _, size in MACOS_ICONSET}, reverse=True)
        frames = [render_square(pick_source(sz, primary, favicon), sz) for sz in sizes]
        frames[0].save(icns_path, format="ICNS", append_images=frames[1:])


def build_linux(out: Path, primary: Image.Image, favicon: Image.Image) -> None:
    """Emit the freedesktop hicolor tree, flat copies, a pixmap, and a .desktop."""
    lin = out / "linux"

    for s in LINUX_HICOLOR_SIZES:
        save_png(render_square(pick_source(s, primary, favicon), s),
                 lin / "hicolor" / f"{s}x{s}" / "apps" / "rustysnes.png")

    for s in LINUX_FLAT_SIZES:
        save_png(render_square(pick_source(s, primary, favicon), s), lin / "png" / f"rustysnes-{s}x{s}.png")

    # 512 px master for the legacy /usr/share/pixmaps location.
    save_png(render_square(primary, 512), lin / "rustysnes.png")

    (lin / "rustysnes.desktop").write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=RustySNES\n"
        "GenericName=SNES Emulator\n"
        "Comment=Cycle-accurate Super Nintendo Entertainment System emulator\n"
        "Exec=rustysnes %f\n"
        "Icon=rustysnes\n"
        "Terminal=false\n"
        "Categories=Game;Emulator;\n"
        "Keywords=SNES;SuperNintendo;Nintendo;emulator;retro;\n"
        "MimeType=application/x-snes-rom;\n",
        encoding="utf-8",
    )


def build_web(out: Path, primary: Image.Image, favicon: Image.Image) -> None:
    """Emit browser favicons, Apple/Android touch icons, a tile, and a manifest."""
    web = out / "web"

    # Multi-size favicon.ico from the simplified emblem (small-size legibility).
    save_ico([render_square(favicon, s) for s in WEB_ICO_SIZES], WEB_ICO_SIZES, web / "favicon.ico")

    for name, size in WEB_PNG.items():
        save_png(render_square(pick_source(size, primary, favicon), size), web / name)

    (web / "site.webmanifest").write_text(
        "{\n"
        '  "name": "RustySNES",\n'
        '  "short_name": "RustySNES",\n'
        '  "icons": [\n'
        '    { "src": "favicon-16x16.png", "sizes": "16x16", "type": "image/png" },\n'
        '    { "src": "favicon-32x32.png", "sizes": "32x32", "type": "image/png" },\n'
        '    { "src": "favicon-96x96.png", "sizes": "96x96", "type": "image/png" },\n'
        '    { "src": "android-chrome-192x192.png", "sizes": "192x192", "type": "image/png" },\n'
        '    { "src": "android-chrome-512x512.png", "sizes": "512x512", "type": "image/png", "purpose": "any maskable" }\n'
        "  ],\n"
        '  "theme_color": "#04050f",\n'
        '  "background_color": "#04050f",\n'
        '  "display": "standalone"\n'
        "}\n",
        encoding="utf-8",
    )


def build_branding(out: Path, banner: Image.Image) -> None:
    """Emit the horizontal logo banner at five exact 3:1 widths."""
    brand = out / "branding"
    w, h = banner.size
    for target_w in BANNER_WIDTHS:
        target_h = round(h * (target_w / w))
        img = banner if target_w == w else banner.resize((target_w, target_h), Image.LANCZOS)
        save_png(img, brand / f"rustysnes-banner-{target_w}x{target_h}.png")


def build_master(out: Path, primary: Image.Image, favicon: Image.Image, banner: Image.Image) -> None:
    """Emit hi-res derived masters (handy for store art / further edits)."""
    mas = out / "master"
    save_png(render_square(primary, 1024), mas / "rustysnes-app-icon-master.png")
    save_png(render_square(favicon, 1024), mas / "rustysnes-small-icon-master.png")
    save_png(banner, mas / "rustysnes-logo-banner-master.png")


# --------------------------------------------------------------------------- #
# Entry point                                                                  #
# --------------------------------------------------------------------------- #

def main() -> None:
    here = Path(__file__).resolve().parent
    src = here / "source"
    ap = argparse.ArgumentParser(description="RustySNES consolidated icon generator")
    ap.add_argument("--primary", default=src / "rustysnes_primary_app_icon_1x1.png", type=Path)
    ap.add_argument("--favicon", default=src / "rustysnes_simplified_favicon_icon_1x1.png", type=Path)
    ap.add_argument("--banner", default=src / "rustysnes_primary_logo_banner_3x1.png", type=Path)
    ap.add_argument("--out", default=here, type=Path)
    args = ap.parse_args()

    primary = load_rgba(args.primary)
    favicon = load_rgba(args.favicon)
    banner = load_rgba(args.banner)

    out = args.out
    out.mkdir(parents=True, exist_ok=True)

    build_windows(out, primary, favicon)
    build_macos(out, primary, favicon)
    build_linux(out, primary, favicon)
    build_web(out, primary, favicon)
    build_branding(out, banner)
    build_master(out, primary, favicon, banner)

    total = sum(len(files) for _, _, files in os.walk(out))
    print(f"Done. {total} files under {out}/")


if __name__ == "__main__":
    main()
