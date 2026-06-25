//! SPC700 instruction execution — the cycle-accurate per-opcode dispatch.
//!
//! Structure (dispatch table, addressing-mode handlers, cycle placement of `idle`/dummy
//! reads) is derived clean-room from ares `instruction.cpp` / `instructions.cpp` (ISC) and
//! pinned to the SingleStepTests/spc700 oracle. Every `read`/`write`/`idle` is one cycle.
//!
//! PC/displacement math is byte-oriented; the u16↔u8 truncations and signed reinterpretations
//! below are intentional and bounded by the architecture.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_const_for_fn,
    clippy::needless_pass_by_ref_mut
)]

use crate::spc700::{AluB, AluU, AluW, Index, Spc700, Spc700Bus};

impl Spc700 {
    /// Execute exactly one instruction, returning nothing; the bus accumulates the cycle count.
    ///
    /// `STOP`/`SLEEP` halt the core: once halted, `step` performs the documented dead-cycle
    /// pair (`read(PC)` + `idle`) per call and makes no architectural progress until reset.
    #[allow(clippy::too_many_lines)]
    pub fn step(&mut self, bus: &mut impl Spc700Bus) {
        if self.stopped || self.waiting {
            // Halted: ares loops read(PC)+idle until a sync/interrupt. One pair per call.
            bus.read(self.regs.pc);
            bus.idle();
            return;
        }

        let opcode = self.fetch(bus);
        match opcode {
            0x00 => self.i_nop(bus),
            0x01 => self.i_call_table(bus, 0),
            0x02 => self.i_abs_bit_set(bus, 0, true),
            0x03 => self.i_branch_bit(bus, 0, true),
            0x04 => self.i_direct_read(bus, AluB::Or, Reg::A),
            0x05 => self.i_absolute_read(bus, AluB::Or, Reg::A),
            0x06 => self.i_indirect_x_read(bus, AluB::Or),
            0x07 => self.i_indexed_indirect_read(bus, AluB::Or),
            0x08 => self.i_immediate_read(bus, AluB::Or, Reg::A),
            0x09 => self.i_direct_direct_modify(bus, AluB::Or),
            0x0A => self.i_abs_bit_modify(bus, 0),
            0x0B => self.i_direct_modify(bus, AluU::Asl),
            0x0C => self.i_absolute_modify(bus, AluU::Asl),
            0x0D => self.i_push(bus, self.regs.psw.bits()),
            0x0E => self.i_test_set_bits(bus, true),
            0x0F => self.i_break(bus),
            0x10 => self.i_branch(bus, !self.regs.psw.n()),
            0x11 => self.i_call_table(bus, 1),
            0x12 => self.i_abs_bit_set(bus, 0, false),
            0x13 => self.i_branch_bit(bus, 0, false),
            0x14 => self.i_direct_indexed_read(bus, AluB::Or, Reg::A, Index::X),
            0x15 => self.i_absolute_indexed_read(bus, AluB::Or, Index::X),
            0x16 => self.i_absolute_indexed_read(bus, AluB::Or, Index::Y),
            0x17 => self.i_indirect_indexed_read(bus, AluB::Or),
            0x18 => self.i_direct_immediate_modify(bus, AluB::Or),
            0x19 => self.i_indirect_x_write_indirect_y(bus, AluB::Or),
            0x1A => self.i_direct_modify_word(bus, -1),
            0x1B => self.i_direct_indexed_modify(bus, AluU::Asl),
            0x1C => self.i_implied_modify(bus, AluU::Asl, Reg::A),
            0x1D => self.i_implied_modify(bus, AluU::Dec, Reg::X),
            0x1E => self.i_absolute_read(bus, AluB::Cmp, Reg::X),
            0x1F => self.i_jump_indirect_x(bus),
            0x20 => self.i_flag_p(bus, false),
            0x21 => self.i_call_table(bus, 2),
            0x22 => self.i_abs_bit_set(bus, 1, true),
            0x23 => self.i_branch_bit(bus, 1, true),
            0x24 => self.i_direct_read(bus, AluB::And, Reg::A),
            0x25 => self.i_absolute_read(bus, AluB::And, Reg::A),
            0x26 => self.i_indirect_x_read(bus, AluB::And),
            0x27 => self.i_indexed_indirect_read(bus, AluB::And),
            0x28 => self.i_immediate_read(bus, AluB::And, Reg::A),
            0x29 => self.i_direct_direct_modify(bus, AluB::And),
            0x2A => self.i_abs_bit_modify(bus, 1),
            0x2B => self.i_direct_modify(bus, AluU::Rol),
            0x2C => self.i_absolute_modify(bus, AluU::Rol),
            0x2D => self.i_push(bus, self.regs.a),
            0x2E => self.i_branch_not_direct(bus),
            0x2F => self.i_branch(bus, true),
            0x30 => self.i_branch(bus, self.regs.psw.n()),
            0x31 => self.i_call_table(bus, 3),
            0x32 => self.i_abs_bit_set(bus, 1, false),
            0x33 => self.i_branch_bit(bus, 1, false),
            0x34 => self.i_direct_indexed_read(bus, AluB::And, Reg::A, Index::X),
            0x35 => self.i_absolute_indexed_read(bus, AluB::And, Index::X),
            0x36 => self.i_absolute_indexed_read(bus, AluB::And, Index::Y),
            0x37 => self.i_indirect_indexed_read(bus, AluB::And),
            0x38 => self.i_direct_immediate_modify(bus, AluB::And),
            0x39 => self.i_indirect_x_write_indirect_y(bus, AluB::And),
            0x3A => self.i_direct_modify_word(bus, 1),
            0x3B => self.i_direct_indexed_modify(bus, AluU::Rol),
            0x3C => self.i_implied_modify(bus, AluU::Rol, Reg::A),
            0x3D => self.i_implied_modify(bus, AluU::Inc, Reg::X),
            0x3E => self.i_direct_read(bus, AluB::Cmp, Reg::X),
            0x3F => self.i_call_absolute(bus),
            0x40 => self.i_flag_p(bus, true),
            0x41 => self.i_call_table(bus, 4),
            0x42 => self.i_abs_bit_set(bus, 2, true),
            0x43 => self.i_branch_bit(bus, 2, true),
            0x44 => self.i_direct_read(bus, AluB::Eor, Reg::A),
            0x45 => self.i_absolute_read(bus, AluB::Eor, Reg::A),
            0x46 => self.i_indirect_x_read(bus, AluB::Eor),
            0x47 => self.i_indexed_indirect_read(bus, AluB::Eor),
            0x48 => self.i_immediate_read(bus, AluB::Eor, Reg::A),
            0x49 => self.i_direct_direct_modify(bus, AluB::Eor),
            0x4A => self.i_abs_bit_modify(bus, 2),
            0x4B => self.i_direct_modify(bus, AluU::Lsr),
            0x4C => self.i_absolute_modify(bus, AluU::Lsr),
            0x4D => self.i_push(bus, self.regs.x),
            0x4E => self.i_test_set_bits(bus, false),
            0x4F => self.i_call_page(bus),
            0x50 => self.i_branch(bus, !self.regs.psw.v()),
            0x51 => self.i_call_table(bus, 5),
            0x52 => self.i_abs_bit_set(bus, 2, false),
            0x53 => self.i_branch_bit(bus, 2, false),
            0x54 => self.i_direct_indexed_read(bus, AluB::Eor, Reg::A, Index::X),
            0x55 => self.i_absolute_indexed_read(bus, AluB::Eor, Index::X),
            0x56 => self.i_absolute_indexed_read(bus, AluB::Eor, Index::Y),
            0x57 => self.i_indirect_indexed_read(bus, AluB::Eor),
            0x58 => self.i_direct_immediate_modify(bus, AluB::Eor),
            0x59 => self.i_indirect_x_write_indirect_y(bus, AluB::Eor),
            0x5A => self.i_direct_compare_word(bus, AluW::Cmp),
            0x5B => self.i_direct_indexed_modify(bus, AluU::Lsr),
            0x5C => self.i_implied_modify(bus, AluU::Lsr, Reg::A),
            0x5D => self.i_transfer(bus, Reg::A, Reg::X),
            0x5E => self.i_absolute_read(bus, AluB::Cmp, Reg::Y),
            0x5F => self.i_jump_absolute(bus),
            0x60 => self.i_flag_c(bus, false),
            0x61 => self.i_call_table(bus, 6),
            0x62 => self.i_abs_bit_set(bus, 3, true),
            0x63 => self.i_branch_bit(bus, 3, true),
            0x64 => self.i_direct_read(bus, AluB::Cmp, Reg::A),
            0x65 => self.i_absolute_read(bus, AluB::Cmp, Reg::A),
            0x66 => self.i_indirect_x_read(bus, AluB::Cmp),
            0x67 => self.i_indexed_indirect_read(bus, AluB::Cmp),
            0x68 => self.i_immediate_read(bus, AluB::Cmp, Reg::A),
            0x69 => self.i_direct_direct_compare(bus),
            0x6A => self.i_abs_bit_modify(bus, 3),
            0x6B => self.i_direct_modify(bus, AluU::Ror),
            0x6C => self.i_absolute_modify(bus, AluU::Ror),
            0x6D => self.i_push(bus, self.regs.y),
            0x6E => self.i_branch_not_direct_decrement(bus),
            0x6F => self.i_return_subroutine(bus),
            0x70 => self.i_branch(bus, self.regs.psw.v()),
            0x71 => self.i_call_table(bus, 7),
            0x72 => self.i_abs_bit_set(bus, 3, false),
            0x73 => self.i_branch_bit(bus, 3, false),
            0x74 => self.i_direct_indexed_read(bus, AluB::Cmp, Reg::A, Index::X),
            0x75 => self.i_absolute_indexed_read(bus, AluB::Cmp, Index::X),
            0x76 => self.i_absolute_indexed_read(bus, AluB::Cmp, Index::Y),
            0x77 => self.i_indirect_indexed_read(bus, AluB::Cmp),
            0x78 => self.i_direct_immediate_compare(bus),
            0x79 => self.i_indirect_x_compare_indirect_y(bus),
            0x7A => self.i_direct_read_word(bus, AluW::Add),
            0x7B => self.i_direct_indexed_modify(bus, AluU::Ror),
            0x7C => self.i_implied_modify(bus, AluU::Ror, Reg::A),
            0x7D => self.i_transfer(bus, Reg::X, Reg::A),
            0x7E => self.i_direct_read(bus, AluB::Cmp, Reg::Y),
            0x7F => self.i_return_interrupt(bus),
            0x80 => self.i_flag_c(bus, true),
            0x81 => self.i_call_table(bus, 8),
            0x82 => self.i_abs_bit_set(bus, 4, true),
            0x83 => self.i_branch_bit(bus, 4, true),
            0x84 => self.i_direct_read(bus, AluB::Adc, Reg::A),
            0x85 => self.i_absolute_read(bus, AluB::Adc, Reg::A),
            0x86 => self.i_indirect_x_read(bus, AluB::Adc),
            0x87 => self.i_indexed_indirect_read(bus, AluB::Adc),
            0x88 => self.i_immediate_read(bus, AluB::Adc, Reg::A),
            0x89 => self.i_direct_direct_modify(bus, AluB::Adc),
            0x8A => self.i_abs_bit_modify(bus, 4),
            0x8B => self.i_direct_modify(bus, AluU::Dec),
            0x8C => self.i_absolute_modify(bus, AluU::Dec),
            0x8D => self.i_immediate_read(bus, AluB::Ld, Reg::Y),
            0x8E => self.i_pull_p(bus),
            0x8F => self.i_direct_immediate_write(bus),
            0x90 => self.i_branch(bus, !self.regs.psw.c()),
            0x91 => self.i_call_table(bus, 9),
            0x92 => self.i_abs_bit_set(bus, 4, false),
            0x93 => self.i_branch_bit(bus, 4, false),
            0x94 => self.i_direct_indexed_read(bus, AluB::Adc, Reg::A, Index::X),
            0x95 => self.i_absolute_indexed_read(bus, AluB::Adc, Index::X),
            0x96 => self.i_absolute_indexed_read(bus, AluB::Adc, Index::Y),
            0x97 => self.i_indirect_indexed_read(bus, AluB::Adc),
            0x98 => self.i_direct_immediate_modify(bus, AluB::Adc),
            0x99 => self.i_indirect_x_write_indirect_y(bus, AluB::Adc),
            0x9A => self.i_direct_read_word(bus, AluW::Sub),
            0x9B => self.i_direct_indexed_modify(bus, AluU::Dec),
            0x9C => self.i_implied_modify(bus, AluU::Dec, Reg::A),
            0x9D => self.i_transfer(bus, Reg::S, Reg::X),
            0x9E => self.i_divide(bus),
            0x9F => self.i_exchange_nibble(bus),
            0xA0 => self.i_flag_i(bus, true),
            0xA1 => self.i_call_table(bus, 10),
            0xA2 => self.i_abs_bit_set(bus, 5, true),
            0xA3 => self.i_branch_bit(bus, 5, true),
            0xA4 => self.i_direct_read(bus, AluB::Sbc, Reg::A),
            0xA5 => self.i_absolute_read(bus, AluB::Sbc, Reg::A),
            0xA6 => self.i_indirect_x_read(bus, AluB::Sbc),
            0xA7 => self.i_indexed_indirect_read(bus, AluB::Sbc),
            0xA8 => self.i_immediate_read(bus, AluB::Sbc, Reg::A),
            0xA9 => self.i_direct_direct_modify(bus, AluB::Sbc),
            0xAA => self.i_abs_bit_modify(bus, 5),
            0xAB => self.i_direct_modify(bus, AluU::Inc),
            0xAC => self.i_absolute_modify(bus, AluU::Inc),
            0xAD => self.i_immediate_read(bus, AluB::Cmp, Reg::Y),
            0xAE => self.i_pull(bus, Reg::A),
            0xAF => self.i_indirect_x_increment_write(bus),
            0xB0 => self.i_branch(bus, self.regs.psw.c()),
            0xB1 => self.i_call_table(bus, 11),
            0xB2 => self.i_abs_bit_set(bus, 5, false),
            0xB3 => self.i_branch_bit(bus, 5, false),
            0xB4 => self.i_direct_indexed_read(bus, AluB::Sbc, Reg::A, Index::X),
            0xB5 => self.i_absolute_indexed_read(bus, AluB::Sbc, Index::X),
            0xB6 => self.i_absolute_indexed_read(bus, AluB::Sbc, Index::Y),
            0xB7 => self.i_indirect_indexed_read(bus, AluB::Sbc),
            0xB8 => self.i_direct_immediate_modify(bus, AluB::Sbc),
            0xB9 => self.i_indirect_x_write_indirect_y(bus, AluB::Sbc),
            0xBA => self.i_direct_read_word(bus, AluW::Ld),
            0xBB => self.i_direct_indexed_modify(bus, AluU::Inc),
            0xBC => self.i_implied_modify(bus, AluU::Inc, Reg::A),
            0xBD => self.i_transfer(bus, Reg::X, Reg::S),
            0xBE => self.i_decimal_adjust_sub(bus),
            0xBF => self.i_indirect_x_increment_read(bus),
            0xC0 => self.i_flag_i(bus, false),
            0xC1 => self.i_call_table(bus, 12),
            0xC2 => self.i_abs_bit_set(bus, 6, true),
            0xC3 => self.i_branch_bit(bus, 6, true),
            0xC4 => self.i_direct_write(bus, Reg::A),
            0xC5 => self.i_absolute_write(bus, Reg::A),
            0xC6 => self.i_indirect_x_write(bus, Reg::A),
            0xC7 => self.i_indexed_indirect_write(bus, Reg::A),
            0xC8 => self.i_immediate_read(bus, AluB::Cmp, Reg::X),
            0xC9 => self.i_absolute_write(bus, Reg::X),
            0xCA => self.i_abs_bit_modify(bus, 6),
            0xCB => self.i_direct_write(bus, Reg::Y),
            0xCC => self.i_absolute_write(bus, Reg::Y),
            0xCD => self.i_immediate_read(bus, AluB::Ld, Reg::X),
            0xCE => self.i_pull(bus, Reg::X),
            0xCF => self.i_multiply(bus),
            0xD0 => self.i_branch(bus, !self.regs.psw.z()),
            0xD1 => self.i_call_table(bus, 13),
            0xD2 => self.i_abs_bit_set(bus, 6, false),
            0xD3 => self.i_branch_bit(bus, 6, false),
            0xD4 => self.i_direct_indexed_write(bus, Reg::A, Index::X),
            0xD5 => self.i_absolute_indexed_write(bus, Index::X),
            0xD6 => self.i_absolute_indexed_write(bus, Index::Y),
            0xD7 => self.i_indirect_indexed_write(bus, Reg::A),
            0xD8 => self.i_direct_write(bus, Reg::X),
            0xD9 => self.i_direct_indexed_write(bus, Reg::X, Index::Y),
            0xDA => self.i_direct_write_word(bus),
            0xDB => self.i_direct_indexed_write(bus, Reg::Y, Index::X),
            0xDC => self.i_implied_modify(bus, AluU::Dec, Reg::Y),
            0xDD => self.i_transfer(bus, Reg::Y, Reg::A),
            0xDE => self.i_branch_not_direct_indexed(bus),
            0xDF => self.i_decimal_adjust_add(bus),
            0xE0 => self.i_overflow_clear(bus),
            0xE1 => self.i_call_table(bus, 14),
            0xE2 => self.i_abs_bit_set(bus, 7, true),
            0xE3 => self.i_branch_bit(bus, 7, true),
            0xE4 => self.i_direct_read(bus, AluB::Ld, Reg::A),
            0xE5 => self.i_absolute_read(bus, AluB::Ld, Reg::A),
            0xE6 => self.i_indirect_x_read(bus, AluB::Ld),
            0xE7 => self.i_indexed_indirect_read(bus, AluB::Ld),
            0xE8 => self.i_immediate_read(bus, AluB::Ld, Reg::A),
            0xE9 => self.i_absolute_read(bus, AluB::Ld, Reg::X),
            0xEA => self.i_abs_bit_modify(bus, 7),
            0xEB => self.i_direct_read(bus, AluB::Ld, Reg::Y),
            0xEC => self.i_absolute_read(bus, AluB::Ld, Reg::Y),
            0xED => self.i_complement_carry(bus),
            0xEE => self.i_pull(bus, Reg::Y),
            0xEF => self.i_sleep(bus),
            0xF0 => self.i_branch(bus, self.regs.psw.z()),
            0xF1 => self.i_call_table(bus, 15),
            0xF2 => self.i_abs_bit_set(bus, 7, false),
            0xF3 => self.i_branch_bit(bus, 7, false),
            0xF4 => self.i_direct_indexed_read(bus, AluB::Ld, Reg::A, Index::X),
            0xF5 => self.i_absolute_indexed_read(bus, AluB::Ld, Index::X),
            0xF6 => self.i_absolute_indexed_read(bus, AluB::Ld, Index::Y),
            0xF7 => self.i_indirect_indexed_read(bus, AluB::Ld),
            0xF8 => self.i_direct_read(bus, AluB::Ld, Reg::X),
            0xF9 => self.i_direct_indexed_read(bus, AluB::Ld, Reg::X, Index::Y),
            0xFA => self.i_direct_direct_write(bus),
            0xFB => self.i_direct_indexed_read(bus, AluB::Ld, Reg::Y, Index::X),
            0xFC => self.i_implied_modify(bus, AluU::Inc, Reg::Y),
            0xFD => self.i_transfer(bus, Reg::A, Reg::Y),
            0xFE => self.i_branch_not_y_decrement(bus),
            0xFF => self.i_stop(bus),
        }
    }
}

/// A target/source 8-bit register selector for the addressing-mode handlers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reg {
    A,
    X,
    Y,
    S,
}

impl Spc700 {
    fn get(&self, r: Reg) -> u8 {
        match r {
            Reg::A => self.regs.a,
            Reg::X => self.regs.x,
            Reg::Y => self.regs.y,
            Reg::S => self.regs.sp,
        }
    }

    fn set(&mut self, r: Reg, v: u8) {
        match r {
            Reg::A => self.regs.a = v,
            Reg::X => self.regs.x = v,
            Reg::Y => self.regs.y = v,
            Reg::S => self.regs.sp = v,
        }
    }

    const fn index(&self, i: Index) -> u8 {
        match i {
            Index::X => self.regs.x,
            Index::Y => self.regs.y,
        }
    }

    fn fetch16(&mut self, bus: &mut impl Spc700Bus) -> u16 {
        let lo = self.fetch(bus);
        let hi = self.fetch(bus);
        u16::from(lo) | (u16::from(hi) << 8)
    }

    // ---- instruction handlers (one per ares instructionXxx) ----

    fn i_nop(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
    }

    fn i_immediate_read(&mut self, bus: &mut impl Spc700Bus, op: AluB, target: Reg) {
        let data = self.fetch(bus);
        let r = self.alu_b(op, self.get(target), data);
        if op != AluB::Cmp {
            self.set(target, r);
        }
    }

    fn i_direct_read(&mut self, bus: &mut impl Spc700Bus, op: AluB, target: Reg) {
        let addr = self.fetch(bus);
        let data = self.load(bus, addr);
        let r = self.alu_b(op, self.get(target), data);
        if op != AluB::Cmp {
            self.set(target, r);
        }
    }

    fn i_absolute_read(&mut self, bus: &mut impl Spc700Bus, op: AluB, target: Reg) {
        let addr = self.fetch16(bus);
        let data = bus.read(addr);
        let r = self.alu_b(op, self.get(target), data);
        if op != AluB::Cmp {
            self.set(target, r);
        }
    }

    fn i_indirect_x_read(&mut self, bus: &mut impl Spc700Bus, op: AluB) {
        bus.read(self.regs.pc);
        let data = self.load(bus, self.regs.x);
        let r = self.alu_b(op, self.regs.a, data);
        if op != AluB::Cmp {
            self.regs.a = r;
        }
    }

    fn i_indexed_indirect_read(&mut self, bus: &mut impl Spc700Bus, op: AluB) {
        let indirect = self.fetch(bus);
        bus.idle();
        let lo = self.load(bus, indirect.wrapping_add(self.regs.x));
        let hi = self.load(bus, indirect.wrapping_add(self.regs.x).wrapping_add(1));
        let addr = u16::from(lo) | (u16::from(hi) << 8);
        let data = bus.read(addr);
        let r = self.alu_b(op, self.regs.a, data);
        if op != AluB::Cmp {
            self.regs.a = r;
        }
    }

    fn i_indirect_indexed_read(&mut self, bus: &mut impl Spc700Bus, op: AluB) {
        let indirect = self.fetch(bus);
        bus.idle();
        let lo = self.load(bus, indirect);
        let hi = self.load(bus, indirect.wrapping_add(1));
        let addr = u16::from(lo) | (u16::from(hi) << 8);
        let data = bus.read(addr.wrapping_add(u16::from(self.regs.y)));
        let r = self.alu_b(op, self.regs.a, data);
        if op != AluB::Cmp {
            self.regs.a = r;
        }
    }

    fn i_direct_indexed_read(
        &mut self,
        bus: &mut impl Spc700Bus,
        op: AluB,
        target: Reg,
        idx: Index,
    ) {
        let addr = self.fetch(bus);
        bus.idle();
        let data = self.load(bus, addr.wrapping_add(self.index(idx)));
        let r = self.alu_b(op, self.get(target), data);
        if op != AluB::Cmp {
            self.set(target, r);
        }
    }

    fn i_absolute_indexed_read(&mut self, bus: &mut impl Spc700Bus, op: AluB, idx: Index) {
        let addr = self.fetch16(bus);
        bus.idle();
        let data = bus.read(addr.wrapping_add(u16::from(self.index(idx))));
        let r = self.alu_b(op, self.regs.a, data);
        if op != AluB::Cmp {
            self.regs.a = r;
        }
    }

    fn i_implied_modify(&mut self, bus: &mut impl Spc700Bus, op: AluU, target: Reg) {
        bus.read(self.regs.pc);
        let v = self.alu_u(op, self.get(target));
        self.set(target, v);
    }

    fn i_direct_modify(&mut self, bus: &mut impl Spc700Bus, op: AluU) {
        let addr = self.fetch(bus);
        let data = self.load(bus, addr);
        let v = self.alu_u(op, data);
        self.store(bus, addr, v);
    }

    fn i_direct_indexed_modify(&mut self, bus: &mut impl Spc700Bus, op: AluU) {
        let addr = self.fetch(bus);
        bus.idle();
        let ea = addr.wrapping_add(self.regs.x);
        let data = self.load(bus, ea);
        let v = self.alu_u(op, data);
        self.store(bus, ea, v);
    }

    fn i_absolute_modify(&mut self, bus: &mut impl Spc700Bus, op: AluU) {
        let addr = self.fetch16(bus);
        let data = bus.read(addr);
        let v = self.alu_u(op, data);
        bus.write(addr, v);
    }

    fn i_direct_write(&mut self, bus: &mut impl Spc700Bus, src: Reg) {
        let addr = self.fetch(bus);
        self.load(bus, addr);
        self.store(bus, addr, self.get(src));
    }

    fn i_absolute_write(&mut self, bus: &mut impl Spc700Bus, src: Reg) {
        let addr = self.fetch16(bus);
        bus.read(addr);
        bus.write(addr, self.get(src));
    }

    fn i_direct_indexed_write(&mut self, bus: &mut impl Spc700Bus, src: Reg, idx: Index) {
        let addr = self.fetch(bus);
        bus.idle();
        let ea = addr.wrapping_add(self.index(idx));
        self.load(bus, ea);
        self.store(bus, ea, self.get(src));
    }

    fn i_absolute_indexed_write(&mut self, bus: &mut impl Spc700Bus, idx: Index) {
        let addr = self.fetch16(bus);
        bus.idle();
        let ea = addr.wrapping_add(u16::from(self.index(idx)));
        bus.read(ea);
        bus.write(ea, self.regs.a);
    }

    fn i_indirect_x_write(&mut self, bus: &mut impl Spc700Bus, src: Reg) {
        bus.read(self.regs.pc);
        self.load(bus, self.regs.x);
        self.store(bus, self.regs.x, self.get(src));
    }

    fn i_indexed_indirect_write(&mut self, bus: &mut impl Spc700Bus, src: Reg) {
        let indirect = self.fetch(bus);
        bus.idle();
        let lo = self.load(bus, indirect.wrapping_add(self.regs.x));
        let hi = self.load(bus, indirect.wrapping_add(self.regs.x).wrapping_add(1));
        let addr = u16::from(lo) | (u16::from(hi) << 8);
        bus.read(addr);
        bus.write(addr, self.get(src));
    }

    fn i_indirect_indexed_write(&mut self, bus: &mut impl Spc700Bus, src: Reg) {
        let indirect = self.fetch(bus);
        let lo = self.load(bus, indirect);
        let hi = self.load(bus, indirect.wrapping_add(1));
        let addr = u16::from(lo) | (u16::from(hi) << 8);
        bus.idle();
        let ea = addr.wrapping_add(u16::from(self.regs.y));
        bus.read(ea);
        bus.write(ea, self.get(src));
    }

    fn i_indirect_x_increment_read(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        let data = self.load(bus, self.regs.x);
        self.regs.x = self.regs.x.wrapping_add(1);
        bus.idle();
        self.regs.a = data;
        self.regs.psw.set_nz(data);
    }

    fn i_indirect_x_increment_write(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        self.store(bus, self.regs.x, self.regs.a);
        self.regs.x = self.regs.x.wrapping_add(1);
    }

    fn i_indirect_x_compare_indirect_y(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        let rhs = self.load(bus, self.regs.y);
        let lhs = self.load(bus, self.regs.x);
        self.alu_b(AluB::Cmp, lhs, rhs);
        bus.idle();
    }

    fn i_indirect_x_write_indirect_y(&mut self, bus: &mut impl Spc700Bus, op: AluB) {
        bus.read(self.regs.pc);
        let rhs = self.load(bus, self.regs.y);
        let lhs = self.load(bus, self.regs.x);
        let r = self.alu_b(op, lhs, rhs);
        self.store(bus, self.regs.x, r);
    }

    fn i_direct_direct_modify(&mut self, bus: &mut impl Spc700Bus, op: AluB) {
        let source = self.fetch(bus);
        let rhs = self.load(bus, source);
        let target = self.fetch(bus);
        let lhs = self.load(bus, target);
        let r = self.alu_b(op, lhs, rhs);
        self.store(bus, target, r);
    }

    fn i_direct_direct_compare(&mut self, bus: &mut impl Spc700Bus) {
        let source = self.fetch(bus);
        let rhs = self.load(bus, source);
        let target = self.fetch(bus);
        let lhs = self.load(bus, target);
        self.alu_b(AluB::Cmp, lhs, rhs);
        bus.idle();
    }

    fn i_direct_direct_write(&mut self, bus: &mut impl Spc700Bus) {
        let source = self.fetch(bus);
        let data = self.load(bus, source);
        let target = self.fetch(bus);
        self.store(bus, target, data);
    }

    fn i_direct_immediate_modify(&mut self, bus: &mut impl Spc700Bus, op: AluB) {
        let immediate = self.fetch(bus);
        let addr = self.fetch(bus);
        let data = self.load(bus, addr);
        let r = self.alu_b(op, data, immediate);
        self.store(bus, addr, r);
    }

    fn i_direct_immediate_compare(&mut self, bus: &mut impl Spc700Bus) {
        let immediate = self.fetch(bus);
        let addr = self.fetch(bus);
        let data = self.load(bus, addr);
        self.alu_b(AluB::Cmp, data, immediate);
        bus.idle();
    }

    fn i_direct_immediate_write(&mut self, bus: &mut impl Spc700Bus) {
        let immediate = self.fetch(bus);
        let addr = self.fetch(bus);
        self.load(bus, addr);
        self.store(bus, addr, immediate);
    }

    fn i_direct_read_word(&mut self, bus: &mut impl Spc700Bus, op: AluW) {
        let addr = self.fetch(bus);
        let lo = self.load(bus, addr);
        bus.idle();
        let hi = self.load(bus, addr.wrapping_add(1));
        let data = u16::from(lo) | (u16::from(hi) << 8);
        let r = self.alu_w(op, self.regs.ya(), data);
        self.regs.set_ya(r);
    }

    fn i_direct_compare_word(&mut self, bus: &mut impl Spc700Bus, op: AluW) {
        let addr = self.fetch(bus);
        let lo = self.load(bus, addr);
        let hi = self.load(bus, addr.wrapping_add(1));
        let data = u16::from(lo) | (u16::from(hi) << 8);
        self.alu_w(op, self.regs.ya(), data);
    }

    fn i_direct_modify_word(&mut self, bus: &mut impl Spc700Bus, adjust: i32) {
        let addr = self.fetch(bus);
        let lo = self.load(bus, addr);
        let mut data = (i32::from(lo) + adjust) as u16;
        self.store(bus, addr, data as u8);
        let hi = self.load(bus, addr.wrapping_add(1));
        data = data.wrapping_add(u16::from(hi) << 8);
        self.store(bus, addr.wrapping_add(1), (data >> 8) as u8);
        self.regs.psw.set_z(data == 0);
        self.regs.psw.set_n(data & 0x8000 != 0);
    }

    fn i_direct_write_word(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch(bus);
        self.load(bus, addr);
        self.store(bus, addr, self.regs.a);
        self.store(bus, addr.wrapping_add(1), self.regs.y);
    }

    fn i_transfer(&mut self, bus: &mut impl Spc700Bus, from: Reg, to: Reg) {
        bus.read(self.regs.pc);
        let v = self.get(from);
        self.set(to, v);
        if to != Reg::S {
            self.regs.psw.set_nz(v);
        }
    }

    fn i_push(&mut self, bus: &mut impl Spc700Bus, data: u8) {
        bus.read(self.regs.pc);
        self.push(bus, data);
        bus.idle();
    }

    fn i_pull(&mut self, bus: &mut impl Spc700Bus, target: Reg) {
        bus.read(self.regs.pc);
        bus.idle();
        let v = self.pull(bus);
        self.set(target, v);
    }

    fn i_pull_p(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        let v = self.pull(bus);
        self.regs.psw = crate::psw::Psw::from_bits(v);
    }

    fn i_branch(&mut self, bus: &mut impl Spc700Bus, take: bool) {
        let data = self.fetch(bus);
        if !take {
            return;
        }
        bus.idle();
        bus.idle();
        self.regs.pc = self.regs.pc.wrapping_add_signed(i16::from(data as i8));
    }

    fn i_branch_bit(&mut self, bus: &mut impl Spc700Bus, bit: u8, set: bool) {
        let addr = self.fetch(bus);
        let data = self.load(bus, addr);
        bus.idle();
        let disp = self.fetch(bus);
        if (data & (1 << bit) != 0) != set {
            return;
        }
        bus.idle();
        bus.idle();
        self.regs.pc = self.regs.pc.wrapping_add_signed(i16::from(disp as i8));
    }

    fn i_branch_not_direct(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch(bus);
        let data = self.load(bus, addr);
        bus.idle();
        let disp = self.fetch(bus);
        if self.regs.a == data {
            return;
        }
        bus.idle();
        bus.idle();
        self.regs.pc = self.regs.pc.wrapping_add_signed(i16::from(disp as i8));
    }

    fn i_branch_not_direct_decrement(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch(bus);
        let data = self.load(bus, addr).wrapping_sub(1);
        self.store(bus, addr, data);
        let disp = self.fetch(bus);
        if data == 0 {
            return;
        }
        bus.idle();
        bus.idle();
        self.regs.pc = self.regs.pc.wrapping_add_signed(i16::from(disp as i8));
    }

    fn i_branch_not_direct_indexed(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch(bus);
        bus.idle();
        let data = self.load(bus, addr.wrapping_add(self.regs.x));
        bus.idle();
        let disp = self.fetch(bus);
        if self.regs.a == data {
            return;
        }
        bus.idle();
        bus.idle();
        self.regs.pc = self.regs.pc.wrapping_add_signed(i16::from(disp as i8));
    }

    fn i_branch_not_y_decrement(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        let disp = self.fetch(bus);
        self.regs.y = self.regs.y.wrapping_sub(1);
        if self.regs.y == 0 {
            return;
        }
        bus.idle();
        bus.idle();
        self.regs.pc = self.regs.pc.wrapping_add_signed(i16::from(disp as i8));
    }

    fn i_jump_absolute(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch16(bus);
        self.regs.pc = addr;
    }

    fn i_jump_indirect_x(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch16(bus);
        bus.idle();
        let base = addr.wrapping_add(u16::from(self.regs.x));
        let lo = bus.read(base);
        let hi = bus.read(base.wrapping_add(1));
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
    }

    fn i_call_absolute(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch16(bus);
        bus.idle();
        self.push(bus, (self.regs.pc >> 8) as u8);
        self.push(bus, self.regs.pc as u8);
        bus.idle();
        bus.idle();
        self.regs.pc = addr;
    }

    fn i_call_page(&mut self, bus: &mut impl Spc700Bus) {
        let addr = self.fetch(bus);
        bus.idle();
        self.push(bus, (self.regs.pc >> 8) as u8);
        self.push(bus, self.regs.pc as u8);
        bus.idle();
        self.regs.pc = 0xFF00 | u16::from(addr);
    }

    fn i_call_table(&mut self, bus: &mut impl Spc700Bus, vector: u16) {
        bus.read(self.regs.pc);
        bus.idle();
        self.push(bus, (self.regs.pc >> 8) as u8);
        self.push(bus, self.regs.pc as u8);
        bus.idle();
        let address = 0xFFDE - (vector << 1);
        let lo = bus.read(address);
        let hi = bus.read(address.wrapping_add(1));
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
    }

    fn i_return_subroutine(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        let lo = self.pull(bus);
        let hi = self.pull(bus);
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
    }

    fn i_return_interrupt(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        let p = self.pull(bus);
        self.regs.psw = crate::psw::Psw::from_bits(p);
        let lo = self.pull(bus);
        let hi = self.pull(bus);
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
    }

    fn i_break(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        self.push(bus, (self.regs.pc >> 8) as u8);
        self.push(bus, self.regs.pc as u8);
        self.push(bus, self.regs.psw.bits());
        bus.idle();
        let lo = bus.read(0xFFDE);
        let hi = bus.read(0xFFDF);
        self.regs.pc = u16::from(lo) | (u16::from(hi) << 8);
        self.regs.psw.set_i(false);
        self.regs.psw.set_b(true);
    }

    fn i_test_set_bits(&mut self, bus: &mut impl Spc700Bus, set: bool) {
        let addr = self.fetch16(bus);
        let data = bus.read(addr);
        let diff = self.regs.a.wrapping_sub(data);
        self.regs.psw.set_z(diff == 0);
        self.regs.psw.set_n(diff & 0x80 != 0);
        bus.read(addr);
        let v = if set {
            data | self.regs.a
        } else {
            data & !self.regs.a
        };
        bus.write(addr, v);
    }

    fn i_abs_bit_set(&mut self, bus: &mut impl Spc700Bus, bit: u8, value: bool) {
        let addr = self.fetch(bus);
        let mut data = self.load(bus, addr);
        if value {
            data |= 1 << bit;
        } else {
            data &= !(1 << bit);
        }
        self.store(bus, addr, data);
    }

    fn i_abs_bit_modify(&mut self, bus: &mut impl Spc700Bus, mode: u8) {
        let lo = self.fetch(bus);
        let hi = self.fetch(bus);
        let packed = u16::from(lo) | (u16::from(hi) << 8);
        let bit = (packed >> 13) as u8;
        let addr = packed & 0x1FFF;
        let mut data = bus.read(addr);
        let bitval = data & (1 << bit) != 0;
        let mut c = self.regs.psw.c();
        match mode {
            0 => {
                bus.idle();
                c |= bitval;
            }
            1 => {
                bus.idle();
                c |= !bitval;
            }
            2 => c &= bitval,
            3 => c &= !bitval,
            4 => {
                bus.idle();
                c ^= bitval;
            }
            5 => c = bitval,
            6 => {
                bus.idle();
                if c {
                    data |= 1 << bit;
                } else {
                    data &= !(1 << bit);
                }
                bus.write(addr, data);
            }
            _ => {
                data ^= 1 << bit;
                bus.write(addr, data);
            }
        }
        self.regs.psw.set_c(c);
    }

    fn i_multiply(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        for _ in 0..7 {
            bus.idle();
        }
        let ya = u16::from(self.regs.y) * u16::from(self.regs.a);
        self.regs.a = ya as u8;
        self.regs.y = (ya >> 8) as u8;
        self.regs.psw.set_z(self.regs.y == 0);
        self.regs.psw.set_n(self.regs.y & 0x80 != 0);
    }

    fn i_divide(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        for _ in 0..10 {
            bus.idle();
        }
        let ya = u32::from(self.regs.ya());
        let x = u32::from(self.regs.x);
        self.regs
            .psw
            .set_h((self.regs.y & 15) >= (self.regs.x & 15));
        self.regs.psw.set_v(u32::from(self.regs.y) >= x);
        if x == 0 {
            // Hardware divide-by-zero yields garbage but never traps; match ares overflow path
            // which still divides by (256 - X) etc. With X=0 ares relies on the second branch.
            let quotient = 255 - (ya.wrapping_sub(0) / 256);
            let remainder = (ya.wrapping_sub(0)) % 256;
            self.regs.a = quotient as u8;
            self.regs.y = remainder as u8;
        } else if u32::from(self.regs.y) < (x << 1) {
            self.regs.a = (ya / x) as u8;
            self.regs.y = (ya % x) as u8;
        } else {
            self.regs.a = (255 - (ya - (x << 9)) / (256 - x)) as u8;
            self.regs.y = (x + (ya - (x << 9)) % (256 - x)) as u8;
        }
        self.regs.psw.set_z(self.regs.a == 0);
        self.regs.psw.set_n(self.regs.a & 0x80 != 0);
    }

    fn i_exchange_nibble(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        bus.idle();
        bus.idle();
        self.regs.a = self.regs.a.rotate_left(4); // XCN: swap the two nibbles of A
        self.regs.psw.set_nz(self.regs.a);
    }

    fn i_decimal_adjust_add(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        if self.regs.psw.c() || self.regs.a > 0x99 {
            self.regs.a = self.regs.a.wrapping_add(0x60);
            self.regs.psw.set_c(true);
        }
        if self.regs.psw.h() || (self.regs.a & 15) > 0x09 {
            self.regs.a = self.regs.a.wrapping_add(0x06);
        }
        self.regs.psw.set_nz(self.regs.a);
    }

    fn i_decimal_adjust_sub(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        if !self.regs.psw.c() || self.regs.a > 0x99 {
            self.regs.a = self.regs.a.wrapping_sub(0x60);
            self.regs.psw.set_c(false);
        }
        if !self.regs.psw.h() || (self.regs.a & 15) > 0x09 {
            self.regs.a = self.regs.a.wrapping_sub(0x06);
        }
        self.regs.psw.set_nz(self.regs.a);
    }

    fn i_flag_c(&mut self, bus: &mut impl Spc700Bus, value: bool) {
        bus.read(self.regs.pc);
        self.regs.psw.set_c(value);
    }

    fn i_flag_p(&mut self, bus: &mut impl Spc700Bus, value: bool) {
        bus.read(self.regs.pc);
        self.regs.psw.set_p(value);
    }

    fn i_flag_i(&mut self, bus: &mut impl Spc700Bus, value: bool) {
        bus.read(self.regs.pc);
        bus.idle(); // SETI/CLRI carry an extra idle cycle (ares instructionFlagSet IF branch)
        self.regs.psw.set_i(value);
    }

    fn i_complement_carry(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        bus.idle();
        self.regs.psw.set_c(!self.regs.psw.c());
    }

    fn i_overflow_clear(&mut self, bus: &mut impl Spc700Bus) {
        bus.read(self.regs.pc);
        self.regs.psw.set_h(false);
        self.regs.psw.set_v(false);
    }

    fn i_stop(&mut self, bus: &mut impl Spc700Bus) {
        self.stopped = true;
        // ares loops `read(PC)`+`idle` until sync; the oracle captures a fixed 3-iteration window
        // (opcode fetch already happened → total 1 + 3×2 = 7 cycles, PC at opcode+1).
        self.halt_window(bus);
    }

    fn i_sleep(&mut self, bus: &mut impl Spc700Bus) {
        self.waiting = true;
        self.halt_window(bus);
    }

    fn halt_window(&mut self, bus: &mut impl Spc700Bus) {
        for _ in 0..3 {
            bus.read(self.regs.pc);
            bus.idle();
        }
    }
}
