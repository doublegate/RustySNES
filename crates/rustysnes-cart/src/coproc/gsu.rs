//! The GSU (Argonaut RISC) — the Super FX coprocessor core.
//!
//! The GSU is a 16-bit RISC engine that runs a program out of the Game Pak ROM (or RAM) and
//! plots into a bitmap held in the Game Pak RAM. It powers Star Fox, Stunt Race FX, Vortex
//! (GSU-1, ~10.74 MHz) and Yoshi's Island, Doom (GSU-2, ~21.47 MHz). Unlike the NEC DSP family
//! there is **no chip-ROM dump** — the GSU's program lives in the cartridge ROM the user already
//! owns, so the core is functional the moment a Super FX cart loads (`docs/cart.md`).
//!
//! This is a clean-room re-implementation of ares' `GSU` + `SuperFX` components (ISC) in safe
//! `no_std` Rust. The instruction encoding, the ALT-prefix mode machine, the pixel-plot pipeline,
//! the 256-byte/32-line opcode cache, and the ROM/RAM buffer latency are hardware facts; the
//! decode here mirrors the published Super FX instruction set, not ares' source layout.
//!
//! ## Host-synchronization model (no free-running scheduler tick)
//!
//! The GSU is started by the SNES CPU writing the high byte of R15 (the program counter) at
//! `$301F`, which sets the **Go** flag and begins execution at `(PBR:R15)`. The chip then runs
//! autonomously until it executes `STOP`, which clears Go and (optionally) raises the cart IRQ.
//! Software polls the status flag register (SFR, `$3030/$3031`) for Go to clear. Because Go is
//! the only observable coupling between the two clocks — exactly the RQM role the DSP-1 uses —
//! the board runs the GSU **to completion the instant Go is set** ([`Gsu::run_until_stopped`]),
//! capped against a runaway program. This keeps the bus boundary byte-exact and fully
//! deterministic (`docs/adr/0004`) without a per-master-clock core hook, mirroring the DSP-1
//! `run_until_rqm` pattern. While Go is set the GSU owns the shared ROM/RAM (the CPU sees the
//! snooze vector / open bus — see [`crate::coproc::superfx`]); run-to-completion serialises that
//! arbitration naturally.

// The GSU aliases every register as signed and unsigned, the status register is a bitfield of
// single-bit flags, and the plot/character addressing is dense with deliberate narrowing casts.
// Flagging each would bury real issues, so the cast family + bitfield lints are allowed here.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::unreadable_literal,
    clippy::struct_excessive_bools,
    clippy::missing_const_for_fn,
    clippy::verbose_bit_mask,
    clippy::if_not_else
)]

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

/// The Game Pak ROM/RAM the GSU reads and plots into, with the GSU-internal 24-bit decode.
///
/// The board constructs this each run, borrowing its ROM (shared, read-only) and its Game Pak
/// RAM (the plot target). The address map is the GSU's own view (ares `SuperFX::read`/`write`):
/// banks `$00-$3F` fold LoROM-style, `$40-$5F` are linear ROM, `$70-$71` are the RAM bitmap.
pub struct GsuMem<'a> {
    /// Game Pak ROM image (already mirror-rounded to a power-of-two `rom_mask + 1`).
    pub rom: &'a [u8],
    /// `rom.len().next_power_of_two() - 1` (ares `romSizeRound(rom.size()) - 1`).
    pub rom_mask: u32,
    /// Game Pak RAM (the GSU plot bitmap), `ram_mask + 1` bytes.
    pub ram: &'a mut [u8],
    /// `ram.len() - 1` (RAM is always a power of two).
    pub ram_mask: u32,
}

impl GsuMem<'_> {
    /// GSU-internal read of a 24-bit address (ROM banks `$00-$5F`, RAM banks `$70-$71`).
    #[must_use]
    fn read(&self, address: u32) -> u8 {
        if address & 0xc0_0000 == 0x00_0000 {
            // $00-3F:any — LoROM fold: ((A & $3F0000) >> 1) | (A & $7FFF).
            let off = (((address & 0x3f_0000) >> 1) | (address & 0x7fff)) & self.rom_mask;
            return self.rom.get(off as usize).copied().unwrap_or(0);
        }
        if address & 0xe0_0000 == 0x40_0000 {
            // $40-5F:0000-FFFF — linear ROM.
            let off = address & self.rom_mask;
            return self.rom.get(off as usize).copied().unwrap_or(0);
        }
        if address & 0xfe_0000 == 0x70_0000 {
            // $70-71:0000-FFFF — Game Pak RAM.
            let off = address & self.ram_mask;
            return self.ram.get(off as usize).copied().unwrap_or(0);
        }
        0
    }

    /// GSU-internal write — only the Game Pak RAM (`$70-$71`) is writable.
    fn write(&mut self, address: u32, data: u8) {
        if address & 0xfe_0000 == 0x70_0000 {
            let off = (address & self.ram_mask) as usize;
            if let Some(slot) = self.ram.get_mut(off) {
                *slot = data;
            }
        }
    }
}

// --- Status flag register (SFR) bit masks (ares `registers.hpp::SFR`). ---------------------

/// Zero flag.
const SFR_Z: u16 = 1 << 1;
/// Carry flag.
const SFR_CY: u16 = 1 << 2;
/// Sign flag.
const SFR_S: u16 = 1 << 3;
/// Overflow flag.
const SFR_OV: u16 = 1 << 4;
/// Go flag — the GSU is running.
const SFR_G: u16 = 1 << 5;
/// ROM-buffer (R14) read pending flag.
const SFR_R: u16 = 1 << 6;
/// ALT1 instruction-mode prefix.
const SFR_ALT1: u16 = 1 << 8;
/// ALT2 instruction-mode prefix.
const SFR_ALT2: u16 = 1 << 9;
/// WITH (register-pair) prefix flag.
const SFR_B: u16 = 1 << 12;
/// Interrupt-request flag (raised by STOP).
const SFR_IRQ: u16 = 1 << 15;
/// The host-visible SFR read mask (ares `SFR::operator u32`).
const SFR_READ_MASK: u16 = 0x9f7e;

/// The two-deep pixel cache (ares `PixelCache`): one 8x1 strip of pending plot colours.
#[derive(Clone, Copy)]
struct PixelCache {
    /// Character-strip offset `(y << 5) + (x >> 3)`; `!0` = empty.
    offset: u16,
    /// Per-column bit-pending mask (bit set = column written since the strip opened).
    bitpend: u8,
    /// The 8 column colours (index `(x & 7) ^ 7`).
    data: [u8; 8],
}

impl PixelCache {
    const fn empty() -> Self {
        Self {
            offset: !0,
            bitpend: 0,
            data: [0; 8],
        }
    }
}

/// The Super FX GSU core — registers, status, caches, and the ROM/RAM buffer latches.
///
/// The board owns the ROM/RAM bytes and supplies them via [`GsuMem`] on each run; this struct is
/// the pure register/state machine (so the whole thing is `Clone` for save-states).
pub struct Gsu {
    /// R0-R15 general-purpose registers (R15 = program counter).
    r: [u16; 16],
    /// `true` when R14 was written this instruction (triggers a ROM-buffer fetch).
    r14_mod: bool,
    /// `true` when R15 was written this instruction (an explicit PC change — no auto-increment).
    r15_mod: bool,
    /// Status flag register.
    sfr: u16,
    /// Source-register select (set by FROM/WITH prefixes).
    sreg: usize,
    /// Destination-register select (set by TO/WITH prefixes).
    dreg: usize,
    /// The prefetched opcode (the 1-instruction pipeline — gives the GSU its branch delay slot).
    pipeline: u8,
    /// Program bank register.
    pbr: u8,
    /// ROM bank register (R14-relative ROM-buffer bank).
    rombr: u8,
    /// RAM bank register (0/1).
    rambr: bool,
    /// Cache base register.
    cbr: u16,
    /// Screen base register (the plot bitmap base, `<< 10`).
    scbr: u8,
    /// Screen-mode register: height select + ROM/RAM-on + colour depth.
    scmr_ht: u8,
    /// SCMR ROM-on: GSU owns the ROM bus.
    scmr_ron: bool,
    /// SCMR RAM-on: GSU owns the RAM bus.
    scmr_ran: bool,
    /// SCMR colour-depth mode (0/1/2/3 → 2/4/4/8 bpp).
    scmr_md: u8,
    /// Colour register (the plot colour).
    colr: u8,
    /// Plot-option register: OBJ mode, freeze-high, high-nibble, dither, transparent.
    por_obj: bool,
    por_freezehigh: bool,
    por_highnibble: bool,
    por_dither: bool,
    por_transparent: bool,
    /// Backup-RAM write-enable register.
    bramr: bool,
    /// Version code register.
    vcr: u8,
    /// CFGR IRQ-mask bit (when set, STOP does not raise IRQ).
    cfgr_irq: bool,
    /// CFGR MS0 fast-multiply bit.
    cfgr_ms0: bool,
    /// Clock-select register (`true` = 21 MHz; halves access latencies).
    clsr: bool,
    /// Clocks until the ROM data register (`romdr`) is valid.
    romcl: u32,
    /// ROM-buffer data register (the byte at `(ROMBR:R14)`).
    romdr: u8,
    /// Clocks until the pending RAM-buffer write commits.
    ramcl: u32,
    /// RAM-buffer address register (the pending write address).
    ramar: u16,
    /// RAM-buffer data register (the pending write byte).
    ramdr: u8,
    /// The last RAM address touched by a load/store (ares `regs.ramaddr`).
    ramaddr: u16,
    /// 512-byte opcode cache buffer (32 lines x 16 bytes).
    cache_buffer: Box<[u8; 512]>,
    /// Per-line cache-valid flags.
    cache_valid: [bool; 32],
    /// The two-deep plot pixel cache.
    pixelcache: [PixelCache; 2],
    /// Cumulative GSU clock ticks (for the timing model + the runaway cap).
    clocks: u64,
    /// Cumulative instructions executed (the liveness counter the board exposes).
    instructions: u64,
    /// Per-bus-access clock checkpoints queued by [`Gsu::step`] within the instruction
    /// currently being drained by [`Gsu::step_one`]; `pending_idx` is the next unread entry
    /// (an index cursor avoids an O(n) pop-front on every access).
    pending_clocks: Vec<u32>,
    /// Read cursor into `pending_clocks`.
    pending_idx: usize,
    /// Master clocks still owed on the checkpoint most recently pulled by [`Gsu::tick`] before
    /// the next one may be pulled — the countdown that paces per-access checkpoints out one
    /// master clock at a time (mirrors the `romcl`/`ramcl` delayed-commit pattern, generalized
    /// to gate overall instruction progress rather than just ROM/RAM buffer latches). The
    /// instruction's memory side effects are NOT deferred to when this reaches zero — they
    /// already happened, eagerly, the moment [`Gsu::step_one`] pulled the checkpoint; `owed`
    /// only withholds the checkpoint *after* it (i.e. gates how soon the GSU can make its next
    /// move), which is the piece that lets the CPU's own instructions interleave in between.
    owed: u32,
}

impl Clone for Gsu {
    fn clone(&self) -> Self {
        Self {
            cache_buffer: self.cache_buffer.clone(),
            pending_clocks: self.pending_clocks.clone(),
            ..*self
        }
    }
}

impl core::fmt::Debug for Gsu {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Gsu")
            .field("go", &self.go())
            .field("pbr", &self.pbr)
            .field("r15", &self.r[15])
            .field("instructions", &self.instructions)
            .finish_non_exhaustive()
    }
}

impl Default for Gsu {
    fn default() -> Self {
        Self::new()
    }
}

impl Gsu {
    /// Power-on a GSU core (ares `GSU::power` + `SuperFX::power`).
    ///
    /// # Panics
    /// Never in practice: the only fallible step is sizing the fixed 512-byte cache buffer, which
    /// cannot fail for the constant length.
    #[must_use]
    pub fn new() -> Self {
        Self {
            r: [0; 16],
            r14_mod: false,
            r15_mod: false,
            sfr: 0,
            sreg: 0,
            dreg: 0,
            pbr: 0,
            rombr: 0,
            rambr: false,
            cbr: 0,
            scbr: 0,
            scmr_ht: 0,
            scmr_ron: false,
            scmr_ran: false,
            scmr_md: 0,
            colr: 0,
            por_obj: false,
            por_freezehigh: false,
            por_highnibble: false,
            por_dither: false,
            por_transparent: false,
            bramr: false,
            vcr: 0x04,
            cfgr_irq: false,
            cfgr_ms0: false,
            clsr: false,
            romcl: 0,
            romdr: 0,
            ramcl: 0,
            ramar: 0,
            ramdr: 0,
            ramaddr: 0,
            pipeline: 0x01, // nop
            cache_buffer: vec![0u8; 512].into_boxed_slice().try_into().unwrap(),
            cache_valid: [false; 32],
            pixelcache: [PixelCache::empty(); 2],
            clocks: 0,
            instructions: 0,
            pending_clocks: Vec::new(),
            pending_idx: 0,
            owed: 0,
        }
    }

    /// Whether the Go flag is set (the GSU is running / owns the bus).
    #[must_use]
    pub const fn go(&self) -> bool {
        self.sfr & SFR_G != 0
    }

    /// Whether the GSU currently owns the ROM bus (Go + SCMR ROM-on).
    #[must_use]
    pub const fn owns_rom(&self) -> bool {
        self.go() && self.scmr_ron
    }

    /// Whether the GSU currently owns the RAM bus (Go + SCMR RAM-on).
    #[must_use]
    pub const fn owns_ram(&self) -> bool {
        self.go() && self.scmr_ran
    }

    /// Whether the GSU is asserting its IRQ line (SFR IRQ flag, gated by CFGR IRQ-mask).
    #[must_use]
    pub const fn irq_pending(&self) -> bool {
        self.sfr & SFR_IRQ != 0
    }

    /// The number of instructions executed since power-on.
    #[must_use]
    pub const fn instructions(&self) -> u64 {
        self.instructions
    }

    /// The number of GSU clock cycles executed.
    #[must_use]
    pub const fn clocks(&self) -> u64 {
        self.clocks
    }

    // --- SFR helpers. ----------------------------------------------------------------------

    #[inline]
    const fn flag(&self, mask: u16) -> bool {
        self.sfr & mask != 0
    }

    #[inline]
    fn set_flag(&mut self, mask: u16, value: bool) {
        if value {
            self.sfr |= mask;
        } else {
            self.sfr &= !mask;
        }
    }

    /// The composite ALT mode (alt1 | alt2 << 1) ares uses for the prefixed-instruction split.
    #[inline]
    const fn alt1(&self) -> bool {
        self.flag(SFR_ALT1)
    }
    #[inline]
    const fn alt2(&self) -> bool {
        self.flag(SFR_ALT2)
    }

    // --- Register write with R14/R15 modified tracking (ares `Register::modified`). --------

    #[inline]
    fn write_r(&mut self, n: usize, val: u16) {
        self.r[n] = val;
        match n {
            14 => self.r14_mod = true,
            15 => self.r15_mod = true,
            _ => {}
        }
    }

    /// Source register (FROM / WITH selected).
    #[inline]
    const fn sr(&self) -> u16 {
        self.r[self.sreg]
    }

    /// Write the destination register (TO / WITH selected).
    #[inline]
    fn set_dr(&mut self, val: u16) {
        let d = self.dreg;
        self.write_r(d, val);
    }

    /// Per-instruction prefix reset (ares `Registers::reset` — NOT power-on).
    #[inline]
    fn reset_prefix(&mut self) {
        self.sfr &= !(SFR_B | SFR_ALT1 | SFR_ALT2);
        self.sreg = 0;
        self.dreg = 0;
    }

    // --- The host-sync driver. -------------------------------------------------------------

    /// Drain one bus-access clock checkpoint (ares `SuperFX::step`/`Thread::synchronize`
    /// granularity — `sfc/coprocessor/superfx/timing.cpp`) and report its clock count, running
    /// as many further GSU instructions as needed to produce one. The board's host loop calls
    /// this in a loop, folding each returned delta into the master clock immediately, so the
    /// PPU/NMI/APU advance in step with the GSU at true per-access granularity instead of
    /// crediting an entire instruction — or an entire multi-instruction burst — all at once
    /// (which let a two-pass render, e.g. a screen split into a left-half then a right-half
    /// `Go` burst, desync the two halves' notion of "now"). Each instruction still runs to
    /// completion eagerly inside `Gsu::main_step` — only the *reporting* of its accesses to
    /// the caller is paced one at a time, which is externally indistinguishable from true
    /// per-access interleaving because nothing GSU-side reads back caller/Bus state
    /// mid-instruction. Returns `0` once `Go` clears and there is nothing left queued.
    pub fn step_one(&mut self, mem: &mut GsuMem) -> u32 {
        loop {
            if self.pending_idx < self.pending_clocks.len() {
                let clocks = self.pending_clocks[self.pending_idx];
                self.pending_idx += 1;
                if self.pending_idx == self.pending_clocks.len() {
                    self.pending_clocks.clear();
                    self.pending_idx = 0;
                }
                if clocks > 0 {
                    return clocks;
                }
                continue;
            }
            if !self.go() {
                return 0;
            }
            self.main_step(mem);
        }
    }

    /// Advance by exactly one master clock (ares `SuperFX::main`, scheduled by its `Thread` at
    /// native master-clock granularity — `sfc/coprocessor/superfx/superfx.cpp`). The board's
    /// host loop calls this from inside the Bus's own per-master-tick loop, unconditionally,
    /// every single tick — the same place the PPU dot and the APU's SMP-cycle release happen —
    /// so the GSU genuinely interleaves with the CPU's own instruction stream instead of a `Go`
    /// burst draining to completion "atomically" inside the one bus write that armed it (which
    /// left the CPU unable to do any of its own work, or service a second `Go` burst, until the
    /// first fully finished). `owed` paces [`Gsu::step_one`]'s per-access checkpoints out one
    /// master clock at a time: the instruction's own side effects still land eagerly the moment
    /// its checkpoint is pulled (no full deferred-commit — see the field doc), but the *next*
    /// GSU instruction can no longer start until the real number of elapsed master clocks the
    /// current one costs has actually passed, letting the CPU run freely in between.
    pub fn tick(&mut self, mem: &mut GsuMem) {
        if self.owed > 0 {
            self.owed -= 1;
            return;
        }
        let clocks = self.step_one(mem);
        if clocks > 0 {
            self.owed = clocks - 1;
        }
    }

    /// Run the GSU to completion: step instructions while Go is set, capped against a runaway
    /// program. Called by the board the instant the CPU sets Go (the DSP-1 `run_until_rqm`
    /// analogue). Returns when the program executes `STOP` (Go clears) or the cap trips.
    ///
    /// Prefer [`Gsu::step_one`] driven by the board's host loop when master-clock-accurate
    /// interleaving matters (e.g. a screen render split across multiple `Go` bursts within one
    /// displayed frame) — this method is for callers that only need the final result.
    pub fn run_until_stopped(&mut self, mem: &mut GsuMem) {
        /// Instruction cap: a generous bound (~30 frames of GSU work) so a wedged program can't
        /// spin the host forever. A correct program reaches `STOP` long before this.
        const MAX_INSTRUCTIONS: u64 = 8_000_000;
        let start = self.instructions;
        while self.go() && self.instructions - start < MAX_INSTRUCTIONS {
            self.main_step(mem);
        }
        // Nobody drains the per-access checkpoints this run pushed (only `step_one`'s callers
        // want them) — clear them so they don't accumulate across repeated calls.
        self.pending_clocks.clear();
        self.pending_idx = 0;
    }

    /// Execute one GSU instruction (ares `SuperFX::main`, the Go-set path).
    fn main_step(&mut self, mem: &mut GsuMem) {
        let opcode = self.peekpipe(mem);
        self.instructions = self.instructions.wrapping_add(1);
        self.execute(opcode, mem);

        if self.r14_mod {
            self.r14_mod = false;
            self.update_rom_buffer();
        }
        if self.r15_mod {
            self.r15_mod = false;
        } else {
            self.r[15] = self.r[15].wrapping_add(1);
        }
    }

    // --- Timing / ROM-RAM buffer latency (ares `timing.cpp`). ------------------------------

    /// Advance the GSU clock by `clocks`, committing the ROM/RAM buffer when their latency
    /// elapses.
    fn step(&mut self, clocks: u32, mem: &mut GsuMem) {
        if self.romcl != 0 {
            self.romcl -= clocks.min(self.romcl);
            if self.romcl == 0 {
                self.set_flag(SFR_R, false);
                self.romdr = mem.read((u32::from(self.rombr) << 16) + u32::from(self.r[14]));
            }
        }
        if self.ramcl != 0 {
            self.ramcl -= clocks.min(self.ramcl);
            if self.ramcl == 0 {
                let addr = 0x70_0000 + (u32::from(self.rambr) << 16) + u32::from(self.ramar);
                mem.write(addr, self.ramdr);
            }
        }
        self.clocks = self.clocks.wrapping_add(u64::from(clocks));
        // Queue this access's clocks as a synchronization checkpoint (ares `SuperFX::step`
        // calls `Thread::synchronize(cpu)` at exactly this point, every single bus access —
        // `sfc/coprocessor/superfx/timing.cpp`). `step_one` drains one checkpoint per call so
        // the board's host loop can fold each one into the master clock immediately instead of
        // crediting an entire instruction (or an entire multi-instruction burst) at once. The
        // instruction still runs to completion eagerly here — only the *reporting* of its bus
        // accesses to the Bus is deferred/paced, which is externally indistinguishable from true
        // per-access interleaving because nothing GSU-side reads back Bus state mid-instruction.
        self.pending_clocks.push(clocks);
    }

    /// Latency for a normal (non-cache-hit) clocked access — halved at 21 MHz (CLSR).
    #[inline]
    const fn access_latency(&self) -> u32 {
        if self.clsr { 5 } else { 6 }
    }

    fn sync_rom_buffer(&mut self, mem: &mut GsuMem) {
        if self.romcl != 0 {
            self.step(self.romcl, mem);
        }
    }

    fn read_rom_buffer(&mut self, mem: &mut GsuMem) -> u8 {
        self.sync_rom_buffer(mem);
        self.romdr
    }

    fn update_rom_buffer(&mut self) {
        self.set_flag(SFR_R, true);
        self.romcl = self.access_latency();
    }

    fn sync_ram_buffer(&mut self, mem: &mut GsuMem) {
        if self.ramcl != 0 {
            self.step(self.ramcl, mem);
        }
    }

    fn read_ram_buffer(&mut self, mem: &mut GsuMem, address: u16) -> u8 {
        self.sync_ram_buffer(mem);
        mem.read(0x70_0000 + (u32::from(self.rambr) << 16) + u32::from(address))
    }

    fn write_ram_buffer(&mut self, mem: &mut GsuMem, address: u16, data: u8) {
        self.sync_ram_buffer(mem);
        self.ramcl = self.access_latency();
        self.ramar = address;
        self.ramdr = data;
    }

    // --- Opcode fetch + cache (ares `memory.cpp`). -----------------------------------------

    fn read_opcode(&mut self, address: u16, mem: &mut GsuMem) -> u8 {
        let offset = address.wrapping_sub(self.cbr);
        if offset < 512 {
            let line = (offset >> 4) as usize;
            if !self.cache_valid[line] {
                let dp0 = (offset & 0xfff0) as usize;
                let sp0 = (u32::from(self.pbr) << 16)
                    + u32::from(self.cbr.wrapping_add(dp0 as u16) & 0xfff0);
                for i in 0..16 {
                    self.step(self.access_latency(), mem);
                    self.cache_buffer[dp0 + i] = mem.read(sp0 + i as u32);
                }
                self.cache_valid[line] = true;
            } else {
                self.step(if self.clsr { 1 } else { 2 }, mem);
            }
            return self.cache_buffer[offset as usize];
        }

        if self.pbr <= 0x5f {
            self.sync_rom_buffer(mem);
        } else {
            self.sync_ram_buffer(mem);
        }
        self.step(self.access_latency(), mem);
        mem.read((u32::from(self.pbr) << 16) | u32::from(address))
    }

    /// Fetch the next opcode without advancing R15 (ares `peekpipe`).
    fn peekpipe(&mut self, mem: &mut GsuMem) -> u8 {
        let result = self.pipeline;
        self.pipeline = self.read_opcode(self.r[15], mem);
        self.r15_mod = false;
        result
    }

    /// Consume an operand byte, advancing R15 (ares `pipe`).
    fn pipe(&mut self, mem: &mut GsuMem) -> u8 {
        let result = self.pipeline;
        self.r[15] = self.r[15].wrapping_add(1);
        self.pipeline = self.read_opcode(self.r[15], mem);
        self.r15_mod = false;
        result
    }

    fn flush_cache(&mut self) {
        self.cache_valid = [false; 32];
    }

    /// Host read/write of the cache window (`$3100-$32FF`, ares `readCache`/`writeCache`).
    fn read_cache(&self, address: u16) -> u8 {
        self.cache_buffer[(address.wrapping_add(self.cbr) & 511) as usize]
    }

    fn write_cache(&mut self, address: u16, data: u8) {
        let a = address.wrapping_add(self.cbr) & 511;
        self.cache_buffer[a as usize] = data;
        if a & 15 == 15 {
            self.cache_valid[(a >> 4) as usize] = true;
        }
    }

    // --- Plot / colour / rpix pipeline (ares `core.cpp`). ----------------------------------

    /// Apply the colour-register transform for `source` (ares `SuperFX::color`).
    const fn color(&self, source: u8) -> u8 {
        if self.por_highnibble {
            return (self.colr & 0xf0) | (source >> 4);
        }
        if self.por_freezehigh {
            return (self.colr & 0xf0) | (source & 0x0f);
        }
        source
    }

    /// Read back the plotted pixel at `(x, y)` (ares `SuperFX::rpix`); also flushes both caches.
    fn rpix(&mut self, x: u8, y: u8, mem: &mut GsuMem) -> u8 {
        // ares flushes both strips in place (clearing their bitpend so they are not re-flushed).
        // We flush by value, then clear the originals to match.
        let pc1 = self.pixelcache[1];
        self.flush_pixel_cache_into(pc1, mem);
        self.pixelcache[1].bitpend = 0;
        let pc0 = self.pixelcache[0];
        self.flush_pixel_cache_into(pc0, mem);
        self.pixelcache[0].bitpend = 0;

        let bpp = Self::bpp(self.scmr_md);
        let addr = self.char_address(x, y, bpp);
        let mut data = 0u8;
        let xi = (x & 7) ^ 7;
        for n in 0..bpp {
            let byte = ((n >> 1) << 4) + (n & 1);
            self.step(if self.clsr { 5 } else { 6 }, mem);
            data |= ((mem.read(addr + byte) >> xi) & 1) << n;
        }
        data
    }

    /// Bits-per-pixel for SCMR mode (ares `2 << (md - (md >> 1))` = {2, 4, 4, 8}).
    const fn bpp(md: u8) -> u32 {
        (2 << (md - (md >> 1))) as u32
    }

    /// The Game Pak RAM byte address of the character row holding `(x, y)` (ares character-number
    /// math, shared by `plot`/`rpix`/`flushPixelCache`).
    fn char_address(&self, x: u8, y: u8, bpp: u32) -> u32 {
        let x = u32::from(x);
        let y = u32::from(y);
        let cn = match if self.por_obj { 3 } else { self.scmr_ht } {
            0 => ((x & 0xf8) << 1) + ((y & 0xf8) >> 3),
            1 => ((x & 0xf8) << 1) + ((x & 0xf8) >> 1) + ((y & 0xf8) >> 3),
            2 => ((x & 0xf8) << 1) + (x & 0xf8) + ((y & 0xf8) >> 3),
            _ => ((y & 0x80) << 2) + ((x & 0x80) << 1) + ((y & 0x78) << 1) + ((x & 0x78) >> 3),
        };
        0x70_0000 + (cn * (bpp << 3)) + (u32::from(self.scbr) << 10) + ((y & 0x07) * 2)
    }

    /// Flush one pixel-cache strip to the Game Pak RAM (ares `flushPixelCache`). The strip is
    /// taken by value; callers clear the live `pixelcache` entry's `bitpend` themselves.
    fn flush_pixel_cache_into(&mut self, cache: PixelCache, mem: &mut GsuMem) {
        if cache.bitpend == 0 {
            return;
        }
        let x = (cache.offset << 3) as u8;
        let y = (cache.offset >> 5) as u8;
        let bpp = Self::bpp(self.scmr_md);
        let addr = self.char_address(x, y, bpp);

        for n in 0..bpp {
            let byte = ((n >> 1) << 4) + (n & 1);
            let mut data = 0u8;
            for col in 0..8 {
                data |= ((cache.data[col] >> n) & 1) << col;
            }
            if cache.bitpend != 0xff {
                self.step(if self.clsr { 5 } else { 6 }, mem);
                data &= cache.bitpend;
                data |= mem.read(addr + byte) & !cache.bitpend;
            }
            self.step(if self.clsr { 5 } else { 6 }, mem);
            mem.write(addr + byte, data);
        }
    }

    // --- Host register interface ($3000-$32FF), used by the board. -------------------------

    /// Host read of a GSU register (`address` = `$0000-$02FF` window offset; ares `readIO`).
    pub fn read_register(&mut self, address: u16) -> u8 {
        let address = 0x3000 | (address & 0x1ff);
        if (0x3100..=0x32ff).contains(&address) {
            return self.read_cache(address - 0x3100);
        }
        if (0x3000..=0x301f).contains(&address) {
            let r = self.r[(address >> 1 & 15) as usize];
            return (r >> ((address & 1) << 3)) as u8;
        }
        match address {
            0x3030 => self.sfr as u8,
            0x3031 => {
                let r = (self.sfr & SFR_READ_MASK) >> 8;
                self.set_flag(SFR_IRQ, false);
                r as u8
            }
            0x3034 => self.pbr,
            0x3036 => self.rombr,
            0x303b => self.vcr,
            0x303c => u8::from(self.rambr),
            0x303e => self.cbr as u8,
            0x303f => (self.cbr >> 8) as u8,
            _ => 0,
        }
    }

    /// Host write of a GSU register. Returns `true` if this write set the Go flag (0->1), so the
    /// board knows to run the GSU to completion (ares `writeIO`).
    #[must_use]
    pub fn write_register(&mut self, address: u16, data: u8) -> bool {
        let address = 0x3000 | (address & 0x1ff);
        let was_go = self.go();

        if (0x3100..=0x32ff).contains(&address) {
            self.write_cache(address - 0x3100, data);
            return false;
        }
        if (0x3000..=0x301f).contains(&address) {
            let n = (address >> 1 & 15) as usize;
            if address & 1 == 0 {
                self.r[n] = (self.r[n] & 0xff00) | u16::from(data);
            } else {
                self.r[n] = (u16::from(data) << 8) | (self.r[n] & 0x00ff);
            }
            if n == 14 {
                self.update_rom_buffer();
            }
            if address == 0x301f {
                self.set_flag(SFR_G, true);
            }
            return !was_go && self.go();
        }

        match address {
            0x3030 => {
                let g = self.go();
                self.sfr = (self.sfr & 0xff00) | u16::from(data);
                if g && !self.go() {
                    self.cbr = 0;
                    self.flush_cache();
                }
            }
            0x3031 => self.sfr = (u16::from(data) << 8) | (self.sfr & 0x00ff),
            0x3033 => self.bramr = data & 0x01 != 0,
            0x3034 => {
                self.pbr = data & 0x7f;
                self.flush_cache();
            }
            0x3037 => {
                self.cfgr_irq = data & 0x80 != 0;
                self.cfgr_ms0 = data & 0x20 != 0;
            }
            0x3038 => self.scbr = data,
            0x3039 => self.clsr = data & 0x01 != 0,
            0x303a => self.set_scmr(data),
            _ => {}
        }
        !was_go && self.go()
    }

    /// Decode the SCMR byte (ares `SCMR::operator=`).
    fn set_scmr(&mut self, data: u8) {
        self.scmr_ht = (u8::from(data & 0x20 != 0) << 1) | u8::from(data & 0x04 != 0);
        self.scmr_ron = data & 0x10 != 0;
        self.scmr_ran = data & 0x08 != 0;
        self.scmr_md = data & 0x03;
    }

    // --- The instruction set (ares `instructions.cpp` + `instruction.cpp` switch). ---------

    #[allow(clippy::too_many_lines)]
    fn execute(&mut self, opcode: u8, mem: &mut GsuMem) {
        let n = (opcode & 0x0f) as usize;
        match opcode {
            0x00 => self.i_stop(),
            0x01 => self.reset_prefix(),      // nop
            0x02 => self.i_cache(),           // cache
            0x03 => self.i_lsr(),             // lsr
            0x04 => self.i_rol(),             // rol
            0x05 => self.i_branch(true, mem), // bra
            0x06 => self.i_branch(self.flag(SFR_S) == self.flag(SFR_OV), mem), // bge? -> see below
            0x07 => self.i_branch(self.flag(SFR_S) != self.flag(SFR_OV), mem),
            0x08 => self.i_branch(!self.flag(SFR_Z), mem), // bne
            0x09 => self.i_branch(self.flag(SFR_Z), mem),  // beq
            0x0a => self.i_branch(!self.flag(SFR_S), mem), // bpl
            0x0b => self.i_branch(self.flag(SFR_S), mem),  // bmi
            0x0c => self.i_branch(!self.flag(SFR_CY), mem), // bcc
            0x0d => self.i_branch(self.flag(SFR_CY), mem), // bcs
            0x0e => self.i_branch(!self.flag(SFR_OV), mem), // bvc
            0x0f => self.i_branch(self.flag(SFR_OV), mem), // bvs
            0x10..=0x1f => self.i_to_move(n),              // to/move
            0x20..=0x2f => self.i_with(n),                 // with
            0x30..=0x3b => self.i_store(n, mem),           // stw/stb
            0x3c => self.i_loop(),                         // loop
            0x3d => self.i_alt1(),
            0x3e => self.i_alt2(),
            0x3f => self.i_alt3(),
            0x40..=0x4b => self.i_load(n, mem),        // ldw/ldb
            0x4c => self.i_plot_rpix(mem),             // plot/rpix
            0x4d => self.i_swap(),                     // swap
            0x4e => self.i_color_cmode(),              // color/cmode
            0x4f => self.i_not(),                      // not
            0x50..=0x5f => self.i_add_adc(n, mem),     // add/adc
            0x60..=0x6f => self.i_sub_sbc_cmp(n, mem), // sub/sbc/cmp
            0x70 => self.i_merge(),                    // merge
            0x71..=0x7f => self.i_and_bic(n, mem),     // and/bic
            0x80..=0x8f => self.i_mult_umult(n, mem),  // mult/umult
            0x90 => self.i_sbk(mem),                   // sbk
            0x91..=0x94 => self.i_link(n),             // link
            0x95 => self.i_sex(),                      // sex
            0x96 => self.i_asr_div2(),                 // asr/div2
            0x97 => self.i_ror(),                      // ror
            0x98..=0x9d => self.i_jmp_ljmp(n),         // jmp/ljmp
            0x9e => self.i_lob(),                      // lob
            0x9f => self.i_fmult_lmult(mem),           // fmult/lmult
            0xa0..=0xaf => self.i_ibt_lms_sms(n, mem), // ibt/lms/sms
            0xb0..=0xbf => self.i_from_moves(n),       // from/moves
            0xc0 => self.i_hib(),                      // hib
            0xc1..=0xcf => self.i_or_xor(n, mem),      // or/xor
            0xd0..=0xde => self.i_inc(n),              // inc
            0xdf => self.i_getc_ramb_romb(mem),        // getc/ramb/romb
            0xe0..=0xee => self.i_dec(n),              // dec
            0xef => self.i_getb(mem),                  // getb
            0xf0..=0xff => self.i_iwt_lm_sm(n, mem),   // iwt/lm/sm
        }
    }

    fn set_sz_from(&mut self, value: u16) {
        self.set_flag(SFR_S, value & 0x8000 != 0);
        self.set_flag(SFR_Z, value == 0);
    }

    // $00 stop
    fn i_stop(&mut self) {
        if !self.cfgr_irq {
            self.set_flag(SFR_IRQ, true);
        }
        self.set_flag(SFR_G, false);
        self.pipeline = 0x01;
        self.reset_prefix();
    }

    // $02 cache
    fn i_cache(&mut self) {
        if self.cbr != (self.r[15] & 0xfff0) {
            self.cbr = self.r[15] & 0xfff0;
            self.flush_cache();
        }
        self.reset_prefix();
    }

    // $03 lsr
    fn i_lsr(&mut self) {
        self.set_flag(SFR_CY, self.sr() & 1 != 0);
        let r = self.sr() >> 1;
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
    }

    // $04 rol
    fn i_rol(&mut self) {
        let carry = self.sr() & 0x8000 != 0;
        let r = (self.sr() << 1) | u16::from(self.flag(SFR_CY));
        self.set_dr(r);
        self.set_flag(SFR_S, r & 0x8000 != 0);
        self.set_flag(SFR_CY, carry);
        self.set_flag(SFR_Z, r == 0);
        self.reset_prefix();
    }

    // $05-$0f branch (delay slot via the pipeline)
    fn i_branch(&mut self, take: bool, mem: &mut GsuMem) {
        let displacement = self.pipe(mem) as i8;
        if take {
            self.r[15] = self.r[15].wrapping_add(displacement as u16);
            self.r15_mod = true;
        }
    }

    // $10-$1f to rN / move rN
    fn i_to_move(&mut self, n: usize) {
        if !self.flag(SFR_B) {
            self.dreg = n;
        } else {
            let v = self.sr();
            self.write_r(n, v);
            self.reset_prefix();
        }
    }

    // $20-$2f with rN
    fn i_with(&mut self, n: usize) {
        self.sreg = n;
        self.dreg = n;
        self.set_flag(SFR_B, true);
    }

    // $30-$3b stw (rN) / stb (rN)
    fn i_store(&mut self, n: usize, mem: &mut GsuMem) {
        self.ramaddr = self.r[n];
        let s = self.sr();
        self.write_ram_buffer(mem, self.ramaddr, s as u8);
        if !self.alt1() {
            self.write_ram_buffer(mem, self.ramaddr ^ 1, (s >> 8) as u8);
        }
        self.reset_prefix();
    }

    // $3c loop
    fn i_loop(&mut self) {
        self.r[12] = self.r[12].wrapping_sub(1);
        self.set_flag(SFR_S, self.r[12] & 0x8000 != 0);
        self.set_flag(SFR_Z, self.r[12] == 0);
        if !self.flag(SFR_Z) {
            self.r[15] = self.r[13];
            self.r15_mod = true;
        }
        self.reset_prefix();
    }

    // $3d/$3e/$3f alt prefixes
    fn i_alt1(&mut self) {
        self.set_flag(SFR_B, false);
        self.set_flag(SFR_ALT1, true);
    }
    fn i_alt2(&mut self) {
        self.set_flag(SFR_B, false);
        self.set_flag(SFR_ALT2, true);
    }
    fn i_alt3(&mut self) {
        self.set_flag(SFR_B, false);
        self.set_flag(SFR_ALT1, true);
        self.set_flag(SFR_ALT2, true);
    }

    // $40-$4b ldw (rN) / ldb (rN)
    fn i_load(&mut self, n: usize, mem: &mut GsuMem) {
        self.ramaddr = self.r[n];
        let mut v = u16::from(self.read_ram_buffer(mem, self.ramaddr));
        if !self.alt1() {
            v |= u16::from(self.read_ram_buffer(mem, self.ramaddr ^ 1)) << 8;
        }
        self.set_dr(v);
        self.reset_prefix();
    }

    // $4c plot / rpix
    fn i_plot_rpix(&mut self, mem: &mut GsuMem) {
        if !self.alt1() {
            self.plot(self.r[1] as u8, self.r[2] as u8, mem);
            self.r[1] = self.r[1].wrapping_add(1);
        } else {
            let v = u16::from(self.rpix(self.r[1] as u8, self.r[2] as u8, mem));
            self.set_dr(v);
            self.set_sz_from(v);
        }
        self.reset_prefix();
    }

    /// Plot a pixel at `(x, y)` into the pixel cache (ares `SuperFX::plot`), evicting a completed
    /// strip to the Game Pak RAM via `mem`.
    fn plot(&mut self, x: u8, y: u8, mem: &mut GsuMem) {
        if !self.por_transparent {
            let transparent = if self.scmr_md == 3 {
                if self.por_freezehigh {
                    self.colr & 0x0f == 0
                } else {
                    self.colr == 0
                }
            } else {
                self.colr & 0x0f == 0
            };
            if transparent {
                return;
            }
        }

        let mut color = self.colr;
        if self.por_dither && self.scmr_md != 3 {
            if (x ^ y) & 1 != 0 {
                color >>= 4;
            }
            color &= 0x0f;
        }

        let offset = (u16::from(y) << 5) + (u16::from(x) >> 3);
        if offset != self.pixelcache[0].offset {
            let evicted = self.pixelcache[1];
            self.flush_pixel_cache_into(evicted, mem);
            self.pixelcache[1] = self.pixelcache[0];
            self.pixelcache[0].bitpend = 0;
            self.pixelcache[0].offset = offset;
        }

        let xi = ((x & 7) ^ 7) as usize;
        self.pixelcache[0].data[xi] = color;
        self.pixelcache[0].bitpend |= 1 << xi;
        if self.pixelcache[0].bitpend == 0xff {
            let evicted = self.pixelcache[1];
            self.flush_pixel_cache_into(evicted, mem);
            self.pixelcache[1] = self.pixelcache[0];
            self.pixelcache[0].bitpend = 0;
        }
    }

    // $4d swap
    fn i_swap(&mut self) {
        let r = (self.sr() >> 8) | (self.sr() << 8);
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
    }

    // $4e color / cmode
    fn i_color_cmode(&mut self) {
        if self.alt1() {
            self.set_por(self.sr() as u8);
        } else {
            self.colr = self.color(self.sr() as u8);
        }
        self.reset_prefix();
    }

    /// Decode the POR (plot-option register) byte (ares `POR::operator=`).
    fn set_por(&mut self, data: u8) {
        self.por_obj = data & 0x10 != 0;
        self.por_freezehigh = data & 0x08 != 0;
        self.por_highnibble = data & 0x04 != 0;
        self.por_dither = data & 0x02 != 0;
        self.por_transparent = data & 0x01 != 0;
    }

    // $4f not
    fn i_not(&mut self) {
        let r = !self.sr();
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
    }

    // $50-$5f add/adc rN/#N
    fn i_add_adc(&mut self, n: usize, _mem: &mut GsuMem) {
        let operand: i32 = if self.alt2() {
            n as i32
        } else {
            i32::from(self.r[n])
        };
        let sr = i32::from(self.sr());
        let carry = if self.alt1() {
            i32::from(self.flag(SFR_CY))
        } else {
            0
        };
        let r = sr + operand + carry;
        self.set_flag(SFR_OV, !(sr ^ operand) & (operand ^ r) & 0x8000 != 0);
        self.set_flag(SFR_S, r & 0x8000 != 0);
        self.set_flag(SFR_CY, r >= 0x10000);
        self.set_flag(SFR_Z, (r as u16) == 0);
        self.set_dr(r as u16);
        self.reset_prefix();
    }

    // $60-$6f sub/sbc/cmp
    fn i_sub_sbc_cmp(&mut self, n: usize, _mem: &mut GsuMem) {
        let operand: i32 = if !self.alt2() || self.alt1() {
            i32::from(self.r[n])
        } else {
            n as i32
        };
        let sr = i32::from(self.sr());
        let borrow = if !self.alt2() && self.alt1() {
            i32::from(!self.flag(SFR_CY))
        } else {
            0
        };
        let r = sr - operand - borrow;
        self.set_flag(SFR_OV, (sr ^ operand) & (sr ^ r) & 0x8000 != 0);
        self.set_flag(SFR_S, r & 0x8000 != 0);
        self.set_flag(SFR_CY, r >= 0);
        self.set_flag(SFR_Z, (r as u16) == 0);
        if !self.alt2() || !self.alt1() {
            self.set_dr(r as u16);
        }
        self.reset_prefix();
    }

    // $70 merge
    fn i_merge(&mut self) {
        let r = (self.r[7] & 0xff00) | (self.r[8] >> 8);
        self.set_dr(r);
        self.set_flag(SFR_OV, r & 0xc0c0 != 0);
        self.set_flag(SFR_S, r & 0x8080 != 0);
        self.set_flag(SFR_CY, r & 0xe0e0 != 0);
        self.set_flag(SFR_Z, r & 0xf0f0 != 0);
        self.reset_prefix();
    }

    // $71-$7f and/bic
    fn i_and_bic(&mut self, n: usize, _mem: &mut GsuMem) {
        let operand = if self.alt2() { n as u16 } else { self.r[n] };
        let operand = if self.alt1() { !operand } else { operand };
        let r = self.sr() & operand;
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
    }

    // $80-$8f mult/umult
    fn i_mult_umult(&mut self, n: usize, mem: &mut GsuMem) {
        let operand = if self.alt2() { n as u16 } else { self.r[n] };
        let r = if self.alt1() {
            u16::from(self.sr() as u8).wrapping_mul(u16::from(operand as u8))
        } else {
            ((self.sr() as i8 as i16).wrapping_mul(operand as i8 as i16)) as u16
        };
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
        if !self.cfgr_ms0 {
            self.step(if self.clsr { 1 } else { 2 }, mem);
        }
    }

    // $90 sbk
    fn i_sbk(&mut self, mem: &mut GsuMem) {
        let s = self.sr();
        self.write_ram_buffer(mem, self.ramaddr, s as u8);
        self.write_ram_buffer(mem, self.ramaddr ^ 1, (s >> 8) as u8);
        self.reset_prefix();
    }

    // $91-$94 link #N
    fn i_link(&mut self, n: usize) {
        self.r[11] = self.r[15].wrapping_add(n as u16);
        self.reset_prefix();
    }

    // $95 sex
    fn i_sex(&mut self) {
        let r = (self.sr() as i8 as i16) as u16;
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
    }

    // $96 asr/div2
    fn i_asr_div2(&mut self) {
        // ares: dr = ((i16)sr >> 1) + (alt1 ? ((sr + 1) >> 16) : 0). The 32-bit `(sr + 1) >> 16`
        // term is the div2 round-toward-zero correction; it is 0 for any 16-bit `sr` except the
        // negative-odd boundary, so the faithful 32-bit form is kept verbatim.
        self.set_flag(SFR_CY, self.sr() & 1 != 0);
        let correction = if self.alt1() {
            (i32::from(self.sr()) + 1) >> 16
        } else {
            0
        };
        let r = ((i32::from(self.sr() as i16) >> 1) + correction) as u16;
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
    }

    // $97 ror
    fn i_ror(&mut self) {
        let carry = self.sr() & 1 != 0;
        let r = (u16::from(self.flag(SFR_CY)) << 15) | (self.sr() >> 1);
        self.set_dr(r);
        self.set_flag(SFR_S, r & 0x8000 != 0);
        self.set_flag(SFR_CY, carry);
        self.set_flag(SFR_Z, r == 0);
        self.reset_prefix();
    }

    // $98-$9d jmp/ljmp
    fn i_jmp_ljmp(&mut self, n: usize) {
        if !self.alt1() {
            self.r[15] = self.r[n];
            self.r15_mod = true;
        } else {
            self.pbr = (self.r[n] & 0x7f) as u8;
            self.r[15] = self.sr();
            self.r15_mod = true;
            self.cbr = self.r[15] & 0xfff0;
            self.flush_cache();
        }
        self.reset_prefix();
    }

    // $9e lob
    fn i_lob(&mut self) {
        let r = self.sr() & 0xff;
        self.set_dr(r);
        self.set_flag(SFR_S, r & 0x80 != 0);
        self.set_flag(SFR_Z, r == 0);
        self.reset_prefix();
    }

    // $9f fmult/lmult
    fn i_fmult_lmult(&mut self, mem: &mut GsuMem) {
        let result = ((self.sr() as i16 as i32) * (self.r[6] as i16 as i32)) as u32;
        if self.alt1() {
            self.r[4] = result as u16;
        }
        let r = (result >> 16) as u16;
        self.set_dr(r);
        self.set_flag(SFR_S, r & 0x8000 != 0);
        self.set_flag(SFR_CY, result & 0x8000 != 0);
        self.set_flag(SFR_Z, r == 0);
        self.reset_prefix();
        let mul = if self.cfgr_ms0 { 3 } else { 7 };
        self.step(mul * if self.clsr { 1 } else { 2 }, mem);
    }

    // $a0-$af ibt/lms/sms
    fn i_ibt_lms_sms(&mut self, n: usize, mem: &mut GsuMem) {
        if self.alt1() {
            self.ramaddr = u16::from(self.pipe(mem)) << 1;
            let lo = u16::from(self.read_ram_buffer(mem, self.ramaddr));
            let hi = u16::from(self.read_ram_buffer(mem, self.ramaddr ^ 1)) << 8;
            self.write_r(n, hi | lo);
        } else if self.alt2() {
            self.ramaddr = u16::from(self.pipe(mem)) << 1;
            let v = self.r[n];
            self.write_ram_buffer(mem, self.ramaddr, v as u8);
            self.write_ram_buffer(mem, self.ramaddr ^ 1, (v >> 8) as u8);
        } else {
            let imm = (self.pipe(mem) as i8 as i16) as u16;
            self.write_r(n, imm);
        }
        self.reset_prefix();
    }

    // $b0-$bf from/moves
    fn i_from_moves(&mut self, n: usize) {
        if !self.flag(SFR_B) {
            self.sreg = n;
        } else {
            let v = self.r[n];
            self.set_dr(v);
            self.set_flag(SFR_OV, v & 0x80 != 0);
            self.set_flag(SFR_S, v & 0x8000 != 0);
            self.set_flag(SFR_Z, v == 0);
            self.reset_prefix();
        }
    }

    // $c0 hib
    fn i_hib(&mut self) {
        let r = self.sr() >> 8;
        self.set_dr(r);
        self.set_flag(SFR_S, r & 0x80 != 0);
        self.set_flag(SFR_Z, r == 0);
        self.reset_prefix();
    }

    // $c1-$cf or/xor
    fn i_or_xor(&mut self, n: usize, _mem: &mut GsuMem) {
        let operand = if self.alt2() { n as u16 } else { self.r[n] };
        let r = if self.alt1() {
            self.sr() ^ operand
        } else {
            self.sr() | operand
        };
        self.set_dr(r);
        self.set_sz_from(r);
        self.reset_prefix();
    }

    // $d0-$de inc
    fn i_inc(&mut self, n: usize) {
        let v = self.r[n].wrapping_add(1);
        self.write_r(n, v);
        self.set_sz_from(v);
        self.reset_prefix();
    }

    // $df getc/ramb/romb
    fn i_getc_ramb_romb(&mut self, mem: &mut GsuMem) {
        if !self.alt2() {
            let c = self.read_rom_buffer(mem);
            self.colr = self.color(c);
        } else if !self.alt1() {
            self.sync_ram_buffer(mem);
            self.rambr = self.sr() & 0x01 != 0;
        } else {
            self.sync_rom_buffer(mem);
            self.rombr = (self.sr() & 0x7f) as u8;
        }
        self.reset_prefix();
    }

    // $e0-$ee dec
    fn i_dec(&mut self, n: usize) {
        let v = self.r[n].wrapping_sub(1);
        self.write_r(n, v);
        self.set_sz_from(v);
        self.reset_prefix();
    }

    // $ef getb/getbh/getbl/getbs
    fn i_getb(&mut self, mem: &mut GsuMem) {
        let rb = u16::from(self.read_rom_buffer(mem));
        let mode = (u8::from(self.alt2()) << 1) | u8::from(self.alt1());
        let r = match mode {
            0 => rb,
            1 => (rb << 8) | (self.sr() & 0xff),
            2 => (self.sr() & 0xff00) | rb,
            _ => (rb as u8 as i8 as i16) as u16,
        };
        self.set_dr(r);
        self.reset_prefix();
    }

    // $f0-$ff iwt/lm/sm
    fn i_iwt_lm_sm(&mut self, n: usize, mem: &mut GsuMem) {
        if self.alt1() {
            let mut addr = u16::from(self.pipe(mem));
            addr |= u16::from(self.pipe(mem)) << 8;
            self.ramaddr = addr;
            let lo = u16::from(self.read_ram_buffer(mem, addr));
            let hi = u16::from(self.read_ram_buffer(mem, addr ^ 1)) << 8;
            self.write_r(n, hi | lo);
        } else if self.alt2() {
            let mut addr = u16::from(self.pipe(mem));
            addr |= u16::from(self.pipe(mem)) << 8;
            self.ramaddr = addr;
            let v = self.r[n];
            self.write_ram_buffer(mem, addr, v as u8);
            self.write_ram_buffer(mem, addr ^ 1, (v >> 8) as u8);
        } else {
            let lo = u16::from(self.pipe(mem));
            let hi = u16::from(self.pipe(mem)) << 8;
            self.write_r(n, hi | lo);
        }
        self.reset_prefix();
    }

    /// Bound on `pending_clocks`' saved length: `step` pushes at most a handful of bus-access
    /// checkpoints per instruction, so a claimed length beyond this is corrupt/hostile input, not
    /// a value real execution could ever produce (the zip-bomb-style "reject an absurd claimed
    /// size" posture, not a hardware width mask).
    const MAX_SAVED_PENDING_CLOCKS: usize = 64;

    /// Write this core's full mutable state — every register, the status/control fields, both
    /// bus-buffer latches, the opcode cache, the plot pixel cache, the liveness counters, and the
    /// in-flight per-access checkpoint queue (`pending_clocks`/`pending_idx`/`owed`) — into a
    /// `"GSU0"` section. There is no firmware/ROM byte here to exclude: the GSU's program lives in
    /// the cart's own ROM, which `System::save_state` captures separately (`docs/adr/0003`). The
    /// checkpoint queue matters because [`Self::tick`]-driven (master-clock-interleaved) execution
    /// can leave a `Go` burst genuinely mid-flight at any save point, unlike the run-to-completion
    /// [`Self::run_until_stopped`] path, which always drains it back to empty first.
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"GSU0", |s| {
            for &reg in &self.r {
                s.write_u16(reg);
            }
            s.write_bool(self.r14_mod);
            s.write_bool(self.r15_mod);
            s.write_u16(self.sfr);
            #[allow(clippy::cast_possible_truncation)] // sreg/dreg are always 0-15
            s.write_u8(self.sreg as u8);
            #[allow(clippy::cast_possible_truncation)]
            s.write_u8(self.dreg as u8);
            s.write_u8(self.pipeline);
            s.write_u8(self.pbr);
            s.write_u8(self.rombr);
            s.write_bool(self.rambr);
            s.write_u16(self.cbr);
            s.write_u8(self.scbr);
            s.write_u8(self.scmr_ht);
            s.write_bool(self.scmr_ron);
            s.write_bool(self.scmr_ran);
            s.write_u8(self.scmr_md);
            s.write_u8(self.colr);
            s.write_bool(self.por_obj);
            s.write_bool(self.por_freezehigh);
            s.write_bool(self.por_highnibble);
            s.write_bool(self.por_dither);
            s.write_bool(self.por_transparent);
            s.write_bool(self.bramr);
            s.write_u8(self.vcr);
            s.write_bool(self.cfgr_irq);
            s.write_bool(self.cfgr_ms0);
            s.write_bool(self.clsr);
            s.write_u32(self.romcl);
            s.write_u8(self.romdr);
            s.write_u32(self.ramcl);
            s.write_u16(self.ramar);
            s.write_u8(self.ramdr);
            s.write_u16(self.ramaddr);
            s.write_bytes(&*self.cache_buffer);
            for &valid in &self.cache_valid {
                s.write_bool(valid);
            }
            for pc in &self.pixelcache {
                s.write_u16(pc.offset);
                s.write_u8(pc.bitpend);
                s.write_bytes(&pc.data);
            }
            s.write_u64(self.clocks);
            s.write_u64(self.instructions);
            #[allow(clippy::cast_possible_truncation)] // bounded by MAX_SAVED_PENDING_CLOCKS
            s.write_u32(self.pending_clocks.len() as u32);
            for &c in &self.pending_clocks {
                s.write_u32(c);
            }
            #[allow(clippy::cast_possible_truncation)] // <= pending_clocks.len(), same bound
            s.write_u32(self.pending_idx as u32);
            s.write_u32(self.owed);
        });
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input, a section with unconsumed trailing bytes, or
    /// [`SaveStateError::Invalid`] if the saved `pending_clocks` length exceeds
    /// `MAX_SAVED_PENDING_CLOCKS` or `pending_idx` exceeds the restored queue's length
    /// (both would otherwise let a corrupted save-state desync [`Self::step_one`]'s cursor).
    /// `sreg`/`dreg` are masked to 4 bits — they index the 16-entry register file directly.
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"GSU0")?;
        for reg in &mut self.r {
            *reg = s.read_u16()?;
        }
        self.r14_mod = s.read_bool()?;
        self.r15_mod = s.read_bool()?;
        self.sfr = s.read_u16()?;
        self.sreg = usize::from(s.read_u8()? & 0xF);
        self.dreg = usize::from(s.read_u8()? & 0xF);
        self.pipeline = s.read_u8()?;
        self.pbr = s.read_u8()?;
        self.rombr = s.read_u8()?;
        self.rambr = s.read_bool()?;
        self.cbr = s.read_u16()?;
        self.scbr = s.read_u8()?;
        self.scmr_ht = s.read_u8()?;
        self.scmr_ron = s.read_bool()?;
        self.scmr_ran = s.read_bool()?;
        self.scmr_md = s.read_u8()?;
        self.colr = s.read_u8()?;
        self.por_obj = s.read_bool()?;
        self.por_freezehigh = s.read_bool()?;
        self.por_highnibble = s.read_bool()?;
        self.por_dither = s.read_bool()?;
        self.por_transparent = s.read_bool()?;
        self.bramr = s.read_bool()?;
        self.vcr = s.read_u8()?;
        self.cfgr_irq = s.read_bool()?;
        self.cfgr_ms0 = s.read_bool()?;
        self.clsr = s.read_bool()?;
        self.romcl = s.read_u32()?;
        self.romdr = s.read_u8()?;
        self.ramcl = s.read_u32()?;
        self.ramar = s.read_u16()?;
        self.ramdr = s.read_u8()?;
        self.ramaddr = s.read_u16()?;
        self.cache_buffer.copy_from_slice(s.read_bytes(512)?);
        for valid in &mut self.cache_valid {
            *valid = s.read_bool()?;
        }
        for pc in &mut self.pixelcache {
            pc.offset = s.read_u16()?;
            pc.bitpend = s.read_u8()?;
            pc.data.copy_from_slice(s.read_bytes(8)?);
        }
        self.clocks = s.read_u64()?;
        self.instructions = s.read_u64()?;
        let pending_len = s.read_u32()? as usize;
        if pending_len > Self::MAX_SAVED_PENDING_CLOCKS {
            return Err(SaveStateError::Invalid(alloc::format!(
                "GSU pending_clocks length {pending_len} exceeds the sane bound of {}",
                Self::MAX_SAVED_PENDING_CLOCKS
            )));
        }
        self.pending_clocks.clear();
        for _ in 0..pending_len {
            self.pending_clocks.push(s.read_u32()?);
        }
        let pending_idx = s.read_u32()? as usize;
        if pending_idx > self.pending_clocks.len() {
            return Err(SaveStateError::Invalid(alloc::format!(
                "GSU pending_idx {pending_idx} exceeds the restored queue length {}",
                self.pending_clocks.len()
            )));
        }
        self.pending_idx = pending_idx;
        self.owed = s.read_u32()?;
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "GSU0 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        Ok(())
    }
}
