//! RustySNES side of the `$213E` over-flag eval-line probe (`scripts/probes/eval-line-213e/`). Runs
//! the probe ROM and reports the first scanline whose range/time over-flag reads set, by reading the
//! WRAM array the ROM's per-scanline H-IRQ populates. MesenCE reads the same array via
//! `probe_mesen.lua`, so the eval-line offset is directly comparable. The per-dot PPU is the only
//! compositor, so this reports the per-dot eval line.
#![allow(missing_docs)] // small standalone probe binary, not a library API surface.
use std::fmt::Write as _;

use rustysnes_core::System;
use rustysnes_core::cart::Cart;

fn main() {
    let path = std::env::args().nth(1).expect("usage: probe_213e <rom>");
    let rom = std::fs::read(&path).expect("read rom");
    let cart = Cart::from_rom(&rom).expect("parse rom");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..12 {
        sys.run_frame();
    }
    let mut first_range = None;
    let mut first_time = None;
    let mut window = String::new();
    for s in 96u32..=112 {
        let v = sys.bus.peek_wram(0x7E_1000 + s);
        let _ = write!(window, "{s}:{v:02x} ");
        if v & 0x40 != 0 && first_range.is_none() {
            first_range = Some(s);
        }
        if v & 0x80 != 0 && first_time.is_none() {
            first_time = Some(s);
        }
    }
    println!("RUSTY range_over first-set scanline={first_range:?} time_over={first_time:?}");
    println!("RUSTY window {window}");
}
