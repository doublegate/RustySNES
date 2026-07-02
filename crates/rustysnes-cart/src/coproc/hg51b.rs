//! The Hitachi HG51B S169 core — the CX4 coprocessor's CPU (Mega Man X2, Mega Man X3).
//!
//! Clean-room port of ares' `HG51B` component (ISC, `component/processor/hg51b/`). Unlike the
//! NEC DSP family (a fully separate chip program dumped to firmware), the HG51B's PROGRAM lives
//! in the cartridge ROM the user already owns — it fetches 256-word pages into a 2-page on-chip
//! instruction cache from cart ROM through the same bus the S-CPU sees (`cache()`), architecturally
//! closer to this project's GSU/SA-1 ports than to DSP-1. Only a small 3 KiB **data ROM** (a
//! trig/sqrt constant lookup table, `cx4.rom`) is a genuine external chip dump.
//!
//! Fixed-width 16-bit instruction word; ~30 real mnemonics (ALU/shift/branch/load-store/RAM-ROM
//! access) decoded from the top nibble (see `dispatch`). A 3 KiB data RAM (`$000-$BFF`, folded
//! from a `$000-$FFF` address space per the real chip's `>=$C00 -> -$400` quirk) and a 1024-entry
//! 24-bit data ROM back the math tables; 16 general-purpose 24-bit registers; an 8-deep hardware
//! call stack; a DMA unit and a suspend/wait state machine round out the chip.

// Chip-name jargon (HG51B, CX4, GPR, ...) is not Rust code; the register/IO/cache state is
// naturally dense with small bitfields and hardware-mirrored casts.
#![allow(
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::similar_names,
    // The register/ALU/IO methods below are a direct, dense port of ares' hardware register
    // switch statements; several happen not to touch runtime-only state and so LOOK const-
    // eligible to clippy, but marking them const would be cosmetic noise against the source of
    // truth's own (non-const) shape and buys nothing since none are called from a const context.
    clippy::missing_const_for_fn
)]

use alloc::boxed::Box;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

/// The host-facing memory surface the HG51B core reads/writes through.
///
/// The S-CPU's cart ROM (shared, read-only from the chip's side) and any other
/// externally-addressable region a specific board wires up (CX4 has none beyond ROM;
/// `read`/`write`/`is_rom`/`is_ram` mirror ares' `HG51B::{read,write,isROM,isRAM}` virtual hooks).
pub trait Hg51bBus {
    /// Whether `address` (a chip-relative linear address) is cart ROM.
    fn is_rom(&self, address: u32) -> bool;
    /// Whether `address` is chip-visible RAM (CX4 has none; always `false`).
    fn is_ram(&self, address: u32) -> bool;
    /// Read a byte at a chip-relative linear address (cache refill, DMA, bus-port reads).
    fn read(&mut self, address: u32) -> u8;
    /// Write a byte at a chip-relative linear address (DMA only for CX4 — no chip-side RAM).
    fn write(&mut self, address: u32, data: u8);
}

#[derive(Debug, Clone, Copy, Default)]
struct Registers {
    pb: u16,  // program bank (15-bit)
    pc: u8,   // program counter (within the 256-word cached page)
    n: bool,  // negative
    z: bool,  // zero
    c: bool,  // carry
    v: bool,  // overflow
    i: bool,  // interrupt pending (latched by an enabled halt)
    a: u32,   // accumulator (24-bit)
    p: u16,   // page register (15-bit)
    mul: u64, // multiplier result (48-bit)
    mdr: u32, // bus memory data register (24-bit)
    rom: u32, // data-ROM read buffer (24-bit)
    ram: u32, // data-RAM read/write buffer (24-bit)
    mar: u32, // bus memory address register (24-bit)
    dpr: u32, // data-RAM address pointer (24-bit)
    gpr: [u32; 16],
}

#[derive(Debug, Clone, Copy, Default)]
struct CacheState {
    enable: bool,
    page: bool,
    lock: [bool; 2],
    address: [u32; 2],
    base: u32,
    pb: u16,
    pc: u8,
}

#[derive(Debug, Clone, Copy, Default)]
struct DmaState {
    enable: bool,
    source: u32,
    target: u32,
    length: u16,
}

#[derive(Debug, Clone, Copy, Default)]
struct BusState {
    enable: bool,
    reading: bool,
    writing: bool,
    pending: u32,
    address: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Wait {
    rom: u32,
    ram: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Suspend {
    enable: bool,
    duration: u32,
}

#[derive(Debug, Clone, Copy)]
struct Io {
    lock: bool,
    halt: bool, // starts true (chip idle until the host writes cache.pc)
    irq: bool,  // false = enabled, true = disabled
    rom_mapping: bool,
    vector: [u8; 32],
    wait: Wait,
    suspend: Suspend,
    cache: CacheState,
    dma: DmaState,
    bus: BusState,
}

impl Default for Io {
    fn default() -> Self {
        Self {
            lock: false,
            halt: true,
            irq: false,
            rom_mapping: true,
            vector: [0; 32],
            wait: Wait::default(),
            suspend: Suspend::default(),
            cache: CacheState::default(),
            dma: DmaState::default(),
            bus: BusState::default(),
        }
    }
}

/// The HG51B S169 core (CX4's CPU).
///
/// Free-runs synchronously to its next halt/wait state via [`Hg51b::run_until_halt`] — the same
/// run-to-completion host-sync pattern this project's GSU (`Go`-bit) and DSP-1 (`RQM`) engines
/// use, since the pc-write trigger (`$7f4f` while halted) is the only observable coupling to the
/// S-CPU (`docs/cart.md` §CX4).
pub struct Hg51b {
    r: Registers,
    io: Io,
    program_ram: Box<[[u16; 256]; 2]>,
    data_rom: Box<[u32; 1024]>,
    data_ram: Box<[u8; 3072]>,
    stack: [u32; 8],
    data_rom_loaded: bool,
    /// Guards against a runaway/malformed program looping forever inside one host trigger.
    instructions_run: u64,
}

/// Hard cap on instructions executed per [`Hg51b::run_until_halt`] call — a runaway or malformed
/// program halts the host call rather than the emulator (mirrors the GSU/DSP-1 engines' caps).
const RUN_CAP: u64 = 20_000_000;

impl core::fmt::Debug for Hg51b {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Hg51b")
            .field("halted", &self.io.halt)
            .field("data_rom_loaded", &self.data_rom_loaded)
            .field("pb", &self.r.pb)
            .field("pc", &self.r.pc)
            .field("instructions_run", &self.instructions_run)
            .finish_non_exhaustive()
    }
}

impl Default for Hg51b {
    fn default() -> Self {
        Self::new()
    }
}

impl Hg51b {
    /// Construct a powered-off HG51B (inert until [`Hg51b::load_data_rom`] supplies the constant
    /// table — `docs/adr/0003`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            r: Registers::default(),
            io: Io::default(),
            program_ram: Box::new([[0u16; 256]; 2]),
            data_rom: Box::new([0u32; 1024]),
            data_ram: Box::new([0u8; 3072]),
            stack: [0; 8],
            data_rom_loaded: false,
            instructions_run: 0,
        }
    }

    /// Load the 3072-byte (1024 x 24-bit, 3 bytes/word little-endian) data-ROM constant table
    /// (`cx4.rom`). Returns `false` (and leaves the chip inert) if the dump is the wrong size.
    pub fn load_data_rom(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < self.data_rom.len() * 3 {
            return false;
        }
        for (i, word) in self.data_rom.iter_mut().enumerate() {
            let o = i * 3;
            *word =
                u32::from(bytes[o]) | u32::from(bytes[o + 1]) << 8 | u32::from(bytes[o + 2]) << 16;
        }
        self.data_rom_loaded = true;
        true
    }

    /// Whether the data-ROM constant table has been supplied (the chip is functional).
    #[must_use]
    pub const fn data_rom_loaded(&self) -> bool {
        self.data_rom_loaded
    }

    /// Total instructions executed since power-on (debugger/diagnostics).
    #[must_use]
    pub const fn instructions_run(&self) -> u64 {
        self.instructions_run
    }

    // --- Host-visible chip-relative address space (used by the board's addressDRAM/addressIO). --

    /// Read the 3 KiB data RAM at a chip-relative offset (`$000-$FFF`, folded per the `>=$C00`
    /// hardware quirk — see `fold_dram`).
    #[must_use]
    pub fn read_dram(&self, offset: u32) -> u8 {
        let a = Self::fold_dram(offset);
        if a >= 0xC00 {
            0
        } else {
            self.data_ram[a as usize]
        }
    }

    /// Write the 3 KiB data RAM at a chip-relative offset (see [`Self::read_dram`]).
    pub fn write_dram(&mut self, offset: u32, data: u8) {
        let a = Self::fold_dram(offset);
        if a < 0xC00 {
            self.data_ram[a as usize] = data;
        }
    }

    const fn fold_dram(offset: u32) -> u32 {
        let a = offset & 0xFFF;
        if a >= 0xC00 { a - 0x400 } else { a }
    }

    /// Read the fixed IO register block (`$7F40-$7FEF`, DMA/cache/wait/IRQ/vector/GPR-mirror
    /// controls — ares `HitachiDSP::readIO`). `local` is the address already folded to
    /// `0x7C00 | (address & 0x3FF)` by the caller (the board owns the bus-window decode).
    #[must_use]
    pub fn read_io(&mut self, local: u32) -> u8 {
        match local {
            0x7F40 => byte(self.io.dma.source, 0),
            0x7F41 => byte(self.io.dma.source, 1),
            0x7F42 => byte(self.io.dma.source, 2),
            0x7F43 => (self.io.dma.length & 0xFF) as u8,
            0x7F44 => (self.io.dma.length >> 8) as u8,
            0x7F45 => byte(self.io.dma.target, 0),
            0x7F46 => byte(self.io.dma.target, 1),
            0x7F47 => byte(self.io.dma.target, 2),
            0x7F48 => u8::from(self.io.cache.page),
            0x7F49 => byte(self.io.cache.base, 0),
            0x7F4A => byte(self.io.cache.base, 1),
            0x7F4B => byte(self.io.cache.base, 2),
            0x7F4C => u8::from(self.io.cache.lock[0]) | (u8::from(self.io.cache.lock[1]) << 1),
            0x7F4D => (self.io.cache.pb & 0xFF) as u8,
            0x7F4E => (self.io.cache.pb >> 8) as u8,
            0x7F4F => self.io.cache.pc,
            0x7F50 => (self.io.wait.ram as u8) | ((self.io.wait.rom as u8) << 4),
            0x7F51 => u8::from(self.io.irq),
            0x7F52 => u8::from(self.io.rom_mapping),
            0x7F53 | 0x7F54 | 0x7F55 | 0x7F56 | 0x7F57 | 0x7F59 | 0x7F5B | 0x7F5C | 0x7F5D
            | 0x7F5E | 0x7F5F => {
                u8::from(self.io.suspend.enable)
                    | (u8::from(self.r.i) << 1)
                    | (u8::from(self.running()) << 6)
                    | (u8::from(self.busy()) << 7)
            }
            0x7F60..=0x7F7F => self.io.vector[(local & 0x1F) as usize],
            0x7F80..=0x7FAF | 0x7FC0..=0x7FEF => {
                let a = local & 0x3F;
                byte(self.r.gpr[(a / 3) as usize], (a % 3) as u8)
            }
            _ => 0,
        }
    }

    /// Write the fixed IO register block (see [`Self::read_io`]). `bus` supplies the chip-ROM
    /// access this may need to kick off (a cache-page refill on the pc-write trigger).
    pub fn write_io(&mut self, local: u32, data: u8, bus: &mut impl Hg51bBus) {
        match local {
            0x7F40 => set_byte(&mut self.io.dma.source, 0, data),
            0x7F41 => set_byte(&mut self.io.dma.source, 1, data),
            0x7F42 => set_byte(&mut self.io.dma.source, 2, data),
            0x7F43 => self.io.dma.length = (self.io.dma.length & 0xFF00) | u16::from(data),
            0x7F44 => self.io.dma.length = (self.io.dma.length & 0x00FF) | (u16::from(data) << 8),
            0x7F45 => set_byte(&mut self.io.dma.target, 0, data),
            0x7F46 => set_byte(&mut self.io.dma.target, 1, data),
            0x7F47 => {
                set_byte(&mut self.io.dma.target, 2, data);
                if self.io.halt {
                    self.io.dma.enable = true;
                    self.run_until_halt(bus);
                }
            }
            0x7F48 => {
                self.io.cache.page = data & 1 != 0;
                if self.io.halt {
                    self.io.cache.enable = true;
                    self.run_until_halt(bus);
                }
            }
            0x7F49 => set_byte(&mut self.io.cache.base, 0, data),
            0x7F4A => set_byte(&mut self.io.cache.base, 1, data),
            0x7F4B => set_byte(&mut self.io.cache.base, 2, data),
            0x7F4C => {
                self.io.cache.lock[0] = data & 1 != 0;
                self.io.cache.lock[1] = data & 2 != 0;
            }
            0x7F4D => self.io.cache.pb = (self.io.cache.pb & 0xFF00) | u16::from(data),
            0x7F4E => {
                self.io.cache.pb = (self.io.cache.pb & 0x00FF) | (u16::from(data & 0x7F) << 8);
            }
            0x7F4F => {
                self.io.cache.pc = data;
                if self.io.halt {
                    self.io.halt = false;
                    self.r.pb = self.io.cache.pb;
                    self.r.pc = self.io.cache.pc;
                    self.run_until_halt(bus);
                }
            }
            0x7F50 => {
                self.io.wait.ram = u32::from(data & 7);
                self.io.wait.rom = u32::from((data >> 4) & 7);
            }
            0x7F51 => self.io.irq = data & 1 != 0,
            0x7F52 => self.io.rom_mapping = data & 1 != 0,
            0x7F53 => {
                self.io.lock = false;
                self.io.halt = true;
            }
            0x7F55 => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 0;
            }
            0x7F56 => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 32;
            }
            0x7F57 => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 64;
            }
            0x7F58 => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 96;
            }
            0x7F59 => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 128;
            }
            0x7F5A => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 160;
            }
            0x7F5B => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 192;
            }
            0x7F5C => {
                self.io.suspend.enable = true;
                self.io.suspend.duration = 224;
            }
            0x7F5D => self.io.suspend.enable = false,
            0x7F5E => self.r.i = false,
            0x7F60..=0x7F7F => self.io.vector[(local & 0x1F) as usize] = data,
            0x7F80..=0x7FAF | 0x7FC0..=0x7FEF => {
                let a = local & 0x3F;
                set_byte(&mut self.r.gpr[(a / 3) as usize], (a % 3) as u8, data);
            }
            _ => {}
        }
    }

    /// Whether the chip is doing anything at all (cache/dma/bus pending, or not halted).
    #[must_use]
    pub const fn running(&self) -> bool {
        self.io.cache.enable || self.io.dma.enable || self.io.bus.pending > 0 || !self.io.halt
    }

    /// Whether the chip is mid-cache/dma/bus-access (narrower than [`Self::running`]).
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.io.cache.enable || self.io.dma.enable || self.io.bus.pending > 0
    }

    /// Whether the host IRQ line should be asserted (raised when the chip halts with IRQs
    /// enabled — `docs/cart.md` §CX4's execution-model note).
    #[must_use]
    pub const fn irq_pending(&self) -> bool {
        self.r.i
    }

    /// Run the chip to its next halt (or `RUN_CAP` instructions, whichever comes first) —
    /// the host-sync run-to-completion pattern (see the struct doc).
    pub fn run_until_halt(&mut self, bus: &mut impl Hg51bBus) {
        if !self.data_rom_loaded {
            return;
        }
        let mut n = 0u64;
        // Mirrors ares' `main()` dispatch order: lock/suspend/cache/dma are each serviced to
        // completion EVEN WHILE HALTED (a DMA or cache-page load triggered by a host write while
        // the chip is halted — `$7F47`/`$7F48` — must still run; only bare instruction execution
        // is gated on `!halt`), so the loop condition is "any of those is pending", not just
        // "not halted".
        while (self.io.lock
            || self.io.suspend.enable
            || self.io.cache.enable
            || self.io.dma.enable
            || !self.io.halt)
            && n < RUN_CAP
        {
            self.main(bus);
            n += 1;
            self.instructions_run += 1;
        }
    }

    fn main(&mut self, bus: &mut impl Hg51bBus) {
        if self.io.lock {
            return self.step(1);
        }
        if self.io.suspend.enable {
            return self.suspend();
        }
        if self.io.cache.enable {
            self.cache(bus);
            return;
        }
        if self.io.dma.enable {
            return self.dma(bus);
        }
        if self.io.halt {
            return self.step(1);
        }
        self.execute(bus);
    }

    fn step(&mut self, clocks: u32) {
        if !self.io.bus.enable {
            return;
        }
        if self.io.bus.pending > clocks {
            self.io.bus.pending -= clocks;
        } else {
            self.io.bus.enable = false;
            self.io.bus.pending = 0;
        }
    }

    /// The bus-port async access completion, deferred here since our `run_until_halt` is
    /// synchronous rather than clock-ticked: performed inline the instant `step` would have
    /// cleared `bus.pending` in ares' cycle-ticked model.
    fn finish_bus_access(&mut self, bus: &mut impl Hg51bBus) {
        if self.io.bus.enable && self.io.bus.pending == 0 {
            self.io.bus.enable = false;
            if self.io.bus.reading {
                self.io.bus.reading = false;
                self.r.mdr = u32::from(bus.read(self.io.bus.address));
            }
            if self.io.bus.writing {
                self.io.bus.writing = false;
                bus.write(self.io.bus.address, self.r.mdr as u8);
            }
        }
    }

    fn wait(&self, address: u32, bus: &impl Hg51bBus) -> u32 {
        if bus.is_rom(address) {
            return 1 + self.io.wait.rom;
        }
        if bus.is_ram(address) {
            return 1 + self.io.wait.ram;
        }
        1
    }

    fn execute(&mut self, bus: &mut impl Hg51bBus) {
        if !self.cache(bus) {
            self.io.halt = true;
            return;
        }
        let opcode = self.program_ram[usize::from(self.io.cache.page)][usize::from(self.r.pc)];
        self.advance(bus);
        self.step(1);
        self.dispatch(opcode, bus);
    }

    fn advance(&mut self, bus: &mut impl Hg51bBus) {
        let (pc, overflow) = self.r.pc.overflowing_add(1);
        self.r.pc = pc;
        if overflow {
            if self.io.cache.page {
                self.io.halt = true;
                return;
            }
            self.io.cache.page = true;
            if self.io.cache.lock[usize::from(self.io.cache.page)] {
                self.io.halt = true;
                return;
            }
            self.r.pb = self.r.p;
            if !self.cache(bus) {
                self.io.halt = true;
            }
        }
    }

    fn suspend(&mut self) {
        if self.io.suspend.duration == 0 {
            self.step(1);
            return;
        }
        self.step(self.io.suspend.duration);
        self.io.suspend.duration = 0;
        self.io.suspend.enable = false;
    }

    /// Refill the requested cache page from cart ROM if not already resident. Returns `false`
    /// (chip halts) if both pages are locked/unavailable.
    fn cache(&mut self, bus: &mut impl Hg51bBus) -> bool {
        let address = self.io.cache.base.wrapping_add(u32::from(self.r.pb) * 512);
        if self.io.cache.address[usize::from(self.io.cache.page)] == address {
            self.io.cache.enable = false;
            return true;
        }
        self.io.cache.page = !self.io.cache.page;
        if self.io.cache.address[usize::from(self.io.cache.page)] == address {
            self.io.cache.enable = false;
            return true;
        }
        if self.io.cache.lock[usize::from(self.io.cache.page)] {
            self.io.cache.page = !self.io.cache.page;
        }
        if self.io.cache.lock[usize::from(self.io.cache.page)] {
            self.io.cache.enable = false;
            return false;
        }

        self.io.cache.address[usize::from(self.io.cache.page)] = address;
        let mut a = address;
        for offset in 0..256usize {
            self.step(self.wait(a, bus));
            let lo = bus.read(a);
            a = a.wrapping_add(1);
            let hi = bus.read(a);
            a = a.wrapping_add(1);
            self.program_ram[usize::from(self.io.cache.page)][offset] =
                u16::from(lo) | (u16::from(hi) << 8);
        }
        self.io.cache.enable = false;
        true
    }

    fn dma(&mut self, bus: &mut impl Hg51bBus) {
        for offset in 0..u32::from(self.io.dma.length) {
            let source = self.io.dma.source.wrapping_add(offset) & 0xFF_FFFF;
            let target = self.io.dma.target.wrapping_add(offset) & 0xFF_FFFF;
            if bus.is_rom(source) && bus.is_rom(target) {
                self.io.lock = true;
                return;
            }
            if bus.is_ram(source) && bus.is_ram(target) {
                self.io.lock = true;
                return;
            }
            self.step(self.wait(source, bus));
            let data = bus.read(source);
            self.step(self.wait(target, bus));
            bus.write(target, data);
        }
        self.io.dma.enable = false;
    }

    // --- Register-index space (readRegister/writeRegister; distinct from the host IO block). --

    fn read_register(&mut self, address: u16, bus: &mut impl Hg51bBus) -> u32 {
        let v = match address {
            0x01 => (self.r.mul >> 24) as u32 & 0xFF_FFFF,
            0x02 => self.r.mul as u32 & 0xFF_FFFF,
            0x03 => self.r.mdr,
            0x08 => self.r.rom,
            0x0C => self.r.ram,
            0x13 => self.r.mar,
            0x1C => self.r.dpr,
            0x20 => u32::from(self.r.pc),
            0x28 => u32::from(self.r.p),
            0x2E => {
                self.io.bus.enable = true;
                self.io.bus.reading = true;
                self.io.bus.pending = 1 + self.io.wait.rom;
                self.io.bus.address = self.r.mar;
                self.finish_bus_access(bus);
                0
            }
            0x2F => {
                self.io.bus.enable = true;
                self.io.bus.reading = true;
                self.io.bus.pending = 1 + self.io.wait.ram;
                self.io.bus.address = self.r.mar;
                self.finish_bus_access(bus);
                0
            }
            // 0x50 (the all-zero constant) falls through to the wildcard arm below.
            0x51 => 0xFF_FFFF,
            0x52 => 0x00_FF00,
            0x53 => 0xFF_0000,
            0x54 => 0x00_FFFF,
            0x55 => 0xFF_FF00,
            0x56 => 0x80_0000,
            0x57 => 0x7F_FFFF,
            0x58 => 0x00_8000,
            0x59 => 0x00_7FFF,
            0x5A => 0xFF_7FFF,
            0x5B => 0xFF_FF7F,
            0x5C => 0x01_0000,
            0x5D => 0xFE_FFFF,
            0x5E => 0x00_0100,
            0x5F => 0x00_FEFF,
            0x60..=0x7F => self.r.gpr[usize::from(address & 0xF)],
            _ => 0,
        };
        v & 0xFF_FFFF
    }

    fn write_register(&mut self, address: u16, data: u32, bus: &mut impl Hg51bBus) {
        let data = data & 0xFF_FFFF;
        match address {
            0x01 => self.r.mul = (self.r.mul & 0x00_FF_FF_FF) | (u64::from(data) << 24),
            0x02 => self.r.mul = (self.r.mul & 0xFF_FF_FF_00_00_00) | u64::from(data),
            0x03 => self.r.mdr = data,
            0x08 => self.r.rom = data,
            0x0C => self.r.ram = data,
            0x13 => self.r.mar = data,
            0x1C => self.r.dpr = data,
            0x20 => self.r.pc = data as u8,
            0x28 => self.r.p = data as u16 & 0x7FFF,
            0x2E => {
                self.io.bus.enable = true;
                self.io.bus.writing = true;
                self.io.bus.pending = 1 + self.io.wait.rom;
                self.io.bus.address = self.r.mar;
                self.finish_bus_access(bus);
            }
            0x2F => {
                self.io.bus.enable = true;
                self.io.bus.writing = true;
                self.io.bus.pending = 1 + self.io.wait.ram;
                self.io.bus.address = self.r.mar;
                self.finish_bus_access(bus);
            }
            0x60..=0x7F => self.r.gpr[usize::from(address & 0xF)] = data,
            _ => {}
        }
    }

    /// Write this core's mutable state — every register, the IO block, the two cached program
    /// pages, the 3 KiB data RAM, and the 8-deep call stack — into an `"HG51"` section. The data
    /// ROM (`cx4.rom`, firmware) is deliberately NOT written, per `docs/adr/0003`'s "never embed a
    /// chip-ROM dump in a save-state" posture: it is reloaded fresh via [`Self::load_data_rom`]
    /// before a matching [`Self::load_state`] call. `instructions_run` (a debugger counter, not
    /// emulated-hardware state) is also omitted.
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"HG51", |s| {
            s.write_u16(self.r.pb);
            s.write_u8(self.r.pc);
            s.write_bool(self.r.n);
            s.write_bool(self.r.z);
            s.write_bool(self.r.c);
            s.write_bool(self.r.v);
            s.write_bool(self.r.i);
            s.write_u32(self.r.a);
            s.write_u16(self.r.p);
            s.write_u64(self.r.mul);
            s.write_u32(self.r.mdr);
            s.write_u32(self.r.rom);
            s.write_u32(self.r.ram);
            s.write_u32(self.r.mar);
            s.write_u32(self.r.dpr);
            for &g in &self.r.gpr {
                s.write_u32(g);
            }
            s.write_bool(self.io.lock);
            s.write_bool(self.io.halt);
            s.write_bool(self.io.irq);
            s.write_bool(self.io.rom_mapping);
            s.write_bytes(&self.io.vector);
            s.write_u32(self.io.wait.rom);
            s.write_u32(self.io.wait.ram);
            s.write_bool(self.io.suspend.enable);
            s.write_u32(self.io.suspend.duration);
            s.write_bool(self.io.cache.enable);
            s.write_bool(self.io.cache.page);
            s.write_bool(self.io.cache.lock[0]);
            s.write_bool(self.io.cache.lock[1]);
            s.write_u32(self.io.cache.address[0]);
            s.write_u32(self.io.cache.address[1]);
            s.write_u32(self.io.cache.base);
            s.write_u16(self.io.cache.pb);
            s.write_u8(self.io.cache.pc);
            s.write_bool(self.io.dma.enable);
            s.write_u32(self.io.dma.source);
            s.write_u32(self.io.dma.target);
            s.write_u16(self.io.dma.length);
            s.write_bool(self.io.bus.enable);
            s.write_bool(self.io.bus.reading);
            s.write_bool(self.io.bus.writing);
            s.write_u32(self.io.bus.pending);
            s.write_u32(self.io.bus.address);
            for page in self.program_ram.iter() {
                for &word in page {
                    s.write_u16(word);
                }
            }
            for &byte in self.data_ram.iter() {
                s.write_u8(byte);
            }
            for &word in &self.stack {
                s.write_u32(word);
            }
        });
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input (the section framing itself already rejects
    /// a wrong tag or a truncated read). No field here is used as an unchecked array index by a
    /// width narrower than its own type (`pc`/`cache.pc` are `u8` matching the 256-entry cache
    /// page exactly; `cache.page` is a `bool` matching the 2-entry page arrays exactly), so unlike
    /// the NEC DSP engine's `load_state`, no additional masking is needed here for memory safety.
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"HG51")?;
        self.r.pb = s.read_u16()?;
        self.r.pc = s.read_u8()?;
        self.r.n = s.read_bool()?;
        self.r.z = s.read_bool()?;
        self.r.c = s.read_bool()?;
        self.r.v = s.read_bool()?;
        self.r.i = s.read_bool()?;
        self.r.a = s.read_u32()?;
        self.r.p = s.read_u16()?;
        self.r.mul = s.read_u64()?;
        self.r.mdr = s.read_u32()?;
        self.r.rom = s.read_u32()?;
        self.r.ram = s.read_u32()?;
        self.r.mar = s.read_u32()?;
        self.r.dpr = s.read_u32()?;
        for g in &mut self.r.gpr {
            *g = s.read_u32()?;
        }
        self.io.lock = s.read_bool()?;
        self.io.halt = s.read_bool()?;
        self.io.irq = s.read_bool()?;
        self.io.rom_mapping = s.read_bool()?;
        self.io.vector = s.read_bytes(32)?.try_into().unwrap_or([0; 32]);
        self.io.wait.rom = s.read_u32()?;
        self.io.wait.ram = s.read_u32()?;
        self.io.suspend.enable = s.read_bool()?;
        self.io.suspend.duration = s.read_u32()?;
        self.io.cache.enable = s.read_bool()?;
        self.io.cache.page = s.read_bool()?;
        self.io.cache.lock[0] = s.read_bool()?;
        self.io.cache.lock[1] = s.read_bool()?;
        self.io.cache.address[0] = s.read_u32()?;
        self.io.cache.address[1] = s.read_u32()?;
        self.io.cache.base = s.read_u32()?;
        self.io.cache.pb = s.read_u16()?;
        self.io.cache.pc = s.read_u8()?;
        self.io.dma.enable = s.read_bool()?;
        self.io.dma.source = s.read_u32()?;
        self.io.dma.target = s.read_u32()?;
        self.io.dma.length = s.read_u16()?;
        self.io.bus.enable = s.read_bool()?;
        self.io.bus.reading = s.read_bool()?;
        self.io.bus.writing = s.read_bool()?;
        self.io.bus.pending = s.read_u32()?;
        self.io.bus.address = s.read_u32()?;
        for page in self.program_ram.iter_mut() {
            for word in page.iter_mut() {
                *word = s.read_u16()?;
            }
        }
        for byte in self.data_ram.iter_mut() {
            *byte = s.read_u8()?;
        }
        for word in &mut self.stack {
            *word = s.read_u32()?;
        }
        Ok(())
    }
}

const fn byte(v: u32, i: u8) -> u8 {
    (v >> (i * 8)) as u8
}

fn set_byte(v: &mut u32, i: u8, data: u8) {
    let shift = i * 8;
    *v = (*v & !(0xFF << shift)) | (u32::from(data) << shift);
}

include!("hg51b_instructions.rs");
