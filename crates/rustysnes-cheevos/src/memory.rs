//! `RetroAchievements` flat-address -> SNES CPU-bus address mapping.
//!
//! `RetroAchievements` addresses SNES memory as a flat space whose first (and,
//! for this pass, only-supported) bank is the console's full 128 KiB WRAM,
//! identity-mapped to `$7E0000..=$7FFFFF`:
//!
//!   - `0x00_0000..0x02_0000` (128 KiB) -> WRAM `$7E0000..=$7FFFFF`
//!
//! Verified against the actual RetroAchievements/RASnes9x integration
//! (`win32/RetroAchievements.cpp`, `RA_InstallMemoryBank(0, ByteReader,
//! ByteWriter, 0x20000)`, whose `ByteReader` returns `Memory.RAM[nOffs %
//! 0x20000]` — a straight, unscrambled 128 KiB WRAM byte offset), not
//! guessed. Cartridge SRAM (`RASnes9x`'s bank 1) is not exposed here — an
//! honest, documented scope cut for this pass (most SNES achievement sets
//! target WRAM; a follow-up can add the SRAM bank once needed).
//!
//! Anything outside the WRAM window has no supported SNES-bus equivalent for
//! achievement purposes and maps to `None` (the trampoline reports 0 bytes
//! read, which rcheevos treats as an invalid address).
//!
//! This is kept here, pure and unit-tested, so the memory source stays
//! agnostic: callers supply a `FnMut(u32) -> u8` peeking the CPU bus (SNES
//! addresses are 24-bit, `$bank:offset`) and never need to know the RA layout.

/// Size of the SNES's WRAM (`$7E0000-$7FFFFF`), and so the size of RA's flat
/// address space this mapping supports.
const WRAM_SIZE: u32 = 0x0002_0000;
/// Base of WRAM on the SNES CPU bus (`$7E0000`).
const WRAM_BASE: u32 = 0x007E_0000;

/// Translate a `RetroAchievements` flat address to a SNES CPU-bus address.
///
/// Returns `None` for addresses that have no supported SNES-bus equivalent
/// (currently: anything past the 128 KiB WRAM window).
#[must_use]
pub const fn ra_addr_to_snes(addr: u32) -> Option<u32> {
    if addr < WRAM_SIZE {
        Some(WRAM_BASE + addr)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wram_window_boundaries() {
        // First WRAM byte: RA flat 0 -> $7E0000.
        assert_eq!(ra_addr_to_snes(0x0000_0000), Some(0x007E_0000));
        // Last WRAM byte: RA flat 0x1FFFF -> $7FFFFF.
        assert_eq!(ra_addr_to_snes(0x0001_FFFF), Some(0x007F_FFFF));
    }

    #[test]
    fn out_of_range_is_none() {
        // One past the WRAM window.
        assert_eq!(ra_addr_to_snes(0x0002_0000), None);
        assert_eq!(ra_addr_to_snes(0xFFFF_FFFF), None);
    }
}
