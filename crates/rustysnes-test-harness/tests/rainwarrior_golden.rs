#![allow(missing_docs)]
//! rainwarrior (Brad Smith / bbbradsmith) SNES test ROMs — deterministic gates.
//!
//! Homebrew test ROMs from <https://github.com/bbbradsmith/SNES_stuff>, in two gate styles, covering
//! PPU and input paths the AccuracySNES suites cover least:
//!
//! - **Framebuffer goldens** (`rainwarrior_framebuffers_match_golden`): boot on a real
//!   `rustysnes_core::System`, run 60 frames with a fixed per-ROM input, FNV-1a-hash the PPU
//!   framebuffer, and compare against `tests/golden/rainwarrior-framebuffer.tsv`.
//!   - `twoship` (Mode 5 512-px hi-res) / `elasticity` (per-scanline high-colour) — no input;
//!     cross-validated vs MesenCE (`twoship` 26/26 colors exact; `elasticity` +2 MesenCE-brightness
//!     colors — `docs/adr/0013`, `scripts/perdot_crossval.sh`).
//!   - `ctrltest`/`ctrltest_auto`/`ctrltest_simple` — `PAD_CONTRACT` held on both ports; exercises the
//!     `$4016`/`$4017` manual + `$4218`-`$421F` auto-read display path (independently reference-
//!     cross-validated by AccuracySNES Group F).
//!   - `mset` — a Mouse connected on port 2 and driven; exercises the 32-bit mouse-report read path.
//! - **Self-scoring** (`rainwarrior_multest_runs_without_failure`): `multest_mul16`/`multest_div16`
//!   sweep value pairs through the SNES hardware multiply/divide unit and **halt with a failure
//!   message on the first wrong result** (a full run is ~100h). The gate runs a sample and asserts
//!   the ROM never halts — a mul/div bug would stop it fast.
//!
//! The ROMs are in the **gitignored** `tests/roms/external/rainwarrior/` tier (no explicit upstream
//! license — usable locally, not redistributable; only derived hashes are committed, per
//! `docs/adr/0003`), so both tests **self-skip** when the dir is absent (CI, fresh clone).
#![cfg(feature = "test-roms")]

use std::collections::HashMap;
use std::path::PathBuf;

use rustysnes_core::controller::PortDevice;
use rustysnes_core::{System, cart::Cart};

/// Frames to run before hashing a framebuffer golden (matches the MesenCE cross-check `MCE_FRAMES`).
const FRAMES: u32 = 60;

/// Per-ROM input applied each frame so the hashed frame is deterministic.
#[derive(Clone, Copy)]
enum Input {
    /// No input driven (the hi-res / high-colour demos).
    None,
    /// `PAD_CONTRACT` held on ports 1/2 (AccuracySNES Group F masks).
    Pads(u16, u16),
    /// A Mouse connected on port 2, driven with a fixed relative delta + left button.
    Mouse2 { dx: i32, dy: i32 },
}

/// The framebuffer-golden ROMs and their inputs — pinned here (not inferred from the TSV) so the
/// gate cannot be silently narrowed by editing the golden.
const FB_ROMS: [(&str, Input); 6] = [
    ("twoship", Input::None),
    ("elasticity", Input::None),
    ("ctrltest", Input::Pads(0x9050, 0x60A0)),
    ("ctrltest_auto", Input::Pads(0x9050, 0x60A0)),
    ("ctrltest_simple", Input::Pads(0x9050, 0x60A0)),
    ("mset", Input::Mouse2 { dx: 5, dy: -3 }),
];

/// The self-scoring multiply/divide ROMs.
const MULTEST_ROMS: [&str; 2] = ["multest_mul16", "multest_div16"];

/// `multest` sample length (~10 s per ROM) — enough to exercise many value pairs; a bug halts sooner.
const MULTEST_SAMPLE_FRAMES: u32 = 600;

/// A `multest` "halt" is the PC pinned to one address across this many consecutive frame boundaries;
/// a running sweep never stays put (its combination loop moves the PC every frame).
const MULTEST_HALT_FRAMES: u32 = 120;

fn roms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/rainwarrior")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/rainwarrior-framebuffer.tsv")
}

/// FNV-1a over the 15-bit-per-pixel framebuffer (same visual-golden hash as `undisbeliever_golden`).
fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn load_system(name: &str) -> Option<System> {
    let rom = std::fs::read(roms_dir().join(format!("{name}.sfc"))).ok()?;
    let mut sys = System::new(0);
    sys.bus.cart = Some(Cart::from_rom(&rom).ok()?);
    sys.reset();
    Some(sys)
}

fn boot_and_hash(name: &str, input: Input) -> Option<u64> {
    let mut sys = load_system(name)?;
    if let Input::Mouse2 { .. } = input {
        sys.bus.set_port_device(1, PortDevice::Mouse);
    }
    for _ in 0..FRAMES {
        match input {
            Input::None => {}
            Input::Pads(p1, p2) => {
                sys.bus.set_joypad(0, p1);
                sys.bus.set_joypad(1, p2);
            }
            Input::Mouse2 { dx, dy } => sys.bus.set_mouse(1, dx, dy, true, false),
        }
        sys.run_frame();
    }
    Some(hash_fb(sys.bus.framebuffer()))
}

fn load_golden() -> HashMap<String, u64> {
    let text = std::fs::read_to_string(golden_path()).expect("read rainwarrior golden TSV");
    let mut map = HashMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, hash) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("golden line {}: not `name<TAB>hash`: {line:?}", n + 1));
        let name = name.trim().to_string();
        let hash = u64::from_str_radix(hash.trim(), 16)
            .unwrap_or_else(|_| panic!("golden line {}: hash not hex: {hash:?}", n + 1));
        assert!(
            map.insert(name.clone(), hash).is_none(),
            "golden line {}: duplicate baseline for {name:?}",
            n + 1
        );
    }
    map
}

#[test]
fn rainwarrior_framebuffers_match_golden() {
    if !roms_dir().is_dir() {
        eprintln!("SKIP rainwarrior_golden: ROM dir absent (gitignored external tier)");
        return;
    }
    let golden = load_golden();
    // The golden must name EXACTLY the framebuffer-ROM set — no silent narrowing, no stray extras.
    let golden_names: std::collections::HashSet<&str> = golden.keys().map(String::as_str).collect();
    let required: std::collections::HashSet<&str> = FB_ROMS.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        golden_names, required,
        "golden TSV must pin exactly the framebuffer ROMs {required:?}, found {golden_names:?}"
    );

    let mut mismatches = Vec::new();
    for &(name, input) in &FB_ROMS {
        let Some(got) = boot_and_hash(name, input) else {
            mismatches.push(format!("{name}: absent or failed to boot/hash"));
            continue;
        };
        // Determinism: a second run with the same input must produce the identical hash.
        let again = boot_and_hash(name, input).unwrap_or(0);
        assert_eq!(
            got, again,
            "{name}: framebuffer is NON-deterministic across runs"
        );

        let exp = golden[name];
        if exp != got {
            mismatches.push(format!("{name}: got {got:#018x} expected {exp:#018x}"));
        }
    }

    assert!(
        mismatches.is_empty(),
        "rainwarrior framebuffer golden mismatches:\n  {}",
        mismatches.join("\n  ")
    );
    eprintln!(
        "rainwarrior_golden: all {} framebuffer(s) matched",
        FB_ROMS.len()
    );
}

/// `multest_mul16` / `multest_div16` sweep the SNES hardware multiply/divide unit and halt with a
/// failure message on the first wrong result. Run a sample of frames and require the ROM to keep
/// running — a halt (PC pinned for a long stretch) means it found a mul/div result mismatch.
#[test]
fn rainwarrior_multest_runs_without_failure() {
    if !roms_dir().is_dir() {
        eprintln!("SKIP rainwarrior_multest: ROM dir absent (gitignored external tier)");
        return;
    }
    // Check every ROM before reporting so one halting ROM doesn't hide the other's result.
    let mut checked = 0u32;
    let mut failures = Vec::new();
    for name in MULTEST_ROMS {
        let Some(mut sys) = load_system(name) else {
            eprintln!("multest {name}: absent — skipping this ROM");
            continue;
        };
        checked += 1;
        let mut last_pc = u16::MAX;
        let mut stable = 0u32;
        for _ in 0..MULTEST_SAMPLE_FRAMES {
            sys.run_frame();
            let pc = sys.cpu.regs.pc;
            if pc == last_pc {
                stable += 1;
                if stable >= MULTEST_HALT_FRAMES {
                    failures.push(format!(
                        "{name}: HALTED at {:02X}:{:04X} within {MULTEST_SAMPLE_FRAMES} frames \
                         (hardware multiply/divide sweep found a wrong result)",
                        sys.cpu.regs.pbr, sys.cpu.regs.pc
                    ));
                    break;
                }
            } else {
                stable = 0;
            }
            last_pc = pc;
        }
    }
    assert!(
        failures.is_empty(),
        "multest mul/div accuracy regression:\n  {}",
        failures.join("\n  ")
    );
    assert!(checked > 0, "no multest ROMs were present to check");
    eprintln!("rainwarrior_multest: {checked} ROM(s) swept without a mul/div failure");
}
