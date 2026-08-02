//! The documented 65C816 opcode map — addressing mode, length, and whether the opcode continues
//! the straight line.
//!
//! # Where the numbers come from
//!
//! **Table 5-4** of the WDC W65C816S datasheet (`ref-docs/2026-07-20-wdc-w65c816s-citation.md`) is
//! the opcode matrix: mnemonic, addressing-mode symbol, cycles and bytes for all 256 opcodes.
//! Lengths here are that table's byte counts, and the addressing modes are its symbols. Nothing is
//! read out of `crates/rustysnes-cpu/` — `A6.15` is a scored row, so its expectation has to come
//! from outside the thing being tested.
//!
//! # Why the map is built from rules
//!
//! The 65C816 matrix is columnar, and the datasheet presents it that way: the eight ALU operations
//! share one fifteen-entry addressing-mode column, the read-modify-writes share another, the
//! conditional branches occupy `$10` apart in the `$x0` column. Typing 256 rows out by hand
//! introduces transcription errors the rules cannot have. [`table`] asserts that the rules filled
//! all 256 slots **exactly once**, so a rule that overlaps another or misses a slot fails the build
//! rather than shipping a hole — which for this row would be a hole in the very claim it makes.
//!
//! # Lengths are stated at `m = 1, x = 1`
//!
//! Immediate operands are one byte or two depending on the `m` and `x` flags, so a length table is
//! meaningless without pinning them. Everything here is the 8-bit-accumulator, 8-bit-index case,
//! which is what `sweep.rs`'s sandbox establishes with `sep #$30` — and which it had to learn the
//! hard way, having first set only `sep #$20` and measured `LDX #imm` as a three-byte fetch against
//! a two-byte expectation.

/// Whether an opcode continues the straight line, and if not, why not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flow {
    /// Executes and falls through to the next instruction.
    Straight,
    /// A relative branch. With a displacement of zero the taken path lands on the following
    /// instruction, which is where a not-taken branch would have gone — so either way the sandbox
    /// resumes at the same address and the length claim is still what is being tested.
    Branch,
    /// Leaves the sandbox, with the reason. These are the opcodes for which "the following
    /// instruction" is not a thing that exists.
    Leaves(&'static str),
}

/// The addressing mode, which is what decides the length.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// No operand: implied, accumulator, or stack.
    Implied,
    /// One immediate byte (at `m = 1` / `x = 1`).
    Immediate,
    /// A direct-page offset, indexed or not, direct or indirect.
    Direct,
    /// A 16-bit absolute address, indexed or not.
    Absolute,
    /// A 24-bit absolute long address.
    Long,
    /// An 8-bit stack-relative offset.
    StackRelative,
    /// An 8-bit signed displacement.
    Relative,
    /// A 16-bit signed displacement (`BRL`, `PER`).
    RelativeLong,
    /// Two direct-page-ish bytes: `MVN`/`MVP`'s source and destination banks.
    BlockMove,
}

impl Mode {
    /// Instruction length in bytes, opcode included, at `m = 1` and `x = 1`.
    #[must_use]
    pub const fn len(self) -> u8 {
        match self {
            Self::Implied => 1,
            Self::Immediate | Self::Direct | Self::StackRelative | Self::Relative => 2,
            Self::Absolute | Self::RelativeLong | Self::BlockMove => 3,
            Self::Long => 4,
        }
    }
}

/// One opcode's documented shape.
#[derive(Clone, Copy, Debug)]
pub struct Op {
    /// The opcode byte.
    pub code: u8,
    /// Its addressing mode, which decides [`Op::len`].
    pub mode: Mode,
    /// Whether it continues the straight line.
    pub flow: Flow,
    /// Mnemonic plus mode, for the generated failure text and the catalog.
    pub name: &'static str,
}

impl Op {
    /// Instruction length in bytes at `m = 1`, `x = 1`.
    #[must_use]
    pub const fn len(self) -> u8 {
        self.mode.len()
    }
}

/// The eight operations sharing the ALU addressing-mode column, by the high nibble they base at.
const ALU: [(u8, &str); 8] = [
    (0x00, "ORA"),
    (0x20, "AND"),
    (0x40, "EOR"),
    (0x60, "ADC"),
    (0x80, "STA"),
    (0xA0, "LDA"),
    (0xC0, "CMP"),
    (0xE0, "SBC"),
];

/// The ALU column: `(offset from the base, mode, suffix)`.
///
/// `$09` — the immediate — is deliberately absent: `STA` has no immediate form and `$89` is
/// `BIT #`, so the column does not actually tile there. Handled in [`SINGLES`], one row each.
const ALU_MODES: [(u8, Mode, &str); 14] = [
    (0x01, Mode::Direct, "(dp,X)"),
    (0x03, Mode::StackRelative, "sr,S"),
    (0x05, Mode::Direct, "dp"),
    (0x07, Mode::Direct, "[dp]"),
    (0x0D, Mode::Absolute, "abs"),
    (0x0F, Mode::Long, "long"),
    (0x11, Mode::Direct, "(dp),Y"),
    (0x12, Mode::Direct, "(dp)"),
    (0x13, Mode::StackRelative, "(sr,S),Y"),
    (0x15, Mode::Direct, "dp,X"),
    (0x17, Mode::Direct, "[dp],Y"),
    (0x19, Mode::Absolute, "abs,Y"),
    (0x1D, Mode::Absolute, "abs,X"),
    (0x1F, Mode::Long, "long,X"),
];

/// The read-modify-write group, by the high nibble it bases at. `DEC`/`INC` are the two whose
/// accumulator form sits at `$3A`/`$1A` rather than in this column, so they carry only four entries
/// here and their odd ones out are in [`SINGLES`].
const RMW: [(u8, &str); 6] = [
    (0x00, "ASL"),
    (0x20, "ROL"),
    (0x40, "LSR"),
    (0x60, "ROR"),
    (0xC0, "DEC"),
    (0xE0, "INC"),
];

/// The read-modify-write column: `(offset, mode, suffix)`.
const RMW_MODES: [(u8, Mode, &str); 4] = [
    (0x06, Mode::Direct, "dp"),
    (0x0E, Mode::Absolute, "abs"),
    (0x16, Mode::Direct, "dp,X"),
    (0x1E, Mode::Absolute, "abs,X"),
];

/// The eight conditional branches, `$10` apart in the `$x0` column.
const BRANCHES: [(u8, &str); 8] = [
    (0x10, "BPL"),
    (0x30, "BMI"),
    (0x50, "BVC"),
    (0x70, "BVS"),
    (0x90, "BCC"),
    (0xB0, "BCS"),
    (0xD0, "BNE"),
    (0xF0, "BEQ"),
];

/// Everything the columns do not cover.
///
/// `(code, mode, flow, name)`, grouped the way the datasheet groups them so a run can be checked
/// against one block of Table 5-4 at a time.
const SINGLES: &[(u8, Mode, Flow, &str)] = &[
    // The ALU immediates. `$89` is `BIT #`, not a store — the one place the ALU column does not
    // tile, and the reason `$09` is not in `ALU_MODES`.
    (0x09, Mode::Immediate, Flow::Straight, "ORA #"),
    (0x29, Mode::Immediate, Flow::Straight, "AND #"),
    (0x49, Mode::Immediate, Flow::Straight, "EOR #"),
    (0x69, Mode::Immediate, Flow::Straight, "ADC #"),
    (0x89, Mode::Immediate, Flow::Straight, "BIT #"),
    (0xA9, Mode::Immediate, Flow::Straight, "LDA #"),
    (0xC9, Mode::Immediate, Flow::Straight, "CMP #"),
    (0xE9, Mode::Immediate, Flow::Straight, "SBC #"),
    // The accumulator forms of the shift and increment group.
    (0x0A, Mode::Implied, Flow::Straight, "ASL A"),
    (0x2A, Mode::Implied, Flow::Straight, "ROL A"),
    (0x4A, Mode::Implied, Flow::Straight, "LSR A"),
    (0x6A, Mode::Implied, Flow::Straight, "ROR A"),
    (0x1A, Mode::Implied, Flow::Straight, "INC A"),
    (0x3A, Mode::Implied, Flow::Straight, "DEC A"),
    // `STZ`, which has no eight-way base of its own.
    (0x64, Mode::Direct, Flow::Straight, "STZ dp"),
    (0x74, Mode::Direct, Flow::Straight, "STZ dp,X"),
    (0x9C, Mode::Absolute, Flow::Straight, "STZ abs"),
    (0x9E, Mode::Absolute, Flow::Straight, "STZ abs,X"),
    // `BIT`, `TRB`, `TSB`.
    (0x24, Mode::Direct, Flow::Straight, "BIT dp"),
    (0x2C, Mode::Absolute, Flow::Straight, "BIT abs"),
    (0x34, Mode::Direct, Flow::Straight, "BIT dp,X"),
    (0x3C, Mode::Absolute, Flow::Straight, "BIT abs,X"),
    (0x14, Mode::Direct, Flow::Straight, "TRB dp"),
    (0x1C, Mode::Absolute, Flow::Straight, "TRB abs"),
    (0x04, Mode::Direct, Flow::Straight, "TSB dp"),
    (0x0C, Mode::Absolute, Flow::Straight, "TSB abs"),
    // Index-register loads, stores and compares.
    (0xA0, Mode::Immediate, Flow::Straight, "LDY #"),
    (0xA4, Mode::Direct, Flow::Straight, "LDY dp"),
    (0xAC, Mode::Absolute, Flow::Straight, "LDY abs"),
    (0xB4, Mode::Direct, Flow::Straight, "LDY dp,X"),
    (0xBC, Mode::Absolute, Flow::Straight, "LDY abs,X"),
    (0xA2, Mode::Immediate, Flow::Straight, "LDX #"),
    (0xA6, Mode::Direct, Flow::Straight, "LDX dp"),
    (0xAE, Mode::Absolute, Flow::Straight, "LDX abs"),
    (0xB6, Mode::Direct, Flow::Straight, "LDX dp,Y"),
    (0xBE, Mode::Absolute, Flow::Straight, "LDX abs,Y"),
    (0x84, Mode::Direct, Flow::Straight, "STY dp"),
    (0x8C, Mode::Absolute, Flow::Straight, "STY abs"),
    (0x94, Mode::Direct, Flow::Straight, "STY dp,X"),
    (0x86, Mode::Direct, Flow::Straight, "STX dp"),
    (0x8E, Mode::Absolute, Flow::Straight, "STX abs"),
    (0x96, Mode::Direct, Flow::Straight, "STX dp,Y"),
    (0xC0, Mode::Immediate, Flow::Straight, "CPY #"),
    (0xC4, Mode::Direct, Flow::Straight, "CPY dp"),
    (0xCC, Mode::Absolute, Flow::Straight, "CPY abs"),
    (0xE0, Mode::Immediate, Flow::Straight, "CPX #"),
    (0xE4, Mode::Direct, Flow::Straight, "CPX dp"),
    (0xEC, Mode::Absolute, Flow::Straight, "CPX abs"),
    // Register transfers, increments and decrements.
    (0xAA, Mode::Implied, Flow::Straight, "TAX"),
    (0xA8, Mode::Implied, Flow::Straight, "TAY"),
    (0x8A, Mode::Implied, Flow::Straight, "TXA"),
    (0x98, Mode::Implied, Flow::Straight, "TYA"),
    (0xBA, Mode::Implied, Flow::Straight, "TSX"),
    (0x9A, Mode::Implied, Flow::Straight, "TXS"),
    (0x9B, Mode::Implied, Flow::Straight, "TXY"),
    (0xBB, Mode::Implied, Flow::Straight, "TYX"),
    (0x5B, Mode::Implied, Flow::Straight, "TCD"),
    (0x7B, Mode::Implied, Flow::Straight, "TDC"),
    (0x1B, Mode::Implied, Flow::Straight, "TCS"),
    (0x3B, Mode::Implied, Flow::Straight, "TSC"),
    (0xE8, Mode::Implied, Flow::Straight, "INX"),
    (0xC8, Mode::Implied, Flow::Straight, "INY"),
    (0xCA, Mode::Implied, Flow::Straight, "DEX"),
    (0x88, Mode::Implied, Flow::Straight, "DEY"),
    // Flags.
    (0x18, Mode::Implied, Flow::Straight, "CLC"),
    (0x38, Mode::Implied, Flow::Straight, "SEC"),
    (0x58, Mode::Implied, Flow::Straight, "CLI"),
    (0x78, Mode::Implied, Flow::Straight, "SEI"),
    (0xB8, Mode::Implied, Flow::Straight, "CLV"),
    (0xD8, Mode::Implied, Flow::Straight, "CLD"),
    (0xF8, Mode::Implied, Flow::Straight, "SED"),
    (0xC2, Mode::Immediate, Flow::Straight, "REP #"),
    (0xE2, Mode::Immediate, Flow::Straight, "SEP #"),
    (0xFB, Mode::Implied, Flow::Straight, "XCE"),
    // Stack.
    (0x48, Mode::Implied, Flow::Straight, "PHA"),
    (0x68, Mode::Implied, Flow::Straight, "PLA"),
    (0xDA, Mode::Implied, Flow::Straight, "PHX"),
    (0xFA, Mode::Implied, Flow::Straight, "PLX"),
    (0x5A, Mode::Implied, Flow::Straight, "PHY"),
    (0x7A, Mode::Implied, Flow::Straight, "PLY"),
    (0x08, Mode::Implied, Flow::Straight, "PHP"),
    (0x28, Mode::Implied, Flow::Straight, "PLP"),
    (0x8B, Mode::Implied, Flow::Straight, "PHB"),
    (0xAB, Mode::Implied, Flow::Straight, "PLB"),
    (0x0B, Mode::Implied, Flow::Straight, "PHD"),
    (0x2B, Mode::Implied, Flow::Straight, "PLD"),
    (0x4B, Mode::Implied, Flow::Straight, "PHK"),
    (0xF4, Mode::Absolute, Flow::Straight, "PEA abs"),
    (0xD4, Mode::Direct, Flow::Straight, "PEI dp"),
    (0x62, Mode::RelativeLong, Flow::Straight, "PER rel16"),
    // No-ops and the reserved two-byte no-op.
    (0xEA, Mode::Implied, Flow::Straight, "NOP"),
    (0xEB, Mode::Implied, Flow::Straight, "XBA"),
    (0x42, Mode::Immediate, Flow::Straight, "WDM"),
    // Block moves. Two operand bytes, and they repeat until the counter runs out — but they do
    // return to the following instruction, so the length claim is testable.
    (0x54, Mode::BlockMove, Flow::Straight, "MVN"),
    (0x44, Mode::BlockMove, Flow::Straight, "MVP"),
    // The conditional branches' unconditional relatives.
    (0x80, Mode::Relative, Flow::Branch, "BRA"),
    (0x82, Mode::RelativeLong, Flow::Branch, "BRL"),
    // The opcodes that leave the sandbox.
    (0x4C, Mode::Absolute, JUMP, "JMP abs"),
    (0x6C, Mode::Absolute, JUMP, "JMP (abs)"),
    (0x7C, Mode::Absolute, JUMP, "JMP (abs,X)"),
    (0x5C, Mode::Long, JUMP, "JML long"),
    (0xDC, Mode::Absolute, JUMP, "JML [abs]"),
    (0x20, Mode::Absolute, CALL, "JSR abs"),
    (0xFC, Mode::Absolute, CALL, "JSR (abs,X)"),
    (0x22, Mode::Long, CALL, "JSL long"),
    (0x60, Mode::Implied, RETURN, "RTS"),
    (0x6B, Mode::Implied, RETURN, "RTL"),
    (0x40, Mode::Implied, RETURN, "RTI"),
    (0x00, Mode::Immediate, VECTOR, "BRK"),
    (0x02, Mode::Immediate, VECTOR, "COP"),
    (0xDB, Mode::Implied, STOPS, "STP"),
    (0xCB, Mode::Implied, WAITS, "WAI"),
];

/// A jump's target is a fixed address the sandbox does not control per copy.
const JUMP: Flow = Flow::Leaves(
    "a jump sets PC outright, so there is no `following instruction` for a length claim to be about",
);

/// A call jumps and pushes.
const CALL: Flow = Flow::Leaves(
    "a call jumps and pushes a return address; where it resumes is decided by the RTS that matches \
     it, not by this opcode's length",
);

/// A return pops an address the sandbox did not push.
const RETURN: Flow = Flow::Leaves(
    "a return pops an address the sandbox never pushed, and resumes there rather than after itself",
);

/// The software interrupts vector away.
const VECTOR: Flow = Flow::Leaves(
    "vectors through the cartridge's own handler, which is a different measurement — `A6.07` owns \
     the one about BRK's skipped signature byte",
);

/// `STP` is the row's own named exception.
const STOPS: Flow = Flow::Leaves(
    "halts the processor until RESET, which is exactly what this row asserts is the only opcode \
     that does so — a self-scoring battery that executed it would never report anything again",
);

/// `WAI` needs an interrupt to come back.
const WAITS: Flow = Flow::Leaves(
    "waits for an interrupt. It does resume at the following instruction once one arrives, so \
     unlike STP it is recoverable — but the wait is the subject and the length is not",
);

/// The whole 256-entry map.
///
/// # Panics
///
/// If the rules leave a slot empty or fill one twice. The columns and [`SINGLES`] are two
/// independent descriptions of the same matrix, and a clash between them means one of the two is
/// wrong about the map's shape — which for this row would be an error inside the claim itself.
#[must_use]
pub fn table() -> Vec<Op> {
    let mut slots: Vec<Option<Op>> = vec![None; 256];

    let mut put = |code: u8, mode: Mode, flow: Flow, name: String| {
        let name: &'static str = Box::leak(name.into_boxed_str());
        assert!(
            slots[code as usize].is_none(),
            "opcode ${code:02X} is described twice — as {} and as {name}",
            slots[code as usize].expect("checked").name
        );
        slots[code as usize] = Some(Op {
            code,
            mode,
            flow,
            name,
        });
    };

    for (base, op) in ALU {
        for (offset, mode, form) in ALU_MODES {
            put(base + offset, mode, Flow::Straight, format!("{op} {form}"));
        }
    }
    for (base, op) in RMW {
        for (offset, mode, form) in RMW_MODES {
            put(base + offset, mode, Flow::Straight, format!("{op} {form}"));
        }
    }
    for (code, op) in BRANCHES {
        put(code, Mode::Relative, Flow::Branch, format!("{op} rel"));
    }
    for &(code, mode, flow, name) in SINGLES {
        put(code, mode, flow, name.to_owned());
    }

    let holes: Vec<String> = slots
        .iter()
        .enumerate()
        .filter(|(_, op)| op.is_none())
        .map(|(code, _)| format!("${code:02X}"))
        .collect();
    assert!(
        holes.is_empty(),
        "{} opcode(s) have no description: {}",
        holes.len(),
        holes.join(" ")
    );
    slots.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::{Flow, Mode, table};

    /// Every slot filled exactly once. The 65C816 has no undefined opcodes, which is the very
    /// thing `A6.15` asserts — so a hole here would be a hole inside the claim.
    #[test]
    fn the_matrix_is_complete_and_has_no_overlaps() {
        let t = table();
        assert_eq!(t.len(), 256);
        for (i, op) in t.iter().enumerate() {
            assert_eq!(usize::from(op.code), i);
            assert!(
                (1..=4).contains(&op.len()),
                "${i:02X} has length {}",
                op.len()
            );
        }
    }

    /// Fifteen opcodes leave the sandbox, and they are a named list rather than a count that can
    /// drift. If this moves, the row's account of what it does not execute has to move with it.
    #[test]
    fn fifteen_opcodes_leave_the_sandbox() {
        let leaves: Vec<_> = table()
            .into_iter()
            .filter(|op| matches!(op.flow, Flow::Leaves(_)))
            .map(|op| op.name)
            .collect();
        assert_eq!(leaves.len(), 15, "the list is {leaves:?}");
        for expected in [
            "JMP abs",
            "JMP (abs)",
            "JMP (abs,X)",
            "JML long",
            "JML [abs]",
            "JSR abs",
            "JSR (abs,X)",
            "JSL long",
            "RTS",
            "RTL",
            "RTI",
            "BRK",
            "COP",
            "STP",
            "WAI",
        ] {
            assert!(
                leaves.contains(&expected),
                "{expected} is not in {leaves:?}"
            );
        }
    }

    /// Spot-checks against Table 5-4, chosen where a columnar rule could plausibly be wrong: the
    /// place the ALU column does *not* tile, the two long-relative operands, the block moves, and
    /// the four-byte long modes.
    #[test]
    fn the_awkward_corners_match_the_datasheet() {
        let t = table();
        assert_eq!(t[0x89].name, "BIT #", "$89 is BIT #, not a store");
        assert_eq!(t[0x82].mode, Mode::RelativeLong, "BRL is three bytes");
        assert_eq!(t[0x62].mode, Mode::RelativeLong, "PER is three bytes");
        assert_eq!(t[0x54].mode, Mode::BlockMove, "MVN carries two bank bytes");
        assert_eq!(t[0x44].mode, Mode::BlockMove, "MVP carries two bank bytes");
        assert_eq!(t[0x0F].len(), 4, "ORA long is four bytes");
        assert_eq!(t[0x1F].len(), 4, "ORA long,X is four bytes");
        assert_eq!(t[0x22].len(), 4, "JSL is four bytes");
        assert_eq!(t[0x5C].len(), 4, "JML is four bytes");
        assert_eq!(t[0xEA].name, "NOP");
        assert_eq!(t[0x42].len(), 2, "WDM is a reserved TWO-byte no-op");
    }

    /// Nine opcodes are four bytes: the eight ALU long forms' two columns less the ones that are
    /// jumps, plus `JSL` and `JML`. Counting them is a cheap check that the long columns tiled.
    #[test]
    fn the_long_modes_are_where_they_should_be() {
        let four: Vec<_> = table()
            .into_iter()
            .filter(|op| op.len() == 4)
            .map(|op| op.code)
            .collect();
        // Eight `op long` at $x F, eight `op long,X` at $1F, plus JSL ($22) and JML ($5C).
        assert_eq!(four.len(), 18, "the four-byte set is {four:02X?}");
    }
}
