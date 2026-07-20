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

Sources, all three agreeing: Nintendo's development manual (primary, expressed in MHz — 3.58 /
2.68 / 1.79, which are master/6, master/8 and master/12); anomie's `memmap.txt` v1.1, whose changelog
records *"Tested the memory access speed of all 256-byte memory blocks, and filled in the table with
the findings"* — an exhaustive hardware sweep; and fullsnes, independently, which additionally
documents `REFRESH` as a physical S-CPU pin.

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
anomie: DMA runs at 2.68 MHz *"regardless of the address"* — **8 master clocks per byte, region
independent** — and `$420D` bit 0 defaults to 0 (slow) at power-on and reset.

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

## 8. Open follow-up

WDC's datasheets have accumulated typos across revisions, and a single vendor rendering is a single
point of failure. **GTE** and **VLSI (VL65C816)** both published their own detailed
instruction-operation tables — anomie preferred the GTE one, *"particularly nice, as it identifies
the CPU activity for each cycle of the instruction"*. Three vendor datasheets agreeing on a cycle
row would be materially stronger than one. Worth chasing before the sweep is scored.
