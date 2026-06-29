#![allow(missing_docs)]
//! DSP-1 (`µPD77C25`) on-cart coprocessor — boot + determinism + firmware-differential gate.
//!
//! Boots the locally-staged DSP-1 commercial dumps on the full `rustysnes_core::System` with the
//! user-supplied `dsp1.rom` / `dsp1b.rom` firmware installed, and asserts three things:
//!
//! 1. **Detection + mapping + handshake** — the header resolves `Coprocessor::Dsp`, the firmware
//!    installs (a `Core/Curated` board, never silently degraded — `docs/adr/0003`), and the game
//!    actually talks to the chip: the DSP register window is hit (`host_accesses > 0`), which can
//!    only happen if the bus window is mapped right *and* the `µPD77C25` hands the RQM handshake
//!    back (otherwise the game wedges on its first poll and never issues a second access).
//! 2. **Determinism** — same seed + ROM + firmware ⇒ a bit-identical framebuffer across two runs
//!    (the determinism contract, `docs/adr/0004`), matched against a committed golden hash.
//! 3. **The coprocessor is live** — for the Mode-7 titles (Super Mario Kart, Aim for the Ace)
//!    booting the *same* ROM **without** the firmware yields a *different* framebuffer, proving the
//!    `µPD77C25` math observably drives rendering. (Pilotwings / Super Bases Loaded 2 gate the DSP
//!    behind gameplay, so their attract screens are firmware-independent — the access count, not a
//!    pixel diff, is their live-chip signal.) The gate asserts at least one title shows the diff.
//!
//! ROMs and firmware live under the gitignored `tests/roms/external/`; the test self-skips when
//! they are absent so CI without the local corpus stays green. Re-bless `dsp1-framebuffer.tsv`
//! with `BLESS_DSP1=1` when an intentional change lands.
#![cfg(feature = "test-roms")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rustysnes_core::cart::Coprocessor;
use rustysnes_core::{System, cart::Cart};

/// Frames to run before hashing — enough for the Mode-7 titles to reach their DSP-driven screen.
const FRAMES: u32 = 180;

fn external_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/dsp1-framebuffer.tsv")
}

/// The first available DSP-1 firmware dump (gitignored, user-supplied). Prefers `dsp1b` (the
/// revised firmware most games shipped with), falling back to `dsp1`.
fn firmware() -> Option<Vec<u8>> {
    let dir = external_dir().join("firmware");
    for name in ["dsp1b.rom", "dsp1.rom"] {
        if let Ok(bytes) = std::fs::read(dir.join(name)) {
            return Some(bytes);
        }
    }
    None
}

/// The staged DSP-1 commercial dumps (`<dir>, <file>`), relative to `external/commercial`.
const ROMS: &[(&str, &str)] = &[
    ("LoRom/DSP-1", "Pilotwings.sfc"),
    ("LoRom/DSP-1", "Super Bases Loaded 2.sfc"),
    ("HiRom/DSP-1", "Super Mario Kart.sfc"),
    ("HiRom/DSP-1", "Ace wo Nerae_ _Aim for the Ace__.sfc"),
];

fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A boot result: the detected coprocessor, the framebuffer hash, and the DSP host-access count.
struct Boot {
    copro: Coprocessor,
    fb_hash: u64,
    accesses: u64,
}

/// Boot a ROM (optionally installing `fw`) for [`FRAMES`] frames.
fn boot(path: &Path, fw: Option<&[u8]>) -> Option<Boot> {
    let rom = std::fs::read(path).ok()?;
    let mut cart = Cart::from_rom(&rom).ok()?;
    let copro = cart.header.coprocessor;
    if let Some(bytes) = fw {
        assert!(
            cart.install_coprocessor_firmware(bytes),
            "DSP-1 cart must accept the firmware dump"
        );
    }
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..FRAMES {
        sys.run_frame();
    }
    let accesses = sys
        .bus
        .cart
        .as_ref()
        .map_or(0, |c| c.board.coprocessor_host_accesses());
    Some(Boot {
        copro,
        fb_hash: hash_fb(sys.bus.framebuffer()),
        accesses,
    })
}

fn load_golden() -> HashMap<String, u64> {
    std::fs::read_to_string(golden_path())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('\t')?;
            Some((
                k.to_string(),
                u64::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok()?,
            ))
        })
        .collect()
}

#[test]
fn dsp1_boots_deterministic_and_firmware_drives_picture() {
    let commercial = external_dir().join("commercial");
    let Some(fw) = firmware() else {
        eprintln!("SKIP dsp1_oncart: firmware (tests/roms/external/firmware/dsp1*.rom) absent");
        return;
    };
    if !commercial.is_dir() {
        eprintln!("SKIP dsp1_oncart: commercial corpus absent");
        return;
    }

    let golden = load_golden();
    let bless = std::env::var("BLESS_DSP1").is_ok();
    let mut blessed = Vec::new();
    let mut mismatches = Vec::new();
    let mut checked = 0u32;
    let mut booted = 0u32;
    let mut accesses: HashMap<&str, u64> = HashMap::new();

    for (dir, file) in ROMS {
        let path = commercial.join(dir).join(file);
        if !path.is_file() {
            eprintln!("skip (absent): {dir}/{file}");
            continue;
        }
        let stem = Path::new(file).file_stem().unwrap().to_str().unwrap();

        let Some(with_fw) = boot(&path, Some(&fw)) else {
            mismatches.push(format!("{stem}: failed to boot"));
            continue;
        };
        booted += 1;
        accesses.insert(stem, with_fw.accesses);
        assert_eq!(
            with_fw.copro,
            Coprocessor::Dsp,
            "{stem}: header must detect DSP-1"
        );
        eprintln!("{stem}: copro=Dsp accesses={}", with_fw.accesses);

        if bless {
            blessed.push(format!("{stem}\t{:#018x}", with_fw.fb_hash));
            checked += 1;
            continue;
        }
        match golden.get(stem) {
            Some(&exp) if exp == with_fw.fb_hash => checked += 1,
            Some(&exp) => {
                mismatches.push(format!(
                    "{stem}: got {:#018x} expected {exp:#018x}",
                    with_fw.fb_hash
                ));
            }
            None => mismatches.push(format!(
                "{stem}: no golden entry (got {:#018x})",
                with_fw.fb_hash
            )),
        }
    }

    if booted > 0 && !bless {
        // Both the LoROM and HiROM DSP-1 bus windows must be proven live on a real game: the
        // game can only rack up host accesses if the window is mapped right *and* the `µPD77C25`
        // returns the RQM handshake (else it wedges on the first poll). Super Mario Kart (HiROM)
        // streams Mode-7 projection math every frame; Pilotwings (LoROM) probes the chip at boot.
        let smk = "Super Mario Kart";
        let pw = "Pilotwings";
        if let Some(&n) = accesses.get(smk) {
            assert!(
                n > 1000,
                "{smk}: only {n} DSP accesses — HiROM DSP-1 window not live"
            );

            // Determinism (the anchor title): a second run is bit-identical (`docs/adr/0004`).
            let again = boot(
                &commercial.join("HiRom/DSP-1").join("Super Mario Kart.sfc"),
                Some(&fw),
            )
            .expect("smk re-boot");
            let first = golden.get(smk).copied();
            assert!(
                first == Some(again.fb_hash) || first.is_none(),
                "{smk}: framebuffer NON-deterministic / golden drift"
            );

            // The live-chip pixel signal: the DSP-1 math observably changes the picture.
            let without_fw = boot(
                &commercial.join("HiRom/DSP-1").join("Super Mario Kart.sfc"),
                None,
            )
            .expect("smk no-fw boot");
            assert_ne!(
                again.fb_hash, without_fw.fb_hash,
                "{smk}: identical with/without DSP-1 firmware — the chip math is not reaching the \
                 framebuffer"
            );
        }
        if let Some(&n) = accesses.get(pw) {
            assert!(n > 0, "{pw}: no DSP accesses — LoROM DSP-1 window not live");
        }
    }

    if bless {
        blessed.sort();
        std::fs::write(golden_path(), format!("{}\n", blessed.join("\n"))).expect("write golden");
        eprintln!("BLESSED dsp1-framebuffer.tsv ({checked} entries)");
        return;
    }

    eprintln!("dsp1 golden: {checked} matched");
    assert!(
        mismatches.is_empty(),
        "DSP-1 boot/golden mismatches (re-bless with BLESS_DSP1=1 if intentional):\n{}",
        mismatches.join("\n")
    );
}
