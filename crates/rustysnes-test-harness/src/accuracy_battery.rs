//! Accuracy-battery scorer — the SNES port of RustyNES's `AccuracyCoin` driver.
//!
//! Boots the SNES accuracy-battery ROM (the AccuracyCoin-equivalent), drives it through its
//! self-test, decodes the on-screen / in-memory PASS/FAIL grid, and reports a pass rate. CI
//! gates on the target (≥90% by v1.0, 100% the goal). Like AccuracyCoin, the result is read
//! from the framebuffer / a result region rather than a clean status byte.

use rustysnes_core::System;

/// The decoded result of one accuracy-battery run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccuracyReport {
    /// Number of sub-tests that passed.
    pub passed: u32,
    /// Number that failed.
    pub failed: u32,
    /// Number with multiple acceptable behaviours (counted as a pass, like AccuracyCoin's
    /// "partial").
    pub partial: u32,
}

impl AccuracyReport {
    /// Total sub-tests with a verdict (excludes unassigned/blank grid slots).
    #[must_use]
    pub const fn total(self) -> u32 {
        self.passed + self.failed + self.partial
    }

    /// Pass rate in `0.0..=1.0`: `(passed + partial) / total`. `0.0` when nothing ran.
    #[must_use]
    pub fn pass_rate(self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            f64::from(self.passed + self.partial) / f64::from(self.total())
        }
    }

    /// Whether the run meets a CI gate threshold (e.g. `0.90`).
    #[must_use]
    pub fn meets(self, threshold: f64) -> bool {
        self.pass_rate() >= threshold
    }
}

/// Boot + drive the accuracy battery on `system`, returning the decoded report.
///
/// SKELETON: drives nothing yet (no PPU to decode a result grid from). Wire the boot/drive/
/// decode sequence once the PPU + framebuffer land. Mirror AccuracyCoin's sequence: boot,
/// advance past the splash, press Start, sample the result grid until it stabilizes, decode.
#[must_use]
pub fn score_accuracy_battery(system: &mut System) -> AccuracyReport {
    let _ = system;
    // TODO(T-04): the boot → run-all → grid-decode sequence; return real counts.
    AccuracyReport::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_rate_math() {
        let r = AccuracyReport {
            passed: 8,
            failed: 1,
            partial: 1,
        };
        assert_eq!(r.total(), 10);
        assert!((r.pass_rate() - 0.9).abs() < 1e-9);
        assert!(r.meets(0.9));
        assert!(!r.meets(0.95));
    }

    #[test]
    fn empty_report_is_zero() {
        assert!(AccuracyReport::default().pass_rate().abs() < f64::EPSILON);
        assert!(!AccuracyReport::default().meets(0.0001));
    }
}
