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
    vec![
        f1_01(),
        f1_02(),
        f1_04(),
        // F1.07 must precede F1.05 and F1.06: its first phase reads $4218 before anything has
        // armed auto-read, and those two arm it. Its guard catches the mistake rather than
        // hiding it, which is how this ordering was found.
        f1_07(),
        f1_05(),
        f1_06(),
        f1_14(),
    ]
}

/// With `$4200` bit 0 clear, `$4218`-`$421F` stop being written.
///
/// Auto-read is the hardware's own controller poll: once per frame, during the first vblank lines,
/// it clocks sixteen bits out of each port into `$4218`-`$421F`. Bit 0 of `$4200` is the only thing
/// that arms it, and with the bit clear those registers are simply not written — they hold whatever
/// was last put there, indefinitely. Software that disarms auto-read to poll by hand relies on
/// that: if the hardware kept writing, its own reads would fight the automatic ones.
///
/// # Proving a *non*-update needs the register to have been made to change
///
/// This is the shape that goes wrong quietly, and it did: an earlier attempt at this row was
/// written and withdrawn because every phase read `$0000`. `$4218` powers up at zero, and with
/// nothing held an auto-read writes zero, so "it did not change while auto-read was off" was
/// equally true of a core that never stopped reading.
///
/// The host input contract is what makes it assertable. Four buttons are held for the whole run, so
/// an auto-read writes `$9050` where the power-on state is `$0000`, and the difference between the
/// two is the sentinel:
///
/// | phase | `$4200.0` | `$4218` |
/// |---|---|---|
/// | A | 0 — never yet armed | `$0000`, the power-on state |
/// | B | **1** | `$9050` — a poll ran |
/// | C | **0** | still `$9050` — nothing wrote it |
///
/// The guard is `A != B`. If arming auto-read did not change the register, phase C's reading equals
/// phase A's for a reason that has nothing to do with `$4200`, and the assertion would pass on a
/// core that ignores bit 0 entirely. It also catches the ordering hazard directly: had some earlier
/// test left auto-read armed, phase A would already read `$9050` and this test would say so rather
/// than quietly measuring nothing.
///
/// # This test must run before anything else arms auto-read
///
/// Phase A's whole value is that `$4218` has never been written, so `F1.07` is ordered ahead of
/// `F1.05` and `F1.06`, which arm auto-read to read the result. Putting them first made phase A
/// read `$9050` and the guard fired — correctly, and with the message that names this exact
/// hazard. The ordering is a real constraint on the group, not a preference, and the guard is what
/// keeps it from becoming a silent one: a future test that arms auto-read earlier turns this into
/// a failure rather than into a test that quietly compares two identical readings.
///
/// # `$4200` is put back before anything is judged
///
/// The battery reads pads by hand through `$4016` and runs with `$4200 = $00`. A failure exits
/// through `test_restore`, which does not touch it, so the restore happens before the assertions
/// rather than after — the same ordering `F1.14` uses and for the same reason.
fn f1_07() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("--- A: auto-read has never been armed, so $4218 is still whatever power-on left ---");
    a.l("lda $4218");
    a.l("sta f:$7E01EA");
    a.c("--- B: armed for two whole frames, so a poll has certainly run ---");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $4200         ; auto-joypad enable, and nothing else");
    a.l("jsl wait_vblank_far");
    a.l("jsl wait_vblank_far");
    f1_settle_auto_read(&mut a, "armed");
    a.l("lda $4218");
    a.l("sta f:$7E01EC");
    a.c("--- C: disarmed for two more; nothing should write $4218 now ---");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("jsl wait_vblank_far");
    a.l("jsl wait_vblank_far");
    f1_settle_auto_read(&mut a, "off");
    a.l("lda $4218");
    a.l("sta f:$7E01EE");
    a.c("Put $4200 back before judging: the battery expects it zero and a failure leaves through");
    a.c("test_restore, which does not touch it.");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("rep #$30");
    a.l("lda f:$7E01EA");
    a.record(212, "F1.07 $4218 before auto-read was ever armed");
    a.l("lda f:$7E01EC");
    a.record(213, "F1.07 $4218 after two frames with auto-read armed");
    a.l("lda f:$7E01EE");
    a.record(214, "F1.07 $4218 after two more with it disarmed");
    a.c("The guard: without a change to hold onto, phase C proves nothing.");
    a.l("lda f:$7E01EA");
    a.l("cmp f:$7E01EC");
    a.fail_if_eq(
        "arming auto-read did not change $4218, so there is no sentinel for phase C to preserve \
         and 'it did not change' would be true of a core that polls regardless. Either the poll \
         is not running, or something earlier in the battery left auto-read armed and phase A \
         already held a polled value",
    );
    a.c("And with the enable clear, nothing writes those registers at all.");
    a.l("lda f:$7E01EE");
    a.l("cmp f:$7E01EC");
    a.fail_if_ne(
        "$4218 changed over two frames with $4200 bit 0 clear, so auto-read is running whether or \
         not it is armed — software that disarms it to hand-poll the ports would find its own \
         reads fighting the hardware's",
    );
    a.finish(
        "F1.07",
        'F',
        "Auto-read needs $4200.0",
        Provenance::Documented(
            "fullsnes and the SNESdev Wiki: bit 0 of $4200 arms the automatic joypad read, and \
             with it clear $4218-$421F are not written",
        ),
        Kind::Scored,
        None,
    )
}

/// A standard pad's signature nibble is `0000` — the four bits after its twelve buttons.
///
/// The controller shifts out sixteen bits: twelve buttons, then four more that identify what is
/// plugged in. A standard pad drives all four low. Everything else on the port announces itself
/// there instead — a mouse reports `0001`, an NTT Data keypad `0100` — so software distinguishes
/// peripherals by reading a nibble rather than by guessing from behaviour.
///
/// # Two halves, and the guard is the interesting one
///
/// The signature is the bottom four bits of `$4218`, and "they are zero" is satisfied by a core
/// that reports nothing at all. So the twelve button bits are checked first, against the host input
/// contract: `$9050` has `B`, `Start`, `X` and `R` held, and its bottom nibble is zero *because a
/// standard pad's signature is zero*, which is exactly the arrangement that lets one reading serve
/// both purposes.
///
/// | assertion | checks | catches |
/// |---|---|---|
/// | bits 15-4 == `$905` | the twelve button bits | a poll that never ran, or reported wrongly |
/// | bits 3-0 == `0000` | the signature nibble | an invented peripheral, or floating bits |
///
/// The guard deliberately **masks the signature nibble out**. Comparing the whole word against
/// `$9050` would be a strictly stronger check that happens to include the nibble — and then the
/// nibble assertion below could never fire, because anything that broke it would break the guard
/// first. An assertion that cannot fail is the same vacuity this battery keeps finding, arrived at
/// from a different direction, and it was arrived at here: injecting a false signature into the
/// core failed the guard and left the assertion the test is named for untouched.
fn f1_05() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    f1_auto_read(&mut a, "sig");
    a.l("lda $4218");
    a.l("sta f:$7E01F0");
    a.l("sep #$20");
    a.l("stz $4200         ; disarm before judging: the battery runs with auto-read off");
    a.l("rep #$30");
    a.l("lda f:$7E01F0");
    a.record(215, "F1.05 JOY1 after an armed auto-read");
    a.c("The guard: the twelve button bits only. Masking the nibble out is what leaves the");
    a.c("assertion below something of its own to catch.");
    a.l("lda f:$7E01F0");
    a.l("and #$FFF0");
    a.l("cmp #PAD_CONTRACT & $FFF0");
    a.fail_if_ne(
        "an armed auto-read did not report the buttons the host is holding, so the signature \
         nibble below is being read out of a register nothing wrote",
    );
    a.c("And the signature itself: four zeroes say 'standard pad' and nothing else.");
    a.l("lda f:$7E01F0");
    a.l("and #$000F");
    a.assert_a16_range(
        0x00,
        0x00,
        "a standard pad's four signature bits did not read as 0000 — a core reporting a non-zero \
         nibble is announcing a peripheral that is not there, and software that switches on the \
         signature would decode the pad as a mouse or a keypad",
    );
    a.finish(
        "F1.05",
        'F',
        "Pad signature is 0000",
        Provenance::Documented(
            "fullsnes and the SNESdev Wiki: bits 3-0 of the auto-read result identify the device, \
             and a standard controller reports 0000",
        ),
        Kind::Scored,
        None,
    )
}

/// The first bit clocked out is `B`, and it lands in bit 15 of `$4219`.
///
/// Auto-read shifts the same sixteen bits a manual read would, in the same order, into the same
/// place: the first bit out is the most significant bit of the sixteen-bit result. `F1.01` asserts
/// that order for a *manual* read; this asserts that the hardware's own read agrees with it, which
/// is not the same claim — a core could implement the two paths independently and have exactly one
/// of them backwards.
///
/// # Adjacent bits, so "stuck" and "correct" are distinguishable
///
/// The host contract holds `B` and does not hold `Y`, and they are the top two bits of the result.
/// Checking both means a byte of all ones fails as loudly as a byte of all zeroes:
///
/// | bit | button | held | expected |
/// |---|---|---|---|
/// | 15 | `B` | yes | 1 |
/// | 14 | `Y` | no | 0 |
///
/// Bit 15 alone would pass on a core that reports `$FF` for the high byte, which is precisely what
/// an unimplemented auto-read looks like on hardware where the line idles high.
fn f1_06() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    f1_auto_read(&mut a, "first");
    a.l("sep #$20");
    a.l("lda $4219");
    a.l("sta f:$7E01F2");
    a.l("stz $4200         ; disarm before judging");
    a.l("rep #$30");
    a.l("lda f:$7E01F2");
    a.l("and #$00FF");
    a.record(216, "F1.06 JOY1 high byte after an armed auto-read");
    a.c("B is held and is the first bit clocked, so it is the top bit of the result.");
    a.l("sep #$20");
    a.l("lda f:$7E01F2");
    a.l("and #$80");
    a.assert_a8(
        0x80,
        "bit 15 of the auto-read result was clear although the host is holding B, so the first bit \
         clocked is not landing in the most significant position",
    );
    a.c("And Y, the second bit and the one next to it, is not held — so the byte is not stuck.");
    a.l("lda f:$7E01F2");
    a.l("and #$40");
    a.assert_a8(
        0x00,
        "bit 14 was set although Y is not held. Bit 15 passed, so what this catches is a high byte \
         reading as all ones — an unimplemented auto-read on hardware whose line idles high looks \
         exactly like a correct one if only the top bit is checked",
    );
    a.finish(
        "F1.06",
        'F',
        "First bit clocked is B",
        Provenance::Documented(
            "fullsnes and the SNESdev Wiki: the auto-read result holds the sixteen shifted bits in \
             clock order, most significant first, so B is bit 15",
        ),
        Kind::Scored,
        None,
    )
}

/// Emit: arm auto-read and let two whole frames pass, so a poll has certainly run.
///
/// Two rather than one because a single frame can be entered part-way through — the test body
/// starts wherever the previous test left the beam — and a poll that has already happened this
/// frame will not happen again. `$4200` is written with bit 0 and nothing else, so no interrupt is
/// enabled as a side effect.
fn f1_auto_read(a: &mut Asm, tag: &str) {
    a.c(&format!("Arm auto-read and give it two frames ({tag})."));
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $4200");
    a.l("jsl wait_vblank_far");
    a.l("jsl wait_vblank_far");
    f1_settle_auto_read(a, tag);
}

/// Emit: burn well past the start of vblank, so the automatic read has finished writing.
///
/// `wait_vblank_far` returns at the *start* of vblank, which is exactly when the automatic read
/// begins — and it takes about three scanlines to clock thirty-two bits out of the ports. Reading
/// `$4218` immediately therefore samples a register the hardware is in the middle of filling.
///
/// This is not hypothetical. Mesen2 clears the result registers when the read starts and fills them
/// as it goes, so `F1.06` read `$4219` as `$00` there while passing on RustySNES and snes9x, both of
/// which write the result in one step. Two of three cores agreeing is not what makes the third
/// right — `F1.12` says results are valid by `V = $E3`, and a test that reads before then is
/// sampling a documented transient.
///
/// The burn is roughly ten thousand cycles, about seven scanlines: comfortably past the read and
/// comfortably inside vblank, which is thirty-eight lines even in 224-line NTSC.
fn f1_settle_auto_read(a: &mut Asm, tag: &str) {
    a.l("rep #$30");
    a.l("ldx #$0800");
    a.label(&format!("ar_{tag}"));
    a.l("dex");
    a.l(&format!("bne @ar_{tag}"));
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

/// The manual read order is `B Y Select Start Up Down Left Right A X L R`, MSB first.
///
/// Writing `1` then `0` to `$4016` latches the pad and starts the shift register; each subsequent
/// read returns the next button in a fixed order, most significant first. Every driver that polls
/// by hand depends on that order, and a core that has it wrong produces a game where the buttons
/// are simply the wrong buttons — a failure that is obvious to a player and invisible to a test
/// that only checks whether *something* was pressed.
///
/// # This is the row the host input contract exists for
///
/// With nothing held, all sixteen bits are zero and any order produces the same answer. The cart's
/// runners therefore hold `PAD_CONTRACT` — `B + Start + X + R`, or `$9050` — for the whole run
/// (`runtime.inc` documents the choice). Reading the sixteen bits MSB-first and comparing against
/// that value asserts the order outright: every held button is in a different position, and the
/// value is asymmetric under bit reversal, so an LSB-first implementation reads `$0A09` rather
/// than passing by accident.
///
/// | failure | reads |
/// |---|---|
/// | LSB first | `$0A09` |
/// | only the first byte reported | `$9000` |
/// | bytes swapped | `$5090` |
/// | a stuck-high line | `$FFFF` |
///
/// # The read is open-coded rather than calling the runtime's helper
///
/// `read_pad` in `runtime.s` does exactly this and leaves the result in `V_PAD_HELD`, and using it
/// would make this a test of the helper. The sixteen reads are written out here so what is asserted
/// is the hardware's order and not the runtime's agreement with itself.
fn f1_01() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Latch the pad, then clock sixteen bits into a 16-bit accumulator, MSB first.");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta JOYSER0");
    a.l("lda #$00");
    a.l("sta JOYSER0");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("sta f:$7E01E8");
    a.l("ldx #$0010");
    a.label("bit");
    a.l("sep #$20");
    a.l("lda JOYSER0");
    a.l("lsr               ; data bit into carry");
    a.l("rep #$20");
    a.l("lda f:$7E01E8");
    a.l("rol               ; and into the accumulator, most significant first");
    a.l("sta f:$7E01E8");
    a.l("dex");
    a.l("bne @bit");
    a.l("lda f:$7E01E8");
    a.record(211, "F1.01 the sixteen manual pad bits, MSB first");
    a.c("The host contract holds B, Start, X and R — four buttons in four different positions.");
    a.l("lda f:$7E01E8");
    a.l("cmp #PAD_CONTRACT");
    a.fail_if_ne(
        "the sixteen manually-clocked pad bits did not match the buttons the host is holding. \
         $0A09 means the shift register is being read least-significant-bit first, $9000 that only \
         the first byte is reported, $5090 that the two bytes are swapped, and $FFFF that the line \
         is stuck high",
    );
    a.finish(
        "F1.01",
        'F',
        "Manual pad read order",
        Provenance::Documented(
            "fullsnes and the SNESdev Wiki controller protocol: the shift register presents B, Y, \
             Select, Start, Up, Down, Left, Right, A, X, L, R and then four zero bits",
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
    a.c("Latch, then clock out the sixteen data bits, ANDing them together. The host input");
    a.c("contract holds four buttons and not the other eight, so at least one data bit must read");
    a.c("0 — which is the one thing that has to be true for 'the reads after them are all 1' to");
    a.c("say anything. Without it a core that returns 1 to every read passes below trivially.");
    a.l("lda #$01");
    a.l("sta JOYSER0");
    a.l("lda #$00");
    a.l("sta JOYSER0");
    a.l("lda #$01");
    a.l("sta f:$7E0100         ; the AND of the first sixteen reads");
    a.l("ldx #$10");
    a.label("data");
    a.l("lda JOYSER0");
    a.l("and #$01");
    a.l("and f:$7E0100");
    a.l("sta f:$7E0100");
    a.l("dex");
    a.l("bne @data");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x00,
        "every one of the sixteen data bits read as 1, so the line is stuck high and the reads \
         below would report that rather than the pad running out of data",
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
