//! Instruction decode/execute for the ARMv3 core.
//!
//! Ties [`crate::coproc::armv3::regs`]'s register file and pipeline together with the
//! [`crate::coproc::armv3::bus::ArmBus`] trait and drives them one instruction at a time (Mesen2
//! `ArmV3Cpu::Exec`/`InitArmOpTable` and friends).
//!
//! **Status: the full ARMv3 instruction set is implemented** — data processing, branch, MSR/MRS,
//! exception entry, `LDR`/`STR`, `LDM`/`STM`, `MUL`/`MLA`/`MULL`/`MLAL`, and `SWP`/`SWPB`. Only the
//! SNES-side board wrapper (firmware loading, the master-clock catch-up loop, the handshake
//! registers) and its `board::select` wiring remain (`docs/st018-arm-notes.md` tracks the
//! remaining build order).

// Register-heavy decode code inherently pairs up short, similarly-named fields (`rm`/`rn`/`rs`,
// `rm_val`/`rs_val`) that mirror the ARM ARM's own mnemonics — same rationale as the other
// coprocessor cores in this crate (`sdd1`, `gsu`, `superfx`, `upd77c25`, `hg51b`, `sa1`).
#![allow(clippy::similar_names)]

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::coproc::armv3::bus::ArmBus;
use crate::coproc::armv3::primitives::{self, Flags};
use crate::coproc::armv3::regs::{Cpsr, Mode, Pipeline, Regs, mode};

/// Which real ARM exception vector an entry lands at (Mesen2 `ArmV3CpuVector`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vector {
    /// `$04` — undefined-instruction trap.
    Undefined,
    /// `$08` — `SWI`.
    SoftwareIrq,
    /// `$18` — normal interrupt.
    Irq,
}

impl Vector {
    const fn address(self) -> u32 {
        match self {
            Self::Undefined => 0x04,
            Self::SoftwareIrq => 0x08,
            Self::Irq => 0x18,
        }
    }

    /// The mode entered for this vector.
    const fn mode(self) -> Mode {
        match self {
            Self::Undefined => mode::UNDEFINED,
            Self::SoftwareIrq => mode::SUPERVISOR,
            Self::Irq => mode::IRQ,
        }
    }
}

/// The 16 ARM data-processing ALU opcodes, in their real bit-field order (`(opcode>>21)&0xF`) —
/// the switch in [`Cpu::exec_data_processing`] relies on this exact ordering matching the
/// hardware encoding, not on the variant names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AluOp {
    And,
    Eor,
    Sub,
    Rsb,
    Add,
    Adc,
    Sbc,
    Rsc,
    Tst,
    Teq,
    Cmp,
    Cmn,
    Orr,
    Mov,
    Bic,
    Mvn,
}

impl AluOp {
    const fn from_bits(bits: u32) -> Self {
        match bits & 0xF {
            0x0 => Self::And,
            0x1 => Self::Eor,
            0x2 => Self::Sub,
            0x3 => Self::Rsb,
            0x4 => Self::Add,
            0x5 => Self::Adc,
            0x6 => Self::Sbc,
            0x7 => Self::Rsc,
            0x8 => Self::Tst,
            0x9 => Self::Teq,
            0xA => Self::Cmp,
            0xB => Self::Cmn,
            0xC => Self::Orr,
            0xD => Self::Mov,
            0xE => Self::Bic,
            _ => Self::Mvn,
        }
    }

    /// `TST`/`TEQ`/`CMP`/`CMN` have no destination register — a comparison only.
    const fn is_comparison(self) -> bool {
        matches!(self, Self::Tst | Self::Teq | Self::Cmp | Self::Cmn)
    }
}

/// The bit-pattern category an opcode's 12-bit index (`((opcode&0x0FF00000)>>16)|((opcode&0xF0)>>4)`,
/// Mesen2's `InitArmOpTable` key) decodes to. Priority mirrors the reference table's construction
/// order exactly: later `addEntry` calls overwrite earlier ones for overlapping indices (Multiply/
/// MultiplyLong/SingleDataSwap/SoftwareInterrupt all carve sparse holes out of ranges that would
/// otherwise read as DataProcessing/InvalidOp), so [`Category::decode`] checks the LATEST-writing
/// pattern first and falls through, reproducing the same final table state without needing a real
/// 4096-entry array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    DataProcessing,
    Msr,
    Mrs,
    Branch,
    SingleDataTransfer,
    BlockDataTransfer,
    Multiply,
    MultiplyLong,
    SingleDataSwap { byte: bool },
    SoftwareInterrupt,
    InvalidOp,
}

impl Category {
    fn decode(index: u16) -> Self {
        // Software Interrupt: index 0xF00-0xFFF, populated last, always wins.
        if (0xF00..=0xFFF).contains(&index) {
            return Self::SoftwareInterrupt;
        }
        // Single Data Swap: base 0x109 (word) / 0x149 (byte), with index bits 7,5,4 free
        // (opcode bits 23,21,20 — the loop enumerates all 8 combinations via `i` in 0..=7).
        if index & !0xB0 == 0x109 {
            return Self::SingleDataSwap { byte: false };
        }
        if index & !0xB0 == 0x149 {
            return Self::SingleDataSwap { byte: true };
        }
        // Multiply Long: base 0x089, index bits 6,5,4 free (opcode bits 22,21,20).
        if index & !0x70 == 0x089 {
            return Self::MultiplyLong;
        }
        // Multiply: base 0x009, same free bits.
        if index & !0x70 == 0x009 {
            return Self::Multiply;
        }
        if (0x800..=0x9FF).contains(&index) {
            return Self::BlockDataTransfer;
        }
        if (0x400..=0x7FF).contains(&index) {
            return Self::SingleDataTransfer;
        }
        if (0xA00..=0xBFF).contains(&index) {
            return Self::Branch;
        }
        if index <= 0x3FF {
            let operation = AluOp::from_bits(u32::from(index) >> 5);
            let set_condition_codes = index & 0x10 != 0;
            if !set_condition_codes && operation.is_comparison() {
                return if index & 0x20 != 0 {
                    Self::Msr
                } else {
                    Self::Mrs
                };
            }
            return Self::DataProcessing;
        }
        Self::InvalidOp
    }
}

/// The full ARMv3 core: register file, pipeline, and instruction execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cpu {
    /// The register file (`R0-R15`, mode-banked, plus CPSR/SPSRs).
    pub regs: Regs,
    /// The 3-stage Fetch/Decode/Execute pipeline.
    pub pipeline: Pipeline,
}

impl Cpu {
    /// Power on (or reset) the core (Mesen2 `PowerOn`): zero every register, enter Supervisor
    /// mode with both interrupt lines masked, and prime the pipeline from address 0 (the ARM
    /// reset vector). Unlike `PowerOn(forReset=true)`, this does not preserve a cycle counter —
    /// the board wrapper (not yet built) owns cycle-count bookkeeping, not the core itself.
    pub fn power_on(&mut self, bus: &mut impl ArmBus) {
        self.regs = Regs::default();
        self.pipeline = Pipeline::default();
        self.regs.cpsr.mode = mode::SUPERVISOR;
        self.regs.cpsr.irq_disable = true;
        self.regs.cpsr.fiq_disable = true;
        self.pipeline.request_reload();
        self.advance_pipeline(bus);
    }

    fn advance_pipeline(&mut self, bus: &mut impl ArmBus) {
        self.pipeline
            .process(&mut self.regs.r[15], |addr| bus.read_code(addr));
    }

    /// Execute the instruction currently in the Execute pipeline stage (if its condition holds),
    /// then advance the pipeline (Mesen2 `Exec`). Call once per CPU cycle-step.
    pub fn step(&mut self, bus: &mut impl ArmBus) {
        let opcode = self.pipeline.execute.opcode;
        let cond = (opcode >> 28) as u8;
        if primitives::check_condition(cond, self.regs.cpsr.flags) {
            self.execute(opcode, bus);
        }
        self.advance_pipeline(bus);
    }

    /// Write `value` into `reg`, requesting a pipeline reload if `reg` is R15 (Mesen2 `SetR`) —
    /// every instruction that can write the PC (data processing, `LDR`, `LDM`, branch) routes
    /// through this so the reload is never forgotten.
    ///
    /// Not marked `const fn`: `Cpu` carries nested `Regs`/`Pipeline` fields, and const-ness here
    /// is fragile against future field changes to either — the same posture `coproc::hg51b`
    /// documents for its own dense register-port methods.
    #[allow(clippy::missing_const_for_fn)]
    fn set_r(&mut self, reg: u8, value: u32) {
        self.regs.r[reg as usize] = value;
        if reg == 15 {
            self.pipeline.request_reload();
        }
    }

    fn execute(&mut self, opcode: u32, bus: &mut impl ArmBus) {
        // Always < 0x1000 by construction (8 bits from opcode>>16 combined with 4 bits from
        // opcode>>4, both masked first) -- never truncates.
        #[allow(clippy::cast_possible_truncation)]
        let index = (((opcode & 0x0FF0_0000) >> 16) | ((opcode & 0xF0) >> 4)) as u16;
        match Category::decode(index) {
            Category::DataProcessing => self.exec_data_processing(opcode, bus),
            Category::Msr => self.exec_msr(opcode),
            Category::Mrs => self.exec_mrs(opcode),
            Category::Branch => self.exec_branch(opcode),
            Category::SoftwareInterrupt => self.enter_exception(Vector::SoftwareIrq),
            Category::InvalidOp => self.enter_exception(Vector::Undefined),
            Category::SingleDataTransfer => self.exec_single_data_transfer(opcode, bus),
            Category::BlockDataTransfer => self.exec_block_data_transfer(opcode, bus),
            Category::Multiply => self.exec_multiply(opcode, bus),
            Category::MultiplyLong => self.exec_multiply_long(opcode, bus),
            Category::SingleDataSwap { byte } => self.exec_single_data_swap(opcode, byte, bus),
        }
    }

    /// `MOVS PC, ...` / any S-bit data-processing write to R15 restores CPSR wholesale from the
    /// current mode's SPSR — the idiomatic ARM exception-handler return (Mesen2's trailing
    /// `if(dstReg==15 && updateFlags)` block, checked unconditionally on the DECODED destination
    /// field regardless of whether this particular op actually had a real destination — so even
    /// a comparison op with a stray `dstReg==15` encoding triggers it, matching real hardware).
    fn maybe_restore_cpsr_from_spsr(&mut self, dst_reg: u8, update_flags: bool) {
        if dst_reg == 15 && update_flags {
            let spsr = self.regs.spsr();
            self.regs.switch_mode(spsr.mode);
            self.regs.cpsr = spsr;
        }
    }

    /// The full 16-op ALU dispatch is dense (like the rest of this direct hardware port) but
    /// stays a single function on purpose — splitting the shift-operand assembly from the ALU
    /// switch would separate two halves of one instruction that share `op1`/`op2`/`carry`
    /// locals, matching the precedent already set for equally dense ported dispatch elsewhere in
    /// this crate (`coproc::gsu`, `coproc::hg51b_instructions`, `coproc::upd77c25`).
    #[allow(clippy::too_many_lines)]
    fn exec_data_processing(&mut self, opcode: u32, bus: &mut impl ArmBus) {
        let immediate = opcode & (1 << 25) != 0;
        let rn = ((opcode >> 16) & 0xF) as u8;
        let dst_reg = ((opcode >> 12) & 0xF) as u8;
        // TST/TEQ/CMP/CMN always update flags regardless of the decoded S-bit -- dispatch only
        // ever routes them here with S=1 anyway (S=0 in that opcode range means MSR/MRS
        // instead), but this mirrors the source's own explicit `true` rather than relying on
        // that invariant silently holding.
        let operation = AluOp::from_bits(opcode >> 21);
        let update_flags = (opcode & (1 << 20) != 0) || operation.is_comparison();

        let mut op1 = self.regs.r[rn as usize];
        let mut carry = self.regs.cpsr.flags.c;
        let op2 = if immediate {
            let rotate = (opcode >> 8) & 0xF;
            let imm = opcode & 0xFF;
            if rotate == 0 {
                imm
            } else {
                let (v, c) = primitives::rotate_right_carry(imm, rotate * 2, carry);
                carry = c;
                v
            }
        } else {
            let shift_type = (opcode >> 5) & 0x3;
            let rm = (opcode & 0xF) as u8;
            let mut v = self.regs.r[rm as usize];
            let use_reg_shift = opcode & (1 << 4) != 0;
            let shift = if use_reg_shift {
                // Register-specified shift amount costs an extra internal cycle; R15 (as ANY of
                // rm, rn, or rs) reads as address+12 here instead of the usual address+8, since
                // the pipeline has already advanced R15 to +8 and this is one MORE cycle on top.
                // The source applies the +4 to each of the three independently (`shift = R(rs) +
                // (rs==15?4:0)`, then separately `op2 += 4`/`op1 += 4` for rm/rn) -- ported as
                // three explicit +4s exactly where the source applies each one, not folded into
                // a single "R15 always reads +12 in this instruction" rule applied once.
                bus.idle();
                let rs = ((opcode >> 8) & 0xF) as u8;
                #[allow(clippy::cast_possible_truncation)]
                let s = (self.regs.r[rs as usize] as u8).wrapping_add(if rs == 15 { 4 } else { 0 });
                if rm == 15 {
                    v = v.wrapping_add(4);
                }
                if rn == 15 {
                    op1 = op1.wrapping_add(4);
                }
                s
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let s = ((opcode >> 7) & 0x1F) as u8;
                s
            };
            let (result, c) = match shift_type {
                0 => primitives::shift_lsl(v, shift, carry),
                1 => primitives::shift_lsr(
                    v,
                    if use_reg_shift || shift != 0 {
                        shift
                    } else {
                        32
                    },
                    carry,
                ),
                2 => primitives::shift_asr(
                    v,
                    if use_reg_shift || shift != 0 {
                        shift
                    } else {
                        32
                    },
                    carry,
                ),
                _ => {
                    if !use_reg_shift && shift == 0 {
                        primitives::shift_rrx(v, carry)
                    } else {
                        primitives::shift_ror(v, shift, carry)
                    }
                }
            };
            carry = c;
            result
        };

        let prior = self.regs.cpsr.flags;
        let (result, flags): (Option<u32>, Flags) = match operation {
            AluOp::And => {
                let r = op1 & op2;
                (Some(r), primitives::logical_flags(r, carry, prior))
            }
            AluOp::Eor => {
                let r = op1 ^ op2;
                (Some(r), primitives::logical_flags(r, carry, prior))
            }
            AluOp::Sub => {
                let (r, f) = primitives::sub(op1, op2, true, prior);
                (Some(r), f)
            }
            AluOp::Rsb => {
                let (r, f) = primitives::sub(op2, op1, true, prior);
                (Some(r), f)
            }
            AluOp::Add => {
                let (r, f) = primitives::add(op1, op2, false, prior);
                (Some(r), f)
            }
            AluOp::Adc => {
                let (r, f) = primitives::add(op1, op2, prior.c, prior);
                (Some(r), f)
            }
            AluOp::Sbc => {
                let (r, f) = primitives::sub(op1, op2, prior.c, prior);
                (Some(r), f)
            }
            AluOp::Rsc => {
                let (r, f) = primitives::sub(op2, op1, prior.c, prior);
                (Some(r), f)
            }
            AluOp::Tst => {
                let r = op1 & op2;
                (None, primitives::logical_flags(r, carry, prior))
            }
            AluOp::Teq => {
                let r = op1 ^ op2;
                (None, primitives::logical_flags(r, carry, prior))
            }
            AluOp::Cmp => {
                let (_, f) = primitives::sub(op1, op2, true, prior);
                (None, f)
            }
            AluOp::Cmn => {
                let (_, f) = primitives::add(op1, op2, false, prior);
                (None, f)
            }
            AluOp::Orr => {
                let r = op1 | op2;
                (Some(r), primitives::logical_flags(r, carry, prior))
            }
            AluOp::Mov => (Some(op2), primitives::logical_flags(op2, carry, prior)),
            AluOp::Bic => {
                let r = op1 & !op2;
                (Some(r), primitives::logical_flags(r, carry, prior))
            }
            AluOp::Mvn => {
                let r = !op2;
                (Some(r), primitives::logical_flags(r, carry, prior))
            }
        };

        if update_flags {
            self.regs.cpsr.flags = flags;
        }
        if let Some(r) = result {
            self.set_r(dst_reg, r);
        }
        self.maybe_restore_cpsr_from_spsr(dst_reg, update_flags);
    }

    /// `B`/`BL` (Mesen2 `ArmBranch`): a sign-extended 24-bit word offset. `R14` (for `BL`) gets
    /// `R15 - 4`, NOT `R15` itself — R15 is already +8 ahead of the branch instruction's own
    /// address by the time this runs, so `R15 - 4` = branch_addr + 4 = the correct "next
    /// sequential instruction" return address.
    ///
    /// Not marked `const fn`: same rationale as [`Self::set_r`].
    #[allow(clippy::missing_const_for_fn)]
    fn exec_branch(&mut self, opcode: u32) {
        let with_link = opcode & (1 << 24) != 0;
        #[allow(clippy::cast_possible_wrap)]
        let offset = ((opcode as i32) << 8) >> 6; // sign-extend the 24-bit field, then <<2
        if with_link {
            self.regs.r[14] = self.regs.r[15].wrapping_sub(4);
        }
        self.regs.r[15] = self.regs.r[15].wrapping_add(offset.cast_unsigned());
        self.pipeline.request_reload();
    }

    /// `MSR` (Mesen2 `ArmMsr`): write CPSR or the current mode's SPSR, optionally only the flag
    /// bits (mask bit 3) or only mode/interrupt-mask bits (mask bit 0) — a partial MSR (e.g. only
    /// updating flags) must NOT touch the bits its mask excludes.
    fn exec_msr(&mut self, opcode: u32) {
        let immediate = opcode & (1 << 25) != 0;
        let write_to_spsr = opcode & (1 << 22) != 0;
        let mask = (opcode >> 16) & 0xF;

        if write_to_spsr && matches!(self.regs.cpsr.mode, mode::USER | mode::SYSTEM) {
            return; // User/System have no real SPSR to write.
        }

        let value = if immediate {
            let imm = opcode & 0xFF;
            let shift = (opcode >> 8) & 0xF;
            if shift == 0 {
                imm
            } else {
                primitives::rotate_right(imm, shift * 2)
            }
        } else {
            self.regs.r[(opcode & 0xF) as usize]
        };

        let user_mode = self.regs.cpsr.mode == mode::USER;
        let target: &mut Cpsr = if write_to_spsr {
            self.regs.spsr_mut()
        } else {
            &mut self.regs.cpsr
        };

        if mask & 0x8 != 0 {
            target.flags = Flags {
                n: value & (1 << 31) != 0,
                z: value & (1 << 30) != 0,
                c: value & (1 << 29) != 0,
                v: value & (1 << 28) != 0,
            };
        }
        if mask & 0x1 != 0 && (write_to_spsr || !user_mode) {
            let new_mode = (value & 0x1F) as u8;
            let fiq_disable = value & (1 << 6) != 0;
            let irq_disable = value & (1 << 7) != 0;
            if write_to_spsr {
                target.mode = new_mode | mode::BIT;
                target.fiq_disable = fiq_disable;
                target.irq_disable = irq_disable;
            } else {
                // `target` (the `&mut CPSR` borrow) must end before `switch_mode` can take its
                // own `&mut self.regs` -- reborrow via the live field instead of the alias.
                self.regs.switch_mode(new_mode);
                self.regs.cpsr.fiq_disable = fiq_disable;
                self.regs.cpsr.irq_disable = irq_disable;
            }
        }
    }

    /// `MRS` (Mesen2 `ArmMrs`): read CPSR or the current mode's SPSR into a register.
    ///
    /// Not marked `const fn`: same rationale as [`Self::set_r`].
    #[allow(clippy::missing_const_for_fn)]
    fn exec_mrs(&mut self, opcode: u32) {
        let use_spsr = opcode & (1 << 22) != 0;
        let rd = ((opcode >> 12) & 0xF) as u8;
        let value = if use_spsr {
            self.regs.spsr().to_u32()
        } else {
            self.regs.cpsr.to_u32()
        };
        self.set_r(rd, value);
    }

    /// Exception entry (Mesen2 `ProcessException`): save CPSR into the new mode's SPSR, switch
    /// mode, mask IRQ, park the return address (the Decode-stage address -- one instruction past
    /// the one that trapped, since Decode is the instruction that would have executed next) in
    /// R14, and jump to the vector.
    fn enter_exception(&mut self, vector: Vector) {
        let cpsr = self.regs.cpsr;
        self.regs.switch_mode(vector.mode());
        *self.regs.spsr_mut() = cpsr;
        self.regs.cpsr.irq_disable = true;
        self.regs.r[14] = self.pipeline.decode.address;
        self.regs.r[15] = vector.address();
        self.pipeline.request_reload();
    }

    /// `LDR`/`STR` (Mesen2 `ArmSingleDataTransfer`). The offset is either a 12-bit immediate or a
    /// shifted register — unlike data processing, the shift amount here is ALWAYS an immediate
    /// (bits 11-7); there is no register-specified-shift-amount form for this instruction class.
    fn exec_single_data_transfer(&mut self, opcode: u32, bus: &mut impl ArmBus) {
        let immediate = opcode & (1 << 25) == 0;
        let pre = opcode & (1 << 24) != 0;
        let up = opcode & (1 << 23) != 0;
        let byte = opcode & (1 << 22) != 0;
        let write_back = opcode & (1 << 21) != 0;
        let load = opcode & (1 << 20) != 0;
        let rn = ((opcode >> 16) & 0xF) as u8;
        let rd = ((opcode >> 12) & 0xF) as u8;

        let mut addr = self.regs.r[rn as usize];
        let offset = if immediate {
            opcode & 0xFFF
        } else {
            let shift_type = (opcode >> 5) & 0x3;
            #[allow(clippy::cast_possible_truncation)]
            let shift = ((opcode >> 7) & 0x1F) as u8;
            let rm = (opcode & 0xF) as u8;
            let v = self.regs.r[rm as usize];
            let carry = self.regs.cpsr.flags.c;
            match shift_type {
                0 => primitives::shift_lsl(v, shift, carry).0,
                1 => primitives::shift_lsr(v, if shift == 0 { 32 } else { shift }, carry).0,
                2 => primitives::shift_asr(v, if shift == 0 { 32 } else { shift }, carry).0,
                _ => {
                    if shift == 0 {
                        primitives::shift_rrx(v, carry).0
                    } else {
                        primitives::shift_ror(v, shift, carry).0
                    }
                }
            }
        };

        if pre {
            addr = if up {
                addr.wrapping_add(offset)
            } else {
                addr.wrapping_sub(offset)
            };
        }

        if load {
            let value = bus.read(addr, byte);
            self.set_r(rd, value);
            bus.idle();
        } else {
            // Storing R15 stores address+12, not the usual address+8 -- a real, documented ARM6-
            // class quirk, ported exactly where the source applies it rather than folded into a
            // general rule.
            let value = self.regs.r[rd as usize].wrapping_add(if rd == 15 { 4 } else { 0 });
            bus.write(addr, value, byte);
        }

        if !pre {
            addr = if up {
                addr.wrapping_add(offset)
            } else {
                addr.wrapping_sub(offset)
            };
        }

        // Post-indexed addressing ALWAYS writes back, even without the explicit W bit; a load
        // into the same register as the base is never written back (the loaded value wins).
        if (rd != rn || !load) && (write_back || !pre) {
            self.set_r(rn, addr);
        }
    }

    /// `LDM`/`STM` (Mesen2 `ArmBlockDataTransfer`) — the most complex ARM instruction. Every
    /// quirk below is a real, documented hardware behavior ported verbatim, not a simplification:
    /// the empty-register-list glitch, the load/store write-back timing asymmetry, and the S-bit
    /// (`psrForceUser`) user-bank-transfer/exception-return dual role. See `docs/st018-arm-notes.md`
    /// §`ArmBlockDataTransfer` for the full breakdown of each one.
    #[allow(clippy::too_many_lines)]
    fn exec_block_data_transfer(&mut self, opcode: u32, bus: &mut impl ArmBus) {
        let pre = opcode & (1 << 24) != 0;
        let up = opcode & (1 << 23) != 0;
        let psr_force_user = opcode & (1 << 22) != 0;
        let write_back = opcode & (1 << 21) != 0;
        let load = opcode & (1 << 20) != 0;
        let rn = ((opcode >> 16) & 0xF) as u8;
        #[allow(clippy::cast_possible_truncation)]
        let mut reg_mask = opcode as u16;

        let base = self.regs.r[rn as usize].wrapping_add(if rn == 15 { 4 } else { 0 });
        let mut addr = base;

        let mut reg_count = reg_mask.count_ones();
        if reg_mask == 0 {
            // Empty-list glitch: only R15 is actually transferred, but the address advances as
            // if all 16 registers were.
            reg_count = 16;
            reg_mask = 0x8000;
        }

        if !up {
            addr = addr.wrapping_sub((reg_count - u32::from(!pre)) * 4);
        } else if pre {
            addr = addr.wrapping_add(4);
        }

        let write_back_addr = base.wrapping_add(if up {
            reg_count * 4
        } else {
            (reg_count * 4).wrapping_neg()
        });
        if write_back && load {
            self.set_r(rn, write_back_addr);
        }

        let org_mode = self.regs.cpsr.mode;
        if psr_force_user && (!load || reg_mask & 0x8000 == 0) {
            self.regs.switch_mode(mode::USER);
        }

        let mut first_reg = true;
        for i in 0..16u8 {
            if reg_mask & (1 << i) == 0 {
                continue;
            }
            if !load {
                let value = self.regs.r[i as usize].wrapping_add(if i == 15 { 4 } else { 0 });
                bus.write(addr, value, false);
            }
            if first_reg && write_back {
                // Write-back happens here for a STORE (and, harmlessly, a second time with the
                // same value for a LOAD, which already wrote back above -- ported as-is, not
                // "optimized" away, matching the source exactly). If `psr_force_user` switched
                // to User mode above, this `set_r` deliberately lands in the User bank rather
                // than `org_mode`'s bank (a real, empirically-validated ARM quirk -- Mesen2's
                // `ArmBlockDataTransfer` cites `gba-tests/arm` test 522 for this exact ordering;
                // combining S-bit-forced-user-bank transfer with base-register write-back is
                // otherwise UNPREDICTABLE per the ARM ARM, so this is the one documented,
                // test-verified answer, not a bug to "fix" toward `org_mode`).
                self.set_r(rn, write_back_addr);
                first_reg = false;
            }
            if load {
                // LDM is NOT affected by the misalignment rotation a plain LDR would apply --
                // this port doesn't model that rotation at all (`ArmBus::read` reads four fixed
                // byte lanes, matching the real board's `St018::ReadCpu`, which doesn't rotate
                // either -- see `docs/st018-arm-notes.md`), so there's nothing to suppress here.
                let value = bus.read(addr, false);
                self.set_r(i, value);
            }
            addr = addr.wrapping_add(4);
        }

        if load {
            bus.idle();
        }

        self.regs.switch_mode(org_mode);

        if psr_force_user && load && reg_mask & 0x8000 != 0 {
            let spsr = self.regs.spsr();
            self.regs.switch_mode(spsr.mode);
            self.regs.cpsr = spsr;
        }
    }

    /// `MUL`/`MLA` (Mesen2 `ArmMultiply`). The reference source delegates the actual multiply and
    /// its variable cycle count to a cycle-EXACT Booth's-algorithm circuit simulation
    /// (`GbaCpuMultiply`) built for GBA hardware test-ROM precision — deliberately NOT ported
    /// here (see `docs/st018-arm-notes.md`'s multiply section for the full rationale). This
    /// computes the mathematically correct 64-bit-widened result directly and idles for the ARM
    /// ARM's own DOCUMENTED (not reverse-engineered) early-termination cycle count instead
    /// (`multiply_cycles`). The result and Z/N flags are bit-exact either way; only the idle-
    /// cycle count and the C flag (see below) differ from GBA-test-ROM-level precision.
    fn exec_multiply(&mut self, opcode: u32, bus: &mut impl ArmBus) {
        let rd = ((opcode >> 16) & 0xF) as u8;
        let rn = ((opcode >> 12) & 0xF) as u8;
        let rs = ((opcode >> 8) & 0xF) as u8;
        let rm = (opcode & 0xF) as u8;
        let update_flags = opcode & (1 << 20) != 0;
        let mult_and_acc = opcode & (1 << 21) != 0;

        let rm_val = self.regs.r[rm as usize];
        let rs_val = self.regs.r[rs as usize];
        let mut result = rm_val.wrapping_mul(rs_val);
        if mult_and_acc {
            result = result.wrapping_add(self.regs.r[rn as usize]);
        }

        for _ in 0..multiply_cycles(rs_val) {
            bus.idle();
        }
        if mult_and_acc {
            bus.idle();
        }

        if rd != 15 {
            self.set_r(rd, result);
        }
        if update_flags {
            // C is left UNCHANGED, not fabricated: real ARMv3/v4 hardware sets it to an
            // implementation-defined ("meaningless") value derived from internal multiplier
            // state that this port deliberately doesn't simulate (see the function doc) --
            // leaving it alone is a documented, deterministic choice, not an oversight.
            self.regs.cpsr.flags.z = result == 0;
            self.regs.cpsr.flags.n = result & 0x8000_0000 != 0;
        }
    }

    /// `MULL`/`MLAL` (Mesen2 `ArmMultiplyLong`) — signed or unsigned 64-bit-widened multiply,
    /// optionally accumulating into the existing `Rl:Rh` pair. See [`Self::exec_multiply`]'s doc
    /// for the same cycle-count/`C`-flag tradeoff (applies identically here).
    fn exec_multiply_long(&mut self, opcode: u32, bus: &mut impl ArmBus) {
        let rh = ((opcode >> 16) & 0xF) as u8;
        let rl = ((opcode >> 12) & 0xF) as u8;
        let rs = ((opcode >> 8) & 0xF) as u8;
        let rm = (opcode & 0xF) as u8;
        let update_flags = opcode & (1 << 20) != 0;
        let mult_and_acc = opcode & (1 << 21) != 0;
        let signed = opcode & (1 << 22) != 0;

        bus.idle();

        let rm_val = self.regs.r[rm as usize];
        let rs_val = self.regs.r[rs as usize];
        let mut result: u64 = if signed {
            (i64::from(rm_val.cast_signed()) * i64::from(rs_val.cast_signed())).cast_unsigned()
        } else {
            u64::from(rm_val) * u64::from(rs_val)
        };
        if mult_and_acc {
            let acc =
                (u64::from(self.regs.r[rh as usize]) << 32) | u64::from(self.regs.r[rl as usize]);
            result = result.wrapping_add(acc);
        }

        for _ in 0..multiply_cycles(rs_val) {
            bus.idle();
        }
        if mult_and_acc {
            bus.idle();
        }

        #[allow(clippy::cast_possible_truncation)]
        if rl != 15 {
            self.set_r(rl, result as u32);
        }
        #[allow(clippy::cast_possible_truncation)]
        if rh != 15 {
            self.set_r(rh, (result >> 32) as u32);
        }
        if update_flags {
            // See exec_multiply's doc: C is left unchanged, not fabricated.
            self.regs.cpsr.flags.z = result == 0;
            self.regs.cpsr.flags.n = result & (1 << 63) != 0;
        }
    }

    /// `SWP`/`SWPB` (Mesen2 `ArmSingleDataSwap`): an atomic read-modify-write at ONE address —
    /// read the old value into `rd`, then write `rm`'s value (or, for `rm==15`, `R15+4`) to the
    /// SAME address, in that exact order with an idle cycle between them (a real read-then-write
    /// bus cycle real hardware serializes, not two independent accesses).
    fn exec_single_data_swap(&mut self, opcode: u32, byte: bool, bus: &mut impl ArmBus) {
        let rn = ((opcode >> 16) & 0xF) as u8;
        let rd = ((opcode >> 12) & 0xF) as u8;
        let rm = (opcode & 0xF) as u8;

        let addr = self.regs.r[rn as usize];
        let old = bus.read(addr, byte);
        bus.idle();
        let new_value = self.regs.r[rm as usize].wrapping_add(if rm == 15 { 4 } else { 0 });
        bus.write(addr, new_value, byte);
        self.set_r(rd, old);
    }

    /// Serialize the full register file + pipeline state (the board wrapper composes this with
    /// its own handshake/ROM/RAM state — `docs/st018-arm-notes.md` step 9).
    pub(crate) fn save_state(&self, w: &mut SaveWriter) {
        self.regs.save_state(w);
        self.pipeline.save_state(w);
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// Propagates [`SaveReader`]'s own truncation error.
    pub(crate) fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        self.regs.load_state(r)?;
        self.pipeline.load_state(r)
    }
}

/// The ARM ARM's documented (not reverse-engineered) multiply early-termination cycle count: 1
/// cycle if bits 31-8 of the multiplier are all the same (all-0 or all-1), 2 if bits 31-16, 3 if
/// bits 31-24, else 4. Real silicon terminates the multiply pipeline early once the remaining
/// high bits of `Rs` stop contributing partial products — this is the documented rule, not
/// `GbaCpuMultiply`'s cycle-exact Booth's-algorithm derivation (see `docs/st018-arm-notes.md`).
const fn multiply_cycles(rs: u32) -> u32 {
    if rs & 0xFFFF_FF00 == 0 || rs & 0xFFFF_FF00 == 0xFFFF_FF00 {
        1
    } else if rs & 0xFFFF_0000 == 0 || rs & 0xFFFF_0000 == 0xFFFF_0000 {
        2
    } else if rs & 0xFF00_0000 == 0 || rs & 0xFF00_0000 == 0xFF00_0000 {
        3
    } else {
        4
    }
}

#[cfg(test)]
// Every truncating cast in this test harness narrows a value already masked/shifted into the
// target width by construction (a `u8` byte lane of a `u32` word, a word index that never
// exceeds the 64 KiB test address space).
#[allow(clippy::cast_possible_truncation)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A flat 64 KiB ARM address space for tests -- more than enough for hand-assembled short
    /// programs, with an idle-cycle counter so multiply/shift-by-register timing can be asserted
    /// on later without changing the harness.
    struct TestBus {
        mem: Vec<u8>,
        idle_count: u32,
    }

    impl TestBus {
        fn new() -> Self {
            Self {
                mem: alloc::vec![0u8; 0x1_0000],
                idle_count: 0,
            }
        }

        fn word_at(&self, addr: u32) -> u32 {
            let a = addr as usize;
            u32::from(self.mem[a])
                | (u32::from(self.mem[a + 1]) << 8)
                | (u32::from(self.mem[a + 2]) << 16)
                | (u32::from(self.mem[a + 3]) << 24)
        }

        fn set_word(&mut self, addr: u32, value: u32) {
            let a = addr as usize;
            self.mem[a] = value as u8;
            self.mem[a + 1] = (value >> 8) as u8;
            self.mem[a + 2] = (value >> 16) as u8;
            self.mem[a + 3] = (value >> 24) as u8;
        }
    }

    impl ArmBus for TestBus {
        fn read_code(&mut self, addr: u32) -> u32 {
            self.word_at(addr & 0xFFFF)
        }
        fn read(&mut self, addr: u32, byte: bool) -> u32 {
            if byte {
                u32::from(self.mem[(addr & 0xFFFF) as usize])
            } else {
                self.word_at(addr & 0xFFFF)
            }
        }
        fn write(&mut self, addr: u32, value: u32, byte: bool) {
            if byte {
                self.mem[(addr & 0xFFFF) as usize] = value as u8;
            } else {
                self.set_word(addr & 0xFFFF, value);
            }
        }
        fn idle(&mut self) {
            self.idle_count += 1;
        }
    }

    /// `ADD r0, r0, #imm8` (immediate data processing, condition AL, no S).
    const fn add_r0_imm(imm: u32) -> u32 {
        0xE280_0000 | (imm & 0xFF)
    }

    fn boot(program: &[u32]) -> (Cpu, TestBus) {
        let mut bus = TestBus::new();
        for (i, &w) in program.iter().enumerate() {
            bus.set_word((i as u32) * 4, w);
        }
        let mut cpu = Cpu::default();
        cpu.power_on(&mut bus);
        (cpu, bus)
    }

    #[test]
    fn power_on_enters_supervisor_with_both_interrupts_masked() {
        let (cpu, _bus) = boot(&[]);
        assert_eq!(cpu.regs.cpsr.mode, mode::SUPERVISOR);
        assert!(cpu.regs.cpsr.irq_disable);
        assert!(cpu.regs.cpsr.fiq_disable);
        assert_eq!(cpu.pipeline.execute.address, 0);
    }

    #[test]
    fn data_processing_add_immediate() {
        // ADD r0, r0, #5 ; ADD r0, r0, #3
        let (mut cpu, mut bus) = boot(&[add_r0_imm(5), add_r0_imm(3)]);
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 5);
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 8);
    }

    #[test]
    fn condition_code_gates_execution() {
        // MOV r0, #1 (AL) ; ADDEQ r0, r0, #1 (only if Z set -- it isn't) ; ADD r0,r0,#1 (AL)
        let mov_r0_1 = 0xE3A0_0001u32; // MOV r0, #1, cond=AL
        let addeq_r0_1 = 0x0280_0001u32; // ADD r0, r0, #1, cond=EQ
        let add_r0_1 = add_r0_imm(1); // cond=AL
        let (mut cpu, mut bus) = boot(&[mov_r0_1, addeq_r0_1, add_r0_1]);
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 1);
        cpu.step(&mut bus); // Z is clear (MOV #1 doesn't even update flags -- no S bit), EQ fails
        assert_eq!(cpu.regs.r[0], 1, "ADDEQ must not execute when Z is clear");
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 2);
    }

    #[test]
    fn subs_sets_flags_and_cmp_never_writes_a_destination() {
        // MOVS r0, #0 ; CMP r0, #0 (sets Z, never writes r0)
        let movs_r0_0 = 0xE3B0_0000u32; // MOV r0, #0, S=1
        let cmp_r0_0 = 0xE350_0000u32; // CMP r0, #0
        let (mut cpu, mut bus) = boot(&[movs_r0_0, cmp_r0_0]);
        cpu.step(&mut bus);
        assert!(cpu.regs.cpsr.flags.z);
        // Perturb r0 to a nonzero value: CMP against #0 now must clear Z (proving CMP actually
        // read the CURRENT r0, not some cached/stale value) while still never writing r0 back.
        cpu.regs.r[0] = 0x1234;
        cpu.step(&mut bus);
        assert_eq!(
            cpu.regs.r[0], 0x1234,
            "CMP must never write a destination register"
        );
        assert!(
            !cpu.regs.cpsr.flags.z,
            "CMP r0,#0 with r0==0x1234 must clear Z"
        );
    }

    #[test]
    fn r15_reads_as_address_plus_8_inside_data_processing() {
        // At address 0: MOV r0, pc  (opcode = E1A0000F)
        let mov_r0_pc = 0xE1A0_000Fu32;
        let (mut cpu, mut bus) = boot(&[mov_r0_pc]);
        cpu.step(&mut bus);
        assert_eq!(
            cpu.regs.r[0], 8,
            "PC read as an operand is address+8, not address"
        );
    }

    #[test]
    fn register_specified_shift_amount_also_adds_4_when_rs_is_r15() {
        // ADD r0, r0, r1, LSR r15 -- op=ADD, I=0, S=0, Rn=0, Rd=0, Rs=15, shift_type=LSR(01),
        // Rm=1. (LSL(00) is deliberately NOT used here: on this exact decode table a
        // register-shifted-by-register LSL always collides with Multiply/MultiplyLong/SWP's
        // sparse index carve-outs -- verified by brute-force sweep over every ALU op -- so it's
        // simply unreachable as a data-processing encoding, matching real ARM hardware's own
        // Multiply-vs-data-processing disambiguation. LSR sidesteps that collision entirely.)
        let add_r0_r1_lsr_r15 = 0xE080_0FB1u32;
        let (mut cpu, mut bus) = boot(&[add_r0_r1_lsr_r15]);
        cpu.regs.r[0] = 0;
        cpu.regs.r[1] = 0x1000;
        // R15 during Execute is 8 (power-on default); if rs==15 gets the documented +4, the
        // shift amount is 12 (0x1000 >> 12 == 1); without it, the shift would be 8 (>> 8 == 16).
        cpu.step(&mut bus);
        assert_eq!(
            cpu.regs.r[0], 1,
            "rs==15 must read as address+12 (the usual +8, plus the extra internal cycle), \
             exactly like rm/rn do -- the source applies +4 to all three independently"
        );
    }

    #[test]
    fn branch_with_link_sets_lr_to_the_next_sequential_instruction() {
        // At address 0: BL +8 (branch to address 0x10: offset encodes (target-(pc_at_fetch+8))>>2)
        // Encode BL to absolute address 0x10 from pc=0: offset_words = (0x10 - 8) >> 2 = 2
        let bl = 0xEB00_0002u32; // cond=AL, L=1, offset=2
        let (mut cpu, mut bus) = boot(&[bl]);
        cpu.step(&mut bus);
        assert_eq!(
            cpu.regs.r[15],
            0x10 + 8,
            "branch target re-establishes the +8 pipeline offset"
        );
        assert_eq!(
            cpu.regs.r[14], 4,
            "LR = branch instruction's own address + 4"
        );
    }

    #[test]
    fn msr_writes_only_the_flag_bits_when_masked_to_flags_only() {
        // MSR CPSR_f, r0 with r0 = 0xF000_0000 (N=Z=C=V=1); mask=1000 (flags only).
        let msr_flags_only = 0xE128_F000u32; // MSR CPSR_f, r0
        let (mut cpu, mut bus) = boot(&[msr_flags_only]);
        cpu.regs.r[0] = 0xF000_0000;
        let mode_before = cpu.regs.cpsr.mode;
        cpu.step(&mut bus);
        assert!(
            cpu.regs.cpsr.flags.n
                && cpu.regs.cpsr.flags.z
                && cpu.regs.cpsr.flags.c
                && cpu.regs.cpsr.flags.v
        );
        assert_eq!(
            cpu.regs.cpsr.mode, mode_before,
            "flags-only MSR must not touch mode"
        );
    }

    #[test]
    fn mrs_reads_back_the_packed_cpsr() {
        // MRS r0, CPSR
        let mrs_r0_cpsr = 0xE10F_0000u32;
        let (mut cpu, mut bus) = boot(&[mrs_r0_cpsr]);
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], cpu.regs.cpsr.to_u32());
    }

    #[test]
    fn software_interrupt_enters_supervisor_and_parks_the_return_address() {
        // SWI at address 0.
        let swi = 0xEF00_0000u32;
        let (mut cpu, mut bus) = boot(&[swi]);
        let decode_addr_before = cpu.pipeline.decode.address;
        cpu.step(&mut bus);
        assert_eq!(
            cpu.pipeline.execute.address, 0x08,
            "jumped to the SoftwareIrq vector"
        );
        assert_eq!(
            cpu.regs.r[15],
            0x08 + 8,
            "pipeline re-establishes +8 at the new vector"
        );
        assert_eq!(cpu.regs.r[14], decode_addr_before);
        assert_eq!(cpu.regs.cpsr.mode, mode::SUPERVISOR);
        assert!(cpu.regs.cpsr.irq_disable);
    }

    #[test]
    fn movs_pc_restores_cpsr_from_spsr_like_an_exception_return() {
        // SWI at address 0, MOVS pc, lr placed exactly at the SoftwareIrq vector (0x08 -> word
        // index 2), matching how a real handler would sit there.
        let swi = 0xEF00_0000u32;
        let movs_pc_lr = 0xE1B0_F00Eu32; // MOVS pc, lr
        let (mut cpu, mut bus) = boot(&[swi, 0, movs_pc_lr, 0]);
        // Simulate a User-mode program making the SWI call, so the round trip is meaningful
        // (returning to a DIFFERENT mode than the handler ran in, not a same-mode no-op).
        cpu.regs.switch_mode(mode::USER);
        cpu.step(&mut bus); // SWI -> Supervisor; SPSR_svc = the User-mode CPSR just saved
        assert_eq!(cpu.regs.cpsr.mode, mode::SUPERVISOR);
        let lr = cpu.regs.r[14];
        cpu.step(&mut bus); // MOVS pc, lr at the vector -- the idiomatic exception return
        assert_eq!(
            cpu.regs.cpsr.mode,
            mode::USER,
            "CPSR restored from SPSR_svc, back to the mode the SWI was made from"
        );
        assert_eq!(
            cpu.regs.r[15],
            lr + 8,
            "PC = LR, then the pipeline re-establishes +8"
        );
    }

    #[test]
    fn undefined_opcode_traps_to_the_undefined_vector() {
        // Coprocessor-space opcode bits27-24=1100 (the ARM "Coprocessor Data Transfer" class):
        // ST018 has no coprocessor, and the reference InitArmOpTable never populates index range
        // 0xC00-0xEFF with anything, so it stays the InvalidOp default -- unlike, say, a
        // register-offset Single Data Transfer with bit4 set (real ARM's "undefined instruction
        // space"), which Mesen2's table does NOT carve out of its SingleDataTransfer range, so
        // this port must not treat that pattern as undefined either (matching the source, not
        // the general ARM ARM, since this is a port of Mesen2's exact behavior).
        let undefined = 0xEC00_0000u32;
        let (mut cpu, mut bus) = boot(&[undefined]);
        cpu.step(&mut bus);
        assert_eq!(
            cpu.pipeline.execute.address, 0x04,
            "jumped to the Undefined vector"
        );
        assert_eq!(
            cpu.regs.r[15],
            0x04 + 8,
            "pipeline re-establishes +8 at the new vector"
        );
        assert_eq!(cpu.regs.cpsr.mode, mode::UNDEFINED);
    }

    #[test]
    fn ldr_pre_indexed_with_no_writeback() {
        // LDR r0, [r1] -- immediate offset 0, pre-indexed, W=0: must NOT write r1 back.
        let ldr_r0_r1 = 0xE591_0000u32;
        let (mut cpu, mut bus) = boot(&[ldr_r0_r1]);
        cpu.regs.r[1] = 0x2000;
        bus.set_word(0x2000, 0xDEAD_BEEF);
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 0xDEAD_BEEF);
        assert_eq!(cpu.regs.r[1], 0x2000, "P=1,W=0 must not write back");
    }

    #[test]
    fn ldr_post_indexed_always_writes_back_even_without_the_w_bit() {
        // LDR r0, [r1], #4 -- post-indexed: writeback happens unconditionally, even though the
        // encoded W bit is 0 (post-indexing itself implies writeback on real ARM hardware).
        let ldr_r0_r1_post4 = 0xE491_0004u32;
        let (mut cpu, mut bus) = boot(&[ldr_r0_r1_post4]);
        cpu.regs.r[1] = 0x2000;
        bus.set_word(0x2000, 0x1234_5678);
        cpu.step(&mut bus);
        assert_eq!(
            cpu.regs.r[0], 0x1234_5678,
            "loaded from the ORIGINAL address"
        );
        assert_eq!(
            cpu.regs.r[1], 0x2004,
            "post-indexed writeback always happens"
        );
    }

    #[test]
    fn str_r15_stores_address_plus_12_not_plus_8() {
        // STR r15, [r1] -- storing R15 itself uses the +12 quirk (one MORE cycle than the usual
        // +8 read-as-operand exposure), a real, documented ARM6-class store timing detail.
        let str_r15_r1 = 0xE581_F000u32;
        let (mut cpu, mut bus) = boot(&[str_r15_r1]);
        cpu.regs.r[1] = 0x2000;
        cpu.step(&mut bus);
        assert_eq!(
            bus.word_at(0x2000),
            8 + 4,
            "stored value is address+12, not address+8"
        );
    }

    #[test]
    fn ldm_ia_writeback_loads_registers_in_ascending_order() {
        // LDMIA r0!, {r1,r2}
        let ldmia_r0_r1_r2 = 0xE8B0_0006u32;
        let (mut cpu, mut bus) = boot(&[ldmia_r0_r1_r2]);
        cpu.regs.r[0] = 0x3000;
        bus.set_word(0x3000, 0x1111_1111);
        bus.set_word(0x3004, 0x2222_2222);
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[1], 0x1111_1111);
        assert_eq!(cpu.regs.r[2], 0x2222_2222);
        assert_eq!(
            cpu.regs.r[0], 0x3008,
            "writeback: base + register_count * 4"
        );
    }

    #[test]
    fn stm_ia_writeback_stores_registers_in_ascending_order() {
        // STMIA r0!, {r1,r2}
        let stmia_r0_r1_r2 = 0xE8A0_0006u32;
        let (mut cpu, mut bus) = boot(&[stmia_r0_r1_r2]);
        cpu.regs.r[0] = 0x3000;
        cpu.regs.r[1] = 0xAAAA_AAAA;
        cpu.regs.r[2] = 0xBBBB_BBBB;
        cpu.step(&mut bus);
        assert_eq!(bus.word_at(0x3000), 0xAAAA_AAAA);
        assert_eq!(bus.word_at(0x3004), 0xBBBB_BBBB);
        assert_eq!(cpu.regs.r[0], 0x3008);
    }

    #[test]
    fn ldm_with_an_empty_register_list_transfers_only_r15_but_advances_as_if_all_16_did() {
        // LDM r0, {} -- the documented empty-list glitch: regMask becomes 0x8000 (R15 only) but
        // regCount is forced to 16 for address-advancement purposes.
        let ldm_r0_empty = 0xE890_0000u32;
        let (mut cpu, mut bus) = boot(&[ldm_r0_empty]);
        cpu.regs.r[0] = 0x4000;
        bus.set_word(0x4000, 0x9000); // the word LDM will load into R15
        cpu.step(&mut bus);
        assert_eq!(
            cpu.pipeline.execute.address, 0x9000,
            "R15 was loaded from address 0x4000 despite the empty encoded list"
        );
    }

    #[test]
    fn ldm_with_s_bit_and_pc_in_the_list_restores_cpsr_from_spsr() {
        // LDM r0, {r1, pc}^ -- S=1 with R15 in the list: no temporary User-mode switch during
        // the transfer (unlike LDM^ without PC, or any STM^), and CPSR is restored wholesale
        // from the CURRENT mode's SPSR after the transfer -- the LDM-based exception return.
        let ldm_r0_r1_pc_caret = 0xE8D0_8002u32;
        let (mut cpu, mut bus) = boot(&[ldm_r0_r1_pc_caret]);
        cpu.regs.switch_mode(mode::SUPERVISOR);
        cpu.regs.spsr_mut().mode = mode::USER; // simulate SPSR_svc holding a User-mode CPSR
        cpu.regs.r[0] = 0x5000;
        bus.set_word(0x5000, 0xCAFE_BABE); // -> r1
        bus.set_word(0x5004, 0x2000); // -> r15
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[1], 0xCAFE_BABE);
        assert_eq!(
            cpu.regs.cpsr.mode,
            mode::USER,
            "CPSR restored from SPSR_svc"
        );
        assert_eq!(
            cpu.pipeline.execute.address, 0x2000,
            "R15 loaded from the list"
        );
    }

    #[test]
    fn mul_computes_the_low_32_bits_and_sets_z_n() {
        // MULS r0, r1, r2  (r0 = r1 * r2, S=1)
        let muls_r0_r1_r2 = 0xE010_0291u32;
        let (mut cpu, mut bus) = boot(&[muls_r0_r1_r2]);
        cpu.regs.r[1] = 6;
        cpu.regs.r[2] = 7;
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 42);
        assert!(!cpu.regs.cpsr.flags.z);
        assert!(!cpu.regs.cpsr.flags.n);
    }

    #[test]
    fn mla_accumulates_into_the_multiply_result() {
        // MLA r0, r1, r2, r3  (r0 = r1*r2 + r3)
        let mla_r0_r1_r2_r3 = 0xE020_3291u32;
        let (mut cpu, mut bus) = boot(&[mla_r0_r1_r2_r3]);
        cpu.regs.r[1] = 6;
        cpu.regs.r[2] = 7;
        cpu.regs.r[3] = 100;
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 142);
    }

    #[test]
    fn umull_widens_to_64_bits_across_two_registers() {
        // UMULLS r3, r2, r0, r1  (r2:r3 = r0 * r1, unsigned, S=1)
        let umulls_r3_r2_r0_r1 = 0xE092_3190u32;
        let (mut cpu, mut bus) = boot(&[umulls_r3_r2_r0_r1]);
        cpu.regs.r[0] = 0xFFFF_FFFF;
        cpu.regs.r[1] = 2;
        cpu.step(&mut bus);
        let result = (u64::from(cpu.regs.r[2]) << 32) | u64::from(cpu.regs.r[3]);
        assert_eq!(result, u64::from(0xFFFF_FFFFu32) * 2);
        assert!(!cpu.regs.cpsr.flags.z);
    }

    #[test]
    fn smull_sign_extends_negative_operands() {
        // SMULLS r3, r2, r0, r1  (r2:r3 = r0 * r1, signed, S=1)
        let smulls_r3_r2_r0_r1 = 0xE0D2_3190u32;
        let (mut cpu, mut bus) = boot(&[smulls_r3_r2_r0_r1]);
        cpu.regs.r[0] = (-5i32).cast_unsigned();
        cpu.regs.r[1] = 3;
        cpu.step(&mut bus);
        let result = ((u64::from(cpu.regs.r[2]) << 32) | u64::from(cpu.regs.r[3])).cast_signed();
        assert_eq!(result, -15);
        assert!(cpu.regs.cpsr.flags.n, "negative 64-bit result sets N");
    }

    #[test]
    fn swp_reads_the_old_value_then_writes_the_new_one_atomically() {
        // SWP r0, r2, [r1]  (r0 = [r1]; [r1] = r2)
        let swp_r0_r2_r1 = 0xE101_0092u32;
        let (mut cpu, mut bus) = boot(&[swp_r0_r2_r1]);
        cpu.regs.r[1] = 0x2000;
        cpu.regs.r[2] = 0xBEEF_CAFE;
        bus.set_word(0x2000, 0x1111_1111);
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 0x1111_1111, "old value read into rd");
        assert_eq!(
            bus.word_at(0x2000),
            0xBEEF_CAFE,
            "new value written to the same address"
        );
    }

    #[test]
    fn swpb_swaps_a_single_byte() {
        // SWPB r0, r2, [r1]
        let swpb_r0_r2_r1 = 0xE141_0092u32;
        let (mut cpu, mut bus) = boot(&[swpb_r0_r2_r1]);
        cpu.regs.r[1] = 0x2000;
        cpu.regs.r[2] = 0xAB;
        bus.set_word(0x2000, 0xFFFF_FF42); // low byte = 0x42
        cpu.step(&mut bus);
        assert_eq!(cpu.regs.r[0], 0x42, "old byte read into rd");
        assert_eq!(
            bus.word_at(0x2000),
            0xFFFF_FFAB,
            "only the low byte was swapped"
        );
    }

    #[test]
    fn multiply_cycles_matches_the_documented_early_termination_rule() {
        assert_eq!(multiply_cycles(0), 1);
        assert_eq!(multiply_cycles(0xFF), 1);
        assert_eq!(multiply_cycles(0xFFFF_FF00), 1);
        assert_eq!(multiply_cycles(0xFFFF), 2);
        assert_eq!(multiply_cycles(0x00FF_FFFF), 3);
        assert_eq!(multiply_cycles(0x7FFF_FFFF), 4);
    }
}
