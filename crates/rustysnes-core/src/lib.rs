//! `rustysnes-core` — the Bus + the master-clock lockstep scheduler. The single crate that
//! knows about every chip; it re-exports their public types so downstream consumers depend
//! on `rustysnes-core`, not the chip crates directly.

#![no_std]
extern crate alloc;

pub mod bus;
pub mod cheat;
pub mod controller;
pub mod dma;
pub mod dma_bus;
pub mod movie;
pub mod sa1_bus;
pub mod scheduler;
// `v0.8.0`, T-81-001b: 65C816 read/write watchpoints. Compiled out entirely when `debug-hooks` is
// off, so a default build carries zero extra code — this module's own doc has the detail.
#[cfg(feature = "debug-hooks")]
pub mod watchpoint;

// Re-export the chip crates (the public surface).
pub use rustysnes_apu as apu;
pub use rustysnes_cart as cart;
pub use rustysnes_cpu as cpu;
pub use rustysnes_ppu as ppu;

pub use bus::Bus;
pub use scheduler::System;
