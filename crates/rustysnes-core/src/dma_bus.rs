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

    /// Advance the master clock by `clocks` *during* a transfer, so time-dependent state that
    /// gates the transfer — chiefly the PPU scanline that decides VRAM/CGRAM/OAM accessibility
    /// (`$2118`/`$2119` land only in force-blank or V-blank) — tracks the transfer in lockstep,
    /// exactly as the real bus keeps scanning while a DMA runs (ares `Channel::step` →
    /// `cpu.step`). A big GP-DMA that starts late in active display and crosses into V-blank must
    /// have its V-blank portion actually reach VRAM; freezing the scanline for the whole burst
    /// would drop every write (the Star Fox framebuffer-transfer bug). Default no-op so the
    /// controller stays unit-testable against an isolated bus with no clock.
    fn step(&mut self, _clocks: u32) {}

    /// The PPU scanline the bus is currently scanning. On real hardware HDMA preempts a running
    /// GP-DMA at the *start of every scanline*; a long GP-DMA that spans scanline boundaries must
    /// therefore let those HDMA transfers interleave (Star Fox force-blanks its framebuffer DMA
    /// via an HDMA-driven `$2100` write on the very lines the DMA is filling). [`crate::dma::Dma`]
    /// drives that interleave itself from inside `run_gp` because the bus's own per-tick HDMA path
    /// is dormant while the bus has lent out its controller. Default `u16::MAX` so a clockless
    /// unit-test bus never trips the interleave.
    fn scanline(&self) -> u16 {
        u16::MAX
    }

    /// The last visible scanline HDMA services this frame (`visible_height`), so the in-GP-DMA
    /// interleave knows the visible-line window. Default `0` (unit-test buses run no HDMA).
    fn visible_height(&self) -> u16 {
        0
    }

    /// The scanline the bus last serviced HDMA for, so the in-GP-DMA interleave resumes from the
    /// bus's own bookkeeping and hands it back afterwards (no line runs twice, none is skipped).
    /// Default `u16::MAX`.
    fn hdma_last_line(&self) -> u16 {
        u16::MAX
    }

    /// Record that HDMA has now been serviced for `line`, syncing the bus's bookkeeping with the
    /// interleave driven from inside `run_gp`. Default no-op.
    fn set_hdma_last_line(&mut self, _line: u16) {}
}
