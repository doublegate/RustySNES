//! THROWAWAY pure pin-exact DSP-1 in-game validation (Pilotwings flight; NOT to be committed).
//!
//! Boots Pilotwings through the FULL bus (so `coprocessor_tick`/`tick_master` free-runs the DSP with
//! NO host-access catch-up), navigates to flight, and asserts the Mode-7 floor is non-degenerate and
//! the framebuffer hash matches the v1.22.0 value-exact baseline (0x5dba62c02af9a44a) byte-for-byte.
//! `cargo test -p rustysnes-test-harness --features "test-roms commercial-roms" --test diag_dsp_pinexact -- --nocapture`
#![cfg(feature = "commercial-roms")]
#![allow(clippy::all, clippy::pedantic, clippy::nursery)] // throwaway diagnostic harness
use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use std::path::PathBuf;

fn ext() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external")
}
fn firmware() -> Option<Vec<u8>> {
    for n in ["dsp1b.rom", "dsp1.rom"] {
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
fn floor_variety(fb: &[u16]) -> usize {
    fb[(256 * 80).min(fb.len())..]
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<u16>>()
        .len()
}

#[test]
fn diag_dsp_pinexact_flight() {
    let Ok(rom) = std::fs::read(ext().join("commercial/LoRom/DSP-1/Pilotwings.sfc")) else {
        eprintln!("SKIP: absent");
        return;
    };
    let mut cart = Cart::from_rom(&rom).unwrap();
    let Some(fw) = firmware() else {
        eprintln!("SKIP: no firmware");
        return;
    };
    assert!(cart.install_coprocessor_firmware(&fw));
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();

    let (mut flight, mut prev, mut cap) = (false, 0u64, 0u32);
    let (mut hash, mut variety) = (0u64, 0usize);
    for f in 0..4000u32 {
        let input = if flight {
            0
        } else if f % 40 < 3 {
            0x1000
        } else if f % 40 < 8 {
            0x0080
        } else {
            0
        };
        sys.bus.set_joypad(0, input);
        sys.bus.set_joypad(1, 0);
        sys.run_frame();
        let dsp = sys
            .bus
            .cart
            .as_ref()
            .map_or(0, Cart::coprocessor_host_accesses);
        let d = dsp.saturating_sub(prev);
        prev = dsp;
        if !flight && d > 500 {
            flight = true;
            eprintln!("flight at frame {f}");
            continue;
        }
        if !flight {
            continue;
        }
        cap += 1;
        if cap == 120 {
            hash = hash_fb(sys.bus.ppu.framebuffer());
            variety = floor_variety(sys.bus.ppu.framebuffer());
        }
        if cap >= 150 {
            break;
        }
    }

    assert!(flight, "never reached flight");
    eprintln!("=== PURE PIN-EXACT DSP-1 FLIGHT ===");
    eprintln!("hash@120 = {hash:#018x}  (v1.22.0 baseline = 0x5dba62c02af9a44a)");
    eprintln!("floor_variety@120 = {variety} (flat ~<=8; perspective ~100s)");
    assert!(
        variety > 32,
        "flight floor is degenerate/flat (variety={variety})"
    );
    assert_eq!(
        hash, 0x5dba62c02af9a44a,
        "pure pin-exact diverged from the v1.22.0 baseline"
    );
}
