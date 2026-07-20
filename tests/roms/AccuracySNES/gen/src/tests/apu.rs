//! Group E — the SPC700 and S-DSP (ticket **T-04-E**).
//!
//! Every test here has the same shape, because the SPC700 is only reachable through four bytes:
//! the cart uploads a small SPC700 program via the IPL boot handshake (`apu_upload` in
//! `asm/runtime.s`), waits for the program to publish a done marker on port 0, and reads its
//! answers off the other three ports. That is what a game's sound driver does at boot, which is
//! why the IPL ROM exists at all.
//!
//! The programs themselves are assembled by `gen/src/spc.rs` — `ca65` does not speak SPC700.
//!
//! **Never hand-write a verdict byte.** Use the assertion helpers, even when the condition does
//! not look like an equality — `assert_a16_range` covers "must not be this value" perfectly well.
//! A hand-written `sta V_TEST_RESULT` puts a failure code in the ROM that the generated
//! `ERROR_CODES.md` cannot know about, so the table silently stops being a complete account of
//! what a failure byte means. This has been got wrong twice in this file; the helpers exist
//! precisely so it cannot be.
//!
//! **Reading `PSW` is the recurring trick.** Several of these assertions are about which flags an
//! instruction sets, and the SPC700 has no "read flags" instruction. `PUSH PSW` / `POP A` does it,
//! but only if nothing between the instruction under test and the push disturbs the flags — which
//! is why the result registers are captured with `MOV dp,A` and `MOV dp,Y`, the two moves that
//! leave flags alone.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::dsl::{Asm, Kind, Provenance, Test};
use crate::spc::{DONE, PORT0, PORT1, PORT2, PORT3, RELEASE, Spc};

/// Hands out a unique suffix for each uploaded SPC700 image's label.
///
/// The images live in a shared segment, so their labels have to be globally unique; a counter is
/// enough because generation is single-threaded and runs the tests in a fixed order. The number
/// means nothing beyond "not the same as the last one".
fn next_prog_id() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Every Group E test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        e1_01(),
        e1_02(),
        e1_04(),
        e1_05(),
        e1_06(),
        e1_13(),
        e1_15(),
        e3_01(),
        e3_11(),
        dsp_addressing(),
        e2_01(),
        e2_05(),
        e3_14(),
        dsp_global_regs(),
        e9_19(),
        e5_07(),
        e5_08(),
        e5_09(),
        e5_11(),
        e7_10(),
        e1_08(),
        e2_08(),
        e2_09(),
        e3_03(),
        e3_04(),
        e3_05(),
        e3_10(),
        e1_10(),
        e1_12(),
        e2_07(),
        e4_01(),
        e4_02(),
        e4_04(),
        e5_02(),
        e9_04(),
        e9_18(),
        e5_03(),
        e5_04(),
        e5_05(),
        e7_01(),
        e7_08(),
        e7_11(),
        e7_14(),
        e7_15(),
    ]
}

/// Emit the cart-side half: upload `prog`, wait for its done marker, leave port values readable.
///
/// The wait is bounded by a counter rather than spinning forever. An APU that never boots is a
/// real failure mode — it is the one thing here the cart cannot recover from — and a test that
/// hangs takes the whole battery with it, reporting nothing at all about the other tests.
///
/// **Register widths on exit: `A` 8-bit, `X`/`Y` 16-bit**, on the path that reaches `@ran`. The
/// caller's `.a8`/`.a16` directives come from its own `sep`/`rep` lines and a helper call is not
/// one of those, so an undocumented width here would have the assembler and the CPU disagreeing
/// about the size of the next immediate — and every instruction after it shifted.
fn upload_and_run(a: &mut Asm, prog: &Spc) {
    // The image goes in the out-of-bank data segment, not inline in the test body: these are
    // several hundred bytes each and bank $00 is finite. `apu_upload` takes a 24-bit pointer
    // anyway, so nothing about the upload cares where it lives.
    let label = format!("apu_prog_{}", next_prog_id());
    a.d(&format!("{label}:"));
    for line in prog.as_ca65("    ").lines() {
        a.d(line);
    }
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Point apu_upload at this test's own program image, which lives in another bank.");
    a.l(&format!("lda #.loword({label})"));
    a.l("sta f:V_APU_SRC");
    a.l("sep #$20");
    a.l(&format!("lda #^{label}"));
    a.l("sta f:V_APU_BANK");
    a.l("rep #$30");
    a.l(&format!("lda #{}", prog.bytes().len()));
    a.l("sta f:V_APU_LEN");
    a.l("lda #$0200");
    a.l("sta f:V_APU_DEST     ; APU RAM $0200: clear of the zero page and the stack");
    a.l("lda #$0200");
    a.l("sta f:V_APU_ENTRY");
    a.l("jsl apu_upload_far");
    a.c("Clear the CPU-side port 0 before the program can look at it. The previous test left the");
    a.c("release byte there, and a program whose release loop sees it immediately jumps back to");
    a.c("the IPL before the cart has read a thing — which reads as a wrong answer, not a race.");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta APUIO0");
    a.c("Wait for the program's done marker, but not forever: an APU that never boots would");
    a.c("otherwise hang the whole battery and report nothing about any other test.");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label("wait");
    a.l("sep #$20");
    a.l("lda APUIO0");
    a.l(&format!("cmp #${DONE:02X}"));
    a.l("beq @ran");
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$8000");
    a.l("bne @wait");
    a.l("bra @timeout");
    a.label("ran");
    a.c(
        "Copy the answers out BEFORE releasing the program: once it jumps to the IPL, the boot ROM",
    );
    a.c("overwrites ports 0 and 1 with its $AA/$BB announcement.");
    a.l("sep #$20");
    a.l("lda APUIO1");
    a.l("sta f:$7E0100");
    a.l("lda APUIO2");
    a.l("sta f:$7E0101");
    a.l("lda APUIO3");
    a.l("sta f:$7E0102");
    a.c("Release: the program hands the APU back to the IPL so the NEXT test can upload at all.");
    a.l(&format!("lda #${RELEASE:02X}"));
    a.l("sta APUIO0");
}

/// Emit the shared tail: jump past the timeout arm, then land where `finish`'s pass stub follows.
///
/// Every test in this group needs it because `upload_and_run` branches to `@timeout` when the APU
/// never answers, and that arm has to record SKIP and leave — a test whose APU did not boot has
/// asserted nothing, and reporting a pass would be a lie about the only thing it was measuring.
fn apu_timeout_arm(a: &mut Asm) {
    a.l("bra @pass");
    a.label("timeout");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("sta f:V_TEST_RESULT   ; SKIP: the APU never published a done marker");
    a.l("jml test_restore");
    a.label("pass");
}

/// `MUL YA` takes its N and Z flags from `Y` alone.
///
/// With `Y = $10` and `A = $10` the product is `$0100`, so `A` ends at `$00` — and yet `Z` is
/// **clear**, because the flags describe the high byte only. A core that sets `Z` from the 16-bit
/// result, or from `A`, gets this exactly backwards, and the failure is invisible to any test that
/// only checks the product.
fn e1_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x10)
        .mov_y_imm(0x10)
        .mul_ya()
        .mov_dp_a(PORT2) // product low, before anything can touch the flags
        .mov_dp_y(PORT3) // product high
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1) // PSW
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Product first: $10 * $10 = $0100.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(0x00, "MUL YA low byte is wrong");
    a.l("lda f:$7E0102");
    a.assert_a8(0x01, "MUL YA high byte is wrong");
    a.c("Then the flags. Z is bit 1 of PSW and must be CLEAR even though A came out $00.");
    a.l("lda f:$7E0100");
    a.l("and #$02");
    a.assert_a8(
        0x00,
        "MUL YA set Z although Y is non-zero — the flags come from Y alone, not from A or YA",
    );
    a.c("N is bit 7, and $01 is positive, so it must be clear too.");
    a.l("lda f:$7E0100");
    a.l("and #$80");
    a.assert_a8(0x00, "MUL YA set N although Y is $01");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.01",
        'E',
        "MUL YA flags from Y",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `DIV YA,X` on its normal branch: `A` is the quotient, `Y` the remainder.
///
/// The baseline the rest of `E1.02`-`E1.07` are read against. `$0020 / $08` is 4 remainder 0 — a
/// case with no overflow, no odd flag behaviour, and nothing to argue about, which is exactly what
/// makes it worth pinning first: every stranger `DIV` assertion is a deviation from this one.
fn e1_02() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_y_imm(0x00)
        .mov_a_imm(0x20) // YA = $0020
        .mov_x_imm(0x08)
        .div_ya_x()
        .mov_dp_a(PORT2) // quotient
        .mov_dp_y(PORT3) // remainder
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(0x04, "DIV YA,X quotient is wrong ($0020 / $08 = 4)");
    a.l("lda f:$7E0102");
    a.assert_a8(0x00, "DIV YA,X remainder is wrong");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.02",
        'E',
        "DIV YA,X normal branch",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `DIV YA,X` takes N and Z from the quotient alone, not from the remainder.
///
/// `$0003 / $08` is quotient 0, remainder 3 — so `Z` is **set** even though `Y` came back
/// non-zero. The errata matters because the remainder is the more interesting half of a divide,
/// and a core that flags the pair, or flags `Y`, reports "non-zero" for a result that is zero.
///
/// The companion case is checked in the same program: `$0020 / $08` is quotient 4, remainder 0,
/// where `Z` must be **clear**. One direction alone would pass on a core that never sets `Z`.
fn e1_06() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // quotient 0, remainder 3 -> Z set
        .mov_y_imm(0x00)
        .mov_a_imm(0x03)
        .mov_x_imm(0x08)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        // quotient 4, remainder 0 -> Z clear
        .mov_y_imm(0x00)
        .mov_a_imm(0x20)
        .mov_x_imm(0x08)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Quotient 0 with remainder 3: Z (bit 1) must be SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$02");
    a.assert_a8(
        0x02,
        "DIV YA,X left Z clear for a zero quotient — the flags come from the quotient, not the \
         remainder",
    );
    a.c("Quotient 4 with remainder 0: Z must be CLEAR. Without this half, a core that never sets");
    a.c("Z at all would pass the check above.");
    a.l("lda f:$7E0102");
    a.l("and #$02");
    a.assert_a8(0x00, "DIV YA,X set Z for a non-zero quotient");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.06",
        'E',
        "DIV flags from quotient",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `MOVW YA,dp` sets N and Z from the whole 16-bit value.
///
/// Loading `$0100` gives `A = $00` and `Y = $01`, and `Z` must be **clear** — a core that flags
/// the accumulator alone sets it. Loading `$8000` gives `A = $00` and `Y = $80`, and `N` must be
/// **set** — the same core leaves it clear. The two cases together pin both flags to the 16-bit
/// value rather than to either byte of it.
fn e1_15() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0x10, 0x00)
        .mov_dp_imm(0x11, 0x01) // $10/$11 = $0100
        .movw_ya_dp(0x10)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        .mov_dp_imm(0x12, 0x00)
        .mov_dp_imm(0x13, 0x80) // $12/$13 = $8000
        .movw_ya_dp(0x12)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("$0100: A is $00, so a core flagging the accumulator alone sets Z. It must be clear.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$02");
    a.assert_a8(
        0x00,
        "MOVW YA,dp set Z for $0100 — the flags describe all sixteen bits, not the low byte",
    );
    a.c("$8000: A is again $00, and N must be SET from bit 15.");
    a.l("lda f:$7E0102");
    a.l("and #$80");
    a.assert_a8(0x80, "MOVW YA,dp left N clear for $8000");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.15",
        'E',
        "MOVW YA sets 16-bit N/Z",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `DIV`'s H flag is a nibble comparison, not a half-carry.
///
/// It is set from `(Y & 15) >= (X & 15)` on the **inputs**, which has nothing to do with any carry
/// the division produces — the name is borrowed and the behaviour is not. Two divides that differ
/// only in which operand has the larger low nibble pin it: `Y=$05, X=$03` sets `H`, and the same
/// pair swapped clears it. A core computing a genuine half-carry gets no consistent answer at all,
/// because there is no half-carry in a division to compute.
fn e1_04() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // (Y & 15) >= (X & 15): 5 >= 3 -> H set
        .mov_y_imm(0x05)
        .mov_a_imm(0x00)
        .mov_x_imm(0x03)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        // 3 >= 5 is false -> H clear
        .mov_y_imm(0x03)
        .mov_a_imm(0x00)
        .mov_x_imm(0x05)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("H is bit 3 of PSW. Y=$05 against X=$03: the low nibbles compare 5 >= 3, so H is SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$08");
    a.assert_a8(
        0x08,
        "DIV left H clear although (Y & 15) >= (X & 15) — H here is a nibble compare, not a carry",
    );
    a.c("Swap the operands and the comparison fails, so H must be CLEAR.");
    a.l("lda f:$7E0102");
    a.l("and #$08");
    a.assert_a8(0x00, "DIV set H although (Y & 15) < (X & 15)");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.04",
        'E',
        "DIV H = nibble compare",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `DIV`'s V flag is bit 8 of the quotient.
///
/// The quotient can exceed 255 — the normal branch only guarantees it is under 512 — so `V` is how
/// the caller learns the byte it was handed is not the whole answer. `$0500 / $03` is 426, which
/// has bit 8 set; `$0300 / $05` is 153, which does not. The two together separate "V tracks the
/// quotient's ninth bit" from "V is set whenever something overflowed", which are the same
/// statement only for the first case.
fn e1_05() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // $0500 / $03 = 426: bit 8 set
        .mov_y_imm(0x05)
        .mov_a_imm(0x00)
        .mov_x_imm(0x03)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        // $0300 / $05 = 153: bit 8 clear
        .mov_y_imm(0x03)
        .mov_a_imm(0x00)
        .mov_x_imm(0x05)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("V is bit 6. Quotient 426 has bit 8 set, so V must be SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$40");
    a.assert_a8(0x40, "DIV left V clear for a quotient of 426 (bit 8 set)");
    a.c("Quotient 153 fits in a byte, so V must be CLEAR.");
    a.l("lda f:$7E0102");
    a.l("and #$40");
    a.assert_a8(
        0x00,
        "DIV set V for a quotient of 153, which fits in eight bits",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.05",
        'E',
        "DIV V is quotient bit 8",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `ADDW` carries into H from bit 11, not from bit 3.
///
/// `H` on the 8-bit adds is the bit-3 carry; on the word adds it is the bit-11 carry, because the
/// flag describes the high byte's low nibble. `$0FFF + $0001` crosses that boundary and `$0100 +
/// $0001` does not. A core that reuses its 8-bit half-carry reports the low byte's carry instead,
/// which is set in neither case here — so the first assertion catches it and the second confirms
/// the flag is not simply stuck.
fn e1_13() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0x10, 0xFF)
        .mov_dp_imm(0x11, 0x0F) // $10/$11 = $0FFF
        .mov_dp_imm(0x12, 0x01)
        .mov_dp_imm(0x13, 0x00) // $12/$13 = $0001
        .mov_dp_imm(0x14, 0x00)
        .mov_dp_imm(0x15, 0x01) // $14/$15 = $0100
        .movw_ya_dp(0x10)
        .addw_ya_dp(0x12) // $0FFF + $0001 = $1000: carries bit 11 -> 12
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        .movw_ya_dp(0x14)
        .addw_ya_dp(0x12) // $0100 + $0001 = $0101: no such carry
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("$0FFF + $0001 crosses bit 11, so H (bit 3 of PSW) must be SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$08");
    a.assert_a8(
        0x08,
        "ADDW left H clear for $0FFF + $0001 — H is the bit-11 carry on the word adds",
    );
    a.c("$0100 + $0001 does not, so H must be CLEAR — which also shows the flag is not stuck.");
    a.l("lda f:$7E0102");
    a.l("and #$08");
    a.assert_a8(0x00, "ADDW set H for $0100 + $0001");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.13",
        'E',
        "ADDW H = bit-11 carry",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Reading a timer counter returns four bits and clears it.
///
/// `$FD`-`$FF` are not registers holding a value; they are counters that a read consumes. The
/// upper nibble is not part of the count and the read has a side effect, so two reads in a row
/// give a number and then zero — which is the entire protocol for using them, and a core that
/// treats them as plain storage returns the same value twice and lets a driver double-count every
/// tick it observes.
///
/// The first read is only required to be non-zero: how far the timer has advanced depends on the
/// delay loop's exact cost, and asserting a specific count would be asserting the loop.
fn e3_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1: the fastest this timer runs
        .mov_dp_imm(0xF1, 0x01) // CONTROL: enable timer 0
        .delay(0x00) // 256 iterations, comfortably several ticks
        // Stop the timer before reading it. The two reads below are about eight cycles apart and a
        // tick at this divider lands every 128, so a tick falling between them is uncommon rather
        // than impossible -- and when it does, the second read is non-zero for a reason that has
        // nothing to do with whether the first one cleared it. It showed up as Mesen2 failing this
        // test on the PAL image only, after an unrelated change shifted the battery's timing.
        // Bit 7 keeps the IPL ROM mapped; see `Spc::release_to_ipl`.
        .mov_dp_imm(0xF1, 0x80)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2) // first read: the accumulated count
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT3) // second read: must be zero, because the first one cleared it
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Both halves of the first read in one assertion: it must have advanced (non-zero, or the");
    a.c("clear check below is vacuous) and it must fit in four bits (the upper nibble is not part");
    a.c("of the count). Expressed through the DSL rather than as hand-written verdict bytes, so");
    a.c("the code and its reason land in the generated ERROR_CODES.md like every other failure.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "the first read of $FD was zero or wider than four bits — a timer counter is a 4-bit \
         value, and a zero here would make the clear check below vacuous",
    );
    a.c("The second read must be zero: reading a timer counter consumes it.");
    a.l("sep #$20");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "the second read of $FD was non-zero — reading a timer counter must clear it",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.01",
        'E',
        "Timer read clears it",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F8` and `$F9` are plain RAM.
///
/// They sit in the middle of the I/O block and are not registers — nothing reads them, nothing
/// writes them, and a program may use them as two spare bytes. Worth pinning precisely because
/// they look like registers: a core that decodes the whole `$F0`-`$FF` range as I/O returns
/// something other than what was stored, and the failure surfaces far from the cause, in whatever
/// used them as scratch.
///
/// **This test was briefly recorded as a Contested golden, and that was wrong.** It appeared to
/// fail on all three implementations, which is this project's signature of a broken test — but the
/// cause was neither the test nor the emulators: an earlier test wrote `$F1` to enable a timer,
/// which also cleared bit 7 and unmapped the IPL ROM, so every APU upload after it silently died.
/// Once the release path re-maps the ROM, all three return what was written. The lesson is that
/// "three-way agreement means the test is wrong" is a good heuristic and not a proof: a harness
/// bug upstream of every implementation produces the same signature.
fn e3_14() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF8, 0x5A)
        .mov_dp_imm(0xF9, 0xA5)
        .mov_a_dp(0xF8)
        .mov_dp_a(PORT2)
        .mov_a_dp(0xF9)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x5A,
        "$F8 did not read back what was written, so it is not behaving as the plain RAM it \
         should be",
    );
    a.l("lda f:$7E0102");
    a.assert_a8(
        0xA5,
        "$F9 did not read back what was written, so it is not behaving as the plain RAM it \
         should be",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.14",
        'E',
        "$F8/$F9 are plain RAM",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F2` bit 7 makes writes through `$F3` do nothing.
///
/// The DSP register file is reached through an address latch (`$F2`) and a data port (`$F3`), and
/// the top bit of the address is not part of the address — it disables writing. A core that masks
/// `$F2` to five bits and ignores bit 7 lets a write through that hardware discards, which is the
/// wrong direction to be wrong in: the value lands, the driver never notices, and the sound is
/// subtly off rather than absent.
///
/// Checked by writing a known value, attempting to overwrite it with the bit set, and reading
/// back. The read is done with the bit clear, so the test cannot pass by the *read* also being
/// suppressed.
fn e3_11() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // MVOLL ($0C) = $7F, the value that must survive
        .mov_dp_imm(0xF2, 0x0C)
        .mov_dp_imm(0xF3, 0x7F)
        // Same register, address bit 7 set: this write must be discarded
        .mov_dp_imm(0xF2, 0x8C)
        .mov_dp_imm(0xF3, 0x00)
        // Read back with the bit clear
        .mov_dp_imm(0xF2, 0x0C)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT2)
        // Control: the same sequence WITHOUT bit 7 must take effect, or the check above would
        // pass on a core that simply never writes the DSP at all.
        .mov_dp_imm(0xF2, 0x0C)
        .mov_dp_imm(0xF3, 0x33)
        .mov_dp_imm(0xF2, 0x0C)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("The suppressed write must not have landed: MVOLL still holds $7F.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "a write through $F3 with $F2 bit 7 set took effect — that bit disables writing",
    );
    a.c("And an ordinary write must still work, or the check above proves only that nothing");
    a.c("reaches the DSP at all.");
    a.l("lda f:$7E0102");
    a.assert_a8(0x33, "an ordinary DSP write did not take effect");
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.11",
        'E',
        "$F2 bit 7 blocks writes",
        Provenance::Documented("SNESdev Wiki, S-DSP; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// DSP registers are independently addressable through `$F2`/`$F3`.
///
/// Foundational rather than exotic: every other DSP assertion is reached through this latch, so a
/// core that mis-decodes the address — masking too few bits, aliasing voice registers onto each
/// other, or latching the address at the wrong moment — makes every DSP test downstream
/// meaningless rather than failing.
///
/// Three registers in different parts of the file are written with distinct values and then read
/// back in a different order. The reordering is the point: reading them back in write order would
/// pass on a core that simply returns the last value written, which is the most likely way to get
/// the latch wrong.
fn dsp_addressing() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF2, 0x00) // voice 0 VOLL
        .mov_dp_imm(0xF3, 0x11)
        .mov_dp_imm(0xF2, 0x10) // voice 1 VOLL
        .mov_dp_imm(0xF3, 0x22)
        .mov_dp_imm(0xF2, 0x0C) // MVOLL
        .mov_dp_imm(0xF3, 0x33)
        // Read back in a different order than they were written.
        .mov_dp_imm(0xF2, 0x10)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT1)
        .mov_dp_imm(0xF2, 0x00)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT2)
        .mov_dp_imm(0xF2, 0x0C)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Read back out of order: voice 1, then voice 0, then the master volume.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x22,
        "voice 1's VOLL did not read back — the DSP address latch is mis-decoded",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x11,
        "voice 0's VOLL did not read back; if it holds voice 1's value the voices are aliased",
    );
    a.l("lda f:$7E0102");
    a.assert_a8(0x33, "MVOLL did not read back");
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.11b",
        'E',
        "DSP register addressing",
        Provenance::Documented("SNESdev Wiki, S-DSP registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Writing `ENDX` clears it; it is not a register you can set.
///
/// `$7C` reports which voices have reached the end of their sample, and **any** write clears all
/// eight bits regardless of the value written. A core that models it as ordinary storage returns
/// whatever was written — so writing `$FF` and reading `$FF` back is the exact signature of
/// getting this wrong, and it is what a driver polling for sample-end would see as "every voice
/// finished" forever.
///
/// The assertion is deliberately "not `$FF`" rather than "exactly `$00`": with no sample playing
/// there is nothing to set the bits in the first place, so requiring zero would pass on a core
/// that had simply never implemented the register at all. What this test can prove is the narrower
/// and still useful thing — that the write did not stick.
///
/// **The read waits before looking.** `ENDX`, `OUTX` and `ENVX` are written back from an internal
/// buffer once per sample, and a CPU write landing one or two clocks before that writeback is lost
/// (`E7.17`) — so an immediate read-back is racing a hazard the hardware documentation warns about,
/// and its answer depends on which DSP clock the write happened to land on. That is not a detail
/// this test is about, and it is not hypothetical: with no delay here, one added byte elsewhere in
/// the battery was enough to move the write into the window and flip the result on snes9x at PAL
/// timing while leaving NTSC alone. Waiting a few samples asserts the same thing about the same
/// write, minus the coin flip.
fn e9_19() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x7C)
        .mov_dp_a(0xF2) // address latch: select ENDX ($7C)
        .mov_a_imm(0xFF)
        .mov_dp_a(0xF3) // data port: any write clears ENDX, so this must not store $FF
        .delay(0x40) // let the writeback window pass -- see above
        .mov_a_imm(0x7C)
        .mov_dp_a(0xF2) // select it again to read back
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("A core storing the write returns $FF. Anything else means the write was treated as a");
    a.c("clear, which is the documented behaviour.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x00,
        0xFE,
        "ENDX read back as $FF, so the write was stored rather than treated as a clear",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.19",
        'E',
        "ENDX write clears it",
        Provenance::Documented("SNESdev Wiki, S-DSP registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The DSP's global registers are individually addressable and hold what is written.
///
/// The companion to the voice-register test: `$x C`/`$x D` are the global block — master and echo
/// volumes, echo feedback — and they are decoded from the same latch by a different part of the
/// address. A core that gets the voice registers right and aliases the globals (or vice versa)
/// passes one test and fails the other, which is why both exist.
///
/// Written low-to-high and read back high-to-low, so a core that simply returns the last value
/// written cannot pass.
fn dsp_global_regs() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    for (reg, val) in [(0x0Cu8, 0x11u8), (0x1C, 0x22), (0x2C, 0x33), (0x3C, 0x44)] {
        prog.mov_a_imm(reg).mov_dp_a(0xF2);
        prog.mov_a_imm(val).mov_dp_a(0xF3);
    }
    for (reg, port) in [(0x3Cu8, PORT1), (0x2C, PORT2), (0x1C, PORT3)] {
        prog.mov_a_imm(reg).mov_dp_a(0xF2);
        prog.mov_a_dp(0xF3).mov_dp_a(port);
    }
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Read back in the reverse of the write order: EVOLR, EVOLL, MVOLR.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(0x44, "EVOLR ($3C) did not read back");
    a.l("lda f:$7E0101");
    a.assert_a8(0x33, "EVOLL ($2C) did not read back");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x22,
        "MVOLR ($1C) did not read back; if it holds another register's value the globals are \
         aliased",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.11c",
        'E',
        "DSP global registers",
        Provenance::Documented("SNESdev Wiki, S-DSP registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// Voice playback: the part of the S-DSP that only moves when a sample is actually running.
// ---------------------------------------------------------------------------------------------
//
// Everything above pokes DSP registers and reads them back, which proves the address latch works
// and nothing else. The assertions below need a voice to be *playing*: `ENDX` only sets when a
// block with the end flag is decoded, and an envelope only reaches a value by being stepped. So
// each of these uploads a program that plants a BRR sample and a sample directory in APU RAM,
// points a voice at it, keys it on, waits, and reports what the DSP says afterwards.
//
// Three details make the difference between a test and a coin flip:
//
// * **A sample that does not end must be surrounded.** A block whose end flag is clear does not
//   stop; the DSP walks forward into whatever bytes follow, and some byte of the program's own
//   code eventually decodes as a header with the end flag set -- so `ENDX` sets for a reason that
//   has nothing to do with the test. The one test in that position pads its sample with silence
//   AND plays at a sixteenth of the sample rate, so the padding lasts far longer than the settle.
//   The other samples all carry an end flag, and need no padding at all: a looping sample repeats
//   forever and an end-without-loop sample stops the voice.
// * **The directory entry the test does not want used must be defined**, not merely absent.
//   `E5.11` distinguishes a correct entry address from a wrong one, and "wrong" has to point at
//   something known — here at address `$0000`, whose zero header decodes as silence forever.
// * **`KON` is cleared after keying on.** A core that re-keys a voice for as long as the bit is
//   set would hold the envelope at its attack value, and `E5.07` — which asserts the envelope
//   collapses — would fail against the core rather than against the behaviour.

/// Where `upload_and_run` places a program image in APU RAM.
const IMAGE_BASE: u16 = 0x0200;

/// The page the sample directory lives on, as the DSP's `DIR` register names it.
///
/// Page 1 is the stack page, and the entries sit at its very bottom while the stack is at its top
/// (`SP = $EF`): far enough apart that no program here comes close. A page of its own would cost
/// several hundred bytes of upload padding to reach, since a directory must be page-aligned.
const DIR_PAGE: u8 = 0x01;

/// Select a DSP register and write it.
fn dsp_write(p: &mut Spc, reg: u8, val: u8) {
    p.mov_a_imm(reg).mov_dp_a(0xF2);
    p.mov_a_imm(val).mov_dp_a(0xF3);
}

/// Select a DSP register and park its value in one of the four ports for the cart to read.
fn dsp_read_to(p: &mut Spc, reg: u8, port: u8) {
    p.mov_a_imm(reg).mov_dp_a(0xF2);
    p.mov_a_dp(0xF3).mov_dp_a(port);
}

/// One nine-byte BRR block: a header plus eight bytes of two four-bit samples each.
///
/// `flags` is the header's low two bits — bit 1 loop, bit 0 end — spelled that way round because
/// the header is `ssssffle` and the pair is routinely quoted as a "code": 0 normal, 1 end+mute,
/// 2 loop without end (which behaves as 0), 3 end+loop.
fn brr_block(shift: u8, filter: u8, flags: u8, hi: u8, lo: u8) -> Vec<u8> {
    let mut v = vec![(shift << 4) | (filter << 2) | flags];
    v.extend(core::iter::repeat_n((hi << 4) | lo, 8));
    v
}

/// `blocks`, followed by `run_out` blocks of silence for a non-looping voice to run out into.
///
/// `run_out` is zero for every sample that carries an end flag somewhere, which is most of them —
/// the padding is not free, and five copies of a generous run-out overflowed the ROM bank the
/// tests are linked into.
fn brr_sample(blocks: &[Vec<u8>], run_out: usize) -> Vec<u8> {
    let mut v: Vec<u8> = blocks.concat();
    for _ in 0..run_out {
        v.extend(brr_block(0, 0, 0, 0, 0));
    }
    v
}

/// Build a program that plays `sample` on voice 0 through directory entry `srcn` and reports.
///
/// The reports are always the same three registers, in the same three ports: `ENDX` (`$7C`),
/// voice 0's `ENVX` (`$08`), and voice 0's `OUTX` (`$09`). Each test asserts on the one it is
/// about; a shared shape is worth more here than a minimal one, because the setup is long and a
/// difference between two of these programs should be a difference the test is *about*.
///
/// `pitch_hi` is the high byte of the voice's pitch: `$10` is one sample per output sample, `$01`
/// is a sixteenth of that. `settle` is a count of delay loops after key-on, each roughly a thousand
/// SPC700 cycles — a few dozen output samples. Both are deliberately coarse: these assertions are
/// about what the DSP eventually reports, not about when.
/// How one voice test differs from the next.
///
/// A struct rather than a widening argument list, because most of these fields are the same in
/// most tests and the interesting thing about any one program is the one or two that are not.
#[derive(Clone, Copy)]
struct Voice {
    /// Which directory entry the voice plays from.
    srcn: u8,
    /// High byte of the pitch. `$10` is one sample per output sample.
    pitch_hi: u8,
    /// `VxADSR1`: bit 7 enables the ADSR generator, bits 6-4 decay, bits 3-0 attack.
    adsr1: u8,
    /// `VxADSR2`: bits 7-5 sustain level, bits 4-0 sustain rate.
    adsr2: u8,
    /// `VxGAIN`, consulted only while `adsr1` bit 7 is clear.
    gain: u8,
    /// `NON`: one bit per voice, replacing its sample with the noise generator.
    non: u8,
    /// Delay loops between key-on and the read, each roughly a thousand SPC700 cycles.
    settle: u8,
    /// A `(register, value, extra delay loops)` write made *after* settling — key-off, a `FLG`
    /// reset, anything whose effect is the thing being measured. The delay is how long the effect
    /// is given before the read.
    late: Option<(u8, u8, u8)>,
}

impl Voice {
    /// A looping voice held at full direct gain: the shape most of these tests vary from.
    const fn direct_gain() -> Self {
        Self {
            srcn: 0,
            pitch_hi: 0x10,
            adsr1: 0x00,
            adsr2: 0x00,
            gain: 0x7F,
            non: 0x00,
            settle: 4,
            late: None,
        }
    }
}

fn voice_program(sample: &[u8], v: Voice) -> Spc {
    let mut p = Spc::new();
    let addr = p.data_first(IMAGE_BASE, sample);
    p.mov_x_imm(0xEF).mov_sp_x();

    // The directory: four bytes per entry, start address then loop address, both little-endian.
    // Entry `srcn` gets the sample; the other of the first two entries is pointed at $0000, whose
    // zero header decodes as silence and never sets ENDX. An entry that is merely never written
    // would leave "wrong entry" meaning "whatever APU RAM happened to hold".
    let dir = u16::from(DIR_PAGE) << 8;
    for entry in 0u16..2 {
        let src = if u8::try_from(entry).expect("two entries") == v.srcn {
            addr
        } else {
            0x0000
        };
        let [lo, hi] = src.to_le_bytes();
        let base = dir + entry * 4;
        p.mov_a_imm(lo).mov_abs_a(base);
        p.mov_a_imm(hi).mov_abs_a(base + 1);
        p.mov_a_imm(lo).mov_abs_a(base + 2); // loop address: the same block, so code 3 repeats it
        p.mov_a_imm(hi).mov_abs_a(base + 3);
    }

    // Global state. FLG $20 leaves the DSP running and unmuted with echo *writes* disabled, which
    // is what a driver does before it has an echo buffer; the reset and mute bits are what the
    // power-on value has set. Noise, echo and pitch modulation are cleared explicitly rather than
    // assumed, since a previous test's program shares the same DSP.
    dsp_write(&mut p, 0x6C, 0x20); // FLG
    dsp_write(&mut p, 0x5C, 0x00); // KOF
    dsp_write(&mut p, 0x3D, v.non); // NON
    dsp_write(&mut p, 0x4D, 0x00); // EON
    dsp_write(&mut p, 0x2D, 0x00); // PMON
    dsp_write(&mut p, 0x5D, DIR_PAGE); // DIR
    dsp_write(&mut p, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut p, 0x1C, 0x7F); // MVOLR

    dsp_write(&mut p, 0x00, 0x7F); // VOL L
    dsp_write(&mut p, 0x01, 0x7F); // VOL R
    dsp_write(&mut p, 0x02, 0x00); // PITCH low
    dsp_write(&mut p, 0x03, v.pitch_hi); // PITCH high: $10 is one sample per output sample
    dsp_write(&mut p, 0x04, v.srcn); // SRCN
    // ADSR2 and GAIN are written BEFORE ADSR1, which is the order the errata asks for (`E7.18`):
    // the mode is decided by ADSR1 bit 7, so writing it last means the generator is never briefly
    // running against parameters meant for the other mode.
    dsp_write(&mut p, 0x06, v.adsr2); // ADSR2
    dsp_write(&mut p, 0x07, v.gain); // GAIN, consulted only while ADSR1 bit 7 is clear
    dsp_write(&mut p, 0x05, v.adsr1); // ADSR1

    dsp_write(&mut p, 0x7C, 0x00); // ENDX: any write clears it, so start from a known state
    dsp_write(&mut p, 0x4C, 0x01); // KON voice 0
    p.delay(0x00);
    dsp_write(&mut p, 0x4C, 0x00); // and clear it — see the module comment

    for _ in 0..v.settle {
        p.delay(0x00);
    }

    if let Some((reg, val, after)) = v.late {
        dsp_write(&mut p, reg, val);
        for _ in 0..after {
            p.delay(0x00);
        }
    }

    dsp_read_to(&mut p, 0x7C, PORT1); // ENDX
    dsp_read_to(&mut p, 0x08, PORT2); // voice 0 ENVX
    dsp_read_to(&mut p, 0x09, PORT3); // voice 0 OUTX
    p.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();
    p
}

/// A pair of handlers for the vector tests: one that means "arrived here", one that means "arrived
/// somewhere else".
///
/// Both end the program, so whichever runs is the one the cart hears from. That is what turns a
/// mis-computed vector from a hang into a *wrong answer* — a test whose only failure mode is the
/// timeout reports SKIP, which says the APU did not answer rather than that it answered wrongly.
///
/// **They restore `PSW` before handing back**, which is not tidiness. `BRK` sets the `B` flag and
/// nothing clears it afterwards, so a handler that simply finishes leaves `B` set in the SPC700 for
/// the whole rest of the battery — and `E4.02`, which reads the register state the IPL hands over,
/// then sees `$1A` where it expects `$0A`. It did, on the first run of these two tests. A test that
/// changes processor state every later test can see has to put it back.
fn vector_handlers(ok: u8, bad: u8) -> (Spc, Spc) {
    let mk = |mark: u8| {
        let mut p = Spc::new();
        p.mov_x_imm(0xEF)
            .mov_sp_x()
            .mov_a_imm(0x02)
            .push_a()
            .pop_psw() // clear B, which BRK set and nothing else clears
            .mov_a_imm(mark)
            .mov_dp_a(PORT1)
            .mov_a_imm(DONE)
            .mov_dp_a(PORT0)
            .release_to_ipl();
        p
    };
    (mk(ok), mk(bad))
}

/// `TCALL n` vectors through `[$FFDE - n*2]`, counting *down* from the top of the table.
///
/// Sixteen one-byte call instructions sharing a vector table at the very top of the address space —
/// which is inside the boot ROM while it is mapped, and ordinary RAM once it is not. The stride and
/// the direction are both easy to get backwards, and a driver using `TCALL` for its dispatch table
/// (they are one byte, which is the whole point) lands somewhere arbitrary if either is wrong.
///
/// The program unmaps the boot ROM so the table is writable, then plants the *right* handler at
/// `TCALL 1`'s slot and a different one either side of it. So a core that miscounts does not hang —
/// it runs the other handler and reports the wrong mark, which is a failure the cart can describe.
fn e2_08() -> Test {
    let (ok, bad) = vector_handlers(0xA1, 0xB2);

    let mut prog = Spc::new();
    let mut blob = ok.bytes().to_vec();
    let bad_at = u16::try_from(blob.len()).expect("handlers are small");
    blob.extend_from_slice(bad.bytes());
    let ok_addr = prog.data_first(IMAGE_BASE, &blob);
    let bad_addr = ok_addr + bad_at;

    prog.mov_x_imm(0xEF).mov_sp_x().mov_dp_imm(0xF1, 0x00); // unmap the boot ROM: the vector table is RAM again
    for (slot, addr) in [
        (0xFFDCu16, ok_addr), // TCALL 1
        (0xFFDE, bad_addr),   // TCALL 0 — one slot the wrong way
        (0xFFDA, bad_addr),   // TCALL 2 — the other way
    ] {
        let [lo, hi] = addr.to_le_bytes();
        prog.mov_a_imm(lo).mov_abs_a(slot);
        prog.mov_a_imm(hi).mov_abs_a(slot + 1);
    }
    prog.tcall(1);

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xA1,
        "TCALL 1 did not vector through $FFDC — $B2 means it read a neighbouring slot, so the \
         table is indexed with the wrong stride or the wrong direction",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.08",
        'E',
        "TCALL vector table",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `BRK` vectors through `$FFDE` — the same slot as `TCALL 0`, not one of its own.
///
/// The SPC700 has no separate break vector. `BRK` pushes `PC` and `PSW`, sets the `B` flag, and
/// jumps through the table entry `TCALL 0` already uses, so a program that installs a `TCALL 0`
/// handler has installed a `BRK` handler whether it meant to or not. A core that gives `BRK` its own
/// vector — the 65816 has one at `$FFE6`, which is where the instinct comes from — sends a stray
/// `BRK` somewhere the program never planned for.
///
/// Same shape as `E2.08`: the right handler at `$FFDE`, a different one next door, so a wrong vector
/// is a wrong answer rather than a hang.
fn e2_09() -> Test {
    let (ok, bad) = vector_handlers(0xC3, 0xD4);

    let mut prog = Spc::new();
    let mut blob = ok.bytes().to_vec();
    let bad_at = u16::try_from(blob.len()).expect("handlers are small");
    blob.extend_from_slice(bad.bytes());
    let ok_addr = prog.data_first(IMAGE_BASE, &blob);
    let bad_addr = ok_addr + bad_at;

    prog.mov_x_imm(0xEF).mov_sp_x().mov_dp_imm(0xF1, 0x00); // unmap the boot ROM
    for (slot, addr) in [(0xFFDEu16, ok_addr), (0xFFDC, bad_addr)] {
        let [lo, hi] = addr.to_le_bytes();
        prog.mov_a_imm(lo).mov_abs_a(slot);
        prog.mov_a_imm(hi).mov_abs_a(slot + 1);
    }
    prog.brk();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xC3,
        "BRK did not vector through $FFDE, the TCALL 0 slot — the SPC700 has no break vector of \
         its own",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.09",
        'E',
        "BRK shares TCALL 0",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F1` bit 5 clears the port 3 input latch.
///
/// The bits are strobes rather than switches: writing a 1 clears the corresponding pair of
/// CPU-to-APU input latches immediately, so a driver can drop stale commands without a second
/// write. A core that ignores them leaves a command the driver believed it had discarded sitting in
/// the port.
///
/// The value it clears is one the upload itself left there: `apu_upload` puts the entry address in
/// ports 2 and 3, so port 3 holds `$02`, the high byte of `$0200`. Using what the handshake already
/// wrote means the test needs nothing the mechanism does not already do.
///
/// **Two thirds of the dossier row are deliberately not covered here, and both need something this
/// test cannot reach.** Port 2's latch holds `$00` — the low byte of the same entry address — which
/// is indistinguishable from cleared, so only port 3 is checked. And the *non-persistence* half
/// ("the bit does not stay set") needs a second value to appear in a latch after the strobe, which
/// only the cart can put there; that needs a mid-program cart-to-APU handshake the upload mechanism
/// does not have. What is asserted is the immediate clear, and the failure text says so.
fn e3_03() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_dp(0xF7)
        .mov_dp_a(PORT1) // the IPL left the entry address's high byte here
        .mov_dp_imm(0xF1, 0xA0) // bit 5 clears ports 2 and 3; bit 7 keeps the boot ROM mapped
        .mov_a_dp(0xF7)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Port 3 holds $02, the high byte of the $0200 entry address the upload wrote there. If it");
    a.c("does not, the clear below would be measuring nothing.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x02,
        "port 3 did not hold the entry address's high byte, so the latch-clear check below would \
         be vacuous",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(0x00, "$F1 bit 5 did not clear the port 2/3 input latches");
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.03",
        'E',
        "$F1 bit 5 clears port 3",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F1` bit 7 controls what `$FFC0`-`$FFFF` *reads* as; writes always reach the RAM underneath.
///
/// The boot ROM is an overlay, not a region. A store to `$FFC0` lands in APU RAM whether or not the
/// ROM is mapped over it — the write is simply invisible until the overlay is switched off. That is
/// what makes the boot ROM's own space usable as ordinary RAM by a driver that no longer needs it,
/// and it is why an emulator that treats `$FFC0`-`$FFFF` as read-only while mapped loses a driver's
/// data with no error anywhere.
///
/// The whole claim in one program: write a byte with the ROM mapped, read back the *ROM* byte,
/// unmap, read back the *written* byte. `$CD` is the first byte of the canonical listing, which
/// `E4.01` checks in full.
fn e3_04() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF1, 0x80) // boot ROM mapped
        .mov_a_imm(0x5A)
        .mov_abs_a(0xFFC0) // goes to the RAM underneath, invisibly
        .mov_a_abs(0xFFC0)
        .mov_dp_a(PORT1) // still reads the ROM
        .mov_dp_imm(0xF1, 0x00) // unmap it
        .mov_a_abs(0xFFC0)
        .mov_dp_a(PORT2) // now the write is visible
        .mov_dp_imm(0xF1, 0x80) // and put it back, or there is no IPL to hand control to
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xCD,
        "a read of $FFC0 with the boot ROM mapped did not return the ROM's first byte",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x5A,
        "the byte written to $FFC0 while the ROM was mapped did not reach the RAM underneath — a \
         read-only overlay loses a driver's data silently",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.04",
        'E',
        "Writes pass under IPL",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A timer divider of `$00` means 256, not zero and not one.
///
/// `TnDIV` is the reload value of an 8-bit pre-divider, so writing `$00` selects its full period —
/// the *slowest* setting available, 256 times slower than `$01`. Read as a literal zero it becomes
/// either a division by nothing (a timer running 256 times too fast) or a stopped timer, and a
/// sound driver's tempo is wrong by more than two orders of magnitude either way.
///
/// Both halves are measured over the same delay: at `$01` the counter must have advanced, at `$00`
/// it must not have. The timer is stopped before each read, because the counter is four bits and the
/// reads are a few cycles apart — the same race `E3.01` was rebuilt to avoid.
fn e3_05() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1, the fastest
        .mov_dp_imm(0xF1, 0x81) // enable timer 0, boot ROM stays mapped
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80) // stop before reading
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT1)
        .mov_dp_imm(0xFA, 0x00) // T0DIV = 0, which means 256
        .mov_dp_imm(0xF1, 0x81) // re-enable: a 0->1 on the enable bit restarts the divider
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("The control: at divider 1 the counter advanced over this delay. Without it, the check");
    a.c("below would pass on a timer that never ran at all.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "timer 0 did not advance at divider 1, so the divider-0 check below would be vacuous",
    );
    a.c("And at divider 0 — meaning 256 — the same delay is nowhere near one tick.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "timer 0 ticked at divider $00 over a delay that is 256 times too short, so $00 was read \
         as a small number rather than as 256",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.05",
        'E',
        "TnDIV $00 means 256",
        Provenance::Documented("SNESdev Wiki, SPC700 timers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `TEST` bit 1 is the RAM write enable, and clearing it makes stores do nothing.
///
/// `$F0` is a hardware test register no game should touch, which is exactly why an emulator is
/// likely to model it as ordinary storage — and then a ROM that *does* touch it behaves differently
/// for reasons nothing in the trace explains. Bit 1 gates every write into APU RAM; with it clear,
/// stores execute, take their cycles, and change nothing.
///
/// The program seeds a byte, disables writes, stores a different byte, restores the register, and
/// only then reads back — reading while writes are disabled would be measuring the read path
/// instead. The final store proves the gate reopened, without which "the value did not change"
/// would also be what a broken write path looks like.
fn e3_10() -> Test {
    const SCRATCH: u16 = 0x0510;

    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x11)
        .mov_abs_a(SCRATCH) // seeded with writes enabled
        .mov_dp_imm(0xF0, 0x08) // TEST: bit 1 clear, bit 3 as it powers up
        .mov_a_imm(0x22)
        .mov_abs_a(SCRATCH) // executes, changes nothing
        .mov_dp_imm(0xF0, 0x0A) // restore the power-on value
        .mov_a_abs(SCRATCH)
        .mov_dp_a(PORT1)
        .mov_a_imm(0x33)
        .mov_abs_a(SCRATCH) // and writes work again
        .mov_a_abs(SCRATCH)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x11,
        "a store landed in APU RAM with TEST bit 1 clear, so the RAM write enable is not modelled",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x33,
        "the store after restoring TEST did not land either, so the check above says nothing about \
         bit 1",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.10",
        'E',
        "TEST gates RAM writes",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `CLRV` clears the half-carry as well as the overflow flag.
///
/// The mnemonic names one flag and the instruction clears two. Nothing else on the SPC700 clears
/// `H` on its own, so a decimal-arithmetic routine that uses `CLRV` to prepare for `DAA` is relying
/// on the second effect — and on a core that clears only `V`, the stale `H` silently changes what
/// `DAA` does.
///
/// An `ADC` of `$7F + $01` sets both flags first: the signed result overflows and the low nibble
/// carries. The reading before `CLRV` is reported too, because "both flags are clear afterwards" is
/// vacuous unless they were set to begin with.
fn e1_12() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .clrc()
        .mov_a_imm(0x7F)
        .adc_a_imm(0x01) // -> $80: V set (signed overflow) and H set (nibble carry)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1) // PSW with both set
        .clrv()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2) // PSW after CLRV
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("V is bit 6, H is bit 3. Both must be set before CLRV or the check after it proves");
    a.c("nothing.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("and #$48");
    a.assert_a8(
        0x48,
        "ADC $7F + $01 did not set both V and H, so the CLRV check below would be vacuous",
    );
    a.l("lda f:$7E0101");
    a.l("and #$48");
    a.assert_a8(
        0x00,
        "CLRV left a flag set — it clears H as well as V, and nothing else on the SPC700 clears H",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.12",
        'E',
        "CLRV clears H too",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `DAA` applies two adjustments, and the second one can carry out of the byte.
///
/// `if (C || A > $99) { A += $60; C = 1; }` then `if (H || (A & 15) > 9) { A += 6; }`. Two cases
/// pin both halves: `$0A` trips only the low-nibble test and becomes `$10`; `$9A` trips both, and
/// the `+$60` followed by `+6` wraps it to `$00` with carry set. A core implementing `DAA` as a
/// single table lookup, or as the 65C816's decimal mode, gets the second case wrong.
///
/// `CLRC` and `CLRV` set up the entry flags — `CLRV` because it is the only way to clear `H`
/// (`E1.12`), and a stale `H` would trip the second adjustment for a reason the test is not about.
fn e1_08() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .clrc()
        .clrv() // clears H as well — see E1.12
        .mov_a_imm(0x0A)
        .daa()
        .mov_dp_a(PORT1) // only the low-nibble adjustment: $0A + 6 = $10
        .clrc()
        .clrv()
        .mov_a_imm(0x9A)
        .daa()
        .mov_dp_a(PORT2) // both: $9A + $60 = $FA, then + 6 = $00 with carry
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x10,
        "DAA on $0A did not apply the low-nibble adjustment, so $0A + 6 = $10 did not happen",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "DAA on $9A did not wrap to $00 — both adjustments apply, and $9A + $60 + 6 leaves the byte",
    );
    a.c("And the carry the first adjustment sets, which is what makes the wrap a decimal result");
    a.c("rather than a lost hundred.");
    a.l("lda f:$7E0102");
    a.l("and #$01");
    a.assert_a8(0x01, "DAA on $9A did not set the carry");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.08",
        'E',
        "DAA adjustments",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `TSET1` is an equality test, not a result test: `N`/`Z` come from `A - target` *before* the write.
///
/// The instruction ORs `A` into the target and reports flags — but the flags describe a comparison
/// of `A` against the target's **old** value, exactly as `CMP` would. That is the opposite of what
/// the mnemonic suggests, and the difference is visible whenever the result is non-zero but the
/// operands were equal: `$55` set into `$55` leaves `$55`, so a core reporting flags from the result
/// says "not zero" where the hardware says "equal".
///
/// Both cases are here because the second is the discriminator and the first is what proves the
/// instruction did its job at all — the target must come back with `A`'s bits set.
fn e1_10() -> Test {
    const SCRATCH: u16 = 0x0500;

    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x30)
        .mov_abs_a(SCRATCH)
        .mov_a_imm(0x0F)
        .tset1_abs(SCRATCH) // $0F vs $30: unequal, and the target becomes $3F
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1)
        .mov_a_abs(SCRATCH)
        .mov_dp_a(PORT2)
        .mov_a_imm(0x55)
        .mov_abs_a(SCRATCH + 1)
        .mov_a_imm(0x55)
        .tset1_abs(SCRATCH + 1) // equal, though the result $55 is not zero
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Unequal operands: Z (bit 1) clear, and the target came back with A's bits set.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("and #$02");
    a.assert_a8(0x00, "TSET1 set Z although A and the target differed");
    a.l("lda f:$7E0101");
    a.assert_a8(0x3F, "TSET1 did not OR A into its target");
    a.c("Equal operands, non-zero result. This is the case that separates a comparison from a");
    a.c("result: the hardware says equal, a core reading flags off the result says not-zero.");
    a.l("lda f:$7E0102");
    a.l("and #$02");
    a.assert_a8(
        0x02,
        "TSET1 did not set Z for equal operands, so its flags describe the result rather than a \
         comparison against the target's old value",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.10",
        'E',
        "TSET1 is a compare",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// A `CALL` pushes the address it will return to, not that address minus one.
///
/// The 65816 pushes `return - 1` and `RTS` compensates; the SPC700 does not, and a core that copies
/// the 65816's convention returns one byte early — into the middle of whatever instruction follows
/// the call. Nothing about that is subtle once it happens, and nothing about it is visible until it
/// does.
///
/// The subroutine never returns: it pops the two pushed bytes, reports them, and finishes the
/// program. Popping is the only way to *see* what was pushed, and having seen it there is nothing
/// left on the stack to return with. The expected value is computed from the program's own layout —
/// the offset immediately after the `CALL` — rather than written down, so it cannot drift out of
/// step with the code.
fn e2_07() -> Test {
    let mut sub = Spc::new();
    sub.pop_a()
        .mov_dp_a(PORT1) // first pop: the low byte, if the push order is high-then-low
        .pop_a()
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut prog = Spc::new();
    let routine = prog.data_first(IMAGE_BASE, sub.bytes());
    prog.mov_x_imm(0xEF).mov_sp_x().call_abs(routine);
    let expected = IMAGE_BASE + u16::try_from(prog.here()).expect("program fits APU RAM");

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        (expected & 0xFF) as u8,
        "the low byte of the pushed return address is wrong; one less than expected means the \
         65816's return-minus-one convention was applied",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        (expected >> 8) as u8,
        "the high byte of the pushed return address is wrong",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.07",
        'E',
        "CALL pushes exact addr",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The IPL boot ROM is the same 64 bytes on every SNES.
///
/// `$FFC0`-`$FFFF` is a mask ROM inside the SPC700 — not part of the cartridge, not part of APU
/// RAM, and byte-identical on every console ever made. Everything about the audio boot depends on
/// it: a game's driver reaches the APU only through the handshake this ROM implements, so a wrong
/// byte in it does not degrade audio, it prevents any audio at all.
///
/// The program walks all 64 bytes and reports two checks: their sum, and a position-weighted
/// rolling value (`r = r * 2 + b`). The sum alone would accept any permutation of the same bytes,
/// which is precisely the mistake an emulator hand-transcribing the listing would make — the rolling
/// value is order-sensitive and costs three instructions.
///
/// It maps `$F1` bit 7 first. Every other program in this group leaves that bit alone, but a test
/// that reads the boot ROM cannot assume it is the boot ROM that is mapped there.
fn e4_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF1, 0x80) // map the IPL ROM at $FFC0-$FFFF
        .mov_dp_imm(0x10, 0x00) // sum
        .mov_dp_imm(0x11, 0x00) // rolling value
        .mov_x_imm(0x00);
    let loop_top = prog.here();
    prog.mov_a_dp(0x11)
        .asl_a()
        .mov_dp_a(0x12) // rolling * 2
        .mov_a_abs_x(0xFFC0)
        .mov_dp_a(0x13) // this byte
        .clrc()
        .adc_a_dp(0x12)
        .mov_dp_a(0x11) // rolling = rolling * 2 + byte
        .mov_a_dp(0x10)
        .clrc()
        .adc_a_dp(0x13)
        .mov_dp_a(0x10) // sum += byte
        .inc_x()
        .cmp_x_imm(64);
    prog.bne_back(loop_top);
    prog.mov_a_dp(0x10)
        .mov_dp_a(PORT1)
        .mov_a_dp(0x11)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xB8,
        "the IPL ROM's bytes do not sum to $B8, so it is not the canonical boot ROM",
    );
    a.c("And the order, which a sum cannot see: r = r*2 + b over the same 64 bytes.");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x4F,
        "the IPL ROM summed correctly but its rolling checksum is wrong, so the bytes are right \
         and their order is not",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E4.01",
        'E',
        "IPL ROM contents",
        Provenance::Documented("the canonical 64-byte IPL listing; fullsnes, SNESdev Wiki"),
        Kind::Scored,
        None,
    )
}

/// The IPL hands a program a defined register state, not whatever it happened to leave.
///
/// `A = 0`, `X = 0`, `Y = 0`, `PSW = $02` — `Z` set, everything else clear. A driver that relies on
/// it (and they do: the entry state is why so many drivers open with a `MOV` rather than a load)
/// breaks on a core that jumps to the program with its own leftovers in the registers.
///
/// It depends on no earlier test having left a sticky flag set, which is a real coupling rather than
/// a theoretical one: `E2.09` executes a `BRK`, `BRK` sets `B`, and nothing on the SPC700 clears it
/// short of a `POP PSW`. That test's handler restores `PSW` before handing back for exactly this
/// reason — see [`vector_handlers`].
///
/// The program's first three instructions capture the state before anything can disturb it, using
/// only the flag-free moves. `Y` and `A` are reported bitwise-ORed together rather than separately: both
/// must be zero, so their OR being zero says both, and it buys a third register out of the three
/// mailbox bytes available. `SP` is the one part not checked — reading it needs a register this
/// test would then have to report somewhere.
///
/// **The `PSW` assertion masks the half-carry bit, and the reason is a finding.** RustySNES, snes9x
/// and Mesen2 all hand over `$0A`, not the documented `$02`: `Z` as described, plus `H` left set by
/// the boot ROM's own arithmetic. Three independent implementations agreeing that the listing is
/// incomplete is worth more than a fourth opinion, but it is not licence to assert `$0A` — that
/// would be scoring a measured value against a citation that says something else. So the test
/// asserts the documented bits (`Z` set, `N`/`V`/`I`/`C` clear) and publishes the full byte to the
/// measurement channel, where a number can be reported without being scored.
fn e4_02() -> Test {
    let mut prog = Spc::new();
    prog.mov_dp_a(0x10) // A at entry, stashed (flag-free)
        .mov_dp_x(PORT2) // X at entry (flag-free)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1) // PSW at entry
        .mov_a_y()
        .or_a_dp(0x10)
        .mov_dp_a(PORT3) // Y | A, which is zero only if both are
        .mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.c("Publish the whole byte first — see the note above about $0A against a documented $02.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(112, "IPL handoff PSW");
    a.c("Then assert the documented bits only: Z set, N/V/I/C clear. Bit 3 (H) is masked out.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("and #$F7");
    a.assert_a8(
        0x02,
        "the IPL handed over with PSW other than $02 once the half-carry bit is masked — Z must \
         be set and N, V, I and C clear",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(0x00, "the IPL handed over with X non-zero");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "the IPL handed over with A or Y non-zero (they are reported ORed together)",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E4.02",
        'E',
        "IPL handoff state",
        Provenance::Documented("fullsnes, SNESdev Wiki, APU boot handshake"),
        Kind::Scored,
        None,
    )
}

/// An idle IPL announces itself as `$BBAA`, and it is the only thing a driver may wait for.
///
/// Port 0 reads `$AA` and port 1 reads `$BB` whenever the boot ROM is sitting in its ready loop —
/// at power-on, and again every time a program hands control back. It is the one piece of APU state
/// a game can check *before* it has uploaded anything, so every sound driver in existence opens by
/// polling for it.
///
/// No upload: this reads the two ports directly, which is exactly what the driver does. What it
/// measures is that the previous test's release actually returned the APU to the boot ROM — a core
/// that never re-announces leaves every later upload waiting on a handshake that will not come.
///
/// It *polls* rather than reading once, and that is not a weakening. The previous test released the
/// APU a few 65816 instructions ago; the SPC700 has to notice the release byte, jump to `$FFC0`, and
/// run the announcement, which takes real time on a processor running at a twentieth of the CPU's
/// clock. Reading once asserts that the handoff is instantaneous, which is not the claim and is not
/// true. Polling with a bound is what a driver does, and a core that never announces still fails —
/// it runs out the bound and reports SKIP rather than a pass.
///
/// **It polls the second byte and asserts the first**, which makes the test an ordering claim as
/// well. The boot ROM stores `$AA` to port 0 and then `$BB` to port 1, two separate instructions, so
/// once `$BB` is visible `$AA` must already be. Doing it the other way round — poll for `$AA`, then
/// read port 1 — lands in the gap between the two stores, and snes9x failed exactly that on the
/// first version of this test. A driver polling for `$AA` alone and then trusting port 1 has the
/// same bug.
fn e4_04() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label("wait");
    a.l("sep #$20");
    a.l("lda APUIO1");
    a.l("cmp #$BB");
    a.l("beq @ready");
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$4000");
    a.l("bne @wait");
    a.c("Never announced. SKIP rather than FAIL: an APU that is not in its boot ROM has told us");
    a.c("nothing about what the boot ROM announces, which is the only thing being measured.");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("sta f:V_TEST_RESULT");
    a.l("jml test_restore");
    a.label("ready");
    a.c("Port 1 is $BB, which the boot ROM writes second — so port 0 must already hold the $AA it");
    a.c("writes first. The pair is the announcement; $BB alone is a byte a core could leave");
    a.c("anywhere.");
    a.l("sep #$20");
    a.l("lda APUIO0");
    a.assert_a8(
        0xAA,
        "port 1 announced $BB but port 0 does not read $AA, so the ready word is not $BBAA — or \
         the two bytes are written in the wrong order",
    );
    a.finish(
        "E4.04",
        'E',
        "IPL ready announcement",
        Provenance::Documented("fullsnes, SNESdev Wiki, APU boot handshake"),
        Kind::Scored,
        None,
    )
}

/// A looping block, for tests whose voice must simply keep playing.
///
/// Code 3 — end *and* loop — so the block repeats forever: the envelope is then the only thing in
/// the program that can move, which is what every envelope test below needs.
fn looping_sample() -> Vec<u8> {
    brr_sample(&[brr_block(0x8, 0, 0b11, 0x7, 0x9)], 0)
}

/// A looping block whose every nibble is the same, so the voice's output is a constant.
///
/// The three BRR-arithmetic tests below all read `VxOUTX`, and a sample whose nibbles alternate
/// gives an output that alternates with it — which of the two the read catches then depends on the
/// exact sample the DSP is on. With every nibble identical, filter 0 decodes the same value every
/// time and gaussian interpolation of a constant is that constant, so the reading is stable and the
/// assertion is about the arithmetic rather than about when the cart looked.
fn constant_sample(shift: u8, nibble: u8) -> Vec<u8> {
    filtered_sample(shift, 0, nibble)
}

/// [`constant_sample`] with a filter of the caller's choosing.
///
/// A constant input is the clearest way to see what a filter does: filter 0 reproduces it, and
/// every other filter is a recurrence over the samples before it, so it settles at a value the
/// filter's own formula decides rather than at the input.
fn filtered_sample(shift: u8, filter: u8, nibble: u8) -> Vec<u8> {
    brr_sample(&[brr_block(shift, filter, 0b11, nibble, nibble)], 0)
}

/// BRR nibbles are signed: `$8` is `-8`, not `+8`.
///
/// Each nibble is a two's-complement four-bit value in `-8..+7`, so the top bit is a sign and a
/// core reading them as unsigned produces a waveform that is entirely positive — audible as a DC
/// offset and a wrong shape rather than as silence. With every nibble `$8` and the envelope at full
/// direct gain, `VxOUTX` — the post-envelope sample's high byte — must have its sign bit set.
///
/// Its control is `E5.03`, which asserts the *positive* half from the same shift and the same
/// envelope. Either test alone is satisfied by a core that always reports one sign.
fn e5_02() -> Test {
    let prog = voice_program(&constant_sample(0x8, 0x8), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x80,
        0xFF,
        "a sample of $8 nibbles produced a positive output, so the nibbles were read as unsigned",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.02",
        'E',
        "BRR nibbles are signed",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// A decoded BRR sample is `(nibble << shift) >> 1`, and it reaches the output.
///
/// The positive control for `E5.02` and the "it plays at all" control for `E5.04`: the same shift,
/// the same envelope, nibbles of `+7`, and `VxOUTX` must be positive and non-zero. What it pins is
/// narrow but load-bearing — that a decoded sample of this magnitude survives the envelope and the
/// interpolator to a reading the cart can see.
///
/// It asserts a range rather than the exact byte on purpose. The exact value is
/// `((nibble << shift) >> 1) * E >> 11`, high byte, and `E` here is a direct gain of `$7F0` rather
/// than full scale — so pinning the byte would be asserting the envelope's exact value through a
/// test about BRR decoding, and it would move if the gain in the shared setup ever changed.
fn e5_03() -> Test {
    let prog = voice_program(&constant_sample(0x8, 0x7), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x01,
        0x7F,
        "a sample of $7 nibbles did not produce a positive non-zero output; zero means nothing \
         reached the output at all, and a negative value means the nibbles were sign-confused",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.03",
        'E',
        "BRR sample arithmetic",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Shifts 13, 14 and 15 do not shift: they collapse the sample to `$0000` or `$F800`.
///
/// The header's shift field goes to 15 but the decoder only implements 0-12; the top three are a
/// documented special case that discards the nibble's magnitude entirely and keeps only its sign —
/// `$0000` for a positive nibble, `$F800` for a negative one. A core that takes the shift at face
/// value produces an enormous sample instead of a silent one, which is the difference between a
/// quiet passage and a burst of noise.
///
/// The nibbles here are `+7`, so the documented output is zero. Zero is also what silence looks
/// like, which is exactly why `E5.03` exists: it is the same sample at a legal shift, and it must
/// read non-zero.
fn e5_04() -> Test {
    let prog = voice_program(&constant_sample(0xD, 0x7), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "shift 13 did not collapse a positive sample to zero, so the invalid shifts are being \
         applied as ordinary ones",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.04",
        'E',
        "Invalid shift collapses",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Filter 1 is a recurrence, not a scale factor: a constant input settles far above itself.
///
/// The filter keeps most of the previous output and adds the new sample, so a constant input does
/// not stay at its own value — it converges on a fixed point an order of magnitude higher. With the
/// same shift and nibble as `E5.03`, whose filter-0 reading is single-digit, `VxOUTX` here settles
/// in the `$40`-`$7F` band and stays there: a genuine fixed point, not a moment in a waveform.
///
/// The bounds are deliberately loose, and it is worth saying why rather than quietly picking a
/// number. The exact fixed point is the documented recurrence's, scaled by an envelope that is a
/// direct gain of `$7F0` rather than full scale, and divided down by the decoder's internal
/// representation — a chain in which every link belongs to a different assertion. What this test
/// claims is the part that is filter 1's alone: that it accumulates. A core that ignores the filter
/// field reports `E5.03`'s single-digit answer and fails by an enormous margin, which is the
/// failure worth catching — the documentation is emphatic that these formulas are exact and that
/// simplifying them breaks game audio.
fn e5_05() -> Test {
    let prog = voice_program(&filtered_sample(0x8, 1, 0x7), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x40,
        0x7F,
        "filter 1 did not settle well above its constant input — a single-digit reading is \
         filter 0's answer, so the filter was not applied",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.05",
        'E',
        "BRR filter 1",
        Provenance::Documented("fullsnes, S-DSP BRR filters; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// A linear-decrease envelope that runs out of room clamps at zero; it does not wrap.
///
/// Key-on puts the envelope at zero, and a linear-decrease GAIN steps it down by 32 every sample,
/// so the very first step underflows. The hardware holds it at zero. A core that lets the eleven-bit
/// value wrap reports something near full scale instead — silence becoming maximum volume, which is
/// the loudest possible way to get an envelope wrong.
///
/// Its control is `E7.11`, the same custom-GAIN machinery driving the ramp the other way: without
/// it, "the envelope is zero" is also what a core with no GAIN ramps at all reports.
fn e7_14() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            gain: 0x9F, // custom, linear decrease, rate $1F (every sample)
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "a linear-decrease envelope did not clamp at zero; a large reading means it wrapped",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.14",
        'E',
        "GAIN decrease clamps",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// The envelope's full scale is `$7FF`, and `VxENVX` reports it shifted down four.
///
/// A voice attacked at rate `$F` reaches maximum essentially at once — the documented step for that
/// rate is `+1024` per sample, against `+32` for every other — and with the sustain level at `7`
/// the boundary is the top of the range, so it arrives and stays. `VxENVX` then reads exactly
/// `$7F`.
///
/// The exactness is the test. An eleven-bit envelope reported as `E >> 4` cannot produce a value
/// above `$7F`, so bit 7 is always clear; a core carrying a full byte of envelope, or shifting by
/// three, reports `$FF` or `$FE` here and is otherwise indistinguishable — every other envelope
/// test only ever checks a direction or a range.
fn e7_15() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            // ADSR on, attack $F. The decay rate does not matter here: sustain level 7 puts the
            // decay boundary at the top of the range, so the envelope is in sustain from the
            // moment it arrives, and sustain rate 0 never fires, so it stays.
            adsr1: 0x8F,
            adsr2: 0xE0,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "a fully attacked envelope did not read $7F; $FF or $FE means ENVX is not E >> 4 of an \
         eleven-bit envelope",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.15",
        'E',
        "ENVX is E >> 4",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Key-off releases at a fixed rate, every sample, all the way to zero.
///
/// Release is not a rate you can choose: it steps `-8` every sample regardless of anything in
/// `ADSR` or `GAIN`, which takes an envelope from full scale to silence in about eight
/// milliseconds. This voice is held at a direct gain of `$7F` that nothing else would ever move —
/// `E7.10` asserts exactly that — so a reading of zero after `KOF` can only be the release path.
///
/// The one thing it cannot distinguish is a core that stops the voice outright on key-off instead
/// of releasing it, since both end at zero. That distinction needs a reading *during* the ramp, and
/// the delay loop here is too coarse to place one.
fn e7_08() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            late: Some((0x5C, 0x01, 12)), // KOF voice 0
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the envelope was not zero well after key-off, so release did not run to silence",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.08",
        'E',
        "Key-off releases to zero",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Custom GAIN, linear increase: the envelope climbs from zero to full scale on its own.
///
/// With `ADSR1` bit 7 clear and `VxGAIN` bit 7 set, the low five bits are a rate and bits 6-5 pick
/// one of four ramps. Mode `10` is linear increase, `+32` per step, and rate `$1F` steps every
/// sample — so a voice keyed on at envelope zero reaches `$7FF` in sixty-four samples and holds
/// there.
///
/// Reaching full scale is the whole assertion, and it is worth stating what that separates: a core
/// treating a custom-GAIN byte as a *direct* value would set the envelope to `$1F << 4` and report
/// `$1F`, which is the mistake this shape of register invites.
fn e7_11() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            gain: 0xDF, // custom, linear increase, rate $1F (every sample)
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "a linear-increase GAIN did not reach full scale; $1F means the mode bits were ignored \
         and the byte was taken as a direct value",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.11",
        'E',
        "GAIN linear increase",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Rate 0 never fires, so a ramp configured with it does not move at all.
///
/// The rate table's first entry is not "as fast as possible" but "never": a rate of 0 disables the
/// step entirely. The same linear-increase GAIN as `E7.11` with rate 0 therefore leaves the
/// envelope where key-on put it, at zero.
///
/// It is the pair to `E7.11`, and needs to be: on its own, "the envelope did not move" is also what
/// a core with no GAIN ramps at all reports, and what a voice that never started reports. Only the
/// two together say that the ramp works *and* that rate 0 switches it off.
fn e7_01() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            gain: 0xC0, // custom, linear increase, rate 0 — which never fires
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the envelope moved although the GAIN rate was 0, which never fires",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.01",
        'E',
        "Rate 0 never fires",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// `FLG`'s reset bit keys every voice off and zeroes every envelope.
///
/// Setting bit 7 of `$6C` is what a driver does before it has configured anything, and it is not a
/// gentle stop: it behaves as `KOFF = $FF` with the envelopes forced to zero rather than released.
/// This voice is held at a direct gain of `$7F` that nothing else would move, so a reading of zero
/// afterwards is the reset and nothing else.
///
/// It is the same observation as `E7.08` reached by a different route, and the pair is worth having:
/// a core that implemented `FLG` reset as "stop the DSP" would leave the last envelope value
/// visible, which passes neither.
fn e9_18() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            // FLG: reset (bit 7) plus the echo-write disable the setup already uses.
            late: Some((0x6C, 0xA0, 4)),
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the envelope survived a FLG reset, so the reset bit did not force the voices off",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.18",
        'E',
        "FLG reset kills voices",
        Provenance::Documented("SNESdev Wiki, S-DSP; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A noise voice still decodes its BRR sample, so an end-without-loop block silences it.
///
/// `NON` replaces a voice's *sample* with the noise generator, and it is easy to assume that leaves
/// the sample pointer unused. It does not: the voice keeps decoding BRR underneath, which means the
/// end-and-mute flag still reaches it and still forces the envelope to zero. A driver that parks a
/// noise voice on whatever sample address happens to be there gets silence at an unpredictable
/// moment, and a core that skips decoding for noise voices never reproduces it.
///
/// Its control is `E5.07`: the identical sample and the identical read with `NON` clear. Without
/// that pair, "the envelope is zero" would also be what a core that simply cannot play a noise
/// voice reports.
fn e9_04() -> Test {
    let sample = brr_sample(&[brr_block(0x8, 0, 0b01, 0x7, 0x9)], 0);
    let prog = voice_program(
        &sample,
        Voice {
            non: 0x01,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "a noise voice's envelope survived an end-without-loop block, so noise voices are not \
         decoding BRR underneath",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.04",
        'E',
        "Noise voices decode BRR",
        Provenance::Documented("fullsnes, S-DSP noise; anomie's DSP doc — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// A direct GAIN value *is* the envelope: `ENVX` reads back the byte that was written.
///
/// With `ADSR1` bit 7 clear the ADSR generator is off and `VxGAIN` governs the envelope; with
/// `VxGAIN` bit 7 also clear the mode is direct, and the envelope is set to `G << 4` rather than
/// ramped toward it. `VxENVX` reports `E >> 4`, so the two shifts cancel and a direct gain of `$7F`
/// reads back as exactly `$7F` — an exact number, on a register that is otherwise only ever
/// checked for being "about right".
///
/// The voice is playing a looping sample throughout, so nothing else has cause to move the
/// envelope. That matters: the same read on a voice that had finished would report `$00` for a
/// reason this test is not about (see `E5.07`).
fn e7_10() -> Test {
    // Code 3 — end and loop — so the voice repeats this block forever and never runs out.
    let sample = brr_sample(&[brr_block(0x8, 0, 0b11, 0x7, 0x9)], 0);
    let prog = voice_program(&sample, Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "ENVX did not read back the direct GAIN value; a ramp toward it, or a missing >>4, both \
         land somewhere else",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.10",
        'E',
        "Direct GAIN is envelope",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `ENDX` sets when a block carrying the end flag is decoded.
///
/// The voice plays two blocks, the second of which is code 3 — end and loop — so the sample
/// repeats and the only thing that can have set `ENDX` is the end flag itself. A core that never
/// implemented the register, or that only sets it when a voice *stops*, reports nothing here; a
/// driver waiting on `ENDX` to swap a sample would wait forever.
fn e5_09() -> Test {
    let sample = brr_sample(
        &[
            brr_block(0x8, 0, 0b00, 0x7, 0x9),
            brr_block(0x8, 0, 0b11, 0x9, 0x7),
        ],
        0,
    );
    let prog = voice_program(&sample, Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Bit 0 is voice 0. Masked rather than compared whole: the other seven voices were never");
    a.c("keyed on, but nothing in this test says what a core leaves in their bits.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$0001");
    a.assert_a16_range(
        1,
        1,
        "ENDX bit 0 never set although the voice decoded a block with the end flag",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.09",
        'E',
        "ENDX sets on end block",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// The loop flag alone means nothing: code 2 behaves exactly as code 0.
///
/// Both header bits are read as a pair, and only the end bit stops anything. A block with the loop
/// bit set and the end bit clear is an ordinary block — the loop address is consulted when a block
/// *ends*, and this one does not. A core that treats the loop bit as "this is the last block"
/// sets `ENDX` here, and would then also jump back to the loop point in the middle of a sample.
///
/// Without an end flag the voice keeps decoding forward, so this is the one voice test that has to
/// bound where it gets to: it plays at a sixteenth of the sample rate and pads the sample with six
/// blocks of silence, which is minutes of settle time away rather than the two delay loops it
/// actually waits.
///
/// It is the pair to `E5.09`, which sets `ENDX` from an otherwise identical program with one header
/// bit different. Without that pairing this assertion would also pass on a voice that never
/// started, since "did not set a bit" is what silence looks like too.
fn e5_08() -> Test {
    let sample = brr_sample(
        &[
            brr_block(0x8, 0, 0b10, 0x7, 0x9),
            brr_block(0x8, 0, 0b10, 0x9, 0x7),
        ],
        6,
    );
    let prog = voice_program(
        &sample,
        Voice {
            pitch_hi: 0x01,
            settle: 2,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$0001");
    a.assert_a16_range(
        0,
        0,
        "ENDX bit 0 set although no block carried the end flag, so the loop bit was read as one",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.08",
        'E',
        "Loop flag without end",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// End without loop silences the voice immediately, whatever the envelope was doing.
///
/// Code 1 — end set, loop clear — puts the voice into release with an envelope of zero the moment
/// the block finishes, rather than releasing it at the configured rate. The envelope here is a
/// direct GAIN of `$7F`, which nothing about the envelope generator would ever move on its own, so
/// a reading of `$00` afterwards can only have come from the end-and-mute path.
///
/// This is the pair to `E7.10`: identical setup, identical read, one header bit different, and the
/// answers are opposite. Neither test alone separates "the envelope works" from "the envelope is
/// stuck at whatever was written".
fn e5_07() -> Test {
    let sample = brr_sample(&[brr_block(0x8, 0, 0b01, 0x7, 0x9)], 0);
    let prog = voice_program(&sample, Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "ENVX was not zero after an end-without-loop block, so end+mute did not force release",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.07",
        'E',
        "End+mute zeroes env",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// A sample directory entry is at `DIR * $100 + SRCN * 4`.
///
/// The same sample as `E5.09`, reached through entry **1** instead of entry 0 — and entry 0 is
/// pointed at address `$0000`, whose zero header decodes as silence that never ends. So a core
/// that folds `SRCN` in with the wrong stride, or ignores it, plays silence and reports nothing;
/// only the documented address arrives at a sample with an end flag in it.
///
/// The decoy matters more than it looks. With entry 0 simply left unwritten, "wrong entry" would
/// mean "whatever APU RAM happened to hold", which is neither silence nor a sample reliably.
fn e5_11() -> Test {
    let sample = brr_sample(
        &[
            brr_block(0x8, 0, 0b00, 0x7, 0x9),
            brr_block(0x8, 0, 0b11, 0x9, 0x7),
        ],
        0,
    );
    let prog = voice_program(
        &sample,
        Voice {
            srcn: 1,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$0001");
    a.assert_a16_range(
        1,
        1,
        "ENDX never set for SRCN 1, so the directory entry was not read from DIR*$100 + SRCN*4",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.11",
        'E',
        "Directory entry address",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Direct-page indexing wraps inside the page.
///
/// `MOV A,$FF+X` with `X = 2` reads direct-page `$01`, not `$0101`. The index is added to the
/// 8-bit offset and the result stays in the page the `P` flag selects — it does not carry into the
/// page above. A core that computes the address as a 16-bit sum reads a byte from the wrong page
/// entirely, which is silent until something lives there.
///
/// `$0101` — where a 16-bit sum *would* land — is poisoned with a third value rather than left to
/// whatever APU RAM holds. Otherwise the test asserts only that the wrong page did not happen to
/// contain the expected marker, which is a weaker claim that quietly depends on power-on state.
fn e2_05() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0x01, 0x5A) // where a wrapped index must land
        .mov_dp_imm(0xFF, 0x99) // and where the un-indexed offset points
        .mov_a_imm(0x33)
        .mov_abs_a(0x0101) // and where a 16-bit sum would land, poisoned so it cannot match
        .mov_x_imm(0x02)
        .mov_a_dp_x(0xFF) // $FF + 2 -> $01 if it wraps, $0101 if it does not
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x5A,
        "$FF + X did not wrap within the direct page; a 16-bit sum would read $0101 instead",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.05",
        'E',
        "DP index wraps in page",
        Provenance::Documented("SNESdev Wiki, SPC700 addressing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A store to a timer counter clears it, because stores dummy-read their destination.
///
/// `MOV $FD,A` writes nothing useful — the counter is read-only — but the instruction reads its
/// destination first, and reading a timer counter *consumes* it. So a store to `$FD` clears
/// Timer 0 as surely as a load does, which is a trap for any driver that "initialises" the
/// counters by writing them.
///
/// Both readings are asserted directly rather than against each other. The first version of this
/// test asked only that the post-store reading be *smaller* than a control reading taken over the
/// same delay, and that version failed on one reference emulator while passing here — not because
/// either core was wrong, but because a core that does not clear leaves an arbitrary value in the
/// counter, and an arbitrary value lands inside a difference range often enough to decide the
/// test by luck. Requiring the control to have advanced and the post-store reading to be empty is
/// the stronger claim and the stable one.
fn e2_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1
        .mov_dp_imm(0xF1, 0x81) // enable timer 0, and KEEP the IPL ROM mapped (bit 7)
        // --- control: delay, then read. The counter must have advanced. ---
        .delay(0x00)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2)
        // --- the store: delay again, store to $FD, then read. The store's dummy read cleared it. ---
        .delay(0x00)
        .mov_a_imm(0x00)
        .mov_dp_a(0xFD) // MOV $FD,A — a store, whose dummy read consumes the counter
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Control first: without a store in the way, the counter advanced.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        2,
        15,
        "timer 0 did not advance over the control delay, so the check below is vacuous — it would \
         pass on a counter that was empty the whole time",
    );
    a.c("And immediately after the store the counter is essentially empty. Asserted directly");
    a.c("rather than as a difference: a core that does NOT clear leaves an arbitrary value there,");
    a.c("and an arbitrary value lands inside a difference range often enough to pass by luck.");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        1,
        "the counter was not empty immediately after a store to $FD, so the store's dummy read \
         did not consume it",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.01",
        'E',
        "Store dummy-reads target",
        Provenance::Documented("SNESdev Wiki, SPC700; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}
