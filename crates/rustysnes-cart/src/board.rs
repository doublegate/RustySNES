//! The [`Board`] trait — the SNES analogue of RustyNES's `Mapper`.
//!
//! A "board" is one cartridge PCB family: a base address-mapping mode (LoROM / HiROM /
//! ExHiROM) plus any on-cart coprocessor. The 65C816 bus calls [`Board::read24`] /
//! [`Board::write24`] with a full 24-bit `(bank << 16) | addr`; the board decodes its own
//! mapping. Coprocessor boards additionally implement the default-no-op hooks
//! ([`Board::coprocessor_tick`], the `notify_*` family) — exactly the RustyNES pattern where
//! per-board IRQ/cycle quirks live INSIDE the board, called via default-no-op trait hooks.
//!
//! See `docs/cart.md` for the per-board / per-coprocessor table and the decode formulas.

use alloc::boxed::Box;
use alloc::vec;

use crate::header::{Coprocessor as CoproId, Header, MapMode};

/// The result of a board's address decode: where a 24-bit CPU address lands.
///
/// The bus uses this to route a read/write to the right backing store, and the default
/// [`Board::read24`] / [`Board::write24`] consume it directly over the board's own storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedAddr {
    /// Maps into ROM at the given linear byte offset (already mirror-folded to `rom_size`).
    Rom(u32),
    /// Maps into cartridge SRAM at the given offset (already wrapped to `sram_size`).
    Sram(u32),
    /// Maps into a coprocessor register window (the board handles it internally).
    Coprocessor,
    /// Open bus / unmapped (returns the last bus value).
    Open,
}

/// Identifies which coprocessor (if any) a board carries.
///
/// Mirrors the header's [`CoproId`] but is re-exported here so downstream callers can match on
/// a board's coprocessor without importing the header module.
pub type Coprocessor = CoproId;

/// A cartridge board: its address mapping + any coprocessor behavior.
///
/// The default-no-op hooks are the load-bearing port of RustyNES's `Mapper::notify_*`:
/// the CPU/PPU/scheduler call them unconditionally, and only coprocessor boards override
/// them. Keep every board-specific quirk INSIDE its `impl Board` — never special-case a
/// board from the bus or the PPU.
pub trait Board {
    /// Human-readable board name (for the debugger + logs), e.g. `"LoROM"`, `"HiROM+DSP-1"`.
    fn name(&self) -> &'static str;

    /// Which coprocessor this board carries (or [`Coprocessor::None`]).
    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::None
    }

    /// Decode a 24-bit CPU address `(bank << 16) | addr` to its backing store. The returned
    /// [`MappedAddr::Rom`] / [`MappedAddr::Sram`] offsets are already folded into range.
    fn map(&self, addr24: u32) -> MappedAddr;

    /// Read a byte at a 24-bit CPU address. Default routes through [`Self::map`] over the
    /// board's own storage; coprocessor boards override to intercept their register windows.
    fn read24(&mut self, addr24: u32) -> u8 {
        match self.map(addr24) {
            MappedAddr::Rom(off) => self.rom().get(off as usize).copied().unwrap_or(0),
            MappedAddr::Sram(off) => self.sram().get(off as usize).copied().unwrap_or(0),
            MappedAddr::Coprocessor | MappedAddr::Open => 0,
        }
    }

    /// Write a byte at a 24-bit CPU address (SRAM, coprocessor registers, bank latches).
    /// Default writes through [`Self::map`] to SRAM only — ROM and open bus are read-only.
    fn write24(&mut self, addr24: u32, val: u8) {
        if let MappedAddr::Sram(off) = self.map(addr24)
            && let Some(slot) = self.sram_mut().get_mut(off as usize)
        {
            *slot = val;
        }
    }

    /// The board's ROM backing store (for save-states / debugging). Read-only.
    fn rom(&self) -> &[u8];

    /// The board's SRAM backing store (for battery saves / save-states). Read-only.
    fn sram(&self) -> &[u8];

    /// The board's SRAM backing store, mutable (for battery restore / save-state load).
    fn sram_mut(&mut self) -> &mut [u8];

    // --- Default-no-op coprocessor / IRQ hooks (the `notify_a12`-equivalents). ---

    /// Advance the on-cart coprocessor by one of its clock units. Default no-op (base
    /// LoROM/HiROM/ExHiROM have no coprocessor). Super FX / SA-1 / DSP-n override this; the
    /// scheduler drives it from the master-clock loop on the coprocessor's divisor.
    fn coprocessor_tick(&mut self) {}

    /// Notify the board that the PPU is starting a new scanline. Default no-op. (Reserved for
    /// boards whose coprocessor or IRQ counter is scanline-aligned.)
    fn notify_scanline(&mut self) {}

    /// Notify the board of one elapsed CPU cycle. Default no-op. Coprocessors with a
    /// CPU-cycle-driven IRQ/refresh counter override this.
    fn notify_cpu_cycle(&mut self) {}

    /// Whether the board is currently asserting its IRQ line (SA-1, Super FX, SPC7110 RTC).
    /// Default `false`. The bus ORs this with the other IRQ sources.
    fn irq_pending(&self) -> bool {
        false
    }

    /// Supply a coprocessor firmware dump (e.g. the DSP-1 `dsp1.rom`). Default `false` — a base
    /// board has no firmware to load. A chip-ROM-dump coprocessor returns `true` once the dump is
    /// accepted; without it the board is non-functional, never silently degraded (`docs/adr/0003`).
    fn load_firmware(&mut self, _bytes: &[u8]) -> bool {
        false
    }

    /// Count of host accesses to the coprocessor's data ports since power-on (debugger /
    /// diagnostics). Default `0` — base boards have no coprocessor.
    fn coprocessor_host_accesses(&self) -> u64 {
        0
    }
}

/// Fold a linear ROM offset into a `size`-byte image with hardware-accurate mirroring.
///
/// Power-of-two sizes are a plain `& (size - 1)`. Non-power-of-two images (e.g. 6 MiB =
/// 4 MiB + 2 MiB) mirror the way real address decoding does: the largest power-of-two block
/// addresses linearly and the remainder mirrors within itself. This matches ares' `Bus::mirror`
/// (clean-room re-implementation — the algorithm is hardware fact).
#[must_use]
const fn mirror(mut address: u32, size: u32) -> u32 {
    if size == 0 {
        return 0;
    }
    let mut base = 0u32;
    let mut mask = 1u32 << 23;
    let mut size = size;
    while address >= size {
        while address & mask == 0 {
            mask >>= 1;
        }
        address -= mask;
        if size > mask {
            size -= mask;
            base += mask;
        }
        mask >>= 1;
    }
    base + address
}

/// Select the concrete board for a parsed header, allocating ROM + zeroed SRAM.
///
/// `rom` must be the copier-prefix-stripped image (see [`Header::detect`]). The base map mode
/// picks the base board; a detected coprocessor wraps it (DSP-1 → [`crate::coproc::Dsp1Board`]).
/// Coprocessor boards that depend on a chip-ROM dump are inert until the firmware is supplied via
/// [`Board::load_firmware`] (`docs/adr/0003`).
#[must_use]
pub fn select(header: &Header, rom: &[u8]) -> Box<dyn Board> {
    let rom_len = rom.len();
    let rom: Box<[u8]> = Box::from(rom);
    let sram = vec![0u8; header.sram_size].into_boxed_slice();
    let base: Box<dyn Board> = match header.map_mode {
        MapMode::LoRom => Box::new(LoRom::new(rom, sram)),
        MapMode::HiRom => Box::new(HiRom::new(rom, sram)),
        MapMode::ExHiRom => Box::new(ExHiRom::new(rom, sram)),
    };
    match header.coprocessor {
        CoproId::Dsp => Box::new(crate::coproc::Dsp1Board::new(
            base,
            header.map_mode,
            rom_len,
        )),
        // Other coprocessor families land in later sprints; until then the cart runs as its base
        // board (the coprocessor window is simply unmapped, never silently faked).
        _ => base,
    }
}

/// LoROM (mode `$20`): 32 KiB ROM windows in `$8000–$FFFF` of each bank.
#[derive(Debug, Clone)]
pub struct LoRom {
    rom: Box<[u8]>,
    sram: Box<[u8]>,
}

impl LoRom {
    /// Construct a LoROM board from owned ROM + (zeroed, header-sized) SRAM.
    #[must_use]
    pub const fn new(rom: Box<[u8]>, sram: Box<[u8]>) -> Self {
        Self { rom, sram }
    }
}

impl Board for LoRom {
    fn name(&self) -> &'static str {
        "LoROM"
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;

        // SRAM: banks $70–$7D and $F0–$FF, $0000–$7FFF (when present).
        if !self.sram.is_empty() {
            let lo = bank & 0x7F;
            if (0x70..=0x7D).contains(&lo) && addr < 0x8000 {
                let idx = (lo - 0x70) * 0x8000 + addr;
                #[allow(clippy::cast_possible_truncation)]
                let len = self.sram.len() as u32;
                return MappedAddr::Sram(idx % len);
            }
        }

        // ROM: $8000–$FFFF of every bank; offset = ((bank & 0x7F) << 15) | (addr & 0x7FFF).
        if addr >= 0x8000 {
            let off = ((bank & 0x7F) << 15) | (addr & 0x7FFF);
            #[allow(clippy::cast_possible_truncation)]
            let size = self.rom.len() as u32;
            return MappedAddr::Rom(mirror(off, size));
        }

        MappedAddr::Open
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
}

/// HiROM (mode `$21`): 64 KiB linear ROM banks; full ROM at `$C0–$FF`.
#[derive(Debug, Clone)]
pub struct HiRom {
    rom: Box<[u8]>,
    sram: Box<[u8]>,
}

impl HiRom {
    /// Construct a HiROM board from owned ROM + (zeroed, header-sized) SRAM.
    #[must_use]
    pub const fn new(rom: Box<[u8]>, sram: Box<[u8]>) -> Self {
        Self { rom, sram }
    }
}

impl Board for HiRom {
    fn name(&self) -> &'static str {
        "HiROM"
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;

        // SRAM: banks $20–$3F and $A0–$BF, $6000–$7FFF (when present).
        if !self.sram.is_empty() {
            let lo = bank & 0x7F;
            if (0x20..=0x3F).contains(&lo) && (0x6000..0x8000).contains(&addr) {
                let idx = (lo - 0x20) * 0x2000 + (addr - 0x6000);
                #[allow(clippy::cast_possible_truncation)]
                let len = self.sram.len() as u32;
                return MappedAddr::Sram(idx % len);
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        let size = self.rom.len() as u32;

        // Linear ROM: banks $40–$7D and $C0–$FF, full 64 KiB → offset = (bank & 0x3F) << 16 | addr.
        let lo = bank & 0x7F;
        if (0x40..=0x7D).contains(&lo) || bank >= 0xC0 {
            let off = ((bank & 0x3F) << 16) | addr;
            return MappedAddr::Rom(mirror(off, size));
        }

        // Windowed ROM: banks $00–$3F and $80–$BF, $8000–$FFFF → same linear offset.
        if (lo < 0x40) && addr >= 0x8000 {
            let off = ((bank & 0x3F) << 16) | addr;
            return MappedAddr::Rom(mirror(off, size));
        }

        MappedAddr::Open
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
}

/// ExHiROM (mode `$25`): the extended HiROM layout for >4 MiB titles.
///
/// Banks `$80–$FF` address the first 4 MiB; banks `$00–$7D` address the extra (high) 4 MiB.
/// The ROM offset's bit 22 is the inverse of address bit 23, so the high banks select the
/// upper image half. See `docs/cart.md` §ExHiROM.
#[derive(Debug, Clone)]
pub struct ExHiRom {
    rom: Box<[u8]>,
    sram: Box<[u8]>,
}

impl ExHiRom {
    /// Construct an ExHiROM board from owned ROM + (zeroed, header-sized) SRAM.
    #[must_use]
    pub const fn new(rom: Box<[u8]>, sram: Box<[u8]>) -> Self {
        Self { rom, sram }
    }
}

impl Board for ExHiRom {
    fn name(&self) -> &'static str {
        "ExHiROM"
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;

        // SRAM: banks $20–$3F / $A0–$BF, $6000–$7FFF (HiROM-style), present only on $00–$3F
        // half where ROM isn't already mapped at $6000.
        if !self.sram.is_empty() && (0x20..=0x3F).contains(&(bank & 0x7F)) {
            // On ExHiROM the SRAM appears only when the bank's $8000 window isn't ROM; here we
            // expose it at $6000–$7FFF of banks $20–$3F / $A0–$BF, matching HiROM SRAM windows.
            if (0x6000..0x8000).contains(&addr) && bank < 0x80 {
                let idx = ((bank & 0x3F) - 0x20) * 0x2000 + (addr - 0x6000);
                #[allow(clippy::cast_possible_truncation)]
                let len = self.sram.len() as u32;
                return MappedAddr::Sram(idx % len);
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        let size = self.rom.len() as u32;
        let lo = bank & 0x7F;

        // bit 22 of the ROM offset = inverse of A23 (i.e. of bank bit 7). Banks $80–$FF (A23=1)
        // → high bit 0 → first 4 MiB; banks $00–$7D (A23=0) → high bit 1 → extra 4 MiB.
        let high = if bank & 0x80 != 0 { 0 } else { 1u32 << 22 };

        // Linear ROM region: banks $40–$7D / $C0–$FF, full 64 KiB.
        if (0x40..=0x7D).contains(&lo) || bank >= 0xC0 {
            let off = high | ((bank & 0x3F) << 16) | addr;
            return MappedAddr::Rom(mirror(off, size));
        }

        // Windowed ROM: banks $00–$3F / $80–$BF, $8000–$FFFF.
        if lo < 0x40 && addr >= 0x8000 {
            let off = high | ((bank & 0x3F) << 16) | addr;
            return MappedAddr::Rom(mirror(off, size));
        }

        MappedAddr::Open
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(v: alloc::vec::Vec<u8>) -> Box<[u8]> {
        v.into_boxed_slice()
    }

    #[test]
    fn base_boards_default_no_coprocessor() {
        let lo = LoRom::new(boxed(vec![0; 0x8000]), boxed(vec![]));
        assert_eq!(lo.coprocessor(), Coprocessor::None);
        let hi = HiRom::new(boxed(vec![0; 0x8000]), boxed(vec![]));
        assert!(!hi.irq_pending());
        let mut ex = ExHiRom::new(boxed(vec![0; 0x8000]), boxed(vec![]));
        ex.coprocessor_tick();
        ex.notify_scanline();
        ex.notify_cpu_cycle();
    }

    #[test]
    fn lorom_decode_and_windowing() {
        // 64 KiB ROM = two 32 KiB banks. Mark distinctive bytes.
        let mut rom = vec![0u8; 0x1_0000];
        rom[0x0000] = 0xAA; // bank $00:$8000
        rom[0x7FFF] = 0xBB; // bank $00:$FFFF
        rom[0x8000] = 0xCC; // bank $01:$8000
        let mut b = LoRom::new(boxed(rom), boxed(vec![]));

        assert_eq!(b.read24(0x00_8000), 0xAA);
        assert_eq!(b.read24(0x00_FFFF), 0xBB);
        assert_eq!(b.read24(0x01_8000), 0xCC);
        // $80 mirror of $00.
        assert_eq!(b.read24(0x80_8000), 0xAA);
        // $0000–$7FFF of a ROM bank is open bus (no SRAM here).
        assert_eq!(b.map(0x00_0000), MappedAddr::Open);
    }

    #[test]
    fn lorom_sram_roundtrip() {
        let rom = vec![0u8; 0x1_0000];
        let sram = vec![0u8; 0x2000]; // 8 KiB
        let mut b = LoRom::new(boxed(rom), boxed(sram));
        // bank $70:$0000.
        b.write24(0x70_0000, 0x42);
        assert_eq!(b.read24(0x70_0000), 0x42);
        // mirror at $F0.
        assert_eq!(b.read24(0xF0_0000), 0x42);
        assert_eq!(b.sram()[0], 0x42);
    }

    #[test]
    fn hirom_decode_linear_and_window() {
        // 64 KiB ROM. $C0:$0000 should be ROM offset 0; $00:$8000 should be ROM offset $8000.
        let mut rom = vec![0u8; 0x1_0000];
        rom[0x0000] = 0x11; // C0:0000
        rom[0x8000] = 0x22; // C0:8000 and 00:8000
        let mut b = HiRom::new(boxed(rom), boxed(vec![]));

        assert_eq!(b.read24(0xC0_0000), 0x11);
        assert_eq!(b.read24(0xC0_8000), 0x22);
        // $00:$8000 windows the same linear offset $8000.
        assert_eq!(b.read24(0x00_8000), 0x22);
        // $00:$0000 is not ROM (no SRAM) → open.
        assert_eq!(b.map(0x00_0000), MappedAddr::Open);
    }

    #[test]
    fn hirom_sram_roundtrip() {
        let rom = vec![0u8; 0x1_0000];
        let sram = vec![0u8; 0x2000];
        let mut b = HiRom::new(boxed(rom), boxed(sram));
        b.write24(0x20_6000, 0x99);
        assert_eq!(b.read24(0x20_6000), 0x99);
        assert_eq!(b.read24(0xA0_6000), 0x99); // mirror
    }

    #[test]
    fn non_power_of_two_mirroring() {
        // 6 MiB image (4 MiB + 2 MiB). Address 5 MiB (0x500000) folds to 4 MiB + 1 MiB = 0x500000
        // (in range). Address 6 MiB (0x600000) wraps into the 2 MiB tail: -> 0x400000.
        let size = 0x60_0000;
        assert_eq!(mirror(0x10_0000, size), 0x10_0000); // in range, identity
        assert_eq!(mirror(0x60_0000, size), 0x40_0000); // first past end mirrors into tail
        // pure power-of-two behaves like a mask.
        assert_eq!(mirror(0x1_0001, 0x1_0000), 0x0001);
    }

    #[test]
    fn exhirom_high_and_low_banks() {
        // 8 MiB ROM. $C0:$0000 → first 4 MiB offset 0. $40:$0000 → extra 4 MiB offset 0x400000.
        let mut rom = vec![0u8; 0x80_0000];
        rom[0x0000] = 0x55; // first half base
        rom[0x40_0000] = 0x66; // extra half base
        let mut b = ExHiRom::new(boxed(rom), boxed(vec![]));
        assert_eq!(b.read24(0xC0_0000), 0x55);
        assert_eq!(b.read24(0x40_0000), 0x66);
    }
}
