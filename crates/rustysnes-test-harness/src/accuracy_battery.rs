//! Accuracy-battery scoring — the tally type behind the AccuracySNES gate.
//!
//! The battery itself is `tests/accuracysnes.rs`, driving the first-party AccuracySNES
//! cartridge (`tests/roms/AccuracySNES/`). This module holds only the arithmetic both it and any
//! future group share.
//!
//! # History
//!
//! This started as ticket **T-04**: a skeleton `score_accuracy_battery()` that was to boot an
//! AccuracyCoin-equivalent ROM, drive it, and decode an on-screen PASS/FAIL grid. It sat
//! unimplemented for a long time for a simple reason recorded in `docs/STATUS.md` — *"no
//! publicly available SNES ROM plays the AccuracyCoin role"*. AccuracySNES is that ROM, written
//! first-party, and the stub is now gone.
//!
//! Two things changed from the original design, both deliberate:
//!
//! - **Scoring reads RAM, not the framebuffer.** The skeleton planned to *"decode the on-screen
//!   PASS/FAIL grid … read from the framebuffer / a result region rather than a clean status
//!   byte"*. RustyNES did exactly that for AccuracyCoin and its grid-stride bug silently skipped
//!   31 cells, reporting 75.93% where the truth was 64.03%. Since we author this cart, it
//!   publishes a proper machine-readable results block instead.
//! - **No input driving.** The skeleton planned to *"advance past the splash, press Start"*.
//!   AccuracySNES runs the whole battery unprompted, so the harness never touches the pad.

/// The decoded result of one accuracy-battery run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccuracyReport {
    /// Sub-tests that passed outright.
    pub passed: u32,
    /// Sub-tests that failed.
    pub failed: u32,
    /// Sub-tests that passed reporting a **variant** — one of several behaviours real hardware
    /// legitimately exhibits (5A22 v1/v2, PPU2 v1/v2/v3, 1CHIP vs 3CHIP, NTSC vs PAL). Counted
    /// as a pass, matching AccuracyCoin's treatment of the same idea.
    pub partial: u32,
}

impl AccuracyReport {
    /// Total sub-tests with a verdict (excludes not-run, skipped, and golden-vector slots).
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

    #[test]
    fn variants_count_as_passes() {
        let r = AccuracyReport {
            passed: 0,
            failed: 0,
            partial: 3,
        };
        assert!((r.pass_rate() - 1.0).abs() < f64::EPSILON);
    }
}
