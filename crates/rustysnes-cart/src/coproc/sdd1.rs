//! The S-DD1 board — Nintendo's lossless decompression ASIC (Star Ocean, Street Fighter Alpha 2).
//!
//! Clean-room port of ares' `SDD1` component (ISC, `sfc/coprocessor/sdd1/`): a Ricoh-style
//! Golomb-code + adaptive-binary-arithmetic entropy decoder that streams decompressed bytes
//! DURING a DMA transfer, not on a plain memory read. There is no chip-ROM dump — the algorithm
//! runs entirely against the cart's own (compressed) ROM data — so the board is functional the
//! moment the cart loads (`docs/adr/0003`).
//!
//! S-DD1 owns its ROM mapping directly (like Super FX / SA-1, not wrapped over a base board):
//! its bank-fold formula differs from plain LoROM/HiROM, and its DMA-decompression hook needs to
//! see the FULL `$00-3F,$80-BF:$8000-FFFF` + `$C0-FF:$0000-FFFF` read path uninterrupted.
//!
//! Bus window (bank:addr):
//!
//! | Region | Target |
//! |---|---|
//! | `$00-3F,$80-BF:$4800,$4801,$4804-$4807` | control registers (`$4802,$4803` fall through to ROM) |
//! | `$00-3F,$80-BF:$8000-FFFF` | banked ROM via the `$4804-$4807` MMC registers (plain read, `$20-3F`/`$A0-BF` A21 fold when `r4805`/`r4807` bit 7 set) |
//! | `$C0-FF:$0000-FFFF` | banked ROM via MMC, OR a live decompression stream if a DMA channel matching this address is armed ([`Board::notify_dma_channel`] is the DMA-address/size snoop; `rustysnes-core` calls it on every `$43n2-$43n6` write) |
//!
//! S-DD1 always uses DMA **fixed-address transfer mode**, so the source address never advances —
//! every byte of a transfer reads the SAME address, which really means "keep streaming from
//! here"; `Decompressor::init` is called once per DMA (on the first matching read), then
//! `Decompressor::read` once per subsequent byte.

// Chip-name jargon (S-DD1, MMC, Golomb, ...) is not Rust code; the entropy-coder state is
// naturally dense with small bitfields ported verbatim from ares' constant tables.
#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::cast_possible_truncation
)]

use alloc::boxed::Box;
use alloc::vec;

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::header::MapMode;

mod decompressor;
use decompressor::Decompressor;

/// One DMA channel's snooped source address + remaining byte count (`Board::notify_dma_channel`).
#[derive(Debug, Clone, Copy, Default)]
struct DmaSnoop {
    address: u32,
    size: u16,
}

/// A cartridge carrying an S-DD1 (owns its ROM mapping directly — see the module doc).
pub struct Sdd1Board {
    rom: Box<[u8]>,
    sram: Box<[u8]>,
    r4800: u8, // hard enable (per-channel bitmask)
    r4801: u8, // soft enable (per-channel bitmask)
    r4804: u8, // MMC bank for $C0-CF
    r4805: u8, // MMC bank for $D0-DF (+ bit7: fold $20-3F:8000-FFFF onto $00-1F)
    r4806: u8, // MMC bank for $E0-EF
    r4807: u8, // MMC bank for $F0-FF (+ bit7: fold $A0-BF:8000-FFFF onto $80-9F)
    dma: [DmaSnoop; 8],
    dma_ready: bool,
    decompressor: Decompressor,
}

impl core::fmt::Debug for Sdd1Board {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Sdd1Board")
            .field("rom_len", &self.rom.len())
            .field("r4800", &self.r4800)
            .field("r4801", &self.r4801)
            .finish_non_exhaustive()
    }
}

impl Sdd1Board {
    /// Build an S-DD1 board directly from the cart's raw ROM bytes + header SRAM size.
    #[must_use]
    pub fn new(rom: Box<[u8]>, sram_size: usize) -> Self {
        Self {
            rom,
            sram: vec![0u8; sram_size].into_boxed_slice(),
            r4800: 0,
            r4801: 0,
            r4804: 0x00,
            r4805: 0x01,
            r4806: 0x02,
            r4807: 0x03,
            dma: [DmaSnoop::default(); 8],
            dma_ready: false,
            decompressor: Decompressor::new(),
        }
    }

    /// The banked-ROM read every non-decompressing access resolves to (ares `mmcRead`): which of
    /// the four `$4804-$4807` MMC registers backs this address depends on bits 20-21.
    fn mmc_read(&self, address: u32) -> u8 {
        let bank_reg = match (address >> 20) & 0x3 {
            0 => self.r4804,
            1 => self.r4805,
            2 => self.r4806,
            _ => self.r4807,
        };
        let off = (u32::from(bank_reg & 0xF) << 20) | (address & 0x0F_FFFF);
        self.rom
            .get((off as usize) % self.rom.len().max(1))
            .copied()
            .unwrap_or(0)
    }

    /// The full S-CPU-facing ROM read (ares `mcuRead`): banks `$00-3F,$80-BF:$8000-FFFF` fold
    /// like LoROM (with the `r4805`/`r4807` bit-7 A21 quirk for the upper half); banks `$C0-FF`
    /// either stream decompression (if an armed DMA channel's snooped address matches) or fall
    /// through to `mmc_read`.
    fn mcu_read(&mut self, address: u32) -> u8 {
        let a = address & 0xFF_FFFF;
        if a & 0x40_0000 == 0 {
            // $00-3F,$80-BF:$8000-FFFF
            let mut a = a;
            if a & 0x80_0000 == 0 && a & 0x20_0000 != 0 && self.r4805 & 0x80 != 0 {
                a &= !0x20_0000; // 20-3f:8000-ffff fold
            }
            if a & 0x80_0000 != 0 && a & 0x20_0000 != 0 && self.r4807 & 0x80 != 0 {
                a &= !0x20_0000; // a0-bf:8000-ffff fold
            }
            let off = ((a >> 16) & 0x3F) << 15 | (a & 0x7FFF);
            return self
                .rom
                .get((off as usize) % self.rom.len().max(1))
                .copied()
                .unwrap_or(0);
        }

        // $C0-FF:$0000-FFFF
        if self.r4800 & self.r4801 != 0 {
            for n in 0..8usize {
                let armed = self.r4800 & (1 << n) != 0 && self.r4801 & (1 << n) != 0;
                if armed && self.dma[n].address & 0xFF_FFFF == a {
                    let mmc_map = self.mmc_map();
                    if !self.dma_ready {
                        self.decompressor.init(a, &self.rom, &mmc_map);
                        self.dma_ready = true;
                    }
                    let data = self.decompressor.read(&self.rom, &mmc_map);
                    self.dma[n].size = self.dma[n].size.wrapping_sub(1);
                    if self.dma[n].size == 0 {
                        self.dma_ready = false;
                        self.r4801 &= !(1 << n);
                    }
                    return data;
                }
            }
        }
        self.mmc_read(a)
    }

    const fn mmc_map(&self) -> [u8; 4] {
        [self.r4804, self.r4805, self.r4806, self.r4807]
    }
}

impl Board for Sdd1Board {
    fn name(&self) -> &'static str {
        "ExHiROM+S-DD1"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::SDd1
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        let a = addr24 & 0xFF_FFFF;
        let bank = (a >> 16) & 0xFF;
        let addr = a & 0xFFFF;
        let in_ctrl = matches!(bank, 0x00..=0x3F | 0x80..=0xBF)
            && matches!(addr, 0x4800 | 0x4801 | 0x4804..=0x4807);
        if in_ctrl {
            MappedAddr::Coprocessor
        } else {
            MappedAddr::Rom(a) // ROM/decompression, resolved in read24 (not a plain offset)
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        let a = addr24 & 0xFF_FFFF;
        let bank = (a >> 16) & 0xFF;
        let addr = a & 0xFFFF;
        if matches!(bank, 0x00..=0x3F | 0x80..=0xBF) {
            match addr {
                0x4800 => return self.r4800,
                0x4801 => return self.r4801,
                0x4804 => return self.r4804,
                0x4805 => return self.r4805,
                0x4806 => return self.r4806,
                0x4807 => return self.r4807,
                0x6000..=0x7FFF if !self.sram.is_empty() => {
                    let off = (((bank - if bank >= 0x80 { 0x80 } else { 0x00 }) as usize) * 0x2000
                        + (addr - 0x6000) as usize)
                        % self.sram.len();
                    return self.sram[off];
                }
                _ => {}
            }
        }
        self.mcu_read(a)
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        let a = addr24 & 0xFF_FFFF;
        let bank = (a >> 16) & 0xFF;
        let addr = a & 0xFFFF;
        if matches!(bank, 0x00..=0x3F | 0x80..=0xBF) {
            match addr {
                0x4800 => self.r4800 = val,
                0x4801 => self.r4801 = val,
                0x4804 => self.r4804 = val & 0x8F,
                0x4805 => self.r4805 = val & 0x8F,
                0x4806 => self.r4806 = val & 0x8F,
                0x4807 => self.r4807 = val & 0x8F,
                0x6000..=0x7FFF if !self.sram.is_empty() => {
                    let off = (((bank - if bank >= 0x80 { 0x80 } else { 0x00 }) as usize) * 0x2000
                        + (addr - 0x6000) as usize)
                        % self.sram.len();
                    self.sram[off] = val;
                }
                // ROM is read-only (ares `writeROM` is a no-op).
                _ => {}
            }
        }
    }

    fn rom(&self) -> &[u8] {
        &self.rom
    }

    fn sram(&self) -> &[u8] {
        &self.sram
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        &mut self.sram
    }

    fn notify_dma_channel(&mut self, channel: usize, address: u32, count: u16) {
        if let Some(c) = self.dma.get_mut(channel & 7) {
            c.address = address;
            c.size = count;
        }
    }
}

/// Build an [`Sdd1Board`] for a cart detected as S-DD1 (`board::select`).
#[must_use]
pub fn select(_map_mode: MapMode, rom: Box<[u8]>, sram_size: usize) -> Sdd1Board {
    Sdd1Board::new(rom, sram_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> Sdd1Board {
        Sdd1Board::new(vec![0u8; 0x40_0000].into_boxed_slice(), 0)
    }

    #[test]
    fn control_register_roundtrip() {
        let mut b = board();
        b.write24(0x00_4800, 0xAB);
        assert_eq!(b.read24(0x00_4800), 0xAB);
        b.write24(0x00_4804, 0xFF); // masked to 0x8F
        assert_eq!(b.read24(0x00_4804), 0x8F);
    }

    #[test]
    fn mmc_bank_selects_rom_quarter() {
        let mut b = board();
        b.rom[0x10_0000] = 0x55; // bank-1 (r4805 default) quarter, offset 0
        assert_eq!(b.mmc_read(0x10_0000), 0x55);
    }

    #[test]
    fn dma_snoop_arms_and_disarms_a_channel() {
        let mut b = board();
        b.notify_dma_channel(0, 0xC0_0000, 4);
        assert_eq!(b.dma[0].address, 0xC0_0000);
        assert_eq!(b.dma[0].size, 4);
    }
}
