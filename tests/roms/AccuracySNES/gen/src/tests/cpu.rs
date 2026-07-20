//! Group A — the WDC 65C816 CPU.
//!
//! Per `docs/accuracysnes-research-dossier.md` §5.A. Sub-groups A1-A9. Each function here is one
//! menu entry; a menu entry may carry several sub-assertions, each with its own failure code, so
//! a failing emulator learns *which* case broke rather than just "FAIL" (AccuracyCoin's model).
//!
//! Entry state for every test, established by the runtime: native mode, `A`/`X`/`Y` 16-bit,
//! `DP = $0000`, `DBR = $00`, stack in bank `$00`. Tests may corrupt any of that freely —
//! `test_restore` puts it all back.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// Every Group A test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        // --- A1: emulation vs native mode ---
        a1_01(),
        a1_02(),
        a1_03(),
        a1_04(),
        a1_05(),
        // --- A2: direct-page wrapping ---
        a2_01(),
        a2_02(),
        a2_03(),
        a2_04(),
        a2_05(),
        // --- A3: stack wrapping ---
        a3_01(),
        a3_02(),
        a3_03(),
        a3_04(),
        a3_05(),
        // --- A4: absolute / jump wrapping ---
        a4_01(),
        a4_02(),
        a4_03(),
        a4_04(),
        a4_05(),
        // --- A5: cycle counts ---
        a5_01(),
        a5_02(),
        a5_03(),
        a5_04(),
        a5_05(),
        a5_06(),
        // --- A6: interrupts ---
        a6_01(),
        a6_02(),
        a6_03(),
        a6_04(),
        a6_05(),
        a6_06(),
        a6_07(),
        a6_08(),
        // --- A7: decimal mode ---
        a7_01(),
        a7_02(),
        a7_03(),
        a7_04(),
        // --- A8: block move ---
        a8_01(),
        a8_02(),
        a8_03(),
        // --- A9: misc flags ---
        a9_01(),
        a9_02(),
        // --- T-04-A: closing out the remaining enumerated Group A behaviour ---
        a1_06(),
        a5_07(),
        a6_09(),
        a5_08(),
    ]
}

// ---------------------------------------------------------------------------------------------
// A1 — emulation vs native mode
// ---------------------------------------------------------------------------------------------

/// `XCE` into emulation forces 8-bit index registers, clearing `XH`/`YH`.
fn a1_01() -> Test {
    let mut a = Asm::new();
    a.c("Entering emulation forces x=1, which zeroes the high bytes of X and Y.");
    a.l("rep #$30");
    a.l("ldx #$1234");
    a.l("ldy #$5678");
    a.enter_emulation();
    a.enter_native();
    a.l("rep #$30");
    a.assert_x16(0x0034, "XH not cleared by entering emulation mode");
    a.assert_y16(0x0078, "YH not cleared by entering emulation mode");
    a.finish(
        "A1.01",
        'A',
        "XCE clears XH/YH",
        Provenance::Documented("SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// `SEP #$10` narrows the index registers and clears their high bytes; widening does not
/// restore them.
fn a1_02() -> Test {
    let mut a = Asm::new();
    a.c("SEP #$10 sets x=1 -> XH/YH are destroyed, not merely hidden.");
    a.l("rep #$30");
    a.l("ldx #$1234");
    a.l("ldy #$5678");
    a.l("sep #$10");
    a.l("rep #$10");
    a.assert_x16(0x0034, "XH survived SEP #$10 (must be cleared, not masked)");
    a.assert_y16(0x0078, "YH survived SEP #$10 (must be cleared, not masked)");
    a.finish(
        "A1.02",
        'A',
        "SEP #$10 clears XH/YH",
        Provenance::Documented("SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// Entering emulation mode forces the stack high byte to `$01`, preserving the low byte.
fn a1_03() -> Test {
    let mut a = Asm::new();
    a.c("E=1 confines the stack to page 1: SH is forced to $01, SL is untouched.");
    a.l("rep #$30");
    a.l("lda #$05AB");
    a.l("tcs");
    a.enter_emulation();
    a.enter_native();
    a.l("rep #$30");
    a.l("tsc");
    a.assert_a16(0x01AB, "SH not forced to $01 on entering emulation mode");
    a.finish(
        "A1.03",
        'A',
        "E=1 forces SH=$01",
        Provenance::Documented("6502.org 65c816opcodes; WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// In emulation mode `TXS` writes only the low byte: `X=$FF` yields `S=$01FF`, not `$00FF`.
fn a1_04() -> Test {
    let mut a = Asm::new();
    a.c("TXS in emulation: 8-bit X into SL, SH stays $01.");
    a.enter_emulation();
    a.l("ldx #$FF");
    a.l("txs");
    a.enter_native();
    a.l("rep #$30");
    a.l("tsc");
    a.assert_a16(0x01FF, "TXS in emulation did not keep SH=$01");
    a.finish(
        "A1.04",
        'A',
        "TXS in emu -> $01FF",
        Provenance::Documented("6502.org 65c816opcodes"),
        Kind::Scored,
        None,
    )
}

/// `TCS` transfers all 16 bits of `C` even when the accumulator is 8-bit, but emulation still
/// forces `SH`.
fn a1_05() -> Test {
    let mut a = Asm::new();
    a.c("TCS is always 16-bit; in emulation SH is then forced back to $01.");
    a.l("rep #$30");
    a.l("lda #$1234");
    a.c("C keeps $1234 internally even though the 8-bit view shows only $34.");
    a.enter_emulation();
    a.l("tcs               ; S = $0134");
    a.enter_native();
    a.l("rep #$30");
    a.l("tsc");
    a.assert_a16(0x0134, "TCS in emulation did not yield $0134");
    a.finish(
        "A1.05",
        'A',
        "TCS in emu -> $0134",
        Provenance::Documented("6502.org 65c816opcodes"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A2 — direct-page wrapping
// ---------------------------------------------------------------------------------------------

/// `d,X` never crosses a bank boundary: the effective address wraps inside bank `$00`.
///
/// This is why the ROM is 128 KiB — inside a 32 KiB image bank `$00` and bank `$01` mirror the
/// same bytes and the wrap is unobservable. Each bank carries a signature byte at `$xx:8005`.
fn a2_01() -> Test {
    let mut a = Asm::new();
    a.c("D=$FFFF, X=$8000, lda $06,X  ->  $FFFF+$06+$8000 = $18005, wraps to $00:8005.");
    a.c("A bank-crossing implementation would read $01:8005 instead (a different ROM bank).");
    a.l("rep #$30");
    a.l("lda #$FFFF");
    a.l("tcd");
    a.l("ldx #$8000");
    a.l("sep #$20          ; 8-bit A for the byte compare");
    a.l("lda $06,X");
    a.assert_a8(
        0xA0,
        "d,X crossed into bank $01 instead of wrapping within bank $00",
    );
    a.finish(
        "A2.01",
        'A',
        "d,X never crosses bank",
        Provenance::Documented("superfamicom.org 65816 reference, worked example"),
        Kind::Scored,
        None,
    )
}

/// Emulation mode with `DL == $00` wraps the direct-page index inside the page.
fn a2_02() -> Test {
    let mut a = Asm::new();
    a.c("E=1, DL=$00: lda $FF,X with X=$01 wraps to D+$00, not D+$100.");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd               ; D=$0000, so DL=$00");
    a.c("Seed the two candidate addresses through a long store (DBR-independent).");
    a.l("sep #$20");
    a.l("lda #$5A");
    a.l("sta f:$7E0000     ; the wrapped target");
    a.l("lda #$A5");
    a.l("sta f:$7E0100     ; the non-wrapped target");
    a.l("rep #$30");
    a.l("ldx #$0001");
    a.enter_emulation();
    a.l("lda $FF,X         ; 8-bit in emulation");
    a.enter_native();
    a.assert_a8(
        0x5A,
        "emulation DL=$00 did not page-wrap the direct-page index",
    );
    a.finish(
        "A2.02",
        'A',
        "E=1 DL=$00 page wraps",
        Provenance::Documented("superfamicom.org 65816 reference"),
        Kind::Scored,
        None,
    )
}

/// Emulation mode with `DL != $00` does *not* page-wrap — it carries into the next page.
fn a2_03() -> Test {
    let mut a = Asm::new();
    a.c("E=1, DL=$10: the page-wrap special case does not apply; the index carries.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$3C");
    a.l("sta f:$7E0110     ; D+$FF+$01 = $0010+$0100 = $0110");
    a.l("lda #$C3");
    a.l("sta f:$7E0010     ; the address a wrongly-wrapping core would hit");
    a.l("rep #$30");
    a.l("lda #$0010");
    a.l("tcd               ; D=$0010, DL=$10 (non-zero)");
    a.l("ldx #$0001");
    a.enter_emulation();
    a.l("lda $FF,X");
    a.enter_native();
    a.assert_a8(
        0x3C,
        "emulation DL!=$00 wrongly page-wrapped the direct-page index",
    );
    a.finish(
        "A2.03",
        'A',
        "E=1 DL!=$00 carries",
        Provenance::Documented("superfamicom.org 65816 reference"),
        Kind::Scored,
        None,
    )
}

/// Native mode always carries out of the page, regardless of `DL`.
fn a2_04() -> Test {
    let mut a = Asm::new();
    a.c("E=0, D=$0000: lda $FF,X with X=$01 reaches $0100 — no 6502-style page wrap.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$77");
    a.l("sta f:$7E0100");
    a.l("lda #$88");
    a.l("sta f:$7E0000");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("ldx #$0001");
    a.l("sep #$20");
    a.l("lda $FF,X");
    a.assert_a8(
        0x77,
        "native mode page-wrapped the direct-page index (it must carry)",
    );
    a.finish(
        "A2.04",
        'A',
        "Native always carries",
        Provenance::Documented("superfamicom.org 65816 reference"),
        Kind::Scored,
        None,
    )
}

/// `[dp]` is a "new" addressing mode: its pointer fetch never page-wraps, even at `E=1`/`DL=$00`.
fn a2_05() -> Test {
    let mut a = Asm::new();
    a.c("E=1, D=$0000, [dp] with dp=$FF: pointer bytes come from $FF, $0100, $0101 —");
    a.c("the 6502 page-wrap does NOT apply to the 65816's new addressing modes.");
    a.l("rep #$30");
    a.c("Build a 24-bit pointer to $7E1234 across the page boundary.");
    a.l("sep #$20");
    a.l("lda #$34");
    a.l("sta f:$7E00FF");
    a.l("lda #$12");
    a.l("sta f:$7E0100");
    a.l("lda #$7E");
    a.l("sta f:$7E0101");
    a.l("lda #$6B");
    a.l("sta f:$7E1234     ; the value a correct core will fetch");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("sep #$20");
    a.l("lda [$FF]");
    a.assert_a8(0x6B, "[dp] pointer fetch page-wrapped (new modes must not)");
    a.finish(
        "A2.05",
        'A',
        "[dp] never page-wraps",
        Provenance::Documented("superfamicom.org 65816 reference; WDC datasheet"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A3 — stack wrapping
// ---------------------------------------------------------------------------------------------

/// Emulation mode confines the stack to page 1: pushing past `$0100` wraps to `$01FF`.
fn a3_01() -> Test {
    let mut a = Asm::new();
    a.c("E=1, S=$0100: PHA writes $01:0100 and S wraps to $01FF.");
    a.l("rep #$30");
    a.l("lda #$0100");
    a.l("tcs");
    a.enter_emulation();
    a.l("lda #$9D");
    a.l("pha               ; writes $00:0100, S -> $01FF");
    a.enter_native();
    a.l("rep #$30");
    a.l("tsc");
    a.assert_a16(0x01FF, "emulation stack did not wrap within page 1");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.c("Note: $00:0100 is the low-WRAM mirror of $7E:0100.");
    a.assert_a8(0x9D, "PHA did not write the byte at $0100");
    a.finish(
        "A3.01",
        'A',
        "E=1 stack wraps pg1",
        Provenance::Documented("WDC datasheet; 6502.org 65c816opcodes"),
        Kind::Scored,
        None,
    )
}

/// `PEA` escapes page 1 even in emulation mode — one of the ten "new" instructions that ignore
/// the emulation stack confinement.
fn a3_02() -> Test {
    let mut a = Asm::new();
    a.c("E=1, S=$0100, PEA $1234: writes $00:0100 and $00:00FF, S -> $01FE.");
    a.c("$01FF must be left untouched — proof the push escaped page 1.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$EE");
    a.l("sta f:$7E01FF     ; sentinel that must survive");
    a.l("rep #$30");
    a.l("lda #$0100");
    a.l("tcs");
    a.enter_emulation();
    a.l("pea $1234");
    a.enter_native();
    a.l("rep #$30");
    a.l("tsc");
    a.assert_a16(0x01FE, "PEA in emulation did not leave S at $01FE");
    a.l("sep #$20");
    a.l("lda f:$7E01FF");
    a.assert_a8(0xEE, "PEA wrongly wrapped into page 1 and clobbered $01FF");
    a.l("lda f:$7E0100");
    a.assert_a8(0x12, "PEA high byte not written to $0100");
    a.l("lda f:$7E00FF");
    a.assert_a8(
        0x34,
        "PEA low byte not written to $00FF (it must escape page 1)",
    );
    a.finish(
        "A3.02",
        'A',
        "PEA escapes page 1",
        Provenance::Documented("WDC datasheet; hardware-confirmed per superfamicom.org"),
        Kind::Scored,
        None,
    )
}

/// The sharpest old-vs-new discriminator: at the same `S`, `PLD` escapes page 1 while `PLY`
/// does not.
fn a3_03() -> Test {
    let mut a = Asm::new();
    a.c("E=1, S=$01FF. PLD (new) pulls from $0200/$0201; PLY (old) pulls from $0100.");
    a.c("This pair is the cleanest test of the emulation-mode stack-confinement rule.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$AB");
    a.l("sta f:$7E0200");
    a.l("lda #$CD");
    a.l("sta f:$7E0201");
    a.l("lda #$5E");
    a.l("sta f:$7E0100");
    a.l("rep #$30");
    a.c("--- PLD: must escape ---");
    a.l("lda #$01FF");
    a.l("tcs");
    a.enter_emulation();
    a.l("pld");
    a.enter_native();
    a.l("rep #$30");
    a.l("tdc");
    a.assert_a16(
        0xCDAB,
        "PLD did not pull from $0200/$0201 (it must escape page 1)",
    );
    a.c("--- PLY: must NOT escape ---");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("lda #$01FF");
    a.l("tcs");
    a.enter_emulation();
    a.l("ply               ; 8-bit index in emulation");
    a.enter_native();
    a.l("rep #$30");
    a.assert_y16(
        0x005E,
        "PLY escaped page 1 (old instructions must wrap to $0100)",
    );
    a.finish(
        "A3.03",
        'A',
        "PLD escapes, PLY not",
        Provenance::Documented("WDC datasheet; superfamicom.org escape list"),
        Kind::Scored,
        None,
    )
}

/// Stack-relative addressing `d,S` escapes page 1 in emulation mode.
fn a3_04() -> Test {
    let mut a = Asm::new();
    a.c("E=1, S=$01FF, lda $02,S -> $00:0201, not a page-1 wrap.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$71");
    a.l("sta f:$7E0201");
    a.l("lda #$17");
    a.l("sta f:$7E0101");
    a.l("rep #$30");
    a.l("lda #$01FF");
    a.l("tcs");
    a.enter_emulation();
    a.l("lda $02,S");
    a.enter_native();
    a.assert_a8(0x71, "d,S wrapped inside page 1 (it must escape)");
    a.finish(
        "A3.04",
        'A',
        "d,S escapes page 1",
        Provenance::Documented("WDC datasheet; hardware-confirmed per superfamicom.org"),
        Kind::Scored,
        None,
    )
}

/// The stack is always in bank `$00`, even in native mode with a 16-bit stack pointer.
fn a3_05() -> Test {
    let mut a = Asm::new();
    a.c("Native, S=$0000: PHA writes $00:0000 and S wraps to $FFFF — never bank $01.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0000");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcs");
    a.l("sep #$20");
    a.l("lda #$4D");
    a.l("pha");
    a.l("rep #$30");
    a.l("tsc");
    a.assert_a16(0xFFFF, "native stack did not wrap to $FFFF within bank $00");
    a.l("sep #$20");
    a.l("lda f:$7E0000");
    a.assert_a8(0x4D, "push at S=$0000 did not land at $00:0000");
    a.finish(
        "A3.05",
        'A',
        "Stack in bank $00",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A4 — absolute / jump wrapping
// ---------------------------------------------------------------------------------------------

/// The NMOS 6502 `JMP ($xxFF)` page-boundary bug is fixed on the 65816.
fn a4_01() -> Test {
    let mut a = Asm::new();
    a.c("JMP ($12FF) must read its high byte from $1300, not wrap to $1200.");
    a.l("rep #$30");
    a.c("Point $12FF/$1300 at the success continuation.");
    a.l("lda #.LOWORD(@landed)");
    a.l("sta f:$7E12FF");
    a.c("Poison $1200 so a buggy (wrapping) core lands somewhere else.");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E1200");
    a.l("rep #$30");
    a.c("$12FF is reachable as bank-$00 low WRAM mirror.");
    a.l("jmp ($12FF)");
    a.label("landed");
    a.l("nop");
    a.finish(
        "A4.01",
        'A',
        "JMP (a) no page bug",
        Provenance::Documented("WDC datasheet; SNESdev Errata"),
        Kind::Scored,
        None,
    )
}

/// `JMP (a)` reads its pointer from bank `$00` regardless of the program bank.
fn a4_02() -> Test {
    let mut a = Asm::new();
    a.c("The indirect pointer for JMP (a) always comes from bank $00.");
    a.c("Executed from bank $00 here, so this asserts the pointer fetch itself works;");
    a.c("the cross-bank half of the rule is exercised by A4.03.");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@landed)");
    a.l("sta f:$7E0300");
    a.l("jmp ($0300)");
    a.label("landed");
    a.l("nop");
    a.finish(
        "A4.02",
        'A',
        "JMP (a) ptr bank $00",
        Provenance::Documented("SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// `abs,X` carries out of the data bank into the next bank.
fn a4_03() -> Test {
    let mut a = Asm::new();
    a.c("DBR=$00, X=$8006, lda $FFFF,X  ->  $00FFFF + $8006 = $01:8005,");
    a.c("i.e. the bank $01 signature byte. A core that masked the effective address to");
    a.c("16 bits would wrap back into bank $00 and read $FF instead.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb               ; DBR = $00");
    a.l("ldx #$8006");
    a.l("sep #$20");
    a.l("lda $FFFF,X");
    a.assert_a8(0xA1, "abs,X did not carry into bank $01");
    a.finish(
        "A4.03",
        'A',
        "abs,X carries bank",
        Provenance::Documented("WDC datasheet; superfamicom.org"),
        Kind::Scored,
        None,
    )
}

/// A 16-bit absolute read spanning `$FFFF` continues into the next bank.
fn a4_04() -> Test {
    let mut a = Asm::new();
    a.c("DBR=$00, m=0, lda $FFFF: low byte from $00:FFFF, high byte from $01:0000.");
    a.c("$00:FFFF is the emulation-mode IRQ vector high byte; $01:0000 is low WRAM.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$C7");
    a.l("sta f:$7E0000     ; = $01:0000 through the low-WRAM mirror");
    a.l("rep #$30");
    a.l("lda $FFFF");
    a.l("xba");
    a.l("and #$00FF");
    a.assert_a16(
        0x00C7,
        "16-bit absolute read did not carry into the next bank",
    );
    a.finish(
        "A4.04",
        'A',
        "16-bit abs carries bank",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// `long,X` carries across banks as a full 24-bit addition.
fn a4_05() -> Test {
    let mut a = Asm::new();
    a.c("lda $00FFFF,X with X=$8006 -> $00FFFF + $8006 = $01:8005 (bank $01 signature).");
    a.l("rep #$30");
    a.l("ldx #$8006");
    a.l("sep #$20");
    a.l("lda f:$00FFFF,X");
    a.assert_a8(0xA1, "long,X did not carry into bank $01");
    a.finish(
        "A4.05",
        'A',
        "long,X carries bank",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A7 — decimal mode
// ---------------------------------------------------------------------------------------------

/// 8-bit BCD addition.
fn a7_01() -> Test {
    let mut a = Asm::new();
    a.c("SED; CLC; LDA #$09; ADC #$01 -> A=$10, C=0.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("sed");
    a.l("clc");
    a.l("lda #$09");
    a.l("adc #$01");
    a.l("cld");
    a.assert_a8(0x10, "8-bit BCD ADC produced the wrong result");
    a.finish(
        "A7.01",
        'A',
        "8-bit BCD ADC",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// 16-bit BCD addition operates across the full accumulator.
fn a7_02() -> Test {
    let mut a = Asm::new();
    a.c("m=0; SED; CLC; LDA #$0999; ADC #$0001 -> A=$1000.");
    a.l("rep #$30");
    a.l("sed");
    a.l("clc");
    a.l("lda #$0999");
    a.l("adc #$0001");
    a.l("cld");
    a.assert_a16(0x1000, "16-bit BCD ADC produced the wrong result");
    a.finish(
        "A7.02",
        'A',
        "16-bit BCD ADC",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// 8-bit BCD subtraction.
fn a7_03() -> Test {
    let mut a = Asm::new();
    a.c("SED; SEC; LDA #$10; SBC #$01 -> A=$09.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("sed");
    a.l("sec");
    a.l("lda #$10");
    a.l("sbc #$01");
    a.l("cld");
    a.assert_a8(0x09, "8-bit BCD SBC produced the wrong result");
    a.finish(
        "A7.03",
        'A',
        "8-bit BCD SBC",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// The overflow flag after a decimal-mode `ADC` — a golden vector, never scored.
///
/// This is the one Group A behaviour that cannot honestly be asserted. Reading ares, bsnes, and
/// Mesen2 side by side, all three compute `V` with the *identical* binary-overflow formula
/// evaluated **before** the BCD `+$60` correction — so they agree, but only because they picked
/// the same convention, not because hardware defines one. Asserting their shared answer would
/// manufacture authority the sources do not have.
///
/// So the test reports what it observed as a **variant code** (1 = `V` set, 2 = `V` clear) and
/// is marked [`Kind::Golden`], which keeps it out of the pass rate entirely. If a future
/// hardware run settles it, this becomes a scored test and the tier changes with it.
fn a7_04() -> Test {
    let mut a = Asm::new();
    a.c("SED; CLC; LDA #$99; ADC #$01 -> A=$00 with carry out. The A result is documented and");
    a.c("is checked; V is NOT — it is captured and reported as a variant instead.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("sed");
    a.l("clc");
    a.l("lda #$99");
    a.l("adc #$01");
    a.l("cld");
    a.l("php");
    a.l("pla               ; stash P before the compare clobbers it");
    a.l("sta f:$7E0060");
    a.l("txa");
    a.c("Re-run to get A back for the documented half of the check.");
    a.l("sed");
    a.l("clc");
    a.l("lda #$99");
    a.l("adc #$01");
    a.l("cld");
    a.assert_a8(0x00, "decimal ADC $99+$01 did not produce $00");
    a.c("Now branch on the observed V bit (P bit 6) and report it as a variant.");
    a.l("lda f:$7E0060");
    a.l("and #$40");
    a.l("beq :+");
    a.l("lda #$03          ; variant 1 = V observed SET   ((1<<1)|1)");
    a.l("sta f:$7EE010");
    a.l("jmp test_restore");
    a.l(":");
    a.l("lda #$05          ; variant 2 = V observed CLEAR ((2<<1)|1)");
    a.l("sta f:$7EE010");
    a.l("jmp test_restore");
    a.finish(
        "A7.04",
        'A',
        "Decimal V (golden)",
        Provenance::Contested(
            "V in decimal mode is undefined; ares/bsnes/Mesen2 share a convention, \
             which is agreement, not authority",
        ),
        Kind::Golden,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A8 — block move
// ---------------------------------------------------------------------------------------------

/// `MVN` copies forward and leaves `A = $FFFF`.
fn a8_01() -> Test {
    let mut a = Asm::new();
    a.c("Copy 4 bytes within bank $7E using MVN; A must end at $FFFF.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$11");
    a.l("sta f:$7E2000");
    a.l("lda #$22");
    a.l("sta f:$7E2001");
    a.l("lda #$33");
    a.l("sta f:$7E2002");
    a.l("lda #$44");
    a.l("sta f:$7E2003");
    a.l("rep #$30");
    a.l("ldx #$2000");
    a.l("ldy #$2100");
    a.l("lda #$0003        ; count-1");
    a.l("mvn #$7E,#$7E    ; literal bank numbers; `mvn $7E,$7E` would mean bank $00");
    a.l("phk");
    a.l("plb               ; MVN left DBR = destination bank; restore for later reads");
    a.assert_a16(0xFFFF, "MVN did not leave A = $FFFF");
    a.l("sep #$20");
    a.l("lda f:$7E2100");
    a.assert_a8(0x11, "MVN did not copy the first byte");
    a.l("lda f:$7E2103");
    a.assert_a8(0x44, "MVN did not copy the last byte");
    a.finish(
        "A8.01",
        'A',
        "MVN fwd, A=$FFFF",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// `MVN` leaves `DBR` set to the destination bank — permanently.
fn a8_02() -> Test {
    let mut a = Asm::new();
    a.c("After MVN $7E,$7E the data bank register is the destination bank ($7E), not $00.");
    a.l("rep #$30");
    a.l("ldx #$2000");
    a.l("ldy #$2100");
    a.l("lda #$0000        ; one byte");
    a.l("mvn #$7E,#$7E    ; literal bank numbers; `mvn $7E,$7E` would mean bank $00");
    a.l("phb");
    a.l("sep #$20");
    a.l("pla");
    a.assert_a8(0x7E, "MVN did not leave DBR = destination bank");
    a.finish(
        "A8.02",
        'A',
        "MVN sets DBR=dest",
        Provenance::Documented("SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// `MVP` copies backward (descending addresses).
fn a8_03() -> Test {
    let mut a = Asm::new();
    a.c("MVP walks X and Y downward; point them at the LAST byte of each block.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$AA");
    a.l("sta f:$7E2200");
    a.l("lda #$BB");
    a.l("sta f:$7E2201");
    a.l("rep #$30");
    a.l("ldx #$2201");
    a.l("ldy #$2301");
    a.l("lda #$0001        ; two bytes");
    a.l("mvp #$7E,#$7E    ; literal bank numbers; `mvp $7E,$7E` would mean bank $00");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda f:$7E2300");
    a.assert_a8(0xAA, "MVP did not copy the low byte");
    a.l("lda f:$7E2301");
    a.assert_a8(0xBB, "MVP did not copy the high byte");
    a.finish(
        "A8.03",
        'A',
        "MVP copies backward",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A9 — misc flags
// ---------------------------------------------------------------------------------------------

/// `BIT #imm` affects only `Z`; `BIT abs` also sets `N` and `V` from memory bits 7/6 (or 15/14).
fn a9_01() -> Test {
    let mut a = Asm::new();
    a.c("BIT #imm must leave N and V untouched; BIT dp must set them from the operand.");
    a.c("BIT has no long-addressing mode, so the operand is staged in the direct page");
    a.c("(DP = $0000, so dp $50 is $00:0050, the low-WRAM mirror of $7E:0050).");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$C0");
    a.l("sta f:$7E0050");
    a.c("Clear N and V, then BIT #$00 — Z should set, N/V must stay clear.");
    a.l("clv");
    a.l("lda #$01");
    a.l("bit #$00");
    a.l("php");
    a.l("pla");
    a.l("and #$C0          ; N and V");
    a.assert_a8(0x00, "BIT #imm wrongly modified N or V");
    a.c("Now BIT dp against $C0 — both N and V must set.");
    a.l("lda #$FF");
    a.l("bit $50");
    a.l("php");
    a.l("pla");
    a.l("and #$C0");
    a.assert_a8(0xC0, "BIT dp did not set N and V from memory bits 7/6");
    a.finish(
        "A9.01",
        'A',
        "BIT imm vs abs",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// `XBA` swaps the accumulator halves and sets `N`/`Z` from the new low byte.
fn a9_02() -> Test {
    let mut a = Asm::new();
    a.c("XBA is always a 16-bit operation on C, regardless of the m flag.");
    a.l("rep #$30");
    a.l("lda #$1234");
    a.l("xba");
    a.assert_a16(0x3412, "XBA did not swap the accumulator halves");
    a.finish(
        "A9.02",
        'A',
        "XBA swaps A halves",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A5 — cycle counts
//
// Measured through the PPU's H counter (see `Asm::measure_begin`). Each test compares TWO
// measurements that differ only in the property under test, so the constant measurement overhead
// cancels and only the property remains. One CPU cycle is 6 master clocks = 1.5 dots, so every
// sequence is repeated 16 times: 16 cycles = 96 clocks = 24 dots, well clear of the counter's
// +/-1 dot resolution.
//
// `DOTS_PER_16_CYCLES` is the expected delta for a one-cycle-per-iteration difference.
// ---------------------------------------------------------------------------------------------

/// Dots elapsed per 8 extra *internal* CPU cycles: 8 x 6 master clocks / 4 clocks-per-dot.
/// Internal cycles are always 6 clocks; a memory access to WRAM is 8, which is why the RMW test
/// below expects a different figure.
const DOTS_PER_8_INTERNAL: u16 = 12;

/// Tolerance on every timing comparison, in dots.
///
/// Measured, not guessed: eight runs of an identical sequence span exactly one dot, which is the
/// irreducible quantisation of a 6/8-master-clock CPU cycle against a 4-clock dot. Two dots is
/// therefore generous; anything wider would start to blur "the penalty applied" into "it did not".
const TOL: u16 = 2;

/// The `+1 w` penalty: direct-page addressing costs an extra cycle when `DL != 0`.
///
/// Singled out because it is, by a wide margin, the most commonly mis-implemented 65816 timing
/// rule — it keys off the *direct-page register's* low byte, not the effective address.
fn a5_01() -> Test {
    let mut a = Asm::new();
    a.c("Identical instruction, identical operand — only D changes. D=$0000 costs N cycles,");
    a.c("D=$0001 costs N+1, because the penalty keys off DL != 0.");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.l("lda #$0000");
    a.l("tcd               ; DL = $00: no penalty");
    a.measure_begin();
    a.repeat(8, &["lda $10,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080     ; baseline");
    a.l("lda #$0001");
    a.l("tcd               ; DL = $01: one extra cycle per access");
    a.measure_begin();
    a.repeat(8, &["lda $10,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080");
    a.assert_a16_range(
        DOTS_PER_8_INTERNAL - TOL,
        DOTS_PER_8_INTERNAL + TOL,
        "direct-page +1 w penalty (DL != 0) not applied",
    );
    a.finish(
        "A5.01",
        'A',
        "+1 w when DL != 0",
        Provenance::Documented("WDC datasheet; superfamicom.org cycle-count tables"),
        Kind::Scored,
        None,
    )
}

/// The `+1 p` penalty: an indexed **read** costs an extra cycle when it crosses a page.
fn a5_02() -> Test {
    let mut a = Asm::new();
    a.c("lda $10FF,X with X=$01 crosses a page; with X=$00 it does not.");
    a.c("The index must be 8-BIT: with a 16-bit index the cycle is unconditional, so the two");
    a.c("measurements would be identical and the test would prove nothing.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$10");
    a.l("ldx #$00");
    a.measure_begin();
    a.repeat(8, &["lda a:$10FF,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080");
    a.l("sep #$10");
    a.l("ldx #$01          ; now every access crosses into the next page");
    a.measure_begin();
    a.repeat(8, &["lda a:$10FF,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080");
    a.assert_a16_range(
        DOTS_PER_8_INTERNAL - TOL,
        DOTS_PER_8_INTERNAL + TOL,
        "indexed read did not pay the +1 p page-cross penalty",
    );
    a.finish(
        "A5.02",
        'A',
        "+1 p on indexed reads",
        Provenance::Documented("WDC datasheet; superfamicom.org cycle-count tables"),
        Kind::Scored,
        None,
    )
}

/// Indexed **stores** always pay the page-cross cycle, crossing or not.
///
/// The asymmetry with [`a5_02`] is the point: a core that treats `+1 p` uniformly across reads
/// and writes gets one of the two wrong, and this pair catches whichever it is.
fn a5_03() -> Test {
    let mut a = Asm::new();
    a.c("sta $10FF,X costs the same whether or not it crosses a page — stores always pay the");
    a.c("cycle, so unlike the load in A5.02 there is no difference to find. Same 8-bit index.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$10");
    a.l("ldx #$00");
    a.measure_begin();
    a.repeat(8, &["sta a:$10FF,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080");
    a.l("sep #$10");
    a.l("ldx #$01          ; crosses a page, but must cost the same");
    a.measure_begin();
    a.repeat(8, &["sta a:$10FF,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080");
    a.assert_abs_le(
        TOL,
        "indexed store timing changed with page crossing (it must not)",
    );
    a.finish(
        "A5.03",
        'A',
        "Stores always pay +1 p",
        Provenance::Documented("WDC datasheet; superfamicom.org cycle-count tables"),
        Kind::Scored,
        None,
    )
}

/// Decimal mode costs nothing extra on the 65816 — unlike the 65C02, where it adds a cycle.
fn a5_04() -> Test {
    let mut a = Asm::new();
    a.c(
        "Same ADC, binary vs decimal. On the 65C02 decimal adds a cycle; on the 65816 it does not.",
    );
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("cld");
    a.measure_begin();
    a.repeat(8, &["clc", "adc #$01"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080     ; baseline");
    a.l("sep #$20");
    a.l("sed");
    a.measure_begin();
    a.repeat(8, &["clc", "adc #$01"]);
    a.measure_end();
    a.l("sep #$20");
    a.l("cld               ; leave decimal before any further arithmetic");
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080");
    a.assert_abs_le(
        TOL,
        "decimal mode changed instruction timing (it must not on the 65816)",
    );
    a.finish(
        "A5.04",
        'A',
        "Decimal costs no cycles",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// A 16-bit read-modify-write costs **two** more cycles than its 8-bit form, not one.
///
/// undisbeliever's widely-copied opcode table lists this as `+1`, which is a transcription error:
/// the extra byte has to be both read and written back. Those two cycles are direct-page memory
/// accesses at 8 master clocks each, not internal cycles at 6, so the expected delta is
/// `8 reps x 2 x 8 / 4 = 32` dots rather than twice [`DOTS_PER_8_INTERNAL`].
fn a5_05() -> Test {
    let mut a = Asm::new();
    a.c("ASL dp with m=1 vs m=0. The 16-bit form reads and writes an extra byte.");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("sep #$20          ; 8-bit accumulator -> 8-bit RMW");
    a.measure_begin();
    a.repeat(8, &["asl $20"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080");
    a.l("rep #$20          ; 16-bit accumulator -> 16-bit RMW");
    a.measure_begin();
    a.repeat(8, &["asl $20"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080");
    a.assert_a16_range(
        32 - TOL,
        32 + TOL,
        "16-bit RMW is not +2 cycles over the 8-bit form",
    );
    a.finish(
        "A5.05",
        'A',
        "16-bit RMW is +2",
        Provenance::Documented("WDC datasheet (corrects undisbeliever's table)"),
        Kind::Scored,
        None,
    )
}

/// A taken branch costs one more cycle than an untaken one.
fn a5_06() -> Test {
    let mut a = Asm::new();
    a.c("BCC taken vs not taken, same instruction, same target, one cycle apart.");
    a.l("rep #$30");
    a.l("sec               ; carry set -> BCC never taken");
    a.measure_begin();
    a.repeat(8, &["bcc *+2"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080");
    a.l("clc               ; carry clear -> BCC always taken");
    a.measure_begin();
    a.repeat(8, &["bcc *+2"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080");
    a.assert_a16_range(
        DOTS_PER_8_INTERNAL - TOL,
        DOTS_PER_8_INTERNAL + TOL,
        "a taken branch did not cost one extra cycle",
    );
    a.finish(
        "A5.06",
        'A',
        "Taken branch costs +1",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// A6 — interrupts
//
// The cartridge vectors are fixed at link time, so BRK and COP go through trampolines in the
// runtime that jump via a bank-$00 RAM pointer. Each test installs its own handler for the
// duration of the test — the same trick AccuracyCoin uses by pointing the NES vectors into RAM.
//
// Each body starts by jumping over its handler, since the generated pass/fail stubs follow the
// body and control must not fall through into handler code.
// ---------------------------------------------------------------------------------------------

/// `BRK` vectors through `$FFE6` in native mode and the handler runs in bank `$00`.
fn a6_01() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.label("handler");
    a.l("sep #$20");
    a.l("lda #$A5");
    a.l("sta f:$7E0090     ; prove the handler actually ran");
    a.l("rep #$30");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0090");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_BRK_VEC");
    a.l("brk");
    a.l(".byte $EA         ; BRK signature byte");
    a.assert_mem8(
        0x7E_0090,
        0xA5,
        "native BRK did not reach the installed handler",
    );
    a.finish(
        "A6.01",
        'A',
        "BRK vectors natively",
        Provenance::Documented("WDC datasheet; SNESdev Wiki vectors"),
        Kind::Scored,
        None,
    )
}

/// `COP` vectors through `$FFE4`, separately from `BRK`.
fn a6_02() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.label("handler");
    a.l("sep #$20");
    a.l("lda #$C0");
    a.l("sta f:$7E0091");
    a.l("rep #$30");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0091");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_COP_VEC");
    a.c("Encoded as raw bytes rather than `cop #$00`. Immediate addressing on COP is accepted by");
    a.c("recent ca65 git builds but rejected as an illegal addressing mode by the 2.19 release");
    a.c("that CI installs from apt, and the ROM must assemble identically on both.");
    a.l(".byte $02, $00    ; cop #$00");
    a.assert_mem8(0x7E_0091, 0xC0, "COP did not reach its own vector");
    a.finish(
        "A6.02",
        'A',
        "COP has its own vector",
        Provenance::Documented("WDC datasheet; SNESdev Wiki vectors"),
        Kind::Scored,
        None,
    )
}

/// The decimal flag is cleared on interrupt entry — a 65C02/65816 behaviour the NMOS 6502 lacks.
fn a6_03() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.label("handler");
    a.l("sep #$20");
    a.l("php");
    a.l("pla");
    a.l("and #$08          ; D flag");
    a.l("sta f:$7E0092");
    a.l("rep #$30");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_BRK_VEC");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("sta f:$7E0092     ; poison, so 'handler never ran' cannot look like a pass");
    a.l("sed");
    a.l("brk");
    a.l(".byte $EA");
    a.l("cld");
    a.assert_mem8(0x7E_0092, 0x00, "D was not cleared on interrupt entry");
    a.finish(
        "A6.03",
        'A',
        "D cleared on interrupt",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// The interrupt-disable flag is set on interrupt entry.
fn a6_04() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.label("handler");
    a.l("sep #$20");
    a.l("php");
    a.l("pla");
    a.l("and #$04          ; I flag");
    a.l("sta f:$7E0093");
    a.l("rep #$30");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_BRK_VEC");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0093");
    a.l("cli");
    a.l("brk");
    a.l(".byte $EA");
    a.assert_mem8(0x7E_0093, 0x04, "I was not set on interrupt entry");
    a.finish(
        "A6.04",
        'A',
        "I set on interrupt",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// A native-mode interrupt pushes **four** bytes: `PBR`, `PCH`, `PCL`, `P`.
fn a6_05() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.label("handler");
    a.l("rep #$30");
    a.l("tsc");
    a.l("sta f:$7E0094     ; stack pointer as seen inside the handler");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_BRK_VEC");
    a.l("tsc");
    a.l("sta f:$7E0096     ; stack pointer before the interrupt");
    a.l("brk");
    a.l(".byte $EA");
    a.l("rep #$30");
    a.l("lda f:$7E0096");
    a.l("sec");
    a.l("sbc f:$7E0094");
    a.assert_a16(4, "native interrupt did not push exactly 4 bytes");
    a.finish(
        "A6.05",
        'A',
        "Native pushes 4 bytes",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// An emulation-mode interrupt pushes **three** bytes — no program bank.
///
/// Only the stack pointer's LOW byte is compared. In emulation `S` is confined to `$01xx`, so the
/// low byte carries all the information — and, crucially, `REP` is ignored while `E=1` (that is
/// what A1.03 tests), so an attempt to widen the accumulator inside this test would silently do
/// nothing and a 16-bit store would write one byte over stale memory. Cross-validation caught
/// exactly that: an earlier version of this test passed on RustySNES and snes9x, whose leftover
/// high bytes happened to match, and failed on Mesen2, whose did not.
fn a6_06() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.label("handler");
    a.l("tsx               ; X = S low byte (8-bit index in emulation)");
    a.l("txa");
    a.l("sta f:$7E0098");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_BRK_VEC");
    a.c("Poison both slots so a handler that never runs cannot look like a pass.");
    a.l("sep #$20");
    a.l("lda #$EE");
    a.l("sta f:$7E0098");
    a.l("lda #$EE");
    a.l("sta f:$7E0099");
    a.enter_emulation();
    a.l("tsx");
    a.l("txa");
    a.l("sta f:$7E0099     ; S low byte before the interrupt");
    a.l("brk");
    a.l(".byte $EA");
    a.enter_native();
    a.l("sep #$20");
    a.l("lda f:$7E0099");
    a.l("sec");
    a.l("sbc f:$7E0098");
    a.assert_a8(3, "emulation interrupt did not push exactly 3 bytes");
    a.finish(
        "A6.06",
        'A',
        "Emulation pushes 3 bytes",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// `BRK` is a two-byte instruction: the pushed `PC` skips the signature byte.
///
/// Proven behaviourally rather than by inspecting the stack — the signature byte is `SEC`, so if
/// `RTI` came back one byte short it would execute and set carry.
fn a6_07() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.label("handler");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_BRK_VEC");
    a.l("clc               ; RTI restores P, so carry comes back CLEAR...");
    a.l("brk");
    a.l(".byte $38         ; ...unless PC returned here, where $38 is SEC");
    a.l("php");
    a.l("sep #$20");
    a.l("pla");
    a.l("and #$01          ; carry");
    a.assert_a8(0x00, "BRK pushed PC+1, so the signature byte executed");
    a.finish(
        "A6.07",
        'A',
        "BRK skips signature byte",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

/// `WDM` (`$42`) is a reserved two-byte no-op, not a one-byte one.
///
/// Same trick as [`a6_07`]: the second byte is `SEC`, so a core that treats `WDM` as one byte
/// executes it and sets carry.
fn a6_08() -> Test {
    let mut a = Asm::new();
    a.c("WDM consumes its operand byte. If it did not, the $38 below would run as SEC.");
    a.l("rep #$30");
    a.l("lda #$1234");
    a.l("clc");
    a.l(".byte $42, $38    ; WDM #$38  (the operand is SEC if wrongly executed)");
    a.l("sta f:$7E00A0     ; stash A before the carry check clobbers it");
    a.l("php");
    a.l("sep #$20");
    a.l("pla");
    a.l("and #$01          ; carry");
    a.assert_a8(0x00, "WDM did not consume its operand byte");
    a.l("rep #$30");
    a.l("lda f:$7E00A0");
    a.assert_a16(0x1234, "WDM disturbed the accumulator");
    a.finish(
        "A6.08",
        'A',
        "WDM is a 2-byte NOP",
        Provenance::Documented("WDC datasheet"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// T-04-A — the remaining enumerated Group A behaviour
//
// Several of these clobber the stack pointer or the direct-page register. Every one of them
// restores the clobbered register **before** its assertion, never after: an assertion that fails
// jumps straight to a failure stub and then to `test_restore`, so a restore placed after the
// comparison simply does not run on the failing path and takes the rest of the battery down with
// it. Stash, restore, then assert.
// ---------------------------------------------------------------------------------------------

/// `TCD` and `TDC` move all 16 bits regardless of the `m` flag.
///
/// The direct-page register has no 8-bit form, so the accumulator width must not gate the
/// transfer. A core that routes these through its generic "respect `m`" transfer path loses the
/// high byte of `D` and every direct-page access afterwards resolves to the wrong page.
fn a1_06() -> Test {
    let mut a = Asm::new();
    a.c("Set D with m=1, read it back with m=0. D is restored before the assertion because a");
    a.c("failing path would otherwise leave every direct-page access in the battery relocated.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("lda #$1234");
    a.l("sep #$20          ; m=1: an 8-bit accumulator must not narrow the transfer");
    a.l("tcd");
    a.l("rep #$20");
    a.l("tdc");
    a.l("sta f:$7E0114");
    a.l("lda #$0000");
    a.l("tcd               ; restore D BEFORE asserting");
    a.l("lda f:$7E0114");
    a.assert_a16(
        0x1234,
        "TCD/TDC narrowed to 8 bits under m=1 (they are always 16-bit)",
    );
    a.finish(
        "A1.06",
        'A',
        "TCD/TDC always 16-bit",
        Provenance::Documented("WDC datasheet; SNESdev Wiki, 65C816"),
        Kind::Scored,
        None,
    )
}

/// Read-modify-write `abs,X` pays a flat cost — there is no page-cross penalty.
///
/// Unlike a plain indexed read, an RMW always performs the same bus sequence, so crossing a page
/// must cost nothing extra. A core that applies its generic indexed-addressing penalty to RMW
/// instructions makes `ASL $1234,X` cost 8 instead of 7 whenever the index carries.
fn a5_07() -> Test {
    let mut a = Asm::new();
    a.c("Same instruction, once without a page cross and once with. The index must be 8-BIT: a");
    a.c("16-bit index makes the penalty unconditional, which is what A5.02 establishes.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$10");
    a.l("ldx #$00");
    a.measure_begin();
    a.repeat(8, &["asl a:$1234,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0080");
    a.l("sep #$10");
    a.l("ldx #$FF          ; $1234 + $FF = $1333 — crosses into the next page");
    a.measure_begin();
    a.repeat(8, &["asl a:$1234,x"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0080");
    a.assert_abs_le(
        TOL,
        "RMW abs,X paid a page-cross penalty (its cost is flat)",
    );
    a.finish(
        "A5.07",
        'A',
        "RMW abs,X is flat",
        Provenance::Documented("WDC datasheet; undisbeliever's timing tables"),
        Kind::Scored,
        None,
    )
}

/// `BRK` sets the `B` flag in the status byte it pushes, in emulation mode.
///
/// Emulation mode has no separate `BRK` vector — software `BRK` and a hardware IRQ arrive at the
/// same `$FFFE`, and bit 4 of the pushed status byte is the *only* thing that tells the handler
/// which happened. A core that pushes `P` verbatim leaves a handler unable to distinguish them.
fn a6_09() -> Test {
    let mut a = Asm::new();
    a.c("The handler recovers the pushed P through the stack. In emulation BRK pushes PCH, PCL,");
    a.c("P — so P is the last byte written, at $01:(S+1), and TSX gives S's low byte.");
    a.l("jmp @start");
    a.label("handler");
    a.l("tsx");
    a.l("lda a:$0101,x     ; the pushed status byte");
    a.l("sta f:$7E009A");
    a.l("rti");
    a.label("start");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler)");
    a.l("sta a:V_BRK_VEC");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E009A     ; poison, so a handler that never runs cannot pass");
    a.enter_emulation();
    a.l("brk");
    a.l(".byte $EA");
    a.enter_native();
    a.l("sep #$20");
    a.l("lda f:$7E009A");
    a.l("and #$10");
    a.assert_a8(
        0x10,
        "BRK did not set the B flag in the status byte it pushed",
    );
    a.finish(
        "A6.09",
        'A',
        "BRK sets B in pushed P",
        Provenance::Documented("WDC datasheet; SNESdev Wiki, 65C816 interrupts"),
        Kind::Scored,
        None,
    )
}

/// The `A5.22` cycle spot checks — **a golden vector, and the reason T-04-I needs an oracle**.
///
/// # What this measures
///
/// 65816 *cycles* do not map to a fixed number of dots: each cycle is 6, 8 or 12 master clocks
/// depending on what it touches. With code in bank `$00` ROM and the stack in low WRAM (both
/// 8-clock) and internal cycles at 6:
///
/// ```text
/// clocks = 8*mem + 6*internal,  and  cycles = mem + internal
///       => clocks = 6*cycles + 2*mem
/// ```
///
/// where `mem` is instruction length plus data/stack accesses. That second term is what a naive
/// "cycles x constant" conversion misses, and it is why `NOP` and `LDA #imm` — both 2 cycles —
/// cost different amounts of time. From the dossier's cited counts: `NOP` 14 clocks, `XBA` 20,
/// `REP #imm` 22, `PHD` 30, `PLD` 36. Each case below is differential against `NOP`, so the fixed
/// measurement overhead cancels.
///
/// # Why it records instead of asserting
///
/// Written as a scored test first. It failed on **all three** emulators — and on *different*
/// sub-assertions: snes9x on `XBA`, RustySNES on `REP`. Identical failure everywhere usually means
/// the test is wrong; failure at *different* points means something else, namely that the three
/// references do not agree with each other on instruction-level timing. Nothing here can decide
/// which is right, because the only oracle available is the emulators themselves.
///
/// So it reports a bitmask of which expectations matched — bit 0 `XBA`, bit 1 `REP`, bit 2
/// `PHD`/`PLD` — and stays out of the pass rate.
///
/// **This is the blocking finding for T-04-I.** A 256-opcode sweep has exactly this problem 256
/// times over: the mechanism is straightforward, but scoring it requires a per-opcode timing table
/// from an external source (undisbeliever's tables are the obvious candidate) rather than from any
/// emulator. Until that table is sourced and its provenance recorded, a sweep can only ever produce
/// a fingerprint to compare implementations against — useful, but not a pass rate.
fn a5_08() -> Test {
    let mut a = Asm::new();
    a.c("Three differential measurements against NOP; report which matched as a bitmask.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0094     ; the result bitmask");
    a.c("--- baseline: 32 NOPs ---");
    a.measure_begin();
    a.repeat(32, &["nop"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0090");
    a.c("--- XBA: expected +6 clocks each, +48 dots over 32 ---");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(32, &["xba"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0090");
    a.l("cmp #48 - 2");
    a.l("bcc :+");
    a.l("cmp #48 + 3");
    a.l("bcs :+");
    a.l("sep #$20");
    a.l("lda f:$7E0094");
    a.l("ora #$01");
    a.l("sta f:$7E0094");
    a.l("rep #$20");
    a.l(":");
    a.c("--- REP #$00: expected +8 clocks each, +64 dots over 32 ---");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(32, &[".byte $C2, $00   ; rep #$00"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0090");
    a.l("cmp #64 - 2");
    a.l("bcc :+");
    a.l("cmp #64 + 3");
    a.l("bcs :+");
    a.l("sep #$20");
    a.l("lda f:$7E0094");
    a.l("ora #$02");
    a.l("sta f:$7E0094");
    a.l("rep #$20");
    a.l(":");
    a.c("--- PHD+PLD: expected 66 clocks per pair against 28, so +76 dots over 8 ---");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(16, &["nop"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0092");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(8, &["phd", "pld"]);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0092");
    a.l("cmp #76 - 2");
    a.l("bcc :+");
    a.l("cmp #76 + 3");
    a.l("bcs :+");
    a.l("sep #$20");
    a.l("lda f:$7E0094");
    a.l("ora #$04");
    a.l("sta f:$7E0094");
    a.l("rep #$20");
    a.l(":");
    a.c("--- report the bitmask as the variant code ---");
    a.l("sep #$20");
    a.l("lda f:$7E0094");
    a.l("asl a");
    a.l("ora #$01");
    a.l("sta f:$7EE010");
    a.l("jmp test_restore");
    a.finish(
        "A5.08",
        'A',
        "Cycle spot checks (gold)",
        Provenance::Contested(
            "the three reference emulators disagree with each other on instruction-level \
             timing; no external per-opcode timing table is sourced yet",
        ),
        Kind::Golden,
        None,
    )
}
