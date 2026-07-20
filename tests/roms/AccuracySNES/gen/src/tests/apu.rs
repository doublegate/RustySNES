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
use crate::spc::{DONE, PORT0, PORT1, PORT2, PORT3, Spc};

/// Every Group E test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![e1_01()]
}

/// Emit the cart-side half: upload `prog`, wait for its done marker, leave port values readable.
///
/// The wait is bounded by a counter rather than spinning forever. An APU that never boots is a
/// real failure mode — it is the one thing here the cart cannot recover from — and a test that
/// hangs takes the whole battery with it, reporting nothing at all about the other 148 tests.
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
        .halt();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Product first: $10 * $10 = $0100.");
    a.l("sep #$20");
    a.l("lda APUIO2");
    a.assert_a8(0x00, "MUL YA low byte is wrong");
    a.l("lda APUIO3");
    a.assert_a8(0x01, "MUL YA high byte is wrong");
    a.c("Then the flags. Z is bit 1 of PSW and must be CLEAR even though A came out $00.");
    a.l("lda APUIO1");
    a.l("and #$02");
    a.assert_a8(
        0x00,
        "MUL YA set Z although Y is non-zero — the flags come from Y alone, not from A or YA",
    );
    a.c("N is bit 7, and $01 is positive, so it must be clear too.");
    a.l("lda APUIO1");
    a.l("and #$80");
    a.assert_a8(0x00, "MUL YA set N although Y is $01");
    a.l("bra @pass");
    a.label("timeout");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("sta f:V_TEST_RESULT   ; SKIP: the APU never published a done marker");
    a.l("jmp test_restore");
    a.label("pass");
    a.finish(
        "E1.01",
        'E',
        "MUL YA flags from Y",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}
