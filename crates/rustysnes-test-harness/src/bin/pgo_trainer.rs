//! PGO training-workload binary (`v1.19.0 "Afterburner"`) — build this with `cargo pgo build`,
//! run it, and its `.profraw` samples feed `cargo pgo optimize build`'s merged profile for the
//! shipping `rustysnes` binary. See `scripts/pgo/run.sh` and `.github/workflows/pgo.yml`.
//!
//! Reuses the same `System` API as `crates/rustysnes-core/benches/headless_frame.rs`
//! (`System::new` → `bus.cart = Some(...)` → `reset()` → `run_frame()` loop), just run at full
//! native speed over many frames across a handful of committed ROMs instead of one
//! Criterion-timed ROM — broader control-flow coverage (HDMA/PPU-timing-glitch paths, a
//! CPU-instruction-heavy suite) than a single steady-state ROM would exercise alone.
#![allow(missing_docs)] // small standalone training binary, not a library API surface.

use std::env;
use std::hint::black_box;
use std::path::{Path, PathBuf};

use rustysnes_core::System;
use rustysnes_core::cart::Cart;

// Committed, permissively-licensed (MIT/Zlib) ROMs only — see `tests/roms/README.md`. Gitignored
// `external/`/commercial corpora are never present on a CI runner, so training is deliberately
// scoped to what's always there. Chosen for breadth: `gilyon/cputest-full` is CPU-instruction
// heavy; the `undisbeliever` HDMA-glitch and INIDISP-hammer ROMs exercise PPU/DMA timing edge
// cases the single steady-state `headless_frame` bench ROM doesn't touch.
const TRAINING_ROMS: &[&str] = &[
    "tests/roms/gilyon/cputest/cputest-full.sfc",
    "tests/roms/undisbeliever/hdma-2100-glitch.sfc",
    "tests/roms/undisbeliever/hdma-21ff-2100-0f-glitch.sfc",
    "tests/roms/undisbeliever/inidisp_hammer_0f8f.sfc",
    "tests/roms/undisbeliever/inidisp_enable_display_mid_frame.sfc",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn train_one(path: &Path, frames: u64) {
    let rom =
        std::fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let cart =
        Cart::from_rom(&rom).unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..frames {
        sys.run_frame();
        black_box(sys.bus.framebuffer());
    }
    println!("pgo_trainer: trained {frames} frames on {}", path.display());
}

fn main() {
    let frames: u64 = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);

    let root = workspace_root();
    for rel in TRAINING_ROMS {
        train_one(&root.join(rel), frames);
    }
    println!(
        "pgo_trainer: done ({} ROM(s) x {frames} frames)",
        TRAINING_ROMS.len()
    );
}
