//! The [`Board`] trait — the SNES analogue of RustyNES's `Mapper`.
//!
//! A "board" is one cartridge PCB family: a base address-mapping mode (LoROM / HiROM /
//! ExHiROM) plus any on-cart coprocessor. The 65C816 bus calls [`Board::read24`] /
//! [`Board::write24`] with a full 24-bit `(bank << 16) | addr`; the board decodes its own
//! mapping. Coprocessor boards additionally implement the default-no-op hooks
//! ([`Board::coprocessor_tick`], the `notify_*` family) — exactly the RustyNES pattern where
//! per-board IRQ/cycle quirks live INSIDE the board, called via default-no-op trait hooks.
//!
//! See `docs/cart.md` and `docs/mappers.md` for the per-board / per-coprocessor table.

use alloc::boxed::Box;

use crate::header::{Coprocessor as CoproId, Header, MapMode};

/// The result of a board's address decode: where a 24-bit CPU address lands.
///
/// The bus uses this to route a read/write to the right backing store. Stub today; the real
/// decode fills the concrete offsets per the LoROM/HiROM/ExHiROM tables in `docs/cart.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedAddr {
    /// Maps into ROM at the given linear byte offset.
    Rom(u32),
    /// Maps into cartridge SRAM (battery-backed save RAM) at the given offset.
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

    /// Decode a 24-bit CPU address `(bank << 16) | addr` to its backing store.
    fn map(&self, addr24: u32) -> MappedAddr;

    /// Read a byte at a 24-bit CPU address. Default routes through [`Self::map`] over the
    /// board's own storage; coprocessor boards override to intercept their register windows.
    fn read24(&mut self, addr24: u32) -> u8 {
        let _ = self.map(addr24);
        0 // TODO(T-21): read the decoded backing store once ROM/SRAM storage lands.
    }

    /// Write a byte at a 24-bit CPU address (SRAM, coprocessor registers, bank latches).
    fn write24(&mut self, addr24: u32, val: u8) {
        let _ = (addr24, val); // TODO(T-21): SRAM + coprocessor-register writes.
    }

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
}

/// Select the concrete board for a parsed header. Base map mode → base board; a non-`None`
/// coprocessor id → its coprocessor board (all stubs today, tier-annotated in `tier.rs`).
#[must_use]
pub fn select(header: &Header) -> Box<dyn Board> {
    // TODO(T-21): branch on `header.coprocessor` to the SuperFx/Sa1/DspN/... boards once
    // those land. For now every coprocessor falls through to its base map-mode board so the
    // workspace stays coherent and compiling.
    match header.map_mode {
        MapMode::LoRom => Box::new(LoRom::new()),
        MapMode::HiRom => Box::new(HiRom::new()),
        MapMode::ExHiRom => Box::new(ExHiRom::new()),
    }
}

/// LoROM (mode `$20`): 32 KiB ROM banks mapped into the upper half of each bank.
#[derive(Debug, Default, Clone)]
pub struct LoRom {
    // TODO(T-21): ROM/SRAM slices + the bank-decode state.
}

impl LoRom {
    /// Construct a LoROM board.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Board for LoRom {
    fn name(&self) -> &'static str {
        "LoROM"
    }

    fn map(&self, _addr24: u32) -> MappedAddr {
        // TODO(T-21): LoROM decode — `(bank & 0x7F, addr >= 0x8000)` → 32 KiB windowed ROM,
        // `$70-$7D:$0000-$7FFF` → SRAM. See `docs/cart.md` §LoROM.
        MappedAddr::Open
    }
}

/// HiROM (mode `$21`): 64 KiB ROM banks mapped across the full bank.
#[derive(Debug, Default, Clone)]
pub struct HiRom {
    // TODO(T-21): ROM/SRAM slices + the bank-decode state.
}

impl HiRom {
    /// Construct a HiROM board.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Board for HiRom {
    fn name(&self) -> &'static str {
        "HiROM"
    }

    fn map(&self, _addr24: u32) -> MappedAddr {
        // TODO(T-21): HiROM decode. See `docs/cart.md` §HiROM.
        MappedAddr::Open
    }
}

/// ExHiROM (mode `$25`): the extended HiROM layout for >32 Mbit titles.
#[derive(Debug, Default, Clone)]
pub struct ExHiRom {
    // TODO(T-21): ROM/SRAM slices + the extended-bank-decode state.
}

impl ExHiRom {
    /// Construct an ExHiROM board.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Board for ExHiRom {
    fn name(&self) -> &'static str {
        "ExHiROM"
    }

    fn map(&self, _addr24: u32) -> MappedAddr {
        // TODO(T-21): ExHiROM decode. See `docs/cart.md` §ExHiROM.
        MappedAddr::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_boards_default_no_coprocessor() {
        assert_eq!(LoRom::new().coprocessor(), Coprocessor::None);
        assert!(!HiRom::new().irq_pending());
        // Default hooks must be callable + no-op.
        let mut b = ExHiRom::new();
        b.coprocessor_tick();
        b.notify_scanline();
        b.notify_cpu_cycle();
    }
}
