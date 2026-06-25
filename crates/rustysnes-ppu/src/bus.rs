//! The PPU-side narrow bus trait — the SNES port of RustyNES's `PpuBus`.
//!
//! The PPU owns its 64 KiB VRAM, CGRAM, and OAM directly; those are NOT on this trait. This
//! trait is ONLY for what the PPU must reach through the cartridge: extended Mode 7 / coprocessor
//! reads on enhancement boards, and the board IRQ/scanline notify hooks (the `notify_a12`
//! analogues). In production, `rustysnes-core` implements [`VideoBus`] over the cart's
//! [`rustysnes_cart::Board`]; in unit tests a tiny in-memory impl suffices.

/// The narrow bus interface the PPU sees. Every method has a default so a minimal test impl
/// (`struct DummyBus; impl VideoBus for DummyBus {}`) compiles.
pub trait VideoBus {
    /// Cartridge-mediated read for the PPU (Mode 7 extended-bank fetches on coprocessor
    /// boards route here; plain boards return open bus). Default `0`.
    fn cart_read(&mut self, _addr24: u32) -> u8 {
        0
    }

    /// Notify the board that the PPU is starting a new scanline (for scanline-aligned
    /// coprocessor / IRQ logic). Default no-op.
    fn notify_scanline(&mut self) {}

    /// Notify the board that the PPU has entered vertical blank. Default no-op.
    fn notify_vblank(&mut self) {}
}

/// A no-op [`VideoBus`] for unit-testing the PPU in isolation (the one-directional graph is
/// what makes per-chip testing possible).
#[derive(Debug, Default)]
pub struct NullVideoBus;

impl VideoBus for NullVideoBus {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_bus_reads_open() {
        let mut bus = NullVideoBus;
        assert_eq!(bus.cart_read(0x00_8000), 0);
        bus.notify_scanline();
        bus.notify_vblank();
    }
}
