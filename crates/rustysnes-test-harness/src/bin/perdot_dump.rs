//! Per-dot compositor cross-check dumper (T-CA-10, `docs/adr/0014`).
//!
//! Renders one ROM through RustySNES for a fixed number of frames and prints its framebuffer as a
//! canonical `0RRRRRGGGGGBBBBB` distinct-color histogram, so `scripts/perdot_crossval.sh` can compare
//! it against the same ROM rendered in MesenCE (`scripts/perdot_capture.lua`). Build it
//! `--features per-dot-compositor` to exercise the accurate per-dot path (the harness feature
//! propagates down to `rustysnes-core/per-dot-compositor`); without the feature it renders the batch
//! path, which is how the harness reports the flag-OFF baseline.
//!
//! Usage: `cargo run -q -p rustysnes-test-harness --features per-dot-compositor --bin perdot_dump --
//! <rom.sfc> [frames]` (frames default 60).
//!
//! Output (one line, parseable by the shell): `PERDOT distinct=<n> colors=<hhhh:count,...>` sorted by
//! canonical value. The distinct-color SET + counts is the robust cross-emulator signal — it is
//! immune to the ~7-row overscan offset between RustySNES (composites from scanline 0) and MesenCE
//! (renders into a 239-row buffer with blank top rows).
#![allow(missing_docs)] // small standalone cross-check binary, not a library API surface.

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
    let frames: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(60);

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
