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
//! FastROM bit). The CPU model charges one "CPU cycle" per bus access via [`CpuBus::on_cpu_cycle`]
//! — but that call carries no address, so the Bus stashes the access speed of the most recent
//! [`CpuBus::read24`]/[`CpuBus::write24`] in `clock.next_speed` and consumes it on the next
//! `on_cpu_cycle`. Internal (no-bus) CPU cycles leave `next_speed` at the default `6`. Each
//! master-clock advance steps the PPU dot clock (4 master/dot) and the SPC accumulator in
//! lockstep, so a mid-instruction PPU event (an HV-IRQ at a precise dot, a mid-scanline register
//! write seen by the next instruction) lands at the right time without per-quirk patches.

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

use crate::dma::Dma;
use crate::dma_bus::DmaBus;

/// WRAM size — the SNES has 128 KiB of work RAM (`$7E0000-$7FFFFF`).
const WRAM_SIZE: usize = 128 * 1024;
/// Master clocks per PPU dot (nominal; long-dot remainder folded into the 1364/1360/1368 line).
const MASTER_PER_DOT: u32 = 4;
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
    /// Master clocks the *next* `on_cpu_cycle` should advance by (set by `read24`/`write24`).
    next_speed: u32,
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
            next_speed: 6,
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

/// The CPU multiply/divide unit (`$4202-$4206` → `$4214-$4217`). The SNES computes these with a
/// hardware latency; the deterministic core resolves them instantly (the result is what tests
/// read), which is accurate for every documented program.
#[derive(Debug, Clone, Default)]
struct MulDiv {
    mpya: u8,
    dividend: u16,
    rddiv: u16,
    rdmpy: u16,
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
        }
    }

    /// Set the latched controller state for a player (`0` = P1, `1` = P2). 12-bit `BYsSUDLR....`.
    pub fn set_joypad(&mut self, player: usize, state: u16) {
        if let Some(slot) = self.joypad.get_mut(player) {
            *slot = state;
        }
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
            if self.clock.dot_accum >= MASTER_PER_DOT {
                self.clock.dot_accum -= MASTER_PER_DOT;
                self.tick_ppu_dot();
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
        }
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
        let cost = dma.run_gp(mask, self);
        self.dma = dma;
        self.advance_master(cost);
    }

    /// Run one visible-scanline's HDMA (called by the scheduler at the line boundary), charging
    /// the per-line budget to the master clock.
    pub fn run_hdma_line(&mut self) {
        if self.dma.hdma_enable == 0 {
            return;
        }
        let mut dma = core::mem::take(&mut self.dma);
        let cost = dma.hdma_run(self);
        self.dma = dma;
        self.advance_master(cost);
    }

    /// Per-frame HDMA setup (called at V=0), charging its cost to the master clock.
    pub fn hdma_frame_setup(&mut self) {
        let mut dma = core::mem::take(&mut self.dma);
        dma.hdma_reset();
        let cost = dma.hdma_setup(self);
        self.dma = dma;
        self.advance_master(cost);
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
                    self.dma
                        .write_reg(((addr >> 4) & 0xF) as usize, (addr & 0xF) as u8, val);
                }
                _ => self.cart_write_raw(addr24, val),
            },
            _ => self.cart_write_raw(addr24, val),
        }
    }

    fn cart_read_raw(&mut self, addr24: u32) -> u8 {
        self.cart
            .as_mut()
            .map_or(self.open_bus, |c| c.read24(addr24))
    }

    fn cart_write_raw(&mut self, addr24: u32, val: u8) {
        if let Some(c) = self.cart.as_mut() {
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
}

/// A cart-only view of the Bus for the PPU's `tick_dot` (split borrow: the PPU may need a
/// cart-mediated read for Mode 7 / coprocessor boards without aliasing the whole Bus).
struct CartView<'a> {
    cart: &'a mut Option<Cart>,
    open: u8,
}

impl VideoBus for CartView<'_> {
    fn cart_read(&mut self, addr24: u32) -> u8 {
        self.cart.as_mut().map_or(self.open, |c| c.read24(addr24))
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
}

/// The 65C816's view: route a 24-bit access + drive the master clock in lockstep.
impl CpuBus for Bus {
    fn read24(&mut self, addr24: u32) -> u8 {
        self.clock.next_speed = self.access_speed(addr24);
        let val = self.decode_read(addr24);
        self.open_bus = val;
        val
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        self.clock.next_speed = self.access_speed(addr24);
        self.open_bus = val;
        self.decode_write(addr24, val);
    }

    fn poll_nmi(&mut self) -> bool {
        core::mem::take(&mut self.clock.nmi_line)
    }

    fn poll_irq(&mut self) -> bool {
        self.clock.irq_line
    }

    fn on_cpu_cycle(&mut self) {
        // Advance the master clock by the speed of the access this cycle paid for (set by
        // read24/write24); an internal CPU cycle leaves it at the default 6.
        let speed = self.clock.next_speed;
        self.clock.next_speed = 6;
        self.advance_master(speed);
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

    #[test]
    fn master_clock_advances_on_access() {
        let mut bus = Bus::default();
        let before = bus.clock.master;
        <Bus as CpuBus>::read24(&mut bus, 0x00_8000); // sets next_speed = 8
        bus.on_cpu_cycle(); // advances 8
        assert_eq!(bus.clock.master, before + 8);
    }
}
