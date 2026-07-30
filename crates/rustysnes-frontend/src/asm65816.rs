//! A one-line inline 65C816 assembler for the debugger (`v1.25.0`, T-FP-C2).
//!
//! # Why this assembles by searching the disassembler
//!
//! The obvious implementation is a second opcode table mapping mnemonic + addressing mode back to a
//! byte. That table would be a **duplicate** of `rustysnes_cpu::disasm`'s, maintained by hand, and
//! the failure mode of a duplicated table is the worst kind: it stays plausible while being wrong
//! for one opcode, and the assembler is exactly the tool you would use to investigate the bug it
//! causes.
//!
//! So instead: for each candidate opcode and operand width, synthesize the bytes, run the **real**
//! disassembler over them, and keep the encoding whose disassembly matches the requested text. The
//! encoder is correct by construction against the decoder, a round-trip test is the natural test,
//! and adding an opcode to the decoder makes it assemblable for free. The search is at most a few
//! hundred cheap decodes per line — invisible for something driven by a text box.
//!
//! # What it does not do
//!
//! One instruction at a time: no labels, no directives, no expressions, no multi-line input. A
//! debugger patch is "make this branch unconditional", not a build system. Anything longer belongs
//! in `ca65`.

use rustysnes_core::cpu::disasm::disassemble_one;

/// Longest 65C816 instruction: opcode plus a 24-bit operand.
pub const MAX_LEN: usize = 4;

/// Why a line could not be assembled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmError {
    /// The line held no mnemonic.
    Empty,
    /// No opcode encodes this mnemonic with this operand form.
    ///
    /// Deliberately one error rather than separate "unknown mnemonic" and "bad operand" cases: the
    /// search cannot distinguish them without the very reverse table this module exists to avoid,
    /// and a wrong guess about which it was would send the user looking in the wrong place.
    NoEncoding(String),
    /// A branch target that is not reachable from here.
    ///
    /// Reported separately because it is the one failure the user can fix by moving the target
    /// rather than by rewriting the instruction — a generic "no encoding" would hide that.
    BranchOutOfRange {
        /// How far away the target is, in bytes.
        distance: i64,
        /// The furthest this branch form can reach.
        limit: i64,
    },
}

impl core::fmt::Display for AsmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => write!(f, "nothing to assemble"),
            Self::NoEncoding(s) => write!(f, "no 65816 encoding for: {s}"),
            Self::BranchOutOfRange { distance, limit } => {
                write!(
                    f,
                    "branch target is {distance} bytes away (limit +/-{limit})"
                )
            }
        }
    }
}

impl core::error::Error for AsmError {}

/// Assemble one instruction to be placed at `pbr:pc`, with the current `M`/`X` widths.
///
/// The address matters because branch operands are PC-relative: the same `BRA $8010` assembles to a
/// different byte at every address. The widths matter because `LDA #$12` is two bytes with `M=1`
/// and three with `M=0` — assembling with the wrong width is how a patch silently desynchronises
/// everything after it, which is the same trap the AccuracySNES generator documents for `ca65`.
///
/// # Errors
/// [`AsmError::NoEncoding`] when no opcode matches, or [`AsmError::BranchOutOfRange`] when a branch
/// cannot reach its target.
pub fn assemble(line: &str, pbr: u8, pc: u16, m8: bool, x8: bool) -> Result<Vec<u8>, AsmError> {
    let want = normalize(line);
    if want.is_empty() {
        return Err(AsmError::Empty);
    }

    // Try every opcode. For each, try every operand value the requested text could imply, by
    // decoding what the disassembler would print and comparing.
    for opcode in 0..=u8::MAX {
        if let Some(bytes) = try_opcode(opcode, &want, pbr, pc, m8, x8) {
            return Ok(bytes);
        }
    }
    // A branch whose mnemonic exists but whose target is unreachable gets the specific error.
    if let Some(err) = branch_range_error(&want, pbr, pc) {
        return Err(err);
    }
    Err(AsmError::NoEncoding(line.trim().to_string()))
}

/// Try to encode `want` as `opcode`, returning the bytes if the round-trip matches.
fn try_opcode(opcode: u8, want: &str, pbr: u8, pc: u16, m8: bool, x8: bool) -> Option<Vec<u8>> {
    // The operand length this opcode takes, learned from the disassembler itself rather than from a
    // second table: decode it once with zeroed operand bytes and read back the reported length.
    let probe = [opcode, 0, 0, 0];
    let (_, len) = decode(&probe, pbr, pc, m8, x8);
    if len == 0 || len > MAX_LEN {
        return None;
    }
    // The mnemonic must match before trying any operand value, or every operand-less opcode would
    // be probed against every operand in turn.
    let (probe_text, _) = decode(&probe, pbr, pc, m8, x8);
    if mnemonic_of(&probe_text) != mnemonic_of(want) {
        return None;
    }
    if len == 1 {
        return (normalize(&probe_text) == want).then(|| vec![opcode]);
    }

    let operand = parse_operand_value(want)?;
    let mut bytes = vec![opcode];
    match len {
        2 => {
            // A 1-byte operand is either a plain byte or a relative branch displacement. Both are
            // covered by trying the value directly and, for branches, the computed displacement.
            for candidate in one_byte_candidates(operand, pbr, pc) {
                bytes.truncate(1);
                bytes.push(candidate);
                if matches(&bytes, want, pbr, pc, m8, x8) {
                    return Some(bytes);
                }
            }
            None
        }
        3 => {
            for candidate in two_byte_candidates(operand, pc) {
                bytes.truncate(1);
                bytes.extend_from_slice(&candidate.to_le_bytes());
                if matches(&bytes, want, pbr, pc, m8, x8) {
                    return Some(bytes);
                }
            }
            None
        }
        4 => {
            let v = u32::try_from(operand & 0x00FF_FFFF).ok()?;
            bytes.extend_from_slice(&v.to_le_bytes()[..3]);
            matches(&bytes, want, pbr, pc, m8, x8).then_some(bytes)
        }
        _ => None,
    }
}

/// One-byte operand candidates: the literal value, and the PC-relative displacement to it.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn one_byte_candidates(operand: i64, pbr: u8, pc: u16) -> Vec<u8> {
    // Masked first, so the narrowing is exact rather than lossy.
    let mut out = vec![(operand & 0xFF) as u8];
    if let Some(disp) = relative_disp(operand, pbr, pc, 2)
        && let Ok(b) = i8::try_from(disp)
    {
        let byte = b.cast_unsigned();
        if !out.contains(&byte) {
            out.push(byte);
        }
    }
    out
}

/// Two-byte operand candidates: the literal value, and a long-branch displacement.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn two_byte_candidates(operand: i64, pc: u16) -> Vec<u16> {
    // Masked first, so the narrowing is exact rather than lossy.
    let mut out = vec![(operand & 0xFFFF) as u16];
    // `BRL`/`PER` are 16-bit PC-relative; the target's bank is the program bank by definition.
    let target = operand & 0xFFFF;
    let disp = target - (i64::from(pc) + 3);
    if let Ok(d) = i16::try_from(disp) {
        let word = d.cast_unsigned();
        if !out.contains(&word) {
            out.push(word);
        }
    }
    out
}

/// The PC-relative displacement from the instruction at `pc` (of length `len`) to `target`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn relative_disp(target: i64, pbr: u8, pc: u16, len: i64) -> Option<i64> {
    // A short branch cannot leave the bank, so a target in another bank is not a candidate at all.
    // Masked first, so the narrowing is exact.
    let target_bank = ((target >> 16) & 0xFF) as u8;
    if target != (target & 0xFFFF) && target_bank != pbr {
        return None;
    }
    Some((target & 0xFFFF) - (i64::from(pc) + len))
}

/// Whether these bytes disassemble to exactly the requested text.
fn matches(bytes: &[u8], want: &str, pbr: u8, pc: u16, m8: bool, x8: bool) -> bool {
    let (text, len) = decode(bytes, pbr, pc, m8, x8);
    len == bytes.len() && normalize(&text) == want
}

/// Run the real disassembler over `bytes` as if they sat at `pbr:pc`.
fn decode(bytes: &[u8], pbr: u8, pc: u16, m8: bool, x8: bool) -> (String, usize) {
    let base = (u32::from(pbr) << 16) | u32::from(pc);
    disassemble_one(
        |addr| {
            let idx = addr.wrapping_sub(base) as usize;
            bytes.get(idx).copied().unwrap_or(0)
        },
        pbr,
        pc,
        m8,
        x8,
    )
}

/// Normalize a line for comparison: uppercase, single-spaced, no comment, no trailing punctuation.
///
/// Both sides of every comparison go through this, so the assembler accepts whatever spacing and
/// case a user types without the disassembler having to emit it.
#[must_use]
pub fn normalize(line: &str) -> String {
    let without_comment = line.split(';').next().unwrap_or("");
    let mut out = String::with_capacity(without_comment.len());
    let mut last_space = true;
    for c in without_comment.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(c.to_ascii_uppercase());
            last_space = false;
        }
    }
    out.trim().to_string()
}

/// The mnemonic (first word) of a normalized line.
fn mnemonic_of(line: &str) -> &str {
    line.split(' ').next().unwrap_or("")
}

/// Extract the first numeric literal from an operand, in any base the disassembler prints or a user
/// types (`$hex`, `0xhex`, decimal).
fn parse_operand_value(want: &str) -> Option<i64> {
    let rest = want.split_once(' ')?.1;
    for (i, c) in rest.char_indices() {
        if c == '$' {
            let hex: String = rest[i + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .collect();
            return i64::from_str_radix(&hex, 16).ok();
        }
        if c.is_ascii_digit() {
            let dec: String = rest[i..].chars().take_while(char::is_ascii_digit).collect();
            return dec.parse().ok();
        }
    }
    None
}

/// If `want` is a branch whose target is simply too far, describe that specifically.
fn branch_range_error(want: &str, pbr: u8, pc: u16) -> Option<AsmError> {
    const SHORT: [&str; 8] = ["BRA", "BEQ", "BNE", "BCC", "BCS", "BPL", "BMI", "BVC"];
    let mnemonic = mnemonic_of(want);
    if !SHORT.contains(&mnemonic) {
        return None;
    }
    let target = parse_operand_value(want)?;
    let disp = relative_disp(target, pbr, pc, 2)?;
    (i8::try_from(disp).is_err()).then_some(AsmError::BranchOutOfRange {
        distance: disp,
        limit: 127,
    })
}

#[cfg(test)]
mod tests {
    use super::{AsmError, assemble, decode, normalize};

    /// The defining property: whatever this assembles, the real disassembler reads back as the
    /// same instruction. That is what makes a second opcode table unnecessary.
    #[test]
    fn assembly_round_trips_through_the_disassembler() {
        let cases = [
            ("NOP", 0x00_8000u32, true, true),
            ("LDA #$12", 0x00_8000, true, true),
            ("LDA #$1234", 0x00_8000, false, true),
            ("LDX #$34", 0x00_8000, true, true),
            ("STA $7E0000", 0x00_8000, true, true),
            ("JSR $9000", 0x00_8000, true, true),
            ("JSL $018000", 0x00_8000, true, true),
            ("RTS", 0x00_8000, true, true),
            ("SEP #$20", 0x00_8000, true, true),
            ("REP #$30", 0x00_8000, true, true),
        ];
        for (src, addr, m8, x8) in cases {
            #[allow(clippy::cast_possible_truncation)]
            let (pbr, pc) = ((addr >> 16) as u8, addr as u16);
            let bytes = assemble(src, pbr, pc, m8, x8)
                .unwrap_or_else(|e| panic!("{src} failed to assemble: {e}"));
            let (text, len) = decode(&bytes, pbr, pc, m8, x8);
            assert_eq!(len, bytes.len(), "{src}: length mismatch");
            assert_eq!(
                normalize(&text),
                normalize(src),
                "{src}: round-trip differs"
            );
        }
    }

    /// The `M`/`X` widths change the encoding length, and assembling with the wrong one is how a
    /// patch silently desynchronises everything after it.
    #[test]
    fn immediate_width_follows_the_m_and_x_flags() {
        let narrow = assemble("LDA #$12", 0x00, 0x8000, true, true).expect("m8");
        assert_eq!(narrow.len(), 2);
        let wide = assemble("LDA #$0012", 0x00, 0x8000, false, true).expect("m16");
        assert_eq!(wide.len(), 3);

        let x_narrow = assemble("LDX #$12", 0x00, 0x8000, true, true).expect("x8");
        assert_eq!(x_narrow.len(), 2);
        let x_wide = assemble("LDX #$0012", 0x00, 0x8000, true, false).expect("x16");
        assert_eq!(x_wide.len(), 3);
    }

    /// A branch operand is PC-relative, so the same source assembles to a different byte at a
    /// different address — the reason `assemble` takes one.
    #[test]
    fn branch_displacement_depends_on_the_address() {
        let at_8000 = assemble("BRA $8010", 0x00, 0x8000, true, true).expect("bra");
        let at_8004 = assemble("BRA $8010", 0x00, 0x8004, true, true).expect("bra");
        assert_eq!(at_8000.len(), 2);
        assert_eq!(at_8004.len(), 2);
        assert_ne!(
            at_8000[1], at_8004[1],
            "the displacement must account for the instruction's own address"
        );
        // And each still round-trips at its own address.
        for (bytes, pc) in [(at_8000, 0x8000u16), (at_8004, 0x8004)] {
            let (text, _) = decode(&bytes, 0x00, pc, true, true);
            assert_eq!(normalize(&text), "BRA $8010");
        }
    }

    /// A backwards branch is a negative displacement, which must encode as two's complement.
    #[test]
    fn backwards_branches_encode_negative_displacements() {
        let bytes = assemble("BRA $8000", 0x00, 0x8010, true, true).expect("bra back");
        assert_eq!(bytes.len(), 2);
        assert!(bytes[1] >= 0x80, "expected a negative displacement byte");
        let (text, _) = decode(&bytes, 0x00, 0x8010, true, true);
        assert_eq!(normalize(&text), "BRA $8000");
    }

    /// An unreachable branch is reported as *out of range*, not as an unknown encoding — that is
    /// the one failure the user fixes by moving the target rather than rewriting the line.
    #[test]
    fn out_of_range_branches_say_so() {
        let err = assemble("BRA $9000", 0x00, 0x8000, true, true).expect_err("too far");
        match err {
            AsmError::BranchOutOfRange { distance, limit } => {
                assert!(distance > 127, "distance {distance}");
                assert_eq!(limit, 127);
            }
            other => panic!("expected BranchOutOfRange, got {other}"),
        }
    }

    /// Input the assembler cannot encode is refused by name rather than silently producing
    /// something else.
    #[test]
    fn unencodable_input_is_refused() {
        assert_eq!(assemble("", 0, 0x8000, true, true), Err(AsmError::Empty));
        assert_eq!(
            assemble("   ; just a comment", 0, 0x8000, true, true),
            Err(AsmError::Empty)
        );
        assert!(matches!(
            assemble("FLOOB #$12", 0, 0x8000, true, true),
            Err(AsmError::NoEncoding(_))
        ));
        // A real mnemonic with an operand form it does not have.
        assert!(matches!(
            assemble("RTS #$12", 0, 0x8000, true, true),
            Err(AsmError::NoEncoding(_))
        ));
    }

    /// Whatever spacing and case the user types is accepted; a trailing comment is ignored.
    #[test]
    fn input_normalization_is_forgiving() {
        assert_eq!(normalize("  lda   #$12  ; load "), "LDA #$12");
        assert_eq!(normalize("NOP"), "NOP");
        assert_eq!(normalize(""), "");
        let a = assemble("lda #$12", 0, 0x8000, true, true).expect("lower");
        let b = assemble("LDA    #$12   ; comment", 0, 0x8000, true, true).expect("spaced");
        assert_eq!(a, b);
    }
}
