#![allow(missing_docs)]
//! The save-state round-trip determinism test — T-52-004, `docs/adr/0006`'s actual spec.
//!
//! Extends the existing determinism-contract test pattern (`docs/adr/0004`): boot a ROM, run N
//! frames, take a [`rustysnes_core::System::save_state`] snapshot, restore it onto a SEPARATE,
//! freshly-booted `System` (the same ROM loaded fresh, mirroring the "no ROM byte embedded"
//! contract every coprocessor firmware already follows), then run BOTH the original (continuing
//! uninterrupted) and the restored system for the same further N frames and assert the
//! framebuffer + queued audio samples are byte-identical between the two. A save-state that
//! silently dropped or corrupted any piece of state would show up here as a divergence, not just
//! a "did it error" check — this is the property every downstream Reach feature (rewind,
//! run-ahead, netplay rollback, TAS replay) actually depends on.
//!
//! Run across a representative sample, per ADR 0006's T-52-004 ticket:
//! - a `Core`/`Curated` coprocessor ROM (a free/CC0 Super FX Krom test ROM, `tests/roms/
//!   external/krom/CHIP/GSU/`, gitignored, self-skips when absent);
//! - a `BestEffort` coprocessor ROM (any commercial dump under `tests/roms/external/commercial/`
//!   whose header resolves a `BestEffort`-tier coprocessor — S-DD1 needs no firmware to run, so
//!   it's preferred when present; gitignored, self-skips when absent);
//! - a no-coprocessor ROM (the committed gilyon `cputest-basic.sfc`, always present, so this
//!   sample never self-skips even without the external corpus staged).
#![cfg(feature = "test-roms")]

use std::path::{Path, PathBuf};

use rustysnes_core::System;
use rustysnes_core::cart::Coprocessor;
use rustysnes_core::cart::tier::{BoardTier, board_tier};

/// Frames to run before taking the snapshot.
const FRAMES_BEFORE_SAVE: u32 = 30;
/// Frames to run on both systems after the snapshot/restore, before comparing.
const FRAMES_AFTER: u32 = 30;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn hash_audio(samples: &[(i16, i16)]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &(l, r) in samples {
        h ^= u64::from(l.cast_unsigned());
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= u64::from(r.cast_unsigned());
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Boot `rom` fresh (optionally installing `firmware`), run it for `frames` frames, and return
/// the booted `System`. `None` if the ROM fails to parse.
fn boot(rom: &[u8], firmware: Option<&[u8]>, frames: u32) -> Option<System> {
    let mut cart = rustysnes_core::cart::Cart::from_rom(rom).ok()?;
    if let Some(fw) = firmware {
        cart.install_coprocessor_firmware(fw);
    }
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..frames {
        sys.run_frame();
    }
    Some(sys)
}

/// The round-trip check: continuing the original system uninterrupted must produce the exact
/// same framebuffer + audio as saving, restoring onto a fresh system, and continuing that one.
fn check_round_trip(rom_path: &Path, firmware: Option<&[u8]>) -> Result<(), String> {
    let path = rom_path.display();
    let rom = std::fs::read(rom_path).map_err(|e| format!("read {path}: {e}"))?;

    let mut original =
        boot(&rom, firmware, FRAMES_BEFORE_SAVE).ok_or_else(|| format!("failed to boot {path}"))?;
    // The audio FIFO is host-side instrumentation, not core state (it's never written by
    // Dsp::save_state — see that method's own doc), so the boot phase's samples must be
    // discarded here rather than left to leak into the post-restore comparison below (the
    // `restored` system, booted with 0 frames, would never have accumulated them).
    let mut discard = Vec::new();
    original.bus.apu.drain_audio(&mut discard);
    let snapshot = original.save_state();

    // A separate System, the same ROM loaded fresh (never from the save-state's own bytes — a
    // save-state never embeds a ROM byte, docs/adr/0003), then restored from the snapshot.
    let mut restored =
        boot(&rom, firmware, 0).ok_or_else(|| format!("failed to re-boot {path}"))?;
    restored
        .load_state(&snapshot)
        .map_err(|e| format!("{path}: load_state failed: {e}"))?;

    // Drain every frame (not just at the end): 30 frames of undrained audio can overflow the
    // APU's bounded output FIFO (`AUDIO_FIFO_CAP`), silently dropping samples and making the
    // comparison miss a real divergence.
    let mut audio_orig = Vec::new();
    let mut audio_rest = Vec::new();
    for _ in 0..FRAMES_AFTER {
        original.run_frame();
        original.bus.apu.drain_audio(&mut audio_orig);
        restored.run_frame();
        restored.bus.apu.drain_audio(&mut audio_rest);
    }

    let fb_orig = original.bus.framebuffer();
    let fb_rest = restored.bus.framebuffer();
    if fb_orig != fb_rest {
        return Err(format!(
            "{path}: framebuffer diverged after restore (orig={:#018x} restored={:#018x})",
            hash_fb(fb_orig),
            hash_fb(fb_rest),
        ));
    }
    if audio_orig != audio_rest {
        return Err(format!(
            "{path}: audio diverged after restore (orig={:#018x} restored={:#018x}, \
             {} vs {} samples)",
            hash_audio(&audio_orig),
            hash_audio(&audio_rest),
            audio_orig.len(),
            audio_rest.len(),
        ));
    }
    Ok(())
}

/// Recursively collect every `*.sfc` under `dir`, sorted for determinism.
fn collect_roms(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sfc")) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn no_coprocessor_rom_round_trips() {
    // The committed gilyon cputest ROM — no coprocessor, always present (never self-skips).
    let path = workspace_root().join("tests/roms/gilyon/cputest/cputest-basic.sfc");
    check_round_trip(&path, None).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn core_or_curated_coprocessor_rom_round_trips() {
    // A free/CC0 Super FX (Curated-tier) Krom test ROM.
    let dir = workspace_root().join("tests/roms/external/krom/CHIP/GSU");
    let roms = collect_roms(&dir);
    let Some(path) = roms.first() else {
        eprintln!("SKIP core_or_curated_coprocessor_rom_round_trips: no GSU .sfc ROMs found");
        return;
    };
    check_round_trip(path, None).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn best_effort_coprocessor_rom_round_trips() {
    // Any locally-staged commercial dump whose header resolves a BestEffort-tier coprocessor.
    // S-DD1 needs no firmware dump to run, so it's the natural pick when present.
    let dir = workspace_root().join("tests/roms/external/commercial");
    if !dir.is_dir() {
        eprintln!("SKIP best_effort_coprocessor_rom_round_trips: commercial corpus absent");
        return;
    }
    let roms = collect_roms(&dir);
    // Any firmware-free BestEffort ROM works (CX4 needs cx4.rom to do anything observable;
    // S-DD1/SPC7110/OBC1 run firmware-free), but prefer S-DD1 specifically when one exists,
    // rather than whichever firmware-free BestEffort title happens to sort first.
    let mut fallback = None;
    let mut picked = None;
    for path in &roms {
        let Ok(rom) = std::fs::read(path) else {
            continue;
        };
        let Ok(cart) = rustysnes_core::cart::Cart::from_rom(&rom) else {
            continue;
        };
        if board_tier(cart.header.coprocessor) == BoardTier::BestEffort
            && cart.header.coprocessor != Coprocessor::Cx4
        {
            if cart.header.coprocessor == Coprocessor::SDd1 {
                picked = Some(path.clone());
                break;
            }
            fallback.get_or_insert_with(|| path.clone());
        }
    }
    let Some(path) = picked.or(fallback) else {
        eprintln!(
            "SKIP best_effort_coprocessor_rom_round_trips: no firmware-free BestEffort ROM found \
             in the local commercial corpus"
        );
        return;
    };
    check_round_trip(&path, None).unwrap_or_else(|e| panic!("{e}"));
}
