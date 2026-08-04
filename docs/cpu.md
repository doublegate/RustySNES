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

The 5A22's CPU I/O registers (`$4200-$421F`) — `NMITIMEN`, `RDNMI` (`$4210`), `TIMEUP` (`$4211`),
`HVBJOY` (`$4212`), the auto-joypad ports (`$4218-$421F`), etc. — are handled in `rustysnes-core`'s
bus, not the 65816 core, so their exact bit layouts and read side effects (RDNMI/TIMEUP open-bus in
the unused bits + read-clearing flags; HVBJOY vblank/hblank/auto-joypad-busy; the timed 4224-clock
automatic joypad read) are specified in **`docs/scheduler.md` §NMI/IRQ**, the owning subsystem doc.

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

**Per-scanline DRAM-refresh stall (external to instruction timing).** Independently of any opcode,
the 5A22's memory controller steals the bus once per scanline to refresh work RAM, freezing the
65C816 for 40 master clocks at line-clock ≈ 536. RustySNES injects this in the scheduler
(`Bus::advance_master`), **not** in opcode cycle counts — so the single-step CPU oracle is unaffected
— but it does move where an in-flight instruction's remaining accesses land relative to the beam: an
ISR whose execution straddles the refresh point commits its register write ~10 dots later, which is
the observable behind mid-line raster timing. Because the frame length is fixed by the PPU
dot-counter rollover (357,368 master clocks for NTSC) rather than by CPU work, the stall reallocates
the CPU's share of that fixed budget rather than lengthening the frame. See `docs/dram-refresh.md`
and `docs/scheduler.md` §DRAM refresh.

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

**Oracle results** (SingleStepTests/65816, `crates/rustysnes-test-harness/tests/cpu_oracle.rs`): **5,119,710 / 5,120,000
(99.994%)** full passes (state + RAM + cycle) across the entire set (512 opcode files × 10,000
tests, both emulation `.e` and native `.n`). The **290 residuals are all emulation-mode cases where
SingleStepTests is the lone outlier** and RustySNES matches the SNES-accurate references
(bsnes/ares/MesenCE/snes9x) and its own AccuracySNES first-party suite — two documented
inter-reference divergences resolved toward hardware (`docs/adr/0002`), not bugs:

- **247** across the eight `(dp,X)` emulation files (`01/21/41/61/81/a1/c1/e1.e`): the WDC 65816
  silicon bug that wraps the `(dp,X)` pointer high byte within the page when `E=1 && DL!=0`.
  bsnes/ares `readDirectX` and MesenCE `GetDirectAddressIndirectWordWithPageWrap` both model it and
  call it a CPU bug; gilyon's `cputest-full` tests it (and now passes). SST reads it linearly.
- **43** in `fc.e` (`JSR (addr,X)`, emulation): the return-address push **escapes the emulation-mode
  page-1 stack bound** — on a page crossing (`S=$xx00`) the second byte lands at `$00FF`, not
  `$01FF`. Confirmed by three independent SNES-accurate implementations: bsnes/ares (one lineage)
  `instructionCallIndexedIndirect` (`pushN`), snes9x `OpFCE1` (`PushW`/`WRAP_BANK`, source comment
  *"JSR (a,X) is a new instruction, and so doesn't respect the emu-mode stack bounds"*), and MesenCE
  `JSR_AbsIdxXInd` (`PushWord(PC, allowEmulationMode=false)` + `RestrictStackPointerValue()`); the
  AccuracySNES `A3.08` result is cross-validation on snes9x + Mesen2, not a separate oracle. SST
  confines the push to page 1 and is the lone outlier; RustySNES matches these implementations on
  every physically-reachable state (`S.h=$01`).

Provenance supports resolving both toward hardware. The `SingleStepTests/65816` set documents no
generation method — its README calls it "an aid to reimplementation," with no stated hardware
oracle. The `(dp,X)` wrap and the `$FC`/`PLB` page-1 escapes, by contrast, are **hardware-derived**:
gilyon's `snes-tests` are credited on nesdev with *discovering two undocumented behaviors that all
major emulators had not implemented* (the `(dp,X)` wrap among them), which bsnes/ares/MesenCE then
adopted — so SST is simply the test set that predates those findings. The general rule is documented
in the WDC W65C816S datasheet and confirmed on real SNES hardware (nesdev
[t=14281](https://forums.nesdev.org/viewtopic.php?t=14281)): emulation mode preserves 6502 behavior
only for the modes that exist on the 6502; **65816-specific additions (`JSR (a,X)`, `PLD`/`PLB`,
`PEA`/`PEI`/`PER`, and the `[dp]`/`[dp],Y`/`PEI` long-pointer read that increments from `$00FE/FF`
into the stack) use native behavior in emulation mode.** RustySNES models every one of these
(`push_n*`/`pull_n*`, `direct_n_addr`), matching bsnes and passing gilyon `cputest-full` — so the
two SST divergences are the *complete* known set, not a sample.

An earlier figure of `5,119,999 / 5,120,000` in this doc predated both the `A3.08` `$FC` push fix
and the `(dp,X)` wrap; sampled runs (`RUSTYSNES_ORACLE_PER_FILE=200`) miss most of these page-edge
cases and still read ~100%. The four block-move files (`44/54.e/.n`
MVP/MVN) pass too: the CPU keeps true per-byte hardware semantics, and since the block-move
fixtures cap each case at a fixed non-architectural cycle budget (A never wraps), the oracle
runner replays the move to that budget for the cross-check. This satisfies Phase 1's
per-opcode-oracle exit criterion (`docs/adr/0005` self-gen oracle of record is the follow-on).

**Implemented & tested:**

- Full register file (A/X/Y 8-16 bit by M/X, S/D/DBR/PBR/PC, P + hidden E), with 8-bit index
  high-byte zeroing and emulation-mode page-`$01` stack confinement.
- **Emulation-mode invariants** asserted at every instruction boundary (and before stack
  pushes), per the documented emulation-mode rules: `S.h` forced to `$01`, `M`/`X` status bits forced set, `X.h`/`Y.h`
  forced to `0`. This is what makes the `.e` (emulation) oracle tests pass.
- **Direct-page addressing** per the documented 65C816 addressing rules: the emulation-mode `DL==0`
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
  NMI/IRQ polled at the instruction boundary. Hardware NMI/IRQ open with **two** internal
  (dead) cycles before pushing the return frame (WDC sequence: 2 internal + push PBR[native] +
  push PCH + push PCL + push P + 2 vector fetches). BRK/COP (their own opcodes, cycle-validated
  by the single-step oracle) are handled separately and keep their standard-table counts — the
  two-cycle interrupt open is on the hardware NMI/IRQ path only, and also feeds interrupt
  *latency* (it delays an IRQ-driven register write by one cycle, one input to the
  `hdmaen_latch_test` HDMAEN-vs-latch race; `docs/scheduler.md` §H/V-IRQ).
- **Cycle counts** are the standard-table base plus the documented `+1` adjustments (M=0
  16-bit access, D-low≠0, indexed page-cross, branch-taken / emulation branch page-cross), and
  `STP`/`WAI` cost the oracle's 4 cycles. `step()` returns the CPU-cycle count == the
  `cycles`-field delta == the number of `Bus::on_cpu_cycle()` ticks; the per-access 6/8/12
  master-clock weighting stays the Bus's job, as the scaffold intended.

Full-run measurement (all 512 files × **10 000** tests = 5 120 000) is **5 119 710 / 5 120 000
= 99.994%** full passes (state + RAM + cycle). All 290 residuals are the two emulation-mode
SST-outlier divergences described above (247 `(dp,X)` high-byte wrap + 43 `fc.e` `$FC` push), each
resolved toward the SNES-accurate references (bsnes/ares/MesenCE/snes9x) and the AccuracySNES
first-party suite rather than toward SingleStepTests (`docs/adr/0002` posture). The `(dp,X)` wrap in
particular makes gilyon's `cputest-full` (1610 tests, gated in `tests/gilyon_oncart.rs`) pass in
full; leaving it linear to keep the SST cases would fail real hardware and every accurate emulator.

**Approximated / deferred (Phase 1 scope honesty):**

- **`MVN`/`MVP` block moves** keep true per-byte hardware semantics — one byte per `step()` with
  PC rewound to re-enter (`if(A.w--) PC.w -= 3`, ares `instructionBlockMove`), so a real N-byte
  move takes N steps and a 3-byte move is correct. The SingleStepTests block-move fixtures cap
  each case at a fixed, non-architectural cycle budget (A never wraps; every test moves exactly
  14 bytes), so `crates/rustysnes-test-harness/tests/cpu_oracle.rs` replays the move to that recorded budget for the
  cross-check (see the runner's MVN/MVP note). With that, the four block-move files pass.
- Cycle counts are per-instruction tallies, not a cycle-by-cycle bus-pin trace. The **read/write
  access order** is now cross-checked against the SingleStepTests per-cycle trace by
  `crates/rustysnes-test-harness/tests/cpu_oracle.rs` (T-CA-11): final state passes 99.99% and cycle *count* 100%, but among the
  state-and-cycle-exact, non-block-move cases the access *order* matches only ~25% (state/cycle-failing
  cases are excluded from that denominator — their access mismatch would be a symptom of the semantic
  divergence, not a dummy-cycle fact). The divergence is confined to the **dummy cycles** of read-modify-write
  (`ASL/LSR/ROL/ROR/INC/DEC/TSB/TRB`), indexed-indirect pointer fetches, and stack pushes, where the
  emulator performs an internal `io` where hardware drives a real dummy read/write. Because the end
  state and cycle count are exact and the only open-bus-order case that mattered is handled
  (`docs/scheduler.md` §Open bus via DMA/HDMA, T-CA-12), a pin-exact rewrite has no current
  ROM-observable payoff — and a real risk (a dummy read of a clear-on-read I/O register has side
  effects the current `io` avoids), so it stays deferred. The oracle metric now makes it trackable if
  a title ever depends on it (T-CA-11 in `to-dos/TIER1-CYCLE-ACCURACY.md`).
- `STP` idles until reset; `WAI` idles until a polled NMI/IRQ, then spends **one extra internal
  cycle leaving the wait** before the interrupt sequence begins (`Cpu::wake_from_wai`) — matching
  ares' trailing `idle()` in `instructionWait` and MesenCE's `Idle()` in `ProcessHaltedState`.
  Without it a `WAI`-gated handler's first store landed ~1.5 dots early; adding it makes
  `undisbeliever`'s `inidisp_brightness_delay` match MesenCE exactly and moves the `hdmaen_latch`
  banding toward the reference (`docs/scheduler.md` §H/V-IRQ). The residual raster-boundary offset
  vs MesenCE is now the irreducible ares(`HTIME+3.5`)-vs-MesenCE(`HTIME+4.5`) trigger disagreement
  plus the interrupt-open read-vs-idle nuance — RustySNES follows ares (AccuracySNES B-group).
- ABORT and sub-instruction interrupt injection mid-RMW are not modelled.
