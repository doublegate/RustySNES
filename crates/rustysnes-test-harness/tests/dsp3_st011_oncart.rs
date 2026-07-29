//! DSP-3 (`µPD7725`) + ST011 (`µPD96050`) on-cart coprocessors — boot + liveness + determinism.
//!
//! Boots the two locally-staged commercial dumps on the full `rustysnes_core::System` with the
//! user-supplied `dsp3.rom` / `st011.rom` firmware installed, and asserts, per chip:
//!
//! 1. **Detection + mapping** — the board resolves to the right variant (`LoROM+DSP-3` /
//!    `LoROM+ST011`), the firmware installs (`Core/Curated`-family, never silently degraded —
//!    `docs/adr/0003`), and the game actually reaches the chip's register window
//!    (`host_accesses > 0`). DSP-3 (SD Gundam GX) hits the chip immediately; ST011 (a shogi game)
//!    gates its AI behind menu input, so the run drives Start/A to reach it.
//! 2. **Determinism** — same seed + ROM + firmware + input ⇒ a bit-identical framebuffer across two
//!    runs (the hard AV contract, `docs/adr/0004`).
//!
//! ROMs + firmware live under the gitignored `tests/roms/external/`; the test self-skips when either
//! is absent, keeping fresh clones + CI green.
#![cfg(feature = "test-roms")]
use rustysnes_core::{System, cart::Cart};
use std::path::PathBuf;

fn external_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external")
}

fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Boot `rom_rel` with `fw_name` installed for `frames`. When `drive_input`, alternate Start / A /
/// release (40 frames each) to walk past a title/menu into gameplay. Returns `(board, accesses, fb)`.
fn boot(
    rom_rel: &str,
    fw_name: &str,
    frames: u32,
    drive_input: bool,
) -> Option<(&'static str, u64, u64)> {
    let rom = std::fs::read(external_dir().join(rom_rel)).ok()?;
    let fw = std::fs::read(external_dir().join("firmware").join(fw_name)).ok()?;
    let mut cart = Cart::from_rom(&rom).ok()?;
    let board = cart.board.name();
    assert!(
        cart.install_coprocessor_firmware(&fw),
        "{rom_rel}: board must accept the {fw_name} dump"
    );
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for f in 0..frames {
        if drive_input {
            let buttons: u16 = match (f / 40) % 3 {
                0 => 0x1000, // Start
                1 => 0x0080, // A
                _ => 0x0000, // release
            };
            sys.bus.set_joypad(0, buttons);
        }
        sys.run_frame();
    }
    let accesses = sys
        .bus
        .cart
        .as_ref()
        .map_or(0, |c| c.board.coprocessor_host_accesses());
    Some((board, accesses, hash_fb(sys.bus.framebuffer())))
}

#[test]
fn dsp3_boots_live_and_deterministic() {
    let rom = "commercial/LoRom/DSP-3/SD Gundam GX (Japan).sfc";
    let Some((board, accesses, fb)) = boot(rom, "dsp3.rom", 200, false) else {
        eprintln!("SKIP dsp3_oncart: SD Gundam GX and/or dsp3.rom absent");
        return;
    };
    assert_eq!(
        board, "LoROM+DSP-3",
        "SD Gundam GX must resolve to the DSP-3 board"
    );
    assert!(
        accesses > 0,
        "SD Gundam GX must reach the DSP-3 register window (host_accesses > 0); got {accesses}"
    );
    let (_, _, fb2) = boot(rom, "dsp3.rom", 200, false).unwrap();
    assert_eq!(fb, fb2, "DSP-3 boot must be deterministic");
}

#[test]
fn st011_boots_live_and_deterministic() {
    let rom = "commercial/LoRom/ST011/Hayazashi Nidan Morita Shougi (Japan).sfc";
    // ST011's shogi AI is behind menu input; ~700 driven frames reach it (empirically live by ~600).
    let Some((board, accesses, fb)) = boot(rom, "st011.rom", 700, true) else {
        eprintln!("SKIP st011_oncart: Morita Shougi and/or st011.rom absent");
        return;
    };
    assert_eq!(
        board, "LoROM+ST011",
        "Morita Shougi must resolve to the ST011 board"
    );
    assert!(
        accesses > 0,
        "Morita Shougi must reach the ST011 register window under input (host_accesses > 0); got {accesses}"
    );
    let (_, _, fb2) = boot(rom, "st011.rom", 700, true).unwrap();
    assert_eq!(
        fb, fb2,
        "ST011 boot must be deterministic under the same input"
    );
}
