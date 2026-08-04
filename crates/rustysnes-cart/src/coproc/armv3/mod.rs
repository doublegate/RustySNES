//! The ARMv3 (ARM6-class, pre-Thumb) CPU core — ST018's LLE engine.
//!
//! ST018 is Hayazashi Nidan Morita Shogi 2's coprocessor; see [`board`]'s doc for the detection
//! research — an earlier version of this doc wrongly attributed this chip to Star Ocean, which
//! uses S-DD1 only, no ARM chip.
//!
//! **Implemented from the published ARM architecture definition** — the ARMv3/ARM6 instruction
//! set (data processing, branch, PSR transfer, single/block data transfer, multiply, swap), its
//! register banking, and its 3-stage prefetch pipeline are an architecture, documented by ARM and
//! by decades of public reference material. Every instruction's semantics and the PC+8 pipeline
//! rule are written from that documentation; the working notes are `docs/st018-arm-notes.md`.
//!
//! **Reference emulators were used as behavioural oracles, not as source. No third-party emulator
//! code is incorporated.** The scope decision was to model the ST018's actual part — an
//! ARMv3/ARM6-class chip that predates Thumb — rather than the ARM7TDMI superset that ares'
//! `armdsp` reuses; Mesen2 makes the same scoping choice, and its behaviour was the primary
//! cross-check while this core was built. **Mesen2 is GPLv3** (an earlier revision of this comment
//! stated "MIT", which was simply wrong), so it can only ever be an oracle here — and that is what
//! it was.
//!
//! One consequence of oracle use is recorded honestly in [`cpu`]'s undefined-opcode test: where the
//! ARM architecture reserves an "undefined instruction" encoding that the ST018's decode does not
//! appear to trap, this core follows the *observed* behaviour rather than the architectural
//! generality. That is a fact about the chip as best it can be observed, adopted from
//! cross-checking — not a transcription of anyone's decode table.
//!
//! Full architecture notes (register banking, the pipeline's PC+8 timing, every instruction's
//! documented hardware quirks, the board bus protocol) live in `docs/st018-arm-notes.md`, kept in
//! sync with this module as it's built out.
//!
//! Built bottom-up, in the order `docs/st018-arm-notes.md` lays out:
//! 1. [`primitives`] — the barrel shifter, condition codes, ALU core (pure functions, no state).
//! 2. [`regs`] — the register file, mode-switch banking, and the 3-stage pipeline model.
//! 3. [`bus`] + [`cpu`] — the full instruction set: data processing, branch, MSR/MRS, exception
//!    entry, `LDR`/`STR`, `LDM`/`STM`, multiply/multiply-long, and `SWP`/`SWPB`.
//! 4. [`board`] — the SNES-side board wrapper, wired into `board::select`.

pub mod board;
pub mod bus;
pub mod cpu;
pub mod primitives;
pub mod regs;

pub use board::St018Board;
pub use bus::ArmBus;
pub use cpu::Cpu;
pub use primitives::{
    Flags, add, check_condition, logical_flags, rotate_right, rotate_right_carry, shift_asr,
    shift_lsl, shift_lsr, shift_ror, shift_rrx, sub,
};
pub use regs::{Cpsr, Mode, Pipeline, Regs};
