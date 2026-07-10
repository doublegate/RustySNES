//! The Bus owns everything mutable.
//!
//! It holds the PPU1/PPU2, the SPC700+S-DSP, the cart (→ board / coprocessor), WRAM,
//! controllers, the open-bus latch, the CPU-side registers (`$4200-$421F`), the mul/div unit,
//! and the DMA/HDMA controller. The 65C816 borrows `&mut Bus` during an instruction; the PPU and
//! DMA see narrower bus traits ([`rustysnes_ppu::VideoBus`], [`crate::dma_bus::DmaBus`])
//! implemented on this same struct. The APU owns its ARAM/DSP internally; the Bus drives it
//! through [`rustysnes_apu::Apu`] directly — the four `$2140-$2143` port latches via
//! [`rustysnes_apu::Apu::cpu_read_port`]/[`rustysnes_apu::Apu::cpu_write_port`] and the SPC clock
//! via [`rustysnes_apu::Apu::advance_smp_cycle`] (the integer-accumulator async resync).
//!
//! ## The master clock lives here
//!
//! The SNES CPU cycle is **6, 8, or 12 master clocks** depending on the address region (and the
//! FastROM bit). The CPU asks the Bus for the access cost via [`CpuBus::access_cycles`] (ares
//! `wait`) and drives the clock with [`CpuBus::advance`] (ares `step`), sequencing the advance
//! around each [`CpuBus::read24`]/[`CpuBus::write24`] so the access lands at the hardware-exact
//! instant — a write at the end of its cycle, a read four clocks before it. Each master-clock
//! advance steps the PPU dot clock (4 master/dot) and the SPC accumulator in lockstep, so a
//! mid-instruction PPU event (an HV-IRQ at a precise dot, a mid-scanline register write seen at
//! the right hcounter) lands at the right time without per-quirk patches.

// Byte-splitting a 16-bit register into its low/high `u8` (`reg as u8`, `(reg >> 8) as u8`) and
// folding addresses to `u16`/`usize` is the bread-and-butter of a memory bus; flagging each
// deliberate narrowing cast would bury real issues, so the cast-precision family is allowed for
// this module only (mirrors `rustysnes-cpu/src/exec.rs`).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::struct_excessive_bools
)]

use alloc::boxed::Box;

use rustysnes_apu::Apu;
use rustysnes_cart::{Cart, Region};
use rustysnes_cpu::Bus as CpuBus;
use rustysnes_ppu::{Ppu, Region as PpuRegion, VideoBus};
use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::dma::Dma;
use crate::dma_bus::DmaBus;

/// WRAM size — the SNES has 128 KiB of work RAM (`$7E0000-$7FFFFF`).
const WRAM_SIZE: usize = 128 * 1024;
/// Master clocks per PPU dot (nominal; long-dot remainder folded into the 1364/1360/1368 line).
const MASTER_PER_DOT: u32 = 4;
/// PPU dot at which each visible scanline's HDMA transfer fires — ares' `hdmaPosition` of hcounter
/// 1104 (`sfc/cpu/timing.cpp`) divided by [`MASTER_PER_DOT`]. Running the table at this exact dot
/// (rather than the scanline boundary) latches a mid-line `$420C` write on the hardware-correct
/// scanline, which is what makes the `hdmaen_latch_test` show a banded HDMAEN-vs-latch crossing.
/// Defined equal to `rustysnes_ppu::RENDER_DOT` (PPU-owned single source of truth, since this is
/// fundamentally a video-timing fact) — `hdma_run_dot_matches_ppu_render_dot` below asserts the
/// two never drift apart.
const HDMA_RUN_DOT: u16 = rustysnes_ppu::RENDER_DOT;
/// SPC700 fractional-clock numerator (master ticks → SMP **base** clocks).
///
/// The unit the APU advances per [`rustysnes_apu::Apu::advance_smp_cycle`] call is one SMP *base*
/// clock = `apuFrequency / 12` (ares `SMP::create(apuFrequency()/12, …)`; `apuFrequency =
/// 32040 × 768 = 24_606_720` Hz → base = `2_050_560` Hz). A normal SMP access is `SMP_WAIT` = 2
/// base clocks, giving the ~1.025 MHz effective opcode rate and an exact `32_040` Hz S-DSP sample.
///
/// The async resync (`docs/scheduler.md` §async-resync, ADR 0004) is an **integer** accumulator —
/// no floats, so the SPC domain is bit-deterministic. The exact rational is
/// `2_050_560 / 21_477_270` (SMP base rate over the NTSC master rate); gcd = 30, giving the reduced
/// `68_352 / 715_909` kept here to bound accumulator growth (`spc_accum` stays below `SPC_DEN`).
const SPC_NUM: u64 = 68_352;
/// SPC700 fractional-clock denominator: the NTSC master clock Hz, reduced by gcd = 30.
const SPC_DEN: u64 = 715_909;

/// The master-clock phase + the CPU-side timing registers the Bus advances in lockstep.
#[derive(Debug, Clone)]
pub struct Clock {
    /// Cumulative master-clock ticks since power-on.
    pub master: u64,
    /// Master cycles owed to the PPU before its next dot.
    dot_accum: u32,
    /// Fractional accumulator for the asynchronous SPC700 domain.
    spc_accum: u64,
    /// `$420D` MEMSEL bit 0 — FastROM (`true` = 6-clock WS2 ROM, `false` = 8-clock).
    fast_rom: bool,
    /// `$4200` NMITIMEN — bit7 NMI-enable, bit5 V-IRQ, bit4 H-IRQ, bit0 auto-joypad.
    nmitimen: u8,
    /// Latched NMI edge awaiting the `CPU` poll (set at `VBlank` only when NMI is enabled).
    nmi_line: bool,
    /// `$4210` RDNMI bit7 — the `VBlank`-occurred flag. Set at `VBlank` start **regardless** of
    /// the `NMITIMEN` enable (hardware), cleared on read. ROMs poll this to sync to `VBlank`
    /// without taking the interrupt (e.g. gilyon's `wait_for_vblank`).
    rdnmi_flag: bool,
    /// Level IRQ line (HV-IRQ / coprocessor / APU timer), cleared on `$4211` read.
    irq_line: bool,
    /// `$4207/8` HTIME — the H-IRQ comparator.
    htime: u16,
    /// `$4209/A` VTIME — the V-IRQ comparator.
    vtime: u16,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            master: 0,
            dot_accum: 0,
            spc_accum: 0,
            fast_rom: false,
            nmitimen: 0,
            nmi_line: false,
            rdnmi_flag: false,
            irq_line: false,
            htime: 0x01FF,
            vtime: 0x01FF,
        }
    }
}

/// The CPU multiply/divide unit (`$4202-$4206` → `$4214-$4217`). The SNES computes these with an
/// 8-CPU-cycle hardware latency; the deterministic core resolves them instantly (the result is
/// what tests read), which is accurate for every documented program that waits for the real
/// hardware's own latency before reading `RDMPY`/`RDDIV` — as every known commercial title does.
///
/// **Deliberately not modeled: the SNESdev-documented overlapping-operation errata** ("Starting
/// a multiplication (`$4203` WRMPYB) or division (`$4206` WRDIVB) while the 5A22 is still
/// processing a previous multiplication or division can cause the 5A22 to output erroneous
/// values to `RDDIV` and/or `RDMPY`," <https://snes.nesdev.org/wiki/Errata>). This is genuinely
/// **undefined** hardware behavior — no canonical "corrupted" value is documented anywhere, so
/// there is nothing correct to port; inventing a specific fabricated corruption value would
/// itself violate the determinism contract's spirit (`docs/adr/0004`) by pretending a real, one
/// true answer exists for a case real hardware itself doesn't define one for. No known program
/// relies on this (a program that hit it would already be behaving unpredictably on real
/// hardware), so this is a **documented, intentional non-goal**, not an open gap — see
/// `to-dos/VERSION-PLAN.md`'s `v0.5.0 "Fidelity"` hardware-gotcha list for the same reasoning.
#[derive(Debug, Clone, Default)]
struct MulDiv {
    mpya: u8,
    dividend: u16,
    rddiv: u16,
    rdmpy: u16,
}

impl Clock {
    fn save_state(&self, s: &mut SaveWriter) {
        s.write_u64(self.master);
        s.write_u32(self.dot_accum);
        s.write_u64(self.spc_accum);
        s.write_bool(self.fast_rom);
        s.write_u8(self.nmitimen);
        s.write_bool(self.nmi_line);
        s.write_bool(self.rdnmi_flag);
        s.write_bool(self.irq_line);
        s.write_u16(self.htime);
        s.write_u16(self.vtime);
    }

    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.master = s.read_u64()?;
        self.dot_accum = s.read_u32()?;
        self.spc_accum = s.read_u64()?;
        self.fast_rom = s.read_bool()?;
        self.nmitimen = s.read_u8()?;
        self.nmi_line = s.read_bool()?;
        self.rdnmi_flag = s.read_bool()?;
        self.irq_line = s.read_bool()?;
        // htime/vtime are 9-bit comparators (write24 masks bit 8 with & 1 at $4208/$420A already).
        self.htime = s.read_u16()? & 0x01FF;
        self.vtime = s.read_u16()? & 0x01FF;
        Ok(())
    }
}

impl MulDiv {
    fn save_state(&self, s: &mut SaveWriter) {
        s.write_u8(self.mpya);
        s.write_u16(self.dividend);
        s.write_u16(self.rddiv);
        s.write_u16(self.rdmpy);
    }

    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.mpya = s.read_u8()?;
        self.dividend = s.read_u16()?;
        self.rddiv = s.read_u16()?;
        self.rdmpy = s.read_u16()?;
        Ok(())
    }
}

/// Everything mutable lives here.
pub struct Bus {
    /// The video subsystem (PPU1 + PPU2).
    pub ppu: Ppu,
    /// The audio subsystem (SPC700 + S-DSP + ARAM).
    pub apu: Apu,
    /// The loaded cartridge (board mapping + any coprocessor), or `None` before a ROM loads.
    pub cart: Option<Cart>,
    /// The 8-channel DMA/HDMA controller (`$420B`/`$420C`, `$43xx`).
    pub dma: Dma,
    /// The master-clock phase + CPU timing registers.
    pub clock: Clock,
    /// 128 KiB work RAM (`$7E0000-$7FFFFF`).
    wram: Box<[u8; WRAM_SIZE]>,
    /// WRAM port address (`$2181-$2183`), auto-incremented by `$2180` access.
    wram_addr: u32,
    /// Controller shift latches (`$4016/$4017`) + the auto-read result (`$4218-$421F`).
    joypad: [u16; 2],
    joypad_strobe: bool,
    /// Open-bus latch: the last value driven on the data bus.
    #[allow(clippy::struct_field_names)] // "open_bus" is the hardware name for the latch.
    open_bus: u8,
    muldiv: MulDiv,
    /// The last visible scanline HDMA was serviced on, so [`Bus::advance_master`] runs each line's
    /// HDMA exactly once — even when the master clock is being advanced *inside* a GP-DMA (real
    /// hardware interleaves HDMA at the start of every scanline, preempting the general DMA).
    last_hdma_line: u16,
    /// Re-entrancy guard: true while an HDMA transfer's own cycle cost is being charged, so the
    /// nested `advance_master` doesn't recursively re-trigger HDMA for the same line.
    in_hdma: bool,
    /// Whether this frame's V=0 HDMA setup (table reset + reload) has already fired, so it runs
    /// exactly once per frame independent of the per-line run at [`HDMA_RUN_DOT`].
    hdma_setup_done: bool,
    /// Active cheat-code patches (`v0.8.0`, T-81-003) — checked on every CPU-visible read in
    /// [`CpuBus::read24`]. Empty (the default, and the only state possible unless a frontend
    /// explicitly calls [`Self::set_cheats`]) costs exactly one `is_empty()` branch per read.
    cheats: alloc::vec::Vec<crate::cheat::CheatPatch>,
    /// 65C816 read/write watchpoints (`v0.8.0`, T-81-001b) — compiled out entirely when
    /// `debug-hooks` is off. See [`crate::watchpoint`]'s module doc.
    #[cfg(feature = "debug-hooks")]
    watchpoints: crate::watchpoint::WatchpointState,
    /// The CPU's `PBR:PC` at the moment of its current access, set by [`Self::set_debug_pc`]
    /// (the scheduler calls it before each [`rustysnes_cpu::Cpu::step`]) — feeds
    /// [`crate::watchpoint::WatchpointHit::pbr_pc`]. `debug-hooks`-only, same as `watchpoints`.
    #[cfg(feature = "debug-hooks")]
    debug_pc: u32,
}

impl Default for Bus {
    fn default() -> Self {
        Self::new(Region::Ntsc)
    }
}

impl Bus {
    /// Construct a power-on Bus for the given console region.
    ///
    /// # Panics
    /// Panics only if the 128 KiB WRAM allocation cannot be sized to the fixed `WRAM_SIZE` array
    /// (an out-of-memory condition at power-on), which cannot happen for the constant size.
    #[must_use]
    pub fn new(region: Region) -> Self {
        let ppu_region = match region {
            Region::Ntsc => PpuRegion::Ntsc,
            Region::Pal => PpuRegion::Pal,
        };
        Self {
            ppu: Ppu::with_region(ppu_region),
            apu: Apu::new(),
            cart: None,
            dma: Dma::new(),
            clock: Clock::default(),
            wram: alloc::vec![0u8; WRAM_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            wram_addr: 0,
            joypad: [0; 2],
            joypad_strobe: false,
            open_bus: 0,
            muldiv: MulDiv::default(),
            last_hdma_line: u16::MAX,
            in_hdma: false,
            hdma_setup_done: false,
            cheats: alloc::vec::Vec::new(),
            #[cfg(feature = "debug-hooks")]
            watchpoints: crate::watchpoint::WatchpointState::default(),
            #[cfg(feature = "debug-hooks")]
            debug_pc: 0,
        }
    }

    /// Reconfigure the PPU's region (line count / 50-vs-60 Hz status bit) from the installed
    /// cart's header, auto-detecting NTSC vs PAL rather than requiring the frontend to guess or
    /// hardcode it. A no-op when no cart is installed. Region only ever affects the PPU's
    /// line-count/status-bit timeline here — the differing NTSC/PAL master-clock *rate* (Hz) is a
    /// real-world audio/video pacing concern the frontend owns (`docs/adr/0004`); the core's
    /// master-clock counter is a pure tick count, not wall-clock time, so nothing else in the
    /// core depends on which oscillator frequency a real console would use.
    // Deliberately NOT `const fn`: `Bus` holds heap-allocated/complex nested state (`Box`-owned
    // WRAM, the PPU/APU), and this method reads a `Cart` (a `Box<dyn Board>` behind it) — pinning
    // this to a `const` API guarantee for no actual const-context caller buys nothing and would
    // force a breaking API change the moment any of that state gains a genuinely non-const need
    // (logging, validation, additional resets).
    #[allow(clippy::missing_const_for_fn)]
    pub fn sync_region_from_cart(&mut self) {
        let Some(cart) = &self.cart else { return };
        let ppu_region = match cart.header.region {
            Region::Ntsc => PpuRegion::Ntsc,
            Region::Pal => PpuRegion::Pal,
        };
        self.ppu.set_region(ppu_region);
    }

    /// Set the latched controller state for a player (`0` = P1, `1` = P2). 12-bit `BYsSUDLR....`.
    pub fn set_joypad(&mut self, player: usize, state: u16) {
        if let Some(slot) = self.joypad.get_mut(player) {
            *slot = state;
        }
    }

    /// The latched controller state for a player (`0` = P1, `1` = P2) — the read side of
    /// [`Self::set_joypad`], for TAS movie recording (`crate::movie::MovieRecorder`) and the
    /// debugger overlay.
    #[must_use]
    pub fn joypad(&self, player: usize) -> u16 {
        self.joypad.get(player).copied().unwrap_or(0)
    }

    /// Non-intrusive read of WRAM for the test harness + debugger (does NOT advance the clock,
    /// touch open bus, or trip register side effects). I/O registers and the cart region return
    /// `0` — this is for inspecting RAM-resident test-result variables, not for emulation.
    #[must_use]
    pub fn peek_wram(&self, addr24: u32) -> u8 {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = (addr24 & 0xFFFF) as u16;
        match bank {
            0x7E..=0x7F => self.wram[(addr24 & 0x1_FFFF) as usize],
            0x00..=0x3F | 0x80..=0xBF if addr < 0x2000 => self.wram[(addr & 0x1FFF) as usize],
            _ => 0,
        }
    }

    /// Non-intrusive write of WRAM (the write counterpart to [`Self::peek_wram`], same
    /// addressing, same "no clock/open-bus/register side effects" contract) — for `rustysnes-
    /// script`'s Lua `emu.write`. A write to an address outside WRAM's mirrors is silently
    /// ignored (matching `peek_wram`'s `_ => 0` read side) rather than erroring, since a script
    /// address is arbitrary user input, not a bug to surface loudly. (Cheat codes, T-81-003, use
    /// [`Self::set_cheats`]'s CPU-read intercept instead — real Game Genie/Pro Action Replay
    /// codes overwhelmingly target cartridge ROM, which this WRAM-only accessor cannot reach.)
    pub fn poke_wram(&mut self, addr24: u32, val: u8) {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = (addr24 & 0xFFFF) as u16;
        match bank {
            0x7E..=0x7F => self.wram[(addr24 & 0x1_FFFF) as usize] = val,
            0x00..=0x3F | 0x80..=0xBF if addr < 0x2000 => self.wram[(addr & 0x1FFF) as usize] = val,
            _ => {}
        }
    }

    /// Install the currently-active cheat-code patches (`v0.8.0`, T-81-003), replacing any
    /// previously installed set. [`CpuBus::read24`] checks this list on every CPU-visible read
    /// and substitutes a matching patch's value — the same point in the pipeline real Game
    /// Genie/Pro Action Replay hardware intercepts at, which is why this is a read intercept and
    /// not a `poke_wram`-style direct write: those codes overwhelmingly target cartridge ROM, not
    /// WRAM, so a write-based model would silently do nothing for the vast majority of real
    /// codes. The underlying ROM/RAM byte is never modified — only what the CPU observes reading
    /// it.
    pub fn set_cheats(&mut self, patches: &[crate::cheat::CheatPatch]) {
        self.cheats.clear();
        self.cheats.extend_from_slice(patches);
    }

    /// Install the currently-armed read/write watchpoints (`v0.8.0`, T-81-001b), replacing any
    /// previously installed set. See [`crate::watchpoint::WatchpointState::set_watchpoints`].
    #[cfg(feature = "debug-hooks")]
    pub fn set_watchpoints(&mut self, points: &[crate::watchpoint::Watchpoint]) {
        self.watchpoints.set_watchpoints(points);
    }

    /// Drain every watchpoint hit recorded since the last call.
    #[cfg(feature = "debug-hooks")]
    pub fn take_watchpoint_hits(&mut self) -> alloc::vec::Vec<crate::watchpoint::WatchpointHit> {
        self.watchpoints.take_hits()
    }

    /// Record the CPU's current `PBR:PC` (24-bit, `$bank:offset`) so a watchpoint hit during the
    /// access this instruction is about to make can attribute itself to the right instruction.
    /// The scheduler calls this once before each [`rustysnes_cpu::Cpu::step`]
    /// ([`crate::scheduler::System::run_frame`]/[`crate::scheduler::System::step_instruction`]).
    #[cfg(feature = "debug-hooks")]
    pub const fn set_debug_pc(&mut self, pbr_pc: u32) {
        self.debug_pc = pbr_pc;
    }

    /// Whether the PPU has a finished frame ready to present.
    #[must_use]
    pub const fn frame_ready(&self) -> bool {
        self.ppu.frame_ready()
    }

    /// The PPU framebuffer (256×239 15-bit BGR).
    #[must_use]
    pub fn framebuffer(&self) -> &[u16] {
        self.ppu.framebuffer()
    }

    // --- The master-clock advance (the lockstep heart). ------------------------------------

    /// Advance the master clock by `n` ticks, stepping the PPU dot clock + SPC accumulator in
    /// lockstep and re-deriving the NMI/HV-IRQ phases.
    fn advance_master(&mut self, n: u32) {
        for _ in 0..n {
            self.clock.master = self.clock.master.wrapping_add(1);
            self.clock.dot_accum += 1;
            // Captured BEFORE `tick_ppu_dot()` (if it fires this sub-tick) increments the PPU's
            // dot counter — this is the exact dot value [`Ppu::tick_dot`]'s own render-vs-HDMA
            // ordering decision used internally (it composites the finishing line using the
            // pre-increment `h`, then increments). Reading `self.ppu.dot()` fresh AFTER the call
            // instead (an earlier draft did this) observes the POST-increment value, so the HDMA
            // run-check below would fire a whole dot-window early — on the FIRST of the four
            // master-clock sub-ticks where the dot reads [`HDMA_RUN_DOT`], not the LAST (the one
            // coincident with the render call) — silently putting HDMA back ahead of render for
            // the same line, exactly the ordering this fix exists to prevent.
            let pre_tick_dot = self.ppu.dot();
            let dot_ticked = if self.clock.dot_accum >= MASTER_PER_DOT {
                self.clock.dot_accum -= MASTER_PER_DOT;
                self.tick_ppu_dot();
                true
            } else {
                false
            };
            // HDMA, clock-driven so both its per-frame init (V=0) and per-line transfers stay
            // scanline-accurate even while the master clock is being advanced *inside* a GP-DMA —
            // hardware re-initializes HDMA at V=0 and interleaves a transfer at the start of every
            // visible scanline, preempting the general DMA, regardless of a DMA spanning the frame
            // boundary. Driving it from the scheduler instead delayed the V=0 init behind a
            // frame-crossing framebuffer DMA, shifting the whole HDMA table late (Star Fox's
            // force-blank then missed its own framebuffer DMA). The `in_hdma` guard stops the
            // transfer's own cost (the nested `advance_master`) from re-triggering the same line.
            if !self.in_hdma && self.dma.hdma_enable != 0 {
                let v = self.ppu.scanline();
                let vh = self.ppu.visible_height();
                // ares services HDMA at two distinct points (`sfc/cpu/timing.cpp`): a once-per-frame
                // *setup* at V=0 (`service_hdma_line(0, …)` resets the tables + reloads), and a
                // per-visible-line *run* at hcounter 1104 = [`HDMA_RUN_DOT`]. Running the transfer at
                // that exact dot — not at the scanline boundary — latches a mid-line HDMAEN write on
                // the hardware-correct scanline (the `hdmaen_latch_test` crossing). `dot_ticked` gates
                // this to the one sub-tick that actually advanced the dot (see `pre_tick_dot`'s doc).
                if v == 0 {
                    if !self.hdma_setup_done {
                        self.hdma_setup_done = true;
                        self.service_hdma(0, vh);
                    }
                } else {
                    self.hdma_setup_done = false;
                    if v <= vh
                        && dot_ticked
                        && pre_tick_dot == HDMA_RUN_DOT
                        && self.last_hdma_line != v
                    {
                        self.last_hdma_line = v;
                        self.service_hdma(v, vh);
                    }
                }
            }
            self.clock.spc_accum += SPC_NUM;
            while self.clock.spc_accum >= SPC_DEN {
                self.clock.spc_accum -= SPC_DEN;
                // Release one SPC700 master cycle in lockstep with the master clock. The four
                // CPU↔APU port latches live INSIDE the `Apu` (`cpu_read_port`/`cpu_write_port`),
                // so advancing here at master-clock granularity means a CPU read of $2140-$2143
                // already observes every SMP port write up to this exact master instant — the
                // deterministic async resync (T-31-003; `docs/scheduler.md` §async-resync).
                self.apu.advance_smp_cycle();
            }
            // Release a host-synced coprocessor (Super FX/GSU) one master clock at a time, in
            // lockstep with the CPU's own instruction stream — not drained to completion inside
            // the single bus write that arms it. Real hardware runs the GSU as a genuinely
            // concurrent cothread (ares `SuperFX : Thread`); the CPU keeps executing its own
            // instructions while the GSU works and only observes the result whenever it next
            // polls, instead of the entire render completing "atomically" before the CPU's next
            // instruction can run (`Board::coprocessor_tick` doc has the detail).
            if let Some(c) = self.cart.as_mut() {
                c.coprocessor_tick();
            }
        }
    }

    /// Run one HDMA phase (`line == 0` → per-frame reset+setup; else the visible-line transfer),
    /// charging its master-clock cost back onto the scheduler. The `in_hdma` re-entrancy guard
    /// stops the nested `advance_master(cost)` from re-triggering HDMA for the same line.
    fn service_hdma(&mut self, line: u16, vh: u16) {
        self.in_hdma = true;
        let mut dma = core::mem::take(&mut self.dma);
        let cost = dma.service_hdma_line(line, vh, self);
        self.dma = dma;
        if cost > 0 {
            self.advance_master(cost);
        }
        self.in_hdma = false;
    }

    /// Tick the PPU one dot through a cart-only view (split borrow), then harvest its NMI/IRQ.
    fn tick_ppu_dot(&mut self) {
        // Keep the PPU the single owner of the dot-phase HV-IRQ comparison.
        let enable_h = self.clock.nmitimen & 0x10 != 0;
        let enable_v = self.clock.nmitimen & 0x20 != 0;
        self.ppu
            .set_hv_irq(enable_h, enable_v, self.clock.htime, self.clock.vtime);

        let mut view = CartView {
            cart: &mut self.cart,
            open: self.open_bus,
        };
        self.ppu.tick_dot(&mut view);

        if self.ppu.nmi_pending() {
            self.ppu.ack_nmi();
            // The RDNMI VBlank flag sets unconditionally; the NMI *interrupt* only when enabled.
            self.clock.rdnmi_flag = true;
            if self.clock.nmitimen & 0x80 != 0 {
                self.clock.nmi_line = true;
            }
        }
        if self.ppu.irq_pending() {
            self.ppu.ack_irq();
            self.clock.irq_line = true;
        }
    }

    // --- B-bus ($2100-$21FF) register access (PPU, APU ports, WRAM port). ------------------

    fn b_read(&mut self, low: u8) -> u8 {
        match low {
            0x00..=0x3F => self.ppu.read_reg(0x2100 | u16::from(low)),
            // $2140-$2143 — the four CPU↔APU communication ports. A CPU read returns what the
            // SMP last wrote to that port (a one-way latch, NOT an echo of the CPU's own write).
            // The APU is already advanced up to "now" by the lockstep accumulator in
            // `advance_master`, so this observes every SMP write up to this master instant.
            0x40..=0x43 => self.apu.cpu_read_port(low & 3),
            0x80 => {
                let v = self.wram[(self.wram_addr & 0x1_FFFF) as usize];
                self.wram_addr = (self.wram_addr + 1) & 0x1_FFFF;
                v
            }
            _ => self.open_bus,
        }
    }

    fn b_write(&mut self, low: u8, val: u8) {
        match low {
            0x00..=0x3F => self.ppu.write_reg(0x2100 | u16::from(low), val),
            // $2140-$2143 — deposit into the CPU→SMP latch the SMP's IPL/program reads at $F4-$F7.
            0x40..=0x43 => self.apu.cpu_write_port(low & 3, val),
            0x80 => {
                self.wram[(self.wram_addr & 0x1_FFFF) as usize] = val;
                self.wram_addr = (self.wram_addr + 1) & 0x1_FFFF;
            }
            0x81 => self.wram_addr = (self.wram_addr & 0x1_FF00) | u32::from(val),
            0x82 => self.wram_addr = (self.wram_addr & 0x1_00FF) | (u32::from(val) << 8),
            0x83 => self.wram_addr = (self.wram_addr & 0x0_FFFF) | (u32::from(val & 1) << 16),
            _ => {}
        }
    }

    // --- CPU registers ($4016/$4017 + $4200-$421F). ---------------------------------------

    fn read_cpu_reg(&mut self, addr: u16) -> u8 {
        match addr {
            0x4016 => {
                let bit = ((self.joypad[0] & 0x8000) >> 15) as u8;
                self.joypad[0] = (self.joypad[0] << 1) | 1;
                (self.open_bus & 0xFC) | bit
            }
            0x4017 => {
                let bit = ((self.joypad[1] & 0x8000) >> 15) as u8;
                self.joypad[1] = (self.joypad[1] << 1) | 1;
                (self.open_bus & 0xE0) | 0x1C | bit
            }
            0x4210 => {
                // RDNMI: bit7 = VBlank-occurred flag (read clears), bits0-3 = CPU version (2).
                let v = (u8::from(self.clock.rdnmi_flag) << 7) | 0x02;
                self.clock.rdnmi_flag = false;
                v
            }
            0x4211 => {
                // TIMEUP: bit7 = irq flag (read clears).
                let v = u8::from(self.clock.irq_line) << 7;
                self.clock.irq_line = false;
                v
            }
            0x4212 => {
                // HVBJOY: bit7 vblank, bit6 hblank.
                (u8::from(self.ppu.in_vblank()) << 7) | (u8::from(self.ppu.in_hblank()) << 6)
            }
            0x4214 => self.muldiv.rddiv as u8,
            0x4215 => (self.muldiv.rddiv >> 8) as u8,
            0x4216 => self.muldiv.rdmpy as u8,
            0x4217 => (self.muldiv.rdmpy >> 8) as u8,
            0x4218..=0x421F => {
                // Auto-joypad read result: $4218/9 = pad1, $421A/B = pad2.
                let pad = usize::from(addr >= 0x421A);
                if addr & 1 == 0 {
                    self.joypad[pad] as u8
                } else {
                    (self.joypad[pad] >> 8) as u8
                }
            }
            _ => self.open_bus,
        }
    }

    fn write_cpu_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0x4016 => self.joypad_strobe = val & 1 != 0,
            0x4200 => self.clock.nmitimen = val,
            0x4202 => self.muldiv.mpya = val,
            0x4203 => self.muldiv.rdmpy = u16::from(self.muldiv.mpya) * u16::from(val),
            0x4204 => self.muldiv.dividend = (self.muldiv.dividend & 0xFF00) | u16::from(val),
            0x4205 => {
                self.muldiv.dividend = (self.muldiv.dividend & 0x00FF) | (u16::from(val) << 8);
            }
            0x4206 => {
                if val == 0 {
                    self.muldiv.rddiv = 0xFFFF;
                    self.muldiv.rdmpy = self.muldiv.dividend;
                } else {
                    self.muldiv.rddiv = self.muldiv.dividend / u16::from(val);
                    self.muldiv.rdmpy = self.muldiv.dividend % u16::from(val);
                }
            }
            0x4207 => self.clock.htime = (self.clock.htime & 0x0100) | u16::from(val),
            0x4208 => self.clock.htime = (self.clock.htime & 0x00FF) | (u16::from(val & 1) << 8),
            0x4209 => self.clock.vtime = (self.clock.vtime & 0x0100) | u16::from(val),
            0x420A => self.clock.vtime = (self.clock.vtime & 0x00FF) | (u16::from(val & 1) << 8),
            0x420B => self.run_gp_dma(val),
            0x420C => self.dma.hdma_enable = val,
            0x420D => self.clock.fast_rom = val & 1 != 0,
            _ => {}
        }
    }

    /// Run GP-DMA to completion (CPU halted), advancing the master clock by the transfer cost.
    fn run_gp_dma(&mut self, mask: u8) {
        let mut dma = core::mem::take(&mut self.dma);
        // `run_gp` advances the master clock itself, byte-by-byte, via `DmaBus::step` (so the PPU
        // scanline stays current and V-blank-crossing VRAM writes actually land). Do NOT charge
        // the returned cost again here — that would double the DMA's wall-time.
        let _cost = dma.run_gp(mask, self);
        self.dma = dma;
    }

    // --- The 24-bit memory decode. ---------------------------------------------------------

    fn decode_read(&mut self, addr24: u32) -> u8 {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = (addr24 & 0xFFFF) as u16;
        match bank {
            0x7E..=0x7F => self.wram[(addr24 & 0x1_FFFF) as usize],
            0x00..=0x3F | 0x80..=0xBF => match addr {
                0x0000..=0x1FFF => self.wram[(addr & 0x1FFF) as usize],
                0x2100..=0x21FF => self.b_read(addr as u8),
                0x4016 | 0x4017 | 0x4200..=0x421F => self.read_cpu_reg(addr),
                0x4300..=0x437F => self
                    .dma
                    .read_reg(((addr >> 4) & 0xF) as usize, (addr & 0xF) as u8),
                _ => self.cart_read_raw(addr24),
            },
            _ => self.cart_read_raw(addr24),
        }
    }

    fn decode_write(&mut self, addr24: u32, val: u8) {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = (addr24 & 0xFFFF) as u16;
        match bank {
            0x7E..=0x7F => self.wram[(addr24 & 0x1_FFFF) as usize] = val,
            0x00..=0x3F | 0x80..=0xBF => match addr {
                0x0000..=0x1FFF => self.wram[(addr & 0x1FFF) as usize] = val,
                0x2100..=0x21FF => self.b_write(addr as u8, val),
                0x4016 | 0x4200..=0x421F => self.write_cpu_reg(addr, val),
                0x4300..=0x437F => {
                    let ch = ((addr >> 4) & 0xF) as usize;
                    let reg = (addr & 0xF) as u8;
                    self.dma.write_reg(ch, reg, val);
                    // S-DD1's DMA-address/size snoop (Board::notify_dma_channel doc) — only the
                    // registers that hold that state are worth reporting on.
                    if matches!(reg, 2..=6)
                        && let Some(c) = self.dma.channels.get(ch & 7)
                    {
                        let address = (u32::from(c.source_bank) << 16) | u32::from(c.source_addr);
                        if let Some(cart) = self.cart.as_mut() {
                            cart.board
                                .notify_dma_channel(ch & 7, address, c.count_or_indirect);
                        }
                    }
                }
                _ => self.cart_write_raw(addr24, val),
            },
            _ => self.cart_write_raw(addr24, val),
        }
    }

    fn cart_read_raw(&mut self, addr24: u32) -> u8 {
        let open_bus = self.open_bus;
        self.cart
            .as_mut()
            .map_or(open_bus, |c| c.read24(addr24, open_bus))
    }

    fn cart_write_raw(&mut self, addr24: u32, val: u8) {
        if let Some(c) = self.cart.as_mut() {
            // Arms a host-synced coprocessor (Super FX/GSU) if this write set Go — it does not
            // run it. `advance_master`'s per-tick loop drives it forward one master clock at a
            // time via `Board::coprocessor_tick`, genuinely concurrently with the CPU's own
            // subsequent instructions (`Board::coprocessor_tick` doc has the detail).
            c.write24(addr24, val);
        }
    }

    /// The access speed (master clocks) for a 24-bit CPU access. Ported from ares `CPU::wait`.
    const fn access_speed(&self, addr24: u32) -> u32 {
        // $00-3F/$80-BF:8000-FFFF and $40-7F/$C0-FF:0000-FFFF (ROM region).
        if addr24 & 0x40_8000 != 0 {
            return if addr24 & 0x80_0000 != 0 {
                if self.clock.fast_rom { 6 } else { 8 }
            } else {
                8
            };
        }
        // $00-3F/$80-BF:0000-1FFF (WRAM mirror) and :6000-7FFF (expansion).
        if addr24.wrapping_add(0x6000) & 0x4000 != 0 {
            return 8;
        }
        // $00-3F/$80-BF:2000-3FFF (PPU/APU) and :4200-5FFF (CPU/DMA regs).
        if addr24.wrapping_sub(0x4000) & 0x7E00 != 0 {
            return 6;
        }
        // $00-3F/$80-BF:4000-41FF (joypad serial).
        12
    }

    /// Write the PPU's own section, the APU's own section, the DMA controller's own section,
    /// then a `"BUS0"` section for WRAM + the Bus's own timing/register state, then (if a cart is
    /// loaded) its battery SRAM + coprocessor state as a final untagged tail (a presence flag,
    /// the length-prefixed SRAM bytes, then the board's own `save_state` bytes — the cart has no
    /// single section of its own since its payload is really "however many bytes the board's own
    /// implementation writes"). The cart's ROM/header are NOT written: the caller must reload the
    /// same ROM (`Cart::load`) and install it before calling [`Bus::load_state`], the same "never
    /// embed a ROM byte" contract every coprocessor board in `rustysnes-cart` already follows.
    pub fn save_state(&self, w: &mut SaveWriter) {
        self.ppu.save_state(w);
        self.apu.save_state(w);
        self.dma.save_state(w);
        w.section(*b"BUS0", |s| {
            self.clock.save_state(s);
            self.muldiv.save_state(s);
            s.write_bytes(&*self.wram);
            s.write_u32(self.wram_addr);
            s.write_u16(self.joypad[0]);
            s.write_u16(self.joypad[1]);
            s.write_bool(self.joypad_strobe);
            s.write_u8(self.open_bus);
            s.write_u16(self.last_hdma_line);
            s.write_bool(self.in_hdma);
            s.write_bool(self.hdma_setup_done);
        });
        match &self.cart {
            Some(cart) => {
                w.write_bool(true);
                w.write_len_prefixed(cart.board.sram());
                cart.board.save_state(w);
            }
            None => w.write_bool(false),
        }
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input, a section with unconsumed trailing bytes,
    /// or [`SaveStateError::Invalid`] if the save-state's cart presence doesn't match this
    /// `Bus`'s own (a save-state taken with a cart loaded can only be restored onto a `Bus` that
    /// already has the SAME cart's ROM loaded — via [`rustysnes_cart::Cart::load`] — installed
    /// first; there is no ROM byte in the save-state to reconstruct it from) or if a restored
    /// SRAM image's length doesn't match the installed cart's own SRAM size (a mismatched ROM).
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        self.ppu.load_state(r)?;
        self.apu.load_state(r)?;
        self.dma.load_state(r)?;
        let mut s = r.expect_section(*b"BUS0")?;
        self.clock.load_state(&mut s)?;
        self.muldiv.load_state(&mut s)?;
        self.wram.copy_from_slice(s.read_bytes(WRAM_SIZE)?);
        // wram_addr is a 17-bit register (every use site already masks it & 0x1_FFFF).
        self.wram_addr = s.read_u32()? & 0x1_FFFF;
        self.joypad[0] = s.read_u16()?;
        self.joypad[1] = s.read_u16()?;
        self.joypad_strobe = s.read_bool()?;
        self.open_bus = s.read_u8()?;
        self.last_hdma_line = s.read_u16()?;
        self.in_hdma = s.read_bool()?;
        self.hdma_setup_done = s.read_bool()?;
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "BUS0 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        let had_cart = r.read_bool()?;
        match (&mut self.cart, had_cart) {
            (Some(cart), true) => {
                let sram = r.read_len_prefixed()?;
                if sram.len() != cart.board.sram().len() {
                    return Err(SaveStateError::Invalid(alloc::format!(
                        "save-state SRAM length {} does not match the installed cart's {} \
                         (wrong ROM loaded before restoring?)",
                        sram.len(),
                        cart.board.sram().len()
                    )));
                }
                cart.board.sram_mut().copy_from_slice(sram);
                cart.board.load_state(r)?;
            }
            (None, false) => {}
            (Some(_), false) | (None, true) => {
                return Err(SaveStateError::Invalid(alloc::string::String::from(
                    "save-state cart presence does not match this Bus's installed cart \
                     (load the same ROM before restoring, or restore onto a fresh Bus)",
                )));
            }
        }
        Ok(())
    }
}

/// A cart-only view of the Bus for the PPU's `tick_dot` (split borrow: the PPU may need a
/// cart-mediated read for Mode 7 / coprocessor boards without aliasing the whole Bus).
struct CartView<'a> {
    cart: &'a mut Option<Cart>,
    open: u8,
}

impl VideoBus for CartView<'_> {
    fn cart_read(&mut self, addr24: u32) -> u8 {
        let open = self.open;
        self.cart.as_mut().map_or(open, |c| c.read24(addr24, open))
    }
}

/// The DMA controller's view: A-bus (24-bit) via the decode, B-bus via `b_read`/`b_write`.
impl DmaBus for Bus {
    fn read_a(&mut self, addr: u32) -> u8 {
        // The A-bus cannot reach the B-bus or the CPU/DMA I/O registers (ares `validA`).
        let bank = (addr >> 16) & 0xFF;
        let off = addr & 0xFFFF;
        if matches!(bank, 0x00..=0x3F | 0x80..=0xBF)
            && matches!(off, 0x2100..=0x21FF | 0x4000..=0x43FF)
        {
            return self.open_bus;
        }
        self.decode_read(addr)
    }
    fn write_a(&mut self, addr: u32, val: u8) {
        let bank = (addr >> 16) & 0xFF;
        let off = addr & 0xFFFF;
        if matches!(bank, 0x00..=0x3F | 0x80..=0xBF)
            && matches!(off, 0x2100..=0x21FF | 0x4000..=0x43FF)
        {
            return;
        }
        self.decode_write(addr, val);
    }
    fn read_b(&mut self, addr: u8) -> u8 {
        self.b_read(addr)
    }
    fn write_b(&mut self, addr: u8, val: u8) {
        self.b_write(addr, val);
    }
    fn step(&mut self, clocks: u32) {
        // Advance the whole system (PPU dot clock, SPC, host-synced coprocessor) mid-DMA so the
        // scanline that gates VRAM/CGRAM/OAM access is current at each transferred byte.
        self.advance_master(clocks);
    }
    fn scanline(&self) -> u16 {
        self.ppu.scanline()
    }
    fn visible_height(&self) -> u16 {
        self.ppu.visible_height()
    }
    fn hdma_last_line(&self) -> u16 {
        self.last_hdma_line
    }
    fn set_hdma_last_line(&mut self, line: u16) {
        self.last_hdma_line = line;
    }
}

/// The 65C816's view: route a 24-bit access + drive the master clock in lockstep.
impl CpuBus for Bus {
    // `decode_read` must always run first for its side effects (e.g. an NMI-flag-clear-on-read
    // register) even when a cheat overrides the value the CPU observes — so this can't be
    // rephrased as a plain `if/else` expression the way clippy suggests.
    #[allow(clippy::useless_let_if_seq)]
    fn read24(&mut self, addr24: u32) -> u8 {
        let mut val = self.decode_read(addr24);
        // Cheat-code intercept (`v0.8.0`, T-81-003) — `self.cheats` is empty in every build that
        // never calls `set_cheats`, so this costs one branch when inactive. See `set_cheats`'s
        // doc for why this is a read intercept rather than a WRAM poke.
        if !self.cheats.is_empty()
            && let Some(patch) = self.cheats.iter().find(|p| p.address == addr24)
        {
            val = patch.value;
        }
        self.open_bus = val;
        // `v0.8.0`, T-81-001b: logs the value actually observed (post-cheat-intercept), matching
        // what the CPU itself sees. Compiled out entirely when `debug-hooks` is off.
        #[cfg(feature = "debug-hooks")]
        self.watchpoints.check(addr24, val, false, self.debug_pc);
        val
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        self.open_bus = val;
        #[cfg(feature = "debug-hooks")]
        self.watchpoints.check(addr24, val, true, self.debug_pc);
        self.decode_write(addr24, val);
    }

    fn access_cycles(&self, addr24: u32) -> u32 {
        self.access_speed(addr24)
    }

    fn advance(&mut self, clocks: u32) {
        // ares `CPU::step`: tick the PPU dot clock, SPC, host-synced coprocessor, and HDMA in
        // lockstep. The CPU sequences its calls to this around each access (see `CpuBus`) so a
        // register write lands at the hardware-exact hcounter.
        self.advance_master(clocks);
    }

    fn poll_nmi(&mut self) -> bool {
        core::mem::take(&mut self.clock.nmi_line)
    }

    fn poll_irq(&mut self) -> bool {
        // OR the PPU/APU HV-IRQ level with any on-cart coprocessor IRQ (SA-1 → S-CPU, SPC7110 RTC,
        // …). The `Board::irq_pending` hook is documented to be ORed here; base/host-sync boards
        // return `false` so non-coprocessor carts are unaffected.
        self.clock.irq_line || self.cart.as_ref().is_some_and(|c| c.board.irq_pending())
    }
}

impl core::fmt::Debug for Bus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Bus")
            .field("cart", &self.cart.as_ref().map(|c| c.board.name()))
            .field("master", &self.clock.master)
            .field("open_bus", &self.open_bus)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `HDMA_RUN_DOT` is now literally `= rustysnes_ppu::RENDER_DOT`, so this can never actually
    /// fail post-refactor -- kept as a named regression lock so a future edit that reintroduces a
    /// separate literal (e.g. during a merge) fails loudly instead of silently drifting the two
    /// dot values apart again (`docs/ppu.md` §Mid-scanline/HDMA-driven register timing).
    #[test]
    fn hdma_run_dot_matches_ppu_render_dot() {
        assert_eq!(HDMA_RUN_DOT, rustysnes_ppu::RENDER_DOT);
    }

    #[test]
    fn default_bus_has_no_cart_and_reads_open() {
        let mut bus = Bus::default();
        assert!(bus.cart.is_none());
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_8000), 0);
    }

    #[test]
    fn wram_round_trips() {
        let mut bus = Bus::default();
        <Bus as CpuBus>::write24(&mut bus, 0x7E_1234, 0xAB);
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x7E_1234), 0xAB);
        // Low mirror in bank 0 aliases the same WRAM.
        <Bus as CpuBus>::write24(&mut bus, 0x00_0042, 0x99);
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x7E_0042), 0x99);
    }

    #[test]
    fn access_speed_map() {
        let bus = Bus::default();
        assert_eq!(bus.access_speed(0x00_0042), 8); // WRAM mirror
        assert_eq!(bus.access_speed(0x00_2100), 6); // PPU
        assert_eq!(bus.access_speed(0x00_4016), 12); // joypad
        assert_eq!(bus.access_speed(0x00_4200), 6); // CPU regs
        assert_eq!(bus.access_speed(0x00_8000), 8); // WS1 ROM (always 8)
        assert_eq!(bus.access_speed(0x80_8000), 8); // WS2 ROM, SlowROM default
    }

    #[test]
    fn memsel_fastrom_speeds_up_ws2() {
        let mut bus = Bus::default();
        <Bus as CpuBus>::write24(&mut bus, 0x00_420D, 0x01); // MEMSEL FastROM
        assert_eq!(bus.access_speed(0x80_8000), 6);
        assert_eq!(bus.access_speed(0x00_8000), 8); // WS1 unaffected
    }

    #[test]
    fn muldiv_unit() {
        let mut bus = Bus::default();
        <Bus as CpuBus>::write24(&mut bus, 0x00_4202, 0x10); // MPYA
        <Bus as CpuBus>::write24(&mut bus, 0x00_4203, 0x10); // MPYB -> 0x100
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4216), 0x00);
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4217), 0x01);
        <Bus as CpuBus>::write24(&mut bus, 0x00_4204, 0x64); // dividend lo = 100
        <Bus as CpuBus>::write24(&mut bus, 0x00_4205, 0x00);
        <Bus as CpuBus>::write24(&mut bus, 0x00_4206, 0x07); // / 7 -> 14 r 2
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4214), 14);
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4216), 2);
    }

    /// `$4202` (MPYA) is a stable latch, not re-armed per multiply — real hardware documents
    /// that a fresh `$4203` (WRMPYB) write alone starts a new multiply against whatever MPYA
    /// already holds (`SNESdev`'s Multiplication page). The genuinely undefined case (starting a
    /// new multiply/divide before the previous one's 8-cycle latency elapses, `SNESdev`'s Errata
    /// page) is deliberately NOT covered here — see `MulDiv`'s own doc comment for why there is
    /// no correct value to assert against.
    #[test]
    fn muldiv_mpya_latch_survives_across_sequential_multiplies() {
        let mut bus = Bus::default();
        <Bus as CpuBus>::write24(&mut bus, 0x00_4202, 0x05); // MPYA = 5
        <Bus as CpuBus>::write24(&mut bus, 0x00_4203, 0x06); // MPYB -> 5*6 = 30
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4216), 30);
        // MPYA is untouched by the write above; a fresh $4203 alone starts another multiply
        // against the SAME latched 5.
        <Bus as CpuBus>::write24(&mut bus, 0x00_4203, 0x07); // MPYB -> 5*7 = 35
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4216), 35);
    }

    #[test]
    fn master_clock_advances_on_access() {
        let mut bus = Bus::default();
        let before = bus.clock.master;
        // SlowROM ($00:8000) costs 8 master clocks; `advance` is what moves the clock.
        let speed = <Bus as CpuBus>::access_cycles(&bus, 0x00_8000);
        assert_eq!(speed, 8);
        <Bus as CpuBus>::advance(&mut bus, speed);
        assert_eq!(bus.clock.master, before + 8);
        // `read24`/`write24` are pure accesses now — they do not move the clock.
        <Bus as CpuBus>::read24(&mut bus, 0x00_8000);
        assert_eq!(bus.clock.master, before + 8);
    }
}
