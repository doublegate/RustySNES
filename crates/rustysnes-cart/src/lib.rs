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
pub mod coproc;
pub mod header;
pub mod tier;

pub use board::{Board, Coprocessor, ExHiRom, HiRom, LoRom, MappedAddr};
pub use coproc::{Dsp1Board, Gsu, Revision, Sa1Board, SuperFxBoard, Upd77c25};
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
    /// Strips a 512-byte copier prefix when present, detects the internal header, and builds
    /// the matching board backed by the real ROM bytes + a zeroed, header-sized SRAM.
    ///
    /// # Errors
    /// Returns [`HeaderError`] if the image is too small or no internal header scores a valid
    /// map mode at any candidate offset (LoROM `$7FC0` / HiROM `$FFC0` / ExHiROM `$40FFC0`).
    pub fn load(rom: &[u8]) -> Result<Self, HeaderError> {
        let header = Header::detect(rom)?;
        // `Header::detect` records the copier-prefix length; build the board from the stripped
        // image so ROM offset 0 is the first real cartridge byte.
        let stripped = &rom[header.copier_prefix..];
        let board = board::select(&header, stripped);
        Ok(Self { header, board })
    }

    /// Decode a raw ROM image into a [`Cart`]. Alias of [`Cart::load`].
    ///
    /// # Errors
    /// See [`Cart::load`].
    pub fn from_rom(rom: &[u8]) -> Result<Self, HeaderError> {
        Self::load(rom)
    }

    /// Read a byte at a 24-bit CPU address `(bank << 16) | addr` via the active board.
    pub fn read24(&mut self, addr24: u32) -> u8 {
        self.board.read24(addr24)
    }

    /// Write a byte at a 24-bit CPU address `(bank << 16) | addr` via the active board.
    pub fn write24(&mut self, addr24: u32, val: u8) {
        self.board.write24(addr24, val);
    }

    /// Borrow the current SRAM contents (for a battery save). Empty if the cart has no SRAM.
    #[must_use]
    pub fn save_sram(&self) -> &[u8] {
        self.board.sram()
    }

    /// Restore SRAM contents (from a battery save). Copies up to the board's SRAM length; a
    /// shorter slice leaves the tail zeroed, a longer one is truncated.
    pub fn load_sram(&mut self, data: &[u8]) {
        let dst = self.board.sram_mut();
        let n = data.len().min(dst.len());
        dst[..n].copy_from_slice(&data[..n]);
    }

    /// Advance any on-cart coprocessor by one of its clock units. Default boards no-op.
    pub fn coprocessor_tick(&mut self) {
        self.board.coprocessor_tick();
    }

    /// Supply a coprocessor firmware dump (the user-provided chip ROM, e.g. DSP-1 `dsp1.rom`).
    ///
    /// Returns `true` if this cart's board carried a chip-ROM-dump coprocessor that accepted the
    /// image. A cart without such a coprocessor (or a dump of the wrong size) returns `false` and
    /// is unchanged — the honesty posture of `docs/adr/0003`: absent the dump the coprocessor is
    /// non-functional, never silently degraded.
    pub fn install_coprocessor_firmware(&mut self, bytes: &[u8]) -> bool {
        self.board.load_firmware(bytes)
    }

    /// The specific firmware file name this cart's board expects, if it knows exactly which chip
    /// dump it needs (see [`crate::board::Board::firmware_hint`]). `None` for boards that accept
    /// any same-family dump (or carry no firmware-dependent coprocessor at all).
    #[must_use]
    pub fn firmware_hint(&self) -> Option<&'static str> {
        self.board.firmware_hint()
    }

    /// Count of host accesses to the coprocessor's data ports since power-on (debugger/diag).
    #[must_use]
    pub fn coprocessor_host_accesses(&self) -> u64 {
        self.board.coprocessor_host_accesses()
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
        assert!(Cart::load(&[]).is_err());
    }

    #[test]
    fn load_lorom_and_sram_roundtrip() {
        use alloc::vec;
        use alloc::vec::Vec;
        // Minimal valid LoROM image (64 KiB) with 8 KiB battery SRAM.
        let mut rom: Vec<u8> = vec![0u8; 0x1_0000];
        let base = 0x7FC0;
        for (i, b) in b"RUSTYSNES LOROM TEST ".iter().enumerate() {
            rom[base + i] = *b;
        }
        rom[base + 0x15] = 0x20; // LoROM, slow
        rom[base + 0x16] = 0x02; // ROM+RAM+battery
        rom[base + 0x18] = 0x03; // 8 KiB SRAM
        rom[base + 0x19] = 0x01; // North America / NTSC
        let checksum: u16 = 0x4321;
        rom[base + 0x1C..base + 0x1E].copy_from_slice(&(!checksum).to_le_bytes());
        rom[base + 0x1E..base + 0x20].copy_from_slice(&checksum.to_le_bytes());
        rom[base + 0x3C..base + 0x3E].copy_from_slice(&0x8000u16.to_le_bytes());
        rom[0x1234] = 0x5A; // a ROM byte to read back

        let mut cart = Cart::load(&rom).expect("valid lorom");
        assert_eq!(cart.header.map_mode, MapMode::LoRom);
        assert_eq!(cart.header.sram_size, 0x2000);
        // ROM offset 0x1234 lives at bank $00:$9234 ($8000 + 0x1234).
        assert_eq!(cart.read24(0x00_9234), 0x5A);
        // SRAM round-trip.
        cart.write24(0x70_0010, 0x77);
        assert_eq!(cart.read24(0x70_0010), 0x77);
        assert_eq!(cart.save_sram()[0x10], 0x77);
        // load_sram restores.
        let mut snap = vec![0u8; 0x2000];
        snap[0x10] = 0xEE;
        cart.load_sram(&snap);
        assert_eq!(cart.read24(0x70_0010), 0xEE);
    }
}
