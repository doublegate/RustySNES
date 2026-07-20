#!/usr/bin/env bash
# Cross-validate AccuracySNES against independent reference emulators.
#
# The in-repo harness proves the cart agrees with RustySNES, which on its own proves nothing —
# we wrote both. This script runs the same image on emulators we did not write. Agreement means
# the expected values are defensible; disagreement is a finding either way.
#
# Two independent lineages are covered:
#   * Mesen2   — via its headless --testrunner plus a Lua script that reads the results block.
#   * snes9x   — via a tiny libretro host that reads RETRO_MEMORY_SYSTEM_RAM directly.
#
# bsnes and ares are deliberately NOT here, for concrete reasons:
#   * bsnes' libretro target stubs out retro_get_memory_data entirely (returns nullptr), so there
#     is no way to read WRAM from it headlessly.
#   * ares has no headless mode and no memory-dump CLI at all.
#   * Separately, ares' wdc65816 core is a lineal copy of bsnes' (a full diff shows only type
#     renames), so even if both could be driven they would count as one opinion, not two.
# Those two are covered by source review instead — see docs/accuracysnes-research-dossier.md.
#
# Usage:  scripts/accuracysnes/crossval.sh
# Exit:   0 if every available reference agrees (zero failing tests), non-zero otherwise.

set -uo pipefail

# Where the reference-emulator clones live. Overridable so this can be run from a git worktree
# without symlinking `ref-proj` into it — a symlink there is machine-specific, and one was once
# committed by accident because .gitignore's `/ref-proj/` matches a directory but not a symlink.
#
# Resolved BEFORE the cd below, and against the caller's working directory, so that a relative
# `REF_PROJ=../ref-proj` means what the caller meant rather than being silently reinterpreted
# relative to the repository root.
if [[ -n ${REF_PROJ:-} && ${REF_PROJ} != /* ]]; then
    REF_PROJ=$PWD/$REF_PROJ
fi

cd "$(dirname "$0")/../.."

ROM=tests/roms/AccuracySNES/build/accuracysnes.sfc
HOST=${TMPDIR:-/tmp}/accuracysnes_lrcv
REF_PROJ=${REF_PROJ:-ref-proj}

MESEN=$REF_PROJ/Mesen2/bin/linux-x64/Release/linux-x64/publish/Mesen.dll
SNES9X=$REF_PROJ/snes9x/libretro/snes9x_libretro.so

if [[ ! -f $ROM ]]; then
    echo "error: $ROM not found — run 'cargo run -p accuracysnes-gen' first" >&2
    exit 1
fi

rc=0
ran=0

# --- Known reference divergences ------------------------------------------------------------------
#
# A reference emulator being wrong is a real possibility, and the gate has to be able to say so
# without either (a) silently lowering the bar or (b) forcing a well-evidenced test to be weakened
# to whatever all references happen to agree on. Each entry is one reference failing one test, with
# the citation for why the CART is right and the reference is wrong. Anything NOT listed here that
# fails is a genuine disagreement and still fails the gate.
#
# Format: "<reference>:<expected failing test count>  # <test> — <why>"
#
# snes9x, 1 test (B5.05): the multiply/divide latches power up as $4202=$FF, $4204/05=$FFFF.
#   Documented independently by anomie regs.txt r1157 ("$4202 holds the value $ff on power on") and
#   nocash fullsnes (which lists $4202-$4206 as "(FFh)" power-up); implemented by bsnes
#   (sfc/cpu/cpu.hpp), ares, and Mesen2 (AluMulDiv::Initialize). snes9x's S9xSoftResetPPU
#   blanket-memsets $4200-$42FF to zero and special-cases only $4201/$4213, so it reports 0 x N.
#
# snes9x, +1 test (A5.S17 "Sweep: WDM"): WDM ($42) is a reserved TWO-byte no-op costing 2 cycles /
#   2 bus accesses = 16 master clocks. undisbeliever's table gives $42 as 2 bytes / 2 cycles; the
#   WDC, GTE and VLSI instruction-operation tables agree; Mesen2 and RustySNES both measure it.
#   snes9x gets WDM's LENGTH right (it passes A6.08, the functional two-byte test) but not its
#   timing, which is a narrower and more interesting bug than it first looks.
SNES9X_KNOWN_FAILURES=2

# --- snes9x, via the libretro host --------------------------------------------------------------
if [[ -f $SNES9X ]]; then
    cc -O2 -o "$HOST" scripts/accuracysnes/libretro_crossval.c -ldl || exit 1
    echo "=== snes9x (libretro) ==="
    if "$HOST" "$SNES9X" "$ROM" 1200; then
        n=0
    else
        n=$?
    fi
    if [[ $n -eq $SNES9X_KNOWN_FAILURES ]]; then
        if [[ $n -eq 0 ]]; then
            echo "snes9x: OK"
        else
            echo "snes9x: OK ($n known divergence(s) — see SNES9X_KNOWN_FAILURES above)"
        fi
    else
        echo "snes9x: $n failing test(s), expected $SNES9X_KNOWN_FAILURES" >&2
        rc=1
    fi
    ran=$((ran + 1))
else
    echo "skip snes9x: build it with 'make -C ref-proj/snes9x/libretro'" >&2
fi

# --- Mesen2, via its headless test runner --------------------------------------------------------
if [[ -f $MESEN ]] && command -v dotnet >/dev/null; then
    echo "=== Mesen2 (headless test runner) ==="
    dotnet "$MESEN" --testrunner "$ROM" scripts/accuracysnes/mesen_crossval.lua --timeout=60 \
        >/dev/null 2>&1
    code=$?
    case $code in
        0)   echo "Mesen2: OK (0 failing tests)" ;;
        253) echo "Mesen2: results block never appeared (bad magic)" >&2; rc=1 ;;
        254) echo "Mesen2: timed out before the battery finished" >&2; rc=1 ;;
        *)   echo "Mesen2: $code failing test(s)" >&2; rc=1 ;;
    esac
    ran=$((ran + 1))
else
    echo "skip Mesen2: build it with 'make -C ref-proj/Mesen2'" >&2
fi

if [[ $ran -eq 0 ]]; then
    echo "error: no reference emulator available; nothing was cross-validated" >&2
    exit 2
fi

echo
if [[ $rc -eq 0 ]]; then
    echo "cross-validation: $ran reference(s) agree with the cart"
else
    echo "cross-validation: DISAGREEMENT — investigate before trusting the pass rate" >&2
fi
exit $rc
