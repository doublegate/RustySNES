//! CPU golden-log differ — the SNES port of RustyNES's `nestest` differ.
//!
//! Forces the 65C816 to the suite's entry point, captures `(PBR:PC, A, X, Y, S, P, cycle)`
//! per instruction, and diffs against a bundled golden log (the SNES analogue of
//! `nestest.log` — a 65C816 exerciser trace). The FIRST mismatch fails and prints the diff.

use rustysnes_core::cpu::Cpu;

/// One line of the golden reference trace. The exact field set tracks `docs/cpu.md`'s trace
/// format; this is the skeleton. The real golden log is committed under `tests/golden/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldenLine {
    /// Full 24-bit program address `(PBR << 16) | PC`.
    pub pc24: u32,
    /// Accumulator (16-bit; high byte 0 when the m flag selects 8-bit A).
    pub a: u16,
    /// X index register.
    pub x: u16,
    /// Y index register.
    pub y: u16,
    /// Stack pointer.
    pub s: u16,
    /// Processor status byte (the m/x/E-mode bits included).
    pub p: u8,
    /// Cumulative master-clock cycle count at this instruction's fetch.
    pub cycle: u64,
}

/// Parse one golden-log line. Format TBD from the chosen exerciser; skeleton returns `None`.
#[must_use]
#[allow(clippy::missing_const_for_fn)] // reason: the real parser allocates; const fits only the stub.
pub fn parse_golden_line(_text: &str) -> Option<GoldenLine> {
    // TODO(T-04): parse the committed golden-log line format (mirror RustyNES's
    // `parse_log_line`). Pin the exact format against the exerciser ROM first.
    None
}

/// Diff a captured trace against the golden reference. Returns the index of the first
/// mismatching line, or `None` when they match line-for-line up to `golden.len()`.
#[must_use]
pub fn diff_against_golden(captured: &[GoldenLine], golden: &[GoldenLine]) -> Option<usize> {
    golden
        .iter()
        .zip(captured)
        .position(|(g, c)| g != c)
        .or_else(|| (captured.len() < golden.len()).then_some(captured.len()))
}

/// Build a fresh 65C816 positioned at a golden-log "automation" entry point.
///
/// The SNES analogue of nestest's `PC=$C000` start. The concrete entry vector + reset cycle
/// count come from the exerciser ROM; this is the skeleton.
#[must_use]
pub fn cpu_for_golden_log() -> Cpu {
    // TODO(T-04): set PBR:PC to the exerciser entry + the post-reset cycle count.
    Cpu::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(pc24: u32, cycle: u64) -> GoldenLine {
        GoldenLine {
            pc24,
            a: 0,
            x: 0,
            y: 0,
            s: 0x01FF,
            p: 0,
            cycle,
        }
    }

    #[test]
    fn identical_traces_match() {
        let g = [line(0x00_8000, 0), line(0x00_8002, 6)];
        assert_eq!(diff_against_golden(&g, &g), None);
    }

    #[test]
    fn first_mismatch_is_reported() {
        let g = [line(0x00_8000, 0), line(0x00_8002, 6)];
        let c = [line(0x00_8000, 0), line(0x00_8002, 7)];
        assert_eq!(diff_against_golden(&c, &g), Some(1));
    }

    #[test]
    fn truncated_capture_is_a_mismatch() {
        let g = [line(0x00_8000, 0), line(0x00_8002, 6)];
        let c = [line(0x00_8000, 0)];
        assert_eq!(diff_against_golden(&c, &g), Some(1));
    }
}
