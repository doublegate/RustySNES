//! Group G — the cartridge itself: header and memory map (ticket **T-04-G**).
//!
//! The group's dossier scope also covers power-on and reset state, and **none of that is here
//! yet**: the battery runs long after reset, through a runtime that has already written most of the
//! registers whose power-on values the dossier enumerates. Reaching them needs a test that runs
//! before the runtime does, which is a mechanism rather than a test.
//!
//! The one group whose subject is not a chip. What it asserts is that the *emulator's* view of the
//! cartridge matches the one every other assertion silently depends on: that the header is where
//! LoROM says it is, that its self-check holds, and that a bank number decodes to the ROM offset
//! the mapping formula gives. Every test in every other group is running out of a ROM addressed
//! through that formula — if it were wrong, the failures would appear anywhere but here.
//!
//! Which is exactly why these are worth writing down rather than assuming. A mapping bug that
//! happens to be self-consistent produces a battery that passes and a commercial ROM that does not
//! boot.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// Every Group G test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![g1_10(), g1_12(), g1_14()]
}

/// The header's checksum and its complement must XOR to `$FFFF`.
///
/// Two words at `$FFDC` and `$FFDE` that are bitwise inverses. Every emulator uses the pair to
/// *find* the header — a candidate location whose two words do not complement is not a header — so
/// the invariant is load-bearing before a single instruction runs, and a cart that fails it may be
/// detected as the wrong mapping or rejected outright.
///
/// The cart checking its own header is not circular. The generator computes the checksum and the
/// linker places it; this asserts that what the CPU *reads back through the memory map* is that
/// pair, which is a statement about the map as much as about the bytes.
fn g1_10() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("lda f:$00FFDC        ; the complement");
    a.l("eor f:$00FFDE        ; XOR the checksum");
    a.assert_a16(
        0xFFFF,
        "the header's checksum and complement do not XOR to $FFFF — the pair every emulator uses \
         to recognise a header at all",
    );
    a.finish(
        "G1.10",
        'G',
        "Checksum XOR complement",
        Provenance::Documented("SNESdev Wiki, cartridge header; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A LoROM header sits at `$00:FFC0`, and its map-mode byte says so.
///
/// The header's location *is* the mapping: LoROM puts it at ROM offset `$7FC0`, which the LoROM
/// formula exposes at `$00:FFC0`, while HiROM's lives at `$00:FFC0` of a differently-decoded image.
/// An emulator that guesses the mapping wrong reads a header out of the middle of the code and gets
/// a title of garbage — which is why the map-mode byte is checked here from *inside* a running
/// LoROM image: if the guess were wrong, this test would not be executing.
///
/// `$FFD5 = $20` is LoROM, SlowROM. `$FFD7 = $07` is `log2(131072) - 10`, the 128 KiB this image
/// actually is — a second, independent statement about the same header, and the one that catches a
/// map-mode byte read from the right place in the wrong image.
fn g1_12() -> Test {
    let mut a = Asm::new();
    a.l("sep #$20");
    a.l("lda f:$00FFD5");
    a.assert_a8(
        0x20,
        "the map-mode byte at $FFD5 is not $20 (LoROM, SlowROM), so the header is not where LoROM \
         puts it",
    );
    a.l("lda f:$00FFD7");
    a.assert_a8(
        0x07,
        "the ROM-size byte at $FFD7 is not 7 (128 KiB), so the header was read from the right \
         address of the wrong image",
    );
    a.c("And the title, whose first byte is the one thing a human recognises in a hex dump.");
    a.l("lda f:$00FFC0");
    a.assert_a8(
        b'A',
        "the title does not begin at $FFC0 with ACCURACYSNES's first letter",
    );
    a.finish(
        "G1.12",
        'G',
        "LoROM header location",
        Provenance::Documented("SNESdev Wiki, cartridge header; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// LoROM decodes a bank as `((bank & $7F) << 15) | (addr & $7FFF)`.
///
/// Two claims in one formula, and this image is built to make both observable. `& $7F` means banks
/// `$80`-`$FF` mirror `$00`-`$7F`, so `$80:8005` and `$00:8005` are the same ROM byte. `<< 15`
/// means each bank maps its own **32 KiB**, so `$01:8005` is a *different* byte — which is only
/// checkable in an image larger than 32 KiB, and is the reason this cart is 128 KiB rather than the
/// minimum.
///
/// Each bank carries its own signature byte at `$xx:8005` for exactly this. A core that decoded
/// banks as 64 KiB, or ignored the mirror, reads the wrong signature and says which mistake it
/// made.
fn g1_14() -> Test {
    let mut a = Asm::new();
    a.l("sep #$20");
    a.l("lda f:$008005");
    a.assert_a8(
        0xA0,
        "bank $00's signature is wrong — the image is not mapped as expected",
    );
    a.c("Bank $01 is a DIFFERENT 32 KiB. A core using a 64 KiB stride reads bank $00's byte here.");
    a.l("lda f:$018005");
    a.assert_a8(
        0xA1,
        "bank $01 did not map its own 32 KiB — reading $A0 means the bank stride is 64 KiB, not 32",
    );
    a.c("And $80 mirrors $00, because the decode masks the top bit off the bank number.");
    a.l("lda f:$808005");
    a.assert_a8(
        0xA0,
        "bank $80 did not mirror bank $00 — the LoROM decode masks the bank with $7F",
    );
    a.finish(
        "G1.14",
        'G',
        "LoROM bank decode",
        Provenance::Documented("SNESdev Wiki, memory map; fullsnes"),
        Kind::Scored,
        None,
    )
}
