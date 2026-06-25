//! The master-clock lockstep scheduler — the heart of the emulator.
//!
//! Timing master: the 21.477 MHz SNES master crystal. One [`System::tick_one_master`]
//! advances the master clock by one unit; every other chip advances when its divisor fires.
//! LOCKSTEP, not catch-up — this is why mid-instruction timing-master events (an HV-IRQ at a
//! precise dot, a mid-scanline register write seen by the next instruction) work without
//! per-quirk patches.
//!
//! ## The divisor table (exact counts are TODO — the docs agent fills `docs/scheduler.md`)
//!
//! - **65C816 (main CPU):** region-variable. A memory access costs 6 master cycles (FastROM,
//!   3.58 MHz), 8 (SlowROM, 2.68 MHz), or 12 (the slow I/O / `$4016` region, 1.79 MHz).
//!   `MASTER_PER_CPU_FAST/SLOW/XSLOW` below are placeholders pending the research pass.
//! - **PPU1/PPU2 (video):** the dot clock. ~4 master cycles per dot (the exact dot-stretch on
//!   dots 323/327 of certain scanlines is a TODO). The PPU is ticked one dot at a time.
//! - **SPC700 + S-DSP (audio):** a SECOND, asynchronous ~1.024 MHz clock domain (see below).
//!
//! ## The SPC700 second clock domain
//!
//! The SPC700 runs from its own ~24.576 MHz crystal (≈1.024 MHz after its /24 divider), which
//! is NOT a clean integer ratio of the 21.477 MHz main master clock. Modeling it as a separate
//! OS thread would break the determinism contract, so instead we run it on the SAME master
//! timeline: every master tick adds `SPC_NUM` to a fractional accumulator and the SPC700
//! advances one of its cycles each time the accumulator crosses `SPC_DEN`. The two domains are
//! only ever *resynchronized* — they never share state directly — at the four `$2140-$2143`
//! communication ports (the [`rustysnes_apu::AudioBus`] read/write-port boundary). Because the
//! accumulator is deterministic, the cross-domain port handshake is reproducible bit-for-bit.

use rustysnes_apu::AudioBus;
use rustysnes_cpu::Bus as CpuBus;

use crate::bus::Bus;

// --- Divisor placeholders. Exact values come from the Step-2 research pass; the docs agent
// fills `docs/scheduler.md`. Kept here so the loop shape is concrete and compiling. ---

/// Master cycles per master tick of the FastROM (3.58 MHz) 65C816 access. TODO(T-21): verify.
const MASTER_PER_CPU_FAST: u32 = 6;
/// Master cycles per master tick of the PPU dot clock. TODO(T-21): verify dot-stretch.
const MASTER_PER_DOT: u32 = 4;
/// SPC700 fractional-clock numerator (master ticks → SPC cycles). TODO(T-21): exact ratio.
const SPC_NUM: u64 = 1_024_000;
/// SPC700 fractional-clock denominator. TODO(T-21): exact ratio (≈ the master clock Hz).
const SPC_DEN: u64 = 21_477_270;

/// Owns the run loop. Determinism contract: same seed + ROM + input => bit-identical AV.
#[derive(Debug)]
pub struct System {
    /// The Bus — owns everything mutable (PPU/APU/cart/WRAM/controllers/DMA).
    pub bus: Bus,
    /// Per-power-on CPU/PPU phase alignment, from a SEEDED PRNG (never OS RNG).
    phase: u8,
    /// Cumulative master-clock ticks since power-on.
    master_ticks: u64,
    /// Master cycles still owed to the PPU before its next dot fires.
    dot_accum: u32,
    /// Fractional accumulator for the asynchronous SPC700 clock domain (see module docs).
    spc_accum: u64,
}

impl System {
    /// Power on with a determinism seed (drives the phase alignment).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            bus: Bus::default(),
            // TODO(T-21): derive the full CPU/PPU phase from a seeded PRNG; `% 4` is a stub.
            phase: (seed % 4) as u8,
            master_ticks: 0,
            dot_accum: 0,
            spc_accum: 0,
        }
    }

    /// Advance one unit of the master clock, firing each chip's divisor in lockstep order.
    ///
    /// This is the single owner of the run loop. The CPU/PPU/coprocessor/SPC700 advance only
    /// from here; nothing else drives time. `docs/scheduler.md` holds the authoritative table.
    // reason: the real loop drives the chips (mutating the bus); `const` fits only the
    // accumulator-only skeleton and must be removed when the chip ticks are wired in.
    #[allow(clippy::missing_const_for_fn)]
    pub fn tick_one_master(&mut self) {
        self.master_ticks = self.master_ticks.wrapping_add(1);

        // --- PPU dot clock: advance one dot every `MASTER_PER_DOT` master cycles. ---
        self.dot_accum += 1;
        if self.dot_accum >= MASTER_PER_DOT {
            self.dot_accum -= MASTER_PER_DOT;
            // The PPU sees the cart-mediated VideoBus view of the Bus.
            // TODO(T-21): split-borrow so `self.bus.ppu.tick_dot(&mut self.bus)` is possible;
            // today the PPU tick is staged behind the real model. See `docs/ppu.md`.
        }

        // --- SPC700 second clock domain: fractional accumulator (see module docs). ---
        self.spc_accum += SPC_NUM;
        while self.spc_accum >= SPC_DEN {
            self.spc_accum -= SPC_DEN;
            // The SPC700/S-DSP advance one of THEIR cycles; resync only at the four ports.
            // TODO(T-21): `self.bus.apu.tick(&mut self.bus)` once the split-borrow lands.
        }

        // TODO(T-21): the 65C816 advances every `MASTER_PER_CPU_FAST`/SLOW/XSLOW master
        // cycles depending on the region of the address it is accessing; an in-flight DMA
        // steals the bus. Drive `Cpu::step(&mut self.bus)` from here.
        let _ = (self.phase, MASTER_PER_CPU_FAST);
    }

    /// Run until the next frame boundary (one vblank). Convenience wrapper for the frontend /
    /// harness; the real boundary detection lands with the PPU model.
    // reason: the real loop calls `tick_one_master` (mutating); `const` fits only the no-op stub.
    #[allow(clippy::missing_const_for_fn, clippy::unused_self)]
    pub fn run_frame(&mut self) {
        // TODO(T-21): loop `tick_one_master` until the PPU signals end-of-frame.
    }
}

/// Compile-time proof that `Bus` satisfies every narrow chip-bus trait the lockstep
/// scheduler relies on. A change that breaks one of these fails the build here.
/// (`VideoBus` is also impl'd on `Bus`; verified by the `bus` module's tests.)
const fn _assert_bus_traits<B: CpuBus + AudioBus>() {}
const _: fn() = _assert_bus_traits::<Bus>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_advance_master_clock() {
        let mut sys = System::new(0);
        for _ in 0..MASTER_PER_DOT * 4 {
            sys.tick_one_master();
        }
        assert_eq!(sys.master_ticks, u64::from(MASTER_PER_DOT) * 4);
    }

    #[test]
    fn spc_domain_accrues_deterministically() {
        // Same seed => identical SPC accumulator trajectory (determinism contract).
        let mut a = System::new(7);
        let mut b = System::new(7);
        for _ in 0..1000 {
            a.tick_one_master();
            b.tick_one_master();
        }
        assert_eq!(a.spc_accum, b.spc_accum);
    }
}
