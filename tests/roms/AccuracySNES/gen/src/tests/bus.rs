//! Group B — the 5A22: bus access speed, clock geometry, interrupt flags, multiply/divide.
//!
//! Per `docs/accuracysnes-research-dossier.md` §5.B and ticket **T-04-B**.
//!
//! Group B matters out of proportion to its size: every other subsystem is scheduled against this
//! one, so a wrong access-speed table or a mistimed vblank flag shows up as a symptom somewhere
//! else entirely. It is also the group that most directly reuses the H-counter measurement
//! primitive Group A's cycle tests are built on.
//!
//! This first batch takes the parts that need no new machinery: the multiply/divide unit and the
//! NMI flag are plain register reads, and the two access-speed tests are differential measurements
//! of the shape `A5` already established.
//!
//! Deliberately **not** here yet: the scanline-geometry assertions (`B2`), which need the short
//! scanline and frame-length totals rather than a within-line delta; DRAM refresh (`B3`), where
//! `docs/accuracy-ledger.md` and the dossier actively disagree about scope and the tests must
//! probe *position* rather than aggregate frame length; and the IRQ-timing half of `B4`, which
//! needs the IRQ to be armed and acknowledged around a measurement.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// Every Group B test in this batch, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        // --- B1: memory access speed ---
        b1_01(),
        b1_02(),
        // --- B2: frame geometry ---
        b2_04(),
        // --- B4: NMI flag mechanics and the IRQ timers ---
        b4_03(),
        b4_04(),
        b4_05(),
        b4_08(),
        b4_12(),
        b4_15(),
        // --- B5: the multiply/divide unit ---
        b5_01(),
        b5_02(),
        b5_03(),
        b5_04(),
        b5_05(),
        // --- second Group B batch ---
        b1_03(),
        b1_04(),
        b2_06(),
        b2_05(),
        b4_14(),
        b2_10(),
        b4_07(),
        b4_09(),
    ]
}

/// Dots elapsed for `n` master clocks. A dot is 4 master clocks.
const fn dots(master_clocks: u16) -> u16 {
    master_clocks / 4
}

/// Measurement tolerance in dots, matching Group A's.
///
/// The CPU's 6- and 8-clock cycles do not divide evenly into the PPU's 4-clock dot, and
/// `hv_begin` releases anywhere inside a 16-dot window, so repeated runs land a dot or two apart.
const TOL: u16 = 2;

// ---------------------------------------------------------------------------------------------
// B1 — memory access speed
// ---------------------------------------------------------------------------------------------

/// `MEMSEL` (`$420D` bit 0) switches banks `$80`+ ROM between 8 and 6 master clocks per access.
///
/// This is FastROM, and it is worth 25% of the CPU's memory bandwidth. The test reads through a
/// **long** address in bank `$80` so the access is the thing being timed; the code itself keeps
/// executing from bank `$00`, which is always 8 clocks regardless of `MEMSEL`, so toggling the bit
/// cannot change the cost of the loop that does the measuring.
fn b1_01() -> Test {
    let mut a = Asm::new();
    a.c("32 long reads from bank $80, once slow and once fast. 2 master clocks saved per access");
    a.c("over 32 accesses is 64 clocks = 16 dots, comfortably outside the measurement jitter.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $420D         ; MEMSEL = 0: banks $80+ run at 8 clocks");
    a.measure_begin();
    a.repeat(32, &["lda f:$808000"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $420D         ; MEMSEL = 1: FastROM, 6 clocks");
    a.measure_begin();
    a.repeat(32, &["lda f:$808000"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0082");
    a.c("Restore the slow default before asserting — a failing path exits immediately.");
    a.l("sep #$20");
    a.l("stz $420D");
    a.l("rep #$20");
    a.l("lda f:$7E0080");
    a.l("sec");
    a.l("sbc f:$7E0082     ; slow - fast, so a working FastROM is positive");
    a.assert_a16_range(
        dots(32 * 2) - TOL,
        dots(32 * 2) + TOL,
        "MEMSEL did not change bank $80 access speed by 2 master clocks per access",
    );
    a.finish(
        "B1.01",
        'B',
        "MEMSEL selects FastROM",
        Provenance::Documented("SNESdev Wiki, Memory map / timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The joypad serial ports (`$4000`-`$41FF`) are the slowest region on the bus: 12 master clocks.
///
/// Everything else in CPU MMIO runs at 6, so a read of `$4016` costs exactly twice a read of
/// `$4212`. A core with a single flat "MMIO is 6 clocks" rule loses that, and the error compounds
/// in any polling loop — which is precisely where joypad reads live.
fn b1_02() -> Test {
    let mut a = Asm::new();
    a.c("$4016 (JOYSER0, 12 clocks) against $4212 (HVBJOY, 6 clocks). $4212 is chosen as the");
    a.c("baseline because it is the one CPU MMIO read with no side effect at all: $4210 and $4211");
    a.c("are read-to-clear, and clearing a pending flag mid-measurement would change the test.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(16, &["lda $4212"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(16, &["lda $4016"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080     ; joypad - mmio");
    a.assert_a16_range(
        dots(16 * 6) - TOL,
        dots(16 * 6) + TOL,
        "the joypad ports were not 6 master clocks slower per access than CPU MMIO",
    );
    a.finish(
        "B1.02",
        'B',
        "JOYSER is 12 clocks",
        Provenance::Documented("SNESdev Wiki, Memory map / timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// B4 — NMI flag mechanics
// ---------------------------------------------------------------------------------------------

/// `RDNMI` (`$4210`) bit 7 is set at the start of vblank, independently of whether NMI is enabled.
///
/// The flag tracks the vblank *event*, not the interrupt: `$4200` bit 7 gates whether the CPU is
/// interrupted, and a core that only sets the flag when NMI is enabled breaks every game that
/// polls `$4210` with interrupts off — which is exactly how this battery's own runtime works.
fn b4_03() -> Test {
    let mut a = Asm::new();
    a.c("The runtime keeps NMI disabled and polls, so this samples the flag with $4200.7 clear.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $4210         ; clear any flag left pending by an earlier test");
    a.l("jsr wait_vblank   ; land on a vblank boundary");
    a.l("jsr wait_vblank   ; and again, so the flag was set by THIS vblank");
    a.l("lda $4210");
    a.l("and #$80");
    a.assert_a8(
        0x80,
        "RDNMI bit 7 was not set at vblank while NMI was disabled",
    );
    a.finish(
        "B4.03",
        'B',
        "RDNMI sets at vblank",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `RDNMI` is read-to-clear: a second read in the same vblank returns bit 7 clear.
///
/// Split from B4.03 because the two failure modes are opposite — a core can set the flag correctly
/// and never clear it, which makes a polling loop spin forever on a stale vblank.
fn b4_04() -> Test {
    let mut a = Asm::new();
    a.c("Read twice back to back inside one vblank. The first read must report and consume the");
    a.c("flag; the second must find it gone.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $4210");
    a.l("jsr wait_vblank");
    a.l("jsr wait_vblank");
    a.l("lda $4210");
    a.l("and #$80");
    a.assert_a8(
        0x80,
        "RDNMI bit 7 was not set on the first read of a vblank",
    );
    a.l("lda $4210         ; the same vblank, immediately after");
    a.l("and #$80");
    a.assert_a8(0x00, "RDNMI did not clear on read");
    a.finish(
        "B4.04",
        'B',
        "RDNMI is read-to-clear",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The CPU revision nibble in `$4210` bits 3-0, recorded rather than asserted.
///
/// A property of the console a cartridge is in, not of the architecture — and it *gates* real
/// behaviour: the two revision-specific DMA/HDMA bugs in `D3` only reproduce on particular
/// revisions, so any test for those has to read this first rather than assume.
fn b4_15() -> Test {
    let mut a = Asm::new();
    a.c("Report the low nibble of $4210 as the variant code.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $4210");
    a.l("and #$0F          ; CPU revision");
    a.l("asl a");
    a.l("ora #$01          ; encode as (revision << 1) | 1");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B4.15",
        'B',
        "CPU revision (golden)",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Golden,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// B5 — the multiply/divide unit
// ---------------------------------------------------------------------------------------------

/// Unsigned 8x8 multiply: `$4202` x `$4203` lands in `RDMPY` (`$4216`/`$4217`).
///
/// The unit is not instantaneous — the product needs 8 CPU cycles after the write to `$4203` — so
/// the delay below is part of the contract, not padding. Reading too early on hardware returns a
/// partial result.
fn b5_01() -> Test {
    let mut a = Asm::new();
    a.c("$5A * $03 = $010E, chosen so the answer straddles both result bytes and neither can be");
    a.c("right by accident if the halves are swapped.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$5A");
    a.l("sta $4202");
    a.l("lda #$03");
    a.l("sta $4203         ; the write to $4203 starts the multiply");
    a.repeat(8, &["nop"]);
    a.l("rep #$20");
    a.l("lda $4216");
    a.assert_a16(0x010E, "8x8 multiply produced the wrong product");
    a.finish(
        "B5.01",
        'B',
        "8x8 unsigned multiply",
        Provenance::Documented("SNESdev Wiki, CPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Unsigned 16/8 divide: quotient in `RDDIV` (`$4214`/`$4215`), remainder in `RDMPY`.
///
/// The remainder sharing `RDMPY` with the multiplier is the detail worth pinning: the two units
/// overlap in their output registers, which is also why B5.04 exists.
fn b5_02() -> Test {
    let mut a = Asm::new();
    a.c("$04D2 / $07 = $B0 remainder $02 (1234 / 7 = 176 r2). The divide needs 16 CPU cycles.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("lda #$04D2");
    a.l("sta $4204         ; 16-bit store fills $4204/$4205");
    a.l("sep #$20");
    a.l("lda #$07");
    a.l("sta $4206         ; the write to $4206 starts the divide");
    a.repeat(16, &["nop"]);
    a.l("rep #$20");
    a.l("lda $4214");
    a.assert_a16(0x00B0, "16/8 divide produced the wrong quotient");
    a.l("lda $4216");
    a.assert_a16(0x0002, "16/8 divide produced the wrong remainder");
    a.finish(
        "B5.02",
        'B',
        "16/8 unsigned divide",
        Provenance::Documented("SNESdev Wiki, CPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Divide by zero yields quotient `$FFFF` and remainder = dividend.
///
/// There is no fault and no trap — the unit simply saturates, and code that divides by a computed
/// zero keeps running with those values. Worth asserting because "we never divide by zero" is not
/// something an emulator gets to assume about the software it runs.
fn b5_03() -> Test {
    let mut a = Asm::new();
    a.c("$1234 / 0 -> quotient $FFFF, remainder $1234.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("lda #$1234");
    a.l("sta $4204");
    a.l("sep #$20");
    a.l("stz $4206         ; divide by zero");
    a.repeat(16, &["nop"]);
    a.l("rep #$20");
    a.l("lda $4214");
    a.assert_a16(
        0xFFFF,
        "divide by zero did not saturate the quotient to $FFFF",
    );
    a.l("lda $4216");
    a.assert_a16(
        0x1234,
        "divide by zero did not leave the dividend as the remainder",
    );
    a.finish(
        "B5.03",
        'B',
        "Divide by zero",
        Provenance::Documented("SNESdev Wiki, CPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Overlapping a multiply and a divide — **a golden vector, never scored**.
///
/// The two units share `RDMPY`, and the SNESdev Errata page states outright that starting one
/// while the other is in flight is undefined. Both operations are started back to back here and
/// the low byte of the shared register is recorded as a variant code. Asserting any particular
/// answer would be inventing authority the sources explicitly decline to give — the same reasoning
/// that keeps `A7.04` (decimal-mode `V`) out of the pass rate.
fn b5_04() -> Test {
    let mut a = Asm::new();
    a.c("Start a divide, then start a multiply before it can finish, and report what RDMPY holds.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("lda #$1234");
    a.l("sta $4204");
    a.l("sep #$20");
    a.l("lda #$07");
    a.l("sta $4206         ; divide begins");
    a.l("lda #$5A");
    a.l("sta $4202");
    a.l("lda #$03");
    a.l("sta $4203         ; multiply begins while the divide is still in flight");
    a.repeat(16, &["nop"]);
    a.l("lda $4216         ; whatever the shared register ended up holding");
    a.l("and #$0F          ; low nibble only — the full byte does not fit a variant code");
    a.l("asl a");
    a.l("ora #$01");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B5.04",
        'B',
        "Mul/div overlap (golden)",
        Provenance::Contested(
            "SNESdev Errata states overlapping $4203/$4206 operation is undefined",
        ),
        Kind::Golden,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// B2 — frame geometry
// ---------------------------------------------------------------------------------------------

/// Emit: measure this machine's frame height, leaving the maximum V in a 16-bit accumulator.
///
/// 261 on NTSC, 311 on PAL. Used by tests whose expected value depends on frame length, so they
/// can branch on what they MEASURED rather than on the region bit — whose position was itself
/// contested (`B2.10`) and which a frame-length test must not depend on.
///
/// **Register width contract: returns with `A` and `X`/`Y` all 16-bit** (the polling loop ends in
/// `rep #$30`). Costs a frame; call it once.
pub fn measure_frame_height(a: &mut Asm) {
    a.l("sep #$20");
    a.l("stz $2133         ; SETINI: no interlace, which would add a line");
    a.l("jsr wait_vblank");
    a.l("jsr wait_vblank   ; a settled frame");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("sta f:$7E0124     ; running maximum");
    a.label("fh_loop");
    read_v(a);
    a.l("cmp f:$7E0124");
    a.l("bcc :+");
    a.l("sta f:$7E0124");
    a.l(":");
    a.l("cmp #100          ; below 100 means the counter wrapped into the next frame");
    a.l("bcs @fh_loop");
    a.l("lda f:$7E0124");
}

/// Emit: latch the counters and leave the 9-bit V position in a 16-bit accumulator.
///
/// **Register width contract: returns with `A` 16-bit; `X`/`Y` are untouched.** Entry width does
/// not matter — the emitter sets what it needs. Stated explicitly because the generator tracks
/// widths file-globally to decide between `.a8` and `.a16`, and the dangerous direction is silent:
/// if the assembler believes `A` is 16-bit while the CPU has it 8-bit, immediate operands assemble
/// one byte short and every instruction after them shifts.
pub fn read_v(a: &mut Asm) {
    a.l("sep #$20");
    a.l("lda $213F         ; reset the counter read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213D         ; V low");
    a.l("xba");
    a.l("lda $213D");
    a.l("and #$01          ; bit 0 is V bit 8; bits 1-7 are PPU2 open bus");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
}

/// An NTSC frame is 262 lines, so the V counter tops out at 261.
///
/// Frame length is the denominator of every timing budget on the machine — a core one line out
/// gives games a vblank that is 340 dots too long or too short, which is exactly the kind of error
/// that only shows up as a game-specific glitch much later.
///
/// Sampled rather than counted: the loop latches V repeatedly from the start of vblank until the
/// counter wraps, tracking the maximum. Each iteration costs a handful of dots against a scanline
/// of 340, so the top line cannot be missed.
///
/// **Region.** The battery ships as two images differing in one header byte, so this same test
/// runs on both an NTSC and a PAL machine (`build/accuracysnes-pal.sfc`). It stands down as SKIP
/// on PAL rather than asserting the wrong number, and `B2.05` is its mirror. The skip predicate is
/// the *measured* line count, not the region bit: which bit of `$213F` carries the region is
/// contested (`B2.10`), and a frame-height test must not depend on the thing it is evidence for.
fn b2_04() -> Test {
    let mut a = Asm::new();
    a.c("Start at vblank, poll V until it wraps to the top of the next frame, keep the maximum.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $2133         ; SETINI: no interlace (see B2.05 — test order must not decide this)");
    a.l("jsr wait_vblank");
    a.l("jsr wait_vblank   ; V is now at the first vblank line of a settled frame");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("sta f:$7E0120     ; running maximum");
    a.label("vloop");
    read_v(&mut a);
    a.l("cmp f:$7E0120");
    a.l("bcc :+");
    a.l("sta f:$7E0120");
    a.l(":");
    a.l("cmp #100          ; below 100 means the counter has wrapped into the next frame");
    a.l("bcs @vloop");
    a.l("lda f:$7E0120");
    a.l("cmp #311");
    a.l("bne :+");
    a.skip("V topped out at 311 — this is a PAL machine, so B2.05 is the applicable assertion");
    a.l(":");
    a.assert_a16(
        261,
        "the V counter did not reach 261 (an NTSC frame is 262 lines, 0-261)",
    );
    a.finish(
        "B2.04",
        'B',
        "NTSC frame is 262 lines",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A PAL frame is 312 lines, so the V counter tops out at 311.
///
/// The mirror of `B2.04`, and the reason the battery ships a PAL image at all. "This needs a PAL
/// console" is only half true: a console's region fixes the timing, but which timing an emulator
/// boots is decided by the cart header's country code, so a one-byte header change exercises PAL
/// on every emulator with no harness-side switch a reference emulator has no equivalent of. On real
/// hardware the console still wins, which is why this decides what it is running on by measurement
/// rather than by trusting its own header.
fn b2_05() -> Test {
    let mut a = Asm::new();
    a.c("Identical to B2.04's measurement; only the expected line count differs.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $2133         ; SETINI: no interlace — B2.06 leaves it ON, and an interlaced PAL");
    a.c("frame is 313 lines on the long field, so measuring frame height without clearing this");
    a.c("measures B2.06's leftovers instead. Found by the PAL image: B2.04 skipped (it saw 311)");
    a.c("while B2.05 failed, which is only possible if the two measured different machines.");
    a.l("jsr wait_vblank");
    a.l("jsr wait_vblank   ; a settled frame under the cleared setting");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("sta f:$7E0120     ; running maximum");
    a.label("vloop");
    read_v(&mut a);
    a.l("cmp f:$7E0120");
    a.l("bcc :+");
    a.l("sta f:$7E0120");
    a.l(":");
    a.l("cmp #100          ; below 100 means the counter has wrapped into the next frame");
    a.l("bcs @vloop");
    a.l("lda f:$7E0120");
    a.l("cmp #261");
    a.l("bne :+");
    a.skip("V topped out at 261 — this is an NTSC machine, so B2.04 is the applicable assertion");
    a.l(":");
    a.assert_a16(
        311,
        "the V counter did not reach 311 (a PAL frame is 312 lines, 0-311)",
    );
    a.finish(
        "B2.05",
        'B',
        "PAL frame is 312 lines",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `RDNMI` bit 7 clears by itself at the end of vblank, not only when read.
///
/// A core that clears the flag *only* on read leaves it set through the whole active display, so
/// code that polls `$4210` outside vblank sees a vblank that already ended and acts a frame late.
/// This is the counterpart to B4.04: together they pin both ways the flag can go away.
fn b4_05() -> Test {
    let mut a = Asm::new();
    a.c("Reach vblank and deliberately do NOT read $4210, then wait for active display and read.");
    a.c("The flag must already be gone.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $4210         ; clear anything left pending by an earlier test");
    a.l("jsr wait_vblank");
    a.l("jsr wait_vblank   ; in vblank, flag set, and left unread");
    a.label("wa");
    a.l("lda $4212");
    a.l("and #$80");
    a.l("bne @wa           ; wait for vblank to end");
    a.l("lda $4210");
    a.l("and #$80");
    a.assert_a8(
        0x00,
        "RDNMI stayed set past the end of vblank (it must auto-clear, not only clear on read)",
    );
    a.finish(
        "B4.05",
        'B',
        "RDNMI auto-clears",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A V-IRQ fires on the programmed scanline.
///
/// Armed with interrupts masked and observed by polling `$4211`, so the test measures *when the
/// comparator matched* without depending on interrupt dispatch. Raster effects are built on this:
/// a V-IRQ that fires a line early or late tears the split it was scheduled for.
fn b4_08() -> Test {
    let mut a = Asm::new();
    a.c("VTIME = 100, V-IRQ only. I is set so nothing vectors; $4211 is polled instead. The V");
    a.c("counter is latched the moment the flag appears and must still be on the programmed line.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei               ; observe the comparator, do not dispatch");
    a.l("lda #100");
    a.l("sta $4209");
    a.l("stz $420A         ; VTIME = 100");
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$20");
    a.l("sta $4200         ; V-IRQ enabled, NMI off, auto-joypad off");
    a.label("wirq");
    a.l("lda $4211");
    a.l("and #$80");
    a.l("beq @wirq");
    read_v(&mut a);
    a.l("sta f:$7E0122");
    a.c("Disarm before asserting — a failing path exits straight to test_restore.");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.l("rep #$20");
    a.l("lda f:$7E0122");
    a.assert_a16_range(
        100,
        102,
        "the V-IRQ did not fire on the programmed scanline",
    );
    a.finish(
        "B4.08",
        'B',
        "V-IRQ fires at VTIME",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Reading `$4211` releases the IRQ latch immediately, even while the trigger line is still current.
///
/// The V-IRQ is a one-shot per frame, not a level held for the whole scanline: once acknowledged it
/// stays clear even though the counter has not moved off the programmed line yet. A core that
/// re-asserts while `V == VTIME` produces a storm of spurious interrupts for the rest of the line.
fn b4_12() -> Test {
    let mut a = Asm::new();
    a.c("Read $4211 twice back to back at the moment it fires. The second read is still on the");
    a.c("same scanline, and must find the latch already released by the first.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");
    a.l("lda #100");
    a.l("sta $4209");
    a.l("stz $420A");
    a.l("lda $4211");
    a.l("lda #$20");
    a.l("sta $4200");
    a.label("wirq2");
    a.l("lda $4211");
    a.l("and #$80");
    a.l("beq @wirq2        ; this read both detects and acknowledges");
    a.c("Disarm BEFORE looking again. The claim is that a read releases the latch; while the");
    a.c(
        "comparator still matches -- a V-only IRQ matches for the whole scanline -- a core is free",
    );
    a.c("to re-assert it, and a second read on the same line then says nothing about the release.");
    a.c("Asserting the stronger thing made this test depend on where in the scanline the polling");
    a.c("loop happened to catch the flag: it began failing on Mesen2 when an unrelated change");
    a.c("moved the battery's code by a few bytes.");
    a.l("stz $4200         ; disarm, so nothing can re-assert what the read released");
    a.l("lda $4211         ; must now read clear");
    a.l("sta f:$7E0124");
    a.l("lda f:$7E0124");
    a.l("and #$80");
    a.assert_a8(0x00, "$4211 did not release the IRQ latch on read");
    a.finish(
        "B4.12",
        'B',
        "$4211 read releases IRQ",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Power-on state of the multiply/divide latches: `WRMPYA` = `$FF`, `WRDIV` = `$FFFF`.
///
/// Both are **write-only**, so this is not a readback — it is the latch observed through the unit
/// it feeds. The runtime writes only `$4203` (multiplying whatever `$4202` already held) and only
/// `$4206` (dividing whatever `$4204/05` held), before `init_registers` zeroes them, and stashes
/// the results in the capture block.
///
/// Scored, on two independent documentation lineages that agree and nothing contradicting them in
/// nineteen years: anomie's `regs.txt` (r1157) states the values flatly in a document that marks
/// its uncertain claims with `(?)` and marks neither of these; nocash's fullsnes independently
/// lists `$4202`-`$4206` as `(FFh)` at power-up under a legend separating power-up from reset.
/// bsnes, ares and Mesen2 all implement it.
///
/// **snes9x fails this test**, and that is a snes9x bug rather than counter-evidence: its
/// `S9xSoftResetPPU` blanket-`memset`s `$4200-$42FF` to zero and special-cases only `$4201`/`$4213`.
/// The divergence is declared in `scripts/accuracysnes/crossval.sh` so the cross-validation gate
/// stays meaningful instead of being weakened to unanimity.
///
/// Deliberately **not** asserted: the power-on contents of `$4203` and `$4206`. Multiplication only
/// starts on a write to `$4203` and division on a write to `$4206`, so their power-on values can
/// never influence a readable result — fullsnes says `$FF`, Mesen2 uses `$00`, and nothing can tell
/// the difference.
fn b5_05() -> Test {
    let mut a = Asm::new();
    a.c(
        "$FF x 2 = $01FE, and $FFFF / 2 = $7FFF remainder 1, read from the pre-init capture block.",
    );
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("lda f:$7EE040     ; V_PO_MPY");
    a.assert_a16(
        0x01FE,
        "$4202 did not power up as $FF (the captured product was not $FF x 2)",
    );
    a.l("lda f:$7EE042     ; V_PO_DIV");
    a.assert_a16(
        0x7FFF,
        "$4204/05 did not power up as $FFFF (the captured quotient was not $FFFF / 2)",
    );
    a.l("lda f:$7EE044     ; V_PO_DIVREM");
    a.assert_a16(0x0001, "the captured power-on divide remainder was wrong");
    a.finish(
        "B5.05",
        'B',
        "Mul/div power-on state",
        Provenance::Documented(
            "anomie regs.txt r1157 and nocash fullsnes, independently; implemented by \
             bsnes/ares/Mesen2. No known hardware test ROM",
        ),
        Kind::Scored,
        None,
    )
}

/// An internal cycle costs 6 master clocks — the CPU's native rate, independent of any address.
///
/// This is the floor the whole timing model rests on: `clocks = 6*cycles + 2*mem` is only true
/// because an internal cycle is 6 and a memory cycle in an 8-clock region is 8. Nintendo states the
/// core side of it directly — the CPU *"is operated internally with a 3.58MHz clock speed"*, which
/// is master/6.
///
/// `XBA` and `NOP` differ by exactly one internal cycle (`XBA` is 1 access + 2 internal, `NOP` is
/// 1 + 1) and are both single-byte, so the difference between them isolates the internal cycle with
/// no memory-access term to cancel.
fn b1_03() -> Test {
    let mut a = Asm::new();
    a.c("XBA minus NOP is exactly one internal cycle: 6 clocks, so 24 dots over 16 repeats.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$30");
    a.measure_begin();
    a.repeat(16, &["nop"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0098");
    a.record(70, "16 NOP, absolute");
    a.l("sep #$30");
    a.measure_begin();
    a.repeat(16, &["xba"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0098");
    a.record(71, "16 XBA - 16 NOP = 16 internal cycles");
    a.assert_a16_range(
        24 - TOL,
        24 + TOL,
        "one internal cycle did not cost 6 master clocks",
    );
    a.finish(
        "B1.03",
        'B',
        "Internal cycles are 6",
        Provenance::Documented("SNES Development Manual Bk I 21.1; SNESdev Wiki; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// DMA transfers at 8 master clocks per byte **regardless of the source region**.
///
/// Nintendo states it outright — DMA runs at 2.68 MHz *"regardless of the address"* — and 2.68 MHz
/// is master/8. The trap for an emulator is reusing the CPU's memory-speed map for DMA: a
/// `MEMSEL`-fast source would then transfer quicker, which hardware does not do.
///
/// So the test is a **differential between two source regions of different CPU speed**: bank `$80`
/// with `MEMSEL` set is a 6-clock region for the CPU, bank `$00` is 8. If DMA is region-independent
/// both transfers take the same time; if the DMA borrows the CPU's map they differ by 2 clocks a
/// byte. Comparing two DMAs rather than asserting an absolute also cancels the fixed setup cost,
/// which the manual does not specify.
fn b1_04() -> Test {
    let mut a = Asm::new();
    a.c("32 bytes to the VRAM port, once from a MEMSEL-fast bank and once from a slow one.");
    a.c("Forced blank is in force for the whole battery, so VRAM is writable throughout.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $420D         ; MEMSEL = 1: banks $80+ are a 6-clock region for the CPU");
    a.l("lda #$80");
    a.l("sta $2115         ; VMAIN step 1, increment after the high byte");
    a.c("--- channel 0: A->B, mode 1 (two registers), destination $2118 ---");
    a.l("lda #$01");
    a.l("sta $4300");
    a.l("lda #$18");
    a.l("sta $4301");
    a.c("--- transfer 1: source $80:8000, the CPU-fast bank ---");
    a.l("rep #$30");
    a.l("ldx #$1800");
    a.l("stx $2116");
    a.l("ldx #$8000");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $4304");
    a.l("rep #$30");
    a.l("ldx #$0020");
    a.l("stx $4305         ; 32 bytes");
    a.l("sep #$20");
    a.measure_begin();
    a.l("lda #$01");
    a.l("sta $420B         ; start channel 0");
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E009A");
    a.record(72, "DMA 32B from a MEMSEL-fast bank");
    a.c("--- transfer 2: identical, but sourced from bank $00 (always 8 clocks for the CPU) ---");
    a.l("rep #$30");
    a.l("ldx #$1900");
    a.l("stx $2116");
    a.l("ldx #$8000");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("stz $4304         ; bank $00");
    a.l("rep #$30");
    a.l("ldx #$0020");
    a.l("stx $4305");
    a.l("sep #$20");
    a.measure_begin();
    a.l("lda #$01");
    a.l("sta $420B");
    a.measure_end();
    a.measure_result();
    a.record(73, "DMA 32B from a slow bank");
    a.c("--- restore MEMSEL before asserting; a failing path exits immediately ---");
    a.l("sep #$20");
    a.l("stz $420D");
    a.l("rep #$20");
    a.l("sec");
    a.l("sbc f:$7E009A");
    a.assert_abs_le(
        TOL,
        "DMA timing changed with the source region — it must be 8 clocks/byte regardless",
    );
    a.finish(
        "B1.04",
        'B',
        "DMA speed is uniform",
        Provenance::Documented(
            "SNES Development Manual Bk I 21.1 (DMA at 2.68MHz regardless of address)",
        ),
        Kind::Scored,
        None,
    )
}

/// Screen interlace (`$2133` bit 0) adds a scanline to the frame.
///
/// An interlaced NTSC field runs 263 lines rather than 262, so the V counter reaches 262. Sampled
/// the same way as `B2.04`: poll from vblank until the counter wraps, keeping the maximum.
fn b2_06() -> Test {
    let mut a = Asm::new();
    a.c("Enable interlace, let a full frame pass so the setting is stable, then find V's maximum.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $2133         ; SETINI bit 0 = screen interlace");
    a.l("jsr wait_vblank");
    a.l("jsr wait_vblank   ; a settled frame under the new setting");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("sta f:$7E0126");
    a.label("iloop");
    read_v(&mut a);
    a.l("cmp f:$7E0126");
    a.l("bcc :+");
    a.l("sta f:$7E0126");
    a.l(":");
    a.l("cmp #100");
    a.l("bcs @iloop");
    a.l("sep #$20");
    a.l("stz $2133         ; restore before asserting");
    a.l("rep #$20");
    a.l("lda f:$7E0126");
    a.record(74, "V counter maximum with interlace enabled");
    a.c("Report which line count was seen: variant 1 = 261 (no extra line), 2 = 262, 3 = other.");
    a.c("Both comparisons run while A is still 16-bit and the answer is staged in X; narrowing on");
    a.c("one branch would leave the generator's width tracker wrong for the other path's `cmp`.");
    a.l("ldx #$0007        ; default: variant 3 = something else");
    a.l("cmp #261");
    a.l("bne :+");
    a.l("ldx #$0003        ; variant 1 = 261, no extra line");
    a.l(":");
    a.l("cmp #262");
    a.l("bne :+");
    a.l("ldx #$0005        ; variant 2 = 262, interlace added a line");
    a.l(":");
    a.l("sep #$20");
    a.l("txa");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B2.06",
        'B',
        "Interlace line count",
        Provenance::Contested(
            "the dossier conditions the extra line on $213F.7 (the field), which this test does \
             not control — sampling V across an uncontrolled field cannot assert a line count",
        ),
        Kind::Golden,
        None,
    )
}

/// The 50/60 Hz region bit in `$213F` — **a golden vector**, because the sources conflict.
///
/// The dossier records the disagreement: SNESdev's PPU-registers page places the region bit at
/// bit 3, while bits 3-0 are the PPU2 version field, which would make bit 3 unreadable as a region
/// flag. fullsnes and the dossier's resolution put it at **bit 4**.
///
/// Rather than assert either reading, the test reports both bits: variant = `(bit4 << 1) | bit3`.
/// On an NTSC console with the resolution correct, bit 4 is clear and the variant distinguishes the
/// two candidate encodings without this cart taking a side.
fn b2_10() -> Test {
    let mut a = Asm::new();
    a.c("Report $213F bits 4 and 3 together so the encoding conflict is visible in the result.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $213F");
    a.l("sta f:$7E0128");
    a.l("and #$10          ; candidate region bit per fullsnes");
    a.l("lsr a");
    a.l("lsr a");
    a.l("lsr a           ; -> bit 1");
    a.l("sta f:$7E0129");
    a.l("lda f:$7E0128");
    a.l("and #$08          ; candidate region bit per SNESdev PPU registers");
    a.l("lsr a");
    a.l("lsr a");
    a.l("lsr a           ; -> bit 0");
    a.l("ora f:$7E0129");
    a.l("asl a");
    a.l("ora #$01");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B2.10",
        'B',
        "Region bit (golden)",
        Provenance::Contested(
            "SNESdev PPU registers places the 50/60Hz bit at bit 3, which overlaps the PPU2 \
             version field; fullsnes places it at bit 4",
        ),
        Kind::Golden,
        None,
    )
}

/// Interrupt dispatch is deferred to an instruction boundary (golden vector).
///
/// The dossier states this as "the poll occurs just before the final CPU cycle, so handler entry is
/// 6-12 master cycles after assertion". That exact claim is sub-cycle and a cart cannot see it: the
/// finest clock the CPU can read is the H counter at 4 master clocks per dot, and reading it costs
/// more than the interval being measured.
///
/// What *is* observable is the consequence, and it is the part that matters to software: if the
/// poll happens at an instruction boundary rather than continuously, then an interrupt asserting
/// during a long instruction waits for that instruction to retire. So this measures handler entry
/// twice — once with the CPU spinning on two-cycle `NOP`s, once with it spinning on the longest
/// instruction pair a test can safely execute — and records both, plus the difference.
///
/// **Golden, not scored.** The absolute numbers depend on where in the spin loop the interrupt
/// happens to land, which the cart cannot control to a dot. Their *difference* is the signal, and
/// the point is to make it comparable across emulators rather than to assert a threshold nothing
/// independent has established. `A5.08` is the precedent: a measurement whose references disagree
/// is recorded, not scored.
fn b4_14() -> Test {
    let mut a = Asm::new();
    a.c("Arm an H-IRQ at a known dot, install a handler that latches H on entry, and spin. The");
    a.c(
        "latched dot minus HTIME is the dispatch latency. Run it twice with different spin bodies.",
    );
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");

    a.c("Install the handler. It latches H, acknowledges, flags the spin loop, and returns.");
    a.l("rep #$20");
    a.l("lda #@handler");
    a.l("sta a:V_IRQ_VEC");
    a.l("sep #$20");

    a.c("Pass 1: spin on NOPs, the shortest instruction there is.");
    arm_h_irq(&mut a);
    a.l("cli");
    a.label("spin1");
    a.repeat(4, &["nop"]);
    a.l("lda f:$7E0134");
    a.l("beq @spin1");
    a.l("sei");
    a.l("rep #$20");
    a.l("lda f:$7E0136     ; H latched on handler entry");
    a.l("sec");
    a.l("sbc #200          ; minus HTIME: the dispatch latency in dots");
    a.record(100, "B4.14 dispatch latency, NOP spin (dots)");
    a.l("sta f:$7E0138");
    a.l("sep #$20");

    a.c("Pass 2: spin on JSL/RTL. If the poll were continuous rather than at an instruction");
    a.c("boundary, this would enter the handler in the same place as pass 1.");
    arm_h_irq(&mut a);
    a.l("cli");
    a.label("spin2");
    a.l("jsl @far");
    a.l("sep #$20");
    a.l("lda f:$7E0134");
    a.l("beq @spin2");
    a.l("sei");
    a.l("rep #$20");
    a.l("lda f:$7E0136");
    a.l("sec");
    a.l("sbc #200");
    a.record(101, "B4.14 dispatch latency, JSL/RTL spin (dots)");
    a.l("sec");
    a.l("sbc f:$7E0138     ; the extra delay a long instruction imposes");
    a.record(102, "B4.14 extra latency from the long spin body (dots)");

    a.c("Restore the default handler before leaving — the vector is global state.");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.l("rep #$20");
    a.l("lda #irq_stub");
    a.l("sta a:V_IRQ_VEC");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta f:V_TEST_RESULT   ; golden: the numbers live in the measurement channel");
    a.l("jml test_restore");

    a.c("--- the far routine the long spin calls ---");
    a.label("far");
    a.l("rtl");

    a.c("--- the handler ---");
    a.label("handler");
    a.l("rep #$30");
    a.l("pha");
    a.l("sep #$20");
    a.l("lda $2137         ; latch H and V at handler entry");
    a.l("lda $213C");
    a.l("xba");
    a.l("lda $213C");
    a.l("and #$01");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
    a.l("sta f:$7E0136");
    a.l("sep #$20");
    a.l("lda $4211         ; acknowledge");
    a.l("lda #$01");
    a.l("sta f:$7E0134     ; tell the spin loop to stop");
    a.l("rep #$20");
    a.l("pla");
    a.l("rti");

    a.finish(
        "B4.14",
        'B',
        "IRQ dispatch latency",
        Provenance::Documented(
            "SNESdev Wiki, Timing; fullsnes — the sub-cycle poll point is not CPU-observable, so \
             its consequence is measured instead",
        ),
        Kind::Golden,
        None,
    )
}

/// Arm an H-IRQ at dot 200, clearing the stale latch and the handler's rendezvous byte.
///
/// Shared by `B4.14`'s two passes so the only difference between them is the spin body — which is
/// the whole experiment.
fn arm_h_irq(a: &mut Asm) {
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0134     ; rendezvous byte: the handler sets it (STZ has no long form)");
    a.l("lda #200");
    a.l("sta $4207");
    a.l("stz $4208         ; HTIME = 200");
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$10");
    a.l("sta $4200         ; H-IRQ enabled, NMI off, auto-joypad off");
}

/// An H-IRQ fires on the programmed dot, and the horizontal comparator lags `HTIME`.
///
/// The counterpart to `B4.08`'s V-IRQ. Armed with interrupts masked and observed by polling
/// `$4211`, so it measures the comparator rather than interrupt dispatch. The latched H position is
/// bounded generously: the assertion is that the IRQ fires *near the programmed dot*, not an exact
/// dot, because the comparator's documented lag is a fractional-dot quantity this cart cannot
/// resolve.
fn b4_07() -> Test {
    let mut a = Asm::new();
    a.c("HTIME = 128, H-IRQ only. Poll $4211, then latch H and check it is on the programmed dot.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");
    a.l("lda #$80");
    a.l("sta $4207");
    a.l("stz $4208         ; HTIME = 128");
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$10");
    a.l("sta $4200         ; H-IRQ enabled, NMI off, auto-joypad off");
    a.label("wh");
    a.l("lda $4211");
    a.l("and #$80");
    a.l("beq @wh");
    a.l("sep #$20");
    a.l("lda $213F         ; reset the counter read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213C");
    a.l("xba");
    a.l("lda $213C");
    a.l("and #$01");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
    a.l("sta f:$7E012A");
    a.record(75, "H position when the H-IRQ was observed");
    a.c("--- disarm before asserting ---");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.l("rep #$20");
    a.l("lda f:$7E012A");
    a.c("Report the latched H in 32-dot buckets. The poll loop is coarser than the dot the IRQ");
    a.c("actually fires on, so the exact position is not resolvable from software this way.");
    a.l("lsr a");
    a.l("lsr a");
    a.l("lsr a");
    a.l("lsr a");
    a.l("lsr a           ; H / 32");
    a.l("sep #$20");
    a.l("asl a");
    a.l("ora #$01");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B4.07",
        'B',
        "H-IRQ position (golden)",
        Provenance::Contested(
            "the $4211 poll loop is coarser than the dot the comparator fires on, so the exact \
             H position is not resolvable from software by polling",
        ),
        Kind::Golden,
        None,
    )
}

/// An HV-IRQ requires **both** comparators to match, not either.
///
/// With `$4200` selecting both, an IRQ must not fire on every line that reaches `HTIME`, nor at the
/// start of `VTIME`'s line — only where the two coincide. A core that ORs the conditions fires
/// hundreds of times a frame instead of once, which this catches by checking the V position at the
/// moment the IRQ appears.
fn b4_09() -> Test {
    let mut a = Asm::new();
    a.c("HTIME = 128, VTIME = 150, both enabled. The IRQ must appear on line 150 specifically.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");
    a.l("lda #$80");
    a.l("sta $4207");
    a.l("stz $4208         ; HTIME = 128");
    a.l("lda #150");
    a.l("sta $4209");
    a.l("stz $420A         ; VTIME = 150");
    a.l("lda $4211");
    a.l("lda #$30");
    a.l("sta $4200         ; both H-IRQ and V-IRQ enabled");
    a.label("whv");
    a.l("lda $4211");
    a.l("and #$80");
    a.l("beq @whv");
    read_v(&mut a);
    a.l("sta f:$7E012C");
    a.record(76, "V position when the HV-IRQ was observed");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.l("rep #$20");
    a.l("lda f:$7E012C");
    a.assert_a16_range(
        150,
        152,
        "the HV-IRQ did not require both comparators to match",
    );
    a.finish(
        "B4.09",
        'B',
        "HV-IRQ needs both",
        Provenance::Documented("SNESdev Wiki, Timing; fullsnes"),
        Kind::Scored,
        None,
    )
}
