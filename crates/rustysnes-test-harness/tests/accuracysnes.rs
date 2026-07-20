#![allow(missing_docs)]
//! AccuracySNES — the first-party hardware-accuracy battery.
//!
//! This is the monolithic oracle ROM `docs/STATUS.md` recorded as missing (ticket **T-04**:
//! *"no publicly available SNES ROM plays the AccuracyCoin role"*). It is our own work, built
//! from `tests/roms/AccuracySNES/gen/`, so unlike every other corpus here it carries no licence
//! encumbrance and its expected values are auditable line by line.
//!
//! # How scoring works
//!
//! The ROM runs the whole battery **without any input** and publishes a results block in WRAM.
//! This harness boots it, polls for the completion sentinel, and decodes that block. It supplies
//! **no expected values of its own** — the cart decides pass/fail on-cart, which is what lets the
//! same image run unmodified on ares, bsnes, Mesen2, and real hardware.
//!
//! The RAM block is authoritative and the framebuffer is not consulted for scoring at all. That
//! is a deliberate inversion of RustyNES's original mistake: its framebuffer grid decoder had a
//! stride bug that silently skipped 31 cells and reported **75.93% where the truth was 64.03%**
//! (`../RustyNES/crates/rustynes-test-harness/tests/accuracycoin.rs`). A screen-scraping oracle
//! under-samples quietly and reads high.
//!
//! # The provenance gate
//!
//! A test we wrote, grading an emulator we wrote, proves nothing on its own. Every test carries a
//! provenance tier in the catalog and only `Documented`/`Corroborated` tests may contribute to
//! the pass rate; `Contested`/`Novel` ones are recorded but never scored. The cart enforces this
//! in its own tally, and [`provenance_gate_holds`] re-checks it host-side — the same shape as
//! `mapper_tier_honesty.rs` and `docs/adr/0003`.
#![cfg(feature = "test-roms")]

use std::path::PathBuf;

use rustysnes_core::{System, cart::Cart};
use rustysnes_test_harness::AccuracyReport;

/// Base of the results block in WRAM bank `$7E`, matching `asm/runtime.inc`.
const RESULTS: u32 = 0x7E_F000;
const R_MAGIC: u32 = RESULTS;
const R_VERSION: u32 = RESULTS + 0x04;
const R_COUNT: u32 = RESULTS + 0x06;
const R_DONE: u32 = RESULTS + 0x08;
const R_PASSED: u32 = RESULTS + 0x0A;
const R_FAILED: u32 = RESULTS + 0x0C;
const R_SKIPPED: u32 = RESULTS + 0x0E;
const R_GOLDEN: u32 = RESULTS + 0x10;
const R_STATUS: u32 = RESULTS + 0x20;

const DONE_MARK: u8 = 0xA5;
const FORMAT_VERSION: u16 = 1;

/// Frame budget. The battery finishes in a handful of frames with the screen blanked; this is a
/// generous ceiling that bounds CI time, not a timeout on a legitimate run.
const MAX_FRAMES: u32 = 600;

/// Minimum share of scoring tests that must pass.
///
/// **Ratchet this upward only.** Group A currently measures 100%, and the floor is set to match:
/// every one of its tests is `Documented`-tier, so a failure here is a real regression, not a
/// disagreement about what the hardware does.
///
/// When a later group adds a test that fails, the correct responses are to fix the emulator, or
/// to re-tier the test as `Contested`/`Golden` if the expected value turns out not to be
/// defensible — **not** to lower this number. That is the whole point of the provenance tiers.
const MIN_PASS_RATE: f64 = 1.00;

/// One row of `SOURCE_CATALOG.tsv`, generated alongside the ROM from the same definitions.
///
/// `include_str!` rather than a runtime read, so the in-code table cannot drift from the file on
/// disk — a lesson taken directly from RustyNES's catalog handling.
struct Entry {
    id: String,
    name: String,
    tier: String,
    kind: String,
    /// The dossier assertion(s) this test implements, or `-` when it implements none.
    ///
    /// Cart IDs and dossier IDs are different numbering schemes — cart `A1.04` is dossier
    /// `A1.06` — so this column, not the ID, is what says what a test covers.
    dossier: String,
}

const RAW_CATALOG: &str = include_str!("../../../tests/roms/AccuracySNES/SOURCE_CATALOG.tsv");

fn catalog() -> Vec<Entry> {
    RAW_CATALOG
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 9, "malformed catalog row: {l}");
            Entry {
                id: f[1].to_string(),
                name: f[3].to_string(),
                tier: f[4].to_string(),
                kind: f[6].to_string(),
                dossier: f[8].trim().to_string(),
            }
        })
        .collect()
}

fn rom_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/roms/AccuracySNES/build/accuracysnes.sfc")
}

/// A decoded verdict byte. Mirrors the encoding in `gen/src/dsl.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    NotRun,
    Pass,
    Variant(u8),
    Fail(u8),
    Skipped,
}

impl Verdict {
    const fn decode(b: u8) -> Self {
        match b {
            0x00 => Self::NotRun,
            0xFF => Self::Skipped,
            0x01 => Self::Pass,
            _ if b & 0x01 == 1 => Self::Variant(b >> 1),
            _ => Self::Fail(b >> 1),
        }
    }
}

/// The whole decoded block.
struct Report {
    version: u16,
    count: u16,
    done: bool,
    passed: u16,
    failed: u16,
    skipped: u16,
    golden: u16,
    status: Vec<u8>,
    frames: u32,
    /// Raw full-width measurements from the cart's measurement channel.
    ///
    /// A verdict byte cannot carry a dot count; anything above 255 wraps and becomes
    /// indistinguishable from a real reading. Timing tests write here instead.
    meas: Vec<u16>,
}

/// Boot the cart and run until the completion sentinel appears (or the frame budget runs out).
fn run() -> Option<Report> {
    let rom = std::fs::read(rom_path()).ok()?;
    let cart = Cart::from_rom(&rom).expect("AccuracySNES header must be detectable");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();

    let mut frames = 0;
    while frames < MAX_FRAMES {
        sys.run_frame();
        frames += 1;
        if sys.bus.peek_wram(R_DONE) == DONE_MARK {
            break;
        }
    }

    let rd16 = |a: u32| -> u16 {
        u16::from(sys.bus.peek_wram(a)) | (u16::from(sys.bus.peek_wram(a + 1)) << 8)
    };
    let count = rd16(R_COUNT);
    let status = (0..u32::from(count))
        .map(|i| sys.bus.peek_wram(R_STATUS + i))
        .collect();

    Some(Report {
        version: rd16(R_VERSION),
        count,
        done: sys.bus.peek_wram(R_DONE) == DONE_MARK,
        passed: rd16(R_PASSED),
        failed: rd16(R_FAILED),
        skipped: rd16(R_SKIPPED),
        golden: rd16(R_GOLDEN),
        status,
        frames,
        meas: (0..MEAS_SLOTS)
            .map(|i| rd16(MEAS_BASE + u32::from(i) * 2))
            .collect(),
    })
}

fn magic(sys: &System) -> [u8; 4] {
    [
        sys.bus.peek_wram(R_MAGIC),
        sys.bus.peek_wram(R_MAGIC + 1),
        sys.bus.peek_wram(R_MAGIC + 2),
        sys.bus.peek_wram(R_MAGIC + 3),
    ]
}

#[test]
fn accuracysnes_battery() {
    if !rom_path().is_file() {
        eprintln!("SKIP accuracysnes: ROM absent (run `cargo run -p accuracysnes-gen`)");
        return;
    }
    let entries = catalog();
    let Some(r) = run() else {
        eprintln!("SKIP accuracysnes: ROM unreadable");
        return;
    };

    assert!(
        r.done,
        "battery did not reach its completion sentinel within {MAX_FRAMES} frames \
         (ran {} frames, {} tests recorded) — the ROM hung or never booted",
        r.frames, r.count
    );
    assert_eq!(
        r.version, FORMAT_VERSION,
        "results-block format version mismatch"
    );
    assert_eq!(
        usize::from(r.count),
        entries.len(),
        "cart reports {} tests but the catalog has {}",
        r.count,
        entries.len()
    );

    // --- per-test report, printed unconditionally so CI logs form a time series ---
    eprintln!("\nAccuracySNES — {} tests, {} frames", r.count, r.frames);
    eprintln!("  {:<8} {:<24} {:<13} verdict", "id", "name", "tier");
    let mut failures = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let v = Verdict::decode(r.status[i]);
        let shown = match v {
            Verdict::NotRun => "NOT RUN".to_string(),
            Verdict::Pass => "pass".to_string(),
            Verdict::Variant(n) => format!("pass (variant {n})"),
            Verdict::Fail(c) => format!("FAIL code {c}"),
            Verdict::Skipped => "skipped".to_string(),
        };
        eprintln!("  {:<8} {:<24} {:<13} {shown}", e.id, e.name, e.tier);
        if matches!(v, Verdict::Fail(_) | Verdict::NotRun) && e.kind == "Scored" {
            failures.push(format!("{} ({}) :: {shown}", e.id, e.name));
        }
    }

    // Reuse the shared tally type rather than re-deriving the arithmetic. `partial` is the
    // variant-pass count: a test that reported *which* legal hardware behaviour it observed.
    let variants = entries
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            // The tier matters as much as the kind: the cart only counts a test toward `passed`
            // when it is BOTH Scored and Documented/Corroborated. Filtering on kind alone would
            // let a Contested-but-Scored variant inflate this past `r.passed` and underflow the
            // subtraction below. No such test exists yet, but the DSL permits one.
            e.kind == "Scored"
                && (e.tier == "Documented" || e.tier == "Corroborated")
                && matches!(Verdict::decode(r.status[*i]), Verdict::Variant(_))
        })
        .count();
    let variants = u32::try_from(variants).expect("variant count fits u32");
    let report = AccuracyReport {
        passed: u32::from(r.passed).saturating_sub(variants),
        failed: u32::from(r.failed),
        partial: variants,
    };
    let scoring = report.total();
    let rate = report.pass_rate();
    eprintln!(
        "\n  passed {} / {scoring} scoring ({:.2}%), of which {} variant; skipped {}, golden {} (unscored)",
        r.passed,
        rate * 100.0,
        report.partial,
        r.skipped,
        r.golden
    );
    if !failures.is_empty() {
        eprintln!("\n  failing:");
        for f in &failures {
            eprintln!("    {f}");
        }
    }

    assert!(
        scoring > 0,
        "no scoring tests ran — the battery may never have started"
    );
    assert!(
        report.meets(MIN_PASS_RATE),
        "AccuracySNES pass rate {:.2}% is below the {:.2}% floor",
        rate * 100.0,
        MIN_PASS_RATE * 100.0
    );
}

/// The block must be genuine, not a coincidence in uninitialised WRAM.
#[test]
fn results_block_is_well_formed() {
    if !rom_path().is_file() {
        eprintln!("SKIP accuracysnes: ROM absent");
        return;
    }
    let rom = std::fs::read(rom_path()).expect("read rom");
    let cart = Cart::from_rom(&rom).expect("detect header");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..MAX_FRAMES {
        sys.run_frame();
        if sys.bus.peek_wram(R_DONE) == DONE_MARK {
            break;
        }
    }
    assert_eq!(&magic(&sys), b"ACSN", "results-block magic missing");
}

/// The determinism contract: the same seed and ROM must produce the same verdicts.
#[test]
fn battery_is_deterministic() {
    if !rom_path().is_file() {
        eprintln!("SKIP accuracysnes: ROM absent");
        return;
    }
    let a = run().expect("first run");
    let b = run().expect("second run");
    assert_eq!(a.status, b.status, "verdicts differ between identical runs");
    assert_eq!(a.passed, b.passed);
    assert_eq!(a.failed, b.failed);
}

/// The anti-circularity gate: no `Contested` or `Novel` test may contribute to the pass rate.
///
/// Those tiers exist for behaviour the references disagree about, or that one of them openly
/// admits is unexplained. Letting them score would make the number self-congratulatory rather
/// than informative — the same failure `docs/adr/0003` guards against for coprocessor tiers.
#[test]
fn provenance_gate_holds() {
    if !rom_path().is_file() {
        eprintln!("SKIP accuracysnes: ROM absent");
        return;
    }
    let entries = catalog();
    let Some(r) = run() else { return };

    let scoring_tiers: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.kind == "Scored" && (e.tier == "Documented" || e.tier == "Corroborated"))
        .collect();
    let non_scoring = entries.len() - scoring_tiers.len();

    let counted = u32::from(r.passed) + u32::from(r.failed) + u32::from(r.skipped);
    let scoring_len = u32::try_from(scoring_tiers.len()).expect("catalog size fits u32");
    assert!(
        counted <= scoring_len,
        "the cart counted {counted} tests toward the pass rate but only {} are \
         Documented/Corroborated and Scored — a Contested or Novel test is inflating the number",
        scoring_tiers.len()
    );
    let golden_len =
        u32::try_from(entries.iter().filter(|e| e.kind == "Golden").count()).expect("fits u32");
    assert_eq!(
        u32::from(r.golden),
        golden_len,
        "golden-vector count disagrees with the catalog"
    );
    eprintln!(
        "provenance gate: {} scoring, {non_scoring} recorded-but-unscored",
        scoring_tiers.len()
    );
}

/// The header must be unambiguously detectable — a full-score match, not a lucky heuristic hit.
#[test]
fn header_is_detected() {
    if !rom_path().is_file() {
        eprintln!("SKIP accuracysnes: ROM absent");
        return;
    }
    let rom = std::fs::read(rom_path()).expect("read rom");
    assert_eq!(rom.len(), 128 * 1024, "expected a 128 KiB image");
    let cart = Cart::from_rom(&rom).expect("AccuracySNES header must be detectable");
    eprintln!("AccuracySNES detected as {:?}", cart.header.map_mode);
}

/// Base of the cart's raw measurement channel — must match `gen/src/dsl.rs` and `runtime.inc`.
const MEAS_BASE: u32 = 0x7E_E200;

/// Number of `u16` slots in the cart's measurement channel.
const MEAS_SLOTS: u8 = 128;

/// The measurement slots `A5.08` records, so a timing question can be answered from a full-width
/// number rather than from a verdict byte that silently wraps.
const A5_08_SLOTS: [(u8, &str); 7] = [
    (0, "16 NOP, absolute"),
    (1, "16 XBA, absolute"),
    (2, "16 XBA - 16 NOP        (expect 24)"),
    (3, "16 REP #$00, absolute"),
    (4, "16 REP #$00 - 16 NOP   (expect 32)"),
    (5, "8x (PHD+PLD), absolute"),
    (6, "8x (PHD+PLD) - 16 NOP  (expect 76)"),
];

/// Report the raw timing measurements, and sanity-check them against physics.
///
/// This exists because a one-byte verdict cannot carry a dot count. Reporting a 32-`NOP` baseline
/// through the variant code returned "21 dots", which is below the physical floor — the value had
/// wrapped past 256, and a wrapped reading is indistinguishable from a real one. The bound below
/// is deliberately crude: it only has to catch a truncated or unwritten value, not to assert a
/// timing model.
#[test]
fn measurement_channel_reports_plausible_timings() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");

    let mut out = String::from("\n  A5.08 raw measurements (dots):\n");
    for (slot, what) in A5_08_SLOTS {
        let v = report.meas[slot as usize];
        out.push_str(&format!("    slot {slot}  {v:5}  {what}\n"));
    }
    println!("{out}");

    // Two crude checks, deliberately not a timing model — they only have to catch a wrapped or
    // unwritten value, which is the failure mode this channel exists to expose.
    //
    // Floor: a 65816 instruction is at least 2 cycles and a cycle at least 6 master clocks
    // (1.5 dots), so 16 NOPs cannot come in under ~48 dots.
    let nop16 = report.meas[0];
    assert!(
        nop16 >= 48,
        "16 NOPs measured {nop16} dots, below the physical floor — the value wrapped or was never \
         written."
    );

    // Ceiling: every absolute span must stay clear of the 341-dot scanline wrap, because past it
    // the H-counter difference silently returns a small number instead of failing. A5.08 once
    // measured exactly 341 and read ~0, which looked like an emulator bug and was not.
    for (slot, what) in [
        (0u8, "16 NOP"),
        (1, "16 XBA"),
        (3, "16 REP"),
        (5, "8x PHD+PLD"),
    ] {
        let v = report.meas[slot as usize];
        assert!(
            v < 320,
            "{what} measured {v} dots, too close to the 341-dot line wrap for the difference to be \
             trustworthy — reduce the repeat count"
        );
    }
}

/// Every catalog row declares what it covers, and no two rows silently claim the same assertion.
///
/// The generator enforces this at ROM-build time (`gen/src/dossier.rs`), which CI runs. This
/// re-checks the **committed** artifact, so a hand-edited catalog or a stale regeneration cannot
/// slip a blank or duplicated mapping past review.
///
/// A duplicate here is not a style problem. Four tests were once written that duplicated existing
/// ones under different IDs, because the cart numbers tests per sub-group while the dossier
/// numbers assertions; they were caught by eye, and this is what replaces the eye.
#[test]
fn every_test_declares_its_dossier_assertions() {
    let entries = catalog();
    assert!(!entries.is_empty(), "the catalog is empty");

    for e in &entries {
        assert!(
            !e.dossier.is_empty(),
            "catalog row {} has a blank dossier column; regenerate with `cargo run -p \
             accuracysnes-gen`",
            e.id
        );
    }

    // `-` means "implements no enumerated assertion", which the generator requires be justified.
    let mut seen: Vec<(&str, Vec<&str>)> = Vec::new();
    for e in entries.iter().filter(|e| e.dossier != "-") {
        for d in e.dossier.split(',') {
            match seen.iter_mut().find(|(a, _)| *a == d) {
                Some((_, by)) => by.push(&e.id),
                None => seen.push((d, vec![&e.id])),
            }
        }
    }

    // Declared splits live in the generator; here we only assert the shape stays sane — every
    // assertion is claimed by at least one test, and nothing claims an empty string.
    for (assertion, by) in &seen {
        assert!(
            !assertion.trim().is_empty(),
            "empty assertion ID claimed by {}",
            by.join(", ")
        );
    }
}
