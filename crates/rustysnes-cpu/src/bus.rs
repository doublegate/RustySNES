//! The CPU-side bus trait — the SNES port of RustyNES's `rustynes-cpu::Bus`.
//!
//! The 65C816 has a 24-bit address space (`(bank << 16) | addr`). It borrows `&mut impl Bus`
//! for the duration of an instruction; the bus fans the access out to WRAM, the PPU/APU
//! register windows, the controllers, the DMA/HDMA registers, and the cartridge board. The
//! concrete impl is in `rustysnes-core`; a tiny [`NullBus`] here lets the CPU be unit-tested
//! in isolation (the one-directional graph is what makes that possible).

/// Address-space bus seen by the 65C816.
///
/// Timing follows ares `sfc/cpu/memory.cpp`: the CPU asks the bus how many master clocks an
/// access costs ([`Bus::access_cycles`], ares `wait`), then interleaves the clock advance
/// ([`Bus::advance`], ares `step`) around the access so it lands at the hardware-exact instant —
/// a write at the END of its cycle (advance the full cost, then write), a read four clocks before
/// the end (advance cost−4, read, advance 4). This phase matters: it decides the exact hcounter
/// at which a register write becomes visible to the PPU/HDMA (e.g. the HDMAEN mid-scanline latch).
pub trait Bus {
    /// Read a byte at a 24-bit address (no clock advance — the CPU sequences timing via
    /// [`Bus::advance`] around this call).
    fn read24(&mut self, addr24: u32) -> u8;

    /// Write a byte at a 24-bit address (no clock advance — see [`Bus::read24`]).
    fn write24(&mut self, addr24: u32, val: u8);

    /// Master clocks this access costs (ares `CPU::wait`): the region-variable access speed
    /// (FastROM vs SlowROM, WRAM, I/O). Defaults to the SlowROM/internal-cycle cost of 6.
    fn access_cycles(&self, _addr24: u32) -> u32 {
        6
    }

    /// Advance the system clock by `clocks` master ticks (ares `CPU::step`), ticking the
    /// PPU/APU/coprocessor and HDMA in lockstep. Default no-op for buses whose timebase is
    /// charged elsewhere (the SA-1 second CPU) or that don't model timing (unit-test buses).
    fn advance(&mut self, _clocks: u32) {}

    /// Edge-triggered NMI poll (PPU vblank → CPU). Returns `true` once per high→low edge.
    fn poll_nmi(&mut self) -> bool {
        false
    }

    /// Level-sensitive IRQ poll (PPU HV-IRQ, on-cart coprocessor, APU timer). Honored only
    /// when the CPU's I flag is clear.
    fn poll_irq(&mut self) -> bool {
        false
    }
}

/// A no-op [`Bus`] (all reads open, writes dropped) for unit-testing the CPU in isolation.
#[derive(Debug, Default)]
pub struct NullBus;

impl Bus for NullBus {
    fn read24(&mut self, _addr24: u32) -> u8 {
        0
    }
    fn write24(&mut self, _addr24: u32, _val: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_bus_defaults() {
        let mut bus = NullBus;
        assert_eq!(bus.read24(0x00_8000), 0);
        bus.write24(0x00_8000, 0xFF);
        assert!(!bus.poll_nmi());
        assert!(!bus.poll_irq());
        assert_eq!(bus.access_cycles(0x00_8000), 6);
        bus.advance(6);
    }
}
