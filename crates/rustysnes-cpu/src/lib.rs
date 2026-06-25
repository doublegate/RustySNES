//! `rustysnes-cpu` — WDC 65C816 (cpu).
//!
//! 16-bit 65C816 main CPU; emulation/native modes; region-variable (FastROM/SlowROM) access
//! speed. The CPU borrows `&mut impl Bus` for the duration of an instruction (the RustyNES
//! "Bus owns everything mutable" rule — the CPU never owns the PPU/APU/cart). The concrete
//! `Bus` impl lives in `rustysnes-core`; this crate only sees the narrow [`Bus`] trait.
//!
//! Part of the one-directional chip-crate graph (see `docs/architecture.md`): this crate
//! does NOT depend on the other chip crates. `#![no_std]` + alloc so it cross-compiles to a
//! bare-metal target; only the frontend carries `std` + `unsafe`.

#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

pub mod bus;
pub use bus::Bus;

/// WDC 65C816 state. Replace this stub with the real model; pin behavior against the test
/// ROMs FIRST (test-ROM-is-spec), then implement until they pass.
#[derive(Debug, Default, Clone)]
pub struct Cpu {
    // TODO(T-01): registers (A/X/Y 8/16-bit, D, DBR, PBR, S, P with the m/x mode flags,
    // emulation-mode E latch) + internal state per `docs/cpu.md`.
    /// Cumulative master-clock cycles consumed (region-variable: a FastROM access costs
    /// fewer master cycles than a SlowROM one). Used by the golden-log differ + scheduler.
    pub cycles: u64,
}

impl Cpu {
    /// Construct at power-on. Phase alignment comes from a *seeded* PRNG (determinism
    /// contract — see `docs/adr/0004`), never the OS RNG.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute one 65C816 instruction against `bus`, returning the master-clock cycles it
    /// consumed (region-variable). Hot path: keep allocation-free.
    // reason: a real instruction step mutates CPU + bus; `const` is correct only for the
    // empty skeleton body and would have to be removed the moment decode lands.
    #[allow(clippy::missing_const_for_fn, clippy::unused_self)]
    pub fn step(&mut self, bus: &mut impl Bus) -> u32 {
        // TODO(T-01): decode + execute one opcode; honor the m/x width flags + emulation mode;
        // poll NMI/IRQ at the documented sub-instruction points (RustyNES test-ROM-is-spec).
        let _ = bus;
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::NullBus;

    #[test]
    fn constructs() {
        let _ = Cpu::new();
    }

    #[test]
    fn steps_against_null_bus() {
        let mut cpu = Cpu::new();
        let mut bus = NullBus;
        assert_eq!(cpu.step(&mut bus), 0);
    }
}
