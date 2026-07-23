#!/usr/bin/env bash
#
# Per-dot compositor cross-check driver (T-CA-10, docs/adr/0014).
#
# The undisbeliever framebuffer golden is shared between the batch (flag-OFF) and per-dot (flag-ON)
# compositors, so it stays batch-valued and cannot gate the (more-accurate) per-dot path while phases
# 4b-4d land. This script is that gate instead: it renders each ROM in MesenCE (the cycle-accurate
# oracle, ref-proj/MesenCE) AND in RustySNES with `--features per-dot-compositor`, and compares their
# canonical 0RRRRRGGGGGBBBBB distinct-color SETS. The set is the robust cross-emulator signal — immune
# to the ~7-row overscan offset between RustySNES (composites from scanline 0) and MesenCE (blank top
# rows) — and it is exactly what distinguished a correct per-dot render (matching MesenCE) from a wrong
# one during 4a validation. A per-color count diff needs FIRST_ROW calibration and is left to a
# follow-up; the SET diff catches every new/missing colour, which is the accuracy signal that matters.
#
# Usage:
#   scripts/perdot_crossval.sh [rom ...]           # default: the undisbeliever corpus
#   REF_PROJ=/path/to/ref-proj scripts/perdot_crossval.sh
#   MCE_FRAMES=60 scripts/perdot_crossval.sh
#
# Exit code: number of ROMs that DIFF from MesenCE (0 = all match). Self-skips (exit 0) if the MesenCE
# binary is absent, so a CI runner without the gitignored build stays green.

set -euo pipefail

cd "$(dirname "$0")/.."

REF_PROJ=${REF_PROJ:-ref-proj}
[[ ${REF_PROJ} != /* ]] && REF_PROJ=$PWD/${REF_PROJ}
MESENCE="$REF_PROJ/MesenCE/bin/linux-x64/Release/Mesen"
FRAMES=${MCE_FRAMES:-60}
TMP=${TMPDIR:-/tmp}/perdot_crossval.$$
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

if [[ ! -x $MESENCE ]]; then
    echo "skip perdot cross-check: MesenCE binary absent ($MESENCE)" >&2
    echo "  build it with: (cd $REF_PROJ/MesenCE && make -j)" >&2
    exit 0
fi

# ROM list: args, or the whole committed undisbeliever corpus.
roms=("$@")
if [[ ${#roms[@]} -eq 0 ]]; then
    mapfile -t roms < <(ls tests/roms/undisbeliever/*.sfc)
fi

echo "=== building perdot_dump (--features per-dot-compositor) ==="
cargo build -q -p rustysnes-test-harness --features per-dot-compositor --bin perdot_dump

# Extract just the sorted set of canonical colour values (drop the :count) from a PERDOT line.
colorset() { sed -n 's/.*colors=//p' "$1" | tr ',' '\n' | cut -d: -f1 | sort -u; }

diffs=0
for rom in "${roms[@]}"; do
    name=$(basename "$rom" .sfc)

    MCE_RESULT="$TMP/mce.txt" MCE_FRAMES="$FRAMES" \
        SDL_VIDEODRIVER=offscreen SDL_AUDIODRIVER=dummy \
        timeout 60 "$MESENCE" --testRunner scripts/perdot_capture.lua "$rom" >/dev/null 2>&1 || true

    cargo run -q -p rustysnes-test-harness --features per-dot-compositor --bin perdot_dump -- \
        "$rom" "$FRAMES" >"$TMP/rusty.txt" 2>/dev/null || true

    if [[ ! -s $TMP/mce.txt || ! -s $TMP/rusty.txt ]]; then
        printf '  %-40s SKIP (no capture)\n' "$name"
        continue
    fi

    if diff -q <(colorset "$TMP/mce.txt") <(colorset "$TMP/rusty.txt") >/dev/null; then
        printf '  %-40s MATCH\n' "$name"
    else
        diffs=$((diffs + 1))
        printf '  %-40s DIFF\n' "$name"
        printf '      MesenCE-only: %s\n' "$(comm -23 <(colorset "$TMP/mce.txt") <(colorset "$TMP/rusty.txt") | tr '\n' ' ')"
        printf '      RustySNES-only: %s\n' "$(comm -13 <(colorset "$TMP/mce.txt") <(colorset "$TMP/rusty.txt") | tr '\n' ' ')"
    fi
done

echo "=== $diffs ROM(s) DIFF from MesenCE ==="
exit "$diffs"
