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
    a.l("jmp test_restore");
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
    a.l("jmp test_restore");
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

/// Emit: latch the counters and leave the 9-bit V position in a 16-bit accumulator.
fn read_v(a: &mut Asm) {
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
fn b2_04() -> Test {
    let mut a = Asm::new();
    a.c("Start at vblank, poll V until it wraps to the top of the next frame, keep the maximum.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("jsr wait_vblank   ; V is now at the first vblank line");
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
    a.l("lda $4211         ; immediately again, still on line 100");
    a.l("sta f:$7E0124");
    a.l("stz $4200         ; disarm before asserting");
    a.l("lda $4211");
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
