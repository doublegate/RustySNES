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
        a2_07(),
        a2_10(),
        // --- A3: stack wrapping ---
        a3_01(),
        a3_02(),
        a3_03(),
        a3_04(),
        a3_05(),
        a3_07(),
        a3_09(),
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
        a9_03(),
        a1_07(),
        a9_04(),
        a2_11(),
        a7_05(),
        a6_10(),
        a8_04(),
        a1_08(),
        a1_09(),
        a8_05(),
        a1_10(),
        a4_07(),
        a2_12(),
        a4_09(),
        a4_10(),
        a8_06(),
        a3_08(),
        a3_06(),
        a5_09(),
        a5_10(),
        a6_11(),
        a8_07(),
        a6_12(),
        a4_11(),
        a4_12(),
        a6_13(),
        a6_14(),
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

/// `JSL` escapes page 1: the third pushed byte lands below `$0100`, not wrapped above it.
///
/// The emulation-mode stack is confined to `$01xx` for the instructions the 6502 had. `JSL` is not
/// one of them — it pushes three bytes, and from `S = $0101` the last of them goes to `$00FF`. A
/// core applying the confinement rule uniformly wraps that byte to `$01FF` instead, which corrupts
/// whatever is at the top of the stack page.
///
/// The canary at `$01FF` is the whole discriminator: untouched if the push escaped, overwritten if
/// it wrapped. What landed at `$00FF` is deliberately not asserted — it is the low byte of a return
/// address, so pinning its value would pin where in the ROM this test is assembled, and even
/// "non-zero" is a layout assumption that a `JSL` landing on a `$xxFF` boundary would break. The
/// exact-value form of that claim is `A3.09`, where the pushed bytes are `D`'s and known.
///
/// **The `RTL` half of the dossier row is deliberately not exercised, and the reason is a
/// measurement.** The first version of this test called `JSL` and let the subroutine `RTL` back;
/// RustySNES, snes9x and Mesen2 all *hung*. Three implementations failing identically is the
/// signature of a broken test rather than three broken emulators: after the escaping pushes `S` is
/// `$00FE`, and emulation mode forces the stack's high byte back to `$01` at the next instruction,
/// so there is no return address to pull. The subroutine here therefore leaves emulation mode,
/// rebuilds the stack and jumps back instead of returning.
fn a3_07() -> Test {
    let mut a = Asm::new();
    a.c("The subroutine, jumped over. It does not RTL — see the note above — it leaves emulation");
    a.c("mode, puts the stack somewhere defined, and rejoins the test.");
    a.l("bra @body");
    a.label("sub");
    a.l("clc");
    a.l("xce               ; -> native");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("lda f:V_SAVED_S");
    a.l("tcs");
    a.l("jmp @after");
    a.label("body");
    a.c("Seed the canary at $01FF and clear $00FF, so each has a distinct 'was written' value.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$EE");
    a.l("sta f:$7E01FF");
    a.l("lda #$00");
    a.l("sta f:$7E00FF");
    a.l("rep #$30");
    a.c("E=1, S=$0101: JSL pushes PB to $0101, PCH to $0100, PCL to $00FF.");
    a.l("lda #$0101");
    a.l("tcs");
    a.enter_emulation();
    a.l("jsl @sub");
    a.label("after");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda f:$7E01FF");
    a.assert_a8(
        0xEE,
        "JSL wrapped its third push into page 1 and clobbered $01FF, instead of escaping to $00FF",
    );
    a.finish(
        "A3.07",
        'A',
        "JSL escapes page 1",
        Provenance::Documented("WDC datasheet; superfamicom.org escape list"),
        Kind::Scored,
        None,
    )
}

/// `PHD` escapes page 1: from `S = $0100` its two bytes land at `$0100` and `$00FF`.
///
/// Another instruction the 6502 never had, and so another one the emulation-mode confinement rule
/// does not apply to. From `S = $0100` the first push fills the last byte of the page and the
/// second one leaves it — a core that wraps writes to `$01FF` instead and destroys the top of the
/// stack.
///
/// The dossier pairs `PHD` with `PER`; only `PHD` is asserted here. `PER` pushes a PC-relative
/// address, so its expected value depends on where in the ROM this test happens to be assembled,
/// and a test that has to be re-derived whenever the code above it moves is a test that will one
/// day be re-derived wrongly.
fn a3_09() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$EE");
    a.l("sta f:$7E01FF"); // the canary a wrapping push would destroy
    a.l("rep #$30");
    a.l("lda #$1234");
    a.l("tcd");
    a.l("lda #$0100");
    a.l("tcs");
    a.enter_emulation();
    a.l("phd");
    a.enter_native();
    a.l("rep #$30");
    a.c("Put D back before anything else reads a direct-page address.");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("sep #$20");
    a.l("lda f:$7E01FF");
    a.assert_a8(0xEE, "PHD wrapped into page 1 and clobbered $01FF");
    a.l("lda f:$7E0100");
    a.assert_a8(0x12, "PHD did not write D's high byte to $0100");
    a.l("lda f:$7E00FF");
    a.assert_a8(
        0x34,
        "PHD did not write D's low byte to $00FF — it must escape page 1",
    );
    a.finish(
        "A3.09",
        'A',
        "PHD escapes page 1",
        Provenance::Documented("WDC datasheet; superfamicom.org escape list"),
        Kind::Scored,
        None,
    )
}

/// `(dp),Y` carries into the next bank once the pointer has been loaded.
///
/// The pointer fetch is confined to the direct page, but the `Y` that is added afterwards is added
/// to a full 24-bit address formed with the data bank — so a pointer of `$FFFF` plus `Y = 2` lands
/// in the *next* bank, not back at the bottom of this one. A core that wraps within the bank reads
/// something 64 KB away from what the program meant, which for a table crossing a bank boundary is
/// the difference between data and whatever precedes it.
///
/// Both candidate addresses are seeded with different values, so the wrong answer is a specific
/// wrong byte rather than whatever memory happened to hold. WRAM is used for both because it is the
/// only region where two consecutive banks are readable and writable.
fn a2_07() -> Test {
    let mut a = Asm::new();
    a.c("$7F:0001 is where the bank carry must land; $7E:0001 is where a bank-wrapping core");
    a.c("would look instead.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$5A");
    a.l("sta f:$7F0001");
    a.l("lda #$99");
    a.l("sta f:$7E0001");
    a.c("Direct page $10/$11 holds the pointer $FFFF; DBR = $7E; Y = 2.");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("lda #$FFFF");
    a.l("sta f:$7E0010");
    a.l("ldy #$0002");
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l("pha");
    a.l("plb");
    a.l("lda ($10),y");
    a.c("Restore the data bank before anything else uses absolute addressing.");
    a.l("phk");
    a.l("plb");
    a.assert_a8(
        0x5A,
        "(dp),Y did not carry into the next bank — $99 means it wrapped inside the data bank",
    );
    a.finish(
        "A2.07",
        'A',
        "(dp),Y carries bank",
        Provenance::Documented("WDC datasheet; superfamicom.org addressing notes"),
        Kind::Scored,
        None,
    )
}

/// `PEI (dp)` reads its pointer without page-wrapping, even at `E=1` with `DL = $00`.
///
/// The direct-page page-wrap rule — a 16-bit read at `$FF` fetching its high byte from `$00` of the
/// same page — applies to the instructions the 6502 had. `PEI` is not one of them, so it reads
/// `$00FF` and `$0100`, straight through the page boundary. It is the same old-versus-new split as
/// `PLD` against `PLY` (`A3.03`), on the fetch side rather than the stack side.
///
/// `$0000` is seeded with a third value, so a page-wrapping core pushes a *specific* wrong word
/// rather than something incidental. The pushed bytes are read out of WRAM rather than pulled off
/// the stack, which keeps the stack pointer's own emulation-mode behaviour out of an assertion that
/// is not about it.
fn a2_10() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$34");
    a.l("sta f:$7E00FF"); // the low byte of the pointer
    a.l("lda #$12");
    a.l("sta f:$7E0100"); // its high byte, if the read does not wrap
    a.l("lda #$99");
    a.l("sta f:$7E0000"); // and where a wrapping read would take it from
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("lda #$01FF");
    a.l("tcs");
    a.enter_emulation();
    a.l("pei ($FF)");
    a.enter_native();
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda f:$7E01FF");
    a.assert_a8(
        0x12,
        "PEI page-wrapped its pointer fetch — $99 is the byte at $0000, which only an old-style \
         wrap would read",
    );
    a.l("lda f:$7E01FE");
    a.assert_a8(0x34, "PEI did not push the pointer's low byte");
    a.finish(
        "A2.10",
        'A',
        "PEI does not page-wrap",
        Provenance::Documented("WDC datasheet; superfamicom.org addressing notes"),
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

/// `JSR (a,X)` escapes page 1: the second pushed byte lands below `$0100`.
///
/// The companion to `A3.07`, and not a duplicate of it: `JSL` pushes three bytes and `JSR (a,X)`
/// pushes two, so they cross the page-1 floor from different starting alignments and through
/// different opcodes. A core that special-cases the escape per instruction — which is exactly how
/// this gets implemented — can get one right and the other wrong.
///
/// From `S = $0100` the first push lands at `$0100`, the last byte of the stack page, and the
/// second escapes to `$00FF`. A core applying the 6502's page-1 confinement uniformly wraps that
/// second push to `$01FF` instead.
///
/// The canary at `$01FF` is the discriminator: untouched if the push escaped, overwritten if it
/// wrapped. As in `A3.07`, what landed at `$00FF` is deliberately not asserted — it is half of a
/// return address, so its value would pin where in the ROM this test is assembled.
///
/// **The subroutine does not `RTS`,** for the reason `A3.07` records the hard way: after an
/// escaping push `S` is below `$0100`, emulation mode forces the stack's high byte back to `$01`,
/// and there is no return address left to pull. It rebuilds the stack and jumps back instead.
///
/// The pointer is read from `$00:0210` and no claim is made about *which bank* it comes from —
/// banks `$00-$3F` alias the same WRAM below `$2000`, so a bank claim there would be unfalsifiable
/// (see the plan's `A4.06`/`A4.08` entry). This test is about the pushes.
fn a3_08() -> Test {
    let mut a = Asm::new();
    a.c("The subroutine, jumped over. It leaves emulation, rebuilds the stack, and rejoins.");
    a.l("bra @body");
    a.label("sub");
    a.l("clc");
    a.l("xce               ; -> native");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("lda f:V_SAVED_S");
    a.l("tcs");
    a.l("jmp @after");
    a.label("body");
    a.c("Canary at the top of the stack page: a wrapped second push lands exactly here.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$EE");
    a.l("sta f:$7E01FF");
    a.c("Seed $00FF with the COMPLEMENT of the byte the push must deliver. Left as stale WRAM it");
    a.c("could already hold that byte, and the positive check below would then pass for a core");
    a.c("that never made the second push at all -- an outcome dependent on test order.");
    a.l("lda #(<(@after-1)) ^ $FF");
    a.l("sta f:$7E00FF");
    a.c("Pointer for the indirect jump, in low WRAM (mirrored at $00:0210).");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@sub)");
    a.l("sta f:$7E0210");
    a.c("E=1, S=$0100: the first push lands at $0100, the second must escape to $00FF.");
    a.l("lda #$0100");
    a.l("tcs");
    a.enter_emulation();
    a.l("ldx #$00");
    a.l("jsr ($0210,x)");
    a.label("after");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda f:$7E01FF");
    a.assert_a8(
        0xEE,
        "JSR (a,X) wrapped its second push into page 1 and clobbered $01FF, \
         instead of escaping to $00FF",
    );
    a.c("The canary alone is vacuous: a core that pushed one byte, or none, would also leave");
    a.c("$01FF untouched. So check the escaped byte positively — $00FF must hold the LOW half of");
    a.c("the pushed return address, which JSR defines as (@after - 1).");
    a.c("");
    a.c("S cannot serve as the control here: emulation mode forces the stack's high byte back to");
    a.c("$01 at the next instruction boundary, so S reads $01FE whether the push escaped or not.");
    a.c("");
    a.c("The expected byte is computed by the assembler rather than written out, so this pins the");
    a.c("relationship and not the ROM layout — which is why the value itself is still not named.");
    a.l("lda f:$7E00FF");
    a.l("sec");
    a.l("sbc #<(@after-1)");
    a.assert_a8(
        0x00,
        "the second return-address byte did not land at $00FF — JSR (a,X) pushed fewer \
         bytes than it should, or put them elsewhere",
    );
    a.finish(
        "A3.08",
        'A',
        "JSR (a,X) escapes page 1",
        Provenance::Documented("WDC datasheet; superfamicom.org escape list"),
        Kind::Scored,
        None,
    )
}

/// `(d,S),Y` escapes page 1 for its pointer read, and bank-carries for its data read.
///
/// Two independent claims, and the test seeds a **distinct wrong answer for each** so a failure
/// says which one broke:
///
/// * **Escape.** The pointer address is `S + d` as a full 16-bit sum, with no page-1 masking —
///   ares' `readStack` is `read(n16(S.w + address))`. From `S = $01FE` and `d = $04` that is
///   `$0202`. A core applying the 6502's page-1 confinement computes `($FE + $04) & $FF = $02` and
///   reads `$0102` instead.
/// * **Bank carry.** The data address is `DBR:pointer + Y` as a 24-bit sum. With the pointer at
///   `$FFFF`, `DBR = $7E` and `Y = 2`, that is `$7F:0001` — a core masking to 16 bits reads
///   `$7E:0001`.
///
/// The three outcomes are seeded `$5A` (both right), `$99` (escaped but masked the bank carry) and
/// `$77` (confined the pointer read), so the assertion distinguishes correct behaviour from each
/// broken alternative rather than from "not `$5A`".
///
/// `DBR` is set before `S` is moved, because loading it costs a `PHA`/`PLB` pair and doing that
/// with the stack pointer parked one byte below the page boundary would push through the very
/// boundary under test.
fn a3_06() -> Test {
    let mut a = Asm::new();
    a.c("Seed all three candidate results distinctly.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$5A");
    a.l("sta f:$7F0001     ; escaped pointer + bank carry: the documented answer");
    a.l("lda #$99");
    a.l("sta f:$7E0001     ; escaped pointer, but the bank carry masked away");
    a.l("lda #$77");
    a.l("sta f:$7E0302     ; pointer read confined to page 1, then + Y");
    a.c("Two pointers: the one at the escaped address, and the one a confined core would find.");
    a.l("rep #$30");
    a.l("lda #$FFFF");
    a.l("sta f:$7E0202     ; at S+d = $01FE+$04, escaping page 1");
    a.l("lda #$0300");
    a.l("sta f:$7E0102     ; at ($FE+$04) & $FF, inside page 1");
    a.c("DBR first — PHA/PLB uses the stack, so do it before S is parked at the boundary.");
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l("pha");
    a.l("plb");
    a.l("rep #$30");
    a.l("lda #$01FE");
    a.l("tcs");
    a.enter_emulation();
    a.l("ldy #$02");
    a.l("lda ($04,s),y");
    a.enter_native();
    a.assert_a8(
        0x5A,
        "(d,S),Y misread: $77 = pointer confined to page 1, $99 = bank carry masked to 16 bits",
    );
    a.finish(
        "A3.06",
        'A',
        "(d,S),Y escape + carry",
        Provenance::Documented("WDC datasheet; superfamicom.org escape list"),
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

/// The `+1 m` penalty: a memory operation costs one extra access when the accumulator is 16-bit.
///
/// `REP #$20` clears `m`, and every load that moves the accumulator then transfers two bytes
/// instead of one. On this machine that extra access is 8 master clocks, so sixteen of them is 32
/// dots and comfortably resolvable.
///
/// # Why this is measurable when `A5.20` was not
///
/// Both spans stay **inside a single scanline**, so this uses the narrow instrument and never
/// reaches the long dots at `H >= 323` or the line-length approximation that made the block-move
/// measurement alignment-dependent (`T-06-A`). The differential cancels the instrument's own
/// overhead, and the only thing changing between the two measurements is the `m` bit — same
/// opcodes, same address, same alignment.
///
/// # Slot choice is not arbitrary
///
/// The recording slots are in the 116+ range because **the opcode sweep owns slots 8 through 75**
/// (`sweep.rs`: `slot_base = 8 + index * 2`, 34 tests). The first draft of this test used 20-25 and
/// its raw numbers came back as the sweep's baseline spans — a passing assertion sitting next to a
/// recorded value that contradicted it. The channel has no allocator, so a slot has to be checked
/// against every writer, not just against the `record` calls that happen to use literals.
fn a5_09() -> Test {
    let mut a = Asm::new();
    a.c("Same instruction, same address, twice: once 8-bit, once 16-bit.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("--- m=1: LDA abs moves one byte ---");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(16, &["lda $0000"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0096");
    a.record(116, "16x LDA abs, m=1");
    a.c("--- m=0: the same LDA moves two ---");
    a.l("rep #$20");
    a.measure_begin();
    a.repeat(16, &["lda $0000"]);
    a.measure_end();
    a.measure_result();
    a.record(117, "16x LDA abs, m=0");
    a.l("sec");
    a.l("sbc f:$7E0096");
    a.record(118, "16x (m=0 - m=1), expect 32");
    a.assert_a16_range(
        32 - TOL,
        32 + TOL,
        "LDA abs did not cost one extra 8-clock access with m=0",
    );
    a.finish(
        "A5.09",
        'A',
        "+1 m width penalty",
        Provenance::Documented(
            "WDC/GTE/VLSI instruction-operation tables; docs/accuracysnes-timing-oracle.md",
        ),
        Kind::Scored,
        None,
    )
}

/// The `+1 x` penalty: an index-register operation costs one extra access when `x` is clear.
///
/// The `A5.09` argument with the other width bit. `REP #$10` widens `X`/`Y`, and `LDX abs` then
/// loads two bytes rather than one.
///
/// It is a separate assertion rather than a restatement: `m` and `x` are independent bits and the
/// penalty applies per operand class, so a core deriving one width from the other — or applying the
/// accumulator's penalty to index operations — passes `A5.09` and fails here. `m` is held 8-bit
/// across both spans for exactly that reason, leaving the `x` bit as the only difference.
fn a5_10() -> Test {
    let mut a = Asm::new();
    a.c("m is held 8-bit throughout, so only the x bit differs between the two spans.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("--- x=1: LDX abs moves one byte ---");
    a.l("sep #$30");
    a.measure_begin();
    a.repeat(16, &["ldx $0000"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0096");
    a.record(119, "16x LDX abs, x=1");
    a.c("--- x=0: the same LDX moves two ---");
    a.l("sep #$20");
    a.l("rep #$10");
    a.measure_begin();
    a.repeat(16, &["ldx $0000"]);
    a.measure_end();
    a.measure_result();
    a.record(120, "16x LDX abs, x=0");
    a.l("sec");
    a.l("sbc f:$7E0096");
    a.record(121, "16x (x=0 - x=1), expect 32");
    a.assert_a16_range(
        32 - TOL,
        32 + TOL,
        "LDX abs did not cost one extra 8-clock access with x=0",
    );
    a.finish(
        "A5.10",
        'A',
        "+1 x width penalty",
        Provenance::Documented(
            "WDC/GTE/VLSI instruction-operation tables; docs/accuracysnes-timing-oracle.md",
        ),
        Kind::Scored,
        None,
    )
}

/// `WAI` wakes on a **masked** IRQ and resumes in line rather than vectoring.
///
/// `WAI` stops the CPU until an interrupt line asserts. The `I` flag does not gate the *wake* —
/// only the *vector*. So `SEI; WAI` is a legitimate "sync to the interrupt line" primitive: the
/// CPU resumes at the instruction after `WAI` and no handler runs. A core that gates the wake on
/// `I` hangs forever here, and a core that vectors anyway runs the handler.
///
/// # Both halves are pinned
///
/// Asserting only "the handler did not run" would be vacuous — a `WAI` implemented as a no-op also
/// never runs the handler. So the test checks two things:
///
/// * `$4211` bit 7 is **set** on resumption, proving an IRQ actually fired and therefore that
///   `WAI` waited for it rather than falling straight through;
/// * the handler's flag is still clear, proving it woke without vectoring.
///
/// `$4211` is read once and stashed, because reading it clears the latch.
///
/// # How the two failure modes present
///
/// Verified by injecting each bug rather than assumed. A core that **vectors anyway** fails
/// cleanly with this test's second code. A core that **gates the wake on `I`** does not fail this
/// test at all — it hangs, and the harness reports *"battery did not reach its completion sentinel
/// within 600 frames"*. That is still a hard failure, but it is a battery-level one with no
/// per-test verdict, because a CPU that never resumes cannot write a result byte. `A3.07` records
/// the same shape from the other direction, where a wrong `RTL` hung all three cores.
fn a6_11() -> Test {
    let mut a = Asm::new();
    a.c("The handler must NOT run. It sets a flag so its running is observable.");
    a.l("bra @body");
    a.label("handler");
    a.l("sep #$20");
    a.l(".a8");
    a.l("lda #$01");
    a.l("sta f:$7E0142");
    a.l("lda $4211        ; acknowledge so the line drops");
    a.l("rti");
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("rep #$20");
    a.l("lda #@handler");
    a.l("sta a:V_IRQ_VEC");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0142     ; handler-ran flag, clear");
    a.l("lda #200");
    a.l("sta $4207");
    a.l("stz $4208         ; HTIME = 200");
    a.l("lda $4211         ; clear any stale latch");
    a.l("sei               ; I = 1: the IRQ is masked, so it must not vector");
    a.l("lda #$10");
    a.l("sta $4200         ; H-IRQ enabled");
    a.l("wai");
    a.c("--- resumed in line, or we never got here ---");
    a.l("lda $4211");
    a.l("sta f:$7E0144     ; stash: reading $4211 clears the latch");
    a.l("stz $4200         ; disarm before asserting; a failure exits immediately");
    a.l("lda f:$7E0144");
    a.l("and #$80");
    a.assert_a8(
        0x80,
        "WAI returned with no IRQ pending — it fell through instead of waiting",
    );
    a.l("lda f:$7E0142");
    a.assert_a8(
        0x00,
        "WAI with I=1 vectored to the handler instead of resuming in line",
    );
    a.finish(
        "A6.11",
        'A',
        "WAI wakes, no vector",
        Provenance::Documented(
            "WDC datasheet: WAI wakes on the interrupt line; I gates the vector",
        ),
        Kind::Scored,
        None,
    )
}

/// How long `WAI` takes to resume once the interrupt line asserts — a golden vector, never scored.
///
/// The dossier states the wake latency as **1 cycle**. That is 6 master clocks, or 1.5 dots — below
/// what this cartridge can resolve. The H counter is the only clock a cart can read, the latch
/// sequence itself costs several cycles, and `T-06-A` establishes that dot lengths are not even
/// uniform in the reference cores. A scored assertion at that precision would be measuring the
/// instrument.
///
/// So the observation is recorded instead: arm an H-IRQ at a known `HTIME`, `SEI; WAI`, and latch H
/// as the first thing after resumption. The reported variant is `(latched H - HTIME)` in 4-dot
/// buckets — coarse enough to be stable across alignment, fine enough that a core waking a whole
/// scanline late announces itself. `B4.14` does the same for interrupt *dispatch* latency and is
/// the precedent for reporting rather than asserting here.
///
/// `WAI` is used with `I = 1` deliberately, so this measures the wake alone and not the wake plus a
/// vector fetch — `A6.11` establishes that the masked case resumes in line without vectoring.
fn a6_12() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sei               ; masked: WAI must wake without vectoring (A6.11)");
    a.l("lda #200");
    a.l("sta $4207");
    a.l("stz $4208         ; HTIME = 200");
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$10");
    a.l("sta $4200         ; H-IRQ enabled");
    a.l("wai");
    a.c("Latch H as the very first thing after resuming — every instruction here is latency.");
    a.l("lda $213F         ; reset the counter read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213C");
    a.l("xba");
    a.l("lda $213C");
    a.l("and #$01");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
    a.record(122, "H latched immediately after WAI resumed");
    a.l("sec");
    a.l("sbc #200          ; minus HTIME: the wake latency in dots");
    a.record(123, "WAI wake latency (dots)");
    a.c("Disarm before reporting; a failing path would exit immediately.");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4211");
    a.c("Report in 4-dot buckets: 1 cycle is 1.5 dots, so the exact value is not resolvable and");
    a.c("a variant claiming otherwise would be measuring the latch sequence, not the wake.");
    a.l("rep #$20");
    a.l("lda f:$7EE2F6     ; the latency recorded above");
    a.l("lsr a");
    a.l("lsr a");
    a.l("sep #$20");
    a.l("and #$3F");
    a.l("asl a");
    a.l("ora #$01");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "A6.12",
        'A',
        "WAI wake lat (golden)",
        Provenance::Contested(
            "the stated 1-cycle latency is 1.5 dots, below what a cart can resolve; \
             recorded in buckets rather than asserted",
        ),
        Kind::Golden,
        None,
    )
}

/// `JMP (a,X)` takes its pointer from the **program bank**, wrapping inside it rather than carrying.
///
/// `$FFFE + X` must stay in the current program bank. A core computing the pointer address as a
/// flat 24-bit sum reads it from the next bank instead. The errata's worked example is `PBR = $05`,
/// `X = $04`, `JMP ($FFFE,X)` reaching `$05:0002`.
///
/// # Why an earlier version of this asserted nothing
///
/// `A4.06` and `A4.08` tried this with the pointer in low WRAM and were **withdrawn as vacuous**:
/// banks `$00-$3F` all alias the same 8 KiB below `$2000`, so `$00:1000` and `$01:1000` are
/// literally the same bytes and a carrying core read the identical pointer. Cross-validation could
/// not catch it either — a test that cannot fail passes on every implementation.
///
/// The discriminating fixture already existed and is what `lorom.cfg` builds a 128 KiB image for:
/// a per-bank signature block at `$xx:8000`, whose bytes 8-9 now hold the address of `bankprobe_0`
/// in bank `$00` and `bankprobe_1` in bank `$01`. Those are different ROM, not mirrors.
///
/// `ldx #$800A` makes `$FFFE + X` wrap to `$8008` in the program bank, or carry to `$01:8008`
/// otherwise. Each stub records its own identity and returns through `V_BANKPROBE_RET`, so **both
/// outcomes come back through the same path** — the property the withdrawn pair lacked, and the
/// reason their wrong answer would have been a crash rather than a verdict.
fn a4_11() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Install the continuation both stubs return through, as a 24-bit pointer.");
    a.l("lda #.LOWORD(@landed)");
    a.l("sta a:V_BANKPROBE_RET");
    a.l("sep #$20");
    a.l("lda #^@landed");
    a.l("sta a:V_BANKPROBE_RET+2");
    a.c("Poison the result so 'neither stub ran' is distinguishable from either answer.");
    a.l("lda #$FF");
    a.l("sta a:V_BANKPROBE");
    a.l("rep #$30");
    a.l("ldx #$800A        ; $FFFE + $800A = $1_8008, wrapping to $8008 in the program bank");
    a.l("jmp ($FFFE,x)");
    a.label("landed");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda a:V_BANKPROBE");
    a.assert_a8(
        0x00,
        "JMP (a,X) did not take its pointer from the program bank: $01 = carried into bank $01, \
         $FF = neither stub ran",
    );
    a.finish(
        "A4.11",
        'A',
        "JMP (a,X) ptr bank",
        Provenance::Documented("SNESdev Errata, 65C816 section (worked example PBR=$05)"),
        Kind::Scored,
        None,
    )
}

/// `JSR (a,X)` takes its pointer from the program bank too, exactly as `JMP (a,X)` does.
///
/// The companion to `A4.11`, and not a restatement of it: `JSR` and `JMP` are separate opcodes with
/// separate address-formation paths in most cores, and the push makes `JSR` the more intricate of
/// the pair — so a bank rule fixed in one is routinely missed in the other.
///
/// Uses the same bank-probe fixture: `ldx #$800A` puts `$FFFE + X` at `$8008`, which holds
/// `bankprobe_0` in bank `$00` and `bankprobe_1` in bank `$01`, and each stub records which one ran
/// before returning through `V_BANKPROBE_RET`.
///
/// **The pushed return address is abandoned on purpose.** The stubs exit with `jml`, not `RTS`, so
/// the stack is left unbalanced — `test_restore` re-establishes `S` from `V_SAVED_S`, which is what
/// makes a test free to corrupt it. `A3.08` records why returning normally is not an option after a
/// push that escapes its page, and the same reasoning applies to any probe reached this way.
fn a4_12() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Install the continuation both stubs return through, as a 24-bit pointer.");
    a.l("lda #.LOWORD(@landed)");
    a.l("sta a:V_BANKPROBE_RET");
    a.l("sep #$20");
    a.l("lda #^@landed");
    a.l("sta a:V_BANKPROBE_RET+2");
    a.c("Poison the result so 'neither stub ran' stays distinguishable from either answer.");
    a.l("lda #$FF");
    a.l("sta a:V_BANKPROBE");
    a.l("rep #$30");
    a.l("ldx #$800A        ; $FFFE + $800A = $1_8008, wrapping to $8008 in the program bank");
    a.l("jsr ($FFFE,x)");
    a.label("landed");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda a:V_BANKPROBE");
    a.assert_a8(
        0x00,
        "JSR (a,X) did not take its pointer from the program bank: $01 = carried into bank $01, \
         $FF = neither stub ran",
    );
    a.finish(
        "A4.12",
        'A',
        "JSR (a,X) ptr bank",
        Provenance::Documented("SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// An interrupt handler runs with `PBR = $00`, whatever bank was executing.
///
/// The vector table lives at `$00:FFxx` and the fetched vector is a 16-bit address, so taking an
/// interrupt forces the program bank to `$00`. A core that leaves `PBR` alone jumps to the right
/// offset in the *wrong* bank — and in ordinary code, where everything already runs in bank `$00`,
/// that is completely invisible.
///
/// # Which is why the interrupted code has to run somewhere else
///
/// Every test body in this group executes in bank `$00`, so an interrupt taken from one would
/// report `PBR = $00` whether or not the core forces it. The test would pass on a broken core: the
/// vacuity that withdrew `A4.06`.
///
/// So the interrupted code is a ten-byte stub assembled **into WRAM** and jumped to with `JML`, the
/// same technique `A4.09` uses to reach a bank boundary. It spins on a flag the handler sets, so
/// `PBR` is `$7E` at the moment the IRQ arrives and a core that does not force `$00` reports `$7E`.
///
/// ```text
///   $7E:3000  AF 92 01 7E   LDA $7E0192   ; the handler's rendezvous flag
///   $7E:3004  F0 FA         BEQ -6        ; spin until it is set
///   $7E:3006  5C .. .. 00   JML @after    ; back to bank $00
/// ```
///
/// # How the failure presents, which is not always a clean code
///
/// Verified by injecting the bug — deleting `self.regs.pbr = 0` from the hardware-interrupt path
/// in `rustysnes-cpu`. A core that does not force the bank does not merely report `$7E`: it
/// **never reaches this handler at all**, because it jumps to the handler's 16-bit offset inside
/// bank `$7E`, which is WRAM. What runs there is arbitrary, so the observed verdict was a spurious
/// `variant 1` rather than this test's failure code — which would not have dropped the pass rate.
///
/// That is a property of the defect, not a fixable weakness of the test: once a core mis-vectors,
/// no code this test placed anywhere can be relied on to run. `A6.11` records the same shape from
/// the other direction, where gating `WAI`'s wake hangs the battery instead of failing a test. The
/// honest reading is that a broken core here shows up as a battery *anomaly* — wrong verdict, hang
/// or clean failure depending on what the stray execution touches — rather than as a tidy red row,
/// and the `$FF` poison exists so that "the handler never ran" is at least distinguishable when
/// the verdict does survive.
fn a6_13() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("handler");
    a.l("rep #$30");
    a.l("pha");
    a.l("sep #$20");
    a.l(".a8");
    a.c("PHK pushes the CURRENT program bank, which is the whole question.");
    a.l("phk");
    a.l("pla");
    a.l("sta f:$7E0190");
    a.l("lda $4211         ; acknowledge");
    a.l("lda #$01");
    a.l("sta f:$7E0192     ; release the spin loop");
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
    a.l("sei");
    a.l("rep #$20");
    a.l("lda #@handler");
    a.l("sta a:V_IRQ_VEC");
    a.c("Poison the result: $FF distinguishes 'the handler never ran' from either bank.");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("sta f:$7E0190");
    a.l("lda #$00");
    a.l("sta f:$7E0192");
    a.c("Assemble the spin stub into bank $7E. It must live outside bank $00 for this test to");
    a.c("mean anything -- see the note above.");
    a.l("lda #$AF");
    a.l("sta f:$7E3000     ; LDA long");
    a.l("lda #$92");
    a.l("sta f:$7E3001");
    a.l("lda #$01");
    a.l("sta f:$7E3002");
    a.l("lda #$7E");
    a.l("sta f:$7E3003     ; ...$7E0192, the rendezvous flag");
    a.l("lda #$F0");
    a.l("sta f:$7E3004     ; BEQ");
    a.l("lda #$FA");
    a.l("sta f:$7E3005     ; -6: back to the LDA");
    a.l("lda #$5C");
    a.l("sta f:$7E3006     ; JML");
    a.l("rep #$20");
    a.l("lda #.LOWORD(@after)");
    a.l("sta f:$7E3007");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E3009     ; ...back into bank $00");
    a.c("Arm an H-IRQ, unmask, and run the stub. The interrupt lands with PBR = $7E.");
    a.l("lda #200");
    a.l("sta $4207");
    a.l("stz $4208         ; HTIME = 200");
    a.l("lda $4211         ; clear any stale latch");
    a.l("lda #$10");
    a.l("sta $4200         ; H-IRQ enabled");
    a.l("cli");
    a.l("jml $7E3000");
    a.label("after");
    a.l("sep #$20");
    a.l("sei");
    a.l("stz $4200");
    a.l("lda $4211");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda f:$7E0190");
    a.assert_a8(
        0x00,
        "the interrupt handler did not run with PBR = $00 — $7E means the program bank was left \
         as the interrupted code's, $FF that the handler never ran at all",
    );
    a.finish(
        "A6.13",
        'A',
        "IRQ handler PBR = $00",
        Provenance::Documented(
            "WDC datasheet: the vector is 16-bit, so the handler runs in bank 0",
        ),
        Kind::Scored,
        None,
    )
}

/// `RTI` pulls a number of bytes that matches the mode: native takes `PBR`, emulation does not.
///
/// A native interrupt pushes `PBR`, `PCH`, `PCL`, `P` — four bytes — and `RTI` pulls all four. An
/// emulation-mode interrupt pushes three and `RTI` pulls three. A core that uses one count for both
/// modes leaves the stack pointer off by one every time an interrupt returns, which corrupts
/// whatever the interrupted code had below it.
///
/// # Why `S` is the observable, and not "did we return to the right place"
///
/// Pulling one byte too many still returns to the *correct* `PC`: `P`, `PCL` and `PCH` come off
/// first and only the extra `PBR` byte is spurious. So a returned-to-the-right-place check would
/// pass on a broken core — vacuous in the way that withdrew `A4.06`. What the extra pull actually
/// disturbs is `S`, which ends one higher than it started.
///
/// So the test brackets the whole interrupt with a stack-pointer reading: `S` before the `BRK` and
/// `S` after the `RTI` must be identical. That is also why this does not need the wrong answer to
/// crash — unlike `A6.09`, where a mis-vectoring core executes arbitrary memory, here both a
/// correct and a broken core return normally and differ only in a register.
///
/// Only the low byte of `S` is compared: emulation mode forces `m = 1`, so a 16-bit store is not
/// available there, and the high byte is pinned to `$01` by the mode anyway. An off-by-one is
/// entirely visible in the low byte.
///
/// `BRK`'s signature byte is skipped by the push of `PC + 2`, so the `RTI` lands after it — the
/// same mechanism `A6.07` asserts directly.
fn a6_14() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("brkh");
    a.c("The handler does nothing but return: this row is about RTI's pull count, and any work");
    a.c("here would need its own register discipline to stay out of the way of that.");
    a.l("rti");
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("rep #$20");
    a.l("lda #@brkh");
    a.l("sta a:V_BRK_VEC");
    a.c("Park the stack somewhere unambiguous inside page 1, then drop to emulation mode.");
    a.l("lda #$01F0");
    a.l("tcs");
    a.enter_emulation();
    a.l("tsc");
    a.l("sta f:$7E01A0     ; S before the interrupt (low byte; SH is pinned to $01 here)");
    a.l("brk");
    a.l(".byte $EA         ; BRK's signature byte, skipped by the pushed PC+2");
    a.l("tsc");
    a.l("sta f:$7E01A2     ; S after RTI returned");
    a.enter_native();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("Equal means RTI pulled exactly what the interrupt pushed. One higher means it also took");
    a.c("a PBR byte that emulation mode never pushed.");
    a.l("lda f:$7E01A0");
    a.l("cmp f:$7E01A2");
    a.fail_if_ne(
        "emulation-mode RTI did not restore S — it pulled a different number of bytes than the \
         interrupt pushed",
    );
    a.finish(
        "A6.14",
        'A',
        "RTI pull matches mode",
        Provenance::Documented("WDC datasheet: RTI pulls PBR in native mode only"),
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
    a.l("jml test_restore");
    a.l(":");
    a.l("lda #$05          ; variant 2 = V observed CLEAR ((2<<1)|1)");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
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

/// `CLC; XCE` entered native with carry clear is a complete no-op.
///
/// `XCE` swaps carry with `E`. Entered native (`E = 0`) with carry clear, both bits end clear, so
/// nothing changes at all — not `E`, not carry, not the registers. That is the point. A core that treats any `XCE` as a mode
/// *transition* and re-initialises on it (resetting register widths, truncating the high bytes of
/// `A`/`X`/`Y`, forcing the stack to page 1) passes every test that only checks the flag and
/// corrupts the machine here.
fn a1_08() -> Test {
    let mut a = Asm::new();
    a.c("Plant distinguishable 16-bit values in all three registers, then execute the no-op XCE.");
    a.l("rep #$30");
    a.l("lda #$1234");
    a.l("ldx #$5678");
    a.l("ldy #$9ABC");
    a.l("clc");
    a.l("xce               ; C=0 in, E=0 out: nothing should change");
    a.c("Stash A before the status check, because reading P costs a PLA into the accumulator. A");
    a.c("core that truncated A to its low byte would otherwise be invisible: the PLA overwrites");
    a.c("the evidence before anything compares it.");
    a.l("sta f:$7E0094");
    a.c("Widths next: if m or x were disturbed the comparisons below would be 8-bit and could");
    a.c("pass on the low byte alone.");
    a.l("php");
    a.l("sep #$20");
    a.l("pla");
    a.c("Keep bit 0: the carry is half of what XCE exchanges, so discarding it would let a core");
    a.c("that leaves C set pass a test whose entire claim is that nothing changed.");
    a.l("and #$31");
    a.assert_a8(
        0x00,
        "CLC/XCE in native mode disturbed the m/x width bits or the carry",
    );
    a.l("rep #$30");
    a.l("lda f:$7E0094");
    a.assert_a16(0x1234, "CLC/XCE in native mode disturbed A");
    a.assert_x16(0x5678, "CLC/XCE in native mode disturbed X");
    a.assert_y16(0x9ABC, "CLC/XCE in native mode disturbed Y");
    a.finish(
        "A1.08",
        'A',
        "CLC/XCE is a no-op",
        Provenance::Documented("WDC datasheet: XCE exchanges carry and E, nothing else"),
        Kind::Scored,
        None,
    )
}

/// `REP #$30` in emulation mode cannot clear `m` or `x`.
///
/// Hardware forces `m = x = 1` for as long as `E = 1`, and `REP` does not override it. The
/// consequence that makes this observable is the one the DSL's `exit_emulation` warns about:
/// leaving emulation widens nothing, because `m`/`x` were held at 1 the whole time and stay there.
///
/// So the test reads the width bits *after* returning to native mode. Doing it inside emulation
/// would be much weaker — `P` bits 4 and 5 are `B` and unused there, not `x` and `m`, so the
/// obvious in-emulation check reads a register that does not carry the answer.
fn a1_09() -> Test {
    let mut a = Asm::new();
    a.c("Enter emulation, attempt to widen both registers, then leave.");
    a.l("rep #$30");
    a.enter_emulation();
    a.l("rep #$30           ; must be ignored: E=1 pins m=x=1");
    a.enter_native();
    a.c("Native again. If REP had taken effect, m and x would now be 0.");
    a.l("php");
    a.l("pla");
    a.l("and #$30");
    a.assert_a8(0x30, "REP #$30 cleared m/x while E=1");
    a.finish(
        "A1.09",
        'A',
        "REP cannot widen in E=1",
        Provenance::Documented("WDC datasheet; SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// `MVN` wraps `X` inside the source bank and `Y` inside the destination bank, independently.
///
/// The index registers are 16-bit offsets into their own banks; neither carries into the next one.
/// Starting `X` at `$FFFF` and moving two bytes therefore reads `$7E:FFFF` and then `$7E:0000` —
/// wrapping to the bottom of the *same* bank, not advancing into `$7F`.
///
/// `Y` is started well away from its own wrap so the two are separable: if the assertion on `X`
/// and the assertion on `Y` could both be explained by one shared counter, the test would not show
/// that they wrap independently.
fn a8_05() -> Test {
    let mut a = Asm::new();
    a.c("X starts at the top of the source bank, Y in the middle of the destination bank.");
    a.l("rep #$30");
    a.l("ldx #$FFFF");
    a.l("ldy #$1000");
    a.l("lda #$0001        ; count-1: two bytes moved");
    a.l("mvn #$7E,#$7E    ; literal bank numbers; `mvn $7E,$7E` would mean bank $00");
    a.l("phk");
    a.l("plb               ; MVN left DBR = $7E");
    a.assert_x16(0x0001, "X did not wrap inside the source bank");
    a.assert_y16(0x1002, "Y did not advance independently of X");
    a.finish(
        "A8.05",
        'A',
        "MVN index wrap",
        Provenance::Documented("WDC datasheet: the block-move indices are bank offsets"),
        Kind::Scored,
        None,
    )
}

/// The `m`/`x` bits resist `PLP` in emulation mode too — the third of three paths.
///
/// `A1.09` covers the `REP` path and the mode-entry path is covered by every emulation test in the
/// group. This is the one left: pulling a processor status byte whose `m`/`x` bits are clear.
///
/// It is worth its own test because the mechanism is different. `REP` is an instruction the core
/// can special-case; `PLP` writes `P` wholesale from memory, so a core that implements the
/// emulation-mode pin as a mask applied in `REP`/`SEP` rather than as a property of `P` itself will
/// pass `A1.09` and fail here. As with `A1.09`, the bits are read after returning to native mode,
/// because in emulation `P` bits 4 and 5 are `B` and unused.
///
/// The pulled byte keeps `I` set. Clearing it would enable interrupts inside a test that is not
/// about interrupts.
fn a1_10() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.enter_emulation();
    a.c("Pull a P with m/x clear (bits 5 and 4) but I still set.");
    a.l("lda #$04");
    a.l("pha");
    a.l("plp               ; must not widen anything: E=1 pins m=x=1");
    a.enter_native();
    a.l("php");
    a.l("pla");
    a.l("and #$30");
    a.assert_a8(0x30, "PLP cleared m/x while E=1");
    a.finish(
        "A1.10",
        'A',
        "PLP cannot widen in E=1",
        Provenance::Documented("WDC datasheet: m/x are forced while E=1, whatever writes P"),
        Kind::Scored,
        None,
    )
}

/// `JML [a]` takes a full 24-bit destination, including the bank byte.
///
/// The third byte of the pointer is the whole point of the long-indirect form, and a core that
/// loads only the low word lands at the right offset in the *wrong* bank — which, in code that
/// happens to be bank-agnostic, then runs correctly and hides the bug.
///
/// # Making the bank byte observable
///
/// The destination bank is `$80`, which in this LoROM image mirrors bank `$00`, so both a correct
/// core and a bank-ignoring one execute the same instructions and neither crashes. What separates
/// them is `PBR` afterwards: pushed with `PHK`, it reads `$80` only if the bank byte was actually
/// taken. Landing somewhere that behaves identically and then asking where we are is what makes
/// this a clean assertion rather than a crash test.
fn a4_07() -> Test {
    let mut a = Asm::new();
    a.c("Build a 24-bit pointer at $00:1000 (low WRAM mirror): offset = @landed, bank = $80.");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@landed)");
    a.l("sta f:$7E1000");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta f:$7E1002");
    a.l("jml [$1000]");
    a.label("landed");
    a.c("PHK reports the bank actually being executed from.");
    a.l("phk");
    a.l("pla");
    a.assert_a8(0x80, "JML [a] ignored the pointer's bank byte");
    a.finish(
        "A4.07",
        'A',
        "JML [a] 24-bit dest",
        Provenance::Documented("WDC datasheet; SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// `[dp],Y` carries out of the pointer's bank.
///
/// The long-indirect indexed form takes its bank from the third byte of the direct-page pointer,
/// not from `DBR`, and adding `Y` may carry past the end of that bank into the next one. A core
/// that masks the sum to 16 bits reads from the bottom of the same bank instead.
///
/// Both candidate addresses are seeded with distinguishable bytes, so the failure says which way
/// the core went rather than merely that it was wrong.
fn a2_12() -> Test {
    let mut a = Asm::new();
    a.c("$7F:0001 is where the bank carry must land; $7E:0001 is where a masking core looks.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$3C");
    a.l("sta f:$7F0001");
    a.l("lda #$C3");
    a.l("sta f:$7E0001");
    a.c("Direct page $10..$12 holds the 24-bit pointer $7E:FFFF; Y = 2.");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("lda #$FFFF");
    a.l("sta f:$7E0010");
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l("sta f:$7E0012");
    a.l("rep #$10");
    a.l("ldy #$0002");
    a.c("DBR is deliberately left alone: [dp],Y must ignore it and use the pointer's bank.");
    a.l("lda [$10],y");
    a.assert_a8(
        0x3C,
        "[dp],Y did not carry into the next bank — $C3 means it masked to 16 bits",
    );
    a.finish(
        "A2.12",
        'A',
        "[dp],Y bank carry",
        Provenance::Documented("SNESdev Errata, 65C816 section; anomie's addressing notes"),
        Kind::Scored,
        None,
    )
}

/// `PC` wraps inside its bank on an operand fetch — it does not carry into the next one.
///
/// An instruction whose opcode sits at `$xx:FFFF` takes its operand from `$xx:0000`, back at the
/// bottom of the *same* bank. A core holding `PC` as a flat 24-bit value fetches from `$xx+1:0000`
/// instead and executes a different instruction stream entirely.
///
/// # Why this one runs from WRAM
///
/// The test has to execute at a bank boundary, and bank `$00`'s boundary is unavailable: `$00:FFE0`
/// upward is the vector table. So the instruction stream is assembled **as data** into bank `$7E`
/// and jumped to. Both banks of WRAM are writable, which also makes the wrong answer observable:
/// `$7F:0000` is seeded with a different immediate, so a carrying core loads a distinguishable
/// value rather than crashing.
///
/// Layout, with the wrap falling between the opcode and its operand:
///
/// ```text
///   $7E:FFFF  A9        LDA #imm      <- opcode, last byte of the bank
///   $7E:0000  5A        the immediate <- operand, only reachable by wrapping
///   $7E:0001  5C ...    JML back to bank $00
///   $7F:0000  A5        what a carrying core would take as the immediate
/// ```
fn a4_09() -> Test {
    let mut a = Asm::new();
    a.c("Assemble the wrapped instruction stream into bank $7E as data.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$A9          ; LDA #imm");
    a.l("sta f:$7EFFFF");
    a.l("lda #$5A          ; the immediate, at the WRAPPED address");
    a.l("sta f:$7E0000");
    a.l("lda #$A5          ; what a bank-carrying core would fetch instead");
    a.l("sta f:$7F0000");
    a.c("JML back to bank $00, assembled by hand: 5C lo hi bank.");
    a.l("lda #$5C");
    a.l("sta f:$7E0001");
    a.l("rep #$20");
    a.l("lda #.LOWORD(@landed)");
    a.l("sta f:$7E0002");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0004");
    a.c("Enter the stream. A is 8-bit, so the LDA there takes a one-byte operand.");
    a.l("jml $7EFFFF");
    a.label("landed");
    a.assert_a8(
        0x5A,
        "PC carried into the next bank on the operand fetch — $A5 means it read $7F:0000",
    );
    a.finish(
        "A4.09",
        'A',
        "PC wraps in bank",
        Provenance::Documented("WDC datasheet; SNESdev Errata, 65C816 section"),
        Kind::Scored,
        None,
    )
}

/// Where a branch lands when its target crosses the end of a bank — a golden vector, never scored.
///
/// The documented rule is that the target wraps inside the bank, like every other `PC` arithmetic
/// on this core (`A4.09`). The reason this one records instead of asserting is upstream's own
/// hedge: the timing/behaviour table marks the relative addressing modes `r`/`rl` **"XXX:
/// untested"**, in as many words. That is a source declining to vouch for its own row, and this
/// cart does not convert an untested claim into a pass rate — the same call already made for
/// decimal-mode `V` (`A7.04`).
///
/// # Both outcomes are made survivable
///
/// A golden vector has to come back from either answer, so both candidate landing sites are seeded
/// with a `JML` home rather than leaving one of them to run whatever happens to be in memory:
///
/// ```text
///   $7E:FFFD  80 10     BRA +$10   -> PC after operand is $7E:FFFF, target $FFFF+$10
///   $7E:000F  5C ...    JML @wrapped   <- where wrapping lands
///   $7F:000F  5C ...    JML @carried   <- where a 24-bit-flat core lands
/// ```
///
/// Variant 1 is the wrap (what the documentation predicts); variant 2 is the bank carry. A core
/// that changes its mind about this announces itself immediately, which is the point of recording
/// it at all.
fn a4_10() -> Test {
    let mut a = Asm::new();
    a.c("BRA at the top of bank $7E, displacement +$10.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$80          ; BRA");
    a.l("sta f:$7EFFFD");
    a.l("lda #$10          ; +16 from $FFFF");
    a.l("sta f:$7EFFFE");
    a.c("Seed both landing sites with a long jump home.");
    a.l("lda #$5C");
    a.l("sta f:$7E000F");
    a.l("sta f:$7F000F");
    a.l("rep #$20");
    a.l("lda #.LOWORD(@wrapped)");
    a.l("sta f:$7E0010");
    a.l("lda #.LOWORD(@carried)");
    a.l("sta f:$7F0010");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0012");
    a.l("sta f:$7F0012");
    a.l("jml $7EFFFD");
    a.l("@wrapped:");
    a.l("sep #$20");
    a.l("lda #$03          ; variant 1 = wrapped inside the bank (documented)");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l("@carried:");
    a.l("sep #$20");
    a.l("lda #$05          ; variant 2 = carried into the next bank");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "A4.10",
        'A',
        "Branch wrap (golden)",
        Provenance::Contested(
            "upstream marks the relative addressing modes r/rl \"XXX: untested\"; \
             no source vouches for the bank-boundary case",
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

/// In emulation mode the block-move offsets are confined to `$00xx`.
///
/// `E = 1` forces `x = 1`, so `X` and `Y` are 8-bit and their high bytes read as zero. A block move
/// therefore addresses only the bottom page of each bank, and incrementing an offset past `$FF`
/// wraps to `$00` inside that page rather than advancing to `$0100`.
///
/// # Making the wrong answer visible
///
/// The count comes from the full 16-bit `C`, which cannot be loaded once `E = 1`, so it is set in
/// native mode before the switch — `C` survives `XCE`, only the index width changes.
///
/// `X` starts at `$FF` and two bytes are moved, so the source addresses are `$7E:00FF` and then
/// either `$7E:0000` (confined, correct) or `$7E:0100` (a core that let the offset grow to 16
/// bits). All three addresses are seeded with distinct bytes, so the second destination byte says
/// which of the two happened rather than merely that something was wrong.
fn a8_06() -> Test {
    let mut a = Asm::new();
    a.c("Seed the two candidate second-source bytes distinctly, and clear the destination.");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$11");
    a.l("sta f:$7E00FF     ; first source byte");
    a.l("lda #$22");
    a.l("sta f:$7E0000     ; second source IF the offset wraps inside page 0");
    a.l("lda #$33");
    a.l("sta f:$7E0100     ; second source if the offset grew past $FF");
    a.l("lda #$00");
    a.l("sta f:$7E0050");
    a.l("sta f:$7E0051");
    a.c("The count lives in the full 16-bit C, which cannot be loaded in emulation mode, so set");
    a.c("it first: XCE changes the index width, not C.");
    a.l("rep #$30");
    a.l("lda #$0001        ; count-1: two bytes");
    a.enter_emulation();
    a.l("ldx #$FF          ; one byte short of the page end");
    a.l("ldy #$50");
    a.l("mvn #$7E,#$7E    ; literal bank numbers; `mvn $7E,$7E` would mean bank $00");
    a.enter_native();
    a.l("phk");
    a.l("plb               ; MVN left DBR = $7E");
    a.l("lda f:$7E0050");
    a.assert_a8(0x11, "the first block-move byte did not arrive");
    a.l("lda f:$7E0051");
    a.assert_a8(
        0x22,
        "the source offset was not confined to $00xx — $33 means it advanced to $0100",
    );
    a.c("--- the destination index, which the move above never takes past $FF ---");
    a.c(
        "Without this half a core with a 16-bit Y passes on the source assertion alone: Y only ran",
    );
    a.c("from $50 to $52 there and never reached its own boundary.");
    a.l("lda #$44");
    a.l("sta f:$7E0060");
    a.l("lda #$55");
    a.l("sta f:$7E0061");
    a.l("lda #$00");
    a.l("sta f:$7E0000     ; cleared: this is where a confined destination offset wraps to");
    a.l("sta f:$7E0100     ; and this is where an unconfined one would write");
    a.l("rep #$30");
    a.l("lda #$0001        ; count-1: two bytes");
    a.enter_emulation();
    a.l("ldx #$60");
    a.l("ldy #$FF          ; one byte short of the page end");
    a.l("mvn #$7E,#$7E");
    a.enter_native();
    a.l("phk");
    a.l("plb");
    a.l("lda f:$7E00FF");
    a.assert_a8(0x44, "the first destination byte did not arrive");
    a.l("lda f:$7E0000");
    a.assert_a8(
        0x55,
        "the destination offset was not confined to $00xx — it wrote past the page instead",
    );
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x00,
        "the destination offset advanced to $0100 instead of wrapping inside page 0",
    );
    a.finish(
        "A8.06",
        'A',
        "E=1 confines MVN offsets",
        Provenance::Documented("WDC datasheet: E=1 forces x=1, so the indices are 8-bit"),
        Kind::Scored,
        None,
    )
}

/// The register-preserving, invocation-counting NMI handler `A8.06` uses.
///
/// Two details are load-bearing rather than hygiene, and both cost a debugging round when they were
/// missing:
///
/// * **`A`/`X`/`Y` are preserved.** `RTI` restores `P`, `PC` and `PBR` — not the registers — and a
///   block move keeps its remaining count in `A` and its offsets in `X`/`Y`. A handler that
///   clobbers them makes the move resume wrong, which reads exactly like the resume-short defect
///   `A8.06` exists to detect.
/// * **`$4210` is read long.** `MVN` sets `DBR` to its destination bank while it runs, so an
///   interrupt taken mid-move enters here with `DBR = $7F`; absolute `lda $4210` would read WRAM
///   instead of acknowledging `RDNMI`, the NMI line would stay asserted, and the handler would
///   re-enter until the move starved.
fn emit_nmi_counting_handler(a: &mut Asm) {
    a.label("nmi");
    a.c("Preserve A/X/Y. RTI restores P, PC and PBR — NOT the registers — and for THIS test that");
    a.c("is load-bearing rather than hygiene: MVN keeps its remaining byte count in A and its");
    a.c("offsets in X and Y, so a handler that clobbers them makes the move resume wrong. The");
    a.c("first version of this test omitted the pushes and lost the tail of the block, which read");
    a.c("exactly like the resume-short defect the test exists to detect.");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("pha");
    a.l("phx");
    a.l("phy");
    a.l("sep #$20");
    a.l(".a8");
    a.l("lda f:$7E0150");
    a.l("inc a");
    a.l("sta f:$7E0150");
    a.c("LONG addressing, and this is not tidiness: MVN sets DBR to its DESTINATION bank while it");
    a.c("runs, so an interrupt taken mid-move enters the handler with DBR = $7F. Absolute `lda");
    a.c("$4210` would read WRAM at $7F:4210 instead of acknowledging RDNMI, the NMI line would");
    a.c("stay asserted, and the handler would re-enter until the move starved.");
    a.l("lda f:$004210     ; acknowledge RDNMI");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("ply");
    a.l("plx");
    a.l("pla");
    a.l("rti");
}

/// `MVN` is interruptible mid-block: an NMI during a long block move resumes it correctly.
///
/// A block move is a loop *inside a single opcode* — it decrements `A` and rewinds `PC` by 3 until
/// the count exhausts. Hardware takes interrupts between iterations, and the pushed `PC` points at
/// the `MVN` itself, so `RTI` re-enters it and the remaining bytes copy. A core that treats the
/// whole move as atomic delays the interrupt; one that resumes with a corrupted count copies the
/// wrong number of bytes.
///
/// # Why this needs the NMI runtime, and both halves pinned
///
/// This is the row the NMI vector wiring was built for: nothing else in the battery generates a
/// real interrupt mid-instruction. The handler counts its invocations, and **that count is half
/// the assertion** — checking only that the block copied correctly would be vacuous, because a
/// core where the NMI never fires at all copies it correctly too. That is the same trap `D2.07`
/// carries and the one that withdrew `A4.06`.
///
/// The move is **8192 bytes**, which at roughly 52 clocks a byte is about 426,000 master clocks —
/// longer than one 357,368-clock frame, so a `VBlank` NMI must land inside it. Only three source
/// bytes are seeded and three destination bytes checked (first, middle, **last**); the last is the
/// one that matters, because a core that loses its place on resumption finishes short and only the
/// tail shows it.
fn a8_07() -> Test {
    let mut a = Asm::new();
    a.c("The NMI handler counts invocations and acknowledges RDNMI.");
    a.l("bra @body");
    emit_nmi_counting_handler(&mut a);
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("rep #$20");
    a.l("lda #@nmi");
    a.l("sta a:V_NMI_VEC");
    a.c("Seed only the three source bytes that get checked, and clear their destinations so");
    a.c("'arrived' is distinguishable from 'was already there'.");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0150     ; NMI count");
    a.l("lda #$A1");
    a.l("sta f:$7E2000     ; source offset 0");
    a.l("lda #$B2");
    a.l("sta f:$7E3000     ; source offset 4096");
    a.l("lda #$C3");
    a.l("sta f:$7E3FFF     ; source offset 8191 — the last byte moved");
    a.l("lda #$00");
    a.l("sta f:$7F0000");
    a.l("sta f:$7F1000");
    a.l("sta f:$7F1FFF");
    a.c("Arm VBlank NMI. NMI ignores the I flag, so no CLI is needed.");
    a.l("lda #$80");
    a.l("sta $4200");
    a.c("8192 bytes from $7E:2000 to $7F:0000. Emitted raw: the machine encoding is");
    a.c("54 <dest> <src>, the reverse of how the mnemonic reads (A8.04 asserts exactly this).");
    a.l("rep #$30");
    a.l("ldx #$2000");
    a.l("ldy #$0000");
    a.l("lda #$1FFF        ; count-1");
    a.l(".byte $54, $7F, $7E");
    a.l("phk");
    a.l("plb               ; MVN left DBR = $7F");
    a.l("sep #$20");
    a.l("stz $4200         ; disarm before asserting; a failure exits immediately");
    a.l("lda $4210         ; clear any pending RDNMI latch");
    a.c("Half one: the interrupt actually happened. Without this the copy check below is");
    a.c("satisfied just as well by a core that never took an NMI at all.");
    a.c("Normalise any non-zero count to 1: how MANY NMIs land depends on where in the frame the");
    a.c("move started, which is not something the cart fixes, so only 'at least one' is asserted.");
    a.l("lda f:$7E0150");
    a.l("beq :+");
    a.l("lda #$01");
    a.l(":");
    a.assert_a8(
        0x01,
        "no NMI fired during the block move — the copy check below proves nothing without it",
    );
    a.c("Half two: every part of the block arrived, the last byte most of all.");
    a.l("lda f:$7F0000");
    a.assert_a8(0xA1, "the first byte of the block move did not arrive");
    a.l("lda f:$7F1000");
    a.assert_a8(0xB2, "the middle of the block move did not arrive");
    a.l("lda f:$7F1FFF");
    a.assert_a8(
        0xC3,
        "the LAST byte did not arrive — MVN resumed short after the interrupt",
    );
    a.finish(
        "A8.07",
        'A',
        "MVN interruptible",
        Provenance::Documented(
            "WDC datasheet: MVN rewinds PC by 3 per iteration, so RTI re-enters",
        ),
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

/// The `A5.22` cycle spot checks: `XBA` = 3 cycles, `REP`/`SEP` = 3, `PHD` = 4, `PLD` = 5.
///
/// # Converting cited cycle counts into measurable time
///
/// 65816 *cycles* do not map to a fixed number of dots: each is 6, 8 or 12 master clocks depending
/// on what it touches. With code in bank `$00` ROM and the stack in low WRAM (both 8-clock) and
/// internal cycles at 6:
///
/// ```text
/// clocks = 8*mem + 6*internal,  cycles = mem + internal   =>   clocks = 6*cycles + 2*mem
/// ```
///
/// `mem` being instruction length plus data/stack accesses. That second term is why `NOP` and
/// `LDA #imm` — both 2 cycles — do not cost the same. From the cited counts: `NOP` 14 clocks,
/// `XBA` 20, `REP #imm` 22, `PHD` 30, `PLD` 36. See `docs/accuracysnes-timing-oracle.md`; the
/// structure is corroborated by all three vendor renderings of the instruction-operation table.
///
/// # Why the repeat count is 16 and not 32
///
/// **The measurement cannot span a scanline.** `hv_begin`/`hv_end` difference the H counter, which
/// wraps at `DOTS_PER_LINE` (341), so a longer span silently returns a small number rather than
/// failing. This test previously used 32 repeats: the `REP` block landed at exactly 341 dots
/// absolute and measured ~0, which read as "RustySNES gets `REP` wrong". It does not — its `REP` is
/// opcode fetch + operand fetch + one internal cycle, precisely what the datasheets specify. The
/// bug was the measurement wrapping, and it was invisible until the full-width measurement channel
/// existed to show the raw numbers.
///
/// At 16 repeats against a 16-`NOP` baseline every span stays under 300 dots absolute, with the
/// raw values recorded to the channel so the next person can check rather than infer.
fn a5_08() -> Test {
    let mut a = Asm::new();
    a.c("Differential against NOP, 16 repeats, so no span approaches the 341-dot line wrap.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("--- baseline: 16 NOPs ---");
    a.measure_begin();
    a.repeat(16, &["nop"]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0090");
    a.record(0, "16 NOP, absolute");
    a.c("--- XBA: +6 clocks each over NOP, so +24 dots over 16 ---");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(16, &["xba"]);
    a.measure_end();
    a.measure_result();
    a.record(1, "16 XBA, absolute");
    a.l("sec");
    a.l("sbc f:$7E0090");
    a.record(2, "16 XBA - 16 NOP");
    a.assert_a16_range(
        24 - TOL,
        24 + TOL,
        "XBA did not cost 3 cycles (1 more than NOP)",
    );
    a.c("--- REP #$00: 2 bytes, so +8 clocks each, +32 dots over 16. Emitted as raw bytes so the");
    a.c("generator's width tracker is not told the accumulator changed size.");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(16, &[".byte $C2, $00   ; rep #$00"]);
    a.measure_end();
    a.measure_result();
    a.record(3, "16 REP #$00, absolute");
    a.l("sec");
    a.l("sbc f:$7E0090");
    a.record(4, "16 REP #$00 - 16 NOP");
    a.assert_a16_range(
        32 - TOL,
        32 + TOL,
        "REP #imm did not cost 3 cycles / 2 accesses",
    );
    a.c("--- PHD+PLD: 30 + 36 = 66 clocks per pair against 2 NOPs at 28, so +76 dots over 8 ---");
    a.l("sep #$20");
    a.measure_begin();
    a.repeat(8, &["phd", "pld"]);
    a.measure_end();
    a.measure_result();
    a.record(5, "8x (PHD+PLD), absolute");
    a.l("sec");
    a.l("sbc f:$7E0090");
    a.record(6, "8x (PHD+PLD) - 16 NOP");
    a.assert_a16_range(
        76 - TOL,
        76 + TOL,
        "PHD/PLD did not cost 4 and 5 cycles with 3 accesses each",
    );
    a.finish(
        "A5.08",
        'A',
        "A5.22 cycle spot checks",
        Provenance::Documented(
            "WDC/GTE/VLSI instruction-operation tables agree on these rows; \
             docs/accuracysnes-timing-oracle.md",
        ),
        Kind::Scored,
        None,
    )
}

/// Emulation-mode read-modify-write: does the modify cycle perform a **write**?
///
/// WDC's note (17) states that *"in the emulation mode, during a R-M-W instruction the RWB is low
/// during both write and modify cycles"*. GTE's and VLSI's renderings of the same table are silent
/// on it — so this is a **single-vendor claim**, and the kind of thing a test cartridge exists to
/// settle rather than inherit.
///
/// # How a bus signal becomes observable from software
///
/// `RWB` low means a write. If the modify cycle writes, an R-M-W against a **write-sensitive**
/// register performs *two* writes rather than one. `$2102`/`$2104` is the natural probe: OAM's data
/// port auto-increments its address on every write, so the address counter afterwards counts
/// writes directly, whatever values were written.
///
/// The sequence sets the OAM address, runs one `INC $2104` in emulation mode, then rewinds and
/// reads back to see how far the counter moved. One write leaves it at 1; a modify-cycle write
/// leaves it at 2.
///
/// # The seed value is load-bearing
///
/// `OAM[1]` is seeded `$99`, and specifically **not** `$22`, because `$2104` is write-only: the
/// R-M-W's read returns **open bus**, which is the last byte fetched — `$21`, the high byte of the
/// operand address — and `INC` makes that `$22`. Seeding `$22` therefore collides:
///
/// - one write  -> `OAM[0] = $22`, `OAM[1]` keeps its seed
/// - two writes -> `OAM[0] = $21` (the unmodified value), `OAM[1] = $22` (the modified one)
///
/// With a `$22` seed both paths leave `$22` in `OAM[1]` and the probe silently always reports one
/// write. The first version of this test did exactly that, and the three-way emulator split it
/// appeared to show was partly an artifact of the collision rather than a real disagreement.
///
/// # Why this reports rather than asserts
///
/// Two of the three vendor tables decline to state it, so neither answer has the weight to score.
/// Variant 1 = one write (the counter moved 1), variant 2 = two writes (WDC's note holds).
/// Recording it makes the observation available to every emulator that runs the cart without
/// anyone's convention being promoted to a pass rate.
fn a9_03() -> Test {
    let mut a = Asm::new();
    a.c("Seed OAM words 0 and 1, aim the port at word 0, then do one R-M-W on $2104 in emulation");
    a.c("mode. The OAM address counter afterwards reports how many writes actually happened.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("--- seed two words so a stray second write is visible, and clear the probe ---");
    a.l("stz $2102");
    a.l("stz $2103");
    a.l("lda #$11");
    a.l("sta $2104");
    a.l("lda #$99       ; NOT $22 — see the collision note below");
    a.l("sta $2104");
    a.l("lda #$33");
    a.l("sta $2104");
    a.l("lda #$44");
    a.l("sta $2104");
    a.c("--- aim at word 0 and perform ONE read-modify-write on the data port ---");
    a.l("stz $2102");
    a.l("stz $2103");
    a.enter_emulation();
    a.l("inc a:$2104       ; R-M-W on the write-sensitive OAM data port");
    a.enter_native();
    a.c("--- how far did the address counter move? rewind and count back ---");
    a.l("sep #$20");
    a.l("stz $2102");
    a.l("stz $2103");
    a.l("lda $2138");
    a.l("sta f:$7E0130     ; byte 0 after the R-M-W");
    a.l("lda $2138");
    a.l("sta f:$7E0131     ; byte 1");
    a.c("A single write advances the port by one byte; a modify-cycle write advances it by two.");
    a.c("Byte 1 still holding its seed means one write; overwritten means two.");
    a.l("lda f:$7E0131");
    a.l("cmp #$99");
    a.l("bne @two");
    a.l("lda #$03          ; variant 1 = one write — the modify cycle did not write");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.label("two");
    a.l("lda #$05          ; variant 2 = two writes — WDC note (17) holds");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "A9.03",
        'A',
        "E=1 R-M-W modify write",
        Provenance::Contested(
            "WDC note (17) asserts RWB is low during the modify cycle in emulation mode; \
             the GTE and VLSI renderings of the same table are silent",
        ),
        Kind::Golden,
        None,
    )
}

/// `TCS` and `TXS` set **no flags**, where every other transfer sets N and Z.
///
/// The stack pointer is not data, so moving a value into it does not describe that value — and a
/// core that routes all the transfers through one flag-setting helper gets this wrong in a way
/// nothing crashes on. It changes which branch is taken after a `TXS`, which is the sort of bug
/// that produces one wrong branch a frame in code nobody suspects.
///
/// **Both instructions are given the value the stack pointer already holds**, so `S` is written
/// with what was in it and the stack survives the test. That matters more than it looks: `TXS` in
/// native mode is a full 16-bit write, so a test that put anything else there would be pushing its
/// own `PHP` into ROM.
///
/// **Both flags are checked, and both are planted at the value a flag-setting transfer would have
/// to change.** `S` is `$1FFF`, whose own flags are `N` clear and `Z` clear — so `Z` is set with
/// `BIT #imm` (which affects `Z` alone, per `A9.01`, and therefore leaves the accumulator the
/// transfer is about to move) and `N` is set with `SEP #$80`, which writes `P` directly and touches
/// no register at all. Checking `Z` alone would have been satisfied by a core that wrongly updates
/// `N`: `$1FFF` has bit 15 clear, so `N` would have been clear before and after.
///
/// The third part is the control, and without it the first two are satisfied by a core that never
/// sets any flags at all: `TXA` moving `$8000` **must** set `N` and clear `Z`.
fn a1_07() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c(
        "TCS: hand it the stack pointer's own value, so S is unchanged and PHP still lands in RAM.",
    );
    a.l("tsc               ; A = S, which is $1FFF: bit 15 clear, non-zero");
    a.l("sep #$80          ; N = 1, and SEP writes P directly without touching A");
    a.l("bit #$0000        ; Z = 1, and BIT #imm touches nothing else");
    a.l("tcs               ; the instruction under test");
    a.l("php");
    a.l("sep #$20");
    a.l("pla");
    a.l("and #$82          ; N and Z together");
    a.assert_a8(
        0x82,
        "TCS changed N or Z, so it set flags from the value transferred — the stack pointer is \
         not data and moving a value into it describes nothing",
    );
    a.c("Both flags, not just Z: S is $1FFF, whose own flags are N clear and Z clear, so each of");
    a.c("the two is planted at the value a flag-setting transfer would have to change.");
    a.c("TXS, the same way: X gets S's own value first.");
    a.l("rep #$30");
    a.l("tsx               ; X = S");
    a.l("sep #$80          ; N = 1");
    a.l("bit #$0000        ; Z = 1");
    a.l("txs               ; the instruction under test");
    a.l("php");
    a.l("sep #$20");
    a.l("pla");
    a.l("and #$82");
    a.assert_a8(
        0x82,
        "TXS changed N or Z, so it set flags from the value transferred",
    );
    a.c("The control: TXA is an ordinary transfer and MUST set N and clear Z.");
    a.l("rep #$30");
    a.l("ldx #$8000");
    a.l("rep #$80          ; N = 0, so a transfer that sets flags has to set it");
    a.l("bit #$0000        ; Z = 1, so a transfer that sets flags has to clear it");
    a.l("txa               ; A = $8000: negative, non-zero");
    a.l("php");
    a.l("sep #$20");
    a.l("pla");
    a.l("and #$82          ; N and Z together");
    a.assert_a8(
        0x80,
        "TXA did not set N and clear Z from $8000, so this core sets no transfer flags at all and \
         the two assertions above say nothing",
    );
    a.l("rep #$30");
    a.l("rep #$80          ; leave N as the battery found it");
    a.finish(
        "A1.07",
        'A',
        "TCS/TXS set no flags",
        Provenance::Documented("WDC 65C816 datasheet; 6502.org 65c816opcodes"),
        Kind::Scored,
        None,
    )
}

/// `ORA [d]` reaches through a **24-bit** pointer — the Super Mario World case.
///
/// Direct-page indirect long is the addressing mode a core is most likely to implement as its
/// 16-bit sibling with the data bank glued on, because for a pointer that happens to live in bank
/// `$00` the two are identical. This image is 128 KiB precisely so they are not: each bank carries
/// its own signature byte at `$xx:8005`, so a pointer whose stored bank is `$01` reads `$A1` where
/// a `(d)`-style fetch would read `$A0`.
///
/// Three parts, and the middle one is the assertion:
///
/// 1. the pointer aimed at bank `$00` reads `$A0` — the mode works at all;
/// 2. the same pointer with `$01` in its third byte reads `$A1` — **the bank byte is honoured**;
/// 3. `$0F` already in the accumulator comes back as `$AF` — it is an `ORA` and not a load.
///
/// Part 3 is not padding. Parts 1 and 2 both start from `A = $00`, where `ORA` and `LDA` are
/// indistinguishable, so without it the test would say nothing about the operation it names.
fn a9_04() -> Test {
    const PTR: &str = "$60";

    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("lda #$0000");
    a.l("tcd               ; DP = 0, so the pointer lives at $00:0060");
    a.c("Pointer -> $00:8005, bank $00's signature byte.");
    a.l("lda #$8005");
    a.l(&format!("sta {PTR}"));
    a.l("sep #$30");
    a.l("lda #$00");
    a.l(&format!("sta {PTR}+2      ; the bank byte"));
    a.l("lda #$00");
    a.l(&format!("ora [{PTR}]"));
    a.assert_a8(
        0xA0,
        "ORA [d] through a pointer at $00:8005 did not read bank $00's signature byte",
    );
    a.c("Now the same pointer with bank $01. A (d)-style fetch through DBR still reads $A0 here.");
    a.l("lda #$01");
    a.l(&format!("sta {PTR}+2"));
    a.l("lda #$00");
    a.l(&format!("ora [{PTR}]"));
    a.assert_a8(
        0xA1,
        "ORA [d] did not read bank $01's signature through a pointer whose third byte is $01, so \
         the effective address was not built from that byte",
    );
    a.c("And it really is an OR: $0F in the accumulator survives into the result.");
    a.l("lda #$0F");
    a.l(&format!("ora [{PTR}]"));
    a.assert_a8(
        0xAF,
        "ORA [d] replaced the accumulator instead of OR-ing into it, so the two readings above \
         were loads and say nothing about ORA",
    );
    a.l("rep #$30");
    a.finish(
        "A9.04",
        'A',
        "ORA [d] is 24-bit",
        Provenance::Documented("WDC 65C816 datasheet; the Super Mario World case, SNESdev Wiki"),
        Kind::Scored,
        None,
    )
}

/// `(dp,X)` inherits the `d,X` rules: the **pointer fetch** wraps inside bank `$00`.
///
/// `A2.01` establishes that `d,X` never crosses a bank boundary. This asserts that the same is true
/// of the address `(dp,X)` reads its *pointer* from, which is a separate piece of code in most
/// cores and is the one place the rule is easy to forget — the pointer fetch happens before the
/// mode looks like an indexed access at all.
///
/// `D = $FFFF`, `X = $8000`, operand `$06`: the sum is `$18005`, which must wrap to `$00:8005` and
/// not reach bank `$01`. Bank `$00`'s signature there is `$A0` and bank `$01`'s is `$A1`, so the
/// two cases fetch **different pointers** — and both of those pointers are aimed at bytes this test
/// has planted, `$5A` at `$00:00A0` and `$3C` at `$00:00A1`. So a core that crosses the bank does
/// not read garbage or fault; it reads `$3C`, and the failure says which mistake it made.
///
/// That is the whole reason the image is 128 KiB. Inside a 32 KiB ROM the two banks mirror the same
/// bytes, both cases fetch the same pointer, and the assertion is unfalsifiable.
fn a2_11() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Plant the two possible destinations before touching D: $A0 -> $5A, $A1 -> $3C.");
    a.l("sep #$20");
    a.l("lda #$5A");
    a.l("sta a:$00A0       ; where bank $00's pointer aims");
    a.l("lda #$3C");
    a.l("sta a:$00A1       ; where bank $01's pointer would aim");
    a.c("D = $FFFF, X = $8000, operand $06: $FFFF + $06 + $8000 = $18005, wrapping to $00:8005.");
    a.l("rep #$30");
    a.l("lda #$FFFF");
    a.l("tcd");
    a.l("ldx #$8000");
    a.l("sep #$20");
    a.l("lda ($06,X)");
    a.l("sta f:$7E0132     ; stash before restoring D, which needs a 16-bit A");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("tcd               ; restore D BEFORE asserting, or every later dp access is relocated");
    a.l("sep #$20");
    a.l("lda f:$7E0132");
    a.assert_a8(
        0x5A,
        "(dp,X) fetched its pointer from bank $01 — reading $3C means the pointer address was \
         allowed to carry out of bank $00 instead of wrapping inside it",
    );
    a.l("rep #$30");
    a.finish(
        "A2.11",
        'A',
        "(dp,X) pointer wraps",
        Provenance::Documented("6502.org 65c816opcodes; superfamicom.org addressing modes"),
        Kind::Scored,
        None,
    )
}

/// `N` and `Z` are valid in decimal mode, unlike on the NMOS 6502.
///
/// The 6502 left `N` and `Z` describing the *binary* sum while `A` held the decimal one; the 65C02
/// and the 65C816 fixed it, at the cost of a cycle. A core that reuses a 6502's flag logic and
/// bolts BCD correction on afterwards reproduces the old behaviour exactly, and the symptom is a
/// branch taken wrongly in scorekeeping code — arithmetic that looks right in a memory dump and
/// behaves wrongly.
///
/// Both readings are chosen so the two answers **differ**, which is what a reading has to do here —
/// on an input where the binary and decimal results share a sign and a zero-ness, the two flag
/// models agree and the reading says nothing. These are two such inputs, one per flag, not the only
/// two:
///
/// * `$99 + $01` is `$9A` in binary and `$00` in decimal, so `Z` distinguishes them.
/// * `$79 + $79` is `$F2` in binary and `$58` in decimal, so `N` does.
///
/// The accumulator is asserted too, since a flag that describes the right value is only meaningful
/// if the value is right; `A7.01` already covers the arithmetic itself, and this narrows to the two
/// readings where flags and result disagree.
fn a7_05() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("sed");
    a.c("$99 + $01 = $00 in decimal, $9A in binary. Z is set only for the decimal answer.");
    a.l("clc");
    a.l("lda #$99");
    a.l("adc #$01");
    a.l("php");
    a.l("sta f:$7E0133");
    a.l("pla");
    a.l("and #$02          ; Z");
    a.l("sta f:$7E0134");
    a.c("$79 + $79 = $58 in decimal, $F2 in binary. N is clear only for the decimal answer.");
    a.l("clc");
    a.l("lda #$79");
    a.l("adc #$79");
    a.l("php");
    a.l("sta f:$7E0135");
    a.l("pla");
    a.l("and #$80          ; N");
    a.l("sta f:$7E0136");
    a.l("cld               ; decimal off before anything else runs");
    a.l("lda f:$7E0133");
    a.assert_a8(
        0x00,
        "SED: $99 + $01 did not give $00, so the BCD addition itself is wrong",
    );
    a.l("lda f:$7E0134");
    a.assert_a8(
        0x02,
        "Z was clear after a decimal $99 + $01 = $00, so the flags describe the binary sum $9A — \
         the NMOS 6502's behaviour, which the 65C816 does not have",
    );
    a.l("lda f:$7E0135");
    a.assert_a8(
        0x58,
        "SED: $79 + $79 did not give $58, so the BCD addition itself is wrong",
    );
    a.l("lda f:$7E0136");
    a.assert_a8(
        0x00,
        "N was set after a decimal $79 + $79 = $58, so the flags describe the binary sum $F2",
    );
    a.l("rep #$30");
    a.finish(
        "A7.05",
        'A',
        "N/Z valid in decimal",
        Provenance::Documented("WDC 65C816 datasheet; 6502.org 65c816opcodes decimal notes"),
        Kind::Scored,
        None,
    )
}

/// Emulation mode uses its **own** vector table: `COP` goes through `$FFF4`, not `$FFE4`.
///
/// The two tables sit sixteen bytes apart and a core that keeps one set of vectors, or that picks
/// the table from something other than the E flag, lands in the wrong handler. Nothing about that
/// is visible in ordinary code — a game's `COP` handler is usually the same routine either way —
/// which is exactly why it survives.
///
/// **The cart could not see it either until this test**: `$FFE4` and `$FFF4` both pointed at the
/// same trampoline, so a core taking the native vector in emulation mode ran the same handler and
/// passed. The runtime now gives the emulation vectors their own trampolines and their own RAM
/// pointers, which is what makes the two distinguishable at all.
///
/// Both handlers are installed and both are live, so a core that takes the wrong table does not
/// hang or fault: it runs the *other* handler, writes the other marker, and the failure says which
/// vector it used. A test whose only failure mode is a crash reports nothing.
fn a6_10() -> Test {
    let mut a = Asm::new();
    a.l("jmp @start");
    a.c("Both handlers run in EMULATION mode, so ca65 has to be told the registers are 8 bits");
    a.c("wide here. Without it the test body's own `.a16` state is still in force, `lda #$E0`");
    a.c("assembles as three bytes, the CPU takes two, and the third ($00) is executed as a BRK.");
    a.c("The first draft of this test did exactly that and reported neither handler.");
    a.l(".a8");
    a.l(".i8");
    a.c("The emulation handler: the one that must run. RTI here is an emulation-mode RTI, pulling");
    a.c("three bytes, which is correct because the interrupt was taken in emulation mode.");
    a.label("handler_e");
    a.l("lda #$E0");
    a.l("sta f:$7E0094");
    a.l("rti");
    a.c("And the native handler, installed and reachable, so taking the wrong table is a wrong");
    a.c("ANSWER rather than a hang. It is entered in emulation mode too — the mode is decided by");
    a.c(
        "the COP, not by which vector was used — so its RTI returns just as cleanly; the marker it",
    );
    a.c("leaves behind is the whole of its job.");
    a.label("handler_n");
    a.l("lda #$B0");
    a.l("sta f:$7E0094");
    a.l("rti");
    a.l(".a16");
    a.l(".i16");
    a.label("start");
    a.l("rep #$30");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0094");
    a.l("rep #$30");
    a.l("lda #.LOWORD(@handler_e)");
    a.l("sta a:V_COP_VEC_E   ; reached through $FFF4");
    a.l("lda #.LOWORD(@handler_n)");
    a.l("sta a:V_COP_VEC     ; reached through $FFE4 — the wrong one from emulation mode");
    a.enter_emulation();
    a.c("Raw bytes rather than `cop #$00`: ca65 2.19, which CI installs, rejects the immediate");
    a.c("form as an illegal addressing mode. See A6.02.");
    a.l(".byte $02, $00    ; cop #$00");
    a.enter_native();
    a.l("rep #$30");
    a.assert_mem8(
        0x7E_0094,
        0xE0,
        "COP in emulation mode did not vector through $FFF4 — the $B0 marker means it took the \
         native table's $FFE4 instead, and $00 means it reached neither handler",
    );
    a.finish(
        "A6.10",
        'A',
        "Emulation COP vector",
        Provenance::Documented("WDC 65C816 datasheet, vector table; SNESdev Wiki vectors"),
        Kind::Scored,
        None,
    )
}

/// The machine encoding is `$54 <dest> <src>` — the **destination bank byte comes first**, which is
/// the reverse of how the mnemonic is written.
///
/// `MVN $00,$7E` assembles to `54 7E 00`. Assemblers hide this, so a core written against the
/// mnemonic rather than the opcode table copies in the wrong direction and nothing in the source
/// looks wrong. It is a one-byte mistake with no symptom short of data appearing where it should not.
///
/// The test emits the three bytes **by hand** rather than through `mvn`, because going through the
/// assembler would test the assembler's convention rather than the core's decoding of the bytes.
///
/// Both interpretations are made to land somewhere known, so the failure names the mistake instead
/// of producing garbage:
///
/// * decoded correctly, it reads bank `$00`'s signature `$A0` at `$00:8005` and writes it to
///   `$7E:0300`;
/// * decoded with the operands swapped, it reads `$7E:8005` — seeded `$3C` here for exactly this —
///   and writes it to `$00:0300`, which is the same byte as `$7E:0300` through the low-WRAM mirror.
///
/// So one destination byte distinguishes the two, and the value found there says which way the
/// operands were read.
fn a8_04() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Seed the byte a swapped decode would read, and the destination it would leave alone.");
    a.l("sep #$20");
    a.l("lda #$3C");
    a.l("sta f:$7E8005     ; what `read from bank $7E` would find");
    a.l("lda #$5A");
    a.l("sta f:$7E0300     ; the destination, so 'untouched' is distinguishable too");
    a.c("MVN of one byte: X = source offset, Y = destination offset, A = count - 1.");
    a.l("rep #$30");
    a.l("ldx #$8005");
    a.l("ldy #$0300");
    a.l("lda #$0000        ; one byte");
    a.c("$54 then DESTINATION then SOURCE. Written as bytes so this tests the core's decoding");
    a.c("rather than ca65's operand order — which is `mvn <src>,<dest>`, so `mvn #$00,#$7E`");
    a.c("assembles to exactly these three bytes. Checked by assembling it, not assumed.");
    a.l(".byte $54, $7E, $00");
    a.l("phk");
    a.l("plb               ; MVN leaves DBR = destination bank; restore it before reading back");
    a.l("sep #$20");
    a.l("lda f:$7E0300");
    a.assert_a8(
        0xA0,
        "MVN read its operands in mnemonic order rather than machine order — $3C means source and \
         destination were swapped, and $5A means the move did not happen at all",
    );
    a.l("rep #$30");
    a.finish(
        "A8.04",
        'A',
        "MVN encodes dest first",
        Provenance::Documented("WDC 65C816 datasheet, opcode table; 6502.org 65c816opcodes"),
        Kind::Scored,
        None,
    )
}
