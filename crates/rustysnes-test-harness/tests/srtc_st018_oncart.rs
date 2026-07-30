//! S-RTC (Daikaijuu Monogatari II) + ST018 (Nidan Morita Shogi 2) on-cart — detection + boot +
//! determinism. Both are `BestEffort`: their coprocessor's core function is usage-gated (the S-RTC
//! clock is read at specific moments; the ST018 shogi AI runs only on the computer's move — deep
//! gameplay a headless run doesn't reach), so this validates that the real cart **detects to the
//! right board, boots to real (non-blank) content, and is deterministic** (fail-closed: a present
//! cart that won't read/parse/install-firmware panics; only an absent dump self-skips) — not that
//! the coprocessor is exercised. That is the
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

/// Non-zero pixel count. A boot that reached real content (not a blank / stuck / crash screen) has a
/// substantial fraction of its 256x224 framebuffer non-zero: the SPC7110 frozen-boot case sat at
/// ~1.4% (883/61184), while both carts here clear 24% (S-RTC) / >99% (ST018) — so `len/12` (~8%)
/// cleanly separates "booted to a picture" from a stuck/blank screen.
fn nonzero(fb: &[u16]) -> usize {
    fb.iter().filter(|&&p| p != 0).count()
}

/// Boot `rom_rel` (installing `fw_name` if given) for `frames`, returning `(board_name, framebuffer)`.
///
/// **Fail-closed:** returns `None` (→ self-skip) ONLY when a required gitignored file is *absent*. A
/// present-but-unreadable/unparseable ROM, or a present firmware that won't install, panics — a
/// broken cart must fail the test, never silently skip it.
fn boot(rom_rel: &str, fw_name: Option<&str>, frames: u32) -> Option<(&'static str, Vec<u16>)> {
    let rom_path = ext().join(rom_rel);
    if !rom_path.exists() {
        return None;
    }
    let fw = match fw_name {
        Some(n) => {
            let p = ext().join("firmware").join(n);
            if !p.exists() {
                return None; // firmware is user-supplied + gitignored too
            }
            Some(std::fs::read(&p).expect("read present firmware"))
        }
        None => None,
    };
    let rom = std::fs::read(&rom_path).expect("read present ROM");
    let mut cart = Cart::from_rom(&rom).expect("present ROM must parse");
    let board = cart.board.name();
    if let Some(bytes) = &fw {
        assert!(
            cart.install_coprocessor_firmware(bytes),
            "{rom_rel}: present firmware must install"
        );
    }
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..frames {
        sys.run_frame();
    }
    Some((board, sys.bus.framebuffer().to_vec()))
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
    assert!(
        nonzero(&fb) > fb.len() / 12,
        "S-RTC must boot to real content, not a blank/stuck screen ({}/{} non-zero)",
        nonzero(&fb),
        fb.len()
    );
    let (_, fb2) = boot(rom, None, 200).unwrap();
    assert_eq!(
        fb, fb2,
        "S-RTC boot must be deterministic (full framebuffer)"
    );
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
    assert!(
        nonzero(&fb) > fb.len() / 12,
        "ST018 must boot to real content, not a blank/stuck screen ({}/{} non-zero)",
        nonzero(&fb),
        fb.len()
    );
    let (_, fb2) = boot(rom, Some("st018.rom"), 200).unwrap();
    assert_eq!(
        fb, fb2,
        "ST018 boot must be deterministic (full framebuffer)"
    );
}
