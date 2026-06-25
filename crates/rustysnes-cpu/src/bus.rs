//! The CPU-side bus trait — the SNES port of RustyNES's `rustynes-cpu::Bus`.
//!
//! The 65C816 has a 24-bit address space (`(bank << 16) | addr`). It borrows `&mut impl Bus`
//! for the duration of an instruction; the bus fans the access out to WRAM, the PPU/APU
//! register windows, the controllers, the DMA/HDMA registers, and the cartridge board. The
//! concrete impl is in `rustysnes-core`; a tiny [`NullBus`] here lets the CPU be unit-tested
//! in isolation (the one-directional graph is what makes that possible).

/// Address-space bus seen by the 65C816.
pub trait Bus {
    /// Read a byte at a 24-bit address. Implementors are responsible for the region-variable
    /// access-cycle accounting (FastROM vs SlowROM) and ticking the other chips in lockstep.
    fn read24(&mut self, addr24: u32) -> u8;

    /// Write a byte at a 24-bit address.
    fn write24(&mut self, addr24: u32, val: u8);

    /// Edge-triggered NMI poll (PPU vblank → CPU). Returns `true` once per high→low edge.
    fn poll_nmi(&mut self) -> bool {
        false
    }

    /// Level-sensitive IRQ poll (PPU HV-IRQ, on-cart coprocessor, APU timer). Honored only
    /// when the CPU's I flag is clear.
    fn poll_irq(&mut self) -> bool {
        false
    }

    /// Called once per CPU cycle consumed. The lockstep scheduler uses this to advance the
    /// PPU/APU/coprocessor; the test harness uses it to count cycles for golden-log compare.
    fn on_cpu_cycle(&mut self) {}
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
        bus.on_cpu_cycle();
    }
}
