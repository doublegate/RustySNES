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

use crate::dsl::{Asm, Kind, Provenance, Test};
use crate::spc::{DONE, PORT0, PORT1, PORT2, PORT3, RELEASE, Spc};

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
    // `jmp`, not `bra`: the image being jumped over is the SPC700 program itself, and a program
    // that carries BRR sample data is several hundred bytes -- far past a branch's reach.
    a.l("jmp @body");
    a.label("prog");
    for line in prog.as_ca65("    ").lines() {
        a.l(line.trim_start());
    }
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Point apu_upload at this test's own program image.");
    a.l("lda #@prog");
    a.l("sta f:V_APU_SRC");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta f:V_APU_BANK");
    a.l("rep #$30");
    a.l(&format!("lda #{}", prog.bytes().len()));
    a.l("sta f:V_APU_LEN");
    a.l("lda #$0200");
    a.l("sta f:V_APU_DEST     ; APU RAM $0200: clear of the zero page and the stack");
    a.l("lda #$0200");
    a.l("sta f:V_APU_ENTRY");
    a.l("jsr apu_upload");
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
    a.l("jmp test_restore");
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
fn voice_program(sample: &[u8], srcn: u8, pitch_hi: u8, settle: u8) -> Spc {
    let mut p = Spc::new();
    let addr = p.data_first(IMAGE_BASE, sample);
    p.mov_x_imm(0xEF).mov_sp_x();

    // The directory: four bytes per entry, start address then loop address, both little-endian.
    // Entry `srcn` gets the sample; the other of the first two entries is pointed at $0000, whose
    // zero header decodes as silence and never sets ENDX. An entry that is merely never written
    // would leave "wrong entry" meaning "whatever APU RAM happened to hold".
    let dir = u16::from(DIR_PAGE) << 8;
    for entry in 0u16..2 {
        let src = if u8::try_from(entry).expect("two entries") == srcn {
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
    dsp_write(&mut p, 0x3D, 0x00); // NON
    dsp_write(&mut p, 0x4D, 0x00); // EON
    dsp_write(&mut p, 0x2D, 0x00); // PMON
    dsp_write(&mut p, 0x5D, DIR_PAGE); // DIR
    dsp_write(&mut p, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut p, 0x1C, 0x7F); // MVOLR

    dsp_write(&mut p, 0x00, 0x7F); // VOL L
    dsp_write(&mut p, 0x01, 0x7F); // VOL R
    dsp_write(&mut p, 0x02, 0x00); // PITCH low
    dsp_write(&mut p, 0x03, pitch_hi); // PITCH high: $10 is one sample per output sample
    dsp_write(&mut p, 0x04, srcn); // SRCN
    dsp_write(&mut p, 0x05, 0x00); // ADSR1: ADSR disabled, so GAIN is in charge
    dsp_write(&mut p, 0x06, 0x00); // ADSR2
    dsp_write(&mut p, 0x07, 0x7F); // GAIN: bit 7 clear is direct gain, envelope = $7F << 4

    dsp_write(&mut p, 0x7C, 0x00); // ENDX: any write clears it, so start from a known state
    dsp_write(&mut p, 0x4C, 0x01); // KON voice 0
    p.delay(0x00);
    dsp_write(&mut p, 0x4C, 0x00); // and clear it — see the module comment

    for _ in 0..settle {
        p.delay(0x00);
    }

    dsp_read_to(&mut p, 0x7C, PORT1); // ENDX
    dsp_read_to(&mut p, 0x08, PORT2); // voice 0 ENVX
    dsp_read_to(&mut p, 0x09, PORT3); // voice 0 OUTX
    p.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();
    p
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
    let prog = voice_program(&sample, 0, 0x10, 4);

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
    let prog = voice_program(&sample, 0, 0x10, 4);

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
    let prog = voice_program(&sample, 0, 0x01, 2);

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
    let prog = voice_program(&sample, 0, 0x10, 4);

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
    let prog = voice_program(&sample, 1, 0x10, 4);

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
