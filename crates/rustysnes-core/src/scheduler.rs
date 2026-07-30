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

use alloc::vec::Vec;

use rustysnes_cpu::{Cpu, Regs};
use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::bus::Bus;
use crate::sa1_bus::Sa1Bus;

/// The save-state format's major version (`docs/adr/0006-save-state-format.md`). Bump this any
/// time a section's on-disk layout changes in a way an older reader can't skip past; the reader
/// (this crate's [`System::load_state`]) rejects any `found > FORMAT_VERSION` rather than
/// silently misinterpreting a newer layout.
///
/// `2` (`v0.7.0 "Resolution"`): `rustysnes-ppu`'s `PPU0` section grew — the framebuffer's backing
/// storage is now always allocated at hi-res capacity (512×239, up from 256×239) to support true
/// hi-res (Modes 5/6) output, and a new `frame_hires` bool was added — a real byte-layout change
/// to an existing section (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision). Note this
/// bump only guards against loading a *newer*-than-supported blob (`load_state` rejects `found >
/// FORMAT_VERSION`); it does not add graceful old-format loading — a genuinely older blob loaded
/// by this code fails with a real parse/truncation error (proven by
/// `crates/rustysnes-test-harness/tests/save_state_backward_compat.rs`'s `tests/golden/
/// savestate-v1-gilyon.bin` fixture), not silent misinterpretation. See
/// `docs/adr/0006-save-state-format.md`'s bump log for the full record.
///
/// `3` (`v0.9.0`, Phase 7 niche peripherals): `crate::bus`'s `BUS0` section grew — a new WRIO
/// (`$4201`/`$4213`) `pio` byte plus each controller port's [`crate::controller::PortState`]
/// (device selection + Mouse/Super Scope/Super Multitap runtime state). Same guarantee as the `2`
/// bump above: a `FORMAT_VERSION < 3` blob fails loudly (a `BUS0` section-length mismatch), not
/// silently.
///
/// `5` (Tier-1 T-CA-01/03): `crate::bus`'s `BUS0` section grew again — the in-flight automatic
/// joypad read's start snapshot (`joypad_auto_pending`) and busy deadline (`auto_joypad_busy_until`),
/// so a save taken during the ~4224-clock auto-read window restores identical machine state. Same
/// old-blob-fails-loudly guarantee.
///
/// `6` (Tier-1 T-CA-02): `crate::bus`'s `BUS0` section grew by two bytes — the RDNMI/TIMEUP hold
/// flags (`rdnmi_hold`/`irq_hold`), so a save taken during the four-master-clock window after a
/// `VBlank`/IRQ edge (when a `$4210`/`$4211` read returns the flag without clearing it) restores
/// identical machine state. Same old-blob-fails-loudly guarantee.
///
/// `7` (T-CA-10 Phase 4b): `rustysnes_ppu`'s `PPU0` section grew by one byte — the OAM
/// sprite-evaluation seed (`pd_oam_eval_seed`), from which the in-render `$2104` redirect derives
/// the evaluator's OAM index. It is captured at line start and diverges from `OAMADDR` after
/// redirected active-display writes (with priority rotation), so it cannot be re-derived on load and
/// must persist for a mid-line save to restore identical machine state (mirrors MesenCE serializing
/// `_oamEvaluationIndex`). Same old-blob-fails-loudly guarantee.
///
/// `8` (T-CA-10 Phase 4b over-flag cursor): `PPU0` grew by one more byte — `pd_over_eval_seed`, the
/// line-start priority-rotation seed the sprite over-flag (STAT77) timing evaluates from. Like
/// `pd_oam_eval_seed` it is captured at line start and diverges from `OAMADDR` mid-line, so the
/// over-flag set-dots (re-derived on load) must key off the persisted seed rather than the live
/// address, or a mid-line save/load on a priority-rotated line would shift `$213E` timing. Same
/// old-blob-fails-loudly guarantee.
///
/// `9` (auto-read start timing): `rustysnes_core`'s `BUS0` section grew by eight bytes —
/// `auto_joypad_start_at`, the `clock.master` instant an armed automatic joypad read is scheduled to
/// begin. Hardware starts the read ~dot 32.5-95.5 into the first vblank line, not at the vblank edge,
/// so a save taken in that window must restore the pending start or the read never begins on load
/// (`$4212` bit 0 and `$4218-$421F` would desync). Same old-blob-fails-loudly guarantee.
///
/// `10` (NEC DSP pin-exact clock): the `NDSP` coprocessor section grew by eight bytes — `dsp_accum`,
/// the µPD77C25/µPD96050's master-clock fractional-accumulator phase. The DSP now free-runs on its
/// own 7.6/11 MHz divisor (stepped every master tick via `coprocessor_tick`) instead of catching up
/// synchronously on each host DR access, so its sub-master-clock position must persist or a
/// mid-computation save would restore the wrong RQM-handshake timing. Same old-blob-fails-loudly
/// guarantee (only carts carrying a NEC DSP — DSP-1/2/4/ST010 — have an `NDSP` section at all).
const FORMAT_VERSION: u16 = 10;
/// The save-state envelope's leading magic bytes — identifies the blob as a RustySNES save-state
/// before anything else is trusted.
const MAGIC: &[u8; 4] = b"RSNS";

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
    /// `PBR:PC` of the instruction `trace_step` recorded, carried to `trace_after_step` so the
    /// control-flow classifier can name where the transfer came *from* (`v1.25.0`, T-FP-C1).
    /// Debugger-only state; never in a save state.
    #[cfg(feature = "debug-hooks")]
    pending_trace_pc: u32,
    /// That instruction's opcode, likewise carried across the step.
    #[cfg(feature = "debug-hooks")]
    pending_trace_opcode: u8,
    /// `Cpu::interrupts_taken` as of `trace_step`. A step that vectored an NMI/IRQ never fetched an
    /// opcode at all, so the classifier must read this rather than `pending_trace_opcode` — which
    /// on such a step still holds the *previous* instruction's byte.
    #[cfg(feature = "debug-hooks")]
    pending_trace_interrupts: u64,
    /// Whether `trace_step` actually armed the fields above this step. Without it, arming tracing
    /// between the two hooks would classify against whatever the last traced step left behind.
    #[cfg(feature = "debug-hooks")]
    pending_trace_armed: bool,
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
            #[cfg(feature = "debug-hooks")]
            pending_trace_pc: 0,
            #[cfg(feature = "debug-hooks")]
            pending_trace_opcode: 0,
            #[cfg(feature = "debug-hooks")]
            pending_trace_interrupts: 0,
            #[cfg(feature = "debug-hooks")]
            pending_trace_armed: false,
        }
    }

    /// Reset the CPU from the cart's emulation reset vector (`$00FFFC`). Safe to call with no
    /// cart (the CPU reads open bus and parks); the boot flag tracks readiness. Auto-detects
    /// NTSC vs PAL from the installed cart's header (`Bus::sync_region_from_cart`) before
    /// resetting the CPU, so a PAL cart boots at the PAL line count/timing without the caller
    /// (frontend or test) having to know or guess the region up front.
    pub fn reset(&mut self) {
        self.bus.sync_region_from_cart();
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
            #[cfg(feature = "debug-hooks")]
            self.bus
                .set_debug_pc((u32::from(self.cpu.regs.pbr) << 16) | u32::from(self.cpu.regs.pc));
            #[cfg(feature = "debug-hooks")]
            self.trace_step();
            self.cpu.step(&mut self.bus);
            #[cfg(feature = "debug-hooks")]
            self.trace_after_step();
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

    /// The SA-1 second CPU's architectural register file, or `None` when no SA-1 cart is
    /// installed. For the debugger overlay's Cart panel (`docs/frontend.md` §Debugger overlay).
    #[must_use]
    pub fn sa1_regs(&self) -> Option<Regs> {
        self.sa1_cpu.as_ref().map(|c| c.regs)
    }

    /// The determinism seed this `System` was constructed with (`Self::new`). A TAS movie's
    /// power-on start point records this so a replay can verify the caller reconstructed the
    /// System with the exact same seed before calling [`crate::movie::Movie::seek_to_start`] —
    /// a different seed gives different power-on phase alignment, breaking bit-identical replay
    /// even against the same ROM and input log (`docs/adr/0004`).
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Step a single CPU instruction (drives the whole machine in lockstep via the Bus).
    pub fn step_instruction(&mut self) {
        if !self.booted {
            self.reset();
        }
        #[cfg(feature = "debug-hooks")]
        self.bus
            .set_debug_pc((u32::from(self.cpu.regs.pbr) << 16) | u32::from(self.cpu.regs.pc));
        #[cfg(feature = "debug-hooks")]
        self.trace_step();
        self.cpu.step(&mut self.bus);
        #[cfg(feature = "debug-hooks")]
        self.trace_after_step();
        if self.sa1_cpu.is_some() {
            self.run_sa1();
        }
    }

    /// Record the instruction about to execute into the trace ring (`v1.25.0`, T-FP-C1).
    ///
    /// Pre-execution, because a trace is read to answer "what was the machine holding when it
    /// decided to do that" — see [`crate::trace::TraceEntry`]. Costs one `bool` test when tracing
    /// is disarmed, which is every build that has not explicitly turned it on.
    #[cfg(feature = "debug-hooks")]
    fn trace_step(&mut self) {
        if !self.bus.trace().is_tracing() {
            self.pending_trace_armed = false;
            return;
        }
        let regs = self.cpu.regs;
        let pbr_pc = (u32::from(regs.pbr) << 16) | u32::from(regs.pc);
        // `Bus::peek` rather than a live read: a debugger fetch must not perturb the open-bus latch
        // or trip a watchpoint, the same rule the disassembler already follows.
        let opcode = self.bus.peek(pbr_pc);
        self.bus.trace_mut().record_step(crate::trace::TraceEntry {
            pbr_pc,
            opcode,
            a: regs.a,
            x: regs.x,
            y: regs.y,
            sp: regs.s,
            dp: regs.d,
            p: regs.p.bits(),
            db: regs.dbr,
            emulation: regs.emulation,
        });
        self.pending_trace_pc = pbr_pc;
        self.pending_trace_opcode = opcode;
        self.pending_trace_interrupts = self.cpu.interrupts_taken;
        self.pending_trace_armed = true;
    }

    /// Classify the instruction that just ran into a control-flow event (`v1.25.0`, T-FP-C1).
    ///
    /// Derived from the opcode plus the *actual* post-execution `PBR:PC` rather than from decoding
    /// the operand: a `JSR` whose target is computed (`JSR (a,X)`) has no static destination, and a
    /// conditional path would have to re-implement the CPU to know whether it was taken. Reading
    /// where the CPU actually went cannot be wrong about either.
    ///
    /// An interrupt taken *between* instructions is caught **before** the opcode is consulted, and
    /// has to be: on a step that vectored, the CPU fetched no opcode at all, so
    /// `pending_trace_opcode` still holds the *previous* instruction's byte. Classifying from it
    /// would not merely miss the interrupt — an NMI arriving right after a `JSR` would be recorded
    /// as a second `Call`, with `to` pointing at the NMI vector's target. `Cpu::interrupts_taken`
    /// is the unambiguous signal, which is why this is one classifier rather than separate call and
    /// interrupt hooks.
    #[cfg(feature = "debug-hooks")]
    fn trace_after_step(&mut self) {
        // `pending_trace_armed` guards the case where tracing was turned on between the two hooks:
        // the fields below would then describe some earlier step entirely.
        if !self.bus.trace().is_tracing() || !self.pending_trace_armed {
            return;
        }
        if self.cpu.interrupts_taken != self.pending_trace_interrupts {
            let kind = if self.cpu.last_interrupt_was_nmi {
                crate::trace::EventKind::Nmi
            } else {
                crate::trace::EventKind::Irq
            };
            let to = (u32::from(self.cpu.regs.pbr) << 16) | u32::from(self.cpu.regs.pc);
            self.bus
                .trace_mut()
                .record_event(kind, self.pending_trace_pc, to);
            return;
        }
        let kind = match self.pending_trace_opcode {
            // JSR a, JSL al, JSR (a,X)
            0x20 | 0x22 | 0xFC => crate::trace::EventKind::Call,
            // RTS, RTL
            0x60 | 0x6B => crate::trace::EventKind::Return,
            0x40 => crate::trace::EventKind::Rti,
            0x00 => crate::trace::EventKind::Brk,
            0x02 => crate::trace::EventKind::Cop,
            _ => return,
        };
        let to = (u32::from(self.cpu.regs.pbr) << 16) | u32::from(self.cpu.regs.pc);
        self.bus
            .trace_mut()
            .record_event(kind, self.pending_trace_pc, to);
    }

    /// Advance by one CPU instruction (kept for API compatibility with the old skeleton). The
    /// real timebase advances through the CPU's bus accesses, not a bare master tick.
    pub fn tick_one_master(&mut self) {
        let _ = self.seed;
        self.step_instruction();
    }

    /// Serialize the entire emulated machine — the main CPU, the whole [`Bus`] (PPU, APU, DMA,
    /// WRAM, plus the cart's coprocessor state and battery SRAM if a cart is loaded), the
    /// determinism seed, the boot/HDMA-line bookkeeping, and (if present) the SA-1 second CPU
    /// plus its master-clock catch-up accounting — into a versioned binary blob
    /// (`docs/adr/0006-save-state-format.md`). The blob leads with a 4-byte magic and a `u16`
    /// format-version header that [`Self::load_state`] checks before trusting anything else. The
    /// cart's ROM is never embedded (`docs/adr/0003`'s "never embed a ROM/firmware byte" posture,
    /// already applied to every coprocessor's firmware) — restoring a cart-carrying save-state
    /// requires the caller to have already loaded the SAME ROM onto the target `System` first.
    #[must_use]
    pub fn save_state(&self) -> Vec<u8> {
        let mut w = SaveWriter::new();
        w.write_bytes(MAGIC);
        w.write_u16(FORMAT_VERSION);
        self.cpu.save_state(&mut w);
        self.bus.save_state(&mut w);
        w.section(*b"SYS0", |s| {
            s.write_u64(self.seed);
            s.write_bool(self.booted);
            s.write_u16(self.last_line);
            match &self.sa1_cpu {
                Some(cpu) => {
                    s.write_bool(true);
                    cpu.save_state(s);
                }
                None => s.write_bool(false),
            }
            s.write_u64(self.sa1_last_master);
            s.write_u64(self.sa1_credit);
        });
        w.into_bytes()
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError::BadMagic`] if `bytes` doesn't lead with the expected magic (not a
    /// RustySNES save-state at all); [`SaveStateError::UnsupportedVersion`] if the format version
    /// is newer than this build understands; [`SaveStateError`] on truncated/corrupt input or a
    /// section with unconsumed trailing bytes; or [`SaveStateError::Invalid`] if the save-state's
    /// SA-1-second-CPU presence, or (via [`Bus::load_state`]) cart presence/SRAM size, doesn't
    /// match this `System`'s own installed state — restoring onto the wrong ROM/board
    /// configuration is rejected rather than silently corrupting it.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), SaveStateError> {
        let mut r = SaveReader::new(bytes);
        if r.read_bytes(4)? != MAGIC {
            return Err(SaveStateError::BadMagic);
        }
        let version = r.read_u16()?;
        if version > FORMAT_VERSION {
            return Err(SaveStateError::UnsupportedVersion {
                found: version,
                max: FORMAT_VERSION,
            });
        }
        self.cpu.load_state(&mut r)?;
        self.bus.load_state(&mut r)?;
        let mut s = r.expect_section(*b"SYS0")?;
        self.seed = s.read_u64()?;
        self.booted = s.read_bool()?;
        self.last_line = s.read_u16()?;
        let had_sa1 = s.read_bool()?;
        match (&mut self.sa1_cpu, had_sa1) {
            (Some(cpu), true) => cpu.load_state(&mut s)?,
            (None, false) => {}
            (Some(_), false) | (None, true) => {
                return Err(SaveStateError::Invalid(alloc::string::String::from(
                    "save-state SA-1 second-CPU presence does not match this System's \
                     installed cart (load the same ROM before restoring)",
                )));
            }
        }
        self.sa1_last_master = s.read_u64()?;
        self.sa1_credit = s.read_u64()?;
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "SYS0 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        // SYS0 is the envelope's last section; reject anything appended after it (a corrupted or
        // concatenated blob), the same "no unconsumed trailing bytes" posture every nested
        // section's own load_state already enforces on itself.
        if r.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "save-state has {} trailing byte(s) after the SYS0 section",
                r.remaining()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustysnes_cart::Cart;
    use rustysnes_ppu::Region as PpuRegion;

    /// A minimal synthetic LoROM header (offset `$7FC0`) with a controllable region byte
    /// (`$7FD9` for this LoROM offset — `field::REGION` in `rustysnes-cart`'s header module,
    /// duplicated here rather than exported, since it's a small, stable, publicly-documented
    /// header layout, `docs/cartridge-format.md`). `region == 0x00` selects Japan/NTSC; `0x02`
    /// selects Europe/PAL (`Header::region_from_code`'s `0x02..=0x0C -> Pal` range).
    fn synth_rom(region: u8) -> alloc::vec::Vec<u8> {
        let mut rom = alloc::vec![0u8; 0x1_0000];
        let h = 0x7FC0;
        rom[h + 0x15] = 0x20; // MAP_MODE: slow LoROM
        rom[h + 0x16] = 0x00; // CHIPSET: ROM only
        rom[h + 0x18] = 0x00; // RAM_SIZE: none
        rom[h + 0x19] = region; // REGION
        let checksum: u16 = 0x1234;
        let complement = !checksum;
        rom[h + 0x1C..h + 0x1E].copy_from_slice(&complement.to_le_bytes());
        rom[h + 0x1E..h + 0x20].copy_from_slice(&checksum.to_le_bytes());
        rom[h + 0x3C..h + 0x3E].copy_from_slice(&0x8000u16.to_le_bytes()); // reset vector
        rom
    }

    #[test]
    fn ntsc_cart_auto_detects_ntsc_region_on_reset() {
        let mut sys = System::new(0);
        sys.bus.cart = Some(Cart::from_rom(&synth_rom(0x00)).expect("ntsc header"));
        sys.reset();
        assert_eq!(sys.bus.ppu.region(), PpuRegion::Ntsc);
        assert_eq!(sys.bus.ppu.region().lines_per_frame(), 262);
    }

    #[test]
    fn pal_cart_auto_detects_pal_region_on_reset() {
        let mut sys = System::new(0);
        sys.bus.cart = Some(Cart::from_rom(&synth_rom(0x02)).expect("pal header"));
        sys.reset();
        assert_eq!(sys.bus.ppu.region(), PpuRegion::Pal);
        assert_eq!(sys.bus.ppu.region().lines_per_frame(), 312);

        // End-to-end: booting and running one full frame actually completes at the PAL line
        // count, not just the region flag being set (proves the auto-detected region reaches
        // the PPU's real dot/scanline timeline, not merely a cosmetic label).
        sys.run_frame();
        assert_eq!(sys.bus.ppu.frame_count(), 1);
    }

    #[test]
    fn no_cart_reset_does_not_touch_region() {
        // sync_region_from_cart is a no-op with no cart installed; region stays at whatever the
        // Bus was constructed with (System::new always builds NTSC by default).
        let mut sys = System::new(0);
        sys.reset();
        assert_eq!(sys.bus.ppu.region(), PpuRegion::Ntsc);
    }

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

    #[test]
    fn system_state_round_trips_without_a_cart() {
        let mut sys = System::new(42);
        sys.reset();
        sys.cpu.regs.a = 0x1234;
        sys.bus.clock.master = 999;

        let bytes = sys.save_state();

        let mut fresh = System::new(0);
        fresh.load_state(&bytes).unwrap();

        assert_eq!(fresh.cpu.regs.a, 0x1234);
        assert_eq!(fresh.bus.clock.master, 999);
        assert_eq!(fresh.seed, 42);
    }

    #[test]
    fn bad_magic_is_rejected_not_panicked_on() {
        let sys = System::new(0);
        let mut bytes = sys.save_state();
        bytes[0] = b'X';

        let mut fresh = System::new(0);
        assert!(matches!(
            fresh.load_state(&bytes),
            Err(SaveStateError::BadMagic)
        ));
    }

    #[test]
    fn newer_format_version_is_rejected_not_panicked_on() {
        let sys = System::new(0);
        let mut bytes = sys.save_state();
        // The u16 format-version field immediately follows the 4-byte magic.
        bytes[4..6].copy_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());

        let mut fresh = System::new(0);
        assert!(matches!(
            fresh.load_state(&bytes),
            Err(SaveStateError::UnsupportedVersion { .. })
        ));
    }

    /// A hardware NMI must be recorded as an `Nmi` control-flow event.
    ///
    /// This is not a redundant assertion about an enum: the classifier originally read only
    /// `pending_trace_opcode`, and on a step that vectors, the CPU fetches **no** opcode — the
    /// field still holds the PREVIOUS instruction's byte. So an NMI arriving after a `JSR` was
    /// recorded as a second `Call` whose `to` pointed at the NMI vector's target, and an NMI after
    /// an ordinary instruction was dropped entirely. The `to` assertion below is what pins the
    /// difference: it must be the handler, not wherever the last instruction went.
    #[cfg(feature = "debug-hooks")]
    #[test]
    fn a_hardware_nmi_is_recorded_as_an_nmi_event() {
        const HANDLER: u16 = 0x9000;
        let mut rom = synth_rom(0x00);
        // `NOP` everywhere the CPU can land, so nothing else generates a control-flow event
        // (a zero byte would be `BRK`, which does).
        rom[0..0x7FC0].fill(0xEA);
        // The program enables the VBlank NMI itself, rather than the test reaching into the
        // Bus: `LDA #$80` / `STA $4200` (NMITIMEN bit 7). Neither is a control-flow instruction,
        // so neither can produce an event of its own.
        rom[0x0000..0x0005].copy_from_slice(&[0xA9, 0x80, 0x8D, 0x00, 0x42]);
        // Emulation-mode NMI vector ($00:FFFA -> rom[$7FFA] under LoROM).
        rom[0x7FFA..0x7FFC].copy_from_slice(&HANDLER.to_le_bytes());
        rom[0x7FFC..0x7FFE].copy_from_slice(&0x8000u16.to_le_bytes()); // reset vector

        let mut sys = System::new(0);
        sys.bus.cart = Some(Cart::from_rom(&rom).expect("synth rom"));
        sys.reset();
        sys.bus.trace_mut().set_tracing(true);

        // Run at most two frames' worth of instructions; VBlank arrives well inside that.
        for _ in 0..200_000 {
            sys.step_instruction();
            if !sys.bus.trace().events().is_empty() {
                break;
            }
        }

        let events = sys.bus.trace().events();
        let nmi = events
            .iter()
            .find(|e| e.kind == crate::trace::EventKind::Nmi)
            .expect("an enabled VBlank NMI must produce an Nmi event");
        assert_eq!(
            nmi.to,
            u32::from(HANDLER),
            "`to` must be the NMI handler, not where the previous instruction went"
        );
        assert!(
            !events
                .iter()
                .any(|e| e.kind == crate::trace::EventKind::Call),
            "a NOP-only program plus one NMI must not record a Call"
        );
    }
}
