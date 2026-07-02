// Instruction dispatch + ALU algorithms for `Hg51b`, split out of `hg51b.rs` for readability
// (`include!`d there, so this is still `impl Hg51b` in the same module — see that file's doc for
// the register/opcode source references). Mirrors ares' `instruction.cpp` decode table +
// `instructions.cpp` semantics; the shift-select table `[0, 1, 8, 16]` is a hardware fact (the
// `ss` opcode field selects the accumulator's pre-shift amount for ALU ops).

const SHIFTS: [u32; 4] = [0, 1, 8, 16];

/// Which flag (if any) gates a conditional branch/skip, evaluated at execution time (the flag's
/// live value when the instruction runs, not when it was decoded) — matches ares passing
/// `const n1&` flag references into `instructionJMP`/`JSR`/`SKIP`.
#[derive(Clone, Copy)]
enum Cond {
    Always,
    Eq,
    Ge,
    Mi,
    Vs,
}

impl Hg51b {
    fn cond(&self, c: Cond) -> bool {
        match c {
            Cond::Always => true,
            Cond::Eq => self.r.z,
            Cond::Ge => self.r.c,
            Cond::Mi => self.r.n,
            Cond::Vs => self.r.v,
        }
    }

    /// Decode and execute one 16-bit opcode (ares' generated `instructionTable[opcode]`).
    ///
    /// Each arm is `(mask, value)` transcribed directly from the corresponding `pattern("...")`
    /// string in ares' `instruction.cpp` (a `.` character = don't-care = mask bit 0). These
    /// patterns partition the full 65536-opcode space with no overlaps by construction (ares
    /// enumerates every don't-care combination exactly once and asserts no double-bind), so a
    /// sequential first-match chain is unambiguous regardless of arm order.
    #[allow(clippy::too_many_lines)]
    fn dispatch(&mut self, opcode: u16, bus: &mut impl Hg51bBus) {
        macro_rules! is {
            ($mask:expr, $val:expr) => {
                opcode & $mask == $val
            };
        }

        if is!(0xFC00, 0x0800) {
            self.op_jmp(opcode, Cond::Always, bus);
        } else if is!(0xFC00, 0x0C00) {
            self.op_jmp(opcode, Cond::Eq, bus);
        } else if is!(0xFC00, 0x1000) {
            self.op_jmp(opcode, Cond::Ge, bus);
        } else if is!(0xFC00, 0x1400) {
            self.op_jmp(opcode, Cond::Mi, bus);
        } else if is!(0xFC00, 0x1800) {
            self.op_jmp(opcode, Cond::Vs, bus);
        } else if is!(0xFC00, 0x1C00) {
            // WAIT: handled by the host bus-port step machinery; nothing else to do inline.
        } else if is!(0xFF00, 0x2400) {
            self.op_skip(opcode, self.r.v);
        } else if is!(0xFF00, 0x2500) {
            self.op_skip(opcode, self.r.c);
        } else if is!(0xFF00, 0x2600) {
            self.op_skip(opcode, self.r.z);
        } else if is!(0xFF00, 0x2700) {
            self.op_skip(opcode, self.r.n);
        } else if is!(0xFC00, 0x2800) {
            self.op_jsr(opcode, Cond::Always, bus);
        } else if is!(0xFC00, 0x2C00) {
            self.op_jsr(opcode, Cond::Eq, bus);
        } else if is!(0xFC00, 0x3000) {
            self.op_jsr(opcode, Cond::Ge, bus);
        } else if is!(0xFC00, 0x3400) {
            self.op_jsr(opcode, Cond::Mi, bus);
        } else if is!(0xFC00, 0x3800) {
            self.op_jsr(opcode, Cond::Vs, bus);
        } else if is!(0xFC00, 0x3C00) {
            self.op_rts();
        } else if is!(0xFC00, 0x4000) {
            self.r.mar = self.r.mar.wrapping_add(1) & 0xFF_FFFF;
        } else if is!(0xFC00, 0x4800) {
            self.op_cmpr_reg(opcode);
        } else if is!(0xFC00, 0x4C00) {
            self.op_cmpr_imm(opcode);
        } else if is!(0xFC00, 0x5000) {
            self.op_cmp_reg(opcode);
        } else if is!(0xFC00, 0x5400) {
            self.op_cmp_imm(opcode);
        } else if is!(0xFF00, 0x5900) {
            self.r.a = self.alg_sx(i32::from(self.r.a as i8) as u32);
        } else if is!(0xFF00, 0x5A00) {
            self.r.a = self.alg_sx(i32::from(self.r.a as i16) as u32);
        } else if is!(0xFF00, 0x6000) {
            self.r.a = self.read_register(opcode & 0x7F, bus);
        } else if is!(0xFF00, 0x6100) {
            self.r.mdr = self.read_register(opcode & 0x7F, bus);
        } else if is!(0xFF00, 0x6200) {
            self.r.mar = self.read_register(opcode & 0x7F, bus);
        } else if is!(0xFF00, 0x6300) {
            self.r.p = self.r.gpr[usize::from(opcode & 0xF)] as u16 & 0x7FFF;
        } else if is!(0xFF00, 0x6400) {
            self.r.a = u32::from(opcode & 0xFF);
        } else if is!(0xFF00, 0x6500) {
            self.r.mdr = u32::from(opcode & 0xFF);
        } else if is!(0xFF00, 0x6600) {
            self.r.mar = u32::from(opcode & 0xFF);
        } else if is!(0xFF00, 0x6700) {
            self.r.p = opcode & 0xFF;
        } else if is!(0xFF00, 0x6800) {
            self.op_rdram_reg(0);
        } else if is!(0xFF00, 0x6900) {
            self.op_rdram_reg(1);
        } else if is!(0xFF00, 0x6A00) {
            self.op_rdram_reg(2);
        } else if is!(0xFF00, 0x6C00) {
            self.op_rdram_imm(0, opcode);
        } else if is!(0xFF00, 0x6D00) {
            self.op_rdram_imm(1, opcode);
        } else if is!(0xFF00, 0x6E00) {
            self.op_rdram_imm(2, opcode);
        } else if is!(0xFC00, 0x7000) {
            self.r.rom = self.data_rom[(self.r.a & 0x3FF) as usize];
        } else if is!(0xFC00, 0x7400) {
            self.r.rom = self.data_rom[usize::from(opcode & 0x3FF)];
        } else if is!(0xFF00, 0x7C00) {
            self.r.p = (self.r.p & 0xFF00) | (opcode & 0xFF);
        } else if is!(0xFF00, 0x7D00) {
            self.r.p = (self.r.p & 0x00FF) | ((opcode & 0x7F) << 8);
        } else if is!(0xFC00, 0x8000) {
            self.op_add_reg(opcode);
        } else if is!(0xFC00, 0x8400) {
            self.op_add_imm(opcode);
        } else if is!(0xFC00, 0x8800) {
            self.op_subr_reg(opcode);
        } else if is!(0xFC00, 0x8C00) {
            self.op_subr_imm(opcode);
        } else if is!(0xFC00, 0x9000) {
            self.op_sub_reg(opcode);
        } else if is!(0xFC00, 0x9400) {
            self.op_sub_imm(opcode);
        } else if is!(0xFC00, 0x9800) {
            self.op_mul_reg(opcode);
        } else if is!(0xFC00, 0x9C00) {
            self.op_mul_imm(opcode);
        } else if is!(0xFC00, 0xA000) {
            self.op_xnor_reg(opcode);
        } else if is!(0xFC00, 0xA400) {
            self.op_xnor_imm(opcode);
        } else if is!(0xFC00, 0xA800) {
            self.op_xor_reg(opcode);
        } else if is!(0xFC00, 0xAC00) {
            self.op_xor_imm(opcode);
        } else if is!(0xFC00, 0xB000) {
            self.op_and_reg(opcode);
        } else if is!(0xFC00, 0xB400) {
            self.op_and_imm(opcode);
        } else if is!(0xFC00, 0xB800) {
            self.op_or_reg(opcode);
        } else if is!(0xFC00, 0xBC00) {
            self.op_or_imm(opcode);
        } else if is!(0xFC00, 0xC000) {
            self.op_shr_reg(opcode);
        } else if is!(0xFC00, 0xC400) {
            self.op_shr_imm(opcode);
        } else if is!(0xFC00, 0xC800) {
            self.op_asr_reg(opcode);
        } else if is!(0xFC00, 0xCC00) {
            self.op_asr_imm(opcode);
        } else if is!(0xFC00, 0xD000) {
            self.op_ror_reg(opcode);
        } else if is!(0xFC00, 0xD400) {
            self.op_ror_imm(opcode);
        } else if is!(0xFC00, 0xD800) {
            self.op_shl_reg(opcode);
        } else if is!(0xFC00, 0xDC00) {
            self.op_shl_imm(opcode);
        } else if is!(0xFF00, 0xE000) {
            let v = self.r.a;
            self.write_register(opcode & 0x7F, v, bus);
        } else if is!(0xFF00, 0xE100) {
            let v = self.r.mdr;
            self.write_register(opcode & 0x7F, v, bus);
        } else if is!(0xFF00, 0xE800) {
            self.op_wrram_reg(0);
        } else if is!(0xFF00, 0xE900) {
            self.op_wrram_reg(1);
        } else if is!(0xFF00, 0xEA00) {
            self.op_wrram_reg(2);
        } else if is!(0xFF00, 0xEC00) {
            self.op_wrram_imm(0, opcode);
        } else if is!(0xFF00, 0xED00) {
            self.op_wrram_imm(1, opcode);
        } else if is!(0xFF00, 0xEE00) {
            self.op_wrram_imm(2, opcode);
        } else if is!(0xFC00, 0xF000) {
            let reg = usize::from(opcode & 0xF);
            core::mem::swap(&mut self.r.a, &mut self.r.gpr[reg]);
        } else if is!(0xFC00, 0xF800) {
            self.r.a = 0;
            self.r.p = 0;
            self.r.ram = 0;
            self.r.dpr = 0;
        } else if is!(0xFC00, 0xFC00) {
            self.io.halt = true;
        }
        // Every other (mask,value) combination is an unconnected opcode slot bound to NOP.
    }

    // --- Branch / call / skip. -------------------------------------------------------------

    fn op_jmp(&mut self, opcode: u16, c: Cond, bus: &mut impl Hg51bBus) {
        if !self.cond(c) {
            return;
        }
        let far = opcode & 0x0200 != 0;
        let data = (opcode & 0xFF) as u8;
        if far {
            self.r.pb = self.r.p;
        }
        self.r.pc = data;
        self.step(2);
        self.finish_bus_access(bus);
    }

    fn op_jsr(&mut self, opcode: u16, c: Cond, bus: &mut impl Hg51bBus) {
        if !self.cond(c) {
            return;
        }
        self.push();
        let far = opcode & 0x0200 != 0;
        let data = (opcode & 0xFF) as u8;
        if far {
            self.r.pb = self.r.p;
        }
        self.r.pc = data;
        self.step(2);
        self.finish_bus_access(bus);
    }

    fn op_rts(&mut self) {
        self.pull();
        self.step(2);
    }

    fn op_skip(&mut self, opcode: u16, flag: bool) {
        let take = opcode & 1 != 0;
        if flag != take {
            return;
        }
        // `advance()` needs a bus for a possible cache refill; SKIP's own opcode fetch already
        // guaranteed the current page is resident, and skipping just moves `pc` (and possibly
        // flips page on wrap, per `advance`), so a fetch-less bus is safe here — mirrors ares
        // calling `advance()` (no bus in its virtual-dispatch signature either).
        self.advance_no_bus();
        self.step(1);
    }

    /// [`Self::advance`] without triggering a cache-page load — used only by SKIP, which (per
    /// ares) calls the bare `advance()` with no read side effect expected on the common
    /// (non-page-wrap) path. On the rare page-wrap edge, we fall back to halting like a failed
    /// cache load would, rather than silently reading stale cache content.
    fn advance_no_bus(&mut self) {
        let (pc, overflow) = self.r.pc.overflowing_add(1);
        self.r.pc = pc;
        if overflow {
            self.io.halt = true;
        }
    }

    // --- RDRAM / WRRAM (data-RAM access; the `>=$C00 -> -$400` fold matches real hardware). --

    fn op_rdram_reg(&mut self, byte_idx: u32) {
        let mut address = self.r.a & 0xFFF;
        if address >= 0xC00 {
            address -= 0x400;
        }
        let v = self.data_ram[address as usize];
        set_byte(&mut self.r.ram, byte_idx as u8, v);
    }

    fn op_rdram_imm(&mut self, byte_idx: u32, opcode: u16) {
        let imm = u32::from(opcode & 0xFF);
        let mut address = (self.r.dpr.wrapping_add(imm)) & 0xFFF;
        if address >= 0xC00 {
            address -= 0x400;
        }
        let v = self.data_ram[address as usize];
        set_byte(&mut self.r.ram, byte_idx as u8, v);
    }

    fn op_wrram_reg(&mut self, byte_idx: u32) {
        let mut address = self.r.a & 0xFFF;
        if address >= 0xC00 {
            address -= 0x400;
        }
        self.data_ram[address as usize] = byte(self.r.ram, byte_idx as u8);
    }

    fn op_wrram_imm(&mut self, byte_idx: u32, opcode: u16) {
        let imm = u32::from(opcode & 0xFF);
        let mut address = (self.r.dpr.wrapping_add(imm)) & 0xFFF;
        if address >= 0xC00 {
            address -= 0x400;
        }
        self.data_ram[address as usize] = byte(self.r.ram, byte_idx as u8);
    }

    // --- ALU ops (shift-select variants share this shape). ----------------------------------

    fn op_add_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.r.a = self.alg_add(self.r.a << shift, y);
    }
    fn op_add_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.r.a = self.alg_add(self.r.a << shift, imm);
    }
    fn op_subr_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.r.a = self.alg_sub(y, self.r.a << shift);
    }
    fn op_subr_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.r.a = self.alg_sub(imm, self.r.a << shift);
    }
    fn op_sub_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.r.a = self.alg_sub(self.r.a << shift, y);
    }
    fn op_sub_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.r.a = self.alg_sub(self.r.a << shift, imm);
    }
    fn op_cmp_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.alg_sub(self.r.a << shift, y);
    }
    fn op_cmp_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.alg_sub(self.r.a << shift, imm);
    }
    fn op_cmpr_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.alg_sub(y, self.r.a << shift);
    }
    fn op_cmpr_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.alg_sub(imm, self.r.a << shift);
    }
    fn op_and_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.r.a = self.alg_and(self.r.a << shift, y);
    }
    fn op_and_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.r.a = self.alg_and(self.r.a << shift, imm);
    }
    fn op_or_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.r.a = self.alg_or(self.r.a << shift, y);
    }
    fn op_or_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.r.a = self.alg_or(self.r.a << shift, imm);
    }
    fn op_xor_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.r.a = self.alg_xor(self.r.a << shift, y);
    }
    fn op_xor_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.r.a = self.alg_xor(self.r.a << shift, imm);
    }
    fn op_xnor_reg(&mut self, opcode: u16) {
        let (reg, shift) = reg_shift(opcode);
        let y = self.reg_value_no_bus(reg);
        self.r.a = self.alg_xnor(self.r.a << shift, y);
    }
    fn op_xnor_imm(&mut self, opcode: u16) {
        let (imm, shift) = imm_shift(opcode);
        self.r.a = self.alg_xnor(self.r.a << shift, imm);
    }

    fn op_mul_reg(&mut self, opcode: u16) {
        let reg = opcode & 0x7F;
        let y = self.reg_value_no_bus(reg);
        self.r.mul = alg_mul(self.r.a, y);
    }
    fn op_mul_imm(&mut self, opcode: u16) {
        let imm = u32::from(opcode & 0xFF);
        self.r.mul = alg_mul(self.r.a, imm);
    }

    fn op_shr_reg(&mut self, opcode: u16) {
        let reg = opcode & 0x7F;
        let s = self.reg_value_no_bus(reg) & 0x1F;
        self.r.a = self.alg_shr(self.r.a, s);
    }
    fn op_shr_imm(&mut self, opcode: u16) {
        self.r.a = self.alg_shr(self.r.a, u32::from(opcode & 0x1F));
    }
    fn op_asr_reg(&mut self, opcode: u16) {
        let reg = opcode & 0x7F;
        let s = self.reg_value_no_bus(reg) & 0x1F;
        self.r.a = self.alg_asr(self.r.a, s);
    }
    fn op_asr_imm(&mut self, opcode: u16) {
        self.r.a = self.alg_asr(self.r.a, u32::from(opcode & 0x1F));
    }
    fn op_ror_reg(&mut self, opcode: u16) {
        let reg = opcode & 0x7F;
        let s = self.reg_value_no_bus(reg) & 0x1F;
        self.r.a = self.alg_ror(self.r.a, s);
    }
    fn op_ror_imm(&mut self, opcode: u16) {
        self.r.a = self.alg_ror(self.r.a, u32::from(opcode & 0x1F));
    }
    fn op_shl_reg(&mut self, opcode: u16) {
        let reg = opcode & 0x7F;
        let s = self.reg_value_no_bus(reg) & 0x1F;
        self.r.a = self.alg_shl(self.r.a, s);
    }
    fn op_shl_imm(&mut self, opcode: u16) {
        self.r.a = self.alg_shl(self.r.a, u32::from(opcode & 0x1F));
    }

    /// [`Self::read_register`] for operand fetches that (per ares' real hardware behavior) never
    /// legitimately target the async bus-port registers (`$2E`/`$2F`) mid-ALU-op; using a no-op
    /// bus here avoids threading a live `&mut impl Hg51bBus` through every ALU opcode variant
    /// for a case that doesn't arise in practice (`$2E`/`$2F` are host-driven memory-fetch
    /// triggers, not ALU operands).
    fn reg_value_no_bus(&mut self, reg: u16) -> u32 {
        match reg {
            0x2E | 0x2F => 0,
            _ => self.read_register(reg, &mut NullBus),
        }
    }

    // --- Stack. ------------------------------------------------------------------------------

    fn push(&mut self) {
        for i in (1..8).rev() {
            self.stack[i] = self.stack[i - 1];
        }
        self.stack[0] = (u32::from(self.r.pb) << 8) | u32::from(self.r.pc);
    }

    fn pull(&mut self) {
        let pc = self.stack[0];
        for i in 0..7 {
            self.stack[i] = self.stack[i + 1];
        }
        self.stack[7] = 0;
        self.r.pb = (pc >> 8) as u16 & 0x7FFF;
        self.r.pc = pc as u8;
    }

    // --- ALU algorithms (flags set here, mirroring ares' algorithmXXX). ---------------------

    fn alg_add(&mut self, x: u32, y: u32) -> u32 {
        let x24 = x & 0xFF_FFFF;
        let y24 = y & 0xFF_FFFF;
        let z = x24.wrapping_add(y24);
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z.trailing_zeros() >= 24;
        self.r.c = z > 0xFF_FFFF;
        self.r.v = (!(x24 ^ y24) & (x24 ^ z)) & 0x80_0000 != 0;
        z & 0xFF_FFFF
    }

    fn alg_sub(&mut self, x: u32, y: u32) -> u32 {
        let x24 = x & 0xFF_FFFF;
        let y24 = y & 0xFF_FFFF;
        let z = x24.wrapping_sub(y24);
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z.trailing_zeros() >= 24;
        self.r.c = x24 >= y24;
        self.r.v = (!(x24 ^ y24) & (x24 ^ z)) & 0x80_0000 != 0;
        z & 0xFF_FFFF
    }

    fn alg_and(&mut self, x: u32, y: u32) -> u32 {
        let z = (x & y) & 0xFF_FFFF;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_or(&mut self, x: u32, y: u32) -> u32 {
        let z = (x | y) & 0xFF_FFFF;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_xor(&mut self, x: u32, y: u32) -> u32 {
        let z = (x ^ y) & 0xFF_FFFF;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_xnor(&mut self, x: u32, y: u32) -> u32 {
        let z = (!x ^ y) & 0xFF_FFFF;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_sx(&mut self, x: u32) -> u32 {
        let z = x & 0xFF_FFFF;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_shl(&mut self, a: u32, s: u32) -> u32 {
        let s = if s > 24 { 0 } else { s };
        let z = (a << s) & 0xFF_FFFF;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_shr(&mut self, a: u32, s: u32) -> u32 {
        let s = if s > 24 { 0 } else { s };
        let z = (a & 0xFF_FFFF) >> s;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_asr(&mut self, a: u32, s: u32) -> u32 {
        let s = if s > 24 { 0 } else { s };
        let signed = sign_extend24(a) >> s;
        let z = signed as u32 & 0xFF_FFFF;
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }

    fn alg_ror(&mut self, a: u32, s: u32) -> u32 {
        let s = if s > 24 { 0 } else { s };
        let a = a & 0xFF_FFFF;
        let z = if s == 0 { a } else { ((a >> s) | (a << (24 - s))) & 0xFF_FFFF };
        self.r.n = z & 0x80_0000 != 0;
        self.r.z = z == 0;
        z
    }
}

fn reg_shift(opcode: u16) -> (u16, u32) {
    let reg = opcode & 0x7F;
    let shift = SHIFTS[usize::from((opcode >> 8) & 0x3)];
    (reg, shift)
}

fn imm_shift(opcode: u16) -> (u32, u32) {
    let imm = u32::from(opcode & 0xFF);
    let shift = SHIFTS[usize::from((opcode >> 8) & 0x3)];
    (imm, shift)
}

const fn alg_mul(x: u32, y: u32) -> u64 {
    let xi = sign_extend24(x);
    let yi = sign_extend24(y);
    (xi as i64 * yi as i64) as u64 & 0xFFFF_FFFF_FFFF
}

/// Sign-extend a 24-bit value (stored in the low 24 bits of a `u32`) to `i32`.
const fn sign_extend24(v: u32) -> i32 {
    (((v & 0xFF_FFFF) << 8) as i32) >> 8
}

/// A no-op bus for register-index reads that structurally can't hit the async bus-port trigger
/// (see [`Hg51b::reg_value_no_bus`]) — never actually invoked, since those cases are filtered out
/// before reaching [`Hg51b::read_register`].
struct NullBus;
impl Hg51bBus for NullBus {
    fn is_rom(&self, _address: u32) -> bool {
        false
    }
    fn is_ram(&self, _address: u32) -> bool {
        false
    }
    fn read(&mut self, _address: u32) -> u8 {
        0
    }
    fn write(&mut self, _address: u32, _data: u8) {}
}
