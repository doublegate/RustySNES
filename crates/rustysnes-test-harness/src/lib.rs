//! `rustysnes-test-harness` — the accuracy gate. See `docs/testing-strategy.md`.
//!
//! The real oracles are the integration tests in `tests/`: the per-opcode 65816 and SPC700 JSON
//! runners, the on-cart suites (gilyon, undisbeliever, blargg `spc_*`), the coprocessor
//! liveness checks, the board-tier honesty gate, and — since Phase A of the AccuracySNES work —
//! `tests/accuracysnes.rs`, which drives this project's own first-party hardware-accuracy
//! cartridge (`tests/roms/AccuracySNES/`).
//!
//! This library holds the small shared pieces those tests build on. Some of it is still
//! skeletal (`runner`, `golden_log`); each such item says so in its own docs.

#![warn(missing_docs)]

pub mod accuracy_battery;
pub mod golden_log;
pub mod runner;
pub mod visual;

pub use accuracy_battery::AccuracyReport;
pub use golden_log::{GoldenLine, diff_against_golden};
pub use runner::{TestResult, run_until_complete};
pub use visual::{FrameHash, compare_snapshot};

/// Returns the crate version string.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }
}
