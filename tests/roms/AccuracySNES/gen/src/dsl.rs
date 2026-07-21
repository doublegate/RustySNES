//! The test-definition DSL.
//!
//! Every AccuracySNES test is authored **once**, here, as a Rust value. From that single
//! definition the generator emits both the 65816 assembly that runs on-cart and the
//! `SOURCE_CATALOG.tsv` row the host harness scores against — so the two can never drift
//! (the repo's *Golden-Vector Parity* pattern applied to a test ROM).
//!
//! # The on-cart contract
//!
//! The runtime calls each test as a subroutine with a canonical entry state:
//!
//! - native mode (`E = 0`), `A`/`X`/`Y` all 16-bit (`rep #$30`)
//! - `DP = $0000`, `DBR = $00`, stack in bank `$00`
//! - `SAVED_S` already holds the pre-call stack pointer
//!
//! A test exits by storing its verdict byte to [`TEST_RESULT`] and jumping to `test_restore`,
//! which re-establishes that canonical state (including the stack) before the runner continues.
//! A test therefore may freely corrupt `S`, `DP`, `DBR`, and the `E`/`M`/`X` flags — the restore
//! path fixes all of them. This is what makes the emulation-mode and stack-wrap groups safe to
//! write.
//!
//! # Verdict encoding
//!
//! One byte per test, matching AccuracyCoin's convention in spirit (see
//! `docs/accuracysnes-research-dossier.md` §1.4):
//!
//! | Byte | Meaning |
//! |---|---|
//! | `$00` | not run |
//! | `$01` | PASS |
//! | `(n << 1) \| 1` | PASS, variant `n` — which legal hardware behaviour was observed |
//! | `n << 1` (non-zero) | FAIL, error code `n` identifying the sub-assertion |
//! | `$FF` | skipped (auto-gated by revision/region, or user-marked) |

use core::fmt::Write as _;

/// WRAM address the running test stores its verdict byte to (long-addressed, so it works in
/// emulation mode and under any `DBR`).
pub const TEST_RESULT: &str = "$7EE010";

/// Base of the raw measurement channel in WRAM — 64 u16 slots, read by the host harness.
///
/// Shared with `asm/runtime.inc` and `tests/accuracysnes.rs`, which must agree on it byte for byte.
pub const MEAS_BASE: u32 = 0x7E_E200;

/// Number of `u16` slots in the measurement channel.
pub const MEAS_SLOTS: u8 = 128;

/// Verdict byte meaning "the test passed with no variant".
pub const PASS: u8 = 0x01;

/// How well-established the behaviour a test asserts actually is.
///
/// Only [`Provenance::Documented`] and [`Provenance::Corroborated`] may back the pass-rate
/// number; the other two are recorded but never scored. This is the anti-circularity gate — a
/// test we wrote, grading an emulator we wrote, proves nothing unless the expected value came
/// from somewhere else. Enforced by the harness, mirroring `docs/adr/0003`'s honesty gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "Corroborated/Novel are part of the tier contract and are used by later groups; \
              Group A happens to be entirely Documented/Contested"
)]
pub enum Provenance {
    /// A primary reference states the behaviour outright (SNESdev wiki/Errata, anomie, fullsnes,
    /// the WDC datasheet). The payload cites which.
    Documented(&'static str),
    /// Independent implementations agree in source. The payload cites where.
    ///
    /// **ares and bsnes count as ONE reference, not two.** A full diff of their `wdc65816`
    /// cores shows only type renames (`uint8`→`n8`, `uint16(x)`→`n16(x)`); ares' 65816 is a
    /// lineal descendant of bsnes'. Mesen2 is the genuinely independent second opinion, so
    /// this tier means "the bsnes/ares lineage and Mesen2 agree" — two implementations, not
    /// three. Verified 2026-07-19; see `docs/accuracysnes-research-dossier.md`.
    Corroborated(&'static str),
    /// References disagree, or one admits the behaviour is unexplained. Never scored.
    Contested(&'static str),
    /// Our own hypothesis with no external backing. Never scored until promoted.
    Novel(&'static str),
}

impl Provenance {
    /// The catalog tier name, as written to `SOURCE_CATALOG.tsv`.
    #[must_use]
    pub const fn tier(self) -> &'static str {
        match self {
            Self::Documented(_) => "Documented",
            Self::Corroborated(_) => "Corroborated",
            Self::Contested(_) => "Contested",
            Self::Novel(_) => "Novel",
        }
    }

    /// The citation backing this tier.
    #[must_use]
    pub const fn cite(self) -> &'static str {
        match self {
            Self::Documented(c) | Self::Corroborated(c) | Self::Contested(c) | Self::Novel(c) => c,
        }
    }

    /// Whether a test with this provenance may contribute to the pass rate.
    #[must_use]
    pub const fn scores(self) -> bool {
        matches!(self, Self::Documented(_) | Self::Corroborated(_))
    }
}

/// What kind of answer a test produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A normal pass/fail assertion.
    Scored,
    /// The behaviour is genuinely undefined on hardware, so the test records what it observed
    /// and is never counted as pass or fail. `$4203`/`$4206` overlap, decimal-mode `V`, the WRAM
    /// power-on fill, and post-reset `ENDX` are all in this class.
    Golden,
}

/// One test: an on-cart subroutine plus everything the catalog and README need to describe it.
#[derive(Debug, Clone)]
pub struct Test {
    /// Dossier ID, e.g. `"A3.02"`.
    pub id: &'static str,
    /// Group letter, e.g. `'A'`.
    pub group: char,
    /// Human-readable name shown in the on-cart menu (kept short — the menu is 26 columns).
    pub name: &'static str,
    /// Where the expected behaviour came from.
    pub provenance: Provenance,
    /// Scored or golden-vector.
    pub kind: Kind,
    /// The generated assembly body (label + code + exit stubs).
    pub body: String,
    /// Read-only bytes this test needs somewhere other than bank $00, emitted verbatim into the
    /// `APUDATA` segment. Empty for almost every test: it exists for the SPC700 program images,
    /// which are large, are pure data, and were pushing bank $00 over its limit.
    pub data: String,
    /// `(code, description)` for every failure code this test can emit, for the README.
    pub codes: Vec<(u8, String)>,
}

impl Test {
    /// The assembly label for this test's entry point.
    #[must_use]
    pub fn label(&self) -> String {
        format!("test_{}", self.id.to_lowercase().replace('.', "_"))
    }
}

/// Builds one test's assembly body, allocating failure codes as assertions are added.
///
/// Codes start at 1 and increment per assertion, matching AccuracyCoin's "hex error code
/// identifying the exact sub-assertion" convention — a bare FAIL is useless to an emulator
/// author, so every assertion gets its own identifier.
pub struct Asm {
    lines: Vec<String>,
    data: Vec<String>,
    next_code: u8,
    codes: Vec<(u8, String)>,
}

impl Default for Asm {
    fn default() -> Self {
        Self::new()
    }
}

impl Asm {
    /// A fresh, empty body.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            lines: Vec::new(),
            data: Vec::new(),
            next_code: 1,
            codes: Vec::new(),
        }
    }

    /// Emit a line into the out-of-bank `APUDATA` segment rather than into the test body.
    ///
    /// For read-only blobs that the test refers to by address instead of executing in place. Bank
    /// $00 holds the runtime, the font, every test body and the catalog, and it is finite; a
    /// several-hundred-byte SPC700 image per test is the one thing here big enough to matter.
    pub fn d(&mut self, line: &str) -> &mut Self {
        self.data.push(line.to_string());
        self
    }

    /// Emit a raw assembly line (indented one level), tracking accumulator/index width.
    ///
    /// ca65's `.a8`/`.a16`/`.i8`/`.i16` state is **file-global** and survives `.proc` and
    /// `.segment` boundaries, and the dangerous direction is silent: if the assembler believes
    /// `A` is 16-bit while the CPU has it 8-bit, `lda #$12` emits three bytes and the CPU
    /// executes the stray `$00` as `BRK`. `.smart` only helps for straight-line code, and these
    /// tests are full of branches. So every width-changing instruction emits its matching
    /// directive right here, at the point of use.
    pub fn l(&mut self, line: &str) -> &mut Self {
        self.lines.push(format!("    {line}"));
        let op = line
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let (a, i) = match op.as_str() {
            "rep #$30" | "rep #$38" => (Some(true), Some(true)),
            "rep #$20" => (Some(true), None),
            "rep #$10" => (None, Some(true)),
            "sep #$30" | "sep #$38" => (Some(false), Some(false)),
            "sep #$20" => (Some(false), None),
            "sep #$10" => (None, Some(false)),
            _ => (None, None),
        };
        if let Some(wide) = a {
            self.lines.push(if wide {
                "    .a16".into()
            } else {
                "    .a8".into()
            });
        }
        if let Some(wide) = i {
            self.lines.push(if wide {
                "    .i16".into()
            } else {
                "    .i8".into()
            });
        }
        self
    }

    /// Repeat a block of instructions `reps` times, unrolled.
    ///
    /// Cycle tests amplify a one-cycle difference into something the dot counter can actually
    /// resolve: one CPU cycle is 6 master clocks = 1.5 dots, below the counter's resolution, but
    /// sixteen of them is 24 dots and unambiguous.
    pub fn repeat(&mut self, reps: u32, body: &[&str]) -> &mut Self {
        for _ in 0..reps {
            for line in body {
                self.l(line);
            }
        }
        self
    }

    /// Begin a cycle measurement: sync to the start of a scanline and latch the H counter.
    ///
    /// Deliberately scanline-local. Line length is not something a portable test can assume —
    /// NTSC has a short line at V=240, PAL a long one, and emulators disagree on whether the H
    /// counter tops out at 339 or 340 — so a measurement that crossed a line boundary would
    /// depend on exactly the convention under dispute.
    /// Begin a cycle measurement.
    ///
    /// **The measured span must stay under one scanline.** `measure_end` differences the H counter,
    /// which wraps at 341 dots, so a longer span silently returns a small number instead of
    /// failing — a wrapped reading is indistinguishable from a real one. `A5.08` once measured a
    /// 341-dot span, read ~0, and appeared to prove the emulator wrong about `REP`. Keep repeat
    /// counts modest, and record raw values with [`Asm::record`] so a suspicious result can be
    /// checked rather than inferred.
    pub fn measure_begin(&mut self) -> &mut Self {
        self.l("jsr hv_begin");
        self
    }

    /// End a cycle measurement, leaving the elapsed dot count in a 16-bit `A`.
    ///
    /// The `jsr`/latch overhead is identical at both ends of every measurement, so it cancels
    /// when two measurements are compared — which is the only way these tests use the number.
    /// The helper preserves the processor status, so the caller's width is untouched; read the
    /// result with [`Asm::measure_result`], which sets the width explicitly.
    pub fn measure_end(&mut self) -> &mut Self {
        self.l("jsr hv_end");
        self
    }

    /// Load the elapsed dot count of the last measurement into a 16-bit `A`.
    pub fn measure_result(&mut self) -> &mut Self {
        self.l("rep #$30");
        self.l("lda f:$7E0048     ; V_H1 = elapsed dots");
        self
    }

    /// Switch to emulation mode. Hardware forces `m = x = 1`, so both widths become 8-bit and
    /// the assembler must be told.
    pub fn enter_emulation(&mut self) -> &mut Self {
        self.lines.push("    sec".into());
        self.lines
            .push("    xce               ; -> emulation".into());
        self.lines.push("    .a8".into());
        self.lines.push("    .i8".into());
        self
    }

    /// Switch back to native mode.
    ///
    /// Leaving emulation does **not** widen anything: `m` and `x` were forced to 1 and stay 1,
    /// so `A`/`X`/`Y` remain 8-bit until an explicit `rep`. Getting this wrong is the single
    /// easiest way to desync the assembler from the CPU in these tests.
    pub fn enter_native(&mut self) -> &mut Self {
        self.lines.push("    clc".into());
        self.lines
            .push("    xce               ; -> native (m/x stay 1: still 8-bit)".into());
        self.lines.push("    .a8".into());
        self.lines.push("    .i8".into());
        self
    }

    /// Emit a comment line.
    pub fn c(&mut self, text: &str) -> &mut Self {
        self.lines.push(format!("    ; {text}"));
        self
    }

    /// Emit a local label (cheap-local, scoped to the enclosing proc).
    pub fn label(&mut self, name: &str) -> &mut Self {
        self.lines.push(format!("@{name}:"));
        self
    }

    /// Allocate the next failure code, recording `why` for the README.
    fn alloc(&mut self, why: &str) -> u8 {
        let code = self.next_code;
        assert!(code < 0x80, "more than 127 sub-assertions in one test");
        self.next_code += 1;
        self.codes.push((code, why.to_string()));
        code
    }

    /// Branch to a failure exit when the previous comparison was not equal.
    ///
    /// Uses `beq :+ / jmp` rather than a bare `bne` so the failure stub is reachable from
    /// anywhere in the test regardless of the ±127-byte branch range.
    /// Fail unless the last comparison was equal.
    ///
    /// Public because the `assert_*` family compares against a **constant**, and some assertions
    /// are between two values only known at run time — `D2.07` compares a DMA destination against
    /// the ROM bytes the DMA read, so that it pins the transfer rather than the image layout.
    /// Prefer the `assert_*` helpers whenever the expected value is a constant; they allocate the
    /// failure code and read better.
    pub fn fail_if_ne(&mut self, why: &str) -> &mut Self {
        let code = self.alloc(why);
        self.lines.push("    beq :+".into());
        self.lines.push(format!("    jmp @fail{code}"));
        self.lines.push("  :".into());
        self
    }

    /// Record the 16-bit accumulator into measurement slot `slot`, leaving `A` untouched.
    ///
    /// The verdict byte a test reports cannot carry a measurement — a dot count does not fit in a
    /// variant code, and a value that wraps past 256 is indistinguishable from a real one. A
    /// 32-`NOP` baseline reported that way read back as "21 dots", which is below the physical
    /// floor. Measurements therefore go to the full-width channel at [`MEAS_BASE`] and are read by
    /// the host harness.
    ///
    /// **Requires `A` 16-bit**; leaves it 16-bit and unchanged.
    ///
    /// # Panics
    /// If `slot` is outside the block.
    pub fn record(&mut self, slot: u8, why: &str) -> &mut Self {
        assert!(
            slot < MEAS_SLOTS,
            "measurement slot {slot} is outside the block"
        );
        let addr = MEAS_BASE + u32::from(slot) * 2;
        self.c(&format!("record slot {slot}: {why}"));
        self.l(&format!("sta f:${addr:06X}"))
    }

    /// Stand the test down as SKIP, unconditionally, from wherever control has reached.
    ///
    /// This is how a region-dependent assertion behaves on the console it does not apply to. It
    /// matters that the mechanism is SKIP rather than PASS: a test that quietly passes on the
    /// machine it cannot test is indistinguishable from one that verified something, and the pass
    /// rate would then include assertions nothing checked.
    ///
    /// The predicate must be derived from something already **measured**, never from a register
    /// whose meaning is itself under test — the region bit's position is contested (`B2.10`), so
    /// skipping on it would make a frame-height test depend on the very thing it is evidence for.
    ///
    /// Call from 16-bit `A`/`X` context: control does not return, but the assembler's width
    /// belief is restored afterwards so the fall-through path (the code that runs when the skip
    /// condition was false) still assembles as 16-bit. Leaving `.a8` in force here silently turns
    /// the next `cmp #$0105` into an 8-bit compare — which ca65 catches as a range error only
    /// because the constant happens to exceed 255.
    pub fn skip(&mut self, why: &str) -> &mut Self {
        self.c(&format!("SKIP: {why}"));
        self.l("sep #$20");
        self.l("lda #VERDICT_SKIP");
        self.l("sta f:V_TEST_RESULT");
        self.l("jml test_restore");
        self.lines
            .push("    ; unreachable — restores the assembler's width belief only".into());
        self.lines.push("    .a16".into());
        self.lines.push("    .i16".into());
        self
    }

    /// Assert the 16-bit accumulator equals `val`. Requires `A` currently 16-bit.
    pub fn assert_a16(&mut self, val: u16, why: &str) -> &mut Self {
        self.l(&format!("cmp #${val:04X}"));
        self.fail_if_ne(why)
    }

    /// Assert the 8-bit accumulator equals `val`. Requires `A` currently 8-bit.
    pub fn assert_a8(&mut self, val: u8, why: &str) -> &mut Self {
        self.l(&format!("cmp #${val:02X}"));
        self.fail_if_ne(why)
    }

    /// Assert the 16-bit `X` equals `val`.
    pub fn assert_x16(&mut self, val: u16, why: &str) -> &mut Self {
        self.l(&format!("cpx #${val:04X}"));
        self.fail_if_ne(why)
    }

    /// Assert a 16-bit `A` lies in `lo..=hi`.
    ///
    /// Cycle measurements carry a few dots of phase jitter: the CPU's 6- and 8-clock cycles do
    /// not divide evenly into the PPU's 4-clock dot, and `hv_begin` releases anywhere in a
    /// 16-dot window, so the same sequence can land a handful of dots apart between runs.
    /// Timing assertions therefore bound a range rather than demanding equality — wide enough to
    /// absorb the jitter, far too narrow to confuse "the penalty was applied" with "it was not".
    pub fn assert_a16_range(&mut self, lo: u16, hi: u16, why: &str) -> &mut Self {
        let code = self.alloc(why);
        self.l(&format!("cmp #${lo:04X}"));
        self.lines.push("    bcs :+".into());
        self.lines.push(format!("    jmp @fail{code}"));
        self.lines.push("  :".into());
        self.l(&format!("cmp #${:04X}", hi.wrapping_add(1)));
        self.lines.push("    bcc :+".into());
        self.lines.push(format!("    jmp @fail{code}"));
        self.lines.push("  :".into());
        self
    }

    /// Assert a 16-bit `A`, read as signed, has magnitude at most `limit`.
    ///
    /// Used by the "these two must cost the same" tests, where the measured difference can land
    /// either side of zero.
    pub fn assert_abs_le(&mut self, limit: u16, why: &str) -> &mut Self {
        let code = self.alloc(why);
        self.l("cmp #$8000");
        self.lines.push("    bcc :+".into());
        self.l("eor #$FFFF");
        self.l("inc a             ; negate: take the magnitude");
        self.lines.push("  :".into());
        self.l(&format!("cmp #${:04X}", limit + 1));
        self.lines.push("    bcc :+".into());
        self.lines.push(format!("    jmp @fail{code}"));
        self.lines.push("  :".into());
        self
    }

    /// Assert an 8-bit byte at a 24-bit address equals `val`. Leaves `A` 8-bit.
    ///
    /// Long-addressed, so it works irrespective of `DBR` and in emulation mode — which matters
    /// for the interrupt tests, whose handlers report their findings through WRAM.
    pub fn assert_mem8(&mut self, addr: u32, val: u8, why: &str) -> &mut Self {
        self.l("sep #$20");
        self.l(&format!("lda f:${addr:06X}"));
        self.l(&format!("cmp #${val:02X}"));
        self.fail_if_ne(why)
    }

    /// Assert the 16-bit `Y` equals `val`.
    pub fn assert_y16(&mut self, val: u16, why: &str) -> &mut Self {
        self.l(&format!("cpy #${val:04X}"));
        self.fail_if_ne(why)
    }

    /// Finish the test, emitting the pass stub and one failure stub per allocated code.
    ///
    /// `variant` is `None` for a plain pass, or `Some(n)` to report which legal hardware
    /// behaviour was observed.
    #[must_use]
    pub fn finish(
        mut self,
        id: &'static str,
        group: char,
        name: &'static str,
        provenance: Provenance,
        kind: Kind,
        variant: Option<u8>,
    ) -> Test {
        let pass_byte = variant.map_or(PASS, |v| (v << 1) | 1);
        let mut body = String::new();
        let label = format!("test_{}", id.to_lowercase().replace('.', "_"));

        let _ = writeln!(body, "; {id} — {name}");
        let _ = writeln!(
            body,
            "; provenance: {} ({})",
            provenance.tier(),
            provenance.cite()
        );
        let _ = writeln!(body, ".proc {label}");
        // The runtime guarantees this entry state; restate it so the assembler's file-global
        // width tracking is correct no matter what the previous test left behind.
        let _ = writeln!(body, "    .a16");
        let _ = writeln!(body, "    .i16");
        for line in &self.lines {
            let _ = writeln!(body, "{line}");
        }
        // Pass stub. `sep #$20` + `.a8` makes the stub self-correcting regardless of the width
        // in force wherever control jumped from.
        //
        // Skipped when the body already ends in an unconditional `jml test_restore` — which is how
        // every golden vector exits, having written its own variant code. Emitting it there would
        // be unreachable bytes in the ROM and, worse, would read as a second exit path that does
        // not exist.
        let body_exits = self
            .lines
            .last()
            .is_some_and(|l| l.trim() == "jml test_restore");
        if !body_exits {
            let _ = writeln!(body, "    sep #$20");
            let _ = writeln!(body, "    .a8");
            let _ = writeln!(body, "    lda #${pass_byte:02X}");
            let _ = writeln!(body, "    sta f:{TEST_RESULT}");
            let _ = writeln!(body, "    jml test_restore");
        }
        // Failure stubs, one per allocated code.
        for (code, why) in &self.codes {
            let byte = code << 1;
            let _ = writeln!(body, "@fail{code}:");
            let _ = writeln!(body, "    ; {why}");
            let _ = writeln!(body, "    sep #$20");
            let _ = writeln!(body, "    .a8");
            let _ = writeln!(body, "    lda #${byte:02X}");
            let _ = writeln!(body, "    sta f:{TEST_RESULT}");
            let _ = writeln!(body, "    jml test_restore");
        }
        let _ = writeln!(body, ".endproc");

        let codes = core::mem::take(&mut self.codes);
        let mut data = String::new();
        for line in &self.data {
            let _ = writeln!(data, "{line}");
        }

        Test {
            id,
            group,
            name,
            provenance,
            kind,
            body,
            data,
            codes,
        }
    }
}
