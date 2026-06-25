//! The narrow bus interface the DMA controller drives.
//!
//! The DMA controller moves bytes between the **A-bus** (the 24-bit CPU address space — WRAM,
//! ROM, SRAM) and the **B-bus** (the `$2100-$21FF` PPU/APU register window, addressed by its
//! low byte). Keeping this a trait decouples [`crate::dma::Dma`] from the concrete [`crate::Bus`]
//! so the transfer logic is unit-testable in isolation. The A-bus invalid-region rules (DMA
//! cannot touch `$2100-$21FF`, `$4000-$43FF` via the A-bus) and the WRAM↔WRAM exclusion are
//! enforced by the concrete `Bus` impl, which returns open bus / drops the write there.

/// Read/write split across the SNES A-bus (24-bit) and B-bus (`$21xx`) for the DMA controller.
pub trait DmaBus {
    /// Read a byte from the A-bus (24-bit CPU address). Invalid A-bus regions return open bus.
    fn read_a(&mut self, addr: u32) -> u8;

    /// Write a byte to the A-bus (24-bit CPU address). Invalid regions are dropped.
    fn write_a(&mut self, addr: u32, val: u8);

    /// Read a byte from the B-bus register `$2100 | addr`.
    fn read_b(&mut self, addr: u8) -> u8;

    /// Write a byte to the B-bus register `$2100 | addr`.
    fn write_b(&mut self, addr: u8, val: u8);
}
