//! The test definitions, one module per group.
//!
//! Groups B-G land in later phases (see `to-dos/ROADMAP.md`); Phase A ships Group A only.

pub mod cpu;

use crate::dsl::Test;

/// Every test in the battery, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    cpu::all()
}
