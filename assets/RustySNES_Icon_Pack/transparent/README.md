# RustySNES Icon Pack — Transparent Variants

Transparent-background versions of the RustySNES logo and icon, for embedding on
light or arbitrary backgrounds (READMEs, websites, slides, overlays) where the
opaque dark art would show as a black box.

> These are **additive** — they do not replace anything in the parent pack. The
> opaque assets remain the correct choice for OS app icons (macOS `.icns` is
> full-bleed; iOS `apple-touch-icon` must be opaque; Android maskable needs an
> opaque safe zone).

## How they were made

Unlike RustyNES's own `make_transparent.py` — which has no vector source and so
has to *approximate* transparency with a corner flood-fill colour-key on its
opaque raster masters (a "best-effort" keying that can leave a faint dark fringe
where the emblem's outer glow fades into the background) — RustySNES ships real
**vector masters** (`source/svg/*_transparent.svg`: the same artwork with the
background `<rect>` simply omitted). `make_transparent.py` renders those directly
via `rsvg-convert` at each target size, so every file here is exact, antialiased,
glow-fringe-free alpha — no colour-key approximation needed.

Regenerate:

```bash
python3 make_transparent.py   # reads source/svg/*_transparent.svg, writes transparent/
```

(Requires the `rsvg-convert` CLI — `librsvg` — on `PATH`.)

## Contents

```text
transparent/
├── icon/
│   ├── rustysnes-icon-transparent-master.png   (1024, full-res)
│   ├── rustysnes-icon-transparent-<n>.png       (1024,512,256,128,64)
│   └── rustysnes-transparent.ico                (16..256, multi-res)
├── favicon/
│   └── rustysnes-favicon-transparent-<n>.png    (64,48,32,16; simplified emblem)
└── logo/
    └── rustysnes-logo-transparent-<w>x<h>.png   (2172×724, 1200×400, 600×200)
```

Verified by compositing over a checkerboard — the emblem, circuit traces,
wordmark, and tagline stay crisp with a clean alpha edge and no dark halo.
