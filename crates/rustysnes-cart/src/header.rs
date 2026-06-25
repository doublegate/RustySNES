//! SNES internal-header detection skeleton.
//!
//! Unlike the NES iNES header (a clean 16-byte prefix), the SNES header lives INSIDE the ROM
//! at a map-mode-dependent offset: LoROM `$7FC0`, HiROM `$FFC0`, ExHiROM `$40FFC0` (plus an
//! optional 512-byte copier prefix to skip). Detection scores each candidate offset by
//! checksum / reset-vector / printable-title plausibility and picks the best. This module is
//! the skeleton of that decision; the scoring heuristic itself is a TODO.

use thiserror::Error;

/// Errors from header detection.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderError {
    /// The ROM image is too small to contain any candidate header.
    #[error("rom too small: {0} bytes (need at least one 64 KiB bank)")]
    TooSmall(usize),
    /// No candidate offset scored as a valid SNES header.
    #[error("no valid SNES header found at any candidate offset")]
    NoValidHeader,
}

/// The cartridge map mode (header byte `$xFD5`, low bits). Determines the base board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapMode {
    /// LoROM (`$20`).
    LoRom,
    /// HiROM (`$21`).
    HiRom,
    /// ExHiROM (`$25`).
    ExHiRom,
}

/// Console region, derived from the destination-code header byte (`$xFD9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    /// NTSC (60 Hz) — Japan / North America destination codes.
    Ntsc,
    /// PAL (50 Hz) — Europe / Australia destination codes.
    Pal,
}

/// On-cart coprocessor, derived from the chipset header byte (`$xFD6`) + the chipset-subtype
/// byte. Tier-annotated in [`crate::tier`]. Stubs today; the boards land in later sprints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Coprocessor {
    /// No coprocessor — plain LoROM/HiROM/ExHiROM ROM (+ optional SRAM).
    None,
    /// NEC uPD77C25 DSP-1..4 (Pilotwings, Super Mario Kart, ...).
    Dsp,
    /// Super FX / GSU-1/2 (Star Fox, Yoshi's Island).
    SuperFx,
    /// SA-1 (Super Mario RPG, Kirby Super Star).
    Sa1,
    /// S-DD1 decompression (Star Ocean, Street Fighter Alpha 2).
    SDd1,
    /// SPC7110 decompression + RTC (Far East of Eden Zero).
    Spc7110,
    /// Capcom CX4 (Mega Man X2/X3).
    Cx4,
    /// OBC1 (Metal Combat).
    Obc1,
}

/// A parsed SNES internal header.
///
/// Replace the stub field set / scoring with the real decode; pin against the test ROMs
/// FIRST (test-ROM-is-spec), then implement until detection matches the suites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The byte offset the header was found at (LoROM `$7FC0` / HiROM `$FFC0` / ExHiROM
    /// `$40FFC0`), after any copier-prefix skip.
    pub offset: usize,
    /// The base map mode.
    pub map_mode: MapMode,
    /// The console region.
    pub region: Region,
    /// The on-cart coprocessor (or [`Coprocessor::None`]).
    pub coprocessor: Coprocessor,
    /// ROM size in bytes.
    pub rom_size: usize,
    /// SRAM (battery save-RAM) size in bytes (0 if none).
    pub sram_size: usize,
    /// Whether the cart is battery-backed (has persistent SRAM / RTC).
    pub has_battery: bool,
}

impl Header {
    /// The candidate header offsets (before any 512-byte copier-prefix adjustment).
    const CANDIDATES: [(usize, MapMode); 3] = [
        (0x7FC0, MapMode::LoRom),
        (0xFFC0, MapMode::HiRom),
        (0x40_FFC0, MapMode::ExHiRom),
    ];

    /// Detect the internal header in a raw ROM image.
    ///
    /// # Errors
    /// [`HeaderError::TooSmall`] if the image can't hold a single bank, or
    /// [`HeaderError::NoValidHeader`] if no candidate offset scores as a valid header.
    pub fn detect(rom: &[u8]) -> Result<Self, HeaderError> {
        if rom.len() < 0x8000 {
            return Err(HeaderError::TooSmall(rom.len()));
        }
        // TODO(T-21): skip a 512-byte copier prefix when `len % 0x8000 == 0x200`; score each
        // candidate (complement+checksum pair at `$xFDC`/`$xFDE`, a plausible reset vector at
        // `$xFFC`, a printable 21-byte title at `$xFC0`) and pick the highest. For now pick
        // the first candidate that fits so the workspace is coherent and compiling.
        for (offset, map_mode) in Self::CANDIDATES {
            // The 64-byte header region at `offset..offset+0x40` must fit in the image.
            if rom.len() >= offset + 0x40 {
                return Ok(Self {
                    offset,
                    map_mode,
                    region: Region::Ntsc,
                    coprocessor: Coprocessor::None,
                    rom_size: rom.len(),
                    sram_size: 0,
                    has_battery: false,
                });
            }
        }
        Err(HeaderError::NoValidHeader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn too_small_rejected() {
        assert_eq!(Header::detect(&[]), Err(HeaderError::TooSmall(0)));
    }

    #[test]
    fn one_bank_picks_lorom_offset() {
        let rom = vec![0u8; 0x8000];
        let h = Header::detect(&rom).expect("32 KiB bank should detect");
        assert_eq!(h.map_mode, MapMode::LoRom);
        assert_eq!(h.offset, 0x7FC0);
    }
}
