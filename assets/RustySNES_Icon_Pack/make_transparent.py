#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
make_transparent.py
===================

Transparent-background variants of the RustySNES logo + icon.

Unlike RustyNES's own `make_transparent.py` — which has no vector source and so
has to *approximate* transparency with a corner flood-fill colour-key on its
opaque raster masters (a "best-effort" keying that leaves a faint dark fringe
where the emblem's outer glow fades into the background) — RustySNES ships real
vector masters (`source/svg/*_transparent.svg`: the same artwork with the
background `<rect>` simply omitted). This script renders those directly via
`rsvg-convert` at each target size, so every transparent output here is exact,
antialiased, glow-fringe-free alpha — no colour-key approximation needed.

Requires the `rsvg-convert` CLI (librsvg) on PATH.

Usage
-----
    python3 make_transparent.py                 # defaults: read source/svg/, write transparent/
"""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

# Square-icon (primary art) transparent sizes.
ICON_SIZES = [1024, 512, 256, 128, 64]
# Simplified-emblem favicon transparent sizes.
FAVICON_SIZES = [64, 48, 32, 16]
# Logo/banner transparent widths (aspect preserved, banner master is 2172x724).
LOGO_WIDTHS = [2172, 1200, 600]
BANNER_ASPECT = 724 / 2172

# Windows .ico frames for the transparent icon.
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]


def render_svg(svg: Path, out: Path, width: int, height: int | None = None) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    args = ["rsvg-convert", "-w", str(width)]
    if height is not None:
        args += ["-h", str(height)]
    args += [str(svg), "-o", str(out)]
    subprocess.run(args, check=True)


def main() -> None:
    here = Path(__file__).resolve().parent
    svg = here / "source" / "svg"
    ap = argparse.ArgumentParser(description="RustySNES transparent-variant generator")
    ap.add_argument("--primary", default=svg / "rustysnes_primary_app_icon_1x1_transparent.svg", type=Path)
    ap.add_argument("--favicon", default=svg / "rustysnes_simplified_favicon_icon_1x1_transparent.svg", type=Path)
    ap.add_argument("--banner", default=svg / "rustysnes_primary_logo_banner_3x1_transparent.svg", type=Path)
    ap.add_argument("--out", default=here / "transparent", type=Path)
    args = ap.parse_args()

    out = args.out

    # --- Icon (primary art) --------------------------------------------------- #
    render_svg(args.primary, out / "icon" / "rustysnes-icon-transparent-master.png", 1024)
    for s in ICON_SIZES:
        render_svg(args.primary, out / "icon" / f"rustysnes-icon-transparent-{s}.png", s)

    # A transparent multi-res .ico (Pillow assembles the pre-rendered PNG frames).
    from PIL import Image

    frames = [Image.open(out / "icon" / f"rustysnes-icon-transparent-{s}.png").convert("RGBA")
              if s in ICON_SIZES else None for s in ICO_SIZES]
    # Render any ICO_SIZES not already covered by ICON_SIZES directly.
    for i, s in enumerate(ICO_SIZES):
        if frames[i] is None:
            tmp = out / "icon" / f"_tmp_{s}.png"
            render_svg(args.primary, tmp, s)
            frames[i] = Image.open(tmp).convert("RGBA")
            tmp.unlink()
    (out / "icon").mkdir(parents=True, exist_ok=True)
    frames[-1].save(out / "icon" / "rustysnes-transparent.ico", format="ICO",
                     sizes=[(s, s) for s in ICO_SIZES], append_images=frames[:-1])

    # --- Favicon (simplified emblem) ------------------------------------------ #
    for s in FAVICON_SIZES:
        render_svg(args.favicon, out / "favicon" / f"rustysnes-favicon-transparent-{s}.png", s)

    # --- Logo / banner ---------------------------------------------------------- #
    for tw in LOGO_WIDTHS:
        th = round(tw * BANNER_ASPECT)
        render_svg(args.banner, out / "logo" / f"rustysnes-logo-transparent-{tw}x{th}.png", tw, th)

    total = sum(1 for _ in out.rglob("*") if _.is_file())
    print(f"Done. {total} transparent files under {out}/ (rendered directly from vector source, no colour-key)")


if __name__ == "__main__":
    main()
