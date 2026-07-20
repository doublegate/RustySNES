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
    a.l("bra @body");
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
