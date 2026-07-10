//! A minimal 65C816 disassembler for human-facing debug output.
//!
//! Decode-only: [`disassemble_one`] takes a byte-peek closure (`FnMut`, so a caller may plug in
//! a real `Bus::read24` when a side-effect-free peek accessor isn't convenient) and does nothing
//! but decode the bytes it's handed — it never touches CPU state and has no connection to
//! [`crate::exec`], the real cycle-accurate interpreter, so a bug here can never affect emulation
//! correctness, only what a debugger prints. Prefer a genuinely read-only peek where one exists;
//! the closure type doesn't enforce that, it only enables it. Built for the frontend's debugger
//! overlay (`docs/frontend.md` §Debugger overlay) and for ad hoc instruction-level tracing (e.g.
//! `docs/audit/`'s boot investigations).

use alloc::format;
use alloc::string::String;

/// Addressing mode, used only to compute operand length and format the operand string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Implied,
    Accumulator,
    /// Immediate, width follows the `M` (accumulator/memory) flag.
    ImmediateM,
    /// Immediate, width follows the `X` (index) flag.
    ImmediateX,
    /// Immediate, always one byte (`REP`/`SEP`/`COP`/`BRK`/`WDM`).
    Immediate8,
    Direct,
    DirectX,
    DirectY,
    DirectIndirect,
    DirectIndirectX,
    DirectIndirectY,
    DirectIndirectLong,
    DirectIndirectLongY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    AbsoluteIndirect,
    AbsoluteIndirectX,
    AbsoluteIndirectLong,
    AbsoluteLong,
    AbsoluteLongX,
    StackRelative,
    StackRelativeIndirectY,
    Relative,
    RelativeLong,
    BlockMove,
}

/// `(mnemonic, addressing mode)` for every one of the 256 opcodes — the fixed, standard WDC
/// 65C816 opcode map (identical across every 65816 reference/emulator; not derived from
/// `crate::exec`, which decodes the same map for execution rather than display).
#[rustfmt::skip]
const OPCODES: [(&str, Mode); 256] = {
    use Mode::{
        Absolute as A, AbsoluteIndirect as AI, AbsoluteIndirectLong as AIL,
        AbsoluteIndirectX as AIX, AbsoluteLong as AL, AbsoluteLongX as ALX, AbsoluteX as AX,
        AbsoluteY as AY, Accumulator as Acc, BlockMove as BM, Direct as D, DirectIndirect as DI,
        DirectIndirectLong as DIL, DirectIndirectLongY as DILY, DirectIndirectX as DIX,
        DirectIndirectY as DIY, DirectX as DX, DirectY as DY, Immediate8 as I8, ImmediateM as IM,
        ImmediateX as IX, Implied as Imp, Relative as Rel, RelativeLong as RelL,
        StackRelative as SR, StackRelativeIndirectY as SRIY,
    };
    [
        ("BRK", I8), ("ORA", DIX), ("COP", I8), ("ORA", SR), ("TSB", D), ("ORA", D), ("ASL", D), ("ORA", DIL),
        ("PHP", Imp), ("ORA", IM), ("ASL", Acc), ("PHD", Imp), ("TSB", A), ("ORA", A), ("ASL", A), ("ORA", AL),
        ("BPL", Rel), ("ORA", DIY), ("ORA", DI), ("ORA", SRIY), ("TRB", D), ("ORA", DX), ("ASL", DX), ("ORA", DILY),
        ("CLC", Imp), ("ORA", AY), ("INC", Acc), ("TCS", Imp), ("TRB", A), ("ORA", AX), ("ASL", AX), ("ORA", ALX),
        ("JSR", A), ("AND", DIX), ("JSL", AL), ("AND", SR), ("BIT", D), ("AND", D), ("ROL", D), ("AND", DIL),
        ("PLP", Imp), ("AND", IM), ("ROL", Acc), ("PLD", Imp), ("BIT", A), ("AND", A), ("ROL", A), ("AND", AL),
        ("BMI", Rel), ("AND", DIY), ("AND", DI), ("AND", SRIY), ("BIT", DX), ("AND", DX), ("ROL", DX), ("AND", DILY),
        ("SEC", Imp), ("AND", AY), ("DEC", Acc), ("TSC", Imp), ("BIT", AX), ("AND", AX), ("ROL", AX), ("AND", ALX),
        ("RTI", Imp), ("EOR", DIX), ("WDM", I8), ("EOR", SR), ("MVP", BM), ("EOR", D), ("LSR", D), ("EOR", DIL),
        ("PHA", Imp), ("EOR", IM), ("LSR", Acc), ("PHK", Imp), ("JMP", A), ("EOR", A), ("LSR", A), ("EOR", AL),
        ("BVC", Rel), ("EOR", DIY), ("EOR", DI), ("EOR", SRIY), ("MVN", BM), ("EOR", DX), ("LSR", DX), ("EOR", DILY),
        ("CLI", Imp), ("EOR", AY), ("PHY", Imp), ("TCD", Imp), ("JMP", AL), ("EOR", AX), ("LSR", AX), ("EOR", ALX),
        ("RTS", Imp), ("ADC", DIX), ("PER", RelL), ("ADC", SR), ("STZ", D), ("ADC", D), ("ROR", D), ("ADC", DIL),
        ("PLA", Imp), ("ADC", IM), ("ROR", Acc), ("RTL", Imp), ("JMP", AI), ("ADC", A), ("ROR", A), ("ADC", AL),
        ("BVS", Rel), ("ADC", DIY), ("ADC", DI), ("ADC", SRIY), ("STZ", DX), ("ADC", DX), ("ROR", DX), ("ADC", DILY),
        ("SEI", Imp), ("ADC", AY), ("PLY", Imp), ("TDC", Imp), ("JMP", AIX), ("ADC", AX), ("ROR", AX), ("ADC", ALX),
        ("BRA", Rel), ("STA", DIX), ("BRL", RelL), ("STA", SR), ("STY", D), ("STA", D), ("STX", D), ("STA", DIL),
        ("DEY", Imp), ("BIT", IM), ("TXA", Imp), ("PHB", Imp), ("STY", A), ("STA", A), ("STX", A), ("STA", AL),
        ("BCC", Rel), ("STA", DIY), ("STA", DI), ("STA", SRIY), ("STY", DX), ("STA", DX), ("STX", DY), ("STA", DILY),
        ("TYA", Imp), ("STA", AY), ("TXS", Imp), ("TXY", Imp), ("STZ", A), ("STA", AX), ("STZ", AX), ("STA", ALX),
        ("LDY", IX), ("LDA", DIX), ("LDX", IX), ("LDA", SR), ("LDY", D), ("LDA", D), ("LDX", D), ("LDA", DIL),
        ("TAY", Imp), ("LDA", IM), ("TAX", Imp), ("PLB", Imp), ("LDY", A), ("LDA", A), ("LDX", A), ("LDA", AL),
        ("BCS", Rel), ("LDA", DIY), ("LDA", DI), ("LDA", SRIY), ("LDY", DX), ("LDA", DX), ("LDX", DY), ("LDA", DILY),
        ("CLV", Imp), ("LDA", AY), ("TSX", Imp), ("TYX", Imp), ("LDY", AX), ("LDA", AX), ("LDX", AY), ("LDA", ALX),
        ("CPY", IX), ("CMP", DIX), ("REP", I8), ("CMP", SR), ("CPY", D), ("CMP", D), ("DEC", D), ("CMP", DIL),
        ("INY", Imp), ("CMP", IM), ("DEX", Imp), ("WAI", Imp), ("CPY", A), ("CMP", A), ("DEC", A), ("CMP", AL),
        ("BNE", Rel), ("CMP", DIY), ("CMP", DI), ("CMP", SRIY), ("PEI", D), ("CMP", DX), ("DEC", DX), ("CMP", DILY),
        ("CLD", Imp), ("CMP", AY), ("PHX", Imp), ("STP", Imp), ("JMP", AIL), ("CMP", AX), ("DEC", AX), ("CMP", ALX),
        ("CPX", IX), ("SBC", DIX), ("SEP", I8), ("SBC", SR), ("CPX", D), ("SBC", D), ("INC", D), ("SBC", DIL),
        ("INX", Imp), ("SBC", IM), ("NOP", Imp), ("XBA", Imp), ("CPX", A), ("SBC", A), ("INC", A), ("SBC", AL),
        ("BEQ", Rel), ("SBC", DIY), ("SBC", DI), ("SBC", SRIY), ("PEA", A), ("SBC", DX), ("INC", DX), ("SBC", DILY),
        ("SED", Imp), ("SBC", AY), ("PLX", Imp), ("XCE", Imp), ("JSR", AIX), ("SBC", AX), ("INC", AX), ("SBC", ALX),
    ]
};

/// Operand byte count (excludes the opcode byte itself).
const fn operand_len(mode: Mode, m8: bool, x8: bool) -> usize {
    match mode {
        Mode::Implied | Mode::Accumulator => 0,
        Mode::ImmediateM => {
            if m8 {
                1
            } else {
                2
            }
        }
        Mode::ImmediateX => {
            if x8 {
                1
            } else {
                2
            }
        }
        Mode::Immediate8
        | Mode::Direct
        | Mode::DirectX
        | Mode::DirectY
        | Mode::DirectIndirect
        | Mode::DirectIndirectX
        | Mode::DirectIndirectY
        | Mode::DirectIndirectLong
        | Mode::DirectIndirectLongY
        | Mode::StackRelative
        | Mode::StackRelativeIndirectY
        | Mode::Relative => 1,
        Mode::Absolute
        | Mode::AbsoluteX
        | Mode::AbsoluteY
        | Mode::AbsoluteIndirect
        | Mode::AbsoluteIndirectX
        | Mode::RelativeLong
        | Mode::BlockMove => 2,
        Mode::AbsoluteLong | Mode::AbsoluteLongX | Mode::AbsoluteIndirectLong => 3,
    }
}

/// Disassemble one instruction at `pbr:pc`.
///
/// Reads operand bytes via `peek` (a 24-bit-address byte accessor — prefer a genuinely
/// side-effect-free peek, e.g. `Bus::peek_*`, over a real bus `read` where one is available).
/// `m8`/`x8` are [`crate::regs::Regs::m8`]/[`crate::regs::Regs::x8`] (`true` = 8-bit width) —
/// needed because `LDA #`-style immediates are 1 or 2 operand bytes depending on them.
///
/// Returns `(text, length)`: a human-readable `"MNEMONIC operand"` string, and the total
/// instruction length in bytes (opcode + operand, always >= 1) so a caller can advance to the
/// next instruction without re-decoding.
///
/// # Panics
/// Never in practice: the only `unwrap`/slice-conversion is over a fixed 2-byte prefix of the
/// local 3-byte `op` buffer, never over caller-controlled data.
#[must_use]
pub fn disassemble_one(
    mut peek: impl FnMut(u32) -> u8,
    pbr: u8,
    pc: u16,
    m8: bool,
    x8: bool,
) -> (String, usize) {
    let base = (u32::from(pbr) << 16) | u32::from(pc);
    let opcode = peek(base);
    let (mnemonic, mode) = OPCODES[opcode as usize];
    let n = operand_len(mode, m8, x8);
    let mut op = [0u8; 3];
    for (i, byte) in op.iter_mut().enumerate().take(n) {
        let i = u32::try_from(i).unwrap_or(u32::MAX);
        *byte = peek(base.wrapping_add(1 + i) & 0x00FF_FFFF);
    }
    let text = match mode {
        Mode::Implied => String::from(mnemonic),
        Mode::Accumulator => format!("{mnemonic} A"),
        Mode::ImmediateM | Mode::ImmediateX | Mode::Immediate8 => {
            if n == 2 {
                format!(
                    "{mnemonic} #${:04X}",
                    u16::from(op[0]) | (u16::from(op[1]) << 8)
                )
            } else {
                format!("{mnemonic} #${:02X}", op[0])
            }
        }
        Mode::Direct => format!("{mnemonic} ${:02X}", op[0]),
        Mode::DirectX => format!("{mnemonic} ${:02X},X", op[0]),
        Mode::DirectY => format!("{mnemonic} ${:02X},Y", op[0]),
        Mode::DirectIndirect => format!("{mnemonic} (${:02X})", op[0]),
        Mode::DirectIndirectX => format!("{mnemonic} (${:02X},X)", op[0]),
        Mode::DirectIndirectY => format!("{mnemonic} (${:02X}),Y", op[0]),
        Mode::DirectIndirectLong => format!("{mnemonic} [${:02X}]", op[0]),
        Mode::DirectIndirectLongY => format!("{mnemonic} [${:02X}],Y", op[0]),
        Mode::StackRelative => format!("{mnemonic} ${:02X},S", op[0]),
        Mode::StackRelativeIndirectY => format!("{mnemonic} (${:02X},S),Y", op[0]),
        Mode::Absolute => format!(
            "{mnemonic} ${:04X}",
            u16::from(op[0]) | (u16::from(op[1]) << 8)
        ),
        Mode::AbsoluteX => format!(
            "{mnemonic} ${:04X},X",
            u16::from(op[0]) | (u16::from(op[1]) << 8)
        ),
        Mode::AbsoluteY => format!(
            "{mnemonic} ${:04X},Y",
            u16::from(op[0]) | (u16::from(op[1]) << 8)
        ),
        Mode::AbsoluteIndirect => format!(
            "{mnemonic} (${:04X})",
            u16::from(op[0]) | (u16::from(op[1]) << 8)
        ),
        Mode::AbsoluteIndirectX => format!(
            "{mnemonic} (${:04X},X)",
            u16::from(op[0]) | (u16::from(op[1]) << 8)
        ),
        Mode::AbsoluteIndirectLong => format!(
            "{mnemonic} [${:04X}]",
            u16::from(op[0]) | (u16::from(op[1]) << 8)
        ),
        Mode::AbsoluteLong => format!("{mnemonic} ${:02X}{:02X}{:02X}", op[2], op[1], op[0]),
        Mode::AbsoluteLongX => format!("{mnemonic} ${:02X}{:02X}{:02X},X", op[2], op[1], op[0]),
        Mode::Relative => {
            let rel = i16::from(op[0].cast_signed());
            let target = pc.wrapping_add(2).wrapping_add(rel.cast_unsigned());
            format!("{mnemonic} ${target:04X}")
        }
        Mode::RelativeLong => {
            let rel = i16::from_le_bytes([op[0], op[1]]);
            let target = pc.wrapping_add(3).wrapping_add(rel.cast_unsigned());
            format!("{mnemonic} ${target:04X}")
        }
        Mode::BlockMove => format!("{mnemonic} ${:02X},${:02X}", op[0], op[1]),
    };
    (text, 1 + n)
}

#[cfg(test)]
mod tests {
    use super::disassemble_one;

    fn rom(bytes: &'static [u8]) -> impl Fn(u32) -> u8 {
        move |addr: u32| bytes.get(addr as usize).copied().unwrap_or(0)
    }

    /// Like [`rom`], but `bytes[0]` is placed at address `base` instead of address `0` — needed
    /// whenever the test passes a non-zero `pc` (relative-branch tests compute their target from
    /// `pc`, so the opcode bytes must actually live there).
    fn rom_at(base: u32, bytes: &'static [u8]) -> impl Fn(u32) -> u8 {
        move |addr: u32| {
            addr.checked_sub(base)
                .and_then(|i| bytes.get(i as usize))
                .copied()
                .unwrap_or(0)
        }
    }

    #[test]
    fn decodes_implied() {
        let (text, len) = disassemble_one(rom(&[0xEA]), 0, 0, true, true);
        assert_eq!(text, "NOP");
        assert_eq!(len, 1);
    }

    #[test]
    fn decodes_immediate_8bit_accumulator() {
        // LDA #$42 with M=1 (8-bit accumulator).
        let (text, len) = disassemble_one(rom(&[0xA9, 0x42, 0xFF]), 0, 0, true, true);
        assert_eq!(text, "LDA #$42");
        assert_eq!(len, 2);
    }

    #[test]
    fn decodes_immediate_16bit_accumulator() {
        // LDA #$1234 with M=0 (16-bit accumulator).
        let (text, len) = disassemble_one(rom(&[0xA9, 0x34, 0x12]), 0, 0, false, true);
        assert_eq!(text, "LDA #$1234");
        assert_eq!(len, 3);
    }

    #[test]
    fn decodes_absolute() {
        let (text, len) = disassemble_one(rom(&[0xAD, 0x00, 0x42]), 0, 0, true, true);
        assert_eq!(text, "LDA $4200");
        assert_eq!(len, 3);
    }

    #[test]
    fn decodes_absolute_long() {
        let (text, len) = disassemble_one(rom(&[0xAF, 0x00, 0x80, 0xC0]), 0, 0, true, true);
        assert_eq!(text, "LDA $C08000");
        assert_eq!(len, 4);
    }

    #[test]
    fn decodes_direct_page_indexed() {
        let (text, len) = disassemble_one(rom(&[0xB5, 0x10]), 0, 0, true, true);
        assert_eq!(text, "LDA $10,X");
        assert_eq!(len, 2);
    }

    #[test]
    fn decodes_forward_branch() {
        // BNE $+4 from PC=$1000 -> target = 1000 + 2 + 2 = $1004.
        let (text, len) = disassemble_one(rom_at(0x1000, &[0xD0, 0x02]), 0, 0x1000, true, true);
        assert_eq!(text, "BNE $1004");
        assert_eq!(len, 2);
    }

    #[test]
    fn decodes_backward_branch() {
        // BRA $-2 from PC=$1000 -> target = 1000 + 2 - 2 = $1000 (branch to self).
        let (text, len) = disassemble_one(rom_at(0x1000, &[0x80, 0xFE]), 0, 0x1000, true, true);
        assert_eq!(text, "BRA $1000");
        assert_eq!(len, 2);
    }

    #[test]
    fn decodes_block_move() {
        let (text, len) = disassemble_one(rom(&[0x54, 0x00, 0x7E]), 0, 0, true, true);
        assert_eq!(text, "MVN $00,$7E");
        assert_eq!(len, 3);
    }

    #[test]
    fn decodes_relative_long() {
        // BRL $+4 from PC=$2000 -> target = 2000 + 3 + 4 = $2007.
        let (text, len) =
            disassemble_one(rom_at(0x2000, &[0x82, 0x04, 0x00]), 0, 0x2000, true, true);
        assert_eq!(text, "BRL $2007");
        assert_eq!(len, 3);
    }

    #[test]
    fn every_opcode_decodes_without_panicking() {
        for opcode in 0u8..=255 {
            let bytes: [u8; 4] = [opcode, 0, 0, 0];
            let (text, len) = disassemble_one(
                move |addr: u32| bytes.get(addr as usize).copied().unwrap_or(0),
                0,
                0,
                true,
                true,
            );
            assert!(!text.is_empty());
            assert!((1..=4).contains(&len));
        }
    }
}
