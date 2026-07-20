//! The test definitions, one module per group.
//!
//! Group A (CPU) landed in Phase A; Group C (PPU) is arriving in Phase B. Groups B, D-G
//! land in later phases (see `to-dos/ROADMAP.md`).

pub mod cpu;
pub mod ppu;

use crate::dsl::Test;

/// Every test in the battery, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    let mut v = cpu::all();
    v.extend(ppu::all());
    v
}
