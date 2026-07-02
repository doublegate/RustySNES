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
use crate::sa1_bus::Sa1Bus;

/// A generous instruction budget per frame so a wedged ROM can't spin forever in `run_frame`.
const MAX_STEPS_PER_FRAME: u64 = 2_000_000;

/// The SA-1 65C816 runs at ~10.74 MHz = master clock / 2, so each SA-1 CPU cycle is **2 master
/// clocks**. The scheduler advances the SA-1 in a deterministic catch-up bounded by the master
/// clock that the (untouched) main CPU has already advanced.
const SA1_MASTER_PER_CYCLE: u64 = 2;

/// Safety cap on SA-1 instructions executed in a single catch-up call (a wedged SA-1 program can't
/// spin forever); far above any real per-step budget.
const MAX_SA1_STEPS_PER_CALL: u32 = 200_000;

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
    /// The second 65C816 (the SA-1's CPU), present only when an SA-1 cart is installed. Stepped in
    /// deterministic catch-up against the main CPU's master-clock advance (`docs/scheduler.md`
    /// §SA-1). `None` for every non-SA-1 cart, so the main CPU's behaviour/timing is unchanged.
    sa1_cpu: Option<Cpu>,
    /// Master-clock value last accounted to the SA-1 catch-up (delta = now − this).
    sa1_last_master: u64,
    /// Sub-cycle master-clock credit carried between SA-1 catch-up calls.
    sa1_credit: u64,
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
            sa1_cpu: None,
            sa1_last_master: 0,
            sa1_credit: 0,
        }
    }

    /// Reset the CPU from the cart's emulation reset vector (`$00FFFC`). Safe to call with no
    /// cart (the CPU reads open bus and parks); the boot flag tracks readiness.
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus);
        self.booted = self.bus.cart.is_some();
        self.last_line = self.bus.ppu.scanline();
        // Instantiate the SA-1's second CPU iff the installed cart carries one. It stays held in
        // reset (the SA-1 board powers up with RESB asserted) until the main CPU clears RESB, at
        // which point `run_sa1` resets it from the SA-1 reset vector (CRV).
        self.sa1_cpu = self
            .bus
            .cart
            .as_ref()
            .filter(|c| c.board.has_second_cpu())
            .map(|_| Cpu::new());
        self.sa1_last_master = self.bus.clock.master;
        self.sa1_credit = 0;
    }

    /// Advance the SA-1's second CPU to catch up with the master clock the main CPU has elapsed
    /// since the last call. Deterministic and bounded entirely by `bus.clock.master` (which is a
    /// pure function of the untouched main CPU), so installing the second CPU never perturbs the
    /// main CPU's behaviour or the existing scheduler timing.
    fn run_sa1(&mut self) {
        let Some(mut cpu) = self.sa1_cpu.take() else {
            return;
        };
        let now = self.bus.clock.master;
        let delta = now.wrapping_sub(self.sa1_last_master);
        self.sa1_last_master = now;
        let mut credit = self.sa1_credit + delta;

        if let Some(cart) = self.bus.cart.as_mut() {
            let board = cart.board.as_mut();
            if board.has_second_cpu() {
                if board.second_cpu_take_reset() {
                    let mut adapter = Sa1Bus { board: &mut *board };
                    cpu.reset(&mut adapter);
                }
                let mut guard = 0u32;
                while credit >= SA1_MASTER_PER_CYCLE && guard < MAX_SA1_STEPS_PER_CALL {
                    guard += 1;
                    if board.second_cpu_running() {
                        let cyc = {
                            let mut adapter = Sa1Bus { board: &mut *board };
                            cpu.step(&mut adapter)
                        };
                        // SA-1 cycles → master clocks (×2). `cyc` is a single instruction's count,
                        // so this never overflows a u32.
                        let clocks = cyc.max(1).saturating_mul(2);
                        board.second_cpu_tick(clocks);
                        credit = credit.saturating_sub(u64::from(clocks));
                    } else {
                        // Held in reset / asleep: drain the budget into the timer in one go (keeps
                        // the H/V counters advancing) and stop stepping the CPU.
                        let drain = credit & !1;
                        board.second_cpu_tick(u32::try_from(drain).unwrap_or(u32::MAX) & !1);
                        credit &= 1;
                    }
                }
            } else {
                credit = 0;
            }
        } else {
            credit = 0;
        }

        self.sa1_credit = credit;
        self.sa1_cpu = Some(cpu);
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

        // HDMA per-frame init + per-line transfers are now driven clock-accurately from
        // `Bus::advance_master` (at V=0 and each visible line), so they stay correct even when a
        // framebuffer DMA spans the frame boundary. The scheduler no longer sequences HDMA.

        while self.bus.ppu.frame_count() == start_frame && steps < MAX_STEPS_PER_FRAME {
            self.cpu.step(&mut self.bus);
            steps += 1;

            // HDMA is now serviced clock-accurately inside `Bus::advance_master` (so it stays
            // line-accurate even mid-GP-DMA); the scheduler no longer polls scanline boundaries.

            // Catch the SA-1 up to the master clock (no-op when no SA-1 cart is installed).
            if self.sa1_cpu.is_some() {
                self.run_sa1();
            }
        }
    }

    /// Cumulative cycles the SA-1's second CPU has executed since power-on, or `None` when no SA-1
    /// cart is installed. A non-zero value is the SA-1 liveness signal: the second 65C816 actually
    /// fetched + executed out of the cart ROM (many SA-1 titles run their main logic on the SA-1).
    #[must_use]
    pub fn sa1_cycles(&self) -> Option<u64> {
        self.sa1_cpu.as_ref().map(|c| c.cycles)
    }

    /// Step a single CPU instruction (drives the whole machine in lockstep via the Bus).
    pub fn step_instruction(&mut self) {
        if !self.booted {
            self.reset();
        }
        self.cpu.step(&mut self.bus);
        if self.sa1_cpu.is_some() {
            self.run_sa1();
        }
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
