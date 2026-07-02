//! The SPC7110 board — Hudson's decompression + memory-mapping ASIC (Far East of Eden Zero /
//! Tengai Makyou Zero).
//!
//! Clean-room port of ares' `SPC7110` component (ISC, `sfc/coprocessor/spc7110/`): a decompression
//! unit (DCU, an adaptive binary range coder over 1/2/4bpp tile planes — `decompressor`), a data
//! port unit (a seekable data-ROM cursor with auto-increment), an arithmetic-logic unit (16x16
//! multiply, 32/16 divide), and a memory-control unit (four independently-bankable 1 MiB data-ROM
//! windows). There is no chip-ROM dump for the DCU/data-port/ALU — the algorithm runs entirely
//! against the cart's own PROM/DROM (`docs/adr/0003`).
//!
//! SPC7110 owns its ROM/RAM mapping directly (like S-DD1/Super FX/SA-1): its `$00-3F,$80-BF:8000-
//! FFFF` + `$C0-FF:0000-FFFF` window folds to a UNIFIED linear data-ROM address (`(bank & 0x3F) <<
//! 16 | offset`, ares `mcuromRead`'s doc comment) that a plain LoROM/HiROM board's fold can't
//! express, and the register window additionally mirrors onto whole banks `$50`/`$58`.
//!
//! Cartridge geometry: unlike every other coprocessor here, SPC7110 carts physically carry TWO
//! ROM chips — a small plain "PROM" (CPU-executable program code, LoROM-style banks `00-0F`) and a
//! much larger "DROM" (compressed/plain data, addressed only through this board's registers). A
//! combined `.sfc` dump concatenates PROM then DROM. There is no header field or generic formula
//! that recovers the split for every SPC7110 title (this project has exactly one SPC7110 ROM to
//! validate against, mirroring the single-title validation basis already used for CX4/DSP-4/ST010/
//! OBC1): [`select`] uses the split that matches Far East of Eden Zero's known physical cartridge
//! (1 MiB PROM + the remainder as DROM) when the ROM is at least 2 MiB, else treats the whole
//! image as DROM (the "no PROM" SPC7110 layout used by titles like Momotarou Dentetsu VII).
//!
//! Bus window (bank:addr):
//!
//! | Region | Target |
//! |---|---|
//! | `$00-3F,$80-BF:$4800-$483F`, whole bank `$50`, whole bank `$58` | registers (DCU/data-port/ALU/memory-control) |
//! | `$00-3F,$80-BF:$8000-FFFF`, `$40-$7D`/`$C0-FF:$0000-FFFF` | PROM (if present) or DROM, banked via `$4830-$4833` + `$4834`; `$40-$7D` mirrors `$C0-FF` (standard HiROM fold — confirmed against Far East of Eden Zero's real boot code, which does execute from this range) |
//! | `$00-3F,$80-BF:$6000-$7FFF` | battery SRAM (folds to banks `00-07`), gated on `$4830` bit 7 |

// Chip-name jargon (SPC7110, DCU, MCU, ...) is not Rust code; the register file is naturally dense
// with small bitfields ported verbatim from ares.
#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::cast_possible_truncation
)]

use alloc::boxed::Box;
use alloc::vec;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::coproc::epsonrtc::EpsonRtc;
use crate::header::MapMode;

mod decompressor;
use decompressor::Decompressor;

/// A cartridge carrying an SPC7110 (owns its ROM/RAM mapping directly — see the module doc).
pub struct Spc7110Board {
    prom: Box<[u8]>,
    drom: Box<[u8]>,
    ram: Box<[u8]>,

    // decompression unit
    r4801: u8,
    r4802: u8,
    r4803: u8,
    r4804: u8,
    r4805: u8,
    r4806: u8,
    r4807: u8,
    r4809: u8,
    r480a: u8,
    r480b: u8,
    r480c: u8,
    dcu_mode: u8,
    dcu_address: u32,
    dcu_offset: u32,
    dcu_tile: [u8; 32],
    decompressor: Decompressor,

    // data port unit
    r4810: u8,
    r4811: u8,
    r4812: u8,
    r4813: u8,
    r4814: u8,
    r4815: u8,
    r4816: u8,
    r4817: u8,
    r4818: u8,

    // arithmetic logic unit
    r4820: u8,
    r4821: u8,
    r4822: u8,
    r4823: u8,
    r4824: u8,
    r4825: u8,
    r4826: u8,
    r4827: u8,
    r4828: u8,
    r4829: u8,
    r482a: u8,
    r482b: u8,
    r482c: u8,
    r482d: u8,
    r482e: u8,
    r482f: u8,

    // memory control unit
    r4830: u8,
    r4831: u8,
    r4832: u8,
    r4833: u8,
    r4834: u8,

    /// The Epson RTC-4513 fitted alongside SPC7110 on exactly one commercial cart (Far East of
    /// Eden Zero — see `coproc::epsonrtc`'s module doc). Present unconditionally: titles without
    /// an RTC simply never address `$4840-$4842`, so it stays inert and harmless.
    rtc: EpsonRtc,
}

impl core::fmt::Debug for Spc7110Board {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Spc7110Board")
            .field("prom_len", &self.prom.len())
            .field("drom_len", &self.drom.len())
            .finish_non_exhaustive()
    }
}

impl Spc7110Board {
    /// Build an SPC7110 board directly from the cart's split PROM/DROM + header SRAM size.
    #[must_use]
    pub fn new(prom: Box<[u8]>, drom: Box<[u8]>, sram_size: usize) -> Self {
        Self {
            prom,
            drom,
            ram: vec![0u8; sram_size].into_boxed_slice(),
            r4801: 0,
            r4802: 0,
            r4803: 0,
            r4804: 0,
            r4805: 0,
            r4806: 0,
            r4807: 0,
            r4809: 0,
            r480a: 0,
            r480b: 0,
            r480c: 0,
            dcu_mode: 0,
            dcu_address: 0,
            dcu_offset: 0,
            dcu_tile: [0; 32],
            decompressor: Decompressor::new(),
            r4810: 0,
            r4811: 0,
            r4812: 0,
            r4813: 0,
            r4814: 0,
            r4815: 0,
            r4816: 0,
            r4817: 0,
            r4818: 0,
            r4820: 0,
            r4821: 0,
            r4822: 0,
            r4823: 0,
            r4824: 0,
            r4825: 0,
            r4826: 0,
            r4827: 0,
            r4828: 0,
            r4829: 0,
            r482a: 0,
            r482b: 0,
            r482c: 0,
            r482d: 0,
            r482e: 0,
            r482f: 0,
            r4830: 0,
            r4831: 0,
            r4832: 0x01,
            r4833: 0x02,
            r4834: 0,
            rtc: EpsonRtc::new(),
        }
    }

    // ============================
    // data ROM (ares data.cpp)
    // ============================

    fn datarom_read(&self, address: u32) -> u8 {
        let size = 1u32 << (self.r4834 & 3); // in MiB
        let mask = 0x0010_0000u32.wrapping_mul(size).wrapping_sub(1);
        let offset = address & mask;
        if (self.r4834 & 3) != 3 && (address & 0x40_0000) != 0 {
            return 0;
        }
        self.drom
            .get((offset as usize) % self.drom.len().max(1))
            .copied()
            .unwrap_or(0)
    }

    fn data_offset(&self) -> u32 {
        u32::from(self.r4811) | (u32::from(self.r4812) << 8) | (u32::from(self.r4813) << 16)
    }

    fn data_adjust(&self) -> u16 {
        u16::from(self.r4814) | (u16::from(self.r4815) << 8)
    }

    fn data_stride(&self) -> u16 {
        u16::from(self.r4816) | (u16::from(self.r4817) << 8)
    }

    const fn set_data_offset(&mut self, address: u32) {
        self.r4811 = address as u8;
        self.r4812 = (address >> 8) as u8;
        self.r4813 = (address >> 16) as u8;
    }

    const fn set_data_adjust(&mut self, address: u16) {
        self.r4814 = address as u8;
        self.r4815 = (address >> 8) as u8;
    }

    fn data_port_read(&mut self) {
        let offset = self.data_offset();
        let mut adjust = if self.r4818 & 2 != 0 {
            u32::from(self.data_adjust())
        } else {
            0
        };
        if self.r4818 & 8 != 0 {
            adjust = sign_extend16(adjust as u16);
        }
        self.r4810 = self.datarom_read(offset.wrapping_add(adjust));
    }

    fn data_port_increment_4810(&mut self) {
        let offset = self.data_offset();
        let mut stride = if self.r4818 & 1 != 0 {
            u32::from(self.data_stride())
        } else {
            1
        };
        let mut adjust = u32::from(self.data_adjust());
        if self.r4818 & 4 != 0 {
            stride = sign_extend16(stride as u16);
        }
        if self.r4818 & 8 != 0 {
            adjust = sign_extend16(adjust as u16);
        }
        if self.r4818 & 16 == 0 {
            self.set_data_offset(offset.wrapping_add(stride));
        } else {
            self.set_data_adjust(adjust.wrapping_add(stride) as u16);
        }
        self.data_port_read();
    }

    fn data_port_increment_seek(&mut self, select: u8) {
        if self.r4818 >> 5 != select {
            return;
        }
        let offset = self.data_offset();
        let mut adjust = u32::from(self.data_adjust());
        if self.r4818 & 8 != 0 {
            adjust = sign_extend16(adjust as u16);
        }
        self.set_data_offset(offset.wrapping_add(adjust));
        self.data_port_read();
    }

    // ============================
    // decompression unit (ares dcu.cpp)
    // ============================

    fn dcu_load_address(&mut self) {
        let table =
            u32::from(self.r4801) | (u32::from(self.r4802) << 8) | (u32::from(self.r4803) << 16);
        let index = u32::from(self.r4804) << 2;
        let address = table.wrapping_add(index);
        self.dcu_mode = self.datarom_read(address);
        self.dcu_address = (u32::from(self.datarom_read(address.wrapping_add(1))) << 16)
            | (u32::from(self.datarom_read(address.wrapping_add(2))) << 8)
            | u32::from(self.datarom_read(address.wrapping_add(3)));
    }

    fn dcu_begin_transfer(&mut self) {
        if self.dcu_mode == 3 {
            return; // invalid mode
        }
        let origin = self.dcu_address;
        self.decompressor
            .initialize(u32::from(self.dcu_mode), origin, |o| {
                Self::datarom_read_static(&self.drom, self.r4834, o)
            });
        self.decompress_one();
        let seek: u16 = if self.r480b & 2 != 0 {
            u16::from(self.r4805) | (u16::from(self.r4806) << 8)
        } else {
            0
        };
        for _ in 0..seek {
            self.decompress_one();
        }
        self.r480c |= 0x80;
        self.dcu_offset = 0;
    }

    fn decompress_one(&mut self) {
        let drom = &self.drom;
        let r4834 = self.r4834;
        self.decompressor
            .decode(|o| Self::datarom_read_static(drom, r4834, o));
    }

    /// [`Self::datarom_read`] without a `&self` borrow, for use inside closures that also need to
    /// mutate `self.decompressor` (see [`Self::dcu_begin_transfer`]/[`Self::decompress_one`]).
    fn datarom_read_static(drom: &[u8], r4834: u8, address: u32) -> u8 {
        let size = 1u32 << (r4834 & 3);
        let mask = 0x0010_0000u32.wrapping_mul(size).wrapping_sub(1);
        let offset = address & mask;
        if (r4834 & 3) != 3 && (address & 0x40_0000) != 0 {
            return 0;
        }
        drom.get((offset as usize) % drom.len().max(1))
            .copied()
            .unwrap_or(0)
    }

    fn dcu_read(&mut self) -> u8 {
        if self.r480c & 0x80 == 0 {
            return 0;
        }

        if self.dcu_offset == 0 {
            for row in 0..8usize {
                match self.decompressor.bpp {
                    1 => self.dcu_tile[row] = self.decompressor.result as u8,
                    2 => {
                        self.dcu_tile[row * 2] = self.decompressor.result as u8;
                        self.dcu_tile[row * 2 + 1] = (self.decompressor.result >> 8) as u8;
                    }
                    4 => {
                        self.dcu_tile[row * 2] = self.decompressor.result as u8;
                        self.dcu_tile[row * 2 + 1] = (self.decompressor.result >> 8) as u8;
                        self.dcu_tile[row * 2 + 16] = (self.decompressor.result >> 16) as u8;
                        self.dcu_tile[row * 2 + 17] = (self.decompressor.result >> 24) as u8;
                    }
                    _ => {}
                }
                let seek = if self.r480b & 1 != 0 { self.r4807 } else { 1 };
                for _ in 0..seek {
                    self.decompress_one();
                }
            }
        }

        let data = self.dcu_tile[self.dcu_offset as usize];
        self.dcu_offset += 1;
        self.dcu_offset &= 8 * self.decompressor.bpp - 1;
        data
    }

    // ============================
    // arithmetic logic unit (ares alu.cpp)
    // ============================

    fn alu_multiply(&mut self) {
        let result: u32 = if self.r482e & 1 != 0 {
            let r0 = i32::from(i16::from_le_bytes([self.r4824, self.r4825]));
            let r1 = i32::from(i16::from_le_bytes([self.r4820, self.r4821]));
            r0.wrapping_mul(r1).cast_unsigned()
        } else {
            let r0 = u32::from(u16::from_le_bytes([self.r4824, self.r4825]));
            let r1 = u32::from(u16::from_le_bytes([self.r4820, self.r4821]));
            r0.wrapping_mul(r1)
        };
        self.r4828 = result as u8;
        self.r4829 = (result >> 8) as u8;
        self.r482a = (result >> 16) as u8;
        self.r482b = (result >> 24) as u8;
        self.r482f &= 0x7f;
    }

    fn alu_divide(&mut self) {
        let (quotient, remainder): (u32, u16) = if self.r482e & 1 != 0 {
            let dividend = i32::from_le_bytes([self.r4820, self.r4821, self.r4822, self.r4823]);
            let divisor = i16::from_le_bytes([self.r4826, self.r4827]);
            if divisor == 0 {
                (0, dividend.cast_unsigned() as u16)
            } else {
                (
                    dividend.wrapping_div(i32::from(divisor)).cast_unsigned(),
                    dividend.wrapping_rem(i32::from(divisor)).cast_unsigned() as u16,
                )
            }
        } else {
            let dividend = u32::from_le_bytes([self.r4820, self.r4821, self.r4822, self.r4823]);
            let divisor = u16::from_le_bytes([self.r4826, self.r4827]);
            if divisor == 0 {
                (0, dividend as u16)
            } else {
                (
                    dividend / u32::from(divisor),
                    (dividend % u32::from(divisor)) as u16,
                )
            }
        };
        self.r4828 = quotient as u8;
        self.r4829 = (quotient >> 8) as u8;
        self.r482a = (quotient >> 16) as u8;
        self.r482b = (quotient >> 24) as u8;
        self.r482c = remainder as u8;
        self.r482d = (remainder >> 8) as u8;
        self.r482f &= 0x7f;
    }

    // ============================
    // memory control unit (ares spc7110.cpp mcuromRead/mcuramRead)
    // ============================

    /// The unified linear data-ROM address ares' `mcuromRead` doc comment describes: banks
    /// `$00-3F,$80-BF` at `$8000-FFFF` and banks `$C0-FF` at `$0000-FFFF` both fold to
    /// `(bank & 0x3F) << 16 | offset`.
    const fn mcurom_linear(bank: u32, offset: u32) -> u32 {
        ((bank & 0x3F) << 16) | offset
    }

    fn mcurom_read(&self, linear: u32) -> u8 {
        if linear < 0x10_0000 {
            let a = linear & 0x0f_ffff;
            if !self.prom.is_empty() {
                return self.prom[(a as usize) % self.prom.len()];
            }
            return self.datarom_read(a | (0x10_0000 * u32::from(self.r4830 & 7)));
        }
        if linear < 0x20_0000 {
            let a = linear & 0x0f_ffff;
            if self.r4834 & 4 != 0 && !self.prom.is_empty() {
                return self.prom[((0x10_0000 + a) as usize) % self.prom.len()];
            }
            return self.datarom_read(a | (0x10_0000 * u32::from(self.r4831 & 7)));
        }
        if linear < 0x30_0000 {
            let a = linear & 0x0f_ffff;
            return self.datarom_read(a | (0x10_0000 * u32::from(self.r4832 & 7)));
        }
        if linear < 0x40_0000 {
            let a = linear & 0x0f_ffff;
            return self.datarom_read(a | (0x10_0000 * u32::from(self.r4833 & 7)));
        }
        0
    }

    fn mcuram_addr(&self, bank: u32, offset_in_window: u32) -> usize {
        let a = ((bank & 0x07) << 13) | (offset_in_window & 0x1FFF);
        (a as usize) % self.ram.len().max(1)
    }

    // ============================
    // registers (ares spc7110.cpp read/write)
    // ============================

    /// Normalize a raw 24-bit CPU address to the `$4800-$483F` register space, folding the
    /// bank-`$50`/`$58` full-bank mirrors (ares `read`/`write`'s shared prologue).
    const fn reg_addr(addr24: u32) -> u16 {
        let bank = (addr24 >> 16) & 0xff;
        if bank == 0x50 {
            return 0x4800;
        }
        if bank == 0x58 {
            return 0x4808;
        }
        (0x4800 | (addr24 & 0x3f)) as u16
    }

    fn read_reg(&mut self, addr: u16) -> u8 {
        match addr {
            0x4800 => {
                let counter = u16::from(self.r4809) | (u16::from(self.r480a) << 8);
                let counter = counter.wrapping_sub(1);
                self.r4809 = counter as u8;
                self.r480a = (counter >> 8) as u8;
                self.dcu_read()
            }
            0x4801 => self.r4801,
            0x4802 => self.r4802,
            0x4803 => self.r4803,
            0x4804 => self.r4804,
            0x4805 => self.r4805,
            0x4806 => self.r4806,
            0x4807 => self.r4807,
            // $4808 always reads 0 (ares `case 0x4808: return 0x00;`) — same as the wildcard.
            0x4809 => self.r4809,
            0x480a => self.r480a,
            0x480b => self.r480b,
            0x480c => self.r480c,
            0x4810 => {
                let data = self.r4810;
                self.data_port_increment_4810();
                data
            }
            0x4811 => self.r4811,
            0x4812 => self.r4812,
            0x4813 => self.r4813,
            0x4814 => self.r4814,
            0x4815 => self.r4815,
            0x4816 => self.r4816,
            0x4817 => self.r4817,
            0x4818 => self.r4818,
            0x481a => {
                self.data_port_increment_seek(3);
                0
            }
            0x4820 => self.r4820,
            0x4821 => self.r4821,
            0x4822 => self.r4822,
            0x4823 => self.r4823,
            0x4824 => self.r4824,
            0x4825 => self.r4825,
            0x4826 => self.r4826,
            0x4827 => self.r4827,
            0x4828 => self.r4828,
            0x4829 => self.r4829,
            0x482a => self.r482a,
            0x482b => self.r482b,
            0x482c => self.r482c,
            0x482d => self.r482d,
            0x482e => self.r482e,
            0x482f => self.r482f,
            0x4830 => self.r4830,
            0x4831 => self.r4831,
            0x4832 => self.r4832,
            0x4833 => self.r4833,
            0x4834 => self.r4834,
            _ => 0,
        }
    }

    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0x4801 => self.r4801 = data,
            0x4802 => self.r4802 = data,
            0x4803 => self.r4803 = data & 0x7f,
            0x4804 => {
                self.r4804 = data;
                self.dcu_load_address();
            }
            0x4805 => self.r4805 = data,
            0x4806 => {
                self.r4806 = data;
                self.r480c &= 0x7f;
                self.dcu_begin_transfer();
            }
            0x4807 => self.r4807 = data,
            0x4809 => self.r4809 = data,
            0x480a => self.r480a = data,
            0x480b => self.r480b = data & 0x03,
            0x4811 => self.r4811 = data,
            0x4812 => self.r4812 = data,
            0x4813 => {
                self.r4813 = data & 0x7f;
                self.data_port_read();
            }
            0x4814 => {
                self.r4814 = data;
                self.data_port_increment_seek(1);
            }
            0x4815 => {
                self.r4815 = data;
                if self.r4818 & 2 != 0 {
                    self.data_port_read();
                }
                self.data_port_increment_seek(2);
            }
            0x4816 => self.r4816 = data,
            0x4817 => self.r4817 = data,
            0x4818 => {
                self.r4818 = data & 0x7f;
                self.data_port_read();
            }
            0x4820 => self.r4820 = data,
            0x4821 => self.r4821 = data,
            0x4822 => self.r4822 = data,
            0x4823 => self.r4823 = data,
            0x4824 => self.r4824 = data,
            0x4825 => {
                self.r4825 = data;
                self.r482f |= 0x81;
                self.alu_multiply();
            }
            0x4826 => self.r4826 = data,
            0x4827 => {
                self.r4827 = data;
                self.r482f |= 0x80;
                self.alu_divide();
            }
            0x482e => self.r482e = data & 0x01,
            0x4830 => self.r4830 = data & 0x87,
            0x4831 => self.r4831 = data & 0x07,
            0x4832 => self.r4832 = data & 0x07,
            0x4833 => self.r4833 = data & 0x07,
            0x4834 => self.r4834 = data & 0x07,
            _ => {}
        }
    }
}

impl Spc7110Board {
    /// Write every register across the DCU/data-port/ALU/memory-control units, the DCU's
    /// `dcu_tile` scratch buffer, the decompressor's mid-stream state, and the paired RTC into an
    /// `"SP70"` section. PROM/DROM/battery-SRAM are never written (`System::save_state` captures
    /// SRAM separately via `Board::sram`; ROM is never embedded, `docs/adr/0003`).
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"SP70", |s| {
            s.write_u8(self.r4801);
            s.write_u8(self.r4802);
            s.write_u8(self.r4803);
            s.write_u8(self.r4804);
            s.write_u8(self.r4805);
            s.write_u8(self.r4806);
            s.write_u8(self.r4807);
            s.write_u8(self.r4809);
            s.write_u8(self.r480a);
            s.write_u8(self.r480b);
            s.write_u8(self.r480c);
            s.write_u8(self.dcu_mode);
            s.write_u32(self.dcu_address);
            s.write_u32(self.dcu_offset);
            s.write_bytes(&self.dcu_tile);
            s.write_u8(self.r4810);
            s.write_u8(self.r4811);
            s.write_u8(self.r4812);
            s.write_u8(self.r4813);
            s.write_u8(self.r4814);
            s.write_u8(self.r4815);
            s.write_u8(self.r4816);
            s.write_u8(self.r4817);
            s.write_u8(self.r4818);
            s.write_u8(self.r4820);
            s.write_u8(self.r4821);
            s.write_u8(self.r4822);
            s.write_u8(self.r4823);
            s.write_u8(self.r4824);
            s.write_u8(self.r4825);
            s.write_u8(self.r4826);
            s.write_u8(self.r4827);
            s.write_u8(self.r4828);
            s.write_u8(self.r4829);
            s.write_u8(self.r482a);
            s.write_u8(self.r482b);
            s.write_u8(self.r482c);
            s.write_u8(self.r482d);
            s.write_u8(self.r482e);
            s.write_u8(self.r482f);
            s.write_u8(self.r4830);
            s.write_u8(self.r4831);
            s.write_u8(self.r4832);
            s.write_u8(self.r4833);
            s.write_u8(self.r4834);
        });
        self.decompressor.save_state(w);
        self.rtc.save_state(w);
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input, a section with unconsumed trailing bytes,
    /// or whatever `Decompressor::load_state`/`EpsonRtc::load_state` themselves reject.
    /// `dcu_offset` is masked to `& 31`: it indexes the fixed 32-byte `dcu_tile` directly
    /// (`dcu_tile[self.dcu_offset as usize]`), and every normal-operation mutator already masks
    /// it to `8 * bpp - 1` (at most 31) before storing, so this applies the engine's own
    /// existing invariant rather than new validation policy. Every other register here is
    /// restored verbatim (unmasked): each is already only ever used in ways that can't index an
    /// array or overflow a shift by an unmasked amount, matching how `dcu_mode` itself (a raw
    /// data-ROM byte with no enforced range even during normal execution) is handled.
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"SP70")?;
        self.r4801 = s.read_u8()?;
        self.r4802 = s.read_u8()?;
        self.r4803 = s.read_u8()?;
        self.r4804 = s.read_u8()?;
        self.r4805 = s.read_u8()?;
        self.r4806 = s.read_u8()?;
        self.r4807 = s.read_u8()?;
        self.r4809 = s.read_u8()?;
        self.r480a = s.read_u8()?;
        self.r480b = s.read_u8()?;
        self.r480c = s.read_u8()?;
        self.dcu_mode = s.read_u8()?;
        self.dcu_address = s.read_u32()?;
        self.dcu_offset = s.read_u32()? & 31;
        self.dcu_tile.copy_from_slice(s.read_bytes(32)?);
        self.r4810 = s.read_u8()?;
        self.r4811 = s.read_u8()?;
        self.r4812 = s.read_u8()?;
        self.r4813 = s.read_u8()?;
        self.r4814 = s.read_u8()?;
        self.r4815 = s.read_u8()?;
        self.r4816 = s.read_u8()?;
        self.r4817 = s.read_u8()?;
        self.r4818 = s.read_u8()?;
        self.r4820 = s.read_u8()?;
        self.r4821 = s.read_u8()?;
        self.r4822 = s.read_u8()?;
        self.r4823 = s.read_u8()?;
        self.r4824 = s.read_u8()?;
        self.r4825 = s.read_u8()?;
        self.r4826 = s.read_u8()?;
        self.r4827 = s.read_u8()?;
        self.r4828 = s.read_u8()?;
        self.r4829 = s.read_u8()?;
        self.r482a = s.read_u8()?;
        self.r482b = s.read_u8()?;
        self.r482c = s.read_u8()?;
        self.r482d = s.read_u8()?;
        self.r482e = s.read_u8()?;
        self.r482f = s.read_u8()?;
        self.r4830 = s.read_u8()?;
        self.r4831 = s.read_u8()?;
        self.r4832 = s.read_u8()?;
        self.r4833 = s.read_u8()?;
        self.r4834 = s.read_u8()?;
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "SP70 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        self.decompressor.load_state(r)?;
        self.rtc.load_state(r)
    }
}

/// Sign-extend a 16-bit value into a `u32` (ares `(i16)adjust`/`(i16)stride` idiom).
const fn sign_extend16(v: u16) -> u32 {
    (v.cast_signed() as i32).cast_unsigned()
}

impl Board for Spc7110Board {
    fn name(&self) -> &'static str {
        "HiROM+SPC7110"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::Spc7110
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        let a = addr24 & 0xff_ffff;
        let bank = (a >> 16) & 0xff;
        let off = a & 0xffff;
        if bank == 0x50 || bank == 0x58 {
            return MappedAddr::Coprocessor;
        }
        if matches!(bank, 0x00..=0x3f | 0x80..=0xbf) {
            if (0x4800..=0x483f).contains(&off) || (0x4840..=0x4842).contains(&off) {
                return MappedAddr::Coprocessor;
            }
            if (0x6000..=0x7fff).contains(&off) {
                return MappedAddr::Sram(0);
            }
            if off >= 0x8000 {
                return MappedAddr::Rom(a);
            }
        }
        if matches!(bank, 0x40..=0x7d | 0xc0..=0xff) {
            return MappedAddr::Rom(a);
        }
        MappedAddr::Open
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        let a = addr24 & 0xff_ffff;
        let bank = (a >> 16) & 0xff;
        let off = a & 0xffff;

        if bank == 0x50 || bank == 0x58 {
            return self.read_reg(Self::reg_addr(a));
        }
        if matches!(bank, 0x00..=0x3f | 0x80..=0xbf) {
            if (0x4800..=0x483f).contains(&off) {
                return self.read_reg(Self::reg_addr(a));
            }
            if (0x4840..=0x4842).contains(&off) {
                return self.rtc.read(off - 0x4840);
            }
            if (0x6000..=0x7fff).contains(&off) {
                if self.r4830 & 0x80 == 0 {
                    return 0;
                }
                let idx = self.mcuram_addr(bank, off - 0x6000);
                return self.ram[idx];
            }
            if off >= 0x8000 {
                return self.mcurom_read(Self::mcurom_linear(bank, off));
            }
        }
        if matches!(bank, 0x40..=0x7d | 0xc0..=0xff) {
            return self.mcurom_read(Self::mcurom_linear(bank, off));
        }
        0
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        let a = addr24 & 0xff_ffff;
        let bank = (a >> 16) & 0xff;
        let off = a & 0xffff;

        if bank == 0x50 || bank == 0x58 {
            self.write_reg(Self::reg_addr(a), val);
            return;
        }
        if matches!(bank, 0x00..=0x3f | 0x80..=0xbf) {
            if (0x4800..=0x483f).contains(&off) {
                self.write_reg(Self::reg_addr(a), val);
                return;
            }
            if (0x4840..=0x4842).contains(&off) {
                self.rtc.write(off - 0x4840, val);
                return;
            }
            if (0x6000..=0x7fff).contains(&off) {
                if self.r4830 & 0x80 == 0 {
                    return;
                }
                let idx = self.mcuram_addr(bank, off - 0x6000);
                self.ram[idx] = val;
            }
            // ROM is read-only (ares `mcuromWrite` is a no-op).
        }
    }

    fn rom(&self) -> &[u8] {
        &self.drom
    }

    fn sram(&self) -> &[u8] {
        &self.ram
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        &mut self.ram
    }

    fn save_state(&self, w: &mut SaveWriter) {
        Self::save_state(self, w);
    }

    fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        Self::load_state(self, r)
    }
}

/// Build a [`Spc7110Board`] for a cart detected as SPC7110 (`board::select`).
///
/// Splits `rom` into PROM + DROM using Far East of Eden Zero's known physical geometry (see the
/// module doc); other SPC7110 titles' exact split is not derivable from a raw dump without an
/// external database.
#[must_use]
pub fn select(_map_mode: MapMode, rom: Box<[u8]>, sram_size: usize) -> Spc7110Board {
    const PROM_SIZE: usize = 0x10_0000;
    if rom.len() > PROM_SIZE {
        let (prom, drom) = rom.split_at(PROM_SIZE);
        Spc7110Board::new(
            prom.to_vec().into_boxed_slice(),
            drom.to_vec().into_boxed_slice(),
            sram_size,
        )
    } else {
        Spc7110Board::new(Box::new([]), rom, sram_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> Spc7110Board {
        Spc7110Board::new(
            vec![0u8; 0x10_0000].into_boxed_slice(),
            vec![0u8; 0x60_0000].into_boxed_slice(),
            0x2000,
        )
    }

    #[test]
    fn control_register_roundtrip() {
        let mut b = board();
        b.write24(0x00_4801, 0xAB);
        assert_eq!(b.read24(0x00_4801), 0xAB);
        b.write24(0x00_4830, 0xFF); // masked to 0x87
        assert_eq!(b.read24(0x00_4830), 0x87);
    }

    #[test]
    fn bank_50_and_58_mirror_the_data_port_and_dummy_register() {
        let mut b = board();
        // Bank $58 is always a dummy 0 read regardless of offset.
        assert_eq!(b.read24(0x58_1234), 0x00);
        // Bank $50 mirrors the decompression data-read port ($4800); with no active stream
        // (r480c bit7 clear) it returns 0 rather than panicking.
        assert_eq!(b.read24(0x50_ABCD), 0x00);
    }

    #[test]
    fn alu_multiply_unsigned() {
        let mut b = board();
        b.write24(0x00_4820, 5); // multiplicand low
        b.write24(0x00_4821, 0);
        b.write24(0x00_4824, 3); // multiplier low
        b.write24(0x00_4825, 0); // triggers the multiply
        assert_eq!(b.read24(0x00_4828), 15);
        assert_eq!(b.read24(0x00_4829), 0);
    }

    #[test]
    fn alu_divide_unsigned() {
        let mut b = board();
        b.write24(0x00_4820, 10);
        b.write24(0x00_4821, 0);
        b.write24(0x00_4822, 0);
        b.write24(0x00_4823, 0);
        b.write24(0x00_4826, 3);
        b.write24(0x00_4827, 0); // triggers the divide
        assert_eq!(b.read24(0x00_4828), 3); // quotient
        assert_eq!(b.read24(0x00_482c), 1); // remainder
    }

    #[test]
    fn mcurom_linear_folds_both_windows_to_the_same_space() {
        // bank 00 offset 8000-ffff and bank c0 offset 0000-ffff both land under 0x100000.
        assert!(Spc7110Board::mcurom_linear(0x00, 0x8000) < 0x10_0000);
        assert!(Spc7110Board::mcurom_linear(0xc0, 0x0000) < 0x10_0000);
    }

    #[test]
    fn register_and_rtc_state_round_trips_through_save_state() {
        let mut b = board();
        b.write24(0x00_4801, 0xAB);
        b.write24(0x00_4830, 0xFF); // masked to 0x87
        b.rtc.write(0, 1); // RTC chip select
        b.rtc.write(1, 0x03); // mode: write
        b.rtc.write(1, 0x00); // seek to offset 0
        b.rtc.write(1, 0x07); // write secondlo = 7

        let mut w = SaveWriter::new();
        b.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = board();
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.read24(0x00_4801), 0xAB);
        assert_eq!(fresh.read24(0x00_4830), 0x87);
        // Read the restored RTC clock field back out through its own public read/seek protocol
        // (mirrors epsonrtc.rs's own round-trip test — `secondlo` is private to that module).
        // Deselect then reselect first: the restored state is already mid-Write with chipselect
        // already 1, and a same-value chipselect write does NOT reset the state machine (matches
        // real hardware — see epsonrtc.rs's own round-trip test for the identical deselect step).
        fresh.rtc.write(0, 0);
        fresh.rtc.write(0, 1);
        fresh.rtc.write(1, 0x0c); // mode: read
        fresh.rtc.write(1, 0x00); // seek to offset 0
        assert_eq!(fresh.rtc.read(1), 7);
        assert_eq!(r.remaining(), 0);
    }
}
