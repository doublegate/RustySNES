//! The ARMv3 (ARM6-class, pre-Thumb) CPU core — ST018's LLE engine (Star Ocean).
//!
//! **Status: foundation only.** This module currently ports the pure, state-free primitives every
//! ARM instruction is built from — the barrel shifter, the condition-code checker, and the
//! Add/Sub/logical-op ALU core — each verified against the ARM Architecture Reference Manual's own
//! documented truth tables. It does NOT yet implement instruction decode, the register file, mode
//! banking, the 3-stage pipeline, or any board wiring; `ST018` is not yet reachable from
//! `board::select`. See the `st018-armv3-scoping` session memory for the full architecture notes
//! and the suggested build order this module follows (barrel shifter + condition codes + ALU core
//! first, deliberately, since they're testable in complete isolation from the pipeline-timing
//! complexity that the rest of the core depends on getting exactly right).
//!
//! Clean-room port of Mesen2's `ArmV3Cpu` (MIT, `Core/SNES/Coprocessors/ST018/ArmV3Cpu.cpp`) —
//! chosen over ares' `sfc/coprocessor/armdsp`, which instead reuses ares' generic shared
//! `component/processor/arm7tdmi` (a full ARM+Thumb ARM7TDMI superset the real ST018 chip, an
//! ARMv3/ARM6-class part that predates Thumb, never needed). Mesen2's dedicated `ArmV3Cpu` is the
//! more faithful, more focused scope.

// Chip-name jargon (ARMv3, CPSR, SPSR, ...) is not Rust code. `Flags` is a direct port of the
// architectural N/Z/C/V condition-code register — four independent hardware bits, not a
// state-machine candidate for an enum, so `struct_excessive_bools` is noise here.
#![allow(clippy::doc_markdown, clippy::struct_excessive_bools)]

/// The four ARM condition-code flags (CPSR bits 31-28 / N,Z,C,V).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// Negative (bit 31 of the last flag-setting result).
    pub n: bool,
    /// Zero (the last flag-setting result was all-zero).
    pub z: bool,
    /// Carry (carry-out of the last flag-setting add/subtract/shift).
    pub c: bool,
    /// Overflow (signed overflow of the last flag-setting add/subtract).
    pub v: bool,
}

/// Evaluate a 4-bit ARM condition code against the current flags (Mesen2 `CheckConditions`).
///
/// `cond` is the top 4 bits of every ARM instruction word (`opcode >> 28`); only `0..=15` are
/// meaningful (`15` = NV, reserved/never on this architecture generation — ported as `false`
/// verbatim, matching the source rather than guessing at later-ARM NV semantics).
#[must_use]
pub const fn check_condition(cond: u8, f: Flags) -> bool {
    match cond & 0xF {
        0 => f.z,                   // EQ
        1 => !f.z,                  // NE
        2 => f.c,                   // CS/HS
        3 => !f.c,                  // CC/LO
        4 => f.n,                   // MI
        5 => !f.n,                  // PL
        6 => f.v,                   // VS
        7 => !f.v,                  // VC
        8 => f.c && !f.z,           // HI
        9 => !f.c || f.z,           // LS
        10 => f.n == f.v,           // GE
        11 => f.n != f.v,           // LT
        12 => !f.z && (f.n == f.v), // GT
        13 => f.z || (f.n != f.v),  // LE
        14 => true,                 // AL
        _ => false,                 // NV (15) — reserved, never taken
    }
}

/// ARM `ADD`-family: `op1 + op2 + carry_in`.
///
/// Uses the exact overflow/carry formulas the ARM ARM specifies (Mesen2 `Add`) — NOT
/// reimplemented from first principles, since the signed-overflow and carry-out derivations
/// below are the well-known highest-bug-density spot in a from-scratch ARM core. Returns
/// `(result, flags_if_update_requested)`; the caller decides whether to commit the returned
/// flags (mirrors the `updateFlags`/`S`-bit gate every ARM ALU instruction carries).
#[must_use]
pub const fn add(op1: u32, op2: u32, carry_in: bool, prior: Flags) -> (u32, Flags) {
    let result = op1.wrapping_add(op2).wrapping_add(carry_in as u32);
    let overflow = (!(op1 ^ op2) & (op1 ^ result)) & 0x8000_0000 != 0;
    let carry = (op1 ^ op2 ^ (overflow as u32).wrapping_shl(31) ^ result) & 0x8000_0000 != 0;
    let flags = Flags {
        n: result & 0x8000_0000 != 0,
        z: result == 0,
        c: carry,
        v: overflow,
    };
    let _ = prior;
    (result, flags)
}

/// ARM `SUB`-family: `Add(op1, !op2, carry_in, ...)`.
///
/// ARM's subtract IS add-with-inverted-operand-and-carry, not an independently-implemented
/// subtraction (Mesen2 `Sub`); porting it as a direct call to [`add`], not a separate formula,
/// is deliberate — the two must never drift.
#[must_use]
pub const fn sub(op1: u32, op2: u32, carry_in: bool, prior: Flags) -> (u32, Flags) {
    add(op1, !op2, carry_in, prior)
}

/// ARM logical-op flag update (`AND`/`EOR`/`ORR`/`MOV`/`BIC`/`MVN`/`TST`/`TEQ`, Mesen2 `LogicalOp`).
///
/// `V` is left UNAFFECTED (only `ADD`/`SUB`-family ops touch it); `C` becomes the barrel
/// shifter's carry-out (or is preserved when the shift was `LSL #0` — the caller passes the
/// shifter's carry-out unconditionally, which is already correct in that case since `LSL #0`
/// returns the flags' existing carry unchanged, see [`shift_lsl`]).
#[must_use]
pub const fn logical_flags(result: u32, shifter_carry: bool, prior: Flags) -> Flags {
    Flags {
        n: result & 0x8000_0000 != 0,
        z: result == 0,
        c: shifter_carry,
        v: prior.v,
    }
}

/// `ROR` by a fixed 1-31 amount with no carry-out tracking (Mesen2's 2-argument `RotateRight`).
///
/// Used by `MSR`'s immediate-operand rotate. `shift` must be `1..=31`; the immediate encodings
/// that call this always derive it as `(nibble) * 2` from a nonzero nibble, so it's never 0.
#[must_use]
pub const fn rotate_right(value: u32, shift: u32) -> u32 {
    value.rotate_right(shift)
}

/// `ROR` by a fixed 1-31 amount, also returning the carry-out (bit `shift-1` of `value`) — Mesen2's
/// 3-argument `RotateRight`, used by `ArmDataProcessing`'s immediate-operand rotate.
#[must_use]
pub const fn rotate_right_carry(value: u32, shift: u32) -> (u32, bool) {
    let carry = (value >> (shift - 1)) & 1 != 0;
    (rotate_right(value, shift), carry)
}

/// `LSL` (logical shift left) by a register-derived amount `0..=255` (Mesen2 `ShiftLsl`).
///
/// `shift == 0` is a documented ARM no-op: both `value` and `carry` pass through UNCHANGED (the
/// existing carry flag is preserved, not recomputed) — every ARM shift function shares this
/// "shift 0 changes nothing" contract, ported here via the same `if shift != 0` guard the source
/// uses rather than folding it into the arithmetic (shifting a `u32` by literally 32 or more is
/// disallowed in Rust — every branch below is guarded to never execute one).
#[must_use]
pub const fn shift_lsl(value: u32, shift: u8, carry: bool) -> (u32, bool) {
    if shift == 0 {
        return (value, carry);
    }
    let carry = if shift < 33 {
        value & (1u32 << (32 - shift as u32)) != 0
    } else {
        false
    };
    let value = if shift < 32 { value << shift } else { 0 };
    (value, carry)
}

/// `LSR` (logical shift right) by a register-derived amount `0..=255` (Mesen2 `ShiftLsr`). See
/// [`shift_lsl`] for the shared `shift == 0` no-op contract.
#[must_use]
pub const fn shift_lsr(value: u32, shift: u8, carry: bool) -> (u32, bool) {
    if shift == 0 {
        return (value, carry);
    }
    let carry = if shift < 33 {
        value & (1u32 << (shift as u32 - 1)) != 0
    } else {
        false
    };
    let value = if shift < 32 { value >> shift } else { 0 };
    (value, carry)
}

/// `ASR` (arithmetic shift right, sign-extending) by a register-derived amount `0..=255`.
///
/// (Mesen2 `ShiftAsr`.) For `shift >= 32` the result is the sign bit smeared across all 32 bits
/// (an ASR by 31 of the original value achieves this — never a literal `>> 32`); carry-out is
/// the sign bit itself in that case. See [`shift_lsl`] for the shared `shift == 0` no-op contract.
#[must_use]
pub const fn shift_asr(value: u32, shift: u8, carry: bool) -> (u32, bool) {
    if shift == 0 {
        return (value, carry);
    }
    let sign = value & 0x8000_0000 != 0;
    let carry = if shift < 33 {
        value & (1u32 << (shift as u32 - 1)) != 0
    } else {
        sign
    };
    let value = if shift < 32 {
        (value.cast_signed() >> shift).cast_unsigned()
    } else {
        (value.cast_signed() >> 31).cast_unsigned()
    };
    (value, carry)
}

/// `ROR` (rotate right) by a register-derived amount `0..=255` (Mesen2 `ShiftRor`).
///
/// The rotate amount is first reduced mod 32 (`shift & 0x1F`); if that reduction is itself 0
/// (i.e. the original amount was a nonzero multiple of 32), `value` is left UNCHANGED but carry
/// still becomes bit 31 of `value` — a real, easy-to-miss ARM ARM special case, ported exactly
/// as Mesen2 encodes it (the inner `if shift != 0` only guards the rotate, not the carry
/// update). See [`shift_lsl`] for the shared outer `shift == 0` no-op contract (the ORIGINAL,
/// pre-mask amount — distinct from the inner post-mask check).
#[must_use]
pub const fn shift_ror(value: u32, shift: u8, carry: bool) -> (u32, bool) {
    if shift == 0 {
        return (value, carry);
    }
    let masked = shift & 0x1F;
    let value = if masked == 0 {
        value
    } else {
        rotate_right(value, masked as u32)
    };
    let carry = value & 0x8000_0000 != 0;
    (value, carry)
}

/// `RRX` (rotate right extended by 1, through the carry flag; Mesen2 `ShiftRrx`).
///
/// The immediate-operand encoding `ROR #0` is repurposed to mean this: the incoming carry
/// becomes bit 31 of the result, and the outgoing carry becomes the value's own bit 0.
#[must_use]
pub const fn shift_rrx(value: u32, carry_in: bool) -> (u32, bool) {
    let carry_out = value & 1 != 0;
    let result = (value >> 1) | ((carry_in as u32) << 31);
    (result, carry_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Positional booleans read naturally at every call site below (`f(N, Z, C, V)`, matching the
    // ARM ARM's own NZCV ordering) — a struct-literal helper would be noisier for a test-only fn.
    #[allow(clippy::fn_params_excessive_bools)]
    fn f(n: bool, z: bool, c: bool, v: bool) -> Flags {
        Flags { n, z, c, v }
    }

    #[test]
    fn condition_codes_match_the_arm_arm_truth_table() {
        let nzcv = f(true, true, true, true);
        let clear = f(false, false, false, false);
        assert!(check_condition(0, f(false, true, false, false))); // EQ, Z set
        assert!(!check_condition(0, clear)); // EQ, Z clear
        assert!(check_condition(1, clear)); // NE, Z clear
        assert!(check_condition(2, f(false, false, true, false))); // CS, C set
        assert!(check_condition(3, clear)); // CC, C clear
        assert!(check_condition(4, f(true, false, false, false))); // MI, N set
        assert!(check_condition(5, clear)); // PL, N clear
        assert!(check_condition(6, f(false, false, false, true))); // VS, V set
        assert!(check_condition(7, clear)); // VC, V clear
        assert!(check_condition(8, f(false, false, true, false))); // HI, C set && Z clear
        assert!(!check_condition(8, nzcv)); // HI fails when Z set even if C set
        assert!(check_condition(9, clear)); // LS, C clear
        assert!(check_condition(10, clear)); // GE, N==V (both false)
        assert!(check_condition(10, nzcv)); // GE, N==V (both true)
        assert!(check_condition(11, f(true, false, false, false))); // LT, N!=V
        assert!(check_condition(12, clear)); // GT, Z clear && N==V
        assert!(!check_condition(12, f(false, true, false, false))); // GT fails when Z set
        assert!(check_condition(13, f(false, true, false, false))); // LE, Z set
        assert!(check_condition(14, clear)); // AL always true
        assert!(!check_condition(15, nzcv)); // NV always false
    }

    #[test]
    fn add_carry_and_overflow_match_known_arm_cases() {
        // 0x7FFFFFFF + 1 = signed overflow (positive + positive -> negative), no unsigned carry.
        let (r, fl) = add(0x7FFF_FFFF, 1, false, Flags::default());
        assert_eq!(r, 0x8000_0000);
        assert!(fl.v, "signed overflow expected");
        assert!(!fl.c, "no unsigned carry expected");
        assert!(fl.n);
        assert!(!fl.z);

        // 0xFFFFFFFF + 1 = unsigned carry out, result 0, no signed overflow.
        let (r, fl) = add(0xFFFF_FFFF, 1, false, Flags::default());
        assert_eq!(r, 0);
        assert!(fl.c, "unsigned carry expected");
        assert!(!fl.v, "no signed overflow expected");
        assert!(fl.z);

        // Carry-in propagates like a real add-with-carry.
        let (r, _) = add(1, 1, true, Flags::default());
        assert_eq!(r, 3);
    }

    #[test]
    fn sub_is_add_with_inverted_operand_and_carry() {
        // ARM SUB passes carry=true for a plain subtract with no borrow-in (SBC uses the real C).
        let (r, fl) = sub(5, 3, true, Flags::default());
        assert_eq!(r, 2);
        assert!(fl.c, "no borrow: 5 >= 3");

        // 0 - 1 borrows: unsigned carry clear, result wraps to 0xFFFFFFFF.
        let (r, fl) = sub(0, 1, true, Flags::default());
        assert_eq!(r, 0xFFFF_FFFF);
        assert!(!fl.c, "borrow occurred");
    }

    #[test]
    fn logical_flags_leaves_overflow_untouched_and_uses_shifter_carry() {
        let prior = f(false, false, false, true); // V already set
        let fl = logical_flags(0x8000_0000, true, prior);
        assert!(fl.n);
        assert!(!fl.z);
        assert!(fl.c);
        assert!(fl.v, "V must be preserved, not recomputed, by a logical op");
    }

    #[test]
    fn shift_by_zero_is_a_true_no_op_on_every_shifter() {
        for carry in [true, false] {
            assert_eq!(shift_lsl(0x1234, 0, carry), (0x1234, carry));
            assert_eq!(shift_lsr(0x1234, 0, carry), (0x1234, carry));
            assert_eq!(shift_asr(0x1234, 0, carry), (0x1234, carry));
            assert_eq!(shift_ror(0x1234, 0, carry), (0x1234, carry));
        }
    }

    #[test]
    fn lsl_boundary_cases_32_and_beyond() {
        // LSL #1: 0x8000_0000 -> 0, carry = old bit 31.
        assert_eq!(shift_lsl(0x8000_0000, 1, false), (0, true));
        // LSL #32: result 0, carry = bit 0 of the original value.
        assert_eq!(shift_lsl(1, 32, false), (0, true));
        assert_eq!(shift_lsl(2, 32, false), (0, false));
        // LSL #33 (and beyond): result 0, carry 0.
        assert_eq!(shift_lsl(0xFFFF_FFFF, 33, false), (0, false));
    }

    #[test]
    fn lsr_boundary_cases_32_and_beyond() {
        // LSR #1: bit 0 -> carry.
        assert_eq!(shift_lsr(1, 1, false), (0, true));
        // LSR #32: result 0, carry = bit 31 of the original value.
        assert_eq!(shift_lsr(0x8000_0000, 32, false), (0, true));
        assert_eq!(shift_lsr(0x7FFF_FFFF, 32, false), (0, false));
        // LSR #33+: result 0, carry 0.
        assert_eq!(shift_lsr(0xFFFF_FFFF, 33, false), (0, false));
    }

    #[test]
    fn asr_sign_extends_and_boundary_cases_saturate_to_the_sign_bit() {
        // ASR #1 of a negative value sign-extends (top bit stays set) and carries out bit 0.
        assert_eq!(shift_asr(0x8000_0001, 1, false), (0xC000_0000, true));
        // ASR #32+ of a negative value: all 1s, carry = sign bit (1).
        assert_eq!(shift_asr(0x8000_0000, 32, false), (0xFFFF_FFFF, true));
        assert_eq!(shift_asr(0x8000_0000, 40, false), (0xFFFF_FFFF, true));
        // ASR #32+ of a positive value: all 0s, carry = sign bit (0).
        assert_eq!(shift_asr(0x7FFF_FFFF, 32, false), (0, false));
    }

    #[test]
    fn ror_by_a_multiple_of_32_leaves_value_unchanged_but_still_updates_carry() {
        // shift=32 masks to 0: value unchanged, carry = bit 31 of the (unchanged) value.
        assert_eq!(shift_ror(0x8000_0001, 32, false), (0x8000_0001, true));
        assert_eq!(shift_ror(0x0000_0001, 32, false), (0x0000_0001, false));
    }

    #[test]
    fn ror_ordinary_rotation() {
        // ROR #1 of a value with bit 0 set: that bit moves to bit 31, becomes the new carry too.
        assert_eq!(shift_ror(0x0000_0001, 1, false), (0x8000_0000, true));
        // ROR #4 of 0x1 -> 0x1000_0000, no carry (bit 31 clear).
        assert_eq!(shift_ror(0x0000_0001, 4, false), (0x1000_0000, false));
    }

    #[test]
    fn rrx_rotates_through_the_carry_flag() {
        // RRX with carry-in=1: bit 31 of the result becomes 1; bit 0 of value becomes carry-out.
        assert_eq!(shift_rrx(0x0000_0000, true), (0x8000_0000, false));
        assert_eq!(shift_rrx(0x0000_0001, false), (0x0000_0000, true));
        assert_eq!(shift_rrx(0x8000_0001, true), (0xC000_0000, true));
    }

    #[test]
    fn rotate_right_matches_the_manual_bit_algebra() {
        assert_eq!(rotate_right(0x0000_0001, 1), 0x8000_0000);
        let (v, c) = rotate_right_carry(0x0000_0001, 1);
        assert_eq!(v, 0x8000_0000);
        assert!(c, "carry = bit(shift-1) = bit 0 of the original value");
    }
}
