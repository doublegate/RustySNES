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

    /// `DBNZ dp,rel` — `$6E`. Decrements the direct-page byte and branches if it is not zero.
    ///
    /// `rel` is the displacement from the byte after the instruction, so `0` falls through whether
    /// or not the branch is taken — which is what `E2.04` wants: it is testing the *access
    /// pattern*, and a branch that went anywhere would be a second variable.
    pub fn dbnz_dp(&mut self, dp: u8, rel: i8) -> &mut Self {
        self.push(&[0x6E, dp, rel.to_le_bytes()[0]])
    }

    /// `MOV (X),A` — `$C6`. Stores `A` at the direct-page address `X` points at.
    ///
    /// The counterpart to [`Spc::or_a_x_ind`], and it exists for the same test: `E4.03` has to
    /// *dirty* the zero page before it can prove the IPL cleans it.
    pub fn mov_x_ind_a(&mut self) -> &mut Self {
        self.push(&[0xC6])
    }

    /// `OR A,(X)` — `$06`. Ors `A` with the direct-page byte `X` points at.
    ///
    /// The indirect form rather than `OR A,dp+X` because the index *is* the address here: `E4.03`
    /// sweeps the whole zero page accumulating a single OR, and only backward branches exist in
    /// this builder, so a loop that cannot exit early is the shape that fits.
    pub fn or_a_x_ind(&mut self) -> &mut Self {
        self.push(&[0x06])
    }

    /// `MOV A,dp+X` — `$F4`. The index wraps **within the direct page**, so `$FF + 2` is `$01`,
    /// not `$0101`.
    pub fn mov_a_dp_x(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xF4, dp])
    }

    /// `MOV !abs,A` — `$C5`. The only store here that reaches outside the direct page, which is
    /// what the voice tests need: a BRR sample directory lives at a page the DSP's `DIR` register
    /// names, and that page is deliberately not the one the program's variables are in.
    pub fn mov_abs_a(&mut self, addr: u16) -> &mut Self {
        let [lo, hi] = addr.to_le_bytes();
        self.push(&[0xC5, lo, hi])
    }

    /// Place `data` at the start of the program, jumped over, and return its APU RAM address.
    ///
    /// The S-DSP reads sample data out of APU RAM by address, so a test that plays a sample has to
    /// get bytes there and then *know where they are*. Putting them first solves the ordering
    /// problem: an address computed from the program's base is fixed before a single instruction is
    /// emitted, whereas data appended at the end moves every time the code above it changes.
    ///
    /// The skip is a `JMP` (`$5F`) rather than a `BRA` on purpose — a sample plus its run-out
    /// padding is far longer than a branch's reach, and the padding is not optional (see the
    /// voice tests: the DSP walks forward out of a non-looping sample into whatever follows it).
    ///
    /// # Panics
    ///
    /// If the program is not empty. The address it returns is only correct for data at the base.
    pub fn data_first(&mut self, base: u16, data: &[u8]) -> u16 {
        assert!(
            self.bytes.is_empty(),
            "data_first must be the first thing emitted; its address is the program's base"
        );
        let len = u16::try_from(data.len()).expect("a data block is far smaller than APU RAM");
        let entry = base + 3;
        let after = entry + len;
        let [lo, hi] = after.to_le_bytes();
        self.push(&[0x5F, lo, hi]); // JMP past the data
        self.push(data);
        entry
    }

    /// `MOV A,!abs+X` — `$F5`. The indexed absolute read the IPL-ROM checksum walks with.
    pub fn mov_a_abs_x(&mut self, addr: u16) -> &mut Self {
        let [lo, hi] = addr.to_le_bytes();
        self.push(&[0xF5, lo, hi])
    }

    /// `MOV dp,X` — `$D8`. Flag-free, like its `A` and `Y` counterparts.
    pub fn mov_dp_x(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xD8, dp])
    }

    /// `MOV A,Y` — `$DD`.
    pub fn mov_a_y(&mut self) -> &mut Self {
        self.push(&[0xDD])
    }

    /// `OR A,dp` — `$04`.
    pub fn or_a_dp(&mut self, dp: u8) -> &mut Self {
        self.push(&[0x04, dp])
    }

    /// `ADC A,dp` — `$84`. Adds the carry, so pair it with [`Spc::clrc`].
    pub fn adc_a_dp(&mut self, dp: u8) -> &mut Self {
        self.push(&[0x84, dp])
    }

    /// `CLRC` — `$60`.
    pub fn clrc(&mut self) -> &mut Self {
        self.push(&[0x60])
    }

    /// `ASL A` — `$1C`.
    pub fn asl_a(&mut self) -> &mut Self {
        self.push(&[0x1C])
    }

    /// `INC X` — `$3D`.
    pub fn inc_x(&mut self) -> &mut Self {
        self.push(&[0x3D])
    }

    /// `CMP X,#imm` — `$C8`.
    pub fn cmp_x_imm(&mut self, v: u8) -> &mut Self {
        self.push(&[0xC8, v])
    }

    /// The current offset, for [`Spc::bne_back`] to branch to.
    #[must_use]
    pub const fn here(&self) -> usize {
        self.bytes.len()
    }

    /// `BNE` back to a point recorded by [`Spc::here`].
    ///
    /// Backwards only, and the displacement is computed rather than written by hand — the same
    /// reasoning as in [`Spc::release_to_ipl`], where a hand-counted offset was right until an
    /// instruction moved.
    ///
    /// # Panics
    ///
    /// If the target is not already emitted, or is further back than a branch can reach. The
    /// first is what makes "backwards only" a contract rather than a comment: a forward
    /// displacement would assemble cleanly and jump into whatever gets emitted next, which is the
    /// kind of mistake that surfaces as an emulator disagreement rather than as a generator bug.
    pub fn bne_back(&mut self, target: usize) -> &mut Self {
        assert!(
            target <= self.bytes.len(),
            "bne_back target {target} is ahead of the current offset {}; this branch is backwards \
             only",
            self.bytes.len()
        );
        let after = self.bytes.len() + 2;
        let rel = i64::try_from(target).expect("offset fits i64")
            - i64::try_from(after).expect("offset fits i64");
        let rel = i8::try_from(rel).expect("branch target is out of reach");
        self.push(&[0xD0, rel.to_le_bytes()[0]])
    }

    /// `ADC A,#imm` — `$88`.
    pub fn adc_a_imm(&mut self, v: u8) -> &mut Self {
        self.push(&[0x88, v])
    }

    /// `CLRV` — `$E0`. Clears `V` **and** `H`, which is the whole point of the test that uses it.
    pub fn clrv(&mut self) -> &mut Self {
        self.push(&[0xE0])
    }

    /// `MOV dp,dp` — `$FA`. One of the two stores that does *not* dummy-read its destination.
    ///
    /// **This function takes `(dst, src)`, in mnemonic order.** The reversal is in the *encoding*
    /// only: the opcode's operand bytes are source first, then destination, which is why the `push`
    /// below looks swapped. Call it the way you would write the instruction.
    pub fn mov_dp_dp(&mut self, dst: u8, src: u8) -> &mut Self {
        self.push(&[0xFA, src, dst])
    }

    /// `MOVW dp,YA` — `$DA`. Writes two bytes, but dummy-reads only the **low** one, which is what
    /// makes it distinguishable from two separate stores.
    pub fn movw_dp_ya(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xDA, dp])
    }

    /// `INC dp` — `$AB`. A read-modify-write, and so a second kind of direct-page access from the
    /// one `MOV` exercises.
    pub fn inc_dp(&mut self, dp: u8) -> &mut Self {
        self.push(&[0xAB, dp])
    }

    /// `CLRP` — `$20`. Direct page moves to `$00xx`.
    pub fn clrp(&mut self) -> &mut Self {
        self.push(&[0x20])
    }

    /// `SETP` — `$40`. Direct page moves to `$01xx` — the same page the stack lives on.
    pub fn setp(&mut self) -> &mut Self {
        self.push(&[0x40])
    }

    /// `SETC` — `$80`.
    pub fn setc(&mut self) -> &mut Self {
        self.push(&[0x80])
    }

    /// `DAS` — `$BE`. Decimal-adjust after subtraction: the mirror of [`Spc::daa`], and it reads
    /// the *inverted* sense of both flags.
    pub fn das(&mut self) -> &mut Self {
        self.push(&[0xBE])
    }

    /// `DAA` — `$DF`. Decimal-adjust after addition.
    pub fn daa(&mut self) -> &mut Self {
        self.push(&[0xDF])
    }

    /// `TSET1 !abs` — `$0E`. Sets the bits of `A` in the target, and sets `N`/`Z` from a
    /// *comparison* of `A` against the target's old value rather than from the result.
    pub fn tset1_abs(&mut self, addr: u16) -> &mut Self {
        let [lo, hi] = addr.to_le_bytes();
        self.push(&[0x0E, lo, hi])
    }

    /// `CALL !abs` — `$3F`.
    pub fn call_abs(&mut self, addr: u16) -> &mut Self {
        let [lo, hi] = addr.to_le_bytes();
        self.push(&[0x3F, lo, hi])
    }

    /// `MOV A,!abs` — `$E5`. The read counterpart of [`Spc::mov_abs_a`].
    pub fn mov_a_abs(&mut self, addr: u16) -> &mut Self {
        let [lo, hi] = addr.to_le_bytes();
        self.push(&[0xE5, lo, hi])
    }

    /// `PUSH A` — `$2D`.
    pub fn push_a(&mut self) -> &mut Self {
        self.push(&[0x2D])
    }

    /// `POP PSW` — `$8E`. The only way to clear the `B` flag short of `RETI`.
    pub fn pop_psw(&mut self) -> &mut Self {
        self.push(&[0x8E])
    }

    /// `TCALL n` — `$n1`. Vectors through `[$FFDE - n*2]`.
    ///
    /// # Panics
    ///
    /// If `n` is above 15; there are sixteen vectors.
    pub fn tcall(&mut self, n: u8) -> &mut Self {
        assert!(n < 16, "TCALL takes a vector 0-15, not {n}");
        self.push(&[(n << 4) | 0x01])
    }

    /// `BRK` — `$0F`. Vectors through `$FFDE`, the same slot as `TCALL 0`.
    pub fn brk(&mut self) -> &mut Self {
        self.push(&[0x0F])
    }

    /// `CMP A,#imm` — `$68`.
    pub fn cmp_a_imm(&mut self, v: u8) -> &mut Self {
        self.push(&[0x68, v])
    }

    /// `ADDW YA,dp` — `$7A`. A true 16-bit add: `H` is the carry from bit 11 into bit 12, and `Z`
    /// describes all sixteen bits rather than either byte.
    pub fn addw_ya_dp(&mut self, dp: u8) -> &mut Self {
        self.push(&[0x7A, dp])
    }

    /// `XCN` (`$9F`) — exchange the two nibbles of `A`.
    ///
    /// One byte, and by far the most expensive register-only operation the SPC700 has at five
    /// cycles. `E1.14` measures that cost against [`Self::nop`].
    pub fn xcn(&mut self) -> &mut Self {
        self.push(&[0x9F])
    }

    /// `NOP` (`$00`) — two cycles, and the cheapest one-byte instruction there is.
    ///
    /// Used as the baseline a timed instruction is measured against: two blocks of the same length
    /// differ only by the per-instruction cost, so the loop and the timer plumbing cancel out.
    pub fn nop(&mut self) -> &mut Self {
        self.push(&[0x00])
    }

    /// Burn roughly `iters * 6` SPC700 cycles with a `DBNZ Y` loop (`$FE`).
    ///
    /// Used only where a test needs *time to pass* rather than a specific number of cycles — a
    /// timer to tick, say. Deliberately approximate: a test that depended on the exact count would
    /// be asserting this loop's cycle cost, which is not what it is for. `iters = 0` means 256,
    /// because `DBNZ` decrements before testing.
    pub fn delay(&mut self, iters: u8) -> &mut Self {
        self.mov_y_imm(iters);
        self.push(&[0xFE, 0xFE]) // DBNZ Y, -2: branch back to the DBNZ itself
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
        // Re-map the IPL ROM before jumping into it. `$F1` bit 7 controls whether `$FFC0`-`$FFFF`
        // reads as the boot ROM or as RAM, and any program that touched `$F1` for its own reasons
        // — enabling a timer, say — will have cleared it. Jumping to `$FFC0` then lands in zeroed
        // RAM, the SMP wanders off, and EVERY LATER UPLOAD FAILS, because there is no IPL left to
        // handshake with. That is exactly what happened: one timer test wrote `$F1 = $01`, and
        // every APU test after it silently died, which read as "the DSP is unreachable".
        self.mov_a_imm(0x80).mov_dp_a(0xF1);
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
