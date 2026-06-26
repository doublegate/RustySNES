//! End-to-end "is it playable" smoke test: drive a real staged commercial ROM through the SAME
//! [`EmuCore`] path the GUI uses (load → `run_frame` → framebuffer → audio) and assert the machine
//! actually boots — a structured (non-blank) picture AND a non-silent audio stream.
//!
//! Gated on the gitignored staging ROM, so it SKIPS cleanly on a checkout that has no ROMs (the
//! same posture as the test-harness ROM suites). Never commits ROM bytes.

#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use rustysnes_frontend::config::Region;
use rustysnes_frontend::emu::EmuCore;

/// Candidate staged ROMs (first found wins). Plain LoROM titles that need no coprocessor firmware.
const CANDIDATES: &[&str] = &[
    "../../tests/roms/external/commercial/LoRom/None/Super Mario World.sfc",
    "../../tests/roms/external/commercial/LoRom/None/Super Mario World (USA).sfc",
];

fn find_rom() -> Option<std::path::PathBuf> {
    // The test runs with CWD = the crate dir; also try the workspace-root-relative form.
    for c in CANDIDATES {
        let p = Path::new(c);
        if p.exists() {
            return Some(p.to_path_buf());
        }
        let alt = Path::new(c.trim_start_matches("../../"));
        if alt.exists() {
            return Some(alt.to_path_buf());
        }
    }
    None
}

#[test]
fn real_game_boots_with_picture_and_sound() {
    let Some(rom_path) = find_rom() else {
        eprintln!(
            "SKIP: no staged commercial ROM found (gitignored); skipping playable smoke test"
        );
        return;
    };

    let bytes = std::fs::read(&rom_path).expect("read staged ROM");
    let mut core = EmuCore::new(0, Region::Ntsc);
    core.load_rom(&bytes).expect("load ROM");
    assert!(core.rom_loaded(), "ROM should be marked loaded");
    assert!(
        !core.needs_firmware(),
        "the chosen smoke-test ROM must need no coprocessor firmware"
    );

    // Run ~2 seconds of emulation through the GUI's frame path and collect AV evidence.
    let mut audio_total = 0usize;
    let mut audio_nonsilent = false;
    for _ in 0..120 {
        core.run_frame();
        for &(l, r) in core.audio() {
            audio_total += 1;
            if l != 0 || r != 0 {
                audio_nonsilent = true;
            }
        }
    }

    // --- Video: the framebuffer must be structured, not a flat fill. ---
    let fb = core.framebuffer();
    let (w, h) = core.fb_dims();
    assert_eq!(
        fb.len(),
        (w * h * 4) as usize,
        "framebuffer sized to active mode"
    );
    let first = &fb[0..4];
    let distinct = fb.chunks_exact(4).any(|px| px != first);
    assert!(
        distinct,
        "framebuffer is a flat fill ({first:?}) after 120 frames — PPU produced no picture"
    );
    // And some non-black pixels (alpha is always 0xFF for rendered content).
    let lit = fb
        .chunks_exact(4)
        .any(|px| px[0] != 0 || px[1] != 0 || px[2] != 0);
    assert!(lit, "framebuffer is entirely black after 120 frames");

    // --- Audio: the S-DSP must have produced samples, and not pure silence. ---
    assert!(
        audio_total > 100,
        "expected a stream of S-DSP samples over 120 frames, got {audio_total}"
    );
    assert!(
        audio_nonsilent,
        "audio stream was pure silence over 120 frames ({audio_total} samples)"
    );

    eprintln!(
        "PLAYABLE: {} — {}x{} picture, {} audio samples ({} non-silent stream)",
        rom_path.display(),
        w,
        h,
        audio_total,
        if audio_nonsilent { "yes" } else { "no" }
    );
}
