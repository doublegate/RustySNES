//! HiROM / ExHiROM image tests — Group G rows that need a second cartridge layout (ticket T-04-G).
//!
//! These do NOT run in the LoROM battery. They are authored here and emitted into the parallel
//! HiROM image (`build/accuracysnes-hirom.sfc`, linked with `hirom.cfg` and `header-hirom.s`) by
//! `main.rs`, so the emulator's HiROM board decode and its battery-SRAM window are self-scored
//! on-cart rather than only unit-tested in `rustysnes-cart`. See `docs/accuracysnes-plan.md`.
//!
//! Position-independence: every body here reaches memory through long addressing (`lda f:`) and the
//! assertion helpers (`sta f:V_TEST_RESULT`, no `jsr`), so it runs correctly wherever `hirom.cfg`
//! places it (the `$C0` linear half, dispatched through the 24-bit `_test_entries` table).

use crate::dsl::{Asm, Kind, Provenance, Test};

/// The HiROM battery, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![g1_15(), g1_17()]
}

/// HiROM decode: the `$00:8000-$FFFF` window and the `$C0`/`$40` linear banks decode to the same ROM.
///
/// This runs inside the HiROM image, so its passing at all is already evidence the emulator selected
/// the HiROM board (a LoROM decode would map `$00:8000` to ROM offset 0 and the runtime would not be
/// where the reset vector points — the cart would never reach here). On top of that it asserts:
///
/// - `$FFD5 == $21` — the header reads as HiROM, SlowROM.
/// - the byte at ROM offset `$FFC0` (the title's first letter) reads identically through the
///   `$00:FFC0` window and the `$C0:FFC0` linear bank — the two views HiROM maps to one ROM offset.
/// - `$40:FFC0` mirrors `$C0:FFC0` — the `$40-$7D` banks mirror `$C0-$FD`.
///
/// A core that got `((bank & $3F) << 16) | addr` wrong, or that failed to mirror `$40-$7D` onto
/// `$C0-$FD`, returns different bytes for the same ROM offset and fails here.
fn g1_15() -> Test {
    let mut a = Asm::new();
    a.l("sep #$20");
    a.c("The header must read as HiROM.");
    a.l("lda f:$00FFD5");
    a.assert_a8(
        0x21,
        "the map-mode byte at $FFD5 is not $21 (HiROM, SlowROM), so the emulator did not select \
         the HiROM board for this image",
    );
    a.c("HiROM maps the $00:8000-$FFFF window and the $C0-$FF linear banks to the same ROM. The");
    a.c("title's first byte at $FFC0 is a fixed landmark in the upper half, reachable both ways.");
    a.l("lda f:$00FFC0");
    a.l("sta f:$7E0120       ; the byte as seen through the $00:8000 window");
    a.l("lda f:$C0FFC0");
    a.l("cmp f:$7E0120");
    a.fail_if_ne(
        "$00:FFC0 (the HiROM window) and $C0:FFC0 (the linear high bank) returned different bytes, \
         so the two views do not decode to the same ROM offset — the HiROM decode is wrong",
    );
    a.c("The $40-$7D banks mirror $C0-$FD, so $40:FFC0 must read the same byte.");
    a.l("lda f:$40FFC0");
    a.l("cmp f:$7E0120");
    a.fail_if_ne(
        "$40:FFC0 does not match $C0:FFC0 — the $40-$7D HiROM bank mirror of $C0-$FD is wrong",
    );
    a.finish(
        "G1.15",
        'G',
        "HiROM decode",
        Provenance::Documented(
            "SNESdev Wiki, memory map (HiROM: banks $C0-$FF linear, $00-$3F:$8000-$FFFF window, \
             $40-$7D mirror $C0-$FD); fullsnes",
        ),
        Kind::Scored,
        None,
    )
}

/// HiROM battery SRAM maps at `$20-$3F:$6000-$7FFF`, mirrored at `$A0-$BF`.
///
/// The header declares 8 KiB of battery SRAM (`$FFD6 = $02`, `$FFD8 = $03`), so the emulator must
/// expose writable RAM in the HiROM SRAM window. The test writes two distinctive bytes through
/// `$20:6000`, reads them back (a core that did not map SRAM there returns open bus / ROM, not the
/// value), and confirms the `$A0-$BF` banks alias the same SRAM.
fn g1_17() -> Test {
    let mut a = Asm::new();
    a.l("sep #$20");
    a.c("HiROM battery SRAM is at $20-$3F:$6000-$7FFF. Write two distinctive bytes and read back.");
    a.l("lda #$5A");
    a.l("sta f:$206000");
    a.l("lda #$A5");
    a.l("sta f:$206001");
    a.l("lda f:$206000");
    a.assert_a8(
        0x5A,
        "SRAM at $20:6000 did not read back the value just written — the HiROM battery-SRAM window \
         ($20-$3F:$6000-$7FFF) is not mapped as writable RAM",
    );
    a.l("lda f:$206001");
    a.assert_a8(
        0xA5,
        "SRAM at $20:6001 did not read back — the SRAM window is not backed by persistent RAM \
         across the two accesses",
    );
    a.c("The $A0-$BF banks mirror $20-$3F, so $A0:6000 must read the same SRAM byte.");
    a.l("lda f:$A06000");
    a.assert_a8(
        0x5A,
        "$A0:6000 does not mirror $20:6000 — the $A0-$BF HiROM SRAM mirror is wrong",
    );
    a.finish(
        "G1.17",
        'G',
        "HiROM SRAM window",
        Provenance::Documented(
            "SNESdev Wiki, memory map (HiROM SRAM at $20-$3F:$6000-$7FFF, mirrored $A0-$BF); fullsnes",
        ),
        Kind::Scored,
        None,
    )
}
