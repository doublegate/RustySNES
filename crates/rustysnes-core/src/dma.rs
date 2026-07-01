//! GP-DMA + HDMA — the 8-channel DMA controller (`$420B`/`$420C`, `$43n0-$43nA`).
//!
//! Clean-room port of the ares (ISC, vendor-ok) `sfc/cpu/dma.cpp` transfer model; never a
//! verbatim copy. The DMA controller moves bytes between the A-bus (the 24-bit CPU address
//! space) and the B-bus (the PPU/APU register window `$2100-$21FF`). Two flavors:
//!
//! - **GP-DMA** (`MDMAEN $420B`): writing a non-zero mask runs every selected channel to
//!   completion **with the CPU fully halted**, `8` master clocks per byte (+ per-channel and
//!   alignment overhead). Cannot cross a bank (`sourceAddress` wraps in-bank).
//! - **HDMA** (`HDMAEN $420C`): per visible scanline, fires at H≈`$116`; each active channel
//!   transfers its line entry (direct or indirect), `8` clocks/byte plus overhead, and HDMA
//!   **preempts** an in-flight GP-DMA.
//!
//! The controller never touches the concrete `Bus` directly: it drives the [`DmaBus`] trait so
//! it stays decoupled (and unit-testable in isolation). The master-clock cost is reported back
//! to the caller (the scheduler advances the clock by it).

// Byte-splitting 16-bit DMA registers into `u8` halves is the core of the controller; the
// deliberate narrowing casts are allowed module-wide (mirrors the CPU/bus modules).
#![allow(clippy::cast_possible_truncation, clippy::cast_lossless)]

use crate::dma_bus::DmaBus;

/// Per-mode B-bus register count (how many distinct B-bus addresses a transfer unit touches).
/// ares `lengths[8] = {1, 2, 2, 4, 4, 4, 2, 4}`.
const MODE_LENGTHS: [u8; 8] = [1, 2, 2, 4, 4, 4, 2, 4];

/// One of the 8 DMA channels (`$43n0-$43nA`).
#[derive(Debug, Clone, Copy)]
pub struct Channel {
    /// `$43n0` DMAP — transfer params: bit7 direction (0 = A→B, 1 = B→A), bit6 indirect (HDMA),
    /// bit4 reverse (GP-DMA addr decrement), bit3 fixed (GP-DMA addr no-change), bits2-0 mode.
    pub dmap: u8,
    /// `$43n1` BBAD — B-bus target low byte (the `$21xx` register).
    pub target: u8,
    /// `$43n2-3` A1T — A-bus source address (GP) / table address (HDMA).
    pub source_addr: u16,
    /// `$43n4` A1B — A-bus source bank.
    pub source_bank: u8,
    /// `$43n5-6` DAS — GP byte count / HDMA indirect address.
    pub count_or_indirect: u16,
    /// `$43n7` DASB — HDMA indirect bank.
    pub indirect_bank: u8,
    /// `$43n8-9` A2A — HDMA table running address.
    pub hdma_addr: u16,
    /// `$43nA` NTRL — HDMA line counter (bit7 = repeat, bits0-6 = lines).
    pub line_counter: u8,
    /// HDMA: this channel has finished its table for the frame.
    pub hdma_completed: bool,
    /// HDMA: perform a transfer on this line (vs. just decrement the counter).
    pub hdma_do_transfer: bool,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            dmap: 0xFF,
            target: 0xFF,
            source_addr: 0xFFFF,
            source_bank: 0xFF,
            count_or_indirect: 0xFFFF,
            indirect_bank: 0xFF,
            hdma_addr: 0xFFFF,
            line_counter: 0xFF,
            hdma_completed: false,
            hdma_do_transfer: false,
        }
    }
}

impl Channel {
    const fn direction_b_to_a(self) -> bool {
        self.dmap & 0x80 != 0
    }
    const fn indirect(self) -> bool {
        self.dmap & 0x40 != 0
    }
    const fn reverse(self) -> bool {
        self.dmap & 0x10 != 0
    }
    const fn fixed(self) -> bool {
        self.dmap & 0x08 != 0
    }
    const fn mode(self) -> u8 {
        self.dmap & 0x07
    }

    /// The B-bus address for transfer-unit byte `index` (ares `Channel::transfer` switch).
    const fn b_address(self, index: u8) -> u8 {
        let bump = match self.mode() {
            1 | 5 => index & 1,
            3 | 7 => (index >> 1) & 1,
            4 => index,
            _ => 0, // modes 0, 2, 6
        };
        self.target.wrapping_add(bump)
    }
}

/// The 8-channel DMA controller plus the `MDMAEN`/`HDMAEN` enables.
#[derive(Debug, Clone, Default)]
pub struct Dma {
    /// The 8 channels.
    pub channels: [Channel; 8],
    /// `$420B` MDMAEN — GP-DMA enable mask (write triggers the run).
    pub gp_enable: u8,
    /// `$420C` HDMAEN — HDMA enable mask.
    pub hdma_enable: u8,
}

impl Dma {
    /// Construct a power-on DMA controller (all channels open, no transfers pending).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Write a DMA channel register `$43nA` (or the `$420B/$420C` enables, handled by the bus).
    /// `reg` is the low byte (`$00-$0A`); `ch` is the channel index `0-7`.
    pub fn write_reg(&mut self, ch: usize, reg: u8, val: u8) {
        let c = &mut self.channels[ch & 7];
        match reg {
            0x0 => c.dmap = val,
            0x1 => c.target = val,
            0x2 => c.source_addr = (c.source_addr & 0xFF00) | u16::from(val),
            0x3 => c.source_addr = (c.source_addr & 0x00FF) | (u16::from(val) << 8),
            0x4 => c.source_bank = val,
            0x5 => c.count_or_indirect = (c.count_or_indirect & 0xFF00) | u16::from(val),
            0x6 => c.count_or_indirect = (c.count_or_indirect & 0x00FF) | (u16::from(val) << 8),
            0x7 => c.indirect_bank = val,
            0x8 => c.hdma_addr = (c.hdma_addr & 0xFF00) | u16::from(val),
            0x9 => c.hdma_addr = (c.hdma_addr & 0x00FF) | (u16::from(val) << 8),
            0xA => c.line_counter = val,
            _ => {}
        }
    }

    /// Read a DMA channel register `$43nA`.
    #[must_use]
    pub const fn read_reg(&self, ch: usize, reg: u8) -> u8 {
        let c = &self.channels[ch & 7];
        match reg {
            0x0 => c.dmap,
            0x1 => c.target,
            0x2 => c.source_addr as u8,
            0x3 => (c.source_addr >> 8) as u8,
            0x4 => c.source_bank,
            0x5 => c.count_or_indirect as u8,
            0x6 => (c.count_or_indirect >> 8) as u8,
            0x7 => c.indirect_bank,
            0x8 => c.hdma_addr as u8,
            0x9 => (c.hdma_addr >> 8) as u8,
            0xA => c.line_counter,
            _ => 0,
        }
    }

    /// Run all GP-DMA channels selected by `mask` (`$420B` write) to completion. The CPU is
    /// considered halted for the whole run; the returned value is the **master-clock cost**
    /// (the scheduler advances the clock by it). Ported from ares `Channel::dmaRun`.
    #[must_use]
    pub fn run_gp(&mut self, mask: u8, bus: &mut impl DmaBus) -> u32 {
        let mut cost: u32 = 0;
        if mask == 0 {
            return 0;
        }
        // Whole-transfer alignment overhead (ares charges 8 once before the run). Each `bus.step`
        // advances the master clock *now* so the PPU scanline is current at every B-bus write —
        // see `DmaBus::step`. The returned `cost` is the same total, retained for callers/tests;
        // the concrete Bus advances via `step` and must NOT re-charge `cost` afterwards.
        //
        // While this transfer runs, the bus has lent us its controller, so its own per-tick HDMA
        // path is dormant; we interleave HDMA at every scanline boundary the transfer crosses
        // (hardware: HDMA preempts general DMA at each scanline start). `last_line` seeds from the
        // bus's own HDMA bookkeeping so no line runs twice or is skipped.
        let mut last_line = bus.hdma_last_line();
        bus.step(8);
        cost += 8;
        cost += self.service_hdma_during_gp(&mut last_line, bus);
        for ch in 0..8 {
            if mask & (1 << ch) == 0 {
                continue;
            }
            bus.step(8); // per-channel setup
            cost += 8;
            cost += self.service_hdma_during_gp(&mut last_line, bus);
            let channel = self.channels[ch];
            let mut src = channel.source_addr;
            // `count == 0` means 0x10000 bytes (ares decrements then tests).
            let mut remaining = channel.count_or_indirect;
            let mut index: u8 = 0;
            loop {
                let a = (u32::from(channel.source_bank) << 16) | u32::from(src);
                let b = channel.b_address(index);
                // ares `Channel::transfer`: the access side steps 4 clocks, reads, steps 4 more,
                // then the write side lands (no extra step) — 8 clocks/byte with the destination
                // write occurring after the scanline has advanced.
                bus.step(4);
                if channel.direction_b_to_a() {
                    let data = bus.read_b(b);
                    bus.step(4);
                    bus.write_a(a, data);
                } else {
                    let data = bus.read_a(a);
                    bus.step(4);
                    bus.write_b(b, data);
                }
                cost += 8; // 8 master clocks per byte
                // HDMA preempts at scanline starts — interleave it if this byte crossed a line.
                cost += self.service_hdma_during_gp(&mut last_line, bus);
                if !channel.fixed() {
                    src = if channel.reverse() {
                        src.wrapping_sub(1)
                    } else {
                        src.wrapping_add(1)
                    };
                }
                index = index.wrapping_add(1);
                remaining = remaining.wrapping_sub(1);
                if remaining == 0 {
                    break;
                }
            }
            // Reflect the consumed source address back (hardware leaves it advanced).
            self.channels[ch].source_addr = src;
            self.channels[ch].count_or_indirect = 0;
        }
        // Clear the enable mask — GP-DMA is one-shot.
        self.gp_enable = 0;
        cost
    }

    /// Move one byte for one transfer unit between A-bus and B-bus (ares `Channel::transfer`,
    /// minus the WRAM↔WRAM invalid case which the bus enforces via open-bus on `$2180`).
    fn transfer_unit(channel: Channel, a_addr: u32, b_addr: u8, bus: &mut impl DmaBus) {
        if channel.direction_b_to_a() {
            let data = bus.read_b(b_addr);
            bus.write_a(a_addr, data);
        } else {
            let data = bus.read_a(a_addr);
            bus.write_b(b_addr, data);
        }
    }

    // ---- HDMA -------------------------------------------------------------------------------

    /// Service one scanline's HDMA lifecycle: at V=0 reset the bookkeeping and load the tables,
    /// on each visible line (`1..=vh`) run one transfer, otherwise nothing. Returns the
    /// master-clock cost. Shared by the per-master-tick path (`Bus::advance_master`) and the
    /// in-GP-DMA interleave (`Dma::run_gp` via `Self::service_hdma_during_gp`) so HDMA stays
    /// scanline-accurate even while a GP-DMA is advancing the clock across line boundaries.
    #[must_use]
    pub fn service_hdma_line(&mut self, line: u16, vh: u16, bus: &mut impl DmaBus) -> u32 {
        // ares `timing.cpp`: `hdmaReset()` runs at frame start regardless of HDMAEN; only the
        // subsequent `hdmaSetup()` is gated on any channel being enabled. Resetting
        // unconditionally clears `hdma_completed` so a channel finished last frame can go active
        // again if HDMAEN enables it mid-frame (`hdma_setup` itself no-ops when HDMAEN==0).
        if line == 0 {
            self.hdma_reset();
            return self.hdma_setup(bus);
        }
        if self.hdma_enable == 0 {
            return 0;
        }
        if line <= vh { self.hdma_run(bus) } else { 0 }
    }

    /// Interleave HDMA into a running GP-DMA: while the bus took our controller (so its own
    /// per-tick HDMA path is dormant), fire HDMA at each scanline boundary the GP-DMA crosses,
    /// mirroring how HDMA preempts general DMA at the start of every scanline on hardware.
    /// `last_line` carries the last-serviced scanline across byte iterations; the bus's own
    /// bookkeeping is synced so `Bus::advance_master` resumes cleanly afterward. Returns the
    /// master-clock cost of any transfer performed (already stepped onto `bus`).
    fn service_hdma_during_gp(&mut self, last_line: &mut u16, bus: &mut impl DmaBus) -> u32 {
        if self.hdma_enable == 0 {
            return 0;
        }
        let line = bus.scanline();
        if line == *last_line {
            return 0;
        }
        *last_line = line;
        bus.set_hdma_last_line(line);
        let vh = bus.visible_height();
        let cost = self.service_hdma_line(line, vh, bus);
        if cost > 0 {
            bus.step(cost);
        }
        cost
    }

    /// Reset every channel's HDMA bookkeeping at the start of a frame (V=0). ares `hdmaReset`.
    pub fn hdma_reset(&mut self) {
        for c in &mut self.channels {
            c.hdma_completed = false;
            c.hdma_do_transfer = false;
        }
    }

    /// Per-frame HDMA setup: load each enabled channel's table pointer + first line entry.
    /// Returns the master-clock cost. ares `hdmaSetup` + `Channel::hdmaSetup/hdmaReload`.
    #[must_use]
    pub fn hdma_setup(&mut self, bus: &mut impl DmaBus) -> u32 {
        if self.hdma_enable == 0 {
            return 0;
        }
        let mut cost: u32 = 8;
        for ch in 0..8 {
            // ares `Channel::hdmaSetup`: `hdmaDoTransfer = true` for EVERY channel, then the
            // early-out for disabled ones. A channel disabled at frame start keeps its stale
            // address/line_counter; if HDMAEN enables it mid-frame it resumes transferring from
            // there (the "HDMAEN latch" quirk). Skipping the flag here would wrongly leave a
            // mid-frame-enabled channel dormant for the rest of the frame.
            self.channels[ch].hdma_do_transfer = true;
            if self.hdma_enable & (1 << ch) == 0 {
                continue;
            }
            self.channels[ch].hdma_addr = self.channels[ch].source_addr;
            self.channels[ch].line_counter = 0;
            cost += self.hdma_reload(ch, bus);
        }
        cost
    }

    /// Reload a channel's line counter / indirect pointer when the counter reaches 0 (ares
    /// `Channel::hdmaReload`). Returns the master-clock cost of the table reads.
    fn hdma_reload(&mut self, ch: usize, bus: &mut impl DmaBus) -> u32 {
        let mut cost = 0;
        let bank = self.channels[ch].source_bank;
        let mut addr = self.channels[ch].hdma_addr;

        // The line counter's low 7 bits reaching 0 means "reload" (bit7 is the repeat flag).
        if self.channels[ch].line_counter.trailing_zeros() >= 7 {
            let data = bus.read_a((u32::from(bank) << 16) | u32::from(addr));
            cost += 8;
            self.channels[ch].line_counter = data;
            addr = addr.wrapping_add(1);

            let completed = self.channels[ch].line_counter == 0;
            self.channels[ch].hdma_completed = completed;
            self.channels[ch].hdma_do_transfer = !completed;

            if self.channels[ch].indirect() {
                let lo = bus.read_a((u32::from(bank) << 16) | u32::from(addr));
                cost += 8;
                addr = addr.wrapping_add(1);
                // A finished table whose final entry is the indirect low byte stops here (ares
                // skips the high-byte fetch); otherwise read the high byte and combine.
                let indirect = if completed && self.hdma_finished(ch) {
                    u16::from(lo)
                } else {
                    let hi = bus.read_a((u32::from(bank) << 16) | u32::from(addr));
                    cost += 8;
                    addr = addr.wrapping_add(1);
                    (u16::from(hi) << 8) | u16::from(lo)
                };
                self.channels[ch].count_or_indirect = indirect;
            }
        }
        self.channels[ch].hdma_addr = addr;
        cost
    }

    /// Whether every channel after `ch` has finished (ares `Channel::hdmaFinished`).
    fn hdma_finished(&self, ch: usize) -> bool {
        ((ch + 1)..8).all(|i| self.hdma_enable & (1 << i) == 0 || self.channels[i].hdma_completed)
    }

    const fn hdma_active(&self, ch: usize) -> bool {
        self.hdma_enable & (1 << ch) != 0 && !self.channels[ch].hdma_completed
    }

    /// Run one visible-scanline's HDMA for all active channels (ares `hdmaRun` →
    /// `hdmaTransfer` + `hdmaAdvance`). Returns the master-clock cost (the per-line budget).
    #[must_use]
    pub fn hdma_run(&mut self, bus: &mut impl DmaBus) -> u32 {
        if self.hdma_enable == 0 {
            return 0;
        }
        let mut cost: u32 = 8; // per-line overhead
        // Transfer pass.
        for ch in 0..8 {
            if !self.hdma_active(ch) || !self.channels[ch].hdma_do_transfer {
                continue;
            }
            let channel = self.channels[ch];
            let len = MODE_LENGTHS[channel.mode() as usize];
            let indirect = channel.indirect();
            for index in 0..len {
                // Indirect channels stream from `indirectBank:count_or_indirect`; direct channels
                // stream from `sourceBank:hdma_addr`. Each byte advances the running pointer.
                let ptr = if indirect {
                    self.channels[ch].count_or_indirect
                } else {
                    self.channels[ch].hdma_addr
                };
                let a_addr = (u32::from(if indirect {
                    channel.indirect_bank
                } else {
                    channel.source_bank
                }) << 16)
                    | u32::from(ptr);
                let b = channel.b_address(index);
                Self::transfer_unit(channel, a_addr, b, bus);
                cost += 8;
                let next = ptr.wrapping_add(1);
                if indirect {
                    self.channels[ch].count_or_indirect = next;
                } else {
                    self.channels[ch].hdma_addr = next;
                }
            }
        }
        // Advance pass: decrement counters + reload at zero.
        for ch in 0..8 {
            if !self.hdma_active(ch) {
                continue;
            }
            self.channels[ch].line_counter = self.channels[ch].line_counter.wrapping_sub(1);
            self.channels[ch].hdma_do_transfer = self.channels[ch].line_counter & 0x80 != 0;
            cost += self.hdma_reload(ch, bus);
        }
        cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// A tiny A-bus (64 KiB flat) + B-bus ($21xx) recorder for testing transfers.
    struct TestBus {
        a: Vec<u8>,
        b: [u8; 256],
    }
    impl DmaBus for TestBus {
        fn read_a(&mut self, addr: u32) -> u8 {
            *self.a.get((addr & 0xFFFF) as usize).unwrap_or(&0)
        }
        fn write_a(&mut self, addr: u32, val: u8) {
            let i = (addr & 0xFFFF) as usize;
            if i < self.a.len() {
                self.a[i] = val;
            }
        }
        fn read_b(&mut self, addr: u8) -> u8 {
            self.b[addr as usize]
        }
        fn write_b(&mut self, addr: u8, val: u8) {
            self.b[addr as usize] = val;
        }
    }

    #[test]
    fn gp_dma_mode0_copies_block_to_b_bus() {
        let mut bus = TestBus {
            a: vec![0; 0x10000],
            b: [0; 256],
        };
        for i in 0..4u32 {
            bus.a[(0x1000 + i) as usize] = (0xA0 + i) as u8;
        }
        let mut dma = Dma::new();
        // channel 0: mode 0 (single reg), A→B, source $00:1000, target $18 (VMDATA), 4 bytes.
        dma.write_reg(0, 0x0, 0x00); // DMAP: A→B, mode 0
        dma.write_reg(0, 0x1, 0x18); // BBAD
        dma.write_reg(0, 0x2, 0x00); // A1TL
        dma.write_reg(0, 0x3, 0x10); // A1TH -> $1000
        dma.write_reg(0, 0x4, 0x00); // A1B
        dma.write_reg(0, 0x5, 0x04); // DASL = 4
        dma.write_reg(0, 0x6, 0x00); // DASH
        let cost = dma.run_gp(0x01, &mut bus);
        // Mode 0 hammers a single B address, so the last byte wins.
        assert_eq!(bus.b[0x18], 0xA3);
        assert!(cost >= 8 + 8 + 4 * 8); // alignment + channel + 4 bytes
    }

    #[test]
    fn gp_dma_mode1_alternates_two_b_regs() {
        let mut bus = TestBus {
            a: vec![0; 0x10000],
            b: [0; 256],
        };
        for i in 0..4u32 {
            bus.a[(0x2000 + i) as usize] = (0x10 + i) as u8;
        }
        let mut dma = Dma::new();
        dma.write_reg(0, 0x0, 0x01); // mode 1 (2 regs)
        dma.write_reg(0, 0x1, 0x18); // BBAD base
        dma.write_reg(0, 0x2, 0x00);
        dma.write_reg(0, 0x3, 0x20); // $2000
        dma.write_reg(0, 0x4, 0x00);
        dma.write_reg(0, 0x5, 0x04);
        dma.write_reg(0, 0x6, 0x00);
        let _ = dma.run_gp(0x01, &mut bus);
        // even bytes -> $18, odd bytes -> $19; last even=0x12, last odd=0x13.
        assert_eq!(bus.b[0x18], 0x12);
        assert_eq!(bus.b[0x19], 0x13);
    }

    #[test]
    fn gp_dma_enable_is_one_shot() {
        let mut bus = TestBus {
            a: vec![0; 0x10000],
            b: [0; 256],
        };
        let mut dma = Dma::new();
        dma.write_reg(0, 0x5, 0x01);
        dma.gp_enable = 0x01;
        let _ = dma.run_gp(0x01, &mut bus);
        assert_eq!(dma.gp_enable, 0);
    }
}
