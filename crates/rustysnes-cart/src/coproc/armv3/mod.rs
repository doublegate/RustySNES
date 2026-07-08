//! The ARMv3 (ARM6-class, pre-Thumb) CPU core — ST018's LLE engine (Star Ocean).
//!
//! Clean-room port of Mesen2's `ArmV3Cpu` (MIT, `Core/SNES/Coprocessors/ST018/ArmV3Cpu.cpp`) —
//! chosen over ares' `sfc/coprocessor/armdsp`, which instead reuses ares' generic shared
//! `component/processor/arm7tdmi` (a full ARM+Thumb ARM7TDMI superset the real ST018 chip, an
//! ARMv3/ARM6-class part that predates Thumb, never needed). Mesen2's dedicated `ArmV3Cpu` is the
//! more faithful, more focused scope. Full architecture notes (register banking, the pipeline's
//! PC+8 timing, every instruction's documented hardware quirks, the board bus protocol) live in
//! `docs/st018-arm-notes.md`, kept in sync with this module as it's built out.
//!
//! Built bottom-up, in the order `docs/st018-arm-notes.md` lays out:
//! 1. [`primitives`] — the barrel shifter, condition codes, ALU core (pure functions, no state).
//! 2. [`regs`] — the register file, mode-switch banking, and the 3-stage pipeline model.
//! 3. Instruction decode/execute (in progress).
//! 4. The `ST018` board wrapper (not yet started; not reachable from `board::select`).

pub mod primitives;
pub mod regs;

pub use primitives::{
    Flags, add, check_condition, logical_flags, rotate_right, rotate_right_carry, shift_asr,
    shift_lsl, shift_lsr, shift_ror, shift_rrx, sub,
};
pub use regs::{Cpsr, Mode, Pipeline, Regs};
