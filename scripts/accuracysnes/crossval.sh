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
# snes9x, +1 test (E3.09 "Waits: CPU 10, timer 8"): the THIRD row off that same missing `case 0xf0`,
#   and the one that shows why the shape is worth stating. `$F0` bits 4-5 and 6-7 select the SMP's
#   wait state; with the register discarded, selector 2 runs at selector 0's speed and the row's
#   ratio reads 1x where hardware gives 4x. **Mesen2 and ares both PASS it**, so this is 3-vs-1 with
#   snes9x the documented outlier -- and RustySNES only joined the majority in the same change that
#   added the row: its `$F0` selectors were parsed into fields nothing downstream ever read. ares
#   and bsnes are the citation (`sfc/smp/timing.cpp`), carrying `cycleWaitStates {2,4,10,20}` against
#   `timerWaitStates {2,4,8,16}` and the comment that the timers are not affected by the divider
#   glitch.
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
# snes9x, +1 test (cart C3.12 "CGRAM taken in render" = dossier C3.04): the CGRAM sibling of C1.08.
#   (These are cart IDs — what the on-cart battery reports failing — not dossier IDs.) A $2122 write during
#   active display commits to the colour the PPU is drawing (its internal CGRAM address), not the CPU
#   CGADD — with every layer off that colour is the backdrop, index 0. Mesen2 models it (writes use
#   InternalCgramAddress when !CanAccessCgram); snes9x uses the programmed CGADD regardless of the
#   rendering state, so the write lands the wrong colour and the test fails. Documented by nocash
#   fullsnes and the SNESdev Wiki. Read at a controlled dot (H+V IRQ + SEI/WAI), region-independent.
# snes9x, +1 test (cart C7.10 "OAM write to high table" = dossier C7.16): the Uniracers case. An OAM $2104 write during
#   sprite evaluation is driven to the evaluator's address, which is even and in the low table, so it
#   only latches there and the byte lands in the high table at 0x200 | ((evalAddr & 0x1F0) >> 4).
#   Mesen2 models it (same remap); snes9x writes the CPU OAMADDR regardless of rendering, so nothing
#   reaches the high table and the scan finds no write. Documented by nocash fullsnes and the SNESdev
#   Wiki. Read at a controlled eval-phase dot (H+V IRQ + SEI/WAI); the high table is scanned rather
#   than pinned to one byte, so the verdict is independent of the exact eval index and the region.
# snes9x, +1 test (F1.10 "Auto-read start race"): the automatic joypad read begins a few dozen cycles
#   into the first vblank line, not at the vblank edge, so a $4212 bit-0 poll at NMI entry sees the read
#   not-yet-started (bit 0 = 0) even though it is armed. snes9x performs the read as an instant latch
#   with a 3-scanline busy flag anchored at vblank start, so $4212 already reads busy at entry and it
#   fails the test's phase A. RustySNES passes it since the auto-read-start-timing fix, and it is the
#   ONLY one that does: this line used to say "Mesen2 delays the start and passes", and that is
#   RETRACTED -- measured 2026-08-01, Mesen2 fails it, and so does ares. See the F1.10 entry under
#   MESEN2_KNOWN_FAILURES for the 1-vs-3 statement. Documented by nocash fullsnes (the read starts
#   ~dot 32.5-95.5 of the first vblank line). Region-independent — snes9x fails it identically NTSC
#   and PAL.
# snes9x, +1 test (F1.08 "Auto-read start dot"): the golden sibling of F1.10. After the vblank edge the
#   cart requires $4212 bit 0 to read *closed* first (proving the read has not started), then latches the
#   H counter at the closed->open transition as the start dot (~149 with instrument latency; RustySNES
#   and a cycle-accurate core agree). snes9x's instant latch has busy already open at the edge, so the
#   closed-at-edge guard takes the not-started path and the row fails — the same instant-latch divergence
#   as F1.10, on the same nocash-documented behaviour. (F1.09, the busy *duration* golden, still PASSES on
#   snes9x: its latch both sets and clears within the window at a slightly shorter count — 30 vs 33 — a
#   recorded golden difference, not a failing test.) Region-independent.
# snes9x, +1 test (C7.05 "RangeOver dot = idx*2", fails phase B with code 2): the row asserts that
#   Range Over trips at the *evaluation cycle of the 33rd in-range sprite*, H = OAM.INDEX * 2, by
#   sampling the same dot with the 33rd sprite first at index 32 (set dot 65, must read SET) and then
#   at index 72 (set dot 145, must read CLEAR). snes9x reads SET in both, i.e. its Range Over does not
#   move with the index — a scanline-granularity flag rather than a per-sprite one, which is the
#   modelling difference the row exists to detect. Documented by nocash fullsnes and the SNESdev Wiki
#   (range evaluation walks OAM two cycles per sprite). RustySNES's per-dot position is anchored to
#   MesenCE on the *line* by scripts/probes/eval-line-213e; the *dot* itself is documentation-anchored
#   only, because the Mesen2 headless runner times out in this environment. Region-independent.
# snes9x, +1 test (C7.06 "TimeOver by YLOC+1", fails phase A with code 1): the same scanline-granular
#   sprite model as C7.05, seen from the other side. The row requires Time Over to be clear while the
#   sprites' OWN line is being evaluated (V = 100) and set once the line they paint on has begun
#   (V = 101 = YLOC + 1); snes9x already reads it set on V = 100, i.e. it flags the tile-budget
#   overflow a line early because it evaluates and paints in the same pass. The 20-sprite/8x8 control
#   passes there, so this is the position being wrong, not the budget. Documented by nocash fullsnes
#   and the SNESdev Wiki (34-tile budget, raised by the fetch phase). Region-independent.
SNES9X_KNOWN_FAILURES=15

# --- Mesen2's own known divergences -------------------------------------------------------------
#
# Same mechanism as SNES9X_KNOWN_FAILURES, and set only after the failing SET (not the count) was
# read at `R_DONE == $A5` and reproduced exactly across runs, both completing at frame 479. A count
# alone would have hidden the composition, which is the whole point of enumerating them:
#
# **Key these on the ROW, not the index.** The catalogue index of a row moves whenever a test is
# added ahead of it, and this comment has already gone stale once: it named `idx279 F1.03` and
# `idx286 F1.10`, and in the current catalogue those indices are `F1.01` and `F1.08`. An index in a
# comment is a fact with a shelf life; the row name is not. Map with SOURCE_CATALOG.tsv.
#
# MEASURED 2026-08-01 with `scripts/accuracysnes/mesen_failing_set_probe.lua` (read at R_DONE, frame
# 482): the failing set is exactly `F1.03` and `F1.10`, both code 2. The two fail for DIFFERENT
# reasons, and lumping them under one rationale is what hid that for as long as it did.
#
#   F1.03  code 2  | The port-2 limitation, and the ONLY row here it explains. F1.03 clocks both
#                  | ports out of one latch ($4016 and $4017), so it genuinely needs a second
#                  | controller held. mesen_crossval.lua makes exactly ONE emu.setInput call because
#                  | in this build the port argument does not select a controller -- 0/1/2 all land
#                  | on controller 1 -- so a port-2 call would overwrite port 1 with a mask
#                  | containing no Start and the cart would never leave its menu. Covered by the
#                  | in-repo harness and snes9x, which do drive both ports.
#
# `F1.10` USED to be a second entry here. It is not any more, and the reason is worth more than the
# entry was.
#
# It was first attributed to the port-2 limitation, which is wrong: `f1_require_contract` reads
# $4016 only, and code 2 means "$4212 read busy at the very start of the vblank line". Measured
# 2026-08-01, Mesen2 DID fail it, alongside snes9x and ares -- which read as "RustySNES passes
# alone", and was published that way.
#
# **Then rewriting `E3.06` -- an unrelated APU row -- made Mesen2 PASS `F1.10`.** Three identical
# runs before, three identical runs after. The rewrite changed one uploaded program's length, which
# moved the cart's execution phase, which moved when `F1.10` samples `$4212` relative to the vblank
# edge.
#
# So `F1.10`'s Mesen2 verdict is **phase-fragile**: it encodes where the cart happens to be, not only
# what Mesen2 models. That is the same trap already recorded for `E8.01` and for the scene field
# gate, in a third place. snes9x and ares fail it stably; Mesen2 flips. **Do not restore a
# known-failure entry for it** -- either pin the row's sampling point, or accept that Mesen2's
# verdict on it carries no information.
#
# **The Mesen2 runner can also under-report under load.** One run returned a smaller count than every
# other while four other `dotnet` processes were live. `--timeout=60` is a wall-clock bound, so a
# loaded machine cuts the battery short and reports FEWER failures -- which reads as "things
# improved", the most dangerous direction for a gate to be wrong in. Re-run idle before believing a
# drop, and use `mesen_failing_set_probe.lua` to read the SET rather than trusting the count.
MESEN2_KNOWN_FAILURES=1

# The PAL image's own count: only F1.03 fails there, so F1.10 passes on PAL and fails on NTSC under
# the same port-2 limitation. That asymmetry has NOT been explained and is recorded as an open
# question rather than dressed up -- it is the same shape as the region flip that turned out to be
# E8.01's whole problem, and it deserves the same suspicion. Set separately rather than reusing the
# NTSC constant because the two genuinely differ, and a shared constant would hide it.
MESEN2_PAL_KNOWN_FAILURES=1

# --- ares' own known divergences -----------------------------------------------------------------
#
# ares is the THIRD reference, added 2026-08-01 (`scripts/accuracysnes/ares_host/`). It matters
# because this project's provenance rule counts ares and bsnes as ONE reference — so any row where
# RustySNES and snes9x face Mesen2 is only 2-vs-1 if ares is not already on RustySNES's side, and
# before this there was no way to check.
#
# Five rows, and THREE of them are rows snes9x already fails for reasons documented above — ares is
# corroborating snes9x there, not standing alone:
#
#   C7.05  code 1  | Range Over's set dot. snes9x fails it too, on a DIFFERENT code (2), so the two
#                  | dissenters do not even agree with each other about how it is wrong.
#   C7.10  code 1  | The Uniracers OAM-address takeover. snes9x fails it identically; Mesen2 models
#                  | it and RustySNES does too. 2-vs-2.
#   F1.10  code 2  | The auto-read start race — and it is EXCLUDED from the count below rather than
#                  | carried in it, because on ares its verdict is not reproducible. MEASURED
#                  | 2026-08-01, same binary and same image, eight runs at HEAD: `04` every time.
#                  | Adding `E9.09` — an unrelated APU row — made it read `01` on two runs of five
#                  | and `04` on the other three. The row samples `$4212` right at the vblank edge,
#                  | so the cart's execution phase decides it and any change anywhere ahead of
#                  | Group F moves that phase. Mesen2 showed the same thing from the other side
#                  | (see MESEN2_KNOWN_FAILURES): its `F1.10` verdict flipped to PASSING when
#                  | `E3.06` was rewritten. A count that includes this row is a gate that flakes on
#                  | work that has nothing to do with it, so the count excludes it BY NAME, resolved
#                  | through SOURCE_CATALOG.tsv at run time — the catalogue index is the unstable
#                  | key and must never be baked in. Two of three references now demonstrate the
#                  | row's cross-host verdict carries no information; RustySNES passes it because
#                  | of a deliberate fix, and that stands, but pinning its sampling point is the
#                  | only thing that would make the references' verdicts mean anything.
#
# The fifth is a bug in ares rather than a disagreement:
#
#   E8.02  code 3  | ares' `$F1` (CONTROL) handler NEGATES the timer-2 counter reset:
#                  |     if(timer0.enable.raise(data.bit(0)))  { timer0.stage2 = 0; ... }
#                  |     if(timer1.enable.raise(data.bit(1)))  { timer1.stage2 = 0; ... }
#                  |     if(!timer2.enable.raise(data.bit(2))) { timer2.stage2 = 0; ... }
#                  |         ^ negated, and only here  (ares/sfc/smp/io.cpp)
#                  | `raise()` is true on a 0->1 transition, so timers 0 and 1 reset ON a raise --
#                  | the documented behaviour -- while timer 2 resets on anything EXCEPT one. Both
#                  | The row enables timer 2 via `$F1`, runs an interval, then writes `$F1` again to
#                  | STOP it, and that stopping write has already zeroed T2OUT -- so it reads 0.
#                  | ares is internally inconsistent here, which is the strongest evidence available
#                  | that it is a stray `!`: its own timers 0 and 1 do the un-negated version, and
#                  | RustySNES, snes9x and Mesen2 all treat the three identically. The first bug this
#                  | cart has found in a REFERENCE emulator.
#
# **`E3.06` used to be a sixth entry and is not any more, and it is worth saying why.** It failed on
# ares for a reason that turned out to be the CART's: it read `TnOUT` once at the end of the
# interval, and `TnOUT` is a four-bit read-and-clear counter, so the useful range was 8..15 with the
# wrap one tick above. Rewritten to POLL and accumulate -- and to assert the ratio rather than two
# absolute counts -- RustySNES and ares now report the identical 6 and 46. The row proves ares'
# timer 2 does run at 64 kHz; the old instrument could not see it. A ceiling in the instrument reads
# exactly like a defect in the thing measured.
#
# Three, not four: `F1.10` is excluded by name — see its entry above.
ARES_KNOWN_FAILURES=3

# The row excluded from every host's failing count. Named, never indexed: the catalogue index moves
# whenever a test is added ahead of it, which is exactly the circumstance that exposed the problem.
#
# It started as an ares-only exclusion and is now applied to Mesen2 as well, on the same evidence
# and for the same reason. Adding `B2.07` made Mesen2's PAL verdict on this row flip between runs of
# a single build — two runs of the same image gave `F1.03` alone and then `F1.03` + `F1.10`. That is
# the third host/image combination to show it. The row samples `$4212` right at the vblank edge, so
# ANY test added ahead of Group F moves the phase that decides it, and a gate that counts it fails
# on work with nothing to do with controller ports.
UNSTABLE_ROW=F1.10
UNSTABLE_INDEX=$(awk -F'\t' -v id="$UNSTABLE_ROW" '$2 == id { print $1; exit }' \
    tests/roms/AccuracySNES/SOURCE_CATALOG.tsv)
if [[ -z $UNSTABLE_INDEX ]]; then
    echo "error: $UNSTABLE_ROW is not in SOURCE_CATALOG.tsv — the exclusion is stale" >&2
    exit 1
fi
export ACCURACYSNES_SKIP_INDEX=$UNSTABLE_INDEX

# Where the built host lives. Not built by this script: it is a C++ link against ares' static libs
# and takes minutes, so it is opt-in via `scripts/accuracysnes/ares_host/build.sh` and this block
# skips cleanly when the binary is absent — the same shape as the snes9x and Mesen2 blocks.
ARES_HOST=${ARES_HOST:-${TMPDIR:-/tmp}/ares_host}

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
        "$MESEN2_KNOWN_FAILURES")
             echo "Mesen2: OK ($code known divergence(s) — see MESEN2_KNOWN_FAILURES above)" ;;
        *)   echo "Mesen2: $code failing test(s), expected $MESEN2_KNOWN_FAILURES" >&2; rc=1 ;;
    esac
    ran=$((ran + 1))
else
    echo "skip Mesen2: build it with 'make -C ref-proj/Mesen2'" >&2
fi

# --- ares, via the headless host ------------------------------------------------------------------
if [[ -x $ARES_HOST ]]; then
    echo "=== ares (headless host) ==="
    # The host prints the results block on stdout; its failing count is derived here rather than
    # returned as an exit code, because a status byte is `even = FAIL` and that is worth reading in
    # one place rather than encoding twice.
    # Parity off the LAST HEX DIGIT, not arithmetic on the string: POSIX awk does not parse "0x01",
    # so `("0x" $3) % 2` is 0 for every byte and counts all 337 non-skipped rows as failures. That is
    # a gate reporting catastrophe out of a parsing bug, which is worth the comment.
    # $FF is SKIP and $00 is NOT-RUN; both are excluded, matching how the other hosts count.
    # UNSTABLE_ROW is excluded too, through the shared index resolved above.
    n=$("$ARES_HOST" "$ROM" 900 2>/dev/null |
        awk -v skip="$UNSTABLE_INDEX" '$1 == "status" && $2 + 0 != skip + 0 && $3 != "ff" && $3 != "00" &&
                             $3 ~ /[02468aceACE]$/ { c++ }
             END { print c + 0 }')
    if [[ $n -eq $ARES_KNOWN_FAILURES ]]; then
        echo "ares: OK ($n known divergence(s) — see ARES_KNOWN_FAILURES above)"
    else
        echo "ares: $n failing test(s), expected $ARES_KNOWN_FAILURES" >&2
        rc=1
    fi
    ran=$((ran + 1))
else
    echo "skip ares: build it with 'bash scripts/accuracysnes/ares_host/build.sh'" >&2
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
            "$MESEN2_PAL_KNOWN_FAILURES")
                 echo "Mesen2 PAL: OK ($code known divergence(s))" ;;
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
