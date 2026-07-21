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
        b2_01(),
        b4_16(),
        b4_17(),
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
        // --- B3: DRAM refresh ---
        b3_01(),
        b4_13(),
        b4_11(),
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

/// Reading `$4211` releases the IRQ latch: the flag is cleared by the read, not by anything else.
///
/// A driver's interrupt handler acknowledges by reading, and a core that leaves the flag set after
/// a read re-enters the handler forever.
///
/// **What this test does *not* settle is whether a core may re-assert the flag while `V == VTIME`
/// still holds**, and an earlier version of it accidentally did. It read `$4211` twice back to
/// back, with the trigger still armed, and expected the second read to find the latch clear —
/// which is the stronger claim that the V-IRQ is a one-shot per frame rather than a level held for
/// the whole scanline. RustySNES and snes9x agreed; Mesen2 reported the flag set again. The dossier
/// says only that a read releases the latch, so the test now disarms `$4200` first and asserts
/// exactly that. The stronger property is a real question and is worth its own test, with its own
/// citation, once there is one.
fn b4_12() -> Test {
    let mut a = Asm::new();
    a.c("Wait for the IRQ, acknowledging it with the same read that detects it. Then disarm, so");
    a.c("nothing can re-assert what the read released, and look again.");
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
    a.record(128, "16 NOP, absolute");
    a.l("sep #$30");
    a.measure_begin();
    a.repeat(16, &["xba"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0098");
    a.record(129, "16 XBA - 16 NOP = 16 internal cycles");
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
    a.record(130, "DMA 32B from a MEMSEL-fast bank");
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
    a.record(131, "DMA 32B from a slow bank");
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
    a.record(132, "V counter maximum with interlace enabled");
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
    arm_h_irq(&mut a, 200);
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
    arm_h_irq(&mut a, 200);
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
fn arm_h_irq(a: &mut Asm, htime: u16) {
    assert!(htime < 340, "HTIME {htime} is past the end of a scanline");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0134     ; rendezvous byte: the handler sets it (STZ has no long form)");
    a.l(&format!("lda #${:02X}", htime & 0xFF));
    a.l("sta $4207");
    // HTIME is 9 bits. `stz $4208` was fine while every caller used a value under 256, but
    // `B4.16` needs 330 — writing only the low byte would silently arm it at 74 instead.
    a.l(&format!("lda #${:02X}", htime >> 8));
    a.l(&format!("sta $4208         ; HTIME = {htime}"));
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$10");
    a.l("sta $4200         ; H-IRQ enabled, NMI off, auto-joypad off");
}

/// Enabling `$4200.7` while `RDNMI` is already set fires an NMI immediately.
///
/// The NMI enable is a **level**, not an edge: `$4210` bit 7 latches at the start of vblank and
/// stays latched until read, so setting `NMITIMEN` bit 7 while that latch is already up delivers
/// the interrupt at once rather than waiting for the next vblank. Marked `[ERRATA]` in the dossier,
/// and it is the kind of thing a core gets wrong by modelling NMI as "fire once when vblank
/// begins" — which is right for every ordinary program and wrong here.
///
/// # The shape that makes it falsifiable
///
/// The test enters vblank, **deliberately does not read `$4210`** (that would clear the latch), and
/// only then installs a handler and enables NMI. The vblank edge is already past, so:
///
/// * a core honouring the level fires immediately and the handler sets its flag;
/// * a core firing only on the edge has already missed it, and the flag stays clear until the next
///   frame — which this test never waits for.
///
/// So "did the handler run" separates the two without needing to time anything. `$4210` is read
/// only *after* the observation, to leave the latch clean for whatever runs next.
///
/// The handler preserves `A`: `RTI` restores `P`, `PC` and `PBR` but not the registers, and this
/// one runs at an arbitrary point in the enabling code.
fn b4_17() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("handler");
    a.l("rep #$30");
    a.l("pha");
    a.l("sep #$20");
    a.l(".a8");
    a.l("lda #$01");
    a.l("sta f:$7E01C0     ; the handler ran");
    a.l("lda f:$004210     ; acknowledge; long, so DBR cannot matter");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("pla");
    a.l("rti");
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $4200         ; NMI off while this is set up");
    a.l("lda $4210         ; clear any latch left by an earlier test");
    a.l("rep #$20");
    a.l("lda #@handler");
    a.l("sta a:V_NMI_VEC");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E01C0");
    a.c("Enter vblank and leave the RDNMI latch ALONE -- reading $4210 here would clear the very");
    a.c("condition under test.");
    a.l("jsr wait_vblank");
    a.c("The vblank edge is now behind us. A core that fires only on that edge has missed it.");
    a.l("lda #$80");
    a.l("sta $4200         ; enable NMI with the latch already up");
    a.c("A few instructions for the interrupt to be taken; it should already have happened.");
    a.l("nop");
    a.l("nop");
    a.l("nop");
    a.l("nop");
    a.l("stz $4200         ; disarm before asserting; a failure exits immediately");
    a.l("lda $4210         ; leave the latch clean for whatever runs next");
    a.l("lda f:$7E01C0");
    a.assert_a8(
        0x01,
        "enabling NMI with RDNMI already latched did not fire an NMI — the core is treating the \
         enable as an edge rather than a level",
    );
    a.finish(
        "B4.17",
        'B',
        "NMI enable is a level",
        Provenance::Documented("SNESdev Wiki NMITIMEN/RDNMI [ERRATA]; fullsnes $4200/$4210"),
        Kind::Scored,
        None,
    )
}

/// A scanline has **340** dots, numbered `0..=339`. There is no dot 340.
///
/// The line is 1364 master clocks, and two different models satisfy that: `341 × 4`, and
/// `338 × 4 + 2 × 6` with dots 323 and 327 taking six clocks. Frame timing cannot tell them apart —
/// both give the same total — which is why a core can carry the wrong one indefinitely and pass
/// every refresh-rate test. What separates them is the H counter itself: the uniform model has a
/// dot 340 that hardware never reports.
///
/// fullsnes settles it by measurement rather than prose. Its *PPU H-Counter-Latch Quantities*
/// histogram samples `$2137` once per master clock across a whole line and reports dots 323 and 327
/// latching **six** times each, every other dot four, and dot 340 **never**. bsnes, ares and Mesen2
/// implement exactly that; snes9x uses 322/326 and is the outlier, so it is not the oracle here.
///
/// # What a cart can and cannot see
///
/// The six-clock dots are **not** cart-observable. Distinguishing a four-clock dot from a six-clock
/// one needs sampling at a rate faster than the dot itself, and the tightest `$2137`/`$213C` loop
/// the 65816 can write is some tens of clocks — the excess is four master clocks in 1364, well
/// under one sample. That half of the row rests on the line still totalling 1364, which the
/// region and refresh-rate tests already pin from the other side.
///
/// The dot *count* is observable, and directly: latch H often enough, across enough lines, and the
/// largest value seen is the last dot that exists.
///
/// | model | maximum `OPHCT` |
/// |---|---:|
/// | uniform `341 × 4` | 340 |
/// | **hardware** | **339** |
///
/// # What is assertable is one-sided, and finding that out cost a design
///
/// The obvious test — sample H a few thousand times and assert the maximum is exactly 339 — is not
/// portable, because **which dots get sampled depends on the core's instruction timing**. The loop
/// samples roughly every fifth dot and relies on its phase drifting between lines to cover the
/// rest; the drift is `1364 mod (loop period)`, and 1364 factors as `2² × 11 × 31`, so a period
/// that shares a large factor with it covers only a sparse lattice forever. Measured: RustySNES
/// reaches 339, Mesen2 338, snes9x 332 — three different answers from three cores that agree about
/// the dot count.
///
/// So the assertion is one-sided: **no sample may ever exceed 339**. Reaching 340 proves a dot 340
/// exists; failing to reach 339 proves nothing either way. That asymmetry is real rather than a
/// concession — the defect this guards against is an extra dot, and an extra dot can only ever show
/// up as a reading that is too *high*.
///
/// | model | can this test see it? |
/// |---|---|
/// | uniform `341 × 4` | yes — a sample lands on 340 and the assertion fails |
/// | hardware `338 × 4 + 2 × 6` | passes, at whatever maximum the lattice reaches |
///
/// The loop still jitters, one iteration in two executing an extra `NOP`, because wider coverage
/// makes the *positive* detection more likely even though it cannot be guaranteed. The first
/// version did not, its period was evidently a divisor of 1364, and the maximum came back 336.
///
/// The lower guard is deliberately loose — 300, not 335 — and bounded only from below. It exists to
/// catch a run that never reached hblank at all, not to pin the maximum, and a guard of `335..=339`
/// would make the assertion below unable to fire: a core reporting 340 would trip the *guard*, and
/// the failure would read as "the sampling never reached the end of a line" when the sampling was
/// fine and the model was not. The injection said exactly that before this was fixed.
fn b2_01() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Sample H repeatedly and keep the largest value seen. The loop period does not divide a");
    a.c("line, so the phase drifts and every dot is reached within a few lines.");
    a.l("lda #$0000");
    a.l("sta f:$7E0220     ; the running maximum");
    a.l("ldy #$0800        ; 2048 samples, spanning well over a hundred lines");
    a.label("smax");
    a.l("sep #$20");
    a.l("lda $213F         ; reset the counter read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213C         ; H low");
    a.l("xba");
    a.l("lda $213C");
    a.l("and #$01          ; bit 0 is H bit 8; bits 1-7 are PPU2 open bus");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
    a.l("cmp f:$7E0220");
    a.l("bcc :+");
    a.l("sta f:$7E0220");
    a.l(":");
    a.c("Jitter the loop. A fixed period is a hazard here: 1364 factors as 2^2 x 11 x 31, and the");
    a.c(
        "first version's period was evidently one of its divisors -- the phase never drifted, dots",
    );
    a.c("337-339 were never sampled, and the maximum came back 336. Alternating two periods makes");
    a.c("the two-iteration period land off every divisor whatever the one-iteration period is.");
    a.l("tya");
    a.l("and #$0001");
    a.l("beq :+");
    a.l("nop");
    a.l(":");
    a.l("dey");
    a.l("bne @smax");
    a.l("lda f:$7E0220");
    a.record(230, "B2.01 the largest H the counter ever latched");
    a.c("The guard: a run that never sampled the end of a line would report a small maximum and");
    a.c("pass the assertion below for entirely the wrong reason.");
    a.l("lda f:$7E0220");
    a.assert_a16_range(
        300,
        0x1FF,
        "the largest H latched over two thousand samples was below 300, so the sampling never \
         reached hblank and says nothing about which dots exist",
    );
    a.c("And nothing may exceed 339. Reaching 340 proves a dot hardware never reports; not");
    a.c("reaching 339 proves nothing, which is why this is asserted in one direction only.");
    a.l("lda f:$7E0220");
    a.assert_a16_range(
        0,
        339,
        "the H counter latched a value above 339, so the model has a dot hardware never reports — \
         fullsnes' latch histogram records dot 340 latching zero times — and the line's 1364 \
         clocks are being spread over 341 uniform dots instead of 340 with 323 and 327 taking six",
    );
    a.finish(
        "B2.01",
        'B',
        "No dot above 339",
        Provenance::Corroborated(
            "fullsnes' PPU H-Counter-Latch Quantities histogram, a direct hardware measurement: \
             dots 323 and 327 latch six times, dot 340 never. bsnes, ares and Mesen2 all implement \
             it; snes9x uses 322/326 and is the outlier",
        ),
        Kind::Scored,
        None,
    )
}

/// Where an H-IRQ actually fires, measured either side of the long dots — a golden vector.
///
/// # This exists to guard `T-06-A`, and must be blessed before that change lands
///
/// `T-06-A` replaces the uniform 4-clocks-per-dot model with hardware's: 340 dots per line, of
/// which 323 and 327 are 6 clocks. It also has to move the H-IRQ comparator into the clock domain,
/// because `HIRQ_TRIGGER_DELAY` is a *dot-domain rounding* of ares' clock-domain compare
/// (`hcounter(10) == (HTIME+1) << 2`, i.e. `HTIME + 3.5` dots) and is exact only while every dot is
/// 4 clocks.
///
/// **Nothing currently covers raster-IRQ position**, so that change would pass its own acceptance
/// criteria — no scene moves, no timing test regresses — while silently shifting every H-IRQ by up
/// to a dot. This records the position at an `HTIME` below the long dots and one above, so the pair
/// straddles the boundary and the before/after is a fact rather than a hope.
///
/// # Why not `B4.07`, and why a handler rather than a poll
///
/// `B4.07` reports H in **32-dot buckets**, and its own doc says why: *"the `$4211` poll loop is
/// coarser than the dot the comparator fires on"*. A shift of up to 4 dots does not move a 32-dot
/// bucket, and a polling companion would inherit the same blindness. The handler path latches H
/// within a few cycles of the interrupt being taken, which is fine enough — it is what `B4.14`
/// already uses to measure dispatch latency, and both readings here are recorded **raw** rather
/// than bucketed.
///
/// Golden rather than scored: the exact latched dot is what `T-06-A` is about to change, and no
/// source pins it at single-dot precision. The variant is only whether both readings arrived, so
/// the row announces a core that stops firing at all; the numbers themselves live in the
/// measurement channel.
fn b4_16() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
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
    a.l("rep #$30");
    a.l("pla");
    a.l("rti");
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");
    a.l("rep #$20");
    a.l("lda #@handler");
    a.l("sta a:V_IRQ_VEC");
    a.l("sep #$20");
    a.c("--- below the long dots: HTIME = 100 ---");
    arm_h_irq(&mut a, 100);
    a.l("cli");
    a.label("spin1");
    a.repeat(4, &["nop"]);
    a.l("lda f:$7E0134");
    a.l("beq @spin1");
    a.l("sei");
    a.l("rep #$20");
    a.l("lda f:$7E0136");
    a.record(126, "B4.16 H latched, HTIME=100 (below dots 323/327)");
    a.l("sep #$20");
    a.c("--- above both long dots: HTIME = 330 ---");
    arm_h_irq(&mut a, 330);
    a.l("cli");
    a.label("spin2");
    a.repeat(4, &["nop"]);
    a.l("lda f:$7E0134");
    a.l("beq @spin2");
    a.l("sei");
    a.l("rep #$20");
    a.l("lda f:$7E0136");
    a.record(127, "B4.16 H latched, HTIME=330 (above dots 323/327)");
    a.c("Disarm before reporting.");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.c("Variant 1 = both readings arrived. The numbers are the point and live in the channel;");
    a.c("the verdict only announces a core whose H-IRQ stopped firing at one of the two.");
    a.l("lda #$03");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B4.16",
        'B',
        "H-IRQ position (golden)",
        Provenance::Contested(
            "no source pins the fired dot at single-dot precision; recorded as the before/after \
             guard for T-06-A's clock-domain comparator change",
        ),
        Kind::Golden,
        None,
    )
}

/// No IRQ triggers for dot 153 on the last scanline of a frame — a golden vector.
///
/// superfamicom.org's timing page states it outright: *"no IRQ will trigger for dot 153 on the
/// short scanline in non-interlace mode, and no IRQ will trigger for dot 153 on the last scanline
/// of any frame."* It gives no mechanism, and neither does fullsnes, which is where the wiki's
/// timing text comes from — so despite reading like two sources this is **one**.
///
/// # Why it is recorded and not scored, and why the emulator was not changed to match
///
/// Nothing implements it. ares, bsnes, Mesen2 and snes9x were all searched for the exception and
/// none of the four carries it; the dossier independently lists "the two no-IRQ-at-dot-153
/// exceptions" among the behaviours with no public test-ROM coverage. So a scored assertion would
/// fail on every core including this one, and the diagnostic rule says a result like that is either
/// a broken test or a behaviour nobody models — here, the second.
///
/// Making RustySNES honour it was the other option and was rejected. It would rest on a
/// single-source claim that no hardware test verifies, it would make this core the only one
/// suppressing the interrupt, and the cost of being wrong is a *missing* interrupt in real games —
/// the expensive direction to get wrong. Recording the observation costs nothing, covers the row
/// honestly, and is exactly the evidence that would justify the change if someone confirms it on
/// hardware.
///
/// # The controls are what make the recording mean something
///
/// "No interrupt at (last line, dot 153)" is worthless on its own — a core that cannot raise an
/// HV-IRQ at all produces the same silence. Two controls run alongside it, both of which must fire:
///
/// * **dot 153 one line earlier**, which proves dot 153 is not itself unreachable; and
/// * **dot 100 on the last line**, which proves the last line is not simply inert.
///
/// Only when both fire does the first reading say anything, and the verdict reports that as its own
/// variant rather than folding an inconclusive run in with a real observation.
///
/// The last line is *measured*, not assumed: it is 261 on NTSC and 311 on PAL, the battery ships
/// both images, and `B2.10` establishes that the region bit is too contested to branch on.
fn b4_11() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");
    a.l("stz $4200");
    a.l("lda $4211");
    a.c("Measure the last line rather than assuming it: 261 on NTSC, 311 on PAL, and the region");
    a.c("bit is too contested (B2.10) to branch on.");
    measure_frame_height(&mut a);
    a.l("sta f:$7E0172     ; the last line of a frame");
    a.l("dec a");
    a.l("sta f:$7E0174     ; and the one before it, for the control");
    a.c("--- the assertion: dot 153 on the last scanline ---");
    arm_hv_irq_var(&mut a, 153, 0x7E_0172);
    irq_within_frames(&mut a, 3, "e");
    a.l("sep #$20");
    a.l("lda f:$7E0170");
    a.l("sta f:$7E0177");
    a.c("--- control: the same dot one line earlier must fire ---");
    arm_hv_irq_var(&mut a, 153, 0x7E_0174);
    irq_within_frames(&mut a, 3, "f");
    a.l("sep #$20");
    a.l("lda f:$7E0170");
    a.l("sta f:$7E0178");
    a.c("--- control: a different dot on the same last line must fire ---");
    arm_hv_irq_var(&mut a, 100, 0x7E_0172);
    irq_within_frames(&mut a, 3, "g");
    a.l("sep #$20");
    a.l("lda f:$7E0170");
    a.l("sta f:$7E0179");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.c("Publish all three readings: the verdict compresses them, the channel keeps them.");
    a.l("rep #$30");
    a.l("lda f:$7E0172");
    a.record(77, "B4.11 the measured last line of a frame");
    a.l("lda f:$7E0177");
    a.l("and #$00FF");
    a.record(
        78,
        "B4.11 fired at dot 153 on the last line (0 = suppressed)",
    );
    a.l("lda f:$7E0178");
    a.l("and #$00FF");
    a.record(79, "B4.11 control: fired at dot 153 one line earlier");
    a.l("lda f:$7E0179");
    a.l("and #$00FF");
    a.record(80, "B4.11 control: fired at dot 100 on the last line");
    a.c("Either control staying silent means the HV-IRQ path never worked here, and the reading");
    a.c("above is not evidence of a suppression.");
    a.l("sep #$30");
    a.l("lda f:$7E0178");
    a.l("beq :+");
    a.l("lda f:$7E0179");
    a.l("beq :+");
    a.l("lda f:$7E0177");
    a.l("beq :++");
    a.l("lda #$03          ; variant 1 = it fired; the exception is not modelled here");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("lda #$07          ; variant 3 = a control did not fire; inconclusive");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("lda #$05          ; variant 2 = suppressed at dot 153 while both controls fired");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B4.11",
        'B',
        "Dot 153, last line",
        Provenance::Contested(
            "superfamicom.org's timing page states the exception and gives no mechanism; its \
             timing text derives from fullsnes, so the two are one source, and no test ROM \
             verifies it. ares, bsnes, Mesen2 and snes9x were each searched and none implements it",
        ),
        Kind::Golden,
        None,
    )
}

/// Arm an HV-IRQ at a constant `HTIME` and a `VTIME` held in a 24-bit variable.
///
/// `VTIME` comes from memory because the line under test is the *measured* last line of a frame,
/// which differs by region and which `B2.10` forbids deriving from the region bit.
fn arm_hv_irq_var(a: &mut Asm, htime: u16, vtime_addr: u32) {
    assert!(htime < 340, "HTIME {htime} is past the end of a scanline");
    a.l("sep #$20");
    a.l(&format!("lda #${:02X}", htime & 0xFF));
    a.l("sta $4207");
    a.l(&format!("lda #${:02X}", htime >> 8));
    a.l(&format!("sta $4208         ; HTIME = {htime}"));
    a.l(&format!("lda f:${vtime_addr:06X}"));
    a.l("sta $4209");
    a.l(&format!("lda f:${:06X}", vtime_addr + 1));
    a.l("and #$01");
    a.l("sta $420A         ; VTIME from the measured line");
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$30");
    a.l("sta $4200         ; both comparators, so the match is one dot on one line");
}

/// `HTIME` and `VTIME` are 9-bit, and a value past the end of the counter simply never matches.
///
/// `$4207`/`$4208` hold `HTIME` and `$4209`/`$420A` hold `VTIME`, nine bits each — so both accept
/// values up to 511 while the counters they are compared against top out at 339 dots and 261 (NTSC)
/// or 311 (PAL) lines. The assertion is that the surplus range is *inert*: the comparator never
/// matches, and no interrupt ever arrives.
///
/// # Pinning the negative
///
/// "No interrupt arrived" is the weakest kind of observation — a core with a broken timer, a masked
/// enable, or a comparator that never fires produces exactly the same silence. Two things make it
/// mean something here.
///
/// **A positive control precedes each half.** `HTIME = 100` and `VTIME = 100` are armed the same
/// way through the same poll, and each must fire. If the machinery is broken the control fails
/// first, and the silence that follows is never mistaken for evidence.
///
/// **Both plausible wrong answers are loud.** A core that keeps only the low eight bits of the
/// register arms at `400 & $FF` = **144**, which every scanline and every frame reaches. A core
/// that reduces the value modulo the line length arms at `400 - 341` = **59**. Neither is quiet:
/// both fire, and fire often, so the failure is an interrupt that should not exist rather than a
/// subtle position error.
///
/// # Why the wait is bounded
///
/// Every other timer test in this group spins until its interrupt appears, which is exactly wrong
/// for an assertion whose expected outcome is *nothing*. The poll here counts vblank edges and
/// gives up after a fixed number of frames, so "never fires" is a finite observation instead of a
/// hang and a battery-level timeout.
fn b4_13() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");
    a.l("stz $4200");
    a.l("lda $4211         ; clear any latch an earlier test left");
    a.c("--- control: HTIME = 100 is squarely in range and must fire ---");
    arm_h_timer_raw(&mut a, 100);
    irq_within_frames(&mut a, 2, "a");
    a.l("sep #$20");
    a.l("lda f:$7E0170");
    a.assert_a8(
        0x01,
        "no H-IRQ arrived with HTIME = 100, which is in range — so the out-of-range check that \
         follows would report silence for the wrong reason",
    );
    a.c("--- HTIME = 400 is past the end of every scanline and must never match ---");
    arm_h_timer_raw(&mut a, 400);
    irq_within_frames(&mut a, 3, "b");
    a.l("sep #$20");
    a.l("lda f:$7E0170");
    a.assert_a8(
        0x00,
        "an H-IRQ fired with HTIME = 400, which no scanline reaches: a core keeping only the low \
         eight bits arms at 144, one reducing modulo the line length arms at 59",
    );
    a.c("--- control: VTIME = 100 is in range on both NTSC and PAL and must fire ---");
    arm_v_timer_raw(&mut a, 100);
    irq_within_frames(&mut a, 2, "c");
    a.l("sep #$20");
    a.l("lda f:$7E0170");
    a.assert_a8(
        0x01,
        "no V-IRQ arrived with VTIME = 100, which is in range on both regions — the out-of-range \
         check that follows would report silence for the wrong reason",
    );
    a.c("--- VTIME = 400 is past the last line of either region and must never match ---");
    a.c("400 rather than 300: 300 is out of range on NTSC but a real line on PAL, and the battery");
    a.c("ships both images. 400 is beyond 261 and 311 alike, so one assertion covers both.");
    arm_v_timer_raw(&mut a, 400);
    irq_within_frames(&mut a, 3, "d");
    a.l("sep #$20");
    a.l("lda f:$7E0170");
    a.assert_a8(
        0x00,
        "a V-IRQ fired with VTIME = 400, which is past the last line of either region: a core \
         keeping only the low eight bits arms at 144",
    );
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.finish(
        "B4.13",
        'B',
        "Timer range is 9-bit",
        Provenance::Documented(
            "fullsnes $4207-$420A: HTIME is 0-339 and VTIME 0-261 (NTSC) / 0-311 (PAL), both held \
             in nine bits",
        ),
        Kind::Scored,
        None,
    )
}

/// Arm the H timer at an arbitrary nine-bit `HTIME`, including values off the end of a scanline.
///
/// Deliberately separate from [`arm_h_irq`], which asserts its argument is a real dot and should
/// keep doing so — every other caller wants that guard. This one exists for `B4.13`, whose whole
/// subject is what the hardware does with a value that is *not* a real dot.
fn arm_h_timer_raw(a: &mut Asm, htime: u16) {
    a.l("sep #$20");
    a.l(&format!("lda #${:02X}", htime & 0xFF));
    a.l("sta $4207");
    a.l(&format!("lda #${:02X}", htime >> 8));
    a.l(&format!("sta $4208         ; HTIME = {htime}"));
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$10");
    a.l("sta $4200         ; H-IRQ only");
}

/// Arm the V timer at an arbitrary nine-bit `VTIME`, including values past the last line.
fn arm_v_timer_raw(a: &mut Asm, vtime: u16) {
    a.l("sep #$20");
    a.l(&format!("lda #${:02X}", vtime & 0xFF));
    a.l("sta $4209");
    a.l(&format!("lda #${:02X}", vtime >> 8));
    a.l(&format!("sta $420A         ; VTIME = {vtime}"));
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$20");
    a.l("sta $4200         ; V-IRQ only");
}

/// Emit a bounded wait for an IRQ: poll `$4211` while counting `frames` vblank edges.
///
/// Leaves `$7E0170` as 1 if the flag was ever seen and 0 if it was not. `tag` disambiguates the
/// cheap-local labels, which are proc-scoped, so several waits can share one test body.
///
/// The frame counting is what makes a negative result reportable. Spinning until the interrupt
/// appears — what every other timer test here does — turns "it never fires" into a battery timeout
/// with no per-test verdict, which is a different and much less useful failure.
fn irq_within_frames(a: &mut Asm, frames: u8, tag: &str) {
    a.c("Bounded wait: poll $4211 while counting vblank edges.");
    a.l("sep #$30");
    a.l("lda #$00");
    a.l("sta f:$7E0170     ; the fired flag (STZ has no long form)");
    a.l(&format!("ldx #{frames}"));
    a.label(&format!("out{tag}"));
    a.label(&format!("act{tag}"));
    a.l("lda $4211");
    a.l("and #$80");
    a.l(&format!("bne @hit{tag}"));
    a.l("lda $4212");
    a.l("and #$80");
    a.l(&format!(
        "bne @act{tag}     ; still in vblank; wait for active display"
    ));
    a.label(&format!("vbl{tag}"));
    a.l("lda $4211");
    a.l("and #$80");
    a.l(&format!("bne @hit{tag}"));
    a.l("lda $4212");
    a.l("and #$80");
    a.l(&format!(
        "beq @vbl{tag}     ; wait for the next vblank edge"
    ));
    a.l("dex");
    a.l(&format!("bne @out{tag}"));
    a.l(&format!("bra @done{tag}"));
    a.label(&format!("hit{tag}"));
    a.l("lda #$01");
    a.l("sta f:$7E0170");
    a.label(&format!("done{tag}"));
}

/// The DRAM refresh pause, probed by the tight H-counter loop `B3.03` names -- a golden vector.
///
/// The 5A22 stops the CPU once per scanline to refresh WRAM. `B3.01` puts the pause at 40 master
/// clocks, `B3.02` at clock 538 on the first line and thereafter at the multiple of 8 nearest 536
/// after the previous one, and `B3.03` says the way to see it is a tight loop reading the H
/// counter. This test is that loop.
///
/// # Why it records rather than asserts
///
/// Three separate reasons, any one of which would be enough:
///
/// * `docs/accuracy-ledger.md` scopes refresh out of RustySNES *empirically* -- measured over 500
///   frames across 3 ROMs, the CPU-driven model already produces the correct ~357,368-clock NTSC
///   frame, so bolting on an extra stall would make frame length wrong.
/// * ares says in its own source that its refresh pattern is "technically" wrong and only averages
///   out (`sfc/cpu/timing.cpp:23`). A reference that disclaims itself is not an oracle.
/// * The loop's own period is far coarser than the pause. Each iteration costs four bus accesses
///   from an 8-clock bank plus the index work -- about 60 dots -- so with three intervals the
///   pause is resolved as an outlier, nowhere near the multiple-of-8 precision `B3.02` states.
///
/// So the numbers go to the measurement channel and the verdict only says which shape was seen.
/// What makes that worth having: **the shape is discriminating even though the position is not.**
/// A core modelling the pause shows one interval about ten dots longer than the others; a core
/// modelling none shows a flat sequence. That is a yes/no about a whole subsystem, from three
/// subtractions.
///
/// # The window has to stay inside one line
///
/// A first version stored only the low byte of H, which is a whole access cheaper per sample. It
/// did not survive contact: the window is ~60 dots per sample, so it reached past dot 255, the low
/// byte wrapped, and one delta came back as a large positive number that looked exactly like a
/// pause. Storing the full 9-bit counter turns that failure into a **decreasing** sample and hence
/// an impossible delta, which the test reports as variant 3 -- measurement invalid -- rather than
/// as evidence. The sync loop reads both bytes for the same reason: the low byte alone cannot tell
/// dot 8 from dot 264.
fn b3_01() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei");
    a.l("stz $4200         ; no NMI and no timer IRQs");
    a.l("stz $420C         ; no HDMA -- the only other thing that steals CPU cycles per line");
    refresh_sync_to_line_start(&mut a);
    refresh_sample_window(&mut a);
    refresh_reduce(&mut a);
    a.c("The window bounds are recorded too: without them a reader cannot tell whether the pause");
    a.c("was inside the sampled span or never had a chance to appear.");
    a.l("lda f:$7E0168");
    a.record(
        139,
        "B3 shortest interval in dots (the stall-free loop period)",
    );
    a.l("lda f:$7E016A");
    a.record(140, "B3 longest interval in dots");
    a.l("lda f:$7E016C");
    a.record(108, "B3 H at the start of the longest interval");
    a.l("lda f:$7E0160");
    a.record(109, "B3 H at the first sample (window start)");
    a.l("lda f:$7E0166");
    a.record(124, "B3 H at the last sample (window end)");
    a.l("lda f:$7E016A");
    a.l("cmp #200");
    a.l("bcc :+");
    a.c("No loop iteration takes 200 dots. A delta that large means the window crossed a line");
    a.c("boundary and the samples ran backwards -- say so rather than report a giant pause.");
    a.l("sep #$20");
    a.l("lda #$07          ; variant 3 = window left the scanline, measurement invalid");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("rep #$30");
    a.l("lda f:$7E016A");
    a.l("sec");
    a.l("sbc f:$7E0168");
    a.l("cmp #4");
    a.l("bcs :+");
    a.l("sep #$20");
    a.l("lda #$03          ; variant 1 = the intervals are flat; no per-line pause is modelled");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("sep #$20");
    a.l("lda #$05          ; variant 2 = one interval stands out; slots 106/107 size it");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "B3.01",
        'B',
        "DRAM refresh pause",
        Provenance::Contested(
            "fullsnes and anomie put the pause at 40 clocks near line-clock 536, but ares' own \
             source calls its refresh pattern technically wrong and only right on average, and \
             docs/accuracy-ledger.md scopes refresh out of RustySNES on the measurement that its \
             frame length is already correct without one",
        ),
        Kind::Golden,
        None,
    )
}

/// Emit [`b3_01`]'s sync: spin until the H counter is near the start of a line.
///
/// Both H bytes are read. The low byte alone cannot tell dot 8 from dot 264, and a window opened
/// at 264 would spend most of its length in the *next* scanline — where a second refresh pause
/// waits, which would make two intervals long and neither of them interpretable.
fn refresh_sync_to_line_start(a: &mut Asm) {
    a.c("Settle on a frame boundary, then walk forward until H is near the start of a line, so");
    a.c("the window that follows has a whole scanline in front of it.");
    a.l("jsr wait_vblank");
    a.l("sep #$20");
    a.l("rep #$10");
    a.label("sync");
    a.l("lda $213F         ; reset the counter read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213C         ; H low");
    a.l("sta f:$7E0160");
    a.l("lda $213C         ; H high");
    a.l("and #$01");
    a.l("bne @sync         ; H >= 256: no room left in this line");
    a.l("lda f:$7E0160");
    a.l("cmp #16");
    a.l("bcs @sync");
}

/// Emit [`b3_01`]'s sampling loop: four readings of the full 9-bit H counter, ~60 dots apart.
///
/// Storing only the low byte would save an access per iteration, and was tried. It does not work:
/// the window reaches past dot 255, the low byte wraps, and the wrapped delta comes back as a
/// large positive number indistinguishable from a pause. Keeping the high bit makes an overrun
/// show up as a *decreasing* sample instead, which [`b3_01`] reports as an invalid measurement.
///
/// `$213C` is read twice per sample; the second read is what returns the address flipflop to the
/// low byte for the next iteration, so it costs nothing beyond the access it already needs.
fn refresh_sample_window(a: &mut Asm) {
    a.c("Four samples of the full 9-bit counter, into $7E0160..$7E0167 as 16-bit words.");
    a.l("ldx #$0000");
    a.label("sloop");
    a.l("lda $2137         ; latch");
    a.l("lda $213C         ; H low");
    a.l("sta f:$7E0160,x");
    a.l("lda $213C         ; H high");
    a.l("and #$01          ; bits 1-7 are PPU2 open bus");
    a.l("sta f:$7E0161,x");
    a.l("inx");
    a.l("inx");
    a.l("cpx #$0008");
    a.l("bne @sloop");
}

/// Emit [`b3_01`]'s reduction: the shortest interval, the longest, and where the longest starts.
///
/// A stall-free loop gives three identical intervals, so the pause — if the core models one — adds
/// its whole length to exactly one of them. `max - min` is therefore the pause and the sample the
/// longest interval starts from is as close to its position as this method can get.
fn refresh_reduce(a: &mut Asm) {
    a.c("Difference the samples.");
    a.l("rep #$30");
    a.l("lda #$FFFF");
    a.l("sta f:$7E0168     ; running minimum");
    a.l("lda #$0000");
    a.l("sta f:$7E016A     ; running maximum");
    a.l("sta f:$7E016C     ; H at the start of the longest interval");
    a.l("ldx #$0000");
    a.label("dloop");
    a.l("lda f:$7E0162,x");
    a.l("sec");
    a.l("sbc f:$7E0160,x");
    a.l("cmp f:$7E0168");
    a.l("bcs :+");
    a.l("sta f:$7E0168");
    a.l(":");
    a.l("cmp f:$7E016A");
    a.l("bcc :+");
    a.l("beq :+");
    a.l("sta f:$7E016A");
    a.l("lda f:$7E0160,x   ; the sample the longest interval starts from");
    a.l("sta f:$7E016C");
    a.l(":");
    a.l("inx");
    a.l("inx");
    a.l("cpx #$0006");
    a.l("bne @dloop");
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
    a.record(133, "H position when the H-IRQ was observed");
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
