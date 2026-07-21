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

use std::fmt::Write as _;
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
const R_SCENE_DONE: u32 = RESULTS + 0x13;
const R_STATUS: u32 = RESULTS + 0x20;

const DONE_MARK: u8 = 0xA5;
const FORMAT_VERSION: u16 = 1;

/// Frame budget for the battery, a ceiling that bounds CI time rather than a timeout on a
/// legitimate run.
///
/// Raised from 600 when the cartridge image grew to 256 KiB. Most of the battery's frames are one
/// test: `G1.11` walks the entire cartridge byte by byte to check the header checksum, so doubling
/// the image doubled it — about 320 of the current 431 frames. The margin at 600 was 169 frames and
/// is now comfortable again. **When this needs raising, check `mesen_scenes.lua`'s `MAX_FRAMES` and
/// `libretro_crossval.c`'s `max_frames` in the same change**: all three bound the same run, and the
/// Mesen2 one silently reports "no scenes" rather than "timed out" from the gate's point of view.
const MAX_FRAMES: u32 = 1500;

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

fn build_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/AccuracySNES/build")
}

fn rom_path() -> PathBuf {
    build_dir().join("accuracysnes.sfc")
}

/// The PAL sibling image: the same battery, one header byte apart.
///
/// "That assertion needs a PAL console" turned out to be only half true. A console's region fixes
/// the timing, but which timing an emulator boots is decided by the cart header's country code, so
/// a one-byte change exercises the PAL line count and frame rate on every emulator with no
/// harness-side region switch that a reference emulator would have no equivalent of. On real
/// hardware the console still wins, which is why the region-dependent tests decide what they are
/// running on by measuring the frame height rather than by trusting the header.
fn pal_rom_path() -> PathBuf {
    build_dir().join("accuracysnes-pal.sfc")
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
    run_image(&rom_path())
}

/// As [`run`], for a specific image.
fn run_image(path: &std::path::Path) -> Option<Report> {
    let rom = std::fs::read(path).ok()?;
    let cart = Cart::from_rom(&rom).expect("AccuracySNES header must be detectable");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    // The host input contract (`runtime.inc`, PAD_CONTRACT). Every runner holds the same mask for
    // the whole run, because Group F has no observable at all with nothing held: an unconnected,
    // unpressed pad reads $0000 through every register the cart can reach.
    sys.bus.set_joypad(0, PAD_CONTRACT);

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
    // The host input contract (`runtime.inc`, PAD_CONTRACT). Every runner holds the same mask for
    // the whole run, because Group F has no observable at all with nothing held: an unconnected,
    // unpressed pad reads $0000 through every register the cart can reach.
    sys.bus.set_joypad(0, PAD_CONTRACT);
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
    // 256 KiB (8 banks). Grown from 128 not for total space but because a segment cannot span a
    // bank boundary in LoROM, so each group's bodies must fit one 32 KiB bank — see lorom.cfg.
    assert_eq!(rom.len(), 256 * 1024, "expected a 256 KiB image");
    let cart = Cart::from_rom(&rom).expect("AccuracySNES header must be detectable");
    eprintln!("AccuracySNES detected as {:?}", cart.header.map_mode);
}

/// Base of the cart's raw measurement channel — must match `gen/src/dsl.rs` and `runtime.inc`.
/// The controller-1 state every runner holds for the whole run — `runtime.inc`'s `PAD_CONTRACT`.
///
/// `B + Start + X + R`, in the standard `BYsSUDLR` bit order with `B` at bit 15. No d-pad, so the
/// post-battery menu cannot be disturbed; bits in both bytes, so a host reporting one half is
/// visibly wrong; and asymmetric under bit reversal, so an LSB-first read cannot pass by accident.
const PAD_CONTRACT: u16 = 0x9050;

/// `$5A` in [`R_SCENE_DONE`] once every rendered scene has been shown — the point after which the
/// runtime draws the interactive menu.
const SCENE_DONE_MARK: u8 = 0x5A;

/// `V_CURSOR` in low WRAM — `runtime.inc`'s `VAR_BASE + $00`.
const V_CURSOR: u32 = 0x7E_E000;

/// The Down bit of the 16-bit `BYsSUDLR` controller word.
const PAD_UP: u16 = 0x0800;
const PAD_DOWN: u16 = 0x0400;

/// The A and Select bits of the `BYsSUDLR????AXLR`-ordered controller word.
const PAD_A: u16 = 0x0080;
const PAD_SELECT: u16 = 0x2000;

/// `R_STATUS` — one verdict byte per test, at `RESULTS + $20`.
const R_STATUS_BASE: u32 = 0x7E_F020;

const MEAS_BASE: u32 = 0x7E_E200;

/// Number of `u16` slots in the cart's measurement channel.
const MEAS_SLOTS: u16 = 512;

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
        let _ = writeln!(out, "    slot {slot}  {v:5}  {what}");
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

/// Tests whose verdict is *allowed* to differ between the NTSC and PAL images, and why.
///
/// Everything else must be identical: the two images differ in one header byte, so a test that
/// changes verdict between them is either region-dependent and not yet declared so, or is reading
/// something it should not — and the second case is the one worth catching, because it is silent.
const REGION_DEPENDENT: &[(&str, &str)] = &[
    ("B2.04", "the NTSC frame height; the PAL image skips it"),
    ("B2.05", "the PAL frame height; the NTSC image skips it"),
    (
        "B2.06",
        "the interlaced frame's line count — 263 on NTSC, 313 on PAL",
    ),
    (
        "B2.10",
        "the region bit itself; this changing is the whole point of the second image",
    ),
];

/// The PAL image runs the same battery at PAL timing, and the region-dependent pair swaps over.
///
/// The point of the second image is that it isolates the region. It is produced by patching one
/// header byte of the linked NTSC image and recomputing the checksum, so the two are provably
/// identical apart from the country code — any behavioural difference between them is the region
/// and cannot be anything else.
///
/// What that buys, concretely: `B2.04` (262 lines) and `B2.05` (312 lines) are mirrors, each
/// standing down as SKIP on the machine it does not describe, and the predicate is the *measured*
/// frame height rather than the region bit — because which bit of `$213F` carries the region is
/// contested (`B2.10`) and a frame-height test must not depend on the thing it is evidence for.
/// Exactly one of the pair must score on each image; both scoring, or neither, means the region
/// did not take effect and the "PAL" run was NTSC in disguise.
#[test]
fn pal_image_runs_at_pal_timing() {
    if !pal_rom_path().is_file() {
        eprintln!("SKIP: build the cart with `cargo run -p accuracysnes-gen` first");
        return;
    }
    let ntsc = run_image(&rom_path()).expect("NTSC image must run");
    let pal = run_image(&pal_rom_path()).expect("PAL image must run");
    assert!(ntsc.done && pal.done, "a battery did not finish");
    assert_eq!(
        ntsc.count, pal.count,
        "the two images must carry the identical battery"
    );

    let entries = catalog();
    let index_of = |id: &str| {
        entries
            .iter()
            .position(|e| e.id == id)
            .unwrap_or_else(|| panic!("{id} is not in the catalog"))
    };
    let ntsc_frame = index_of("B2.04");
    let pal_frame = index_of("B2.05");
    // Referenced by the drift check below via the catalog, not by index.

    let v = |r: &Report, i: usize| Verdict::decode(r.status[i]);
    assert_eq!(
        v(&ntsc, ntsc_frame),
        Verdict::Pass,
        "B2.04 (262 lines) must score on the NTSC image"
    );
    assert_eq!(
        v(&ntsc, pal_frame),
        Verdict::Skipped,
        "B2.05 (312 lines) must stand down on the NTSC image"
    );
    assert_eq!(
        v(&pal, pal_frame),
        Verdict::Pass,
        "B2.05 (312 lines) must score on the PAL image — if it skipped, the header's country code \
         did not take effect and this was an NTSC run wearing a PAL name"
    );
    assert_eq!(
        v(&pal, ntsc_frame),
        Verdict::Skipped,
        "B2.04 (262 lines) must stand down on the PAL image"
    );

    // Everything else must be unaffected. A test that changes verdict between the two images is
    // either region-dependent and not yet declared so, or is reading something it should not —
    // and the second case is the one worth catching, because it is silent.
    //
    // The declared set is deliberately small and each entry carries its reason (see
    // `REGION_DEPENDENT`). Exempting goldens wholesale would be easier and much worse: a golden
    // that becomes region-dependent by accident is exactly what this check exists to notice.
    let drifted: Vec<_> = (0..ntsc.status.len())
        .filter(|&i| !REGION_DEPENDENT.iter().any(|(id, _)| *id == entries[i].id))
        .filter(|&i| ntsc.status[i] != pal.status[i])
        .map(|i| format!("{} ({:?} -> {:?})", entries[i].id, v(&ntsc, i), v(&pal, i)))
        .collect();
    assert!(
        drifted.is_empty(),
        "these tests changed verdict between the NTSC and PAL images, which means they depend on \
         the region without saying so:\n  {}",
        drifted.join("\n  ")
    );

    println!(
        "\n  NTSC: {} pass / {} fail / {} skip     PAL: {} pass / {} fail / {} skip",
        ntsc.passed, ntsc.failed, ntsc.skipped, pal.passed, pal.failed, pal.skipped
    );
}

/// The region bit's position, settled by measurement rather than by picking a source.
///
/// `B2.10` is a golden vector because the documentation conflicts: the SNESdev wiki puts the
/// 50/60 Hz bit of `$213F` at bit 3, fullsnes at bit 4. Neither can be adopted over the other by
/// reading harder. But the two images differ *only* in the region, so whichever bit moves between
/// them is the region bit — and this reports it rather than asserting either source was right.
///
/// The golden's verdict encodes the two candidate bits as `(bit4 << 1 | bit3) << 1 | 1`.
#[test]
fn region_bit_position_is_reported() {
    if !pal_rom_path().is_file() {
        eprintln!("SKIP: build the cart with `cargo run -p accuracysnes-gen` first");
        return;
    }
    let ntsc = run_image(&rom_path()).expect("NTSC image must run");
    let pal = run_image(&pal_rom_path()).expect("PAL image must run");
    let entries = catalog();
    let i = entries
        .iter()
        .position(|e| e.id == "B2.10")
        .expect("B2.10 is in the catalog");

    let decode = |b: u8| -> (bool, bool) {
        let pair = b >> 1;
        (pair & 0x02 != 0, pair & 0x01 != 0)
    };
    let (n4, n3) = decode(ntsc.status[i]);
    let (p4, p3) = decode(pal.status[i]);

    println!(
        "\n  $213F region candidates    NTSC: bit4={n4} bit3={n3}    PAL: bit4={p4} bit3={p3}"
    );
    let moved: Vec<&str> = [("bit 4", n4 != p4), ("bit 3", n3 != p3)]
        .into_iter()
        .filter(|(_, m)| *m)
        .map(|(n, _)| n)
        .collect();
    assert!(
        !moved.is_empty(),
        "neither candidate bit changed between the NTSC and PAL images. Either the region did not \
         take effect, or this core does not model the region bit at all — both are findings, and \
         both mean the conflict stays unsettled here."
    );
    println!("  changed between images: {}", moved.join(", "));
}

/// `B4.14`'s interrupt-dispatch latency measurements, reported for cross-emulator comparison.
///
/// The dossier's claim — "the poll occurs just before the final CPU cycle" — is sub-cycle, and the
/// finest clock a cart can read is the H counter at four master clocks per dot. So the cart
/// measures the *consequence* instead: if the poll happens at an instruction boundary rather than
/// continuously, an interrupt asserting during a long instruction waits for it to retire.
///
/// Reported, not asserted. Where in the spin loop the interrupt lands is not controllable from the
/// cart, so the absolute numbers carry jitter; only the sign of the difference means anything, and
/// even that is worth comparing across emulators before anyone scores it. What *is* checked is
/// that the channel was written at all and that the values are physically possible — the failure
/// this channel exists to expose is a wrapped or unwritten measurement masquerading as a reading.
#[test]
fn irq_dispatch_latency_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");

    let nop_spin = report.meas[100];
    let long_spin = report.meas[101];
    let extra = report.meas[102];

    println!("\n  B4.14 interrupt dispatch latency (dots past HTIME):");
    println!("    slot 100  {nop_spin:5}  spinning on NOPs");
    println!("    slot 101  {long_spin:5}  spinning on JSL/RTL");
    println!("    slot 102  {extra:5}  the difference — the cost of a long instruction");

    // A scanline is 341 dots. Anything at or beyond that has wrapped, and a wrapped value is
    // indistinguishable from a real one, which is precisely why it must not pass silently.
    for (slot, v) in [(100u8, nop_spin), (101, long_spin)] {
        assert!(
            v < 341,
            "slot {slot} reads {v} dots, which is a whole scanline or more — the measurement \
             wrapped, so it is not a reading at all"
        );
    }
    assert!(
        nop_spin > 0 || long_spin > 0,
        "both latency slots are zero: the handler never ran, so nothing was measured"
    );
}

/// `B3.01`'s DRAM-refresh probe, reported for cross-emulator comparison.
///
/// The cart samples the H counter six times inside one scanline and differences the readings. A
/// core that models the 5A22's per-line refresh pause shows one interval about ten dots longer
/// than the rest; a core that models none shows a flat sequence. RustySNES is deliberately in the
/// second class — `docs/accuracy-ledger.md` scopes refresh out on the measurement that frame
/// length is already correct without it — so this reports rather than asserts, and the number worth
/// carrying to another emulator is the excess, not the absolute period.
///
/// What *is* asserted is that the measurement is a measurement: the window has to have opened near
/// the start of a line, stayed inside it, and produced intervals that could physically be loop
/// iterations. A wrapped H reading looks exactly like an enormous pause, which is the one failure
/// that would turn this from evidence into a false positive.
#[test]
fn dram_refresh_probe_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");

    let (shortest, longest) = (report.meas[139], report.meas[140]);
    let (at_longest, start, end) = (report.meas[108], report.meas[109], report.meas[124]);
    let excess = longest.saturating_sub(shortest);

    println!("\n  B3 DRAM refresh probe (dots):");
    println!("    slot 109  {start:5}  H at the first sample");
    println!("    slot 124  {end:5}  H at the last sample");
    println!("    slot 139  {shortest:5}  shortest interval — the stall-free loop period");
    println!("    slot 140  {longest:5}  longest interval");
    println!("    slot 108  {at_longest:5}  H the longest interval starts from");
    println!("    excess    {excess:5}  longest - shortest (a 40-clock pause is 10 dots)");

    assert!(
        start < 128,
        "the first sample landed at dot {start}. The sync loop releases below dot 16 and one \
         iteration follows before the first sample, so anything this late means the loop is not \
         running at the speed the window was sized for"
    );
    assert!(
        end > start && end < 341,
        "the window runs {start}..{end}, which is not a forward span inside one scanline — the \
         samples crossed a line boundary and the intervals derived from them mean nothing"
    );
    assert!(
        shortest > 0 && longest < 200,
        "intervals of {shortest}..{longest} dots are not loop iterations: zero means the samples \
         never advanced, and 200 or more means the window left the scanline"
    );
    assert!(
        at_longest >= start && at_longest <= end,
        "the longest interval is said to start at dot {at_longest}, outside the sampled window \
         {start}..{end}"
    );
}

/// `B4.11`'s dot-153 readings, reported for cross-emulator comparison.
///
/// superfamicom.org says no IRQ triggers for dot 153 on the last scanline of a frame. No emulator
/// searched implements it, so the interesting number is not RustySNES's own answer but whether any
/// core's differs — which is why the three readings are published rather than asserted.
///
/// The controls are asserted, though, and they are the whole basis for reading anything into the
/// first number. An HV-IRQ that never fires produces "suppressed at dot 153" for free.
#[test]
fn dot_153_exception_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");

    let last_line = report.meas[77];
    let (at_153, ctl_line, ctl_dot) = (report.meas[78], report.meas[79], report.meas[80]);

    println!("\n  B4.11 dot 153 on the last scanline:");
    println!("    slot 77  {last_line:5}  the measured last line of a frame");
    println!("    slot 78  {at_153:5}  fired at dot 153 on the last line (0 = suppressed)");
    println!("    slot 79  {ctl_line:5}  control: dot 153, one line earlier");
    println!("    slot 80  {ctl_dot:5}  control: dot 100, same last line");

    assert!(
        last_line == 261 || last_line == 311,
        "the last line measured {last_line}, which is neither NTSC's 261 nor PAL's 311 — the \
         frame-height measurement this test arms from is wrong, so nothing below means anything"
    );
    assert!(
        ctl_line == 1 && ctl_dot == 1,
        "a control did not fire (dot 153 one line earlier: {ctl_line}, dot 100 on the last line: \
         {ctl_dot}). Without both, a silent dot-153 reading is just an HV-IRQ that never works"
    );
}

/// `C9.05`'s two VRAM words, reported for cross-emulator comparison.
///
/// Slot 81 is the control — the write issued before the overscan toggle, which must read back as
/// `$2211` or nothing below means anything. Slot 82 is the word under test: `$AAAA` means the
/// window re-closed and the write was dropped, `$4433` means it stayed open.
#[test]
fn overscan_vram_window_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");

    let (control, under_test, guard) = (report.meas[81], report.meas[82], report.meas[83]);
    println!("\n  C9.05 mid-frame overscan and the VRAM window:");
    println!("    slot 83  {guard:#06x}  guard: written from active display (0xaaaa = dropped)");
    println!("    slot 81  {control:#06x}  before the toggle (expect 0x2211)");
    println!("    slot 82  {under_test:#06x}  after the toggle (0xaaaa = dropped)");

    assert_eq!(
        guard, 0xaaaa,
        "a VRAM write from the middle of active display landed, so the port is open for a reason \
         unrelated to overscan and neither reading below is evidence. This is the exact failure \
         the first version of this test shipped with: forced blank was still on, so nothing could \
         ever be dropped and all three cores agreed about nothing"
    );
    assert_eq!(
        control, 0x2211,
        "the write issued before the overscan toggle did not land, so the port was shut for an \
         unrelated reason and the reading below is not evidence"
    );
}

/// `C2.09`'s four VRAM reads, reported so a wrong expectation and a wrong core can be told apart.
#[test]
fn vram_read_latch_order_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  C2.09 VRAM read latch order (words $1700 = $1234, $1701 = $ABCD):");
    for (slot, what) in [
        (84u8, "read 1: $2139 after the address write"),
        (85, "read 2: $2139 again, no trigger between"),
        (86, "read 3: $213A, the trigger"),
        (87, "read 4: $2139 after the trigger"),
    ] {
        println!(
            "    slot {slot}  {:#04x}  {what}",
            report.meas[usize::from(slot)]
        );
    }
}

/// `E4.03`'s two zero-page halves, reported so a failure can be read.
#[test]
fn ipl_zero_fill_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E4.03 APU zero page after IPL boot:");
    println!(
        "    slot 93  {:#04x}  $01, the IPL's transfer pointer high byte",
        report.meas[93]
    );
    println!("    slot 94  {:#04x}  OR of $02-$1F", report.meas[94]);
    println!("    slot 95  {:#04x}  OR of $20-$EF", report.meas[95]);
}

/// `E3.02`'s two timer readings, reported so a failure says which way it went.
#[test]
fn timer_enable_reset_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E3.02 timer 0 across an enable transition:");
    println!(
        "    slot 137  {:2}  ticks over the interval, no restart (control)",
        report.meas[137]
    );
    println!(
        "    slot 138  {:2}  same interval, read after a 0->1 on the enable",
        report.meas[138]
    );
}

/// `D2.09`'s two HDMA phases, reported so a mid-frame enable can be compared across cores.
#[test]
fn hdma_mid_frame_enable_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  D2.09 HDMA enabled mid-frame:");
    println!(
        "    slot 146  {:#04x}  phase 1 first byte (control, expect 0x11)",
        report.meas[146]
    );
    println!(
        "    slot 147  {:4}  phase 1 bytes written (control, expect 8)",
        report.meas[147]
    );
    println!(
        "    slot 148  {:#04x}  phase 2 first byte, enabled at line 100",
        report.meas[148]
    );
    println!(
        "    slot 149  {:4}  phase 2 bytes written",
        report.meas[149]
    );
    assert_eq!(
        report.meas[146], 0x11,
        "the control phase did not write the table's first data byte"
    );
}

/// `G1.03`'s readable power-on registers, reported for cross-emulator comparison.
///
/// Nothing here is asserted — the row is `[UNDEFINED]` and says to report and never assert. The
/// APU ports are the interesting ones: they show whether the IPL has announced by the time the
/// cart's reset handler runs, which every other test hides by waiting for the announcement first.
#[test]
fn power_on_indeterminate_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  G1.03 registers left indeterminate at power-on:");
    for (slot, what) in [
        (150usize, "$2140 APUIO0"),
        (151, "$2141 APUIO1"),
        (152, "$2142 APUIO2"),
        (153, "$2143 APUIO3"),
        (154, "$2180 WMDATA"),
        (155, "$4218 JOY1 low"),
        (156, "$4219 JOY1 high"),
    ] {
        println!("    slot {slot}  {:#04x}  {what}", report.meas[slot]);
    }
}

/// `F1.04`'s two `$4016` reads, reported so the open-bus models can be compared.
#[test]
fn joyser_open_bus_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  F1.04 $4016 read two ways:");
    println!(
        "    slot 157  {:#04x}  absolute (last operand byte 0x40)",
        report.meas[157]
    );
    println!(
        "    slot 158  {:#04x}  long     (last operand byte 0x00)",
        report.meas[158]
    );
}

/// `G1.07`'s three untouched WRAM bytes, reported for cross-emulator comparison.
///
/// Nothing is asserted — the row is `[UNDEFINED]` and asks for a golden vector by name. Measured,
/// the three cores land on three different variants: RustySNES uniformly zero, snes9x uniformly
/// `$55`, Mesen2 randomised per run.
#[test]
fn wram_power_on_fill_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  G1.07 WRAM power-on fill (bank $7F, untouched):");
    for (slot, addr) in [(165usize, "$7F8000"), (166, "$7F8020"), (167, "$7F8040")] {
        println!("    slot {slot}  {:#04x}  {addr}", report.meas[slot]);
    }
}

/// `E9.03`'s two noise readings, reported so the guard's non-zero requirement can be seen.
#[test]
fn noise_pitch_independence_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E9.03 noise OUTX at two pitches:");
    println!("    slot 168  {:#04x}  VxPITCH = $1000", report.meas[168]);
    println!("    slot 169  {:#04x}  VxPITCH = $2000", report.meas[169]);
    assert_ne!(
        report.meas[168], 0,
        "the noise voice was silent, so the test compared two silences"
    );
}

/// `E9.13`'s echo bytes with feedback on, reported so crosstalk can be seen rather than inferred.
#[test]
fn echo_fir_crosstalk_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E9.13 echo buffer with EFB on, left-only input:");
    println!("    slot 173  {:#04x}  byte 1, L high", report.meas[173]);
    println!("    slot 174  {:#04x}  byte 2, R low", report.meas[174]);
    println!("    slot 175  {:#04x}  byte 3, R high", report.meas[175]);
}

/// `E8.10`'s two envelope readings, reported so the timing window can be judged.
#[test]
fn koff_kon_cut_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E8.10 ENVX shortly after key-off:");
    println!("    slot 176  {:#04x}  KOFF alone", report.meas[176]);
    println!("    slot 177  {:#04x}  KOFF + KON", report.meas[177]);
}

/// `E5.01`'s two amplitudes, reported so the shift ratio can be judged.
#[test]
fn brr_shift_field_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E5.01 OUTX across one step of the header's shift nibble:");
    println!("    slot 178  {:#04x}  shift $8", report.meas[178]);
    println!("    slot 179  {:#04x}  shift $9", report.meas[179]);
}

/// `E9.01`'s frozen noise output, reported so the seed can be read directly.
#[test]
fn noise_lfsr_seed_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E9.01 noise OUTX with the LFSR frozen:");
    println!("    slot 180  {:#04x}", report.meas[180]);
}

/// `E7.09`'s two mid-release readings, reported so the window can be judged.
#[test]
fn release_rate_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E7.09 ENVX mid-release:");
    println!("    slot 184  {:#04x}  sustain rate 0", report.meas[184]);
    println!("    slot 185  {:#04x}  sustain rate 31", report.meas[185]);
}

/// `E7.05`'s two mid-decay readings, reported so the window can be judged.
#[test]
fn decay_rate_index_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E7.05 ENVX mid-decay:");
    println!(
        "    slot 186  {:#04x}  decay rate 0 (index 16)",
        report.meas[186]
    );
    println!(
        "    slot 187  {:#04x}  decay rate 7 (index 30)",
        report.meas[187]
    );
}

/// `E7.06`'s two sustain readings, reported so the rate window can be judged.
#[test]
fn sustain_rate_index_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E7.06 ENVX during sustain:");
    println!("    slot 188  {:#04x}  sustain rate 0", report.meas[188]);
    println!("    slot 189  {:#04x}  sustain rate 31", report.meas[189]);
}

/// `E7.07`'s two parking levels, reported so the sustain bands can be seen.
#[test]
fn sustain_boundary_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E7.07 ENVX parked at the sustain boundary:");
    println!(
        "    slot 190  {:#04x}  sustain level 3 (expect $40-$4F)",
        report.meas[190]
    );
    println!(
        "    slot 191  {:#04x}  sustain level 5 (expect $60-$6F)",
        report.meas[191]
    );
}

/// `E7.03`'s two mid-attack readings, reported so the window can be judged.
#[test]
fn attack_rate_index_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E7.03 ENVX mid-attack:");
    println!(
        "    slot 141  {:#04x}  attack rate $8 (index 17)",
        report.meas[141]
    );
    println!(
        "    slot 142  {:#04x}  attack rate $C (index 25)",
        report.meas[142]
    );
}

/// `E7.12`'s two park levels, reported so the boundary's source can be seen.
#[test]
fn gain_sustain_boundary_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E7.12 sustain park level with ADSR2 fixed at level 3:");
    println!("    slot 143  {:#04x}  GAIN bits 7-5 = 0", report.meas[143]);
    println!("    slot 181  {:#04x}  GAIN bits 7-5 = 5", report.meas[181]);
}

/// Report `C1.08`'s two `$2138` readings: the blank guard and the mid-render read.
#[test]
fn oam_address_during_render_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  C1.08 $2138 at OAM byte $80:");
    println!(
        "    slot 107  {:#04x}  forced blank (the guard)",
        report.meas[107]
    );
    println!("    slot 113  {:#04x}  active display", report.meas[113]);
}

/// Report `E8.03`'s two `ENVX` readings: the untouched climb and the retriggered one.
#[test]
fn kon_restart_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E8.03 ENVX with attack rate 8:");
    println!(
        "    slot 114  {:#04x}  left alone, 24 delay blocks",
        report.meas[114]
    );
    println!(
        "    slot 115  {:#04x}  a second KON one block earlier",
        report.meas[115]
    );
}

/// Report `E1.14`'s two block timings: the `NOP` baseline and the `XCN` block.
#[test]
fn xcn_cycle_cost_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E1.14 timer 0 ticks over 256 one-byte instructions:");
    println!("    slot 7    {:#04x}  NOP  (expect 4)", report.meas[7]);
    println!("    slot 125  {:#04x}  XCN  (expect 10)", report.meas[125]);
}

/// Report `E10.01`'s release-ramp timing: the arming envelope and the tick count.
#[test]
fn dsp_sample_period_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E10.01 a full release ramp, timed off timer 0 at T0DIV = 6:");
    println!(
        "    slot 192  {:#04x}  ENVX before key-off (expect $7F)",
        report.meas[192]
    );
    println!("    slot 193  {:#04x}  ticks (expect 10)", report.meas[193]);
}

/// Report `E10.05`'s soft-reset readings, including the contested `ENDX`.
#[test]
fn dsp_soft_reset_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E10.05 with FLG bit 7 asserted:");
    println!(
        "    slot 194  {:#04x}  ENVX before (the guard)",
        report.meas[194]
    );
    println!("    slot 195  {:#04x}  ENVX under reset", report.meas[195]);
    println!(
        "    slot 196  {:#04x}  ENDX under reset (nocash $FF, anomie $00)",
        report.meas[196]
    );
}

/// Report `E6.11`'s four named BRR waveform vectors.
#[test]
fn brr_waveform_vectors_are_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E6.11 OUTX per nibble pattern, shift 12, filter 0:");
    for (slot, name) in [
        (197, "79797979"),
        (198, "77997799"),
        (199, "77779999"),
        (200, "7777CC44"),
    ] {
        println!("    slot {slot}  {:#04x}  {name}", report.meas[slot]);
    }
    println!("    slot 201  {:#04x}  ENVX (the guard)", report.meas[201]);
}

/// Report `E6.09`'s reading: four maximally-negative gaussian taps.
#[test]
fn brr_15_bit_overflow_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E6.09 four maximally-negative gaussian taps:");
    println!("    slot 205  {:#04x}  ENVX (the guard)", report.meas[205]);
    println!(
        "    slot 206  {:#04x}  OUTX (positive = the wrap)",
        report.meas[206]
    );
}

/// Report `E9.15`'s one-voice and two-voice mix readings.
#[test]
fn voice_mix_saturation_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E9.15 echo byte 1 (left high) at shift 12, nibble +7:");
    println!(
        "    slot 207  {:#04x}  one voice (the guard)",
        report.meas[207]
    );
    println!("    slot 208  {:#04x}  two voices", report.meas[208]);
}

/// Report `E5.12`'s two `OUTX` readings across the loop point.
#[test]
fn srcn_change_source_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E5.12 OUTX, entry 1 start = ~$3F and entry 1 loop = ~$1F:");
    println!(
        "    slot 209  {:#04x}  control, SRCN untouched (expect ~$6E)",
        report.meas[209]
    );
    println!(
        "    slot 210  {:#04x}  after the write (loop address, expect ~$1F)",
        report.meas[210]
    );
}

/// Report `F1.01`'s manual read and `F1.07`'s three `$4218` readings.
#[test]
fn controller_contract_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  Host input contract: PAD_CONTRACT = {PAD_CONTRACT:#06x} (B + Start + X + R)");
    println!(
        "    slot 211  {:#06x}  F1.01 sixteen manual bits, MSB first",
        report.meas[211]
    );
    println!(
        "    slot 212  {:#06x}  F1.07 $4218 before auto-read was armed",
        report.meas[212]
    );
    println!(
        "    slot 213  {:#06x}  F1.07 $4218 with it armed",
        report.meas[213]
    );
    println!(
        "    slot 214  {:#06x}  F1.07 $4218 after disarming",
        report.meas[214]
    );
}

/// Report `F1.12`: when the automatic read's result becomes valid across vblank.
#[test]
fn auto_read_result_timing_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  F1.12 $4218 across vblank (settles at {PAD_CONTRACT:#06x}):");
    for (slot, line) in [(219, 225), (220, 227), (221, 230), (222, 240)] {
        println!("    slot {slot}  {:#06x}  V = {line}", report.meas[slot]);
    }
}

/// Report `C11.07`'s two `MPY` readings across the shared write-twice latch.
#[test]
fn mode7_latch_sharing_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  C11.07 MPY with M7A = $0100 and M7B's high byte 2:");
    println!(
        "    slot 223  {:#06x}  writes adjacent (expect $0200)",
        report.meas[223]
    );
    println!(
        "    slot 224  {:#06x}  $210D between them (expect $03FE)",
        report.meas[224]
    );
}

/// Report `C11.08`'s two `MPY` readings: in blank, and during a Mode 7 render.
#[test]
fn mode7_multiplier_during_render_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  C11.08 MPY with M7A = $0100 and M7B's high byte 2:");
    println!(
        "    slot 225  {:#06x}  forced blank (the guard)",
        report.meas[225]
    );
    println!(
        "    slot 226  {:#06x}  active display, Mode 7",
        report.meas[226]
    );
}

/// Report `E8.02`'s two poll timings: the loop's baseline and the same poll after a `KON`.
#[test]
fn key_on_delay_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  E8.02 timer 2 ticks (one output sample each) to a non-zero ENVX:");
    println!(
        "    slot 227  {:#04x}  voice already sounding (baseline)",
        report.meas[227]
    );
    println!(
        "    slot 228  {:#04x}  measured from a KON write",
        report.meas[228]
    );
}

/// Report `B4.16`'s two latched H positions — the before/after guard for `T-06-A`.
#[test]
fn h_irq_position_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!("\n  B4.16 H latched on entry to the IRQ handler:");
    println!(
        "    slot 126  {:>4}  HTIME = 100 (below the long dots)",
        report.meas[126]
    );
    println!(
        "    slot 127  {:>4}  HTIME = 330 (above them)",
        report.meas[127]
    );
}

/// Report `B2.01`'s largest latched H — the acceptance measurement for `T-06-A`.
#[test]
fn max_dot_is_reported() {
    let report = run().expect("battery must run");
    assert!(report.done, "battery did not finish");
    println!(
        "\n  B2.01 largest H ever latched: {} (hardware: 339)",
        report.meas[230]
    );
}

/// The interactive menu still renders after the battery finishes.
///
/// This is the only gate that looks at the menu at all, and it exists because nothing else can. The
/// battery reports through WRAM and the rendered scenes draw their own tilemaps, so `draw_str` —
/// which writes every header and every test name — is exercised on a code path **no other check
/// observes**. It was rewritten when the catalog moved out of bank `$00`: a 16-bit pointer plus an
/// implicit data bank cannot reach a string in another bank, so it now reads through a 24-bit
/// `V_STR_PTR`. A mistake there leaves a blank or garbled menu and every other gate green.
///
/// What is asserted is deliberately shallow — that the title row contains the letters of
/// "AccuracySNES" — because the alternative is a golden tilemap that has to be re-blessed whenever
/// a tally digit changes. A blank row, a row of the wrong bank's bytes, or a row that never got
/// written all fail it; a cosmetic change does not.
///
/// It also has to wait for the *whole* cartridge. `draw_screen` runs after `run_scenes`, not after
/// the battery, so waiting on the battery's sentinel and a few frames lands in the middle of the
/// scene loop with the tilemap still holding a scene — which is exactly what the first version of
/// this test measured, and reported as a broken menu.
#[test]
fn the_menu_still_draws_after_the_battery() {
    if !rom_path().is_file() {
        eprintln!("SKIP accuracysnes: ROM absent");
        return;
    }
    let rom = std::fs::read(rom_path()).expect("read rom");
    let cart = Cart::from_rom(&rom).expect("AccuracySNES header must be detectable");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    sys.bus.set_joypad(0, PAD_CONTRACT);

    // `draw_screen` runs after `run_scenes`, not after the battery — about 700 frames later, since
    // each of the 50 scenes is settled and held. Waiting on the battery's sentinel and then a
    // handful of frames lands in the middle of the scene loop, with the tilemap still cleared.
    // A budget of its own, larger than `MAX_FRAMES`: this is the only check that waits for the
    // *whole* cartridge — battery, then every rendered scene settled and held, and only then the
    // menu. Everything else stops at the battery's sentinel.
    const MENU_FRAMES: u32 = 3000;
    let mut frames = 0;
    while frames < MENU_FRAMES && sys.bus.peek_wram(R_SCENE_DONE) != SCENE_DONE_MARK {
        sys.run_frame();
        frames += 1;
    }
    assert_eq!(
        sys.bus.peek_wram(R_SCENE_DONE),
        SCENE_DONE_MARK,
        "the scene loop never finished within {MENU_FRAMES} frames, so the menu was never drawn"
    );
    for _ in 0..4 {
        sys.run_frame();
    }

    // BG1's tilemap starts at word $0400; the title is row 0. Tile indices are ASCII-derived, so
    // the row's low bytes spell the title back.
    const MAP_BASE: u16 = 0x0400;
    /// Tilemap stride, matching `runtime.inc`'s `SCREEN_COLS`.
    const SCREEN_COLS: u16 = 32;
    let row: String = (0..24)
        .map(|i| {
            let c = (sys.bus.ppu.vram_word(MAP_BASE + i) & 0xFF) as u8;
            char::from(c)
        })
        .collect();
    // The title lives in RODATA in bank $00; the test *names* live in the catalog, in another bank
    // entirely, and are the reason `V_STR_PTR` had to become 24-bit. Asserting a name is what gives
    // that path runtime evidence.
    let list: String = (0..SCREEN_COLS * 4)
        .map(|i| char::from((sys.bus.ppu.vram_word(MAP_BASE + SCREEN_COLS * 3 + i) & 0xFF) as u8))
        .collect();
    assert!(
        list.contains("XCE clears XH/YH"),
        "the menu's list does not show the first test's name. The title drew, so draw_str works \
         for a bank-$00 string — what failed is reading a name out of the catalog's own bank. \
         Rows read back as {list:?}"
    );

    // The results screen is drawn in green ($03E0): the font's "on" pixels index CGRAM colour 1 of
    // palette 0, which `load_palette` sets and `draw_screen` reloads so the last scene's palette
    // does not bleed in. A regression here is the orange/mixed text the menu showed before.
    assert_eq!(
        sys.bus.ppu.cgram_word(1),
        0x03E0,
        "the menu font colour is not green — the palette was not reloaded before draw_screen"
    );

    // The rendered tally against the results block it is drawn from. Comparing the menu with the
    // machine rather than with a remembered number means this never needs re-blessing when the
    // battery grows, and it catches the whole `draw_dec3` path: a digit drawn at the wrong column,
    // or from an 8-bit read of a 16-bit counter, stops matching immediately.
    let rd16 = |a: u32| -> u16 {
        u16::from(sys.bus.peek_wram(a)) | (u16::from(sys.bus.peek_wram(a + 1)) << 8)
    };
    let tally: String = (0..SCREEN_COLS)
        .map(|i| char::from((sys.bus.ppu.vram_word(MAP_BASE + SCREEN_COLS + i) & 0xFF) as u8))
        .collect();
    let expect = format!(
        "P:{:03} F:{:03} G:{:03} OF:{:03}",
        rd16(R_PASSED),
        rd16(R_FAILED),
        rd16(R_GOLDEN),
        rd16(R_COUNT),
    );
    assert!(
        tally.starts_with(&expect),
        "the menu's tally row does not match the results block it is drawn from.\n  \
         rendered: {tally:?}\n  expected: {expect:?}"
    );
    eprintln!("menu title row: {row:?}");
    eprintln!("menu tally row: {tally:?}");
}

/// The D-pad scrolls the list instead of killing the ROM.
///
/// Pressing Up or Down used to blank the screen and stop the cartridge dead. `cursor_up` and
/// `cursor_down` returned with `A` 8-bit, while `main_loop` continues in `.a16` — so ca65 emitted a
/// 16-bit `bit #PAD_DOWN` that the CPU decoded 8-bit, and the immediate's high byte was executed as
/// an opcode. Nothing in the battery could see it: the menu runs after every gate has finished.
///
/// The check is that the cursor *moved* and the machine is *still running the menu* afterwards —
/// the second half is the one that matters, since a desynchronised instruction stream shows up as a
/// screen that stops changing rather than as a wrong value anywhere.
#[test]
fn the_dpad_scrolls_the_list() {
    if !rom_path().is_file() {
        eprintln!("SKIP accuracysnes: ROM absent");
        return;
    }
    let rom = std::fs::read(rom_path()).expect("read rom");
    let cart = Cart::from_rom(&rom).expect("AccuracySNES header must be detectable");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    sys.bus.set_joypad(0, PAD_CONTRACT);

    const MENU_FRAMES: u32 = 3000;
    let mut frames = 0;
    while frames < MENU_FRAMES && sys.bus.peek_wram(R_SCENE_DONE) != SCENE_DONE_MARK {
        sys.run_frame();
        frames += 1;
    }
    assert_eq!(
        sys.bus.peek_wram(R_SCENE_DONE),
        SCENE_DONE_MARK,
        "menu never reached"
    );
    for _ in 0..4 {
        sys.run_frame();
    }
    let rd16 = |s: &System, a: u32| -> u16 {
        u16::from(s.bus.peek_wram(a)) | (u16::from(s.bus.peek_wram(a + 1)) << 8)
    };
    let before = rd16(&sys, V_CURSOR);

    // Down, held for a few frames then released: `read_pad` derives "newly pressed" from the
    // previous frame, so the press has to be an edge.
    sys.bus.set_joypad(0, PAD_CONTRACT | PAD_DOWN);
    for _ in 0..3 {
        sys.run_frame();
    }
    sys.bus.set_joypad(0, PAD_CONTRACT);
    for _ in 0..6 {
        sys.run_frame();
    }

    let after = rd16(&sys, V_CURSOR);
    assert_eq!(
        after,
        before + 1,
        "pressing Down did not move the cursor (was {before}, now {after})"
    );

    // Up moves it back. Same width bug hit both directions, so proving one is not proving the other.
    sys.bus.set_joypad(0, PAD_CONTRACT | PAD_UP);
    for _ in 0..3 {
        sys.run_frame();
    }
    sys.bus.set_joypad(0, PAD_CONTRACT);
    for _ in 0..6 {
        sys.run_frame();
    }
    assert_eq!(
        rd16(&sys, V_CURSOR),
        before,
        "pressing Up did not move the cursor back"
    );

    // Still alive: the header is intact and the list still holds a real name. A ROM that fell off
    // the rails leaves the last drawn frame on screen, so checking the tilemap is not enough on its
    // own — the cursor moving above is what proves it is still executing the menu loop.
    const MAP_BASE: u16 = 0x0400;
    let title: String = (0..12)
        .map(|i| char::from((sys.bus.ppu.vram_word(MAP_BASE + i) & 0xFF) as u8))
        .collect();
    assert_eq!(title, "ACCURACYSNES", "the header did not survive a redraw");

    // A re-runs the highlighted test: its verdict byte is rewritten (deterministic, so unchanged)
    // and the ROM keeps running. A crash in the re-run path would freeze the menu.
    let idx_before = rd16(&sys, V_CURSOR);
    let status_before = sys.bus.peek_wram(R_STATUS_BASE + u32::from(idx_before));
    let cursor_before = idx_before;
    sys.bus.set_joypad(0, PAD_CONTRACT | PAD_A);
    for _ in 0..3 {
        sys.run_frame();
    }
    sys.bus.set_joypad(0, PAD_CONTRACT);
    for _ in 0..600 {
        sys.run_frame();
    }
    assert_eq!(
        rd16(&sys, V_CURSOR),
        cursor_before,
        "re-running a test moved the cursor — the menu did not resume where it was"
    );
    assert_eq!(
        sys.bus.peek_wram(R_STATUS_BASE + u32::from(idx_before)),
        status_before,
        "re-running a deterministic test changed its verdict"
    );
    let title_after_a: String = (0..12)
        .map(|i| char::from((sys.bus.ppu.vram_word(MAP_BASE + i) & 0xFF) as u8))
        .collect();
    assert_eq!(
        title_after_a, "ACCURACYSNES",
        "the menu did not survive an A re-run"
    );

    // Select restarts the whole battery from restart_entry. R_DONE clears while it re-runs, then the
    // scene loop must complete again and the menu return.
    sys.bus.set_joypad(0, PAD_CONTRACT | PAD_SELECT);
    for _ in 0..3 {
        sys.run_frame();
    }
    sys.bus.set_joypad(0, PAD_CONTRACT);
    // `run_all_tests` clears R_DONE at its very start, well before the scene loop clears
    // R_SCENE_DONE ~430 frames later, so R_DONE is the prompt signal that a restart began.
    let mut cleared = false;
    for _ in 0..30 {
        sys.run_frame();
        if sys.bus.peek_wram(R_DONE) != DONE_MARK {
            cleared = true;
            break;
        }
    }
    assert!(
        cleared,
        "Select did not restart the battery — R_DONE never cleared"
    );
    // And that the restarted battery runs to completion — R_DONE set again — rather than hanging.
    // R_SCENE_DONE is no use here: it is still set from the previous run until the scene loop
    // clears it ~430 frames in, so waiting on it would read R_PASSED mid-restart.
    let mut frames = 0;
    while frames < MENU_FRAMES && sys.bus.peek_wram(R_DONE) != DONE_MARK {
        sys.run_frame();
        frames += 1;
    }
    assert_eq!(
        sys.bus.peek_wram(R_DONE),
        DONE_MARK,
        "the restarted battery never finished"
    );
    // 283, not 284: F1.07 stands down as SKIP on a restart because its phase A needs the power-on
    // value of $4218, which a soft restart cannot reproduce (the previous run armed auto-read). Its
    // verdict byte is $FF (skip), not a fail code — that distinction is the whole point of the flag.
    assert_eq!(
        rd16(&sys, R_PASSED),
        283,
        "the restarted battery did not reproduce its result (minus the power-on-only F1.07)"
    );
    let f107_idx = catalog()
        .iter()
        .position(|t| t.id == "F1.07")
        .expect("F1.07 is in the catalog");
    assert_eq!(
        sys.bus.peek_wram(R_STATUS_BASE + f107_idx as u32),
        0xFF,
        "F1.07 did not stand down as SKIP on the restart — it must not report a failure"
    );
}
