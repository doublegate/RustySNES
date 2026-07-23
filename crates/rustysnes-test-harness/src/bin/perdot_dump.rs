//! Per-dot compositor cross-check dumper (T-CA-10, `docs/adr/0014`).
//!
//! Renders one ROM through RustySNES for a fixed number of frames and prints its framebuffer as a
//! canonical `0RRRRRGGGGGBBBBB` distinct-color histogram, so `scripts/perdot_crossval.sh` can compare
//! it against the same ROM rendered in MesenCE (`scripts/perdot_capture.lua`). The per-dot PPU is the
//! only compositor (the batch path was removed), so this binary exercises it directly with no flag.
//!
//! Usage: `cargo run -q -p rustysnes-test-harness --bin perdot_dump -- <rom.sfc> [frames]`
//! (frames default 60).
//!
//! Output (one line, parseable by the shell): `PERDOT distinct=<n> colors=<hhhh:count,...>` sorted by
//! canonical value. The distinct-color SET + counts is the robust cross-emulator signal — it is
//! immune to the ~7-row overscan offset between RustySNES (composites from scanline 0) and MesenCE
//! (renders into a 239-row buffer with blank top rows).

use std::collections::BTreeMap;
use std::env;
use std::process::ExitCode;

use rustysnes_core::System;
use rustysnes_core::cart::Cart;

/// RustySNES stores 15-bit BGR555 (`B<<10 | G<<5 | R`). MesenCE / the libretro cross-val host use
/// canonical RGB555 (`R<<10 | G<<5 | B`). Swap the R and B fields so both sides hash the same value.
const fn canonical(px: u16) -> u16 {
    let r = px & 0x1f;
    let g = (px >> 5) & 0x1f;
    let b = (px >> 10) & 0x1f;
    (r << 10) | (g << 5) | b
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(rom_path) = args.next() else {
        eprintln!("usage: perdot_dump <rom.sfc> [frames]");
        return ExitCode::FAILURE;
    };
    // Frame count shares a positive-integer contract with the MesenCE side (`perdot_capture.lua`): a
    // zero/negative/malformed value would make the two renderers sample different frames. Default to 60
    // when omitted, but reject a supplied value that is not a positive integer instead of silently
    // falling back to 60.
    let frames: u32 = match args.next() {
        None => 60,
        Some(s) => match s.parse::<u32>() {
            Ok(n) if n > 0 => n,
            _ => {
                eprintln!("perdot_dump: frame count must be a positive integer, got {s:?}");
                return ExitCode::FAILURE;
            }
        },
    };

    let rom = match std::fs::read(&rom_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("perdot_dump: cannot read {rom_path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let cart = match Cart::from_rom(&rom) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("perdot_dump: {rom_path} is not a valid cartridge: {e:?}");
            return ExitCode::FAILURE;
        }
    };

    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..frames {
        sys.run_frame();
    }

    let mut hist: BTreeMap<u16, u32> = BTreeMap::new();
    for &px in sys.bus.framebuffer() {
        *hist.entry(canonical(px)).or_insert(0) += 1;
    }
    let colors = hist
        .iter()
        .map(|(c, n)| format!("{c:04x}:{n}"))
        .collect::<Vec<_>>()
        .join(",");
    println!("PERDOT distinct={} colors={}", hist.len(), colors);
    ExitCode::SUCCESS
}
