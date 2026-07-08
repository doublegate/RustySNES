//! Headless full-frame throughput — the `docs/performance.md` "≤ ~2 ms per emulated frame"
//! target, measured (not guessed) against a real committed test ROM
//! (`tests/roms/undisbeliever/inidisp_hammer_0f00.sfc`, Zlib-licensed, chosen for having no
//! coprocessor/DMA-heavy content so this benchmark isolates the base CPU+PPU+scheduler cost, not
//! a specific board's own overhead). Results feed `docs/benchmarks.md`.
//!
//! Run: `cargo bench -p rustysnes-core --bench headless_frame`.
#![allow(missing_docs)] // Criterion's macro-generated `main`/harness items have no doc comments.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use rustysnes_core::System;
use rustysnes_core::cart::Cart;

fn rom_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/roms/undisbeliever/inidisp_hammer_0f00.sfc")
}

fn booted_system() -> System {
    let rom = std::fs::read(rom_path()).expect("committed benchmark ROM missing");
    let cart = Cart::from_rom(&rom).expect("committed benchmark ROM failed to parse");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    sys
}

fn headless_frame(c: &mut Criterion) {
    let mut sys = booted_system();
    // Warm up past the ROM's own boot/init sequence so the measured frames are steady-state,
    // not a one-time cold-start cost.
    for _ in 0..16 {
        sys.run_frame();
    }
    c.bench_function("headless_frame_steady_state", |b| {
        b.iter(|| {
            sys.run_frame();
            black_box(sys.bus.framebuffer());
        });
    });
}

criterion_group!(benches, headless_frame);
criterion_main!(benches);
