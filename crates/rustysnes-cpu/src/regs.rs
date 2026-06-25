//! WDC 65C816 register file and processor-status flags.
//!
//! The 65C816 has variable-width data paths: the accumulator `A` is 8- or 16-bit per the
//! `M` status flag, and the index registers `X`/`Y` are 8- or 16-bit per the `X` status
//! flag. In 8-bit index mode the high byte of `X`/`Y` is forced to zero. The hidden `E`
//! (emulation) latch forces `M`/`X` to the 8-bit width and confines the stack to page `$01`.
//!
//! See `docs/cpu.md` ("Registers and state", "Emulation vs native mode").

// Taking the low byte of a 16-bit register to set 8-bit-width flags is a deliberate, ubiquitous
// truncation in the width model; the cast-precision lints would flag every such narrowing.
#![allow(clippy::cast_possible_truncation)]

use bitflags::bitflags;

bitflags! {
    /// Processor status register `P` (8 bits) plus the hidden emulation latch is tracked
    /// separately on [`Regs`]. Bit layout matches the hardware `P` register so `PHP`/`PLP`
    /// round-trip exactly.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Status: u8 {
        /// Carry.
        const C = 0b0000_0001;
        /// Zero.
        const Z = 0b0000_0010;
        /// IRQ disable (IRQ honored only when clear).
        const I = 0b0000_0100;
        /// Decimal mode (BCD arithmetic for `ADC`/`SBC`).
        const D = 0b0000_1000;
        /// Index-register width: set ⇒ 8-bit `X`/`Y`. In emulation mode this also doubles as
        /// the 6502 `B` (break) flag on the stacked status byte.
        const X = 0b0001_0000;
        /// Memory/accumulator width: set ⇒ 8-bit `A`/memory.
        const M = 0b0010_0000;
        /// Overflow.
        const V = 0b0100_0000;
        /// Negative.
        const N = 0b1000_0000;
    }
}

/// The complete architectural register file of the 65C816.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regs {
    /// Accumulator (full 16 bits stored; the active width is the `M` flag).
    pub a: u16,
    /// `X` index register (full 16 bits stored; the active width is the `X` flag).
    pub x: u16,
    /// `Y` index register (full 16 bits stored; the active width is the `X` flag).
    pub y: u16,
    /// Stack pointer (16-bit in native mode; forced to page `$01` in emulation mode).
    pub s: u16,
    /// Direct-page register `D`.
    pub d: u16,
    /// Data bank register `DBR`.
    pub dbr: u8,
    /// Program bank register `PBR` / `K`.
    pub pbr: u8,
    /// Program counter (16-bit; bank is `pbr`).
    pub pc: u16,
    /// Processor status flags `P`.
    pub p: Status,
    /// Hidden emulation-mode latch `E` (`true` at power-on).
    pub emulation: bool,
}

impl Default for Regs {
    fn default() -> Self {
        Self::new()
    }
}

impl Regs {
    /// Power-on register file: emulation mode (`E=1`), `M=1`, `X=1`, `I=1`, `D=0`, with the
    /// stack pointer parked at `$01FF`. `PC` is loaded from the reset vector by
    /// [`crate::Cpu::reset`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: 0x01FF,
            d: 0,
            dbr: 0,
            pbr: 0,
            pc: 0,
            // I and the width flags are set at reset; emulation forces M/X high.
            p: Status::from_bits_truncate(Status::M.bits() | Status::X.bits() | Status::I.bits()),
            emulation: true,
        }
    }

    /// Whether the accumulator / memory width is 8-bit (`M` flag set, or emulation mode).
    #[must_use]
    pub const fn m8(&self) -> bool {
        self.emulation || self.p.contains(Status::M)
    }

    /// Whether the index registers are 8-bit (`X` flag set, or emulation mode).
    #[must_use]
    pub const fn x8(&self) -> bool {
        self.emulation || self.p.contains(Status::X)
    }

    /// Read the accumulator at the active width (high byte masked off when 8-bit).
    #[must_use]
    pub const fn a_val(&self) -> u16 {
        if self.m8() { self.a & 0x00FF } else { self.a }
    }

    /// Write the accumulator at the active width, preserving the hidden high byte (`B`) when
    /// in 8-bit mode — matching hardware, where `A` and `B` are distinct halves.
    pub const fn set_a(&mut self, val: u16) {
        if self.m8() {
            self.a = (self.a & 0xFF00) | (val & 0x00FF);
        } else {
            self.a = val;
        }
    }

    /// Read `X` at the active index width.
    #[must_use]
    pub const fn x_val(&self) -> u16 {
        if self.x8() { self.x & 0x00FF } else { self.x }
    }

    /// Read `Y` at the active index width.
    #[must_use]
    pub const fn y_val(&self) -> u16 {
        if self.x8() { self.y & 0x00FF } else { self.y }
    }

    /// Write `X`; in 8-bit index mode the high byte is forced to zero (hardware behavior).
    pub const fn set_x(&mut self, val: u16) {
        self.x = if self.x8() { val & 0x00FF } else { val };
    }

    /// Write `Y`; in 8-bit index mode the high byte is forced to zero (hardware behavior).
    pub const fn set_y(&mut self, val: u16) {
        self.y = if self.x8() { val & 0x00FF } else { val };
    }

    /// Update `N` and `Z` from an accumulator-width result.
    pub const fn set_nz_m(&mut self, val: u16) {
        if self.m8() {
            self.set_nz8(val as u8);
        } else {
            self.set_nz16(val);
        }
    }

    /// Update `N` and `Z` from an index-width result.
    pub const fn set_nz_x(&mut self, val: u16) {
        if self.x8() {
            self.set_nz8(val as u8);
        } else {
            self.set_nz16(val);
        }
    }

    /// Update `N`/`Z` from an explicit 8-bit value.
    pub const fn set_nz8(&mut self, val: u8) {
        self.set_flag(Status::Z, val == 0);
        self.set_flag(Status::N, val & 0x80 != 0);
    }

    /// Update `N`/`Z` from an explicit 16-bit value.
    pub const fn set_nz16(&mut self, val: u16) {
        self.set_flag(Status::Z, val == 0);
        self.set_flag(Status::N, val & 0x8000 != 0);
    }

    /// Set or clear a single status flag.
    pub const fn set_flag(&mut self, flag: Status, on: bool) {
        if on {
            self.p = Status::from_bits_truncate(self.p.bits() | flag.bits());
        } else {
            self.p = Status::from_bits_truncate(self.p.bits() & !flag.bits());
        }
    }
}
