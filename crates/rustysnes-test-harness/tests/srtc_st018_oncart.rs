//! S-RTC (Daikaijuu Monogatari II) + ST018 (Nidan Morita Shogi 2) on-cart — detection + boot +
//! determinism. Both are `BestEffort`: their coprocessor's core function is usage-gated (the S-RTC
//! clock is read at specific moments; the ST018 shogi AI runs only on the computer's move — deep
//! gameplay a headless run doesn't reach), so this validates that the real cart **detects to the
//! right board, boots, and is deterministic** — not that the coprocessor is exercised. That is the
//! honest upgrade a real dump buys over the prior unit-test-only coverage (`docs/adr/0003`).
//!
//! (SPC7110 / Tengai Makyou Zero is deliberately NOT here: the available dump does not boot to
//! content — see `docs/STATUS.md`.)
//!
//! Self-skips when the gitignored dumps are absent.
#![cfg(feature = "test-roms")]
use rustysnes_core::{System, cart::Cart};
use std::path::PathBuf;

fn ext() -> PathBuf {
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

/// Boot `rom_rel` (installing `fw_name` if given) for `frames`; return `(board_name, fb_hash)`.
fn boot(rom_rel: &str, fw_name: Option<&str>, frames: u32) -> Option<(&'static str, u64)> {
    let rom = std::fs::read(ext().join(rom_rel)).ok()?;
    let mut cart = Cart::from_rom(&rom).ok()?;
    let board = cart.board.name();
    if let Some(n) = fw_name {
        let fw = std::fs::read(ext().join("firmware").join(n)).ok()?;
        assert!(
            cart.install_coprocessor_firmware(&fw),
            "{rom_rel}: must accept {n}"
        );
    }
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..frames {
        sys.run_frame();
    }
    Some((board, hash_fb(sys.bus.framebuffer())))
}

#[test]
fn srtc_detects_boots_deterministic() {
    let rom = "commercial/ExHiRom/S-RTC/Daikaijuu Monogatari II (Japan).sfc";
    let Some((board, fb)) = boot(rom, None, 200) else {
        eprintln!("SKIP srtc_oncart: Daikaijuu Monogatari II absent");
        return;
    };
    assert_eq!(
        board, "ExHiROM+S-RTC",
        "Daikaijuu Monogatari II must resolve to the S-RTC board"
    );
    let (_, fb2) = boot(rom, None, 200).unwrap();
    assert_eq!(fb, fb2, "S-RTC boot must be deterministic");
}

#[test]
fn st018_detects_boots_deterministic() {
    let rom = "commercial/LoRom/ST018/Hayazashi Nidan Morita Shougi 2 (Japan).sfc";
    let Some((board, fb)) = boot(rom, Some("st018.rom"), 200) else {
        eprintln!("SKIP st018_oncart: Nidan Morita Shogi 2 and/or st018.rom absent");
        return;
    };
    assert_eq!(
        board, "LoROM+ST018",
        "Nidan Morita Shogi 2 must resolve to the ST018 board"
    );
    let (_, fb2) = boot(rom, Some("st018.rom"), 200).unwrap();
    assert_eq!(fb, fb2, "ST018 boot must be deterministic");
}
