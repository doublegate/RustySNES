//! RustySNES side of the `$213E` over-flag eval-line probe (`scripts/probes/eval-line-213e/`). Runs
//! the probe ROM and reports the first scanline whose range/time over-flag reads set, by reading the
//! WRAM array the ROM's per-scanline H-IRQ populates. MesenCE reads the same array via
//! `probe_mesen.lua`, so the eval-line offset is directly comparable. Build with or without
//! `--features per-dot-compositor` to compare the batch and per-dot paths.
#![allow(missing_docs)] // small standalone probe binary, not a library API surface.
use std::error::Error;
use std::fmt::Write as _;
use std::process::ExitCode;

use rustysnes_core::System;
use rustysnes_core::cart::Cart;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("probe_213e: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // The ROM path and its bytes are untrusted external input: propagate typed errors rather than
    // panic (the no-`expect`-on-external-input guideline applies to this non-test binary too).
    let path = std::env::args().nth(1).ok_or("usage: probe_213e <rom>")?;
    let rom = std::fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let cart =
        Cart::from_rom(&rom).map_err(|e| format!("{path} is not a valid cartridge: {e:?}"))?;
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
    Ok(())
}
