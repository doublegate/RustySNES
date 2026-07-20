//! The opcode cycle sweep — ticket **T-04-I**.
//!
//! # What this is
//!
//! Per-opcode instruction timing, measured against expectations derived from the manufacturer
//! instruction-operation tables rather than from any emulator. See
//! `docs/accuracysnes-timing-oracle.md` for the oracle and its provenance; the short version is:
//!
//! ```text
//! clocks = 8*mem + 6*internal,  cycles = mem + internal   =>   clocks = 6*cycles + 2*mem
//! ```
//!
//! with code in bank `$00` ROM and the stack in low WRAM (both 8-clock regions) and internal cycles
//! at 6. `mem` is instruction length plus data and stack accesses. Every expectation in [`SAFE`] is
//! computed that way from cycle counts that WDC, GTE and VLSI all agree on.
//!
//! # Why this subset
//!
//! The sweep covers the opcodes whose operands and safety are *unambiguous*: implied,
//! immediate-with-`m=1`/`x=1`, and balanced push/pull pairs. That is deliberate — the dossier's
//! `A5.01`-`A5.08` ask for all 256, and the remainder need per-opcode work this table is the
//! foundation for, not a shortcut around:
//!
//! - **Control flow** (`JMP`, `JSR`, `JSL`, `RTS`, `RTL`, `RTI`, taken branches) moves `PC`, so it
//!   cannot simply be repeated inline. Untaken branches are measurable and belong in a later batch.
//! - **`BRK`/`COP`** vector away. **`STP`** halts the CPU until reset, so a self-scoring battery
//!   that executes it never reports — permanently excluded.
//! - **`WAI`** waits for an interrupt.
//! - **Memory-addressing modes** need a guaranteed-safe operand: with `DBR=$00` an absolute address
//!   can land in MMIO, so each needs a checked target rather than a blanket rule.
//!
//! # The measurement constraint that shapes everything here
//!
//! A measured span must stay under the **341-dot scanline wrap**, past which the H-counter
//! difference silently returns a plausible small number instead of failing. With the fixed ~165-dot
//! measurement overhead, 8 repeats leaves ~77 clocks per instruction of headroom — comfortably
//! above the most expensive opcode here — while keeping 4 dots of resolution per 2-clock
//! difference. `A5.08` records its raw spans for exactly this reason; so does every entry below.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// One sweep entry: a body repeated 8 times, and the expected cost of one iteration.
struct Op {
    /// What is being measured, for the failure message and the catalog.
    name: &'static str,
    /// The instruction(s) making up one iteration. Push/pull are paired so the stack balances.
    body: &'static [&'static str],
    /// Expected master clocks for one iteration, from `6*cycles + 2*mem`.
    clocks: u16,
    /// The derivation, so a disagreement can be argued with rather than just re-measured.
    why: &'static str,
}

/// Repeats per measurement. See the module docs — bounded by the scanline wrap, not by taste.
const REPS: u16 = 8;

/// Master clocks for one `NOP`: 2 cycles, 1 access.
const NOP_CLOCKS: u16 = 14;

/// Tolerance in dots, matching the rest of the battery's timing tests.
const TOL: u16 = 2;

/// Longest `body` any entry in [`SAFE`] may have.
///
/// The baseline is one `NOP` per instruction in the body, so [`NOPS`] has to cover the longest one.
/// Checked rather than assumed — a longer entry added later would otherwise slice out of bounds.
const MAX_BODY: usize = 4;

/// The baseline body, sliced to match the entry under test. Static so no allocation is involved.
const NOPS: [&str; MAX_BODY] = ["nop"; MAX_BODY];

/// The opcodes swept, with their derived expectations.
const SAFE: &[Op] = &[
    Op {
        name: "CLC",
        body: &["clc"],
        clocks: 14,
        why: "2 cycles, 1 access (opcode fetch)",
    },
    Op {
        name: "SEC",
        body: &["sec"],
        clocks: 14,
        why: "2 cycles, 1 access",
    },
    Op {
        name: "CLV",
        body: &["clv"],
        clocks: 14,
        why: "2 cycles, 1 access",
    },
    Op {
        name: "INX",
        body: &["inx"],
        clocks: 14,
        why: "2 cycles, 1 access",
    },
    Op {
        name: "DEX",
        body: &["dex"],
        clocks: 14,
        why: "2 cycles, 1 access",
    },
    Op {
        name: "TAX",
        body: &["tax"],
        clocks: 14,
        why: "2 cycles, 1 access",
    },
    Op {
        name: "TXY",
        body: &["txy"],
        clocks: 14,
        why: "2 cycles, 1 access",
    },
    Op {
        name: "ASL A",
        body: &["asl a"],
        clocks: 14,
        why: "accumulator R-M-W is 2 cycles, 1 access — no memory operand",
    },
    Op {
        name: "XBA",
        body: &["xba"],
        clocks: 20,
        why: "3 cycles, 1 access — the extra cycle is internal",
    },
    Op {
        name: "TCD",
        body: &["tcd"],
        clocks: 14,
        why: "2 cycles, 1 access; 16-bit transfer regardless of m",
    },
    Op {
        name: "LDA #imm",
        body: &["lda #$00"],
        clocks: 16,
        why: "2 cycles, 2 accesses (opcode + operand) at m=1",
    },
    Op {
        name: "LDX #imm",
        body: &["ldx #$00"],
        clocks: 16,
        why: "2 cycles, 2 accesses at x=1",
    },
    Op {
        name: "CMP #imm",
        body: &["cmp #$00"],
        clocks: 16,
        why: "2 cycles, 2 accesses at m=1",
    },
    Op {
        name: "BIT #imm",
        body: &["bit #$00"],
        clocks: 16,
        why: "2 cycles, 2 accesses at m=1",
    },
    Op {
        name: "REP #imm",
        body: &[".byte $C2, $00   ; rep #$00"],
        clocks: 22,
        why: "3 cycles, 2 accesses; raw bytes so the width tracker is not misled",
    },
    Op {
        name: "SEP #imm",
        body: &[".byte $E2, $00   ; sep #$00"],
        clocks: 22,
        why: "3 cycles, 2 accesses",
    },
    Op {
        name: "WDM",
        body: &[".byte $42, $EA   ; wdm"],
        clocks: 16,
        why: "reserved 2-byte no-op: 2 cycles, 2 accesses",
    },
    Op {
        name: "PHA+PLA",
        body: &["pha", "pla"],
        clocks: 50,
        why: "PHA 3 cycles / 2 accesses = 22, PLA 4 / 2 = 28, at m=1",
    },
    Op {
        name: "PHP+PLP",
        body: &["php", "plp"],
        clocks: 50,
        why: "PHP 3 / 2 = 22, PLP 4 / 2 = 28",
    },
    Op {
        name: "PHB+PLB",
        body: &["phb", "plb"],
        clocks: 50,
        why: "PHB 3 / 2 = 22, PLB 4 / 2 = 28",
    },
    Op {
        name: "PHD+PLD",
        body: &["phd", "pld"],
        clocks: 66,
        why: "PHD 4 cycles / 3 accesses = 30, PLD 5 / 3 = 36 (16-bit register, two stack bytes)",
    },
    Op {
        name: "PHX+PLX",
        body: &["phx", "plx"],
        clocks: 50,
        why: "PHX 3 / 2 = 22, PLX 4 / 2 = 28, at x=1",
    },
    // --- memory addressing. Operands are checked-safe WRAM: direct page $10 with D=0 reaches
    // $00:0010, absolute $0400 and long $7E0400 are low WRAM. With DBR=$00 an unchecked absolute
    // operand can land in MMIO, which is why each of these is a named address rather than a rule.
    Op {
        name: "LDA dp",
        body: &["lda $10"],
        clocks: 24,
        why: "3 cycles, 3 accesses (opcode, dp operand, data) at m=1, DL=0 — no internal cycle",
    },
    Op {
        name: "LDA abs",
        body: &["lda a:$0400"],
        clocks: 32,
        why: "4 cycles, 4 accesses (opcode, 2 operand bytes, data) at m=1",
    },
    Op {
        name: "LDA long",
        body: &["lda f:$7E0400"],
        clocks: 40,
        why: "5 cycles, 5 accesses (opcode, 3 operand bytes, data) at m=1",
    },
    Op {
        name: "STA dp",
        body: &["sta $10"],
        clocks: 24,
        why: "3 cycles, 3 accesses; the data cycle is a write",
    },
    Op {
        name: "STA abs",
        body: &["sta a:$0400"],
        clocks: 32,
        why: "4 cycles, 4 accesses at m=1",
    },
    Op {
        name: "LDA dp,X",
        body: &["lda $10,x"],
        clocks: 30,
        why: "4 cycles, 3 accesses + 1 internal for the index add",
    },
    Op {
        name: "LDA abs,X",
        body: &["lda a:$0400,x"],
        clocks: 32,
        why: "4 cycles, 4 accesses at x=1 with no page cross — the +1 p penalty does not apply",
    },
    Op {
        name: "INC dp",
        body: &["inc $10"],
        clocks: 38,
        why: "R-M-W: 5 cycles, 4 accesses (opcode, operand, read, write) + 1 internal modify",
    },
    Op {
        name: "INC abs",
        body: &["inc a:$0400"],
        clocks: 46,
        why: "R-M-W: 6 cycles, 5 accesses + 1 internal modify",
    },
    Op {
        name: "ADC dp",
        body: &["adc $10"],
        clocks: 24,
        why: "3 cycles, 3 accesses at m=1, DL=0",
    },
    Op {
        name: "CMP abs",
        body: &["cmp a:$0400"],
        clocks: 32,
        why: "4 cycles, 4 accesses at m=1",
    },
    // Untaken branch: 2 cycles, 2 accesses, i.e. 2 clocks more than a NOP.
    //
    // The condition has to be established INSIDE the measured body. Setting it in the sandbox does
    // not work: `measure_begin` emits a `jsr`, and the called code clobbers the flags before the
    // branch is reached. The first version of this entry relied on the sandbox's `ldx #$00` leaving
    // Z set and silently measured a TAKEN branch instead.
    //
    // `clv` + `bvs` is deterministic — V is cleared immediately before a branch-if-V-set — and the
    // two costs separate cleanly: untaken gives 4 dots over the two-NOP baseline, taken would give
    // 16. A taken branch cannot be measured by inline repetition at all, and belongs with the
    // control-flow batch.
    Op {
        name: "BVS untaken",
        body: &["clv", "bvs *+2"],
        clocks: 30,
        why: "CLV 2 cycles / 1 access = 14, plus an untaken branch at 2 cycles / 2 accesses = 16",
    },
];

/// Every sweep test, one per opcode entry.
#[must_use]
pub fn all() -> Vec<Test> {
    SAFE.iter().enumerate().map(|(i, op)| one(i, op)).collect()
}

/// Expected dots above the `NOP` baseline for `reps` iterations of `op`.
const fn expected_dots(op_clocks: u16, iters: u16) -> u16 {
    // One iteration may be several instructions; the baseline is one NOP per instruction.
    (op_clocks - NOP_CLOCKS * iters) * REPS / 4
}

fn one(index: usize, op: &Op) -> Test {
    assert!(
        op.body.len() <= MAX_BODY,
        "sweep entry {} has {} instructions, over MAX_BODY ({MAX_BODY}) — raise MAX_BODY",
        op.name,
        op.body.len()
    );
    let iters = u16::try_from(op.body.len()).expect("body length fits u16");
    let expect = expected_dots(op.clocks, iters);
    let slot_base = 8 + u8::try_from(index).expect("sweep index fits u8") * 2;

    let mut a = Asm::new();
    a.c(&format!(
        "{} — expect {} clocks per iteration.",
        op.name, op.clocks
    ));
    a.c(&format!("Derivation: {}.", op.why));
    a.c(&format!(
        "Baseline is {iters} NOP(s) per iteration so the fetch overhead cancels; the difference is"
    ));
    a.c("the instruction's own extra cost. Raw spans are recorded so a failure can be inspected.");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.c("The sandbox establishes m=1 AND x=1, the state every expectation in this table is stated");
    a.c("at. `sep #$20` alone narrows only the accumulator and leaves the index registers 16-bit,");
    a.c("which silently changes the cost of every index-register instruction: LDX #imm becomes a");
    a.c("3-byte 3-cycle fetch, and PHX/PLX push and pull two bytes instead of one. Both showed up");
    a.c("as failures the first time this ran — the sandbox has to match its own preconditions.");
    a.l("sep #$30");
    a.c("X = 0 so the indexed entries have a defined index that cannot cross a page. The baseline");
    a.c("is NOPs, which do not touch X, so this costs nothing in the difference.");
    a.l("ldx #$00");
    a.c("--- baseline ---");
    a.measure_begin();
    a.repeat(u32::from(REPS), &NOPS[..op.body.len()]);
    a.measure_end();
    a.measure_result();
    a.l("sta f:$7E0096");
    a.record(slot_base, "baseline NOPs, absolute");
    a.c("--- the opcode under test ---");
    a.l("sep #$30");
    a.measure_begin();
    a.repeat(u32::from(REPS), op.body);
    a.measure_end();
    a.measure_result();
    a.l("sec");
    a.l("sbc f:$7E0096");
    a.record(slot_base + 1, "measured minus baseline");
    a.assert_a16_range(
        expect.saturating_sub(TOL),
        expect + TOL,
        "measured cost disagrees with the manufacturer tables",
    );
    a.finish(
        Box::leak(format!("A5.S{:02}", index + 1).into_boxed_str()),
        'A',
        Box::leak(format!("Sweep: {}", op.name).into_boxed_str()),
        Provenance::Documented(
            "WDC/GTE/VLSI instruction-operation tables agree; docs/accuracysnes-timing-oracle.md",
        ),
        Kind::Scored,
        None,
    )
}
