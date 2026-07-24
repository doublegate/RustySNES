//! Group F — controller ports (ticket **T-04-F**).
//!
//! Part of the serial protocol — the latch, the shift register, what a pad returns once its
//! sixteen data bits are exhausted — is observable with a controller sitting untouched. The rest of
//! the group is not, and that is the group's defining constraint.
//!
//! **With nothing held, every controller observable the cart can reach is `$0000`**: the manual
//! shift register, the auto-read results in `$4218`-`$421F`, and their power-on state. A test of
//! the read order, of the signature nibble, or of what a *disarmed* auto-read preserves then has
//! nothing to distinguish it from anything else — it passes on a correct implementation and on
//! every broken one. `F1.07` was written and withdrawn for exactly that before the fix.
//!
//! The fix is a contract rather than a mechanism: **every runner holds controller 1 at
//! `PAD_CONTRACT` for the whole run** — the in-repo harness, the snes9x libretro driver, and the
//! Mesen2 script. `asm/runtime.inc` declares the value and explains why it is that value and not
//! another. Adding it found two RustySNES defects immediately, both invisible beforehand because
//! both wrong models also report `$0000` when nothing is held.
//!
//! `runtime.s` still reads `$4016` manually and holds `NMITIMEN` at zero between tests, so
//! auto-joypad read is off by default and nothing clocks a shift register behind a test's back.
//! That matters: the dossier records an AccuracyCoin test that failed spuriously because the menu's
//! own controller read left the register part-shifted, so a test here starts by putting the
//! register somewhere known rather than assuming it. A test that arms auto-read must disarm it
//! again *before* its assertions, since a failure exits through `test_restore`, which does not.
//!
//! **What is still out of reach is anything needing a different peripheral** — a mouse, a Super
//! Scope, a multitap, an NTT keypad — or a second controller: `F1.03` was built and withdrawn
//! because Mesen2's headless runner has no device in port 2 and setting one costs port 1 its input.
//! `docs/accuracysnes-plan.md` has the measurements and what each would take.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// Every Group F test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        f1_01(),
        f1_02(),
        f1_03(),
        f1_04(),
        // F1.07 must precede F1.05 and F1.06: its first phase reads $4218 before anything has
        // armed auto-read, and those two arm it. Its guard catches the mistake rather than
        // hiding it, which is how this ordering was found.
        f1_07(),
        f1_05(),
        f1_06(),
        f1_10(),
        f1_11(),
        f1_12(),
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
    f1_require_contract(&mut a, "c07");
    // Phase A reads $4218 as power-on leaves it, before anything arms auto-read. A menu Select
    // restart re-runs the battery without a real power-on, and the previous run's F1.05/F1.06 have
    // by then armed auto-read and left the pad state in $4218 -- so phase A can no longer read the
    // unwritten value and the "arming changed something" guard cannot fire. This is a genuine
    // power-on dependency, the same class as the Group G rows, so on a restart it stands down as
    // SKIP rather than reporting a failure that only means "this was not a cold boot".
    a.l("sep #$20");
    a.l("lda f:V_RESTARTED");
    a.l("beq :+");
    a.skip("phase A needs power-on $4218; a menu restart is not a power-on");
    a.l(":");
    a.l("rep #$30");
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
    f1_require_contract(&mut a, "c05");
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
    f1_require_contract(&mut a, "c06");
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

/// `$4016` bit 0 must stay low during the automatic read, or the result is corrupt.
///
/// Bit 0 of `$4016` is the ports' latch line. While it is high the shift registers do not shift —
/// they continuously reload from the button lines — so the automatic read, which clocks the line
/// thirty-two times during vblank, gets the *same* bit thirty-two times instead of sixteen
/// different ones. The result is not merely stale, it is uniform: every position holds whatever
/// `B` was.
///
/// This is why software that hand-polls `$4016` must either disarm auto-read first (`F1.07`) or
/// confine its strobing to outside the vblank window. A driver that strobes at the wrong moment
/// corrupts a register it is not even reading.
///
/// | phase | `$4016.0` during the read | `$4218` |
/// |---|---|---|
/// | A | 0 | `$9050` — the host contract, the control |
/// | B | **1** | uniform: `$FFFF` with `B` held, `$0000` without |
///
/// # Both halves of the contract are doing work here
///
/// The control has to be `$9050` exactly, or "phase B differs" could mean the poll never ran. And
/// the corruption is only *visible* because a button is held: with nothing pressed, a correct
/// uniform-`$0000` corruption and a correct `$0000` read are the same sixteen bits. That is the
/// same wall `F1.07` hit before the contract existed, and it is why this row is reachable now and
/// was not before.
///
/// # What is asserted is "differs", not a particular corruption
///
/// Which uniform value appears depends on which bit the latch happens to freeze and on how a core
/// models a shift register that is being reloaded while clocked — detail no source pins down. The
/// row's claim is that the result is wrong, so that is what is checked, with the raw value
/// published for comparison.
fn f1_11() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    f1_require_contract(&mut a, "c11");
    a.c("--- A: the control. Latch low throughout, so the read is the ordinary one ---");
    a.l("sep #$20");
    a.l("stz $4016");
    f1_auto_read(&mut a, "clean");
    a.l("lda $4218");
    a.l("sta f:$7E01F4");
    a.c("--- B: latch held high across the whole window ---");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $4016");
    f1_auto_read(&mut a, "latched");
    a.l("lda $4218");
    a.l("sta f:$7E01F6");
    a.c("Release the latch and disarm before judging: the battery hand-polls $4016 and expects");
    a.c("$4200 to be zero, and a failure leaves through test_restore, which touches neither.");
    a.l("sep #$20");
    a.l("stz $4016");
    a.l("stz $4200");
    a.l("rep #$30");
    a.l("lda f:$7E01F4");
    a.record(
        217,
        "F1.11 JOY1 with the latch low across the auto-read (the control)",
    );
    a.l("lda f:$7E01F6");
    a.record(218, "F1.11 JOY1 with the latch held high across it");
    a.c("The control first: without a correct read to compare against, 'differs' means nothing.");
    a.l("lda f:$7E01F4");
    a.l("cmp #PAD_CONTRACT");
    a.fail_if_ne(
        "the control auto-read did not report the buttons the host is holding, so the comparison \
         below is against a value that is already wrong",
    );
    a.c("And with the latch up, the shift register never shifts: the result must not survive.");
    a.l("lda f:$7E01F6");
    a.l("cmp #PAD_CONTRACT");
    a.fail_if_eq(
        "holding $4016 bit 0 high across the automatic read left $4218 correct, so the read is not \
         going through the ports' shift registers at all — a driver that strobes $4016 during \
         vblank would corrupt the auto-read results on hardware and not here, which is the more \
         dangerous way round",
    );
    a.finish(
        "F1.11",
        'F',
        "Latch corrupts auto-read",
        Provenance::Documented(
            "fullsnes and the SNESdev Wiki: while $4016 bit 0 is high the shift registers reload \
             continuously rather than shifting, so an automatic read taken across it returns the \
             same bit in every position",
        ),
        Kind::Scored,
        None,
    )
}

/// The automatic read does not begin at the vblank edge, so a `$4212` bit-0 poll at NMI entry sees it
/// **not yet started** — the race that lets a naive wait-loop read stale `$4218` data.
///
/// Hardware starts the automatic joypad read ~dot 32.5-95.5 into the first vblank line (fullsnes),
/// not at the vblank edge. So for the few dozen cycles between the edge and the start, `$4212` bit 0
/// reads **not-busy** even though auto-read is armed; a driver that samples `$4212` immediately in its
/// NMI handler and waits for busy=0 as "read done" can read the *previous* frame's result before the
/// current read has even begun.
///
/// # Making it non-vacuous
///
/// "not busy at entry" is also true of a *disarmed* auto-read that never runs — so the test would pass
/// on a core that simply never sets busy. Both halves are therefore load-bearing: phase B asserts the
/// armed read **does** set busy shortly after (so there is a real read to race), and phase A asserts it
/// was **not** already busy at vblank entry. A core that begins the read at the vblank edge fails
/// phase A; one that never starts it fails phase B.
///
/// snes9x performs the read as an instant latch and does not model this window, so it fails phase A —
/// a real snes9x inaccuracy, recorded in `crossval.sh`. RustySNES and Mesen2 both delay the start.
fn f1_10() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    f1_require_contract(&mut a, "c10");
    a.c("Arm auto-read; the read is scheduled at the vblank edge but only begins ~dot 64 into the line.");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $4200");
    a.c("Land at the very start of a fresh vblank line (H is small), BEFORE the read begins. Two waits");
    a.c("so a read is certainly armed for this vblank, and the second lands us early on the first");
    a.c("vblank line (V = 225 NTSC / 240 PAL -- wait_vblank_far keys on the region's own vblank start).");
    a.l("jsl wait_vblank_far");
    a.l("jsl wait_vblank_far");
    a.c(
        "--- A: read $4212 immediately. Armed but not yet started => bit 0 reads 0 (the race). ---",
    );
    a.l("lda $4212");
    a.l("and #$01");
    a.l("sta f:$7E01E0");
    a.c("--- B: spin until the read starts (busy sets), bounded so a read that never starts reports. ---");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.l("sep #$20");
    a.label("f10busy");
    a.l("lda $4212");
    a.l("and #$01");
    a.l("bne @f10started");
    a.l("rep #$20");
    a.l("inx");
    a.l("cpx #$0800         ; ~2048 samples span several scanlines");
    a.l("sep #$20");
    a.l("bne @f10busy");
    a.l("lda #$02           ; sentinel: the armed read never set busy");
    a.l("bra @f10store");
    a.label("f10started");
    a.l("lda #$01");
    a.label("f10store");
    a.l("sta f:$7E01E2");
    a.c("Disarm before judging: the battery hand-polls $4016 with $4200 = 0, and a failure exits");
    a.c("through test_restore, which touches neither $4200 nor $4016.");
    a.l("stz $4200");
    a.l("rep #$30");
    a.l("lda f:$7E01E0");
    a.record(
        247,
        "F1.10 $4212 bit 0 at vblank entry (0 = read not yet started -- the race)",
    );
    a.l("lda f:$7E01E2");
    a.record(
        248,
        "F1.10 $4212 bit 0 became busy later (1 = read started; 2 = never)",
    );
    a.c("Phase B first: without a read that actually starts, 'not busy at entry' proves nothing.");
    a.l("sep #$20");
    a.l("lda f:$7E01E2");
    a.l("cmp #$01");
    a.fail_if_ne(
        "the armed automatic read never set $4212 busy within several scanlines, so there is no read \
         to observe the start of -- 'not busy at vblank entry' would then be satisfied by a read that \
         simply never runs",
    );
    a.l("lda f:$7E01E0");
    a.l("cmp #$00");
    a.fail_if_ne(
        "$4212 bit 0 read busy at the very start of the vblank line, so the automatic read began at \
         the vblank edge rather than a few dozen cycles into the line -- a $4212 poll at NMI entry \
         cannot then observe the not-yet-started window, and a driver waiting for busy to clear would \
         read the previous frame's stale $4218 result",
    );
    a.finish(
        "F1.10",
        'F',
        "Auto-read start race",
        Provenance::Documented(
            "fullsnes: the automatic joypad read begins ~dot 32.5-95.5 of the first vblank line, not \
             at the vblank edge, so $4212 bit 0 reads not-busy for that window and a $4212 poll at \
             NMI entry sees the read not yet started",
        ),
        Kind::Scored,
        None,
    )
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

/// When does the automatic read's result become valid? A golden vector — the sources conflict.
///
/// `F1.12` says results are valid by `V = $E3` (227). `F1.09` says the read takes exactly 4224
/// master cycles, and `F1.08` puts its start between dot 32.5 and 95.5 of the first vblank line.
/// Those do not reconcile: vblank begins at line 225, 4224 cycles is 3.097 scanlines, and a read
/// starting at 225.05 finishes near 228.1 — *after* the line the first row says the answer is ready
/// on. No source states which of the two is the one to believe.
///
/// So this samples `$4218` at four positions across vblank and publishes what it finds:
///
/// | slot | `V` | why this line |
/// |---|---:|---|
/// | 219 | 225 | the first vblank line — the read has just started, if it has |
/// | 220 | 227 | `$E3`, the line `F1.12` names |
/// | 221 | 230 | past `F1.09`'s arithmetic, whichever way it is read |
/// | 222 | 240 | late enough that nothing plausible is still in flight |
///
/// # Only the last reading is asserted
///
/// That the result is *eventually* the buttons being held is not in doubt and is what makes the
/// other three interpretable — without it, four identical wrong values would look like a very
/// stable answer. What is deliberately **not** asserted is where the value first appears, because
/// that is exactly the quantity the sources disagree about, and the cores disagree with each other
/// too: RustySNES and snes9x write the result in one step, while Mesen2 clears the registers when
/// the read starts and fills them as it goes. Both are defensible models of an interval nobody
/// observes directly.
///
/// # What the measurement says, and it favours `F1.09`
///
/// RustySNES and snes9x report `$9050` at all four positions — they write the result in one step,
/// so there is no interval to observe. Mesen2 shows the fill happening:
///
/// | `V` | Mesen2's `$4218` |
/// |---:|---|
/// | 225 | `$00` — cleared, read in progress |
/// | **227** | `$82` — **partially filled** |
/// | 230 | `$50` — settled |
/// | 240 | `$50` |
///
/// So on the only core that models the interval at all, the result is **not** valid at `V = $E3`.
/// That is the boundary `F1.12` states, and this measurement contradicts it while agreeing with
/// `F1.09`'s arithmetic — 4224 cycles from a start on line 225 finishes near line 228. The row's
/// stated boundary appears to be wrong, and nothing here asserts it either way; the numbers are
/// published so a later reader can weigh them against whatever settles the conflict.
///
/// This is also the row that explains why every other Group F test settles about seven scanlines
/// past vblank before reading. `F1.06` was written without that settle and failed on Mesen2 alone,
/// which looked like a Mesen2 defect and was a cart reading a documented transient.
fn f1_12() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    f1_require_contract(&mut a, "c12");
    a.c("Arm auto-read, then take the frame after next so a poll has certainly begun.");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $4200");
    a.l("jsl wait_vblank_far");
    a.l("jsl wait_vblank_far");
    a.l("jsl wait_vblank_far   ; and land on the start of a fresh vblank");
    for (line, dest) in [
        (225u16, 0x7E_01F8u32),
        (227, 0x7E_01FA),
        (230, 0x7E_01FC),
        (240, 0x7E_01FE),
    ] {
        f1_12_spin(&mut a, line);
        a.l("rep #$30");
        a.l("lda $4218");
        a.l(&format!("sta f:${dest:06X}"));
    }
    a.l("sep #$20");
    a.l("stz $4200         ; disarm before judging; the battery runs with auto-read off");
    a.l("rep #$30");
    for (slot, line, src) in [
        (219u16, 225u16, 0x7E_01F8u32),
        (220, 227, 0x7E_01FA),
        (221, 230, 0x7E_01FC),
        (222, 240, 0x7E_01FE),
    ] {
        a.l(&format!("lda f:${src:06X}"));
        a.record(slot, &format!("F1.12 $4218 sampled at V = {line}"));
    }
    a.c("Only the last is asserted: four identical wrong values would otherwise read as a very");
    a.c("stable answer, and it is the settled value that makes the earlier three interpretable.");
    a.l("lda f:$7E01FE");
    a.l("cmp #PAD_CONTRACT");
    a.fail_if_ne(
        "even fifteen scanlines into vblank the automatic read had not produced the buttons the \
         host is holding, so the three earlier samples say nothing about when a correct result \
         appears — they are three samples of a result that never arrived",
    );
    a.l("sep #$20");
    a.l("lda #$03          ; variant 1 = captured; slots 219-222 say when the value settled");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.finish(
        "F1.12",
        'F',
        "Auto-read result timing",
        Provenance::Contested(
            "F1.12 says results are valid by V = $E3, which does not reconcile with F1.09's \
             4224-cycle duration and F1.08's start window; no source says which to believe, and \
             the cores split on whether the result appears at once or progressively",
        ),
        Kind::Golden,
        None,
    )
}

/// Emit: spin until the V counter reads `line`, for [`f1_12`].
///
/// `$213F` resets the counter read flipflops, `$2137` latches H and V together, and `$213D` is then
/// read twice for the nine-bit value. Group F carries its own copy rather than reaching into the
/// PPU group's, following `dma.rs`'s precedent — the emitted cheap-local labels have to be unique
/// per group and a shared helper would need a tag argument threaded through for no benefit.
fn f1_12_spin(a: &mut Asm, line: u16) {
    a.c(&format!("Spin until V = {line}."));
    a.l("sep #$20");
    a.label(&format!("wv{line}"));
    a.l("lda $213F         ; reset the counter read flipflops");
    a.l("lda $2137         ; latch H and V");
    a.l("lda $213D         ; V low");
    a.l("xba");
    a.l("lda $213D");
    a.l("and #$01          ; bit 0 is V bit 8; bits 1-7 are PPU2 open bus");
    a.l("xba");
    a.l("rep #$20");
    a.l("and #$01FF");
    a.l(&format!("cmp #{line}"));
    a.l("sep #$20");
    a.l(&format!("bne @wv{line}"));
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

/// Emit: stand the test down as SKIP unless the host is holding `PAD_CONTRACT`.
///
/// **Every Group F test that asserts against the contract must call this first.** The contract is a
/// property of the *host*, not of the machine, and the three cross-validation runners are the only
/// hosts that implement it. Run the cartridge in an ordinary emulator, or on hardware with a pad
/// sitting untouched, and those tests were asserting against buttons nobody was holding — six of
/// them reported FAIL, on a cart whose entire value is that a failure means something.
///
/// A test that depends on host configuration has to *detect its absence*, exactly like the
/// armed-ness guards elsewhere in the battery. This is that guard, and it was missing from the
/// contract's own tests until the cartridge was run outside the harness for the first time.
///
/// The read is open-coded rather than reusing `read_pad`: the runtime's copy runs once per frame in
/// the menu, and a test asking "is the contract held *now*" should look now.
fn f1_require_contract(a: &mut Asm, tag: &str) {
    a.c("Skip unless the host holds the input contract — see f1_require_contract.");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta $4016");
    a.l("lda #$00");
    a.l("sta $4016");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("sta f:$7E01E6");
    a.l("ldx #$0010");
    a.label(&format!("rq_{tag}"));
    a.l("sep #$20");
    a.l("lda $4016");
    a.l("lsr");
    a.l("rep #$20");
    a.l("lda f:$7E01E6");
    a.l("rol");
    a.l("sta f:$7E01E6");
    a.l("dex");
    a.l(&format!("bne @rq_{tag}"));
    a.l("lda f:$7E01E6");
    a.l("cmp #PAD_CONTRACT");
    a.l("beq :+");
    a.skip(
        "the host is not holding PAD_CONTRACT, so there is nothing for this row to assert against",
    );
    a.l(":");
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
    f1_require_contract(&mut a, "c01");
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

/// One write to `$4016` bit 0 latches BOTH controller ports' shift registers.
///
/// The latch line is shared: a single `$4016.0` strobe parallel-loads controller 1 AND controller 2
/// at once. The host input contract holds port 1 at `$9050` and port 2 at `$60A0` — two masks that
/// share no set bit — so after a single latch, reading each port must return its OWN value. A core
/// that latches only port 1 leaves port 2 unloaded (it reads `$0000`); a core that echoes port 1
/// onto port 2 returns `$9050` where `$60A0` is held. The two distinct masks catch both failures,
/// which is exactly why the contract puts a disjoint mask on port 2.
///
/// This needs a second controller in port 2. The in-repo harness (`set_joypad(1, …)`) and the snes9x
/// libretro driver (`input_state` for port 1) hold it directly; Mesen2's headless `--testrunner`
/// gets a port-2 device from `--snes.port2.type=SnesController` (see `scripts/accuracysnes/crossval.sh`).
/// Without that device an earlier version of this test was withdrawn — `runtime.inc` records the history.
fn f1_03() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    f1_require_contract(&mut a, "c03");
    a.c("One shared latch, then clock BOTH ports out together, MSB first: read $4016 (port 1) and");
    a.c("$4017 (port 2) once each per bit. Both words therefore come from the single latch above.");
    a.l("sep #$20");
    a.l("lda #$01");
    a.l("sta JOYSER0        ; $4016.0 high: latch both ports at once");
    a.l("lda #$00");
    a.l("sta JOYSER0        ; low: begin clocking");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("sta f:$7E01E8      ; port 1 accumulator");
    a.l("sta f:$7E01EA      ; port 2 accumulator");
    a.l("ldx #$0010");
    a.label("bit");
    a.l("sep #$20");
    a.l("lda JOYSER0        ; port 1 data bit");
    a.l("lsr");
    a.l("rep #$20");
    a.l("lda f:$7E01E8");
    a.l("rol");
    a.l("sta f:$7E01E8");
    a.l("sep #$20");
    a.l("lda JOYSER1        ; port 2 data bit — same shared latch, clocked independently");
    a.l("lsr");
    a.l("rep #$20");
    a.l("lda f:$7E01EA");
    a.l("rol");
    a.l("sta f:$7E01EA");
    a.l("dex");
    a.l("bne @bit");
    a.l("lda f:$7E01E8");
    a.record(235, "F1.03 port 1 word after a single $4016 latch");
    a.l("lda f:$7E01EA");
    a.record(238, "F1.03 port 2 word after the same latch");
    a.c("Guard: port 1 must read its own contract, or the manual read is broken and the port-2");
    a.c("assertion below would be measuring nothing.");
    a.l("lda f:$7E01E8");
    a.l("cmp #PAD_CONTRACT");
    a.fail_if_ne(
        "port 1 did not read $9050 after the latch, so the shared-latch reading of port 2 below is \
         not trustworthy — the manual read itself is broken",
    );
    a.c("The assertion: the SAME $4016 write latched port 2 too.");
    a.l("lda f:$7E01EA");
    a.l("cmp #PAD2_CONTRACT");
    a.fail_if_ne(
        "port 2 did not read $60A0 after a single $4016 latch. $0000 means the latch is not shared \
         (port 2 was never loaded); $9050 means the core echoes port 1 onto port 2",
    );
    a.finish(
        "F1.03",
        'F',
        "Shared $4016 latch",
        Provenance::Documented(
            "fullsnes and the SNESdev Wiki controller protocol: bit 0 of $4016 is the shared latch \
             line that parallel-loads both controller ports' shift registers",
        ),
        Kind::Scored,
        None,
    )
}
