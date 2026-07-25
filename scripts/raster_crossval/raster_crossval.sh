#!/usr/bin/env bash
# Mid-line-raster cross-check driver (T-CA-10 Phase 4c, docs/adr/0014).
#
# For a set of write dots, builds the DRAW (composite TM) and FETCH (BG-data BGnNBA) raster ROM
# variants, renders each in RustySNES (the raster_crossval harness) and MesenCE (mce_boundary.lua),
# and prints the boundary columns + the FETCH-minus-DRAW offset for each. The OFFSET is the
# latency-independent, compositor-specific quantity: it should agree between the two emulators (the
# fetch cursor runs ~BG_FETCH_AHEAD columns ahead of the draw cursor) even though the ABSOLUTE
# boundary differs by the emulators' H-IRQ/ISR-latency modelling.
#
# Requires: ca65/ld65 on PATH, and a MesenCE binary. Set MESEN=/path/to/Mesen (default: the built
# ref-proj/MesenCE binary). Run from anywhere; paths are resolved relative to the repo.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
MESEN="${MESEN:-$REPO/ref-proj/MesenCE/bin/linux-x64/Release/Mesen}"
DOTS="${DOTS:-100 128 160}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [[ ! -x "$MESEN" ]]; then
    echo "MesenCE not found at $MESEN (set MESEN=...); RustySNES-only run." >&2
fi

rusty_boundary() { # prints the RustySNES boundary for the current raster.sfc
    # The harness prints `RASTER_BOUNDARY ... boundary=N` on stderr (eprintln!), so merge it.
    cargo test --manifest-path "$REPO/Cargo.toml" -p rustysnes-test-harness --features test-roms \
        --test raster_crossval -- --nocapture 2>&1 \
        | grep -oE 'boundary=[0-9]+' | grep -oE '[0-9]+' || echo "-"
}
mce_boundary() { # prints the MesenCE modal boundary for the current raster.sfc
    [[ -x "$MESEN" ]] || { echo "-"; return; }
    MCE_RESULT="$TMP/mb.txt" MCE_FRAMES=16 SDL_VIDEODRIVER=offscreen SDL_AUDIODRIVER=dummy \
        timeout 120 "$MESEN" --testRunner "$HERE/mce_boundary.lua" "$HERE/raster.sfc" >/dev/null 2>&1 || true
    grep -oE 'modal=[0-9]+' "$TMP/mb.txt" 2>/dev/null | grep -oE '[0-9]+' || echo "-"
}

printf '%-6s | %-16s | %-16s | %s\n' dot 'DRAW  r / mce' 'FETCH r / mce' 'offset r / mce'
printf -- '-------+------------------+------------------+----------------\n'
for dot in $DOTS; do
    bash "$HERE/build.sh" "$dot" >/dev/null 2>&1
    dr=$(rusty_boundary); dm=$(mce_boundary)
    ca65 --cpu 65816 -D RASTER_DOT="$dot" -DFETCH_RASTER -o "$HERE/raster.o" "$HERE/raster.s" 2>/dev/null
    ld65 -C "$HERE/raster.cfg" -o "$HERE/raster.sfc" "$HERE/raster.o" 2>/dev/null
    fr=$(rusty_boundary); fm=$(mce_boundary)
    or=$((fr - dr)); om="-"; [[ "$dm" != "-" && "$fm" != "-" ]] && om=$((fm - dm))
    printf '%-6s | %-16s | %-16s | %s\n' "$dot" "$dr / $dm" "$fr / $fm" "$or / $om"
done
# Restore the default build.
bash "$HERE/build.sh" 128 >/dev/null 2>&1
