//! The OBC1 board — a simple sprite-object-controller ASIC (Metal Combat: Falcon's Revenge).
//!
//! OBC1 builds sprite (OAM) tables in its own dedicated 8 KiB RAM, which the game then DMAs to
//! PPU OAM. There is no on-chip program (no LLE, no chip-ROM dump) — it is a fixed hardware
//! register file over that RAM, so the board is functional the moment the cart loads
//! (`docs/adr/0003` — nothing to silently degrade). Clean-room port of ares' `OBC1` component
//! (ISC): every register at `$1FF0-$1FF7` (bus address `& 0x1FFF`) redirects through a movable
//! `baseptr`/`address`/`shift` cursor into the RAM; every other address is a plain RAM byte.
//!
//! Bus window (`ares` board `SHVC-2E3M-01`/`OBC1-LOROM-RAM`, both LoROM): `$00-3F,$80-BF:$6000-
//! $7FFF` and the mirror `$70-71,$F0-F1:$6000-$7FFF,$E000-$FFFF`, both masked to a repeating 8 KiB
//! window (`addr & 0x1FFF`) over the SAME dedicated RAM — never the cartridge's own SRAM. ROM and
//! everything else delegates to the wrapped base board.

// Chip-name jargon (OBC1, OAM, ...) is not Rust code.
#![allow(clippy::doc_markdown)]

use alloc::boxed::Box;
use alloc::vec;

use crate::board::{Board, Coprocessor, MappedAddr};

/// OBC1's own dedicated RAM size — 8 KiB (`ares` `SHVC-2E3M-01` Save-RAM size, confirmed against
/// Metal Combat: Falcon's Revenge's game-database entry).
const RAM_SIZE: usize = 0x2000;

/// Classify a 24-bit CPU address into the OBC1 window, returning the RAM-relative offset
/// (`addr & 0x1FFF`) if it lands in either bus window.
fn classify(addr24: u32) -> Option<u16> {
    let bank = (addr24 >> 16) & 0xFF;
    let addr = addr24 & 0xFFFF;
    let in_main = matches!(bank, 0x00..=0x3F | 0x80..=0xBF) && (0x6000..=0x7FFF).contains(&addr);
    let in_mirror = matches!(bank, 0x70 | 0x71 | 0xF0 | 0xF1)
        && ((0x6000..=0x7FFF).contains(&addr) || addr >= 0xE000);
    (in_main || in_mirror).then_some((addr & 0x1FFF) as u16)
}

/// Movable read/write cursor into the 8 KiB RAM, reprogrammed via `$1FF5`/`$1FF6`.
#[derive(Debug, Clone, Copy, Default)]
struct Status {
    /// Selected slot index (7-bit, `$1FF6 & 0x7F`).
    address: u16,
    /// Table base (`$1800` or `$1C00`, from `$1FF5` bit 0).
    baseptr: u16,
    /// Bit-pair shift into the packed `$1FF4` byte (`($1FF6 & 3) << 1`).
    shift: u16,
}

/// A LoROM cartridge carrying an OBC1.
pub struct Obc1Board {
    inner: Box<dyn Board>,
    ram: Box<[u8]>,
    status: Status,
}

impl core::fmt::Debug for Obc1Board {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Obc1Board")
            .field("inner", &self.inner.name())
            .field("ram_len", &self.ram.len())
            .field("status", &self.status)
            .finish()
    }
}

impl Obc1Board {
    /// Wrap a base board (`inner`, the cart's LoROM ROM/SRAM decode) with an OBC1. The cursor is
    /// primed from the RAM's own persisted `$1FF5`/`$1FF6` bytes (ares `OBC1::power`), so a
    /// reloaded save continues from where it left off.
    #[must_use]
    pub fn new(inner: Box<dyn Board>) -> Self {
        let ram = vec![0u8; RAM_SIZE].into_boxed_slice();
        let mut b = Self {
            inner,
            ram,
            status: Status::default(),
        };
        b.power();
        b
    }

    /// Re-derive the read/write cursor from the RAM's own persisted control bytes (called once at
    /// construction, mirroring ares' `OBC1::power`).
    fn power(&mut self) {
        self.status.baseptr = if self.ram_read(0x1FF5) & 1 != 0 {
            0x1800
        } else {
            0x1C00
        };
        self.status.address = u16::from(self.ram_read(0x1FF6) & 0x7F);
        self.status.shift = u16::from(self.ram_read(0x1FF6) & 3) << 1;
    }

    fn ram_read(&self, addr: u16) -> u8 {
        self.ram[usize::from(addr & 0x1FFF)]
    }

    fn ram_write(&mut self, addr: u16, val: u8) {
        self.ram[usize::from(addr & 0x1FFF)] = val;
    }

    fn read_register(&self, addr: u16) -> u8 {
        let s = self.status;
        match addr {
            0x1FF0..=0x1FF3 => self.ram_read(s.baseptr + (s.address << 2) + (addr - 0x1FF0)),
            0x1FF4 => self.ram_read(s.baseptr + (s.address >> 2) + 0x200),
            _ => self.ram_read(addr),
        }
    }

    fn write_register(&mut self, addr: u16, val: u8) {
        let s = self.status;
        match addr {
            0x1FF0..=0x1FF3 => self.ram_write(s.baseptr + (s.address << 2) + (addr - 0x1FF0), val),
            0x1FF4 => {
                let slot = s.baseptr + (s.address >> 2) + 0x200;
                let old = self.ram_read(slot);
                let merged = (old & !(3 << s.shift)) | ((val & 3) << s.shift);
                self.ram_write(slot, merged);
            }
            0x1FF5 => {
                self.status.baseptr = if val & 1 != 0 { 0x1800 } else { 0x1C00 };
                self.ram_write(addr, val);
            }
            0x1FF6 => {
                self.status.address = u16::from(val & 0x7F);
                self.status.shift = u16::from(val & 3) << 1;
                self.ram_write(addr, val);
            }
            _ => self.ram_write(addr, val),
        }
    }
}

impl Board for Obc1Board {
    fn name(&self) -> &'static str {
        "LoROM+OBC1"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::Obc1
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        if classify(addr24).is_some() {
            MappedAddr::Coprocessor
        } else {
            self.inner.map(addr24)
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        match classify(addr24) {
            Some(a) => self.read_register(a),
            None => self.inner.read24(addr24),
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        if let Some(a) = classify(addr24) {
            self.write_register(a, val);
        } else {
            self.inner.write24(addr24, val);
        }
    }

    fn rom(&self) -> &[u8] {
        self.inner.rom()
    }

    // OBC1's dedicated RAM IS the save data (ares board type=RAM content=Save) — expose it
    // through the standard SRAM hooks so the existing save-file load/store path just works,
    // rather than the cartridge's own (usually absent) SRAM.
    fn sram(&self) -> &[u8] {
        &self.ram
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        &mut self.ram
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::LoRom;
    use alloc::vec;

    fn board() -> Obc1Board {
        let inner = Box::new(LoRom::new(
            vec![0u8; 0x8_0000].into_boxed_slice(),
            vec![].into_boxed_slice(),
        ));
        Obc1Board::new(inner)
    }

    #[test]
    fn window_classify() {
        assert_eq!(classify(0x00_6000), Some(0x0000));
        assert_eq!(classify(0x3F_7FFF), Some(0x1FFF));
        assert_eq!(classify(0x70_6000), Some(0x0000));
        assert_eq!(classify(0xF1_E000), Some(0x0000));
        assert_eq!(classify(0x00_8000), None); // ROM, not OBC1
    }

    #[test]
    fn slot_read_write_roundtrip() {
        let mut b = board();
        // Select slot 5, baseptr default $1C00 (ram[$1FF5] bit0 == 0 at power-on).
        b.write24(0x00_7FF6, 5);
        b.write24(0x00_7FF0, 0xAA);
        b.write24(0x00_7FF1, 0xBB);
        assert_eq!(b.read24(0x00_7FF0), 0xAA);
        assert_eq!(b.read24(0x00_7FF1), 0xBB);
        // Directly verify the slot landed at baseptr + (5<<2) + 0/1.
        assert_eq!(b.ram[0x1C00 + (5 << 2)], 0xAA);
        assert_eq!(b.ram[0x1C00 + (5 << 2) + 1], 0xBB);
    }

    #[test]
    fn packed_slot_read_modify_write() {
        let mut b = board();
        b.write24(0x00_7FF6, 3); // address=3, shift=(3&3)<<1=6
        b.write24(0x00_7FF4, 0b11); // writes bits [7:6] of the packed byte
        let slot = 0x1C00 + 0x200; // (address=3) >> 2 == 0
        assert_eq!(b.ram[slot], 0b1100_0000);
    }

    #[test]
    fn baseptr_toggle() {
        let mut b = board();
        b.write24(0x00_7FF5, 1);
        assert_eq!(b.status.baseptr, 0x1800);
        b.write24(0x00_7FF5, 0);
        assert_eq!(b.status.baseptr, 0x1C00);
    }
}
