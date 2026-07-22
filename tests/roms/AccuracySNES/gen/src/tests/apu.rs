//! Group E — the SPC700 and S-DSP (ticket **T-04-E**).
//!
//! Every test here has the same shape, because the SPC700 is only reachable through four bytes:
//! the cart uploads a small SPC700 program via the IPL boot handshake (`apu_upload` in
//! `asm/runtime.s`), waits for the program to publish a done marker on port 0, and reads its
//! answers off the other three ports. That is what a game's sound driver does at boot, which is
//! why the IPL ROM exists at all.
//!
//! The programs themselves are assembled by `gen/src/spc.rs` — `ca65` does not speak SPC700.
//!
//! **Never hand-write a verdict byte.** Use the assertion helpers, even when the condition does
//! not look like an equality — `assert_a16_range` covers "must not be this value" perfectly well.
//! A hand-written `sta V_TEST_RESULT` puts a failure code in the ROM that the generated
//! `ERROR_CODES.md` cannot know about, so the table silently stops being a complete account of
//! what a failure byte means. This has been got wrong twice in this file; the helpers exist
//! precisely so it cannot be.
//!
//! **Reading `PSW` is the recurring trick.** Several of these assertions are about which flags an
//! instruction sets, and the SPC700 has no "read flags" instruction. `PUSH PSW` / `POP A` does it,
//! but only if nothing between the instruction under test and the push disturbs the flags — which
//! is why the result registers are captured with `MOV dp,A` and `MOV dp,Y`, the two moves that
//! leave flags alone.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::dsl::{Asm, Kind, Provenance, Test};
use crate::spc::{DONE, PORT0, PORT1, PORT2, PORT3, RELEASE, Spc};

/// Hands out a unique suffix for each uploaded SPC700 image's label.
///
/// The images live in a shared segment, so their labels have to be globally unique; a counter is
/// enough because generation is single-threaded and runs the tests in a fixed order. The number
/// means nothing beyond "not the same as the last one".
fn next_prog_id() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Every Group E test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        // E4.11 MUST stay first: APU RAM's power-on contents survive only until something writes
        // over them, so this is a once-only observation. It is a golden vector precisely so that a
        // mis-ordered run reports a strange recording rather than failing the battery.
        e4_11(),
        e1_01(),
        e1_02(),
        e1_07(),
        e1_04(),
        e1_05(),
        e1_06(),
        e1_13(),
        e1_15(),
        e3_01(),
        e3_02(),
        e3_11(),
        dsp_addressing(),
        e2_01(),
        e2_05(),
        e6_09(),
        e6_11(),
        e10_01(),
        e10_05(),
        e1_14(),
        e2_04(),
        e3_14(),
        dsp_global_regs(),
        e9_19(),
        e9_03(),
        e9_01(),
        e5_07(),
        e5_08(),
        e5_09(),
        e5_01(),
        e5_10(),
        e5_11(),
        e7_10(),
        e1_03(),
        e1_08(),
        e2_08(),
        e2_09(),
        e3_03(),
        e3_04(),
        e3_05(),
        e3_10(),
        e1_09(),
        e1_10(),
        e1_12(),
        e2_02(),
        e2_03(),
        e2_06(),
        e2_07(),
        e4_01(),
        e4_02(),
        e4_03(),
        e4_04(),
        e5_12(),
        e5_02(),
        e7_16(),
        e8_04(),
        e9_04(),
        e9_06(),
        e9_12(),
        e9_15(),
        e9_05(),
        e9_13(),
        e9_10(),
        e9_17(),
        e9_18(),
        e5_03(),
        e5_04(),
        e5_05(),
        e7_01(),
        e7_13(),
        e7_04(),
        e7_09(),
        e7_05(),
        e7_06(),
        e7_07(),
        e7_03(),
        e7_12(),
        e7_08(),
        e8_02(),
        e8_03(),
        e8_07(),
        e8_10(),
        e7_11(),
        e7_14(),
        e7_15(),
        e6_02(),
        e6_02b(),
        e6_02c(),
        e6_02d(),
        e3_06(),
        e3_08(),
    ]
}

/// Emit the cart-side half: upload `prog`, wait for its done marker, leave port values readable.
///
/// The wait is bounded by a counter rather than spinning forever. An APU that never boots is a
/// real failure mode — it is the one thing here the cart cannot recover from — and a test that
/// hangs takes the whole battery with it, reporting nothing at all about the other tests.
///
/// **Register widths on exit: `A` 8-bit, `X`/`Y` 16-bit**, on the path that reaches `@ran`. The
/// caller's `.a8`/`.a16` directives come from its own `sep`/`rep` lines and a helper call is not
/// one of those, so an undocumented width here would have the assembler and the CPU disagreeing
/// about the size of the next immediate — and every instruction after it shifted.
fn upload_and_run(a: &mut Asm, prog: &Spc) {
    upload_and_run_tagged(a, prog, "");
}

/// [`upload_and_run`] with a suffix on its cheap-local labels, so one test can run two programs.
///
/// `E4.03` needs exactly that: one program to dirty the APU zero page and a second to observe that
/// the IPL cleaned it on the way back in. Without distinct labels the second upload redefines
/// `@wait` and `@ran` and the assembler rejects the file.
fn upload_and_run_tagged(a: &mut Asm, prog: &Spc, tag: &str) {
    // The image goes in the out-of-bank data segment, not inline in the test body: these are
    // several hundred bytes each and bank $00 is finite. `apu_upload` takes a 24-bit pointer
    // anyway, so nothing about the upload cares where it lives.
    let label = format!("apu_prog_{}", next_prog_id());
    a.d(&format!("{label}:"));
    for line in prog.as_ca65("    ").lines() {
        a.d(line);
    }
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("Point apu_upload at this test's own program image, which lives in another bank.");
    a.l(&format!("lda #.loword({label})"));
    a.l("sta f:V_APU_SRC");
    a.l("sep #$20");
    a.l(&format!("lda #^{label}"));
    a.l("sta f:V_APU_BANK");
    a.l("rep #$30");
    a.l(&format!("lda #{}", prog.bytes().len()));
    a.l("sta f:V_APU_LEN");
    a.l("lda #$0200");
    a.l("sta f:V_APU_DEST     ; APU RAM $0200: clear of the zero page and the stack");
    a.l("lda #$0200");
    a.l("sta f:V_APU_ENTRY");
    a.l("jsl apu_upload_far");
    a.c("Clear the CPU-side port 0 before the program can look at it. The previous test left the");
    a.c("release byte there, and a program whose release loop sees it immediately jumps back to");
    a.c("the IPL before the cart has read a thing — which reads as a wrong answer, not a race.");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l("sta APUIO0");
    a.c("Wait for the program's done marker, but not forever: an APU that never boots would");
    a.c("otherwise hang the whole battery and report nothing about any other test.");
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label(&format!("wait{tag}"));
    a.l("sep #$20");
    a.l("lda APUIO0");
    a.l(&format!("cmp #${DONE:02X}"));
    a.l(&format!("beq @ran{tag}"));
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$8000");
    a.l(&format!("bne @wait{tag}"));
    // `jmp`, not `bra`: a test that runs two programs (E4.03) puts the whole second upload
    // between this branch and the shared timeout arm, which is well past a branch's reach.
    a.l("jmp @timeout");
    a.label(&format!("ran{tag}"));
    a.c(
        "Copy the answers out BEFORE releasing the program: once it jumps to the IPL, the boot ROM",
    );
    a.c("overwrites ports 0 and 1 with its $AA/$BB announcement.");
    a.l("sep #$20");
    a.l("lda APUIO1");
    a.l("sta f:$7E0100");
    a.l("lda APUIO2");
    a.l("sta f:$7E0101");
    a.l("lda APUIO3");
    a.l("sta f:$7E0102");
    a.c("Release: the program hands the APU back to the IPL so the NEXT test can upload at all.");
    a.l(&format!("lda #${RELEASE:02X}"));
    a.l("sta APUIO0");
}

/// Emit the shared tail: jump past the timeout arm, then land where `finish`'s pass stub follows.
///
/// Every test in this group needs it because `upload_and_run` branches to `@timeout` when the APU
/// never answers, and that arm has to record SKIP and leave — a test whose APU did not boot has
/// asserted nothing, and reporting a pass would be a lie about the only thing it was measuring.
fn apu_timeout_arm(a: &mut Asm) {
    a.l("bra @pass");
    a.label("timeout");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("sta f:V_TEST_RESULT   ; SKIP: the APU never published a done marker");
    a.l("jml test_restore");
    a.label("pass");
}

/// The IPL boot ROM zero-fills APU RAM `$0000-$00EF` before handing control to a program.
///
/// A driver may assume its zero page starts clear, and on a core that skips the fill it instead
/// starts with whatever the RAM powered up holding — which `E4.11` records as a repeating
/// `32x$00, 32x$FF` pattern, so half of it is `$FF`. That is the difference between a silent driver
/// and a screaming one.
///
/// # This test must run before any other APU program, and the reason is mechanical
///
/// The fill happens once, at APU **reset**. Releasing a program back to the IPL re-enters its
/// transfer loop, not its reset path, so the zero page is never refilled for the rest of the
/// session. Every later program in this group runs with its direct page at `$00` and writes
/// variables there — `$01`, `$10`-`$15` and `$20` are all in use across `E1`-`E9`. By the time any
/// of them has run, "is the zero page clear?" has a different and uninteresting answer.
///
/// So this is the **first** entry in [`all`], and that placement is load-bearing rather than
/// cosmetic. The same coupling `E4.02` documents for the `PSW` handoff, one step earlier.
///
/// # It reports two halves so a failure says which kind it is
///
/// The range is swept in two loops and reported separately: `$00-$1F`, which is exactly where the
/// other programs in this group keep their variables, and `$20-$EF`, which nothing in the battery
/// touches. A failure confined to the low half is almost certainly a test-ordering accident — this
/// test running after something else — and the failure message says so. A failure in the high half
/// cannot be that, and means the fill genuinely did not happen.
///
/// # Shape
///
/// Only backward branches exist in the `Spc` builder, deliberately, so there is no early exit: each
/// loop ORs every byte in its range into `A` and the accumulated value is reported at the end. A
/// single non-zero bit anywhere in a range shows up in that range's OR. The program touches no
/// direct-page address itself — its prologue only sets `SP`, and its results go to the port
/// registers at `$F4`-`$F7`, which are not RAM.
/// The IPL boot ROM zero-fills APU RAM `$0000-$00EF` every time it is entered at `$FFC0`.
///
/// A driver may assume its zero page starts clear, and on a core that skips the fill it instead
/// starts with whatever the RAM held before — which is the difference between a silent driver and a
/// screaming one.
///
/// # The obvious version of this test cannot fail, and that is the whole story here
///
/// It was first written as "upload a program, check the zero page is zero", placed first in the
/// group so no other program could have dirtied it. It passed everywhere — and on two of the three
/// cores it proved nothing. **RustySNES and snes9x both boot APU RAM as all-zero**, so a core that
/// never ran the zero-fill at all would produce exactly the same reading there. An armed-ness probe
/// added at `$0420` (outside the filled range) read `$00` on both, confirming the assertions could
/// not fail.
///
/// The fix came from reading `release_to_ipl`: it jumps to **`$FFC0`**, the IPL's *reset* entry,
/// and the zero-fill is the first thing there. So the fill runs again before every upload — which
/// means the way to make this falsifiable is not to run *before* anything else, but to dirty the
/// range deliberately and then go back through the IPL.
///
/// (`E4.11` later measured the power-on state properly and found Mesen2 *randomises* APU RAM, so
/// the original test would in fact have been discriminating there — but not on the other two, and
/// a test that only works on one core is not one this battery can score.)
///
/// So the test uploads two programs. The first fills `$02-$EF` with `$FF` and releases; the second
/// sweeps the same range and reports it. A core that does not zero-fill returns `$FF`, and the
/// assertion has something to catch. Being self-arming, it also no longer depends on its position
/// in the group.
///
/// `$00`/`$01` are excluded rather than asserted: they are the IPL's own transfer-destination
/// pointer, so it necessarily leaves them holding the upload address — measured as `$01 = $02`,
/// the high byte of the `$0200` destination, and reported rather than asserted. A test demanding
/// the whole range be zero would be asserting against the mechanism that does the filling.
///
/// # It must write `PORT1`, and that is `E4.04`'s requirement rather than this test's
///
/// `E4.04` polls port 1 for the boot ROM's `$BB` and then asserts port 0 reads `$AA`. That is only
/// sound if port 1 does *not* already hold `$BB` from an earlier announcement — otherwise the poll
/// matches instantly and port 0 is read while the SMP is still working through the zero-fill.
/// Every other program in this group happens to write `PORT1` with a result and so clears it. The
/// first version of this test did not, and `E4.04` failed immediately after it. Reporting `$01`
/// there restores the invariant deliberately instead of by accident.
///
/// # Shape
///
/// Only backward branches exist in the `Spc` builder, deliberately, so neither loop can exit early;
/// each ORs every byte in its range into `A` and reports the accumulated value. The range is swept
/// in two halves so a partial fill says where it stopped.
fn e4_03() -> Test {
    // Phase 1: fill $02-$EF with $FF, so the range is unambiguously dirty. $00/$01 are left alone
    // -- they are the IPL's own transfer pointer and it rewrites them on the next upload anyway.
    let mut dirty = Spc::new();
    dirty.mov_x_imm(0xEF).mov_sp_x();
    dirty.mov_a_imm(0xFF).mov_x_imm(0x02);
    let fill = dirty.here();
    dirty.mov_x_ind_a().inc_x().cmp_x_imm(0xF0).bne_back(fill);
    dirty.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    // Phase 2: after the release has taken the SMP back through $FFC0, sweep the same range.
    let mut check = Spc::new();
    check.mov_x_imm(0xEF).mov_sp_x();
    // $01 is the IPL's own transfer-destination pointer, so it necessarily holds the upload
    // address rather than zero. Reported for its own sake -- and reporting it also writes PORT1,
    // which E4.04 depends on: see the note in this test's doc comment.
    check.mov_a_dp(0x01).mov_dp_a(PORT1);
    check.mov_a_imm(0x00).mov_x_imm(0x02);
    let low = check.here();
    check.or_a_x_ind().inc_x().cmp_x_imm(0x20).bne_back(low);
    check.mov_dp_a(PORT2);
    check.mov_a_imm(0x00).mov_x_imm(0x20);
    let high = check.here();
    check.or_a_x_ind().inc_x().cmp_x_imm(0xF0).bne_back(high);
    check.mov_dp_a(PORT3);
    check.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    a.c("Phase 1: dirty APU RAM $02-$EF with $FF, then hand back to the IPL.");
    upload_and_run_tagged(&mut a, &dirty, "_d");
    a.c(
        "Phase 2: the release above re-entered the IPL at $FFC0, whose first act is the zero-fill.",
    );
    a.c("Anything still $FF here was not cleaned.");
    upload_and_run_tagged(&mut a, &check, "_c");
    a.c("Record everything before judging: which half fails changes what the failure means.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(
        93,
        "E4.03 APU RAM $01 — the IPL transfer pointer, high byte of the upload address",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(94, "E4.03 OR of APU RAM $02-$1F after the IPL re-entry");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(95, "E4.03 OR of APU RAM $20-$EF after the IPL re-entry");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "APU RAM $02-$1F still held the $FF this test wrote before handing back to the IPL: the \
         boot ROM's zero-fill did not run, or did not reach this far",
    );
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "APU RAM $20-$EF still held the $FF this test wrote before handing back to the IPL: the \
         zero-fill stopped short of the documented $00EF",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E4.03",
        'E',
        "IPL zerofills $00-$EF",
        Provenance::Documented(
            "the canonical 64-byte IPL listing zero-fills $0000-$00EF as the first thing it does \
             at $FFC0, before entering its transfer loop; fullsnes and the SNESdev Wiki both \
             carry it",
        ),
        Kind::Scored,
        None,
    )
}

/// What pattern does APU RAM power up holding? A golden vector: no core models one.
///
/// The dossier records the hardware pattern as repeating **32 bytes of `$00` then 32 of `$FF`**,
/// and marks it chip-dependent and informational — so it is recorded, never scored. What makes the
/// recording worth having is that the three cores do three different things, and **none of them
/// reproduces the documented pattern**:
///
/// | core | `$8000` | `$8020` | `$8040` | variant |
/// |---|---|---|---|---|
/// | RustySNES | `$00` | `$00` | `$00` | 1 — uniformly zero |
/// | snes9x | `$00` | `$00` | `$00` | 1 — uniformly zero |
/// | Mesen2 | *random* | *random* | *random* | 3 — neither |
///
/// Mesen2 **randomises** APU RAM: four consecutive runs returned `$62`, `$18`, `$F2`, `$85` at
/// `$8000`. So its bytes here are not reproducible, and that irreproducibility is the finding
/// rather than a defect in the measurement — which is also why this is a golden vector. A scored
/// test would flap on Mesen2 every run.
///
/// That difference is not a curiosity: it is why `E4.03` dirties the zero page itself instead of
/// trusting the power-on state. On RustySNES and snes9x a "is the zero page zero?" test cannot
/// fail, because the RAM was already zero; on Mesen2 the same test would have been discriminating.
/// Writing one test that works on all three meant not depending on any of it.
///
/// # Three bytes, chosen so the pattern would be unmistakable
///
/// The pattern's period is 64 bytes, so addresses are picked one per half-period:
///
/// | address | offset mod 64 | expected under the pattern |
/// |---|---:|---|
/// | `$8000` | 0 | `$00` |
/// | `$8020` | 32 | **`$FF`** |
/// | `$8040` | 0 | `$00` |
///
/// A core reproducing the pattern reads `$00`, `$FF`, `$00`; one booting all-zero reads three
/// zeroes; anything else — including a randomising core — is reported raw. The middle byte alone
/// separates the two structured hypotheses, and the outer two guard against a core that fills ARAM
/// with `$FF` uniformly.
///
/// # Why the addresses are high, and why this runs first
///
/// Power-on state survives only until something writes over it, so the observation is inherently
/// once-only — the same shape as `capture_power_on` on the CPU side. Two things protect it. The
/// addresses sit at `$8000`, far above both the `$0200` upload area and the `$3000` echo buffer the
/// `E9` tests use, so no other test's data reaches them. And this is the **first** entry in
/// [`all`], before any program has run at all.
///
/// Being a golden vector rather than a scored one is what makes that ordering safe to depend on: if
/// it ever does run late, the result is a recording that looks wrong rather than a battery failure,
/// and the raw bytes in the channel say exactly what was seen.
fn e4_11() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    prog.mov_a_abs(0x8000).mov_dp_a(PORT1);
    prog.mov_a_abs(0x8020).mov_dp_a(PORT2);
    prog.mov_a_abs(0x8040).mov_dp_a(PORT3);
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(96, "E4.11 ARAM $8000 at power-on (pattern predicts $00)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(97, "E4.11 ARAM $8020 at power-on (pattern predicts $FF)");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(98, "E4.11 ARAM $8040 at power-on (pattern predicts $00)");
    a.c("The middle byte is what separates the two live answers; the outer two rule out a core");
    a.c("that filled everything with $FF.");
    a.l("sep #$30");
    a.l("lda f:$7E0100");
    a.l("ora f:$7E0101");
    a.l("ora f:$7E0102");
    a.l("bne :+");
    a.l("lda #$03          ; variant 1 = uniformly zero; no power-on pattern modelled");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("lda f:$7E0100");
    a.l("bne :+");
    a.l("lda f:$7E0102");
    a.l("bne :+");
    a.l("lda f:$7E0101");
    a.l("cmp #$FF");
    a.l("bne :+");
    a.l("lda #$05          ; variant 2 = the documented 32x$00 / 32x$FF pattern");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("lda #$07          ; variant 3 = neither; the raw bytes are in slots 96-98");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    apu_timeout_arm(&mut a);
    a.finish(
        "E4.11",
        'E',
        "ARAM power-on pattern",
        Provenance::Contested(
            "the dossier records a repeating 32x$00 / 32x$FF fill and marks it chip-dependent and \
             informational; RustySNES, snes9x and Mesen2 all boot APU RAM uniformly zero instead",
        ),
        Kind::Golden,
        None,
    )
}

/// A 0→1 transition on a timer's enable bit resets that timer's divider and its output counter.
///
/// `$F1` bits 0-2 are the three timer enables, and turning one on is not merely "resume": the
/// documented behaviour is that the transition clears the timer's stage-2 divider and its stage-3
/// output counter. A core that treats the bit as a pause/resume gate keeps whatever had already
/// accumulated, and a driver that stops and restarts a timer to re-zero it — which is the normal
/// way to do it, since the counter is read-to-clear and reading has side effects — silently gets a
/// stale count on the first read.
///
/// # The control is the same interval without the restart
///
/// "The counter reads zero" is worthless on its own: a timer that never ran reads zero too. So the
/// program measures the same interval twice.
///
/// | phase | sequence | expected |
/// |---|---|---|
/// | 1 | drain, enable, delay, **disable**, read | several ticks — proves the interval counts |
/// | 2 | drain, enable, delay, disable, **re-enable**, read | `$00` — the transition reset it |
///
/// Phase 1 also settles a second question the test would otherwise be exposed to: it reads the
/// counter *after* disabling and still sees the accumulated value, so disabling alone does not
/// clear it. If it did, phase 1 would read zero and the control assertion fails rather than phase 2
/// passing for the wrong reason.
///
/// A core treating the enable as pause/resume returns phase 1's count again in phase 2, so the two
/// readings are equal instead of differing by everything — and both are published, so the failure
/// says which of the two it is.
///
/// The counter is only four bits, so the delay is sized to land the control in the middle of its
/// range: long enough to be unambiguously non-zero, short enough not to wrap past 15 and alias
/// back down toward zero, which would look exactly like the reset this test is trying to observe.
fn e3_02() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    prog.mov_dp_imm(0xFA, 0x01); // T0DIV = 1, the fastest timer 0 can run
    // --- phase 1: how much does this interval accumulate? ---
    prog.mov_a_dp(0xFD); // drain: the counter is read-to-clear
    prog.mov_dp_imm(0xF1, 0x81); // enable timer 0; bit 7 keeps the IPL mapped
    prog.delay(0x60);
    prog.mov_dp_imm(0xF1, 0x80); // stop it, but do not read it yet
    prog.mov_a_dp(0xFD).mov_dp_a(PORT1); // read after the stop: the value survives disabling
    // --- phase 2: the same interval, then a 0->1 transition before reading ---
    prog.mov_a_dp(0xFD); // drain again
    prog.mov_dp_imm(0xF1, 0x81);
    prog.delay(0x60);
    prog.mov_dp_imm(0xF1, 0x80);
    prog.mov_dp_imm(0xF1, 0x81); // the transition under test
    prog.mov_a_dp(0xFD).mov_dp_a(PORT2);
    prog.mov_dp_imm(0xF1, 0x80); // leave the timer off for whatever runs next
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(
        137,
        "E3.02 timer 0 ticks over the interval, with no restart (the control)",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(
        138,
        "E3.02 the same interval, read after a 0->1 on the enable bit",
    );
    a.c("The control first: the interval has to count, or 'it reads zero' below means nothing.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        3,
        15,
        "timer 0 did not accumulate a usable number of ticks over this interval, so the reset \
         check below would pass against nothing — a zero here would also mean disabling the timer \
         clears the counter, which phase 1 is arranged to detect",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        1,
        "the counter did not read zero after the enable bit went 0->1. A core treating the bit as \
         pause/resume returns the control's count here instead, so compare slots 113 and 114: \
         equal means the transition was ignored",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.02",
        'E',
        "Timer enable 0->1 resets",
        Provenance::Documented(
            "SNESdev Wiki SPC700 timers and fullsnes: a 0->1 on a $F1 timer-enable bit resets that \
             timer's stage-2 divider and stage-3 output counter",
        ),
        Kind::Scored,
        None,
    )
}

/// What a soft reset leaves behind — a golden vector, because the two sources disagree.
///
/// `FLG` bit 7 is the DSP's soft reset. While it is asserted the chip behaves as though `FLG` held
/// `$E0`: reset, muted, echo writes disabled, every voice forced into release with a zero envelope.
/// That much both sources agree on and it is asserted here. What they do **not** agree on is what
/// `ENDX` reads afterwards — nocash says `$FF`, anomie says `0` — and the dossier marks the row
/// `[CONFLICT]` and asks for a golden vector by name.
///
/// | slot | reading | why |
/// |---|---|---|
/// | 194 | `ENVX` before the reset | the guard: there has to be an envelope for the reset to clear |
/// | 195 | `ENVX` with the reset asserted | `$00` — the "behaves as `$E0`" half, and it is asserted |
/// | 196 | `ENDX` with the reset asserted | the contested one, recorded and scored not at all |
///
/// # Why the envelope half can be asserted and the `ENDX` half cannot
///
/// "Every voice is forced to release with a zero envelope" is a *behaviour*, stated the same way by
/// both sources and observable without knowing which of them is right about anything else. `ENDX`
/// is a *register value* the two sources contradict each other on, and a battery that asserted
/// either would be picking a source rather than measuring a machine — which is the whole reason
/// `Kind::Golden` exists.
///
/// The voice is keyed on and left to settle at full scale first, so slot 194 reads `$7F` and slot
/// 195's zero means the reset cleared something. Without that, a DSP that had simply never started
/// would report the same `$00` and look like a correct implementation.
///
/// # The settle after the reset is long, and the first version's was not
///
/// "Forced into release" is not the same claim as "zeroed on the spot". RustySNES zeroes the
/// envelope in the cycle the latched reset bit is seen; snes9x starts the release ramp and lets it
/// run down at the usual −8 per sample. Both end at silence; they take wildly different times to
/// get there.
///
/// The first version waited about a sample and a half and asserted `$00`, which is not a test of
/// the row at all — it is a test of *which* of those two implementations a core chose. snes9x and
/// Mesen2 both failed it, and two independent references disagreeing is a statement about the test
/// rather than about the emulator. The settle is now long enough that a ramped release has reached
/// zero too, so what is asserted is the behaviour both sources describe: after the reset, the voice
/// is silent. A core that ignores the bit still reads `$7F` and still fails.
///
/// # What the contested reading turned out to be
///
/// Neither `$FF` nor `$00`: RustySNES and snes9x both report `$01`, the bit the sample's own loop
/// had already set. Both preserve `ENDX` across the reset rather than forcing it either way, which
/// is a third answer to a two-way conflict — and precisely the kind of thing a golden vector exists
/// to notice.
///
/// # The reset is released before the program ends
///
/// A DSP left in reset produces nothing for every test after this one. `voice_program` opens by
/// writing `FLG $20`, so the battery would recover anyway — but relying on the next test to undo
/// this one's state is how an ordering dependency gets built, and this test already has one
/// (`E8.07`) in living memory to learn from.
fn e10_05() -> Test {
    let mut p = Spc::new();
    let addr = p.data_first(IMAGE_BASE, &looping_sample());
    p.mov_x_imm(0xEF).mov_sp_x();
    let dir = u16::from(DIR_PAGE) << 8;
    let [lo, hi] = addr.to_le_bytes();
    p.mov_a_imm(lo).mov_abs_a(dir);
    p.mov_a_imm(hi).mov_abs_a(dir + 1);
    p.mov_a_imm(lo).mov_abs_a(dir + 2);
    p.mov_a_imm(hi).mov_abs_a(dir + 3);
    dsp_write(&mut p, 0x6C, 0x20); // FLG: running, unmuted, echo writes off
    dsp_write(&mut p, 0x5C, 0x00); // KOF
    dsp_write(&mut p, 0x3D, 0x00); // NON
    dsp_write(&mut p, 0x4D, 0x00); // EON
    dsp_write(&mut p, 0x2D, 0x00); // PMON
    dsp_write(&mut p, 0x5D, DIR_PAGE);
    dsp_write(&mut p, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut p, 0x1C, 0x7F); // MVOLR
    dsp_write(&mut p, 0x00, 0x7F); // VOL L
    dsp_write(&mut p, 0x01, 0x7F); // VOL R
    dsp_write(&mut p, 0x02, 0x00); // PITCH low
    dsp_write(&mut p, 0x03, 0x10); // PITCH high
    dsp_write(&mut p, 0x04, 0x00); // SRCN
    dsp_write(&mut p, 0x06, 0x00); // ADSR2
    dsp_write(&mut p, 0x07, 0x7F); // GAIN: direct, full scale
    dsp_write(&mut p, 0x05, 0x00); // ADSR1 bit 7 clear
    dsp_write(&mut p, 0x7C, 0x00); // ENDX, so a set bit later is this run's
    dsp_write(&mut p, 0x4C, 0x01); // KON voice 0
    p.delay(0x00);
    dsp_write(&mut p, 0x4C, 0x00);
    p.delay(0x40); // long enough for the envelope to reach GAIN and the sample to loop
    dsp_read_to(&mut p, 0x08, PORT1); // ENVX before the reset: the guard

    dsp_write(&mut p, 0x6C, 0xE0); // soft reset, with mute and echo-write-disable alongside
    p.delay(0xC0); // long enough for a *ramped* release to have visibly fallen, not just an instant one
    dsp_read_to(&mut p, 0x08, PORT2); // ENVX under reset
    dsp_read_to(&mut p, 0x7C, PORT3); // ENDX under reset: the contested reading
    dsp_write(&mut p, 0x6C, 0x20); // release it, rather than leaving the DSP dead for later tests
    p.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &p);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(194, "E10.05 ENVX before the soft reset (the guard)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(195, "E10.05 ENVX with FLG bit 7 asserted");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(
        196,
        "E10.05 ENDX with FLG bit 7 asserted (nocash $FF, anomie $00)",
    );
    a.c("The guard: a DSP that never started reads zero below without the reset doing anything.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x7C,
        0x7F,
        "the envelope was not at full scale before the reset, so the zero below says nothing about \
         what the reset did",
    );
    a.c("Both sources agree the reset forces every voice to release with a zero envelope.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x00,
        0x00,
        "asserting FLG bit 7 left the envelope where it was, and the settle here is long enough \
         that even a release that ramps rather than zeroing on the spot would have reached silence \
         — so the soft reset is not forcing voices into release at all, and a driver using it to \
         silence the chip would still hear the last note",
    );
    a.c("ENDX is recorded and nothing more: the two sources contradict each other outright.");
    a.l("sep #$20");
    a.l("lda #$03          ; variant 1 = captured; slot 196 is the contested reading");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    apu_timeout_arm(&mut a);
    a.finish(
        "E10.05",
        'E',
        "Soft reset acts as $E0",
        Provenance::Contested(
            "both sources agree FLG bit 7 makes the DSP behave as $E0 and force every voice into \
             release, and contradict each other on what ENDX then reads: nocash says $FF, anomie \
             says 0. The dossier marks the row [CONFLICT] and asks for a golden vector",
        ),
        Kind::Golden,
        None,
    )
}

/// path from a decoded BRR sample to an observable that does not pass through interpolation.
fn e6_09() -> Test {
    let prog = voice_program(&constant_sample(0xC, 0x8), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(205, "E6.09 ENVX at shift 12 (the arming guard)");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(206, "E6.09 OUTX for four maximally-negative gaussian taps");
    a.c("The guard: a silent voice reads $00, which is neither sign and would pass a range test");
    a.c("written carelessly.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x7C,
        0x7F,
        "the envelope was not at full scale, so OUTX is scaled by something this test does not \
         know and its sign is not safe to read",
    );
    a.c("+$3FF8 rather than -$4000: the sign is the whole assertion.");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x01,
        0x7F,
        "four maximally-negative taps interpolated to a negative output, so the gaussian \
         accumulator is saturating or working in wider arithmetic rather than wrapping in 16 bits \
         — a sample written to exploit the wrap plays back without the sign inversion it expects",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E6.09",
        'E',
        "Gaussian sum wraps",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: of the four gaussian additions the first cannot \
             overflow, the second wraps in 16 bits and the third saturates",
        ),
        Kind::Scored,
        None,
    )
}

/// The four named BRR waveform vectors — a golden vector, because that is what the row asks for.
///
/// `E6.11` does not state an expected value for anything. It names four nibble patterns —
/// `79797979`, `77997799`, `77779999`, `7777CC44` — and asks for what a decoder makes of each. They
/// are chosen to walk from the trivial to the awkward: a plain alternation, the same alternation at
/// half the rate, at a quarter, and finally a pattern with a large positive and a large negative
/// nibble adjacent, which is where a filter's accumulator and the 15-bit clamp have the most to
/// disagree about.
///
/// | slot | nibbles | shape |
/// |---|---|---|
/// | 197 | `79797979` | alternating every sample |
/// | 198 | `77997799` | alternating every two |
/// | 199 | `77779999` | alternating every four |
/// | 200 | `7777CC44` | `+7`, then `-4`, then `+4` — sign changes of unequal size |
///
/// All four use shift 12 and filter 0, so the reading is the decoder's own arithmetic rather than
/// a filter's history: filter 0 has no previous-sample term, and shift 12 puts the nibbles high
/// enough in the 15-bit range that a clamp, if there is one, has something to act on.
///
/// # Recorded, not asserted, and not classified either
///
/// There is no expected value to assert — the row's content *is* the measurement. Nor does the
/// verdict classify the readings: gaussian interpolation runs over four consecutive samples, so
/// what `OUTX` holds depends on where in the pattern the DSP was when the cart looked, and a
/// verdict computed from that would change every time anything ahead of this test in the battery
/// shifted the sample phase. That is precisely what happened to `E8.07`. The numbers go to the
/// channel and the verdict says only that all four were taken.
///
/// The envelope is asserted, though, because it is the one thing here that can be silently absent:
/// four `OUTX` readings from a voice that never keyed on are four zeroes, and four zeroes look like
/// a decoder that produces silence rather than like a test that never ran.
///
/// # The measurement confirmed the reason for not classifying
///
/// Against snes9x, the two slow patterns agree exactly — `77779999` reads `$6F` in both and
/// `7777CC44` reads `$4F` — while the two fast ones do not: `$E1` against `$1E` for `79797979`, and
/// one apart for `77997799`. That is the gaussian phase showing itself exactly where the pattern
/// alternates fast enough for it to matter, and it is what a classifying verdict would have turned
/// into a cross-validation failure about nothing.
fn e6_11() -> Test {
    let vectors: [(&str, &[u8], u16, u32); 4] = [
        ("79797979", &[0x79], 197, 0x7E_01F8),
        ("77997799", &[0x77, 0x99], 198, 0x7E_01F9),
        ("77779999", &[0x77, 0x77, 0x99, 0x99], 199, 0x7E_01FA),
        ("7777CC44", &[0x77, 0x77, 0xCC, 0x44], 200, 0x7E_01FB),
    ];

    let mut a = Asm::new();
    for (name, pattern, _, stash) in vectors {
        let sample = brr_sample(&[brr_block_pattern(0xC, 0, 0b11, pattern)], 0);
        let prog = voice_program(&sample, Voice::direct_gain());
        a.c(&format!("--- nibbles {name} ---"));
        upload_and_run_tagged(&mut a, &prog, &format!("_v{name}"));
        a.l("sep #$20");
        a.l("lda f:$7E0102     ; OUTX");
        a.l(&format!("sta f:${stash:06X}"));
    }
    a.c("The envelope from the last run stands for all four: the setup is identical.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01FC");
    a.l("rep #$30");
    for (name, _, slot, stash) in vectors {
        a.l(&format!("lda f:${stash:06X}"));
        a.l("and #$00FF");
        a.record(slot, &format!("E6.11 OUTX for nibbles {name}"));
    }
    a.l("lda f:$7E01FC");
    a.l("and #$00FF");
    a.record(201, "E6.11 ENVX on the last vector (the arming guard)");
    a.c("Four zeroes from a voice that never sounded look exactly like a decoder that outputs");
    a.c("silence, so the envelope is what says the readings are readings.");
    a.l("lda f:$7E01FC");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x7C,
        0x7F,
        "the voice was not at full scale when OUTX was read, so the four waveform readings are \
         scaled by an envelope this test does not know and are not comparable with anything",
    );
    a.l("sep #$20");
    a.l("lda #$03          ; variant 1 = all four captured; slots 197-200 are the vectors");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    apu_timeout_arm(&mut a);
    a.finish(
        "E6.11",
        'E',
        "BRR waveform vectors",
        Provenance::Contested(
            "the dossier names four nibble patterns and asks what a decoder makes of each without \
             stating an expected value for any of them; the row's content is the measurement",
        ),
        Kind::Golden,
        None,
    )
}

/// The DSP emits one output sample every 32 SPC cycles.
///
/// The SPC700 runs at 1.024 MHz and the DSP at 32 kHz, so the two are locked at exactly 32 CPU
/// cycles per output sample. Everything the DSP does per-sample — the envelope step, the `KOFF`
/// poll, the BRR decode — is paced by that ratio, which is why a driver counting SPC cycles can
/// predict how many samples have gone by and why the ratio is worth pinning independently of any
/// one of those behaviours.
///
/// # Turning samples into cycles with the release ramp
///
/// Release steps the envelope down by a fixed 8 every output sample (`E7.09`), and stops at zero.
/// A voice keyed off from a full-scale envelope therefore takes a known number of *samples* to go
/// silent, and timing that with timer 0 converts it to *cycles*:
///
/// | quantity | value |
/// |---|---:|
/// | envelope at `GAIN` `$7F` | `$7F0` = 2032 |
/// | samples to reach zero at −8 | 254 |
/// | cycles, at 32 per sample | 8128 |
/// | timer 0 ticks at `T0DIV = 6` (768 cycles each) | 10 |
///
/// `T0DIV = 6` is chosen to land the answer in the middle of `TnOUT`'s **four bits**. The competing
/// hypotheses are far outside the reading either way: 64 cycles per sample gives 21 ticks, which
/// wraps to 5, and 16 gives 5 — and neither is mistakable for 10.
///
/// # Two guards, because two different things can be silently absent
///
/// `ENVX` is read **before** the key-off and asserted to be at full scale. A voice that never keyed
/// on has a zero envelope, so the poll below would exit on its first pass and report a tick count
/// that means nothing. And the tick count itself is asserted to be non-zero, which catches the
/// opposite failure — a timer that never ran reads zero however long the ramp took.
///
/// The poll loop costs about fifteen cycles a pass, so it can overshoot the moment the envelope
/// reaches zero by that much. Against 8128 that is a fifth of one tick, which is why the assertion
/// can be a two-tick window rather than a proportional one.
fn e10_01() -> Test {
    let mut p = Spc::new();
    let addr = p.data_first(IMAGE_BASE, &looping_sample());
    p.mov_x_imm(0xEF).mov_sp_x();
    let dir = u16::from(DIR_PAGE) << 8;
    let [lo, hi] = addr.to_le_bytes();
    p.mov_a_imm(lo).mov_abs_a(dir);
    p.mov_a_imm(hi).mov_abs_a(dir + 1);
    p.mov_a_imm(lo).mov_abs_a(dir + 2);
    p.mov_a_imm(hi).mov_abs_a(dir + 3);
    dsp_write(&mut p, 0x6C, 0x20); // FLG: running, unmuted, echo writes off
    dsp_write(&mut p, 0x5C, 0x00); // KOF
    dsp_write(&mut p, 0x3D, 0x00); // NON
    dsp_write(&mut p, 0x4D, 0x00); // EON
    dsp_write(&mut p, 0x2D, 0x00); // PMON
    dsp_write(&mut p, 0x5D, DIR_PAGE);
    dsp_write(&mut p, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut p, 0x1C, 0x7F); // MVOLR
    dsp_write(&mut p, 0x00, 0x7F); // VOL L
    dsp_write(&mut p, 0x01, 0x7F); // VOL R
    dsp_write(&mut p, 0x02, 0x00); // PITCH low
    dsp_write(&mut p, 0x03, 0x10); // PITCH high
    dsp_write(&mut p, 0x04, 0x00); // SRCN
    dsp_write(&mut p, 0x06, 0x00); // ADSR2
    dsp_write(&mut p, 0x07, 0x7F); // GAIN: direct, full scale
    dsp_write(&mut p, 0x05, 0x00); // ADSR1 bit 7 clear, so GAIN is what governs
    dsp_write(&mut p, 0x4C, 0x01); // KON voice 0
    p.delay(0x00);
    dsp_write(&mut p, 0x4C, 0x00);
    p.delay(0x20); // let the envelope settle at the GAIN value
    dsp_read_to(&mut p, 0x08, PORT1); // ENVX before the ramp: the arming guard

    p.mov_dp_imm(0xFA, 0x06); // T0DIV = 6: one tick per 768 cycles
    p.mov_a_dp(0xFD); // drain, so the interval starts from zero
    p.mov_dp_imm(0xF1, 0x81); // enable timer 0
    dsp_write(&mut p, 0x5C, 0x01); // KOFF voice 0: the ramp starts here
    let poll = p.here();
    p.mov_a_imm(0x08).mov_dp_a(0xF2);
    p.mov_a_dp(0xF3);
    p.cmp_a_imm(0x00);
    p.bne_back(poll); // spin until ENVX reads zero
    p.mov_dp_imm(0xF1, 0x80); // stop the timer the instant it does
    p.mov_a_dp(0xFD).mov_dp_a(PORT2);
    p.mov_dp_imm(0xF1, 0x80);
    p.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &p);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(192, "E10.01 ENVX before key-off (the arming guard)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(193, "E10.01 timer 0 ticks for a full release ramp");
    a.c("The envelope has to have been at full scale, or the poll exits immediately.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x7C,
        0x7F,
        "the envelope was not at full scale when the voice was keyed off, so the ramp below was \
         shorter than the arithmetic assumes — or absent entirely",
    );
    a.c("254 samples at 32 cycles each is 8128 cycles, which is ten 768-cycle ticks.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        9,
        11,
        "a full release ramp did not take the ten timer-0 ticks that 254 samples at 32 SPC cycles \
         each come to. The envelope was full scale and the release step is fixed, so what is wrong \
         is the sample period: 64 cycles would read 5 after wrapping, and 16 would read 5 outright",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E10.01",
        'E',
        "32 SPC cycles per sample",
        Provenance::Documented(
            "fullsnes and anomie: the SPC700's 1.024 MHz clock and the DSP's 32 kHz output rate \
             fix the ratio at 32 CPU cycles per output sample",
        ),
        Kind::Scored,
        None,
    )
}

/// `XCN` costs five cycles — measured against `NOP`, which costs two.
///
/// `XCN` swaps `A`'s two nibbles and touches nothing else, yet it is the most expensive
/// register-only instruction the SPC700 has. Both it and `NOP` are one byte, so two blocks of 256
/// of each differ in nothing but the per-instruction cost, and every other variable — the upload,
/// the timer plumbing, the enable and stop writes — cancels between them.
///
/// # Reading the cost off timer 0
///
/// `T0DIV = 1` makes timer 0 tick once every 128 SPC cycles, and the counter at `$FD` is read to
/// clear, so each block's cost can be taken cleanly straight after it:
///
/// | block | cycles | ticks |
/// |---|---:|---:|
/// | 256 × `NOP` | 512 | 4 |
/// | 256 × `XCN` | 1280 | 10 |
///
/// 256 is not arbitrary in either direction. Fewer and the difference disappears into the one tick
/// of quantisation either reading can carry; more and the `XCN` block overflows `TnOUT`, which is
/// **four bits wide** — at 512 iterations it would read 4 and be indistinguishable from the `NOP`
/// block. This is the widest window in which both numbers are unambiguous.
///
/// # What a wrong cost would look like
///
/// The tick count is the cycle count divided by 128, so a one-cycle error moves it by two:
///
/// | if `XCN` were | cycles | ticks | verdict |
/// |---|---:|---:|---|
/// | 4 | 1024 | 8 | fails low |
/// | **5** | **1280** | **10** | passes |
/// | 6 | 1536 | 12 | fails high |
///
/// The `NOP` block is the guard, and it is asserted first. Its four ticks establish that the timer
/// ran, that `T0DIV` took, and that the enable/stop pair bracket the block — without it, an `XCN`
/// reading of 10 could as easily come from a timer running at some other rate as from the
/// instruction costing what it should.
fn e1_14() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    prog.mov_dp_imm(0xFA, 0x01); // T0DIV = 1: one tick per 128 cycles, the fastest T0 runs
    prog.mov_a_dp(0xFD); // drain, so the first block starts from zero
    // --- the baseline ---
    prog.mov_dp_imm(0xF1, 0x81);
    for _ in 0..256 {
        prog.nop();
    }
    prog.mov_dp_imm(0xF1, 0x80);
    prog.mov_a_dp(0xFD).mov_dp_a(PORT1); // reading it also clears it for the next block
    // --- the same block length, of the instruction under test ---
    prog.mov_dp_imm(0xF1, 0x81);
    for _ in 0..256 {
        prog.xcn();
    }
    prog.mov_dp_imm(0xF1, 0x80);
    prog.mov_a_dp(0xFD).mov_dp_a(PORT2);
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(7, "E1.14 timer 0 ticks over 256 NOP (the baseline)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(125, "E1.14 timer 0 ticks over 256 XCN");
    a.c("The baseline first: it says the timer ran at the rate the arithmetic below assumes.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        3,
        5,
        "256 NOP did not cost the four timer-0 ticks that 512 cycles at 128 cycles per tick come \
         to, so either NOP is not two cycles or the timer is not running at T0DIV = 1 — and the \
         XCN reading below is then measured against nothing",
    );
    a.c("And XCN's five cycles put its block at ten ticks; four or six would read 8 or 12.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        9,
        11,
        "256 XCN did not cost the ten timer-0 ticks five cycles each come to. The baseline passed, \
         so the timer and the block length are right and the per-instruction cost is not: at 128 \
         cycles per tick this reading is off by at least a whole cycle per XCN",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.14",
        'E',
        "XCN costs five cycles",
        Provenance::Documented(
            "the SNESdev Wiki SPC700 reference and fullsnes both give XCN as 5 cycles, against 2 \
             for NOP",
        ),
        Kind::Scored,
        None,
    )
}

/// `DBNZ dp,rel` is a read-modify-write, so it *reads* its target before decrementing it.
///
/// The access pattern is invisible on ordinary RAM: read, subtract one, write back leaves the same
/// value a bare decrement would. It becomes visible on a target whose read has a side effect — and
/// the SPC700 has three, the timer counters at `$FD`-`$FF`, which are **read-to-clear**. Point
/// `DBNZ` at one of those and the read clears it, whatever the arithmetic then does. A core that
/// implements `DBNZ` as a decrement without the read leaves the counter holding its count.
///
/// # The control is the same interval measured without the `DBNZ`
///
/// "The counter reads zero" proves nothing on its own — a timer that never ran reads zero too. So
/// the program times the same interval twice:
///
/// | phase | sequence | expected |
/// |---|---|---|
/// | 1 | enable, delay, stop, **read `$FD`** | several ticks — proves the interval counts |
/// | 2 | enable, delay, stop, **`DBNZ $FD`**, then read `$FD` | `$00` — the RMW's read cleared it |
///
/// Phase 1 doubles as the drain: reading the counter is what clears it, so phase 2 starts from a
/// known zero without needing a separate step.
///
/// The displacement is `0`, so the branch falls through whether or not it is taken. `DBNZ`'s branch
/// is not what this is about, and a branch that went anywhere would be a second variable in a test
/// with one question.
///
/// The write-back lands on a read-only register and is discarded, which is fine: the claim is that
/// the instruction *reads*, and the read is what leaves a mark.
fn e2_04() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    prog.mov_dp_imm(0xFA, 0x01); // T0DIV = 1, the fastest timer 0 runs
    prog.mov_a_dp(0xFD); // drain before the first interval
    // --- phase 1: the control ---
    prog.mov_dp_imm(0xF1, 0x81);
    prog.delay(0x60);
    prog.mov_dp_imm(0xF1, 0x80);
    prog.mov_a_dp(0xFD).mov_dp_a(PORT1); // reading it here also clears it for phase 2
    // --- phase 2: the same interval, then DBNZ on the counter ---
    prog.mov_dp_imm(0xF1, 0x81);
    prog.delay(0x60);
    prog.mov_dp_imm(0xF1, 0x80);
    prog.dbnz_dp(0xFD, 0);
    prog.mov_a_dp(0xFD).mov_dp_a(PORT2);
    prog.mov_dp_imm(0xF1, 0x80);
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(
        162,
        "E2.04 timer 0 ticks over the interval, read directly (the control)",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(
        163,
        "E2.04 the same interval, read after DBNZ touched the counter",
    );
    a.c("The control first: the interval has to count, or 'it reads zero' below means nothing.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        3,
        15,
        "timer 0 did not accumulate a usable number of ticks over this interval, so the check \
         below would pass against nothing",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        1,
        "the counter still held its count after DBNZ addressed it, so DBNZ never read its target: \
         it is being implemented as a decrement rather than as a read-modify-write",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.04",
        'E',
        "DBNZ dp is an RMW",
        Provenance::Documented(
            "SNESdev Wiki SPC700 reference and fullsnes: DBNZ dp,rel reads its operand, decrements \
             it and writes it back, and $FD-$FF are read-to-clear",
        ),
        Kind::Scored,
        None,
    )
}

/// `VxPITCH` does not affect the noise generator's frequency.
///
/// A voice in noise mode takes its samples from the global LFSR, whose step rate comes from `FLG`
/// bits 0-4 and from nothing else. The voice's own pitch register still controls the *sample*
/// pointer — which is why `E9.04` can show a noise voice decoding BRR at the same time — but it has
/// no bearing on how fast the noise itself advances. A core that runs the LFSR off the voice's
/// pitch counter gives every noise voice a different timbre depending on the note it was keyed at,
/// which is audible and wrong.
///
/// # Two runs that differ in exactly one register
///
/// Both play voice 0 with `NON` set, at the same volume, envelope and settle, and read `VxOUTX` at
/// the same point. The only difference is `VxPITCH`: `$1000` (one sample per output sample) in the
/// first, `$2000` (an octave up) in the second. Equal readings mean the pitch did not reach the
/// noise generator.
///
/// # The LFSR has to start from the same place both times
///
/// It is global and survives between programs, so without a reset the second run would begin from
/// wherever the first left it and the two readings would differ for a reason that has nothing to do
/// with pitch. Both programs therefore pulse `FLG` bit 7 first, which re-seeds the LFSR to `$4000`.
/// That is the whole reason [`Voice::flg_reset`] exists.
///
/// # The guard
///
/// Two zeroes are also equal. `VxOUTX` is asserted non-zero first: if the noise voice were silent —
/// unkeyed, muted, or with the LFSR never stepping — the comparison below would pass without having
/// compared anything. That check is what makes "they match" evidence rather than a coincidence.
fn e9_03() -> Test {
    let noise = |pitch_hi: u8| Voice {
        non: 0x01,
        pitch_hi,
        flg_reset: true,
        settle: 6,
        ..Voice::direct_gain()
    };
    let low = voice_program(&looping_sample(), noise(0x10));
    let high = voice_program(&looping_sample(), noise(0x20));

    let mut a = Asm::new();
    a.c("--- run 1: VxPITCH = $1000 ---");
    upload_and_run_tagged(&mut a, &low, "_lo");
    a.l("sep #$20");
    a.l("lda f:$7E0102");
    a.l("sta f:$7E01DC     ; OUTX, before the second upload overwrites the mailbox copy");
    a.c("--- run 2: VxPITCH = $2000, everything else identical ---");
    upload_and_run_tagged(&mut a, &high, "_hi");
    a.l("rep #$30");
    a.l("lda f:$7E01DC");
    a.l("and #$00FF");
    a.record(168, "E9.03 noise OUTX at VxPITCH = $1000");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(169, "E9.03 noise OUTX at VxPITCH = $2000");
    a.c("The guard: two silences are equal too, and would pass the comparison having compared");
    a.c("nothing at all.");
    a.l("sep #$20");
    a.l("lda f:$7E01DC");
    a.l("cmp #$00");
    a.fail_if_eq(
        "the noise voice produced no output at all, so the comparison below would be between two \
         silences rather than between two noise streams",
    );
    a.l("lda f:$7E01DC");
    a.l("cmp f:$7E0102");
    a.fail_if_ne(
        "the two runs produced different noise output, and they differ only in VxPITCH — so the \
         voice's pitch is reaching the noise generator, whose rate comes from FLG bits 0-4 alone",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.03",
        'E',
        "Pitch not noise rate",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the noise generator's step rate is set by FLG bits \
             0-4, and a voice's pitch register does not participate",
        ),
        Kind::Scored,
        None,
    )
}

/// The BRR header is `ssssffle`: shift in bits 7-4, filter in 3-2, loop in 1, end in 0.
///
/// The other three fields are already pinned elsewhere — `E5.03`/`E5.05` decode through both
/// filters, `E5.04` drives the shift into its invalid range, `E5.08`/`E5.09` separate the loop bit
/// from the end bit. What none of them establishes is *where the shift lives*, and a core reading
/// it from the wrong nibble decodes every sample at the wrong amplitude while still honouring the
/// flags, which looks like a volume bug rather than a header bug.
///
/// # Two blocks differing in one nibble
///
/// A BRR nibble decodes as `(nibble << shift) >> 1`, so raising the shift by one doubles the
/// sample. Two constant samples are played, identical but for the shift field — `$8` then `$9` —
/// and the second's output must be twice the first's.
///
/// That is a claim about the *position* of the field, not just its effect: a core taking the shift
/// from bits 3-0 would read `0` from both headers (the filter and flag bits are zero here) and
/// return the same amplitude twice. One taking it from bits 7-4, as documented, returns a factor of
/// two.
///
/// A constant sample is used because gaussian interpolation of a constant is that constant, so the
/// reading does not depend on which sample the cart happened to catch — the same reasoning
/// `constant_sample` carries for the arithmetic tests.
///
/// # Roughly double, not exactly
///
/// Written as an exact `x2` it failed: the readings are `$06` and `$0D`, and twice six is twelve.
/// `VxOUTX` is the top eight bits of a fifteen-bit sample after gaussian interpolation, so the
/// low-order rounding does not survive the truncation intact — the amplitude doubles, the *reported
/// byte* doubles to within one. The assertion therefore allows a tolerance of two, which is far
/// tighter than the factor of two that separates it from "the shift field was ignored".
///
/// # The guard
///
/// The first reading is asserted non-zero before the two are compared. Zero doubles to zero, so a
/// silent voice would satisfy the ratio without having decoded anything.
fn e5_01() -> Test {
    let low = voice_program(&constant_sample(0x8, 0x7), Voice::direct_gain());
    let high = voice_program(&constant_sample(0x9, 0x7), Voice::direct_gain());

    let mut a = Asm::new();
    a.c("--- shift $8 ---");
    upload_and_run_tagged(&mut a, &low, "_s8");
    a.l("sep #$20");
    a.l("lda f:$7E0102");
    a.l("sta f:$7E01E4     ; OUTX, before the second upload overwrites the mailbox copy");
    a.c("--- shift $9: the same block with one nibble changed ---");
    upload_and_run_tagged(&mut a, &high, "_s9");
    a.l("rep #$30");
    a.l("lda f:$7E01E4");
    a.l("and #$00FF");
    a.record(178, "E5.01 OUTX with header shift $8");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(179, "E5.01 OUTX with header shift $9");
    a.c("The guard: zero doubles to zero, so a silent voice would satisfy the ratio below without");
    a.c("having decoded anything.");
    a.l("sep #$20");
    a.l("lda f:$7E01E4");
    a.l("cmp #$00");
    a.fail_if_eq(
        "the shift-$8 block produced no output, so the comparison below would be between two \
         silences rather than between two amplitudes",
    );
    a.c("Raising the shift by one doubles the sample. A core reading the shift from bits 3-0 sees");
    a.c("zero in both headers and returns the same amplitude twice.");
    a.l("rep #$30");
    a.l("lda f:$7E01E4");
    a.l("and #$00FF");
    a.l("asl a             ; twice the shift-$8 amplitude");
    a.l("sta f:$7E01E6");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.l("sec");
    a.l("sbc f:$7E01E6     ; how far the shift-$9 reading is from exactly double");
    a.assert_abs_le(
        2,
        "raising the header's shift nibble by one did not roughly double the decoded amplitude — \
         an unchanged reading means the shift is being taken from a different part of the header",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.01",
        'E',
        "BRR header layout",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the BRR header is ssssffle, shift in bits 7-4, and a \
             nibble decodes as (nibble << shift) >> 1",
        ),
        Kind::Scored,
        None,
    )
}

/// The noise LFSR starts at `$4000`.
///
/// A soft reset seeds the shift register with `$4000` — not zero, which is the value it would be
/// stuck at forever, and not `$7FFF`. A core seeding it wrongly produces a different noise sequence
/// from the very first sample, which no amount of listening distinguishes from correct noise but
/// which any bit-exact audio comparison fails immediately.
///
/// # Freezing the register is what makes the seed observable
///
/// Normally the LFSR steps every few samples and its value depends on how long the cart waited,
/// which the cart cannot know precisely. But the noise rate is taken from `FLG` bits 0-4 through
/// the same counter table the envelopes use, and **rate 0 never fires** (`E7.01`). So with those
/// bits clear the register is seeded and then never advances, and the voice's output is a direct
/// function of the seed for as long as anyone cares to look.
///
/// Every program in this group leaves `FLG` at `$20`, so the noise rate is zero throughout the
/// battery and the LFSR never advances *anywhere*. What this test reads is therefore the
/// **power-on** seed. The `flg_reset` in its configuration is harmless and correct in intent, but
/// it does no work here: RustySNES's `$6C` write sets the mute and reset flags without touching the
/// shift register, so whether a soft reset re-seeds it is a separate question this test does not
/// answer.
///
/// The arithmetic checks out exactly: `$4000` shifted left one is `$8000`, scaled by the direct
/// gain's `$7F0` envelope gives `$8100`, and `VxOUTX` is that value's top byte — `$81`, which is
/// what all of RustySNES, snes9x and Mesen2 report.
///
/// The voice reads that value because `NON` puts it in noise mode: its output is the LFSR's
/// contents rather than anything decoded from BRR. `$4000` shifted left one is `$8000` — full-scale
/// negative — so `VxOUTX` is strongly negative, and its top bit is set.
///
/// # What the assertion pins, and what it does not
///
/// It asserts the output is negative and near full scale. That separates `$4000` from the two
/// plausible wrong seeds: `$0000` gives silence, and a small seed gives a small output. It does not
/// pin the exact byte, because `VxOUTX` is the envelope-scaled top eight bits and the scaling is
/// `E7.15`'s business rather than this row's.
fn e9_01() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            non: 0x01,       // noise mode: the voice outputs the LFSR
            flg_reset: true, // seed it, and FLG's noise rate stays 0 so it never advances
            settle: 6,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(180, "E9.01 noise OUTX with the LFSR frozen at its seed");
    a.c("$4000 shifted left one is $8000 — full-scale negative — so the top bit must be set.");
    a.l("sep #$20");
    a.l("lda f:$7E0102");
    a.l("and #$80");
    a.assert_a8(
        0x80,
        "the frozen noise output is not negative, so the LFSR was not seeded with $4000: a seed \
         of zero gives silence and any small seed gives a small positive or negative value",
    );
    a.c("And near full scale, which a small seed would not be.");
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x80,
        0x8F,
        "the frozen noise output is negative but nowhere near full scale, so the seed has the \
         right sign and the wrong magnitude",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.01",
        'E',
        "Noise LFSR seed",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the noise shift register resets to $4000, with taps \
             bit0 XOR bit1 feeding bit 14",
        ),
        Kind::Scored,
        None,
    )
}

/// GAIN bent-increase climbs at two slopes, breaking at `$600`, and it reads the *clipped* envelope.
///
/// GAIN mode 7 is a two-slope linear increase: the envelope climbs `+32` per sample until it reaches
/// `$600`, then `+8` per sample the rest of the way to `$7FF`. That break is what shapes the "bent"
/// attack a lot of instruments use. The `[ERRATA]` the row names is in the comparison: the threshold
/// is checked against the **internal** envelope latch as an *unsigned* value, so a voice that came
/// out of a decrease mode with an underflowed (negative) latch reads as `>= $600` and takes the slow
/// slope immediately.
///
/// # Distinguishing the bent curve from a plain `+32`
///
/// A single reading in the slow region separates the two. At rate 31 (every sample) the envelope
/// reaches `$600` in 48 samples and then crawls; a plain `+32` climb with no break saturates at
/// `$7FF` in 64. Read the envelope around 96 samples in:
///
/// | model | envelope at ~96 samples | `ENVX` |
/// |---|---:|---|
/// | bent (`+32` then `+8`) | `$600 + 48 × 8` ≈ `$780` | ≈ `$78` |
/// | plain `+32` throughout | saturated at `$7FF` | `$7F` |
///
/// So an `ENVX` past `$60` (the break happened) but short of `$7F` (it did not saturate) is the
/// signature of the bent curve. A core that ignores the `$600` break saturates and reads `$7F`.
///
/// # The reading is bounded on both sides, and both bounds carry weight
///
/// Below `$68` would mean the envelope never crossed `$600` — the slow slope was never exercised and
/// the test would pass against a core that only ever climbs slowly. Above `$7C` (towards `$7F`) means
/// it saturated, which is exactly the plain-`+32` failure. The window between is where the second
/// slope, and only the second slope, puts it.
fn e7_13() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            adsr1: 0x00, // ADSR1 bit 7 clear: GAIN governs
            gain: 0xFF,  // mode 7 (bent increase), rate 31 (steps every sample)
            settle: 1,   // land in the slow region, past $600 and short of saturation
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(
        233,
        "E7.13 ENVX under GAIN bent-increase, in the slow region past $600",
    );
    a.c("Past $600 the slope is +8, not +32: after ~96 samples the envelope is near $780, not");
    a.c("saturated. A core that ignores the $600 break climbs +32 throughout and saturates at $7FF.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x68,
        0x7C,
        "GAIN bent-increase did not land in its slow region. Below $68 the envelope never reached \
         the $600 break, so the second slope was never tested; at $7F it saturated, which is what a \
         core that climbs a flat +32 and ignores the break does",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.13",
        'E',
        "GAIN bent-increase",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: GAIN mode 7 increases +32 per sample below $600 and +8 \
             above, comparing the internal envelope latch unsigned",
        ),
        Kind::Scored,
        None,
    )
}

/// Attack rate `$F` is the special case: it steps every sample, by `+1024`.
///
/// Every other attack rate advances the envelope by 32 on a counter tick. Rate `$F` does neither
/// part of that — it fires on *every* sample and adds `1024`, so an envelope crosses its full
/// `$7FF` range in two samples rather than the milliseconds any other rate takes. A core folding
/// `$F` into the general `a*2+1` formula gets an attack that is merely fast, and every percussive
/// instrument in a game's soundtrack softens.
///
/// # Two rates, the same interval, an envelope that stays put once it arrives
///
/// The comparison is only stable if the envelope stops moving after the attack, otherwise the
/// reading depends on how far decay has since pulled it back. So both runs set the sustain **level**
/// to its maximum (`ADSR2` bits 7-5 = 7) and the sustain **rate** to zero: the decay phase ends
/// immediately because `E >> 8` already equals 7, and rate 0 never fires (`E7.01`), so the envelope
/// parks at full scale and stays there.
///
/// | run | `ADSR1` | attack rate | expected `ENVX` |
/// |---|---|---|---|
/// | 1 | `$8F` | `$F` — every sample, `+1024` | `$7F`, parked at full scale |
/// | 2 | `$80` | `$0` — the slowest rate there is | near zero, barely started |
///
/// Both bounds are asserted, so neither run can pass by being silent: run 1 must have climbed and
/// run 2 must not have. A core that treats `$F` as an ordinary rate fails run 1; one that treats
/// every rate as instant fails run 2.
fn e7_04() -> Test {
    let parked = |adsr1: u8| Voice {
        adsr1,
        adsr2: 0xE0, // sustain level 7, sustain rate 0: park at full scale once the attack lands
        // No extra settle: the read lands about thirty samples after key-on, which is long enough
        // for a +1024 step to have crossed the range twice over and far short of the sixty-four
        // samples a +32 step needs. A longer wait reaches full scale either way and measures only
        // that the rate index is large -- which is how the first version of this test passed its
        // own injection.
        settle: 0,
        ..Voice::direct_gain()
    };
    let fast = voice_program(&looping_sample(), parked(0x8F));
    let slow = voice_program(&looping_sample(), parked(0x80));

    let mut a = Asm::new();
    a.c("--- attack rate $F ---");
    upload_and_run_tagged(&mut a, &fast, "_fast");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01EC     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- attack rate $0, the same interval ---");
    upload_and_run_tagged(&mut a, &slow, "_slow");
    a.l("rep #$30");
    a.l("lda f:$7E01EC");
    a.l("and #$00FF");
    a.record(182, "E7.04 ENVX after attack rate $F");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(183, "E7.04 ENVX after attack rate $0");
    a.c("Rate $F crosses the whole range in two samples, so by any settle at all it is parked.");
    a.l("lda f:$7E01EC");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x70,
        0x7F,
        "attack rate $F did not reach full scale: it steps +1024 every sample and crosses the \
         envelope's whole range in two, so a core applying the ordinary a*2+1 rate lands short",
    );
    a.c("And rate $0 is the slowest there is, so over the same interval it has barely moved.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x00,
        0x0F,
        "attack rate $0 climbed a long way over an interval rate $F needs two samples for — the \
         two rates are not being distinguished at all",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.04",
        'E',
        "Attack $F is instant",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: attack rate $F fires every sample with a step of \
             +1024, rather than the +32 on a counter tick every other rate uses",
        ),
        Kind::Scored,
        None,
    )
}

/// The release rate is fixed: no `ADSR` setting changes it.
///
/// Release steps the envelope down by 8 every sample and there is no register that alters it — the
/// four `ADSR` rates cover attack, decay and sustain, and none of them reaches the release phase.
/// That is why a driver wanting a custom fade has to leave `ADSR` mode entirely and ramp with
/// `GAIN`, which is the `[ERRATA]` the row records. A core that reuses the sustain rate for release
/// gives every instrument a different tail depending on a field that should not matter.
///
/// # Two envelopes that differ only in a rate release must ignore
///
/// Both voices run in `ADSR` mode with attack `$F` — full scale in two samples (`E7.04`) — and
/// **sustain level 0**, so the decay phase runs the whole way down and the sustain phase is never
/// entered at all. The only difference is the sustain **rate**:
///
/// | run | `ADSR2` | sustain rate |
/// |---|---|---|
/// | 1 | `$00` | `0` — never fires |
/// | 2 | `$1F` | `31` — every sample, the fastest there is |
///
/// Both are then keyed off and read after the same short interval. Equal readings mean release did
/// not consult the field; a core that did would show run 2 far below run 1, because rate 31 is as
/// far from rate 0 as the table goes.
///
/// # Sustain level 0 is what makes the comparison fair, and the first version got it wrong
///
/// Written with sustain level **7** the two runs read `$67` and `$17`, and that difference was
/// entirely legitimate: level 7 ends the decay phase immediately at full scale, so the voice sits
/// in *sustain* for the whole interval before key-off, and the sustain rate decays it there. Run 2
/// was simply starting its release from much lower down. The reading was real and the inference
/// would have been wrong — a defect reported against a core doing exactly the right thing.
///
/// With level 0 the decay never completes, sustain is never reached, the rate has nothing to act
/// on, and both runs enter release from the same place.
///
/// # The guard: the reading has to be mid-ramp
///
/// `late_settle` is one block, so the release has started and not finished. That matters in both
/// directions and the test asserts both bounds: at `$00` the ramp is over and every rate looks
/// identical, at `$7F` it has not begun and the same is true. Only between them does "the two runs
/// agree" say anything, and `E8.10` is the test that establishes this window shows a partial ramp
/// rather than a completed one.
fn e7_09() -> Test {
    let released = |adsr2: u8| Voice {
        adsr1: 0x8F, // ADSR mode, attack $F: at full scale within two samples
        adsr2,
        late: &[(0x5C, 0x01)], // KOFF
        late_settle: 1,        // short: catch the ramp in progress
        ..Voice::direct_gain()
    };
    // Sustain level 0, so the decay phase never ends and the sustain phase is never entered.
    // With sustain level 7 the decay finishes immediately at full scale and the sustain rate then
    // decays the envelope *before* key-off -- which is a real effect, and not this row's.
    let slow = voice_program(&looping_sample(), released(0x00));
    let fast = voice_program(&looping_sample(), released(0x1F));

    let mut a = Asm::new();
    a.c("--- sustain rate 0 ---");
    upload_and_run_tagged(&mut a, &slow, "_sr0");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01F0     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- sustain rate 31, the fastest in the table ---");
    upload_and_run_tagged(&mut a, &fast, "_sr31");
    a.l("rep #$30");
    a.l("lda f:$7E01F0");
    a.l("and #$00FF");
    a.record(184, "E7.09 ENVX mid-release with sustain rate 0");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(185, "E7.09 ENVX mid-release with sustain rate 31");
    a.c("The guard: the reading has to be mid-ramp. At $00 the ramp is over and at $7F it has not");
    a.c("begun, and in both cases every release rate looks the same.");
    a.l("lda f:$7E01F0");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x01,
        0x7E,
        "the sustain-rate-0 run was not caught mid-release, so the comparison below would hold for \
         any release rate at all",
    );
    a.c("Release ignores the field, so the two runs must agree — to within the sample the reading");
    a.c("was taken on. The two runs are separate uploads and their key-off-to-read windows can land");
    a.c(
        "one DSP sample apart, which at -8 per sample is a difference of 8 that says nothing about",
    );
    a.c("the rate. A rate-consulting release would differ by tens: rate 31 empties a full-scale");
    a.c("envelope in a handful of samples where rate 0 never moves it at all.");
    a.l("rep #$30");
    a.l("lda f:$7E01F0");
    a.l("and #$00FF");
    a.l("sta f:$7E01F2");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.l("sec");
    a.l("sbc f:$7E01F2");
    a.assert_abs_le(
        8,
        "changing the ADSR sustain rate moved the release ramp by more than the one sample the two \
         uploads can differ by — release runs at a fixed -8 per sample and consults no rate \
         register, which is exactly why a custom fade has to be done with GAIN instead",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.09",
        'E',
        "Release rate is fixed",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: release steps -8 per sample regardless of ADSR, which \
             is why a custom release has to be built from GAIN",
        ),
        Kind::Scored,
        None,
    )
}

/// The decay rate comes from `ADSR1` bits 6-4, indexed as `d*2+16`.
///
/// Decay's rate is not the raw field: it is `d*2+16`, which places the eight decay settings in the
/// upper half of the same counter table the other phases index into. A core using `d` verbatim, or
/// `d*2` without the offset, decays orders of magnitude too slowly — the envelope barely moves
/// where hardware would have crossed most of its range.
///
/// # Two decay rates, the same interval
///
/// Both voices attack at rate `$F` (full scale within two samples, `E7.04`) and set **sustain level
/// 0**, so the decay phase runs the whole way down and never hands over to sustain. That isolation
/// matters: `E7.09` first measured a sustain-rate effect and nearly attributed it to release, and
/// the same trap is available here in reverse. The only difference between the runs is the decay
/// field:
///
/// | run | `ADSR1` | decay | rate index |
/// |---|---|---|---|
/// | 1 | `$8F` | `0` | 16 |
/// | 2 | `$FF` | `7` | 30 |
///
/// Rate 30 is far faster than 16, so over the same interval run 2 must be well below run 1.
///
/// # The guard
///
/// Run 1 is asserted to be *mid*-decay rather than merely non-zero. If it had already reached the
/// bottom, both runs would read zero and the difference would vanish; if it had not started, both
/// would read full scale. Only between those does "run 2 is much lower" mean the rates differ, and
/// the assertion on the gap is what turns that into a number rather than an impression.
fn e7_05() -> Test {
    let decaying = |adsr1: u8| Voice {
        adsr1,
        adsr2: 0x00, // sustain level 0: decay runs the whole way, sustain is never entered
        // Long enough that the slow rate has visibly moved. Decay is exponential and ENVX is
        // E>>4, so the top of the range compresses -- index 16 needs a long window to leave full
        // scale at all, and at settle 4 it read $7D, one step from the guard's own bound.
        settle: 24,
        ..Voice::direct_gain()
    };
    let slow = voice_program(&looping_sample(), decaying(0x8F));
    let fast = voice_program(&looping_sample(), decaying(0xFF));

    let mut a = Asm::new();
    a.c("--- decay rate 0, index 16 ---");
    upload_and_run_tagged(&mut a, &slow, "_d0");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01F4     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- decay rate 7, index 30: much faster ---");
    upload_and_run_tagged(&mut a, &fast, "_d7");
    a.l("rep #$30");
    a.l("lda f:$7E01F4");
    a.l("and #$00FF");
    a.record(186, "E7.05 ENVX mid-decay at decay rate 0 (index 16)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(187, "E7.05 ENVX mid-decay at decay rate 7 (index 30)");
    a.c("The guard: run 1 has to be mid-decay. At the bottom or the top, both rates look alike.");
    a.l("lda f:$7E01F4");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x10,
        0x7E,
        "the slow decay was not caught in progress — at the bottom of its ramp or still at the \
         top, both decay rates produce the same reading and the gap below means nothing",
    );
    a.c("And the fast rate must have got substantially further down over the same interval.");
    a.l("lda f:$7E01F4");
    a.l("and #$00FF");
    a.l("sec");
    a.l("sbc f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x10,
        0x7F,
        "the two decay rates left the envelope in nearly the same place, so the rate field is not \
         being indexed as d*2+16 — using d verbatim puts both runs in the slow half of the table",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.05",
        'E',
        "Decay index d*2+16",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the decay phase indexes the counter table at d*2+16, \
             stepping E -= 1 then E -= E>>8",
        ),
        Kind::Scored,
        None,
    )
}

/// The sustain rate indexes the counter table **verbatim** — no offset, unlike decay.
///
/// Decay adds 16 to its field (`E7.05`); sustain does not. The difference matters most at the
/// bottom of the range: sustain rate `0` indexes rate 0, which never fires (`E7.01`), so a voice
/// held at sustain with `r = 0` never decays at all. Under any offset — `r+16`, `r*2+16` — that
/// same setting becomes a real rate and the note fades. Every held instrument in a game's
/// soundtrack depends on this, and a core with an offset here loses all of them.
///
/// # `r = 0` is the discriminating case
///
/// Comparing two fast rates would not settle anything: `r = 31` clamps to the top of the table
/// whether or not an offset is applied, so both models agree there. It is the zero end that
/// separates them.
///
/// | run | `ADSR2` | sustain rate | verbatim | with any positive offset |
/// |---|---|---|---|---|
/// | 1 | `$E0` | `0` | never fires — envelope parked | a real rate — envelope decays |
/// | 2 | `$FF` | `31` | the fastest rate — well down | the same, clamped |
///
/// Both voices attack at rate `$F` and set **sustain level 7**, which ends the decay phase
/// immediately at full scale and hands straight over to sustain. That is the configuration
/// `E7.09`'s first draft used by accident and had to abandon: there the sustain phase was
/// contamination, here it is the subject.
///
/// # What each assertion catches
///
/// Run 1 is asserted at exactly full scale, which is the whole "verbatim" claim — anything below it
/// means rate 0 fired. Run 2 is asserted substantially lower, which rules out the opposite failure:
/// a core that ignores the sustain rate entirely would park both runs at `$7F` and satisfy the
/// first assertion alone.
fn e7_06() -> Test {
    let sustained = |adsr2: u8| Voice {
        adsr1: 0x8F, // ADSR mode, attack $F: full scale within two samples
        adsr2,
        settle: 24, // long enough for a mid-table sustain rate to move the envelope
        ..Voice::direct_gain()
    };
    let never = voice_program(&looping_sample(), sustained(0xE0));
    let moderate = voice_program(&looping_sample(), sustained(0xFF));

    let mut a = Asm::new();
    a.c("--- sustain rate 0 ---");
    upload_and_run_tagged(&mut a, &never, "_sr0");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01F8     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- sustain rate 31 ---");
    upload_and_run_tagged(&mut a, &moderate, "_sr31");
    a.l("rep #$30");
    a.l("lda f:$7E01F8");
    a.l("and #$00FF");
    a.record(188, "E7.06 ENVX at sustain rate 0 (verbatim: never fires)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(189, "E7.06 ENVX at sustain rate 31");
    a.c("Rate 0 indexed verbatim never fires, so the envelope is still exactly where it parked.");
    a.l("sep #$20");
    a.l("lda f:$7E01F8");
    a.assert_a8(
        0x7F,
        "sustain rate 0 let the envelope decay, so the field is not being used verbatim — any \
         positive offset turns rate 0 into a real rate and every held note fades",
    );
    a.c("And the fastest rate must have moved a long way, or a core ignoring the field entirely");
    a.c("would pass on the first assertion alone. 31 rather than a mid-table rate because sustain");
    a.c("decay is exponential: from full scale it barely moves at first, and rate 16 only reached");
    a.c("$76 over this same window.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x00,
        0x60,
        "the fastest sustain rate left the envelope at or near full scale, so the sustain rate is \
         not being consulted at all",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.06",
        'E',
        "Sustain index verbatim",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the sustain phase indexes the counter table with the \
             ADSR2 rate field directly, unlike decay's d*2+16",
        ),
        Kind::Scored,
        None,
    )
}

/// Decay hands over to sustain when `E >> 8` equals the sustain level.
///
/// The boundary is a comparison on the envelope's **top three bits**, not a subtraction or a
/// threshold on the whole value: decay runs until `(E >> 8) == SL` and stops there. With the
/// sustain rate at zero the envelope then freezes, so where it parks *is* the boundary, readable
/// directly.
///
/// A core comparing the wrong field — `E >> 4`, or the raw `E` against `SL << 8` — parks the
/// envelope somewhere else entirely, and every instrument's sustain level comes out wrong while
/// attack, decay and release all still behave.
///
/// # Two levels, read where they park
///
/// Both voices attack at rate `$F` and decay at rate 7 (index 30, fast enough to reach either
/// boundary well inside the window — `E7.05` measured that rate crossing most of the range). The
/// sustain **rate** is 0, which never fires, so nothing moves once decay stops.
///
/// | run | `ADSR2` | sustain level | boundary `$100*(l+1)` | `ENVX` = `E >> 4` |
/// |---|---|---|---|---|
/// | 1 | `$60` | 3 | `$400` | `$40`-`$4F` |
/// | 2 | `$A0` | 5 | `$600` | `$60`-`$6F` |
///
/// **The row states the boundary twice and the two statements are not obviously the same.** It
/// gives both `$100*(l+1)` and "compare `(E>>8) == SL`", and reading only the comparison suggests
/// the envelope parks *inside* the level's own band — `$300`-`$3FF` for level 3. Measured, it parks
/// at `$40` and `$60`, which is `$100*(l+1)` exactly: the comparison fires on the value before the
/// decrement, so the envelope comes to rest on the boundary rather than below it. This test was
/// first written against the band reading and failed; the arithmetic in the row's own first clause
/// is the one that holds.
///
/// The two bands do not overlap and neither touches the ends of the range, so the assertions cannot
/// be satisfied by a stuck envelope: full scale is outside both, and zero is outside both. That is
/// what removes the need for a separate guard here — the ranges are the guard.
fn e7_07() -> Test {
    let parked_at = |adsr2: u8| Voice {
        adsr1: 0xFF, // ADSR mode, attack $F, decay 7: reach the boundary quickly
        adsr2,       // sustain level in bits 7-5, sustain rate 0 so it freezes on arrival
        settle: 24,
        ..Voice::direct_gain()
    };
    let low = voice_program(&looping_sample(), parked_at(0x60));
    let high = voice_program(&looping_sample(), parked_at(0xA0));

    let mut a = Asm::new();
    a.c("--- sustain level 3 ---");
    upload_and_run_tagged(&mut a, &low, "_sl3");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01FC     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- sustain level 5 ---");
    upload_and_run_tagged(&mut a, &high, "_sl5");
    a.l("rep #$30");
    a.l("lda f:$7E01FC");
    a.l("and #$00FF");
    a.record(190, "E7.07 ENVX parked at sustain level 3");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(191, "E7.07 ENVX parked at sustain level 5");
    a.c("Level 3 parks on the $100*(l+1) boundary, which is $400, so ENVX is in $40-$4F.");
    a.l("lda f:$7E01FC");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x40,
        0x4F,
        "sustain level 3 did not park the envelope on its $100*(l+1) boundary — anywhere else \
         means a different field is being compared, or the comparison fires after the decrement \
         rather than before it",
    );
    a.c("And level 5 on $600. The bands do not overlap and neither reaches the ends of the range,");
    a.c("so a stuck envelope cannot satisfy both.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x60,
        0x6F,
        "sustain level 5 did not park the envelope on its $100*(l+1) boundary, so the boundary \
         does not track the level",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.07",
        'E',
        "Sustain boundary",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the decay phase ends when (E >> 8) equals the ADSR2 \
             sustain level, giving a boundary of $100 per level",
        ),
        Kind::Scored,
        None,
    )
}

/// The attack rate indexes the counter table at `a*2+1`, stepping `+32`.
///
/// `E7.04` covers the one exception — rate `$F`, which fires every sample with a `+1024` step. This
/// covers the other fifteen, where the field is doubled and offset by one before it reaches the
/// table. A core using `a` verbatim indexes the slow half of the table for every setting, so every
/// attack is far too gradual while decay, sustain and release stay correct.
///
/// # Two rates from the middle of the range
///
/// `$8` and `$C`, giving indices 17 and 25 — both well away from `$F`, so this test says nothing
/// about the special case and `E7.04` says nothing about this one. Both voices hold sustain level 7
/// with rate 0, so whichever finishes its attack parks at full scale instead of decaying back and
/// muddying the comparison.
///
/// | run | `ADSR1` | attack | index | expected |
/// |---|---|---|---|---|
/// | 1 | `$88` | `$8` | 17 | partway up |
/// | 2 | `$8C` | `$C` | 25 | substantially higher |
///
/// # The guard
///
/// Run 1 is asserted to be genuinely mid-attack, not merely non-zero. At the top both rates read
/// `$7F` and the comparison is empty; at the bottom both read `$00` and the same is true. A core
/// indexing with `a` verbatim lands in the slow half of the table for both settings and fails that
/// bound rather than the comparison — which is the more informative failure, since it says the
/// indexing is wrong rather than that two numbers disagreed.
fn e7_03() -> Test {
    let attacking = |adsr1: u8| Voice {
        adsr1,
        adsr2: 0xE0, // sustain level 7, rate 0: park at the top rather than decay back
        settle: 4,
        ..Voice::direct_gain()
    };
    let slow = voice_program(&looping_sample(), attacking(0x88));
    let fast = voice_program(&looping_sample(), attacking(0x8C));

    let mut a = Asm::new();
    a.c("--- attack rate $8, index 17 ---");
    upload_and_run_tagged(&mut a, &slow, "_a8");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E0200     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- attack rate $C, index 25 ---");
    upload_and_run_tagged(&mut a, &fast, "_ac");
    a.l("rep #$30");
    a.l("lda f:$7E0200");
    a.l("and #$00FF");
    a.record(141, "E7.03 ENVX mid-attack at rate $8 (index 17)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(142, "E7.03 ENVX mid-attack at rate $C (index 25)");
    a.c("The guard: run 1 has to be mid-attack. At either end both rates read the same.");
    a.l("lda f:$7E0200");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x01,
        0x7E,
        "the slower attack was not caught in progress — at full scale or still at zero, both \
         rates read alike and the comparison below is empty. A core indexing with `a` verbatim \
         lands here, in the slow half of the table for both settings",
    );
    a.c("And the faster rate must have climbed substantially further over the same interval.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.l("sec");
    a.l("sbc f:$7E0200");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x08,
        0x7F,
        "the two attack rates left the envelope in nearly the same place, so the field is not \
         being doubled before it indexes the table",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.03",
        'E',
        "Attack index a*2+1",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the attack phase indexes the counter table at a*2+1 \
             and steps the envelope by +32",
        ),
        Kind::Scored,
        None,
    )
}

/// Does the decay→sustain boundary come from `VxGAIN` instead of `VxADSR2`? A golden vector.
///
/// The dossier records an `[ERRATA]`: the comparison that ends the decay phase reads its boundary
/// from `VxGAIN` bits 7-5 rather than from `VxADSR2`'s sustain level — even in `ADSR` mode, where
/// `GAIN` is supposed to be ignored entirely. A driver that leaves a stale `GAIN` value behind
/// would then get a sustain level it never asked for.
///
/// `E7.07` establishes the other half of this: with `GAIN` untouched, the boundary sits exactly
/// where `ADSR2` says. This test holds `ADSR2` fixed and moves `GAIN` instead, which is the only
/// arrangement that can tell the two sources apart.
///
/// | run | `ADSR2` | `GAIN` bits 7-5 | if the boundary is `ADSR2`'s | if it is `GAIN`'s |
/// |---|---|---|---|---|
/// | 1 | `$60` (level 3) | 0 | `$40` | near `$00` |
/// | 2 | `$60` (level 3) | 5 | `$40` | `$60` |
///
/// Both voices are in `ADSR` mode — `ADSR1` bit 7 set — so `GAIN` has no legitimate role at all and
/// any difference between the runs is the erratum showing through.
///
/// # Recorded, not scored
///
/// Whether a core models this is not something the dossier settles, and RustySNES demonstrably does
/// not: `E7.07` passes, which means its boundary tracks `ADSR2`. Asserting either behaviour would
/// be asserting an implementation choice. So the test names the two shapes and publishes both park
/// levels; variant 1 is "the boundary came from `ADSR2`", which is what every core measured here
/// reports.
fn e7_12() -> Test {
    let gained = |gain: u8| Voice {
        adsr1: 0xFF, // ADSR mode, attack $F, decay 7
        adsr2: 0x60, // sustain level 3, rate 0: park and freeze
        gain,        // ignored in ADSR mode -- unless the erratum is real
        settle: 24,
        ..Voice::direct_gain()
    };
    let low = voice_program(&looping_sample(), gained(0x00));
    let high = voice_program(&looping_sample(), gained(0xA0));

    let mut a = Asm::new();
    a.c("--- GAIN bits 7-5 = 0 ---");
    upload_and_run_tagged(&mut a, &low, "_g0");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E0204     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- GAIN bits 7-5 = 5, with ADSR2 unchanged ---");
    upload_and_run_tagged(&mut a, &high, "_g5");
    a.l("rep #$30");
    a.l("lda f:$7E0204");
    a.l("and #$00FF");
    a.record(143, "E7.12 park level with GAIN bits 7-5 = 0");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(181, "E7.12 park level with GAIN bits 7-5 = 5");
    a.c("Equal means the boundary came from ADSR2, which is what the register is for. Different");
    a.c("means GAIN reached a comparison it has no business in.");
    a.l("sep #$20");
    a.l("lda f:$7E0204");
    a.l("cmp f:$7E0101");
    a.l("bne :+");
    a.l("lda #$03          ; variant 1 = same level both times; the boundary is ADSR2's");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    a.l(":");
    a.l("lda #$05          ; variant 2 = the level followed GAIN; the erratum is modelled");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.12",
        'E',
        "GAIN sustain boundary",
        Provenance::Contested(
            "the dossier records the GAIN-sourced boundary as [ERRATA]; RustySNES takes it from \
             ADSR2 instead, which E7.07 asserts, so which behaviour is right is not settled here",
        ),
        Kind::Golden,
        None,
    )
}

/// `MUL YA` takes its N and Z flags from `Y` alone.
///
/// With `Y = $10` and `A = $10` the product is `$0100`, so `A` ends at `$00` — and yet `Z` is
/// **clear**, because the flags describe the high byte only. A core that sets `Z` from the 16-bit
/// result, or from `A`, gets this exactly backwards, and the failure is invisible to any test that
/// only checks the product.
fn e1_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x10)
        .mov_y_imm(0x10)
        .mul_ya()
        .mov_dp_a(PORT2) // product low, before anything can touch the flags
        .mov_dp_y(PORT3) // product high
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1) // PSW
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Product first: $10 * $10 = $0100.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(0x00, "MUL YA low byte is wrong");
    a.l("lda f:$7E0102");
    a.assert_a8(0x01, "MUL YA high byte is wrong");
    a.c("Then the flags. Z is bit 1 of PSW and must be CLEAR even though A came out $00.");
    a.l("lda f:$7E0100");
    a.l("and #$02");
    a.assert_a8(
        0x00,
        "MUL YA set Z although Y is non-zero — the flags come from Y alone, not from A or YA",
    );
    a.c("N is bit 7, and $01 is positive, so it must be clear too.");
    a.l("lda f:$7E0100");
    a.l("and #$80");
    a.assert_a8(0x00, "MUL YA set N although Y is $01");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.01",
        'E',
        "MUL YA flags from Y",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `DIV YA,X` is only valid while the quotient fits in nine bits, and 511 is the last one that does.
///
/// The SPC700's divide is a 9-bit-quotient machine: `V` is quotient bit 8 (`E1.05`), so `A` can
/// carry 0-255 and `V` the 256-511 range. Ask it for 512 and there is nowhere to put the answer.
/// The dossier marks the row `[ERRATA]` because what happens then is not "wraps" or "saturates in
/// the obvious way" — the hardware silently changes algorithm, and both halves of the result go
/// wrong together.
///
/// # The pair that shows it
///
/// Both divisions use `X = 2`, and differ only in the dividend:
///
/// | `YA` | true quotient | branch | `A` | `Y` |
/// |---:|---:|---|---|---|
/// | `$03FE` (1022) | **511** | normal (`Y < X<<1`) | `$FF` | `$00` |
/// | `$0400` (1024) | 512 | overflow | `$FF` | **`$02`** |
///
/// `A` is `$FF` in *both* rows, which is the trap: a test checking only the quotient sees the same
/// byte either side of the boundary and concludes nothing happened. **The remainder is what moves.**
/// 1024 / 2 leaves no remainder, and the hardware reports 2 — because past the boundary it is
/// running `E1.03`'s overflow formula, `Y = X + (YA - (X<<9)) % (256 - X)`, which is not a
/// remainder at all.
///
/// # Pinning the negative
///
/// A core that simply computes `YA / X` and `YA % X` and truncates gives `A = $00`, `Y = $00` for
/// the second division — 512 truncated to eight bits. So the wrong answer differs from the right
/// one in *both* reported bytes, and the failure cannot be produced by a rounding difference or an
/// off-by-one. The first division is the control: it is the same instruction with the same divisor
/// one step below the boundary, so a core failing it has a broken `DIV` rather than a boundary bug.
///
/// Two uploads rather than two divisions in one program: each division needs its own `A`/`Y`/`X`
/// setup and reports two bytes, and the three mailbox ports do not hold four values.
fn e1_07() -> Test {
    // Quotient 511 — the last one that fits.
    let mut ok = Spc::new();
    ok.mov_x_imm(0xEF).mov_sp_x();
    ok.mov_a_imm(0xFE).mov_y_imm(0x03).mov_x_imm(0x02);
    ok.div_ya_x().mov_dp_a(PORT1).mov_dp_y(PORT2);
    ok.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    // Quotient 512 — one past it.
    let mut over = Spc::new();
    over.mov_x_imm(0xEF).mov_sp_x();
    over.mov_a_imm(0x00).mov_y_imm(0x04).mov_x_imm(0x02);
    over.div_ya_x().mov_dp_a(PORT1).mov_dp_y(PORT2);
    over.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    a.c("--- $03FE / 2: quotient 511, the last value the 9-bit result can hold ---");
    upload_and_run_tagged(&mut a, &ok, "_ok");
    a.c("Stash it: the second upload overwrites the same three mailbox bytes.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("sta f:$7E01B4");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01B5");
    a.c("--- $0400 / 2: quotient 512, one past the boundary ---");
    upload_and_run_tagged(&mut a, &over, "_ov");
    a.l("rep #$30");
    a.l("lda f:$7E01B4");
    a.l("and #$00FF");
    a.record(99, "E1.07 quotient of $03FE / 2 (true 511)");
    a.l("lda f:$7E01B5");
    a.l("and #$00FF");
    a.record(103, "E1.07 remainder of $03FE / 2 (true 0)");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(
        135,
        "E1.07 quotient of $0400 / 2 (true 512 — cannot be represented)",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(136, "E1.07 remainder of $0400 / 2 (true 0)");
    a.c("The control first: at 511 the division is still exact.");
    a.l("sep #$20");
    a.l("lda f:$7E01B4");
    a.assert_a8(
        0xFF,
        "$03FE / 2 did not give a quotient of 511 (low byte $FF) — the divide is wrong below the \
         boundary, so nothing can be concluded about what happens above it",
    );
    a.l("lda f:$7E01B5");
    a.assert_a8(
        0x00,
        "$03FE / 2 did not give a remainder of 0, so the control division is wrong",
    );
    a.c("And one step past it, where both halves go wrong together.");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xFF,
        "$0400 / 2 returned a quotient other than $FF: a core computing YA/X and truncating gives \
         $00 here, which is the documented-invalid case behaving as though it were valid",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x02,
        "$0400 / 2 did not return the overflow branch's Y = X + (YA - (X<<9)) % (256 - X) = 2. A \
         core computing a true remainder returns 0 — correct arithmetic, and not what the hardware \
         does past a quotient of 511",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.07",
        'E',
        "DIV valid to Q<=511",
        Provenance::Documented(
            "SNESdev Wiki SPC700 reference and fullsnes, both flagging DIV as valid only for \
             quotients up to 511; the values past it follow E1.03's overflow formula",
        ),
        Kind::Scored,
        None,
    )
}

/// `DIV YA,X` on its normal branch: `A` is the quotient, `Y` the remainder.
///
/// The baseline the rest of `E1.02`-`E1.07` are read against. `$0020 / $08` is 4 remainder 0 — a
/// case with no overflow, no odd flag behaviour, and nothing to argue about, which is exactly what
/// makes it worth pinning first: every stranger `DIV` assertion is a deviation from this one.
fn e1_02() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_y_imm(0x00)
        .mov_a_imm(0x20) // YA = $0020
        .mov_x_imm(0x08)
        .div_ya_x()
        .mov_dp_a(PORT2) // quotient
        .mov_dp_y(PORT3) // remainder
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(0x04, "DIV YA,X quotient is wrong ($0020 / $08 = 4)");
    a.l("lda f:$7E0102");
    a.assert_a8(0x00, "DIV YA,X remainder is wrong");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.02",
        'E',
        "DIV YA,X normal branch",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `DIV YA,X` takes N and Z from the quotient alone, not from the remainder.
///
/// `$0003 / $08` is quotient 0, remainder 3 — so `Z` is **set** even though `Y` came back
/// non-zero. The errata matters because the remainder is the more interesting half of a divide,
/// and a core that flags the pair, or flags `Y`, reports "non-zero" for a result that is zero.
///
/// The companion case is checked in the same program: `$0020 / $08` is quotient 4, remainder 0,
/// where `Z` must be **clear**. One direction alone would pass on a core that never sets `Z`.
fn e1_06() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // quotient 0, remainder 3 -> Z set
        .mov_y_imm(0x00)
        .mov_a_imm(0x03)
        .mov_x_imm(0x08)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        // quotient 4, remainder 0 -> Z clear
        .mov_y_imm(0x00)
        .mov_a_imm(0x20)
        .mov_x_imm(0x08)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Quotient 0 with remainder 3: Z (bit 1) must be SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$02");
    a.assert_a8(
        0x02,
        "DIV YA,X left Z clear for a zero quotient — the flags come from the quotient, not the \
         remainder",
    );
    a.c("Quotient 4 with remainder 0: Z must be CLEAR. Without this half, a core that never sets");
    a.c("Z at all would pass the check above.");
    a.l("lda f:$7E0102");
    a.l("and #$02");
    a.assert_a8(0x00, "DIV YA,X set Z for a non-zero quotient");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.06",
        'E',
        "DIV flags from quotient",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `MOVW YA,dp` sets N and Z from the whole 16-bit value.
///
/// Loading `$0100` gives `A = $00` and `Y = $01`, and `Z` must be **clear** — a core that flags
/// the accumulator alone sets it. Loading `$8000` gives `A = $00` and `Y = $80`, and `N` must be
/// **set** — the same core leaves it clear. The two cases together pin both flags to the 16-bit
/// value rather than to either byte of it.
fn e1_15() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0x10, 0x00)
        .mov_dp_imm(0x11, 0x01) // $10/$11 = $0100
        .movw_ya_dp(0x10)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        .mov_dp_imm(0x12, 0x00)
        .mov_dp_imm(0x13, 0x80) // $12/$13 = $8000
        .movw_ya_dp(0x12)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("$0100: A is $00, so a core flagging the accumulator alone sets Z. It must be clear.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$02");
    a.assert_a8(
        0x00,
        "MOVW YA,dp set Z for $0100 — the flags describe all sixteen bits, not the low byte",
    );
    a.c("$8000: A is again $00, and N must be SET from bit 15.");
    a.l("lda f:$7E0102");
    a.l("and #$80");
    a.assert_a8(0x80, "MOVW YA,dp left N clear for $8000");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.15",
        'E',
        "MOVW YA sets 16-bit N/Z",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `DIV`'s H flag is a nibble comparison, not a half-carry.
///
/// It is set from `(Y & 15) >= (X & 15)` on the **inputs**, which has nothing to do with any carry
/// the division produces — the name is borrowed and the behaviour is not. Two divides that differ
/// only in which operand has the larger low nibble pin it: `Y=$05, X=$03` sets `H`, and the same
/// pair swapped clears it. A core computing a genuine half-carry gets no consistent answer at all,
/// because there is no half-carry in a division to compute.
fn e1_04() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // (Y & 15) >= (X & 15): 5 >= 3 -> H set
        .mov_y_imm(0x05)
        .mov_a_imm(0x00)
        .mov_x_imm(0x03)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        // 3 >= 5 is false -> H clear
        .mov_y_imm(0x03)
        .mov_a_imm(0x00)
        .mov_x_imm(0x05)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("H is bit 3 of PSW. Y=$05 against X=$03: the low nibbles compare 5 >= 3, so H is SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$08");
    a.assert_a8(
        0x08,
        "DIV left H clear although (Y & 15) >= (X & 15) — H here is a nibble compare, not a carry",
    );
    a.c("Swap the operands and the comparison fails, so H must be CLEAR.");
    a.l("lda f:$7E0102");
    a.l("and #$08");
    a.assert_a8(0x00, "DIV set H although (Y & 15) < (X & 15)");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.04",
        'E',
        "DIV H = nibble compare",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `DIV`'s V flag is bit 8 of the quotient.
///
/// The quotient can exceed 255 — the normal branch only guarantees it is under 512 — so `V` is how
/// the caller learns the byte it was handed is not the whole answer. `$0500 / $03` is 426, which
/// has bit 8 set; `$0300 / $05` is 153, which does not. The two together separate "V tracks the
/// quotient's ninth bit" from "V is set whenever something overflowed", which are the same
/// statement only for the first case.
fn e1_05() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // $0500 / $03 = 426: bit 8 set
        .mov_y_imm(0x05)
        .mov_a_imm(0x00)
        .mov_x_imm(0x03)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        // $0300 / $05 = 153: bit 8 clear
        .mov_y_imm(0x03)
        .mov_a_imm(0x00)
        .mov_x_imm(0x05)
        .div_ya_x()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("V is bit 6. Quotient 426 has bit 8 set, so V must be SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$40");
    a.assert_a8(0x40, "DIV left V clear for a quotient of 426 (bit 8 set)");
    a.c("Quotient 153 fits in a byte, so V must be CLEAR.");
    a.l("lda f:$7E0102");
    a.l("and #$40");
    a.assert_a8(
        0x00,
        "DIV set V for a quotient of 153, which fits in eight bits",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.05",
        'E',
        "DIV V is quotient bit 8",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `ADDW` carries into H from bit 11, not from bit 3.
///
/// `H` on the 8-bit adds is the bit-3 carry; on the word adds it is the bit-11 carry, because the
/// flag describes the high byte's low nibble. `$0FFF + $0001` crosses that boundary and `$0100 +
/// $0001` does not. A core that reuses its 8-bit half-carry reports the low byte's carry instead,
/// which is set in neither case here — so the first assertion catches it and the second confirms
/// the flag is not simply stuck.
fn e1_13() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0x10, 0xFF)
        .mov_dp_imm(0x11, 0x0F) // $10/$11 = $0FFF
        .mov_dp_imm(0x12, 0x01)
        .mov_dp_imm(0x13, 0x00) // $12/$13 = $0001
        .mov_dp_imm(0x14, 0x00)
        .mov_dp_imm(0x15, 0x01) // $14/$15 = $0100
        .movw_ya_dp(0x10)
        .addw_ya_dp(0x12) // $0FFF + $0001 = $1000: carries bit 11 -> 12
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2)
        .movw_ya_dp(0x14)
        .addw_ya_dp(0x12) // $0100 + $0001 = $0101: no such carry
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("$0FFF + $0001 crosses bit 11, so H (bit 3 of PSW) must be SET.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("and #$08");
    a.assert_a8(
        0x08,
        "ADDW left H clear for $0FFF + $0001 — H is the bit-11 carry on the word adds",
    );
    a.c("$0100 + $0001 does not, so H must be CLEAR — which also shows the flag is not stuck.");
    a.l("lda f:$7E0102");
    a.l("and #$08");
    a.assert_a8(0x00, "ADDW set H for $0100 + $0001");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.13",
        'E',
        "ADDW H = bit-11 carry",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Reading a timer counter returns four bits and clears it.
///
/// `$FD`-`$FF` are not registers holding a value; they are counters that a read consumes. The
/// upper nibble is not part of the count and the read has a side effect, so two reads in a row
/// give a number and then zero — which is the entire protocol for using them, and a core that
/// treats them as plain storage returns the same value twice and lets a driver double-count every
/// tick it observes.
///
/// The first read is only required to be non-zero: how far the timer has advanced depends on the
/// delay loop's exact cost, and asserting a specific count would be asserting the loop.
fn e3_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1: the fastest this timer runs
        .mov_dp_imm(0xF1, 0x01) // CONTROL: enable timer 0
        .delay(0x00) // 256 iterations, comfortably several ticks
        // Stop the timer before reading it. The two reads below are about eight cycles apart and a
        // tick at this divider lands every 128, so a tick falling between them is uncommon rather
        // than impossible -- and when it does, the second read is non-zero for a reason that has
        // nothing to do with whether the first one cleared it. It showed up as Mesen2 failing this
        // test on the PAL image only, after an unrelated change shifted the battery's timing.
        // Bit 7 keeps the IPL ROM mapped; see `Spc::release_to_ipl`.
        .mov_dp_imm(0xF1, 0x80)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2) // first read: the accumulated count
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT3) // second read: must be zero, because the first one cleared it
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Both halves of the first read in one assertion: it must have advanced (non-zero, or the");
    a.c("clear check below is vacuous) and it must fit in four bits (the upper nibble is not part");
    a.c("of the count). Expressed through the DSL rather than as hand-written verdict bytes, so");
    a.c("the code and its reason land in the generated ERROR_CODES.md like every other failure.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "the first read of $FD was zero or wider than four bits — a timer counter is a 4-bit \
         value, and a zero here would make the clear check below vacuous",
    );
    a.c("The second read must be zero: reading a timer counter consumes it.");
    a.l("sep #$20");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "the second read of $FD was non-zero — reading a timer counter must clear it",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.01",
        'E',
        "Timer read clears it",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F8` and `$F9` are plain RAM.
///
/// They sit in the middle of the I/O block and are not registers — nothing reads them, nothing
/// writes them, and a program may use them as two spare bytes. Worth pinning precisely because
/// they look like registers: a core that decodes the whole `$F0`-`$FF` range as I/O returns
/// something other than what was stored, and the failure surfaces far from the cause, in whatever
/// used them as scratch.
///
/// **This test was briefly recorded as a Contested golden, and that was wrong.** It appeared to
/// fail on all three implementations, which is this project's signature of a broken test — but the
/// cause was neither the test nor the emulators: an earlier test wrote `$F1` to enable a timer,
/// which also cleared bit 7 and unmapped the IPL ROM, so every APU upload after it silently died.
/// Once the release path re-maps the ROM, all three return what was written. The lesson is that
/// "three-way agreement means the test is wrong" is a good heuristic and not a proof: a harness
/// bug upstream of every implementation produces the same signature.
fn e3_14() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF8, 0x5A)
        .mov_dp_imm(0xF9, 0xA5)
        .mov_a_dp(0xF8)
        .mov_dp_a(PORT2)
        .mov_a_dp(0xF9)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x5A,
        "$F8 did not read back what was written, so it is not behaving as the plain RAM it \
         should be",
    );
    a.l("lda f:$7E0102");
    a.assert_a8(
        0xA5,
        "$F9 did not read back what was written, so it is not behaving as the plain RAM it \
         should be",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.14",
        'E',
        "$F8/$F9 are plain RAM",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F2` bit 7 makes writes through `$F3` do nothing.
///
/// The DSP register file is reached through an address latch (`$F2`) and a data port (`$F3`), and
/// the top bit of the address is not part of the address — it disables writing. A core that masks
/// `$F2` to five bits and ignores bit 7 lets a write through that hardware discards, which is the
/// wrong direction to be wrong in: the value lands, the driver never notices, and the sound is
/// subtly off rather than absent.
///
/// Checked by writing a known value, attempting to overwrite it with the bit set, and reading
/// back. The read is done with the bit clear, so the test cannot pass by the *read* also being
/// suppressed.
fn e3_11() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // MVOLL ($0C) = $7F, the value that must survive
        .mov_dp_imm(0xF2, 0x0C)
        .mov_dp_imm(0xF3, 0x7F)
        // Same register, address bit 7 set: this write must be discarded
        .mov_dp_imm(0xF2, 0x8C)
        .mov_dp_imm(0xF3, 0x00)
        // Read back with the bit clear
        .mov_dp_imm(0xF2, 0x0C)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT2)
        // Control: the same sequence WITHOUT bit 7 must take effect, or the check above would
        // pass on a core that simply never writes the DSP at all.
        .mov_dp_imm(0xF2, 0x0C)
        .mov_dp_imm(0xF3, 0x33)
        .mov_dp_imm(0xF2, 0x0C)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("The suppressed write must not have landed: MVOLL still holds $7F.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "a write through $F3 with $F2 bit 7 set took effect — that bit disables writing",
    );
    a.c("And an ordinary write must still work, or the check above proves only that nothing");
    a.c("reaches the DSP at all.");
    a.l("lda f:$7E0102");
    a.assert_a8(0x33, "an ordinary DSP write did not take effect");
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.11",
        'E',
        "$F2 bit 7 blocks writes",
        Provenance::Documented("SNESdev Wiki, S-DSP; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// DSP registers are independently addressable through `$F2`/`$F3`.
///
/// Foundational rather than exotic: every other DSP assertion is reached through this latch, so a
/// core that mis-decodes the address — masking too few bits, aliasing voice registers onto each
/// other, or latching the address at the wrong moment — makes every DSP test downstream
/// meaningless rather than failing.
///
/// Three registers in different parts of the file are written with distinct values and then read
/// back in a different order. The reordering is the point: reading them back in write order would
/// pass on a core that simply returns the last value written, which is the most likely way to get
/// the latch wrong.
fn dsp_addressing() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF2, 0x00) // voice 0 VOLL
        .mov_dp_imm(0xF3, 0x11)
        .mov_dp_imm(0xF2, 0x10) // voice 1 VOLL
        .mov_dp_imm(0xF3, 0x22)
        .mov_dp_imm(0xF2, 0x0C) // MVOLL
        .mov_dp_imm(0xF3, 0x33)
        // Read back in a different order than they were written.
        .mov_dp_imm(0xF2, 0x10)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT1)
        .mov_dp_imm(0xF2, 0x00)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT2)
        .mov_dp_imm(0xF2, 0x0C)
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Read back out of order: voice 1, then voice 0, then the master volume.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x22,
        "voice 1's VOLL did not read back — the DSP address latch is mis-decoded",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x11,
        "voice 0's VOLL did not read back; if it holds voice 1's value the voices are aliased",
    );
    a.l("lda f:$7E0102");
    a.assert_a8(0x33, "MVOLL did not read back");
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.11b",
        'E',
        "DSP register addressing",
        Provenance::Documented("SNESdev Wiki, S-DSP registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Writing `ENDX` clears it; it is not a register you can set.
///
/// `$7C` reports which voices have reached the end of their sample, and **any** write clears all
/// eight bits regardless of the value written. A core that models it as ordinary storage returns
/// whatever was written — so writing `$FF` and reading `$FF` back is the exact signature of
/// getting this wrong, and it is what a driver polling for sample-end would see as "every voice
/// finished" forever.
///
/// The assertion is deliberately "not `$FF`" rather than "exactly `$00`": with no sample playing
/// there is nothing to set the bits in the first place, so requiring zero would pass on a core
/// that had simply never implemented the register at all. What this test can prove is the narrower
/// and still useful thing — that the write did not stick.
///
/// **The read waits before looking.** `ENDX`, `OUTX` and `ENVX` are written back from an internal
/// buffer once per sample, and a CPU write landing one or two clocks before that writeback is lost
/// (`E7.17`) — so an immediate read-back is racing a hazard the hardware documentation warns about,
/// and its answer depends on which DSP clock the write happened to land on. That is not a detail
/// this test is about, and it is not hypothetical: with no delay here, one added byte elsewhere in
/// the battery was enough to move the write into the window and flip the result on snes9x at PAL
/// timing while leaving NTSC alone. Waiting a few samples asserts the same thing about the same
/// write, minus the coin flip.
fn e9_19() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x7C)
        .mov_dp_a(0xF2) // address latch: select ENDX ($7C)
        .mov_a_imm(0xFF)
        .mov_dp_a(0xF3) // data port: any write clears ENDX, so this must not store $FF
        .delay(0x40) // let the writeback window pass -- see above
        .mov_a_imm(0x7C)
        .mov_dp_a(0xF2) // select it again to read back
        .mov_a_dp(0xF3)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("A core storing the write returns $FF. Anything else means the write was treated as a");
    a.c("clear, which is the documented behaviour.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x00,
        0xFE,
        "ENDX read back as $FF, so the write was stored rather than treated as a clear",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.19",
        'E',
        "ENDX write clears it",
        Provenance::Documented("SNESdev Wiki, S-DSP registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The DSP's global registers are individually addressable and hold what is written.
///
/// The companion to the voice-register test: `$x C`/`$x D` are the global block — master and echo
/// volumes, echo feedback — and they are decoded from the same latch by a different part of the
/// address. A core that gets the voice registers right and aliases the globals (or vice versa)
/// passes one test and fails the other, which is why both exist.
///
/// Written low-to-high and read back high-to-low, so a core that simply returns the last value
/// written cannot pass.
fn dsp_global_regs() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    for (reg, val) in [(0x0Cu8, 0x11u8), (0x1C, 0x22), (0x2C, 0x33), (0x3C, 0x44)] {
        prog.mov_a_imm(reg).mov_dp_a(0xF2);
        prog.mov_a_imm(val).mov_dp_a(0xF3);
    }
    for (reg, port) in [(0x3Cu8, PORT1), (0x2C, PORT2), (0x1C, PORT3)] {
        prog.mov_a_imm(reg).mov_dp_a(0xF2);
        prog.mov_a_dp(0xF3).mov_dp_a(port);
    }
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Read back in the reverse of the write order: EVOLR, EVOLL, MVOLR.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(0x44, "EVOLR ($3C) did not read back");
    a.l("lda f:$7E0101");
    a.assert_a8(0x33, "EVOLL ($2C) did not read back");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x22,
        "MVOLR ($1C) did not read back; if it holds another register's value the globals are \
         aliased",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.11c",
        'E',
        "DSP global registers",
        Provenance::Documented("SNESdev Wiki, S-DSP registers; fullsnes"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// Voice playback: the part of the S-DSP that only moves when a sample is actually running.
// ---------------------------------------------------------------------------------------------
//
// Everything above pokes DSP registers and reads them back, which proves the address latch works
// and nothing else. The assertions below need a voice to be *playing*: `ENDX` only sets when a
// block with the end flag is decoded, and an envelope only reaches a value by being stepped. So
// each of these uploads a program that plants a BRR sample and a sample directory in APU RAM,
// points a voice at it, keys it on, waits, and reports what the DSP says afterwards.
//
// Three details make the difference between a test and a coin flip:
//
// * **A sample that does not end must be surrounded.** A block whose end flag is clear does not
//   stop; the DSP walks forward into whatever bytes follow, and some byte of the program's own
//   code eventually decodes as a header with the end flag set -- so `ENDX` sets for a reason that
//   has nothing to do with the test. The one test in that position pads its sample with silence
//   AND plays at a sixteenth of the sample rate, so the padding lasts far longer than the settle.
//   The other samples all carry an end flag, and need no padding at all: a looping sample repeats
//   forever and an end-without-loop sample stops the voice.
// * **The directory entry the test does not want used must be defined**, not merely absent.
//   `E5.11` distinguishes a correct entry address from a wrong one, and "wrong" has to point at
//   something known — here at address `$0000`, whose zero header decodes as silence forever.
// * **`KON` is cleared after keying on.** A core that re-keys a voice for as long as the bit is
//   set would hold the envelope at its attack value, and `E5.07` — which asserts the envelope
//   collapses — would fail against the core rather than against the behaviour.

/// Where `upload_and_run` places a program image in APU RAM.
const IMAGE_BASE: u16 = 0x0200;

/// The page the sample directory lives on, as the DSP's `DIR` register names it.
///
/// Page 1 is the stack page, and the entries sit at its very bottom while the stack is at its top
/// (`SP = $EF`): far enough apart that no program here comes close. A page of its own would cost
/// several hundred bytes of upload padding to reach, since a directory must be page-aligned.
const DIR_PAGE: u8 = 0x01;

/// Select a DSP register and write it.
fn dsp_write(p: &mut Spc, reg: u8, val: u8) {
    p.mov_a_imm(reg).mov_dp_a(0xF2);
    p.mov_a_imm(val).mov_dp_a(0xF3);
}

/// Select a DSP register and park its value in one of the four ports for the cart to read.
fn dsp_read_to(p: &mut Spc, reg: u8, port: u8) {
    p.mov_a_imm(reg).mov_dp_a(0xF2);
    p.mov_a_dp(0xF3).mov_dp_a(port);
}

/// One nine-byte BRR block: a header plus eight bytes of two four-bit samples each.
///
/// `flags` is the header's low two bits — bit 1 loop, bit 0 end — spelled that way round because
/// the header is `ssssffle` and the pair is routinely quoted as a "code": 0 normal, 1 end+mute,
/// 2 loop without end (which behaves as 0), 3 end+loop.
fn brr_block(shift: u8, filter: u8, flags: u8, hi: u8, lo: u8) -> Vec<u8> {
    let mut v = vec![(shift << 4) | (filter << 2) | flags];
    v.extend(core::iter::repeat_n((hi << 4) | lo, 8));
    v
}

/// `blocks`, followed by `run_out` blocks of silence for a non-looping voice to run out into.
///
/// `run_out` is zero for every sample that carries an end flag somewhere, which is most of them —
/// the padding is not free, and five copies of a generous run-out overflowed the ROM bank the
/// tests are linked into.
/// One BRR block whose eight data bytes cycle a byte pattern, for the named waveform vectors.
///
/// [`brr_block`] repeats a single nibble pair, which covers every test that wants a constant or a
/// simple alternation. `E6.11` names its vectors as nibble strings — `79797979`, `77997799`,
/// `77779999`, `7777CC44` — and the last two are not expressible as one repeated byte. The pattern
/// is cycled rather than padded, so a four-byte pattern fills the block twice and a one-byte
/// pattern eight times, which is what the nibble strings describe.
fn brr_block_pattern(shift: u8, filter: u8, flags: u8, pattern: &[u8]) -> Vec<u8> {
    let mut v = vec![(shift << 4) | (filter << 2) | flags];
    v.extend(pattern.iter().copied().cycle().take(8));
    v
}

fn brr_sample(blocks: &[Vec<u8>], run_out: usize) -> Vec<u8> {
    let mut v: Vec<u8> = blocks.concat();
    for _ in 0..run_out {
        v.extend(brr_block(0, 0, 0, 0, 0));
    }
    v
}

/// Build a program that plays `sample` on voice 0 through directory entry `srcn` and reports.
///
/// The reports are always the same three registers, in the same three ports: `ENDX` (`$7C`),
/// voice 0's `ENVX` (`$08`), and voice 0's `OUTX` (`$09`). Each test asserts on the one it is
/// about; a shared shape is worth more here than a minimal one, because the setup is long and a
/// difference between two of these programs should be a difference the test is *about*.
///
/// `pitch_hi` is the high byte of the voice's pitch: `$10` is one sample per output sample, `$01`
/// is a sixteenth of that. `settle` is a count of delay loops after key-on, each roughly a thousand
/// SPC700 cycles — a few dozen output samples. Both are deliberately coarse: these assertions are
/// about what the DSP eventually reports, not about when.
/// How one voice test differs from the next.
///
/// A struct rather than a widening argument list, because most of these fields are the same in
/// most tests and the interesting thing about any one program is the one or two that are not.
#[derive(Clone, Copy)]
struct Voice {
    /// Which directory entry the voice plays from.
    srcn: u8,
    /// High byte of the pitch. `$10` is one sample per output sample.
    pitch_hi: u8,
    /// `VxADSR1`: bit 7 enables the ADSR generator, bits 6-4 decay, bits 3-0 attack.
    adsr1: u8,
    /// `VxADSR2`: bits 7-5 sustain level, bits 4-0 sustain rate.
    adsr2: u8,
    /// `VxGAIN`, consulted only while `adsr1` bit 7 is clear.
    gain: u8,
    /// `NON`: one bit per voice, replacing its sample with the noise generator.
    non: u8,
    /// `VxVOLL`/`VxVOLR`, the per-voice volume the mixer applies. Downstream of `VxOUTX`.
    vol: u8,
    /// Delay loops between key-on and the read, each roughly a thousand SPC700 cycles.
    settle: u8,
    /// `(register, value)` writes made *after* settling — key-off, a `FLG` reset, anything whose
    /// effect is the thing being measured. Applied in order, back to back: a test that needs two
    /// registers written *together* (the key-off/key-on collapse cases, say) depends on nothing
    /// running between them.
    late: &'static [(u8, u8)],
    /// A DSP write immediately followed by a second write of the same register, as
    /// `(reg, first, second)`.
    ///
    /// Emitted as one `$F2` select and *two* `$F3` stores, so the two values are about five SPC
    /// cycles apart instead of the twelve two full `dsp_write`s would take. `E8.07` needs the pair
    /// to land inside a single KON/KOFF poll, and at twelve cycles it did not on every core.
    pulse: Option<(u8, u8, u8)>,
    /// Pulse `FLG` bit 7 before configuring anything, resetting the noise LFSR to `$4000`.
    ///
    /// The LFSR is global and survives between programs, so a test whose output depends on it
    /// starts from wherever the previous test left it. Two runs are only comparable if both begin
    /// from the same seed, which is what this buys.
    flg_reset: bool,
    /// DSP writes made **after** `late_settle`, just before the result registers are read.
    ///
    /// For undoing something `late` set up. A bit written in `late` and cleared in `late` is a
    /// pulse of about ten SPC cycles, which `KON`/`KOFF`'s two-sample poll will usually miss
    /// entirely -- `E8.07` is a whole test about that. Clearing it here instead holds it across the
    /// settle, where it is certain to be seen, and still leaves the register tidy for the next
    /// test.
    post: &'static [(u8, u8)],
    /// Delay loops after the `late` writes, before the registers are read.
    late_settle: u8,
}

impl Voice {
    /// A looping voice held at full direct gain: the shape most of these tests vary from.
    const fn direct_gain() -> Self {
        Self {
            srcn: 0,
            pitch_hi: 0x10,
            adsr1: 0x00,
            adsr2: 0x00,
            gain: 0x7F,
            non: 0x00,
            vol: 0x7F,
            settle: 4,
            late: &[],
            pulse: None,
            flg_reset: false,
            post: &[],
            late_settle: 0,
        }
    }
}

fn voice_program(sample: &[u8], v: Voice) -> Spc {
    let mut p = Spc::new();
    let addr = p.data_first(IMAGE_BASE, sample);
    p.mov_x_imm(0xEF).mov_sp_x();

    // The directory: four bytes per entry, start address then loop address, both little-endian.
    // Entry `srcn` gets the sample; the other of the first two entries is pointed at $0000, whose
    // zero header decodes as silence and never sets ENDX. An entry that is merely never written
    // would leave "wrong entry" meaning "whatever APU RAM happened to hold".
    let dir = u16::from(DIR_PAGE) << 8;
    for entry in 0u16..2 {
        let src = if u8::try_from(entry).expect("two entries") == v.srcn {
            addr
        } else {
            0x0000
        };
        let [lo, hi] = src.to_le_bytes();
        let base = dir + entry * 4;
        p.mov_a_imm(lo).mov_abs_a(base);
        p.mov_a_imm(hi).mov_abs_a(base + 1);
        p.mov_a_imm(lo).mov_abs_a(base + 2); // loop address: the same block, so code 3 repeats it
        p.mov_a_imm(hi).mov_abs_a(base + 3);
    }

    // Global state. FLG $20 leaves the DSP running and unmuted with echo *writes* disabled, which
    // is what a driver does before it has an echo buffer; the reset and mute bits are what the
    // power-on value has set. Noise, echo and pitch modulation are cleared explicitly rather than
    // assumed, since a previous test's program shares the same DSP.
    if v.flg_reset {
        // Bit 7 is the DSP's soft reset, and it re-seeds the noise LFSR to $4000. Written with
        // mute and echo-write-disable so nothing is audible while the reset is asserted.
        dsp_write(&mut p, 0x6C, 0xE0); // FLG: reset
    }
    dsp_write(&mut p, 0x6C, 0x20); // FLG
    dsp_write(&mut p, 0x5C, 0x00); // KOF
    dsp_write(&mut p, 0x3D, v.non); // NON
    dsp_write(&mut p, 0x4D, 0x00); // EON
    dsp_write(&mut p, 0x2D, 0x00); // PMON
    dsp_write(&mut p, 0x5D, DIR_PAGE); // DIR
    dsp_write(&mut p, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut p, 0x1C, 0x7F); // MVOLR

    dsp_write(&mut p, 0x00, v.vol); // VOL L
    dsp_write(&mut p, 0x01, v.vol); // VOL R
    dsp_write(&mut p, 0x02, 0x00); // PITCH low
    dsp_write(&mut p, 0x03, v.pitch_hi); // PITCH high: $10 is one sample per output sample
    dsp_write(&mut p, 0x04, v.srcn); // SRCN
    // ADSR2 and GAIN are written BEFORE ADSR1, which is the order the errata asks for (`E7.18`):
    // the mode is decided by ADSR1 bit 7, so writing it last means the generator is never briefly
    // running against parameters meant for the other mode.
    dsp_write(&mut p, 0x06, v.adsr2); // ADSR2
    dsp_write(&mut p, 0x07, v.gain); // GAIN, consulted only while ADSR1 bit 7 is clear
    dsp_write(&mut p, 0x05, v.adsr1); // ADSR1

    dsp_write(&mut p, 0x7C, 0x00); // ENDX: any write clears it, so start from a known state
    dsp_write(&mut p, 0x4C, 0x01); // KON voice 0
    p.delay(0x00);
    dsp_write(&mut p, 0x4C, 0x00); // and clear it — see the module comment

    for _ in 0..v.settle {
        p.delay(0x00);
    }

    for &(reg, val) in v.late {
        dsp_write(&mut p, reg, val);
    }
    if let Some((reg, first, second)) = v.pulse {
        dsp_write(&mut p, reg, first);
        // Only $F3 again: the address latch still holds `reg`, so this is the shortest gap the
        // SPC700 can put between two values of one DSP register.
        p.mov_a_imm(second).mov_dp_a(0xF3);
    }
    for _ in 0..v.late_settle {
        p.delay(0x00);
    }
    for &(reg, val) in v.post {
        dsp_write(&mut p, reg, val);
    }

    dsp_read_to(&mut p, 0x7C, PORT1); // ENDX
    dsp_read_to(&mut p, 0x08, PORT2); // voice 0 ENVX
    dsp_read_to(&mut p, 0x09, PORT3); // voice 0 OUTX
    p.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();
    p
}

/// A pair of handlers for the vector tests: one that means "arrived here", one that means "arrived
/// somewhere else".
///
/// Both end the program, so whichever runs is the one the cart hears from. That is what turns a
/// mis-computed vector from a hang into a *wrong answer* — a test whose only failure mode is the
/// timeout reports SKIP, which says the APU did not answer rather than that it answered wrongly.
///
/// **They restore `PSW` before handing back**, which is not tidiness. `BRK` sets the `B` flag and
/// nothing clears it afterwards, so a handler that simply finishes leaves `B` set in the SPC700 for
/// the whole rest of the battery — and `E4.02`, which reads the register state the IPL hands over,
/// then sees `$1A` where it expects `$0A`. It did, on the first run of these two tests. A test that
/// changes processor state every later test can see has to put it back.
fn vector_handlers(ok: u8, bad: u8) -> (Spc, Spc) {
    let mk = |mark: u8| {
        let mut p = Spc::new();
        p.mov_x_imm(0xEF)
            .mov_sp_x()
            .mov_a_imm(0x02)
            .push_a()
            .pop_psw() // clear B, which BRK set and nothing else clears
            .mov_a_imm(mark)
            .mov_dp_a(PORT1)
            .mov_a_imm(DONE)
            .mov_dp_a(PORT0)
            .release_to_ipl();
        p
    };
    (mk(ok), mk(bad))
}

/// `TCALL n` vectors through `[$FFDE - n*2]`, counting *down* from the top of the table.
///
/// Sixteen one-byte call instructions sharing a vector table at the very top of the address space —
/// which is inside the boot ROM while it is mapped, and ordinary RAM once it is not. The stride and
/// the direction are both easy to get backwards, and a driver using `TCALL` for its dispatch table
/// (they are one byte, which is the whole point) lands somewhere arbitrary if either is wrong.
///
/// The program unmaps the boot ROM so the table is writable, then plants the *right* handler at
/// `TCALL 1`'s slot and a different one either side of it. So a core that miscounts does not hang —
/// it runs the other handler and reports the wrong mark, which is a failure the cart can describe.
fn e2_08() -> Test {
    let (ok, bad) = vector_handlers(0xA1, 0xB2);

    let mut prog = Spc::new();
    let mut blob = ok.bytes().to_vec();
    let bad_at = u16::try_from(blob.len()).expect("handlers are small");
    blob.extend_from_slice(bad.bytes());
    let ok_addr = prog.data_first(IMAGE_BASE, &blob);
    let bad_addr = ok_addr + bad_at;

    prog.mov_x_imm(0xEF).mov_sp_x().mov_dp_imm(0xF1, 0x00); // unmap the boot ROM: the vector table is RAM again
    for (slot, addr) in [
        (0xFFDCu16, ok_addr), // TCALL 1
        (0xFFDE, bad_addr),   // TCALL 0 — one slot the wrong way
        (0xFFDA, bad_addr),   // TCALL 2 — the other way
    ] {
        let [lo, hi] = addr.to_le_bytes();
        prog.mov_a_imm(lo).mov_abs_a(slot);
        prog.mov_a_imm(hi).mov_abs_a(slot + 1);
    }
    prog.tcall(1);

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xA1,
        "TCALL 1 did not vector through $FFDC — $B2 means it read a neighbouring slot, so the \
         table is indexed with the wrong stride or the wrong direction",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.08",
        'E',
        "TCALL vector table",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `BRK` vectors through `$FFDE` — the same slot as `TCALL 0`, not one of its own.
///
/// The SPC700 has no separate break vector. `BRK` pushes `PC` and `PSW`, sets the `B` flag, and
/// jumps through the table entry `TCALL 0` already uses, so a program that installs a `TCALL 0`
/// handler has installed a `BRK` handler whether it meant to or not. A core that gives `BRK` its own
/// vector — the 65816 has one at `$FFE6`, which is where the instinct comes from — sends a stray
/// `BRK` somewhere the program never planned for.
///
/// Same shape as `E2.08`: the right handler at `$FFDE`, a different one next door, so a wrong vector
/// is a wrong answer rather than a hang.
fn e2_09() -> Test {
    let (ok, bad) = vector_handlers(0xC3, 0xD4);

    let mut prog = Spc::new();
    let mut blob = ok.bytes().to_vec();
    let bad_at = u16::try_from(blob.len()).expect("handlers are small");
    blob.extend_from_slice(bad.bytes());
    let ok_addr = prog.data_first(IMAGE_BASE, &blob);
    let bad_addr = ok_addr + bad_at;

    prog.mov_x_imm(0xEF).mov_sp_x().mov_dp_imm(0xF1, 0x00); // unmap the boot ROM
    for (slot, addr) in [(0xFFDEu16, ok_addr), (0xFFDC, bad_addr)] {
        let [lo, hi] = addr.to_le_bytes();
        prog.mov_a_imm(lo).mov_abs_a(slot);
        prog.mov_a_imm(hi).mov_abs_a(slot + 1);
    }
    prog.brk();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xC3,
        "BRK did not vector through $FFDE, the TCALL 0 slot — the SPC700 has no break vector of \
         its own",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.09",
        'E',
        "BRK shares TCALL 0",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F1` bit 5 clears the port 3 input latch.
///
/// The bits are strobes rather than switches: writing a 1 clears the corresponding pair of
/// CPU-to-APU input latches immediately, so a driver can drop stale commands without a second
/// write. A core that ignores them leaves a command the driver believed it had discarded sitting in
/// the port.
///
/// The value it clears is one the upload itself left there: `apu_upload` puts the entry address in
/// ports 2 and 3, so port 3 holds `$02`, the high byte of `$0200`. Using what the handshake already
/// wrote means the test needs nothing the mechanism does not already do.
///
/// **Two thirds of the dossier row are deliberately not covered here, and both need something this
/// test cannot reach.** Port 2's latch holds `$00` — the low byte of the same entry address — which
/// is indistinguishable from cleared, so only port 3 is checked. And the *non-persistence* half
/// ("the bit does not stay set") needs a second value to appear in a latch after the strobe, which
/// only the cart can put there; that needs a mid-program cart-to-APU handshake the upload mechanism
/// does not have. What is asserted is the immediate clear, and the failure text says so.
fn e3_03() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_dp(0xF7)
        .mov_dp_a(PORT1) // the IPL left the entry address's high byte here
        .mov_dp_imm(0xF1, 0xA0) // bit 5 clears ports 2 and 3; bit 7 keeps the boot ROM mapped
        .mov_a_dp(0xF7)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Port 3 holds $02, the high byte of the $0200 entry address the upload wrote there. If it");
    a.c("does not, the clear below would be measuring nothing.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x02,
        "port 3 did not hold the entry address's high byte, so the latch-clear check below would \
         be vacuous",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(0x00, "$F1 bit 5 did not clear the port 2/3 input latches");
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.03",
        'E',
        "$F1 bit 5 clears port 3",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `$F1` bit 7 controls what `$FFC0`-`$FFFF` *reads* as; writes always reach the RAM underneath.
///
/// The boot ROM is an overlay, not a region. A store to `$FFC0` lands in APU RAM whether or not the
/// ROM is mapped over it — the write is simply invisible until the overlay is switched off. That is
/// what makes the boot ROM's own space usable as ordinary RAM by a driver that no longer needs it,
/// and it is why an emulator that treats `$FFC0`-`$FFFF` as read-only while mapped loses a driver's
/// data with no error anywhere.
///
/// The whole claim in one program: write a byte with the ROM mapped, read back the *ROM* byte,
/// unmap, read back the *written* byte. `$CD` is the first byte of the canonical listing, which
/// `E4.01` checks in full.
fn e3_04() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF1, 0x80) // boot ROM mapped
        .mov_a_imm(0x5A)
        .mov_abs_a(0xFFC0) // goes to the RAM underneath, invisibly
        .mov_a_abs(0xFFC0)
        .mov_dp_a(PORT1) // still reads the ROM
        .mov_dp_imm(0xF1, 0x00) // unmap it
        .mov_a_abs(0xFFC0)
        .mov_dp_a(PORT2) // now the write is visible
        .mov_dp_imm(0xF1, 0x80) // and put it back, or there is no IPL to hand control to
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xCD,
        "a read of $FFC0 with the boot ROM mapped did not return the ROM's first byte",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x5A,
        "the byte written to $FFC0 while the ROM was mapped did not reach the RAM underneath — a \
         read-only overlay loses a driver's data silently",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.04",
        'E',
        "Writes pass under IPL",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A timer divider of `$00` means 256, not zero and not one.
///
/// `TnDIV` is the reload value of an 8-bit pre-divider, so writing `$00` selects its full period —
/// the *slowest* setting available, 256 times slower than `$01`. Read as a literal zero it becomes
/// either a division by nothing (a timer running 256 times too fast) or a stopped timer, and a
/// sound driver's tempo is wrong by more than two orders of magnitude either way.
///
/// Both halves are measured over the same delay: at `$01` the counter must have advanced, at `$00`
/// it must not have. The timer is stopped before each read, because the counter is four bits and the
/// reads are a few cycles apart — the same race `E3.01` was rebuilt to avoid.
fn e3_05() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1, the fastest
        .mov_dp_imm(0xF1, 0x81) // enable timer 0, boot ROM stays mapped
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80) // stop before reading
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT1)
        .mov_dp_imm(0xFA, 0x00) // T0DIV = 0, which means 256
        .mov_dp_imm(0xF1, 0x81) // re-enable: a 0->1 on the enable bit restarts the divider
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("The control: at divider 1 the counter advanced over this delay. Without it, the check");
    a.c("below would pass on a timer that never ran at all.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "timer 0 did not advance at divider 1, so the divider-0 check below would be vacuous",
    );
    a.c("And at divider 0 — meaning 256 — the same delay is nowhere near one tick.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "timer 0 ticked at divider $00 over a delay that is 256 times too short, so $00 was read \
         as a small number rather than as 256",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.05",
        'E',
        "TnDIV $00 means 256",
        Provenance::Documented("SNESdev Wiki, SPC700 timers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `TEST` bit 1 is the RAM write enable, and clearing it makes stores do nothing.
///
/// `$F0` is a hardware test register no game should touch, which is exactly why an emulator is
/// likely to model it as ordinary storage — and then a ROM that *does* touch it behaves differently
/// for reasons nothing in the trace explains. Bit 1 gates every write into APU RAM; with it clear,
/// stores execute, take their cycles, and change nothing.
///
/// The program seeds a byte, disables writes, stores a different byte, restores the register, and
/// only then reads back — reading while writes are disabled would be measuring the read path
/// instead. The final store proves the gate reopened, without which "the value did not change"
/// would also be what a broken write path looks like.
fn e3_10() -> Test {
    const SCRATCH: u16 = 0x0510;

    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x11)
        .mov_abs_a(SCRATCH) // seeded with writes enabled
        .mov_dp_imm(0xF0, 0x08) // TEST: bit 1 clear, bit 3 as it powers up
        .mov_a_imm(0x22)
        .mov_abs_a(SCRATCH) // executes, changes nothing
        .mov_dp_imm(0xF0, 0x0A) // restore the power-on value
        .mov_a_abs(SCRATCH)
        .mov_dp_a(PORT1)
        .mov_a_imm(0x33)
        .mov_abs_a(SCRATCH) // and writes work again
        .mov_a_abs(SCRATCH)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x11,
        "a store landed in APU RAM with TEST bit 1 clear, so the RAM write enable is not modelled",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x33,
        "the store after restoring TEST did not land either, so the check above says nothing about \
         bit 1",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.10",
        'E',
        "TEST gates RAM writes",
        Provenance::Documented("SNESdev Wiki, SPC700 I/O; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `DIV YA,X`'s overflow branch computes something entirely different, and sets `V`.
///
/// When `Y >= X << 1` the quotient will not fit in eight bits, and the instruction does not simply
/// saturate: it produces `A = 255 - (YA - (X << 9)) / (256 - X)` and
/// `Y = X + (YA - (X << 9)) % (256 - X)`, which is what the hardware's restoring-division loop
/// leaves behind when it runs off the end. A core that clamps, or returns the true quotient's low
/// byte, gets a different number — and games do hit this, because the check is `Y` against `X`
/// rather than anything about the dividend.
///
/// `YA = $4000`, `X = $10`: `$4000 - $2000 = $2000`, divided by `256 - 16 = 240` is 34 remainder
/// 32, so `A = 255 - 34 = $DD` and `Y = 16 + 32 = $30`. The true quotient would be `$400`, and its
/// low byte `$00` — nothing like `$DD`, which is what makes this a discriminating pair of numbers
/// rather than a coincidence.
///
/// `V` is asserted alongside them as a supporting check — it reports that the quotient overflowed
/// eight bits, which is the only warning a program gets that `A` is not the answer it asked for.
/// The *coverage* of that behaviour belongs to `E1.05`, which tests the flag on its own; claiming
/// it here as well was rejected by the duplicate-assertion gate, correctly.
fn e1_03() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_y_imm(0x40)
        .mov_a_imm(0x00) // YA = $4000
        .mov_x_imm(0x10)
        .div_ya_x()
        .mov_dp_a(PORT2) // quotient byte, before anything can touch the flags
        .mov_dp_y(PORT3) // remainder byte
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1) // PSW
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0xDD,
        "DIV's overflow branch did not produce 255 - (YA - (X << 9)) / (256 - X); $00 means the \
         true quotient's low byte, and $FF means a clamp",
    );
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x30,
        "DIV's overflow branch did not produce X + (YA - (X << 9)) % (256 - X) in Y",
    );
    a.c("V is bit 6, and it is the only warning a program gets that A is not a real quotient.");
    a.l("lda f:$7E0100");
    a.l("and #$40");
    a.assert_a8(
        0x40,
        "DIV did not set V although the quotient overflowed eight bits",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.03",
        'E',
        "DIV overflow branch",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `CLRV` clears the half-carry as well as the overflow flag.
///
/// The mnemonic names one flag and the instruction clears two. Nothing else on the SPC700 clears
/// `H` on its own, so a decimal-arithmetic routine that uses `CLRV` to prepare for `DAA` is relying
/// on the second effect — and on a core that clears only `V`, the stale `H` silently changes what
/// `DAA` does.
///
/// An `ADC` of `$7F + $01` sets both flags first: the signed result overflows and the low nibble
/// carries. The reading before `CLRV` is reported too, because "both flags are clear afterwards" is
/// vacuous unless they were set to begin with.
fn e1_12() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .clrc()
        .mov_a_imm(0x7F)
        .adc_a_imm(0x01) // -> $80: V set (signed overflow) and H set (nibble carry)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1) // PSW with both set
        .clrv()
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT2) // PSW after CLRV
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("V is bit 6, H is bit 3. Both must be set before CLRV or the check after it proves");
    a.c("nothing.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("and #$48");
    a.assert_a8(
        0x48,
        "ADC $7F + $01 did not set both V and H, so the CLRV check below would be vacuous",
    );
    a.l("lda f:$7E0101");
    a.l("and #$48");
    a.assert_a8(
        0x00,
        "CLRV left a flag set — it clears H as well as V, and nothing else on the SPC700 clears H",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.12",
        'E',
        "CLRV clears H too",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `DAA` applies two adjustments, and the second one can carry out of the byte.
///
/// `if (C || A > $99) { A += $60; C = 1; }` then `if (H || (A & 15) > 9) { A += 6; }`. Two cases
/// pin both halves: `$0A` trips only the low-nibble test and becomes `$10`; `$9A` trips both, and
/// the `+$60` followed by `+6` wraps it to `$00` with carry set. A core implementing `DAA` as a
/// single table lookup, or as the 65C816's decimal mode, gets the second case wrong.
///
/// `CLRC` and `CLRV` set up the entry flags — `CLRV` because it is the only way to clear `H`
/// (`E1.12`), and a stale `H` would trip the second adjustment for a reason the test is not about.
fn e1_08() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .clrc()
        .clrv() // clears H as well — see E1.12
        .mov_a_imm(0x0A)
        .daa()
        .mov_dp_a(PORT1) // only the low-nibble adjustment: $0A + 6 = $10
        .clrc()
        .clrv()
        .mov_a_imm(0x9A)
        .daa()
        .mov_dp_a(PORT2) // both: $9A + $60 = $FA, then + 6 = $00 with carry
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x10,
        "DAA on $0A did not apply the low-nibble adjustment, so $0A + 6 = $10 did not happen",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "DAA on $9A did not wrap to $00 — both adjustments apply, and $9A + $60 + 6 leaves the byte",
    );
    a.c("And the carry the first adjustment sets, which is what makes the wrap a decimal result");
    a.c("rather than a lost hundred.");
    a.l("lda f:$7E0102");
    a.l("and #$01");
    a.assert_a8(0x01, "DAA on $9A did not set the carry");
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.08",
        'E',
        "DAA adjustments",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `DAS` reads the *inverted* sense of `C` and `H`, which is what makes it the mirror of `DAA`.
///
/// `DAA` adjusts when a flag is **set**; `DAS` adjusts when one is **clear** —
/// `if (!C || A > $99) { A -= $60; C = 0; }` then `if (!H || (A & 15) > 9) { A -= 6; }`. A core
/// that copies `DAA`'s conditions and only flips the addition to a subtraction adjusts in exactly
/// the wrong cases, which is invisible on the values a test-by-eye would pick and wrong on almost
/// everything else.
///
/// Two runs of the same value differing only in `H`: with `H` set nothing happens to `$15`, and
/// with `H` clear it becomes `$0F`. `C` is set in both so the first condition stays out of the way,
/// and `$15` is chosen because it trips neither of `DAS`'s value tests — every difference between
/// the two answers is the flag.
///
/// Setting `H` needs an `ADC` with a nibble carry, because nothing sets it directly; clearing it
/// needs `CLRV` (`E1.12`), because nothing else clears it either.
fn e1_09() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        // --- H set, C set: no adjustment ---
        .clrv() // clears H as well as V (E1.12)
        .mov_a_imm(0x08)
        .clrc()
        .adc_a_imm(0x08) // $08 + $08 = $10: a carry out of bit 3, so H is set
        .setc()
        .mov_a_imm(0x15) // MOV leaves C and H alone
        .das()
        .mov_dp_a(PORT1)
        // --- H clear, C set: the low-nibble adjustment fires ---
        .clrv()
        .setc()
        .mov_a_imm(0x15)
        .das()
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x15,
        "DAS adjusted $15 with H and C both set — it adjusts when they are CLEAR, which is the \
         opposite of DAA",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x0F,
        "DAS did not subtract 6 from $15 with H clear, so it is not reading the inverted sense of \
         the half-carry",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.09",
        'E',
        "DAS mirrors DAA",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `TSET1` is an equality test, not a result test: `N`/`Z` come from `A - target` *before* the write.
///
/// The instruction ORs `A` into the target and reports flags — but the flags describe a comparison
/// of `A` against the target's **old** value, exactly as `CMP` would. That is the opposite of what
/// the mnemonic suggests, and the difference is visible whenever the result is non-zero but the
/// operands were equal: `$55` set into `$55` leaves `$55`, so a core reporting flags from the result
/// says "not zero" where the hardware says "equal".
///
/// Both cases are here because the second is the discriminator and the first is what proves the
/// instruction did its job at all — the target must come back with `A`'s bits set.
fn e1_10() -> Test {
    const SCRATCH: u16 = 0x0500;

    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(0x30)
        .mov_abs_a(SCRATCH)
        .mov_a_imm(0x0F)
        .tset1_abs(SCRATCH) // $0F vs $30: unequal, and the target becomes $3F
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1)
        .mov_a_abs(SCRATCH)
        .mov_dp_a(PORT2)
        .mov_a_imm(0x55)
        .mov_abs_a(SCRATCH + 1)
        .mov_a_imm(0x55)
        .tset1_abs(SCRATCH + 1) // equal, though the result $55 is not zero
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Unequal operands: Z (bit 1) clear, and the target came back with A's bits set.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("and #$02");
    a.assert_a8(0x00, "TSET1 set Z although A and the target differed");
    a.l("lda f:$7E0101");
    a.assert_a8(0x3F, "TSET1 did not OR A into its target");
    a.c("Equal operands, non-zero result. This is the case that separates a comparison from a");
    a.c("result: the hardware says equal, a core reading flags off the result says not-zero.");
    a.l("lda f:$7E0102");
    a.l("and #$02");
    a.assert_a8(
        0x02,
        "TSET1 did not set Z for equal operands, so its flags describe the result rather than a \
         comparison against the target's old value",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E1.10",
        'E',
        "TSET1 is a compare",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// A `CALL` pushes the address it will return to, not that address minus one.
///
/// The 65816 pushes `return - 1` and `RTS` compensates; the SPC700 does not, and a core that copies
/// the 65816's convention returns one byte early — into the middle of whatever instruction follows
/// the call. Nothing about that is subtle once it happens, and nothing about it is visible until it
/// does.
///
/// The subroutine never returns: it pops the two pushed bytes, reports them, and finishes the
/// program. Popping is the only way to *see* what was pushed, and having seen it there is nothing
/// left on the stack to return with. The expected value is computed from the program's own layout —
/// the offset immediately after the `CALL` — rather than written down, so it cannot drift out of
/// step with the code.
fn e2_07() -> Test {
    let mut sub = Spc::new();
    sub.pop_a()
        .mov_dp_a(PORT1) // first pop: the low byte, if the push order is high-then-low
        .pop_a()
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut prog = Spc::new();
    let routine = prog.data_first(IMAGE_BASE, sub.bytes());
    prog.mov_x_imm(0xEF).mov_sp_x().call_abs(routine);
    let expected = IMAGE_BASE + u16::try_from(prog.here()).expect("program fits APU RAM");

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        (expected & 0xFF) as u8,
        "the low byte of the pushed return address is wrong; one less than expected means the \
         65816's return-minus-one convention was applied",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        (expected >> 8) as u8,
        "the high byte of the pushed return address is wrong",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.07",
        'E',
        "CALL pushes exact addr",
        Provenance::Documented("SNESdev Wiki, SPC700 reference; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The IPL boot ROM is the same 64 bytes on every SNES.
///
/// `$FFC0`-`$FFFF` is a mask ROM inside the SPC700 — not part of the cartridge, not part of APU
/// RAM, and byte-identical on every console ever made. Everything about the audio boot depends on
/// it: a game's driver reaches the APU only through the handshake this ROM implements, so a wrong
/// byte in it does not degrade audio, it prevents any audio at all.
///
/// The program walks all 64 bytes and reports two checks: their sum, and a position-weighted
/// rolling value (`r = r * 2 + b`). The sum alone would accept any permutation of the same bytes,
/// which is precisely the mistake an emulator hand-transcribing the listing would make — the rolling
/// value is order-sensitive and costs three instructions.
///
/// It maps `$F1` bit 7 first. Every other program in this group leaves that bit alone, but a test
/// that reads the boot ROM cannot assume it is the boot ROM that is mapped there.
fn e4_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xF1, 0x80) // map the IPL ROM at $FFC0-$FFFF
        .mov_dp_imm(0x10, 0x00) // sum
        .mov_dp_imm(0x11, 0x00) // rolling value
        .mov_x_imm(0x00);
    let loop_top = prog.here();
    prog.mov_a_dp(0x11)
        .asl_a()
        .mov_dp_a(0x12) // rolling * 2
        .mov_a_abs_x(0xFFC0)
        .mov_dp_a(0x13) // this byte
        .clrc()
        .adc_a_dp(0x12)
        .mov_dp_a(0x11) // rolling = rolling * 2 + byte
        .mov_a_dp(0x10)
        .clrc()
        .adc_a_dp(0x13)
        .mov_dp_a(0x10) // sum += byte
        .inc_x()
        .cmp_x_imm(64);
    prog.bne_back(loop_top);
    prog.mov_a_dp(0x10)
        .mov_dp_a(PORT1)
        .mov_a_dp(0x11)
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0xB8,
        "the IPL ROM's bytes do not sum to $B8, so it is not the canonical boot ROM",
    );
    a.c("And the order, which a sum cannot see: r = r*2 + b over the same 64 bytes.");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x4F,
        "the IPL ROM summed correctly but its rolling checksum is wrong, so the bytes are right \
         and their order is not",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E4.01",
        'E',
        "IPL ROM contents",
        Provenance::Documented("the canonical 64-byte IPL listing; fullsnes, SNESdev Wiki"),
        Kind::Scored,
        None,
    )
}

/// The IPL hands a program a defined register state, not whatever it happened to leave.
///
/// `A = 0`, `X = 0`, `Y = 0`, `PSW = $02` — `Z` set, everything else clear. A driver that relies on
/// it (and they do: the entry state is why so many drivers open with a `MOV` rather than a load)
/// breaks on a core that jumps to the program with its own leftovers in the registers.
///
/// It depends on no earlier test having left a sticky flag set, which is a real coupling rather than
/// a theoretical one: `E2.09` executes a `BRK`, `BRK` sets `B`, and nothing on the SPC700 clears it
/// short of a `POP PSW`. That test's handler restores `PSW` before handing back for exactly this
/// reason — see [`vector_handlers`].
///
/// The program's first three instructions capture the state before anything can disturb it, using
/// only the flag-free moves. `Y` and `A` are reported bitwise-ORed together rather than separately: both
/// must be zero, so their OR being zero says both, and it buys a third register out of the three
/// mailbox bytes available. `SP` is the one part not checked — reading it needs a register this
/// test would then have to report somewhere.
///
/// **The `PSW` assertion masks the half-carry bit, and the reason is a finding.** RustySNES, snes9x
/// and Mesen2 all hand over `$0A`, not the documented `$02`: `Z` as described, plus `H` left set by
/// the boot ROM's own arithmetic. Three independent implementations agreeing that the listing is
/// incomplete is worth more than a fourth opinion, but it is not licence to assert `$0A` — that
/// would be scoring a measured value against a citation that says something else. So the test
/// asserts the documented bits (`Z` set, `N`/`V`/`I`/`C` clear) and publishes the full byte to the
/// measurement channel, where a number can be reported without being scored.
fn e4_02() -> Test {
    let mut prog = Spc::new();
    prog.mov_dp_a(0x10) // A at entry, stashed (flag-free)
        .mov_dp_x(PORT2) // X at entry (flag-free)
        .push_psw()
        .pop_a()
        .mov_dp_a(PORT1) // PSW at entry
        .mov_a_y()
        .or_a_dp(0x10)
        .mov_dp_a(PORT3) // Y | A, which is zero only if both are
        .mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.c("Publish the whole byte first — see the note above about $0A against a documented $02.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(112, "IPL handoff PSW");
    a.c("Then assert the documented bits only: Z set, N/V/I/C clear. Bit 3 (H) is masked out.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("and #$F7");
    a.assert_a8(
        0x02,
        "the IPL handed over with PSW other than $02 once the half-carry bit is masked — Z must \
         be set and N, V, I and C clear",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(0x00, "the IPL handed over with X non-zero");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "the IPL handed over with A or Y non-zero (they are reported ORed together)",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E4.02",
        'E',
        "IPL handoff state",
        Provenance::Documented("fullsnes, SNESdev Wiki, APU boot handshake"),
        Kind::Scored,
        None,
    )
}

/// An idle IPL announces itself as `$BBAA`, and it is the only thing a driver may wait for.
///
/// Port 0 reads `$AA` and port 1 reads `$BB` whenever the boot ROM is sitting in its ready loop —
/// at power-on, and again every time a program hands control back. It is the one piece of APU state
/// a game can check *before* it has uploaded anything, so every sound driver in existence opens by
/// polling for it.
///
/// No upload: this reads the two ports directly, which is exactly what the driver does. What it
/// measures is that the previous test's release actually returned the APU to the boot ROM — a core
/// that never re-announces leaves every later upload waiting on a handshake that will not come.
///
/// It *polls* rather than reading once, and that is not a weakening. The previous test released the
/// APU a few 65816 instructions ago; the SPC700 has to notice the release byte, jump to `$FFC0`, and
/// run the announcement, which takes real time on a processor running at a twentieth of the CPU's
/// clock. Reading once asserts that the handoff is instantaneous, which is not the claim and is not
/// true. Polling with a bound is what a driver does, and a core that never announces still fails —
/// it runs out the bound and reports SKIP rather than a pass.
///
/// **It polls the second byte and asserts the first**, which makes the test an ordering claim as
/// well. The boot ROM stores `$AA` to port 0 and then `$BB` to port 1, two separate instructions, so
/// once `$BB` is visible `$AA` must already be. Doing it the other way round — poll for `$AA`, then
/// read port 1 — lands in the gap between the two stores, and snes9x failed exactly that on the
/// first version of this test. A driver polling for `$AA` alone and then trusting port 1 has the
/// same bug.
fn e4_04() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label("wait");
    a.l("sep #$20");
    a.l("lda APUIO1");
    a.l("cmp #$BB");
    a.l("beq @ready");
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$4000");
    a.l("bne @wait");
    a.c("Never announced. SKIP rather than FAIL: an APU that is not in its boot ROM has told us");
    a.c("nothing about what the boot ROM announces, which is the only thing being measured.");
    a.l("sep #$20");
    a.l("lda #$FF");
    a.l("sta f:V_TEST_RESULT");
    a.l("jml test_restore");
    a.label("ready");
    a.c("Port 1 is $BB, which the boot ROM writes second — so port 0 must already hold the $AA it");
    a.c("writes first. The pair is the announcement; $BB alone is a byte a core could leave");
    a.c("anywhere.");
    a.l("sep #$20");
    a.l("lda APUIO0");
    a.assert_a8(
        0xAA,
        "port 1 announced $BB but port 0 does not read $AA, so the ready word is not $BBAA — or \
         the two bytes are written in the wrong order",
    );
    a.finish(
        "E4.04",
        'E',
        "IPL ready announcement",
        Provenance::Documented("fullsnes, SNESdev Wiki, APU boot handshake"),
        Kind::Scored,
        None,
    )
}

/// A looping block, for tests whose voice must simply keep playing.
///
/// Code 3 — end *and* loop — so the block repeats forever: the envelope is then the only thing in
/// the program that can move, which is what every envelope test below needs.
fn looping_sample() -> Vec<u8> {
    brr_sample(&[brr_block(0x8, 0, 0b11, 0x7, 0x9)], 0)
}

/// A looping block whose every nibble is the same, so the voice's output is a constant.
///
/// The three BRR-arithmetic tests below all read `VxOUTX`, and a sample whose nibbles alternate
/// gives an output that alternates with it — which of the two the read catches then depends on the
/// exact sample the DSP is on. With every nibble identical, filter 0 decodes the same value every
/// time and gaussian interpolation of a constant is that constant, so the reading is stable and the
/// assertion is about the arithmetic rather than about when the cart looked.
fn constant_sample(shift: u8, nibble: u8) -> Vec<u8> {
    filtered_sample(shift, 0, nibble)
}

/// [`constant_sample`] with a filter of the caller's choosing.
///
/// A constant input is the clearest way to see what a filter does: filter 0 reproduces it, and
/// every other filter is a recurrence over the samples before it, so it settles at a value the
/// filter's own formula decides rather than at the input.
fn filtered_sample(shift: u8, filter: u8, nibble: u8) -> Vec<u8> {
    brr_sample(&[brr_block(shift, filter, 0b11, nibble, nibble)], 0)
}

/// A mid-note `SRCN` change takes effect at the next loop point, and takes the new entry's **loop**
/// address.
///
/// A directory entry holds two addresses: where the sample begins and where it returns to when a
/// block's header says loop. A voice that is already running re-reads its entry's *loop* address
/// every sample, so writing `SRCN` under a held note does not switch the waveform immediately —
/// nothing happens until the current sample reaches a loop point, and what plays then is the new
/// entry's loop address rather than its start. A driver swapping instruments under a held note
/// therefore hears the change arrive late, and hears the tail of the new sample rather than its
/// attack.
///
/// # Three constant samples, so the reading names which one is playing
///
/// Every sample is constant at shift 12, differing only in nibble, so `VxOUTX` identifies the
/// source outright rather than by inference — and gaussian interpolation, which mixes four
/// consecutive samples, has nothing to contribute when all four are equal.
///
/// | entry | address | nibble | `OUTX` |
/// |---|---|---:|---|
/// | 0 | start and loop | `+7` | `$6E` — what is playing when the change happens |
/// | 1 | start | `+4` | `$3F` — what a core taking the *start* address would give |
/// | 1 | loop | `+2` | `$1F` — what hardware gives |
///
/// The three readings are far enough apart that the assertion distinguishes all of them: `$6E` says
/// the write never took effect, `$3F` says the wrong address was taken, `$1F` says it worked.
/// Entry 1's two addresses pointing at *different* samples is the whole apparatus — with the usual
/// arrangement, where an entry loops back to its own start, this row has no observable at all.
///
/// # The control is the same timing without the write
///
/// Phase 1 runs the identical delays and never touches `SRCN`, so it reads `$6E`. Without it,
/// "phase 2 reads `$1F`" could mean the voice had wandered somewhere on its own — and the settles
/// here are long, deliberately, so that is not an idle worry.
///
/// # What this does not cover
///
/// The row also says a change that lands *before* the voice has looped takes the new entry's
/// **start** address. In the pipeline that clause is really about the key-on delay: the start
/// address is read only while `keyon_delay` is non-zero, a window five output samples wide — about
/// 160 SPC cycles. The cart's only timing lever is a `DBNZ` delay loop with a granularity of six
/// cycles and an upload-dependent phase, so hitting that window reliably is not something this
/// battery can do. It is left uncovered rather than approximated.
fn e5_12() -> Test {
    /// The sample directory, on the stack page well below the stack itself.
    const DIR_ADDR: u16 = (DIR_PAGE as u16) << 8;

    // Four blocks, so entry 0 runs for about 64 output samples before it loops.
    let held: Vec<u8> = (0..4)
        .flat_map(|i| brr_block(0xC, 0, if i == 3 { 0b11 } else { 0 }, 0x7, 0x7))
        .collect();
    let start_sample = brr_sample(&[brr_block(0xC, 0, 0b11, 0x4, 0x4)], 0);
    let loop_sample = brr_sample(&[brr_block(0xC, 0, 0b11, 0x2, 0x2)], 0);

    // One blob, because `data_first` places the program's whole data area and can only run once.
    // The three samples' addresses are then offsets into it, computed from the same lengths that
    // built it so they cannot drift apart.
    let mut blob = held;
    let off_start = u16::try_from(blob.len()).expect("the data area is far smaller than APU RAM");
    blob.extend_from_slice(&start_sample);
    let off_loop = u16::try_from(blob.len()).expect("the data area is far smaller than APU RAM");
    blob.extend_from_slice(&loop_sample);

    let mut prog = Spc::new();
    let a_held = prog.data_first(IMAGE_BASE, &blob);
    let a_start = a_held + off_start;
    let a_loop = a_held + off_loop;
    prog.mov_x_imm(0xEF).mov_sp_x();
    for (i, addr) in [a_held, a_held, a_start, a_loop].into_iter().enumerate() {
        let [lo, hi] = addr.to_le_bytes();
        let at = DIR_ADDR + u16::try_from(i).expect("four directory words") * 2;
        prog.mov_a_imm(lo).mov_abs_a(at);
        prog.mov_a_imm(hi).mov_abs_a(at + 1);
    }

    dsp_write(&mut prog, 0x6C, 0x20); // FLG: running, unmuted, echo writes off
    dsp_write(&mut prog, 0x5C, 0x00); // KOF
    dsp_write(&mut prog, 0x3D, 0x00); // NON
    dsp_write(&mut prog, 0x4D, 0x00); // EON
    dsp_write(&mut prog, 0x2D, 0x00); // PMON
    dsp_write(&mut prog, 0x5D, DIR_PAGE); // DIR
    dsp_write(&mut prog, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut prog, 0x1C, 0x7F); // MVOLR
    dsp_write(&mut prog, 0x00, 0x7F); // VOL L
    dsp_write(&mut prog, 0x01, 0x7F); // VOL R
    dsp_write(&mut prog, 0x02, 0x00); // PITCH low
    dsp_write(&mut prog, 0x03, 0x10); // PITCH high: one sample per output sample
    dsp_write(&mut prog, 0x06, 0x00); // ADSR2
    dsp_write(&mut prog, 0x07, 0x7F); // GAIN: direct, full scale
    dsp_write(&mut prog, 0x05, 0x00); // ADSR1: GAIN is in charge

    e5_12_phase(&mut prog, false, PORT1); // the control: identical timing, SRCN left alone
    e5_12_phase(&mut prog, true, PORT2); // and the same again, with the mid-note write
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(209, "E5.12 OUTX with SRCN left alone (the control)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(210, "E5.12 OUTX after a mid-note SRCN change");
    a.c("The control: the same delays, no write, so entry 0 is still playing.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x68,
        0x76,
        "the voice was not playing entry 0 after the same delays with SRCN untouched, so it moved \
         on its own and the phase below cannot be attributed to the write",
    );
    a.c("And with the write, entry 1's loop address: nibble +2, so around $1F.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x18,
        0x26,
        "a mid-note SRCN change did not land on the new entry's loop address. Around $6E means the \
         write never took effect at all; around $3F means the entry's start address was taken \
         instead, which is what a voice still inside its key-on delay would do and this one is \
         long past that",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.12",
        'E',
        "SRCN change source",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: a mid-playback SRCN change takes the new entry's start \
             address, or its loop address if the voice has already passed a loop point",
        ),
        Kind::Scored,
        None,
    )
}

/// Emit one of [`e5_12`]'s two phases: key on entry 0, wait `blocks` delay blocks, swap to entry 1.
///
/// The key-off and the settle after it are what make the two phases independent. Without keying the
/// voice off between them the second phase would inherit the first's already-looped state, and both
/// readings would be the loop address — a test that agrees with itself and means nothing.
fn e5_12_phase(prog: &mut Spc, change: bool, port: u8) {
    dsp_write(prog, 0x5C, 0x01); // KOFF, so this phase starts from a stopped voice
    prog.delay(0x40);
    dsp_write(prog, 0x5C, 0x00);
    dsp_write(prog, 0x04, 0x00); // SRCN: entry 0, the four-block sample
    dsp_write(prog, 0x4C, 0x01); // KON
    prog.delay(0x00);
    dsp_write(prog, 0x4C, 0x00);
    prog.delay(0x00); // the voice is well established and has passed at least one loop point
    if change {
        dsp_write(prog, 0x04, 0x01); // SRCN: entry 1, whose two addresses differ
    }
    prog.delay(0x00); // long enough for the next loop point to have come round
    dsp_read_to(prog, 0x09, port); // OUTX names which sample is playing
}

/// BRR nibbles are signed: `$8` is `-8`, not `+8`.
///
/// Each nibble is a two's-complement four-bit value in `-8..+7`, so the top bit is a sign and a
/// core reading them as unsigned produces a waveform that is entirely positive — audible as a DC
/// offset and a wrong shape rather than as silence. With every nibble `$8` and the envelope at full
/// direct gain, `VxOUTX` — the post-envelope sample's high byte — must have its sign bit set.
///
/// Its control is `E5.03`, which asserts the *positive* half from the same shift and the same
/// envelope. Either test alone is satisfied by a core that always reports one sign.
fn e5_02() -> Test {
    let prog = voice_program(&constant_sample(0x8, 0x8), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x80,
        0xFF,
        "a sample of $8 nibbles produced a positive output, so the nibbles were read as unsigned",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.02",
        'E',
        "BRR nibbles are signed",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// A decoded BRR sample is `(nibble << shift) >> 1`, and it reaches the output.
///
/// The positive control for `E5.02` and the "it plays at all" control for `E5.04`: the same shift,
/// the same envelope, nibbles of `+7`, and `VxOUTX` must be positive and non-zero. What it pins is
/// narrow but load-bearing — that a decoded sample of this magnitude survives the envelope and the
/// interpolator to a reading the cart can see.
///
/// It asserts a range rather than the exact byte on purpose. The exact value is
/// `((nibble << shift) >> 1) * E >> 11`, high byte, and `E` here is a direct gain of `$7F0` rather
/// than full scale — so pinning the byte would be asserting the envelope's exact value through a
/// test about BRR decoding, and it would move if the gain in the shared setup ever changed.
fn e5_03() -> Test {
    let prog = voice_program(&constant_sample(0x8, 0x7), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x01,
        0x7F,
        "a sample of $7 nibbles did not produce a positive non-zero output; zero means nothing \
         reached the output at all, and a negative value means the nibbles were sign-confused",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.03",
        'E',
        "BRR sample arithmetic",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Shifts 13, 14 and 15 do not shift: they collapse the sample to `$0000` or `$F800`.
///
/// The header's shift field goes to 15 but the decoder only implements 0-12; the top three are a
/// documented special case that discards the nibble's magnitude entirely and keeps only its sign —
/// `$0000` for a positive nibble, `$F800` for a negative one. A core that takes the shift at face
/// value produces an enormous sample instead of a silent one, which is the difference between a
/// quiet passage and a burst of noise.
///
/// The nibbles here are `+7`, so the documented output is zero. Zero is also what silence looks
/// like, which is exactly why `E5.03` exists: it is the same sample at a legal shift, and it must
/// read non-zero.
fn e5_04() -> Test {
    let prog = voice_program(&constant_sample(0xD, 0x7), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "shift 13 did not collapse a positive sample to zero, so the invalid shifts are being \
         applied as ordinary ones",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.04",
        'E',
        "Invalid shift collapses",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Filter 1 is a recurrence, not a scale factor: a constant input settles far above itself.
///
/// The filter keeps most of the previous output and adds the new sample, so a constant input does
/// not stay at its own value — it converges on a fixed point an order of magnitude higher. With the
/// same shift and nibble as `E5.03`, whose filter-0 reading is single-digit, `VxOUTX` here settles
/// in the `$40`-`$7F` band and stays there: a genuine fixed point, not a moment in a waveform.
///
/// The bounds are deliberately loose, and it is worth saying why rather than quietly picking a
/// number. The exact fixed point is the documented recurrence's, scaled by an envelope that is a
/// direct gain of `$7F0` rather than full scale, and divided down by the decoder's internal
/// representation — a chain in which every link belongs to a different assertion. What this test
/// claims is the part that is filter 1's alone: that it accumulates. A core that ignores the filter
/// field reports `E5.03`'s single-digit answer and fails by an enormous margin, which is the
/// failure worth catching — the documentation is emphatic that these formulas are exact and that
/// simplifying them breaks game audio.
fn e5_05() -> Test {
    let prog = voice_program(&filtered_sample(0x8, 1, 0x7), Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x40,
        0x7F,
        "filter 1 did not settle well above its constant input — a single-digit reading is \
         filter 0's answer, so the filter was not applied",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.05",
        'E',
        "BRR filter 1",
        Provenance::Documented("fullsnes, S-DSP BRR filters; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// A linear-decrease envelope that runs out of room clamps at zero; it does not wrap.
///
/// Key-on puts the envelope at zero, and a linear-decrease GAIN steps it down by 32 every sample,
/// so the very first step underflows. The hardware holds it at zero. A core that lets the eleven-bit
/// value wrap reports something near full scale instead — silence becoming maximum volume, which is
/// the loudest possible way to get an envelope wrong.
///
/// Its control is `E7.11`, the same custom-GAIN machinery driving the ramp the other way: without
/// it, "the envelope is zero" is also what a core with no GAIN ramps at all reports.
fn e7_14() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            gain: 0x9F, // custom, linear decrease, rate $1F (every sample)
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "a linear-decrease envelope did not clamp at zero; a large reading means it wrapped",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.14",
        'E',
        "GAIN decrease clamps",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// The envelope's full scale is `$7FF`, and `VxENVX` reports it shifted down four.
///
/// A voice attacked at rate `$F` reaches maximum essentially at once — the documented step for that
/// rate is `+1024` per sample, against `+32` for every other — and with the sustain level at `7`
/// the boundary is the top of the range, so it arrives and stays. `VxENVX` then reads exactly
/// `$7F`.
///
/// The exactness is the test. An eleven-bit envelope reported as `E >> 4` cannot produce a value
/// above `$7F`, so bit 7 is always clear; a core carrying a full byte of envelope, or shifting by
/// three, reports `$FF` or `$FE` here and is otherwise indistinguishable — every other envelope
/// test only ever checks a direction or a range.
fn e7_15() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            // ADSR on, attack $F. The decay rate does not matter here: sustain level 7 puts the
            // decay boundary at the top of the range, so the envelope is in sustain from the
            // moment it arrives, and sustain rate 0 never fires, so it stays.
            adsr1: 0x8F,
            adsr2: 0xE0,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "a fully attacked envelope did not read $7F; $FF or $FE means ENVX is not E >> 4 of an \
         eleven-bit envelope",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.15",
        'E',
        "ENVX is E >> 4",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// BRR decoding keeps running for a voice that has been released.
///
/// Key-off starts the release ramp; it does not stop the decoder. The voice goes on reading blocks,
/// following loop points and setting `ENDX`, for as long as the DSP runs. A core that treats
/// key-off as "switch this voice off" gets the audible result right — the envelope reaches zero
/// either way — and the state wrong, so a driver watching `ENDX` to know when a released voice's
/// sample has wrapped waits forever.
///
/// # It is the distinction `E7.08` says it cannot make
///
/// `E7.08` keys off the same voice and asserts the envelope reaches zero, and its own note records
/// the gap: *"the one thing it cannot distinguish is a core that stops the voice outright on
/// key-off instead of releasing it, since both end at zero."* This test is that distinction, and it
/// uses the decoder rather than the envelope to make it — because the decoder is the part that
/// keeps running.
///
/// # `ENDX` has to be cleared at key-off, not before it
///
/// The sample is one block carrying end+loop, so `ENDX` sets within a few samples of key-on — long
/// before the key-off. Reading it at the end would then say nothing about what happened after.
///
/// So the late writes clear `ENDX` **and then** key off, in that order, and the settle after them is
/// long enough for the looping block to come round again. Anything found in `ENDX` at the end was
/// therefore set by a decode that happened after the voice was released.
///
/// # The guard
///
/// `ENVX` is asserted zero first. A core whose key-off did nothing at all would leave the decoder
/// running for the ordinary reason — the voice is still playing — and pass the `ENDX` check without
/// having been tested. Requiring the envelope to have released as well means the voice really was
/// keyed off, and `ENDX` is then evidence about a released voice rather than about a running one.
fn e5_10() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            // Order matters: clear ENDX, then release. Reversed, the clear could land after the
            // first post-release decode and erase the very thing being looked for.
            late: &[(0x7C, 0x00), (0x5C, 0x01)],
            late_settle: 12,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("The guard first: the voice has to have actually been released.");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the envelope was not zero after key-off, so the voice was never released and the ENDX \
         reading below would be about an ordinary playing voice",
    );
    a.c("ENDX was cleared at key-off, so anything here was set by a decode that happened after.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$0001");
    a.assert_a16_range(
        1,
        1,
        "ENDX bit 0 did not set again after the voice was released: the core stopped decoding on \
         key-off instead of only releasing the envelope. E7.08 cannot see this — both behaviours \
         take the envelope to zero",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.10",
        'E',
        "Released voice decodes",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: key-off begins the release ramp and does not halt BRR \
             decoding, which continues to follow loop points and set ENDX",
        ),
        Kind::Scored,
        None,
    )
}

/// Key-on takes five output samples to reach the envelope generator.
///
/// A `KON` write does not start a voice immediately. The DSP holds it for five output samples —
/// reading the sample directory, fetching the first BRR block and priming the interpolator — before
/// the envelope begins to climb. At 32 SPC cycles a sample (`E10.01`) that is 160 cycles, and it is
/// why a driver that keys on and immediately reads `ENVX` sees zero and can conclude the voice
/// never started.
///
/// # Timing it against the poll loop that measures it
///
/// The observable is "how long until `ENVX` goes non-zero", polled through the DSP register port.
/// That loop costs about fifteen cycles a pass, which is half a sample — comparable to the thing
/// being measured, so it cannot be ignored and is subtracted instead:
///
/// | phase | voice state | measures |
/// |---|---|---|
/// | A | already sounding at full scale | the poll loop's own latency |
/// | B | keyed off, then keyed on | that latency **plus** the key-on delay |
///
/// The difference is the delay. Timer 2 at `T2DIV = 2` ticks every **32 SPC cycles** — exactly one
/// output sample — so the reading counts samples directly against a baseline of about zero:
///
/// | if the delay were | ticks over baseline |
/// |---|---:|
/// | 0 samples | 0 |
/// | **5 samples, plus the write and the first climbing sample** | **~7** |
/// | 16 samples | 16, which wraps `TnOUT` to 0 |
///
/// `T2DIV = 1` was tried first and is finer, but it put the reading at 15 of `TnOUT`'s 16 values —
/// close enough to the wrap that the NTSC/PAL drift gate caught it failing on the PAL image alone.
/// Resolution is worth nothing if it costs headroom.
///
/// # What the assertion is careful not to claim
///
/// A band, not a number. The `KON` write itself costs about a dozen cycles that phase A does not
/// pay, the poll can overshoot by one pass, and `ENVX` goes non-zero on the *first* sample after
/// the delay rather than the last of it — three small biases in the same direction. So the band is
/// wide enough to hold all of them and still exclude both zero delay and any delay long enough to
/// wrap the counter, which is what the row is actually about.
fn e8_02() -> Test {
    let mut p = e8_02_sounding_voice();

    // T2DIV = 2: one tick per 32 SPC cycles, which is exactly one output sample. T2DIV = 1 was
    // finer and put the reading at 15 of TnOUT's 16 values -- close enough to the wrap that the
    // NTSC/PAL drift gate caught it failing on the PAL image alone. Resolution is worth nothing
    // if it costs headroom.
    p.mov_dp_imm(0xFC, 0x02);
    e8_02_time_to_envx(&mut p, "a", None, PORT1);

    // KOFF, and let the release run all the way to silence. A full-scale envelope is $7F0 and
    // release steps it down by 8 a sample, so that is 254 samples -- and one delay block is about
    // 48. The first version waited two blocks, the envelope was still around $40 when the poll
    // started, and the poll exited on its first pass: the measured "key-on delay" was one tick.
    dsp_write(&mut p, 0x5C, 0x01);
    for _ in 0..7 {
        p.delay(0x00);
    }
    dsp_write(&mut p, 0x5C, 0x00);
    dsp_read_to(&mut p, 0x08, PORT3); // the arming guard: this has to be zero
    e8_02_time_to_envx(&mut p, "b", Some(()), PORT2);
    dsp_write(&mut p, 0x4C, 0x00);
    p.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &p);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(
        227,
        "E8.02 timer 2 ticks to a non-zero ENVX, voice already sounding (baseline)",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(228, "E8.02 the same, measured from a KON write");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(
        229,
        "E8.02 ENVX after the release, before the second key-on (the arming guard)",
    );
    a.c("The voice has to be silent before it is keyed on again. If the release has not finished,");
    a.c("the poll below exits on its first pass and reports a delay of nothing.");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        0,
        "the envelope had not reached zero before the second key-on, so the poll started with \
         ENVX already non-zero and measured the loop rather than the delay",
    );
    a.c("The baseline has to be small, or the poll loop is slower than the delay it measures and");
    a.c("the subtraction below is meaningless.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        4,
        "polling ENVX on an already-sounding voice took more than four 16-cycle ticks, so the loop \
         is slow enough to swamp a five-sample delay and phase B measures the loop rather than the \
         key-on",
    );
    a.c("And the key-on has to cost about seven more ticks, one tick being one sample: the");
    a.c("five-sample delay, the KON write itself, and the first sample on which ENVX climbs.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.l("sec");
    a.l("sbc f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        4,
        11,
        "a KON write did not cost about seven timer-2 ticks more than the same poll on a sounding \
         voice, where a tick is one output sample. Near zero means the voice starts immediately \
         and a driver reading ENVX straight after KON would see it start; near or past sixteen \
         means the counter wrapped and the delay is far longer than five samples",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E8.02",
        'E',
        "Key-on takes 5 samples",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: KON is held for five output samples while the \
             directory and the first BRR block are fetched, before the envelope starts",
        ),
        Kind::Scored,
        None,
    )
}

/// Emit [`e8_02`]'s prologue: a voice in direct-`GAIN` mode, keyed on and settled at full scale.
///
/// Direct `GAIN` rather than `ADSR` because the measurement is of the *key-on* delay and nothing
/// else: with `ADSR1` bit 7 clear the envelope jumps to the `GAIN` value on the first sample it is
/// allowed to move, so "how long until `ENVX` is non-zero" is the delay plus one sample rather than
/// the delay plus an attack ramp.
fn e8_02_sounding_voice() -> Spc {
    let mut p = Spc::new();
    let addr = p.data_first(IMAGE_BASE, &looping_sample());
    p.mov_x_imm(0xEF).mov_sp_x();
    let dir = u16::from(DIR_PAGE) << 8;
    let [lo, hi] = addr.to_le_bytes();
    p.mov_a_imm(lo).mov_abs_a(dir);
    p.mov_a_imm(hi).mov_abs_a(dir + 1);
    p.mov_a_imm(lo).mov_abs_a(dir + 2);
    p.mov_a_imm(hi).mov_abs_a(dir + 3);
    dsp_write(&mut p, 0x6C, 0x20); // FLG
    dsp_write(&mut p, 0x5C, 0x00); // KOF
    dsp_write(&mut p, 0x3D, 0x00); // NON
    dsp_write(&mut p, 0x4D, 0x00); // EON
    dsp_write(&mut p, 0x2D, 0x00); // PMON
    dsp_write(&mut p, 0x5D, DIR_PAGE);
    dsp_write(&mut p, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut p, 0x1C, 0x7F); // MVOLR
    dsp_write(&mut p, 0x00, 0x7F); // VOL L
    dsp_write(&mut p, 0x01, 0x7F); // VOL R
    dsp_write(&mut p, 0x02, 0x00); // PITCH low
    dsp_write(&mut p, 0x03, 0x10); // PITCH high
    dsp_write(&mut p, 0x04, 0x00); // SRCN
    dsp_write(&mut p, 0x06, 0x00); // ADSR2
    dsp_write(&mut p, 0x07, 0x7F); // GAIN: direct, full scale
    dsp_write(&mut p, 0x05, 0x00); // ADSR1 bit 7 clear, so GAIN governs and ENVX jumps at once
    dsp_write(&mut p, 0x4C, 0x01); // KON
    p.delay(0x00);
    dsp_write(&mut p, 0x4C, 0x00);
    p.delay(0x20); // the voice is sounding, which is phase A's premise
    p
}

/// Emit: time how long `ENVX` takes to read non-zero, optionally keying the voice on first.
///
/// The timer is started *before* the `KON` write rather than after, so the write's own cost lands
/// inside the measured interval. Phase A does not pay it, which is one of the small biases the
/// assertion's band is sized to absorb — putting the write outside the interval instead would trade
/// it for a worse one, since the DSP could then act on the key-on before the timer starts.
fn e8_02_time_to_envx(p: &mut Spc, tag: &str, keyon: Option<()>, port: u8) {
    p.mov_a_dp(0xFF); // drain timer 2, which is read-to-clear
    p.mov_dp_imm(0xF1, 0x84); // enable timer 2, IPL ROM still mapped
    if keyon.is_some() {
        dsp_write(p, 0x4C, 0x01);
    }
    let poll = p.here();
    p.mov_a_imm(0x08).mov_dp_a(0xF2);
    p.mov_a_dp(0xF3);
    p.cmp_a_imm(0x00);
    let _ = tag;
    p.beq_back(poll);
    p.mov_dp_imm(0xF1, 0x80);
    p.mov_a_dp(0xFF).mov_dp_a(port);
}

/// `KON` on a voice that is already playing restarts it, zeroing the envelope on the way.
///
/// `KON` is not "start if stopped". Every write of a set bit re-enters the key-on sequence for that
/// voice: the BRR decoder returns to the sample's start address, the envelope is forced to zero, and
/// the attack begins again. That is why a driver retriggering a held note hears it restart from
/// silence rather than continue, and why `KON` cannot be used to "refresh" a voice.
///
/// # The envelope is the observable, and the attack has to be slow enough to see
///
/// Both runs use `ADSR` mode with attack rate 8 — slow enough that the envelope is still climbing
/// well after the key-on delay, so "restarted" and "carried on" are far apart rather than a sample
/// or two of difference.
///
/// | run | late writes | `ENVX` |
/// |---|---|---|
/// | 1 | none | high — 24 delay blocks of climbing |
/// | 2 | `KON` voice 0 | low — the climb started over one delay block ago |
///
/// # Both halves are guarded, and for different reasons
///
/// Run 1 must be **high**: with an attack that never got going, both runs read near zero and the
/// comparison holds for a core that ignores `KON` entirely. Run 2 must be **low in absolute terms**
/// rather than merely lower than run 1 — "lower" would also be satisfied by a core whose envelope
/// happens to be decaying, which is a different phenomenon reading the same way.
///
/// The gap is large by construction, so neither reading is sensitive to which DSP sample the cart
/// caught. A test of this shape that demanded the two agree, or differ exactly, would drift every
/// time anything ahead of it in the battery moved — which is what happened to `E7.09`.
fn e8_03() -> Test {
    let playing = |late: &'static [(u8, u8)], late_settle: u8| Voice {
        adsr1: 0x88, // ADSR mode, attack rate 8: still climbing after the key-on delay
        adsr2: 0x00,
        settle: 24,
        late,
        late_settle,
        ..Voice::direct_gain()
    };
    let held = voice_program(&looping_sample(), playing(&[], 1));
    let retriggered = voice_program(&looping_sample(), playing(&[(0x4C, 0x01)], 1));

    let mut a = Asm::new();
    a.c("--- left alone: the attack has been climbing for 24 delay blocks ---");
    upload_and_run_tagged(&mut a, &held, "_held");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01F4     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- keyed on again one delay block before the reading ---");
    upload_and_run_tagged(&mut a, &retriggered, "_retrig");
    a.l("rep #$30");
    a.l("lda f:$7E01F4");
    a.l("and #$00FF");
    a.record(114, "E8.03 ENVX after 24 blocks of attack, left alone");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(
        115,
        "E8.03 ENVX one block after a second KON on the same playing voice",
    );
    a.c("The guard: without a climb there is nothing for a restart to undo.");
    a.l("lda f:$7E01F4");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x20,
        0x7F,
        "the untouched run's envelope had barely climbed, so both runs read low whatever KON does \
         and the comparison below would hold for a core that ignores it",
    );
    a.c("And the restart has to have zeroed it, not merely lowered it.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x00,
        0x18,
        "writing KON to a voice that was already playing left its envelope near where it had \
         climbed to, so the write was treated as 'start if stopped' — a driver retriggering a held \
         note would hear it continue instead of restarting from silence",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E8.03",
        'E',
        "KON restarts a voice",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: KON re-enters the key-on sequence unconditionally, \
             resetting the BRR pointer and zeroing the envelope, whether or not the voice was \
             already playing",
        ),
        Kind::Scored,
        None,
    )
}

/// `KOFF = $FF` immediately followed by `KOFF = $00` leaves the voices playing.
///
/// `KOFF` is not acted on at the instant it is written: the DSP samples it on a poll that comes
/// round every second output sample. Two writes closer together than that collapse into one
/// reading, and the reading the poll takes is the *second* value — so setting every key-off bit and
/// clearing it again a few cycles later is, to the DSP, a poll that saw `$00`. Nothing is released.
///
/// A core that applies `KOFF` at the moment of the write instead behaves entirely differently: the
/// `$FF` releases every voice, and the `$00` that follows cannot un-release anything, because
/// release is a state the envelope has entered rather than a level held on a register. The voice
/// falls silent.
///
/// # This is the pair to `E7.08`
///
/// The two tests key off the same voice, from the same setup, and expect opposite outcomes.
/// `E7.08` writes `KOFF = $01` once and asserts the envelope reaches zero — a single write is
/// unambiguous, whenever it is sampled. This one writes `$FF` then `$00` back to back and asserts
/// the envelope is *untouched*, still sitting at the direct gain of `$7F` that `E7.10` pins.
///
/// Together they bracket the mechanism: the first shows key-off works, the second shows it is
/// sampled rather than edge-triggered. Either alone is weak — "the envelope is still `$7F`" would
/// be satisfied by a core whose key-off never worked at all, and `E7.08` is what rules that out.
///
/// The gain is direct and constant, so `$7F` is the value a voice that was left alone must read.
/// There is no ramp to time and no window to hit: any release at all, however brief, steps the
/// envelope down by 8 per sample and cannot return.
///
/// # Recorded, not scored: the outcome is phase-dependent
///
/// This shipped as a scored assertion and should not have. The pulse is about five SPC cycles wide
/// and the poll interval is about sixty-four, so a poll falls *inside* the pulse roughly one time
/// in twelve — and which way it goes depends on where the DSP happens to be when the test runs.
/// Adding an unrelated test earlier in the battery shifted that phase and the assertion failed,
/// with nothing about the emulator having changed.
///
/// So it records. The row states the outcome flatly, but the mechanism it describes is the same one
/// `E8.05` and `E8.06` hedge as *"usually"*, and a claim that is usually true is not one a battery
/// can score. Variant 1 is the documented outcome; variant 2 is a poll landing inside the pulse —
/// or a core acting on the write, which `E7.08` is what separates.
///
/// # The pulse has to be short, and the first version was not short enough
///
/// Written as two ordinary `dsp_write`s the pair sits about twelve SPC cycles apart, and that
/// **failed on Mesen2's PAL image while passing on its NTSC one** — the SPC is synchronised to a
/// CPU clock that differs by region, so the same instruction sequence spans a slightly different
/// fraction of the DSP's poll interval. A test claiming to be about the DSP that changes answer
/// with the video standard is measuring the harness, not the hardware.
///
/// It now emits one `$F2` register select and two `$F3` stores, which puts the values about five
/// cycles apart. That is both robust across all four core/region combinations and a better
/// statement of the row, which is precisely about a pulse shorter than the poll interval.
fn e8_07() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            // Back to back, with nothing between them: the two writes have to land inside one
            // poll interval for the collapse to happen at all.
            pulse: Some((0x5C, 0xFF, 0x00)),
            late_settle: 12,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(
        164,
        "E8.07 envelope after a KOFF $FF/$00 pulse ($7F = the pulse was missed)",
    );
    a.c("The verdict deliberately says only that the measurement was taken. Which way the pulse");
    a.c("fell is in slot 164, and classifying on it would make this test's verdict change every");
    a.c("time anything ahead of it in the battery shifts the DSP poll phase -- including a region");
    a.c("switch, which is how the NTSC/PAL drift gate first caught it.");
    a.l("sep #$20");
    a.l("lda #$03          ; variant 1 = captured; $7F in slot 164 means no poll saw the pulse");
    a.l("sta f:$7EE010");
    a.l("jml test_restore");
    apu_timeout_arm(&mut a);
    a.finish(
        "E8.07",
        'E',
        "KOFF pulse collapses",
        Provenance::Contested(
            "KON/KOFF are sampled every second output sample, so whether a short pulse is seen \
             depends on where the poll falls inside it -- which makes the outcome phase-dependent \
             rather than fixed, the same hedge E8.05 and E8.06 carry as \"usually\"",
        ),
        Kind::Golden,
        None,
    )
}

/// `KOFF` and `KON` written together silence a voice faster than `KOFF` alone.
///
/// `KOFF` starts the release ramp, which steps the envelope down by 8 every sample and takes about
/// eight milliseconds to reach silence (`E7.08`). `KON` *zeroes* the envelope outright before its
/// attack begins — and `E8.04` establishes that `KOFF` outranks `KON`, so the attack never starts.
/// The combination therefore gets the zero without the ramp, which is why drivers use it to cut a
/// voice dead rather than fade it.
///
/// A core that treats the pair as "`KOFF` wins, ignore the `KON` entirely" produces the ramp in
/// both cases and is indistinguishable from one that got it right — *if* you only look after the
/// ramp has finished. So this looks early.
///
/// # Two runs, read before the ramp completes
///
/// Both key off; only the second also keys on. The settle after them is deliberately short — one
/// delay block rather than the twelve `E7.08` uses — so the release has visibly started but not
/// finished:
///
/// | run | late writes | expected `ENVX` |
/// |---|---|---|
/// | 1 | `KOFF` | non-zero — mid-ramp |
/// | 2 | `KOFF`, `KON` | `$00` — zeroed outright |
///
/// # The guard
///
/// Run 1 is asserted **non-zero** first, and it is the whole basis of the comparison. If the settle
/// were long enough for the ramp to finish, both runs would read zero and the test would pass
/// having demonstrated nothing — the failure mode that `C9.05` shipped with. A zero there means the
/// window was mistimed, and says so in its own failure code rather than masquerading as a result.
///
/// Run 2 clears `KON` again afterwards. `KON` is write-triggered and non-persistent (`E8.04`), but
/// leaving a bit set in a register the next test inherits is how one test's state becomes another
/// test's mystery.
fn e8_10() -> Test {
    let off_only = voice_program(
        &looping_sample(),
        Voice {
            late: &[(0x5C, 0x01)],
            late_settle: 1,
            ..Voice::direct_gain()
        },
    );
    let off_and_on = voice_program(
        &looping_sample(),
        Voice {
            late: &[(0x5C, 0x01), (0x4C, 0x01)],
            post: &[(0x4C, 0x00)],
            late_settle: 1,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    a.c("--- run 1: KOFF alone, read while the release ramp is still running ---");
    upload_and_run_tagged(&mut a, &off_only, "_off");
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.l("sta f:$7E01E0     ; ENVX, before the second upload overwrites the mailbox copy");
    a.c("--- run 2: KOFF and KON together, same interval ---");
    upload_and_run_tagged(&mut a, &off_and_on, "_both");
    a.l("rep #$30");
    a.l("lda f:$7E01E0");
    a.l("and #$00FF");
    a.record(176, "E8.10 ENVX after KOFF alone (must still be ramping)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(177, "E8.10 ENVX after KOFF + KON together");
    a.c("The guard: if the ramp already finished, both runs read zero and the comparison below");
    a.c("would pass having compared nothing.");
    a.l("sep #$20");
    a.l("lda f:$7E01E0");
    a.l("cmp #$00");
    a.fail_if_eq(
        "the KOFF-only run had already reached silence, so the settle is too long and the \
         comparison below cannot distinguish the two paths",
    );
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "KOFF and KON written together left the envelope mid-ramp, exactly as KOFF alone did — \
         the KON's zeroing of the envelope was dropped along with its attack",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E8.10",
        'E',
        "KOFF+KON cuts faster",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: KON zeroes the envelope before its attack, and KOFF \
             outranks KON, so the pair silences immediately where KOFF alone ramps",
        ),
        Kind::Scored,
        None,
    )
}

/// Key-off releases at a fixed rate, every sample, all the way to zero.
///
/// Release is not a rate you can choose: it steps `-8` every sample regardless of anything in
/// `ADSR` or `GAIN`, which takes an envelope from full scale to silence in about eight
/// milliseconds. This voice is held at a direct gain of `$7F` that nothing else would ever move —
/// `E7.10` asserts exactly that — so a reading of zero after `KOF` can only be the release path.
///
/// The one thing it cannot distinguish is a core that stops the voice outright on key-off instead
/// of releasing it, since both end at zero. That distinction needs a reading *during* the ramp, and
/// the delay loop here is too coarse to place one.
fn e7_08() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            late: &[(0x5C, 0x01)], // KOF voice 0
            late_settle: 12,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the envelope was not zero well after key-off, so release did not run to silence",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.08",
        'E',
        "Key-off releases to zero",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Custom GAIN, linear increase: the envelope climbs from zero to full scale on its own.
///
/// With `ADSR1` bit 7 clear and `VxGAIN` bit 7 set, the low five bits are a rate and bits 6-5 pick
/// one of four ramps. Mode `10` is linear increase, `+32` per step, and rate `$1F` steps every
/// sample — so a voice keyed on at envelope zero reaches `$7FF` in sixty-four samples and holds
/// there.
///
/// Reaching full scale is the whole assertion, and it is worth stating what that separates: a core
/// treating a custom-GAIN byte as a *direct* value would set the envelope to `$1F << 4` and report
/// `$1F`, which is the mistake this shape of register invites.
fn e7_11() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            gain: 0xDF, // custom, linear increase, rate $1F (every sample)
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "a linear-increase GAIN did not reach full scale; $1F means the mode bits were ignored \
         and the byte was taken as a direct value",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.11",
        'E',
        "GAIN linear increase",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Rate 0 never fires, so a ramp configured with it does not move at all.
///
/// The rate table's first entry is not "as fast as possible" but "never": a rate of 0 disables the
/// step entirely. The same linear-increase GAIN as `E7.11` with rate 0 therefore leaves the
/// envelope where key-on put it, at zero.
///
/// It is the pair to `E7.11`, and needs to be: on its own, "the envelope did not move" is also what
/// a core with no GAIN ramps at all reports, and what a voice that never started reports. Only the
/// two together say that the ramp works *and* that rate 0 switches it off.
fn e7_01() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            gain: 0xC0, // custom, linear increase, rate 0 — which never fires
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the envelope moved although the GAIN rate was 0, which never fires",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.01",
        'E',
        "Rate 0 never fires",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// `FLG`'s reset bit keys every voice off and zeroes every envelope.
///
/// Setting bit 7 of `$6C` is what a driver does before it has configured anything, and it is not a
/// gentle stop: it behaves as `KOFF = $FF` with the envelopes forced to zero rather than released.
/// This voice is held at a direct gain of `$7F` that nothing else would move, so a reading of zero
/// afterwards is the reset and nothing else.
///
/// It is the same observation as `E7.08` reached by a different route, and the pair is worth having:
/// a core that implemented `FLG` reset as "stop the DSP" would leave the last envelope value
/// visible, which passes neither.
fn e9_18() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            // FLG: reset (bit 7) plus the echo-write disable the setup already uses.
            late: &[(0x6C, 0xA0)],
            late_settle: 4,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the envelope survived a FLG reset, so the reset bit did not force the voices off",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.18",
        'E',
        "FLG reset kills voices",
        Provenance::Documented("SNESdev Wiki, S-DSP; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `VxOUTX` is read before the per-voice volume is applied: silencing the voice does not silence it.
///
/// The register reports the sample after the envelope and before `VxVOLL`/`VxVOLR`, so a voice
/// turned all the way down still shows the same `OUTX` it showed at full volume. A core that reads
/// the register off the mixer's input instead returns zero, and a driver using `OUTX` to watch a
/// sound's progress loses it the moment the music fades that channel out.
///
/// The control is `E5.03`, which is this voice with the volume left at `$7F`, reading the same
/// band. Together they say the volume does not reach the register, which neither says alone.
fn e7_16() -> Test {
    let prog = voice_program(
        &constant_sample(0x8, 0x7),
        Voice {
            vol: 0x00, // both channels all the way down
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x01,
        0x7F,
        "VxOUTX read zero with the voice volume at zero — the register is sampled before the \
         volume, not after it",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.16",
        'E',
        "OUTX is pre-volume",
        Provenance::Documented("fullsnes, S-DSP envelopes; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// Two loud voices summed into one mix **saturate**; they do not wrap round to the other sign.
///
/// The DSP adds each voice's contribution into the mix one at a time and clamps after every
/// addition. Two voices each near the positive limit therefore come out at the limit, not at their
/// arithmetic sum reinterpreted as a negative number. A driver that mixes several loud voices
/// hears distortion on hardware; a core that wraps produces a loud sign-inverted crack instead,
/// which is far more audible and quite wrong.
///
/// # Reading the mix through the echo buffer
///
/// The mix itself is not readable, but `EON` routes voices into the echo buffer and the echo buffer
/// is ordinary APU RAM the program can read back. With `EVOL` and `EFB` at zero nothing is heard
/// and nothing is fed back, so what lands in the buffer is the summed voice output and nothing
/// else.
///
/// | phase | voices keyed on | buffer byte 1 |
/// |---|---|---|
/// | 1 | voice 0 | large and positive — the guard |
/// | 2 | voices 0 **and** 1 | still positive if the mix saturates; negative if it wraps |
///
/// # Choosing a sample loud enough to overflow and quiet enough to be legible
///
/// Every nibble is `+7` at shift 12, decoding to `$7000` — high enough that two of them exceed the
/// positive limit, low enough that one of them does not. One voice alone therefore has to read back
/// large and positive, and that is asserted first: if a single voice were already at the limit, or
/// already silent, phase 2 would say nothing about addition.
///
/// A constant sample also keeps gaussian interpolation out of it — all four taps carry the same
/// value, so the reading does not depend on which sample the cart caught. `E6.11` is what happens
/// when that is not arranged.
fn e9_15() -> Test {
    /// The page `ESA` names, well clear of the program image at `$0200`.
    const ECHO_PAGE: u8 = 0x30;
    /// Where that page starts. Derived, so the two cannot drift apart.
    const ECHO_ADDR: u16 = (ECHO_PAGE as u16) << 8;
    /// The sample directory, on the stack page well below the stack itself.
    const DIR_ADDR: u16 = (DIR_PAGE as u16) << 8;

    let mut prog = Spc::new();
    let addr = prog.data_first(IMAGE_BASE, &constant_sample(0xC, 0x7));
    prog.mov_x_imm(0xEF).mov_sp_x();
    let [lo, hi] = addr.to_le_bytes();
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 1);
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR + 2);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 3);

    dsp_write(&mut prog, 0x6C, 0x20); // FLG: echo writes off during setup
    dsp_write(&mut prog, 0x6D, ECHO_PAGE); // ESA
    dsp_write(&mut prog, 0x7D, 0x00); // EDL = 0: one four-byte entry at the buffer start
    dsp_write(&mut prog, 0x2C, 0x00); // EVOL L — the echo is measured, never heard
    dsp_write(&mut prog, 0x3C, 0x00); // EVOL R
    dsp_write(&mut prog, 0x0D, 0x00); // EFB — no feedback, so the buffer holds the mix alone
    dsp_write(&mut prog, 0x5D, DIR_PAGE); // DIR
    dsp_write(&mut prog, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut prog, 0x1C, 0x7F); // MVOLR
    dsp_write(&mut prog, 0x3D, 0x00); // NON
    dsp_write(&mut prog, 0x2D, 0x00); // PMON
    for v in [0x00u8, 0x10] {
        dsp_write(&mut prog, v, 0x7F); // VOL L
        dsp_write(&mut prog, v | 0x01, 0x00); // VOL R — one channel is enough to read
        dsp_write(&mut prog, v | 0x02, 0x00); // PITCH low
        dsp_write(&mut prog, v | 0x03, 0x10); // PITCH high: one sample per output sample
        dsp_write(&mut prog, v | 0x04, 0x00); // SRCN: both voices play the same block
        dsp_write(&mut prog, v | 0x06, 0x00); // ADSR2
        dsp_write(&mut prog, v | 0x07, 0x7F); // GAIN: direct, full scale
        dsp_write(&mut prog, v | 0x05, 0x00); // ADSR1: GAIN is in charge
    }
    dsp_write(&mut prog, 0x4D, 0x03); // EON: both voices feed the echo

    // --- phase 1: voice 0 alone, which has to be loud and positive for phase 2 to mean anything ---
    dsp_write(&mut prog, 0x4C, 0x01);
    prog.delay(0x00);
    dsp_write(&mut prog, 0x4C, 0x00);
    prog.delay(0x00);
    e9_15_sample_echo(&mut prog, ECHO_ADDR, PORT1);

    // --- phase 2: voice 1 as well, so two near-limit contributions are added ---
    dsp_write(&mut prog, 0x4C, 0x02);
    prog.delay(0x00);
    dsp_write(&mut prog, 0x4C, 0x00);
    prog.delay(0x00);
    e9_15_sample_echo(&mut prog, ECHO_ADDR, PORT2);
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(207, "E9.15 echo byte 1 with one voice (the guard)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(208, "E9.15 echo byte 1 with two voices at the same level");
    a.c("One voice has to be loud and positive but not already at the limit, or adding a second");
    a.c("says nothing about how the addition behaves.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x40,
        0x7E,
        "one voice alone did not read back as a large positive value below the limit — it was \
         silent, negative, or already saturated, and in every one of those cases the two-voice \
         reading below is uninterpretable",
    );
    a.c("And two of them must clamp at the limit rather than summing past it into the other sign.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x40,
        0x7F,
        "two voices summed to a negative value, so the per-voice mix is wrapping instead of \
         saturating after each addition — several loud voices at once would produce a \
         sign-inverted crack where hardware distorts",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.15",
        'E',
        "Voice mix saturates",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the per-voice mix clamps to 16 bits after each \
             addition rather than accumulating and clamping once",
        ),
        Kind::Scored,
        None,
    )
}

/// Emit: mark [`e9_15`]'s echo window, open echo writes briefly, and read byte 1 back to `port`.
///
/// The window is filled with `$FF` first so "nothing was written" is distinguishable from "a zero
/// was written" — the same reason `E9.05` marks it, and the reason a bare read of an untouched
/// buffer cannot serve as a measurement.
fn e9_15_sample_echo(prog: &mut Spc, echo_addr: u16, port: u8) {
    for i in 0..4u16 {
        prog.mov_a_imm(0xFF).mov_abs_a(echo_addr + i);
    }
    dsp_write(prog, 0x6C, 0x00); // FLG: echo writes on
    prog.delay(0x00);
    dsp_write(prog, 0x6C, 0x20); // and off again, so the read below is of a settled buffer
    prog.mov_a_abs(echo_addr + 1).mov_dp_a(port); // byte 1: the left channel's high byte
}

/// The echo buffer stores **four bytes per entry**, left channel first.
///
/// Each entry is `L` low, `L` high, `R` low, `R` high — two 16-bit samples, little-endian, in that
/// order. `E9.12` already pins what goes *in* each sample (the bottom bit is masked off); this pins
/// the layout around them, which is the part a driver reading the buffer back has to know.
///
/// # Making the two channels disagree is the whole design
///
/// With a voice at equal volume on both sides the four bytes are two identical pairs, and a core
/// writing two bytes per entry, or writing right-then-left, produces exactly the same buffer. So
/// the voice is set to full volume on the **left only**:
///
/// | bytes | channel | expected |
/// |---|---|---|
/// | 0-1 | `L` | written, and not the `$FF` marker |
/// | 2-3 | `R` | written **as zero** |
///
/// A core writing only two bytes per entry leaves bytes 2-3 holding the `$FF` marker. One writing
/// right-then-left puts the zeroes in bytes 0-1 and the signal in 2-3. Neither can produce the
/// expected pattern, and the two failures are distinct codes.
///
/// # The marker is `$FF`, and that is doing real work
///
/// The buffer is painted `$FF` before echo writes are enabled, so "the right channel is zero" and
/// "nothing was written to the right channel" are different readings rather than the same one. This
/// is the same trick `E9.12` uses for its bit-0 check, and the reason both tests can assert a zero
/// at all.
///
/// The left channel is checked first: it is the guard. If echo never wrote, byte 1 is still `$FF`
/// and the test says so, instead of reporting the right channel's untouched marker as a layout
/// error.
fn e9_05() -> Test {
    /// The page `ESA` names, well clear of the program image at `$0200`.
    const ECHO_PAGE: u8 = 0x30;
    /// Where that page starts. Derived, so the two cannot drift apart.
    const ECHO_ADDR: u16 = (ECHO_PAGE as u16) << 8;
    /// The sample directory, on the stack page well below the stack itself.
    const DIR_ADDR: u16 = (DIR_PAGE as u16) << 8;

    let sample = constant_sample(0x8, 0x7);
    let mut prog = Spc::new();
    let addr = prog.data_first(IMAGE_BASE, &sample);
    prog.mov_x_imm(0xEF).mov_sp_x();
    let [lo, hi] = addr.to_le_bytes();
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 1);
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR + 2);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 3);

    dsp_write(&mut prog, 0x6C, 0x20); // FLG: echo writes off during setup
    dsp_write(&mut prog, 0x6D, ECHO_PAGE); // ESA
    dsp_write(&mut prog, 0x7D, 0x00); // EDL = 0: one four-byte entry at the buffer start
    dsp_write(&mut prog, 0x2C, 0x00); // EVOL L — the echo is measured, not heard
    dsp_write(&mut prog, 0x3C, 0x00); // EVOL R
    dsp_write(&mut prog, 0x0D, 0x00); // EFB
    dsp_write(&mut prog, 0x5D, DIR_PAGE); // DIR
    dsp_write(&mut prog, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut prog, 0x1C, 0x7F); // MVOLR
    dsp_write(&mut prog, 0x3D, 0x00); // NON
    dsp_write(&mut prog, 0x2D, 0x00); // PMON
    dsp_write(&mut prog, 0x00, 0x7F); // voice 0 VOL L — full scale
    dsp_write(&mut prog, 0x01, 0x00); // voice 0 VOL R — silent, and that is the point
    dsp_write(&mut prog, 0x02, 0x00); // PITCH low
    dsp_write(&mut prog, 0x03, 0x10); // PITCH high: one sample per output sample
    dsp_write(&mut prog, 0x04, 0x00); // SRCN
    dsp_write(&mut prog, 0x06, 0x00); // ADSR2
    dsp_write(&mut prog, 0x07, 0x7F); // GAIN: direct, full scale
    dsp_write(&mut prog, 0x05, 0x00); // ADSR1: GAIN is in charge
    dsp_write(&mut prog, 0x4D, 0x01); // EON: voice 0 feeds the echo
    dsp_write(&mut prog, 0x4C, 0x01); // KON
    prog.delay(0x00);
    dsp_write(&mut prog, 0x4C, 0x00);
    prog.delay(0x00); // let the voice settle at its constant output

    for i in 0..4u16 {
        prog.mov_a_imm(0xFF).mov_abs_a(ECHO_ADDR + i);
    }
    dsp_write(&mut prog, 0x6C, 0x00); // FLG: echo writes on
    prog.delay(0x00);
    dsp_write(&mut prog, 0x6C, 0x20); // and off again, so the reads below are stable
    prog.mov_a_abs(ECHO_ADDR + 1).mov_dp_a(PORT1); // L high
    prog.mov_a_abs(ECHO_ADDR + 2).mov_dp_a(PORT2); // R low
    prog.mov_a_abs(ECHO_ADDR + 3).mov_dp_a(PORT3); // R high
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(
        170,
        "E9.05 echo buffer byte 1 (L high) — $FF means nothing was written",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(171, "E9.05 echo buffer byte 2 (R low)");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(172, "E9.05 echo buffer byte 3 (R high)");
    a.c("The left channel first: it is the guard. Still $FF means echo never wrote at all, and");
    a.c("the right channel's bytes below would then be reporting the marker, not a layout.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("cmp #$FF");
    a.fail_if_eq(
        "the echo buffer's second byte still held the $FF marker, so nothing was written and the \
         right-channel checks below would be about the paint rather than about the layout",
    );
    a.c("The right channel is silent, so its two bytes must be written zeroes.");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "echo buffer byte 2 is not the right channel's low byte: a core writing two bytes per \
         entry leaves the $FF marker here, and one writing right-then-left leaves the signal",
    );
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x00,
        "echo buffer byte 3 is not the right channel's high byte, so the entry is not four bytes \
         of left-then-right",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.05",
        'E',
        "Echo entry is 4 bytes",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the echo buffer holds four bytes per sample, a 16-bit \
             left sample followed by a 16-bit right one",
        ),
        Kind::Scored,
        None,
    )
}

/// Emit [`e9_13`]'s DSP configuration: a left-only voice, echo feedback on, and one FIR tap.
///
/// Every FIR coefficient is written explicitly rather than assumed. The DSP is shared between
/// programs, so a previous test's taps would become part of this test's feedback loop — which
/// with `EFB` on is the difference between a filter and an oscillator.
fn e9_13_configure(prog: &mut Spc, echo_page: u8) {
    dsp_write(prog, 0x6C, 0x20); // FLG: echo writes off during setup
    dsp_write(prog, 0x6D, echo_page); // ESA
    dsp_write(prog, 0x7D, 0x00); // EDL = 0: one four-byte entry
    dsp_write(prog, 0x2C, 0x00); // EVOL L — measured, not heard
    dsp_write(prog, 0x3C, 0x00); // EVOL R
    dsp_write(prog, 0x0D, 0x40); // EFB: feedback on, so the buffer reaches the FIR
    // Every tap explicitly: a previous program's coefficients would be part of this feedback loop.
    dsp_write(prog, 0x0F, 0x40); // FIR0
    for reg in [0x1F, 0x2F, 0x3F, 0x4F, 0x5F, 0x6F, 0x7F] {
        dsp_write(prog, reg, 0x00); // FIR1-FIR7
    }
    dsp_write(prog, 0x5D, DIR_PAGE); // DIR
    dsp_write(prog, 0x0C, 0x7F); // MVOLL
    dsp_write(prog, 0x1C, 0x7F); // MVOLR
    dsp_write(prog, 0x3D, 0x00); // NON
    dsp_write(prog, 0x2D, 0x00); // PMON
    dsp_write(prog, 0x00, 0x7F); // voice 0 VOL L — full scale
    dsp_write(prog, 0x01, 0x00); // voice 0 VOL R — silent
    dsp_write(prog, 0x02, 0x00); // PITCH low
    dsp_write(prog, 0x03, 0x10); // PITCH high
    dsp_write(prog, 0x04, 0x00); // SRCN
    dsp_write(prog, 0x06, 0x00); // ADSR2
    dsp_write(prog, 0x07, 0x7F); // GAIN: direct, full scale
    dsp_write(prog, 0x05, 0x00); // ADSR1
    dsp_write(prog, 0x4D, 0x01); // EON: voice 0 feeds the echo
}

/// The echo FIR filters the two channels independently — nothing crosses between them.
///
/// Each channel has its own eight-sample history and its own accumulator; the `FIRx` coefficients
/// are shared, but the samples they multiply are not. A core keeping one history, or summing both
/// channels into one accumulator, turns every echo into mono the moment feedback is switched on —
/// audible, and invisible to any test that drives both channels equally.
///
/// # Feedback is what makes this about the FIR
///
/// `E9.05` already shows that an all-left voice writes zeroes into the right half of the buffer, so
/// the *input* mix is separate. That says nothing about the filter: with `EFB = 0` the buffer's
/// contents never reach the FIR at all.
///
/// So this test enables feedback and a single non-zero tap, and lets the echo run. The buffer's
/// right-channel bytes are now the output of the right FIR, fed from the right history, which has
/// only ever seen zeroes. If any of that is shared with the left, the right half stops being zero.
///
/// | register | value | why |
/// |---|---|---|
/// | `VOL L` / `VOL R` | `$7F` / `$00` | the signal exists only on the left |
/// | `EFB` | `$40` | feedback on, so the buffer reaches the FIR |
/// | `FIR0` | `$40` | one tap, at half scale — enough to propagate, short of blowing up |
/// | `FIR1`-`FIR7` | `$00` | written explicitly; a previous test's taps would be feedback too |
///
/// The taps are set explicitly rather than assumed. The DSP is shared between programs, and a
/// leftover tap set would make the feedback path something this test did not choose — which with
/// `EFB` on is the difference between a filter and an oscillator.
///
/// # The right channel is *near* silence, not exactly zero
///
/// This was first written asserting a flat zero, and all three cores returned `$FFFE` — minus two.
/// Three implementations agreeing is the signature of a wrong expectation rather than a shared
/// defect, and the magnitudes say the same thing: the left channel sits around `$0900` and the
/// right is nine bits below it. That is arithmetic residue in the echo path, not a channel leaking
/// into another.
///
/// So the assertion is about **orders of magnitude**: the left's high byte must be substantial, and
/// the right's must be `$00` or `$FF` — within one step of silence either side. Crosstalk from a
/// left channel this large would land in the same magnitude as the left, not nine bits under it.
///
/// # The guard
///
/// The left half is checked first and must not still hold the `$FF` marker. If echo never wrote,
/// the right half would be `$FF` too and would pass the magnitude check for a reason that has
/// nothing to do with crosstalk; the guard turns that into its own failure code.
fn e9_13() -> Test {
    /// The page `ESA` names, well clear of the program image at `$0200`.
    const ECHO_PAGE: u8 = 0x30;
    /// Where that page starts. Derived, so the two cannot drift apart.
    const ECHO_ADDR: u16 = (ECHO_PAGE as u16) << 8;
    /// The sample directory, on the stack page well below the stack itself.
    const DIR_ADDR: u16 = (DIR_PAGE as u16) << 8;

    let sample = constant_sample(0x8, 0x7);
    let mut prog = Spc::new();
    let addr = prog.data_first(IMAGE_BASE, &sample);
    prog.mov_x_imm(0xEF).mov_sp_x();
    let [lo, hi] = addr.to_le_bytes();
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 1);
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR + 2);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 3);

    e9_13_configure(&mut prog, ECHO_PAGE);
    dsp_write(&mut prog, 0x4C, 0x01); // KON
    prog.delay(0x00);
    dsp_write(&mut prog, 0x4C, 0x00);
    prog.delay(0x00);

    for i in 0..4u16 {
        prog.mov_a_imm(0xFF).mov_abs_a(ECHO_ADDR + i);
    }
    dsp_write(&mut prog, 0x6C, 0x00); // FLG: echo writes on
    prog.delay(0x00);
    prog.delay(0x00); // long enough for feedback to go round several times
    dsp_write(&mut prog, 0x6C, 0x20); // and off again, so the reads below are stable
    prog.mov_a_abs(ECHO_ADDR + 1).mov_dp_a(PORT1); // L high
    prog.mov_a_abs(ECHO_ADDR + 2).mov_dp_a(PORT2); // R low
    prog.mov_a_abs(ECHO_ADDR + 3).mov_dp_a(PORT3); // R high
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(
        173,
        "E9.13 echo byte 1 (L high) with feedback on — $FF means nothing was written",
    );
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(174, "E9.13 echo byte 2 (R low)");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(
        175,
        "E9.13 echo byte 3 (R high) — $00 or $FF is near-silence",
    );
    a.c("The guard: if echo never wrote, the right half is still $FF and would fail below for a");
    a.c("reason that has nothing to do with crosstalk.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("cmp #$FF");
    a.fail_if_eq(
        "the echo buffer's left half still held the $FF marker, so nothing was written and the \
         right-channel checks below would be about the paint rather than about the filter",
    );
    a.c("The left channel has to be substantial, or 'the right is small' is trivially true.");
    a.l("lda f:$7E0100");
    a.l("cmp #$01");
    a.l("bcs :+");
    a.l("cmp #$00");
    a.fail_if_ne(
        "the left echo channel's high byte is zero, so the signal this test is checking for \
         leakage of is itself too small for the comparison below to mean anything",
    );
    a.l(":");
    a.c("And the right must stay within one high-byte step of silence: crosstalk from a left");
    a.c("channel this large would land in the same magnitude, not nine bits below it.");
    a.l("lda f:$7E0102");
    a.l("cmp #$FF");
    a.l("beq :+");
    a.l("cmp #$00");
    a.fail_if_ne(
        "the right echo channel's magnitude reached the left's, although only the left was ever \
         fed and feedback is on — the FIR is sharing history or accumulator between the channels",
    );
    a.l(":");
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.13",
        'E',
        "L/R FIR independent",
        Provenance::Documented(
            "fullsnes and anomie's DSP doc: the echo FIR keeps a separate eight-sample history and \
             accumulator per channel; only the coefficients are shared",
        ),
        Kind::Scored,
        None,
    )
}

/// The echo buffer's samples are stored with their bottom bit masked off.
///
/// The DSP writes each echo sample as a 16-bit value masked with `$FFFE`, so the low byte's bit 0 is
/// always zero no matter what the mixer produced. A core that stores the sample verbatim leaves
/// odd values in the buffer, and a driver that reads the buffer back — some do, to fade an echo
/// tail out by hand — sees numbers the hardware cannot produce.
///
/// **The marker is `$FF`, which makes one assertion do two jobs.** Bit 0 of `$FF` is set, so a zero
/// there afterwards proves both that a write happened *and* that what was written is even. Painting
/// with an even marker would have left "nothing was written" and "an even value was written"
/// indistinguishable.
///
/// A voice is playing into the echo (`EON` bit 0) so the value written is not trivially zero; what
/// it is exactly depends on the whole mixer chain, which is why the test asks about one bit rather
/// than about the number.
fn e9_12() -> Test {
    /// The page `ESA` names, well clear of the program image at `$0200`.
    const ECHO_PAGE: u8 = 0x30;
    /// Where that page starts. Derived, so the two cannot drift apart.
    const ECHO_ADDR: u16 = (ECHO_PAGE as u16) << 8;
    /// The sample directory, on the stack page well below the stack itself.
    const DIR_ADDR: u16 = (DIR_PAGE as u16) << 8;

    let sample = constant_sample(0x8, 0x7);
    let mut prog = Spc::new();
    let addr = prog.data_first(IMAGE_BASE, &sample);
    prog.mov_x_imm(0xEF).mov_sp_x();
    let [lo, hi] = addr.to_le_bytes();
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 1);
    prog.mov_a_imm(lo).mov_abs_a(DIR_ADDR + 2);
    prog.mov_a_imm(hi).mov_abs_a(DIR_ADDR + 3);

    dsp_write(&mut prog, 0x6C, 0x20); // FLG: echo writes off while everything is set up
    dsp_write(&mut prog, 0x6D, ECHO_PAGE); // ESA
    dsp_write(&mut prog, 0x7D, 0x00); // EDL = 0: four bytes at the buffer start (`E9.06`)
    dsp_write(&mut prog, 0x2C, 0x00); // EVOL L — the echo is measured, not heard
    dsp_write(&mut prog, 0x3C, 0x00); // EVOL R
    dsp_write(&mut prog, 0x0D, 0x00); // EFB
    dsp_write(&mut prog, 0x5D, DIR_PAGE); // DIR
    dsp_write(&mut prog, 0x0C, 0x7F); // MVOLL
    dsp_write(&mut prog, 0x1C, 0x7F); // MVOLR
    dsp_write(&mut prog, 0x3D, 0x00); // NON
    dsp_write(&mut prog, 0x2D, 0x00); // PMON
    dsp_write(&mut prog, 0x00, 0x7F); // voice 0 VOL L
    dsp_write(&mut prog, 0x01, 0x7F); // voice 0 VOL R
    dsp_write(&mut prog, 0x02, 0x00); // PITCH low
    dsp_write(&mut prog, 0x03, 0x10); // PITCH high: one sample per output sample
    dsp_write(&mut prog, 0x04, 0x00); // SRCN
    dsp_write(&mut prog, 0x06, 0x00); // ADSR2
    dsp_write(&mut prog, 0x07, 0x7F); // GAIN: direct, full scale
    dsp_write(&mut prog, 0x05, 0x00); // ADSR1: GAIN is in charge
    dsp_write(&mut prog, 0x4D, 0x01); // EON: voice 0 feeds the echo
    dsp_write(&mut prog, 0x4C, 0x01); // KON
    prog.delay(0x00);
    dsp_write(&mut prog, 0x4C, 0x00);
    prog.delay(0x00); // let the voice settle at its constant output

    for i in 0..4u16 {
        prog.mov_a_imm(0xFF).mov_abs_a(ECHO_ADDR + i);
    }
    dsp_write(&mut prog, 0x6C, 0x00); // FLG: echo writes on
    prog.delay(0x00); // 256 iterations, not none — see `Spc::delay`
    dsp_write(&mut prog, 0x6C, 0x20); // and off again, so the read below is stable
    prog.mov_a_abs(ECHO_ADDR).mov_dp_a(PORT1);
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("The marker was $FF. A clear bit 0 means the DSP wrote here AND masked the value even.");
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.l("and #$01");
    a.assert_a8(
        0x00,
        "the echo buffer's low byte kept its bottom bit — either nothing was written over the \
         $FF marker, or the sample was stored without the & $FFFE mask",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.12",
        'E',
        "Echo writes are masked",
        Provenance::Documented("fullsnes, S-DSP echo; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// `EDL = 0` is not "no buffer": it is a four-byte one, rewritten in place every sample.
///
/// The natural reading of a length of zero is that echo is off, or that the buffer is empty. It is
/// neither — the DSP writes one sample's worth, four bytes, at the buffer's start, and does it
/// again next sample. A core that treats zero as "skip the write" leaves the buffer alone; one that
/// treats it as a full-size buffer walks off across whatever follows `ESA` in APU RAM, which on a
/// real driver is its own code.
///
/// The test paints eight bytes and reads two of them back: byte 0 must have been overwritten, byte
/// 4 must not. That pair is what separates "wrote four bytes" from both wrong answers — a core
/// that skipped the write fails on byte 0, and one that wrote a longer buffer fails on byte 4.
///
/// The written value is zero and asserted as such rather than as "something else": no voice is
/// keyed on and both echo volumes are zero, so what the mixer produces is exactly zero.
fn e9_06() -> Test {
    /// The page `ESA` names, well clear of the program image at `$0200`.
    const ECHO_PAGE: u8 = 0x30;
    /// Where that page starts. Derived, so the two cannot drift apart.
    const ECHO_ADDR: u16 = (ECHO_PAGE as u16) << 8;

    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    // Writes off BEFORE anything else. The DSP is shared with every earlier test, and painting a
    // marker while it is still writing would leave the buffer in a state neither reading can be
    // trusted against -- in particular a full-size-buffer core could have its pointer past byte 4
    // already, and byte 4 would survive for the wrong reason.
    dsp_write(&mut prog, 0x6C, 0x20); // FLG: echo writes disabled
    dsp_write(&mut prog, 0x6D, ECHO_PAGE); // ESA
    dsp_write(&mut prog, 0x7D, 0x00); // EDL = 0
    dsp_write(&mut prog, 0x4D, 0x00); // EON
    dsp_write(&mut prog, 0x2C, 0x00); // EVOL L
    dsp_write(&mut prog, 0x3C, 0x00); // EVOL R
    dsp_write(&mut prog, 0x0D, 0x00); // EFB
    // ESA and EDL do not take effect instantly (`E9.07`, `E9.08`); give them a moment before the
    // buffer is painted, so the writes measured below are aimed where this test put the marker.
    prog.delay(0x00);
    for i in 0..8u16 {
        prog.mov_a_imm(0x5A).mov_abs_a(ECHO_ADDR + i);
    }
    dsp_write(&mut prog, 0x6C, 0x00); // FLG: echo writes enabled
    prog.delay(0x00); // 256 iterations: many samples' worth of writes
    dsp_write(&mut prog, 0x6C, 0x20); // and disabled again, so the reads below are stable
    prog.mov_a_abs(ECHO_ADDR).mov_dp_a(PORT1);
    prog.mov_a_abs(ECHO_ADDR + 4).mov_dp_a(PORT2);
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x00,
        "the first four bytes of the echo buffer were not written with EDL = 0, so a length of \
         zero was taken to mean no buffer at all",
    );
    a.c("And byte 4 is past the four the DSP writes, so it still holds the marker.");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x5A,
        "byte 4 of the echo buffer was overwritten with EDL = 0 — the buffer is one sample long, \
         and a core writing further walks over whatever follows ESA",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.06",
        'E',
        "EDL 0 is a 4-byte buffer",
        Provenance::Documented("fullsnes, S-DSP echo — flagged as errata; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// `FLG` bit 5 stops the DSP *writing* the echo buffer; nothing else about echo changes.
///
/// The bit is usually described as "echo disable", and it is not: the DSP goes on reading the
/// buffer and feeding it through the FIR, it simply stops writing anything back. A driver that
/// clears the buffer once and sets the bit gets silence; one that sets the bit over a buffer full
/// of noise gets that noise forever, because the same samples circulate unchanged. A core that
/// treats the bit as "echo off" produces silence in both cases and sounds fine until a game does
/// the second thing.
///
/// The test asks the memory rather than the ear. It paints a marker over the buffer's first bytes,
/// waits, and reads them back through APU RAM — twice, once with the bit set and once clear:
///
/// * with writes **disabled**, the marker survives;
/// * with writes **enabled**, it is gone, replaced by the zero the mixer is producing (no voice is
///   keyed on and both echo volumes are zero, so what gets written is deterministic).
///
/// `EDL = 0` is the smallest buffer — four bytes, continuously overwritten at the buffer's start
/// (`E9.06`) — which is what makes a short wait enough and puts the write exactly where the marker
/// is.
fn e9_10() -> Test {
    /// The page `ESA` names, well clear of the program image at `$0200`.
    const ECHO_PAGE: u8 = 0x30;
    /// Where that page starts. Derived, so the two cannot drift apart.
    const ECHO_ADDR: u16 = (ECHO_PAGE as u16) << 8;

    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF).mov_sp_x();
    // Echo pointed at a page well clear of the program image, with the smallest buffer.
    dsp_write(&mut prog, 0x6D, ECHO_PAGE); // ESA
    dsp_write(&mut prog, 0x7D, 0x00); // EDL: four bytes, rewritten every sample
    dsp_write(&mut prog, 0x4D, 0x00); // EON: no voice feeds the echo
    dsp_write(&mut prog, 0x2C, 0x00); // EVOL L
    dsp_write(&mut prog, 0x3C, 0x00); // EVOL R
    dsp_write(&mut prog, 0x0D, 0x00); // EFB

    // Phase 1: writes disabled. The marker must survive.
    dsp_write(&mut prog, 0x6C, 0x20); // FLG: echo writes disabled, no reset, no mute
    for i in 0..4u16 {
        prog.mov_a_imm(0x5A).mov_abs_a(ECHO_ADDR + i);
    }
    prog.delay(0x00); // 256 iterations, not none — see `Spc::delay`
    prog.mov_a_abs(ECHO_ADDR).mov_dp_a(PORT1);

    // Phase 2: writes enabled. The marker must be gone.
    for i in 0..4u16 {
        prog.mov_a_imm(0x5A).mov_abs_a(ECHO_ADDR + i);
    }
    dsp_write(&mut prog, 0x6C, 0x00); // FLG: echo writes enabled
    prog.delay(0x00); // 256 iterations: long enough for many echo writes to land
    prog.mov_a_abs(ECHO_ADDR).mov_dp_a(PORT2);

    dsp_write(&mut prog, 0x6C, 0x20); // put the write-disable back before handing over
    prog.mov_a_imm(DONE).mov_dp_a(PORT0).release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x5A,
        "the echo buffer was written with FLG bit 5 set — the bit disables echo WRITES, and a \
         driver that parks a buffer under it expects to find it intact",
    );
    a.c("And with the bit clear the DSP writes what the mixer is producing, which with no voice");
    a.c("keyed on and both echo volumes at zero is zero.");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "the echo buffer still held the marker with FLG bit 5 clear, so the DSP is not writing it \
         at all",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.10",
        'E',
        "FLG.5 stops echo writes",
        Provenance::Documented("fullsnes, S-DSP echo; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// `KOFF` holds a voice off continuously; a `KON` written while it is set does not restart the voice.
///
/// The two registers are not symmetric. `KON` is a write-triggered edge — it starts a voice once and
/// the bit does not linger — while `KOFF` is a level the DSP consults every time it looks. So a
/// driver that sets `KOFF` and then writes `KON` without clearing it first gets silence, which is a
/// real and confusing way to lose a note.
///
/// Its two controls are already in the battery: `E7.10` is the same voice with no `KOFF` at all,
/// reading `$7F`, and `E7.08` is `KOFF` alone, reading `$00`. This is `KOFF` *and* `KON`, and it
/// must read `$00` — if `KON` were the level and `KOFF` the edge, it would read `$7F`.
fn e8_04() -> Test {
    let prog = voice_program(
        &looping_sample(),
        Voice {
            // Both writes back to back, with nothing between them: KOF first, then a KON that must
            // not take. The settle afterwards is long enough for release to reach zero.
            late: &[(0x5C, 0x01), (0x4C, 0x01)],
            late_settle: 12,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "a KON written while KOFF was still set restarted the voice — KOFF is a level the DSP \
         consults continuously, not an edge that KON can override",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E8.04",
        'E',
        "KOFF outranks KON",
        Provenance::Documented("fullsnes, S-DSP key on/off; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// `FLG`'s mute bit silences the mixer, not the voice: `VxOUTX` is upstream of it and keeps moving.
///
/// Mute is applied where the voices are summed, so everything before that point carries on — the
/// envelope steps, the sample decodes, and `VxOUTX`, which reports the post-envelope pre-volume
/// sample, still reads what it read before. A core that implements mute by zeroing the voices makes
/// `VxOUTX` go quiet too, and a driver watching it to decide when a sound effect has finished waits
/// forever.
///
/// The sample is the constant one the BRR tests use, so the reading is stationary and the assertion
/// is about mute rather than about which sample the cart caught. `E5.03` is the same voice without
/// mute, reading the same band.
fn e9_17() -> Test {
    let prog = voice_program(
        &constant_sample(0x8, 0x7),
        Voice {
            late: &[(0x6C, 0x60)], // FLG: mute, echo writes still disabled, no reset
            late_settle: 4,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0x01,
        0x7F,
        "VxOUTX went quiet when FLG's mute bit was set — mute belongs to the mixer, and OUTX is \
         upstream of it",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.17",
        'E',
        "Mute is after OUTX",
        Provenance::Documented("fullsnes, S-DSP FLG; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// A noise voice still decodes its BRR sample, so an end-without-loop block silences it.
///
/// `NON` replaces a voice's *sample* with the noise generator, and it is easy to assume that leaves
/// the sample pointer unused. It does not: the voice keeps decoding BRR underneath, which means the
/// end-and-mute flag still reaches it and still forces the envelope to zero. A driver that parks a
/// noise voice on whatever sample address happens to be there gets silence at an unpredictable
/// moment, and a core that skips decoding for noise voices never reproduces it.
///
/// Its control is `E5.07`: the identical sample and the identical read with `NON` clear. Without
/// that pair, "the envelope is zero" would also be what a core that simply cannot play a noise
/// voice reports.
fn e9_04() -> Test {
    let sample = brr_sample(&[brr_block(0x8, 0, 0b01, 0x7, 0x9)], 0);
    let prog = voice_program(
        &sample,
        Voice {
            non: 0x01,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "a noise voice's envelope survived an end-without-loop block, so noise voices are not \
         decoding BRR underneath",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E9.04",
        'E',
        "Noise voices decode BRR",
        Provenance::Documented("fullsnes, S-DSP noise; anomie's DSP doc — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// A direct GAIN value *is* the envelope: `ENVX` reads back the byte that was written.
///
/// With `ADSR1` bit 7 clear the ADSR generator is off and `VxGAIN` governs the envelope; with
/// `VxGAIN` bit 7 also clear the mode is direct, and the envelope is set to `G << 4` rather than
/// ramped toward it. `VxENVX` reports `E >> 4`, so the two shifts cancel and a direct gain of `$7F`
/// reads back as exactly `$7F` — an exact number, on a register that is otherwise only ever
/// checked for being "about right".
///
/// The voice is playing a looping sample throughout, so nothing else has cause to move the
/// envelope. That matters: the same read on a voice that had finished would report `$00` for a
/// reason this test is not about (see `E5.07`).
fn e7_10() -> Test {
    // Code 3 — end and loop — so the voice repeats this block forever and never runs out.
    let sample = brr_sample(&[brr_block(0x8, 0, 0b11, 0x7, 0x9)], 0);
    let prog = voice_program(&sample, Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x7F,
        "ENVX did not read back the direct GAIN value; a ramp toward it, or a missing >>4, both \
         land somewhere else",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E7.10",
        'E',
        "Direct GAIN is envelope",
        Provenance::Documented("SNESdev Wiki, S-DSP envelopes; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `ENDX` sets when a block carrying the end flag is decoded.
///
/// The voice plays two blocks, the second of which is code 3 — end and loop — so the sample
/// repeats and the only thing that can have set `ENDX` is the end flag itself. A core that never
/// implemented the register, or that only sets it when a voice *stops*, reports nothing here; a
/// driver waiting on `ENDX` to swap a sample would wait forever.
fn e5_09() -> Test {
    let sample = brr_sample(
        &[
            brr_block(0x8, 0, 0b00, 0x7, 0x9),
            brr_block(0x8, 0, 0b11, 0x9, 0x7),
        ],
        0,
    );
    let prog = voice_program(&sample, Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Bit 0 is voice 0. Masked rather than compared whole: the other seven voices were never");
    a.c("keyed on, but nothing in this test says what a core leaves in their bits.");
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$0001");
    a.assert_a16_range(
        1,
        1,
        "ENDX bit 0 never set although the voice decoded a block with the end flag",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.09",
        'E',
        "ENDX sets on end block",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// The loop flag alone means nothing: code 2 behaves exactly as code 0.
///
/// Both header bits are read as a pair, and only the end bit stops anything. A block with the loop
/// bit set and the end bit clear is an ordinary block — the loop address is consulted when a block
/// *ends*, and this one does not. A core that treats the loop bit as "this is the last block"
/// sets `ENDX` here, and would then also jump back to the loop point in the middle of a sample.
///
/// Without an end flag the voice keeps decoding forward, so this is the one voice test that has to
/// bound where it gets to: it plays at a sixteenth of the sample rate and pads the sample with six
/// blocks of silence, which is minutes of settle time away rather than the two delay loops it
/// actually waits.
///
/// It is the pair to `E5.09`, which sets `ENDX` from an otherwise identical program with one header
/// bit different. Without that pairing this assertion would also pass on a voice that never
/// started, since "did not set a bit" is what silence looks like too.
fn e5_08() -> Test {
    let sample = brr_sample(
        &[
            brr_block(0x8, 0, 0b10, 0x7, 0x9),
            brr_block(0x8, 0, 0b10, 0x9, 0x7),
        ],
        6,
    );
    let prog = voice_program(
        &sample,
        Voice {
            pitch_hi: 0x01,
            settle: 2,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$0001");
    a.assert_a16_range(
        0,
        0,
        "ENDX bit 0 set although no block carried the end flag, so the loop bit was read as one",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.08",
        'E',
        "Loop flag without end",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// End without loop silences the voice immediately, whatever the envelope was doing.
///
/// Code 1 — end set, loop clear — puts the voice into release with an envelope of zero the moment
/// the block finishes, rather than releasing it at the configured rate. The envelope here is a
/// direct GAIN of `$7F`, which nothing about the envelope generator would ever move on its own, so
/// a reading of `$00` afterwards can only have come from the end-and-mute path.
///
/// This is the pair to `E7.10`: identical setup, identical read, one header bit different, and the
/// answers are opposite. Neither test alone separates "the envelope works" from "the envelope is
/// stuck at whatever was written".
fn e5_07() -> Test {
    let sample = brr_sample(&[brr_block(0x8, 0, 0b01, 0x7, 0x9)], 0);
    let prog = voice_program(&sample, Voice::direct_gain());

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x00,
        "ENVX was not zero after an end-without-loop block, so end+mute did not force release",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.07",
        'E',
        "End+mute zeroes env",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// A sample directory entry is at `DIR * $100 + SRCN * 4`.
///
/// The same sample as `E5.09`, reached through entry **1** instead of entry 0 — and entry 0 is
/// pointed at address `$0000`, whose zero header decodes as silence that never ends. So a core
/// that folds `SRCN` in with the wrong stride, or ignores it, plays silence and reports nothing;
/// only the documented address arrives at a sample with an end flag in it.
///
/// The decoy matters more than it looks. With entry 0 simply left unwritten, "wrong entry" would
/// mean "whatever APU RAM happened to hold", which is neither silence nor a sample reliably.
fn e5_11() -> Test {
    let sample = brr_sample(
        &[
            brr_block(0x8, 0, 0b00, 0x7, 0x9),
            brr_block(0x8, 0, 0b11, 0x9, 0x7),
        ],
        0,
    );
    let prog = voice_program(
        &sample,
        Voice {
            srcn: 1,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$0001");
    a.assert_a16_range(
        1,
        1,
        "ENDX never set for SRCN 1, so the directory entry was not read from DIR*$100 + SRCN*4",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E5.11",
        'E',
        "Directory entry address",
        Provenance::Documented("fullsnes, S-DSP BRR; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// `MOV dp,dp` is exempt from the store dummy-read that every other store performs.
///
/// `E2.01` establishes the rule: a store reads its destination first, and against a timer counter —
/// where reading is destructive — that read is visible as the counter being emptied. `$FA` is one
/// of the two opcodes the rule does not apply to, so the same store through it leaves the counter
/// alone.
///
/// The two tests are the same measurement with one instruction changed, which is what makes this an
/// assertion about `$FA` rather than about timers. A core that applies the dummy read uniformly
/// passes `E2.01` and fails here; one that omits it everywhere does the reverse.
///
/// Timer 1 is run alongside and reported as the vacuity guard. Without it, "timer 0 still holds a
/// count" and "the timers never started" are the same reading — and the second would make the
/// assertion pass for the wrong reason.
fn e2_02() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0x10, 0x77) // the byte to store; its value is irrelevant to a read-only target
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1
        .mov_dp_imm(0xFB, 0x01) // T1DIV = 1
        .mov_dp_imm(0xF1, 0x83) // enable timers 0 and 1; bit 7 keeps the IPL ROM mapped
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80) // stop both, so nothing ticks between the reads below
        .mov_dp_dp(0xFD, 0x10) // MOV $FD,$10 — a store with no dummy read
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT1) // must still hold its count
        .mov_a_dp(0xFE)
        .mov_dp_a(PORT2) // the guard: the timers did run
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Timer 1 first: it says the timers ran at all, which is what makes the reading below mean");
    a.c("anything.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "timer 1's counter was empty, so the timers never ran and the check below would pass on a \
         counter that had nothing in it",
    );
    a.c("And timer 0 survived a store to it, because MOV dp,dp does not dummy-read its target.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "timer 0's counter was consumed by MOV dp,dp — that opcode is exempt from the store \
         dummy-read, and a core applying the rule uniformly clears it",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.02",
        'E',
        "MOV dp,dp is exempt",
        Provenance::Documented("SNESdev Wiki, SPC700; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `MOVW dp,YA` dummy-reads its low byte only, not both.
///
/// Stores on this processor read their destination before writing it (`E2.01`), and a sixteen-bit
/// store might reasonably do that twice. It does not: only the low address is read. Pointed at the
/// timer counters — where a read is destructive — that difference is directly visible, because
/// `$FD` comes back empty and `$FE`, one address higher and written by the very same instruction,
/// still holds its count.
///
/// Both timers are stopped before the instruction runs. The counters are four bits and the reads
/// are a handful of cycles apart, so a tick landing in between would answer a question this test is
/// not asking — the same hazard `E3.01` was rebuilt around.
///
/// The `$FE` reading is asserted as a range rather than "not zero": it doubles as the vacuity
/// guard, since a timer that never advanced would leave both counters empty and make the `$FD`
/// assertion meaningless.
fn e2_03() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1
        .mov_dp_imm(0xFB, 0x01) // T1DIV = 1
        .mov_dp_imm(0xF1, 0x83) // enable timers 0 and 1; bit 7 keeps the IPL ROM mapped
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80) // stop both before anything reads them
        .mov_y_imm(0x00)
        .mov_a_imm(0x00)
        .movw_dp_ya(0xFD) // the write goes nowhere; the dummy read is the point
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT1) // consumed by MOVW's dummy read
        .mov_a_dp(0xFE)
        .mov_dp_a(PORT2) // untouched: MOVW never read this one
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Timer 1's counter is the vacuity guard as well as the assertion: if it were empty the");
    a.c("check below would pass on a pair of timers that never ran.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "timer 1's counter was empty, so either the timers never ran or MOVW consumed $FE as well",
    );
    a.c("And timer 0's is empty, because MOVW's dummy read reached $FD and only $FD.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        0,
        "timer 0's counter survived MOVW dp,YA — the instruction dummy-reads its low address",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.03",
        'E',
        "MOVW reads the low byte",
        Provenance::Documented("SNESdev Wiki, SPC700; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

/// `PSW.P` moves the direct page to `$01xx`, and it moves it for *every* direct-page access.
///
/// The bit is easy to implement for the obvious loads and stores and forget for the rest — the
/// read-modify-writes, the bit operations, the pointer fetches behind `[aa]+Y`. A driver that sets
/// `P` to keep its variables clear of the zero page then finds half its accesses going to the wrong
/// place, and the failure looks like memory corruption rather than an addressing bug.
///
/// Two kinds of access are checked, because one proves less than it looks: a `MOV` store, and an
/// `INC dp` — a read-modify-write, which reads through `P`, modifies, and writes back through it.
/// A core that resolves the page once at decode and reuses it passes both; one that resolves it
/// separately for the read and the write can fail the second while passing the first. The `[aa]+Y`
/// pointer fetch the dossier also names is **not** covered here.
///
/// The two pages are seeded with different values first, so "it went to `$0120`" and "it went to
/// `$0020`" are distinguishable answers rather than one answer and one absence. `$0120` is far
/// below the stack, which lives at the top of the same page.
fn e2_06() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .clrp()
        .mov_dp_imm(0x20, 0x11) // $0020, the page-0 copy
        .setp()
        .mov_dp_imm(0x20, 0x5A) // $0120, if P is honoured
        .clrp()
        .mov_a_abs(0x0120)
        .mov_dp_a(PORT1)
        .mov_a_dp(0x20) // page 0 again
        .mov_dp_a(PORT2)
        // A read-modify-write through P: it must read $0120, increment, and write $0120 back.
        .setp()
        .inc_dp(0x20)
        .clrp()
        .mov_a_abs(0x0120)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0100");
    a.assert_a8(
        0x5A,
        "a direct-page store with P set did not reach $0120, so the bit is not selecting the page",
    );
    a.c("And the page-0 copy is untouched, which is the half that catches a core writing BOTH.");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x11,
        "the page-0 byte changed as well, so the store went to $0020 rather than to $0120",
    );
    a.c("And a read-modify-write through P: INC must read $0120 and write it back, not page 0.");
    a.l("lda f:$7E0102");
    a.assert_a8(
        0x5B,
        "INC dp with P set did not increment $0120 — a read-modify-write resolves the page for \
         both halves, and $5A means the write went elsewhere",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.06",
        'E',
        "PSW.P selects the page",
        Provenance::Documented("SNESdev Wiki, SPC700 addressing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// Direct-page indexing wraps inside the page.
///
/// `MOV A,$FF+X` with `X = 2` reads direct-page `$01`, not `$0101`. The index is added to the
/// 8-bit offset and the result stays in the page the `P` flag selects — it does not carry into the
/// page above. A core that computes the address as a 16-bit sum reads a byte from the wrong page
/// entirely, which is silent until something lives there.
///
/// `$0101` — where a 16-bit sum *would* land — is poisoned with a third value rather than left to
/// whatever APU RAM holds. Otherwise the test asserts only that the wrong page did not happen to
/// contain the expected marker, which is a weaker claim that quietly depends on power-on state.
fn e2_05() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0x01, 0x5A) // where a wrapped index must land
        .mov_dp_imm(0xFF, 0x99) // and where the un-indexed offset points
        .mov_a_imm(0x33)
        .mov_abs_a(0x0101) // and where a 16-bit sum would land, poisoned so it cannot match
        .mov_x_imm(0x02)
        .mov_a_dp_x(0xFF) // $FF + 2 -> $01 if it wraps, $0101 if it does not
        .mov_dp_a(PORT2)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("sep #$20");
    a.l("lda f:$7E0101");
    a.assert_a8(
        0x5A,
        "$FF + X did not wrap within the direct page; a 16-bit sum would read $0101 instead",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.05",
        'E',
        "DP index wraps in page",
        Provenance::Documented("SNESdev Wiki, SPC700 addressing; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A store to a timer counter clears it, because stores dummy-read their destination.
///
/// `MOV $FD,A` writes nothing useful — the counter is read-only — but the instruction reads its
/// destination first, and reading a timer counter *consumes* it. So a store to `$FD` clears
/// Timer 0 as surely as a load does, which is a trap for any driver that "initialises" the
/// counters by writing them.
///
/// Both readings are asserted directly rather than against each other. The first version of this
/// test asked only that the post-store reading be *smaller* than a control reading taken over the
/// same delay, and that version failed on one reference emulator while passing here — not because
/// either core was wrong, but because a core that does not clear leaves an arbitrary value in the
/// counter, and an arbitrary value lands inside a difference range often enough to decide the
/// test by luck. Requiring the control to have advanced and the post-store reading to be empty is
/// the stronger claim and the stable one.
fn e2_01() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1
        .mov_dp_imm(0xF1, 0x81) // enable timer 0, and KEEP the IPL ROM mapped (bit 7)
        // --- control: delay, then read. The counter must have advanced. ---
        .delay(0x00)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2)
        // --- the store: delay again, store to $FD, then read. The store's dummy read cleared it. ---
        .delay(0x00)
        .mov_a_imm(0x00)
        .mov_dp_a(0xFD) // MOV $FD,A — a store, whose dummy read consumes the counter
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Control first: without a store in the way, the counter advanced.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        2,
        15,
        "timer 0 did not advance over the control delay, so the check below is vacuous — it would \
         pass on a counter that was empty the whole time",
    );
    a.c("And immediately after the store the counter is essentially empty. Asserted directly");
    a.c("rather than as a difference: a core that does NOT clear leaves an arbitrary value there,");
    a.c("and an arbitrary value lands inside a difference range often enough to pass by luck.");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        1,
        "the counter was not empty immediately after a store to $FD, so the store's dummy read \
         did not consume it",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.01",
        'E',
        "Store dummy-reads target",
        Provenance::Documented("SNESdev Wiki, SPC700; fullsnes — flagged as errata"),
        Kind::Scored,
        None,
    )
}

// ---------------------------------------------------------------------------------------------
// E6 — pitch and the sample counter
// ---------------------------------------------------------------------------------------------

/// A 24-block sample, 384 samples long, ending without a loop so `ENDX` marks the moment it runs
/// out.
///
/// The length is the measurement. `E6.02` and its two siblings all play *this* sample and differ
/// only in the pitch and in how long they wait, so what any one of them reports is a statement
/// about where the pointer had reached — and 384 is chosen so the three waits fall either side of
/// the two finishing times with room to spare.
fn pitch_ramp_sample() -> Vec<u8> {
    let mut blocks: Vec<Vec<u8>> = (0..23).map(|_| brr_block(0x8, 0, 0b00, 0x7, 0x7)).collect();
    blocks.push(brr_block(0x8, 0, 0b01, 0x7, 0x7)); // END without LOOP: the voice stops here
    brr_sample(&blocks, 0)
}

/// Play `pitch_hi` for `settle` delay loops and report `ENDX`.
///
/// The three tests below are one experiment with two knobs, so they share the emitter rather than
/// restating it: a difference between them should be a difference the test is about.
fn pitch_rate_test(
    id: &'static str,
    name: &'static str,
    pitch_hi: u8,
    settle: u8,
    want_end: bool,
    why: &'static str,
) -> Test {
    let prog = voice_program(
        &pitch_ramp_sample(),
        Voice {
            pitch_hi,
            settle,
            ..Voice::direct_gain()
        },
    );

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100     ; ENDX as the program read it");
    a.l("and #$0001        ; voice 0");
    a.assert_a16(u16::from(want_end), why);
    apu_timeout_arm(&mut a);
    a.finish(
        id,
        'E',
        name,
        Provenance::Documented("fullsnes, S-DSP pitch; anomie's DSP doc"),
        Kind::Scored,
        None,
    )
}

/// At `$1000` the voice consumes **one sample per output sample**, so 384 samples are not gone in
/// 256 output samples' time.
///
/// This is the upper half of a bracket, and it is worth being explicit that no single reading of
/// `ENDX` can establish a rate — it says only "finished" or "not finished", which bounds the rate
/// on one side. Three readings of the same 384-sample voice bound it on both:
///
/// | | pitch | waits | `ENDX` | what it establishes |
/// |---|---|---|---|---|
/// | `E6.02` | `$1000` | 6 | clear | fewer than 64 samples per wait |
/// | `E6.02b` | `$1000` | 16 | set | at least 24 — so it is playing, and not arbitrarily slowly |
/// | `E6.02c` | `$2000` | 6 | set | at least 64: strictly faster than `$1000`, same wait |
/// | `E6.02d` | `$2000` | 3 | clear | fewer than 128 — the upper bound that makes it a window |
///
/// A wait is one `settle` loop of the shared `voice_program` (256 iterations of `DBNZ Y`), plus
/// the key-on delay, which is why the counts above are one more than the `settle` values passed in.
/// The sample is 384 long, so "finished after *n* waits" means at least `384/n` samples per wait.
///
/// **What this does not establish is the exact factor**, and the honest statement of the result is
/// the two windows: `$1000` consumes 24-64 samples per wait and `$2000` consumes 64-128. Both
/// windows contain the documented values (48 and 96), and a core that ignores the pitch register
/// entirely cannot satisfy both — but a core scaling by 1.5 rather than 2 also fits. Excluding that
/// needs each rate bracketed between *adjacent* waits, which is where the bisection above actually
/// puts them, but shipping it would mean four assertions with roughly a tenth of the elapsed time
/// in hand. That is the timing-marginal construction this group has been bitten by before, so it
/// is deliberately not shipped; see `docs/accuracysnes-plan.md`.
///
/// **The four waits were found by bisection, then moved away from what it found.** Bisecting on
/// this cart puts the `$1000` voice's finish between the seventh and eighth wait and the `$2000`
/// voice's between the fourth and fifth. A test placed *at* those boundaries would carry the
/// tightest window and the thinnest margin; each of the four above is a wait or more clear of the
/// nearest boundary instead, which is why the windows they state are wider than the bisection knows
/// them to be. That is the trade, taken deliberately: the first attempt here placed the wait by
/// arithmetic and the voice had already finished.
///
/// The measurement is deterministic rather than racy — every cycle of it happens inside the
/// uploaded SPC program, so neither the cart's own code size nor the host's speed can move it. A
/// core modelling a different number of SPC cycles per output sample would move it, but that is
/// `E10.01` and a different assertion, and the margin here is wide enough to absorb the
/// discrepancies the three cross-validated references actually have.
fn e6_02() -> Test {
    pitch_rate_test(
        "E6.02",
        "Pitch $1000 is 1:1",
        0x10,
        5,
        false,
        "a 384-sample voice at pitch $1000 had already finished after six waits, so it is \
         consuming at least 64 samples per wait — a third above 1:1",
    )
}

/// The lower half of `E6.02`'s bracket: given twice as long, the same voice **has** finished.
///
/// Without this a core running the voice arbitrarily slowly — or not at all — passes `E6.02` by
/// doing nothing, which is the failure mode a "still going" assertion always has. Read the table
/// in `E6.02`; neither test means anything alone.
fn e6_02b() -> Test {
    pitch_rate_test(
        "E6.02b",
        "Pitch $1000 does finish",
        0x10,
        15,
        true,
        "a 384-sample voice at pitch $1000 had still not finished after sixteen waits, so it is \
         consuming fewer than 24 samples per wait — half of 1:1 — or not playing at all",
    )
}

/// `$2000` plays faster: the same voice, the same wait, and the opposite verdict.
///
/// One bit changed in the pitch register and nothing else, which is what makes this an assertion
/// about pitch scaling rather than about the sample or the timer. Read with `E6.02` it establishes
/// that `$2000` consumes at least 64 samples per wait where `$1000` consumes fewer than 64 — a
/// strict increase, and the direction the octave predicts. It does not by itself pin the factor at
/// two; `E6.02`'s table says what the four together do and do not establish.
fn e6_02c() -> Test {
    pitch_rate_test(
        "E6.02c",
        "Pitch $2000 is +1 octave",
        0x20,
        5,
        true,
        "a 384-sample voice at pitch $2000 had not finished after six waits, so it is consuming \
         fewer than 64 samples per wait — no faster than $1000 manages in the same time",
    )
}

/// The upper bound on `$2000`, without which `E6.02c` is only half a measurement.
///
/// `E6.02c` says the voice consumes at least 64 samples per wait; a core running it ten times too
/// fast satisfies that just as well as one running it at the documented rate. Three waits are not
/// enough for a 384-sample voice to finish at anything up to 128 samples per wait, so this closes
/// the window from above and turns "at least" into "between".
fn e6_02d() -> Test {
    pitch_rate_test(
        "E6.02d",
        "Pitch $2000 upper bound",
        0x20,
        2,
        false,
        "a 384-sample voice at pitch $2000 had already finished after three waits, so it is \
         consuming at least 128 samples per wait — far above what doubling $1000 would give",
    )
}

/// Timer 2 counts **eight times faster** than timer 0 at the same divider.
///
/// The two timers are fed from different taps of the same clock: `T0` and `T1` from an 8 kHz stage,
/// `T2` from a 64 kHz one, so `TnDIV` means eight times as much wall time on `T0` as on `T2`. A
/// core that runs all three timers off one rate is the obvious mistake, and it is invisible to
/// every other timer test on this cart — `E3.01`, `E3.05` and `E2.01` all use `T0` alone, and a
/// uniform-rate core passes all of them.
///
/// Both timers run over the *same* interval, started by one write and stopped by another, so this
/// is a ratio rather than two independent measurements: whatever the interval actually was, `T2`
/// must show about eight times what `T0` does. The interval is chosen short enough that `T2`'s
/// four-bit counter cannot wrap — a wrap would read as a *small* number and look like a slow timer,
/// which is the one failure this test could not tell from a pass.
///
/// The assertion is a pair of ranges, not two exact counts. Where the interval falls relative to
/// each timer's internal divider phase decides whether the last tick lands inside it, so ±1 is not
/// a defect; a factor of eight is far outside that. A uniform-rate core reads `$01` where this
/// wants nine or more.
fn e3_06() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1
        .mov_dp_imm(0xFC, 0x01) // T2DIV = 1
        .mov_a_dp(0xFD) // drain both counters so the interval starts from zero
        .mov_a_dp(0xFF)
        .mov_dp_imm(0xF1, 0x85) // enable timers 0 and 2 together; bit 7 keeps the IPL mapped
        .delay(0x18) // 24 iterations: long enough for T2 to count, short enough not to wrap
        .mov_dp_imm(0xF1, 0x80) // and stop them together
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2)
        .mov_a_dp(0xFF)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Timer 0 first: one tick, maybe two. Zero would make the ratio below unmeasurable.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        3,
        "timer 0 did not tick once over this interval, or ticked more than three times — either \
         way the interval is not the one this test needs and the ratio below means nothing",
    );
    a.c("Timer 2 over the SAME interval: eight times the rate, so eight or more ticks.");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        8,
        15,
        "timer 2 did not count roughly eight times what timer 0 did over the same interval, so it \
         is not running from the 64 kHz stage — a core reading $01 here runs every timer at 8 kHz",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.06",
        'E',
        "T2 is eight times T0",
        Provenance::Documented("SNESdev Wiki, SPC700 timers; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// `TEST` bit 0 halts the timers, and clearing it lets them run again.
///
/// `$F0` is a hardware test register, so a core is likely to model it as ordinary storage — and
/// then a ROM that writes it behaves differently for reasons nothing in the trace explains. This
/// is the same argument `E3.10` makes for bit 1 and the RAM write enable, one bit over.
///
/// The test is one interval run twice with nothing changed but that bit, which is what makes it an
/// assertion about the bit rather than about the delay: **halted** first, then **running**. Taking
/// the halted reading alone would be satisfied by a timer that never started, and a core modelling
/// `$F0` as RAM passes the second half and fails the first.
///
/// `TEST` is restored to its reset value of `$0A` afterwards. Leaving a hardware test register
/// disturbed would make every later APU test measure this one — the shared-state failure the group
/// has already been bitten by.
fn e3_08() -> Test {
    let mut prog = Spc::new();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFA, 0x01) // T0DIV = 1, the fastest
        .mov_dp_imm(0xF0, 0x0B) // TEST: reset value $0A plus bit 0 — timers halted
        .mov_dp_imm(0xF1, 0x81) // enable timer 0 anyway
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT2) // must be zero: the timer was enabled but halted
        .mov_dp_imm(0xF0, 0x0A) // TEST back to its reset value, timers free to run
        .mov_dp_imm(0xF1, 0x81)
        .delay(0x00)
        .mov_dp_imm(0xF1, 0x80)
        .mov_a_dp(0xFD)
        .mov_dp_a(PORT3) // and now the same delay does advance it
        .mov_a_imm(DONE)
        .mov_dp_a(PORT0)
        .release_to_ipl();

    let mut a = Asm::new();
    upload_and_run(&mut a, &prog);
    a.c("Halted: enabled, at the fastest divider, over a delay that is several ticks long.");
    a.l("rep #$30");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16(
        0,
        "timer 0 advanced while TEST bit 0 was set, so the halt bit is being modelled as ordinary \
         storage rather than as a control",
    );
    a.c("Running: the control, and without it the reading above would pass on a dead timer.");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.assert_a16_range(
        1,
        15,
        "timer 0 did not advance with TEST back at its reset value, so the halted reading above \
         says nothing about the halt bit",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E3.08",
        'E',
        "TEST bit 0 halts timers",
        Provenance::Documented("fullsnes, SPC700 TEST register; ares and bsnes smp/timing"),
        Kind::Scored,
        None,
    )
}
