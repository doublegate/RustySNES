//! Group D — DMA and HDMA (ticket **T-04-D**).
//!
//! The general-purpose DMA controller is unusually pleasant to test on-cart: it moves bytes into
//! memory the CPU can read back, so most of its behaviour is directly self-scoring with no
//! measurement and no host cooperation. That is why this group leads with the transfer modes,
//! the address-step options and the register semantics rather than with timing.
//!
//! Every test here sources its bytes from a `.byte` table emitted at the TOP of its own proc and
//! jumped over (see [`data_table`] for why the top and not the end). Sourcing from an arbitrary
//! ROM address instead would make the expected values depend on whatever the linker happened to
//! place there — a test that breaks whenever an unrelated test is added.

use crate::dsl::{Asm, Kind, Provenance, Test};
use crate::tests::bus::{measure_frame_height, read_v};

/// Every Group D test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        d1_01_mode0(),
        d1_01_mode1(),
        d1_06(),
        d1_07_fixed(),
        d1_07_decrement(),
        d1_10(),
        d1_02(),
        d1_05(),
        d1_09(),
        d2_03(),
        d2_04(),
    ]
}

/// Point channel 0 at the test's own data table and set the byte count.
///
/// Factored out because six of these tests differ only in the destination and the address-step
/// bits, and repeating the setup six times is how the sixth copy ends up subtly different from
/// the other five.
///
/// **Register widths on exit: `A` 8-bit, `X`/`Y` 16-bit.** Stated because the caller's
/// `.a8`/`.a16` directives are emitted from its own `sep`/`rep` lines and a helper call is not one
/// of those — an undocumented width change here would have the assembler and the CPU disagreeing
/// about the size of the next immediate.
fn source_from_table(a: &mut Asm, count: u16) {
    a.l("rep #$30");
    a.l("ldx #@data");
    a.l("stx $4302         ; A-bus address = this test's data table");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304         ; A-bus bank = this bank");
    a.l("rep #$30");
    a.l(&format!("ldx #${count:04X}"));
    a.l("stx $4305         ; byte count");
    a.l("sep #$20");
}

/// Emit the four-byte source table at the TOP of a test, jumped over.
///
/// At the top rather than after the body, because `Asm::finish` appends the pass epilogue — the
/// code that actually records `VERDICT_PASS` — after everything the test emits. A data table
/// ending in `jmp test_restore` therefore jumps *over* that epilogue, and the test reports NOT RUN
/// however well it passed. Three of these did.
fn data_table(a: &mut Asm) {
    a.l("bra @body");
    a.label("data");
    a.l(".byte $11, $22, $33, $44");
    a.label("body");
}

/// Transfer mode 0 writes every byte to the SAME destination register.
///
/// The mode field is three bits and the modes differ only in how many B-bus registers a transfer
/// walks, so a core that treats mode 0 as mode 1 still writes the right bytes — just to the wrong
/// places. Reading them back out of WRAM is what distinguishes the two.
fn d1_01_mode0() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("Mode 0 into $2180 (the WRAM data port): all four bytes land at consecutive WRAM");
    a.c("addresses because $2180 auto-increments WMADD, not because the B-bus address moved.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$05");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0500, clear of every test's scratch");
    a.l("stz $4300         ; DMAP: A->B, increment, mode 0");
    a.l("lda #$80");
    a.l("sta $4301         ; B-bus = $2180");
    source_from_table(&mut a, 4);
    a.l("lda #$01");
    a.l("sta $420B         ; run channel 0");
    a.c("Read the four bytes back and fold them into one word so a single compare covers all.");
    a.l("rep #$30");
    a.l("lda f:$7E0500");
    a.assert_a16(
        0x2211,
        "the first two bytes did not arrive in order at $7E:0500",
    );
    a.l("lda f:$7E0502");
    a.assert_a16(
        0x4433,
        "the last two bytes did not arrive in order at $7E:0502",
    );
    a.finish(
        "D1.01",
        'D',
        "DMA mode 0",
        Provenance::Documented("SNESdev Wiki, DMA; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Transfer mode 1 alternates between two B-bus registers.
///
/// The companion to mode 0, and the pair is what makes either meaningful: mode 1 into
/// `$2118`/`$2119` writes VRAM words, so reading the words back proves the low and high halves
/// went to different registers rather than both to the same one.
fn d1_01_mode1() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("Mode 1 into $2118/$2119: byte 0 -> VMDATAL, byte 1 -> VMDATAH, then repeat.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115         ; VMAIN: step 1 word, increment after the high byte");
    a.l("rep #$30");
    a.l("ldx #$1000");
    a.l("stx $2116         ; VRAM word address $1000, clear of the font and the tilemaps");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $4300         ; DMAP: A->B, increment, mode 1");
    a.l("lda #$18");
    a.l("sta $4301         ; B-bus = $2118");
    source_from_table(&mut a, 4);
    a.l("lda #$01");
    a.l("sta $420B");
    a.c("Read the two words back. Setting VMADDL primes the read latch, so the first 16-bit read");
    a.c("of $2139/$213A already returns word $1000 — no dummy read, which would consume it.");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2115         ; increment after the HIGH byte, so a 16-bit read is one word");
    a.l("rep #$30");
    a.l("ldx #$1000");
    a.l("stx $2116");
    a.l("lda $2139");
    a.assert_a16(
        0x2211,
        "VRAM word 0 is wrong: the two bytes did not split across $2118/$2119",
    );
    a.c("Re-set the address for the second word rather than relying on the read increment: the");
    a.c("prefetch latch makes successive reads a separate question, and mixing the two would");
    a.c("leave a failure ambiguous between the DMA and the read port.");
    a.l("ldx #$1001");
    a.l("stx $2116");
    a.l("lda $2139");
    a.assert_a16(0x4433, "VRAM word 1 is wrong");
    a.finish(
        "D1.01b",
        'D',
        "DMA mode 1",
        Provenance::Documented("SNESdev Wiki, DMA; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The byte counter reaches zero when the transfer ends.
///
/// `$4305/06` is a live counter, not a stored length: it counts down as the transfer runs and
/// reads back as `$0000` afterwards. A core that keeps the programmed length there instead looks
/// identical until a game reads it — which several do, to tell a finished transfer from an
/// interrupted one.
fn d1_06() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("Run a 4-byte transfer, then read $4305/06.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$06");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0600");
    a.l("stz $4300");
    a.l("lda #$80");
    a.l("sta $4301");
    source_from_table(&mut a, 4);
    a.l("lda #$01");
    a.l("sta $420B");
    a.l("rep #$30");
    a.l("lda $4305");
    a.assert_a16(0, "the DMA byte counter did not decrement to zero");
    a.finish(
        "D1.06",
        'D',
        "DMA count hits zero",
        Provenance::Documented("SNESdev Wiki, DMA registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A fixed A-bus address re-reads the same byte for the whole transfer.
///
/// `$4300` bits 4-3 are a two-bit FIELD, not two independent flags: 0 = increment, 1 = fixed,
/// 2 = decrement, 3 = fixed again. Reading them as flags gets fixed and decrement exactly
/// backwards — which is the mistake this test exists to catch, and which its own first version
/// made. Asserting all four bytes are equal, rather than only the first, is what separates "fixed"
/// from "incremented through a table that happens to start with the right byte".
fn d1_07_fixed() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("DMAP bits 4-3 = 1: fixed source. Four bytes from one address must all be $11.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$07");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0700");
    a.l("lda #$08");
    a.l("sta $4300         ; DMAP: A->B, step field = 1 (FIXED), mode 0");
    a.l("lda #$80");
    a.l("sta $4301");
    source_from_table(&mut a, 4);
    a.l("lda #$01");
    a.l("sta $420B");
    a.l("rep #$30");
    a.l("lda f:$7E0700");
    a.assert_a16(0x1111, "bytes 0-1 are not both the fixed source byte");
    a.l("lda f:$7E0702");
    a.assert_a16(0x1111, "bytes 2-3 are not both the fixed source byte");
    a.finish(
        "D1.07",
        'D',
        "DMA fixed A-bus",
        Provenance::Documented("SNESdev Wiki, DMA registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A decrementing A-bus address walks backwards.
///
/// The mirror of the fixed case, and the reason both exist: the two share one two-bit field, so a
/// core that treats them as independent flags gets exactly one of them wrong and the other right
/// — which either test alone would miss. Sourcing from the END of the table means the bytes
/// arrive reversed.
fn d1_07_decrement() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("DMAP bits 4-3 = 2: decrement. Source the LAST byte; the transfer walks back.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$08");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0800");
    a.l("lda #$10");
    a.l("sta $4300         ; DMAP: A->B, step field = 2 (DECREMENT), mode 0");
    a.l("lda #$80");
    a.l("sta $4301");
    a.l("rep #$30");
    a.l("ldx #(@data + 3)");
    a.l("stx $4302         ; start at the last byte");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304");
    a.l("rep #$30");
    a.l("ldx #$0004");
    a.l("stx $4305");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $420B");
    a.l("rep #$30");
    a.l("lda f:$7E0800");
    a.assert_a16(0x3344, "bytes 0-1 are not the table read backwards");
    a.l("lda f:$7E0802");
    a.assert_a16(0x1122, "bytes 2-3 are not the table read backwards");
    a.finish(
        "D1.07b",
        'D',
        "DMA decrementing A-bus",
        Provenance::Documented("SNESdev Wiki, DMA registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$43xB` is a readable scratch latch, and `$43xF` mirrors it.
///
/// Undocumented, and worth pinning precisely because it is: both ares and bsnes model the latch
/// and serialise it into save states, so a core without it silently breaks state compatibility
/// with them. Nothing in the DMA controller uses the value — which is what makes it safe to
/// assert, since no transfer can perturb it.
fn d1_10() -> Test {
    let mut a = Asm::new();
    a.c("Write $43xB, read it back, then read $43xF and require the same value.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$5A");
    a.l("sta $430B");
    a.l("lda $430B");
    a.assert_a8(0x5A, "$430B did not read back the value written to it");
    a.l("lda $430F");
    a.assert_a8(0x5A, "$430F does not mirror $430B");
    a.c("A different channel must have its own latch, not share channel 0's.");
    a.l("lda #$A5");
    a.l("sta $431B");
    a.l("lda $430B");
    a.assert_a8(
        0x5A,
        "writing $431B changed channel 0's latch — the channels are not separate",
    );
    a.l("lda $431F");
    a.assert_a8(0xA5, "$431F does not mirror $431B");
    a.finish(
        "D1.10",
        'D',
        "DMA $43xB scratch latch",
        Provenance::Corroborated("ares and bsnes both model the latch and serialize it"),
        Kind::Scored,
        None,
    )
}

/// A DMA costs 8 master clocks per byte, measured as a differential.
///
/// Absolute timing would fold in the startup overhead and the channel-start alignment, neither of
/// which this is about. Two transfers whose lengths differ by 32 bytes cancel both, leaving
/// 32 x 8 = 256 clocks = 64 dots — far larger than the few dots of phase jitter the measurement
/// harness carries.
fn d1_02() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("Two mode-0 transfers into WRAM, 32 bytes apart in length. The difference is the rate.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$08");
    a.l("sta $4300         ; step field = 1 (fixed): the table is only 4 bytes long");
    a.l("lda #$80");
    a.l("sta $4301");

    a.c("--- 16 bytes ---");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$09");
    a.l("sta $2182");
    a.l("stz $2183");
    source_from_table(&mut a, 16);
    a.measure_begin();
    a.l("lda #$01");
    a.l("sta $420B");
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E00A0");
    a.record(104, "D1.02 16-byte DMA (dots)");

    a.c("--- 48 bytes: 32 more ---");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$0A");
    a.l("sta $2182");
    a.l("stz $2183");
    source_from_table(&mut a, 48);
    a.measure_begin();
    a.l("lda #$01");
    a.l("sta $420B");
    a.measure_end();
    a.measure_result();
    a.record(105, "D1.02 48-byte DMA (dots)");
    a.l("sec");
    a.l("sbc f:$7E00A0");
    a.record(
        106,
        "D1.02 difference — expect 32 bytes x 8 clocks = 64 dots",
    );
    a.assert_a16_range(
        60,
        68,
        "32 extra DMA bytes did not cost 64 dots (8 clocks each)",
    );
    a.finish(
        "D1.02",
        'D',
        "DMA 8 clocks/byte",
        Provenance::Documented("SNESdev Wiki, DMA timing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A byte count of zero means 65536, not zero.
///
/// `$4305/06` is a decrementing counter, so "how many bytes" is really "how many decrements until
/// it reaches zero" — and starting at zero takes the full 16-bit wrap. Getting this wrong is
/// silent in the common case (games rarely program zero deliberately) and catastrophic when it
/// happens, because the transfer either does nothing or runs 65536 bytes into somewhere.
///
/// Observed through TIME rather than through the destination, which is what makes it safe: 65536
/// bytes at 8 master clocks each is 131072 dots, a little over 384 scanlines. Starting from the
/// top of vblank, the V counter therefore lands around line 85 of the *next* frame. A core that
/// transfers nothing leaves it near where it started. The destination is CGRAM, which the transfer
/// overwrites 512 times over and which the next scene rebuilds anyway.
fn d1_05() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("Wait for the top of vblank so the starting line is known, then run a count-0 transfer.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$08");
    a.l("sta $4300         ; A->B, fixed source, mode 0");
    a.l("lda #$22");
    a.l("sta $4301         ; B-bus = $2122 (CGDATA): harmless, and rebuilt before any scene");
    a.l("stz $2121         ; CGADD = 0");
    a.l("rep #$30");
    a.l("ldx #@data");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.l("stx $4305         ; count = 0, which means 65536");
    a.l("sep #$20");
    a.l("jsr wait_vblank   ; start from a known line");
    a.l("lda #$01");
    a.l("sta $420B");
    read_v(&mut a);
    a.l("sta f:$7E00B0     ; where the transfer left the V counter");
    a.record(110, "D1.05 V counter after a count-0 DMA");
    a.c("The landing line depends on FRAME LENGTH, so measure that rather than assume it: 384");
    a.c("lines past line 225 is line 85 of the next NTSC frame and line 297 of the next PAL one.");
    a.c("Measured, not read from the region bit — whose position B2.10 had to settle, and which a");
    a.c("frame-length test must not lean on.");
    measure_frame_height(&mut a);
    a.c("Named labels, not anonymous ones: assert_a16_range emits its own `:` labels, so a `bne :+`");
    a.c(
        "written across one lands INSIDE the assertion rather than after it. That cost a debugging",
    );
    a.c("round here, with the branch silently taking the wrong arm.");
    a.l("cmp #311");
    a.l("bne @ntsc");
    a.l("lda f:$7E00B0");
    a.assert_a16_range(
        275,
        320,
        "on PAL a count-0 DMA did not take ~384 scanlines, so it did not transfer 65536 bytes",
    );
    a.l("bra @done");
    a.label("ntsc");
    a.l("lda f:$7E00B0");
    a.assert_a16_range(
        60,
        110,
        "on NTSC a count-0 DMA did not take ~384 scanlines, so it did not transfer 65536 bytes",
    );
    a.label("done");
    a.finish(
        "D1.05",
        'D',
        "DMA count 0 = 65536",
        Provenance::Documented("SNESdev Wiki, DMA registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The HDMA line-count byte: bit 7 repeats, bits 6-0 count, `$00` terminates.
///
/// HDMA is the one part of the DMA controller that is awkward to observe, because it runs itself
/// once per scanline with no CPU involvement. Pointing it at `$2180` solves that completely: every
/// transfer lands in WRAM at an auto-incrementing address, so a whole frame of HDMA activity
/// becomes a byte sequence the CPU can read back and check exactly — how many writes happened, in
/// what order, and that they then stopped.
///
/// This table is all non-repeat entries, so each `$0N` header should produce exactly ONE write and
/// then idle for `N-1` lines. A core that treats the count as "write this many times" produces
/// far too many bytes; one that ignores `$00` never stops.
fn d2_03() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("table");
    a.l(".byte $03, $11    ; non-repeat, 3 lines: one write of $11");
    a.l(".byte $04, $22    ; non-repeat, 4 lines: one write of $22");
    a.l(".byte $00         ; terminate");
    a.label("body");
    a.c("Point HDMA channel 0 at $2180 with WMADD in WRAM, run one frame, then read the trail.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    setup_hdma_to_wram(&mut a, 0x0A);
    a.c("Two non-repeat entries: exactly two bytes, then nothing.");
    a.l("rep #$30");
    a.l("lda f:$7E0A00");
    a.assert_a16(
        0x2211,
        "the two non-repeat HDMA entries did not write exactly $11 then $22",
    );
    a.l("lda f:$7E0A02");
    a.assert_a16(0x0000, "HDMA kept writing after the $00 terminator");
    a.finish(
        "D2.03",
        'D',
        "HDMA line-count byte",
        Provenance::Documented("SNESdev Wiki, HDMA; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The repeat flag transfers on every line rather than once per entry.
///
/// The counterpart to `D2.03`, and the pair is the point: one table of non-repeat entries and one
/// of repeat entries, differing only in bit 7 of the header bytes. A core that ignores the bit
/// renders the two tables identically, and either test alone would not notice.
fn d2_04() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("table");
    a.l(".byte $83, $11, $22, $33   ; repeat, 3 lines: one write per line");
    a.l(".byte $82, $44, $55        ; repeat, 2 lines");
    a.l(".byte $00                  ; terminate");
    a.label("body");
    a.c("Same setup as D2.03; only the table's bit 7 differs.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    setup_hdma_to_wram(&mut a, 0x0B);
    a.c("Five repeat lines: five bytes in order, then nothing.");
    a.l("rep #$30");
    a.l("lda f:$7E0B00");
    a.assert_a16(0x2211, "repeat bytes 0-1 are wrong");
    a.l("lda f:$7E0B02");
    a.assert_a16(0x4433, "repeat bytes 2-3 are wrong");
    a.l("lda f:$7E0B04");
    a.l("and #$00FF");
    a.assert_a16(0x0055, "repeat byte 4 is wrong");
    a.l("lda f:$7E0B05");
    a.l("and #$00FF");
    a.assert_a16(
        0x0000,
        "HDMA wrote a sixth byte; the repeat counts total five lines",
    );
    a.finish(
        "D2.04",
        'D',
        "HDMA repeat flag",
        Provenance::Documented("SNESdev Wiki, HDMA; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Arm HDMA channel 0 to write single bytes into WRAM page `page`, run one frame, then disarm.
///
/// Writing into WRAM through `$2180` is what makes HDMA self-scoring: `WMADD` auto-increments, so
/// a frame of per-line transfers leaves an exact trail the CPU can read back. The page is cleared
/// first so "HDMA stopped here" is distinguishable from "this byte was already zero".
fn setup_hdma_to_wram(a: &mut Asm, page: u8) {
    a.l("sep #$20");
    a.c("Clear the landing page so a trailing zero means HDMA stopped, not that it never started.");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label("clear");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l(&format!("sta f:$7E{page:02X}00,x"));
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$0010");
    a.l("bne @clear");

    a.l("sep #$20");
    a.l("stz $420C         ; HDMA off while it is being programmed");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l(&format!("lda #${page:02X}"));
    a.l("sta $2182");
    a.l(&format!("stz $2183         ; WMADD = $7E:{page:02X}00"));
    a.l("stz $4300         ; A->B, direct table, mode 0 (one byte per transfer)");
    a.l("lda #$80");
    a.l("sta $4301         ; B-bus = $2180");
    a.l("rep #$30");
    a.l("ldx #@table");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304         ; table address = this bank");
    a.c("Arm during vblank: enabling HDMA mid-frame is its own erratum (D2.09), and this test is");
    a.c("not about that. The channel initialises at the top of the next frame.");
    a.l("jsr wait_vblank");
    a.l("lda #$01");
    a.l("sta $420C         ; HDMAEN channel 0");
    a.l("jsr wait_vblank   ; let the whole active display run");
    a.l("stz $420C         ; disarm before reading, so nothing moves under the checks");
}

/// A WRAM source with `$2180` as the destination performs no write at all.
///
/// The `$2180` asymmetry, and one of the two halves that make it an asymmetry: WRAM to `$2180` is
/// a WRAM-to-WRAM transfer through the data port, and the hardware simply does not perform the
/// write — where the mirrored case (`$2180` as an A-bus *source*) does write, but writes garbage.
/// A core that implements `$2180` as an ordinary port copies the bytes and looks correct until a
/// game relies on the transfer being a no-op.
fn d1_09() -> Test {
    let mut a = Asm::new();
    a.c("Seed the destination, then try to DMA WRAM->$2180 over it. Nothing must change.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$5A");
    a.l("sta f:$7E0C00");
    a.l("sta f:$7E0C01");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$0C");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0C00 — the destination");
    a.l("stz $4300         ; A->B, increment, mode 0");
    a.l("lda #$80");
    a.l("sta $4301         ; B-bus = $2180");
    a.l("rep #$30");
    a.l("ldx #$0D00");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l("sta $4304         ; A-bus = $7E:0D00, i.e. WRAM");
    a.l("rep #$30");
    a.l("ldx #$0002");
    a.l("stx $4305");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $420B");
    a.l("rep #$30");
    a.l("lda f:$7E0C00");
    a.assert_a16(
        0x5A5A,
        "a WRAM->$2180 DMA wrote to WRAM; that transfer must perform no write at all",
    );
    a.finish(
        "D1.09",
        'D',
        "WRAM->$2180 no-write",
        Provenance::Documented("fullsnes: \"does not cause a write to occur\""),
        Kind::Scored,
        None,
    )
}
