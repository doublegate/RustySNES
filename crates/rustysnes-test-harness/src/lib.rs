//! `rustysnes-test-harness` — the AccuracyCoin-equivalent gate. Reuse the RustyNES harness
//! SHAPE (don't reinvent): a CPU golden-log differ, a `run_until_complete()` test-ROM runner,
//! the accuracy-battery scorer, a visual-golden/`.snap` comparator, and (when board breadth is
//! large) the `mapper_tier_honesty` test that forbids a `BestEffort` board backing the
//! accuracy oracle. See `docs/testing-strategy.md`.
//!
//! Each piece is a SKELETON with a clear surface + `TODO(T-PS-NNN)` markers; the real driving
//! logic lands once the chip models exist (test-ROM-is-spec).

#![warn(missing_docs)]

pub mod accuracy_battery;
pub mod golden_log;
pub mod runner;
pub mod visual;

pub use accuracy_battery::{AccuracyReport, score_accuracy_battery};
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
