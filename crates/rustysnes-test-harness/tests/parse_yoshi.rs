#![allow(missing_docs)]
#![cfg(feature = "test-roms")]
use rustysnes_core::cart::Cart;
use std::fs;

#[test]
fn parse_yoshi() {
    // Commercial ROMs are gitignored (never committed), so self-skip when absent — CI runs with
    // `--features test-roms` but without the `external/` corpus, and must stay green.
    let Ok(rom) = fs::read("../../tests/roms/external/commercial/LoRom/GSU-2/Doom.sfc") else {
        eprintln!("commercial ROM absent; skipping parse_yoshi");
        return;
    };
    let cart = Cart::from_rom(&rom).unwrap();
    let offset = cart.header.offset;
    println!("Doom Header: {:?}", cart.header);
    println!("RAM byte: 0x{:02X}", rom[offset + 0x18]);
}
