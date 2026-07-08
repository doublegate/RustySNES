# ST018 — ARMv3 core implementation notes

`ST018` (Star Ocean's coprocessor) is the last unimplemented `v0.4.0 "Completion"` line item
(`to-dos/VERSION-PLAN.md`; the other two, SPC7110's addressing fix and standalone S-RTC, landed in
PR #23). It is a full ARMv3 (ARM6-class, pre-Thumb) CPU core — comparable in scope to
`rustysnes-cpu`'s 65C816 (`exec.rs`), not a small register-file port like this project's other
BestEffort coprocessors. It is landing incrementally (`crate::coproc::armv3`); this document is the
architecture reference for that work, kept up to date alongside the code per the project's
docs-as-spec rule.

**Port source: Mesen2's `Core/SNES/Coprocessors/ST018/`** (`ArmV3Cpu.cpp`/`.h`, `ArmV3Types.h`,
`St018.cpp`/`.h`/`Types.h`), NOT ares' `sfc/coprocessor/armdsp/` — ares reuses its generic shared
`component/processor/arm7tdmi` (a full ARM+Thumb ARM7TDMI superset the real ARMv3-class ST018 chip,
predating Thumb, never needed). Mesen2's dedicated `ArmV3Cpu` is the more faithful, more focused
scope to port.

## Board/bus facts

From `ref-proj/bsnes/bsnes/target-bsnes/resource/system/boards.bml`, `board: EXLOROM-RAM-ARMDSP`
(Star Ocean's board):

```
memory type=ROM content=Program
  map address=00-7d,80-ff:8000-ffff mask=0x8000
  map address=40-6f,c0-ef:0000-7fff mask=0x8000
memory type=RAM content=Save
  map address=70-7d,f0-ff:0000-ffff
processor architecture=ARM6
  map address=00-3f,80-bf:3800-38ff
  memory type=ROM content=Program architecture=ARM6
  memory type=ROM content=Data architecture=ARM6
  memory type=RAM content=Data architecture=ARM6
  oscillator
```

Base board is ExLoROM (this project's `board::ExLoRom` already exists — no new base-board work
needed). The ARM side needs external chip-ROM dumps: `PrgRomSize = 0x20000` (128 KiB program),
`DataRomSize = 0x8000` (32 KiB data), `WorkRamSize = 0x4000` (16 KiB work RAM) — an LLE chip like
the NEC DSP family (user-supplied firmware, `docs/adr/0003`), not a cart-ROM-resident program like
GSU/SA-1/CX4.

## Architecture

- **Register file**: `R[16]` (R15=PC) + banked register sets per mode (`UserRegs[7]`, `FiqRegs[7]`,
  `IrqRegs[2]`, `SupervisorRegs[2]`, `AbortRegs[2]`, `UndefinedRegs[2]`) + a separate SPSR per
  privileged mode (`FiqSpsr`/`IrqSpsr`/`SupervisorSpsr`/`AbortSpsr`/`UndefinedSpsr`) + the live
  CPSR. `SwitchMode` does the bank swap via a bulk copy in the source; port as explicit slice
  copies. 7 modes: User/Fiq/Irq/Supervisor/Abort/Undefined/System (values `0b10000`-`0b11111`,
  high bit always forced set).
- **Condition codes**: straightforward N/Z/C/V logic table (ported in `armv3::check_condition`,
  fully unit-tested against the ARM ARM's own truth table).
- **Barrel shifter** (ported in `armv3::shift_lsl`/`shift_lsr`/`shift_asr`/`shift_ror`/
  `shift_rrx`/`rotate_right`/`rotate_right_carry`): each returns `(value, carry_out)`; the
  `shift<32`/`shift<33` boundary checks handle the ARM-specific "shift amount from a register can
  be 0-255" edge cases (a shift of exactly 32 or more has defined-but-nonobvious behavior per
  operation — LSL by ≥32 gives 0 with carry = bit 0 of value when shift==32 else 0; LSR/ASR mirror
  this from the other end). This is the highest bug-density area in a from-scratch ARM core —
  every boundary case is exhaustively unit-tested.
- **ALU core** (ported in `armv3::add`/`sub`/`logical_flags`): `add` computes the signed-overflow
  flag via `~(op1^op2) & (op1^result) & sign_bit` and carry via `(op1^op2^overflow^result) &
  sign_bit` — NOT the naive wrapping-add carry-out; ported exactly as the source specifies, not
  reimplemented from first principles. `sub(op1, op2, carry, flags)` is literally
  `add(op1, !op2, carry, flags)` — ARM's subtract is add-with-inverted-operand-and-carry, ported
  as a direct call, not a separate formula (the two must never drift).
- **Data processing**: op1/op2 assembly + the 16 ALU ops (AND/EOR/SUB/RSB/ADD/ADC/SBC/RSC/TST/TEQ/
  CMP/CMN/ORR/MOV/BIC/MVN) dispatched by `(opCode>>21)&0xF`.
  - **The PC+8 gotcha**: the register-read helper returns `R[reg]` with NO special-casing for
    R15 — the "PC reads as instruction_address+8" ARM pipeline quirk is achieved IMPLICITLY by
    the 3-stage pipeline's own timing (see below), NOT by adding +8 in the register-read helper.
    The one EXPLICIT exception: the register-specified-shift-amount case manually adds +4 to
    `op1`/`op2` when `rn`/`rm==15`, to compensate for an extra pipeline cycle spent reading the
    shift-amount register — a documented, real ARM CPU exception to the otherwise-implicit +8,
    easy to silently drop in a naive port.
  - **Implicit exception-return side effect**: right after the ALU dispatch, if the destination
    is R15 AND the S-bit is set, CPSR is restored wholesale from the current mode's SPSR (and the
    mode switches to match) — any data-processing instruction with the S-bit set AND R15 as the
    destination acts as an implicit "return from exception" (e.g. `MOVS PC, LR` is the idiomatic
    ARM exception-handler return, relying on exactly this). Must be ported alongside the main ALU
    dispatch, not treated as optional.
- **The pipeline**: 3-stage Fetch→Decode→Execute, `R[15]` tracks the FETCH address (2 stages
  ahead of the instruction currently executing), advanced by `+4` per pipeline-process call, which
  happens AFTER the current Execute-stage instruction runs. This is WHY plain `R(15)` naturally
  reads as `execute_address+8` — the pipeline offset does the work, there's no explicit "+8"
  constant anywhere in the ALU/data-processing code. **Get this pipeline model exactly right
  before porting any instruction that reads R15 as an operand** — every other instruction's
  correctness depends on this being right, exactly the class of bug that was easy to introduce and
  hard to find after the fact when tracking down the SPC7110 boot-crash root cause (`docs/cart.md`
  §SPC7110). A dedicated test asserting R15's observed value at each pipeline stage should land
  before any instruction that consumes it.
- **Branch**: sign-extended 24-bit offset<<2, `R[14]=R[15]-4` for branch-with-link (NOT `R[15]`,
  since R15 is already +8 from the branch instruction's own address at execute time — `R[15]-4` =
  branch_addr+4 = the correct "next sequential instruction" return address).
- **MSR/MRS**: PSR transfer, mode-gated (User mode can't write control bits unless writing SPSR
  in a privileged mode) — needed for ST018 program setup even if games never hit an interrupt path.
- **`ArmMultiply`/`ArmMultiplyLong`** (MUL/MLA/MULL/MLAL/UMULL/UMLAL/SMULL/SMLAL): the reference
  implementation delegates the actual multiply + variable cycle count to a shared helper that is a
  cycle-EXACT simulation of the real ARM Booth's-algorithm multiplier circuit (CSA arrays, booth
  recoding, an author-acknowledged empirically-reverse-engineered correction table) — built for
  GBA hardware test-ROM precision, not because ST018/Star Ocean's actual game logic depends on
  exact multiply timing. **This project's port should NOT reproduce that level of fidelity**:
  implement the multiply variants with a plain correct 64-bit multiply (trivial in Rust) and a
  SIMPLE early-termination cycle-count approximation matching the ARM ARM's documented (not
  reverse-engineered) multiply timing rule — 1 extra cycle if bits 8-31 of the `Rs` operand are
  all 0/1, 2 if bits 16-31, 3 if bits 24-31, 4 otherwise. Getting the RESULT bit-exact matters
  (games depend on it); matching GBA-test-ROM-precision idle-cycle counts does not — nothing in
  this project's determinism contract or accuracy oracle exercises ST018 cycle timing, unlike the
  65C816/PPU/APU, which do have a cycle-exact oracle this project gates on. `rd`/`rh`/`rl == 15`
  writes are simply dropped (the real chip's documented UNPREDICTABLE case).
- **`ArmSingleDataTransfer` (LDR/STR)**: op1 = `R(rn)`; offset is either a 12-bit immediate or a
  shifted register (immediate shift amounts only — unlike data processing, there is no
  register-specified shift-amount variant here). `pre`/`up` control when/whether the offset is
  applied. STR of R15 stores `PC+4` (the same pipeline-relative +4 seen elsewhere for R15-as-
  source). Write-back happens if `writeBack || !pre` (post-indexed addressing ALWAYS writes back,
  even without the explicit W bit — a real, easy-to-miss ARM ARM rule) AND `(rd != rn || !load)`
  (a load into the same register as the base is NOT written back — the loaded value wins).
- **`ArmBlockDataTransfer` (LDM/STM)**: the most complex ARM instruction, with several documented
  hardware quirks that must all be ported verbatim:
  - **Empty register list glitch**: if the 16-bit register mask is 0, real hardware still
    transfers exactly R15 but computes the address delta as if all 16 registers were transferred
    — ported as-is, not "fixed" to be sensible.
  - **Address computation**: for decrementing addressing, the final address is computed by
    walking backward from the last transferred address, not incrementally.
  - **Write-back timing**: happens at the FIRST register actually transferred inside the loop
    (not before the loop starts), except when it's a load with write-back, which writes back
    BEFORE the loop — a genuine load/store asymmetry present in the source, both cases must be
    ported.
  - **S-bit (force-user-bank)**: forces User-mode register banking during the transfer UNLESS
    it's a load that includes R15 in the list — in that case the transfer uses the CURRENT mode's
    banks, and AFTER the transfer completes, CPSR is restored wholesale from the current mode's
    SPSR (an implicit "return from exception" side effect of `LDM ... {..., pc}^`). Mode is
    switched back to the original BEFORE this SPSR-restore check.
  - **LDM's read explicitly skips the usual unaligned-read rotation** that a plain LDR applies —
    a documented exception a naive LDM-as-repeated-LDR port would silently get wrong.
- **`ArmSingleDataSwap` (SWP)**: read-then-write the SAME address atomically (`Read` then an idle
  cycle then `Write`, in that exact order — a real read-modify-write bus cycle real hardware
  serializes), loading the OLD value into the destination register.
- **`ArmSoftwareInterrupt`**: exactly a Supervisor-mode exception entry at the SoftwareIrq vector —
  no per-instruction special-casing beyond that (the SWI comment/immediate field is never read by
  the CPU itself; software convention only). `ArmInvalidOp` (the undefined-instruction trap,
  reached via the dispatch table's default fill) is the same shape at the Undefined vector.
- **Dispatch table**: built once from a 12-bit index `((opCode & 0x0FF00000) >> 16) | ((opCode &
  0xF0) >> 4)` into a lookup table; the bit-range population loops in the reference source are the
  authoritative bit-pattern-to-instruction-class mapping — port these ranges directly rather than
  re-deriving the ARM encoding table from a manual/datasheet.

## Board-side bus protocol (host-sync model)

**ST018 is architecturally an SA-1, NOT a GSU/DSP-1.** The reference `Run()` catches the ARM CPU's
own cycle counter up to the current master-clock value in a loop that steps the ARM CPU once per
catch-up cycle — the ARM runs continuously in lockstep with the master clock, not "triggered, then
runs to completion on a Go/RQM flag" like GSU/DSP-1. `Run()` is called before every register
read/write and at end-of-frame ("catch up before any observation"). **Port this exactly like this
project's existing SA-1 second-CPU deterministic master-clock catch-up** (`rustysnes-core`'s
`run_sa1`, `docs/scheduler.md` §SA-1) — the architecture is already proven in this codebase, just
needs an ARM CPU instance instead of a second 65C816. Do NOT reach for a run-to-completion pattern;
it's the wrong model for this chip.

Bus window is `$00-3F,$80-BF:$3000-$3FFF` (registered whole, dispatched internally), not just
`$3800-38FF` as `boards.bml` implies at a glance:

- SNES-side `$3800` (read): pulls one byte the ARM placed for the SNES, clearing the
  data-available flag on read.
- SNES-side `$3802` (write): pushes one byte to the ARM, setting its data-available flag; (read):
  clears an acknowledge flag.
- SNES-side `$3804`: status register on read (bit 0 = data-for-SNES available, bit 2 = ack, bit 3
  = data-for-ARM available, bit 7 = `!reset`); on write, toggles reset — a `1->0` transition
  triggers a real ARM reset (which preserves the cycle counter across the reset, not zeroing it).
- ARM-side memory map (addressed by the ARM's own 32-bit space, top nibble selects region):
  `0x0xxxxxxx` = PRG ROM (128 KiB), `0x4xxxxxxx` = the same handshake registers mirrored into ARM
  address space, `0xAxxxxxxx` = data ROM (32 KiB), `0xExxxxxxx` = work RAM (16 KiB). A handful of
  ARM-side write addresses in the `$04:...` handshake range are unresolved/unknown even in the
  reference implementation (commented as such) — port as no-ops rather than inventing behavior.
- Every CPU-side memory access increments the ARM's own cycle counter by 1 per byte-lane touched
  (4 for a word access, matching real ARM's byte-lane-serial bus), and every idle cycle also
  increments it by 1 — this is the actual cycle-timing source the catch-up loop consumes.

## Suggested implementation order

1. **Barrel shifter + condition codes + ALU core** — done (`crate::coproc::armv3`, PR #24). Pure
   functions, fully unit-tested against the ARM ARM's documented truth tables — no CPU state
   needed yet.
2. Register file + mode-switch banking, tested via bank round-trips (write a banked reg, switch
   mode and back, confirm the original mode's value is preserved).
3. The 3-stage pipeline model + R15's implicit +8 exposure, with a dedicated test asserting the
   exact R15 value observed by an instruction at each pipeline stage — get this right BEFORE
   porting any instruction, per the gotcha above.
4. Data processing (the biggest single instruction class, but the most mechanical once 1-3 are
   solid).
5. Branch, MSR/MRS, software interrupt/exception entry.
6. LDR/STR (single data transfer) — every addressing-mode permutation.
7. LDM/STM (block data transfer) — the highest-complexity instruction, tackle last with maximum
   pipeline/register-bank confidence already established.
8. Multiply/multiply-long, single data swap.
9. The SNES-side board wrapper: SA-1-style deterministic master-clock catch-up, the handshake
   registers, firmware loading for PRG/data ROM + work RAM save-state.
10. Wire into `board::select` for `Coprocessor::St018` (new header enum variant + chipset-byte
    detection — Star Ocean's title, no existing title-match precedent to crib from).

No commercial Star Ocean ROM exists in this project's local corpus (the same honesty gap already
carried openly for ExLoROM/PAL auto-detect/standalone S-RTC) — this will land `BestEffort`-tier,
unit-tested only, unless a dump becomes available.
