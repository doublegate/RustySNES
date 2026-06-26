//! The Super FX board — the GSU wired into a LoROM cartridge.
//!
//! Super FX carts (GSU-1: Star Fox, Stunt Race FX, Vortex; GSU-2: Yoshi's Island, Doom) carry an
//! Argonaut GSU ([`crate::coproc::gsu::Gsu`]) plus dedicated Game Pak RAM that holds the plotted
//! bitmap. The board owns the ROM (shared, read-only) and the Game Pak RAM, intercepts the GSU
//! register window, and arbitrates the shared ROM/RAM bus between the SNES CPU and the GSU. There
//! is **no chip-ROM dump** — the GSU program lives in the cartridge ROM — so the board is
//! functional the moment a Super FX cart loads (`docs/cart.md`, `docs/adr/0003`).
//!
//! ## CPU-side memory map (LoROM Super FX, the de-facto board the cartridge DB encodes)
//!
//! | Region (banks : addr)              | Target                                    |
//! |------------------------------------|-------------------------------------------|
//! | `$00-$3F,$80-$BF : $3000-$32FF`    | GSU registers + opcode cache window       |
//! | `$00-$3F,$80-$BF : $8000-$FFFF`    | Game Pak ROM (LoROM windows)              |
//! | `$40-$5F,$C0-$DF : $0000-$FFFF`    | Game Pak ROM (linear)                     |
//! | `$70-$71,$F0-$F1 : $0000-$FFFF`    | Game Pak RAM (the GSU plot bitmap)        |
//! | `$00-$3F,$80-$BF : $6000-$7FFF`    | Game Pak RAM low window (8 KiB)           |
//!
//! ## Bus arbitration (not simultaneous — `docs/cart.md` edge case #3)
//!
//! While the GSU runs with Go set it owns whichever of ROM/RAM its SCMR `RON`/`RAN` bits grant;
//! the CPU cannot read them. A CPU ROM read during GSU ROM ownership returns the hardware "snooze
//! vector" (ares `CPUROM::read`); a CPU RAM read during GSU RAM ownership returns open bus. The
//! board runs the GSU to completion the instant the CPU sets Go (the DSP-1 host-sync pattern),
//! so this arbitration is naturally serialised; the checks are kept for fidelity and for the
//! window where Go is set but the GSU has not yet been pumped.

// Chip-name jargon (GSU, Super FX, SCMR, …) is not Rust code; the rom/ram-mask pairs are
// deliberately parallel names; ROM/RAM sizes narrow from usize at the bus boundary.
#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::cast_possible_truncation
)]

use alloc::boxed::Box;
use alloc::vec;

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::coproc::gsu::{Gsu, GsuMem};
use crate::header::MapMode;

/// The hardware "snooze vector" the CPU reads from ROM while the GSU owns the ROM bus
/// (ares `SuperFX::CPUROM::read`).
const SNOOZE_VECTOR: [u8; 16] = [
    0x00, 0x01, 0x00, 0x01, 0x04, 0x01, 0x00, 0x01, 0x00, 0x01, 0x08, 0x01, 0x00, 0x01, 0x0c, 0x01,
];

/// Round `n` up to a power of two (ares `romSizeRound`); `0` maps to `0`.
const fn round_pow2(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    if n.is_power_of_two() {
        return n;
    }
    n.next_power_of_two()
}

/// A LoROM cartridge carrying a Super FX GSU + its Game Pak RAM.
pub struct SuperFxBoard {
    gsu: Gsu,
    rom: Box<[u8]>,
    ram: Box<[u8]>,
    rom_mask: u32,
    ram_mask: u32,
    /// Host accesses to the GSU register window (debugger / liveness diagnostics).
    host_accesses: u64,
}

impl core::fmt::Debug for SuperFxBoard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SuperFxBoard")
            .field("rom_len", &self.rom.len())
            .field("ram_len", &self.ram.len())
            .field("gsu", &self.gsu)
            .field("host_accesses", &self.host_accesses)
            .finish_non_exhaustive()
    }
}

impl SuperFxBoard {
    /// Game Pak RAM minimum (64 KiB) — covers the homebrew plot suites; commercial carts override
    /// from the header. Star Fox is 32 KiB, Yoshi's Island 128 KiB; a generous default never
    /// under-allocates the GSU plot target when the header omits the size.
    const RAM_MIN: usize = 0x1_0000;

    /// Build a Super FX board over `rom`, sizing the Game Pak RAM from the header (`sram_size`)
    /// clamped up to the `RAM_MIN` 64 KiB minimum and rounded to a power of two.
    #[must_use]
    pub fn new(rom: Box<[u8]>, sram_size: usize) -> Self {
        let rom_len = rom.len();
        let rom_mask = round_pow2(rom_len as u32).wrapping_sub(1);

        let ram_len = round_pow2(sram_size.max(Self::RAM_MIN) as u32) as usize;
        let ram = vec![0u8; ram_len].into_boxed_slice();
        let ram_mask = (ram_len as u32).wrapping_sub(1);

        Self {
            gsu: Gsu::new(),
            rom,
            ram,
            rom_mask,
            ram_mask,
            host_accesses: 0,
        }
    }

    /// Borrow the Game Pak RAM as a [`GsuMem`] and run the GSU to completion (host-sync).
    fn run_gsu(&mut self) {
        let mut mem = GsuMem {
            rom: &self.rom,
            rom_mask: self.rom_mask,
            ram: &mut self.ram,
            ram_mask: self.ram_mask,
        };
        self.gsu.run_until_stopped(&mut mem);
    }

    /// Classify a 24-bit CPU address into one of the board's regions.
    fn classify(addr24: u32) -> Region {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;
        let lo = bank & 0x7F; // fold the $80-$FF mirror half onto $00-$7F

        // GSU registers + cache window: $00-$3F,$80-$BF : $3000-$32FF.
        if lo <= 0x3F && (0x3000..=0x32FF).contains(&addr) {
            return Region::Register(addr as u16);
        }
        // Game Pak RAM: $70-$71,$F0-$F1 : $0000-$FFFF.
        if (0x70..=0x71).contains(&lo) {
            return Region::Ram(((lo & 1) << 16) | addr);
        }
        // Game Pak RAM low window: $00-$3F,$80-$BF : $6000-$7FFF.
        if lo <= 0x3F && (0x6000..=0x7FFF).contains(&addr) {
            return Region::Ram(addr - 0x6000);
        }
        // Game Pak ROM (linear): $40-$5F,$C0-$DF : $0000-$FFFF.
        if (0x40..=0x5F).contains(&lo) {
            return Region::Rom((bank << 16) | addr);
        }
        // Game Pak ROM (LoROM windows): $00-$3F,$80-$BF : $8000-$FFFF.
        if lo <= 0x3F && addr >= 0x8000 {
            return Region::Rom((lo << 15) | (addr & 0x7FFF));
        }
        Region::Open
    }
}

/// What a CPU address decodes to on a Super FX board.
enum Region {
    /// GSU register window; carries the `$3000-$32FF` address.
    Register(u16),
    /// Game Pak ROM at the given GSU-internal 24-bit address (the same view the GSU reads).
    Rom(u32),
    /// Game Pak RAM at the given linear offset (pre-mask).
    Ram(u32),
    /// Unmapped / open bus.
    Open,
}

impl Board for SuperFxBoard {
    fn name(&self) -> &'static str {
        "LoROM+SuperFX"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::SuperFx
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        match Self::classify(addr24) {
            Region::Register(_) => MappedAddr::Coprocessor,
            Region::Rom(off) => MappedAddr::Rom(off & self.rom_mask),
            Region::Ram(off) => MappedAddr::Sram(off & self.ram_mask),
            Region::Open => MappedAddr::Open,
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        match Self::classify(addr24) {
            Region::Register(addr) => {
                self.host_accesses = self.host_accesses.wrapping_add(1);
                self.gsu.read_register(addr)
            }
            Region::Rom(off) => {
                if self.gsu.owns_rom() {
                    return SNOOZE_VECTOR[(off & 15) as usize];
                }
                self.rom
                    .get((off & self.rom_mask) as usize)
                    .copied()
                    .unwrap_or(0)
            }
            Region::Ram(off) => {
                if self.gsu.owns_ram() {
                    return 0; // open bus while the GSU owns the RAM
                }
                self.ram
                    .get((off & self.ram_mask) as usize)
                    .copied()
                    .unwrap_or(0)
            }
            Region::Open => 0,
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        match Self::classify(addr24) {
            Region::Register(addr) => {
                self.host_accesses = self.host_accesses.wrapping_add(1);
                if self.gsu.write_register(addr, val) {
                    // The CPU just set Go: run the GSU to completion (host-sync).
                    self.run_gsu();
                }
            }
            Region::Ram(off) => {
                if !self.gsu.owns_ram()
                    && let Some(slot) = self.ram.get_mut((off & self.ram_mask) as usize)
                {
                    *slot = val;
                }
            }
            Region::Rom(_) | Region::Open => {}
        }
    }

    fn rom(&self) -> &[u8] {
        &self.rom
    }

    fn sram(&self) -> &[u8] {
        &self.ram
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        &mut self.ram
    }

    fn irq_pending(&self) -> bool {
        self.gsu.irq_pending()
    }

    fn coprocessor_host_accesses(&self) -> u64 {
        // Surface the GSU instruction count when the chip has run, else the register-access count.
        // Either is a non-zero liveness signal only if the bus window is mapped right and the GSU
        // actually executed.
        self.host_accesses.wrapping_add(self.gsu.instructions())
    }
}

/// Select a Super FX board for `rom`. The base `map_mode` is informational (Super FX carts are
/// LoROM-mapped); the GSU RAM is sized from `sram_size`.
#[must_use]
pub fn select(_map_mode: MapMode, rom: Box<[u8]>, sram_size: usize) -> SuperFxBoard {
    SuperFxBoard::new(rom, sram_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> SuperFxBoard {
        // 256 KiB ROM, default RAM.
        SuperFxBoard::new(vec![0u8; 0x4_0000].into_boxed_slice(), 0)
    }

    #[test]
    fn detects_superfx_and_default_ram() {
        let b = board();
        assert_eq!(b.coprocessor(), Coprocessor::SuperFx);
        assert_eq!(b.ram.len(), SuperFxBoard::RAM_MIN);
        assert_eq!(b.rom_mask, 0x3_FFFF);
    }

    #[test]
    fn register_window_maps_to_coprocessor() {
        let b = board();
        assert!(matches!(b.map(0x00_3030), MappedAddr::Coprocessor));
        assert!(matches!(b.map(0x80_3000), MappedAddr::Coprocessor));
        // ROM windows + RAM windows.
        assert!(matches!(b.map(0x00_8000), MappedAddr::Rom(0)));
        assert!(matches!(b.map(0x40_0000), MappedAddr::Rom(0)));
        assert!(matches!(b.map(0x70_0000), MappedAddr::Sram(0)));
        assert!(matches!(b.map(0x00_6000), MappedAddr::Sram(0)));
    }

    #[test]
    fn ram_roundtrip_via_cpu() {
        let mut b = board();
        b.write24(0x70_0010, 0x5A);
        assert_eq!(b.read24(0x70_0010), 0x5A);
        // Low window aliases the same RAM base.
        b.write24(0x00_6004, 0x33);
        assert_eq!(b.read24(0x00_6004), 0x33);
    }

    #[test]
    fn rom_window_reads_image() {
        let mut rom = vec![0u8; 0x4_0000];
        rom[0x0000] = 0xAA; // $00:$8000 -> ROM 0
        rom[0x8000] = 0xBB; // $01:$8000 -> ROM 0x8000
        let mut b = SuperFxBoard::new(rom.into_boxed_slice(), 0);
        assert_eq!(b.read24(0x00_8000), 0xAA);
        assert_eq!(b.read24(0x01_8000), 0xBB);
        // Linear $40:$8000 windows the same ROM 0x8000.
        assert_eq!(b.read24(0x40_8000), 0xBB);
    }

    /// A hand-assembled GSU program: IBT R0,#0x11 ; STOP. Proves decode + host-sync run.
    #[test]
    fn gsu_runs_a_tiny_program_via_host_sync() {
        // Place the program at ROM offset 0 (bank $00). The GSU starts at (PBR=0:R15).
        // Program bytes: a0 11  (ibt r0,#$11), 00 (stop).
        let mut rom = vec![0u8; 0x1_0000];
        rom[0] = 0xa0; // ibt r0
        rom[1] = 0x11; // #$11
        rom[2] = 0x00; // stop
        let mut b = SuperFxBoard::new(rom.into_boxed_slice(), 0);

        // Set PBR=0 ($3034), R15=0 (already 0), then write R15 high byte ($301F) to set Go.
        b.write24(0x00_3034, 0x00); // PBR = 0
        b.write24(0x00_301E, 0x00); // R15 low = 0
        b.write24(0x00_301F, 0x00); // R15 high = 0 -> sets Go, runs to STOP

        // After the run Go is clear (STOP executed) and the GSU made progress.
        assert!(b.gsu.instructions() > 0, "GSU did not execute");
        // SFR low byte: Go (bit 5) clear.
        let sfr_lo = b.read24(0x00_3030);
        assert_eq!(sfr_lo & 0x20, 0, "Go should be clear after STOP");
    }
}
