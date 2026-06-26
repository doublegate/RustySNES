//! The DSP-1 board — the µPD77C25 wired into a LoROM/HiROM cartridge.
//!
//! DSP-1 (NEC µPD77C25, the `uPD7725` revision of the shared engine) is the Mode-7 3D-math
//! coprocessor in 15+ titles (Super Mario Kart, Pilotwings, Super Bases Loaded 2). It exposes
//! exactly two memory-mapped ports — the data register (DR) and the status register (SR) — at a
//! board-dependent bus window. There is no canonical per-game window table; this board picks the
//! de-facto window from the map mode + ROM size, the heuristic snes9x/bsnes use when no cartridge
//! database is present, which coincides with every ares DSP-1 board definition:
//!
//! | Map mode / size        | DSP window (banks : addr)        | DR / SR split            |
//! |------------------------|----------------------------------|--------------------------|
//! | HiROM                  | `$00–$1F,$80–$9F : $6000–$7FFF`   | DR `$6xxx`, SR `$7xxx`   |
//! | LoROM, ROM ≤ 1 MiB     | `$30–$3F,$B0–$BF : $8000–$FFFF`   | DR `$8–$Bxxx`, SR `$C–$F`|
//! | LoROM, ROM > 1 MiB     | `$60–$6F,$E0–$EF : $0000–$7FFF`   | DR `$0–$3xxx`, SR `$4–$7`|
//!
//! ROM and SRAM decode is delegated to the wrapped base board; only the DSP window is
//! intercepted. The board is functional only once the user supplies the `dsp1.rom` / `dsp1b.rom`
//! firmware dump (`docs/adr/0003` — a chip-ROM-dump coprocessor is never silently degraded).

// Chip-name jargon (µPD77C25, DSP-1, …) is not Rust code.
#![allow(clippy::doc_markdown)]

use alloc::boxed::Box;

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::coproc::upd77c25::{Revision, Upd77c25};
use crate::header::MapMode;

/// Which bus window the DSP-1 DR/SR ports occupy, selected from the cart's map mode + size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DspWindow {
    /// HiROM: `$00–$1F,$80–$9F : $6000–$7FFF`; SR when `addr & 0x1000`.
    HiRom,
    /// LoROM ≤ 1 MiB: `$30–$3F,$B0–$BF : $8000–$FFFF`; SR when `addr & 0x4000`.
    LoRomSmall,
    /// LoROM > 1 MiB: `$60–$6F,$E0–$EF : $0000–$7FFF`; SR when `addr & 0x4000`.
    LoRomLarge,
}

impl DspWindow {
    /// Pick the window for a DSP-1 cart of `map_mode` with `rom_len` bytes of ROM.
    const fn select(map_mode: MapMode, rom_len: usize) -> Self {
        match map_mode {
            MapMode::HiRom | MapMode::ExHiRom => Self::HiRom,
            MapMode::LoRom if rom_len > 0x10_0000 => Self::LoRomLarge,
            MapMode::LoRom => Self::LoRomSmall,
        }
    }

    /// Classify a 24-bit CPU address: `Some(true)` = SR port, `Some(false)` = DR port, `None` =
    /// not in the DSP window (delegate to the base board).
    fn classify(self, addr24: u32) -> Option<bool> {
        let bank = (addr24 >> 16) & 0x7F;
        let addr = addr24 & 0xFFFF;
        match self {
            Self::HiRom => ((0x00..=0x1F).contains(&bank) && (0x6000..=0x7FFF).contains(&addr))
                .then_some(addr & 0x1000 != 0),
            Self::LoRomSmall => {
                ((0x30..=0x3F).contains(&bank) && addr >= 0x8000).then_some(addr & 0x4000 != 0)
            }
            Self::LoRomLarge => {
                ((0x60..=0x6F).contains(&bank) && addr < 0x8000).then_some(addr & 0x4000 != 0)
            }
        }
    }
}

/// A LoROM/HiROM cartridge carrying a DSP-1 (µPD77C25).
pub struct Dsp1Board {
    inner: Box<dyn Board>,
    dsp: Upd77c25,
    window: DspWindow,
    name: &'static str,
}

impl core::fmt::Debug for Dsp1Board {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Dsp1Board")
            .field("name", &self.name)
            .field("inner", &self.inner.name())
            .field("window", &self.window)
            .field("firmware_loaded", &self.dsp.firmware_loaded())
            .finish()
    }
}

impl Dsp1Board {
    /// Wrap a base board (`inner`, a LoROM or HiROM board over the cart's ROM/SRAM) with a
    /// DSP-1, selecting the bus window from `map_mode` + `rom_len`. The DSP is inert until
    /// [`Dsp1Board::load_firmware`] supplies the chip dump.
    #[must_use]
    pub fn new(inner: Box<dyn Board>, map_mode: MapMode, rom_len: usize) -> Self {
        let window = DspWindow::select(map_mode, rom_len);
        let name = match window {
            DspWindow::HiRom => "HiROM+DSP-1",
            DspWindow::LoRomSmall | DspWindow::LoRomLarge => "LoROM+DSP-1",
        };
        Self {
            inner,
            dsp: Upd77c25::new(Revision::Upd7725),
            window,
            name,
        }
    }

    /// Whether the DSP-1 firmware has been supplied (the chip is functional).
    #[must_use]
    pub const fn firmware_loaded(&self) -> bool {
        self.dsp.firmware_loaded()
    }
}

impl Board for Dsp1Board {
    fn name(&self) -> &'static str {
        self.name
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::Dsp
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        if self.window.classify(addr24).is_some() {
            MappedAddr::Coprocessor
        } else {
            self.inner.map(addr24)
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        match self.window.classify(addr24) {
            Some(true) => self.dsp.read_sr(),
            Some(false) => self.dsp.read_dr(),
            None => self.inner.read24(addr24),
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        match self.window.classify(addr24) {
            Some(true) => self.dsp.write_sr(val),
            Some(false) => self.dsp.write_dr(val),
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
        self.dsp.load_firmware(bytes)
    }

    fn coprocessor_host_accesses(&self) -> u64 {
        self.dsp.host_accesses()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::LoRom;
    use alloc::vec;

    fn dsp_lorom_small() -> Dsp1Board {
        let inner = Box::new(LoRom::new(
            vec![0u8; 0x8_0000].into_boxed_slice(),
            vec![].into_boxed_slice(),
        ));
        Dsp1Board::new(inner, MapMode::LoRom, 0x8_0000)
    }

    #[test]
    fn window_split_lorom_small() {
        let w = DspWindow::LoRomSmall;
        assert_eq!(w.classify(0x30_8000), Some(false)); // DR
        assert_eq!(w.classify(0x30_C000), Some(true)); // SR
        assert_eq!(w.classify(0xB0_8000), Some(false)); // mirror DR
        assert_eq!(w.classify(0x00_8000), None); // ROM, not DSP
    }

    #[test]
    fn window_split_hirom_and_large() {
        assert_eq!(DspWindow::HiRom.classify(0x00_6000), Some(false));
        assert_eq!(DspWindow::HiRom.classify(0x00_7000), Some(true));
        assert_eq!(DspWindow::LoRomLarge.classify(0x60_0000), Some(false));
        assert_eq!(DspWindow::LoRomLarge.classify(0x60_4000), Some(true));
    }

    #[test]
    fn inert_without_firmware() {
        let mut b = dsp_lorom_small();
        assert!(!b.firmware_loaded());
        // SR/DR read as open-bus-ish zero until firmware is supplied.
        assert_eq!(b.read24(0x30_C000), 0);
        b.write24(0x30_8000, 0x42);
        assert_eq!(b.read24(0x30_8000), 0);
    }

    #[test]
    fn rejects_short_firmware() {
        let mut b = dsp_lorom_small();
        assert!(!b.load_firmware(&[0u8; 16]));
        assert!(!b.firmware_loaded());
    }
}
