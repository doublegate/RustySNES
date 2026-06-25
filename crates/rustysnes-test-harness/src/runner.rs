//! Test-ROM runner — the SNES port of RustyNES's blargg-style `run_*_until_complete`.
//!
//! Boots a [`rustysnes_core::System`], steps it until the suite's completion sentinel fires,
//! then surfaces the result code + message. Many SNES test suites (PeterLemon/SNES, the
//! SPC700/DSP tests) report status by writing a sentinel + a result string into a fixed WRAM
//! region — the analogue of blargg's `$6000` protocol.

use rustysnes_core::System;

/// The outcome of a test-ROM run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResult {
    /// The suite reported success.
    Passed,
    /// The suite reported a failure with this result code + message.
    Failed {
        /// Suite-specific result code.
        code: u8,
        /// Decoded human-readable status text.
        message: String,
    },
    /// The run hit the step budget without the sentinel firing.
    Timeout,
}

/// Maximum CPU steps before a run is declared a [`TestResult::Timeout`]. Generous; the real
/// per-suite budgets come from `docs/testing-strategy.md`.
const DEFAULT_STEP_BUDGET: u64 = 250_000_000;

/// Step `system` until the suite's completion sentinel, then return the result.
///
/// SKELETON: wire the `$2140`-style result protocol once the chip models can run a ROM. Until
/// then it steps the budget and reports [`TestResult::Timeout`] (no chip yet advances state).
#[must_use]
pub fn run_until_complete(system: &mut System) -> TestResult {
    let mut steps = 0u64;
    while steps < DEFAULT_STEP_BUDGET {
        system.tick_one_master();
        steps += 1;
        // TODO(T-04): poll the suite's WRAM sentinel; when it signals "done", decode the
        // result code + message and return Passed / Failed. Cap the loop here so the skeleton
        // can't run unbounded.
        if steps >= 1 {
            break;
        }
    }
    TestResult::Timeout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_runner_times_out_without_a_loaded_rom() {
        let mut sys = System::new(0);
        assert_eq!(run_until_complete(&mut sys), TestResult::Timeout);
    }
}
