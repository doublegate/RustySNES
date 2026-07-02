//! The NEC µPD77C25 / µPD96050 LLE engine — the shared NEC DSP core.
//!
//! One engine backs **six** SNES coprocessors (`docs/cart.md` §"the shared NEC core"): the
//! `uPD7725` revision is the DSP-1/1A/1B + DSP-2/3/4 chip; the `uPD96050` revision is the
//! ST010 / ST011. Each chip differs only in its firmware (program ROM + data ROM) and its
//! register widths, so this is a single firmware-parameterized core — implement it once, drive
//! each chip's dumped program/data ROM through it.
//!
//! This is a clean-room re-implementation of ares' `uPD96050` component (ISC) in safe
//! `no_std` Rust: the NEC DSP instruction word is a hardware fact, and the decode here mirrors
//! the published instruction encoding (OP / RT / JP / LD), not ares' source layout. The host
//! interface (the DR data register, the SR status register, and the DP data-RAM port) is the
//! memory-mapped surface the SNES CPU sees.
//!
//! ## Host synchronization model
//!
//! The real chip free-runs on its own ~7.6 MHz oscillator and hand-shakes the SNES CPU purely
//! through the **RQM** ("request for master") status bit: the DSP raises RQM when it wants the
//! host to service the data register, the host's access clears it, and the DSP spins on a
//! `JRQM`/`JNRQM` wait loop until serviced. Because RQM is the *only* observable coupling
//! between the two clocks (DSP-1 games always poll `SR.rqm`, never a wall-clock cycle count),
//! [`Upd77c25::run_until_rqm`] advances the engine to its next parked state after every host
//! data-register access. This keeps the bus boundary byte-exact and fully deterministic
//! (`docs/adr/0004`) without a free-running per-master-clock tick.

// The NEC DSP treats every register as both a signed and an unsigned view of the same 16 bits,
// so the faithful port is dense with deliberate bit-pattern casts; the status/condition-flag
// registers are naturally bitfields of single-bit flags; and the chip-name jargon (µPD77C25,
// uPD7725, …) is not Rust code. These lints are noise for hardware-register emulation here.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::similar_names,
    clippy::verbose_bit_mask,
    clippy::doc_markdown,
    clippy::missing_const_for_fn
)]

use alloc::boxed::Box;
use alloc::vec;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

/// Which NEC DSP variant the firmware targets — selects the register widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision {
    /// µPD7725 — DSP-1/1A/1B, DSP-2, DSP-3, DSP-4. 2 K×24 program, 1 K×16 data ROM.
    Upd7725,
    /// µPD96050 — ST010, ST011. 16 K×24 program, 2 K×16 data ROM, 2 K×16 data RAM.
    Upd96050,
}

impl Revision {
    /// Program-counter width mask (program-ROM address space).
    const fn pc_mask(self) -> u16 {
        match self {
            Self::Upd7725 => 0x07FF,  // 11-bit → 2048 words
            Self::Upd96050 => 0x3FFF, // 14-bit → 16384 words
        }
    }

    /// ROM-pointer width mask (data-ROM address space).
    const fn rp_mask(self) -> u16 {
        match self {
            Self::Upd7725 => 0x03FF,  // 10-bit → 1024 words
            Self::Upd96050 => 0x07FF, // 11-bit → 2048 words
        }
    }

    /// Data-pointer width mask (data-RAM address space, internal instruction view).
    const fn dp_mask(self) -> u16 {
        match self {
            Self::Upd7725 => 0x00FF,  // 8-bit → 256 words
            Self::Upd96050 => 0x07FF, // 11-bit → 2048 words
        }
    }

    /// Program-ROM word count.
    const fn program_words(self) -> usize {
        match self {
            Self::Upd7725 => 2048,
            Self::Upd96050 => 16384,
        }
    }

    /// Data-ROM word count.
    const fn data_words(self) -> usize {
        match self {
            Self::Upd7725 => 1024,
            Self::Upd96050 => 2048,
        }
    }
}

/// The six condition/ALU flags carried per accumulator (`flags.a` / `flags.b`).
#[derive(Debug, Clone, Copy, Default)]
struct Flag {
    ov0: bool, // overflow 0
    ov1: bool, // overflow 1
    z: bool,   // zero
    c: bool,   // carry
    s0: bool,  // sign 0
    s1: bool,  // sign 1
}

impl Flag {
    fn to_bits(self) -> u16 {
        u16::from(self.ov0)
            | u16::from(self.ov1) << 1
            | u16::from(self.z) << 2
            | u16::from(self.c) << 3
            | u16::from(self.s0) << 4
            | u16::from(self.s1) << 5
    }
}

/// The 16-bit status register (SR). Only the high byte is exposed to the host on a read.
#[derive(Debug, Clone, Copy, Default)]
struct Status {
    p0: bool,   // output port 0
    p1: bool,   // output port 1
    ei: bool,   // enable interrupts
    sic: bool,  // serial input control
    soc: bool,  // serial output control
    drc: bool,  // data-register size (0 = 16-bit, 1 = 8-bit)
    dma: bool,  // data-register DMA mode
    drs: bool,  // data-register status (1 = active)
    usf0: bool, // user flag 0
    usf1: bool, // user flag 1
    rqm: bool,  // request for master
    siack: bool,
    soack: bool,
}

impl Status {
    fn to_bits(self) -> u16 {
        let drs = self.drs && !self.drc; // when DRC=1, DRS reads 0
        u16::from(self.p0)
            | u16::from(self.p1) << 1
            | u16::from(self.ei) << 7
            | u16::from(self.sic) << 8
            | u16::from(self.soc) << 9
            | u16::from(self.drc) << 10
            | u16::from(self.dma) << 11
            | u16::from(drs) << 12
            | u16::from(self.usf0) << 13
            | u16::from(self.usf1) << 14
            | u16::from(self.rqm) << 15
    }

    fn set_bits(&mut self, data: u16) {
        self.p0 = data & 1 != 0;
        self.p1 = data & 1 << 1 != 0;
        self.ei = data & 1 << 7 != 0;
        self.sic = data & 1 << 8 != 0;
        self.soc = data & 1 << 9 != 0;
        self.drc = data & 1 << 10 != 0;
        self.dma = data & 1 << 11 != 0;
        self.drs = data & 1 << 12 != 0;
        self.usf0 = data & 1 << 13 != 0;
        self.usf1 = data & 1 << 14 != 0;
        self.rqm = data & 1 << 15 != 0;
    }
}

/// The NEC µPD77C25 / µPD96050 LLE engine.
///
/// Construct with [`Upd77c25::new`], load a dumped firmware image with
/// [`Upd77c25::load_firmware`], then drive the host interface through the DR/SR/DP methods. The
/// engine is inert (open-bus reads) until a firmware image is loaded — the honesty posture of
/// `docs/adr/0003`: a chip-ROM-dump-dependent coprocessor is non-functional without the dump,
/// never silently degraded.
#[derive(Debug, Clone)]
pub struct Upd77c25 {
    revision: Revision,
    firmware_loaded: bool,
    /// Count of host DR/SR port accesses (diagnostics / the debugger).
    host_accesses: u64,

    program_rom: Box<[u32]>,
    data_rom: Box<[u16]>,
    data_ram: Box<[u16]>,

    stack: [u16; 16],
    pc: u16,
    rp: u16,
    dp: u16,
    sp: u8,
    si: u16,
    so: u16,
    k: i16,
    l: i16,
    m: i16,
    n: i16,
    a: i16,
    b: i16,
    tr: u16,
    trb: u16,
    dr: u16,
    sr: Status,
    flag_a: Flag,
    flag_b: Flag,
}

impl Upd77c25 {
    /// A hard cap on instructions executed per host-driven catch-up, so a wedged firmware can
    /// never hang the emulator. A single DSP-1 command settles in well under this.
    const RUN_CAP: u32 = 0x10_0000;

    /// Construct a powered-on engine for `revision` with zeroed (unloaded) firmware.
    #[must_use]
    pub fn new(revision: Revision) -> Self {
        let program_rom = vec![0u32; revision.program_words()].into_boxed_slice();
        let data_rom = vec![0u16; revision.data_words()].into_boxed_slice();
        // ares sizes the data RAM at 2048 words for both revisions; the instruction view masks
        // it to the revision's DP width. Keep the full array and mask host accesses to it.
        let data_ram = vec![0u16; 2048].into_boxed_slice();
        let mut me = Self {
            revision,
            firmware_loaded: false,
            host_accesses: 0,
            program_rom,
            data_rom,
            data_ram,
            stack: [0; 16],
            pc: 0,
            rp: 0,
            dp: 0,
            sp: 0,
            si: 0,
            so: 0,
            k: 0,
            l: 0,
            m: 0,
            n: 0,
            a: 0,
            b: 0,
            tr: 0,
            trb: 0,
            dr: 0,
            sr: Status::default(),
            flag_a: Flag::default(),
            flag_b: Flag::default(),
        };
        me.power();
        me
    }

    /// Reset all programmer-visible state (NEC power-on). Firmware contents are retained.
    pub fn power(&mut self) {
        self.stack = [0; 16];
        self.pc = 0;
        self.rp = 0;
        self.dp = 0;
        self.sp = 0;
        self.si = 0;
        self.so = 0;
        self.k = 0;
        self.l = 0;
        self.m = 0;
        self.n = 0;
        self.a = 0;
        self.b = 0;
        self.tr = 0;
        self.trb = 0;
        self.dr = 0;
        self.sr = Status::default();
        self.flag_a = Flag::default();
        self.flag_b = Flag::default();
    }

    /// Whether a firmware image has been loaded (the chip is functional).
    #[must_use]
    pub const fn firmware_loaded(&self) -> bool {
        self.firmware_loaded
    }

    /// Load a packed NEC DSP firmware dump.
    ///
    /// The dump is the program ROM (one little-endian 24-bit word per instruction) immediately
    /// followed by the data ROM (one little-endian 16-bit word per entry) — exactly ares'
    /// `upd7725.program.rom` + `upd7725.data.rom` concatenation. For the µPD7725 (DSP-1) that is
    /// `2048×3 + 1024×2 = 8192` bytes (`dsp1.rom` / `dsp1b.rom`).
    ///
    /// Returns `false` (and loads nothing) if `bytes` is too short for this revision. On success
    /// the engine is powered on and run to its first parked (RQM-set) state, ready for the first
    /// host command.
    pub fn load_firmware(&mut self, bytes: &[u8]) -> bool {
        let prog_bytes = self.program_rom.len() * 3;
        let data_bytes = self.data_rom.len() * 2;
        if bytes.len() < prog_bytes + data_bytes {
            return false;
        }
        for (i, word) in self.program_rom.iter_mut().enumerate() {
            let o = i * 3;
            *word =
                u32::from(bytes[o]) | u32::from(bytes[o + 1]) << 8 | u32::from(bytes[o + 2]) << 16;
        }
        for (i, word) in self.data_rom.iter_mut().enumerate() {
            let o = prog_bytes + i * 2;
            *word = u16::from(bytes[o]) | u16::from(bytes[o + 1]) << 8;
        }
        self.firmware_loaded = true;
        self.power();
        // Prime the engine to its first "ready for a command" parked state.
        self.run_until_rqm();
        true
    }

    // --- Host interface (the memory-mapped DR / SR / DP surface). ---

    /// Read the status register (the high byte the host sees at the SR port).
    #[must_use]
    pub fn read_sr(&self) -> u8 {
        if !self.firmware_loaded {
            return 0;
        }
        (self.sr.to_bits() >> 8) as u8
    }

    /// Write the status register port. The NEC chip ignores host SR writes (no-op), matching
    /// hardware; kept for surface symmetry.
    pub fn write_sr(&mut self, _data: u8) {}

    /// Read a byte from the data register, then catch the engine up to its next parked state.
    #[must_use]
    pub fn read_dr(&mut self) -> u8 {
        if !self.firmware_loaded {
            return 0;
        }
        self.host_accesses += 1;
        let value = if self.sr.drc {
            // 8-bit transfer.
            self.sr.rqm = false;
            self.dr as u8
        } else if self.sr.drs {
            // 16-bit transfer, high byte (completes the word).
            self.sr.rqm = false;
            self.sr.drs = false;
            (self.dr >> 8) as u8
        } else {
            // 16-bit transfer, low byte (begins the word).
            self.sr.drs = true;
            self.dr as u8
        };
        self.run_until_rqm();
        value
    }

    /// Write a byte to the data register, then catch the engine up to its next parked state.
    pub fn write_dr(&mut self, data: u8) {
        if !self.firmware_loaded {
            return;
        }
        self.host_accesses += 1;
        if self.sr.drc {
            // 8-bit transfer.
            self.sr.rqm = false;
            self.dr = (self.dr & 0xFF00) | u16::from(data);
        } else if self.sr.drs {
            // 16-bit transfer, high byte (completes the word).
            self.sr.rqm = false;
            self.sr.drs = false;
            self.dr = u16::from(data) << 8 | (self.dr & 0x00FF);
        } else {
            // 16-bit transfer, low byte (begins the word).
            self.sr.drs = true;
            self.dr = (self.dr & 0xFF00) | u16::from(data);
        }
        self.run_until_rqm();
    }

    /// Read a byte from the data-RAM host port (`address` is the byte address into the window).
    #[must_use]
    pub fn read_dp(&self, address: u16) -> u8 {
        if !self.firmware_loaded {
            return 0;
        }
        let hi = address & 1 != 0;
        let word = self.data_ram[usize::from((address >> 1) & 2047)];
        if hi { (word >> 8) as u8 } else { word as u8 }
    }

    /// Write a byte to the data-RAM host port (`address` is the byte address into the window).
    pub fn write_dp(&mut self, address: u16, data: u8) {
        if !self.firmware_loaded {
            return;
        }
        let hi = address & 1 != 0;
        let slot = &mut self.data_ram[usize::from((address >> 1) & 2047)];
        if hi {
            *slot = (*slot & 0x00FF) | u16::from(data) << 8;
        } else {
            *slot = (*slot & 0xFF00) | u16::from(data);
        }
    }

    /// Advance the engine until it parks waiting for the host (RQM set), capped at `RUN_CAP`
    /// instructions. A no-op when already parked or when no firmware is loaded.
    pub fn run_until_rqm(&mut self) {
        if !self.firmware_loaded {
            return;
        }
        let mut budget = Self::RUN_CAP;
        while !self.sr.rqm && budget > 0 {
            self.exec();
            budget -= 1;
        }
    }

    // --- The instruction core. ---

    fn set_pc(&mut self, value: u16) {
        self.pc = value & self.revision.pc_mask();
    }

    /// Execute one instruction, then update the M/N multiplier outputs (NEC pipelines a
    /// signed `K×L` multiply each cycle).
    pub fn exec(&mut self) {
        let opcode = self.program_rom[usize::from(self.pc)];
        self.set_pc(self.pc.wrapping_add(1));
        match opcode >> 22 {
            0 => self.exec_op(opcode),
            1 => self.exec_rt(opcode),
            2 => self.exec_jp(opcode),
            _ => self.exec_ld(opcode),
        }

        let result = i32::from(self.k) * i32::from(self.l); // sign + 30-bit product
        self.m = (result >> 15) as i16; // sign + top 15 bits
        self.n = (result << 1) as i16; // low 15 bits + zero
    }

    #[allow(clippy::too_many_lines)]
    fn exec_op(&mut self, opcode: u32) {
        let pselect = (opcode >> 20) & 0x3;
        let alu = (opcode >> 16) & 0xF;
        let asl = (opcode >> 15) & 0x1;
        let dpl = (opcode >> 13) & 0x3;
        let dphm = ((opcode >> 9) & 0xF) as u16;
        let rpdcr = (opcode >> 8) & 0x1;
        let src = (opcode >> 4) & 0xF;
        let dst = opcode & 0xF;

        let idb: u16 = match src {
            0 => self.trb,
            1 => self.a as u16,
            2 => self.b as u16,
            3 => self.tr,
            4 => self.dp,
            5 => self.rp,
            6 => self.data_rom[usize::from(self.rp)],
            7 => 0x8000 - u16::from(self.flag_a.s1), // ASL ignored; always SA1
            8 => {
                self.sr.rqm = true;
                self.dr
            }
            9 => self.dr,
            10 => self.sr.to_bits(),
            11 | 12 => self.si,
            13 => self.k as u16,
            14 => self.l as u16,
            _ => self.data_ram[usize::from(self.dp & self.revision.dp_mask())],
        };

        if alu != 0 {
            let mut p: u16 = match pselect {
                0 => self.data_ram[usize::from(self.dp & self.revision.dp_mask())],
                1 => idb,
                2 => self.m as u16,
                _ => self.n as u16,
            };

            let (q, mut flag, c) = if asl == 0 {
                (self.a as u16, self.flag_a, self.flag_b.c)
            } else {
                (self.b as u16, self.flag_b, self.flag_a.c)
            };
            let cu = u16::from(c);

            let r: u16 = match alu {
                1 => q | p,
                2 => q & p,
                3 => q ^ p,
                4 => q.wrapping_sub(p),
                5 => q.wrapping_add(p),
                6 => q.wrapping_sub(p).wrapping_sub(cu),
                7 => q.wrapping_add(p).wrapping_add(cu),
                8 => {
                    p = 1;
                    q.wrapping_sub(1)
                }
                9 => {
                    p = 1;
                    q.wrapping_add(1)
                }
                10 => !q,
                11 => (q >> 1) | (q & 0x8000), // SHR1 (ASR)
                12 => (q << 1) | cu,           // SHL1 (ROL)
                13 => (q << 2) | 3,            // SHL2
                14 => (q << 4) | 15,           // SHL4
                _ => q.rotate_left(8),         // XCHG (byte swap)
            };

            flag.z = r == 0;
            flag.s0 = r & 0x8000 != 0;
            if !flag.ov1 {
                flag.s1 = flag.s0;
            }

            match alu {
                1 | 2 | 3 | 10 | 13 | 14 | 15 => {
                    flag.ov0 = false;
                    flag.ov1 = false;
                    flag.c = false;
                }
                11 => {
                    flag.ov0 = false;
                    flag.ov1 = false;
                    flag.c = q & 1 != 0;
                }
                12 => {
                    flag.ov0 = false;
                    flag.ov1 = false;
                    flag.c = q >> 15 != 0;
                }
                // SUB/ADD/SBB/ADC/DEC/INC.
                _ => {
                    let carries = q ^ p ^ r;
                    let second = if alu & 1 != 0 { r } else { q };
                    let overflow = (q ^ r) & (p ^ second);
                    let ov0 = overflow & 0x8000 != 0;
                    flag.ov1 = if ov0 && flag.ov1 {
                        flag.s0 == flag.s1
                    } else {
                        ov0 || flag.ov1
                    };
                    flag.ov0 = ov0;
                    flag.c = (carries ^ overflow) & 0x8000 != 0;
                }
            }

            if asl == 0 {
                self.a = r as i16;
                self.flag_a = flag;
            } else {
                self.b = r as i16;
                self.flag_b = flag;
            }
        }

        // The move field is an embedded LD: id = idb, dst = dst.
        self.exec_ld(u32::from(idb) << 6 | dst);

        if dst != 4 {
            // if LD did not write DP
            match dpl {
                1 => self.dp = (self.dp & 0xF0) + (self.dp.wrapping_add(1) & 0x0F), // DPINC
                2 => self.dp = (self.dp & 0xF0) + (self.dp.wrapping_sub(1) & 0x0F), // DPDEC
                3 => self.dp &= 0xF0,                                               // DPCLR
                _ => {}
            }
            self.dp ^= dphm << 4;
            self.dp &= self.revision.dp_mask();
        }

        if dst != 5 {
            // if LD did not write RP
            if rpdcr != 0 {
                self.rp = self.rp.wrapping_sub(1) & self.revision.rp_mask();
            }
        }
    }

    fn exec_rt(&mut self, opcode: u32) {
        self.exec_op(opcode);
        self.sp = self.sp.wrapping_sub(1) & 0xF;
        self.pc = self.stack[usize::from(self.sp)] & self.revision.pc_mask();
    }

    fn exec_jp(&mut self, opcode: u32) {
        let brch = (opcode >> 13) & 0x1FF;
        let na = ((opcode >> 2) & 0x7FF) as u16;
        let bank = (opcode & 0x3) as u16;
        let jp = (self.pc & 0x2000) | bank << 11 | na;

        let taken = match brch {
            0x000 => {
                self.set_pc(self.so);
                return;
            }
            0x080 => !self.flag_a.c,
            0x082 => self.flag_a.c,
            0x084 => !self.flag_b.c,
            0x086 => self.flag_b.c,
            0x088 => !self.flag_a.z,
            0x08a => self.flag_a.z,
            0x08c => !self.flag_b.z,
            0x08e => self.flag_b.z,
            0x090 => !self.flag_a.ov0,
            0x092 => self.flag_a.ov0,
            0x094 => !self.flag_b.ov0,
            0x096 => self.flag_b.ov0,
            0x098 => !self.flag_a.ov1,
            0x09a => self.flag_a.ov1,
            0x09c => !self.flag_b.ov1,
            0x09e => self.flag_b.ov1,
            0x0a0 => !self.flag_a.s0,
            0x0a2 => self.flag_a.s0,
            0x0a4 => !self.flag_b.s0,
            0x0a6 => self.flag_b.s0,
            0x0a8 => !self.flag_a.s1,
            0x0aa => self.flag_a.s1,
            0x0ac => !self.flag_b.s1,
            0x0ae => self.flag_b.s1,
            0x0b0 => self.dp & 0x0F == 0x00,
            0x0b1 => self.dp & 0x0F != 0x00,
            0x0b2 => self.dp & 0x0F == 0x0F,
            0x0b3 => self.dp & 0x0F != 0x0F,
            0x0b4 => !self.sr.siack,
            0x0b6 => self.sr.siack,
            0x0b8 => !self.sr.soack,
            0x0ba => self.sr.soack,
            0x0bc => !self.sr.rqm,
            0x0be => self.sr.rqm,
            0x100 => {
                self.set_pc(jp & !0x2000); // LJMP
                return;
            }
            0x101 => {
                self.set_pc(jp | 0x2000); // HJMP
                return;
            }
            0x140 => {
                self.stack[usize::from(self.sp)] = self.pc;
                self.sp = self.sp.wrapping_add(1) & 0xF;
                self.set_pc(jp & !0x2000); // LCALL
                return;
            }
            0x141 => {
                self.stack[usize::from(self.sp)] = self.pc;
                self.sp = self.sp.wrapping_add(1) & 0xF;
                self.set_pc(jp | 0x2000); // HCALL
                return;
            }
            _ => return,
        };
        if taken {
            self.set_pc(jp);
        }
    }

    fn exec_ld(&mut self, opcode: u32) {
        let id = (opcode >> 6) as u16;
        let dst = opcode & 0xF;
        match dst {
            0 => {}
            1 => self.a = id as i16,
            2 => self.b = id as i16,
            3 => self.tr = id,
            4 => self.dp = id & self.revision.dp_mask(),
            5 => self.rp = id & self.revision.rp_mask(),
            6 => {
                self.dr = id;
                self.sr.rqm = true;
            }
            7 => {
                let v = (self.sr.to_bits() & 0x907C) | (id & !0x907C);
                self.sr.set_bits(v);
            }
            8 | 9 => self.so = id,
            10 => self.k = id as i16,
            11 => {
                self.k = id as i16;
                self.l = self.data_rom[usize::from(self.rp)] as i16;
            }
            12 => {
                self.l = id as i16;
                self.k =
                    self.data_ram[usize::from((self.dp | 0x40) & self.revision.dp_mask())] as i16;
            }
            13 => self.l = id as i16,
            14 => self.trb = id,
            _ => self.data_ram[usize::from(self.dp & self.revision.dp_mask())] = id,
        }
    }

    // --- Test / debug accessors. ---

    /// Count of host DR port accesses since power-on (diagnostics).
    #[must_use]
    pub const fn host_accesses(&self) -> u64 {
        self.host_accesses
    }

    /// The current program counter (for unit tests / the debugger).
    #[must_use]
    pub const fn pc(&self) -> u16 {
        self.pc
    }

    /// The RQM (request-for-master) status bit (for unit tests / the debugger).
    #[must_use]
    pub const fn rqm(&self) -> bool {
        self.sr.rqm
    }

    /// Read a data-RAM word directly (for unit tests / save-state inspection).
    #[must_use]
    pub fn data_ram_word(&self, index: usize) -> u16 {
        self.data_ram[index & 2047]
    }

    /// The condition flags packed (`flags.a` in the low 6 bits, `flags.b` in the next 6) — for
    /// unit-test vectors derived from a reference trace.
    #[must_use]
    pub fn flags_packed(&self) -> u16 {
        self.flag_a.to_bits() | self.flag_b.to_bits() << 6
    }

    /// Write this engine's mutable state — every register + the 2048-word data RAM — into a
    /// `"NDSP"` section. Firmware (`program_rom`/`data_rom`) is deliberately NOT written: it is
    /// never embedded in a save-state (the same "never carry a ROM/firmware byte" posture
    /// `docs/adr/0003` already applies elsewhere), reloaded fresh via [`Self::load_firmware`]
    /// before a matching [`Self::load_state`] call. `host_accesses` (a debugger counter, not
    /// emulated-hardware state) is also omitted — restoring it to a stale value would misrepresent
    /// activity that happened after the save, not before it.
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"NDSP", |s| {
            for &word in &self.stack {
                s.write_u16(word);
            }
            s.write_u16(self.pc);
            s.write_u16(self.rp);
            s.write_u16(self.dp);
            s.write_u8(self.sp);
            s.write_u16(self.si);
            s.write_u16(self.so);
            s.write_u16(self.k.cast_unsigned());
            s.write_u16(self.l.cast_unsigned());
            s.write_u16(self.m.cast_unsigned());
            s.write_u16(self.n.cast_unsigned());
            s.write_u16(self.a.cast_unsigned());
            s.write_u16(self.b.cast_unsigned());
            s.write_u16(self.tr);
            s.write_u16(self.trb);
            s.write_u16(self.dr);
            write_status(s, self.sr);
            write_flag(s, self.flag_a);
            write_flag(s, self.flag_b);
            for &word in &self.data_ram {
                s.write_u16(word);
            }
        });
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input (the section framing itself already rejects
    /// a wrong tag or a truncated read — there is no additional semantic range to validate here,
    /// unlike a board-level cursor: every field is a fixed-width register the DSP hardware itself
    /// never partially constrains).
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"NDSP")?;
        for slot in &mut self.stack {
            *slot = s.read_u16()?;
        }
        self.pc = s.read_u16()?;
        self.rp = s.read_u16()?;
        self.dp = s.read_u16()?;
        self.sp = s.read_u8()?;
        self.si = s.read_u16()?;
        self.so = s.read_u16()?;
        self.k = s.read_u16()?.cast_signed();
        self.l = s.read_u16()?.cast_signed();
        self.m = s.read_u16()?.cast_signed();
        self.n = s.read_u16()?.cast_signed();
        self.a = s.read_u16()?.cast_signed();
        self.b = s.read_u16()?.cast_signed();
        self.tr = s.read_u16()?;
        self.trb = s.read_u16()?;
        self.dr = s.read_u16()?;
        self.sr = read_status(&mut s)?;
        self.flag_a = read_flag(&mut s)?;
        self.flag_b = read_flag(&mut s)?;
        for slot in &mut self.data_ram {
            *slot = s.read_u16()?;
        }
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "NDSP section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        Ok(())
    }
}

fn write_flag(s: &mut SaveWriter, f: Flag) {
    s.write_bool(f.ov0);
    s.write_bool(f.ov1);
    s.write_bool(f.z);
    s.write_bool(f.c);
    s.write_bool(f.s0);
    s.write_bool(f.s1);
}

fn read_flag(s: &mut SaveReader) -> Result<Flag, SaveStateError> {
    Ok(Flag {
        ov0: s.read_bool()?,
        ov1: s.read_bool()?,
        z: s.read_bool()?,
        c: s.read_bool()?,
        s0: s.read_bool()?,
        s1: s.read_bool()?,
    })
}

fn write_status(s: &mut SaveWriter, st: Status) {
    s.write_bool(st.p0);
    s.write_bool(st.p1);
    s.write_bool(st.ei);
    s.write_bool(st.sic);
    s.write_bool(st.soc);
    s.write_bool(st.drc);
    s.write_bool(st.dma);
    s.write_bool(st.drs);
    s.write_bool(st.usf0);
    s.write_bool(st.usf1);
    s.write_bool(st.rqm);
    s.write_bool(st.siack);
    s.write_bool(st.soack);
}

fn read_status(s: &mut SaveReader) -> Result<Status, SaveStateError> {
    Ok(Status {
        p0: s.read_bool()?,
        p1: s.read_bool()?,
        ei: s.read_bool()?,
        sic: s.read_bool()?,
        soc: s.read_bool()?,
        drc: s.read_bool()?,
        dma: s.read_bool()?,
        drs: s.read_bool()?,
        usf0: s.read_bool()?,
        usf1: s.read_bool()?,
        rqm: s.read_bool()?,
        siack: s.read_bool()?,
        soack: s.read_bool()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// Pack a µPD7725 firmware image from a list of 24-bit program words (data ROM left zero).
    fn synth_firmware(program: &[u32]) -> Vec<u8> {
        // 2048 program words × 3 bytes (LE) + 1024 data words × 2 bytes (LE) = 8192 bytes.
        let mut prog = vec![0u32; 2048];
        prog[..program.len()].copy_from_slice(program);
        let mut bytes = Vec::with_capacity(8192);
        for w in prog {
            bytes.push((w & 0xFF) as u8);
            bytes.push((w >> 8 & 0xFF) as u8);
            bytes.push((w >> 16 & 0xFF) as u8);
        }
        bytes.resize(8192, 0); // append the zeroed 1024-word data ROM
        bytes
    }

    // --- Opcode constructors (the published NEC DSP encoding). ---

    /// `LD` immediate: `id → dst`. Top two bits select the standalone-LD form.
    const fn ld(id: u16, dst: u32) -> u32 {
        0b11 << 22 | (id as u32) << 6 | dst
    }
    /// `OP` with an ALU op + a move-field destination.
    const fn op(pselect: u32, alu: u32, src: u32, dst: u32, dpl: u32) -> u32 {
        pselect << 20 | alu << 16 | dpl << 13 | src << 4 | dst
    }
    /// `LJMP` to `target`.
    const fn ljmp(target: u32) -> u32 {
        0b10 << 22 | 0x100 << 13 | (target & 0x7FF) << 2
    }

    #[test]
    fn alu_add_sub_multiply_and_dataram_moves() {
        // A hand-assembled program: 5+3 → RAM[0]; 2−3 (borrow) → RAM[1]; (7×6)<<1 via the
        // multiplier → RAM[2]; then load DR=0xBEEF. Exercises LD/OP decode, ADD/SUB flags, the
        // K×L pipeline, the move field, and DPINC pointer stepping.
        let program = [
            ld(5, 1),           // 0: A = 5
            ld(3, 3),           // 1: TR = 3
            op(1, 5, 3, 0, 0),  // 2: A = A + TR = 8        (ADD, p=idb=TR)
            op(0, 0, 1, 15, 1), // 3: RAM[dp=0] = A; DPINC  (move A→RAM, dp→1)
            ld(2, 1),           // 4: A = 2
            op(1, 4, 3, 0, 0),  // 5: A = A − TR = 0xFFFF    (SUB, borrow)
            op(0, 0, 1, 15, 1), // 6: RAM[dp=1] = A; DPINC  (dp→2)
            ld(7, 10),          // 7: K = 7
            ld(6, 13),          // 8: L = 6  ⇒ N = (7×6)<<1 = 84
            ld(0, 1),           // 9: A = 0
            op(3, 5, 0, 0, 0),  // 10: A = A + N = 84        (ADD, p=N)
            op(0, 0, 1, 15, 1), // 11: RAM[dp=2] = A; DPINC (dp→3)
            ld(0xBEEF, 6),      // 12: DR = 0xBEEF, RQM=1
        ];
        let mut eng = Upd77c25::new(Revision::Upd7725);
        // Load the program directly (bypass the run-to-rqm prime so we can single-step).
        let fw = synth_firmware(&program);
        let prog_bytes = eng.program_rom.len() * 3;
        for (i, word) in eng.program_rom.iter_mut().enumerate() {
            let o = i * 3;
            *word = u32::from(fw[o]) | u32::from(fw[o + 1]) << 8 | u32::from(fw[o + 2]) << 16;
        }
        eng.firmware_loaded = true;
        let _ = prog_bytes;

        for _ in 0..13 {
            eng.exec();
        }

        assert_eq!(eng.data_ram_word(0), 8, "5+3");
        assert_eq!(eng.data_ram_word(1), 0xFFFF, "2-3 borrow");
        assert_eq!(eng.data_ram_word(2), 84, "(7*6)<<1 via the K*L multiplier");

        // DR readout: 16-bit transfer, low byte then high byte.
        assert!(eng.rqm(), "DR load raised RQM");
        assert_eq!(eng.dr & 0xFF, 0xEF);
        assert_eq!(eng.dr >> 8, 0xBE);
    }

    #[test]
    fn dr_handshake_and_run_until_rqm() {
        // Program: load DR=0x1234 (raises RQM), then loop. run_until_rqm must park at the LD; a
        // full 16-bit DR read must hand back 0x34 then 0x12 and re-arm via the loop.
        let program = [ld(0x1234, 6), ljmp(0)];
        let mut eng = Upd77c25::new(Revision::Upd7725);
        assert!(eng.load_firmware(&synth_firmware(&program)));
        assert!(eng.firmware_loaded());
        // Priming ran until the LD parked with RQM set.
        assert!(eng.rqm());
        assert_eq!(eng.read_sr() & 0x80, 0x80, "SR high bit = RQM");

        assert_eq!(eng.read_dr(), 0x34); // low byte (drs set, rqm held)
        assert_eq!(eng.read_dr(), 0x12); // high byte (rqm cleared, then re-armed by the loop)
        assert!(eng.rqm(), "loop re-loaded DR and re-raised RQM");
    }

    #[test]
    fn inert_until_firmware() {
        let mut eng = Upd77c25::new(Revision::Upd7725);
        assert!(!eng.firmware_loaded());
        assert_eq!(eng.read_sr(), 0);
        assert_eq!(eng.read_dr(), 0);
        eng.run_until_rqm(); // no-op, must not hang
        assert!(!eng.load_firmware(&[0u8; 100])); // too short
    }
}
