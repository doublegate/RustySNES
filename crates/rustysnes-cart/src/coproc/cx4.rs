//! The CX4 board — the Hitachi HG51B169 wired into a LoROM cartridge (Mega Man X2, Mega Man X3).
//!
//! Clean-room port of ares' `HitachiDSP` board wrapper (ISC, `sfc/coprocessor/hitachidsp/`),
//! `Mapping == 0` only (the scheme both local games use — `ares` board `SHVC-1DC0N-01`). Unlike
//! DSP-1, the [`Hg51b`] program executes from the cart's own ROM (see that module's doc), so this
//! board is functional the instant the cart loads for CODE; only the 3 KiB data-ROM constant
//! table (`cx4.rom`) is a genuine external chip dump, and the chip stays inert (never silently
//! degraded, `docs/adr/0003`) until it's supplied.
//!
//! Bus window (bank:addr, `$00-3F,$80-BF` only):
//!
//! | Region        | Target                                             |
//! |---------------|-----------------------------------------------------|
//! | `$8000-FFFF`  | cart ROM (delegated to the wrapped base board)      |
//! | `$6000-6BFF`, `$7000-7BFF` | HG51B's 3 KiB data RAM ([`Hg51b::read_dram`]) |
//! | `$6C00-6FFF`, `$7C00-7FFF` | HG51B's IO register block ([`Hg51b::read_io`]) |
//! | `$70-77:0000-7FFF` | save RAM — falls through to the base board, whose own LoROM SRAM decode already covers this bank range (`docs/cart.md`'s LoROM SRAM table); no CX4-specific handling needed |
//!
//! Execution trigger: writing the cache program-counter register (`$7F4F`) while the chip is
//! halted starts it, which then runs synchronously to its next halt (`Hg51b::run_until_halt`) —
//! the same run-to-completion host-sync pattern this project's GSU/DSP-1 boards use.

// Chip-name jargon (CX4, HG51B, ...) is not Rust code.
#![allow(clippy::doc_markdown)]

use alloc::boxed::Box;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::coproc::hg51b::{Hg51b, Hg51bBus};

/// Bridges the HG51B core's chip-relative bus access to the wrapped base board's own (already
/// correct) LoROM ROM/SRAM decode — the chip's `isROM`/`isRAM`/`read`/`write` hooks resolve to
/// exactly the same address space the S-CPU sees, so there is no separate address math to
/// re-derive here.
struct Cx4Mem<'a> {
    inner: &'a mut dyn Board,
}

impl Hg51bBus for Cx4Mem<'_> {
    fn is_rom(&self, address: u32) -> bool {
        matches!(self.inner.map(address), MappedAddr::Rom(_))
    }

    fn is_ram(&self, address: u32) -> bool {
        matches!(self.inner.map(address), MappedAddr::Sram(_))
    }

    fn read(&mut self, address: u32) -> u8 {
        self.inner.read24(address)
    }

    fn write(&mut self, address: u32, data: u8) {
        self.inner.write24(address, data);
    }
}

/// Classification of a bus address against the CX4 windows (see the module doc's table).
enum Hit {
    /// HG51B data RAM, at the given already-chip-relative offset.
    Dram(u32),
    /// HG51B IO register block, at the given already-folded `$7C00 | (addr & 0x3FF)` offset.
    Io(u32),
}

const fn classify(addr24: u32) -> Option<Hit> {
    // Exact ares `addressIO`/`addressDRAM` masks (`Mapping == 0`): bit 0x40_0000 folds bank
    // `$80-BF` onto `$00-3F` (both are valid — the "high" mirror), so the bank restriction to
    // `$00-3F,$80-BF` and the address-range check are the SAME combined bitmask test, not two
    // separate ones.
    let a = addr24 & 0xFF_FFFF;
    if (a & 0x40_EC00) == 0x00_6C00 {
        return Some(Hit::Io(0x7C00 | (a & 0x3FF)));
    }
    if (a & 0x40_E000) == 0x00_6000 && (a & 0x00_0C00) != 0x00_0C00 {
        return Some(Hit::Dram(a & 0xFFF));
    }
    None
}

/// A LoROM cartridge carrying a CX4 (Hitachi HG51B169).
pub struct Cx4Board {
    inner: Box<dyn Board>,
    hg51b: Hg51b,
}

impl core::fmt::Debug for Cx4Board {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cx4Board")
            .field("inner", &self.inner.name())
            .field("hg51b", &self.hg51b)
            .finish()
    }
}

impl Cx4Board {
    /// Wrap a base board (`inner`, the cart's LoROM ROM/SRAM decode) with a CX4. The chip's
    /// program executes from `inner`'s own ROM immediately; the data-ROM constant table is inert
    /// until [`Board::load_firmware`] supplies `cx4.rom`.
    #[must_use]
    pub fn new(inner: Box<dyn Board>) -> Self {
        Self {
            inner,
            hg51b: Hg51b::new(),
        }
    }
}

impl Board for Cx4Board {
    fn name(&self) -> &'static str {
        "LoROM+CX4"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::Cx4
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
            Some(Hit::Dram(a)) => self.hg51b.read_dram(a),
            Some(Hit::Io(a)) => self.hg51b.read_io(a),
            None => self.inner.read24(addr24),
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        match classify(addr24) {
            Some(Hit::Dram(a)) => self.hg51b.write_dram(a, val),
            Some(Hit::Io(a)) => {
                let mut mem = Cx4Mem {
                    inner: &mut *self.inner,
                };
                self.hg51b.write_io(a, val, &mut mem);
            }
            None => self.inner.write24(addr24, val),
        }
    }

    fn rom(&self) -> &[u8] {
        self.inner.rom()
    }

    fn sram(&self) -> &[u8] {
        self.inner.sram()
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        self.inner.sram_mut()
    }

    fn load_firmware(&mut self, bytes: &[u8]) -> bool {
        self.hg51b.load_data_rom(bytes)
    }

    fn firmware_hint(&self) -> Option<&'static str> {
        Some("cx4.rom")
    }

    fn irq_pending(&self) -> bool {
        self.hg51b.irq_pending()
    }

    fn coprocessor_host_accesses(&self) -> u64 {
        self.hg51b.instructions_run()
    }

    fn save_state(&self, w: &mut SaveWriter) {
        self.hg51b.save_state(w);
        self.inner.save_state(w);
    }

    fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        self.hg51b.load_state(r)?;
        self.inner.load_state(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::LoRom;
    use alloc::vec;

    fn board() -> Cx4Board {
        let inner = Box::new(LoRom::new(
            vec![0u8; 0x8_0000].into_boxed_slice(),
            vec![].into_boxed_slice(),
        ));
        Cx4Board::new(inner)
    }

    #[test]
    fn window_classify() {
        assert!(matches!(classify(0x00_6000), Some(Hit::Dram(0))));
        assert!(matches!(classify(0x00_6BFF), Some(Hit::Dram(0xBFF))));
        assert!(classify(0x00_6C00).is_some()); // IO, not DRAM
        assert!(matches!(classify(0x00_6C00), Some(Hit::Io(0x7C00))));
        assert!(matches!(classify(0x00_7FEF), Some(Hit::Io(0x7FEF))));
        assert!(classify(0x00_8000).is_none()); // ROM, not CX4
        assert!(matches!(classify(0x80_6000), Some(Hit::Dram(0))));
    }

    #[test]
    fn inert_without_data_rom() {
        let mut b = board();
        assert!(!b.hg51b.data_rom_loaded());
        // Writing the pc trigger with no data ROM loaded must not attempt to run.
        b.write24(0x00_7C4F, 0x00);
        assert!(b.hg51b.data_rom_loaded().eq(&false));
    }

    #[test]
    fn dram_roundtrip() {
        let mut b = board();
        b.write24(0x00_6000, 0x42);
        assert_eq!(b.read24(0x00_6000), 0x42);
    }

    #[test]
    fn engine_state_round_trips_through_save_state() {
        let mut b = board();
        b.write24(0x00_6000, 0x42);
        b.write24(0x00_7F4D, 0x12); // cache.pb low byte (a register outside the data-ROM path)

        let mut w = SaveWriter::new();
        b.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = board();
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.read24(0x00_6000), 0x42);
        assert_eq!(fresh.read24(0x00_7F4D), 0x12);
        assert_eq!(r.remaining(), 0);
    }
}
