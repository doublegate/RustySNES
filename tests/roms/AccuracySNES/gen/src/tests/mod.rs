//! The test definitions, one module per group.
//!
//! Group A (CPU) landed in Phase A; Group C (PPU) and Group B (5A22 bus/clock/timing) are
//! arriving in Phase B. Groups D-G land in later phases — see `docs/accuracysnes-plan.md` for
//! what blocks each, and `to-dos/ROADMAP.md` for the T-04-* ticket IDs.

pub mod bus;
pub mod cpu;
pub mod ppu;

use crate::dsl::Test;

/// Every test in the battery, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    let mut v = cpu::all();
    v.extend(ppu::all());
    v.extend(bus::all());
    v
}
