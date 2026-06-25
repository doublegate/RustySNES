//! Instruction fetch / decode / execute for the 65C816, plus bus plumbing.
//!
//! Behavior is modelled on the bsnes / ares `wdc65816` reference cores (study-only; this is a
//! clean-room Rust implementation). The decimal-mode `ADC`/`SBC` digit-wise correction
//! follows bsnes `algorithms.cpp` exactly. Cycle counts use the standard 65C816 timing tables
//! with the variable adjustments documented in `docs/cpu.md`:
//!
//! - `+1` if `M = 0` (16-bit memory / accumulator access),
//! - `+1` if the low byte of `D` is non-zero (direct-page modes),
//! - `+1` on an indexed page-cross,
//! - `+1` branch taken, `+1` more on emulation-mode page-cross.

// Truncating / sign-changing casts between integer widths are the core of a bit-accurate CPU
// model (e.g. taking the low byte of a 16-bit register, or folding a wider ALU result back to
// the operand width). They are deliberate and ubiquitous here; flagging each one would bury
// real issues, so the cast-precision family is allowed for this module only.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
// `missing_const_for_fn` (nursery) fires on the many small `&mut self` opcode bodies that
// only mutate the register file. Marking each `const` adds no real benefit (they are never
// evaluated at compile time) and is brittle — one non-const callee revokes the whole chain.
// We leave them non-const so the opcode bodies stay uniform and edits don't ripple.
#![allow(clippy::missing_const_for_fn)]

use crate::addr::{Effective, Mode};
use crate::regs::Status;
use crate::{Bus, Cpu, vectors};

impl Cpu {
    // ----------------------------------------------------------------------------------
    // Bus plumbing. Every byte access ticks `on_cpu_cycle` and bumps the cycle counters.
    // ----------------------------------------------------------------------------------

    /// Read a byte at a 24-bit address, charging one CPU cycle.
    pub(crate) fn bus_read8(&mut self, bus: &mut impl Bus, addr: u32) -> u8 {
        let v = bus.read24(addr & 0x00FF_FFFF);
        bus.on_cpu_cycle();
        self.cycles += 1;
        self.cyc += 1;
        v
    }

    /// Write a byte at a 24-bit address, charging one CPU cycle.
    pub(crate) fn bus_write8(&mut self, bus: &mut impl Bus, addr: u32, val: u8) {
        bus.write24(addr & 0x00FF_FFFF, val);
        bus.on_cpu_cycle();
        self.cycles += 1;
        self.cyc += 1;
    }

    /// Internal (no-bus) cycle, e.g. ALU/indexing dead cycles. Charges one CPU cycle.
    pub(crate) fn io(&mut self, bus: &mut impl Bus) {
        bus.on_cpu_cycle();
        self.cycles += 1;
        self.cyc += 1;
    }

    /// Fetch the next program byte from `PBR:PC`, incrementing `PC` with 16-bit bank wrap.
    pub(crate) fn fetch8(&mut self, bus: &mut impl Bus) -> u8 {
        let addr = (u32::from(self.regs.pbr) << 16) | u32::from(self.regs.pc);
        let v = self.bus_read8(bus, addr);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        v
    }

    /// Fetch a little-endian 16-bit program operand.
    pub(crate) fn fetch16(&mut self, bus: &mut impl Bus) -> u16 {
        let lo = self.fetch8(bus);
        let hi = self.fetch8(bus);
        u16::from(lo) | (u16::from(hi) << 8)
    }

    /// Fetch a little-endian 24-bit program operand.
    pub(crate) fn fetch24(&mut self, bus: &mut impl Bus) -> u32 {
        let lo = self.fetch8(bus);
        let hi = self.fetch8(bus);
        let bank = self.fetch8(bus);
        u32::from(lo) | (u32::from(hi) << 8) | (u32::from(bank) << 16)
    }

    /// Read a little-endian 16-bit value from `addr`/`addr+1` (low-byte address wraps within
    /// the same bank, matching hardware for pointer fetches that are kept in-bank).
    fn read16(&mut self, bus: &mut impl Bus, addr: u32) -> u16 {
        let lo = self.bus_read8(bus, addr);
        let hi = self.bus_read8(bus, addr.wrapping_add(1));
        u16::from(lo) | (u16::from(hi) << 8)
    }

    // ----------------------------------------------------------------------------------
    // Stack. Native mode uses the full 16-bit S; emulation mode confines S to page $01.
    // ----------------------------------------------------------------------------------

    /// Push a byte to the stack honoring emulation-mode page-1 wrapping.
    fn push8(&mut self, bus: &mut impl Bus, val: u8) {
        let addr = u32::from(self.regs.s);
        self.bus_write8(bus, addr, val);
        if self.regs.emulation {
            self.regs.s = 0x0100 | u16::from((self.regs.s as u8).wrapping_sub(1));
        } else {
            self.regs.s = self.regs.s.wrapping_sub(1);
        }
    }

    /// Pull a byte from the stack honoring emulation-mode page-1 wrapping.
    fn pull8(&mut self, bus: &mut impl Bus) -> u8 {
        if self.regs.emulation {
            self.regs.s = 0x0100 | u16::from((self.regs.s as u8).wrapping_add(1));
        } else {
            self.regs.s = self.regs.s.wrapping_add(1);
        }
        let addr = u32::from(self.regs.s);
        self.bus_read8(bus, addr)
    }

    /// Push a 16-bit value (high byte first, so it pulls back low-then-high).
    fn push16(&mut self, bus: &mut impl Bus, val: u16) {
        self.push8(bus, (val >> 8) as u8);
        self.push8(bus, val as u8);
    }

    /// Pull a 16-bit value (low byte first).
    fn pull16(&mut self, bus: &mut impl Bus) -> u16 {
        let lo = self.pull8(bus);
        let hi = self.pull8(bus);
        u16::from(lo) | (u16::from(hi) << 8)
    }

    /// Push a byte with a *full 16-bit* `S` decrement (bsnes `pushN`), used by the "new" 65816
    /// stack ops (`PEA`/`PEI`/`PER`/`PHD`). Unlike [`Self::push8`], the intermediate `S` is
    /// **not** confined to page `$01`; emulation-mode confinement is re-applied at the
    /// instruction boundary by [`Self::normalize_emulation`].
    fn push_n8(&mut self, bus: &mut impl Bus, val: u8) {
        let addr = u32::from(self.regs.s);
        self.bus_write8(bus, addr, val);
        self.regs.s = self.regs.s.wrapping_sub(1);
    }

    /// Pull a byte with a *full 16-bit* `S` increment (bsnes `pullN`), used by `PLD`/`PLB`.
    fn pull_n8(&mut self, bus: &mut impl Bus) -> u8 {
        self.regs.s = self.regs.s.wrapping_add(1);
        let addr = u32::from(self.regs.s);
        self.bus_read8(bus, addr)
    }

    /// Push a 16-bit value with full 16-bit `S` decrement (high byte first).
    fn push_n16(&mut self, bus: &mut impl Bus, val: u16) {
        self.push_n8(bus, (val >> 8) as u8);
        self.push_n8(bus, val as u8);
    }

    /// Pull a 16-bit value with full 16-bit `S` increment (low byte first).
    fn pull_n16(&mut self, bus: &mut impl Bus) -> u16 {
        let lo = self.pull_n8(bus);
        let hi = self.pull_n8(bus);
        u16::from(lo) | (u16::from(hi) << 8)
    }

    // ----------------------------------------------------------------------------------
    // Direct-page penalty: +1 cycle when D's low byte is non-zero.
    // ----------------------------------------------------------------------------------

    fn dp_penalty(&mut self, bus: &mut impl Bus) {
        if self.regs.d & 0x00FF != 0 {
            self.io(bus);
        }
    }

    // ----------------------------------------------------------------------------------
    // Direct-page address arithmetic (clean-room port of bsnes `memory.cpp`).
    //
    // `readDirect`  : EF && DL==0 → (D & 0xFF00) | (addr & 0xFF)  [page-locked low byte]
    //                 else        → (D + addr) & 0xFFFF
    // `readDirectN` : always (D + addr) & 0xFFFF  (long-indirect pointer fetch, no page-lock)
    //
    // NOTE on (dp,X): bsnes additionally models an emulation-mode `DL!=0` high-byte wrap in
    // `readDirectX`. The SingleStepTests reference does NOT reproduce that wrap (it reads the
    // pointer high byte linearly), so we follow the oracle and apply only the `DL==0`
    // page-lock here — see [`Self::direct_x_addr`].
    // ----------------------------------------------------------------------------------

    /// Effective bank-`0` address of a direct-page byte at `D + addr`, applying the
    /// emulation-mode page-lock (bsnes `readDirect`). `addr` is the running offset including
    /// any pre-added index.
    fn direct_addr(&self, addr: u16) -> u32 {
        if self.regs.emulation && self.regs.d.trailing_zeros() >= 8 {
            u32::from((self.regs.d & 0xFF00) | (addr & 0x00FF))
        } else {
            u32::from(self.regs.d.wrapping_add(addr))
        }
    }

    /// Effective bank-`0` address for the `(dp,X)` pointer-byte fetch. `addr` already includes
    /// the `+X` from the caller; `offset` is the byte index (`0`/`1`). Applies the `DL==0`
    /// page-lock only (the oracle does not model the bsnes `DL!=0` high-byte wrap).
    fn direct_x_addr(&self, addr: u16, offset: u16) -> u32 {
        self.direct_addr(addr.wrapping_add(offset))
    }

    /// Read a 16-bit direct-page pointer via per-byte [`Self::direct_addr`] (page-locked).
    fn read_dp_ptr16(&mut self, bus: &mut impl Bus, dp: u16) -> u16 {
        let a0 = self.direct_addr(dp);
        let a1 = self.direct_addr(dp.wrapping_add(1));
        let lo = self.bus_read8(bus, a0);
        let hi = self.bus_read8(bus, a1);
        u16::from(lo) | (u16::from(hi) << 8)
    }

    /// Read a 16-bit pointer via per-byte [`Self::direct_x_addr`] for `(dp,X)`. Matches bsnes
    /// `readDirectX(U.l + X.w, 0)` / `(…, 1)`: the index is folded into the address, and the
    /// per-byte offset is `0`/`1`.
    fn read_dp_x_ptr16(&mut self, bus: &mut impl Bus, dp: u16, x: u16) -> u16 {
        let base = dp.wrapping_add(x);
        let a0 = self.direct_x_addr(base, 0);
        let a1 = self.direct_x_addr(base, 1);
        let lo = self.bus_read8(bus, a0);
        let hi = self.bus_read8(bus, a1);
        u16::from(lo) | (u16::from(hi) << 8)
    }

    /// Read a 24-bit long pointer via [`Self::direct_n_addr`] (no page-lock).
    fn read_dp_ptr24(&mut self, bus: &mut impl Bus, dp: u16) -> u32 {
        let lo = self.bus_read8(bus, self.direct_n_addr(dp));
        let mid = self.bus_read8(bus, self.direct_n_addr(dp.wrapping_add(1)));
        let hi = self.bus_read8(bus, self.direct_n_addr(dp.wrapping_add(2)));
        u32::from(lo) | (u32::from(mid) << 8) | (u32::from(hi) << 16)
    }

    /// Long-indirect pointer-byte address: always `(D + addr) & 0xFFFF` (bsnes `readDirectN`).
    fn direct_n_addr(&self, addr: u16) -> u32 {
        u32::from(self.regs.d.wrapping_add(addr))
    }

    // ----------------------------------------------------------------------------------
    // Addressing-mode resolution. Fetches the operand bytes and returns the effective addr.
    // ----------------------------------------------------------------------------------

    /// Resolve an addressing mode to an effective 24-bit address, fetching operand bytes and
    /// charging the direct-page penalty where applicable. Immediate modes are handled by the
    /// caller (they have no address); calling this with an immediate mode is a logic error and
    /// resolves to the current `PC` defensively.
    #[allow(clippy::too_many_lines)] // one arm per addressing mode; splitting would obscure it
    fn resolve(&mut self, bus: &mut impl Bus, mode: Mode) -> Effective {
        match mode {
            Mode::ImmediateM | Mode::ImmediateX => Effective {
                addr: (u32::from(self.regs.pbr) << 16) | u32::from(self.regs.pc),
                page_cross: false,
                bank0_wrap: false,
            },
            Mode::Direct => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                Effective {
                    addr: self.direct_addr(dp),
                    page_cross: false,
                    bank0_wrap: true,
                }
            }
            Mode::DirectX => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                self.io(bus);
                let off = dp.wrapping_add(self.regs.x_val());
                Effective {
                    addr: self.direct_addr(off),
                    page_cross: false,
                    bank0_wrap: true,
                }
            }
            Mode::DirectY => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                self.io(bus);
                let off = dp.wrapping_add(self.regs.y_val());
                Effective {
                    addr: self.direct_addr(off),
                    page_cross: false,
                    bank0_wrap: true,
                }
            }
            Mode::DirectIndirect => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                let base = self.read_dp_ptr16(bus, dp);
                let addr = (u32::from(self.regs.dbr) << 16) | u32::from(base);
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
            Mode::DirectXIndirect => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                self.io(bus);
                let base = self.read_dp_x_ptr16(bus, dp, self.regs.x_val());
                let addr = (u32::from(self.regs.dbr) << 16) | u32::from(base);
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
            Mode::DirectIndirectY => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                let base = self.read_dp_ptr16(bus, dp);
                let base24 = (u32::from(self.regs.dbr) << 16) | u32::from(base);
                let addr = base24.wrapping_add(u32::from(self.regs.y_val())) & 0x00FF_FFFF;
                let page_cross = (base24 & 0xFF00) != (addr & 0xFF00);
                Effective {
                    addr,
                    page_cross,
                    bank0_wrap: false,
                }
            }
            Mode::DirectIndirectLong => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                let addr = self.read_dp_ptr24(bus, dp);
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
            Mode::DirectIndirectLongY => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                let base = self.read_dp_ptr24(bus, dp);
                let addr = base.wrapping_add(u32::from(self.regs.y_val())) & 0x00FF_FFFF;
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
            Mode::Absolute => {
                let off = self.fetch16(bus);
                let addr = (u32::from(self.regs.dbr) << 16) | u32::from(off);
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
            Mode::AbsoluteX => {
                let off = self.fetch16(bus);
                let base = (u32::from(self.regs.dbr) << 16) | u32::from(off);
                let addr = base.wrapping_add(u32::from(self.regs.x_val())) & 0x00FF_FFFF;
                let page_cross = (base & 0xFF00) != (addr & 0xFF00);
                Effective {
                    addr,
                    page_cross,
                    bank0_wrap: false,
                }
            }
            Mode::AbsoluteY => {
                let off = self.fetch16(bus);
                let base = (u32::from(self.regs.dbr) << 16) | u32::from(off);
                let addr = base.wrapping_add(u32::from(self.regs.y_val())) & 0x00FF_FFFF;
                let page_cross = (base & 0xFF00) != (addr & 0xFF00);
                Effective {
                    addr,
                    page_cross,
                    bank0_wrap: false,
                }
            }
            Mode::AbsoluteLong => {
                let addr = self.fetch24(bus);
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
            Mode::AbsoluteLongX => {
                let base = self.fetch24(bus);
                let addr = base.wrapping_add(u32::from(self.regs.x_val())) & 0x00FF_FFFF;
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
            Mode::StackRelative => {
                let sr = u16::from(self.fetch8(bus));
                self.io(bus);
                let ea = self.regs.s.wrapping_add(sr);
                Effective {
                    addr: u32::from(ea),
                    page_cross: false,
                    bank0_wrap: true,
                }
            }
            Mode::StackRelativeIndirectY => {
                let sr = u16::from(self.fetch8(bus));
                self.io(bus);
                let ptr = self.regs.s.wrapping_add(sr);
                let lo = self.bus_read8(bus, u32::from(ptr));
                let hi = self.bus_read8(bus, u32::from(ptr.wrapping_add(1)));
                let base = u16::from(lo) | (u16::from(hi) << 8);
                self.io(bus);
                let base24 = (u32::from(self.regs.dbr) << 16) | u32::from(base);
                let addr = base24.wrapping_add(u32::from(self.regs.y_val())) & 0x00FF_FFFF;
                Effective {
                    addr,
                    page_cross: false,
                    bank0_wrap: false,
                }
            }
        }
    }

    /// Compute the address of operand byte `n` given a resolved effective address, honoring
    /// bank-`0` wrap for direct-page / stack modes (the high byte wraps within `$0000..=$FFFF`
    /// rather than carrying into the next bank).
    fn operand_byte_addr(e: Effective, n: u32) -> u32 {
        if e.bank0_wrap {
            u32::from((e.addr as u16).wrapping_add(n as u16))
        } else {
            e.addr.wrapping_add(n) & 0x00FF_FFFF
        }
    }

    // ----------------------------------------------------------------------------------
    // Width-aware operand read/write at an effective address.
    // ----------------------------------------------------------------------------------

    /// Read an accumulator-width operand (8 or 16 bits per `M`), honoring bank-`0` high-byte
    /// wrap for direct-page / stack operands.
    fn read_m(&mut self, bus: &mut impl Bus, e: Effective) -> u16 {
        if self.regs.m8() {
            u16::from(self.bus_read8(bus, e.addr))
        } else {
            let lo = self.bus_read8(bus, Self::operand_byte_addr(e, 0));
            let hi = self.bus_read8(bus, Self::operand_byte_addr(e, 1));
            u16::from(lo) | (u16::from(hi) << 8)
        }
    }

    /// Read an index-width operand (8 or 16 bits per `X`).
    fn read_x(&mut self, bus: &mut impl Bus, e: Effective) -> u16 {
        if self.regs.x8() {
            u16::from(self.bus_read8(bus, e.addr))
        } else {
            let lo = self.bus_read8(bus, Self::operand_byte_addr(e, 0));
            let hi = self.bus_read8(bus, Self::operand_byte_addr(e, 1));
            u16::from(lo) | (u16::from(hi) << 8)
        }
    }

    /// Write an accumulator-width operand (low byte first; high byte only when `M=0`).
    fn write_m(&mut self, bus: &mut impl Bus, e: Effective, val: u16) {
        self.bus_write8(bus, e.addr, val as u8);
        if !self.regs.m8() {
            self.bus_write8(bus, Self::operand_byte_addr(e, 1), (val >> 8) as u8);
        }
    }

    /// Write an index-width operand.
    fn write_x(&mut self, bus: &mut impl Bus, e: Effective, val: u16) {
        self.bus_write8(bus, e.addr, val as u8);
        if !self.regs.x8() {
            self.bus_write8(bus, Self::operand_byte_addr(e, 1), (val >> 8) as u8);
        }
    }

    /// Immediate accumulator-width operand fetched from the program stream.
    fn imm_m(&mut self, bus: &mut impl Bus) -> u16 {
        if self.regs.m8() {
            u16::from(self.fetch8(bus))
        } else {
            self.fetch16(bus)
        }
    }

    /// Immediate index-width operand fetched from the program stream.
    fn imm_x(&mut self, bus: &mut impl Bus) -> u16 {
        if self.regs.x8() {
            u16::from(self.fetch8(bus))
        } else {
            self.fetch16(bus)
        }
    }

    // ----------------------------------------------------------------------------------
    // ALU primitives (clean-room ports of bsnes `algorithms.cpp` behavior).
    // ----------------------------------------------------------------------------------

    /// `ADC` honoring the `D` (decimal) flag, at the current accumulator width.
    fn adc(&mut self, data: u16) {
        if self.regs.m8() {
            let a = self.regs.a & 0x00FF;
            let d = data & 0x00FF;
            let c = u16::from(self.regs.p.contains(Status::C));
            let mut result: i32 = if self.regs.p.contains(Status::D) {
                let mut r = i32::from(a & 0x0F) + i32::from(d & 0x0F) + i32::from(c);
                if r > 0x09 {
                    r += 0x06;
                }
                let carry = i32::from(r > 0x0F);
                i32::from(a & 0xF0) + i32::from(d & 0xF0) + (carry << 4) + (r & 0x0F)
            } else {
                i32::from(a) + i32::from(d) + i32::from(c)
            };
            let overflow = (!(a ^ d) & (a ^ (result as u16)) & 0x80) != 0;
            self.regs.set_flag(Status::V, overflow);
            if self.regs.p.contains(Status::D) && result > 0x9F {
                result += 0x60;
            }
            self.regs.set_flag(Status::C, result > 0xFF);
            self.regs.set_flag(Status::Z, (result as u8) == 0);
            self.regs.set_flag(Status::N, result & 0x80 != 0);
            self.regs.a = (self.regs.a & 0xFF00) | u16::from(result as u8);
        } else {
            let a = self.regs.a;
            let d = data;
            let c = u32::from(self.regs.p.contains(Status::C));
            let mut result: i64 = if self.regs.p.contains(Status::D) {
                let mut r = i64::from(a & 0x000F) + i64::from(d & 0x000F) + i64::from(c);
                if r > 0x0009 {
                    r += 0x0006;
                }
                let mut carry = i64::from(r > 0x000F);
                r = i64::from(a & 0x00F0) + i64::from(d & 0x00F0) + (carry << 4) + (r & 0x000F);
                if r > 0x009F {
                    r += 0x0060;
                }
                carry = i64::from(r > 0x00FF);
                r = i64::from(a & 0x0F00) + i64::from(d & 0x0F00) + (carry << 8) + (r & 0x00FF);
                if r > 0x09FF {
                    r += 0x0600;
                }
                carry = i64::from(r > 0x0FFF);
                i64::from(a & 0xF000) + i64::from(d & 0xF000) + (carry << 12) + (r & 0x0FFF)
            } else {
                i64::from(a) + i64::from(d) + i64::from(c)
            };
            let overflow = (!(a ^ d) & (a ^ (result as u16)) & 0x8000) != 0;
            self.regs.set_flag(Status::V, overflow);
            if self.regs.p.contains(Status::D) && result > 0x9FFF {
                result += 0x6000;
            }
            self.regs.set_flag(Status::C, result > 0xFFFF);
            self.regs.set_flag(Status::Z, (result as u16) == 0);
            self.regs.set_flag(Status::N, result & 0x8000 != 0);
            self.regs.a = result as u16;
        }
    }

    /// `SBC` honoring the `D` flag, at the current accumulator width.
    fn sbc(&mut self, data: u16) {
        if self.regs.m8() {
            let a = self.regs.a & 0x00FF;
            let d = (!data) & 0x00FF;
            let c = u16::from(self.regs.p.contains(Status::C));
            let mut result: i32 = if self.regs.p.contains(Status::D) {
                let mut r = i32::from(a & 0x0F) + i32::from(d & 0x0F) + i32::from(c);
                if r <= 0x0F {
                    r -= 0x06;
                }
                let carry = i32::from(r > 0x0F);
                i32::from(a & 0xF0) + i32::from(d & 0xF0) + (carry << 4) + (r & 0x0F)
            } else {
                i32::from(a) + i32::from(d) + i32::from(c)
            };
            let overflow = (!(a ^ d) & (a ^ (result as u16)) & 0x80) != 0;
            self.regs.set_flag(Status::V, overflow);
            if self.regs.p.contains(Status::D) && result <= 0xFF {
                result -= 0x60;
            }
            self.regs.set_flag(Status::C, result > 0xFF);
            self.regs.set_flag(Status::Z, (result as u8) == 0);
            self.regs.set_flag(Status::N, result & 0x80 != 0);
            self.regs.a = (self.regs.a & 0xFF00) | u16::from(result as u8);
        } else {
            let a = self.regs.a;
            let d = !data;
            let c = u32::from(self.regs.p.contains(Status::C));
            let mut result: i64 = if self.regs.p.contains(Status::D) {
                let mut r = i64::from(a & 0x000F) + i64::from(d & 0x000F) + i64::from(c);
                if r <= 0x000F {
                    r -= 0x0006;
                }
                let mut carry = i64::from(r > 0x000F);
                r = i64::from(a & 0x00F0) + i64::from(d & 0x00F0) + (carry << 4) + (r & 0x000F);
                if r <= 0x00FF {
                    r -= 0x0060;
                }
                carry = i64::from(r > 0x00FF);
                r = i64::from(a & 0x0F00) + i64::from(d & 0x0F00) + (carry << 8) + (r & 0x00FF);
                if r <= 0x0FFF {
                    r -= 0x0600;
                }
                carry = i64::from(r > 0x0FFF);
                i64::from(a & 0xF000) + i64::from(d & 0xF000) + (carry << 12) + (r & 0x0FFF)
            } else {
                i64::from(a) + i64::from(d) + i64::from(c)
            };
            let overflow = (!(a ^ d) & (a ^ (result as u16)) & 0x8000) != 0;
            self.regs.set_flag(Status::V, overflow);
            if self.regs.p.contains(Status::D) && result <= 0xFFFF {
                result -= 0x6000;
            }
            self.regs.set_flag(Status::C, result > 0xFFFF);
            self.regs.set_flag(Status::Z, (result as u16) == 0);
            self.regs.set_flag(Status::N, result & 0x8000 != 0);
            self.regs.a = result as u16;
        }
    }

    /// Compare a register value against memory data at the active width, setting `N/Z/C`.
    fn compare(&mut self, reg: u16, data: u16, width8: bool) {
        if width8 {
            let result = i32::from(reg & 0xFF) - i32::from(data & 0xFF);
            self.regs.set_flag(Status::C, result >= 0);
            self.regs.set_flag(Status::Z, (result as u8) == 0);
            self.regs.set_flag(Status::N, result & 0x80 != 0);
        } else {
            let result = i32::from(reg) - i32::from(data);
            self.regs.set_flag(Status::C, result >= 0);
            self.regs.set_flag(Status::Z, (result as u16) == 0);
            self.regs.set_flag(Status::N, result & 0x8000 != 0);
        }
    }

    // ----------------------------------------------------------------------------------
    // Top-level step + decode.
    // ----------------------------------------------------------------------------------

    /// Execute one 65C816 instruction against `bus`, returning the **CPU cycles** consumed
    /// (see the crate docs for the unit). Polls NMI then IRQ at the instruction boundary
    /// before fetching; IRQ is honored only when the `I` flag is clear. If the `STP` latch is
    /// set, the CPU stays halted (one idle cycle returned) until [`Cpu::reset`].
    pub fn step(&mut self, bus: &mut impl Bus) -> u32 {
        self.cyc = 0;
        // Hardware holds these invariants continuously in emulation mode, so assert them up
        // front too — instructions that push to the stack must see `S.h = $01` before they run.
        self.normalize_emulation();

        if self.stopped {
            self.io(bus);
            return self.cyc;
        }

        let nmi = bus.poll_nmi();
        let irq = bus.poll_irq();

        if nmi {
            self.waiting = false;
            self.service_interrupt(bus, false, true);
        } else if irq && !self.regs.p.contains(Status::I) {
            self.waiting = false;
            self.service_interrupt(bus, false, false);
        } else if self.waiting {
            // WAI with no pending interrupt: burn an idle cycle and stay parked.
            self.io(bus);
        } else {
            let opcode = self.fetch8(bus);
            self.execute(bus, opcode);
        }

        self.normalize_emulation();
        self.cyc
    }

    /// Re-assert the emulation-mode hardware invariants that hold at every instruction
    /// boundary when `E=1`: the stack pointer high byte is forced to `$01`, the `M`/`X` status
    /// bits read back as set, and the index-register high bytes are zero. The 65816 maintains
    /// these continuously in 6502-emulation mode (matching bsnes, which forces `S.h=0x01`,
    /// `XF=MF=1`, and `X.h=Y.h=0` on every relevant op), so the per-opcode oracle expects them
    /// regardless of which instruction ran.
    fn normalize_emulation(&mut self) {
        if self.regs.emulation {
            self.regs.s = 0x0100 | (self.regs.s & 0x00FF);
            self.regs.set_flag(Status::M, true);
            self.regs.set_flag(Status::X, true);
            self.regs.x &= 0x00FF;
            self.regs.y &= 0x00FF;
        }
    }

    /// Service a hardware/software interrupt (NMI/IRQ/BRK/COP). Pushes the return frame and
    /// loads `PC` from the appropriate emulation/native vector, then sets `I` and clears `D`.
    fn service_interrupt(&mut self, bus: &mut impl Bus, software: bool, nmi: bool) {
        // Two internal cycles for the hardware sequence (signature varies; one suffices for
        // count parity with the reference's interrupt handler timing).
        self.io(bus);
        if !self.regs.emulation {
            self.push8(bus, self.regs.pbr);
        }
        self.push16(bus, self.regs.pc);
        // For emulation mode the B flag (== X bit position) distinguishes BRK from IRQ.
        let mut status = self.regs.p.bits();
        if self.regs.emulation && !software {
            status &= !Status::X.bits(); // B clear for hardware IRQ
        }
        self.push8(bus, status);
        self.regs.set_flag(Status::I, true);
        self.regs.set_flag(Status::D, false);
        let vector = self.interrupt_vector(software, nmi);
        let pc = self.read16(bus, vector);
        self.regs.pc = pc;
        self.regs.pbr = 0;
    }

    /// Select the interrupt vector for the current mode.
    const fn interrupt_vector(&self, software: bool, nmi: bool) -> u32 {
        if nmi {
            if self.regs.emulation {
                vectors::NMI_EMU
            } else {
                vectors::NMI_NATIVE
            }
        } else if software {
            // BRK / IRQ share a vector in emulation mode; native splits them — handled by the
            // caller for COP. Here `software` covers BRK and IRQ.
            if self.regs.emulation {
                vectors::IRQ_BRK_EMU
            } else {
                vectors::IRQ_NATIVE
            }
        } else if self.regs.emulation {
            vectors::IRQ_BRK_EMU
        } else {
            vectors::IRQ_NATIVE
        }
    }

    /// Decode and execute a single opcode. Every one of the 256 opcodes is handled.
    #[allow(clippy::too_many_lines)]
    fn execute(&mut self, bus: &mut impl Bus, opcode: u8) {
        match opcode {
            // ---- ORA ----
            0x09 => {
                let v = self.imm_m(bus);
                self.op_ora(v);
            }
            0x05 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x15 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x0D => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x1D => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x19 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x01 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x11 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x12 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x07 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x17 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x0F => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x1F => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x03 => {
                let e = self.resolve(bus, Mode::StackRelative);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }
            0x13 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                let v = self.read_m(bus, e);
                self.op_ora(v);
            }

            // ---- AND ----
            0x29 => {
                let v = self.imm_m(bus);
                self.op_and(v);
            }
            0x25 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x35 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x2D => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x3D => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x39 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x21 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x31 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x32 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x27 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x37 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x2F => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x3F => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x23 => {
                let e = self.resolve(bus, Mode::StackRelative);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }
            0x33 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                let v = self.read_m(bus, e);
                self.op_and(v);
            }

            // ---- EOR ----
            0x49 => {
                let v = self.imm_m(bus);
                self.op_eor(v);
            }
            0x45 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x55 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x4D => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x5D => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x59 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x41 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x51 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x52 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x47 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x57 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x4F => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x5F => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x43 => {
                let e = self.resolve(bus, Mode::StackRelative);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }
            0x53 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                let v = self.read_m(bus, e);
                self.op_eor(v);
            }

            // ---- ADC ----
            0x69 => {
                let v = self.imm_m(bus);
                self.adc(v);
            }
            0x65 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x75 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x6D => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x7D => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x79 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x61 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x71 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x72 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x67 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x77 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x6F => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x7F => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x63 => {
                let e = self.resolve(bus, Mode::StackRelative);
                let v = self.read_m(bus, e);
                self.adc(v);
            }
            0x73 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                let v = self.read_m(bus, e);
                self.adc(v);
            }

            // ---- SBC ----
            0xE9 => {
                let v = self.imm_m(bus);
                self.sbc(v);
            }
            0xE5 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xF5 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xED => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xFD => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xF9 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xE1 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xF1 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xF2 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xE7 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xF7 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xEF => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xFF => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xE3 => {
                let e = self.resolve(bus, Mode::StackRelative);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }
            0xF3 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                let v = self.read_m(bus, e);
                self.sbc(v);
            }

            // ---- CMP ----
            0xC9 => {
                let v = self.imm_m(bus);
                self.op_cmp(v);
            }
            0xC5 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xD5 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xCD => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xDD => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xD9 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xC1 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xD1 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xD2 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xC7 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xD7 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xCF => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xDF => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xC3 => {
                let e = self.resolve(bus, Mode::StackRelative);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }
            0xD3 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                let v = self.read_m(bus, e);
                self.op_cmp(v);
            }

            // ---- CPX / CPY ----
            0xE0 => {
                let v = self.imm_x(bus);
                self.op_cpx(v);
            }
            0xE4 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_x(bus, e);
                self.op_cpx(v);
            }
            0xEC => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_x(bus, e);
                self.op_cpx(v);
            }
            0xC0 => {
                let v = self.imm_x(bus);
                self.op_cpy(v);
            }
            0xC4 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_x(bus, e);
                self.op_cpy(v);
            }
            0xCC => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_x(bus, e);
                self.op_cpy(v);
            }

            // ---- BIT ----
            0x89 => {
                let v = self.imm_m(bus);
                self.op_bit_imm(v);
            }
            0x24 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.op_bit(v);
            }
            0x34 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.op_bit(v);
            }
            0x2C => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.op_bit(v);
            }
            0x3C => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_bit(v);
            }

            // ---- LDA ----
            0xA9 => {
                let v = self.imm_m(bus);
                self.op_lda(v);
            }
            0xA5 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xB5 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xAD => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xBD => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xB9 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xA1 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xB1 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.idx_penalty(bus, e);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xB2 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xA7 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xB7 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xAF => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xBF => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xA3 => {
                let e = self.resolve(bus, Mode::StackRelative);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }
            0xB3 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                let v = self.read_m(bus, e);
                self.op_lda(v);
            }

            // ---- LDX / LDY ----
            0xA2 => {
                let v = self.imm_x(bus);
                self.op_ldx(v);
            }
            0xA6 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_x(bus, e);
                self.op_ldx(v);
            }
            0xB6 => {
                let e = self.resolve(bus, Mode::DirectY);
                let v = self.read_x(bus, e);
                self.op_ldx(v);
            }
            0xAE => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_x(bus, e);
                self.op_ldx(v);
            }
            0xBE => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.idx_penalty(bus, e);
                let v = self.read_x(bus, e);
                self.op_ldx(v);
            }
            0xA0 => {
                let v = self.imm_x(bus);
                self.op_ldy(v);
            }
            0xA4 => {
                let e = self.resolve(bus, Mode::Direct);
                let v = self.read_x(bus, e);
                self.op_ldy(v);
            }
            0xB4 => {
                let e = self.resolve(bus, Mode::DirectX);
                let v = self.read_x(bus, e);
                self.op_ldy(v);
            }
            0xAC => {
                let e = self.resolve(bus, Mode::Absolute);
                let v = self.read_x(bus, e);
                self.op_ldy(v);
            }
            0xBC => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.idx_penalty(bus, e);
                let v = self.read_x(bus, e);
                self.op_ldy(v);
            }

            // ---- STA ----
            0x85 => {
                let e = self.resolve(bus, Mode::Direct);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x95 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x8D => {
                let e = self.resolve(bus, Mode::Absolute);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x9D => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x99 => {
                let e = self.resolve(bus, Mode::AbsoluteY);
                self.io(bus);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x81 => {
                let e = self.resolve(bus, Mode::DirectXIndirect);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x91 => {
                let e = self.resolve(bus, Mode::DirectIndirectY);
                self.io(bus);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x92 => {
                let e = self.resolve(bus, Mode::DirectIndirect);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x87 => {
                let e = self.resolve(bus, Mode::DirectIndirectLong);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x97 => {
                let e = self.resolve(bus, Mode::DirectIndirectLongY);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x8F => {
                let e = self.resolve(bus, Mode::AbsoluteLong);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x9F => {
                let e = self.resolve(bus, Mode::AbsoluteLongX);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x83 => {
                let e = self.resolve(bus, Mode::StackRelative);
                self.write_m(bus, e, self.regs.a_val());
            }
            0x93 => {
                let e = self.resolve(bus, Mode::StackRelativeIndirectY);
                self.write_m(bus, e, self.regs.a_val());
            }

            // ---- STX / STY ----
            0x86 => {
                let e = self.resolve(bus, Mode::Direct);
                self.write_x(bus, e, self.regs.x_val());
            }
            0x96 => {
                let e = self.resolve(bus, Mode::DirectY);
                self.write_x(bus, e, self.regs.x_val());
            }
            0x8E => {
                let e = self.resolve(bus, Mode::Absolute);
                self.write_x(bus, e, self.regs.x_val());
            }
            0x84 => {
                let e = self.resolve(bus, Mode::Direct);
                self.write_x(bus, e, self.regs.y_val());
            }
            0x94 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.write_x(bus, e, self.regs.y_val());
            }
            0x8C => {
                let e = self.resolve(bus, Mode::Absolute);
                self.write_x(bus, e, self.regs.y_val());
            }

            // ---- STZ ----
            0x64 => {
                let e = self.resolve(bus, Mode::Direct);
                self.write_m(bus, e, 0);
            }
            0x74 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.write_m(bus, e, 0);
            }
            0x9C => {
                let e = self.resolve(bus, Mode::Absolute);
                self.write_m(bus, e, 0);
            }
            0x9E => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.write_m(bus, e, 0);
            }

            // ---- INC / DEC memory + accumulator ----
            0x1A => {
                self.io(bus);
                self.op_inc_a();
            }
            0x3A => {
                self.io(bus);
                self.op_dec_a();
            }
            0xE6 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_inc(bus, e);
            }
            0xF6 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.rmw_inc(bus, e);
            }
            0xEE => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_inc(bus, e);
            }
            0xFE => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.rmw_inc(bus, e);
            }
            0xC6 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_dec(bus, e);
            }
            0xD6 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.rmw_dec(bus, e);
            }
            0xCE => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_dec(bus, e);
            }
            0xDE => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.rmw_dec(bus, e);
            }

            // ---- ASL / LSR / ROL / ROR ----
            0x0A => {
                self.io(bus);
                self.op_asl_a();
            }
            0x06 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_asl(bus, e);
            }
            0x16 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.rmw_asl(bus, e);
            }
            0x0E => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_asl(bus, e);
            }
            0x1E => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.rmw_asl(bus, e);
            }
            0x4A => {
                self.io(bus);
                self.op_lsr_a();
            }
            0x46 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_lsr(bus, e);
            }
            0x56 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.rmw_lsr(bus, e);
            }
            0x4E => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_lsr(bus, e);
            }
            0x5E => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.rmw_lsr(bus, e);
            }
            0x2A => {
                self.io(bus);
                self.op_rol_a();
            }
            0x26 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_rol(bus, e);
            }
            0x36 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.rmw_rol(bus, e);
            }
            0x2E => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_rol(bus, e);
            }
            0x3E => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.rmw_rol(bus, e);
            }
            0x6A => {
                self.io(bus);
                self.op_ror_a();
            }
            0x66 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_ror(bus, e);
            }
            0x76 => {
                let e = self.resolve(bus, Mode::DirectX);
                self.rmw_ror(bus, e);
            }
            0x6E => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_ror(bus, e);
            }
            0x7E => {
                let e = self.resolve(bus, Mode::AbsoluteX);
                self.io(bus);
                self.rmw_ror(bus, e);
            }

            // ---- TSB / TRB ----
            0x04 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_tsb(bus, e);
            }
            0x0C => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_tsb(bus, e);
            }
            0x14 => {
                let e = self.resolve(bus, Mode::Direct);
                self.rmw_trb(bus, e);
            }
            0x1C => {
                let e = self.resolve(bus, Mode::Absolute);
                self.rmw_trb(bus, e);
            }

            // ---- INX/INY/DEX/DEY ----
            0xE8 => {
                self.io(bus);
                let v = self.regs.x_val().wrapping_add(1);
                self.regs.set_x(v);
                self.regs.set_nz_x(self.regs.x_val());
            }
            0xC8 => {
                self.io(bus);
                let v = self.regs.y_val().wrapping_add(1);
                self.regs.set_y(v);
                self.regs.set_nz_x(self.regs.y_val());
            }
            0xCA => {
                self.io(bus);
                let v = self.regs.x_val().wrapping_sub(1);
                self.regs.set_x(v);
                self.regs.set_nz_x(self.regs.x_val());
            }
            0x88 => {
                self.io(bus);
                let v = self.regs.y_val().wrapping_sub(1);
                self.regs.set_y(v);
                self.regs.set_nz_x(self.regs.y_val());
            }

            // ---- Transfers ----
            0xAA => {
                self.io(bus);
                let v = self.regs.a;
                self.regs.set_x(v);
                self.regs.set_nz_x(self.regs.x_val());
            }
            0xA8 => {
                self.io(bus);
                let v = self.regs.a;
                self.regs.set_y(v);
                self.regs.set_nz_x(self.regs.y_val());
            }
            0x8A => {
                self.io(bus);
                let v = self.regs.x_val();
                self.regs.set_a(v);
                self.regs.set_nz_m(self.regs.a_val());
            }
            0x98 => {
                self.io(bus);
                let v = self.regs.y_val();
                self.regs.set_a(v);
                self.regs.set_nz_m(self.regs.a_val());
            }
            0xBA => {
                self.io(bus);
                let v = self.regs.s;
                self.regs.set_x(v);
                self.regs.set_nz_x(self.regs.x_val());
            }
            0x9A => {
                self.io(bus);
                self.op_txs();
            }
            0x9B => {
                self.io(bus);
                let v = self.regs.x_val();
                self.regs.set_y(v);
                self.regs.set_nz_x(self.regs.y_val());
            }
            0xBB => {
                self.io(bus);
                let v = self.regs.y_val();
                self.regs.set_x(v);
                self.regs.set_nz_x(self.regs.x_val());
            }
            0x5B => {
                self.io(bus);
                self.regs.d = self.regs.a;
                self.regs.set_nz16(self.regs.d);
            }
            0x7B => {
                self.io(bus);
                let v = self.regs.d;
                self.regs.a = v;
                self.regs.set_nz16(v);
            }
            0x1B => {
                self.io(bus);
                self.op_tcs();
            }
            0x3B => {
                self.io(bus);
                let v = self.regs.s;
                self.regs.a = v;
                self.regs.set_nz16(v);
            }

            // ---- Stack push/pull ----
            0x48 => {
                self.io(bus);
                self.op_pha(bus);
            }
            0x68 => {
                self.io(bus);
                self.io(bus);
                self.op_pla(bus);
            }
            0xDA => {
                self.io(bus);
                self.op_phx(bus);
            }
            0xFA => {
                self.io(bus);
                self.io(bus);
                self.op_plx(bus);
            }
            0x5A => {
                self.io(bus);
                self.op_phy(bus);
            }
            0x7A => {
                self.io(bus);
                self.io(bus);
                self.op_ply(bus);
            }
            0x08 => {
                self.io(bus);
                self.push8(bus, self.regs.p.bits());
            }
            0x28 => {
                self.io(bus);
                self.io(bus);
                self.op_plp(bus);
            }
            0x8B => {
                self.io(bus);
                self.push8(bus, self.regs.dbr);
            }
            0xAB => {
                self.io(bus);
                self.io(bus);
                let v = self.pull_n8(bus); // PLB uses pullN
                self.regs.dbr = v;
                self.regs.set_nz8(v);
            }
            0x0B => {
                self.io(bus);
                self.push_n16(bus, self.regs.d); // PHD uses pushN
            }
            0x2B => {
                self.io(bus);
                self.io(bus);
                let v = self.pull_n16(bus); // PLD uses pullN
                self.regs.d = v;
                self.regs.set_nz16(v);
            }
            0x4B => {
                self.io(bus);
                self.push8(bus, self.regs.pbr);
            }
            0xF4 => {
                let v = self.fetch16(bus);
                self.push_n16(bus, v); // PEA uses pushN
            } // PEA
            0xD4 => {
                self.dp_penalty(bus);
                let dp = u16::from(self.fetch8(bus));
                // PEI pointer fetch uses readDirectN (no page-lock); push uses pushN.
                let lo = self.bus_read8(bus, self.direct_n_addr(dp));
                let hi = self.bus_read8(bus, self.direct_n_addr(dp.wrapping_add(1)));
                let v = u16::from(lo) | (u16::from(hi) << 8);
                self.push_n16(bus, v);
            } // PEI
            0x62 => {
                let off = self.fetch16(bus);
                self.io(bus);
                let v = self.regs.pc.wrapping_add(off);
                self.push_n16(bus, v); // PER uses pushN
            } // PER

            // ---- Flag ops ----
            0x18 => {
                self.io(bus);
                self.regs.set_flag(Status::C, false);
            }
            0x38 => {
                self.io(bus);
                self.regs.set_flag(Status::C, true);
            }
            0x58 => {
                self.io(bus);
                self.regs.set_flag(Status::I, false);
            }
            0x78 => {
                self.io(bus);
                self.regs.set_flag(Status::I, true);
            }
            0xD8 => {
                self.io(bus);
                self.regs.set_flag(Status::D, false);
            }
            0xF8 => {
                self.io(bus);
                self.regs.set_flag(Status::D, true);
            }
            0xB8 => {
                self.io(bus);
                self.regs.set_flag(Status::V, false);
            }
            0xC2 => {
                let m = self.fetch8(bus);
                self.io(bus);
                self.op_rep(m);
            }
            0xE2 => {
                let m = self.fetch8(bus);
                self.io(bus);
                self.op_sep(m);
            }
            0xFB => {
                self.io(bus);
                self.op_xce();
            }

            // ---- Branches ----
            0x10 => {
                self.branch(bus, !self.regs.p.contains(Status::N));
            }
            0x30 => {
                self.branch(bus, self.regs.p.contains(Status::N));
            }
            0x50 => {
                self.branch(bus, !self.regs.p.contains(Status::V));
            }
            0x70 => {
                self.branch(bus, self.regs.p.contains(Status::V));
            }
            0x90 => {
                self.branch(bus, !self.regs.p.contains(Status::C));
            }
            0xB0 => {
                self.branch(bus, self.regs.p.contains(Status::C));
            }
            0xD0 => {
                self.branch(bus, !self.regs.p.contains(Status::Z));
            }
            0xF0 => {
                self.branch(bus, self.regs.p.contains(Status::Z));
            }
            0x80 => {
                self.branch(bus, true);
            }
            0x82 => {
                self.op_brl(bus);
            }

            // ---- Jumps / calls / returns ----
            0x4C => {
                let off = self.fetch16(bus);
                self.regs.pc = off;
            }
            0x6C => {
                let ptr = self.fetch16(bus);
                let lo = self.bus_read8(bus, u32::from(ptr));
                let hi = self.bus_read8(bus, u32::from(ptr.wrapping_add(1)));
                self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
            }
            0x7C => {
                let base = self.fetch16(bus);
                self.io(bus);
                let ptr = base.wrapping_add(self.regs.x_val());
                let addr = (u32::from(self.regs.pbr) << 16) | u32::from(ptr);
                let lo = self.bus_read8(bus, addr);
                let hi = self.bus_read8(
                    bus,
                    (u32::from(self.regs.pbr) << 16) | u32::from(ptr.wrapping_add(1)),
                );
                self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
            }
            0x5C => {
                let addr = self.fetch24(bus);
                self.regs.pc = addr as u16;
                self.regs.pbr = (addr >> 16) as u8;
            }
            0xDC => {
                let ptr = self.fetch16(bus);
                let lo = self.bus_read8(bus, u32::from(ptr));
                let mid = self.bus_read8(bus, u32::from(ptr.wrapping_add(1)));
                let hi = self.bus_read8(bus, u32::from(ptr.wrapping_add(2)));
                self.regs.pc = u16::from(lo) | (u16::from(mid) << 8);
                self.regs.pbr = hi;
            }
            0x20 => {
                self.op_jsr(bus);
            }
            0xFC => {
                self.op_jsr_abs_x_ind(bus);
            }
            0x22 => {
                self.op_jsl(bus);
            }
            0x60 => {
                self.op_rts(bus);
            }
            0x6B => {
                self.op_rtl(bus);
            }
            0x40 => {
                self.op_rti(bus);
            }

            // ---- Block move ----
            0x54 => {
                self.op_mvn(bus);
            }
            0x44 => {
                self.op_mvp(bus);
            }

            // ---- Interrupts / misc ----
            0x00 => {
                let _ = self.fetch8(bus);
                self.op_brk(bus);
            }
            0x02 => {
                let _ = self.fetch8(bus);
                self.op_cop(bus);
            }
            0xDB => {
                // STP: opcode fetch + 3 internal cycles (oracle = 4 total), then halt.
                self.io(bus);
                self.io(bus);
                self.io(bus);
                self.stopped = true;
            }
            0xCB => {
                // WAI: opcode fetch + 3 internal cycles (oracle = 4 total), then wait.
                self.io(bus);
                self.io(bus);
                self.io(bus);
                self.waiting = true;
            }
            0xEA => {
                self.io(bus);
            }
            0x42 => {
                let _ = self.fetch8(bus);
            } // WDM: 2-byte no-op

            // ---- XBA: exchange the high and low bytes of the 16-bit accumulator ----
            0xEB => {
                self.io(bus);
                self.io(bus);
                self.op_xba();
            }
        }
    }

    // ----------------------------------------------------------------------------------
    // Indexed page-cross penalty (read-path indexed modes).
    // ----------------------------------------------------------------------------------

    fn idx_penalty(&mut self, bus: &mut impl Bus, e: Effective) {
        // In 8-bit index mode a page cross always costs +1; in 16-bit index mode the extra
        // cycle is always taken regardless (the index high byte is always added).
        if e.page_cross || !self.regs.x8() {
            self.io(bus);
        }
    }

    // ----------------------------------------------------------------------------------
    // Simple register ops.
    // ----------------------------------------------------------------------------------

    const fn op_ora(&mut self, v: u16) {
        let r = self.regs.a_val() | v;
        self.regs.set_a(r);
        self.regs.set_nz_m(self.regs.a_val());
    }
    const fn op_and(&mut self, v: u16) {
        let r = self.regs.a_val() & v;
        self.regs.set_a(r);
        self.regs.set_nz_m(self.regs.a_val());
    }
    const fn op_eor(&mut self, v: u16) {
        let r = self.regs.a_val() ^ v;
        self.regs.set_a(r);
        self.regs.set_nz_m(self.regs.a_val());
    }
    const fn op_lda(&mut self, v: u16) {
        self.regs.set_a(v);
        self.regs.set_nz_m(self.regs.a_val());
    }
    const fn op_ldx(&mut self, v: u16) {
        self.regs.set_x(v);
        self.regs.set_nz_x(self.regs.x_val());
    }
    const fn op_ldy(&mut self, v: u16) {
        self.regs.set_y(v);
        self.regs.set_nz_x(self.regs.y_val());
    }
    fn op_cmp(&mut self, v: u16) {
        let a = self.regs.a_val();
        self.compare(a, v, self.regs.m8());
    }
    fn op_cpx(&mut self, v: u16) {
        let x = self.regs.x_val();
        self.compare(x, v, self.regs.x8());
    }
    fn op_cpy(&mut self, v: u16) {
        let y = self.regs.y_val();
        self.compare(y, v, self.regs.x8());
    }
    const fn op_bit(&mut self, v: u16) {
        let a = self.regs.a_val();
        if self.regs.m8() {
            let z = (v & a) & 0xFF;
            self.regs.set_flag(Status::Z, z == 0);
            self.regs.set_flag(Status::V, v & 0x40 != 0);
            self.regs.set_flag(Status::N, v & 0x80 != 0);
        } else {
            self.regs.set_flag(Status::Z, (v & a) == 0);
            self.regs.set_flag(Status::V, v & 0x4000 != 0);
            self.regs.set_flag(Status::N, v & 0x8000 != 0);
        }
    }
    const fn op_bit_imm(&mut self, v: u16) {
        // Immediate BIT affects only Z.
        let a = self.regs.a_val();
        let mask = if self.regs.m8() { 0x00FF } else { 0xFFFF };
        let z = (v & a) & mask;
        self.regs.set_flag(Status::Z, z == 0);
    }
    fn op_inc_a(&mut self) {
        let v = self.regs.a_val().wrapping_add(1);
        self.regs.set_a(v);
        self.regs.set_nz_m(self.regs.a_val());
    }
    fn op_dec_a(&mut self) {
        let v = self.regs.a_val().wrapping_sub(1);
        self.regs.set_a(v);
        self.regs.set_nz_m(self.regs.a_val());
    }
    fn op_asl_a(&mut self) {
        let v = self.regs.a_val();
        let (r, c) = if self.regs.m8() {
            ((v << 1) & 0xFF, v & 0x80 != 0)
        } else {
            (v << 1, v & 0x8000 != 0)
        };
        self.regs.set_a(r);
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(self.regs.a_val());
    }
    fn op_lsr_a(&mut self) {
        let v = self.regs.a_val();
        let c = v & 1 != 0;
        let r = v >> 1;
        self.regs.set_a(r);
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(self.regs.a_val());
    }
    fn op_rol_a(&mut self) {
        let v = self.regs.a_val();
        let carry_in = u16::from(self.regs.p.contains(Status::C));
        let (r, c) = if self.regs.m8() {
            (((v << 1) | carry_in) & 0xFF, v & 0x80 != 0)
        } else {
            ((v << 1) | carry_in, v & 0x8000 != 0)
        };
        self.regs.set_a(r);
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(self.regs.a_val());
    }
    fn op_ror_a(&mut self) {
        let v = self.regs.a_val();
        let carry_in = u16::from(self.regs.p.contains(Status::C));
        let c = v & 1 != 0;
        let r = if self.regs.m8() {
            (v >> 1) | (carry_in << 7)
        } else {
            (v >> 1) | (carry_in << 15)
        };
        self.regs.set_a(r);
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(self.regs.a_val());
    }
    fn op_xba(&mut self) {
        let lo = self.regs.a & 0x00FF;
        let hi = (self.regs.a >> 8) & 0x00FF;
        self.regs.a = (lo << 8) | hi;
        // NZ set from the new low byte (the old high byte), always 8-bit.
        self.regs.set_nz8(hi as u8);
    }
    fn op_txs(&mut self) {
        if self.regs.emulation {
            self.regs.s = 0x0100 | (self.regs.x_val() & 0x00FF);
        } else {
            self.regs.s = self.regs.x_val();
        }
    }
    fn op_tcs(&mut self) {
        if self.regs.emulation {
            self.regs.s = 0x0100 | (self.regs.a & 0x00FF);
        } else {
            self.regs.s = self.regs.a;
        }
    }

    // ----------------------------------------------------------------------------------
    // Read-modify-write memory ops (width-aware, with the internal modify cycle).
    // ----------------------------------------------------------------------------------

    fn rmw_inc(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let r = if self.regs.m8() {
            (v.wrapping_add(1)) & 0xFF
        } else {
            v.wrapping_add(1)
        };
        self.regs.set_nz_m(r);
        self.write_m(bus, e, r);
    }
    fn rmw_dec(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let r = if self.regs.m8() {
            (v.wrapping_sub(1)) & 0xFF
        } else {
            v.wrapping_sub(1)
        };
        self.regs.set_nz_m(r);
        self.write_m(bus, e, r);
    }
    fn rmw_asl(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let (r, c) = if self.regs.m8() {
            ((v << 1) & 0xFF, v & 0x80 != 0)
        } else {
            (v << 1, v & 0x8000 != 0)
        };
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(r);
        self.write_m(bus, e, r);
    }
    fn rmw_lsr(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let c = v & 1 != 0;
        let r = v >> 1;
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(r);
        self.write_m(bus, e, r);
    }
    fn rmw_rol(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let carry_in = u16::from(self.regs.p.contains(Status::C));
        let (r, c) = if self.regs.m8() {
            (((v << 1) | carry_in) & 0xFF, v & 0x80 != 0)
        } else {
            ((v << 1) | carry_in, v & 0x8000 != 0)
        };
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(r);
        self.write_m(bus, e, r);
    }
    fn rmw_ror(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let carry_in = u16::from(self.regs.p.contains(Status::C));
        let c = v & 1 != 0;
        let r = if self.regs.m8() {
            (v >> 1) | (carry_in << 7)
        } else {
            (v >> 1) | (carry_in << 15)
        };
        self.regs.set_flag(Status::C, c);
        self.regs.set_nz_m(r);
        self.write_m(bus, e, r);
    }
    fn rmw_tsb(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let a = self.regs.a_val();
        let mask = if self.regs.m8() { 0x00FF } else { 0xFFFF };
        self.regs.set_flag(Status::Z, (v & a) & mask == 0);
        let r = v | a;
        self.write_m(bus, e, r);
    }
    fn rmw_trb(&mut self, bus: &mut impl Bus, e: Effective) {
        let v = self.read_m(bus, e);
        self.io(bus);
        let a = self.regs.a_val();
        let mask = if self.regs.m8() { 0x00FF } else { 0xFFFF };
        self.regs.set_flag(Status::Z, (v & a) & mask == 0);
        let r = v & !a;
        self.write_m(bus, e, r);
    }

    // ----------------------------------------------------------------------------------
    // Stack ops bodies.
    // ----------------------------------------------------------------------------------

    fn op_pha(&mut self, bus: &mut impl Bus) {
        if self.regs.m8() {
            self.push8(bus, self.regs.a as u8);
        } else {
            self.push16(bus, self.regs.a);
        }
    }
    fn op_pla(&mut self, bus: &mut impl Bus) {
        if self.regs.m8() {
            let v = self.pull8(bus);
            self.regs.a = (self.regs.a & 0xFF00) | u16::from(v);
            self.regs.set_nz8(v);
        } else {
            let v = self.pull16(bus);
            self.regs.a = v;
            self.regs.set_nz16(v);
        }
    }
    fn op_phx(&mut self, bus: &mut impl Bus) {
        if self.regs.x8() {
            self.push8(bus, self.regs.x as u8);
        } else {
            self.push16(bus, self.regs.x);
        }
    }
    fn op_plx(&mut self, bus: &mut impl Bus) {
        if self.regs.x8() {
            let v = self.pull8(bus);
            self.regs.set_x(u16::from(v));
            self.regs.set_nz8(v);
        } else {
            let v = self.pull16(bus);
            self.regs.set_x(v);
            self.regs.set_nz16(v);
        }
    }
    fn op_phy(&mut self, bus: &mut impl Bus) {
        if self.regs.x8() {
            self.push8(bus, self.regs.y as u8);
        } else {
            self.push16(bus, self.regs.y);
        }
    }
    fn op_ply(&mut self, bus: &mut impl Bus) {
        if self.regs.x8() {
            let v = self.pull8(bus);
            self.regs.set_y(u16::from(v));
            self.regs.set_nz8(v);
        } else {
            let v = self.pull16(bus);
            self.regs.set_y(v);
            self.regs.set_nz16(v);
        }
    }
    fn op_plp(&mut self, bus: &mut impl Bus) {
        let v = self.pull8(bus);
        self.regs.p = Status::from_bits_truncate(v);
        if self.regs.emulation {
            // In emulation mode M and X are forced set.
            self.regs.set_flag(Status::M, true);
            self.regs.set_flag(Status::X, true);
        }
        if self.regs.x8() {
            // Re-zero index high bytes if X became 8-bit.
            self.regs.x &= 0x00FF;
            self.regs.y &= 0x00FF;
        }
    }

    // ----------------------------------------------------------------------------------
    // REP / SEP / XCE.
    // ----------------------------------------------------------------------------------

    fn op_rep(&mut self, mask: u8) {
        let new = Status::from_bits_truncate(self.regs.p.bits() & !mask);
        self.regs.p = new;
        if self.regs.emulation {
            self.regs.set_flag(Status::M, true);
            self.regs.set_flag(Status::X, true);
        }
    }
    fn op_sep(&mut self, mask: u8) {
        let new = Status::from_bits_truncate(self.regs.p.bits() | mask);
        self.regs.p = new;
        if self.regs.x8() {
            self.regs.x &= 0x00FF;
            self.regs.y &= 0x00FF;
        }
    }
    fn op_xce(&mut self) {
        let carry = self.regs.p.contains(Status::C);
        let e = self.regs.emulation;
        self.regs.set_flag(Status::C, e);
        self.regs.emulation = carry;
        if self.regs.emulation {
            self.regs.set_flag(Status::M, true);
            self.regs.set_flag(Status::X, true);
            self.regs.x &= 0x00FF;
            self.regs.y &= 0x00FF;
            self.regs.s = 0x0100 | (self.regs.s & 0x00FF);
        }
    }

    // ----------------------------------------------------------------------------------
    // Branches.
    // ----------------------------------------------------------------------------------

    fn branch(&mut self, bus: &mut impl Bus, taken: bool) {
        let off = self.fetch8(bus) as i8;
        if taken {
            self.io(bus);
            let old = self.regs.pc;
            let new = (old as i16).wrapping_add(i16::from(off)) as u16;
            if self.regs.emulation && (old & 0xFF00) != (new & 0xFF00) {
                self.io(bus);
            }
            self.regs.pc = new;
        }
    }
    fn op_brl(&mut self, bus: &mut impl Bus) {
        let off = self.fetch16(bus);
        self.io(bus);
        self.regs.pc = self.regs.pc.wrapping_add(off);
    }

    // ----------------------------------------------------------------------------------
    // Calls / returns.
    // ----------------------------------------------------------------------------------

    fn op_jsr(&mut self, bus: &mut impl Bus) {
        let target = self.fetch16(bus);
        self.io(bus);
        let ret = self.regs.pc.wrapping_sub(1);
        self.push16(bus, ret);
        self.regs.pc = target;
    }
    fn op_jsr_abs_x_ind(&mut self, bus: &mut impl Bus) {
        // JSR (abs,X): push return, then read pointer at PBR:(operand+X).
        let base = self.fetch16(bus);
        let ret = self.regs.pc.wrapping_sub(1);
        self.push16(bus, ret);
        self.io(bus);
        let ptr = base.wrapping_add(self.regs.x_val());
        let lo = self.bus_read8(bus, (u32::from(self.regs.pbr) << 16) | u32::from(ptr));
        let hi = self.bus_read8(
            bus,
            (u32::from(self.regs.pbr) << 16) | u32::from(ptr.wrapping_add(1)),
        );
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
    }
    fn op_jsl(&mut self, bus: &mut impl Bus) {
        let lo = self.fetch8(bus);
        let hi = self.fetch8(bus);
        // ares `CallLong` pushes via `pushN` (full 16-bit S, no emulation page-1 confinement
        // mid-instruction); the boundary `normalize_emulation` re-confines S afterwards. Using
        // the page-locked `push8`/`push16` corrupts S when it crosses a page in emulation mode.
        self.push_n8(bus, self.regs.pbr);
        self.io(bus);
        let bank = self.fetch8(bus);
        let ret = self.regs.pc.wrapping_sub(1);
        self.push_n16(bus, ret);
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
        self.regs.pbr = bank;
    }
    fn op_rts(&mut self, bus: &mut impl Bus) {
        self.io(bus);
        self.io(bus);
        let pc = self.pull16(bus);
        self.io(bus);
        self.regs.pc = pc.wrapping_add(1);
    }
    fn op_rtl(&mut self, bus: &mut impl Bus) {
        self.io(bus);
        self.io(bus);
        // ares `instructionReturnLong` pulls via `pullN` (full 16-bit S, no page-1 confinement),
        // then forces `S.h = 0x01` at the boundary. Page-locked `pull16`/`pull8` corrupt S on a
        // page wrap in emulation mode. `normalize_emulation` re-applies the `S.h=0x01`.
        let pc = self.pull_n16(bus);
        let bank = self.pull_n8(bus);
        self.regs.pc = pc.wrapping_add(1);
        self.regs.pbr = bank;
    }
    fn op_rti(&mut self, bus: &mut impl Bus) {
        self.io(bus);
        self.io(bus);
        let p = self.pull8(bus);
        self.regs.p = Status::from_bits_truncate(p);
        if self.regs.emulation {
            self.regs.set_flag(Status::M, true);
            self.regs.set_flag(Status::X, true);
        }
        let pc = self.pull16(bus);
        self.regs.pc = pc;
        if !self.regs.emulation {
            let bank = self.pull8(bus);
            self.regs.pbr = bank;
        }
        if self.regs.x8() {
            self.regs.x &= 0x00FF;
            self.regs.y &= 0x00FF;
        }
    }

    // ----------------------------------------------------------------------------------
    // Block moves. Each iteration moves one byte; PC stays on the instruction until X/Y/A
    // exhaust (A == 0xFFFF after the final decrement). We model the full transfer per step()
    // for simplicity (count is per-byte accurate; interruptibility is approximated).
    // ----------------------------------------------------------------------------------

    /// `MVN` (`0x54`, adjust `+1`) — block move forward. Clean-room port of ares
    /// `instructionBlockMove8/16`: the source/dest addresses use the **full 16-bit** `X`/`Y`
    /// (`V.b<<16 | X.w`); only the *increment* respects the index width — `X.l += adjust` when
    /// `X=1` (8-bit index, high byte preserved), `X.w += adjust` when `X=0`. The loop tests `A.w`
    /// *then* post-decrements (`if(A.w--) PC.w -= 3`), so it moves `A+1` bytes.
    fn op_mvn(&mut self, bus: &mut impl Bus) {
        self.block_move(bus, 1);
    }
    /// `MVP` (`0x44`, adjust `-1`) — block move backward. See [`Self::op_mvn`].
    fn op_mvp(&mut self, bus: &mut impl Bus) {
        self.block_move(bus, -1i16 as u16);
    }
    /// Shared block-move body (ares `instructionBlockMove8/16`). `adjust` is `+1` (MVN) or `-1`
    /// (MVP, passed as `0xFFFF`).
    fn block_move(&mut self, bus: &mut impl Bus, adjust: u16) {
        let dst_bank = self.fetch8(bus);
        let src_bank = self.fetch8(bus);
        self.regs.dbr = dst_bank;
        // Address uses the full 16-bit X/Y regardless of index width (ares `X.w`).
        let src = (u32::from(src_bank) << 16) | u32::from(self.regs.x);
        let dst = (u32::from(dst_bank) << 16) | u32::from(self.regs.y);
        let b = self.bus_read8(bus, src);
        self.bus_write8(bus, dst, b);
        self.io(bus);
        self.io(bus);
        // Increment respects index width: 8-bit keeps the high byte (`X.l += adjust`).
        if self.regs.x8() {
            self.regs.x =
                (self.regs.x & 0xFF00) | u16::from((self.regs.x as u8).wrapping_add(adjust as u8));
            self.regs.y =
                (self.regs.y & 0xFF00) | u16::from((self.regs.y as u8).wrapping_add(adjust as u8));
        } else {
            self.regs.x = self.regs.x.wrapping_add(adjust);
            self.regs.y = self.regs.y.wrapping_add(adjust);
        }
        // `if(A.w--) PC.w -= 3`: test the pre-decrement value, then decrement.
        let continue_move = self.regs.a != 0;
        self.regs.a = self.regs.a.wrapping_sub(1);
        if continue_move {
            self.regs.pc = self.regs.pc.wrapping_sub(3);
        }
    }

    // ----------------------------------------------------------------------------------
    // Software interrupts BRK / COP.
    // ----------------------------------------------------------------------------------

    fn op_brk(&mut self, bus: &mut impl Bus) {
        // Signature byte already consumed by the caller; PC now points past it.
        if !self.regs.emulation {
            self.push8(bus, self.regs.pbr);
        }
        self.push16(bus, self.regs.pc);
        let mut status = self.regs.p.bits();
        if self.regs.emulation {
            status |= Status::X.bits(); // B set for BRK
        }
        self.push8(bus, status);
        self.regs.set_flag(Status::I, true);
        self.regs.set_flag(Status::D, false);
        let vector = if self.regs.emulation {
            vectors::IRQ_BRK_EMU
        } else {
            vectors::BRK_NATIVE
        };
        let pc = self.read16(bus, vector);
        self.regs.pc = pc;
        self.regs.pbr = 0;
    }
    fn op_cop(&mut self, bus: &mut impl Bus) {
        if !self.regs.emulation {
            self.push8(bus, self.regs.pbr);
        }
        self.push16(bus, self.regs.pc);
        self.push8(bus, self.regs.p.bits());
        self.regs.set_flag(Status::I, true);
        self.regs.set_flag(Status::D, false);
        let vector = if self.regs.emulation {
            vectors::COP_EMU
        } else {
            vectors::COP_NATIVE
        };
        let pc = self.read16(bus, vector);
        self.regs.pc = pc;
        self.regs.pbr = 0;
    }
}
