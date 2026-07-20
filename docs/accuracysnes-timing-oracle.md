# AccuracySNES — the instruction-timing oracle

**Status:** the oracle T-04-I was blocked on. This document is RustySNES's own expression of the
rules, derived from the sources cited in each section. It reproduces no third-party table.

## Why an oracle was needed at all

`A5.08` measured the three reference emulators disagreeing with each other on instruction-level
timing — RustySNES and snes9x report different bitmasks for the same spot checks. So no emulator can
adjudicate, and a 256-opcode sweep scored against one would be circular. Everything below is
therefore held to a single test: *is this traceable to a manufacturer document or a hardware
measurement, with no emulator in the chain?*

## 1. There is no Ricoh 5A22 datasheet

Searched and not found: no vendor datasheet, no pinout from Ricoh, no patent documenting its bus
timing. The 5A22 was a Nintendo-exclusive part with no merchant-market customers, so unlike WDC's
65816 it was never publicly documented. Wikipedia's Ricoh 5A22 article cites exactly one source —
anomie's memory-mapping document — and every downstream encyclopedia inherits from it.

What does exist: a die photograph of the S-CPU-A (siliconpr0n) and reverse-engineered KiCad
schematics from it (`rgalland/SNES_S-CPU_Schematics`). Both are useful structurally and **neither
gives core timing**: the schematics identify all cell types *except* the 65816 section, and the
analysis is metal-layer only.

## 2. The architectural fact that makes the WDC datasheet usable

The 5A22 is a **stock WDC 65816 core plus a clock-stretching wait-state generator**. It is not a
reimplementation, and its instruction timing is not Ricoh's invention. Three independent lines agree:

- The 5A22's own pinout exposes **VDA and VPA as physical pins** — the same signals WDC's Table 5-7
  documents per cycle.
- Nintendo's manual is consulted locally (see `ref-docs/nintendo/`, gitignored — it is Nintendo
  confidential material, never redistributed) and its §21.1 text was read directly rather than taken
  second-hand.
- Nintendo's development manual states the CPU *"is operated internally with a 3.58MHz clock
  speed"* — 3.58 MHz being master/6, i.e. the core's native cycle is the 6-clock floor.
- The community consensus from people who have probed it: the 5A22 holds the core's clock input for
  6, 8 or 12 master clocks depending on what the core puts on its address bus, and adds **no**
  wait states when VDA and VPA are both low.

**Consequence — this is the whole result:** WDC Table 5-7's cycle structure is valid for the SNES,
and the SNES-specific part reduces to a wait-state map. The oracle is therefore two layers, and
both are emulator-independent.

## 3. Layer one — cycle classification (WDC W65C816S, Table 5-7)

Per addressing mode, Table 5-7 gives each cycle's `VDA`/`VPA`/`RWB` state. Classify:

```text
VDA = 0 and VPA = 0   ->  internal operation (no bus access)
otherwise, RWB = 1    ->  read
           RWB = 0    ->  write
```

A useful cross-check that needs no table lookup, since the 65816 (unlike the 6502) does not perform
dummy memory transfers: **internal cycles = total cycles − bytes accessed**.

See `ref-docs/2026-07-20-wdc-w65c816s-citation.md` for the document, its copyright terms, and why
its tables are not reproduced in this repository.

## 4. Layer two — the SNES memory speed map

Master clock 21,477,272.7 Hz (NTSC). A cycle costs 6, 8 or 12 master clocks:

| Region | Master clocks |
|---|---:|
| Internal cycles (`VDA=0, VPA=0`) — **any address** | **6** |
| `$00-$3F` / `$80-$BF` : `$0000-$1FFF` (WRAM mirror) | 8 |
| `$00-$3F` / `$80-$BF` : `$2000-$3FFF` (B-bus, I/O) | 6 |
| `$00-$3F` / `$80-$BF` : `$4000-$41FF` (joypad serial) | **12** |
| `$00-$3F` / `$80-$BF` : `$4200-$5FFF` (CPU I/O) | 6 |
| `$00-$3F` / `$80-$BF` : `$6000-$7FFF` (expansion) | 8 |
| `$00-$3F` : `$8000-$FFFF` (cart) | 8 |
| `$40-$7D` : all (cart) | 8 |
| `$7E-$7F` : all (WRAM) | 8 |
| `$80-$BF` : `$8000-$FFFF`, and `$C0-$FF` : all | **6 if `$420D` bit 0 set, else 8** |

**What corroborates what — the distinction matters.** Nintendo's development manual (Book I,
Ch. 21 §21.1) is primary for the *speeds* and for *`MEMSEL` semantics*, verified against the
document: three clock speeds *"3.58MHz, 2.68MHz, and 1.79 MHz"* selected by address, switchable by
*"D0 of register &lt;420DH&gt;"*, defaulting to 2.68 MHz. It does **not** give the per-range map in
text — that is Figures 2-21-1/2-21-2, an image the OCR cannot read, and §21.1 merely says *"refer to
'Frequency & Address Mapping' for the relation between the address and the clock."*

So the **per-range table above comes from anomie's `memmap.txt` v1.1**, whose changelog records
*"Tested the memory access speed of all 256-byte memory blocks, and filled in the table with the
findings"* — an exhaustive hardware sweep — corroborated by fullsnes independently, which also
documents `REFRESH` as a physical S-CPU pin. Nintendo corroborates the *speed values* those two
report, not the row-by-row assignment.

**The 6/8/12 master-clock framing is ours.** Nintendo expresses everything in MHz and never states
the master clock numerically anywhere in the manual. 3.58 / 2.68 / 1.79 MHz are master/6, master/8
and master/12 given a 21.477 MHz master — a figure that needs a separate source (the console
schematic, or the NTSC colourburst relation 3.579545 × 6).

> **Erratum — do not propagate.** anomie's `memmap.txt` says the FastROM select is **bit 1** of
> `$420D`. It is **bit 0**. `timing.txt`, fullsnes and Nintendo's manual (*"D0 of register
> &lt;420DH&gt;"*) all agree on bit 0, and `memmap.txt` is alone. AccuracySNES `B1.01` asserts bit 0
> and passes on all three emulators.

## 5. Where the datasheet must be overridden

Two classes, both carried deliberately.

**A measured datasheet error.** anomie, from hardware: the first cycle of the IRQ/NMI sequence *"is
an opcode fetch cycle from PB:PC (typically 6 or 8 master cycles), not an IO cycle (always 6 master
cycles) as the datasheet claims."* Likely a WDC documentation error about the real 65816 rather than
a Ricoh change — but either way the measurement wins, and it is a good candidate for a test of its
own.

**Everything the core does not own.** DMA and HDMA stalls, the per-scanline refresh pause,
auto-joypad read, and the `$4000-$41FF` XSlow region are 5A22-external and appear nowhere in Table
5-7. Two facts worth recording from Nintendo's manual, because they are primary and independent of
anomie: DMA runs at 2.68 MHz *"regardless of the address"*, and `$420D` bit 0 *"becomes '0'"* when
power is applied or reset.

Be careful how much weight the first carries. The manual gives a **clock rate**, not a per-cycle
cost table — there is no "8 master cycles per byte" in it, no HDMA per-channel setup cost, and no
DMA/CPU arbitration penalty. Reading 2.68 MHz as 8 master clocks per byte is our inference from the
master-clock relation, and it happens to match anomie; it is not Nintendo stating a cycle count. For
per-cycle DMA and HDMA behaviour anomie remains the only source, and he flags parts of it as
guesswork (*"the exact timing of the read within the DMA period is not known"*).

The manual contains **no refresh content whatsoever** — the word does not appear in a timing sense
anywhere in Book I. The 40-master-clock pause is measured by anomie and corroborated physically by
fullsnes via the `REFRESH` pin, with no primary-source documentation behind it.

## 6. Provenance verdict on anomie's `timing.txt`

**Hardware-measured, not emulator-derived** — which is what makes it usable here.

Its stated method is a physical differential-timing experiment: a series of ROMs differing only in
the master-cycle delay before probing an event, exploiting the observation that the SNES returns to
a known timing position on reset so a deterministic ROM reproduces exactly. That is a measurement
protocol, not a reading of anyone's source.

It also marks its uncertainty consistently — *"the exact timing of the read within the DMA period is
not known"*, *"best guess at this point"*, *"presumably"*, *"current theory is"*, and an entire
renderer-timing section disclaimed as *"most of this is conjecture"* — the same discipline found in
his `regs.txt`. And it *corrects* the datasheet from measurement (§5), which no emulator-derived
document could produce.

Scope limit: `timing.txt` does **not** contain the speed map; that is `memmap.txt`. And the
40-master-clock refresh pause is measured, while its *attribution* to WRAM refresh is explicitly
labelled theory by anomie — fullsnes corroborates the mechanism physically via the `REFRESH` pin,
and Nintendo's manual never mentions refresh at all.

## 7. Sources that must NOT be used as the oracle

- **`SingleStepTests/65816`** (formerly TomHarte). Tempting — its vectors carry the exact
  VDA/VPA/RWB flags needed — but its own documentation says the vectors are produced by *"an
  implementation"* that conforms to documentation and passes other test sets. **Emulator-generated.**
  Fine as a consistency check, worthless as an oracle.
- **`wiki.superfamicom.org/timing`** — this *is* anomie's `timing.txt`, reformatted. Not a second
  vote.
- **Wikipedia / Grokipedia / Fandom / course notes on the 5A22** — all anomie's `memmap.txt`, one
  hop removed.
- **Eyes & Lichty, *Programming the 65816*** — WDC-endorsed, so it corroborates the WDC datasheet
  but is not independent of it.

## 8. Cross-vendor verification — three renderings, and what it caught

The 65816 was second-sourced, and three vendors published their own detailed instruction-operation
tables: **GTE** (1987 Microcircuits Data Book, "Table 9"), **VLSI** (1988, "Table 6"), and **WDC**
(2024, "Table 5-7"). All three carry the same column set — VP, ML, VDA, VPA, address bus, data bus,
R/W — and GTE's and VLSI's notes lists are verbatim identical (11 notes); WDC keeps those and adds
six more. All three are held locally under `ref-docs/datasheets/` (gitignored, all-rights-reserved
manufacturer documents) with a full row-by-row write-up in `COMPARISON.md`.

**The headline is reassuring: five of six sampled instructions agree bit-for-bit across all three** —
`LDA abs`, `STA abs,X` (including note 4's unconditional extra cycle, whose wording is verbatim
identical in all three), `ASL abs` (including `ML` held low across the locked read-modify-write
window and the reverse-order high-then-low writeback), `PLA`, `JSR abs`, and `MVN` across all three
iterations. That is what makes the oracle usable: these are not one document's assertions.

**Three disagreements, and they are the more valuable output.**

| # | What | Disposition |
|---|---|---|
| 1 | **`PHA`: VLSI marks both push cycles `R/W = 1` (read).** GTE and WDC say 0 (write) | A VLSI typesetting slip, localised — its `PEA`/`PEI`/`JSR` writes are correctly 0. A push is a write; this is not a real disagreement. **Do not feed VLSI's `R/W` column in as a blind third vote without inspecting the block.** |
| 2 | **Taken-branch internal cycles: what is on the address bus?** GTE says `PBR,PC+2` then `PBR,PC+2+OFF`; VLSI and WDC both say `PBR,PC+1` for both | **Measure it.** Flags and cycle counts are identical everywhere — only the address-bus contents differ. GTE's reading is the more physically plausible (PC has already passed the operand, and the second dead cycle is the page-fixup cycle). The 2-1 vote is weak evidence: WDC and VLSI plausibly inherited one simplification rather than confirming independently. **This is SNES-observable** — the address driven during an internal cycle is what the bus and MDR see, so open-bus and DMA-interaction behaviour can depend on it. |
| 3 | **`PHx` 16-bit extra cycle: which cycle carries note (1)?** GTE and VLSI attach it to cycle 3a (the register-high push, which only exists when the register is 16-bit); WDC attaches it to cycle 2 and puts its own note (12) on 3a | Annotation-level, no behavioural difference, but WDC stands alone and looks wrong. Prefer the 2-vendor reading. |

Also single-sourced and therefore worth testing rather than trusting: **WDC's note (17)** — *"In the
emulation mode, during a R-M-W instruction the RWB is low during both write and modify cycles"* — is
asserted by WDC alone and unstated by both 1980s renderings.

> **Caveat: do not cross-check using the vendors' opcode *counts*.** All three print per-mode opcode
> counts that contradict their own adjacent mnemonic lists, and WDC and VLSI share the same inherited
> errors (absolute: GTE 16 correct, WDC/VLSI 18 wrong; `abs,X`: GTE/VLSI 11 correct, WDC 12 wrong;
> absolute R-M-W: GTE 8 correct, WDC/VLSI 6 wrong). Only the **cycle rows** are trustworthy.

This exercise justified itself. It caught a vendor typo, an apparently-inherited simplification
masquerading as corroboration, and two single-vendor claims — none of which would have been visible
from WDC alone, and one of which (§8.2) is now a test candidate precisely because the documents
cannot settle it.

## 9. Open follow-up

~~Chase GTE and VLSI renderings~~ — **done, see §8.** anomie's preference for GTE
(*"particularly nice, as it identifies the CPU activity for each cycle of the instruction"*) proved
well placed: where the three disagree, GTE is right or more plausible in every case.

Remaining, in order:

1. **Three new test candidates from §8**, all of which exist because documents alone cannot settle
   them: the taken-branch internal-cycle address bus, WDC's emulation-mode R-M-W `RWB` note (17), and
   the IRQ/NMI first cycle where anomie's measurement already contradicts the datasheet (§5).
2. ~~A 16-bit measurement-reporting channel~~ — **done, and it immediately settled `REP`.**
   `MEAS_BASE` (`$7E:E200`, 64 `u16` slots) carries raw measurements to the host harness, since a
   one-byte verdict cannot hold a dot count. `A5.08` records seven slots and the harness prints and
   sanity-checks them.

   **`REP` is settled: RustySNES was right and the test was wrong.** The raw numbers showed a
   32-`NOP` baseline at 277 dots, so the 32-`REP` block sat at exactly **341 dots — one scanline** —
   and the H-counter difference wrapped to ~0. That read as "the emulator gets `REP` wrong". It does
   not: its `REP` is opcode fetch + operand fetch + one internal cycle, precisely what all three
   vendor tables specify. Rebuilt at 16 repeats, every prediction now lands exactly — `XBA` 24 dots,
   `REP` 32, `PHD`+`PLD` 76 — and `A5.08` is a **scored** test again.

   The general hazard is now documented on `measure_begin` and guarded in the harness: a measured
   span must stay under the 341-dot wrap, because past it the primitive returns a plausible small
   number instead of failing. That is the worst kind of failure — indistinguishable from a real
   reading — and it was invisible until raw values could be read back.

3. ~~Derive per-opcode expectations~~ — **done. The sweep runs (T-04-I).**
   `gen/src/tests/sweep.rs` carries a safe-operand table and a sandbox, and emits one test per
   opcode so a failure names the instruction rather than the batch. Expectations come from
   `6*cycles + 2*mem` against cycle counts all three vendor tables agree on. **22 entries, all
   passing.**

   Two things it caught on its first run, both worth recording:

   - **`LDX #imm` and `PHX`+`PLX` failed** — and the bug was the sandbox, not the emulator. It set
     `sep #$20`, narrowing only the accumulator, so the index registers stayed 16-bit while every
     expectation in the table is stated at `x=1`. At `x=0`, `LDX #imm` is a 3-byte 3-cycle fetch and
     `PHX`/`PLX` move two bytes rather than one. A sandbox has to establish the preconditions its
     table claims; `sep #$30` now does.
   - **snes9x mistimes `WDM`.** It is a reserved *two*-byte no-op costing 2 cycles / 2 accesses.
     snes9x gets the length right — it passes `A6.08`, the functional test — but not the timing.
     Declared in `crossval.sh` with its citation, and a narrower bug than it first appears.

   **Extended to 34 entries** covering implied, immediate, stack pairs, direct page, absolute,
   absolute long, indexed, read-modify-write, stores, and untaken branches — every class that inline
   repetition can measure. Memory operands are named checked-safe WRAM addresses rather than a
   blanket rule, because with `DBR=$00` an absolute operand is within reach of MMIO.

   The untaken-branch entry is worth reading before adding more like it: the condition must be
   established **inside** the measured body. Setting it in the sandbox does not survive, because
   `measure_begin` emits a `jsr` whose callee clobbers the flags. The first version relied on the
   sandbox's `ldx #$00` leaving `Z` set and silently measured a *taken* branch instead — the two
   differ by 12 dots here, so it failed loudly, but a smaller gap would have passed quietly.

   Still outside the sweep, and each needs different machinery rather than more table rows: taken
   branches and control flow (`JMP`, `JSR`, `JSL`, `RTS`, `RTL`, `RTI`) move `PC` and cannot be
   repeated inline; `BRK`/`COP` vector away; `WAI` waits for an interrupt. `STP` is permanently
   excluded — it halts the CPU until reset, so a self-scoring battery that executes it never
   reports.

## 10. The three cross-vendor candidates — assessed

§8 turned up three rows the documents could not settle. Assessing them for testability first, rather
than writing tests and discovering the problem afterwards:

**1. Taken-branch internal-cycle address bus — NOT OBSERVABLE. No test is possible.**
The disagreement is real (GTE says `PBR,PC+2` then `PBR,PC+2+OFF`; VLSI and WDC say `PBR,PC+1`
twice) but it cannot be detected from software on this machine, and an earlier note in this document
claiming it was "SNES-observable" was **wrong**. An internal cycle is defined by `VDA=0, VPA=0`, and
that is precisely the condition under which the 5A22 performs **no bus access at all** — no read, no
write, and no wait state. Nothing fetches from the address, nothing is written to it, and open bus
latches *data*, not addresses. The core drives an address no part of the system consumes. Settling
this needs a logic analyser on the physical address pins, not a test ROM.

**2. WDC note (17), emulation-mode R-M-W `RWB` — testable, and worth writing.**
WDC alone asserts that in emulation mode `RWB` is low during *both* the write and the modify cycle
of a read-modify-write; GTE and VLSI are silent. If true, the modify cycle performs a **write**, so
an R-M-W against a write-sensitive register writes twice — observable through any register with a
write side effect (an auto-incrementing port is the obvious probe). This is the one candidate that
converts cleanly into an on-cart test.

**3. IRQ/NMI first cycle — testable in principle, hard in practice.**
anomie measured it as an opcode fetch (6 or 8 clocks depending on region) against the datasheet's
internal cycle (always 6). The difference is 2 clocks on a single event, well under the measurement
noise floor for one occurrence, and interrupt entry cannot be repeated inline the way an instruction
can. It needs a harness that arms and services many interrupts inside one measurement window —
related to, but distinct from, the control-flow batch.

4. Rockwell never second-sourced the 16-bit part, so three vendors is the ceiling here.
