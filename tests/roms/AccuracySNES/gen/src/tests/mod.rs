//! The test definitions, one module per group.
//!
//! **All seven groups now have shipped tests, and every one of them is partial.** Group A (CPU)
//! landed in Phase A and is the fullest; C (PPU) and B (5A22 bus/clock/timing) followed in Phase B;
//! D (DMA/HDMA), E (SPC700 + S-DSP), F (controller ports) and G (cartridge/memory map) have since
//! opened. `docs/accuracysnes-coverage.md` is the current per-assertion count — it is regenerated
//! with the ROM, so unlike a comment it cannot drift. `docs/accuracysnes-plan.md` says what blocks
//! each remaining block, and `to-dos/ROADMAP.md` carries the T-04-* ticket IDs.

pub mod apu;
pub mod bus;
pub mod cart;
pub mod cpu;
pub mod dma;
pub mod hirom;
pub mod input;
pub mod ppu;
pub mod sweep;

use crate::dsl::Test;

/// Every test in the LoROM battery, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    let mut v = cpu::all();
    v.extend(ppu::all());
    v.extend(bus::all());
    v.extend(dma::all());
    v.extend(apu::all());
    v.extend(input::all());
    v.extend(cart::all());
    v.extend(sweep::all());
    v
}

/// The HiROM image's battery — the Group G rows that need a HiROM cartridge layout to observe.
///
/// Emitted into `build/accuracysnes-hirom.sfc` (linked with `hirom.cfg` + `header-hirom.s`), never
/// into the LoROM image. Small by construction: it shares the LoROM runtime and exists only to
/// self-score the HiROM decode and SRAM window.
#[must_use]
pub fn hirom() -> Vec<Test> {
    hirom::all()
}

/// The ExHiROM image's battery — the Group G row (`G1.16`) that needs the extended >4 MiB two-half
/// layout to observe the A23->A22 half-selection. Emitted into `build/accuracysnes-exhirom.sfc`.
#[must_use]
pub fn exhirom() -> Vec<Test> {
    hirom::exhirom_all()
}
