//! `A6.15` — every 65C816 opcode is defined, and only `STP` hangs.
//!
//! # What the row claims
//!
//! The 65C816 has no undefined opcodes. Unlike the NMOS 6502, whose map is full of jams and
//! unintended combinations, every one of the 256 bytes is a documented instruction with a
//! documented length — and exactly one of them, `STP`, stops the processor until `RESET`.
//!
//! Testing that means **executing** each opcode and proving control comes back at the right place.
//! The cart builds a one-opcode sandbox in WRAM, jumps into it, and counts three outcomes: the
//! opcode returned where its documented length says it should, it returned late, or it did not
//! return at all. The expectation — the length — comes from Table 5-4 of the WDC datasheet via
//! [`crate::cpu_opcodes`], not from `rustysnes-cpu`.
//!
//! # The sandbox
//!
//! ```text
//! BUF+0:        <opcode + safe operand bytes>   ; L bytes, L from the table
//! BUF+L:        JMP $AAAA                       ; $4C $AA $AA  -> the CLEAN exit
//! BUF+L+3..15:  $EA fill                        ; NOP
//! BUF+16:       JMP $B8B8                       ; $4C $B8 $B8  -> the OVERSHOOT exit
//! ```
//!
//! **The exit addresses are chosen so the `JMP`'s own operand bytes are harmless instructions.**
//! `$AA` is `TAX` and `$B8` is `CLV`; both are one byte and touch nothing the sandbox needs. So an
//! opcode that consumes one or two bytes too many lands *inside* the terminator's operand, executes
//! a register transfer, and walks into the `NOP` fill — reaching the overshoot exit rather than
//! doing something unpredictable. Without that property a one-byte overshoot would execute half an
//! address as an instruction.
//!
//! | PC advance | lands on | outcome |
//! |---|---|---|
//! | exactly `L` | `JMP $AAAA` | **OK** |
//! | `L+1` .. `L+15` | `TAX`, or a `NOP` in the fill | **OVERSHOOT**, with the opcode recorded |
//! | `< L`, or `> L+15` | the opcode's own operand bytes, or anywhere | **NO RETURN** — the watchdog |
//!
//! # Why the terminator cannot be `RTS` or `RTL`
//!
//! A one-byte return would be tidier, and it does not work: `TXS` and `TCS` move the stack pointer,
//! and with `x = 1` a `TXS` puts it in page zero — so the return address is no longer where a
//! return would pop it from. Control has to come back without the stack, which means a `JMP`, which
//! means three bytes, which is what makes the operand-byte choice above load-bearing.
//!
//! # The watchdog takes two strikes, not one
//!
//! A battery whose entire value is that it always reports cannot execute arbitrary opcodes with no
//! way back. `runtime.s` carries an NMI trampoline with a settable vector, put there for the
//! `WAI`/`STP` and mid-block-move rows; the ordinary battery keeps `NMITIMEN` clear and polls
//! `$4212`, so it is free.
//!
//! One strike would be wrong. NMI fires once per vblank, and across 241 sandbox runs vblank will
//! eventually land inside a **healthy** one — so "an NMI arrived while a sandbox was active" does
//! not mean "stuck". The handler counts hits per opcode and abandons only on the second: a healthy
//! sandbox is microseconds and can be caught at most once; a jammed one is caught every frame.
//!
//! # Operand safety
//!
//! `DBR = $7E` for the whole sweep, so every absolute operand is WRAM rather than a `DBR = $00`
//! address that could land in MMIO — the hazard `sweep.rs` already documents. `D = $0200` puts
//! direct-page operands in the low-WRAM mirror, clear of the runtime's own variables at `$00`-`$5F`.
//! The window is `$5000`, well away from the sandbox at `$6000`.
//!
//! Four opcodes are dangerous even when they execute correctly, and each is handled in the preamble
//! rather than excluded:
//!
//! - **`MVN`/`MVP`** move `A + 1` bytes. `A = 0` makes that one byte, from `$7E:0000` to itself.
//! - **`XCE`** flips to emulation mode only if `C` is set. `CLC` first makes it a no-op in native
//!   mode — it swaps a clear `C` with a clear `E`.
//! - **`TXS`/`TCS`** move the stack pointer, which is why the exits restore it from WRAM.
//! - **`SED`** leaves decimal mode set, and **`PLP`** can set anything at all; the exits re-establish
//!   `m`, `x`, `d` and `c` explicitly rather than trusting what came back.

use crate::cpu_opcodes::{Flow, table};
use crate::dsl::{Asm, Kind, Provenance, Test};

/// Where the sandbox is assembled. Bank `$7E`, clear of `A8.07`'s block-move source which ends at
/// `$7E:3FFF`.
const BUF: u16 = 0x6000;

/// Where every memory operand points.
const WINDOW: u16 = 0x5000;

/// The clean exit. `$AA` is `TAX` — see the module docs on why the address's own bytes matter.
const EXIT_OK: u16 = 0xAAAA;

/// The overshoot exit. `$B8` is `CLV`.
const EXIT_OVER: u16 = 0xB8B8;

/// Bytes from the start of the sandbox to the overshoot terminator. Four for the longest opcode
/// plus three for the clean terminator leaves nine bytes of fill, which is slack rather than a
/// bound — nothing is expected to land in it.
const OVERSHOOT_AT: u16 = 16;

/// Direct page during a sandbox run: the low-WRAM mirror, clear of the runtime's variables.
const SANDBOX_DP: u16 = 0x0200;

/// Stack pointer during a sandbox run. In page 1, so a stack-relative operand and anything the
/// sandbox pulls stay inside WRAM — see the preamble in [`run_sandbox`] for why the cart's own
/// `$1FFF` is not safe here.
const SANDBOX_SP: u16 = 0x01F0;

/// The operand window must not overlap the sandbox — an absolute store landing there would rewrite
/// the very bytes under test. A compile-time assertion rather than a unit test, because both sides
/// are constants and there is no reason to let a bad pair get as far as a test run.
const _: () = assert!(
    WINDOW + 0x100 <= BUF || BUF + OVERSHOOT_AT + 3 <= WINDOW,
    "the operand window overlaps the sandbox"
);

/// Scratch for the driver, immediately above the sandbox.
///
/// **Not in low WRAM**, and that cost a debugging cycle twice over. `$7E:0170` upward looked free
/// and is used by a `bus.rs` row — WRAM scratch has no allocator and no collision gate, unlike the
/// measurement channel. And any address in `$7E:0200`-`$02FF` would have been inside the sandbox's
/// own **direct page**, so the first opcode with a direct-page operand would overwrite the driver's
/// state with its own test data. `the_scratch_is_clear_of_everything_the_sandbox_touches`
/// is the gate that would have caught both.
mod var {
    /// The opcode under test.
    pub const OP: &str = "$7E6100";
    /// Its documented length.
    pub const LEN: &str = "$7E6101";
    /// The stack pointer to put back on the way out.
    pub const SAVED_SP: &str = "$7E6102";
    /// Non-zero while a sandbox is in flight, so the NMI handler can tell.
    pub const ACTIVE: &str = "$7E6104";
    /// NMI hits against the current opcode. The watchdog abandons on the second.
    pub const HITS: &str = "$7E6105";
    /// Opcodes that returned exactly where their length says.
    pub const OK: &str = "$7E6106";
    /// Opcodes that returned late.
    pub const OVER: &str = "$7E6107";
    /// Opcodes that did not return.
    pub const STUCK: &str = "$7E6108";
    /// The first opcode that was not OK, or `$FF`.
    pub const FIRST_BAD: &str = "$7E6109";
    /// The opcode index, saved across a run because most opcodes clobber `X`.
    pub const SAVED_X: &str = "$7E610A";
}

/// The first byte of [`var`], for the range checks.
#[cfg(test)]
const SCRATCH: u16 = 0x6100;

/// How many bytes [`var`] spans.
#[cfg(test)]
const SCRATCH_LEN: u16 = 0x10;

/// Opcodes the sweep executes: 256 less the fifteen that leave the sandbox.
const EXECUTED: u16 = 241;

/// Build the four ROM tables the driver indexes with the opcode in `X`.
///
/// A length of zero means "not executed", which is how the fifteen control-transfer opcodes are
/// skipped without a second table.
fn tables(a: &mut Asm) {
    let ops = table();
    let mut len = Vec::with_capacity(256);
    let mut bytes: [Vec<u8>; 4] = [const { Vec::new() }; 4];
    for op in &ops {
        let skipped = matches!(op.flow, Flow::Leaves(_));
        len.push(if skipped { 0 } else { op.len() });
        let enc = operand_bytes(op.code, op.mode);
        for (i, b) in enc.iter().enumerate() {
            bytes[i].push(*b);
        }
    }

    a.d("a6_15_len:");
    emit_bytes(a, &len);
    for (i, table) in bytes.iter().enumerate() {
        a.d(&format!("a6_15_b{i}:"));
        emit_bytes(a, table);
    }
}

/// Emit a 256-byte table as `.byte` lines.
fn emit_bytes(a: &mut Asm, data: &[u8]) {
    for chunk in data.chunks(16) {
        let list = chunk
            .iter()
            .map(|b| format!("${b:02X}"))
            .collect::<Vec<_>>()
            .join(",");
        a.d(&format!("    .byte {list}"));
    }
}

/// The four encoded bytes of one opcode, operands filled in safely.
///
/// Unused trailing bytes are `$EA`, so a table read past the instruction's real length lands on a
/// `NOP` rather than on whatever happened to be there.
const fn operand_bytes(code: u8, mode: crate::cpu_opcodes::Mode) -> [u8; 4] {
    use crate::cpu_opcodes::Mode;
    let [wlo, whi] = WINDOW.to_le_bytes();
    let mut out = [0xEA; 4];
    out[0] = code;
    match mode {
        Mode::Implied => {}
        // A direct-page offset into the seeded window page. `$10`/`$12` hold the indirect pointers.
        Mode::Direct | Mode::StackRelative => out[1] = 0x10,
        // An immediate value of zero, and — for a branch — a zero DISPLACEMENT, which is what makes
        // the taken path land on the following instruction, exactly where the not-taken path goes.
        // A branch cannot escape the sandbox either way.
        Mode::Relative | Mode::Immediate => out[1] = 0x00,
        Mode::RelativeLong => {
            out[1] = 0x00;
            out[2] = 0x00;
        }
        Mode::Absolute => {
            out[1] = wlo;
            out[2] = whi;
        }
        Mode::Long => {
            out[1] = wlo;
            out[2] = whi;
            out[3] = 0x7E;
        }
        // `MVN`/`MVP` encode destination then source. Both banks are `$7E`, and `A = 0` in the
        // preamble makes the move exactly one byte.
        Mode::BlockMove => {
            out[1] = 0x7E;
            out[2] = 0x7E;
        }
    }
    out
}

/// `A6.15`.
#[allow(clippy::too_many_lines)]
pub fn a6_15() -> Test {
    let mut a = Asm::new();
    tables(&mut a);

    a.c("The two exit stubs and the NMI watchdog are written into WRAM before the sweep starts.");
    a.l("bra @body");
    watchdog(&mut a);
    a.label("body");
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");

    a.c("Install the NMI handler and arm VBlank NMI. NMI ignores the I flag, so an opcode inside");
    a.c("the sandbox that runs SEI cannot disarm the thing that rescues it.");
    a.l("rep #$20");
    a.l("lda #@nmi");
    a.l("sta a:V_NMI_VEC");

    a.c(
        "Counters, and a first-bad of $00 meaning `none`. $00 is BRK, which is in the set this row",
    );
    a.c("does NOT execute, so it can never be a real answer — where $FF, the obvious poison, is");
    a.c("SBC long,X and very much can be. `The sweep never ran` is caught by the liveness assertion");
    a.c("below, not by this slot.");
    a.c("STZ has no long-addressing form, so every clear here is an explicit LDA #$00 + STA.");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l(&format!("sta f:{}", var::OK));
    a.l(&format!("sta f:{}", var::OVER));
    a.l(&format!("sta f:{}", var::STUCK));
    a.l(&format!("sta f:{}", var::ACTIVE));
    a.l(&format!("sta f:{}", var::FIRST_BAD));

    emit_exit_stubs(&mut a);

    a.l("sep #$20");
    a.l("lda #$80");
    a.l("sta $4200         ; VBlank NMI on — the watchdog's clock");

    a.l("rep #$30");
    a.l("ldx #$0000");
    a.label("oploop");
    a.c("len = a6_15_len[X]. Zero means the opcode leaves the sandbox and is not executed.");
    a.l("sep #$20");
    a.l("lda f:a6_15_len,x");
    a.l(&format!("sta f:{}", var::LEN));
    a.c("Inverted over a JMP: the body between here and @next is far beyond a branch's reach.");
    a.l("bne :+");
    a.l("jmp @next");
    a.l(":");
    a.l("rep #$20");
    a.l("txa");
    a.l("sep #$20");
    a.l(&format!("sta f:{}", var::OP));

    build_sandbox(&mut a);
    run_sandbox(&mut a);

    a.label("next");
    a.l("rep #$30");
    a.l("inx");
    a.l("cpx #$0100");
    a.l("beq :+");
    a.l("jmp @oploop");
    a.l(":");

    a.c("Disarm before asserting: a failure exits immediately and must not leave NMI armed for");
    a.c("whatever runs next.");
    a.l("sep #$20");
    a.l("stz $4200");
    a.l("lda $4210         ; clear any pending RDNMI latch");

    report(a)
}

/// The NMI handler: the watchdog that gets control back when an opcode does not return.
fn watchdog(a: &mut Asm) {
    a.label("nmi");
    a.c("Long addressing throughout: the sandbox may have left DBR and D anywhere, and this");
    a.c("handler runs before anything has put them back.");
    a.c("A is preserved. RTI restores P and PC but not the accumulator, and an interrupt that");
    a.c("silently rewrites A is not transparent to the thing it interrupted.");
    a.l("rep #$30");
    a.l("pha");
    a.l("sep #$20");
    a.l(".a8");
    a.c("LONG addressing, and that is the whole bug this handler shipped with. It runs with the");
    a.c("SANDBOX's DBR, which is $7E — so `lda $4210` reads $7E:4210, a WRAM byte, and RDNMI is");
    a.c("never acknowledged. Every other host happened not to land an NMI where it showed; ares");
    a.c("did, and the row read as a PLA divergence that was nothing of the kind.");
    a.l("lda f:$004210    ; acknowledge RDNMI, DBR or no DBR");
    a.l(&format!("lda f:{}", var::ACTIVE));
    a.l("beq @nmi_out      ; no sandbox in flight — nothing to rescue");
    a.l(&format!("lda f:{}", var::HITS));
    a.l("inc a");
    a.l(&format!("sta f:{}", var::HITS));
    a.l("cmp #$02");
    a.l("bcc @nmi_out      ; first hit: a healthy sandbox can be caught once by chance");
    a.c("Second hit on the same opcode. It is not coming back — abandon the interrupt frame");
    a.c("entirely and re-enter the driver at the stuck exit.");
    a.l("jml @stuck_entry");
    a.label("nmi_out");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("pla");
    a.l("rti");
}

/// Emit the two exit stubs as a ROM table, then copy them into WRAM.
///
/// They have to live at fixed addresses whose *own bytes* are benign instructions (see the module
/// docs), which rules out assembling them in place — so they are sixteen bytes of `.byte` each,
/// with ca65 filling in the return address, copied into `$7E:AAAA` and `$7E:B8B8` before the sweep.
///
/// Each restores rather than assumes, because the sandbox may have run `PLP`, `PLD`, `PLB` or
/// `TXS`: `SEP #$30`, `CLD`, `CLC`, `REP #$30`, put the saved stack pointer back, put the direct
/// page back, then `JML` into the driver. `DBR` is left alone — the stub runs in bank `$7E` so `PHK`/`PLB`
/// there would set the wrong one; the driver does it on arrival instead.
fn emit_exit_stubs(a: &mut Asm) {
    for name in ["ok", "over"] {
        a.d(&format!("a6_15_stub_{name}:"));
        a.d("    .byte $E2,$30            ; SEP #$30");
        a.d("    .byte $D8                ; CLD");
        a.d("    .byte $18                ; CLC");
        a.d("    .byte $C2,$30            ; REP #$30");
        a.d(&format!(
            "    .byte $AF,${:02X},${:02X},${:02X}      ; LDA {} — the saved stack pointer",
            SAVED_SP_ADDR & 0xFF,
            (SAVED_SP_ADDR >> 8) & 0xFF,
            (SAVED_SP_ADDR >> 16) & 0xFF,
            var::SAVED_SP
        ));
        a.d("    .byte $1B                ; TCS");
        a.d("    .byte $A9,$00,$00        ; LDA #$0000");
        a.d("    .byte $5B                ; TCD — the runtime's variables live at D = 0");
        a.d("    .byte $5C,$00,$00,$00    ; JML — the target is patched in below");
    }

    a.c("Copy both stubs into the fixed WRAM addresses the terminators jump to.");
    for (name, addr) in [("ok", EXIT_OK), ("over", EXIT_OVER)] {
        a.l("rep #$30");
        a.l("ldx #$0000");
        a.label(&format!("copy_{name}"));
        a.l("sep #$20");
        a.l(&format!("lda f:a6_15_stub_{name},x"));
        a.l(&format!("sta f:$7E{addr:04X},x"));
        a.l("rep #$30");
        a.l("inx");
        a.l(&format!("cpx #${STUB_LEN:04X}"));
        a.l(&format!("bne @copy_{name}"));
    }

    a.c(
        "Patch each JML's target. It cannot be assembled into the .byte table: that table lives in",
    );
    a.c("the data segment, where this proc's cheap-local labels are out of scope.");
    for (addr, target) in [(EXIT_OK, "@ok_entry"), (EXIT_OVER, "@over_entry")] {
        a.l("sep #$20");
        a.l(&format!("lda #.lobyte({target})"));
        a.l(&format!("sta f:$7E{:04X}", addr + JML_TARGET_AT));
        a.l(&format!("lda #.hibyte({target})"));
        a.l(&format!("sta f:$7E{:04X}", addr + JML_TARGET_AT + 1));
    }
}

/// Offset of the `JML`'s 16-bit target within a stub.
const JML_TARGET_AT: u16 = 16;

/// Bytes in one exit stub. Pinned as a constant because the copy loop counts them, and checked
/// against the emitted table by `the_stub_length_matches_what_is_emitted` — a miscount
/// would copy a truncated stub into WRAM and the first opcode to reach it would run off the end.
const STUB_LEN: u16 = 19;

/// [`var::SAVED_SP`] as a number, for the `LDA long` the stub carries as raw bytes.
const SAVED_SP_ADDR: u32 = 0x007E_6102;

/// Assemble the one-opcode sandbox for the opcode in `X`.
fn build_sandbox(a: &mut Asm) {
    a.c("Copy the four encoded bytes; only the first `len` of them are reached, and the rest are");
    a.c("overwritten by the terminator.");
    a.l("rep #$30");
    a.l("phx");
    for i in 0..4u16 {
        a.l("rep #$30");
        a.l("plx");
        a.l("phx");
        a.l("sep #$20");
        a.l(&format!("lda f:a6_15_b{i},x"));
        a.l("rep #$30");
        a.l(&format!("ldx #${:04X}", BUF + i));
        a.l("sep #$20");
        a.l("sta f:$7E0000,x");
    }
    a.l("rep #$30");
    a.l("plx");

    a.c("The clean terminator at BUF+len, then NOP fill, then the overshoot terminator.");
    a.l("rep #$30");
    a.l("phx");
    a.l("sep #$20");
    a.l(&format!("lda f:{}", var::LEN));
    a.l("rep #$30");
    a.l("and #$00FF");
    a.l("clc");
    a.l(&format!("adc #${BUF:04X}"));
    a.l("tax");
    a.l("sep #$20");
    a.l("lda #$4C");
    a.l("sta f:$7E0000,x");
    a.l("rep #$30");
    a.l("inx");
    a.l("sep #$20");
    a.l(&format!("lda #${:02X}", EXIT_OK & 0xFF));
    a.l("sta f:$7E0000,x");
    a.l("rep #$30");
    a.l("inx");
    a.l("sep #$20");
    a.l(&format!("lda #${:02X}", EXIT_OK >> 8));
    a.l("sta f:$7E0000,x");

    a.c("Fill from there to the overshoot terminator with NOP, then write it.");
    a.l("rep #$30");
    a.l("inx");
    a.label("fill");
    a.l(&format!("cpx #${:04X}", BUF + OVERSHOOT_AT));
    a.l("bcs @filled");
    a.l("sep #$20");
    a.l("lda #$EA");
    a.l("sta f:$7E0000,x");
    a.l("rep #$30");
    a.l("inx");
    a.l("bra @fill");
    a.label("filled");
    a.l("sep #$20");
    a.l("lda #$4C");
    a.l(&format!("sta f:$7E{:04X}", BUF + OVERSHOOT_AT));
    a.l(&format!("lda #${:02X}", EXIT_OVER & 0xFF));
    a.l(&format!("sta f:$7E{:04X}", BUF + OVERSHOOT_AT + 1));
    a.l(&format!("lda #${:02X}", EXIT_OVER >> 8));
    a.l(&format!("sta f:$7E{:04X}", BUF + OVERSHOOT_AT + 2));
    a.l("rep #$30");
    a.l("plx");
}

/// Seed the window, take the machine into its known state, and jump into the sandbox.
fn run_sandbox(a: &mut Asm) {
    a.c("ONE pointer at D+$10, seeded as a 24-BIT $7E:5000. The 16-bit indirects `(dp)`/`(dp),Y`");
    a.c("read its first two bytes and take the bank from DBR, which the preamble sets to $7E; the");
    a.c("long indirects `[dp]`/`[dp],Y` read all three. The first draft seeded two pointers and");
    a.c("gave the long one a bank byte of $00, so `[dp]` reached $00:5000 — unmapped, not WRAM.");
    a.l("rep #$30");
    a.l(&format!("lda #${WINDOW:04X}"));
    a.l(&format!("sta f:$7E{:04X}", SANDBOX_DP + 0x10));
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l(&format!(
        "sta f:$7E{:04X}     ; the pointer's BANK byte",
        SANDBOX_DP + 0x12
    ));

    a.c("Save X across the run — a great many opcodes clobber it — and mark the sandbox active.");
    a.c("Through A: only the accumulator has long addressing, so X cannot be saved directly.");
    a.l("rep #$30");
    a.l("txa");
    a.l(&format!("sta f:{}", var::SAVED_X));
    a.l("sep #$20");
    a.l("lda #$01");
    a.l(&format!("sta f:{}", var::ACTIVE));
    a.l("lda #$00");
    a.l(&format!("sta f:{}", var::HITS));
    a.l("rep #$30");
    a.l("tsc");
    a.l(&format!("sta f:{}", var::SAVED_SP));

    a.c("The preamble is the whole of the danger handling. A = 0 makes MVN/MVP a one-byte move;");
    a.c("CLC makes XCE a no-op in native mode; DBR = $7E keeps every absolute operand in WRAM;");
    a.c("D = $0200 puts direct-page operands in the low-WRAM mirror, clear of the runtime's own.");
    a.c("SP moves into page 1 as well, and that is not cosmetic: the cart's own stack sits at");
    a.c("$1FFF, so a stack-relative operand of $10 would address $00:200F and a PLA would read");
    a.c("$00:2000 — both outside WRAM and into the unmapped/MMIO region. The exits restore the");
    a.c("cart's stack pointer from SAVED_SP, which was captured before this.");
    a.l(&format!("lda #${SANDBOX_SP:04X}"));
    a.l("tcs");
    a.l(&format!("lda #${SANDBOX_DP:04X}"));
    a.l("tcd");
    a.l("sep #$20");
    a.l("lda #$7E");
    a.l("pha");
    a.l("plb");
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l("ldx #$0000");
    a.l("ldy #$0000");
    a.l("sep #$30");
    a.l(".a8");
    a.l(".i8");
    a.l("cld");
    a.l("clc");
    a.l(&format!("jml $7E{BUF:04X}"));

    exits(a);
}

/// The three ways back out of a sandbox.
///
/// Split out of [`run_sandbox`] because the entry sequences are the bulk of it and none of them is
/// about getting *into* the sandbox.
fn exits(a: &mut Asm) {
    a.label("ok_entry");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l(&format!("sta f:{}", var::ACTIVE));
    a.l(&format!("lda f:{}", var::OK));
    a.l("inc a");
    a.l(&format!("sta f:{}", var::OK));
    a.l("rep #$30");
    a.l(&format!("lda f:{}", var::SAVED_X));
    a.l("tax");
    a.l("jmp @next");

    a.label("over_entry");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda #$00");
    a.l(&format!("sta f:{}", var::ACTIVE));
    a.l(&format!("lda f:{}", var::OVER));
    a.l("inc a");
    a.l(&format!("sta f:{}", var::OVER));
    a.l("jsr @note_bad");
    a.l("rep #$30");
    a.l(&format!("lda f:{}", var::SAVED_X));
    a.l("tax");
    a.l("jmp @next");

    a.label("stuck_entry");
    a.c("Entered from the NMI handler, so the stack still holds an interrupt frame. Rebuilding");
    a.c("the machine from saved state rather than returning is the point.");
    a.l("rep #$30");
    a.l(".a16");
    a.l(".i16");
    a.l(&format!("lda f:{}", var::SAVED_SP));
    a.l("tcs");
    a.l("lda #$0000");
    a.l("tcd");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("cld");
    a.l("lda #$00");
    a.l(&format!("sta f:{}", var::ACTIVE));
    a.l(&format!("lda f:{}", var::STUCK));
    a.l("inc a");
    a.l(&format!("sta f:{}", var::STUCK));
    a.l("jsr @note_bad");
    a.l("rep #$30");
    a.l(&format!("lda f:{}", var::SAVED_X));
    a.l("tax");
    a.l("jmp @next");

    a.label("note_bad");
    a.l("sep #$20");
    a.c("Record only the FIRST one; $00 means none has been recorded yet.");
    a.l(&format!("lda f:{}", var::FIRST_BAD));
    a.l("bne :+");
    a.l(&format!("lda f:{}", var::OP));
    a.l(&format!("sta f:{}", var::FIRST_BAD));
    a.l(":");
    a.l("rts");
}

/// Record the three counts and assert the row.
fn report(mut a: Asm) -> Test {
    a.l("rep #$30");
    a.l(&format!("lda f:{}", var::OK));
    a.l("and #$00FF");
    a.record(
        283,
        "A6.15 opcodes that returned at their documented length",
    );
    a.l(&format!("lda f:{}", var::OVER));
    a.l("and #$00FF");
    a.record(284, "A6.15 opcodes that returned LATE (expect 0)");
    a.l(&format!("lda f:{}", var::STUCK));
    a.l("and #$00FF");
    a.record(285, "A6.15 opcodes that did not return (expect 0)");
    a.l(&format!("lda f:{}", var::FIRST_BAD));
    a.l("and #$00FF");
    a.record(
        286,
        "A6.15 first opcode that was not clean ($00 = none; $00 is BRK, never run)",
    );

    a.c("Liveness first, and for the same reason E2.10 checks it first: a driver that fell over");
    a.c("early would report no late and no stuck opcodes, and two zeros would read as a pass.");
    a.l(&format!("lda f:{}", var::OK));
    a.l("and #$00FF");
    a.assert_a16_range(
        EXECUTED,
        EXECUTED,
        "the sweep did not execute 241 opcodes cleanly — it stopped early, or it covered a \
         different set than the 15 documented control-transfer exclusions. Slots 283-286 hold the \
         three counts and the first opcode that was not clean",
    );

    a.c("The row: nothing hung, and nothing consumed a different number of bytes than the WDC");
    a.c("table documents. STP is excluded by name and is the only opcode this cart cannot run.");
    a.l(&format!("lda f:{}", var::OVER));
    a.l("and #$00FF");
    a.l("clc");
    a.l(&format!("adc f:{}", var::STUCK));
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        0,
        "at least one opcode either failed to return or advanced PC by a different number of \
         bytes than Table 5-4 documents for it. Slot 284 counts the late ones, 285 the ones that \
         never came back, and 286 names the first ($00 there means none)",
    );

    a.finish(
        "A6.15",
        'A',
        "all 256 opcodes defined",
        Provenance::Documented("WDC W65C816S datasheet, Table 5-4 opcode matrix"),
        Kind::Scored,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        BUF, EXECUTED, EXIT_OK, EXIT_OVER, OVERSHOOT_AT, SANDBOX_DP, SCRATCH, SCRATCH_LEN,
        STUB_LEN, WINDOW, emit_exit_stubs, operand_bytes,
    };
    use crate::cpu_opcodes::{Flow, table};
    use crate::dsl::Asm;

    /// The exits' own address bytes are executed when an opcode overshoots by one or two, so both
    /// halves of both addresses have to be single-byte instructions that touch nothing the sandbox
    /// needs. `$AA` is `TAX` and `$B8` is `CLV`.
    #[test]
    fn the_exit_addresses_are_made_of_harmless_opcodes() {
        for addr in [EXIT_OK, EXIT_OVER] {
            let [lo, hi] = addr.to_le_bytes();
            assert_eq!(lo, hi, "${addr:04X} — keep both halves the same byte");
            assert!(
                matches!(lo, 0xAA | 0xB8),
                "${lo:02X} is not one of the vetted one-byte opcodes"
            );
        }
    }

    /// The longest opcode plus its terminator has to fit before the overshoot terminator, or a
    /// four-byte instruction would write its own clean exit on top of the overshoot one.
    #[test]
    fn the_longest_opcode_and_its_terminator_fit() {
        let longest = table()
            .iter()
            .map(|op| u16::from(op.len()))
            .max()
            .expect("256 entries");
        assert!(
            longest + 3 <= OVERSHOOT_AT,
            "the longest opcode is {longest} bytes and the overshoot terminator is at +{OVERSHOOT_AT}"
        );
    }

    /// [`super::STUB_LEN`] is what the copy loop counts, so it has to be what the table emits. A
    /// miscount copies a truncated stub into WRAM and the first opcode to reach it runs off the end.
    #[test]
    fn the_stub_length_matches_what_is_emitted() {
        let mut a = Asm::new();
        emit_exit_stubs(&mut a);
        // Two stubs, and every `.byte` line in the data segment belongs to one of them.
        let emitted: usize = a
            .data_lines()
            .iter()
            .filter(|l| l.trim_start().starts_with(".byte"))
            .map(|l| l.split(';').next().unwrap_or(l).matches('$').count())
            .sum();
        assert_eq!(
            emitted,
            2 * usize::from(STUB_LEN),
            "the table emits {emitted} bytes for two stubs of {STUB_LEN}"
        );
    }

    /// The "nothing failed" sentinel is an opcode index like any other, so it has to be one the row
    /// never executes. `$00` is `BRK`, which is in the not-executed set.
    ///
    /// `$FF` was the first choice and is **wrong**: `$FF` is `SBC long,X`, which the row does
    /// execute, so a genuine failure there would have been reported as `none`.
    #[test]
    fn the_no_failure_sentinel_can_never_be_a_real_answer() {
        let t = table();
        assert!(
            matches!(t[0x00].flow, Flow::Leaves(_)),
            "$00 ({}) is executed, so it cannot double as the `none` sentinel",
            t[0x00].name
        );
        assert!(
            !matches!(t[0xFF].flow, Flow::Leaves(_)),
            "$FF is no longer executed — the comment explaining why it is a bad sentinel is stale"
        );
    }

    /// The count the cart asserts has to be the one the table produces.
    #[test]
    fn the_asserted_count_matches_the_table() {
        let n = table()
            .iter()
            .filter(|op| !matches!(op.flow, Flow::Leaves(_)))
            .count();
        assert_eq!(n, usize::from(EXECUTED));
    }

    /// The driver's scratch must be clear of everything the sandbox can write: the sandbox itself,
    /// the operand window, and — the one that is easy to miss — the sandbox's own **direct page**,
    /// which is `$0200`-`$02FF` and reachable by every direct-page operand in the table.
    ///
    /// The first draft put the scratch at `$7E:0170`, which is inside neither of the first two and
    /// is used by a `bus.rs` row; the second would have put it inside the third. WRAM scratch has
    /// no allocator and no collision gate the way the measurement channel does, so this is it.
    #[test]
    fn the_scratch_is_clear_of_everything_the_sandbox_touches() {
        let scratch = SCRATCH..SCRATCH + SCRATCH_LEN;
        for (lo, hi, what) in [
            (BUF, BUF + OVERSHOOT_AT + 3, "the sandbox"),
            (WINDOW, WINDOW + 0x100, "the operand window"),
            (SANDBOX_DP, SANDBOX_DP + 0x100, "the sandbox's direct page"),
        ] {
            assert!(
                scratch.end <= lo || hi <= scratch.start,
                "the scratch at ${SCRATCH:04X} overlaps {what} (${lo:04X}-${hi:04X})"
            );
        }
    }

    /// No operand may name the sandbox, and no direct-page operand may reach the runtime's own
    /// variables in the first `$60` bytes of the mirror.
    #[test]
    fn no_operand_reaches_the_sandbox_or_the_runtime_variables() {
        use crate::cpu_opcodes::Mode;
        for op in table() {
            if matches!(op.flow, Flow::Leaves(_)) {
                continue;
            }
            let b = operand_bytes(op.code, op.mode);
            match op.mode {
                Mode::Direct | Mode::StackRelative => assert!(
                    b[1] >= 0x10,
                    "{} uses direct page ${:02X}, inside the runtime's variables",
                    op.name,
                    b[1]
                ),
                Mode::Absolute | Mode::Long => {
                    let addr = u16::from_le_bytes([b[1], b[2]]);
                    assert!(
                        !(BUF..BUF + OVERSHOOT_AT + 3).contains(&addr),
                        "{} names ${addr:04X}, which is inside the sandbox",
                        op.name
                    );
                }
                _ => {}
            }
        }
    }
}
