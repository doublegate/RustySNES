//! WDC-adjacent Sony **SPC700** (S-SMP) 8-bit core — the SNES audio CPU.
//!
//! A 6502-relative core with 16-bit `YA`, a half-carry/overflow PSW, and a handful of
//! 16-bit word ops (`MOVW`/`ADDW`/`SUBW`/`CMPW`/`INCW`/`DECW`/`DIV`/`MUL`). The core is
//! generic over a [`Spc700Bus`] so the per-opcode oracle can drive it against flat RAM while
//! the real APU wires in ARAM + the memory-mapped registers (IPL ROM, DSP, ports, timers).
//!
//! Every memory access *and* every internal cycle goes through the bus as one tick, so a bus
//! that counts ticks yields the exact instruction cycle count the `SingleStepTests` record.
//! Cycle structure and flag math are derived clean-room from ares (`component/processor/spc700`,
//! ISC) and cross-checked against the SingleStepTests/spc700 oracle.

// SPC700 register/PC math is byte-oriented: u16↔u8 truncation and signed reinterpretation are
// intentional and bounded by the architecture.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::needless_pass_by_ref_mut
)]

use crate::psw::Psw;

/// The narrow seam the SPC700 core drives. Each method models exactly one SPC700 clock cycle.
///
/// The full APU implements this over ARAM + memory-mapped registers; the oracle implements it
/// over a flat 64 KiB array (the `SingleStepTests` memory model: no IPL/IO interception).
pub trait Spc700Bus {
    /// Read one byte at `address`, consuming one SPC700 cycle.
    fn read(&mut self, address: u16) -> u8;
    /// Write one byte at `address`, consuming one SPC700 cycle.
    fn write(&mut self, address: u16, data: u8);
    /// Burn one internal ("idle") SPC700 cycle with no bus transfer.
    fn idle(&mut self);
}

/// SPC700 architectural register file.
#[derive(Debug, Clone, Copy)]
pub struct Registers {
    /// Program counter.
    pub pc: u16,
    /// Accumulator (low byte of `YA`).
    pub a: u8,
    /// X index.
    pub x: u8,
    /// Y index (high byte of `YA`).
    pub y: u8,
    /// Stack pointer (always in page 1: `$0100..=$01FF`).
    pub sp: u8,
    /// Processor status word.
    pub psw: Psw,
}

impl Default for Registers {
    fn default() -> Self {
        // SPC700 power-on (ares `SPC700::power`): PC=0, YA=0, X=0, S=$EF, P=$02.
        Self {
            pc: 0x0000,
            a: 0x00,
            x: 0x00,
            y: 0x00,
            sp: 0xEF,
            psw: Psw::from_bits(0x02),
        }
    }
}

impl Registers {
    /// The 16-bit `YA` pair (`Y` high, `A` low).
    #[must_use]
    pub const fn ya(&self) -> u16 {
        (self.y as u16) << 8 | self.a as u16
    }

    /// Set the 16-bit `YA` pair.
    pub const fn set_ya(&mut self, value: u16) {
        self.a = value as u8;
        self.y = (value >> 8) as u8;
    }
}

/// SPC700 execution core. Holds only registers + a halt flag; memory lives behind the bus.
#[derive(Debug, Clone, Copy, Default)]
pub struct Spc700 {
    /// Architectural registers.
    pub regs: Registers,
    /// `STOP` halt latch — set by `STOP`, never self-clears (only a reset clears it).
    pub stopped: bool,
    /// `SLEEP` (`WAI`) latch — set by `SLEEP`; clears on an (emulated) interrupt.
    pub waiting: bool,
}

/// 8-bit ALU op selector for the read/modify families (one byte in, one byte out).
#[derive(Clone, Copy)]
pub(crate) enum AluU {
    Asl,
    Lsr,
    Rol,
    Ror,
    Dec,
    Inc,
}

/// 8-bit binary op selector (two bytes in, flags + result out).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AluB {
    Or,
    And,
    Eor,
    Adc,
    Sbc,
    Cmp,
    Ld,
}

/// 16-bit word op selector for the `*W` families.
#[derive(Clone, Copy)]
pub(crate) enum AluW {
    Add,
    Sub,
    Cmp,
    Ld,
}

/// Which index register an addressing mode adds.
#[derive(Clone, Copy)]
pub(crate) enum Index {
    X,
    Y,
}

impl Spc700 {
    /// Construct at power-on.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // ---- primitive cycle-consuming bus helpers (mirror ares memory.cpp) ----

    pub(crate) fn fetch(&mut self, bus: &mut impl Spc700Bus) -> u8 {
        let v = bus.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        v
    }

    /// Direct-page load: address is page-relative, page chosen by the P flag.
    pub(crate) fn load(&mut self, bus: &mut impl Spc700Bus, address: u8) -> u8 {
        bus.read(self.dp(address))
    }

    pub(crate) fn store(&mut self, bus: &mut impl Spc700Bus, address: u8, data: u8) {
        bus.write(self.dp(address), data);
    }

    const fn dp(&self, address: u8) -> u16 {
        let page = if self.regs.psw.p() { 0x0100 } else { 0x0000 };
        page | address as u16
    }

    pub(crate) fn pull(&mut self, bus: &mut impl Spc700Bus) -> u8 {
        self.regs.sp = self.regs.sp.wrapping_add(1);
        bus.read(0x0100 | self.regs.sp as u16)
    }

    pub(crate) fn push(&mut self, bus: &mut impl Spc700Bus, data: u8) {
        bus.write(0x0100 | self.regs.sp as u16, data);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
    }

    // ---- 8-bit ALU (ares algorithms.cpp) ----

    pub(crate) fn alu_u(&mut self, op: AluU, x: u8) -> u8 {
        let p = &mut self.regs.psw;
        match op {
            AluU::Asl => {
                p.set_c(x & 0x80 != 0);
                let r = x << 1;
                p.set_nz(r);
                r
            }
            AluU::Lsr => {
                p.set_c(x & 0x01 != 0);
                let r = x >> 1;
                p.set_nz(r);
                r
            }
            AluU::Rol => {
                let carry = u8::from(p.c());
                p.set_c(x & 0x80 != 0);
                let r = (x << 1) | carry;
                p.set_nz(r);
                r
            }
            AluU::Ror => {
                let carry = u8::from(p.c());
                p.set_c(x & 0x01 != 0);
                let r = (carry << 7) | (x >> 1);
                p.set_nz(r);
                r
            }
            AluU::Dec => {
                let r = x.wrapping_sub(1);
                p.set_nz(r);
                r
            }
            AluU::Inc => {
                let r = x.wrapping_add(1);
                p.set_nz(r);
                r
            }
        }
    }

    pub(crate) fn alu_b(&mut self, op: AluB, x: u8, y: u8) -> u8 {
        match op {
            AluB::Or => {
                let r = x | y;
                self.regs.psw.set_nz(r);
                r
            }
            AluB::And => {
                let r = x & y;
                self.regs.psw.set_nz(r);
                r
            }
            AluB::Eor => {
                let r = x ^ y;
                self.regs.psw.set_nz(r);
                r
            }
            AluB::Adc => self.adc(x, y),
            AluB::Sbc => self.adc(x, !y),
            AluB::Cmp => {
                let z = i32::from(x) - i32::from(y);
                let p = &mut self.regs.psw;
                p.set_c(z >= 0);
                p.set_z(z as u8 == 0);
                p.set_n(z & 0x80 != 0);
                x
            }
            AluB::Ld => {
                self.regs.psw.set_nz(y);
                y
            }
        }
    }

    pub(crate) fn adc(&mut self, x: u8, y: u8) -> u8 {
        let z = i32::from(x) + i32::from(y) + i32::from(self.regs.psw.c());
        let p = &mut self.regs.psw;
        p.set_c(z > 0xFF);
        p.set_z(z as u8 == 0);
        p.set_h((i32::from(x) ^ i32::from(y) ^ z) & 0x10 != 0);
        p.set_v((!(i32::from(x) ^ i32::from(y)) & (i32::from(x) ^ z) & 0x80) != 0);
        p.set_n(z & 0x80 != 0);
        z as u8
    }

    pub(crate) fn alu_w(&mut self, op: AluW, x: u16, y: u16) -> u16 {
        match op {
            AluW::Add => {
                self.regs.psw.set_c(false);
                let lo = self.adc(x as u8, y as u8);
                let hi = self.adc((x >> 8) as u8, (y >> 8) as u8);
                let z = u16::from(lo) | (u16::from(hi) << 8);
                self.regs.psw.set_z(z == 0);
                z
            }
            AluW::Sub => {
                self.regs.psw.set_c(true);
                let lo = self.adc(x as u8, !(y as u8));
                let hi = self.adc((x >> 8) as u8, !((y >> 8) as u8));
                let z = u16::from(lo) | (u16::from(hi) << 8);
                self.regs.psw.set_z(z == 0);
                z
            }
            AluW::Cmp => {
                let z = i32::from(x) - i32::from(y);
                let p = &mut self.regs.psw;
                p.set_c(z >= 0);
                p.set_z(z as u16 == 0);
                p.set_n(z & 0x8000 != 0);
                x
            }
            AluW::Ld => {
                let p = &mut self.regs.psw;
                p.set_z(y == 0);
                p.set_n(y & 0x8000 != 0);
                y
            }
        }
    }
}
