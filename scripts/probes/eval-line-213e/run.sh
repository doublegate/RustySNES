#!/usr/bin/env bash
#
# $213E over-flag eval-line probe (T-CA-10). Determines, empirically, which display line the sprite
# range/time over-flags belong to, so the per-dot compositor's incremental range-eval cursor
# (over-flag dot-timing) can be anchored to match MesenCE rather than guessed.
#
# The ROM (`probe.s`) sets up 40 8x8 sprites all at Y=100 and samples STAT77 ($213E) into WRAM
# $7E:1000+scanline every scanline via an H-IRQ. Both emulators then read that array and report the
# first scanline whose range-over (bit 6) / time-over (bit 7) reads set — apples-to-apples.
#
# Baseline finding (2026-07): MesenCE = scanline 100; RustySNES per-dot = 101 (one line late); batch
# = 102 (two late). So the incremental cursor must evaluate scan_y = self.v (the NEXT display line's
# sprites, one line ahead of the paint's scan_y = self.v-1). After implementing it, this probe should
# read 100 for the per-dot build.
#
# Usage: scripts/probes/eval-line-213e/run.sh   (from the repo root; REF_PROJ overrides ref-proj)

set -euo pipefail
cd "$(dirname "$0")"
ROOT=$(git rev-parse --show-toplevel)
REF_PROJ=${REF_PROJ:-$ROOT/ref-proj}
MESEN="$REF_PROJ/MesenCE/bin/linux-x64/Release/Mesen"

if ! command -v ca65 >/dev/null || ! command -v ld65 >/dev/null; then
    echo "need ca65/ld65 (cc65) on PATH to build the probe ROM" >&2
    exit 2
fi

echo "=== building probe ROM ==="
ca65 --cpu 65816 -o probe.o probe.s
ld65 -C probe.cfg -o probe.sfc probe.o
echo "built $(stat -c%s probe.sfc) bytes"

echo "=== RustySNES (batch) ==="
cargo run -q -p rustysnes-test-harness --bin probe_213e --manifest-path "$ROOT/Cargo.toml" -- probe.sfc
echo "=== RustySNES (per-dot compositor) ==="
cargo run -q -p rustysnes-test-harness --features per-dot-compositor --bin probe_213e --manifest-path "$ROOT/Cargo.toml" -- probe.sfc

if [[ -x $MESEN ]]; then
    echo "=== MesenCE (oracle) ==="
    SDL_VIDEODRIVER=offscreen SDL_AUDIODRIVER=dummy timeout 60 "$MESEN" --testRunner probe_mesen.lua probe.sfc 2>&1 | grep -E "^MESEN" || true
else
    echo "=== MesenCE absent ($MESEN) — skipping oracle side ==="
fi
