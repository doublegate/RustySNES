#![allow(missing_docs)]
#![cfg(feature = "test-roms")]
use std::fs;
use rustysnes_core::cart::Cart;

#[test]
fn parse_yoshi() {
    let rom = fs::read("../../tests/roms/external/commercial/LoRom/GSU-2/Doom.sfc").unwrap();
    let cart = Cart::from_rom(&rom).unwrap();
    let offset = cart.header.offset;
    println!("Doom Header: {:?}", cart.header);
    println!("RAM byte: 0x{:02X}", rom[offset + 0x18]);
}
