//! Commercial-ROM boot-screenshot generator — the RustySNES port of RustyNES's
//! `external_coverage` screenshot mechanism.
//!
//! Walks every locally-staged commercial dump under the gitignored
//! `tests/roms/external/commercial/<map>/<chip>/<game>.sfc`, boots each on the full
//! `rustysnes_core::System` (installing the matching coprocessor firmware when the cart needs
//! it), runs it to an attract/title frame, and — only when `RUSTYSNES_DUMP_FRAMES=1` — writes the
//! decoded framebuffer as a PPM under the dump dir, preserving the `<map>/<chip>/` structure so
//! `scripts/screenshots/generate.sh` can tier-split + convert to the committed PNGs.
//!
//! This is a SCREENSHOT GENERATOR, not a correctness gate: with no env var set it just boots each
//! ROM (a smoke net) and self-skips entirely when the corpus is absent (CI stays green). Only the
//! produced PNGs (`screenshots/`) are ever committed — never a ROM or firmware byte (ADR 0003).
//!
//! Usage:
//! ```text
//! RUSTYSNES_DUMP_FRAMES=1 [RUSTYSNES_SHOT_FRAMES=360] [RUSTYSNES_DUMP_DIR=/tmp/...] \
//!   cargo test -p rustysnes-test-harness --features test-roms --test commercial_screenshots -- --nocapture
//! ```
#![cfg(feature = "test-roms")]
// Screenshot generator: bounded framebuffer values narrowed to u8 (intentional), and the
// module doc lists bare env-var names / paths in a usage block.
#![allow(clippy::cast_possible_truncation, clippy::doc_markdown)]

use std::path::{Path, PathBuf};

use rustysnes_core::System;
use rustysnes_core::cart::{Cart, Coprocessor};

fn commercial_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/commercial")
}

fn firmware_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/firmware")
}

/// Firmware dumps to try, in order, for a cart's coprocessor (gitignored, user-supplied).
const fn firmware_candidates(co: Coprocessor) -> &'static [&'static str] {
    match co {
        Coprocessor::Dsp => &["dsp1b.rom", "dsp1.rom", "dsp2.rom", "dsp3.rom", "dsp4.rom"],
        Coprocessor::Cx4 => &["cx4.rom"],
        _ => &[],
    }
}

/// SNES 15-bit BGR555 -> 24-bit RGB (red in the low bits; the 5->8 expansion the frontend uses).
fn bgr555_to_rgb(px: u16) -> [u8; 3] {
    let r5 = u32::from(px & 0x1f);
    let g5 = u32::from((px >> 5) & 0x1f);
    let b5 = u32::from((px >> 10) & 0x1f);
    [
        ((r5 << 3) | (r5 >> 2)) as u8,
        ((g5 << 3) | (g5 >> 2)) as u8,
        ((b5 << 3) | (b5 >> 2)) as u8,
    ]
}

/// Collect every `<map>/<chip>/<game>.sfc` under the commercial dir (relative paths).
fn staged_roms(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for map in read_dirs(root) {
        for chip in read_dirs(&map) {
            for f in std::fs::read_dir(&chip).into_iter().flatten().flatten() {
                let p = f.path();
                if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sfc"))
                    && let Ok(rel) = p.strip_prefix(root)
                {
                    out.push(rel.to_path_buf());
                }
            }
        }
    }
    out.sort();
    out
}

fn read_dirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v
}

#[test]
fn generate_commercial_screenshots() {
    let root = commercial_dir();
    let roms = staged_roms(&root);
    if roms.is_empty() {
        eprintln!(
            "SKIP commercial_screenshots: {} empty (gitignored corpus)",
            root.display()
        );
        return;
    }
    let dump = std::env::var("RUSTYSNES_DUMP_FRAMES").is_ok();
    let frames: u32 = std::env::var("RUSTYSNES_SHOT_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
    let dump_dir = std::env::var("RUSTYSNES_DUMP_DIR")
        .unwrap_or_else(|_| "/tmp/rustysnes-screenshots".to_string());

    let (mut ok, mut shot, mut skipped) = (0u32, 0u32, 0u32);
    for rel in &roms {
        let abs = root.join(rel);
        let Ok(bytes) = std::fs::read(&abs) else {
            continue;
        };
        let Ok(mut cart) = Cart::from_rom(&bytes) else {
            skipped += 1;
            continue;
        };

        // Install the coprocessor firmware the cart needs (if available); else it boots inert.
        let co = cart.header.coprocessor;
        for fw in firmware_candidates(co) {
            if let Ok(f) = std::fs::read(firmware_dir().join(fw))
                && cart.install_coprocessor_firmware(&f)
            {
                break;
            }
        }

        let mut system = System::new(0);
        system.bus.cart = Some(cart);
        // Mirror RustyNES's RepeatStartTap: warm up, then tap Start (joypad bit 12) periodically
        // to advance past title/menu screens into the attract demo / first interactive frame, so
        // the captured screenshot is a meaningful screen rather than a boot/black transition.
        for f in 0..frames {
            let pad: u16 = if f >= 180 && f % 90 < 6 { 0x1000 } else { 0 };
            system.bus.set_joypad(0, pad);
            system.run_frame();
        }
        ok += 1;

        if dump {
            let h = u32::from(system.bus.ppu.visible_height()).min(239);
            let w = 256u32;
            let fb = system.bus.framebuffer();
            let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
            for &px in fb.iter().take((w * h) as usize) {
                ppm.extend_from_slice(&bgr555_to_rgb(px));
            }
            let out = Path::new(&dump_dir).join(rel.with_extension("ppm"));
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            if std::fs::write(&out, ppm).is_ok() {
                shot += 1;
            }
        }
    }
    eprintln!(
        "commercial_screenshots: {ok} booted, {shot} shots written to {dump_dir} ({skipped} unreadable headers){}",
        if dump {
            ""
        } else {
            "  [set RUSTYSNES_DUMP_FRAMES=1 to write PPMs]"
        }
    );
    assert!(ok > 0, "no commercial ROM booted");
}
