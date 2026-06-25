//! `rustysnes-apu` — SPC700 + S-DSP (audio).
//!
//! Sony SPC700 audio CPU + S-DSP + 64 KiB ARAM; async ~1.024 MHz; BRR samples + 8 voices.
//! The SPC700 is a SECOND CPU on its own clock domain: the scheduler runs it on the same
//! master timeline as the 65C816 but on its own divisor, resynchronizing only at the four
//! `$2140-$2143` communication ports (modeled in `rustysnes-core` — NOT a thread, so
//! determinism holds). This crate owns the SPC700 + S-DSP + ARAM; the only thing it reaches
//! through is its own [`AudioBus`] (ARAM/DSP register access + IRQ raise).
//!
//! Part of the one-directional chip-crate graph (see `docs/architecture.md`): this crate
//! does NOT depend on the other chip crates. `#![no_std]` + alloc so it cross-compiles to a
//! bare-metal target; only the frontend carries `std` + `unsafe`.

#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

/// The narrow bus the SPC700/S-DSP sees — the SNES port of RustyNES's `ApuBus`.
///
/// The APU subsystem owns its 64 KiB ARAM internally, so most access is local; this trait is
/// the seam for anything the core mediates (e.g. surfacing the four CPU↔APU port latches and
/// raising the audio-side timer IRQ). Every method has a default so a test impl is trivial.
pub trait AudioBus {
    /// Read one of the four CPU↔APU communication-port latches (`port` 0..=3). This is the
    /// resynchronization boundary between the two clock domains. Default `0`.
    fn read_port(&mut self, _port: u8) -> u8 {
        0
    }

    /// Write one of the four CPU↔APU communication-port latches (`port` 0..=3). Default no-op.
    fn write_port(&mut self, _port: u8, _val: u8) {}

    /// Raise the SPC700-side timer IRQ. Default no-op.
    fn raise_irq(&mut self) {}
}

/// A no-op [`AudioBus`] for unit-testing the APU in isolation.
#[derive(Debug, Default)]
pub struct NullAudioBus;

impl AudioBus for NullAudioBus {}

/// SPC700 + S-DSP state. Replace this stub with the real model; pin behavior against the test
/// ROMs FIRST (test-ROM-is-spec), then implement until they pass.
#[derive(Debug, Default, Clone)]
pub struct Apu {
    // TODO(T-03): SPC700 registers + S-DSP voice state + 64 KiB ARAM + the two SPC700 timers
    // per `docs/apu.md`.
    /// Fractional master-clock accumulator: the SPC700 runs at ~1.024 MHz against the
    /// 21.477 MHz master clock (a non-integer ratio), so the scheduler hands this domain
    /// master ticks and it advances when enough have accrued. See `docs/scheduler.md`
    /// §SPC700 clock domain.
    spc_accum: u64,
}

impl Apu {
    /// Construct at power-on. Phase alignment comes from a *seeded* PRNG (determinism
    /// contract — see `docs/adr/0004`), never the OS RNG.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the SPC700/S-DSP by one of ITS clock cycles. The scheduler decides when this
    /// fires (when its fractional accumulator crosses the divisor); this method is the SPC700
    /// step itself. Hot path: keep allocation-free.
    // reason: a real SPC700 step mutates state; `const` fits only the empty skeleton body.
    #[allow(clippy::missing_const_for_fn)]
    pub fn tick(&mut self, bus: &mut impl AudioBus) {
        // TODO(T-03): one SPC700 instruction-cycle + the S-DSP sample tick; resync the four
        // ports via `bus` at port-access boundaries only.
        let _ = bus;
        let _ = self.spc_accum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs() {
        let _ = Apu::new();
    }

    #[test]
    fn ticks_against_null_bus() {
        let mut apu = Apu::new();
        let mut bus = NullAudioBus;
        apu.tick(&mut bus);
    }
}
