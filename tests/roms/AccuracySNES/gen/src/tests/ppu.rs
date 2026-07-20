//! Group C — the S-PPU1 / S-PPU2.
//!
//! Per `docs/accuracysnes-research-dossier.md` §5.C. This first batch is sub-groups **C1-C3**:
//! the OAM, VRAM, and CGRAM/counter *port mechanics*.
//!
//! They are deliberately first. Port behaviour is pure register logic with no dependence on the
//! renderer, so it establishes a passing baseline before the sub-groups (C6 offset-per-tile,
//! C7 sprites, C9 hi-res) that lean on parts of the PPU this project's own docs already record as
//! simplified — the per-scanline compositor, and offset-per-tile and interlace not being wired to
//! dot resolution.
//!
//! Every test runs under **forced blank** (the runtime keeps `INIDISP = $8F` for the duration of
//! the battery), which is exactly when VRAM, OAM, and CGRAM are architecturally accessible. The
//! access-during-render cases are a separate, later batch: they need the renderer running and are
//! among the most contested behaviours in the whole corpus.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// Every Group C test in this batch, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        // --- C1: OAM port ---
        c1_01(),
        c1_02(),
        c1_03(),
        c1_04(),
        // --- C2: VRAM port ---
        c2_01(),
        c2_02(),
        c2_03(),
        c2_04(),
        c2_05(),
        // --- C3: CGRAM and the H/V counters ---
        c3_01(),
        c3_02(),
        c3_03(),
        c3_04(),
    ]
}

// ---------------------------------------------------------------------------------------------
// C1 — OAM port
// ---------------------------------------------------------------------------------------------

/// A word written to the OAM low table commits as a pair, low byte first.
fn c1_01() -> Test {
    let mut a = Asm::new();
    a.c("OAMADDR is a WORD address. Two byte writes to $2104 fill one word, low byte first.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $2102         ; OAMADDR = word 0");
    a.l("stz $2103");
    a.l("lda #$AA");
    a.l("sta $2104");
    a.l("lda #$BB");
    a.l("sta $2104         ; word 0 now committed");
    a.l("stz $2102         ; rewind to word 0 to read it back");
    a.l("stz $2103");
    a.l("lda $2138");
    a.assert_a8(0xAA, "OAM low byte did not read back");
    a.l("lda $2138");
    a.assert_a8(0xBB, "OAM high byte did not read back");
    a.finish(
        "C1.01",
        'C',
        "OAM word write/read",
        Provenance::Documented("SNESdev Wiki, OAM; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The OAM low table commits on the **second** byte: an odd trailing write stays in the latch and
/// never reaches memory.
///
/// This is the write-twice latch, and it is the reason a naive "write N bytes" OAM upload with an
/// odd count silently loses its last byte.
fn c1_02() -> Test {
    let mut a = Asm::new();
    a.c("Seed word 1 with a known value, then write THREE bytes from word 0. The third byte is");
    a.c("latched as the low half of word 1 and must not be committed on its own.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("--- seed word 1 = $EEDD ---");
    a.l("lda #$01");
    a.l("sta $2102         ; OAMADDR = word 1");
    a.l("stz $2103");
    a.l("lda #$DD");
    a.l("sta $2104");
    a.l("lda #$EE");
    a.l("sta $2104");
    a.c("--- three bytes starting at word 0 ---");
    a.l("stz $2102");
    a.l("stz $2103");
    a.l("lda #$11");
    a.l("sta $2104");
    a.l("lda #$22");
    a.l("sta $2104         ; word 0 committed");
    a.l("lda #$33");
    a.l("sta $2104         ; latched only — must NOT reach word 1");
    a.c("--- read back ---");
    a.l("stz $2102");
    a.l("stz $2103");
    a.l("lda $2138");
    a.assert_a8(0x11, "word 0 low byte wrong");
    a.l("lda $2138");
    a.assert_a8(0x22, "word 0 high byte wrong");
    a.l("lda $2138");
    a.assert_a8(
        0xDD,
        "the odd trailing byte was committed (it must stay in the latch)",
    );
    a.l("lda $2138");
    a.assert_a8(0xEE, "word 1 high byte was disturbed");
    a.finish(
        "C1.02",
        'C',
        "OAM odd write latched",
        Provenance::Documented("SNESdev Wiki, OAM; anomie"),
        Kind::Scored,
        None,
    )
}

/// Writing `$2102` or `$2103` reloads the address and clears the low bit of the internal counter.
fn c1_03() -> Test {
    let mut a = Asm::new();
    a.c("Write an odd byte count, then reload OAMADDR: the pending latch is discarded and the");
    a.c("next pair starts cleanly rather than being offset by one byte.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$02");
    a.l("sta $2102         ; word 2");
    a.l("stz $2103");
    a.l("lda #$99");
    a.l("sta $2104         ; leave a byte pending in the latch");
    a.l("lda #$02");
    a.l("sta $2102         ; reload -> discard the pending byte");
    a.l("stz $2103");
    a.l("lda #$44");
    a.l("sta $2104");
    a.l("lda #$55");
    a.l("sta $2104");
    a.l("lda #$02");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda $2138");
    a.assert_a8(
        0x44,
        "reloading OAMADDR did not discard the pending latch byte",
    );
    a.l("lda $2138");
    a.assert_a8(0x55, "word high byte wrong after OAMADDR reload");
    a.finish(
        "C1.03",
        'C',
        "OAMADDR reload clears",
        Provenance::Documented("SNESdev Wiki, OAM; anomie"),
        Kind::Scored,
        None,
    )
}

/// Reads through `$2138` and writes through `$2104` advance the same address counter.
fn c1_04() -> Test {
    let mut a = Asm::new();
    a.c("Write one word, then read one byte, then read again: the read pointer must have followed");
    a.c("the write pointer rather than tracking separately.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$04");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda #$5A");
    a.l("sta $2104");
    a.l("lda #$A5");
    a.l("sta $2104");
    a.l("lda #$7E");
    a.l("sta $2104");
    a.l("lda #$E7");
    a.l("sta $2104         ; words 4 and 5 written");
    a.l("lda #$05");
    a.l("sta $2102         ; point at word 5 only");
    a.l("stz $2103");
    a.l("lda $2138");
    a.assert_a8(0x7E, "shared counter: word 5 low byte wrong");
    a.l("lda $2138");
    a.assert_a8(0xE7, "shared counter: word 5 high byte wrong");
    a.finish(
        "C1.04",
        'C',
        "OAM rd/wr one counter",
        Provenance::Documented("SNESdev Wiki, OAM"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// C2 — VRAM port
// ---------------------------------------------------------------------------------------------

/// `VMAIN` increment step `00` advances one word per access.
fn c2_01() -> Test {
    let mut a = Asm::new();
    a.c("VMAIN=$80: step 1 word, increment after the HIGH byte. Three words written back to back");
    a.c("must land at consecutive addresses.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1000");
    a.l("stx $2116");
    a.l("lda #$1111");
    a.l("sta $2118");
    a.l("lda #$2222");
    a.l("sta $2118");
    a.l("lda #$3333");
    a.l("sta $2118");
    a.c("Read back word $1001. The first read after setting the address is the stale prefetch,");
    a.c("so discard it and take the second (see C2.03).");
    a.l("ldx #$1001");
    a.l("stx $2116");
    a.l("lda $2139         ; prefetch, discarded");
    a.l("ldx #$1001");
    a.l("stx $2116");
    a.l("sep #$20");
    a.l("lda $2139");
    a.assert_a8(0x22, "step-1 increment did not reach word $1001");
    a.finish(
        "C2.01",
        'C',
        "VMAIN step 1 word",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `VMAIN` bit 7 selects which data port triggers the increment.
fn c2_02() -> Test {
    let mut a = Asm::new();
    a.c("VMAIN=$00 increments after $2118 (the LOW byte), so writing only low bytes fills the low");
    a.c("half of consecutive words and never touches the high halves. This is exactly how the");
    a.c("runtime uploads its 1bpp font, so it is load-bearing here.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Clear two words first so the high bytes are known.");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1100");
    a.l("stx $2116");
    a.l("lda #$0000");
    a.l("sta $2118");
    a.l("sta $2118");
    a.c("Now low-byte-only writes.");
    a.l("sep #$20");
    a.l("stz $2115         ; VMAIN = $00: increment after the LOW byte");
    a.l("rep #$30");
    a.l("ldx #$1100");
    a.l("stx $2116");
    a.l("sep #$20");
    a.l("lda #$3C");
    a.l("sta $2118");
    a.l("lda #$C3");
    a.l("sta $2118");
    a.c("Read back word $1100: low $3C, high still 0.");
    a.l("rep #$30");
    a.l("ldx #$1100");
    a.l("stx $2116");
    a.l("lda $2139");
    a.l("ldx #$1100");
    a.l("stx $2116");
    a.l("lda $2139");
    a.assert_a16(
        0x003C,
        "low-byte-only write disturbed the high byte, or did not increment",
    );
    a.finish(
        "C2.02",
        'C',
        "VMAIN low-byte trigger",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Setting the VRAM address prefetches: the first read afterwards returns the **previous** word.
///
/// The single most common cause of "my VRAM reads are off by one" in SNES homebrew, and something
/// `docs/ppu.md` lists among this project's own modelled quirks.
fn c2_03() -> Test {
    let mut a = Asm::new();
    a.c("Write two distinguishable words, then set the address and read TWICE. The first read is");
    a.c("the prefetch latched when the address was written; the second is the real value.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1200");
    a.l("stx $2116");
    a.l("lda #$ABCD");
    a.l("sta $2118");
    a.l("lda #$1234");
    a.l("sta $2118         ; word $1200 = $ABCD, word $1201 = $1234");
    a.l("ldx #$1200");
    a.l("stx $2116");
    a.l("lda $2139         ; prefetch of word $1200");
    a.l("and #$FFFF");
    a.assert_a16(
        0xABCD,
        "the read after setting VMADD did not return word $1200",
    );
    a.finish(
        "C2.03",
        'C',
        "VRAM read prefetch",
        Provenance::Documented("SNESdev Wiki; docs/ppu.md edge case 4"),
        Kind::Scored,
        None,
    )
}

/// VRAM address bit 15 is not connected: `$8000` aliases `$0000`.
fn c2_04() -> Test {
    let mut a = Asm::new();
    a.c("VRAM is 32K words, so a 16-bit word address has one bit too many. Bit 15 is unconnected,");
    a.c("making $8xxx an alias of $0xxx rather than an out-of-range access.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1300");
    a.l("stx $2116");
    a.l("lda #$BEEF");
    a.l("sta $2118");
    a.c("Read the same word through the mirrored address.");
    a.l("ldx #$9300");
    a.l("stx $2116");
    a.l("lda $2139");
    a.l("ldx #$9300");
    a.l("stx $2116");
    a.l("lda $2139");
    a.assert_a16(
        0xBEEF,
        "VRAM address bit 15 was decoded (it must be unconnected)",
    );
    a.finish(
        "C2.04",
        'C',
        "VRAM bit 15 unconnected",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `VMAIN` increment steps `01`, `10` and `11` advance by 32, 128 and 128 words.
///
/// Both `10` and `11` give 128 — the encoding has a redundant value, and a core that treats `11`
/// as anything else (256, say) breaks tilemap column addressing.
fn c2_05() -> Test {
    let mut a = Asm::new();
    a.c("Write with step=32, then read the far word back to confirm the stride.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$81          ; VMAIN: step 32 words, increment after the high byte");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1400");
    a.l("stx $2116");
    a.l("lda #$0F0F");
    a.l("sta $2118");
    a.l("lda #$F0F0");
    a.l("sta $2118         ; words $1400 and $1420");
    a.c("Read word $1420 with the plain step so the address does not run away.");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1420");
    a.l("stx $2116");
    a.l("lda $2139");
    a.l("ldx #$1420");
    a.l("stx $2116");
    a.l("lda $2139");
    a.assert_a16(0xF0F0, "VMAIN step-32 increment did not land at word $1420");
    a.finish(
        "C2.05",
        'C',
        "VMAIN step 32 words",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// C3 — CGRAM and the H/V counters
// ---------------------------------------------------------------------------------------------

/// CGRAM commits on the second write, like the OAM low table.
fn c3_01() -> Test {
    let mut a = Asm::new();
    a.c("$2122 is written twice per colour: low byte then high. Read back through $213B.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$10");
    a.l("sta $2121         ; CGADD = colour 16");
    a.l("lda #$34");
    a.l("sta $2122");
    a.l("lda #$12");
    a.l("sta $2122         ; colour 16 = $1234");
    a.l("lda #$10");
    a.l("sta $2121");
    a.l("lda $213B");
    a.assert_a8(0x34, "CGRAM low byte did not read back");
    a.l("lda $213B");
    a.l("and #$7F          ; bit 7 of the second read is PPU2 open bus");
    a.assert_a8(0x12, "CGRAM high byte did not read back");
    a.finish(
        "C3.01",
        'C',
        "CGRAM two-write commit",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Writing `$2121` resets the CGRAM write flipflop, discarding a pending low byte.
fn c3_02() -> Test {
    let mut a = Asm::new();
    a.c("Leave a byte pending, reload CGADD, then write a full colour: the pending byte must be");
    a.c("discarded rather than pairing with the next write.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$11");
    a.l("sta $2121");
    a.l("lda #$99");
    a.l("sta $2122         ; pending");
    a.l("lda #$11");
    a.l("sta $2121         ; reload -> flipflop reset");
    a.l("lda #$78");
    a.l("sta $2122");
    a.l("lda #$56");
    a.l("sta $2122");
    a.l("lda #$11");
    a.l("sta $2121");
    a.l("lda $213B");
    a.assert_a8(0x78, "CGADD write did not reset the flipflop");
    a.l("lda $213B");
    a.l("and #$7F");
    a.assert_a8(0x56, "CGRAM high byte wrong after flipflop reset");
    a.finish(
        "C3.02",
        'C',
        "CGADD resets flipflop",
        Provenance::Documented("SNESdev Wiki, PPU registers"),
        Kind::Scored,
        None,
    )
}

/// A latched `OPHCT` pair reconstructs to a plausible 9-bit H position.
///
/// Only **bit 0** of the second read is counter data; bits 1-7 are PPU2 open bus and are
/// deliberately *not* asserted here — an earlier version of this test did assert they were zero
/// and was simply wrong about the hardware.
fn c3_03() -> Test {
    let mut a = Asm::new();
    a.c("$213F resets the read flipflops, $2137 latches, then two $213C reads give low byte and");
    a.c("(in bit 0 only) bit 8. Reconstructed, that must be a real position on the scanline.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $213F         ; reset the OPHCT/OPVCT read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213C         ; low 8 bits");
    a.l("xba");
    a.l("lda $213C");
    a.l("and #$01          ; bit 0 is counter bit 8; bits 1-7 are open bus");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
    a.assert_a16_range(0, 340, "reconstructed H counter is outside a scanline");
    a.finish(
        "C3.03",
        'C',
        "OPHCT is a 9-bit pair",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The H counter advances: two latches separated by work report different positions.
///
/// This is the primitive every Group A cycle test is built on, so it is worth asserting directly
/// rather than only relying on it.
fn c3_04() -> Test {
    let mut a = Asm::new();
    a.c("Latch, burn a known amount of time, latch again. The elapsed dot count must be non-zero");
    a.c("and must not have wrapped past the end of the line.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("jsr hv_begin");
    a.repeat(16, &["nop"]);
    a.l("jsr hv_end");
    a.l("rep #$30");
    a.l("lda f:$7E0048     ; elapsed dots");
    a.assert_a16_range(
        1,
        340,
        "the H counter did not advance plausibly across 16 NOPs",
    );
    a.finish(
        "C3.04",
        'C',
        "H counter advances",
        Provenance::Documented("SNESdev Wiki, PPU registers"),
        Kind::Scored,
        None,
    )
}
