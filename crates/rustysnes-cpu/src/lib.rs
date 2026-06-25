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
//!
//! # Cycle-count unit
//!
//! [`Cpu::step`] returns a count of **CPU cycles** (the number of `bus.on_cpu_cycle()` ticks
//! the instruction consumed: one per bus byte access plus internal I/O cycles), per the
//! standard 65C816 timing tables and the variable-timing rules in `docs/cpu.md`. It does
//! **not** apply the per-access master-clock speed weighting (6/8/12) — that is the Bus's
//! job. The value equals the increment of [`Cpu::cycles`] across the call.

#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

pub mod addr;
pub mod bus;
pub mod exec;
pub mod regs;

pub use addr::{Effective, Mode};
pub use bus::Bus;
pub use regs::{Regs, Status};

/// Native-mode interrupt/exception vector addresses (bank `0`).
pub mod vectors {
    /// Native COP software interrupt vector (`$00FFE4`).
    pub const COP_NATIVE: u32 = 0x00_FFE4;
    /// Native BRK vector (`$00FFE6`).
    pub const BRK_NATIVE: u32 = 0x00_FFE6;
    /// Native ABORT vector (`$00FFE8`).
    pub const ABORT_NATIVE: u32 = 0x00_FFE8;
    /// Native NMI vector (`$00FFEA`).
    pub const NMI_NATIVE: u32 = 0x00_FFEA;
    /// Native IRQ vector (`$00FFEE`).
    pub const IRQ_NATIVE: u32 = 0x00_FFEE;
    /// Emulation COP vector (`$00FFF4`).
    pub const COP_EMU: u32 = 0x00_FFF4;
    /// Emulation ABORT vector (`$00FFF8`).
    pub const ABORT_EMU: u32 = 0x00_FFF8;
    /// Emulation NMI vector (`$00FFFA`).
    pub const NMI_EMU: u32 = 0x00_FFFA;
    /// RESET vector (always emulation-table; `$00FFFC`).
    pub const RESET: u32 = 0x00_FFFC;
    /// Emulation IRQ/BRK vector (`$00FFFE`).
    pub const IRQ_BRK_EMU: u32 = 0x00_FFFE;
}

/// WDC 65C816 CPU core.
///
/// Holds the architectural [`Regs`] register file plus bookkeeping. The model is driven one
/// instruction at a time via [`Cpu::step`]; the bus is borrowed for the call's duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cpu {
    /// Architectural register file (A/X/Y/S/D/DBR/PBR/PC/P/E).
    pub regs: Regs,
    /// Cumulative CPU cycles consumed across all instructions (one per `on_cpu_cycle`).
    pub cycles: u64,
    /// `WAI` latch: the CPU is waiting for an interrupt; cleared when one is taken.
    pub waiting: bool,
    /// `STP` latch: the CPU has been stopped and resumes only on reset.
    pub stopped: bool,
    /// Per-instruction CPU-cycle accumulator (reset at the start of each [`Cpu::step`]).
    cyc: u32,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    /// Construct at power-on (emulation mode). Call [`Cpu::reset`] to load `PC` from the
    /// reset vector before stepping.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            regs: Regs::new(),
            cycles: 0,
            waiting: false,
            stopped: false,
            cyc: 0,
        }
    }

    /// Power-on / reset: force emulation mode (`E=1`, `M=1`, `X=1`, `I=1`, `D=0`), park the
    /// stack at `$01FF`, clear the `WAI`/`STP` latches, and load `PC` from the emulation
    /// RESET vector at `$00FFFC/$00FFFD`. `cycles` is left as-is (cumulative counter).
    pub fn reset(&mut self, bus: &mut impl Bus) {
        self.regs = Regs::new();
        self.waiting = false;
        self.stopped = false;
        self.cyc = 0;
        let lo = self.bus_read8(bus, vectors::RESET);
        let hi = self.bus_read8(bus, vectors::RESET + 1);
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
        self.regs.pbr = 0;
    }
}

#[cfg(test)]
mod tests;
