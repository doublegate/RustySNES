//! The master-clock lockstep scheduler — the run loop that owns the CPU + Bus.
//!
//! Timing master: the 21.477 MHz SNES master crystal. The 65C816 drives the clock: each of its
//! bus accesses advances the master clock by the region access speed (6/8/12), and that advance
//! steps the PPU dot clock + SPC accumulator in lockstep (inside [`crate::Bus`]). This is
//! LOCKSTEP, not catch-up — mid-instruction timing-master events (an HV-IRQ at a precise dot, a
//! mid-scanline register write) land correctly without per-quirk patches (`docs/adr/0001`).
//!
//! The scheduler's job on top of the Bus is the *frame structure*: reset the CPU from the
//! cart's reset vector, step instructions until the PPU signals end-of-frame, and fire the
//! per-line HDMA + the per-frame HDMA setup at the right scanline phases.

use rustysnes_cpu::Cpu;

use crate::bus::Bus;

/// A generous instruction budget per frame so a wedged ROM can't spin forever in `run_frame`.
const MAX_STEPS_PER_FRAME: u64 = 2_000_000;

/// Owns the run loop. Determinism contract: same seed + ROM + input => bit-identical AV.
#[derive(Debug)]
pub struct System {
    /// The Bus — owns everything mutable (PPU/APU/cart/WRAM/controllers/DMA + the master clock).
    pub bus: Bus,
    /// The 65C816 main CPU. It borrows `&mut bus` for each [`Cpu::step`].
    pub cpu: Cpu,
    /// Per-power-on phase alignment, from the determinism seed (never OS RNG).
    seed: u64,
    /// Whether [`System::reset`] has loaded the reset vector for the installed cart.
    booted: bool,
    /// The PPU scanline observed on the previous step (to detect line boundaries for HDMA).
    last_line: u16,
}

impl System {
    /// Power on with a determinism seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            bus: Bus::default(),
            cpu: Cpu::new(),
            seed,
            booted: false,
            last_line: 0,
        }
    }

    /// Reset the CPU from the cart's emulation reset vector (`$00FFFC`). Safe to call with no
    /// cart (the CPU reads open bus and parks); the boot flag tracks readiness.
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus);
        self.booted = self.bus.cart.is_some();
        self.last_line = self.bus.ppu.scanline();
    }

    /// Run one full video frame: step the CPU until the PPU's frame-count advances, firing the
    /// per-frame HDMA setup at the top of the frame and the per-line HDMA at each visible-line
    /// boundary.
    pub fn run_frame(&mut self) {
        if !self.booted {
            self.reset();
        }
        if self.bus.cart.is_none() {
            return; // nothing to run; the frontend shows a blank frame.
        }

        let start_frame = self.bus.ppu.frame_count();
        let mut steps = 0u64;

        // Per-frame HDMA setup happens once at the top of the frame.
        self.bus.hdma_frame_setup();

        while self.bus.ppu.frame_count() == start_frame && steps < MAX_STEPS_PER_FRAME {
            self.cpu.step(&mut self.bus);
            steps += 1;

            // Detect a scanline boundary and run HDMA for the new visible line.
            let line = self.bus.ppu.scanline();
            if line != self.last_line {
                self.last_line = line;
                if line >= 1 && line <= self.bus.ppu.visible_height() {
                    self.bus.run_hdma_line();
                }
            }
        }
    }

    /// Step a single CPU instruction (drives the whole machine in lockstep via the Bus).
    pub fn step_instruction(&mut self) {
        if !self.booted {
            self.reset();
        }
        self.cpu.step(&mut self.bus);
    }

    /// Advance by one CPU instruction (kept for API compatibility with the old skeleton). The
    /// real timebase advances through the CPU's bus accesses, not a bare master tick.
    pub fn tick_one_master(&mut self) {
        let _ = self.seed;
        self.step_instruction();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_system_unbooted() {
        let sys = System::new(0);
        assert!(!sys.booted);
        assert!(sys.bus.cart.is_none());
    }

    #[test]
    fn run_frame_without_cart_is_noop() {
        let mut sys = System::new(0);
        sys.run_frame();
        assert_eq!(sys.bus.ppu.frame_count(), 0);
    }

    #[test]
    fn reset_without_cart_does_not_boot() {
        let mut sys = System::new(0);
        sys.reset();
        assert!(!sys.booted);
    }
}
