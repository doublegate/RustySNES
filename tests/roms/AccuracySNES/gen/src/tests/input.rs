//! Group F — controller ports (ticket **T-04-F**).
//!
//! The first group whose assertions need no button to be *pressed*: the serial protocol itself —
//! the latch, the shift register, what a pad returns once its sixteen data bits are exhausted — is
//! all observable with a controller sitting untouched.
//!
//! The mechanism was already here. `runtime.s` reads `$4016` manually and holds `NMITIMEN` at zero
//! for the whole battery, so auto-joypad read is off and nothing clocks a shift register behind a
//! test's back. That matters: the dossier records an AccuracyCoin test that failed spuriously
//! because the menu's own controller read left the register part-shifted, so a test here starts by
//! putting the register somewhere known rather than assuming it.
//!
//! **What is not reachable is anything that depends on which peripheral is plugged in.** The cart
//! cannot tell "no controller" from "pad past bit 16" — both read as 1 — so an assertion about a
//! port's *identity* is really an assertion about the host's configuration, and the three hosts
//! disagree about theirs. `docs/accuracysnes-plan.md` has the measurements and what a peripheral
//! contract would have to say.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// Every Group F test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![f1_02(), f1_04(), f1_14()]
}

/// `$4016` bits 7-2 are CPU open bus: nothing on the controller port drives them.
///
/// Only bits 1 and 0 carry controller data — port 1's two data lines. The rest of the byte is not
/// driven by anything, so it reads back as whatever the CPU last left on the bus. A core that
/// returns `0` or `1` in those bits instead is inventing a value, and software that masks with
/// `$01` never notices until something reads the whole byte.
///
/// # Asserting it without asserting a particular open-bus model
///
/// "Bits 7-2 equal the open bus" cannot be checked directly, because what the open bus *holds* at
/// the moment of the read is a modelling question this test has no business settling — `C3.11`
/// found RustySNES and snes9x presenting PPU1's latch for `$2137` where Mesen2 presents the CPU's,
/// and the same disagreement is available here.
///
/// What is checkable is that the bits **follow** the bus rather than being manufactured. So the
/// register is read twice through addressing modes whose operand fetches differ in their last byte:
///
/// | read | encoding | last byte fetched before the data cycle |
/// |---|---|---|
/// | `lda $4016` | `AD 16 40` | `$40` |
/// | `lda f:$004016` | `AF 16 40 00` | `$00` |
///
/// A core whose bits 7-2 are open bus reports the operand byte in each case; one that hardcodes
/// them reports the same value twice, whatever that value is.
///
/// Measured, **all three cores return exactly `$41` and `$01`** — so this is scored rather than
/// recorded, and it asserts the two expected bytes rather than merely that they differ.
///
/// # Why it can be scored while the rest of Group F cannot
///
/// `docs/accuracysnes-plan.md` records Group F as blocked on a *peripheral contract*: the cart
/// cannot tell "no controller" from "pad past bit 16", so what each host has plugged in becomes
/// part of the expected value and the three hosts disagree. That blocker applies to bits 1 and 0.
/// It does not apply here, because **this test masks them off**: bits 7-2 are driven by nothing at
/// all, which is precisely the row's claim. Half of Group F is reachable on the same terms.
///
/// `$4017` is deliberately not tested here: its bits 4-2 are pulled high rather than floating, so
/// the row's blanket "bits 7-2" does not describe it and a test that assumed it would be wrong
/// about real hardware.
fn f1_04() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Two reads of the same register, differing only in the addressing mode. The operand bytes");
    a.c("are the last thing on the bus before each data cycle, and they end differently.");
    a.l("sep #$20");
    a.l("lda $4016         ; AD 16 40 — last operand byte $40");
    a.l("sta f:$7E01D0");
    a.l("lda f:$004016     ; AF 16 40 00 — last operand byte $00");
    a.l("sta f:$7E01D1");
    a.l("rep #$30");
    a.l("lda f:$7E01D0");
    a.l("and #$00FF");
    a.record(157, "F1.04 $4016 read as absolute (operand high byte $40)");
    a.l("lda f:$7E01D1");
    a.l("and #$00FF");
    a.record(158, "F1.04 $4016 read as long (operand bank byte $00)");
    a.c(
        "Bits 7-2 only. Bits 1-0 are the controller's, and what is plugged in is a property of the",
    );
    a.c("host rather than of the hardware -- masking them off is what lets this be scored at all.");
    a.l("sep #$20");
    a.l("lda f:$7E01D0");
    a.l("and #$FC");
    a.assert_a8(
        0x40,
        "$4016 read as absolute did not return the operand high byte ($40) in bits 7-2, so those \
         bits are not following the CPU bus",
    );
    a.l("lda f:$7E01D1");
    a.l("and #$FC");
    a.assert_a8(
        0x00,
        "$4016 read as long did not return the operand bank byte ($00) in bits 7-2. Equal to the \
         absolute read's $40 means the bits are manufactured rather than open bus",
    );
    a.finish(
        "F1.04",
        'F',
        "$4016 bits 7-2 open bus",
        Provenance::Corroborated(
            "RustySNES, snes9x and Mesen2 all return $41 for the absolute read and $01 for the \
             long one -- identical bytes, so bits 7-2 follow the CPU bus in all three",
        ),
        Kind::Scored,
        None,
    )
}

/// `$4213` reads back the `$4201` output latch, wired-AND with whatever the pins are being driven to.
///
/// `$4201` (WRIO) drives two pins — bit 6 to controller port 1's IOBIT, bit 7 to port 2's. `$4213`
/// (RDIO) reads those pins back, and the port is **open-collector**: a device on either port can
/// pull its pin low, but nothing can pull one high. So the value read is the AND of what was
/// written with what the outside world is doing, and with nothing driving the pins low it is simply
/// what was written.
///
/// A standard pad drives neither pin, which is what makes this testable without the peripheral
/// contract Group F otherwise needs: the assertion is about the *latch and its read-back path*, and
/// a controller that pulled a pin low would be a different test.
///
/// # Three values, chosen so a stuck bit cannot hide
///
/// `$FF`, `$00` and `$55` in turn. The first two catch a core that returns a constant either way;
/// `$55` catches one that returns "all bits the same" — a mask, a boolean, or the two IOBIT pins
/// smeared across the byte. A core ignoring `$4201` entirely fails the first comparison it reaches.
///
/// # `$4201` is restored before anything is asserted, and that is not tidiness
///
/// Bit 7 gates the `$2137` counter latch (`C3.10`), which a dozen later tests depend on. A failure
/// exits through `test_restore`, which deliberately does not touch `$4201` — so leaving it at `$00`
/// or `$55` here would break the H/V latch for the rest of the battery and turn one failure into a
/// cascade. The readings are taken, the register is put back to `$FF`, and only then are the values
/// judged.
fn f1_14() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    for (val, dest) in [(0xFFu8, 0x7E_01D4u32), (0x00, 0x7E_01D5), (0x55, 0x7E_01D6)] {
        a.l(&format!("lda #${val:02X}"));
        a.l("sta $4201");
        a.l("lda $4213");
        a.l(&format!("sta f:${dest:06X}"));
    }
    a.c("Restore WRIO before judging anything: bit 7 gates the $2137 counter latch that a dozen");
    a.c("later tests use, and a failure exits without passing through here again.");
    a.l("lda #$FF");
    a.l("sta $4201");
    a.l("rep #$30");
    a.l("lda f:$7E01D4");
    a.l("and #$00FF");
    a.record(159, "F1.14 $4213 after writing $4201 = $FF");
    a.l("lda f:$7E01D5");
    a.l("and #$00FF");
    a.record(160, "F1.14 $4213 after writing $4201 = $00");
    a.l("lda f:$7E01D6");
    a.l("and #$00FF");
    a.record(161, "F1.14 $4213 after writing $4201 = $55");
    a.l("sep #$20");
    a.l("lda f:$7E01D4");
    a.assert_a8(
        0xFF,
        "$4213 did not read back the $FF written to $4201, so the output latch is not reaching the \
         read-back path",
    );
    a.l("lda f:$7E01D5");
    a.assert_a8(
        0x00,
        "$4213 did not read back the $00 written to $4201: a core returning $FF here is reporting \
         the pull-ups rather than the latch",
    );
    a.l("lda f:$7E01D6");
    a.assert_a8(
        0x55,
        "$4213 did not read back $55. Both earlier values passed, so the path works for all-ones \
         and all-zeroes but not for a mixed pattern — the bits are not independent",
    );
    a.finish(
        "F1.14",
        'F',
        "$4213 reads $4201 back",
        Provenance::Documented(
            "fullsnes: RDIO reads the WRIO output pins, which are open-collector, so with nothing \
             driving them low the value read is the value written",
        ),
        Kind::Scored,
        None,
    )
}

/// A standard pad returns 1 once its sixteen data bits are gone.
///
/// The pad is a parallel-load shift register: `$4016.0` high loads it from the button lines, low
/// starts clocking, and after sixteen bits there is nothing left driving the line low — so reads
/// seventeen onward return 1. Software identifies peripherals by exactly this, because a multitap
/// and a mouse keep sending, so a core that returns 0 forever makes every device look alike.
///
/// The sixteen data bits are checked first, as the vacuity guard: with nothing held they must all
/// be 0, so a core that returns 1 to *every* read — which would sail through the assertion below —
/// fails here instead.
///
/// **This test found a RustySNES defect the frontend could not.** The gamepad's shift register was
/// the button word itself, so the strobe never reloaded it: the first manual read of a frame
/// consumed the buttons and every later one returned all-ones, and a manual read also corrupted the
/// auto-read result at `$4218-$421F`. A frontend rewrites the button state every frame, which hid
/// both — a game that polls twice in one frame would not have been so lucky.
fn f1_02() -> Test {
    let mut a = Asm::new();
    a.l("sep #$20");
    a.c("Latch, then clock out the sixteen data bits, ORing them together. Nothing is pressed, so");
    a.c("the OR must be 0 — without this a core that returns 1 to every read would pass below.");
    a.l("lda #$01");
    a.l("sta JOYSER0");
    a.l("lda #$00");
    a.l("sta JOYSER0");
    a.l("lda #$00");
    a.l("sta f:$7E0100         ; the OR of the first sixteen reads");
    a.l("ldx #$10");
    a.label("data");
    a.l("lda JOYSER0");
    a.l("and #$01");
    a.l("ora f:$7E0100");
    a.l("sta f:$7E0100");
    a.l("dex");
    a.l("bne @data");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x00,
        "a button read as pressed during the sixteen data bits, so the reads below say nothing \
         about what follows them",
    );
    a.c("Reads 17-20. Every one must be 1: the pad has nothing left to send.");
    a.l("lda #$01");
    a.l("sta f:$7E0100         ; the AND of the next four reads");
    a.l("ldx #$04");
    a.label("ones");
    a.l("lda JOYSER0");
    a.l("and #$01");
    a.l("and f:$7E0100");
    a.l("sta f:$7E0100");
    a.l("dex");
    a.l("bne @ones");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x01,
        "a read past the sixteenth returned 0 — an official pad drives the line high once its \
         data bits are exhausted, and peripherals are identified by not doing so",
    );
    a.finish(
        "F1.02",
        'F',
        "Pad reads 17+ are 1",
        Provenance::Documented("SNESdev Wiki, controller protocol; fullsnes"),
        Kind::Scored,
        None,
    )
}
