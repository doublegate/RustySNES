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

use crate::controller::{PortDevice, PortState};
use crate::dma::Dma;
use crate::dma_bus::DmaBus;

/// WRAM size — the SNES has 128 KiB of work RAM (`$7E0000-$7FFFFF`).
const WRAM_SIZE: usize = 128 * 1024;
/// Master clocks per PPU dot, for every dot except the two long ones.
const MASTER_PER_DOT: u32 = 4;

/// Duration of an automatic joypad read, in master clocks. ares steps `status.autoJoypadCounter`
/// 0 -> 33 once every 128 master clocks (`joypadCounter() = counter.cpu & 127`), so the read is busy
/// for 33 * 128 = 4224 master clocks (~3 scanlines) from vblank entry; `$4212` bit 0 reads 1 and the
/// result is not yet published for that whole window (`sfc/cpu/timing.cpp` `joypadEdge`).
const AUTO_JOYPAD_CLOCKS: u64 = 33 * 128;

/// The two dots that take **6** master clocks instead of 4 (`T-06-A`).
///
/// Hardware's scanline is 340 dots and 1364 master clocks, which `338 × 4 + 2 × 6` satisfies and a
/// uniform `341 × 4` also satisfies — which is why a model can be wrong here and keep perfect frame
/// timing. fullsnes' *PPU H-Counter-Latch Quantities* histogram settles it by measurement rather
/// than prose: sampling `$2137` once per master clock across a line reports dots 323 and 327
/// latching **six** times each, dot 340 **never**, and everything else four. bsnes, ares and Mesen2
/// all implement exactly this; snes9x uses 322/326 and is the outlier.
///
/// Both sit deep in hblank — past the visible window (dots 22-277), past hblank's start at 274, and
/// past [`HDMA_RUN_DOT`] — so dots `0..=322` keep their previous clock alignment exactly and no
/// rendered pixel or HDMA transfer moves. What does change is the `OPHCT`/`$213C` latch value for
/// `H >= 323`, which was up to one whole dot early.
const LONG_DOTS: [u16; 2] = [323, 327];

/// Master clocks the dot currently being completed lasts for.
const fn dot_length(dot: u16) -> u32 {
    if dot == LONG_DOTS[0] || dot == LONG_DOTS[1] {
        MASTER_PER_DOT + 2
    } else {
        MASTER_PER_DOT
    }
}
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
#[derive(Debug, Clone)]
struct MulDiv {
    mpya: u8,
    dividend: u16,
    rddiv: u16,
    rdmpy: u16,
}

impl Default for MulDiv {
    /// Power-on state: `WRMPYA` = `$FF`, `WRDIV` = `$FFFF`, results zeroed.
    ///
    /// These registers are write-only, so the values are not readable directly — but they are real
    /// latches feeding the ALU, and the ALU output is readable, so the state is observable by
    /// starting an operation without writing its first operand: `$4203 = 2` with `$4202` untouched
    /// yields `$01FE`. AccuracySNES `B5.05` probes exactly that.
    ///
    /// Provenance, recorded because this is asserted rather than merely recorded: anomie's
    /// `regs.txt` (r1157) states *"$4202 holds the value $ff on power on and is unchanged on
    /// reset"* and *"WRDIV holds the value $ffff on power on and is unchanged on reset"* — in a
    /// document that explicitly marks its uncertain claims with `(?)` and marks neither of these.
    /// nocash's fullsnes independently lists `$4202`-`$4206` as `(FFh)` power-up under a legend
    /// distinguishing power-up from reset. bsnes (`sfc/cpu/cpu.hpp`), ares and Mesen2
    /// (`AluMulDiv::Initialize`) all implement it. **snes9x does not** — it blanket-`memset`s
    /// `$4200-$42FF` to zero — which is a snes9x bug, not counter-evidence.
    ///
    /// No hardware test ROM is known to verify this; do not claim ROM-verified provenance for it.
    fn default() -> Self {
        Self {
            mpya: 0xFF,
            dividend: 0xFFFF,
            rddiv: 0,
            rdmpy: 0,
        }
    }
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
    /// The buttons currently held, per player — what the frontend last set, and what
    /// `$4218-$421F` reports. **Not** the shift register: a manual read must not destroy it, and
    /// the strobe reloads from it.
    joypad: [u16; 2],
    /// The manual-read shift registers behind `$4016`/`$4017`, reloaded from [`Self::joypad`]
    /// while the strobe is high.
    ///
    /// Separate from the buttons because the pad is a *parallel-load* shift register: `$4016.0`
    /// high loads it from the button lines and low starts clocking, so a program may strobe and
    /// re-read as often as it likes within one frame and get the same answer each time. Sharing one
    /// register with the button state made the second read of a frame return all-ones, and made a
    /// manual read corrupt the auto-read result — both invisible to a frontend that rewrites the
    /// state every frame, and both found by AccuracySNES `F1.02`.
    joypad_shift: [u16; 2],
    /// The automatic-read *result* latched into `$4218`-`$421F`, which is a different thing from
    /// the live controller state in [`Self::joypad`].
    ///
    /// Hardware copies the ports into these registers once per frame, at the start of vblank, and
    /// **only when `$4200` bit 0 is set**. Reporting [`Self::joypad`] directly instead makes
    /// `$4218` track the pad continuously, so software that disarms auto-read to poll `$4016` by
    /// hand still sees the hardware's answer appear underneath it. AccuracySNES `F1.07` — which
    /// could not detect this until the battery gained a host input contract, because with nothing
    /// held both behaviours report `$0000`.
    joypad_auto: [u16; 2],
    /// The port snapshot taken at the START of a timed automatic read, held until the read completes
    /// and then committed to [`Self::joypad_auto`]. See [`Self::begin_auto_joypad`].
    joypad_auto_pending: [u16; 2],
    /// `clock.master` instant the in-flight automatic joypad read completes (0 = idle). While
    /// non-zero, `$4212` bit 0 reads busy and `$4218-$421F` still hold the previous result — the read
    /// publishes at completion, ~[`AUTO_JOYPAD_CLOCKS`] master clocks after vblank entry.
    auto_joypad_busy_until: u64,
    joypad_strobe: bool,
    /// Per-port peripheral state (`v0.9.0`, Phase 7 niche peripherals) — Mouse/Super Scope/Super
    /// Multitap. Idle (and touching nothing on `$4016`/`$4017`'s `data1` bit) unless a port's
    /// [`crate::controller::PortDevice`] is explicitly switched away from the default `Gamepad`
    /// via [`Self::set_port_device`], in which case `joypad[port]`'s own bit is bypassed instead
    /// of merged — see [`Self::port_clock`].
    ports: [PortState; 2],
    /// WRIO ($4201 write / $4213 read) — the programmable I/O port. Bit6 is controller port 1's
    /// IOBIT pin, bit7 port 2's (only port 2's is wired to the PPU H/V-counter latch on real
    /// hardware — a Super Scope's own beam-detection strobe). Reset value `0xFF` (ares
    /// `cpu.hpp`'s `n8 pio = 0xff`).
    pio: u8,
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
            joypad_shift: [0; 2],
            joypad_auto: [0; 2],
            joypad_auto_pending: [0; 2],
            auto_joypad_busy_until: 0,
            joypad_strobe: false,
            ports: [PortState::default(), PortState::default()],
            pio: 0xFF,
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

    /// Compute the automatic-read result from the current port state (does **not** publish it).
    ///
    /// A latch held high (`joypad_strobe`) reloads the shift registers every clock instead of
    /// shifting, so all sixteen bits read back as the first bit — `$FFFF`/`$0000` per the held bit
    /// (AccuracySNES `F1.11`). Otherwise the read returns the latched pad word.
    fn capture_auto_joypad(&self) -> [u16; 2] {
        if self.joypad_strobe {
            core::array::from_fn(|i| {
                if self.joypad[i] & 0x8000 == 0 {
                    0x0000
                } else {
                    0xFFFF
                }
            })
        } else {
            self.joypad
        }
    }

    /// Perform the automatic joypad read **immediately** (no busy window): latch into
    /// [`Self::joypad_auto`]. The scheduler uses the timed [`Self::begin_auto_joypad`] instead;
    /// this instant form is the unit-test helper and any legacy caller.
    #[cfg(test)]
    fn poll_auto_joypad(&mut self) {
        self.joypad_auto = self.capture_auto_joypad();
    }

    /// Begin a timed automatic joypad read at vblank (ares `status.autoJoypadCounter`, modelled as a
    /// master-clock deadline). The controller state is snapshotted **now** (ares latches at counter
    /// 0), but published to [`Self::joypad_auto`] only once the read completes ~[`AUTO_JOYPAD_CLOCKS`]
    /// master clocks later — so `$4218-$421F` read during the window still hold the *previous*
    /// frame's result and `$4212` bit 0 reads busy. Called at vblank entry while `$4200` bit 0 is set.
    fn begin_auto_joypad(&mut self) {
        self.settle_auto_joypad(); // finish any read still nominally in flight from last frame
        self.joypad_auto_pending = self.capture_auto_joypad();
        self.auto_joypad_busy_until = self.clock.master + AUTO_JOYPAD_CLOCKS;
    }

    /// Publish a completed automatic read: once `clock.master` reaches the deadline, commit the
    /// snapshot to [`Self::joypad_auto`] and clear the busy window. Idempotent and cheap; call before
    /// any observation of `$4212` bit 0 or `$4218-$421F`.
    const fn settle_auto_joypad(&mut self) {
        if self.auto_joypad_busy_until != 0 && self.clock.master >= self.auto_joypad_busy_until {
            self.joypad_auto = self.joypad_auto_pending;
            self.auto_joypad_busy_until = 0;
        }
    }

    /// [`Self::poll_auto_joypad`], reachable from the unit tests without running a whole frame.
    #[cfg(test)]
    fn poll_auto_joypad_for_test(&mut self) {
        self.poll_auto_joypad();
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

    /// Select which peripheral is connected to controller port `port` (`0` = port 1, `1` = port
    /// 2). Defaults to [`PortDevice::Gamepad`] on both ports (this project's original,
    /// unchanged behavior) until a frontend explicitly calls this — a host/session configuration
    /// choice, not emulated state (matching [`Self::set_cheats`]/`Self::set_watchpoints`'s own
    /// "re-established by the frontend, not carried in a save-state" posture — a real SNES has no
    /// memory of what was plugged in across a power cycle either).
    pub fn set_port_device(&mut self, port: usize, device: PortDevice) {
        if let Some(p) = self.ports.get_mut(port) {
            p.device = device;
        }
    }

    /// The peripheral currently connected to controller port `port` — for the debugger overlay
    /// and the frontend's own input-routing (`v0.9.0`).
    #[must_use]
    pub fn port_device(&self, port: usize) -> PortDevice {
        self.ports
            .get(port)
            .map_or(PortDevice::Gamepad, |p| p.device)
    }

    /// Feed one frame's worth of SNES Mouse input for port `port` (only meaningful when that
    /// port's device is [`PortDevice::Mouse`]). `dx`/`dy` are raw, unscaled host deltas since the
    /// last call — the SNES Mouse's own speed multiplier and 127-unit clamp are applied
    /// internally at the hardware-accurate point (latch time), matching real hardware. Same
    /// "always replace, re-synced once per frame" convention as [`Self::set_joypad`].
    pub fn set_mouse(&mut self, port: usize, dx: i32, dy: i32, left: bool, right: bool) {
        if let Some(p) = self.ports.get_mut(port) {
            p.mouse.set_input(dx, dy, left, right);
        }
    }

    /// Feed one frame's worth of Super Scope input for port `port` (only meaningful when that
    /// port's device is [`PortDevice::SuperScope`]). `x`/`y` are absolute screen coordinates in
    /// SNES pixel space (`0..256`, `0..240`-ish; a small negative/over-max margin is allowed and
    /// means "aimed off-screen", matching real hardware). `buttons` is a bitmask over
    /// [`crate::controller::scope`]'s `TRIGGER`/`CURSOR`/`TURBO`/`PAUSE` bits — the LIVE physical
    /// switch/button state; this project reproduces real hardware's own edge-detection internally
    /// (`crate::controller::SuperScopeState`), so the frontend should pass the raw host state,
    /// not a pre-toggled value. (A packed bitmask rather than one bool per button, matching
    /// [`Self::set_joypad`]'s own convention.)
    pub fn set_superscope(&mut self, port: usize, x: i32, y: i32, buttons: u8) {
        if let Some(p) = self.ports.get_mut(port) {
            p.super_scope.set_input(x, y, buttons);
        }
    }

    /// Feed one frame's worth of input for Super Multitap sub-pad `sub_index` (`0..=3`) of port
    /// `port` (only meaningful when that port's device is [`PortDevice::Multitap`]) — same 12-bit
    /// `BYsSUDLR....` format and per-frame convention as [`Self::set_joypad`].
    pub fn set_multitap_pad(&mut self, port: usize, sub_index: usize, buttons: u16) {
        if let Some(p) = self.ports.get_mut(port) {
            p.multitap.set_pad(sub_index, buttons);
        }
    }

    /// The current input state of Super Multitap sub-pad `sub_index` of port `port` — the read
    /// side of [`Self::set_multitap_pad`], for the debugger overlay and TAS movie recording.
    #[must_use]
    pub fn multitap_pad(&self, port: usize, sub_index: usize) -> u16 {
        self.ports
            .get(port)
            .map_or(0, |p| p.multitap.pad(sub_index))
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

    /// The full 128 KiB WRAM as a flat byte slice (linear address `0..0x1_FFFF`, the same mapping
    /// [`Self::peek_wram`]'s `0x7E..=0x7F` bank arm uses) — for a host embedder that needs a raw
    /// memory-map pointer (e.g. a libretro core's `RETRO_MEMORY_SYSTEM_RAM`).
    #[must_use]
    pub fn wram(&self) -> &[u8] {
        &*self.wram
    }

    /// The mutable counterpart to [`Self::wram`] — same host-embedder use case (a libretro
    /// frontend's memory-map API hands this pointer to RetroAchievements/cheat tooling that
    /// writes through it directly).
    pub fn wram_mut(&mut self) -> &mut [u8] {
        &mut *self.wram
    }

    /// Non-intrusive read of an arbitrary 24-bit CPU address, for the debugger overlay's
    /// disassembly view (`v0.9.0`, T-81-001 PR B). Unlike [`CpuBus::read24`], this does NOT touch
    /// the open-bus latch, does NOT check watchpoints, and does NOT trigger any I/O register's own
    /// read side effect (VRAM/CGRAM auto-increment, NMI-flag-clear-on-read, the H/V-counter
    /// latch, …) — genuinely just peeking. Real 65C816 code only ever executes from WRAM or cart
    /// ROM/RAM space, so (mirroring [`Self::peek_wram`]'s own "not for register space" posture)
    /// this only special-cases those two regions; any other address returns `0` rather than
    /// reaching into a register's live side effects, which is fine since real code never lives
    /// there anyway. The cart-space branch still calls into the board (some coprocessors gate
    /// their own ROM/RAM reads on internal state), but passes a neutral `0` open-bus fallback
    /// rather than the Bus's real, live latch — this peek must never read *or* write that shared
    /// state.
    pub fn peek(&mut self, addr24: u32) -> u8 {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = (addr24 & 0xFFFF) as u16;
        match bank {
            0x7E..=0x7F => self.wram[(addr24 & 0x1_FFFF) as usize],
            0x00..=0x3F | 0x80..=0xBF if addr < 0x2000 => self.wram[(addr & 0x1FFF) as usize],
            0x00..=0x3F | 0x80..=0xBF if addr < 0x8000 => 0, // I/O register space; not real code.
            _ => self.cart.as_mut().map_or(0, |c| c.read24(addr24, 0)),
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

    /// Set the 8 per-voice audio mute toggles (`v1.0.1`). See
    /// [`rustysnes_apu::dsp::Dsp::set_voice_mutes`]'s doc for why this is a frontend/debug
    /// convenience re-synced once per real frame, not real S-DSP hardware state.
    pub const fn set_voice_mutes(&mut self, mutes: [bool; 8]) {
        self.apu.set_voice_mutes(mutes);
    }

    /// Record the CPU's current `PBR:PC` (24-bit, `$bank:offset`) so a watchpoint hit during the
    /// access this instruction is about to make can attribute itself to the right instruction.
    /// The scheduler calls this once before each [`rustysnes_cpu::Cpu::step`]
    /// ([`crate::scheduler::System::run_frame`]/[`crate::scheduler::System::step_instruction`]).
    #[cfg(feature = "debug-hooks")]
    pub const fn set_debug_pc(&mut self, pbr_pc: u32) {
        self.debug_pc = pbr_pc;
    }

    /// Check a bus access against the armed watchpoint list, tagged with the CPU's `PBR:PC` at
    /// the moment of the access. Shared by [`CpuBus::read24`]/[`write24`](CpuBus::write24) *and*
    /// [`DmaBus`]'s A-bus/B-bus methods (`v1.1.0`) — DMA/HDMA-driven accesses were previously
    /// invisible to watchpoints entirely, which blocked tracing the open-bus-via-DMA-latch
    /// investigation (`docs/scheduler.md` §Open bus via DMA/HDMA); `debug_pc` still reflects the
    /// CPU instruction that initiated the transfer, since nothing updates it mid-DMA.
    #[cfg(feature = "debug-hooks")]
    fn note_bus_access(&mut self, addr24: u32, value: u8, is_write: bool) {
        let pc = self.debug_pc;
        self.watchpoints.check(addr24, value, is_write, pc);
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
            // The threshold is the length of the dot being *completed*, which is why it is taken
            // from `pre_tick_dot` rather than read back after the tick.
            let this_dot = dot_length(pre_tick_dot);
            let dot_ticked = if self.clock.dot_accum >= this_dot {
                self.clock.dot_accum -= this_dot;
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
            // Super Scope beam-position auto-latch (`v0.9.0`) — gated to the one sub-tick that
            // actually advanced the dot, same granularity `dot_ticked` already gives the HDMA
            // check above; a no-op unless port 2 has a Super Scope attached (`Self`'s own doc).
            if dot_ticked {
                self.check_superscope_beam();
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
            // The automatic joypad read, which runs at the start of vblank and only while armed.
            // Sharing this hook with the NMI is exact enough for everything the cart can observe:
            // hardware starts the read a few dozen cycles into the first vblank line and takes
            // about three scanlines over it, and `F1.08`-`F1.10` — the rows about *when* it runs
            // and the race that window creates — are not covered by this battery.
            if self.clock.nmitimen & 0x01 != 0 {
                // The read clocks the ports' shift registers, so it is subject to the latch line.
                // While `$4016` bit 0 is held high the registers reload continuously instead of
                // shifting, and all sixteen clocks return the first bit — the result is uniform,
                // not merely stale. Software that hand-polls `$4016` must disarm auto-read or keep
                // its strobing out of vblank. AccuracySNES `F1.11`; Mesen2 models this, snes9x
                // does not.
                self.begin_auto_joypad();
            }
        }
        // Complete a timed automatic read when its deadline passes, so `$4212` bit 0 and the
        // `$4218-$421F` result reflect the true ~4224-clock window even without an intervening
        // register read. The in-flight snapshot + deadline are serialized (`FORMAT_VERSION` 5), so a
        // save mid-window restores identically.
        self.settle_auto_joypad();
        // RDNMI's VBlank flag is cleared by a read *and*, independently, at the end of VBlank.
        // Modelling only the read left it set through the whole active display, so code that
        // polls $4210 outside VBlank saw a VBlank that had already ended and acted a frame late.
        // Stateless because the flag can only ever be raised during VBlank. AccuracySNES B4.05.
        if !self.ppu.in_vblank() {
            self.clock.rdnmi_flag = false;
        }
        if self.ppu.irq_pending() {
            self.ppu.ack_irq();
            self.clock.irq_line = true;
        }
    }

    // --- B-bus ($2100-$21FF) register access (PPU, APU ports, WRAM port). ------------------

    fn b_read(&mut self, low: u8) -> u8 {
        match low {
            // $2137 (SLHV) is a *software* latch of the H/V counters, and it is gated by the same
            // pin the light gun uses: `$4201` bit 7 drives port 2's IOBIT, and the counter latch
            // only responds while that bit is set. superfamicom.org's register reference is
            // explicit — reading $2137 latches "if bit 7 of $4201 is set", and "when bit a is 0,
            // no latching can occur". `Self::set_pio` already models the falling-edge latch on the
            // same pin; this is the other half of that wiring, and it lives here rather than in
            // the PPU because the Bus is what owns the pin.
            //
            // The read itself still happens: $2137 carries no data of its own and returns PPU1
            // open bus either way, so only the side effect is suppressed. Found by AccuracySNES
            // C3.10; snes9x and Mesen2 both gate it and RustySNES did not.
            0x37 if self.pio & 0x80 == 0 => self.ppu.ppu1_open_bus(),
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
        // Publish a completed automatic joypad read (and clear its busy window) before observing
        // `$4212` bit 0 or the `$4218-$421F` result, so a read at any dot sees the exact state.
        self.settle_auto_joypad();
        match addr {
            0x4016 => {
                let (d1, d2) = self.port_clock(0);
                (self.open_bus & 0xFC) | (d2 << 1) | d1
            }
            0x4017 => {
                let (d1, d2) = self.port_clock(1);
                (self.open_bus & 0xE0) | 0x1C | (d2 << 1) | d1
            }
            0x4213 => {
                // RDIO — WRIO ($4201) read back verbatim (ares `cpu.io.pio`).
                self.pio
            }
            0x4210 => {
                // RDNMI: bit7 = VBlank-occurred flag (read clears), bits4-6 = open bus (the MDR,
                // held in `self.open_bus` — the pre-read last-driven value), bits0-3 = CPU version
                // (2). ares `CPU::readIO` $4210 leaves bits 4-6 as the incoming data (open bus) and
                // only writes `io.version` into bits 0-3 and the flag into bit 7.
                let v = (u8::from(self.clock.rdnmi_flag) << 7) | (self.open_bus & 0x70) | 0x02;
                self.clock.rdnmi_flag = false;
                v
            }
            0x4211 => {
                // TIMEUP: bit7 = irq flag (read clears), bits0-6 = open bus (MDR). ares `readIO`
                // $4211 writes only bit 7 and leaves the rest as the incoming open-bus data.
                let v = (u8::from(self.clock.irq_line) << 7) | (self.open_bus & 0x7F);
                self.clock.irq_line = false;
                v
            }
            0x4212 => {
                // HVBJOY: bit7 vblank, bit6 hblank, bits1-5 open bus, bit0 auto-joypad busy.
                // Busy while auto-read is armed (`$4200` bit 0) AND the timed read has not yet
                // completed (the `settle_auto_joypad` above zeroed the deadline if it passed) —
                // ares `io.autoJoypadPoll && status.autoJoypadCounter < 33`.
                let busy =
                    u8::from(self.clock.nmitimen & 0x01 != 0 && self.auto_joypad_busy_until != 0);
                (u8::from(self.ppu.in_vblank()) << 7)
                    | (u8::from(self.ppu.in_hblank()) << 6)
                    | (self.open_bus & 0x3E)
                    | busy
            }
            0x4214 => self.muldiv.rddiv as u8,
            0x4215 => (self.muldiv.rddiv >> 8) as u8,
            0x4216 => self.muldiv.rdmpy as u8,
            0x4217 => (self.muldiv.rdmpy >> 8) as u8,
            0x4218..=0x421F => {
                // Auto-joypad read result: $4218/9 = pad1, $421A/B = pad2.
                let pad = usize::from(addr >= 0x421A);
                if addr & 1 == 0 {
                    self.joypad_auto[pad] as u8
                } else {
                    (self.joypad_auto[pad] >> 8) as u8
                }
            }
            _ => self.open_bus,
        }
    }

    fn write_cpu_reg(&mut self, addr: u16, val: u8) {
        match addr {
            0x4016 => {
                // The one physical strobe line is wired to BOTH controller ports simultaneously
                // (`rustysnes_core::controller`'s module doc) — `Gamepad` ignores it exactly as
                // before (no functional change to the default path); the other peripherals latch.
                let strobe = val & 1 != 0;
                // A parallel load, not an edge: while the strobe is high the shift registers track
                // the button lines, and the falling edge simply stops them tracking. So the reload
                // happens on the way down as well as while high — otherwise a program that raises
                // the strobe, changes nothing, and lowers it would freeze whatever the buttons were
                // at the *rising* edge rather than at the falling one. Reloading here is what lets
                // a program strobe twice in one frame and read the same buttons twice.
                if strobe || self.joypad_strobe {
                    self.joypad_shift = self.joypad;
                }
                self.joypad_strobe = strobe;
                self.ports[0].latch(strobe);
                self.ports[1].latch(strobe);
            }
            0x4201 => self.set_pio(val),
            0x4200 => {
                let was_enabled = self.clock.nmitimen & 0x80 != 0;
                self.clock.nmitimen = val;
                // The NMI enable is a LEVEL, not an edge. RDNMI's flag latches at the start of
                // VBlank and stays latched until read, so enabling NMI while it is already up
                // delivers the interrupt immediately rather than waiting for the next VBlank.
                // Modelling only the VBlank edge meant a program that latched VBlank, then enabled
                // NMI, silently lost that frame's interrupt. AccuracySNES `B4.06` [ERRATA], which
                // snes9x and Mesen2 both passed while this failed.
                if !was_enabled && val & 0x80 != 0 && self.clock.rdnmi_flag {
                    self.clock.nmi_line = true;
                }
            }
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

    /// One `$4016`/`$4017` clock for controller port `port` — `(data1, data2)`. `Gamepad` (the
    /// default) is untouched, using [`Bus::joypad`]'s own original single-bit model exactly as
    /// before this module existed; every other [`PortDevice`] dispatches to
    /// [`crate::controller::PortState::clock`].
    fn port_clock(&mut self, port: usize) -> (u8, u8) {
        if self.ports[port].device == PortDevice::Gamepad {
            if self.joypad_strobe {
                // Held high, the register is being reloaded continuously, so it never advances:
                // every read returns the first bit. Software that reads without lowering the
                // strobe gets B over and over, which is the behaviour a latch-then-read driver
                // depends on not happening by accident.
                self.joypad_shift[port] = self.joypad[port];
                return (((self.joypad[port] & 0x8000) >> 15) as u8, 0);
            }
            // Shifting in ones is the pad's real behaviour once its sixteen data bits are gone:
            // nothing is left driving the line low, so reads 17-32 return 1. That is how software
            // tells a standard pad from a peripheral.
            let bit = ((self.joypad_shift[port] & 0x8000) >> 15) as u8;
            self.joypad_shift[port] = (self.joypad_shift[port] << 1) | 1;
            return (bit, 0);
        }
        let iobit = self.iobit_pin(port);
        let vh = self.ppu.visible_height();
        self.ports[port].clock(iobit, vh)
    }

    /// The IOBIT pin's current level for controller port `port` — WRIO ($4201/$4213) bit 6 for
    /// port 1, bit 7 for port 2 (ares `Controller::iobit()`).
    const fn iobit_pin(&self, port: usize) -> bool {
        self.pio & (0x40 << port) != 0
    }

    /// WRIO ($4201) write — the falling edge of bit 7 (controller port 2's IOBIT pin) latches the
    /// PPU's H/V dot counters, the exact mechanism a Super Scope's light sensor drives when it
    /// "sees" the CRT beam (ares `cpu/io.cpp`: `if(io.pio.bit(7) && !data.bit(7))
    /// ppu.latchCounters();`). Bit 6 (port 1) has no such wiring on real hardware — a Super Scope
    /// in port 1 simply never gets an auto-latch, matching `SuperScopeState`'s own doc.
    const fn set_pio(&mut self, val: u8) {
        if self.pio & 0x80 != 0 && val & 0x80 == 0 {
            self.ppu.latch_hv_counters();
        }
        self.pio = val;
    }

    /// Per-master-clock Super Scope beam-detection check (`v0.9.0`) — a no-op, one cheap branch,
    /// unless port 2 actually has a Super Scope attached (real hardware: only port 2's IOBIT pin
    /// reaches the PPU latch, `Self::set_pio`). Mirrors ares' `SuperScope::main()`: strobe the
    /// IOBIT pin low-then-high the instant the beam crosses the target dot on the target
    /// scanline, latching the H/V counters exactly as a real light sensor would.
    fn check_superscope_beam(&mut self) {
        if self.ports[1].device != PortDevice::SuperScope {
            return;
        }
        let vh = self.ppu.visible_height();
        let Some((target_v, target_dot)) = self.ports[1].super_scope.beam_target(vh) else {
            return;
        };
        if self.ppu.scanline() == target_v && self.ppu.dot() == target_dot {
            self.set_pio(self.pio & !0x80);
            self.set_pio(self.pio | 0x80);
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
            s.write_u16(self.joypad_shift[0]);
            s.write_u16(self.joypad_shift[1]);
            s.write_u16(self.joypad_auto[0]);
            s.write_u16(self.joypad_auto[1]);
            s.write_bool(self.joypad_strobe);
            s.write_u8(self.open_bus);
            s.write_u16(self.last_hdma_line);
            s.write_bool(self.in_hdma);
            s.write_bool(self.hdma_setup_done);
            s.write_u8(self.pio);
            self.ports[0].save_state(s);
            self.ports[1].save_state(s);
            // In-flight automatic joypad read (`FORMAT_VERSION` 5): the start snapshot + the busy
            // deadline, so a save taken during the ~4224-clock window restores an identical machine
            // state — the busy flag and the deferred `$4218-$421F` publish survive exactly.
            s.write_u16(self.joypad_auto_pending[0]);
            s.write_u16(self.joypad_auto_pending[1]);
            s.write_u64(self.auto_joypad_busy_until);
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
        self.joypad_shift[0] = s.read_u16()?;
        self.joypad_shift[1] = s.read_u16()?;
        self.joypad_auto[0] = s.read_u16()?;
        self.joypad_auto[1] = s.read_u16()?;
        self.joypad_strobe = s.read_bool()?;
        self.open_bus = s.read_u8()?;
        self.last_hdma_line = s.read_u16()?;
        self.in_hdma = s.read_bool()?;
        self.hdma_setup_done = s.read_bool()?;
        self.pio = s.read_u8()?;
        self.ports[0] = crate::controller::PortState::load_state(&mut s)?;
        self.ports[1] = crate::controller::PortState::load_state(&mut s)?;
        // In-flight automatic joypad read (`FORMAT_VERSION` 5). A pre-5 blob's `BUS0` section ends
        // above, so these reads fail loudly on it (the documented "old blob fails, no migration"
        // convention), which is why the format major was bumped.
        self.joypad_auto_pending[0] = s.read_u16()?;
        self.joypad_auto_pending[1] = s.read_u16()?;
        self.auto_joypad_busy_until = s.read_u64()?;
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
            // Matches ares/bsnes `CPU::Channel::readA` exactly: the invalid branch sets `mdr`
            // (this project's `open_bus`) to a hard `0`, not "leave it unchanged" — see
            // `docs/scheduler.md` §Open bus via DMA/HDMA for the full citation trail.
            self.open_bus = 0;
            return 0;
        }
        let val = self.decode_read(addr);
        // DMA/HDMA-driven A-bus reads update the open-bus latch exactly like a CPU read does —
        // confirmed by direct citation of ares' AND bsnes' `CPU::Channel::readA`
        // (`cpu.r.mdr = validA(address) ? bus.read(address, cpu.r.mdr) : 0;`) and their shared
        // `Bus::read`'s default unmapped reader (`[](n24, n8 data) { return data; }` — the
        // open-bus echo mechanism itself). DMA/HDMA *writes* deliberately do NOT update it (see
        // `write_a`/`write_b` below) — ares' `writeA`/`writeB` never touch `mdr` either. See
        // `docs/scheduler.md` §Open bus via DMA/HDMA for the full investigation this fix closes.
        self.open_bus = val;
        #[cfg(feature = "debug-hooks")]
        self.note_bus_access(addr, val, false);
        val
    }
    fn write_a(&mut self, addr: u32, val: u8) {
        let bank = (addr >> 16) & 0xFF;
        let off = addr & 0xFFFF;
        if matches!(bank, 0x00..=0x3F | 0x80..=0xBF)
            && matches!(off, 0x2100..=0x21FF | 0x4000..=0x43FF)
        {
            return;
        }
        #[cfg(feature = "debug-hooks")]
        self.note_bus_access(addr, val, true);
        self.decode_write(addr, val);
    }
    fn read_b(&mut self, addr: u8) -> u8 {
        let val = self.b_read(addr);
        // See `read_a`'s doc above — DMA/HDMA B-bus reads update open_bus too.
        self.open_bus = val;
        #[cfg(feature = "debug-hooks")]
        self.note_bus_access(0x00_2100 | u32::from(addr), val, false);
        val
    }
    fn write_b(&mut self, addr: u8, val: u8) {
        #[cfg(feature = "debug-hooks")]
        self.note_bus_access(0x00_2100 | u32::from(addr), val, true);
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
        self.note_bus_access(addr24, val, false);
        val
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        self.open_bus = val;
        #[cfg(feature = "debug-hooks")]
        self.note_bus_access(addr24, val, true);
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

    /// A strobe reloads the shift register, so two manual reads in one frame agree.
    ///
    /// The regression this locks: the shift register used to *be* the button word, so the first
    /// read consumed it and the second returned all-ones. A frontend rewrites the button state
    /// every frame, which hid it; a game that polls twice per frame would not have been hidden
    /// from it.
    #[test]
    fn strobe_reloads_the_gamepad_shift_register() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x8000); // B held, nothing else

        let read16 = |bus: &mut Bus| {
            bus.write_cpu_reg(0x4016, 0x01);
            bus.write_cpu_reg(0x4016, 0x00);
            let mut bits = 0u16;
            for _ in 0..16 {
                bits = (bits << 1) | u16::from(bus.read_cpu_reg(0x4016) & 1);
            }
            bits
        };

        assert_eq!(read16(&mut bus), 0x8000, "first read of the frame");
        assert_eq!(
            read16(&mut bus),
            0x8000,
            "second read must agree with the first"
        );
    }

    /// The falling edge captures the buttons as they are *then*, not as they were on the way up.
    #[test]
    fn strobe_captures_buttons_at_the_falling_edge() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x0000);
        bus.write_cpu_reg(0x4016, 0x01); // strobe high with nothing held
        bus.set_joypad(0, 0x8000); // B goes down while the strobe is still high
        bus.write_cpu_reg(0x4016, 0x00); // falling edge: this is what must be captured
        assert_eq!(
            bus.read_cpu_reg(0x4016) & 1,
            1,
            "the falling edge froze a stale button word"
        );
    }

    /// Held high, the register never advances: every read is the first bit.
    #[test]
    fn strobe_held_high_does_not_advance() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x8000); // B held, so the first bit is 1 and the rest are 0
        bus.write_cpu_reg(0x4016, 0x01);
        for _ in 0..4 {
            assert_eq!(
                bus.read_cpu_reg(0x4016) & 1,
                1,
                "a read with the strobe high advanced the shift register"
            );
        }
    }

    /// Past sixteen bits a standard pad reads 1 — which is how software identifies it.
    #[test]
    fn gamepad_reads_one_past_its_data_bits() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x0000);
        bus.write_cpu_reg(0x4016, 0x01);
        bus.write_cpu_reg(0x4016, 0x00);
        for _ in 0..16 {
            assert_eq!(
                bus.read_cpu_reg(0x4016) & 1,
                0,
                "a data bit read as pressed"
            );
        }
        for i in 0..4 {
            assert_eq!(
                bus.read_cpu_reg(0x4016) & 1,
                1,
                "read {} past the data bits",
                i + 17
            );
        }
    }

    /// A manual read must not disturb the auto-read result.
    #[test]
    fn manual_read_does_not_consume_the_auto_read_result() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x1234);
        // Stand in for the vblank poll: `$4218` reports the auto-read *result*, not the live pad.
        bus.write_cpu_reg(0x4200, 0x01);
        bus.poll_auto_joypad_for_test();
        bus.write_cpu_reg(0x4200, 0x00);
        bus.write_cpu_reg(0x4016, 0x01);
        bus.write_cpu_reg(0x4016, 0x00);
        for _ in 0..16 {
            let _ = bus.read_cpu_reg(0x4016);
        }
        assert_eq!(bus.read_cpu_reg(0x4218), 0x34);
        assert_eq!(bus.read_cpu_reg(0x4219), 0x12);
    }

    /// A latch held high across the automatic read makes every bit of the result the same.
    ///
    /// The read clocks the ports' shift registers, and while `$4016` bit 0 is high those registers
    /// reload rather than shift — so all sixteen clocks return the first bit. AccuracySNES `F1.11`.
    #[test]
    fn auto_read_is_corrupted_by_a_held_latch() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x9050); // B is held, so the repeated bit is 1
        bus.write_cpu_reg(0x4200, 0x01);
        bus.write_cpu_reg(0x4016, 0x01); // latch high, and left there

        bus.poll_auto_joypad_for_test();
        assert_eq!(bus.read_cpu_reg(0x4218), 0xFF);
        assert_eq!(bus.read_cpu_reg(0x4219), 0xFF);

        // Released, the same poll reports the buttons.
        bus.write_cpu_reg(0x4016, 0x00);
        bus.poll_auto_joypad_for_test();
        assert_eq!(bus.read_cpu_reg(0x4218), 0x50);
        assert_eq!(bus.read_cpu_reg(0x4219), 0x90);
    }

    /// `$4210`/`$4211` return the CPU open bus (MDR) in the bits hardware leaves floating.
    ///
    /// `$4210` RDNMI: bit 7 = the read-clearing `VBlank` flag, bits 4-6 = open bus, bits 0-3 = CPU
    /// version 2. `$4211` TIMEUP: bit 7 = the read-clearing IRQ flag, bits 0-6 = open bus. Matches
    /// ares `CPU::readIO` (which writes only the flag + version and leaves the rest as open bus).
    #[test]
    #[allow(clippy::field_reassign_with_default)] // a full `Bus` struct literal is impractical
    fn rdnmi_timeup_expose_open_bus_in_unused_bits() {
        let mut bus = Bus::default();
        // 0xAB = 1010_1011: distinctive so every masked open-bus position is exercised.
        // $4210 with the flag clear: bits 4-6 = 0xAB & 0x70 = 0x20, version = 0x02 -> 0x22.
        bus.open_bus = 0xAB;
        assert_eq!(bus.read_cpu_reg(0x4210), 0x22);
        // Flag set: bit7 | open-bus 4-6 | version -> 0x80 | 0x20 | 0x02 = 0xA2, and the read clears.
        bus.clock.rdnmi_flag = true;
        bus.open_bus = 0xAB;
        assert_eq!(bus.read_cpu_reg(0x4210), 0xA2);
        assert!(
            !bus.clock.rdnmi_flag,
            "reading $4210 clears the VBlank flag"
        );
        // $4211 with the IRQ flag clear: bits0-6 = 0xAB & 0x7F = 0x2B.
        bus.open_bus = 0xAB;
        assert_eq!(bus.read_cpu_reg(0x4211), 0x2B);
        // IRQ set: 0x80 | 0x2B = 0xAB, and the read clears.
        bus.clock.irq_line = true;
        bus.open_bus = 0xAB;
        assert_eq!(bus.read_cpu_reg(0x4211), 0xAB);
        assert!(!bus.clock.irq_line, "reading $4211 clears the IRQ flag");
    }

    /// The timed automatic read reads busy on `$4212` bit 0 for ~4224 master clocks and publishes
    /// its result only at completion — not instantly at vblank entry.
    #[test]
    fn auto_joypad_read_is_busy_for_its_window_then_publishes() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x1234);
        bus.write_cpu_reg(0x4200, 0x01); // arm auto-read ($4200 bit 0)
        bus.clock.master = 1000;
        bus.begin_auto_joypad();
        // Busy immediately, and the result is NOT yet published (still the power-on $0000).
        assert_eq!(
            bus.read_cpu_reg(0x4212) & 1,
            1,
            "busy at the start of the window"
        );
        assert_eq!(
            bus.read_cpu_reg(0x4218),
            0x00,
            "result not published mid-window"
        );
        // Still busy one clock before the deadline.
        bus.clock.master = 1000 + AUTO_JOYPAD_CLOCKS - 1;
        assert_eq!(
            bus.read_cpu_reg(0x4212) & 1,
            1,
            "still busy just before completion"
        );
        // At the deadline: no longer busy, and the snapshot is now published.
        bus.clock.master = 1000 + AUTO_JOYPAD_CLOCKS;
        assert_eq!(bus.read_cpu_reg(0x4212) & 1, 0, "not busy after the window");
        assert_eq!(
            bus.read_cpu_reg(0x4218),
            0x34,
            "result published at completion"
        );
        assert_eq!(bus.read_cpu_reg(0x4219), 0x12);
    }

    /// A save taken DURING the auto-read busy window restores an identical machine state
    /// (`FORMAT_VERSION` 5): the busy flag and the deferred snapshot survive save/load exactly.
    #[test]
    fn auto_joypad_busy_state_survives_save_load() {
        use rustysnes_savestate::{SaveReader, SaveWriter};
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x1234);
        bus.write_cpu_reg(0x4200, 0x01); // arm auto-read
        bus.clock.master = 5000;
        bus.begin_auto_joypad(); // start a read; deadline = 5000 + 4224
        assert_eq!(bus.read_cpu_reg(0x4212) & 1, 1, "busy before the save");
        // Save mid-window and restore into a fresh Bus.
        let mut w = SaveWriter::new();
        bus.save_state(&mut w);
        let bytes = w.into_bytes();
        let mut fresh = Bus::default();
        let mut r = SaveReader::new(&bytes);
        fresh
            .load_state(&mut r)
            .expect("mid-window round trip must succeed");
        // Still busy, result still deferred (lost if the state were not serialized).
        assert_eq!(
            fresh.read_cpu_reg(0x4212) & 1,
            1,
            "busy state survives the round trip"
        );
        assert_eq!(
            fresh.read_cpu_reg(0x4218),
            0x00,
            "result still deferred after load"
        );
        // Past the restored deadline, the restored snapshot publishes.
        fresh.clock.master = 5000 + AUTO_JOYPAD_CLOCKS;
        assert_eq!(
            fresh.read_cpu_reg(0x4212) & 1,
            0,
            "not busy after the restored deadline"
        );
        assert_eq!(
            fresh.read_cpu_reg(0x4218),
            0x34,
            "restored snapshot publishes at completion"
        );
    }

    /// `$4218` reports only what an *armed* automatic read put there.
    ///
    /// With `$4200` bit 0 clear the registers hold their previous contents indefinitely, so
    /// software that disarms auto-read to poll `$4016` by hand does not find the hardware's answer
    /// appearing underneath it. AccuracySNES `F1.07`.
    #[test]
    fn auto_read_result_only_updates_while_armed() {
        let mut bus = Bus::default();
        bus.set_joypad(0, 0x9050);
        assert_eq!(bus.read_cpu_reg(0x4218), 0x00, "nothing has polled yet");

        bus.write_cpu_reg(0x4200, 0x01);
        bus.poll_auto_joypad_for_test(); // the poll this arming performs at the next vblank
        assert_eq!(bus.read_cpu_reg(0x4218), 0x50);
        assert_eq!(bus.read_cpu_reg(0x4219), 0x90);

        bus.write_cpu_reg(0x4200, 0x00);
        bus.set_joypad(0, 0x0000); // the pad changes, but nothing is armed to notice
        assert_eq!(
            bus.read_cpu_reg(0x4218),
            0x50,
            "a disarmed read must not update"
        );
        assert_eq!(bus.read_cpu_reg(0x4219), 0x90);
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
    fn wram_and_wram_mut_expose_the_same_flat_128kib() {
        let mut bus = Bus::default();
        assert_eq!(bus.wram().len(), 0x2_0000);
        <Bus as CpuBus>::write24(&mut bus, 0x7E_1234, 0xAB);
        assert_eq!(bus.wram()[0x1234], 0xAB);
        bus.wram_mut()[0x5678] = 0xCD;
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x7E_5678), 0xCD);
    }

    #[test]
    fn peek_reads_wram_without_side_effects() {
        let mut bus = Bus::default();
        <Bus as CpuBus>::write24(&mut bus, 0x7E_1234, 0xAB);
        // A real CPU read first, so open_bus is a known, distinct value.
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x7E_1234), 0xAB);
        let open_bus_before = bus.open_bus;
        // `peek` must return the same byte `read24` would, but never touch `open_bus`.
        assert_eq!(bus.peek(0x7E_1234), 0xAB);
        assert_eq!(
            bus.open_bus, open_bus_before,
            "peek must not perturb open_bus"
        );
    }

    #[test]
    fn peek_of_io_register_space_is_zero_not_the_live_register() {
        let mut bus = Bus::default();
        <Bus as CpuBus>::write24(&mut bus, 0x00_4202, 0x10);
        <Bus as CpuBus>::write24(&mut bus, 0x00_4203, 0x10);
        // $4216 (RDMPY) genuinely holds 0x0100 now via `read24`, but `peek` never reaches
        // register space at all (real code never executes from it) — this documents that
        // limitation rather than silently returning a wrong "peek" of live register state.
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4216), 0x00);
        assert_eq!(bus.peek(0x00_4216), 0);
    }

    #[test]
    fn wrio_rdio_round_trips_and_defaults_to_all_ones() {
        let mut bus = Bus::default();
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4213), 0xFF);
        <Bus as CpuBus>::write24(&mut bus, 0x00_4201, 0x55);
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_4213), 0x55);
    }

    #[test]
    fn wrio_bit7_falling_edge_latches_hv_counters() {
        let mut bus = Bus::default();
        // Advance a few dots so the latch has a known, non-zero dot value to observe.
        for _ in 0..40 {
            bus.advance_master(1);
        }
        let dot_before = bus.ppu.dot();
        // Bit 7 starts high (power-on default 0xFF); a write clearing it is the falling edge that
        // should latch the H/V counters (ares `cpu/io.cpp`'s `if(io.pio.bit(7) && !data.bit(7))`).
        <Bus as CpuBus>::write24(&mut bus, 0x00_4201, 0x00);
        let ophct_lo = <Bus as CpuBus>::read24(&mut bus, 0x00_213C);
        #[allow(clippy::cast_possible_truncation)]
        let expected = (dot_before & 0xFF) as u8;
        assert_eq!(
            ophct_lo, expected,
            "WRIO bit7 falling edge should latch OPHCT"
        );
    }

    #[test]
    fn slhv_read_does_not_latch_while_wrio_bit7_is_clear() {
        let mut bus = Bus::default();
        for _ in 0..40 {
            bus.advance_master(1);
        }
        // Clearing bit 7 is itself a falling edge and latches once, here. That is the value the
        // counters must keep: every later $2137 read is gated off and must not disturb it.
        <Bus as CpuBus>::write24(&mut bus, 0x00_4201, 0x00);
        let latched = <Bus as CpuBus>::read24(&mut bus, 0x00_213C);
        for _ in 0..400 {
            bus.advance_master(1);
        }
        <Bus as CpuBus>::read24(&mut bus, 0x00_2137);
        assert_eq!(
            <Bus as CpuBus>::read24(&mut bus, 0x00_213C),
            latched,
            "$2137 latched the counters with WRIO bit 7 clear, where no latching can occur"
        );

        // And with the gate open again it must latch: the 0->1 transition is not itself an edge
        // that latches, so this isolates the read.
        <Bus as CpuBus>::write24(&mut bus, 0x00_4201, 0x80);
        for _ in 0..400 {
            bus.advance_master(1);
        }
        let dot_before = bus.ppu.dot();
        <Bus as CpuBus>::read24(&mut bus, 0x00_2137);
        #[allow(clippy::cast_possible_truncation)]
        let expected = (dot_before & 0xFF) as u8;
        assert_eq!(
            <Bus as CpuBus>::read24(&mut bus, 0x00_213C),
            expected,
            "$2137 did not latch with WRIO bit 7 set"
        );
    }

    #[test]
    fn wrio_bit6_falling_edge_does_not_latch() {
        let mut bus = Bus::default();
        for _ in 0..40 {
            bus.advance_master(1);
        }
        // Port 1's IOBIT (bit 6) has no real-hardware wiring to the PPU latch — only bit 7 does.
        <Bus as CpuBus>::write24(&mut bus, 0x00_4201, 0xBF); // clear bit 6, leave bit 7 set
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_213C), 0);
    }

    #[test]
    fn superscope_beam_latch_fires_at_target_position() {
        let mut bus = Bus::default();
        bus.set_port_device(1, crate::controller::PortDevice::SuperScope);
        bus.set_superscope(1, 10, 5, 0);
        let target_dot = 10 + 24;
        for _ in 0..2_000_000 {
            if bus.ppu.scanline() == 5 && bus.ppu.dot() == target_dot {
                break;
            }
            bus.advance_master(1);
        }
        assert_eq!(
            bus.ppu.scanline(),
            5,
            "should have reached the target scanline"
        );
        assert_eq!(
            bus.ppu.dot(),
            target_dot,
            "should have reached the target dot"
        );
        let ophct_lo = <Bus as CpuBus>::read24(&mut bus, 0x00_213C);
        assert_eq!(
            ophct_lo, target_dot as u8,
            "the beam crossing the target should have auto-latched OPHCT to it"
        );
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
