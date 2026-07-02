//! `rustysnes-apu` — SPC700 (S-SMP) + S-DSP + 64 KiB ARAM (the SNES audio subsystem).
//!
//! Sony SPC700 audio CPU + S-DSP + 64 KiB ARAM on a ~1.024 MHz domain asynchronous to the
//! 21.477 MHz master clock. The SPC700 is a SECOND CPU on its own clock divisor; the scheduler
//! in `rustysnes-core` resynchronizes the two domains only at the four `$2140-$2143`
//! communication ports (and once per scanline), so determinism holds without a thread.
//!
//! Module map:
//! - [`psw`] — the SPC700 status word.
//! - [`spc700`] — the SPC700 core (registers + ALU) generic over [`spc700::Spc700Bus`].
//! - `spc700_exec` — the 256-opcode cycle-accurate dispatch (driven by the oracle).
//! - [`dsp`] — the S-DSP (8 voices, BRR, Gaussian, ADSR/GAIN, noise, echo, mixing).
//! - This file — the [`Apu`] integration surface the core's Bus calls (ARAM, IPL ROM, the
//!   memory-mapped registers `$00F0-$00FF`, the three timers, the four ports, sample retrieval).
//!
//! Part of the one-directional chip-crate graph (`docs/architecture.md`): no dependency on the
//! other chip crates. `#![no_std]` + alloc so it cross-compiles bare-metal; only the frontend
//! carries `std` + `unsafe`.

#![no_std]
#![forbid(unsafe_code)]
// Hardware register machine: register bytes are reinterpreted as signed/unsigned and PC/address
// math truncates by design. The cast and bool-count lints are intentional for this module.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::struct_excessive_bools,
    clippy::large_stack_arrays,
    clippy::missing_const_for_fn
)]
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

pub mod dsp;
pub mod psw;
pub mod spc700;
mod spc700_exec;

mod nostd_math;

use dsp::{ARAM_SIZE, Dsp};
use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};
use spc700::{Spc700, Spc700Bus};

/// SMP base clocks one bus access consumes — ares `cycleWaitStates[0]` (the reset wait state).
///
/// The SMP base clock is `apuFrequency / 12` (ares `SMP::create(apuFrequency()/12, …)`); a normal
/// access is 2 of those base ticks, giving the ~1.024 MHz effective opcode-cycle rate. The
/// per-region external/internal wait-state divider (the glitchy `{2,4,10,20}` table) collapses to
/// this reset default — no committed program reprograms `$F0`'s wait selectors.
const SMP_WAIT: u32 = 2;

/// SMP base clocks per S-DSP **micro-tick**. The S-DSP runs its 32-step voice sequence one
/// [`Dsp::tick`] at a time; 32 ticks = one 32 kHz stereo sample. `apuFrequency = 32040 × 768`, the
/// SMP/DSP base is `apuFrequency / 12`, so one sample is `(32040 × 768 / 12) / 32040 = 64` base
/// clocks, i.e. **32 ticks × 2 base clocks**. Cycle-stepping the DSP (one tick per 2 base clocks)
/// rather than a whole sample per 64 clocks gives a mid-instruction DSP-register read its
/// cycle-correct value.
const DSP_BASE_CLOCKS_PER_TICK: u32 = 2;

/// One micro-step of an in-flight SPC700 instruction, recorded so the core can drain the
/// instruction **one base clock at a time** (cycle-exact lockstep with the main CPU) instead of
/// committing the whole instruction at the budget boundary.
///
/// Each entry carries the number of SMP base clocks the underlying bus operation
/// (`read`/`write`/`idle`) costs and, if that operation was an SMP write to one of the four
/// `$F4-$F7` ports, the `(port, value)` latch update **deferred** to the cycle the operation
/// actually completes on. Reads/idles/ARAM/DSP/timer side effects are applied eagerly when the
/// instruction is recorded (the SMP's internal state is private to the `Apu`, so only the
/// CPU-visible port latch needs sub-instruction timing); the visible port write is the one thing
/// the main CPU can observe mid-instruction, so it is the only deferred effect.
#[derive(Clone, Copy, Default)]
struct CycleStep {
    /// SMP base clocks this micro-op consumes (a normal access = [`SMP_WAIT`]; reads of the
    /// `$F4-$F7` ports split into two half-steps, each `SMP_WAIT >> 1`).
    base_clocks: u32,
    /// A deferred SMP→CPU port-latch write committed when this step's clocks elapse: `Some((n, v))`
    /// sets `io.apu[n] = v` at the precise base cycle, so a CPU read of `$2140+n` at that master
    /// instant observes exactly the value the hardware would have latched.
    port_write: Option<(u8, u8)>,
}

/// The narrow bus the SPC700/S-DSP subsystem surfaces to `rustysnes-core`.
///
/// The APU owns its ARAM and DSP internally, so this trait is only the seam for what the core
/// mediates: the four CPU↔APU port latches (the resync boundary) and the timer IRQ. Every method
/// has a default so a test impl is trivial.
pub trait AudioBus {
    /// Read one of the four CPU↔APU communication-port latches (`port` 0..=3) — what the *CPU*
    /// side last wrote. This is the resynchronization boundary between the two clock domains.
    fn read_port(&mut self, _port: u8) -> u8 {
        0
    }

    /// Write one of the four CPU↔APU communication-port latches (`port` 0..=3) — the *APU* side.
    fn write_port(&mut self, _port: u8, _val: u8) {}

    /// Raise the SPC700-side timer IRQ. Default no-op.
    fn raise_irq(&mut self) {}
}

/// A no-op [`AudioBus`] for unit-testing the APU in isolation.
#[derive(Debug, Default)]
pub struct NullAudioBus;

impl AudioBus for NullAudioBus {}

/// The standard 64-byte SNES IPL boot ROM (`$FFC0-$FFFF`).
///
/// Public-domain Sony S-SMP boot loader — the same 64 bytes in every SNES; reproduced here as a
/// constant (not vendored binary). The reset vector at `[62],[63]` (`$FFC0`) boots the handshake
/// that uploads the audio program over the ports.
pub const IPL_ROM: [u8; 64] = [
    0xCD, 0xEF, 0xBD, 0xE8, 0x00, 0xC6, 0x1D, 0xD0, 0xFC, 0x8F, 0xAA, 0xF4, 0x8F, 0xBB, 0xF5, 0x78,
    0xCC, 0xF4, 0xD0, 0xFB, 0x2F, 0x19, 0xEB, 0xF4, 0xD0, 0xFC, 0x7E, 0xF4, 0xD0, 0x0B, 0xE4, 0xF5,
    0xCB, 0xF4, 0xD7, 0x00, 0xFC, 0xD0, 0xF3, 0xAB, 0x01, 0x10, 0xEF, 0x7E, 0xF4, 0x10, 0xEB, 0xBA,
    0xF6, 0xDA, 0x00, 0xBA, 0xF4, 0xC4, 0xF4, 0xDD, 0x5D, 0xD0, 0xDB, 0x1F, 0x00, 0x00, 0xC0, 0xFF,
];

/// One SPC700 timer (a three-stage divider feeding a 4-bit output counter).
///
/// Timer 0/1 divide the ~1.024 MHz SMP clock to 8 kHz (`DIVISOR` = 128 SMP-clock units); timer 2
/// to 64 kHz (`DIVISOR` = 16). Faithful to ares `SMP::Timer` (stage0..3 + the 1→0 line edge).
#[derive(Debug, Clone, Copy, Default)]
struct Timer {
    divisor: u16,
    stage0: u16,
    stage1: bool,
    stage2: u8,
    stage3: u8, // 4-bit visible counter
    line: bool,
    enable: bool,
    target: u8,
}

impl Timer {
    const fn new(divisor: u16) -> Self {
        Self {
            divisor,
            ..Self::zeroed()
        }
    }

    const fn zeroed() -> Self {
        Self {
            divisor: 0,
            stage0: 0,
            stage1: false,
            stage2: 0,
            stage3: 0,
            line: false,
            enable: false,
            target: 0,
        }
    }

    fn step(&mut self, clocks: u16, timers_enable: bool, timers_disable: bool) {
        self.stage0 += clocks;
        if self.stage0 < self.divisor {
            return;
        }
        self.stage0 -= self.divisor;
        self.stage1 = !self.stage1;
        self.sync_stage1(timers_enable, timers_disable);
    }

    fn sync_stage1(&mut self, timers_enable: bool, timers_disable: bool) {
        let level = self.stage1 && timers_enable && !timers_disable;
        // pulse only on a 1→0 transition of the line
        let fell = self.line && !level;
        self.line = level;
        if !fell || !self.enable {
            return;
        }
        self.stage2 = self.stage2.wrapping_add(1);
        if self.stage2 != self.target {
            return;
        }
        self.stage2 = 0;
        self.stage3 = (self.stage3 + 1) & 0x0F;
    }

    /// Read the 4-bit output and clear it ( T0OUT/T1OUT/T2OUT semantics).
    fn read_out(&mut self) -> u8 {
        let v = self.stage3;
        self.stage3 = 0;
        v
    }

    fn save_state(&self, s: &mut SaveWriter) {
        s.write_u16(self.divisor);
        s.write_u16(self.stage0);
        s.write_bool(self.stage1);
        s.write_u8(self.stage2);
        s.write_u8(self.stage3);
        s.write_bool(self.line);
        s.write_bool(self.enable);
        s.write_u8(self.target);
    }

    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.divisor = s.read_u16()?;
        self.stage0 = s.read_u16()?;
        self.stage1 = s.read_bool()?;
        self.stage2 = s.read_u8()?;
        // stage3 is the 4-bit visible counter (sync_stage1 already masks it & 0x0F on every
        // normal increment); mask on load too rather than trust it verbatim.
        self.stage3 = s.read_u8()? & 0x0F;
        self.line = s.read_bool()?;
        self.enable = s.read_bool()?;
        self.target = s.read_u8()?;
        Ok(())
    }
}

/// Memory-mapped register / control state at `$00F0-$00FF` (ares `SMP::IO`).
#[derive(Debug, Clone, Copy)]
struct Io {
    // $00F0 TEST
    timers_disable: bool,
    ram_writable: bool,
    ram_disable: bool,
    timers_enable: bool,
    external_wait: u8,
    internal_wait: u8,
    // $00F1 CONTROL
    iplrom_enable: bool,
    // $00F2 DSPADDR
    dsp_address: u8,
    // $00F4-F7: CPU→SMP latches (read by the SMP)
    cpu: [u8; 4],
    // $00F4-F7: SMP→CPU latches (written by the SMP, read by the CPU)
    apu: [u8; 4],
    // $00F8-F9 AUX
    aux: [u8; 2],
}

impl Default for Io {
    fn default() -> Self {
        Self {
            timers_disable: false,
            ram_writable: true,
            ram_disable: false,
            timers_enable: true,
            external_wait: 0,
            internal_wait: 0,
            iplrom_enable: true,
            dsp_address: 0,
            cpu: [0; 4],
            apu: [0; 4],
            aux: [0; 2],
        }
    }
}

impl Io {
    fn save_state(&self, s: &mut SaveWriter) {
        s.write_bool(self.timers_disable);
        s.write_bool(self.ram_writable);
        s.write_bool(self.ram_disable);
        s.write_bool(self.timers_enable);
        s.write_u8(self.external_wait);
        s.write_u8(self.internal_wait);
        s.write_bool(self.iplrom_enable);
        s.write_u8(self.dsp_address);
        s.write_bytes(&self.cpu);
        s.write_bytes(&self.apu);
        s.write_bytes(&self.aux);
    }

    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.timers_disable = s.read_bool()?;
        self.ram_writable = s.read_bool()?;
        self.ram_disable = s.read_bool()?;
        self.timers_enable = s.read_bool()?;
        self.external_wait = s.read_u8()?;
        self.internal_wait = s.read_u8()?;
        self.iplrom_enable = s.read_bool()?;
        self.dsp_address = s.read_u8()?;
        self.cpu.copy_from_slice(s.read_bytes(4)?);
        self.apu.copy_from_slice(s.read_bytes(4)?);
        self.aux.copy_from_slice(s.read_bytes(2)?);
        Ok(())
    }
}

/// The complete SNES audio unit: SPC700 + S-DSP + 64 KiB ARAM + timers + ports.
///
/// `rustysnes-core` wires the four CPU-side ports through [`Apu::cpu_read_port`] /
/// [`Apu::cpu_write_port`] (the resync boundary) and pulls audio with [`Apu::sample`]. The SMP is
/// advanced with [`Apu::run_cycles`] — the unit of advancement is **one SPC700 master tick**
/// (one DSP-clock/12 step); the scheduler decides when to call it.
pub struct Apu {
    cpu: Spc700,
    dsp: Dsp,
    aram: Box<[u8; ARAM_SIZE]>,
    io: Io,
    timers: [Timer; 3],
    iplrom: [u8; 64],
    /// SMP-clock units accumulated toward the next DSP 32 kHz output sample (768 per sample).
    dsp_counter: u32,
    /// SPC clock cycles credited by [`Apu::tick`] but not yet consumed by an instruction. The
    /// core hands one SPC cycle per `tick`; we run the next instruction once enough have built up.
    cycle_budget: u32,
    /// Cycle cost of the next instruction (1 until the first instruction runs); paces `tick`.
    next_instruction_cost: u32,
    /// The recorded micro-op timeline of the instruction currently being drained one base clock at
    /// a time by [`Apu::advance_smp_cycle`]. Empty when no instruction is in flight; refilled by
    /// recording the next instruction the moment the previous one fully drains.
    plan: Vec<CycleStep>,
    /// Index into [`Self::plan`] of the next micro-op to drain.
    plan_pos: usize,
    /// Base clocks already drained from `plan[plan_pos]` (a single micro-op may span several base
    /// clocks — a normal access is [`SMP_WAIT`] — and we release exactly one per cycle pump).
    plan_sub: u32,
}

impl core::fmt::Debug for Apu {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Apu")
            .field("cpu", &self.cpu)
            .field("dsp", &self.dsp)
            .field("dsp_counter", &self.dsp_counter)
            .finish_non_exhaustive()
    }
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    /// Construct at power-on. The SPC700 boots from the IPL reset vector with the IPL ROM mapped.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Spc700::new();
        // Boot from the IPL reset vector ($FFC0 reads IPL[62],[63]).
        cpu.regs.pc = u16::from(IPL_ROM[62]) | (u16::from(IPL_ROM[63]) << 8);
        Self {
            cpu,
            dsp: Dsp::new(),
            aram: Box::new([0; ARAM_SIZE]),
            io: Io::default(),
            timers: [Timer::new(128), Timer::new(128), Timer::new(16)],
            iplrom: IPL_ROM,
            dsp_counter: 0,
            cycle_budget: 0,
            next_instruction_cost: 1,
            plan: Vec::new(),
            plan_pos: 0,
            plan_sub: 0,
        }
    }

    /// Advance the SPC700/S-DSP by one of ITS clock cycles — the scheduler's per-SPC-cycle hook.
    ///
    /// The core's lockstep scheduler calls this each time its fractional SPC accumulator crosses
    /// the divisor (so once per ~1.024 MHz cycle). Because the SPC700 model is instruction-grained,
    /// each call credits one cycle to an internal budget and runs the next instruction once the
    /// budget covers it; the DSP is caught up from the cycles consumed. The four-port handshake is
    /// surfaced through `bus` only at port-access boundaries (the resync seam in `rustysnes-core`).
    pub fn tick(&mut self, bus: &mut impl AudioBus) {
        // Mirror the CPU→SMP latches the core mediates into our internal latches before stepping.
        for n in 0..4u8 {
            self.io.cpu[n as usize] = bus.read_port(n);
        }
        self.cycle_budget += 1;
        if self.cpu.stopped {
            self.cycle_budget = 0;
            return;
        }
        if self.cycle_budget >= self.next_instruction_cost {
            self.cycle_budget = 0;
            let consumed = self.step_instruction();
            self.next_instruction_cost = consumed.max(1);
        }
        // Surface any SMP→CPU port writes back to the core's latches.
        for n in 0..4u8 {
            bus.write_port(n, self.io.apu[n as usize]);
        }
    }

    /// Release **exactly one** SMP base clock from the core's lockstep accumulator — the
    /// cycle-exact async-resync pump the core's `Bus` calls once per crossing of the SPC divisor in
    /// `Bus::advance_master`.
    ///
    /// This is true sub-instruction lockstep, not instruction-grained catch-up. The SMP instruction
    /// in flight is decomposed into a recorded micro-op timeline (one `base_clocks` + optional
    /// deferred port write per bus access); each call drains one base clock of it, and an SMP→CPU
    /// port write becomes visible to the main CPU at the
    /// **precise base cycle** it lands on — so when the CPU reads `$2140-$2143` at a given master
    /// instant, the SMP has executed exactly the cycles up to that instant and no further (matching
    /// the ares/bsnes cooperative-thread interleaving, achieved here without coroutines). The DSP /
    /// timers are advanced as part of recording the instruction; only the CPU-observable port latch
    /// needs the per-cycle precision, because it is the sole state shared across the two clock
    /// domains. Integer-only and order-deterministic (`docs/adr/0004`).
    pub fn advance_smp_cycle(&mut self) {
        if self.cpu.stopped {
            // A stopped/sleeping SMP makes no architectural progress and writes no ports; drop any
            // in-flight plan and idle. (Reset is the only exit, handled by the core.)
            self.plan.clear();
            self.plan_pos = 0;
            self.plan_sub = 0;
            return;
        }
        if self.plan_pos >= self.plan.len() {
            // The previous instruction fully drained: record the next one's micro-op timeline.
            self.record_next_instruction();
        }
        self.drain_one_base_clock();
    }

    /// Drain one SMP base clock from the in-flight instruction plan, committing a deferred port
    /// write the moment the micro-op it belongs to completes.
    fn drain_one_base_clock(&mut self) {
        let Some(step) = self.plan.get(self.plan_pos).copied() else {
            return;
        };
        self.plan_sub += 1;
        if self.plan_sub < step.base_clocks {
            return; // micro-op still consuming its wait-state base clocks
        }
        // Commit the deferred SMP→CPU port write as the micro-op completes: the CPU observes the
        // SMP's new latch value exactly at the master instant the SMP write access finishes, which
        // is the cycle-exact boundary a CPU read of $2140-$2143 must see (ares/bsnes interleave).
        if let Some((n, v)) = step.port_write {
            self.io.apu[(n & 3) as usize] = v;
        }
        self.plan_sub = 0;
        self.plan_pos += 1;
    }

    /// Run the next SMP instruction through a recording bus that applies every side effect exactly
    /// as [`SmpBus`] would (so the architectural result is bit-identical to [`Apu::step_instruction`]
    /// and the SPC700 oracle), while capturing the ordered micro-op timeline + per-cycle catch-up of
    /// the DSP into [`Self::plan`] for cycle-exact draining.
    fn record_next_instruction(&mut self) {
        self.plan.clear();
        self.plan_pos = 0;
        self.plan_sub = 0;
        let mut consumed = 0u32;
        let mut bus = RecordingSmpBus {
            aram: &mut self.aram,
            dsp: &mut self.dsp,
            io: &mut self.io,
            timers: &mut self.timers,
            iplrom: &self.iplrom,
            consumed: &mut consumed,
            dsp_counter: &mut self.dsp_counter,
            plan: &mut self.plan,
        };
        self.cpu.step(&mut bus);
        if self.plan.is_empty() {
            // Defensive: an instruction must consume at least one cycle. (Every SPC700 opcode does;
            // this keeps `advance_smp_cycle` from spinning if a future change ever broke that.)
            self.plan.push(CycleStep {
                base_clocks: SMP_WAIT,
                port_write: None,
            });
        }
    }

    /// Advance the SMP by one instruction, then catch the DSP up. The DSP emits one 32 kHz sample
    /// every 768 SMP-clock units; this drives that boundary from the SMP cycle count consumed.
    ///
    /// Returns the number of SMP-clock units the instruction consumed.
    pub fn step_instruction(&mut self) -> u32 {
        let mut bus = SmpBus {
            aram: &mut self.aram,
            dsp: &mut self.dsp,
            io: &mut self.io,
            timers: &mut self.timers,
            iplrom: &self.iplrom,
            cycles: 0,
        };
        self.cpu.step(&mut bus);
        let consumed = bus.cycles;
        // Catch the DSP up, one micro-tick per `DSP_BASE_CLOCKS_PER_TICK` (= 2) base clocks. The
        // unit is the SMP base clock (apuFrequency / 12 ≈ 2.0506 MHz, ares
        // `SMP::create(apuFrequency()/12, …)`); 32 ticks = one 32 kHz stereo sample (64 base
        // clocks). Ticking sub-sample (not a whole 64-clock sample at once) is what gives a DSP
        // register read its cycle-correct value mid-instruction.
        self.dsp_counter += consumed;
        while self.dsp_counter >= DSP_BASE_CLOCKS_PER_TICK {
            self.dsp_counter -= DSP_BASE_CLOCKS_PER_TICK;
            self.dsp.tick(&mut self.aram);
        }
        consumed
    }

    /// Run the SMP until at least `clocks` SMP-clock units have elapsed (a coarse driver for the
    /// scheduler; instruction granularity, so it may slightly overshoot).
    pub fn run_cycles(&mut self, clocks: u32) {
        let mut elapsed = 0;
        while elapsed < clocks {
            if self.cpu.stopped {
                break;
            }
            elapsed += self.step_instruction();
        }
    }

    /// CPU-side read of port `n` (0..=3) at `$2140+n`: returns what the *SMP* last wrote. This is
    /// the resync boundary — the core syncs the SMP up to "now" *before* calling this.
    #[must_use]
    pub fn cpu_read_port(&self, n: u8) -> u8 {
        self.io.apu[(n & 3) as usize]
    }

    /// CPU-side write of port `n` (0..=3) at `$2140+n`: deposits into the CPU→SMP latch.
    pub fn cpu_write_port(&mut self, n: u8, val: u8) {
        self.io.cpu[(n & 3) as usize] = val;
    }

    /// The most-recent 32 kHz stereo sample (left, right), 16-bit signed.
    #[must_use]
    pub fn sample(&self) -> (i16, i16) {
        self.dsp.last_sample()
    }

    /// Drain every 32 kHz stereo sample the S-DSP has emitted since the last drain into `sink`
    /// (in emission order). The frontend calls this once per frame to feed its audio ring; the
    /// FIFO is additive instrumentation over the DAC output and never perturbs synthesis.
    pub fn drain_audio(&mut self, sink: &mut Vec<(i16, i16)>) {
        self.dsp.drain_audio(sink);
    }

    /// Read a DSP register through `$00F2`/`$00F3` semantics (testing / debug).
    #[must_use]
    pub fn dsp_read(&self, address: u8) -> u8 {
        self.dsp.read(address)
    }

    /// Borrow ARAM (read-only) — for save-states / debug.
    #[must_use]
    pub fn aram(&self) -> &[u8; ARAM_SIZE] {
        &self.aram
    }

    /// The SPC700 program counter (debug / test diagnostics).
    #[must_use]
    pub fn smp_pc(&self) -> u16 {
        self.cpu.regs.pc
    }

    /// Whether the SPC700 has executed `STP`/`SLEEP` and halted (debug / test diagnostics).
    #[must_use]
    pub fn smp_stopped(&self) -> bool {
        self.cpu.stopped
    }

    /// Write the SPC700 core, the S-DSP, ARAM, the `$00F0-$00FF` register file, the three
    /// timers, the DSP output-sample counter, and the in-flight instruction micro-op plan
    /// (`plan`/`plan_pos`/`plan_sub`) into an `"APU0"` section. `iplrom` is NOT written: it is
    /// the fixed public-domain 64-byte boot ROM ([`IPL_ROM`]), identical on every SNES and never
    /// user-supplied, so a freshly-constructed `Apu` already carries the exact same bytes
    /// (`docs/adr/0003`'s "never embed a chip-ROM/firmware byte" posture applied to a constant
    /// that needs no embedding in the first place).
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"APU0", |s| {
            self.cpu.save_state(s);
            self.dsp.save_state(s);
            s.write_bytes(&*self.aram);
            self.io.save_state(s);
            for t in &self.timers {
                t.save_state(s);
            }
            s.write_u32(self.dsp_counter);
            s.write_u32(self.cycle_budget);
            s.write_u32(self.next_instruction_cost);
            #[allow(clippy::cast_possible_truncation)] // bounded by MAX_SAVED_PLAN_LEN
            s.write_u32(self.plan.len() as u32);
            for step in &self.plan {
                s.write_u32(step.base_clocks);
                match step.port_write {
                    Some((port, val)) => {
                        s.write_bool(true);
                        s.write_u8(port);
                        s.write_u8(val);
                    }
                    None => s.write_bool(false),
                }
            }
            #[allow(clippy::cast_possible_truncation)] // <= plan.len(), same bound
            s.write_u32(self.plan_pos as u32);
            s.write_u32(self.plan_sub);
        });
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input, a section with unconsumed trailing bytes,
    /// or [`SaveStateError::Invalid`] if the saved instruction plan's claimed length exceeds
    /// `MAX_SAVED_PLAN_LEN` or `plan_pos` exceeds the restored plan's length (mirroring the
    /// GSU's `pending_clocks`/`pending_idx` validation in `rustysnes-cart` — an in-flight
    /// instruction has at most a handful of micro-ops, so a larger claimed length could never
    /// have come from real execution).
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"APU0")?;
        self.cpu.load_state(&mut s)?;
        self.dsp.load_state(&mut s)?;
        self.aram.copy_from_slice(s.read_bytes(ARAM_SIZE)?);
        self.io.load_state(&mut s)?;
        for t in &mut self.timers {
            t.load_state(&mut s)?;
        }
        self.dsp_counter = s.read_u32()?;
        self.cycle_budget = s.read_u32()?;
        self.next_instruction_cost = s.read_u32()?;
        let plan_len = s.read_u32()? as usize;
        if plan_len > MAX_SAVED_PLAN_LEN {
            return Err(SaveStateError::Invalid(alloc::format!(
                "APU instruction plan length {plan_len} exceeds the sane bound of {MAX_SAVED_PLAN_LEN}"
            )));
        }
        self.plan.clear();
        for _ in 0..plan_len {
            let base_clocks = s.read_u32()?;
            let port_write = if s.read_bool()? {
                let port = s.read_u8()?;
                let val = s.read_u8()?;
                Some((port, val))
            } else {
                None
            };
            self.plan.push(CycleStep {
                base_clocks,
                port_write,
            });
        }
        let plan_pos = s.read_u32()? as usize;
        if plan_pos > self.plan.len() {
            return Err(SaveStateError::Invalid(alloc::format!(
                "APU plan_pos {plan_pos} exceeds the restored plan length {}",
                self.plan.len()
            )));
        }
        self.plan_pos = plan_pos;
        self.plan_sub = s.read_u32()?;
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "APU0 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        Ok(())
    }
}

/// Bound on the saved in-flight instruction plan's length: [`Apu::step_instruction`] records at
/// most a handful of micro-ops per instruction, so a claimed length beyond this is corrupt/hostile
/// input, not a value real execution could ever produce (mirrors the GSU's
/// `MAX_SAVED_PENDING_CLOCKS` in `rustysnes-cart`).
const MAX_SAVED_PLAN_LEN: usize = 64;

/// The concrete [`Spc700Bus`] the full APU presents to its SPC700: ARAM + IPL ROM + the
/// memory-mapped registers (`$00F0-$00FF`) + timers + DSP. Cycle counting (`wait`-state model
/// simplified to 1 unit/access at the nominal internal rate) is folded into `cycles`.
struct SmpBus<'a> {
    aram: &'a mut [u8; ARAM_SIZE],
    dsp: &'a mut Dsp,
    io: &'a mut Io,
    timers: &'a mut [Timer; 3],
    iplrom: &'a [u8; 64],
    cycles: u32,
}

impl SmpBus<'_> {
    /// Advance the SMP base clock + the timers by `clocks` base ticks (ares `SMP::step` +
    /// `stepTimers`). One normal bus access is [`SMP_WAIT`] (= ares `cycleWaitStates[0]`) ticks.
    fn step(&mut self, clocks: u32) {
        self.cycles += clocks;
        let te = self.io.timers_enable;
        let td = self.io.timers_disable;
        // Timers advance on the same SMP base timebase as the CPU (ares `timerWaitStates[0]` = 2).
        for t in self.timers.iter_mut() {
            t.step(clocks as u16, te, td);
        }
    }

    fn read_io(&mut self, address: u16) -> Option<u8> {
        if address & 0xFFF0 != 0x00F0 {
            return None;
        }
        Some(match address & 0x0F {
            0x02 => self.io.dsp_address,
            0x03 => self.dsp.read(self.io.dsp_address),
            0x04 => self.io.cpu[0],
            0x05 => self.io.cpu[1],
            0x06 => self.io.cpu[2],
            0x07 => self.io.cpu[3],
            0x08 => self.io.aux[0],
            0x09 => self.io.aux[1],
            0x0D => self.timers[0].read_out(),
            0x0E => self.timers[1].read_out(),
            0x0F => self.timers[2].read_out(),
            // $F0/$F1 TEST/CONTROL and write-only targets read 0.
            _ => 0x00,
        })
    }

    fn write_io(&mut self, address: u16, data: u8) {
        if address & 0xFFF0 != 0x00F0 {
            return;
        }
        match address & 0x0F {
            0x00 => {
                self.io.timers_disable = data & 0x01 != 0;
                self.io.ram_writable = data & 0x02 != 0;
                self.io.ram_disable = data & 0x04 != 0;
                self.io.timers_enable = data & 0x08 != 0;
                self.io.external_wait = (data >> 4) & 0x03;
                self.io.internal_wait = (data >> 6) & 0x03;
                let (te, td) = (self.io.timers_enable, self.io.timers_disable);
                for t in self.timers.iter_mut() {
                    t.sync_stage1(te, td);
                }
            }
            0x01 => {
                for (i, t) in self.timers.iter_mut().enumerate() {
                    let raised = !t.enable && (data & (1 << i) != 0);
                    t.enable = data & (1 << i) != 0;
                    if raised {
                        t.stage2 = 0;
                        t.stage3 = 0;
                    }
                }
                if data & 0x10 != 0 {
                    self.io.cpu[0] = 0;
                    self.io.cpu[1] = 0;
                }
                if data & 0x20 != 0 {
                    self.io.cpu[2] = 0;
                    self.io.cpu[3] = 0;
                }
                self.io.iplrom_enable = data & 0x80 != 0;
            }
            0x02 => self.io.dsp_address = data,
            0x03 => self.dsp.write(self.io.dsp_address, data),
            0x04 => self.io.apu[0] = data,
            0x05 => self.io.apu[1] = data,
            0x06 => self.io.apu[2] = data,
            0x07 => self.io.apu[3] = data,
            0x08 => self.io.aux[0] = data,
            0x09 => self.io.aux[1] = data,
            _ => {} // timer targets handled below
        }
        match address {
            0xFA => self.timers[0].target = data,
            0xFB => self.timers[1].target = data,
            0xFC => self.timers[2].target = data,
            _ => {}
        }
    }

    fn read_ram(&self, address: u16) -> u8 {
        if address >= 0xFFC0 && self.io.iplrom_enable {
            return self.iplrom[(address & 0x3F) as usize];
        }
        if self.io.ram_disable {
            return 0x5A;
        }
        self.aram[address as usize]
    }
}

impl Spc700Bus for SmpBus<'_> {
    fn read(&mut self, address: u16) -> u8 {
        // ares `SMP::read`: a read of $F4-$F7 (the CPU↔APU ports) splits its wait into two halved
        // steps around the data fetch (`wait(1)` twice). At the reset wait state the total is the
        // same SMP_WAIT base clocks as any other access, but the split is preserved for fidelity.
        if address & 0xFFFC == 0x00F4 {
            self.step(SMP_WAIT >> 1);
            let v = self
                .read_io(address)
                .unwrap_or_else(|| self.read_ram(address));
            self.step(SMP_WAIT >> 1);
            return v;
        }
        self.step(SMP_WAIT);
        if let Some(io) = self.read_io(address) {
            return io;
        }
        self.read_ram(address)
    }

    fn write(&mut self, address: u16, data: u8) {
        self.step(SMP_WAIT);
        // Writes to $FFC0-$FFFF always reach ARAM even with the IPL ROM mapped in.
        if self.io.ram_writable && !self.io.ram_disable {
            self.aram[address as usize] = data;
        }
        self.write_io(address, data);
    }

    fn idle(&mut self) {
        self.step(SMP_WAIT);
    }
}

/// A recording variant of [`SmpBus`]: applies every read/write/idle side effect identically (ARAM,
/// DSP, timers, IO registers) **and** records the per-micro-op base-clock timeline + deferred
/// SMP→CPU port writes into a [`CycleStep`] plan, plus catches the S-DSP up from the cumulative base
/// clocks. The recorded instruction's architectural effect is byte-for-byte the same as
/// [`SmpBus`]; the plan is what lets the core drain the instruction one base clock at a time for
/// cycle-exact CPU↔SMP lockstep.
struct RecordingSmpBus<'a> {
    aram: &'a mut [u8; ARAM_SIZE],
    dsp: &'a mut Dsp,
    io: &'a mut Io,
    timers: &'a mut [Timer; 3],
    iplrom: &'a [u8; 64],
    /// SMP base clocks this instruction has consumed so far (diagnostic / boundary accounting).
    consumed: &'a mut u32,
    /// The persistent DSP sample accumulator (caught up inline so a mid-instruction DSP read sees
    /// the same value path the streaming DSP produces).
    dsp_counter: &'a mut u32,
    plan: &'a mut Vec<CycleStep>,
}

impl RecordingSmpBus<'_> {
    /// Advance the timers by `clocks` base ticks, **cycle-step the S-DSP one micro-tick per 2 base
    /// clocks**, and push a micro-op of that length onto the plan. Mirrors [`SmpBus::step`] for the
    /// timers and [`Apu::step_instruction`] for the DSP.
    ///
    /// Ticking the S-DSP here (sub-sample, as the access's base clocks elapse) — rather than a whole
    /// 64-clock sample at the instruction boundary — is what gives an instruction that reads a DSP
    /// register (`$F3`) mid-execution the **cycle-correct** value: the DSP has advanced exactly the
    /// ticks up to that base clock and no further. blargg's `spc_dsp6` / `spc_mem_access_times` use
    /// the DSP as a sub-cycle reference, so this granularity is required for them to resolve.
    fn record(&mut self, clocks: u32) {
        let te = self.io.timers_enable;
        let td = self.io.timers_disable;
        for t in self.timers.iter_mut() {
            t.step(clocks as u16, te, td);
        }
        *self.consumed += clocks;
        *self.dsp_counter += clocks;
        while *self.dsp_counter >= DSP_BASE_CLOCKS_PER_TICK {
            *self.dsp_counter -= DSP_BASE_CLOCKS_PER_TICK;
            self.dsp.tick(self.aram);
        }
        self.plan.push(CycleStep {
            base_clocks: clocks,
            port_write: None,
        });
    }

    /// Attach a deferred SMP→CPU port-latch write to the most recently recorded micro-op, so it
    /// commits when that micro-op's base clocks elapse (the precise cycle the main CPU can observe).
    fn defer_port_on_last(&mut self, port_write: (u8, u8)) {
        if let Some(last) = self.plan.last_mut() {
            last.port_write = Some(port_write);
        }
    }

    fn read_io(&mut self, address: u16) -> Option<u8> {
        if address & 0xFFF0 != 0x00F0 {
            return None;
        }
        Some(match address & 0x0F {
            0x02 => self.io.dsp_address,
            0x03 => self.dsp.read(self.io.dsp_address),
            0x04 => self.io.cpu[0],
            0x05 => self.io.cpu[1],
            0x06 => self.io.cpu[2],
            0x07 => self.io.cpu[3],
            0x08 => self.io.aux[0],
            0x09 => self.io.aux[1],
            0x0D => self.timers[0].read_out(),
            0x0E => self.timers[1].read_out(),
            0x0F => self.timers[2].read_out(),
            _ => 0x00,
        })
    }

    /// Decode an IO write, returning the deferred SMP→CPU port latch update (if any) so it can be
    /// committed at the precise base cycle the write completes on. All other IO effects (timers,
    /// DSP, control) apply immediately, exactly as [`SmpBus::write_io`].
    fn write_io(&mut self, address: u16, data: u8) -> Option<(u8, u8)> {
        if address & 0xFFF0 != 0x00F0 {
            return None;
        }
        let mut deferred_port = None;
        match address & 0x0F {
            0x00 => {
                self.io.timers_disable = data & 0x01 != 0;
                self.io.ram_writable = data & 0x02 != 0;
                self.io.ram_disable = data & 0x04 != 0;
                self.io.timers_enable = data & 0x08 != 0;
                self.io.external_wait = (data >> 4) & 0x03;
                self.io.internal_wait = (data >> 6) & 0x03;
                let (te, td) = (self.io.timers_enable, self.io.timers_disable);
                for t in self.timers.iter_mut() {
                    t.sync_stage1(te, td);
                }
            }
            0x01 => {
                for (i, t) in self.timers.iter_mut().enumerate() {
                    let raised = !t.enable && (data & (1 << i) != 0);
                    t.enable = data & (1 << i) != 0;
                    if raised {
                        t.stage2 = 0;
                        t.stage3 = 0;
                    }
                }
                if data & 0x10 != 0 {
                    self.io.cpu[0] = 0;
                    self.io.cpu[1] = 0;
                }
                if data & 0x20 != 0 {
                    self.io.cpu[2] = 0;
                    self.io.cpu[3] = 0;
                }
                self.io.iplrom_enable = data & 0x80 != 0;
            }
            0x02 => self.io.dsp_address = data,
            0x03 => self.dsp.write(self.io.dsp_address, data),
            // The four SMP→CPU port latches: the value the main CPU reads at $2140-$2143. This is
            // the only effect deferred to the exact base cycle (the rest of the SMP is private).
            0x04 => deferred_port = Some((0, data)),
            0x05 => deferred_port = Some((1, data)),
            0x06 => deferred_port = Some((2, data)),
            0x07 => deferred_port = Some((3, data)),
            0x08 => self.io.aux[0] = data,
            0x09 => self.io.aux[1] = data,
            _ => {}
        }
        match address {
            0xFA => self.timers[0].target = data,
            0xFB => self.timers[1].target = data,
            0xFC => self.timers[2].target = data,
            _ => {}
        }
        deferred_port
    }

    fn read_ram(&self, address: u16) -> u8 {
        if address >= 0xFFC0 && self.io.iplrom_enable {
            return self.iplrom[(address & 0x3F) as usize];
        }
        if self.io.ram_disable {
            return 0x5A;
        }
        self.aram[address as usize]
    }
}

impl Spc700Bus for RecordingSmpBus<'_> {
    fn read(&mut self, address: u16) -> u8 {
        if address & 0xFFFC == 0x00F4 {
            // $F4-$F7 read: ares splits the wait into two halved steps around the fetch.
            self.record(SMP_WAIT >> 1);
            let v = self
                .read_io(address)
                .unwrap_or_else(|| self.read_ram(address));
            self.record(SMP_WAIT >> 1);
            return v;
        }
        self.record(SMP_WAIT);
        if let Some(io) = self.read_io(address) {
            return io;
        }
        self.read_ram(address)
    }

    fn write(&mut self, address: u16, data: u8) {
        // ares/Mesen2 ordering: the access advances the SMP timebase **and clocks the timers BEFORE
        // the write side effect lands** (Mesen2 `Spc::Write` calls `IncCycleCount` first, then
        // applies the store; ares `step()` precedes the store the same way). The
        // timer-target/enable/global-enable write therefore observes this cycle's timer clock as
        // already-happened — the one-access phase the blargg `spc_timer` / `spc_smp` /
        // `spc_mem_access_times` suites pin. (Matches [`SmpBus::write`], which already steps first;
        // the recording bus previously stored first, shifting the timer phase by one access.)
        self.record(SMP_WAIT);
        // Writes to $FFC0-$FFFF always reach ARAM even with the IPL ROM mapped in.
        if self.io.ram_writable && !self.io.ram_disable {
            self.aram[address as usize] = data;
        }
        // Carry the deferred SMP→CPU port latch onto this access's micro-op, so it commits at the
        // precise base cycle the write completes (the cycle-exact CPU↔SMP handshake boundary).
        if let Some(pw) = self.write_io(address, data) {
            self.defer_port_on_last(pw);
        }
    }

    fn idle(&mut self) {
        self.record(SMP_WAIT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spc700::{Spc700, Spc700Bus};

    /// A flat 64 KiB bus for exercising the SPC700 core directly in unit tests.
    struct FlatBus {
        mem: alloc::boxed::Box<[u8; 0x1_0000]>,
        cycles: u32,
    }
    impl FlatBus {
        fn new() -> Self {
            Self {
                mem: alloc::boxed::Box::new([0; 0x1_0000]),
                cycles: 0,
            }
        }
    }
    impl Spc700Bus for FlatBus {
        fn read(&mut self, a: u16) -> u8 {
            self.cycles += 1;
            self.mem[a as usize]
        }
        fn write(&mut self, a: u16, d: u8) {
            self.cycles += 1;
            self.mem[a as usize] = d;
        }
        fn idle(&mut self) {
            self.cycles += 1;
        }
    }

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

    #[test]
    fn power_on_state_matches_hardware() {
        let cpu = Spc700::new();
        assert_eq!(cpu.regs.sp, 0xEF);
        assert_eq!(cpu.regs.psw.bits(), 0x02);
    }

    #[test]
    fn nop_is_two_cycles() {
        let mut cpu = Spc700::new();
        let mut bus = FlatBus::new();
        cpu.regs.pc = 0x0100;
        bus.mem[0x0100] = 0x00; // NOP
        cpu.step(&mut bus);
        assert_eq!(bus.cycles, 2);
        assert_eq!(cpu.regs.pc, 0x0101);
    }

    #[test]
    fn mov_a_immediate_sets_nz() {
        let mut cpu = Spc700::new();
        let mut bus = FlatBus::new();
        cpu.regs.pc = 0x0200;
        bus.mem[0x0200] = 0xE8; // MOV A,#imm
        bus.mem[0x0201] = 0x00;
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.a, 0x00);
        assert!(cpu.regs.psw.z());
        assert!(!cpu.regs.psw.n());
    }

    #[test]
    fn mul_ya() {
        let mut cpu = Spc700::new();
        let mut bus = FlatBus::new();
        cpu.regs.pc = 0x0300;
        cpu.regs.y = 0x10;
        cpu.regs.a = 0x10;
        bus.mem[0x0300] = 0xCF; // MUL YA
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.ya(), 0x0100);
        assert_eq!(bus.cycles, 9); // fetch + read(PC) + 7 idle
    }

    #[test]
    fn ports_are_one_way_latches() {
        // CPU writes a port; the SMP reads what the CPU wrote (not an echo of an SMP write).
        let mut apu = Apu::new();
        apu.cpu_write_port(0, 0xAB);
        // The CPU-side read returns the SMP's last write (default 0), not 0xAB.
        assert_eq!(apu.cpu_read_port(0), 0x00);
        // The CPU→SMP value is now in io.cpu[0] for the SMP to read.
        assert_eq!(apu.io.cpu[0], 0xAB);
    }

    #[test]
    fn timer_divides_and_wraps_4bit() {
        let mut t = Timer::new(16);
        t.enable = true;
        t.target = 1;
        // Each 16 clocks toggles stage1; a 1→0 edge bumps stage2; stage2==target bumps stage3.
        for _ in 0..200 {
            t.step(4, true, false);
        }
        assert!(t.read_out() > 0);
        assert_eq!(t.read_out(), 0); // read clears
    }

    #[test]
    fn ipl_rom_maps_at_reset() {
        let apu = Apu::new();
        // Boot PC must be the IPL reset vector.
        assert_eq!(apu.cpu.regs.pc, 0xFFC0);
    }

    #[test]
    fn run_cycles_advances_and_produces_samples() {
        let mut apu = Apu::new();
        apu.run_cycles(2000);
        // DSP produced at least one sample frame (still silence pre-program, but the path ran).
        let _ = apu.sample();
    }

    #[test]
    fn full_state_round_trips_through_save_state() {
        let mut apu = Apu::new();
        apu.run_cycles(2000);
        apu.cpu.regs.a = 0x42;
        apu.aram[0x100] = 0x7A;
        apu.dsp.write(0x00, 0x11); // voice 0 volume-left register
        let pc_before = apu.cpu.regs.pc;

        let mut w = SaveWriter::new();
        apu.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = Apu::new();
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.cpu.regs.a, 0x42);
        assert_eq!(fresh.cpu.regs.pc, pc_before);
        assert_eq!(fresh.aram[0x100], 0x7A);
        assert_eq!(fresh.dsp.read(0x00), 0x11);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn oversized_instruction_plan_length_is_rejected_not_trusted() {
        let apu = Apu::new();
        let mut w = SaveWriter::new();
        apu.save_state(&mut w);
        let mut bytes = w.into_bytes();

        // Locate the plan-length u32 by replaying load_state's own field order up to it, mirroring
        // the GSU pending_clocks test in rustysnes-cart's gsu.rs.
        let mut r = SaveReader::new(&bytes);
        let mut s = r.expect_section(*b"APU0").unwrap();
        let mut probe = Spc700::new();
        probe.load_state(&mut s).unwrap();
        let mut dsp_probe = Dsp::new();
        dsp_probe.load_state(&mut s).unwrap();
        s.read_bytes(ARAM_SIZE).unwrap();
        let mut io_probe = Io::default();
        io_probe.load_state(&mut s).unwrap();
        for _ in 0..3 {
            let mut t = Timer::default();
            t.load_state(&mut s).unwrap();
        }
        s.read_u32().unwrap(); // dsp_counter
        s.read_u32().unwrap(); // cycle_budget
        s.read_u32().unwrap(); // next_instruction_cost
        let offset = bytes.len() - s.remaining();
        bytes[offset..offset + 4].copy_from_slice(&1000u32.to_le_bytes());

        let mut fresh = Apu::new();
        let mut r2 = SaveReader::new(&bytes);
        assert!(matches!(
            fresh.load_state(&mut r2),
            Err(SaveStateError::Invalid(_))
        ));
    }
}
