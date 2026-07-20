//! Group C — the S-PPU1 / S-PPU2.
//!
//! Per `docs/accuracysnes-research-dossier.md` §5.C.
//!
//! What is here is bounded by one constraint: the cart scores itself out of RAM and never looks at
//! the framebuffer, because that is what lets the identical image run on other emulators and on
//! real hardware. So Group C is built out of the PPU behaviour that is *observable through a
//! register read* — the OAM/VRAM/CGRAM port mechanics and the H/V counters (C1-C3), the two
//! open-bus latches and the version nibbles (C13, C14), the Mode 7 hardware multiply (C11.06), and
//! the sprite over-flags (C7).
//!
//! Most of the rest of Group C — backgrounds and modes, offset-per-tile, colour math and windows,
//! hi-res, mosaic, direct colour — decides what appears on screen and nothing else, so asserting it
//! needs a framebuffer oracle. That is a separate design decision, not something to smuggle in
//! here.
//!
//! Nearly everything runs under **forced blank** (the runtime keeps `INIDISP = $8F` for the
//! duration of the battery), which is exactly when VRAM, OAM, and CGRAM are architecturally
//! accessible. The C7 sprite tests are the exception: over-flags are produced by OAM evaluation,
//! which only happens while the PPU renders, so they release forced blank, render one complete
//! frame, and restore it. The access-during-render cases are a separate, later batch — they need
//! the renderer running *and* are among the most contested behaviours in the whole corpus.

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
        c1_05(),
        // --- C2: VRAM port ---
        c2_01(),
        c2_02(),
        c2_03(),
        c2_04(),
        c2_05(),
        c2_06(),
        // --- C3: CGRAM and the H/V counters ---
        c3_01(),
        c3_02(),
        c3_03(),
        c3_04(),
        c3_05(),
        // --- C13: open bus ---
        c13_01(),
        c13_02(),
        c13_03(),
        // --- C14: version detection (golden) ---
        c14_01(),
        c14_02(),
        // --- C11: Mode 7 hardware multiply ---
        c11_06(),
        c11_06b(),
        // --- C7: sprite evaluation flags (the only Group C tests that render) ---
        c7_01(),
        c7_02(),
        c7_08(),
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

/// OAM is 544 bytes behind a 1024-byte address space: the high table repeats every 32 bytes.
///
/// Word address `$110` is byte `$220`, which the hardware decodes as byte `$200` — the first byte
/// of the high table. A core that allocates a flat 1024-byte OAM array and indexes it directly
/// passes every other OAM test here and fails this one.
fn c1_05() -> Test {
    let mut a = Asm::new();
    a.c("Write through the mirror at word $110 and read back at the real address, word $100.");
    a.c("The high table commits per byte, so no write-twice pairing is involved.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$10");
    a.l("sta $2102");
    a.l("lda #$01          ; OAMADDR = word $110 (byte $220), bit 7 clear");
    a.l("sta $2103");
    a.l("lda #$5C");
    a.l("sta $2104");
    a.l("lda #$C5");
    a.l("sta $2104");
    a.c("--- read the real high-table bytes ---");
    a.l("lda #$00");
    a.l("sta $2102");
    a.l("lda #$01          ; OAMADDR = word $100 (byte $200)");
    a.l("sta $2103");
    a.l("lda $2138");
    a.assert_a8(0x5C, "OAM high table did not mirror: byte $220 -> $200");
    a.l("lda $2138");
    a.assert_a8(0xC5, "OAM high table did not mirror: byte $221 -> $201");
    a.finish(
        "C1.05",
        'C',
        "OAM high table mirror",
        Provenance::Documented("SNESdev Wiki, OAM; fullsnes"),
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

/// `VMAIN` address translation rewrites the address **on the bus**, not the address **register**.
///
/// This is the distinction that makes the feature usable: the register still increments linearly,
/// so consecutive writes walk consecutive registers while landing on the rotated VRAM words. A
/// core that folds the rotation back into the register produces the right first word and then
/// diverges — which is why the second half of this test matters more than the first.
///
/// Remap `01` is the 8-bit rotation, `aaaaaaaa YYYxxxxx -> aaaaaaaa xxxxxYYY`. Register `$1503`
/// therefore drives bus word `$1518`, and register `$1504` drives `$1520` — *not* `$1519`.
fn c2_06() -> Test {
    let mut a = Asm::new();
    a.c(
        "Two back-to-back writes with remap 01 active, then read both target words with remap off.",
    );
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$84          ; VMAIN: remap 01 (8-bit), step 1, increment after the high byte");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1503");
    a.l("stx $2116");
    a.l("lda #$CAFE");
    a.l("sta $2118         ; register $1503 -> bus word $1518");
    a.l("lda #$B0BA");
    a.l("sta $2118         ; register $1504 -> bus word $1520");
    a.c("--- read both back with translation off ---");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115");
    a.l("rep #$30");
    a.l("ldx #$1518");
    a.l("stx $2116");
    a.l("lda $2139");
    a.assert_a16(
        0xCAFE,
        "remap 01 did not translate register $1503 to bus word $1518",
    );
    a.l("ldx #$1520");
    a.l("stx $2116");
    a.l("lda $2139");
    a.assert_a16(
        0xB0BA,
        "the remap fed back into the address register (the second write missed word $1520)",
    );
    a.finish(
        "C2.06",
        'C',
        "VMAIN remap hits bus",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes; anomie"),
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

/// Reading `$213F` resets the `OPHCT` read flipflop, so the next read is the low byte again.
///
/// The latched counter value itself is frozen until the next `$2137` latch, which is what makes
/// this assertable without any timing tolerance: the first and third reads must be **byte
/// identical**, not merely close. A core that keeps one shared flipflop, or that resets it on
/// `$2137` instead of `$213F`, returns the high byte on the third read and fails.
fn c3_05() -> Test {
    let mut a = Asm::new();
    a.c("Latch once, then read low / high / reset / low. Nothing re-latches in between, so the");
    a.c("two low reads sample the same frozen value and any difference is a flipflop bug.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("cld               ; the SBC below must not run in decimal mode");
    a.l("sep #$20");
    a.l("lda $213F         ; reset both read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213C         ; H low");
    a.l("sta f:$7E0100");
    a.l("lda $213C         ; H high — the flipflop is now set");
    a.l("lda $213F         ; reset both flipflops again");
    a.l("lda $213C         ; must be H low once more");
    a.l("sec");
    a.l("sbc f:$7E0100");
    a.assert_a8(
        0x00,
        "$213F did not reset the OPHCT flipflop (the third read was not the low byte)",
    );
    a.finish(
        "C3.05",
        'C',
        "$213F resets flipflop",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// C13 — open bus
// ---------------------------------------------------------------------------------------------

/// `$213E` bit 4 reads back the **PPU1** open-bus latch.
///
/// The PPU drives only the bits it decodes; the rest of the byte comes from whatever the chip last
/// put on its half of the bus. Driving that latch to a known value through an OAM read makes an
/// otherwise invisible piece of state directly assertable.
fn c13_01() -> Test {
    let mut a = Asm::new();
    a.c("Drive PPU1 open bus to $10 via an OAM read, check $213E bit 4, then drive it to $00 and");
    a.c("check again. Only bit 4 is examined: bits 7-6 are the sprite flags and 5-0 the version.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("--- open bus := $10 ---");
    a.l("lda #$08");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda #$10");
    a.l("sta $2104");
    a.l("lda #$00");
    a.l("sta $2104");
    a.l("lda #$08");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda $2138         ; returns $10 and refreshes PPU1 open bus with it");
    a.l("lda $213E");
    a.l("and #$10");
    a.assert_a8(0x10, "$213E bit 4 did not follow PPU1 open bus set to $10");
    a.c("--- open bus := $00 ---");
    a.l("lda #$08");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda #$00");
    a.l("sta $2104");
    a.l("lda #$00");
    a.l("sta $2104");
    a.l("lda #$08");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda $2138         ; returns $00");
    a.l("lda $213E");
    a.l("and #$10");
    a.assert_a8(
        0x00,
        "$213E bit 4 did not follow PPU1 open bus cleared to $00",
    );
    a.finish(
        "C13.01",
        'C',
        "PPU1 open bus in $213E",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$213F` bit 5 reads back the **PPU2** open-bus latch.
fn c13_02() -> Test {
    let mut a = Asm::new();
    a.c("Same shape as C13.01 but on the other chip: CGRAM reads go through PPU2, so a $213B read");
    a.c("is what refreshes this latch.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("--- open bus := $20 ---");
    a.l("lda #$20");
    a.l("sta $2121");
    a.l("lda #$20");
    a.l("sta $2122");
    a.l("lda #$00");
    a.l("sta $2122");
    a.l("lda #$20");
    a.l("sta $2121");
    a.l("lda $213B         ; returns $20 and refreshes PPU2 open bus with it");
    a.l("lda $213F");
    a.l("and #$20");
    a.assert_a8(0x20, "$213F bit 5 did not follow PPU2 open bus set to $20");
    a.c("--- open bus := $00 ---");
    a.l("lda #$20");
    a.l("sta $2121");
    a.l("lda #$00");
    a.l("sta $2122");
    a.l("lda #$00");
    a.l("sta $2122");
    a.l("lda #$20");
    a.l("sta $2121");
    a.l("lda $213B         ; returns $00");
    a.l("lda $213F");
    a.l("and #$20");
    a.assert_a8(
        0x00,
        "$213F bit 5 did not follow PPU2 open bus cleared to $00",
    );
    a.finish(
        "C13.02",
        'C',
        "PPU2 open bus in $213F",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// PPU1 and PPU2 keep **separate** open-bus latches.
///
/// They are two physically distinct chips on two halves of the bus, so refreshing one must leave
/// the other alone. A core with a single shared `open_bus` byte — the natural first implementation
/// — passes C13.01 and C13.02 individually and fails here, which is exactly why this is its own
/// test rather than an extra assertion on either of them.
fn c13_03() -> Test {
    let mut a = Asm::new();
    a.c("Drive the two latches to opposite values and read both back. Then swap and repeat, so a");
    a.c("shared latch cannot pass by accident in one polarity.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("--- seed OAM byte $10 / $00 and CGRAM low byte $20 / $00 ---");
    a.l("lda #$08");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda #$10");
    a.l("sta $2104");
    a.l("lda #$00");
    a.l("sta $2104         ; OAM word 8 = $0010");
    a.c("Word 9 must be seeded too, not assumed zero: it is read below to drive PPU1 open bus to");
    a.c("$00, and whatever the previous tests or the power-on fill left there would otherwise");
    a.c("decide the result. Mesen2 and snes9x disagree on that leftover, which is not a hardware");
    a.c("difference — it is this test failing to control its own inputs.");
    a.l("lda #$00");
    a.l("sta $2104");
    a.l("lda #$00");
    a.l("sta $2104         ; OAM word 9 = $0000");
    a.l("lda #$20");
    a.l("sta $2121");
    a.l("lda #$20");
    a.l("sta $2122");
    a.l("lda #$00");
    a.l("sta $2122         ; colour $20 = $0020");
    a.l("lda #$00");
    a.l("sta $2122");
    a.l("lda #$00");
    a.l("sta $2122         ; colour $21 = $0000, for the same reason as OAM word 9");
    a.c("--- PPU1 := $10, PPU2 := $00 ---");
    a.l("lda #$08");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda $2138         ; PPU1 open bus := $10");
    a.l("lda #$21");
    a.l("sta $2121");
    a.l("lda $213B         ; colour $21 low byte = $00; PPU2 open bus := $00");
    a.l("lda $213E");
    a.l("and #$10");
    a.assert_a8(0x10, "refreshing PPU2 open bus clobbered PPU1's latch");
    a.l("lda $213F");
    a.l("and #$20");
    a.assert_a8(0x00, "PPU2 open bus read back as PPU1's value");
    a.c("--- PPU1 := $00, PPU2 := $20 ---");
    a.l("lda #$09");
    a.l("sta $2102");
    a.l("stz $2103");
    a.l("lda $2138         ; OAM word 9 is zero; PPU1 open bus := $00");
    a.l("lda #$20");
    a.l("sta $2121");
    a.l("lda $213B         ; PPU2 open bus := $20");
    a.l("lda $213E");
    a.l("and #$10");
    a.assert_a8(0x00, "PPU1 open bus read back as PPU2's value");
    a.l("lda $213F");
    a.l("and #$20");
    a.assert_a8(0x20, "refreshing PPU1 open bus clobbered PPU2's latch");
    a.finish(
        "C13.03",
        'C',
        "PPU1/PPU2 bus separate",
        Provenance::Corroborated("the bsnes/ares lineage and Mesen2 model two distinct latches"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// C14 — version detection (golden vectors)
// ---------------------------------------------------------------------------------------------

/// The PPU1 version nibble in `$213E`, recorded rather than asserted.
///
/// Only version 1 has ever been observed in the wild, but the value is a property of the *console*
/// a cartridge happens to be in, not of the SNES architecture. Asserting it would make the battery
/// fail on a hypothetically-correct emulation of a machine we have not seen, so it is recorded as
/// a variant code and kept out of the pass rate.
fn c14_01() -> Test {
    let mut a = Asm::new();
    a.c("Report the low nibble of $213E as the variant code.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $213E");
    a.l("and #$0F          ; PPU1 version");
    a.l("asl a");
    a.l("ora #$01          ; encode as (version << 1) | 1");
    a.l("sta f:$7EE010");
    a.l("jmp test_restore");
    a.finish(
        "C14.01",
        'C',
        "PPU1 version (golden)",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Golden,
        None,
    )
}

/// The PPU2 version nibble in `$213F`, recorded rather than asserted.
///
/// Unlike PPU1 this genuinely varies — versions 1, 2 and 3 all shipped — and it *gates* the
/// `$2100` early-read object-corruption bug, which only reproduces on 3-chip consoles. Any future
/// test for that bug has to read this value first rather than assume a revision.
fn c14_02() -> Test {
    let mut a = Asm::new();
    a.c("Report the low nibble of $213F as the variant code.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda $213F");
    a.l("and #$0F          ; PPU2 version");
    a.l("asl a");
    a.l("ora #$01");
    a.l("sta f:$7EE010");
    a.l("jmp test_restore");
    a.finish(
        "C14.02",
        'C',
        "PPU2 version (golden)",
        Provenance::Documented("SNESdev Wiki, PPU registers; fullsnes"),
        Kind::Golden,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// C11 — Mode 7 hardware multiply
// ---------------------------------------------------------------------------------------------

/// `MPYL/M/H` is `signed16(M7A) * signed8(M7B >> 8)`, as a 24-bit signed product.
///
/// The multiplier is the one piece of the Mode 7 datapath that is directly readable, which makes
/// it the only part of C11 a self-scoring cartridge can assert without a framebuffer oracle. It is
/// also load-bearing well outside Mode 7: games use it as a general-purpose 16x8 signed multiply.
///
/// Both operand registers are written twice through the shared Mode 7 latch, and the multiplicand
/// is the **high byte** of `M7B` — not the whole word, and not the low byte.
fn c11_06() -> Test {
    let mut a = Asm::new();
    a.c("Positive case: M7A = $0100 (256), M7B high byte = $02, so the product is 512 = $000200.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $211B         ; M7A low");
    a.l("lda #$01");
    a.l("sta $211B         ; M7A high -> M7A = $0100");
    a.l("lda #$00");
    a.l("sta $211C         ; M7B low (ignored by the multiply)");
    a.l("lda #$02");
    a.l("sta $211C         ; M7B high -> multiplicand = +2");
    a.l("lda $2134");
    a.assert_a8(0x00, "MPYL wrong for 256 * 2");
    a.l("lda $2135");
    a.assert_a8(0x02, "MPYM wrong for 256 * 2");
    a.l("lda $2136");
    a.assert_a8(0x00, "MPYH wrong for 256 * 2");
    a.c("The low byte of M7B must not participate: rewriting it alone cannot change the product.");
    a.l("lda #$FF");
    a.l("sta $211C         ; M7B low = $FF");
    a.l("lda #$02");
    a.l("sta $211C         ; M7B high still $02");
    a.l("lda $2135");
    a.assert_a8(0x02, "the low byte of M7B leaked into the multiply");
    a.finish(
        "C11.06",
        'C',
        "MPY is 16x8 signed",
        Provenance::Documented("SNESdev Wiki, Mode 7; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The Mode 7 multiply is signed on **both** operands, and the product sign-extends to 24 bits.
///
/// Split from C11.06 so an unsigned or half-signed implementation reports a distinct failure code
/// rather than hiding behind the positive case.
fn c11_06b() -> Test {
    let mut a = Asm::new();
    a.c("Negative multiplicand: M7A = $0002, M7B high = $FF (-1) -> -2 = $FFFFFE.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$02");
    a.l("sta $211B");
    a.l("lda #$00");
    a.l("sta $211B         ; M7A = $0002");
    a.l("lda #$00");
    a.l("sta $211C");
    a.l("lda #$FF");
    a.l("sta $211C         ; multiplicand = -1");
    a.l("lda $2134");
    a.assert_a8(0xFE, "MPYL wrong for 2 * -1 (M7B high must be signed)");
    a.l("lda $2135");
    a.assert_a8(0xFF, "MPYM wrong for 2 * -1");
    a.l("lda $2136");
    a.assert_a8(0xFF, "the product did not sign-extend to 24 bits");
    a.c("Negative multiplier: M7A = $FFFF (-1), M7B high = $02 -> -2 = $FFFFFE.");
    a.l("lda #$FF");
    a.l("sta $211B");
    a.l("lda #$FF");
    a.l("sta $211B         ; M7A = $FFFF");
    a.l("lda #$00");
    a.l("sta $211C");
    a.l("lda #$02");
    a.l("sta $211C");
    a.l("lda $2134");
    a.assert_a8(0xFE, "MPYL wrong for -1 * 2 (M7A must be signed)");
    a.l("lda $2136");
    a.assert_a8(0xFF, "the product did not sign-extend for a negative M7A");
    a.finish(
        "C11.06b",
        'C',
        "MPY sign handling",
        Provenance::Documented("SNESdev Wiki, Mode 7; fullsnes"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// C7 — sprite evaluation flags
//
// These are the only Group C tests that release forced blank. The over-flags are produced by OAM
// range/sliver evaluation, which only runs while the PPU is actually rendering, so unlike every
// other test here they need a real frame to happen. They still score off a register read, never
// off the framebuffer.
// ---------------------------------------------------------------------------------------------

/// Emit a full OAM setup and render exactly one frame, leaving `$213E` in the 8-bit accumulator.
///
/// `on_line` sprites are parked at Y=100; every other sprite goes to Y=$F0, which is below the
/// visible area in 224-line mode and therefore never enters range. Leaving the rest of OAM at
/// whatever the previous tests wrote would make the range count depend on test order.
///
/// The two `wait_vblank` calls are load-bearing: the first lands on a vblank boundary, the second
/// spans one complete active period. A single call would start mid-frame and evaluate only the
/// scanlines that happened to remain, which is exactly the kind of non-determinism that makes a
/// timing-dependent test flake.
/// `tag` disambiguates the emitted cheap-local labels. ca65 resets cheap-local scope only at a
/// non-cheap label, and a test that renders twice has none in between, so the second expansion
/// would redefine the first's labels.
fn setup_and_render(
    a: &mut Asm,
    tag: &str,
    obsel: u8,
    on_line: u16,
    high_table: Option<(u8, u8)>,
    obj_on_main: bool,
) {
    a.l("sep #$20");
    a.l(&format!("lda #${obsel:02X}"));
    a.l("sta $2101         ; OBJSEL: size pair in bits 7-5, name base in bits 1-0");
    a.c("--- low table: `on_line` sprites on one scanline, the rest parked off-screen ---");
    a.l("stz $2102");
    a.l("stz $2103");
    a.l("rep #$10");
    a.l("ldx #$0000");
    a.label(&format!("fill_{tag}"));
    a.l("lda #$00");
    a.l("sta $2104         ; X = 0");
    a.l(&format!("cpx #${on_line:04X}"));
    a.l(&format!("bcs @off_{tag}"));
    a.l("lda #100");
    a.l(&format!("bra @sety_{tag}"));
    a.label(&format!("off_{tag}"));
    a.l("lda #$F0          ; below the visible area in 224-line mode");
    a.label(&format!("sety_{tag}"));
    a.l("sta $2104         ; Y");
    a.l("lda #$00");
    a.l("sta $2104         ; tile");
    a.l("lda #$00");
    a.l("sta $2104         ; attr");
    a.l("inx");
    a.l("cpx #$0080");
    a.l(&format!("bne @fill_{tag}"));
    a.c("--- high table: 32 bytes, 2 bits per sprite (bit 0 = X bit 8, bit 1 = size select) ---");
    a.l("lda #$00");
    a.l("sta $2102");
    a.l("lda #$01");
    a.l("sta $2103         ; OAMADDR = word $100, the high table");
    a.l("ldx #$0000");
    a.label(&format!("hi_{tag}"));
    a.l("lda #$00");
    a.l("sta $2104");
    a.l("inx");
    a.l("cpx #$0020");
    a.l(&format!("bne @hi_{tag}"));
    if let Some((b0, b1)) = high_table {
        a.c("Mark the leading sprites as the large size of the pair.");
        a.l("lda #$00");
        a.l("sta $2102");
        a.l("lda #$01");
        a.l("sta $2103");
        a.l(&format!("lda #${b0:02X}"));
        a.l("sta $2104");
        a.l(&format!("lda #${b1:02X}"));
        a.l("sta $2104");
    }
    a.c("--- render one complete frame, then sample and restore forced blank ---");
    if obj_on_main {
        a.l("lda #$10");
        a.l("sta $212C         ; OBJ on the main screen");
    } else {
        a.l("stz $212C         ; deliberately leave OBJ OFF the main screen");
    }
    a.l("lda #$0F");
    a.l("sta $2100         ; brightness 15, forced blank released");
    a.l("jsr wait_vblank   ; land on a vblank boundary");
    a.l("jsr wait_vblank   ; span one complete active period");
    a.l("lda $213E");
    a.l("pha");
    a.l("lda #$8F");
    a.l("sta $2100         ; forced blank again, as the rest of the battery expects");
    a.l("stz $212C");
    a.l("pla");
}

/// Range Over (`$213E` bit 6) sets when more than 32 sprites fall on one scanline, and only then.
///
/// The negative half matters as much as the positive one: a core that simply never clears the flag
/// passes the 40-sprite case and fails the 2-sprite case.
fn c7_01() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("--- 2 sprites on the line: well under the limit, flag must stay clear ---");
    setup_and_render(&mut a, "a", 0x00, 2, None, true);
    a.l("and #$40");
    a.assert_a8(0x00, "Range Over set with only 2 sprites on the scanline");
    a.c("--- 40 sprites on the line: over the 32-sprite limit ---");
    a.l("rep #$30");
    setup_and_render(&mut a, "b", 0x00, 40, None, true);
    a.l("and #$40");
    a.assert_a8(
        0x40,
        "Range Over did not set with 40 sprites on one scanline",
    );
    a.finish(
        "C7.01",
        'C',
        "Range Over at 32 sprites",
        Provenance::Documented("SNESdev Wiki, Sprites; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Time Over (`$213E` bit 7) is a **sliver** budget, not a sprite count.
///
/// Five 64-pixel-wide sprites are 40 slivers — over the 34-sliver budget — while being nowhere
/// near the 32-sprite range limit. A core that drives Time Over off the sprite count instead of
/// the 8-pixel-column count sees five sprites, sets nothing, and fails here while passing C7.01.
fn c7_02() -> Test {
    let mut a = Asm::new();
    a.c("OBJSEL size select lives in bits 7-5, not the low bits: mode 2 ($40) pairs 8x8 with");
    a.c("64x64, and the high table marks sprites 0-4 as the large member of that pair. Writing");
    a.c("the mode number into the low bits instead sets the tile-name base and silently leaves");
    a.c("the size pair at 8x8/16x16 — 10 slivers, comfortably inside the budget, no flag.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    setup_and_render(&mut a, "a", 0x40, 5, Some((0xAA, 0x02)), true);
    a.l("pha");
    a.l("and #$80");
    a.assert_a8(0x80, "Time Over did not set for 40 slivers from 5 sprites");
    a.l("pla");
    a.l("and #$40");
    a.assert_a8(
        0x00,
        "Range Over set for only 5 sprites (it is a sprite count, not a sliver count)",
    );
    a.finish(
        "C7.02",
        'C',
        "Time Over is slivers",
        Provenance::Documented("SNESdev Wiki, Sprites; fullsnes; anomie"),
        Kind::Scored,
        None,
    )
}

/// The over-flags come from OAM evaluation, which runs whether or not OBJ is on a screen.
///
/// `$212C` bit 4 gates *compositing*, not *evaluation* — the sprite pipeline still walks OAM and
/// still exhausts its budgets. A core that skips evaluation when the layer is disabled reports
/// clean flags for a frame that would have overflowed, which is a plausible optimisation and the
/// reason this is a separate test.
fn c7_08() -> Test {
    let mut a = Asm::new();
    a.c("Same 40-sprite overflow as C7.01, but with OBJ left off the main screen entirely.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    setup_and_render(&mut a, "a", 0x00, 40, None, false);
    a.l("and #$40");
    a.assert_a8(
        0x40,
        "Range Over did not set while OBJ was off the main screen ($212C gates compositing, \
         not evaluation)",
    );
    a.finish(
        "C7.08",
        'C',
        "Flags ignore $212C",
        Provenance::Documented("SNESdev Wiki, Sprites; fullsnes"),
        Kind::Scored,
        None,
    )
}
