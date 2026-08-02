//! `E2.10` — the full 256-opcode SPC700 cycle sweep.
//!
//! # What the row claims
//!
//! For every opcode the SPC700 can execute in a straight line, the cart measures how long it takes
//! and compares that against the cycle count **fullsnes** documents for it. The comparison happens
//! on the SPC, against a table the generator built from the reference (`spc_opcodes.rs`); the cart
//! reports how many opcodes it measured and how many disagreed, and the host supplies no expected
//! values at all. A result therefore means the same thing on any emulator and on a flash cart,
//! which is the property the whole battery is built around.
//!
//! # How one opcode is timed
//!
//! Timer 2 at `T2DIV = 1` steps `T2OUT` once every 32 SMP base clocks — **16 opcode cycles**, a
//! figure that was measured rather than derived (the derivation from `Timer<16>` gives 8 and is
//! exactly half, because `T2OUT` counts stage-2 increments and those happen every *two* stage-1
//! toggles). Sixteen cycles is far too coarse for one instruction, so the sweep never times one:
//! it times a block of six copies, sixteen times over, and reads the difference against the same
//! block built from `NOP`.
//!
//! ```text
//! iteration:  reset X/Y/SP and the safe data window   <- identical in every arm
//!             CALL block                              <- identical in every arm
//!               prologue      4 bytes, 4 cycles       <- identical COST in every arm
//!               opcode x6                             <- the only thing that differs
//!               epilogue      5 bytes, 11 cycles      <- identical in every arm
//!             sum += T2OUT                            <- identical in every arm
//! ```
//!
//! Everything except the six copies is common, so it cancels in the difference, and one cycle of
//! difference becomes `6 * 16 / 16` = **6 ticks**. The sum across iterations is exact to ±1 (the
//! reads are read-and-clear, so they partition the interval), and the baseline carries its own ±1,
//! so the honest band is ±2. The row allows ±3, which is still half a cycle.
//!
//! # Why the block is built in RAM
//!
//! Six copies of each of 256 opcodes, plus a prologue and epilogue each, is some eight kilobytes of
//! straight-line code — more than the APU program budget and more than the ROM bank has. Instead
//! the driver assembles one 27-byte block in RAM per opcode from a table, and calls it. Four
//! **page-aligned** 256-byte tables (length-and-prologue, and the three instruction bytes) plus a
//! fifth of documented cycle counts make every lookup a single `MOV A,!table+X` with the opcode in
//! `X` — no pointer arithmetic anywhere in the driver, which is what keeps it short enough to
//! reason about.
//!
//! # The operands are chosen, and every one of the three rules has teeth
//!
//! An operand must not name an I/O register, must not name the block itself, and must stay in range
//! once an index register is added. The first is not a formality: most SPC700 stores perform a
//! dummy read of their destination, so a *write* to `$FF` would clear `T2OUT` — the sweep would be
//! erasing its own instrument. See [`crate::spc_opcodes::Operands`].
//!
//! # Branches are measured taken, on purpose
//!
//! A relative branch with a displacement of **zero** lands on the following instruction, which is
//! where a not-taken branch would have gone anyway. So a block of six copies runs straight through
//! whichever way each one goes, and the arm can arrange for the taken path — the more interesting
//! of the two, and the one whose cost differs between cores. The prologue sets whichever flag that
//! needs, in a fixed four-byte two-instruction shape so its cost is the same in every arm.
//!
//! # What it does not cover, and why that is not a gap
//!
//! Twenty-five opcodes end the straight line rather than continuing it: the absolute jumps and
//! calls (one encoding cannot make six copies at six addresses each fall into the next), the
//! vectored calls (`TCALL`, `PCALL`, `BRK` — their vectors live in the IPL ROM, which every Group E
//! program keeps mapped so it can hand the APU back), the returns (they pop an address the block
//! never pushed), and `SLEEP`/`STOP`, for which fullsnes itself gives the cycle count as `?`. For
//! those the question this row asks has no answer. They are named in `spc_opcodes.rs`, counted by a
//! unit test there, and the count is asserted on-cart — a sweep that measured 230 or 232 would fail
//! rather than quietly cover a different set.

use crate::dsl::{Asm, Kind, Provenance, Test};
use crate::spc::{DONE, PORT1, PORT2, PORT3, Spc};
use crate::spc_opcodes::{Flag, Measure, Op, Operands, table};

use super::apu::{apu_timeout_arm, upload_and_run_long};

/// Copies of the opcode per block. Six keeps the slowest opcode's iteration under `T2OUT`'s
/// four-bit ceiling: `DIV YA,X` at 12 cycles is 72 cycles of block against a ~87-cycle fixed cost,
/// which is ten ticks with six to spare.
const REPEATS: u8 = 6;

/// Iterations per opcode. `REPEATS * ITERATIONS / 16` = **6 ticks per cycle of difference**, and
/// the largest total — `DIV YA,X` again — stays inside a byte.
const ITERATIONS: u8 = 16;

/// Ticks per cycle of difference. Kept as a named constant because the on-cart arithmetic
/// multiplies by it and the doc comment above reasons about it; they must not drift apart.
const TICKS_PER_CYCLE: u8 = REPEATS * ITERATIONS / 16;

/// How far a measurement may sit from its documented expectation. Two comes from quantisation —
/// ±1 on the arm and ±1 on the baseline — and the third is margin. One cycle is
/// [`TICKS_PER_CYCLE`] away, so the band cannot swallow a real error.
const BAND: u8 = 3;

/// Opcodes the sweep measures: 256 less the ones that end the straight line.
const MEASURED: u8 = 231;

/// Where every Group E program is uploaded, and therefore what an absolute address in this one
/// means. `upload_and_run` sets both the destination and the entry point to it.
const BASE: u16 = 0x0200;

/// APU RAM the sweep uses beyond its uploaded image.
mod ram {
    /// Per-opcode results, one byte each.
    ///
    /// Above the uploaded image, and [`super::e2_10`] asserts that at build time. The first draft
    /// put this at `$0900` and the image reached `$0948`, so the sweep overwrote its own driver as
    /// it recorded results — the battery reported the row as SKIP, because the code it was running
    /// stopped being code.
    pub const RESULTS: u16 = 0x0C00;
    /// The block the driver assembles and calls.
    pub const BLOCK: u16 = 0x0D00;
    /// The window every memory operand points into. Far from the image, the block, the stack and
    /// the zero page, with room above it for an index register to be added.
    pub const WINDOW: u16 = 0x0E00;
}

/// Direct-page variables. `$30` upward is the operand window's own page; `$00-$0F` is the driver.
mod dp {
    /// The opcode being measured.
    pub const OP: u8 = 0x00;
    /// Its length in bytes.
    pub const LEN: u8 = 0x01;
    /// Its prologue index.
    pub const PRO: u8 = 0x02;
    /// Iteration counter.
    pub const ITER: u8 = 0x03;
    /// Accumulated ticks for this opcode.
    pub const SUM: u8 = 0x04;
    /// Where the next byte of the block goes, as an offset from [`super::ram::BLOCK`].
    pub const CURSOR: u8 = 0x05;
    /// Scratch.
    pub const TMP: u8 = 0x06;
    /// How many opcodes were actually measured.
    pub const COUNT: u8 = 0x07;
    /// How many disagreed with their documented cycle count.
    pub const BAD: u8 = 0x08;
    /// The first that did, or `$FF`.
    pub const FIRST_BAD: u8 = 0x09;
    /// The `NOP` baseline.
    pub const BASE: u8 = 0x0A;
    /// The three instruction bytes, staged between the table read and the block write.
    pub const BYTES: u8 = 0x0C;

    /// The low byte of the operand window pointer, and the plain direct-page operand.
    pub const WINDOW_PTR: u8 = 0x30;
    /// A direct-page byte holding `$FF`, so `BBS aa.b` has a set bit to branch on.
    pub const ONES: u8 = 0x32;
}

/// The prologues, indexed by [`Flag`]. Each is exactly two instructions, four bytes and four
/// cycles, so the choice costs nothing that could leak into a difference.
///
/// `CLRV` would be the obvious way to clear `V`, but it is one byte and two cycles where `ADC` is
/// two and two — a uniform *shape* matters more here than a shorter one, because the shape is what
/// guarantees the cost is identical across arms.
const PROLOGUES: [[u8; 4]; 7] = [
    [0xE8, 0x01, 0xE8, 0x01], // MOV A,#$01 x2 — N=0, Z=0, A=1 (the default, and CBNE's A != [aa])
    [0xE8, 0x80, 0xE8, 0x80], // MOV A,#$80 x2 — N=1
    [0xE8, 0x00, 0xE8, 0x00], // MOV A,#$00 x2 — Z=1
    [0xE8, 0xFF, 0x68, 0x00], // MOV A,#$FF; CMP A,#$00 — C=1
    [0xE8, 0x00, 0x68, 0xFF], // MOV A,#$00; CMP A,#$FF — C=0
    [0xE8, 0x7F, 0x88, 0x7F], // MOV A,#$7F; ADC A,#$7F — V=1 whatever the carry in
    [0xE8, 0x00, 0x88, 0x00], // MOV A,#$00; ADC A,#$00 — V=0 whatever the carry in
];

/// Pick the prologue that makes this opcode's branch taken.
const fn prologue_for(measure: Measure) -> u8 {
    match measure {
        Measure::BranchTaken(flag) => match flag {
            Flag::NegSet => 1,
            Flag::ZeroSet => 2,
            Flag::CarrySet => 3,
            Flag::CarryClear => 4,
            Flag::OvfSet => 5,
            Flag::OvfClear => 6,
            // `N = 0`, `Z = 0` and the `CBNE`/`DBNZ` conditions are all what the default arranges.
            Flag::NegClear | Flag::ZeroClear | Flag::None => 0,
        },
        _ => 0,
    }
}

/// The epilogue every block ends with.
///
/// It is not just a `RET`. The block may have pushed (`PUSH A` six times), popped, moved `SP`
/// outright (`MOV SP,X`), or set the direct-page flag (`SETP`, or a `POP PSW` that loaded garbage)
/// — and a `RET` under any of those returns somewhere else. `CLRP` puts the direct page back and
/// `MOV SP,#$ED` puts the stack pointer where `CALL` left it, both at a fixed cost.
///
/// `$ED` is the driver's `$EF` less the two bytes `CALL` pushed.
const EPILOGUE: [u8; 5] = [
    0x20, // CLRP
    0xCD, 0xED, // MOV X,#$ED
    0xBD, // MOV SP,X
    0x6F, // RET
];

/// Fill in an opcode's operand bytes with something safe to execute six times in a row.
///
/// See the module docs on what "safe" has to mean. The choices below are the whole of it:
///
/// | operand | value | why |
/// |---|---|---|
/// | direct page | `$30` | the window pointer's own page, clear of `$00-$0F` and of `$F0-$FF` |
/// | direct page, indexed | `$00` | `X` and `Y` are both `$30`, so `$00 + X` lands on the same byte |
/// | absolute | `$0C00` | far from the image, the block and the stack; `+ $30` stays inside it |
/// | bit address | `$0C00` bit 0 | the same window; the bit index rides in the top three bits |
/// | relative | `0` | the taken path lands on the next instruction — see the module docs |
///
/// `BBS aa.b` is the one exception, and it earns it: with the window byte at zero every `BBS` would
/// find its bit clear and measure the *not-taken* cost while the table says taken. It points at
/// `$32` instead, which the driver keeps at `$FF`.
const fn operand_bytes(op: Op) -> [u8; 3] {
    let [wlo, whi] = ram::WINDOW.to_le_bytes();
    let is_bbs = (op.code & 0x1F) == 0x03;
    // `X` and `Y` both hold `WINDOW_PTR` while a block runs, so an indexed direct-page operand of
    // zero is what lands on the window. Naming the window directly would land 48 bytes past it.
    let indexed = dp::WINDOW_PTR.wrapping_sub(dp::WINDOW_PTR); // = 0, written so the reason shows
    match op.operands {
        Operands::None | Operands::Imm | Operands::Rel => [op.code, 0x00, 0x00],
        Operands::Dp => [op.code, dp::WINDOW_PTR, 0x00],
        // `CBNE aa+X,rr`'s displacement is zero like every other branch's, so it shares this arm.
        Operands::DpIndexed | Operands::DpIndexedRel => [op.code, indexed, 0x00],
        // `cmd aa,bb` encodes the SOURCE first. Both sit in the window's page.
        Operands::DpDp => [op.code, dp::WINDOW_PTR + 1, dp::WINDOW_PTR],
        // `cmd aa,#nn` encodes the immediate first.
        Operands::ImmDp => [op.code, 0x00, dp::WINDOW_PTR],
        // `AbsBit` shares this arm at **bit zero**: the bit index rides in the top three bits of
        // the 16-bit operand, and any bit of a window byte answers the same question — so the sweep
        // picks one rather than deriving one from the opcode, which for this column would be
        // deriving it from nothing.
        Operands::Abs | Operands::AbsBit => [op.code, wlo, whi],
        Operands::DpRel => [
            op.code,
            if is_bbs { dp::ONES } else { dp::WINDOW_PTR },
            0x00,
        ],
    }
}

/// The five page-aligned tables, laid out so a lookup is one `MOV A,!table+X`.
struct Tables {
    /// Length in the low nibble, prologue index in the high one. Zero length means "not measured".
    lenp: u16,
    /// The opcode byte.
    b0: u16,
    /// Operand byte 1.
    b1: u16,
    /// Operand byte 2.
    b2: u16,
    /// The documented cycle count.
    cyc: u16,
    /// The seven prologues, 28 bytes, indexed by `prologue * 4 + i`.
    prologues: u16,
}

/// Build the data blob and return where each table landed.
///
/// The blob is padded so the first table starts on a page boundary. Two hundred and fifty-odd bytes
/// of padding buys an indexed lookup with no pointer arithmetic in it — a trade worth making in a
/// bank with kilobytes free, and not one worth making if it were tight.
fn build_tables(prog: &mut Spc, base: u16) -> Tables {
    let ops = table();
    let mut blob = Vec::new();

    // `data_first` emits a three-byte `JMP` over the data, so the data starts at `base + 3`.
    let data_start = base + 3;
    let first_page = (data_start + 0xFF) & 0xFF00;
    blob.resize(usize::from(first_page - data_start), 0x00);

    let mut lenp = Vec::with_capacity(256);
    let mut b0 = Vec::with_capacity(256);
    let mut b1 = Vec::with_capacity(256);
    let mut b2 = Vec::with_capacity(256);
    let mut cyc = Vec::with_capacity(256);
    for op in &ops {
        let skipped = matches!(op.measure, Measure::NotStraightLine(_));
        let bytes = operand_bytes(*op);
        lenp.push(if skipped {
            0x00
        } else {
            op.len | (prologue_for(op.measure) << 4)
        });
        b0.push(bytes[0]);
        b1.push(bytes[1]);
        b2.push(bytes[2]);
        cyc.push(op.cycles);
    }

    let addr_lenp = first_page;
    let addr_b0 = first_page + 0x100;
    let addr_b1 = first_page + 0x200;
    let addr_b2 = first_page + 0x300;
    let addr_cyc = first_page + 0x400;
    let addr_pro = first_page + 0x500;
    blob.extend_from_slice(&lenp);
    blob.extend_from_slice(&b0);
    blob.extend_from_slice(&b1);
    blob.extend_from_slice(&b2);
    blob.extend_from_slice(&cyc);
    for p in PROLOGUES {
        blob.extend_from_slice(&p);
    }

    let emitted = prog.data_first(base, &blob);
    assert_eq!(
        emitted, data_start,
        "the table blob did not land where the layout assumed"
    );

    Tables {
        lenp: addr_lenp,
        b0: addr_b0,
        b1: addr_b1,
        b2: addr_b2,
        cyc: addr_cyc,
        prologues: addr_pro,
    }
}

/// Emit the sweep driver.
#[allow(clippy::too_many_lines)]
fn driver(prog: &mut Spc, t: &Tables, base: u16) {
    let [wlo, whi] = ram::WINDOW.to_le_bytes();

    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_dp_imm(0xFC, 0x01) // T2DIV = 1
        .mov_dp_imm(0xF1, 0x84) // enable timer 2; bit 7 keeps the IPL mapped
        .mov_dp_imm(dp::COUNT, 0x00)
        .mov_dp_imm(dp::BAD, 0x00)
        .mov_dp_imm(dp::FIRST_BAD, 0xFF)
        .mov_dp_imm(dp::OP, 0x00);

    // ---- pass 1: measure every straight-line opcode ----
    let op_loop = prog.here();
    prog.mov_x_dp(dp::OP)
        .mov_a_abs_x(t.lenp)
        .mov_dp_a(dp::TMP)
        .and_a_imm(0x0F)
        .mov_dp_a(dp::LEN);
    // A short branch cannot clear the whole per-opcode body, so the test is inverted: fall into
    // the body when the length is non-zero, and take an absolute jump to the tail when it is not.
    let measure_it = prog.bne_fwd();
    let skip = prog.jmp_fwd();
    prog.patch_fwd(measure_it);
    prog.mov_a_dp(dp::TMP)
        .xcn()
        .and_a_imm(0x0F)
        .mov_dp_a(dp::PRO);

    // ---- assemble the block: prologue, six copies, epilogue ----
    prog.mov_dp_imm(dp::CURSOR, 0x00);
    // X walks the prologue table from `prologue * 4`; Y counts the four bytes.
    prog.mov_a_dp(dp::PRO)
        .asl_a()
        .asl_a()
        .mov_x_a()
        .mov_y_imm(0x00);
    let pro_loop = prog.here();
    prog.mov_a_abs_x(t.prologues)
        .mov_dp_a(dp::BYTES)
        .inc_x_reg();
    prog.mov_a_x().mov_dp_a(dp::TMP); // stash the prologue-table index
    prog.mov_x_dp(dp::CURSOR)
        .mov_a_dp(dp::BYTES)
        .mov_abs_x_a(ram::BLOCK)
        .inc_dp(dp::CURSOR);
    prog.mov_x_dp(dp::TMP).inc_y().cmp_y_imm(0x04);
    prog.bne_back(pro_loop);

    // Six copies. Three bytes are written every time and the cursor advances by the real length,
    // so a one- or two-byte opcode's tail is simply overwritten by whatever comes next — and the
    // epilogue, written last, overwrites the final tail.
    prog.mov_y_imm(0x00);
    let copy_loop = prog.here();
    prog.mov_x_dp(dp::OP)
        .mov_a_abs_x(t.b0)
        .mov_dp_a(dp::BYTES)
        .mov_a_abs_x(t.b1)
        .mov_dp_a(dp::BYTES + 1)
        .mov_a_abs_x(t.b2)
        .mov_dp_a(dp::BYTES + 2);
    prog.mov_x_dp(dp::CURSOR)
        .mov_a_dp(dp::BYTES)
        .mov_abs_x_a(ram::BLOCK)
        .inc_x_reg()
        .mov_a_dp(dp::BYTES + 1)
        .mov_abs_x_a(ram::BLOCK)
        .inc_x_reg()
        .mov_a_dp(dp::BYTES + 2)
        .mov_abs_x_a(ram::BLOCK);
    prog.mov_a_dp(dp::CURSOR)
        .clrc()
        .adc_a_dp(dp::LEN)
        .mov_dp_a(dp::CURSOR);
    prog.inc_y().cmp_y_imm(REPEATS);
    prog.bne_back(copy_loop);

    for (i, byte) in EPILOGUE.iter().enumerate() {
        prog.mov_x_dp(dp::CURSOR).mov_a_imm(*byte);
        for _ in 0..i {
            prog.inc_x_reg();
        }
        prog.mov_abs_x_a(ram::BLOCK);
    }

    // ---- measure it ----
    prog.mov_dp_imm(dp::ITER, 0x00)
        .mov_dp_imm(dp::SUM, 0x00)
        .mov_a_dp(0xFF); // drain T2OUT so the interval starts from zero
    let iter_loop = prog.here();
    prog.mov_x_imm(0xEF)
        .mov_sp_x()
        .mov_x_imm(dp::WINDOW_PTR)
        .mov_y_imm(dp::WINDOW_PTR)
        .mov_dp_imm(dp::WINDOW_PTR, wlo)
        .mov_dp_imm(dp::WINDOW_PTR + 1, whi)
        .mov_dp_imm(dp::ONES, 0xFF);
    prog.call_abs(ram::BLOCK);
    prog.mov_a_dp(0xFF)
        .mov_dp_a(dp::TMP)
        .mov_a_dp(dp::SUM)
        .clrc()
        .adc_a_dp(dp::TMP)
        .mov_dp_a(dp::SUM)
        .inc_dp(dp::ITER)
        .mov_a_dp(dp::ITER)
        .cmp_a_imm(ITERATIONS);
    prog.bne_back(iter_loop);

    prog.mov_x_dp(dp::OP)
        .mov_a_dp(dp::SUM)
        .mov_abs_x_a(ram::RESULTS)
        .inc_dp(dp::COUNT);

    prog.patch_jmp_fwd(skip, base);
    prog.inc_dp(dp::OP).mov_a_dp(dp::OP).cmp_a_imm(0x00);
    // The body is far beyond a branch's reach in both directions, so the loop is a short
    // conditional exit over a long absolute jump rather than a conditional jump back.
    let swept = prog.beq_fwd();
    prog.jmp_back(op_loop, base);
    prog.patch_fwd(swept);

    // ---- pass 2: compare every result against its documented cycle count ----
    //
    // `NOP` is opcode $00 and costs two cycles, so its result IS the baseline the differences are
    // taken from. Nothing here needs the baseline to be any particular value — only that the same
    // fixed overhead sits in it as in every other arm, which is what the identical loop guarantees.
    prog.mov_x_imm(0x00)
        .mov_a_abs_x(ram::RESULTS)
        .mov_dp_a(dp::BASE);
    prog.mov_dp_imm(dp::OP, 0x00);
    let chk_loop = prog.here();
    prog.mov_x_dp(dp::OP).mov_a_abs_x(t.lenp).and_a_imm(0x0F);
    let chk_skip = prog.beq_fwd();

    // expected = base + (cycles - 2) * TICKS_PER_CYCLE
    prog.mov_a_abs_x(t.cyc).setc().sbc_a_imm(0x02);
    prog.mov_y_a()
        .mov_a_imm(TICKS_PER_CYCLE)
        .mul_ya()
        .clrc()
        .adc_a_dp(dp::BASE)
        .mov_dp_a(dp::TMP);
    // delta = measured - expected, then biased by BAND so the whole allowed range is 0 ..= 2*BAND
    prog.mov_x_dp(dp::OP)
        .mov_a_abs_x(ram::RESULTS)
        .setc()
        .sbc_a_dp(dp::TMP)
        .clrc()
        .adc_a_imm(BAND)
        .cmp_a_imm(BAND * 2 + 1);
    let chk_ok = prog.bcc_fwd();
    prog.inc_dp(dp::BAD).mov_a_dp(dp::FIRST_BAD).cmp_a_imm(0xFF);
    let already = prog.bne_fwd();
    prog.mov_a_dp(dp::OP).mov_dp_a(dp::FIRST_BAD);
    prog.patch_fwd(already);
    prog.patch_fwd(chk_ok);
    prog.patch_fwd(chk_skip);
    prog.inc_dp(dp::OP).mov_a_dp(dp::OP).cmp_a_imm(0x00);
    prog.bne_back(chk_loop);

    prog.mov_dp_imm(0xF1, 0x80) // stop timer 2; the IPL stays mapped for the handover
        .mov_a_dp(dp::BAD)
        .mov_dp_a(PORT1)
        .mov_a_dp(dp::COUNT)
        .mov_dp_a(PORT2)
        .mov_a_dp(dp::FIRST_BAD)
        .mov_dp_a(PORT3)
        .mov_a_imm(DONE)
        .mov_dp_a(0xF4)
        .release_to_ipl();
}

/// The `E2.10` row.
pub fn e2_10() -> Test {
    let mut prog = Spc::new();
    let tables = build_tables(&mut prog, BASE);
    driver(&mut prog, &tables, BASE);
    let top = BASE + u16::try_from(prog.bytes().len()).expect("a program is smaller than RAM");
    assert!(
        top <= ram::RESULTS,
        "the sweep image reaches ${top:04X}, which is inside the result page at \
         ${:04X} — it would overwrite its own driver as it records",
        ram::RESULTS
    );

    let mut a = Asm::new();
    upload_and_run_long(&mut a, &prog);
    a.l("rep #$30");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.record(280, "E2.10 opcodes disagreeing with fullsnes (expect 0)");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.record(281, "E2.10 opcodes measured (expect 231 = 256 - 25)");
    a.l("lda f:$7E0102");
    a.l("and #$00FF");
    a.record(282, "E2.10 first disagreeing opcode ($FF = none)");

    a.c("Liveness first. A driver that fell over after one opcode would report no disagreements,");
    a.c("and a `0` in slot 280 would then read as a pass — so the count of opcodes actually");
    a.c("measured is asserted before anything is concluded from the count of failures.");
    a.l("lda f:$7E0101");
    a.l("and #$00FF");
    a.assert_a16_range(
        u16::from(MEASURED),
        u16::from(MEASURED),
        "the sweep did not measure 231 opcodes — it either stopped early or covered a different \
         set than the 25 documented non-straight-line exclusions. Slot 281 holds the count it \
         reached",
    );

    a.c("The row itself: every measured opcode's timing agrees with the cycle count fullsnes");
    a.c("documents for it, to within half a cycle.");
    a.l("lda f:$7E0100");
    a.l("and #$00FF");
    a.assert_a16_range(
        0,
        0,
        "at least one opcode's measured timing disagrees with the cycle count fullsnes documents \
         for it by more than half a cycle. Slot 280 holds how many, slot 282 the first — and one \
         cycle is six ticks here, so a disagreement is a whole cycle or more, not rounding",
    );
    apu_timeout_arm(&mut a);
    a.finish(
        "E2.10",
        'E',
        "256-opcode cycle sweep",
        Provenance::Documented("fullsnes, SNES APU SPC700 CPU instruction set"),
        Kind::Scored,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::{MEASURED, PROLOGUES, REPEATS, TICKS_PER_CYCLE, build_tables, operand_bytes};
    use crate::spc::Spc;
    use crate::spc_opcodes::{Measure, table};

    /// No operand may name an I/O register. This is the rule with teeth: most SPC700 stores dummy-
    /// read their destination, so an operand of `$FF` would clear `T2OUT` and the sweep would erase
    /// its own instrument mid-measurement.
    #[test]
    fn no_operand_reaches_the_io_page() {
        for op in table() {
            if matches!(op.measure, Measure::NotStraightLine(_)) {
                continue;
            }
            let b = operand_bytes(op);
            for (i, byte) in b.iter().enumerate().take(usize::from(op.len)).skip(1) {
                assert!(
                    *byte < 0xF0 || op.operands == crate::spc_opcodes::Operands::Imm,
                    "{} operand byte {i} is ${byte:02X}, which reaches the I/O page",
                    op.name
                );
            }
        }
    }

    /// Every prologue is the same shape, because its cost has to cancel in the difference.
    #[test]
    fn every_prologue_is_four_bytes_of_two_two_cycle_instructions() {
        for p in PROLOGUES {
            assert_eq!(p.len(), 4);
            // Both instructions are two-byte immediates: `MOV A,#nn` ($E8), `CMP A,#nn` ($68) or
            // `ADC A,#nn` ($88). All three are two cycles.
            assert!(matches!(p[0], 0xE8), "first opcode ${:02X}", p[0]);
            assert!(
                matches!(p[2], 0xE8 | 0x68 | 0x88),
                "second opcode ${:02X}",
                p[2]
            );
        }
    }

    /// The block has to fit in one page, because the driver addresses it with an 8-bit index.
    #[test]
    fn the_assembled_block_fits_the_indexed_write() {
        let longest = 4 + usize::from(REPEATS) * 3 + 5;
        assert!(longest <= 0xFF, "the longest block is {longest} bytes");
    }

    /// The tables land page-aligned, which is the whole reason a lookup is one instruction.
    #[test]
    fn the_tables_are_page_aligned() {
        let mut prog = Spc::new();
        let t = build_tables(&mut prog, 0x0200);
        for addr in [t.lenp, t.b0, t.b1, t.b2, t.cyc, t.prologues] {
            assert_eq!(addr & 0xFF, 0, "table at ${addr:04X} is not page aligned");
        }
    }

    /// The measured count the cart asserts has to be the one the table actually produces.
    #[test]
    fn the_asserted_measured_count_matches_the_table() {
        let n = table()
            .iter()
            .filter(|op| !matches!(op.measure, Measure::NotStraightLine(_)))
            .count();
        assert_eq!(n, usize::from(MEASURED));
    }

    /// The resolution the doc comment claims.
    #[test]
    fn one_cycle_of_difference_is_six_ticks() {
        assert_eq!(TICKS_PER_CYCLE, 6);
    }
}
