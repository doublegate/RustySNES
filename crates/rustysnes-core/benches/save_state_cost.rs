//! `System::save_state()`/`load_state()` cost across three board tiers (`v0.9.0 "Community"`,
//! T-82-001 — pre-work for rollback netplay, which calls save/restore far more often than
//! `RewindBuffer`'s ~10 Hz design point). Results feed `docs/benchmarks.md`.
//!
//! Run: `cargo bench -p rustysnes-core --bench save_state_cost`.
//!
//! The no-coprocessor tier uses the same committed `tests/roms/undisbeliever/
//! inidisp_hammer_0f00.sfc` `headless_frame.rs` does. The Curated (Super FX) and `BestEffort`
//! (CX4) tiers use real commercial ROMs from the gitignored `tests/roms/external/commercial/`
//! corpus (`docs/adr/0003`) — present on a dev machine that has run the coprocessor validation
//! suites, absent in CI/a fresh clone. Each of those two benchmarks self-skips (prints a message,
//! registers nothing) when its ROM is missing, exactly like `commercial_screenshots.rs`'s own
//! self-skip convention — never a hard failure over an intentionally-gitignored corpus.
#![allow(missing_docs)] // Criterion's macro-generated `main`/harness items have no doc comments.

use std::hint::black_box;
use std::path::{Path, PathBuf};

use criterion::{Criterion, criterion_group, criterion_main};
use rustysnes_core::System;
use rustysnes_core::cart::Cart;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn booted_system_from(rom_path: &Path) -> Option<System> {
    let rom = std::fs::read(rom_path).ok()?;
    let cart = Cart::from_rom(&rom).ok()?;
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    // Warm up past the ROM's own boot/init sequence so save/load measures steady-state size and
    // cost, not a cold-start-specific one.
    for _ in 0..16 {
        sys.run_frame();
    }
    Some(sys)
}

fn bench_save_load(c: &mut Criterion, group: &str, sys: &mut System) {
    c.bench_function(&format!("{group}_save_state"), |b| {
        b.iter(|| black_box(sys.save_state()));
    });
    let blob = sys.save_state();
    c.bench_function(&format!("{group}_load_state"), |b| {
        b.iter(|| {
            sys.load_state(black_box(&blob))
                .expect("round-trip load of a blob this same System just saved");
        });
    });
}

fn no_coprocessor(c: &mut Criterion) {
    let rom_path = workspace_root().join("tests/roms/undisbeliever/inidisp_hammer_0f00.sfc");
    let mut sys = booted_system_from(&rom_path).expect("committed benchmark ROM missing");
    bench_save_load(c, "no_coprocessor", &mut sys);
}

fn curated_superfx(c: &mut Criterion) {
    let rom_path = workspace_root()
        .join("tests/roms/external/commercial/LoRom/GSU-2/Super Mario World 2_ Yoshi's Island.sfc");
    let Some(mut sys) = booted_system_from(&rom_path) else {
        eprintln!(
            "SKIP save_state_cost::curated_superfx: no ROM at {} (gitignored corpus)",
            rom_path.display()
        );
        return;
    };
    bench_save_load(c, "curated_superfx", &mut sys);
}

fn besteffort_cx4(c: &mut Criterion) {
    let rom_path =
        workspace_root().join("tests/roms/external/commercial/LoRom/CX4/Mega Man X2.sfc");
    let Some(mut sys) = booted_system_from(&rom_path) else {
        eprintln!(
            "SKIP save_state_cost::besteffort_cx4: no ROM at {} (gitignored corpus)",
            rom_path.display()
        );
        return;
    };
    bench_save_load(c, "besteffort_cx4", &mut sys);
}

criterion_group!(benches, no_coprocessor, curated_superfx, besteffort_cx4);
criterion_main!(benches);
