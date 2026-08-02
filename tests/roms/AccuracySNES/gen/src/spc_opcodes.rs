//! The documented SPC700 opcode map — encoding, length, cycle count, and how each opcode can be
//! measured in place.
//!
//! # Where the numbers come from
//!
//! Every length and cycle count in this table is transcribed from **fullsnes**'s SPC700 instruction
//! set (`ref-docs/fullsnes/40-apu-dsp.md`, the Load/Store, ALU and Jump/Control sections). Nothing
//! here is read out of `crates/rustysnes-apu/`. That direction matters: `E2.10` is a *scored* row,
//! so its expectation has to come from a source outside the thing being tested, or the row is our
//! own arithmetic checked against itself — the `E9.11` failure mode the provenance tier exists to
//! prevent.
//!
//! # Why the table is built from rules rather than typed out
//!
//! The SPC700 map is regular, and fullsnes documents it *as* rules — `OR/AND/EOR/CMP/ADC/SBC` share
//! one operand column at `x + 04/05/06/…`, the shift and increment group shares another at
//! `x + 0B/0C/1B/1C`, and the bit ops are `b * 20 + 02/12/03/13`. Typing 256 lines out by hand
//! would introduce transcription errors the rules cannot have, and would hide the structure that
//! makes the map checkable. [`table`] asserts it filled all 256 slots exactly once, so a rule that
//! overlaps another or misses a slot fails the build rather than shipping a hole.
//!
//! # Straight-line measurability
//!
//! [`Measure`] records whether an opcode can be timed by executing copies of it back to back. Most
//! can. The ones that cannot are not a gap in the sweep — for them the question "how long does this
//! take in a straight line" has no answer, because they are the opcodes that end the straight line.
//! Each carries its reason.

/// How an opcode can be measured by executing copies of it back to back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Measure {
    /// Executes and falls through to the next instruction. Timeable directly.
    Straight,
    /// A relative branch. Timeable with a displacement of zero — the taken path lands on the next
    /// instruction, which is where a not-taken branch would have gone anyway, so a block of copies
    /// runs straight through whichever way each one goes. The recorded cost is the **taken** one,
    /// and [`Flag`] says which condition the arm has to arrange for that to be the path measured.
    BranchTaken(Flag),
    /// Not timeable in place, with the reason. These are the opcodes that leave the block: absolute
    /// jumps and calls (whose operand would have to differ for every copy), the vectored calls
    /// (whose vectors are in the IPL ROM), the returns (which need a stack the block did not push),
    /// and the two that halt the processor outright.
    NotStraightLine(&'static str),
}

/// The processor-status condition an arm must arrange so its branch is taken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flag {
    /// `N = 0`.
    NegClear,
    /// `N = 1`.
    NegSet,
    /// `V = 0`.
    OvfClear,
    /// `V = 1`.
    OvfSet,
    /// `C = 0`.
    CarryClear,
    /// `C = 1`.
    CarrySet,
    /// `Z = 0`.
    ZeroClear,
    /// `Z = 1`.
    ZeroSet,
    /// No flag needed — the branch is unconditional, or its condition is arranged by the driver's
    /// register and memory setup rather than by a flag (`CBNE`, `DBNZ`).
    None,
}

/// One opcode's documented shape.
#[derive(Clone, Copy, Debug)]
pub struct Op {
    /// The opcode byte.
    pub code: u8,
    /// Total instruction length in bytes, opcode included.
    pub len: u8,
    /// Documented cycle count. For a branch this is the **taken** cost, which is what
    /// [`Measure::BranchTaken`] arranges to measure.
    pub cycles: u8,
    /// How it can be timed.
    pub measure: Measure,
    /// Mnemonic, for the generated failure text and the coverage report.
    pub name: &'static str,
    /// What its operand bytes mean.
    ///
    /// Recorded when the entry is built rather than derived from the opcode byte afterwards. The
    /// map's low nibble *nearly* decides the addressing mode, and the first draft of this file
    /// derived it that way — but "nearly" hides at least four traps: `$x9` is `cmd aa,bb` in the
    /// ALU columns and an absolute or indexed `MOV` at `$C9`-`$F9`, and the `+X` variants sit one
    /// `$10` bit away from their unindexed twins in three separate columns. A derived rule that is
    /// wrong about one of them gives `MOV [aa+X],A` a pointer read out of uninitialised memory,
    /// which is zero, which is the sweep driver's own variables. The construction rules already
    /// know the answer; this field is them writing it down.
    pub operands: Operands,
}

/// Operand roles, so the sweep can fill each opcode's operand bytes with something safe.
///
/// "Safe" means three things at once, and every one of them has a way to ruin a measurement:
/// the operand must not name an I/O register (`$F0-$FF` — reading `$FD-$FF` clears the very
/// counters the sweep is reading, and most stores do a dummy read of their destination, so even a
/// *write* to `$FF` would clear `T2OUT`), it must not name the code block itself (an opcode that
/// rewrites the block changes what the remaining copies are), and it must stay in range once the
/// index registers are added.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Operands {
    /// No operand bytes.
    None,
    /// One direct-page address, used directly or as the low half of an indirect pointer
    /// (`[aa]+Y` reads its pointer from `aa`, so it belongs here).
    Dp,
    /// A direct-page address that an index register is added to before use — `aa+X`, `aa+Y`, and
    /// the `[aa+X]` indirect, whose *pointer* is at `aa+X`.
    ///
    /// It needs its own variant because the safe value is the opposite of [`Operands::Dp`]'s. A
    /// plain operand names the window directly; an indexed one has to name `window - X`, so that
    /// adding the index lands on the window rather than 48 bytes past it. Get this wrong on
    /// `MOV [aa+X],A` and the store goes through a pointer read from uninitialised memory — which
    /// is zero, which is the driver's own variables.
    DpIndexed,
    /// Two direct-page addresses, destination then source (`cmd aa,bb` is encoded `bb aa`).
    DpDp,
    /// An immediate byte.
    Imm,
    /// An immediate byte then a direct-page address (`cmd aa,#nn` is encoded `nn aa`).
    ImmDp,
    /// A 16-bit absolute address.
    Abs,
    /// A 13-bit absolute address with a 3-bit bit index in the top bits.
    AbsBit,
    /// A signed relative displacement.
    Rel,
    /// A direct-page address then a relative displacement.
    DpRel,
    /// A direct-page address that `X` is added to, then a relative displacement (`CBNE aa+X,rr`).
    DpIndexedRel,
}

/// The `x` bases of the six ALU operations, in fullsnes's `00+x` form.
const ALU: [(u8, &str); 6] = [
    (0x00, "OR"),
    (0x20, "AND"),
    (0x40, "EOR"),
    (0x60, "CMP"),
    (0x80, "ADC"),
    (0xA0, "SBC"),
];

/// The ALU operand column: `(low nibble offset, length, cycles, operands, suffix)`.
const ALU_FORMS: [(u8, u8, u8, Operands, &str); 12] = [
    (0x04, 2, 3, Operands::Dp, "A,aa"),
    (0x05, 3, 4, Operands::Abs, "A,!aaaa"),
    (0x06, 1, 3, Operands::None, "A,(X)"),
    (0x07, 2, 6, Operands::DpIndexed, "A,[aa+X]"),
    (0x08, 2, 2, Operands::Imm, "A,#nn"),
    (0x09, 3, 6, Operands::DpDp, "aa,bb"),
    (0x14, 2, 4, Operands::DpIndexed, "A,aa+X"),
    (0x15, 3, 5, Operands::Abs, "A,!aaaa+X"),
    (0x16, 3, 5, Operands::Abs, "A,!aaaa+Y"),
    (0x17, 2, 6, Operands::Dp, "A,[aa]+Y"),
    (0x18, 3, 5, Operands::ImmDp, "aa,#nn"),
    (0x19, 1, 5, Operands::None, "(X),(Y)"),
];

/// The `x` bases of the shift / increment group.
const SHIFT: [(u8, &str); 6] = [
    (0x00, "ASL"),
    (0x20, "ROL"),
    (0x40, "LSR"),
    (0x60, "ROR"),
    (0x80, "DEC"),
    (0xA0, "INC"),
];

/// The shift / increment operand column: `(low nibble offset, length, cycles, operands, suffix)`.
const SHIFT_FORMS: [(u8, u8, u8, Operands, &str); 4] = [
    (0x0B, 2, 4, Operands::Dp, "aa"),
    (0x0C, 3, 5, Operands::Abs, "!aaaa"),
    (0x1B, 2, 5, Operands::DpIndexed, "aa+X"),
    (0x1C, 1, 2, Operands::None, "A"),
];

/// Everything the two regular groups do not cover, spelled out.
///
/// `(code, len, cycles, measure, operands, name)`. Grouped in source order by fullsnes's own
/// headings so a
/// reader can check a run of them against one block of the reference at a time — which means
/// counting fields against that tuple, so it has to stay accurate.
const SINGLES: &[(u8, u8, u8, Measure, Operands, &str)] = &[
    // Register manipulation.
    (0xE8, 2, 2, Measure::Straight, Operands::Imm, "MOV A,#nn"),
    (0xCD, 2, 2, Measure::Straight, Operands::Imm, "MOV X,#nn"),
    (0x8D, 2, 2, Measure::Straight, Operands::Imm, "MOV Y,#nn"),
    (0x7D, 1, 2, Measure::Straight, Operands::None, "MOV A,X"),
    (0x5D, 1, 2, Measure::Straight, Operands::None, "MOV X,A"),
    (0xDD, 1, 2, Measure::Straight, Operands::None, "MOV A,Y"),
    (0xFD, 1, 2, Measure::Straight, Operands::None, "MOV Y,A"),
    (0x9D, 1, 2, Measure::Straight, Operands::None, "MOV X,SP"),
    (0xBD, 1, 2, Measure::Straight, Operands::None, "MOV SP,X"),
    // Memory load.
    (0xE4, 2, 3, Measure::Straight, Operands::Dp, "MOV A,aa"),
    (
        0xF4,
        2,
        4,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV A,aa+X",
    ),
    (0xE5, 3, 4, Measure::Straight, Operands::Abs, "MOV A,!aaaa"),
    (
        0xF5,
        3,
        5,
        Measure::Straight,
        Operands::Abs,
        "MOV A,!aaaa+X",
    ),
    (
        0xF6,
        3,
        5,
        Measure::Straight,
        Operands::Abs,
        "MOV A,!aaaa+Y",
    ),
    (0xE6, 1, 3, Measure::Straight, Operands::None, "MOV A,(X)"),
    (0xBF, 1, 4, Measure::Straight, Operands::None, "MOV A,(X)+"),
    (0xF7, 2, 6, Measure::Straight, Operands::Dp, "MOV A,[aa]+Y"),
    (
        0xE7,
        2,
        6,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV A,[aa+X]",
    ),
    (0xF8, 2, 3, Measure::Straight, Operands::Dp, "MOV X,aa"),
    (
        0xF9,
        2,
        4,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV X,aa+Y",
    ),
    (0xE9, 3, 4, Measure::Straight, Operands::Abs, "MOV X,!aaaa"),
    (0xEB, 2, 3, Measure::Straight, Operands::Dp, "MOV Y,aa"),
    (
        0xFB,
        2,
        4,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV Y,aa+X",
    ),
    (0xEC, 3, 4, Measure::Straight, Operands::Abs, "MOV Y,!aaaa"),
    (0xBA, 2, 5, Measure::Straight, Operands::Dp, "MOVW YA,aa"),
    // Memory store.
    (0x8F, 3, 5, Measure::Straight, Operands::ImmDp, "MOV aa,#nn"),
    (0xFA, 3, 5, Measure::Straight, Operands::DpDp, "MOV aa,bb"),
    (0xC4, 2, 4, Measure::Straight, Operands::Dp, "MOV aa,A"),
    (0xD8, 2, 4, Measure::Straight, Operands::Dp, "MOV aa,X"),
    (0xCB, 2, 4, Measure::Straight, Operands::Dp, "MOV aa,Y"),
    (
        0xD4,
        2,
        5,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV aa+X,A",
    ),
    (
        0xDB,
        2,
        5,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV aa+X,Y",
    ),
    (
        0xD9,
        2,
        5,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV aa+Y,X",
    ),
    (0xC5, 3, 5, Measure::Straight, Operands::Abs, "MOV !aaaa,A"),
    (0xC9, 3, 5, Measure::Straight, Operands::Abs, "MOV !aaaa,X"),
    (0xCC, 3, 5, Measure::Straight, Operands::Abs, "MOV !aaaa,Y"),
    (
        0xD5,
        3,
        6,
        Measure::Straight,
        Operands::Abs,
        "MOV !aaaa+X,A",
    ),
    (
        0xD6,
        3,
        6,
        Measure::Straight,
        Operands::Abs,
        "MOV !aaaa+Y,A",
    ),
    (0xAF, 1, 4, Measure::Straight, Operands::None, "MOV (X)+,A"),
    (0xC6, 1, 4, Measure::Straight, Operands::None, "MOV (X),A"),
    (0xD7, 2, 7, Measure::Straight, Operands::Dp, "MOV [aa]+Y,A"),
    (
        0xC7,
        2,
        7,
        Measure::Straight,
        Operands::DpIndexed,
        "MOV [aa+X],A",
    ),
    (0xDA, 2, 5, Measure::Straight, Operands::Dp, "MOVW aa,YA"),
    // Push / pop.
    (0x2D, 1, 4, Measure::Straight, Operands::None, "PUSH A"),
    (0x4D, 1, 4, Measure::Straight, Operands::None, "PUSH X"),
    (0x6D, 1, 4, Measure::Straight, Operands::None, "PUSH Y"),
    (0x0D, 1, 4, Measure::Straight, Operands::None, "PUSH PSW"),
    (0xAE, 1, 4, Measure::Straight, Operands::None, "POP A"),
    (0xCE, 1, 4, Measure::Straight, Operands::None, "POP X"),
    (0xEE, 1, 4, Measure::Straight, Operands::None, "POP Y"),
    (0x8E, 1, 4, Measure::Straight, Operands::None, "POP PSW"),
    // Compare with X / Y.
    (0xC8, 2, 2, Measure::Straight, Operands::Imm, "CMP X,#nn"),
    (0x3E, 2, 3, Measure::Straight, Operands::Dp, "CMP X,aa"),
    (0x1E, 3, 4, Measure::Straight, Operands::Abs, "CMP X,!aaaa"),
    (0xAD, 2, 2, Measure::Straight, Operands::Imm, "CMP Y,#nn"),
    (0x7E, 2, 3, Measure::Straight, Operands::Dp, "CMP Y,aa"),
    (0x5E, 3, 4, Measure::Straight, Operands::Abs, "CMP Y,!aaaa"),
    // Increment / decrement of X and Y, which sit outside the shift group's own column.
    (0x1D, 1, 2, Measure::Straight, Operands::None, "DEC X"),
    (0xDC, 1, 2, Measure::Straight, Operands::None, "DEC Y"),
    (0x3D, 1, 2, Measure::Straight, Operands::None, "INC X"),
    (0xFC, 1, 2, Measure::Straight, Operands::None, "INC Y"),
    // 16-bit ALU.
    (0x7A, 2, 5, Measure::Straight, Operands::Dp, "ADDW YA,aa"),
    (0x9A, 2, 5, Measure::Straight, Operands::Dp, "SUBW YA,aa"),
    (0x5A, 2, 4, Measure::Straight, Operands::Dp, "CMPW YA,aa"),
    (0x3A, 2, 6, Measure::Straight, Operands::Dp, "INCW aa"),
    (0x1A, 2, 6, Measure::Straight, Operands::Dp, "DECW aa"),
    (0x9E, 1, 12, Measure::Straight, Operands::None, "DIV YA,X"),
    (0xCF, 1, 9, Measure::Straight, Operands::None, "MUL YA"),
    // 1-bit ALU on the carry.
    (
        0xEA,
        3,
        5,
        Measure::Straight,
        Operands::AbsBit,
        "NOT1 aaa.b",
    ),
    (
        0xCA,
        3,
        6,
        Measure::Straight,
        Operands::AbsBit,
        "MOV1 aaa.b,C",
    ),
    (
        0xAA,
        3,
        4,
        Measure::Straight,
        Operands::AbsBit,
        "MOV1 C,aaa.b",
    ),
    (
        0x0A,
        3,
        5,
        Measure::Straight,
        Operands::AbsBit,
        "OR1 C,aaa.b",
    ),
    (
        0x2A,
        3,
        5,
        Measure::Straight,
        Operands::AbsBit,
        "OR1 C,/aaa.b",
    ),
    (
        0x4A,
        3,
        4,
        Measure::Straight,
        Operands::AbsBit,
        "AND1 C,aaa.b",
    ),
    (
        0x6A,
        3,
        4,
        Measure::Straight,
        Operands::AbsBit,
        "AND1 C,/aaa.b",
    ),
    (
        0x8A,
        3,
        5,
        Measure::Straight,
        Operands::AbsBit,
        "EOR1 C,aaa.b",
    ),
    (0x60, 1, 2, Measure::Straight, Operands::None, "CLRC"),
    (0x80, 1, 2, Measure::Straight, Operands::None, "SETC"),
    (0xED, 1, 3, Measure::Straight, Operands::None, "NOTC"),
    (0xE0, 1, 2, Measure::Straight, Operands::None, "CLRV"),
    // Special ALU.
    (0xDF, 1, 3, Measure::Straight, Operands::None, "DAA A"),
    (0xBE, 1, 3, Measure::Straight, Operands::None, "DAS A"),
    (0x9F, 1, 5, Measure::Straight, Operands::None, "XCN A"),
    (0x4E, 3, 6, Measure::Straight, Operands::Abs, "TCLR1 !aaaa"),
    (0x0E, 3, 6, Measure::Straight, Operands::Abs, "TSET1 !aaaa"),
    // Conditional jumps. The cycle count is the taken one; see `Measure::BranchTaken`.
    (
        0x10,
        2,
        4,
        Measure::BranchTaken(Flag::NegClear),
        Operands::Rel,
        "BPL rr",
    ),
    (
        0x30,
        2,
        4,
        Measure::BranchTaken(Flag::NegSet),
        Operands::Rel,
        "BMI rr",
    ),
    (
        0x50,
        2,
        4,
        Measure::BranchTaken(Flag::OvfClear),
        Operands::Rel,
        "BVC rr",
    ),
    (
        0x70,
        2,
        4,
        Measure::BranchTaken(Flag::OvfSet),
        Operands::Rel,
        "BVS rr",
    ),
    (
        0x90,
        2,
        4,
        Measure::BranchTaken(Flag::CarryClear),
        Operands::Rel,
        "BCC rr",
    ),
    (
        0xB0,
        2,
        4,
        Measure::BranchTaken(Flag::CarrySet),
        Operands::Rel,
        "BCS rr",
    ),
    (
        0xD0,
        2,
        4,
        Measure::BranchTaken(Flag::ZeroClear),
        Operands::Rel,
        "BNE rr",
    ),
    (
        0xF0,
        2,
        4,
        Measure::BranchTaken(Flag::ZeroSet),
        Operands::Rel,
        "BEQ rr",
    ),
    (
        0x2E,
        3,
        7,
        Measure::BranchTaken(Flag::None),
        Operands::DpRel,
        "CBNE aa,rr",
    ),
    (
        0xDE,
        3,
        8,
        Measure::BranchTaken(Flag::None),
        Operands::DpIndexedRel,
        "CBNE aa+X,rr",
    ),
    (
        0xFE,
        2,
        6,
        Measure::BranchTaken(Flag::None),
        Operands::Rel,
        "DBNZ Y,rr",
    ),
    (
        0x6E,
        3,
        7,
        Measure::BranchTaken(Flag::None),
        Operands::DpRel,
        "DBNZ aa,rr",
    ),
    (
        0x2F,
        2,
        4,
        Measure::BranchTaken(Flag::None),
        Operands::Rel,
        "BRA rr",
    ),
    // The opcodes that end the straight line.
    (0x5F, 3, 3, JMP_ABS, Operands::Abs, "JMP !aaaa"),
    (0x1F, 3, 6, JMP_ABS, Operands::Abs, "JMP [!aaaa+X]"),
    (0x3F, 3, 8, CALL_ABS, Operands::Abs, "CALL !aaaa"),
    (0x4F, 2, 6, IPL_VECTOR, Operands::Imm, "PCALL uu"),
    (0x6F, 1, 5, NEEDS_STACK, Operands::None, "RET"),
    (0x7F, 1, 6, NEEDS_STACK, Operands::None, "RET1"),
    (0x0F, 1, 8, IPL_VECTOR, Operands::None, "BRK"),
    // Wait / delay / control.
    (0x00, 1, 2, Measure::Straight, Operands::None, "NOP"),
    (0xEF, 1, 0, HALTS, Operands::None, "SLEEP"),
    (0xFF, 1, 0, HALTS, Operands::None, "STOP"),
    (0x20, 1, 2, Measure::Straight, Operands::None, "CLRP"),
    (0x40, 1, 2, Measure::Straight, Operands::None, "SETP"),
    (0xA0, 1, 3, Measure::Straight, Operands::None, "EI"),
    (0xC0, 1, 3, Measure::Straight, Operands::None, "DI"),
];

/// An absolute jump's target is a fixed address, so eight copies cannot each reach the next.
const JMP_ABS: Measure = Measure::NotStraightLine(
    "an absolute jump takes one fixed target, and every copy in the block sits at a different \
     address — a single encoding cannot make each copy fall into the next",
);

/// A call has the jump's problem and pushes as well.
const CALL_ABS: Measure = Measure::NotStraightLine(
    "a call has the absolute jump's fixed-target problem and pushes a return address the block \
     never pops",
);

/// The vectored calls read their target out of the IPL ROM.
const IPL_VECTOR: Measure = Measure::NotStraightLine(
    "the target comes from the IPL ROM at $FFC0-$FFFF, which every Group E program keeps mapped so \
     it can hand the APU back — the vector is not the cart's to point anywhere",
);

/// Returns need an address the block did not push.
const NEEDS_STACK: Measure = Measure::NotStraightLine(
    "a return pops an address the block never pushed; priming the stack for eight of them is a \
     different measurement with a different setup cost",
);

/// `SLEEP` and `STOP` never come back.
const HALTS: Measure = Measure::NotStraightLine(
    "halts the processor, and the SNES APU has no interrupt source to wake it — fullsnes gives its \
     cycle count as `?` for the same reason",
);

/// The whole 256-entry map.
///
/// # Panics
///
/// Panics if the rules above leave a slot empty or fill one twice. That is a build-time gate on the
/// table's completeness: the regular groups and [`SINGLES`] are two independent descriptions of the
/// same map, and an overlap between them means one of the two is wrong about the opcode map's
/// shape.
#[must_use]
pub fn table() -> Vec<Op> {
    let mut slots: Vec<Option<Op>> = vec![None; 256];

    let mut put =
        |code: u8, len: u8, cycles: u8, measure: Measure, operands: Operands, name: String| {
            let name: &'static str = Box::leak(name.into_boxed_str());
            assert!(
                slots[code as usize].is_none(),
                "opcode ${code:02X} is described twice — as {} and as {name}",
                slots[code as usize].expect("checked").name
            );
            slots[code as usize] = Some(Op {
                code,
                len,
                cycles,
                measure,
                name,
                operands,
            });
        };

    for (base, op) in ALU {
        for (offset, len, cycles, operands, form) in ALU_FORMS {
            put(
                base + offset,
                len,
                cycles,
                Measure::Straight,
                operands,
                format!("{op} {form}"),
            );
        }
    }
    for (base, op) in SHIFT {
        for (offset, len, cycles, operands, form) in SHIFT_FORMS {
            put(
                base + offset,
                len,
                cycles,
                Measure::Straight,
                operands,
                format!("{op} {form}"),
            );
        }
    }
    for bit in 0..8u8 {
        put(
            bit * 0x20 + 0x02,
            2,
            4,
            Measure::Straight,
            Operands::Dp,
            format!("SET1 aa.{bit}"),
        );
        put(
            bit * 0x20 + 0x12,
            2,
            4,
            Measure::Straight,
            Operands::Dp,
            format!("CLR1 aa.{bit}"),
        );
        put(
            bit * 0x20 + 0x03,
            3,
            7,
            Measure::BranchTaken(Flag::None),
            Operands::DpRel,
            format!("BBS aa.{bit},rr"),
        );
        put(
            bit * 0x20 + 0x13,
            3,
            7,
            Measure::BranchTaken(Flag::None),
            Operands::DpRel,
            format!("BBC aa.{bit},rr"),
        );
        put(
            bit * 0x10 + 0x01,
            1,
            8,
            IPL_VECTOR,
            Operands::None,
            format!("TCALL {bit}"),
        );
        put(
            (bit + 8) * 0x10 + 0x01,
            1,
            8,
            IPL_VECTOR,
            Operands::None,
            format!("TCALL {}", bit + 8),
        );
    }
    for &(code, len, cycles, measure, operands, name) in SINGLES {
        put(code, len, cycles, measure, operands, name.to_owned());
    }

    slots
        .into_iter()
        .enumerate()
        .map(|(code, op)| op.unwrap_or_else(|| panic!("opcode ${code:02X} has no description")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Measure, table};

    /// Every slot filled exactly once — the assertion inside [`table`], reached as a test so the
    /// failure names the opcode rather than arriving as a generator panic mid-build.
    #[test]
    fn the_map_is_complete_and_has_no_overlaps() {
        let t = table();
        assert_eq!(t.len(), 256);
        for (i, op) in t.iter().enumerate() {
            assert_eq!(usize::from(op.code), i);
            assert!(
                (1..=3).contains(&op.len),
                "${:02X} has length {}",
                i,
                op.len
            );
        }
    }

    /// The opcodes that cannot be timed in place are a short, named list. If this count moves, the
    /// coverage report's account of what the sweep leaves out has to move with it.
    #[test]
    fn twenty_five_opcodes_are_not_straight_line() {
        let t = table();
        let excluded: Vec<_> = t
            .iter()
            .filter(|op| matches!(op.measure, Measure::NotStraightLine(_)))
            .map(|op| op.name)
            .collect();
        assert_eq!(
            excluded.len(),
            25,
            "the non-straight-line set is {excluded:?}"
        );
    }

    /// A branch's documented cost here is the taken one, which is what the sweep arranges to
    /// measure. Every branch costs strictly more taken than the two-cycle fetch it would otherwise
    /// be, so a zero would mean a transcription slip.
    #[test]
    fn every_branch_has_a_taken_cost() {
        for op in table() {
            if matches!(op.measure, Measure::BranchTaken(_)) {
                assert!(op.cycles >= 4, "{} is {} cycles taken", op.name, op.cycles);
            }
        }
    }
}
