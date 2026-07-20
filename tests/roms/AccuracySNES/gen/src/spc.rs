//! A tiny SPC700 assembler, for Group E (ticket **T-04-E**).
//!
//! # Why the cart needs one
//!
//! The SPC700 is a separate processor with its own RAM, and the only channel between it and the
//! 65816 is four bytes. Nothing about it is testable from the main CPU directly — so a Group E
//! test uploads a small SPC700 program through the IPL boot handshake, lets it run, and reads its
//! answers back through those same four ports. Writing those programs means emitting SPC700
//! machine code, and `ca65` does not assemble SPC700.
//!
//! # Deliberately minimal
//!
//! One function per instruction the tests actually use, rather than a table-driven assembler for
//! all 256 opcodes. An encoder is only trustworthy where it is exercised, and a mostly-unused
//! table is mostly-unverified — a wrong byte in it would surface as an emulator disagreement
//! rather than as an assembler bug, which is the most expensive way to find it. Every opcode here
//! is used by a committed test, and the pair of tests that read PSW back cover the encoding
//! end to end.
//!
//! Opcode values are from the SPC700 opcode map in `ref-docs/`; each is spelled out at its
//! emitter so a reader can check one without holding the whole map in mind.

/// A program under construction.
#[derive(Default)]
pub struct Spc {
    bytes: Vec<u8>,
}

impl Spc {
    /// Start an empty program.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The assembled bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn push(&mut self, b: &[u8]) -> &mut Self {
        self.bytes.extend_from_slice(b);
        self
    }

    /// `MOV A,#imm` — `$E8`.
    pub fn mov_a_imm(&mut self, v: u8) -> &mut Self {
        self.push(&[0xE8, v])
    }

    /// `MOV X,#imm` — `$CD`.
    pub fn mov_x_imm(&mut self, v: u8) -> &mut Self {
        self.push(&[0xCD, v])
    }

    /// `MOV Y,#imm` — `$8D`.
    pub fn mov_y_imm(&mut self, v: u8) -> &mut Self {
        self.push(&[0x8D, v])
    }

    /// `MOV SP,X` — `$BD`. The IPL leaves the stack somewhere defined but unstated, and a test
    /// that pushes must not depend on that; every program here sets it.
    pub fn mov_sp_x(&mut self) -> &mut Self {
        self.push(&[0xBD])
    }

    /// `MOV dp,A` — `$C4`. Does **not** affect flags, which is what makes it safe for capturing a
    /// result before reading `PSW`.
    pub fn mov_dp_a(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xC4, dp])
    }

    /// `MOV dp,Y` — `$CB`. Also flag-free.
    pub fn mov_dp_y(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xCB, dp])
    }

    /// `MUL YA` — `$CF`. `YA = Y * A`.
    pub fn mul_ya(&mut self) -> &mut Self {
        self.push(&[0xCF])
    }

    /// `PUSH PSW` — `$0D`.
    pub fn push_psw(&mut self) -> &mut Self {
        self.push(&[0x0D])
    }

    /// `POP A` — `$AE`. Does not affect flags, so the popped `PSW` survives inspection.
    pub fn pop_a(&mut self) -> &mut Self {
        self.push(&[0xAE])
    }

    /// `DIV YA,X` — `$9E`. `A = YA / X`, `Y = YA % X`.
    pub fn div_ya_x(&mut self) -> &mut Self {
        self.push(&[0x9E])
    }

    /// `MOVW YA,dp` — `$BA`. Loads a 16-bit value; the flags describe all sixteen bits.
    pub fn movw_ya_dp(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xBA, dp])
    }

    /// `MOV dp,#imm` — `$8F`, encoded **immediate first, then the direct-page address**. That
    /// operand order is the reverse of how the mnemonic reads, and is worth stating rather than
    /// discovering: swapping the two assembles cleanly and stores the wrong byte somewhere else.
    pub fn mov_dp_imm(&mut self, dp: u8, v: u8) -> &mut Self {
        self.push(&[0x8F, v, dp])
    }

    /// `MOV A,dp` — `$E4`. Reading `$F4`-`$F7` from the SPC side returns what the **CPU** wrote;
    /// writing them sets what the CPU reads. One set of registers, two directions.
    pub fn mov_a_dp(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xE4, dp])
    }

    /// `CMP A,#imm` — `$68`.
    pub fn cmp_a_imm(&mut self, v: u8) -> &mut Self {
        self.push(&[0x68, v])
    }

    /// End the program: wait for the cart's release byte, then hand the APU back to the IPL.
    ///
    /// **Every program must end this way, and the reason is not tidiness.** Once a program is
    /// running, the IPL boot ROM is not — so the next test's upload has nothing to handshake with.
    /// The first version of this group ended in `BRA *`, and every APU test after the first one
    /// silently timed out and then read the *previous* test's leftover port values, which look
    /// exactly like a wrong answer rather than like a test that never ran.
    ///
    /// The cart writes [`RELEASE`] to port 0 once it has copied the results out; this polls for it
    /// and jumps to the IPL entry, which re-announces itself with `$AA`/`$BB` for the next upload.
    pub fn release_to_ipl(&mut self) -> &mut Self {
        // @wait: MOV A,$F4 / CMP A,#RELEASE / BNE @wait / JMP $FFC0
        // Built from the individual emitters rather than as a literal byte string, so the
        // encodings it relies on are the same ones every other program uses — an opcode spelled
        // out twice is an opcode that can disagree with itself. The branch offset is computed for
        // the same reason: a hand-counted `$FA` is right until an instruction moves.
        let wait = self.bytes.len();
        self.mov_a_dp(PORT0).cmp_a_imm(RELEASE);
        let after_branch = self.bytes.len() + 2;
        let rel = i64::try_from(wait).expect("program length fits i64")
            - i64::try_from(after_branch).expect("program length fits i64");
        let rel = i8::try_from(rel).expect("the release loop is far shorter than a branch's reach");
        // A branch displacement IS a signed byte reinterpreted as an opcode operand; the
        // two-s-complement bit pattern is the encoding, not a lossy conversion.
        self.push(&[0xD0, rel.to_le_bytes()[0]]); // BNE @wait
        self.push(&[0x5F, 0xC0, 0xFF]) // JMP $FFC0 — the IPL entry
    }

    /// Emit the program as a ca65 `.byte` directive block, wrapped at a readable width.
    #[must_use]
    pub fn as_ca65(&self, indent: &str) -> String {
        use core::fmt::Write as _;
        let mut s = String::new();
        for chunk in self.bytes.chunks(12) {
            let list = chunk
                .iter()
                .map(|b| format!("${b:02X}"))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(s, "{indent}.byte {list}");
        }
        s
    }
}

/// The APU port addresses as the SPC700 sees them. The 65816 sees the same four bytes at
/// `$2140`-`$2143`; they are one set of registers with two names, which is the entire
/// communication channel between the processors.
pub const PORT0: u8 = 0xF4;
/// See [`PORT0`].
pub const PORT1: u8 = 0xF5;
/// See [`PORT0`].
pub const PORT2: u8 = 0xF6;
/// See [`PORT0`].
pub const PORT3: u8 = 0xF7;

/// The marker a finished program writes to port 0, so the cart can tell "done" from "not started".
pub const DONE: u8 = 0x5A;

/// The byte the cart writes back to port 0 once it has copied the results out, releasing the
/// program to hand the APU back to the IPL. See [`Spc::release_to_ipl`].
pub const RELEASE: u8 = 0xA5;
