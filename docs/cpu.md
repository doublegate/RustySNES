# WDC 65C816 (Ricoh 5A22) — RustySNES

**References:** `ref-docs/research-report.md` §4; `docs/scheduler.md` (the access-speed map);
`docs/adr/0001`. Sources cited inline: Super Famicom Dev wiki 65816 reference, undisbeliever
65816 opcodes, SNESdev S-CPU, WDC 65C816 (Wikipedia).

This doc is the SPEC, not history — update it in the same PR as the code. Pin behavior
against the test ROMs first.

## Purpose

The main CPU is a WDC 65C816 16-bit core wrapped in Nintendo's Ricoh **5A22** package. The
5A22 wrapper adds the multiply / divide registers, the DMA/HDMA hardware, the NMI/IRQ
timers, and joypad auto-read — those live in `rustysnes-core` (the Bus), not the CPU crate
(`docs/architecture.md` §2). `rustysnes-cpu` implements only the 65C816 instruction core; it
has no PPU/APU/cart dependency.

## Registers and state

Per `ref-docs/research-report.md` §4 (Super Famicom Dev wiki 65816 reference):

| Register | Width | Role |
|---|---|---|
| A | 8/16 | accumulator (width = M flag) |
| X, Y | 8/16 | index (width = X flag) |
| S | 16 | stack pointer |
| D | 16 | direct-page register |
| DBR | 8 | data bank register |
| PBR (PB/K) | 8 | program bank register |
| PC | 16 | program counter |
| P | 8 | status: N V M X D I Z C (+ the E flag, hidden) |

24-bit address space (16 MiB) via the bank registers (PBR for code, DBR for data).

## Emulation vs native mode (the E flag)

The CPU powers on in **6502 emulation mode** — behaves as a 65C02 with NMOS cycle counts; A
and the index registers are locked to 8 bits. Code does `CLC : XCE` to enter **native mode**,
where the **M** (accumulator/memory width) and **X** (index width) status bits select 8- or
16-bit registers via `REP` / `SEP`. The E flag is exchanged with C by `XCE`
(`ref-docs/research-report.md` §4). RESET forces emulation mode.

## Vectors

RESET / NMI / IRQ / BRK / COP / ABORT, with **separate emulation-mode and native-mode vector
tables** at the top of bank 0 (`ref-docs/research-report.md` §4). The reset vector is read at
power-on / `F3` power-cycle; NMI/IRQ are raised by the scheduler's timer phase
(`docs/scheduler.md` §H/V-IRQ).

## Timing — variable instruction cycles

The CPU is **not** a fixed master-clock divisor. Each memory access costs 6, 8, or 12 master
clocks per the region map in `docs/scheduler.md`; internal (I/O) cycles always cost 6. On top
of the per-access speed, instruction cycle *counts* vary
(`ref-docs/research-report.md` §4):

- **+1 cycle if m = 0** (16-bit memory / accumulator access).
- **+1 cycle if the low byte of D is non-zero** (direct-page misalignment).
- **+1 cycle if an indexed access crosses a page boundary.**

So a single opcode's master-clock cost is `Σ(access_speed_i) + internal_cycles×6`, where the
number of accesses depends on the M/X widths and the addressing mode.

**Sub-cycle access phase (ares `CPU::read`/`write`/`idle`).** Within a cycle the memory access is
*not* simultaneous with the clock advance — the CPU asks the Bus for the access cost
(`Bus::access_cycles`, ares `wait`) and sequences the advance (`Bus::advance`, ares `step`) around
the access so it lands at the hardware-exact instant:

- **write** — advance the full cost, *then* store: the write lands at the **end** of its cycle.
- **read** — advance `cost − 4`, read, then advance `4`: the read lands **four clocks before** the
  cycle end.
- **internal (I/O)** — a flat six-clock advance, no access.

This phase is load-bearing: it fixes the exact hcounter at which a register write becomes visible to
the PPU/HDMA (a store is seen a cycle later than a same-address read). It does not change instruction
cycle *counts*.

## Interfaces (sketch)

```rust
// rustysnes-cpu
pub trait CpuBus {
    /// Returns the value; the impl advances the master clock by the
    /// region's access speed (6/8/12) and ticks PPU/HDMA/timers.
    fn read(&mut self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, value: u8);
    /// An internal (no-bus) cycle: always 6 master clocks.
    fn io(&mut self);
}

pub struct Cpu { /* A,X,Y,S,D,DBR,PBR,PC,P,E */ }
impl Cpu {
    pub fn step(&mut self, bus: &mut impl CpuBus); // one instruction
}
```

## Edge cases and gotchas

1. **Width changes mid-instruction.** `SEP`/`REP` change M/X, which changes the byte count of
   subsequent accesses; the cost formula must read width *after* the flag write.
   (`ref-docs/research-report.md` §4)
2. **Emulation-mode stack wrapping.** In E-mode S is fixed to page 1 ($01xx); native mode
   uses the full 16-bit S. (`ref-docs/research-report.md` §4)
3. **Direct-page misalignment penalty** keys off `D & 0xFF != 0`, not the access address.
4. **Page-cross penalty** applies to indexed modes only, and only when the effective address
   crosses a 256-byte boundary.
5. **WRAM-refresh stall** (`docs/scheduler.md`) and **DMA halt** (`MDMAEN`) freeze the CPU
   mid-instruction — the CPU must be steppable at access granularity, not whole-instruction.

## Test plan

- **Primary oracle:** SingleStepTests/65816 JSON — per-opcode, all addressing modes, 8/16-bit,
  native + emulation, **with cycle-by-cycle bus-pin trace**. *License snag:* the 65816 set
  ships **no LICENSE** — keep it in the gitignored external tier or generate equivalent JSON
  (`docs/testing-strategy.md` §licensing; `docs/adr/0003` posture).
- **Committable layer:** gilyon/snes-tests (MIT) `.sfc` ROMs cover 65C816 opcodes (all addr
  modes, emulation + native, wrapping) with golden `tests*.txt` tables.
- Krom/PeterLemon CPU ROMs (reference-only, no license) for cross-checks.

## Open questions

- Exact master-clock breakdown for the rarer addressing modes — resolved against the
  SingleStepTests bus traces during Phase 1 (`ref-docs/research-report.md` "Open questions"
  #1). The SPC700 is documented separately in `docs/apu.md`.

## Implementation status

Phase-1 instruction core in `crates/rustysnes-cpu` (modules `regs.rs`, `addr.rs`, `exec.rs`).
Behavior modelled clean-room on the bsnes / ares `wdc65816` reference cores (study-only).

**Oracle results** (SingleStepTests/65816, `tests/cpu_oracle.rs`): **100.00% — 5,119,999 /
5,120,000** full passes (state + RAM + cycle) across the entire set (512 opcode files × 10,000
tests, both emulation `.e` and native `.n`); sampled runs (20,480 and 102,400 tests) are a clean
100%. The single residual is one `e1.e` (`SBC (dp,X)`, emulation) case exercising the bsnes
`readDirectX` `DL!=0` high-byte wrap that the rest of the SingleStepTests set does not model — a
documented inter-reference divergence, not point-fixed. The four block-move files (`44/54.e/.n`
MVP/MVN) pass too: the CPU keeps true per-byte hardware semantics, and since the block-move
fixtures cap each case at a fixed non-architectural cycle budget (A never wraps), the oracle
runner replays the move to that budget for the cross-check. This satisfies Phase 1's
per-opcode-oracle exit criterion (`docs/adr/0005` self-gen oracle of record is the follow-on).

**Implemented & tested:**

- Full register file (A/X/Y 8-16 bit by M/X, S/D/DBR/PBR/PC, P + hidden E), with 8-bit index
  high-byte zeroing and emulation-mode page-`$01` stack confinement.
- **Emulation-mode invariants** asserted at every instruction boundary (and before stack
  pushes), matching bsnes: `S.h` forced to `$01`, `M`/`X` status bits forced set, `X.h`/`Y.h`
  forced to `0`. This is what makes the `.e` (emulation) oracle tests pass.
- **Direct-page addressing** ported from bsnes `memory.cpp`: the emulation-mode `DL==0`
  page-lock (`readDirect` → `(D & 0xFF00) | (addr & 0xFF)`), per-byte high-byte wrap for
  pointer fetches, `readDirectN` (no page-lock) for long-indirect, and bank-`0` high-byte
  wrap for 16-bit direct/stack operands.
- **Two stack-push disciplines:** page-`$01`-confined `push`/`pull` (PHA/PHP/PHX/PHY/PHB/PHK/
  JSR/RTS/interrupts) vs. full-16-bit `pushN`/`pullN` then `S.h=$01` at the boundary
  (PEA/PEI/PER/PHD/PLD/PLB), matching bsnes exactly.
- `reset()` → emulation mode, RESET vector load.
- All 256 opcodes execute (none panic): the complete load/store/transfer/stack/flag/ALU/
  inc-dec/shift-rotate/branch/jump-call-return/block-move/interrupt set, plus `XBA`, `STP`,
  `WAI`, `NOP`, `WDM`. All documented addressing modes.
- `ADC`/`SBC` binary **and** decimal mode — digit-wise BCD correction ported exactly from
  bsnes `algorithms.cpp` (the V/C/Z/N semantics, including the `>0x9F` / `<=0xFF` fixups).
- `REP`/`SEP`/`XCE` width + emulation transitions; `PLP`/`RTI` re-mask M/X in emulation.
- Interrupt vectoring (NMI/IRQ/BRK/COP, native vs emulation tables); IRQ gated on `I` clear;
  NMI/IRQ polled at the instruction boundary.
- **Cycle counts** are the standard-table base plus the documented `+1` adjustments (M=0
  16-bit access, D-low≠0, indexed page-cross, branch-taken / emulation branch page-cross), and
  `STP`/`WAI` cost the oracle's 4 cycles. `step()` returns the CPU-cycle count == the
  `cycles`-field delta == the number of `Bus::on_cpu_cycle()` ticks; the per-access 6/8/12
  master-clock weighting stays the Bus's job, as the scaffold intended.

Full-run measurement (all 512 files × **10 000** tests = 5 120 000) is **5 119 999 / 5 120 000
= 100.00%** full passes (state + RAM + cycle). The single residual is one `e1.e`
(`SBC (dp,X)`, emulation) test that exercises the bsnes `readDirectX` `DL!=0` high-byte wrap
which the rest of the SingleStepTests set does not model — a documented inter-reference
divergence (`docs/adr/0002` posture), deliberately not point-fixed because matching it would
regress the other 9 999 `e1.e` tests.

**Approximated / deferred (Phase 1 scope honesty):**

- **`MVN`/`MVP` block moves** keep true per-byte hardware semantics — one byte per `step()` with
  PC rewound to re-enter (`if(A.w--) PC.w -= 3`, ares `instructionBlockMove`), so a real N-byte
  move takes N steps and a 3-byte move is correct. The SingleStepTests block-move fixtures cap
  each case at a fixed, non-architectural cycle budget (A never wraps; every test moves exactly
  14 bytes), so `tests/cpu_oracle.rs` replays the move to that recorded budget for the
  cross-check (see the runner's MVN/MVP note). With that, the four block-move files pass.
- Cycle counts are per-instruction tallies, not a cycle-by-cycle bus-pin trace; access *order*
  is reasonable but not validated against the SingleStepTests pin traces.
- `STP` idles until reset; `WAI` idles until a polled NMI/IRQ — exact wake-edge timing approx.
- ABORT and sub-instruction interrupt injection mid-RMW are not modelled.
