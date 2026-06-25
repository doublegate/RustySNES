//! The `$2100`–`$213F` register file: writes ($2100–$2133) and reads ($2134–$213F).
//!
//! Re-implemented clean-room from `docs/ppu.md` + documented SNES hardware behavior. Notable
//! quirks modeled: the shared BG-offset write latch (PPU1/PPU2 halves), the Mode-7 / scroll
//! write-twice latch, VMAIN address remapping + increment-on-low/high, the CGRAM write-twice
//! latch and `$213B` read-twice, OAM even-byte latch, the VRAM read prefetch latch ($2139/A),
//! and the H/V counter latch ($2137 / $213C/D read-twice / $213F clears).

// reason: `write_reg`/`read_reg` are flat dispatch tables over the 64-entry register file; one
// `match` arm per register is the clearest possible shape and splitting it would scatter the
// hardware map across helpers. The length lint fires purely on that table size.
#![allow(clippy::too_many_lines)]

use crate::{Ppu, Region};

impl Ppu {
    /// Map the running VRAM address through the `VMAIN` remap, yielding the word index.
    const fn vram_mapped_address(&self) -> usize {
        let a = self.io.vram_address;
        let mapped = match self.io.vram_mapping {
            1 => (a & 0xff00) | ((a & 0x001f) << 3) | ((a >> 5) & 0x07),
            2 => (a & 0xfe00) | ((a & 0x003f) << 3) | ((a >> 6) & 0x07),
            3 => (a & 0xfc00) | ((a & 0x007f) << 3) | ((a >> 7) & 0x07),
            _ => a,
        };
        (mapped & 0x7fff) as usize
    }

    /// Whether VRAM/CGRAM/OAM is freely accessible (force-blank or outside active display).
    const fn vram_accessible(&self) -> bool {
        self.io.display_disable || self.v == 0 || self.v > self.visible_height()
    }

    fn vram_read_word(&self) -> u16 {
        self.vram[self.vram_mapped_address()]
    }

    const fn vram_step(&mut self) {
        self.io.vram_address = self
            .io
            .vram_address
            .wrapping_add(self.io.vram_increment_size);
    }

    /// Write one PPU register ($2100–$213F). Writes to read-only addresses are ignored.
    pub fn write_reg(&mut self, addr: u16, val: u8) {
        // Most writes update the PPU1 MDR open-bus latch.
        self.io.ppu1_mdr = val;
        let v = u16::from(val);
        match addr {
            // INIDISP
            0x2100 => {
                self.io.display_brightness = val & 0x0f;
                self.io.display_disable = val & 0x80 != 0;
            }
            // OBSEL
            0x2101 => {
                self.io.obj_tiledata_addr = u16::from(val & 0x07) << 13;
                self.io.obj_nameselect = u16::from((val >> 3) & 0x03);
                self.io.obj_base_size = (val >> 5) & 0x07;
            }
            // OAMADDL
            0x2102 => {
                self.io.oam_base_address = (self.io.oam_base_address & 0x0200) | (v << 1);
                self.io.oam_address = self.io.oam_base_address;
            }
            // OAMADDH
            0x2103 => {
                self.io.oam_base_address =
                    (self.io.oam_base_address & 0x01ff) | (u16::from(val & 0x01) << 9);
                self.io.oam_priority_rotation = val & 0x80 != 0;
                self.io.oam_address = self.io.oam_base_address;
            }
            // OAMDATA
            0x2104 => self.write_oamdata(val),
            // BGMODE
            0x2105 => {
                self.io.bg_mode = val & 0x07;
                self.io.bg3_priority = val & 0x08 != 0;
                self.io.tile_size[0] = val & 0x10 != 0;
                self.io.tile_size[1] = val & 0x20 != 0;
                self.io.tile_size[2] = val & 0x40 != 0;
                self.io.tile_size[3] = val & 0x80 != 0;
            }
            // MOSAIC
            0x2106 => {
                self.io.mosaic_enable[0] = val & 0x01 != 0;
                self.io.mosaic_enable[1] = val & 0x02 != 0;
                self.io.mosaic_enable[2] = val & 0x04 != 0;
                self.io.mosaic_enable[3] = val & 0x08 != 0;
                self.io.mosaic_size = ((val >> 4) & 0x0f) + 1;
            }
            // BGnSC
            0x2107..=0x210a => {
                let bg = (addr - 0x2107) as usize;
                self.io.bg_screen_size[bg] = val & 0x03;
                self.io.bg_screen_addr[bg] = u16::from(val & 0xfc) << 8;
            }
            // BG12NBA
            0x210b => {
                self.io.bg_tiledata_addr[0] = u16::from(val & 0x0f) << 12;
                self.io.bg_tiledata_addr[1] = u16::from((val >> 4) & 0x0f) << 12;
            }
            // BG34NBA
            0x210c => {
                self.io.bg_tiledata_addr[2] = u16::from(val & 0x0f) << 12;
                self.io.bg_tiledata_addr[3] = u16::from((val >> 4) & 0x0f) << 12;
            }
            // BGnHOFS / BGnVOFS (shared scroll write-latch)
            0x210d => self.write_bg_hofs(0, val, true),
            0x210e => self.write_bg_vofs(0, val, true),
            0x210f => self.write_bg_hofs(1, val, false),
            0x2110 => self.write_bg_vofs(1, val, false),
            0x2111 => self.write_bg_hofs(2, val, false),
            0x2112 => self.write_bg_vofs(2, val, false),
            0x2113 => self.write_bg_hofs(3, val, false),
            0x2114 => self.write_bg_vofs(3, val, false),
            // VMAIN
            0x2115 => {
                self.io.vram_increment_size = match val & 0x03 {
                    0 => 1,
                    1 => 32,
                    _ => 128,
                };
                self.io.vram_mapping = (val >> 2) & 0x03;
                self.io.vram_increment_high = val & 0x80 != 0;
            }
            // VMADDL
            0x2116 => {
                self.io.vram_address = (self.io.vram_address & 0xff00) | v;
                self.io.vram_read_latch = self.vram_read_word();
            }
            // VMADDH
            0x2117 => {
                self.io.vram_address = (self.io.vram_address & 0x00ff) | (v << 8);
                self.io.vram_read_latch = self.vram_read_word();
            }
            // VMDATAL
            0x2118 => {
                if self.vram_accessible() {
                    let idx = self.vram_mapped_address();
                    self.vram[idx] = (self.vram[idx] & 0xff00) | v;
                }
                if !self.io.vram_increment_high {
                    self.vram_step();
                }
            }
            // VMDATAH
            0x2119 => {
                if self.vram_accessible() {
                    let idx = self.vram_mapped_address();
                    self.vram[idx] = (self.vram[idx] & 0x00ff) | (v << 8);
                }
                if self.io.vram_increment_high {
                    self.vram_step();
                }
            }
            // M7SEL
            0x211a => {
                self.io.m7_hflip = val & 0x01 != 0;
                self.io.m7_vflip = val & 0x02 != 0;
                self.io.m7_repeat = (val >> 6) & 0x03;
            }
            // M7A..D, M7X/Y — write-twice via the shared mode7 latch
            0x211b => self.io.m7a = self.mode7_latch(val),
            0x211c => self.io.m7b = self.mode7_latch(val),
            0x211d => self.io.m7c = self.mode7_latch(val),
            0x211e => self.io.m7d = self.mode7_latch(val),
            0x211f => self.io.m7x = self.mode7_latch(val),
            0x2120 => self.io.m7y = self.mode7_latch(val),
            // CGADD
            0x2121 => {
                self.io.cgram_address = val;
                self.io.cgram_latch_high = false;
            }
            // CGDATA
            0x2122 => {
                if self.io.cgram_latch_high {
                    let word = (u16::from(val & 0x7f) << 8) | u16::from(self.io.cgram_byte_latch);
                    self.cgram[self.io.cgram_address as usize] = word;
                    self.io.cgram_address = self.io.cgram_address.wrapping_add(1);
                    self.io.cgram_latch_high = false;
                } else {
                    self.io.cgram_byte_latch = val;
                    self.io.cgram_latch_high = true;
                }
            }
            // Windows $2123..$2129 + masks $212A/B
            0x2123 => self.write_wsel(0, 1, val),
            0x2124 => self.write_wsel(2, 3, val),
            0x2125 => self.write_wsel(4, 5, val),
            0x2126 => self.io.win.one_left = val,
            0x2127 => self.io.win.one_right = val,
            0x2128 => self.io.win.two_left = val,
            0x2129 => self.io.win.two_right = val,
            0x212a => {
                self.io.win.layer[0].mask = val & 0x03;
                self.io.win.layer[1].mask = (val >> 2) & 0x03;
                self.io.win.layer[2].mask = (val >> 4) & 0x03;
                self.io.win.layer[3].mask = (val >> 6) & 0x03;
            }
            0x212b => {
                self.io.win.layer[4].mask = val & 0x03;
                self.io.win.layer[5].mask = (val >> 2) & 0x03;
            }
            // TM / TS
            0x212c => self.set_enable(&mut Self::enable_main, val),
            0x212d => self.set_enable(&mut Self::enable_sub, val),
            // TMW / TSW
            0x212e => self.set_enable(&mut Self::enable_win_main, val),
            0x212f => self.set_enable(&mut Self::enable_win_sub, val),
            // CGWSEL
            0x2130 => {
                self.io.direct_color = val & 0x01 != 0;
                self.io.add_subscreen = val & 0x02 != 0;
                self.io.color_window_below = (val >> 4) & 0x03;
                self.io.color_window_above = (val >> 6) & 0x03;
            }
            // CGADSUB
            0x2131 => {
                for (i, e) in self.io.color_math_enable.iter_mut().enumerate() {
                    *e = val & (1 << i) != 0;
                }
                self.io.color_halve = val & 0x40 != 0;
                self.io.color_subtract = val & 0x80 != 0;
            }
            // COLDATA
            0x2132 => {
                let intensity = u16::from(val & 0x1f);
                if val & 0x20 != 0 {
                    self.io.fixed_color = (self.io.fixed_color & !0x001f) | intensity;
                }
                if val & 0x40 != 0 {
                    self.io.fixed_color = (self.io.fixed_color & !0x03e0) | (intensity << 5);
                }
                if val & 0x80 != 0 {
                    self.io.fixed_color = (self.io.fixed_color & !0x7c00) | (intensity << 10);
                }
            }
            // SETINI
            0x2133 => {
                self.io.interlace = val & 0x01 != 0;
                self.io.obj_interlace = val & 0x02 != 0;
                self.io.overscan = val & 0x04 != 0;
                self.io.pseudo_hires = val & 0x08 != 0;
                self.io.extbg = val & 0x40 != 0;
            }
            _ => {}
        }
    }

    fn write_bg_hofs(&mut self, bg: usize, val: u8, is_bg1: bool) {
        let prev1 = self.bgofs_prev1;
        let prev2 = self.bgofs_prev2;
        self.io.bg_hofs[bg] = (u16::from(val) << 8) | (prev1 & !0x07) | (prev2 & 0x07);
        self.bgofs_prev1 = u16::from(val);
        self.bgofs_prev2 = u16::from(val);
        if is_bg1 {
            self.io.m7_hofs = (u16::from(val) << 8) | self.mode7_byte_latch;
            self.mode7_byte_latch = u16::from(val);
        }
    }

    fn write_bg_vofs(&mut self, bg: usize, val: u8, is_bg1: bool) {
        let prev1 = self.bgofs_prev1;
        self.io.bg_vofs[bg] = (u16::from(val) << 8) | prev1;
        self.bgofs_prev1 = u16::from(val);
        if is_bg1 {
            self.io.m7_vofs = (u16::from(val) << 8) | self.mode7_byte_latch;
            self.mode7_byte_latch = u16::from(val);
        }
    }

    fn mode7_latch(&mut self, val: u8) -> u16 {
        let out = (u16::from(val) << 8) | self.mode7_byte_latch;
        self.mode7_byte_latch = u16::from(val);
        out
    }

    const fn write_wsel(&mut self, la: usize, lb: usize, val: u8) {
        self.io.win.layer[la].one_invert = val & 0x01 != 0;
        self.io.win.layer[la].one_enable = val & 0x02 != 0;
        self.io.win.layer[la].two_invert = val & 0x04 != 0;
        self.io.win.layer[la].two_enable = val & 0x08 != 0;
        self.io.win.layer[lb].one_invert = val & 0x10 != 0;
        self.io.win.layer[lb].one_enable = val & 0x20 != 0;
        self.io.win.layer[lb].two_invert = val & 0x40 != 0;
        self.io.win.layer[lb].two_enable = val & 0x80 != 0;
    }

    const fn enable_main(io: &mut crate::Io, i: usize, b: bool) {
        io.main_enable[i] = b;
    }
    const fn enable_sub(io: &mut crate::Io, i: usize, b: bool) {
        io.sub_enable[i] = b;
    }
    const fn enable_win_main(io: &mut crate::Io, i: usize, b: bool) {
        io.win_main_enable[i] = b;
    }
    const fn enable_win_sub(io: &mut crate::Io, i: usize, b: bool) {
        io.win_sub_enable[i] = b;
    }

    fn set_enable(&mut self, f: &mut dyn FnMut(&mut crate::Io, usize, bool), val: u8) {
        for i in 0..5 {
            f(&mut self.io, i, val & (1 << i) != 0);
        }
    }

    const fn write_oamdata(&mut self, val: u8) {
        let addr = self.io.oam_address & 0x03ff;
        let even = addr & 1 == 0;
        if addr >= 0x200 {
            // High table (32 bytes mirrored every 0x20).
            self.oam[(0x200 + (addr & 0x1f)) as usize] = val;
        } else if even {
            self.io.oam_byte_latch = val;
        } else {
            // On the odd write, both the latched even byte and this byte commit.
            let base = (addr & !1) as usize;
            self.oam[base] = self.io.oam_byte_latch;
            self.oam[base + 1] = val;
        }
        self.io.oam_address = (self.io.oam_address + 1) & 0x03ff;
    }

    /// Read one PPU register ($2100–$213F). Reads of write-only registers return the PPU MDR
    /// open-bus latch.
    pub fn read_reg(&mut self, addr: u16) -> u8 {
        match addr {
            // MPYL/M/H — Mode 7 hardware multiply: M7A (signed 16) * (M7B high byte, signed 8).
            0x2134..=0x2136 => {
                let product = i32::from(self.io.m7a as i16) * i32::from((self.io.m7b >> 8) as i8);
                let byte = (addr - 0x2134) as u32 * 8;
                let out = ((product as u32) >> byte) as u8;
                self.io.ppu1_mdr = out;
                out
            }
            // SLHV — latch H/V counters (gated by the CPU's I/O-enable in HW; we latch always).
            0x2137 => {
                self.io.latch_h = self.h;
                self.io.latch_v = self.v;
                self.io.counter_latched = true;
                self.io.ppu1_mdr
            }
            // OAMDATAREAD
            0x2138 => {
                let addr = self.io.oam_address & 0x03ff;
                let out = if addr >= 0x200 {
                    self.oam[(0x200 + (addr & 0x1f)) as usize]
                } else {
                    self.oam[addr as usize]
                };
                self.io.oam_address = (self.io.oam_address + 1) & 0x03ff;
                self.io.ppu1_mdr = out;
                out
            }
            // VMDATALREAD — prefetch latch low byte
            0x2139 => {
                let out = (self.io.vram_read_latch & 0xff) as u8;
                if !self.io.vram_increment_high {
                    self.io.vram_read_latch = self.vram_read_word();
                    self.vram_step();
                }
                self.io.ppu1_mdr = out;
                out
            }
            // VMDATAHREAD — prefetch latch high byte
            0x213a => {
                let out = (self.io.vram_read_latch >> 8) as u8;
                if self.io.vram_increment_high {
                    self.io.vram_read_latch = self.vram_read_word();
                    self.vram_step();
                }
                self.io.ppu1_mdr = out;
                out
            }
            // CGDATAREAD — read twice
            0x213b => {
                let out = if self.io.cgram_latch_high {
                    self.io.cgram_latch_high = false;
                    let hi = ((self.cgram[self.io.cgram_address as usize] >> 8) & 0x7f) as u8;
                    self.io.cgram_address = self.io.cgram_address.wrapping_add(1);
                    (self.io.ppu2_mdr & 0x80) | hi
                } else {
                    self.io.cgram_latch_high = true;
                    (self.cgram[self.io.cgram_address as usize] & 0xff) as u8
                };
                self.io.ppu2_mdr = out;
                out
            }
            // OPHCT — read twice (9-bit)
            0x213c => {
                let out = if self.io.ophct_high_toggle {
                    self.io.ophct_high_toggle = false;
                    (self.io.ppu2_mdr & 0xfe) | ((self.io.latch_h >> 8) & 1) as u8
                } else {
                    self.io.ophct_high_toggle = true;
                    (self.io.latch_h & 0xff) as u8
                };
                self.io.ppu2_mdr = out;
                out
            }
            // OPVCT — read twice (9-bit)
            0x213d => {
                let out = if self.io.opvct_high_toggle {
                    self.io.opvct_high_toggle = false;
                    (self.io.ppu2_mdr & 0xfe) | ((self.io.latch_v >> 8) & 1) as u8
                } else {
                    self.io.opvct_high_toggle = true;
                    (self.io.latch_v & 0xff) as u8
                };
                self.io.ppu2_mdr = out;
                out
            }
            // STAT77 — sprite over/time flags + PPU1 version (1)
            0x213e => {
                let mut out = 0x01u8; // version nibble
                if self.io.range_over {
                    out |= 0x40;
                }
                if self.io.time_over {
                    out |= 0x80;
                }
                out |= self.io.ppu1_mdr & 0x10;
                self.io.ppu1_mdr = out;
                out
            }
            // STAT78 — NTSC/PAL + latch flag + field + PPU2 version (3); read clears latch
            0x213f => {
                let mut out = 0x03u8; // version nibble
                if self.region == Region::Pal {
                    out |= 0x10;
                }
                if self.io.counter_latched {
                    out |= 0x40;
                }
                if self.field {
                    out |= 0x80;
                }
                out |= self.io.ppu2_mdr & 0x20;
                // Reading clears the latch + resets the H/V read toggles.
                self.io.counter_latched = false;
                self.io.ophct_high_toggle = false;
                self.io.opvct_high_toggle = false;
                self.io.ppu2_mdr = out;
                out
            }
            // Write-only registers return the PPU1 open-bus MDR.
            _ => self.io.ppu1_mdr,
        }
    }
}
