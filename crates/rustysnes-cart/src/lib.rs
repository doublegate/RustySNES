//! `rustysnes-cart` — the SNES cartridge / memory-map / coprocessor model (cart).
//!
//! Owns the LoROM / HiROM / ExHiROM address mapping and every on-cart coprocessor
//! (DSP-1..4 / Super FX (GSU) / SA-1 / S-DD1 / SPC7110 / CX4 / OBC1). All board-specific
//! behavior — bank switching, the coprocessor clock, IRQ/refresh hooks — lives behind the
//! [`Board`] trait, not in the PPU or CPU (the RustyNES "mapper logic lives in the mapper"
//! rule, ported). The video chip depends ONLY on this crate for its VRAM/CHR bus.
//!
//! Part of the one-directional chip-crate graph (see `docs/architecture.md`): this crate
//! does NOT depend on the other chip crates. `#![no_std]` + alloc so it cross-compiles to a
//! bare-metal target; only the frontend carries `std` + `unsafe`.

#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

pub mod board;
pub mod header;
pub mod tier;

pub use board::{Board, Coprocessor, ExHiRom, HiRom, LoRom, MappedAddr};
pub use header::{Header, HeaderError, MapMode, Region};
pub use tier::{BoardTier, board_tier};

use alloc::boxed::Box;

/// A loaded cartridge: a parsed [`Header`] + the [`Board`] that implements its mapping and
/// any coprocessor. The [`crate::board`] module decodes the header into the right board.
///
/// Replace the stub internals with the real ROM/SRAM storage; pin behavior against the test
/// ROMs FIRST (test-ROM-is-spec), then implement until they pass.
pub struct Cart {
    /// The parsed cartridge header (map mode, region, coprocessor id, sizes).
    pub header: Header,
    /// The active memory-map board (LoROM / HiROM / ExHiROM + coprocessor hooks).
    pub board: Box<dyn Board>,
}

impl Cart {
    /// Decode a raw ROM image into a [`Cart`] (header detection + board selection).
    ///
    /// # Errors
    /// Returns [`HeaderError`] if the image is too small or no internal header scores a valid
    /// map mode at any candidate offset (LoROM `$7FC0` / HiROM `$FFC0` / ExHiROM `$40FFC0`).
    pub fn from_rom(rom: &[u8]) -> Result<Self, HeaderError> {
        let header = Header::detect(rom)?;
        let board = board::select(&header);
        Ok(Self { header, board })
    }

    /// Advance any on-cart coprocessor by one of its clock units. Default boards no-op.
    pub fn coprocessor_tick(&mut self) {
        self.board.coprocessor_tick();
    }
}

impl core::fmt::Debug for Cart {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cart")
            .field("header", &self.header)
            .field("board", &self.board.name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rom_is_rejected() {
        assert!(Cart::from_rom(&[]).is_err());
    }
}
