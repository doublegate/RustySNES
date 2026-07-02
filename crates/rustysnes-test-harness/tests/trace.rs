#![allow(missing_docs)]
#![cfg(feature = "test-roms")]
use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use std::fs;

#[test]
fn trace_secret_of_mana() {
    // Commercial ROMs are gitignored (never committed), so self-skip when absent — CI runs with
    // `--features test-roms` but without the `external/` corpus, and must stay green.
    let Ok(rom) = fs::read("../../tests/roms/external/commercial/HiRom/None/Secret of Mana.sfc")
    else {
        eprintln!("commercial ROM absent; skipping trace_secret_of_mana");
        return;
    };
    let cart = Cart::from_rom(&rom).unwrap();
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);

    for i in 0..6000 {
        sys.run_frame();
        if i % 100 == 0 {
            println!(
                "Frame {}: PC at {:02X}:{:04X}",
                i, sys.cpu.regs.pbr, sys.cpu.regs.pc
            );
        }
    }
}
