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
number of accesses depends on the M/X widths and the addressing mode. The CPU crate exposes
each cycle's intended access (read/write, address, internal) to the Bus, which returns the
speed; the scheduler advances the master clock accordingly (`docs/scheduler.md` §master-clock
model).

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
