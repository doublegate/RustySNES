//! THROWAWAY variant NEC-DSP before/after hash harness (works on main AND the pin-exact branch):
//! boots DSP-2/DSP-4/ST010 (+ DSP-1 control) and hashes framebuffers at fixed frames, to confirm the
//! master-clock-stepped (pin-exact) model renders each byte-identically to the value-exact baseline.
#![cfg(feature = "commercial-roms")]
#![allow(clippy::all, clippy::pedantic, clippy::nursery)]
use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use std::path::PathBuf;

fn ext() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external")
}
fn firmware(names: &[&str]) -> Option<Vec<u8>> {
    for n in names {
        if let Ok(b) = std::fs::read(ext().join("firmware").join(n)) {
            return Some(b);
        }
    }
    None
}
fn hash_fb(fb: &[u16]) -> u64 {
    fb.iter().fold(1469598103934665603u64, |h, &p| {
        (h ^ u64::from(p)).wrapping_mul(1099511628211)
    })
}

#[test]
fn diag_necdsp_variant_hashes() {
    let cases: &[(&str, &[&str])] = &[
        (
            "commercial/LoRom/DSP-1/Pilotwings.sfc",
            &["dsp1b.rom", "dsp1.rom"],
        ),
        ("commercial/LoRom/DSP-2/Dungeon Master.sfc", &["dsp2.rom"]),
        ("commercial/LoRom/DSP-4/Top Gear 3000.sfc", &["dsp4.rom"]),
        (
            "commercial/LoRom/ST010/F1 ROC II - Race of Champions.sfc",
            &["st010.rom"],
        ),
    ];
    for (rel, fw) in cases {
        let Ok(rom) = std::fs::read(ext().join(rel)) else {
            eprintln!("SKIP {rel}: absent");
            continue;
        };
        let mut cart = Cart::from_rom(&rom).unwrap();
        if let Some(f) = firmware(fw) {
            cart.install_coprocessor_firmware(&f);
        }
        let mut sys = System::new(0);
        sys.bus.cart = Some(cart);
        sys.reset();
        for _ in 0..200 {
            sys.bus.set_joypad(0, 0);
            sys.run_frame();
        }
        let h200 = hash_fb(sys.bus.ppu.framebuffer());
        for _ in 0..100 {
            sys.run_frame();
        }
        let h300 = hash_fb(sys.bus.ppu.framebuffer());
        eprintln!("{rel}: h200={h200:#018x} h300={h300:#018x}");
    }
}
