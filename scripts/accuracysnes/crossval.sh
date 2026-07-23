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
# snes9x, +1 test (E3.10 "TEST gates RAM writes"): the TEST register is not implemented at all.
#   `apu/bapu/smp/memory.cpp`'s `SMP::mmio_write` has no `case 0xf0` — writes to it fall through the
#   switch and are discarded, so bit 1 (the RAM write enable) has no effect and stores land in APU
#   RAM regardless. Documented by the SNESdev Wiki and nocash fullsnes; implemented by Mesen2 and
#   RustySNES, which agree with the cart. No game depends on it — which is exactly why it is the
#   kind of register an emulator leaves out and a test ROM finds.
# snes9x, +1 test (A2.10 "PEI does not page-wrap"): PEI's POINTER FETCH page-wraps at E=1 with
#   DL = $00. `DirectIndirectE1` in `cpuaddr.h` reads the pointer with
#   `Registers.DL ? WRAP_BANK : WRAP_PAGE`, applying the old-instruction direct-page wrap rule, and
#   PEI shares that helper with the genuinely old `(d),Y` modes. snes9x's own comment in `OpD4E1`
#   ("PEI is a new instruction, and so doesn't respect the emu-mode stack bounds") shows it
#   distinguishes new-instruction behaviour for the STACK but not for the fetch. Mesen2 and
#   RustySNES both agree with the cart; the WDC datasheet and superfamicom.org's new-instruction
#   list are the citation.
# snes9x, +1 test (E3.08 "TEST bit 0 halts timers"): the same missing `case 0xf0` as E3.10 above,
#   showing up one bit over. With the TEST register unimplemented, bit 0 cannot halt the timers, so
#   snes9x's timer 0 advances over an interval where the cart, Mesen2 and RustySNES all report it
#   frozen. ares implements it explicitly (`sfc/smp/io.cpp`: `io.timersDisable = data.bit(0)`,
#   followed by `synchronizeStage1()` on all three timers); fullsnes documents bits 0 and 3 as the
#   timer controls. Two tests failing on one missing switch case is the expected shape of this —
#   the register gates several unrelated behaviours, and each one is its own assertion.
# snes9x, +1 test (B4.13 "Timer range is 9-bit"): an H-IRQ fires with HTIME = 400, a position no
#   scanline reaches. The register itself is stored correctly to nine bits (`ppu.cpp`, the $4207 and
#   $4208 cases each keep their half); the defect is downstream, in scheduling. snes9x converts the
#   beam position into an absolute cycle within the line -- `HTimerPosition = IRQHBeamPos *
#   ONE_DOT_CYCLE + IRQTriggerCycles` -- and never asks whether the result exceeds the line length.
#   For HTIME = 400 it lands at 1600 cycles against an H_Max of 1364, so instead of being rejected
#   as unreachable it is carried into the following line and fires there, at about dot 59. That is
#   the "reduced modulo the line length" wrong answer the test's own failure message names. Mesen2
#   and RustySNES both agree with the cart; fullsnes is the citation for the 0-339 range.
# snes9x, +1 test (F1.11 "Latch corrupts auto-read"): holding $4016 bit 0 high across the automatic
#   read leaves $4218-$421F correct there. On hardware the read clocks the ports' shift registers,
#   and while the latch line is high those registers reload rather than shift, so all sixteen clocks
#   return the first bit and the result is uniform. snes9x fills the auto-read result from its
#   latched pad state without consulting the strobe, so a driver that strobes $4016 during vblank
#   corrupts the results on hardware and not there — the more dangerous direction, since code that
#   works under snes9x can be silently wrong on a console. Mesen2 models it (its result reads $FFFF
#   with B held); RustySNES did not either until this row was written, and now does.
# snes9x, +1 test (C1.08 "OAM addr in render"): during active display the renderer drives the OAM
#   address, so a $2138 read returns the sprite-evaluation address (eval_index<<2, below the
#   programmed $80 at the controlled low dot this test reads at), not the CPU's OAMADDR. Mesen2
#   models it (`SnesPpu::GetOamAddress` returns `_oamEvaluationIndex << 2` during rendering) and
#   RustySNES does under the per-dot compositor; snes9x's OAMDATAREAD path uses the CPU OAMADDR
#   regardless of the rendering state, so it reads back the programmed $80 and fails the assertion.
#   Documented by nocash fullsnes and the SNESdev Wiki (the renderer owns the OAM address during
#   active display). The read is taken at a controlled dot (an H+V IRQ + SEI/WAI sync), so the
#   verdict is region-independent — snes9x fails it identically on the NTSC and PAL images.
# snes9x, +1 test (C3.12 "CGRAM taken in render"): the CGRAM sibling of C1.08. A $2122 write during
#   active display commits to the colour the PPU is drawing (its internal CGRAM address), not the CPU
#   CGADD — with every layer off that colour is the backdrop, index 0. Mesen2 models it (writes use
#   InternalCgramAddress when !CanAccessCgram); snes9x uses the programmed CGADD regardless of the
#   rendering state, so the write lands the wrong colour and the test fails. Documented by nocash
#   fullsnes and the SNESdev Wiki. Read at a controlled dot (H+V IRQ + SEI/WAI), region-independent.
SNES9X_KNOWN_FAILURES=9

# --- snes9x, via the libretro host --------------------------------------------------------------
if [[ -f $SNES9X ]]; then
    cc -O2 -o "$HOST" scripts/accuracysnes/libretro_crossval.c -ldl || exit 1
    echo "=== snes9x (libretro) ==="
    if "$HOST" "$SNES9X" "$ROM" 2000; then
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
        --snes.port2.type=SnesController >/dev/null 2>&1
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

# --- the PAL image ------------------------------------------------------------------------------
#
# The same battery at PAL timing. The two images differ in one header byte, so this is the cheapest
# possible isolation of the region: anything that changes between them is the region and can be
# nothing else. Both references must reach the same failing-test count on both images -- the region
# pair (B2.04/B2.05) swaps which of the two SKIPs, and neither ever fails.
PAL_ROM=tests/roms/AccuracySNES/build/accuracysnes-pal.sfc

if [[ -f $PAL_ROM ]]; then
    if [[ -f $SNES9X ]]; then
        echo "=== snes9x (PAL image) ==="
        if "$HOST" "$SNES9X" "$PAL_ROM" 2000 >/dev/null 2>&1; then n=0; else n=$?; fi
        if [[ $n -eq $SNES9X_KNOWN_FAILURES ]]; then
            echo "snes9x PAL: OK ($n known divergence(s))"
        else
            echo "snes9x PAL: $n failing test(s), expected $SNES9X_KNOWN_FAILURES" >&2
            rc=1
        fi
    fi
    if [[ -f $MESEN ]] && command -v dotnet >/dev/null; then
        echo "=== Mesen2 (PAL image) ==="
        dotnet "$MESEN" --testrunner "$PAL_ROM" scripts/accuracysnes/mesen_crossval.lua \
            --timeout=120 --snes.port2.type=SnesController >/dev/null 2>&1
        code=$?
        case $code in
            0)   echo "Mesen2 PAL: OK (0 failing tests)" ;;
            253) echo "Mesen2 PAL: results block never appeared (bad magic)" >&2; rc=1 ;;
            254) echo "Mesen2 PAL: timed out before the battery finished" >&2; rc=1 ;;
            *)   echo "Mesen2 PAL: $code failing test(s)" >&2; rc=1 ;;
        esac
    fi
else
    echo "skip the PAL image: build the cart first (cargo run -p accuracysnes-gen)" >&2
fi

# --- rendered scenes (ADR 0013) ------------------------------------------------------------------
#
# The battery above is self-scoring: the cart decides pass/fail and the references merely have to
# agree. Rendered scenes are the opposite — the cart asserts nothing about pixels, so a golden is
# only worth committing once the references have been shown to agree on the picture. This checks
# exactly that, and it is the rule that keeps a golden from quietly becoming a record of our own
# output. It found two real PPU bugs on the first three scenes it ever ran.
MANIFEST=tests/roms/AccuracySNES/build/scenes.tsv
SCENE_GOLDEN=tests/golden/accuracysnes-scenes.tsv

# Compare `scene<N><TAB><hash>` lines on stdin against the committed goldens, mapping the cart's
# scene numbers to stable names through the manifest. Unblessed scenes are reported, not failed.
check_scenes() {
    local who=$1 bad=0 ok=0 unblessed=0
    while IFS=$'\t' read -r key hash; do
        [[ $key == scene* ]] || continue
        local idx=${key#scene}
        local name
        name=$(awk -F'\t' -v i="$idx" '$1 == i { print $2 }' "$MANIFEST")
        if [[ -z $name ]]; then
            echo "$who: scene $idx is not in $MANIFEST — stale build?" >&2
            bad=$((bad + 1))
            continue
        fi
        local want
        want=$(awk -F'\t' -v n="$name" '$1 == n { print $2 }' "$SCENE_GOLDEN")
        if [[ -z $want ]]; then
            unblessed=$((unblessed + 1))
            echo "$who: $name unblessed (got $hash)"
        elif [[ $want == "$hash" ]]; then
            ok=$((ok + 1))
        else
            echo "$who: $name MISMATCH got $hash, golden $want" >&2
            bad=$((bad + 1))
        fi
    done
    echo "$who: $ok scene(s) match, $unblessed unblessed, $bad mismatched"
    # No scenes at all is a failure, not a clean sheet. It means the host never got as far as the
    # scene loop -- almost always a frame budget that stopped growing with the battery -- and
    # "nothing mismatched" would otherwise report that as a pass.
    if [[ $((ok + unblessed + bad)) -eq 0 ]]; then
        echo "$who: no scenes reported at all — the run did not reach the scene loop" >&2
        return 1
    fi
    return $bad
}

if [[ -f $MANIFEST && -f $SCENE_GOLDEN ]]; then
    if [[ -f $SNES9X ]]; then
        echo "=== snes9x rendered scenes ==="
        # `|| true`: the host's exit code is the battery's failing-test count (2 known
        # divergences for snes9x), and with `pipefail` that would fail this pipeline for a reason
        # that has nothing to do with the scenes. The battery was already graded above.
        #
        # The frame budget covers the battery AND the scene loop that follows it, so it has to grow
        # with the battery. Run short, the cart never reaches the scenes and the host reports zero
        # of them -- which `check_scenes` counts as nothing mismatched, i.e. a silent pass.
        { "$HOST" "$SNES9X" "$ROM" 2600 --scenes 2>/dev/null || true; } | check_scenes "snes9x" \
            || rc=1
    fi
    if [[ -f $MESEN ]] && command -v dotnet >/dev/null; then
        echo "=== Mesen2 rendered scenes ==="
        # 800s, not 400, and not 180 before that. The scene loop runs after the whole battery, and
        # the battery keeps growing -- a timeout that merely fits today produces intermittent
        # "mismatches" that are really a truncated run, and an intermittently-red gate gets
        # ignored, which is worse than a slow one.
        #
        # The last doubling was not gradual growth: G1.11 walks the entire cartridge byte by byte
        # to check the header checksum, so when the image went from 128 KiB to 256 the test's cost
        # doubled with it -- about 320 of the battery's 431 frames are that one test. Summing only
        # the four banks that hold anything would halve it again and would also stop checking the
        # thing most worth checking about a freshly-grown image, which is that the upper banks are
        # mapped at all.
        { dotnet "$MESEN" --testrunner "$ROM" scripts/accuracysnes/mesen_scenes.lua \
            --timeout=800 --snes.port2.type=SnesController 2>/dev/null || true; } \
            | check_scenes "Mesen2" || rc=1
    fi
else
    echo "skip rendered scenes: build the cart first (cargo run -p accuracysnes-gen)" >&2
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
