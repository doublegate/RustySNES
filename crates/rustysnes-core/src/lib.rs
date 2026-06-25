//! `rustysnes-core` — the Bus + the master-clock lockstep scheduler. The single crate that
//! knows about every chip; it re-exports their public types so downstream consumers depend
//! on `rustysnes-core`, not the chip crates directly.

#![no_std]
extern crate alloc;

pub mod bus;
pub mod dma;
pub mod dma_bus;
pub mod scheduler;

// Re-export the chip crates (the public surface).
pub use rustysnes_apu as apu;
pub use rustysnes_cart as cart;
pub use rustysnes_cpu as cpu;
pub use rustysnes_ppu as ppu;

pub use bus::Bus;
pub use scheduler::System;
