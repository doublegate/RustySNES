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
    vec![f1_02()]
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
