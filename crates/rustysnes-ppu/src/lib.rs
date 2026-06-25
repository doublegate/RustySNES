//! `rustysnes-ppu` — PPU1 (5C77) + PPU2 (5C78) (video).
//!
//! Dual-chip PPU: BG modes 0-7 (incl. Mode 7 affine), OAM sprites, the dot-clock timeline.
//! The PPU owns its own VRAM (64 KiB), CGRAM (palette), and OAM. Anything that has to reach
//! the cartridge — Mode 7 / extended-bank reads on coprocessor boards, board IRQ/scanline
//! notifies — goes through the narrow [`VideoBus`] trait, whose only concrete impl in
//! production is the cart-mediated router in `rustysnes-core`. This is the RustyNES `PpuBus`
//! shape, ported: the video chip depends ONLY on `rustysnes-cart` (its memory bus).
//!
//! Part of the one-directional chip-crate graph (see `docs/architecture.md`): this crate
//! does NOT depend on the cpu/apu chip crates. `#![no_std]` + alloc so it cross-compiles to
//! a bare-metal target; only the frontend carries `std` + `unsafe`.

#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

pub mod bus;
pub use bus::VideoBus;

/// PPU1 (5C77) + PPU2 (5C78) state. Replace this stub with the real model; pin behavior
/// against the test ROMs FIRST (test-ROM-is-spec), then implement until they pass.
#[derive(Debug, Default, Clone)]
pub struct Ppu {
    // TODO(T-02): registers + internal state per `docs/ppu.md` (VRAM, CGRAM, OAM, the
    // BG/OBJ pipeline, the dot/scanline counters, the HV-IRQ comparators).
}

impl Ppu {
    /// Construct at power-on. Phase alignment comes from a *seeded* PRNG (determinism
    /// contract — see `docs/adr/0004`), never the OS RNG.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the PPU by one dot. The scheduler calls this at the dot-clock rate; the dot is
    /// the SNES video chip's finest practical timing quantum. Hot path: allocation-free.
    // reason: a real dot mutates PPU + bus; `const` fits only the empty skeleton body.
    #[allow(clippy::missing_const_for_fn, clippy::unused_self)]
    pub fn tick_dot(&mut self, bus: &mut impl VideoBus) {
        // TODO(T-02): one dot of the BG/OBJ pipeline; consult `bus` only for cart-mediated
        // (Mode 7 / coprocessor) reads; fire HV-IRQ comparators here for mid-scanline events.
        let _ = bus;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs() {
        let _ = Ppu::new();
    }
}
