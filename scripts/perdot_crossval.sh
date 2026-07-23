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
# KNOWN ORACLE CAVEAT — master-brightness formula. MesenCE (SnesPpu.cpp:1453 ApplyBrightness) scales a
# channel by `value * ScreenBrightness / 15`; RustySNES (render.rs apply_brightness) uses
# `value * (N + 1) / 16`, which is what fullsnes documents (ref-docs/fullsnes/30-ppu.md:112, "N=1..15:
# Brightness*(N+1)/16"). The two agree only at N=15 (full) and N=0 (black) and differ by up to one step
# per channel for every intermediate brightness. So any ROM that renders a mid-scale or ramped INIDISP
# brightness — notably the `hdma-*-2100-glitch` ROMs, which HDMA-write $2100 down the screen — DIFFs by a
# ±1-per-channel colour SET here where RustySNES is the hardware-correct side. That is a colour-value
# formula gap, NOT a per-dot timing gap: it cannot be closed by compositor work and must not be read as a
# per-dot accuracy regression or "fixed" by matching MesenCE. Treat such DIFFs as expected.
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

# A single positive-integer frame-count contract: MesenCE captures after `TARGET` endFrame callbacks
# and RustySNES renders exactly `FRAMES` frames, so a non-positive/malformed value would make the two
# sides sample different frames and manufacture a false diff. Reject it up front (both capture tools
# validate the same way).
FRAMES=${MCE_FRAMES:-60}
# Positive integer AND within the u32 ceiling that perdot_dump's frame count parses to — otherwise the
# Rust side rejects a >u32 value as out of range and the driver would mis-report it as a skipped capture
# while MesenCE happily captured. 10 digits is the max width of 4294967295; a longer string, or a
# same-width string lexicographically greater, is over the ceiling.
if [[ ! $FRAMES =~ ^[1-9][0-9]*$ ]] \
    || (( ${#FRAMES} > 10 )) \
    || { (( ${#FRAMES} == 10 )) && [[ $FRAMES > "4294967295" ]]; }; then
    echo "perdot_crossval: MCE_FRAMES must be a positive integer <= 4294967295 (u32), got '$FRAMES'" >&2
    exit 2
fi

# mktemp -d, not a $$-predictable path: `mkdir -p` would accept a pre-created dir, letting a local
# attacker who guesses the PID pre-plant mce.txt/rusty.txt symlinks that the later writes follow (CWE-377).
TMP=$(mktemp -d "${TMPDIR:-/tmp}/perdot_crossval.XXXXXX")
trap 'rm -rf "$TMP"' EXIT

if [[ ! -x $MESENCE ]]; then
    echo "skip perdot cross-check: MesenCE binary absent ($MESENCE)" >&2
    echo "  build it with: (cd $REF_PROJ/MesenCE && make -j)" >&2
    exit 0
fi

# ROM list: args, or the whole committed undisbeliever corpus (shell glob, not ls-parsing). nullglob so
# an empty corpus yields an empty array rather than the literal unexpanded pattern being run as a "ROM".
roms=("$@")
if [[ ${#roms[@]} -eq 0 ]]; then
    shopt -s nullglob
    roms=(tests/roms/undisbeliever/*.sfc)
    shopt -u nullglob
fi
if [[ ${#roms[@]} -eq 0 ]]; then
    echo "perdot_crossval: no ROMs to check — the corpus tests/roms/undisbeliever/*.sfc is empty and no ROM args were given" >&2
    exit 2
fi

echo "=== building perdot_dump (--features per-dot-compositor) ==="
cargo build -q -p rustysnes-test-harness --features per-dot-compositor --bin perdot_dump

# Extract just the sorted set of canonical colour values (drop the :count) from a PERDOT line.
colorset() { sed -n 's/.*colors=//p' "$1" | tr ',' '\n' | cut -d: -f1 | sort -u; }

diffs=0
skipped=0
for rom in "${roms[@]}"; do
    name=$(basename "$rom" .sfc)

    # Remove any prior ROM's captures first: Line 59 tolerates a MesenCE failure, and without this a
    # stale mce.txt would survive and be compared against the current RustySNES capture — a false MATCH.
    rm -f "$TMP/mce.txt" "$TMP/rusty.txt"

    MCE_RESULT="$TMP/mce.txt" MCE_FRAMES="$FRAMES" \
        SDL_VIDEODRIVER=offscreen SDL_AUDIODRIVER=dummy \
        timeout 60 "$MESENCE" --testRunner scripts/perdot_capture.lua "$rom" >/dev/null 2>&1 || true

    cargo run -q -p rustysnes-test-harness --features per-dot-compositor --bin perdot_dump -- \
        "$rom" "$FRAMES" >"$TMP/rusty.txt" 2>/dev/null || true

    # A genuinely empty capture (either side failed on THIS ROM) is now unambiguous — no stale file can
    # masquerade as a match. Count and report it rather than swallowing it silently.
    if [[ ! -s $TMP/mce.txt || ! -s $TMP/rusty.txt ]]; then
        skipped=$((skipped + 1))
        printf '  %-40s SKIP (capture failed)\n' "$name"
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

echo "=== $diffs ROM(s) DIFF from MesenCE, $skipped skipped (capture failed) ==="
exit "$diffs"
