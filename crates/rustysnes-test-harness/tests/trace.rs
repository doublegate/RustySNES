#![allow(missing_docs)]
#![cfg(feature = "test-roms")]
use std::fs;
use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use rustysnes_core::dma_bus::DmaBus;

#[test]
fn trace_secret_of_mana() {
    let rom = fs::read("../../tests/roms/external/commercial/HiRom/None/Secret of Mana.sfc").unwrap();
    let cart = Cart::from_rom(&rom).unwrap();
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);

    for i in 0..6000 {
        sys.run_frame();
        if i % 100 == 0 {
            println!("Frame {}: PC at {:02X}:{:04X}", 
                     i, sys.cpu.regs.pbr, sys.cpu.regs.pc);
        }
    }
}
