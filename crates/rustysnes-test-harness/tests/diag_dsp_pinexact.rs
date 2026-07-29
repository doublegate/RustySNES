//! THROWAWAY pin-exact DSP-1 in-game validation (Pilotwings flight; NOT to be committed).
//!
//! Boots Pilotwings through the FULL bus (so `coprocessor_tick`/`tick_master` free-runs the DSP),
//! navigates to flight, and checks the two Phase-A signals:
//!   1. the `read_dr` hybrid fallback fires ZERO times in flight (proof the tick RATE is right — the
//!      DSP produces each value before the game reads it, without any synchronous catch-up), and
//!   2. the Mode-7 floor is non-degenerate (the lower framebuffer has real per-line variety, not a
//!      flat single colour), plus a stable framebuffer hash for a before/after cross-branch compare.
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
/// Distinct pixel values in the lower ~2/3 of the frame (the Mode-7 floor region): a flat/degenerate
/// floor collapses to a handful; a real perspective floor has hundreds.
fn floor_variety(fb: &[u16]) -> usize {
    let w = 256usize;
    let start = w * 80; // below the horizon
    fb[start.min(fb.len())..]
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<u16>>()
        .len()
}

#[test]
fn diag_dsp_pinexact_flight() {
    let rel = "commercial/LoRom/DSP-1/Pilotwings.sfc";
    let Ok(rom) = std::fs::read(ext().join(rel)) else {
        eprintln!("SKIP: {rel} absent");
        return;
    };
    let mut cart = Cart::from_rom(&rom).unwrap();
    let Some(fw) = firmware() else {
        eprintln!("SKIP: no DSP-1 firmware");
        return;
    };
    assert!(cart.install_coprocessor_firmware(&fw), "firmware accepted");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();

    let mut flight_started = false;
    let mut flight_frame = 0u32;
    let mut prev_dsp = 0u64;
    let mut fires_at_flight = 0u64;
    let mut hash = 0u64;
    let mut variety = 0usize;
    let mut captured = 0u32;

    for f in 0..4000u32 {
        let input = if flight_started {
            0
        } else if f % 40 < 3 {
            0x1000 // START pulse
        } else if f % 40 < 8 {
            0x0080 // A
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
        let delta = dsp.saturating_sub(prev_dsp);
        prev_dsp = dsp;
        if !flight_started && delta > 500 {
            flight_started = true;
            flight_frame = f;
            fires_at_flight = sys
                .bus
                .cart
                .as_ref()
                .map_or(0, Cart::coprocessor_hybrid_fires);
            eprintln!(
                "flight detected at frame {f} (dsp_delta={delta}); hybrid_fires so far={fires_at_flight}"
            );
            continue;
        }
        if !flight_started {
            continue;
        }
        captured += 1;
        if captured == 120 {
            hash = hash_fb(sys.bus.ppu.framebuffer());
            variety = floor_variety(sys.bus.ppu.framebuffer());
        }
        if captured >= 150 {
            break;
        }
    }

    assert!(flight_started, "never reached flight");
    let fires_total = sys
        .bus
        .cart
        .as_ref()
        .map_or(0, Cart::coprocessor_hybrid_fires);
    let fires_in_flight = fires_total - fires_at_flight;
    eprintln!("=== PIN-EXACT DSP-1 FLIGHT VALIDATION ===");
    eprintln!("hybrid_fires in flight = {fires_in_flight} (MUST be 0 for pure pin-exact)");
    eprintln!("floor_variety@120 = {variety} distinct pixels (flat floor ~<=8; perspective ~100s)");
    eprintln!("framebuffer hash@120 = {hash:#018x}  (compare vs main/v1.22.0)");

    assert_eq!(
        fires_in_flight, 0,
        "read_dr hybrid fallback fired in flight — tick rate is wrong"
    );
    assert!(
        variety > 32,
        "flight floor is degenerate/flat (variety={variety}) — floor not ramping"
    );
}
