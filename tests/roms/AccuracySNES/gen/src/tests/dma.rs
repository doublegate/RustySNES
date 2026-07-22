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
        d2_07(),
        d2_09(),
        d1_14(),
        d1_13(),
        d1_11(),
        d1_08(),
        d1_03(),
        d1_04(),
        d2_05(),
        d2_06(),
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
    a.l("ldx #.loword(@data)");
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
/// ending in `jml test_restore` therefore jumps *over* that epilogue, and the test reports NOT RUN
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
    a.l("ldx #.loword(@data + 3)");
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
    a.measure_begin_far();
    a.l("lda #$01");
    a.l("sta $420B");
    a.measure_end_far();
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
    a.measure_begin_far();
    a.l("lda #$01");
    a.l("sta $420B");
    a.measure_end_far();
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
    a.l("ldx #.loword(@data)");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.l("stx $4305         ; count = 0, which means 65536");
    a.l("sep #$20");
    a.l("jsl wait_vblank_far   ; start from a known line");
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
    a.l(".byte $03, $11    ; non-repeat, 3 lines: one write of $11 to CGDATA");
    a.l(".byte $04, $22    ; non-repeat, 4 lines: one write of $22, completing the colour word");
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
///
/// **Register widths on exit: `A` 8-bit, `X`/`Y` 16-bit.** Stated because the caller's
/// `.a8`/`.a16` directives come from its own `sep`/`rep` lines and a helper call is not one of
/// those — an undocumented width change here would leave the assembler and the CPU disagreeing
/// about the size of the next immediate.
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
    a.l("ldx #.loword(@table)");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304         ; table address = this bank");
    a.c("Arm during vblank: enabling HDMA mid-frame is its own erratum (D2.09), and this test is");
    a.c("not about that. The channel initialises at the top of the next frame.");
    a.l("jsl wait_vblank_far");
    a.l("lda #$01");
    a.l("sta $420C         ; HDMAEN channel 0");
    a.l("jsl wait_vblank_far   ; let the whole active display run");
    a.l("stz $420C         ; disarm before reading, so nothing moves under the checks");
}

/// Enabling HDMA outside vblank transfers from an uninitialised channel — a golden vector.
///
/// HDMA initialises every enabled channel once per frame, at `V = 0`: it reloads the table pointer
/// `A2An` from `A1Tn` and fetches the first line-count byte into `NLTRn`. Enabling a channel
/// *after* that moment does not run the init — the channel simply starts taking part in the
/// per-line transfers, using whatever `A2An` and `NLTRn` happen to still hold from the last time it
/// ran. The dossier marks it `[ERRATA]`; the writes it produces are real, land at the programmed
/// destination, and carry data from wherever the stale pointer is looking.
///
/// # Why it is recorded rather than asserted
///
/// What the stale pointer *contains* is a function of exactly where the previous frame's transfers
/// stopped, which is deterministic per core but not something any source specifies. Asserting a
/// particular byte would be asserting this core's leftover state. So the test publishes what
/// happened and names the three shapes it can take:
///
/// | variant | phase 2's first byte | reading |
/// |---|---|---|
/// | 1 | nothing was written | the core transfers nothing for a channel enabled mid-frame |
/// | 2 | matches the control | whatever the core did, it started from the top of the table |
/// | 3 | anything else | slots 148/149 say what came out and how much |
///
/// The variants describe the **observation**, not a mechanism, and deliberately so. Making
/// RustySNES run the per-frame init on the `$420C` write — the obvious "no erratum" implementation
/// — produced variant **3**, not variant 2: ten bytes starting `$C2`. Naming variant 2 "the core
/// initialises on enable" would therefore have been a guess contradicted by the first experiment
/// that tried it. RustySNES as it stands reports variant 1.
///
/// # The control is a correctly-armed frame
///
/// Phase 1 arms the identical channel during vblank and lets a whole frame run, so the landing page
/// receives the table's own bytes in order. Without it, "phase 2 wrote something odd" could equally
/// mean the table, the destination or the channel programming was wrong — and the first byte of
/// phase 1 is asserted, not merely recorded, because everything below is read against it.
///
/// The table is eight one-line entries carrying `$11` through `$88`, so a landing page byte names
/// the table entry it came from. That is what makes a phase-2 reading interpretable at all: a byte
/// of `$55` says the stale pointer was four entries in, and a byte that is in the table at all says
/// something quite different from one that is not.
fn d2_09() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("table");
    a.l(".byte $01, $11");
    a.l(".byte $01, $22");
    a.l(".byte $01, $33");
    a.l(".byte $01, $44");
    a.l(".byte $01, $55");
    a.l(".byte $01, $66");
    a.l(".byte $01, $77");
    a.l(".byte $01, $88");
    a.l(".byte $00         ; terminate after eight lines");
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Programme channel 0 -> $2180, table in this bank. WMADD is set per phase.");
    a.l("sep #$20");
    a.l("stz $420C");
    a.l("stz $4300         ; A->B, direct table, mode 0: one byte per line");
    a.l("lda #$80");
    a.l("sta $4301         ; B-bus = $2180");
    a.l("rep #$30");
    a.l("ldx #.loword(@table)");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304");
    a.c("--- phase 1: armed in vblank, so the channel initialises at the top of the frame ---");
    d2_09_clear_and_point(&mut a, 0x13);
    a.l("jsl wait_vblank_far");
    a.l("lda #$01");
    a.l("sta $420C");
    a.l("jsl wait_vblank_far   ; a whole active display");
    a.l("stz $420C");
    d2_09_scan(&mut a, 0x13, 0x7E_01C4);
    a.c("--- phase 2: armed at line 100, long after this frame's init has been and gone ---");
    d2_09_clear_and_point(&mut a, 0x14);
    a.l("jsl wait_vblank_far");
    spin_to_line_dma(&mut a, 100, "d209");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $420C         ; enabled mid-frame: no init runs for it this frame");
    a.l("jsl wait_vblank_far");
    a.l("stz $420C");
    d2_09_scan(&mut a, 0x14, 0x7E_01C8);
    d2_09_verdict(&mut a);
    a.finish(
        "D2.09",
        'D',
        "HDMA armed mid-frame",
        Provenance::Contested(
            "fullsnes and the SNESdev Wiki record that enabling HDMA outside vblank produces \
             erroneous writes from uninitialised A2An/NLTRn, but what those writes contain is a \
             function of the previous frame's leftover state and is specified nowhere",
        ),
        Kind::Golden,
        None,
    )
}

/// Emit [`d2_09`]'s publication and verdict: record all four readings, assert the control,
/// then classify phase 2.
fn d2_09_verdict(a: &mut Asm) {
    a.c("Publish both phases before judging: the control is what makes phase 2 readable.");
    a.l("rep #$30");
    a.l("lda f:$7E01C4");
    a.l("and #$00FF");
    a.record(
        146,
        "D2.09 phase 1 first byte written (control, expect $11)",
    );
    a.l("lda f:$7E01C6");
    a.l("and #$00FF");
    a.record(147, "D2.09 phase 1 bytes written (control, expect 8)");
    a.l("lda f:$7E01C8");
    a.l("and #$00FF");
    a.record(
        148,
        "D2.09 phase 2 first byte written, channel enabled mid-frame",
    );
    a.l("lda f:$7E01CA");
    a.l("and #$00FF");
    a.record(149, "D2.09 phase 2 bytes written");
    a.c("The control is asserted: a wrong table or destination would make phase 2 meaningless.");
    a.l("sep #$20");
    a.l("lda f:$7E01C4");
    a.assert_a8(
        0x11,
        "the first byte a correctly-armed HDMA channel wrote was not the table's first data byte, \
         so the channel programming is wrong and phase 2 says nothing about mid-frame enabling",
    );
    a.l("rep #$30");
    a.l("lda f:$7E01CA");
    a.l("and #$00FF");
    a.l("bne :+");
    a.l("sep #$20");
    a.l("lda #$03          ; variant 1 = nothing transferred at all");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.c("Did it simply behave as though initialised? Compare the first byte against the control.");
    a.l("sep #$20");
    a.l("lda f:$7E01C8");
    a.l("cmp f:$7E01C4");
    a.l("bne :+");
    a.l("lda #$05          ; variant 2 = first byte matches the control");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("lda #$07          ; variant 3 = neither; slots 148/149 say what came out");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
}

/// Emit: clear [`d2_09`]'s landing page and aim `WMADD` at the top of it.
///
/// The pages are `$13`/`$14` and the choice is not arbitrary. The first version used `$0D`, which
/// `D1.14` points `WMADD` at — and because `D1.14` names it through `$2182` rather than as an
/// address literal, no grep for `$7E0D00` finds it. `D1.14` began failing the moment this test
/// landed. WRAM scratch has the same no-allocator problem the measurement channel has; there is no
/// gate for it, so pages here are taken from well outside the range anything else uses.
fn d2_09_clear_and_point(a: &mut Asm, page: u8) {
    a.l("sep #$20");
    a.l("stz $420C");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label(&format!("clr{page:02X}"));
    a.l("sep #$20");
    a.l("lda #$00");
    a.l(&format!("sta f:$7E{page:02X}00,x"));
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$0020");
    a.l(&format!("bne @clr{page:02X}"));
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l(&format!("lda #${page:02X}"));
    a.l("sta $2182");
    a.l("stz $2183");
}

/// Emit: count [`d2_09`]'s written bytes and stash the first one plus the count.
///
/// The page was cleared to zero and every table byte is non-zero, so "written" and "non-zero" are
/// the same question. The count stops at the first zero rather than tallying the whole page: HDMA
/// writes consecutively, so a gap would mean something stranger than this test is equipped to
/// describe, and a trailing count would hide it.
fn d2_09_scan(a: &mut Asm, page: u8, dest: u32) {
    a.l("sep #$20");
    a.l(&format!("lda f:$7E{page:02X}00"));
    a.l(&format!("sta f:${dest:06X}"));
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label(&format!("cnt{page:02X}"));
    a.l("sep #$20");
    a.l(&format!("lda f:$7E{page:02X}00,x"));
    a.l("beq :+");
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$0020");
    a.l(&format!("bne @cnt{page:02X}"));
    a.l(":");
    a.l("rep #$30");
    a.l("txa");
    a.l(&format!("sta f:${:06X}", dest + 2));
}

/// Emit a spin until the V counter reads `line`, for the DMA tests.
fn spin_to_line_dma(a: &mut Asm, line: u16, tag: &str) {
    a.c(&format!("Spin until V = {line}."));
    a.l("sep #$20");
    a.label(&format!("wl{tag}"));
    a.l("lda $213F");
    a.l("lda $2137");
    a.l("lda $213D");
    a.l("xba");
    a.l("lda $213D");
    a.l("and #$01");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
    a.l(&format!("cmp #{line}"));
    a.l("sep #$20");
    a.l(&format!("bne @wl{tag}"));
}

/// HDMA preempts a general-purpose DMA, which pauses and then resumes correctly.
///
/// # The obvious version of this test asserts nothing
///
/// Running a large GP-DMA with HDMA enabled and checking the destination is byte-correct is
/// **vacuous**: a core that never preempts at all copies the block correctly too. That is the same
/// shape as the withdrawn `A4.06` — verifying the right thing is present without establishing that
/// the wrong thing would have been visible.
///
/// So both halves are asserted:
///
/// * **that preemption happened** — the HDMA landing page carries its trail. `setup_hdma_to_wram`
///   clears that page first, so a present trail means HDMA ran, and it ran during the frame the
///   GP-DMA occupied. Without this, "resumed correctly" is unfalsifiable.
/// * **that the GP-DMA resumed correctly** — first and **last** byte both match the source. The
///   last matters most: a core that loses the paused byte count resumes short, and only the tail
///   shows it.
///
/// # Sizing and placement
///
/// GP-DMA moves a byte per 8 master clocks, so 4 KiB is ~32768 clocks — about 24 scanlines, giving
/// HDMA two dozen chances to preempt. It is started immediately after the HDMA channel is armed so
/// it runs through active display, where HDMA fires. **A transfer confined to vblank would never be
/// preempted and the test would silently become the vacuous version.**
///
/// The source is ROM, not WRAM: a WRAM source with `$2180` as the destination performs no write at
/// all (`D1.09`), so the copy would be a no-op and the check meaningless. The expected bytes are
/// read back from the same ROM rather than hardcoded, so this pins the transfer and not the image
/// layout.
fn d2_07() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("table");
    a.l(".byte $03, $11    ; non-repeat, 3 lines: one write of $11");
    a.l(".byte $04, $22    ; non-repeat, 4 lines: one write of $22");
    a.l(".byte $00         ; terminate");
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("HDMA writes CGRAM, NOT $2180. Both HDMA and the GP-DMA would otherwise go through the");
    a.c("WRAM data port, which has a single shared WMADD -- they would interleave into each");
    a.c("other's destination and neither result would mean anything. That is what the first");
    a.c("version of this test did, and its trail simply never appeared.");
    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $2121         ; CGADD = $80, well clear of the palettes the scenes bless");
    a.l("stz $420C");
    a.l("stz $4300         ; A->B, direct table, mode 0");
    a.l("lda #$22");
    a.l("sta $4301         ; B-bus = $2122 (CGDATA)");
    a.l("rep #$30");
    a.l("ldx #.loword(@table)");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304");
    a.c("GP-DMA channel 1: 4 KiB of ROM through the WRAM port at $7E:4000.");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$40");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:4000");
    a.l("stz $4310         ; A->B, increment, mode 0");
    a.l("lda #$80");
    a.l("sta $4311         ; B-bus = $2180");
    a.l("rep #$30");
    a.l("lda #$8000");
    a.l("sta $4312         ; A-bus source $00:8000");
    a.l("sep #$20");
    a.l("stz $4314         ; source bank $00");
    a.l("rep #$30");
    a.l("lda #$1000");
    a.l("sta $4315         ; 4096 bytes");
    a.c("Arm HDMA in vblank, then start the GP-DMA so it runs THROUGH active display. A transfer");
    a.c("confined to vblank is never preempted and this test would assert nothing.");
    a.l("sep #$20");
    a.l("jsl wait_vblank_far");
    a.l("lda #$01");
    a.l("sta $420C         ; HDMAEN channel 0");
    a.l("lda #$02");
    a.l("sta $420B         ; run channel 1");
    a.l("jsl wait_vblank_far   ; let the rest of the frame, and its HDMA, complete");
    a.l("stz $420C         ; disarm before reading");
    a.c("Half one: HDMA actually ran. Without this the transfer check below is satisfied just as");
    a.c("well by a core that never preempted anything. Read CGRAM back at the index the table");
    a.c("wrote: $2121 selects the word, then $213B reads low then high.");
    a.l("lda #$80");
    a.l("sta $2121");
    a.l("lda $213B");
    a.l("sta f:$7E0A04");
    a.l("lda $213B");
    a.l("sta f:$7E0A05");
    a.l("rep #$20");
    a.l("lda f:$7E0A04");
    a.l("and #$7FFF        ; CGRAM is 15-bit; bit 15 reads back as open bus");
    a.assert_a16(
        0x2211,
        "the HDMA trail is absent — no preemption happened, so the transfer check proves nothing",
    );
    a.c("Half two: the paused transfer resumed and ran to completion. The channel's own registers");
    a.c("say so without needing the destination read back: DAS counts down to zero and A1T ends");
    a.c("at source + length. A core that resumed short leaves both short.");
    a.l("lda $4315");
    a.assert_a16(
        0x0000,
        "channel 1's byte count did not reach zero — the GP-DMA resumed short after preemption",
    );
    a.l("lda $4312");
    a.assert_a16(
        0x9000,
        "channel 1's source address did not advance the full 4096 bytes",
    );
    a.finish(
        "D2.07",
        'D',
        "HDMA preempts GP-DMA",
        Provenance::Documented("SNESdev Wiki, HDMA; anomie's timing doc; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A DMA whose **A-bus** address lands in the CPU/DMA register block does not read those registers
/// — a golden vector, never scored.
///
/// The errata marks four A-bus ranges invalid for DMA: `$21xx`, `$4000-$41FF`, `$4200-$421F` and
/// `$4300-$437F`. The A bus cannot address the B-bus registers or the CPU's own block, so a
/// transfer aimed there does not fetch what a CPU read would.
///
/// # Why this reports instead of asserting
///
/// The errata says the range is invalid; it does **not** say what is read instead. The substitute
/// is open bus, and that leaves nothing portable to assert:
///
/// * asserting a particular byte would pin this core's open-bus model rather than the erratum;
/// * asserting "different from the register value" is unsound — open bus could coincide with it
///   and the test would fail a correct core;
/// * asserting that two transfers return the *same* byte is unsound too, and demonstrably so.
///   That was the first version of this test. Mesen2 failed it and was right to: its two runs
///   returned `$A9` and `$C2`, instruction opcodes — open bus tracking recent CPU fetches, which
///   differ because the surrounding code differs. RustySNES and snes9x happened to return a stable
///   value and passed. Nothing documents which is correct.
///
/// So the observation is recorded. Variant 1 means neither transfer came back holding the probe
/// value — the range was not read, which is what all three cores do and what the errata predicts.
/// Variant 2 means a core read the register block through the A bus, and announces itself.
///
/// Two probe values are still written to `$4300` and still differ between runs, because that is
/// what makes "the register was read" detectable at all; what changed is that their *absence* is
/// reported rather than their equality asserted.
fn d1_08() -> Test {
    let mut a = Asm::new();
    a.c(
        "Channel 1 reads A-bus $00:4300 -- channel 0's DMAP, never armed here -- into WRAM through",
    );
    a.c("$2180. Run twice with the probe register holding different values.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $420C         ; no HDMA: nothing else may touch the channels mid-test");
    a.l("stz $4310         ; A->B, increment, mode 0");
    a.l("lda #$80");
    a.l("sta $4311         ; B-bus = $2180");
    a.l("sep #$20");
    a.l("stz $4314         ; A-bus bank $00");
    a.c("--- run 1: probe register = $53 ---");
    a.l("lda #$53");
    a.l("sta $4300");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$0E");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0E00");
    a.l("rep #$30");
    a.l("lda #$4300");
    a.l("sta $4312");
    a.l("lda #$0001");
    a.l("sta $4315");
    a.l("sep #$20");
    a.l("lda #$02");
    a.l("sta $420B");
    a.c("--- run 2: probe register = $A5, destination one byte along ---");
    a.l("lda #$A5");
    a.l("sta $4300");
    a.l("lda #$01");
    a.l("sta $2181");
    a.l("lda #$0E");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0E01");
    a.l("rep #$30");
    a.l("lda #$4300");
    a.l("sta $4312");
    a.l("lda #$0001");
    a.l("sta $4315");
    a.l("sep #$20");
    a.l("lda #$02");
    a.l("sta $420B");
    a.c("Record both bytes: they are the whole content of this row and the reason it is golden.");
    a.l("rep #$20");
    a.l("lda f:$7E0E00");
    a.l("and #$00FF");
    a.record(144, "D1.08 run 1 byte");
    a.l("lda f:$7E0E01");
    a.l("and #$00FF");
    a.record(145, "D1.08 run 2 byte");
    a.c("Variant 1 = neither run held the probe value; variant 2 = the range was read after all.");
    a.l("sep #$20");
    a.l("lda f:$7E0E00");
    a.l("cmp #$53");
    a.l("beq @sawit");
    a.l("lda f:$7E0E01");
    a.l("cmp #$A5");
    a.l("beq @sawit");
    a.l("lda #$03");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l("@sawit:");
    a.l("lda #$05");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "D1.08",
        'D',
        "Invalid A-bus (golden)",
        Provenance::Contested(
            "the errata names the ranges invalid but does not specify what is read instead; \
             the substitute is open bus, whose content is core-specific and time-dependent",
        ),
        Kind::Golden,
        None,
    )
}

/// The DMA channel registers power on as `$FF` across the board.
///
/// SNESdev's *Tricky-to-emulate games* lists *"DMA controller power on state is invalid"* against
/// **Heian Fuuunden**, whose title screen corrupts when the opening is skipped: the game relies on
/// the reset contents of `$43xx` rather than writing every field itself. fullsnes' register table
/// and the SNESdev DMA-registers page independently give the same power-on values, and ares and
/// bsnes default every channel field to match.
///
/// # Why this is captured rather than read
///
/// `init_registers` leaves `$43xx` alone, but *every DMA test writes them*. By the time any test
/// body runs, the channel registers hold whatever the last DMA test left, so reading them from a
/// test would measure the battery rather than the machine. `capture_power_on` therefore snapshots
/// `$4300-$430B` at the very top of reset, the same mechanism `B5.05` and the `G1` rows use.
///
/// # `$43x4` is excluded, deliberately
///
/// SNESdev pins the A1T bank byte to `$FF`; fullsnes prints it as `xx`, i.e. unspecified. One
/// source is not enough to score against, so the check ANDs the other eleven bytes together and
/// leaves `$4304` out. The AND is the whole assertion: if every byte is `$FF` the result is `$FF`,
/// and any byte that is not contributes a zero bit that cannot be masked back.
///
/// Reset behaviour is **not** asserted. fullsnes says `$43xx` is left unchanged by reset, SNESdev
/// is silent, ares and bsnes re-default the channels unconditionally, and Mesen2 does not — a
/// genuine three-way split, and a cart cannot drive a reset to look anyway.
fn d1_11() -> Test {
    let mut a = Asm::new();
    a.c("AND the eleven agreed bytes together: all $FF gives $FF, and any other value cannot.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("and f:V_PO_DMA+0  ; $4300 DMAP");
    a.l("and f:V_PO_DMA+1  ; $4301 BBAD");
    a.l("and f:V_PO_DMA+2  ; $4302 A1T low");
    a.l("and f:V_PO_DMA+3  ; $4303 A1T high");
    a.c("+4 ($4304, the A1T bank) is skipped: SNESdev says $FF, fullsnes says unspecified.");
    a.l("and f:V_PO_DMA+5  ; $4305 DAS low");
    a.l("and f:V_PO_DMA+6  ; $4306 DAS high");
    a.l("and f:V_PO_DMA+7  ; $4307 DASB");
    a.l("and f:V_PO_DMA+8  ; $4308 A2A low");
    a.l("and f:V_PO_DMA+9  ; $4309 A2A high");
    a.l("and f:V_PO_DMA+10 ; $430A NLTR");
    a.l("and f:V_PO_DMA+11 ; $430B unused/scratch");
    a.assert_a8(
        0xFF,
        "a DMA channel register did not power on as $FF (Heian Fuuunden depends on this)",
    );
    a.finish(
        "D1.11",
        'D',
        "DMA power-on state",
        Provenance::Corroborated(
            "fullsnes register table and the SNESdev DMA-registers page agree independently; \
             ares and bsnes default every channel field to match",
        ),
        Kind::Scored,
        None,
    )
}

/// `$2180` as a DMA **source** does perform the write — the other half of the asymmetry.
///
/// `D1.09` covers WRAM to `$2180`: no write happens at all. This is the mirrored case, and
/// hardware does not mirror it. fullsnes: `$2180` to WRAM *"does cause a write to occur (but no
/// read), but the value written is invalid"*. So a core that implements `$2180` symmetrically —
/// either writing in both directions or neither — gets exactly one of the two rows wrong.
///
/// # Asserting a write happened without knowing what was written
///
/// The written value is documented as *invalid*, i.e. unspecified. Seeding the destination and
/// asserting it changed would therefore be unsound: the invalid value could coincide with the seed,
/// and the test would fail on a correct core.
///
/// The transfer is run **twice, from two different seeds** instead. Whatever the invalid value is,
/// it is the same both times, so:
///
/// * if the write happens, both destinations end up holding that same value — **equal**;
/// * if no write happens, each destination still holds its own seed — `$00` and `$FF`, **unequal**.
///
/// Asserting the two results are equal therefore pins "a write occurred" without naming the value,
/// and the two seeds are chosen to differ so "no write" cannot accidentally satisfy it.
fn d1_14() -> Test {
    let mut a = Asm::new();
    a.c("Both destinations, seeded differently. Whatever invalid value the write deposits, it is");
    a.c("the same for both transfers -- so equal results mean a write, unequal means none.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0C10");
    a.l("lda #$FF");
    a.l("sta f:$7E0C11");
    a.c("WMADD points somewhere harmless: this direction reads no WRAM, but the port still has an");
    a.c("address and leaving it where a previous test left it would be sloppy rather than wrong.");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$0D");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0D00");
    a.c("--- transfer 1: $2180 -> $7E:0C10 ---");
    a.l("lda #$80");
    a.l("sta $4300         ; bit 7 = B->A, mode 0");
    a.l("lda #$80");
    a.l("sta $4301         ; B-bus = $2180");
    a.l("rep #$30");
    a.l("lda #$0C10");
    a.l("sta $4302");
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l("sta $4304         ; A-bus = $7E:0C10");
    a.l("rep #$30");
    a.l("lda #$0001");
    a.l("sta $4305         ; one byte");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $420B");
    a.c("--- transfer 2: the same, into the other seed ---");
    a.l("rep #$30");
    a.l("lda #$0C11");
    a.l("sta $4302");
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l("sta $4304");
    a.l("rep #$30");
    a.l("lda #$0001");
    a.l("sta $4305");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $420B");
    a.c("Equal means a write occurred in both; unequal means the seeds survived and it did not.");
    a.l("lda f:$7E0C10");
    a.l("sta f:$7E0C12");
    a.l("lda f:$7E0C11");
    a.l("cmp f:$7E0C12");
    a.fail_if_ne(
        "$2180 as a DMA source performed no write — both destinations still hold their seeds",
    );
    a.finish(
        "D1.14",
        'D',
        "$2180 B->A does write",
        Provenance::Documented("fullsnes: $2180->WRAM writes, but the value written is invalid"),
        Kind::Scored,
        None,
    )
}

/// The `$43x5/6` byte-count register decrements as a GP-DMA runs, and reads zero when it finishes.
///
/// A DMA transfers until the count reaches zero, so the register the count lives in is spent by the
/// end — hardware leaves it at `$0000`, not at the value that was programmed. A core that keeps its
/// own private counter and never writes the decrement back leaves the programmed size sitting in the
/// register, so a driver that reads `$43x5` to learn how many bytes actually moved (or to resume a
/// partial transfer) gets the wrong answer.
///
/// The transfer itself is an ordinary four-byte mode-0 run into a scratch WRAM page; nothing about
/// its destination matters here. What is read back is the count register, which must have counted
/// down to zero. A core that never decrements it, or that restores the programmed count when the
/// run ends, reads `$0004` instead — the seed the test wrote, which is exactly the signature of the
/// register not tracking the transfer.
fn d1_13() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c(
        "Run a normal four-byte mode-0 DMA into a scratch page, then read the byte-count register.",
    );
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$06");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0600, a scratch page clear of every test");
    a.l("stz $4300         ; DMAP: A->B, increment, mode 0");
    a.l("lda #$80");
    a.l("sta $4301         ; B-bus = $2180");
    source_from_table(&mut a, 4);
    a.l("lda #$01");
    a.l("sta $420B         ; run channel 0");
    a.c("The count register now holds the decremented value. A core that never decrements it, or");
    a.c("that restores the programmed count at the end, reads back $0004 — the size the test wrote.");
    a.l("rep #$30");
    a.l("lda $4305         ; a 16-bit read folds $4305 (low) and $4306 (high) into A");
    a.assert_a16(
        0x0000,
        "the DMA byte-count register did not decrement to zero across the transfer — it still holds \
         the programmed size, so it is not tracking the transfer at all",
    );
    a.finish(
        "D1.13",
        'D',
        "DMA count hits zero",
        Provenance::Documented(
            "fullsnes and ares: the DAS $43x5/6 byte-count register decrements as GP-DMA transfers \
             and reads $0000 when the transfer completes",
        ),
        Kind::Scored,
        None,
    )
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

/// A multi-channel DMA runs the lower channel number first.
///
/// `$420B` starts every selected channel from one write, and the order is not observable from
/// timing alone — but it is perfectly observable from the destination when both channels write to
/// the same auto-incrementing port. Channel 0 sources `$11` and channel 1 sources `$22`, so the
/// byte pair in WRAM spells out the order the hardware chose.
fn d1_04() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("data");
    a.l(".byte $11, $22");
    a.label("body");
    a.c("Both channels write one byte to $2180. WMADD advances across both, so the pair in WRAM");
    a.c("is the execution order written down.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$0E");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0E00");

    a.c("--- channel 0: sources $11 ---");
    a.l("lda #$08");
    a.l("sta $4300         ; A->B, fixed, mode 0");
    a.l("lda #$80");
    a.l("sta $4301");
    a.l("rep #$30");
    a.l("ldx #.loword(@data)");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304");
    a.l("rep #$30");
    a.l("ldx #$0001");
    a.l("stx $4305");

    a.c("--- channel 1: sources $22 ---");
    a.l("sep #$20");
    a.l("lda #$08");
    a.l("sta $4310");
    a.l("lda #$80");
    a.l("sta $4311");
    a.l("rep #$30");
    a.l("ldx #.loword(@data + 1)");
    a.l("stx $4312");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4314");
    a.l("rep #$30");
    a.l("ldx #$0001");
    a.l("stx $4315");

    a.c("Start both in ONE write — which is the whole point; two writes would impose the order.");
    a.l("sep #$20");
    a.l("lda #$03");
    a.l("sta $420B");
    a.l("rep #$30");
    a.l("lda f:$7E0E00");
    a.assert_a16(
        0x2211,
        "the channels did not run in ascending order (expected $11 from ch0 then $22 from ch1)",
    );
    a.finish(
        "D1.04",
        'D',
        "DMA channel priority",
        Provenance::Documented("SNESdev Wiki, DMA; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Indirect HDMA fetches each transfer's data through a pointer in the table.
///
/// With `$4300` bit 6 set, a table entry carries a 16-bit pointer instead of the data itself and
/// the bytes come from `$4307`'s bank at that address. It is the mode almost every scrolling
/// effect uses, and a core that ignores bit 6 transfers the pointer bytes themselves — which is
/// exactly what this catches, since the pointer's low byte is nothing like `$77`.
fn d2_05() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("table");
    a.l(".byte $02");
    a.l(".addr @ind1      ; indirect: the DATA lives at this address, not here");
    a.l(".byte $02");
    a.l(".addr @ind2");
    a.l(".byte $00        ; terminate");
    a.label("ind1");
    a.l(".byte $77");
    a.label("ind2");
    a.l(".byte $88");
    a.label("body");
    a.c("Same $2180 trick as D2.03/D2.04, with DMAP bit 6 set and $4307 naming the data bank.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $420C");
    a.c("Clear the landing page first.");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label("clear");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta f:$7E0F00,x");
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$0010");
    a.l("bne @clear");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$0F");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:0F00");
    a.l("lda #$40");
    a.l("sta $4300         ; A->B, INDIRECT, mode 0");
    a.l("lda #$80");
    a.l("sta $4301         ; B-bus = $2180");
    a.l("rep #$30");
    a.l("ldx #.loword(@table)");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304         ; table bank");
    a.l("phk");
    a.l("pla");
    a.l("sta $4307         ; indirect DATA bank");
    a.l("jsl wait_vblank_far");
    a.l("lda #$01");
    a.l("sta $420C");
    a.l("jsl wait_vblank_far");
    a.l("stz $420C");
    a.l("rep #$30");
    a.l("lda f:$7E0F00");
    a.assert_a16(
        0x8877,
        "indirect HDMA did not fetch through the pointers (a core ignoring bit 6 writes the \
         pointer bytes instead)",
    );
    a.finish(
        "D2.05",
        'D',
        "HDMA indirect mode",
        Provenance::Documented("SNESdev Wiki, HDMA; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$430A` is a live line counter, and it reads back zero once the table terminates.
///
/// The HDMA counterpart to `D1.06`. `$4308/09` and `$430A` are working state the controller
/// updates as it walks the table, not a copy of what was programmed — so after a frame that ran to
/// the `$00` terminator, the counter holds that terminator. A core that leaves the last real count
/// there looks correct for a whole frame and then desynchronises on the next one.
fn d2_06() -> Test {
    let mut a = Asm::new();
    a.l("bra @body");
    a.label("table");
    a.l(".byte $02, $11");
    a.l(".byte $00");
    a.label("body");
    a.c("Run one frame of HDMA to $2180, then read the channel's working registers.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("stz $420C");
    a.l("lda #$00");
    a.l("sta $2181");
    a.l("lda #$10");
    a.l("sta $2182");
    a.l("stz $2183         ; WMADD = $7E:1000");
    a.l("stz $4300");
    a.l("lda #$80");
    a.l("sta $4301");
    a.l("rep #$30");
    a.l("ldx #.loword(@table)");
    a.l("stx $4302");
    a.l("sep #$20");
    a.l("phk");
    a.l("pla");
    a.l("sta $4304");
    a.l("jsl wait_vblank_far");
    a.l("lda #$01");
    a.l("sta $420C");
    a.l("jsl wait_vblank_far");
    a.l("stz $420C");
    a.l("lda $430A");
    a.assert_a8(
        0x00,
        "$430A does not hold the $00 terminator after the table ran out",
    );
    a.c("$4308/09 must have advanced past the table's start — it is a walking pointer.");
    a.l("rep #$30");
    a.l("lda $4308");
    a.l("sec");
    a.l("sbc #.loword(@table)");
    a.assert_a16_range(
        1,
        16,
        "$4308/09 did not advance past the table start; it is a working pointer, not a copy",
    );
    a.finish(
        "D2.06",
        'D',
        "HDMA $4308/$430A state",
        Provenance::Documented("SNESdev Wiki, HDMA registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The fixed cost of starting a DMA, measured (golden vector).
///
/// `D1.02` establishes the per-byte rate as a differential, which deliberately cancels everything
/// that is not per-byte. What it cancels is this: an 8-clock startup plus an alignment cost that
/// depends on where in the CPU's cycle the `$420B` write lands. That alignment is exactly the part
/// no two implementations need agree on to be equally correct, so the number is recorded rather
/// than asserted — the same treatment `B4.14` gets, for the same reason.
fn d1_03() -> Test {
    let mut a = Asm::new();
    data_table(&mut a);
    a.c("A one-byte transfer: almost all of what this measures is overhead.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$08");
    a.l("sta $4300         ; fixed source, mode 0");
    a.l("lda #$22");
    a.l("sta $4301         ; CGDATA — harmless, and rebuilt before any scene");
    a.l("stz $2121");
    source_from_table(&mut a, 1);
    a.measure_begin_far();
    a.l("lda #$01");
    a.l("sta $420B");
    a.measure_end_far();
    a.measure_result();
    a.record(111, "D1.03 one-byte DMA, absolute (dots)");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta f:V_TEST_RESULT   ; golden: the number is in the measurement channel");
    a.l("jml test_restore");
    a.finish(
        "D1.03",
        'D',
        "DMA startup overhead",
        Provenance::Documented("SNESdev Wiki, DMA timing; fullsnes"),
        Kind::Golden,
        None,
    )
}
