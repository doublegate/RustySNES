# undisbeliever — 65816 Opcodes (instruction timing table)

**External oracle extract for AccuracySNES.** Preserved verbatim from the source; no values
have been "corrected" by the extractor. See the *Caveats / errata* section for a
source-internal inconsistency that is flagged but **not** edited.

## Provenance

| Field | Value |
|---|---|
| Title | 65816 Opcodes |
| Author | undisbeliever (Marcus Rowe), *"with contributions by InsaneFirebat"* |
| Canonical HTML page | <https://undisbeliever.net/snesdev/65816-opcodes.html> |
| Canonical source (markdown) | <https://github.com/undisbeliever/snesdev-notes/blob/master/pages/65816-opcodes.md> |
| Raw markdown fetched | <https://raw.githubusercontent.com/undisbeliever/snesdev-notes/master/pages/65816-opcodes.md> |
| Section index | <https://undisbeliever.net/snesdev/index.html> |
| Source "Last Modified" | 13 December 2023 |
| Retrieved | 2026-07-20 |
| Opcodes captured | 256 / 256 |

## Licence

The source markdown carries this YAML front matter verbatim:

```yaml
title: 65816 Opcodes
tags: SnesDev
copyright: © 2019 undisbeliever with contributions by InsaneFirebat
license: CC BY-SA 4.0
license-url: https://creativecommons.org/licenses/by-sa/4.0/
```

The rendered page footer states verbatim:

> This page is © 2019 undisbeliever with contributions by InsaneFirebat.
>
> This page is licensed under a [Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA)](https://creativecommons.org/licenses/by-sa/4.0/)

**Verdict: CC BY-SA 4.0 — free/open and redistributable, but COPYLEFT, not permissive.**

* Redistribution and modification are permitted.
* Attribution is required (satisfied by the Provenance table above — keep it with the file).
* **ShareAlike**: any *adapted material* derived from this table must itself be
  distributed under CC BY-SA 4.0 (or a compatible licence). This is a real obligation
  and is stricter than the MIT/CC0/public-domain fixtures the repo normally commits.
* The `undisbeliever/snesdev-notes` GitHub repository has **no top-level `LICENSE` file**
  and the GitHub API reports `license: null` for it; the per-page front matter above is
  the only licence statement, and it applies to this page's content specifically.

**Action required before committing:** this file is safe to redistribute *as an
attributed, separately-licensed vendored reference*, but it is NOT drop-in
"permissive/public-domain". Keep it in a clearly-marked third-party reference directory
with this header intact, do NOT relicense it under the project licence, and if any
generated Rust table is a direct transcription of these values, treat that artifact as
adapted material (or re-derive the table from a public-domain source such as the WDC
W65C816S datasheet tables).

## Document preamble (verbatim from the source)

> Notes about this document:
>
> The psuedo-code described in this document does not describe the 65816
> internals but rather the change in machine state.
>
> The phrase *+1 if index crosses page boundary* means +1 cycle if:
>
>  * The Index register is 16 bits, or
>  * `(Addr + Index) & 0xffff00` ≠ `Addr & 0xffff00`

(The spelling "psuedo-code" is the source's.)

## Modifier notation

The source has **no symbolic shorthand** (no "m / x / w / p" letter codes). Every
conditional modifier is written out in the table's `Extra` column as English prose.
The complete set of distinct `Extra` clauses that occur across all 256 opcodes, with
their occurrence counts, is:

| Clause (verbatim) | Occurrences | Meaning |
|---|---|---|
| `+1 if m=0` | 142 | +1 cycle when the `m` status bit is clear, i.e. 16-bit accumulator/memory |
| `+1 if D.l ≠ 0` | 85 | +1 cycle when the low byte of the Direct Page register `D` is non-zero (direct page not page-aligned). Note the source names the register `D` (renamed from `DP` in commit `a8170f2e`, 2023-12-13) and writes `D.l`, not `DL`. |
| `+1 if x=0` | 26 | +1 cycle when the `x` status bit is clear, i.e. 16-bit index registers |
| `+1 if index crosses page boundary` | 24 | Defined by the preamble above: +1 if the index register is 16 bits **or** `(Addr + Index) & 0xffff00 ≠ Addr & 0xffff00` |
| `+2 if m=0` | 16 | +2 cycles when `m=0` — used for 16-bit read-modify-write to memory |
| `+1 if e=1` | 9 | +1 cycle in emulation mode (`e` set) |
| `+1 if branch taken` | 8 | +1 cycle if the conditional branch is taken |
| `+1 if e=0` | 3 | +1 cycle in native mode (`e` clear) |
| `7 per byte moved` | 2 | MVN/MVP block moves; the `Cycles` column is left blank and this is the whole timing statement |
| `additional cycles needed by interrupt handler to restart the processor` | 1 | WAI ($CB) |

Multiple clauses on one opcode are comma-separated in the `Extra` column and are
reproduced here unchanged.

Mapping to the modifier vocabulary used in the AccuracySNES dossier:
`m` = `+1 if m=0` · `x` = `+1 if x=0` · `w` = `+1 if D.l ≠ 0` ·
`p` = `+1 if index crosses page boundary`. The source additionally distinguishes
`+2 if m=0` (16-bit RMW) and the `e`-flag cases, which have no single-letter code.

## Caveats / errata

1. **16-bit RMW: the source is internally inconsistent and still carries the erratum.**
   For memory read-modify-write instructions, `ASL`, `INC`, `DEC`, `TRB` and `TSB` are
   listed as `+2 if m=0`, but `LSR`, `ROL` and `ROR` are listed as `+1 if m=0`
   (opcodes `$46 $4E $56 $5E` LSR, `$26 $2E $36 $3E` ROL, `$66 $6E $76 $7E` ROR).
   Hardware behaviour — and the dossier's stated correction — is **+2** for all 16-bit
   memory RMW, so the LSR/ROL/ROR rows below are believed wrong by +1 cycle when `m=0`.
   **The values below are left exactly as published.** Do not score a cycle sweep
   against the LSR/ROL/ROR 16-bit RMW rows without applying this correction.
   (Commit `de84e932`, 2021-03-13, "Fix incorrect extra cycles in read-modify-write
   instructions", corrected a *different* RMW problem — it removed the bogus
   "index crosses page boundary" penalty from the absolute-indexed RMW forms — and did
   not touch the `+1` vs `+2` discrepancy, which remains live as of the 2026-07-20 fetch.)

2. **MVN ($54) and MVP ($44)** have an empty `Cycles` cell in the source; the timing is
   given entirely as `7 per byte moved` in the `Extra` column.

3. **JMP/JML ($4C $5C $6C $7C $DC) and WDM ($42)** are published in raw-HTML tables with
   `rowspan` cells rather than markdown tables. Their two alternate syntaxes share one
   row; they are merged below as `A / B` in the Mnemonic column. Values are unchanged.
   The source lists no addressing mode for `WDM` (cell is empty).

4. `WDM` note from the source: *"On the SNES it does nothing. This instruction should not
   be used in your program."*

5. `STP` ($DB) and `WAI` ($CB) have open-ended real timing; only the published figures
   are reproduced.

6. **"Addressing mode" column.** Some of the source's tables (branches, software
   interrupts, clear/set status flags, push/pull, transfer registers) use a `Name`
   column instead of an `Addressing Mode` column. For those opcodes the Mnemonic's
   descriptive name (e.g. "Branch if Plus", "Clear Carry Flag", "Transfer A to Y")
   appears in the Addressing mode column below, verbatim from the source's `Name` cell.
   The branch tables also carry `Condition` columns (e.g. "carry clear" / `c=0`) which
   are not reproduced here as they are not timing data.

7. The source's own sources list is: *A 65816 Primer* (Brett Tabke); *W65C816S 8/16-bit
   Microprocessor Datasheet* (Western Design Center); *Programming the 65816*
   (Eyes & Lichty); *All_About_Your_64 - 65816 Reference* (Ninja/The Dreams);
   the *higan* `wdc65816` source (Near). It is therefore a secondary compilation,
   not a primary hardware measurement.

8. These are **CPU cycle counts**, not SNES master clocks. On the SNES each 65816 cycle
   is 6, 8 or 12 master clocks depending on the memory region and the FastROM (MEMSEL)
   setting; the source does not cover that conversion.

## Table (all 256 opcodes, ordered by opcode byte)

| Opcode | Mnemonic | Addressing mode | Bytes | Cycles | Modifiers |
|---|---|---|---|---|---|
| `$00` | BRK param | Interrupt | 2 | 7 | +1 if e=0 |
| `$01` | ORA (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$02` | COP param | Interrupt | 2 | 7 | +1 if e=0 |
| `$03` | ORA sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$04` | TSB dp | Direct Page | 2 | 5 | +2 if m=0, +1 if D.l ≠ 0 |
| `$05` | ORA dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$06` | ASL dp | Direct Page | 2 | 5 | +2 if m=0, +1 if D.l ≠ 0 |
| `$07` | ORA [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$08` | PHP | Push Processor Status Register | 1 | 3 | — |
| `$09` | ORA #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$0A` | ASL | Accumulator | 1 | 2 | — |
| `$0B` | PHD | Push Direct Page Register | 1 | 4 | — |
| `$0C` | TSB addr | Absolute | 3 | 6 | +2 if m=0 |
| `$0D` | ORA addr | Absolute | 3 | 4 | +1 if m=0 |
| `$0E` | ASL addr | Absolute | 3 | 6 | +2 if m=0 |
| `$0F` | ORA long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$10` | BPL near | Branch if Plus | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$11` | ORA (dp), Y | DP Indirect Indexed, Y | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0, +1 if index crosses page boundary |
| `$12` | ORA (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$13` | ORA (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$14` | TRB dp | Direct Page | 2 | 5 | +2 if m=0, +1 if D.l ≠ 0 |
| `$15` | ORA dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$16` | ASL dp, X | Direct Page Indexed, X | 2 | 6 | +2 if m=0, +1 if D.l ≠ 0 |
| `$17` | ORA [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$18` | CLC | Clear Carry Flag | 1 | 2 | — |
| `$19` | ORA addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$1A` | INC | Accumulator | 1 | 2 | — |
| `$1B` | TCS | Transfer 16 bit A to S | 1 | 2 | — |
| `$1C` | TRB addr | Absolute | 3 | 6 | +2 if m=0 |
| `$1D` | ORA addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$1E` | ASL addr, X | Absolute Indexed, X | 3 | 7 | +2 if m=0 |
| `$1F` | ORA long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |
| `$20` | JSR addr | Absolute | 3 | 6 | — |
| `$21` | AND (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$22` | JSL long | Absolute Long | 4 | 8 | — |
| `$23` | AND sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$24` | BIT dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$25` | AND dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$26` | ROL dp | Direct Page | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$27` | AND [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$28` | PLP | Pull Processor Status Register | 1 | 4 | — |
| `$29` | AND #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$2A` | ROL | Accumulator | 1 | 2 | — |
| `$2B` | PLD | Pull Direct Page Register | 1 | 5 | — |
| `$2C` | BIT addr | Absolute | 3 | 4 | +1 if m=0 |
| `$2D` | AND addr | Absolute | 3 | 4 | +1 if m=0 |
| `$2E` | ROL addr | Absolute | 3 | 6 | +1 if m=0 |
| `$2F` | AND long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$30` | BMI near | Branch if Minus | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$31` | AND (dp), Y | DP Indirect Indexed, Y | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0, +1 if index crosses page boundary |
| `$32` | AND (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$33` | AND (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$34` | BIT dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$35` | AND dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$36` | ROL dp, X | Direct Page Indexed, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$37` | AND [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$38` | SEC | Set Carry Flag | 1 | 2 | — |
| `$39` | AND addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$3A` | DEC | Accumulator | 1 | 2 | — |
| `$3B` | TSC | Transfer S to 16 bit A | 1 | 2 | — |
| `$3C` | BIT addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$3D` | AND addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$3E` | ROL addr, X | Absolute Indexed, X | 3 | 7 | +1 if m=0 |
| `$3F` | AND long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |
| `$40` | RTI | Stack (return interrupt) | 1 | 6 | +1 if e=0 |
| `$41` | EOR (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$42` | WDM #const / WDM param | — | 2 | 2 | — |
| `$43` | EOR sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$44` | MVP srcBank, destBank | Block Move | 3 | (none given) | 7 per byte moved |
| `$45` | EOR dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$46` | LSR dp | Direct Page | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$47` | EOR [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$48` | PHA | Push Accumulator | 1 | 3 | +1 if m=0 |
| `$49` | EOR #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$4A` | LSR | Accumulator | 1 | 2 | — |
| `$4B` | PHK | Push Program Bank Register | 1 | 3 | — |
| `$4C` | JMP addr | Absolute | 3 | 3 | — |
| `$4D` | EOR addr | Absolute | 3 | 4 | +1 if m=0 |
| `$4E` | LSR addr | Absolute | 3 | 6 | +1 if m=0 |
| `$4F` | EOR long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$50` | BVC near | Branch if Overflow Clear | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$51` | EOR (dp), Y | DP Indirect Indexed, Y | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0, +1 if index crosses page boundary |
| `$52` | EOR (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$53` | EOR (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$54` | MVN srcBank, destBank | Block Move | 3 | (none given) | 7 per byte moved |
| `$55` | EOR dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$56` | LSR dp, X | Direct Page Indexed, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$57` | EOR [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$58` | CLI | Clear Interrupt Disable Flag | 1 | 2 | — |
| `$59` | EOR addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$5A` | PHY | Push Index Register Y | 1 | 3 | +1 if x=0 |
| `$5B` | TCD | Transfer 16 bit A to D | 1 | 2 | — |
| `$5C` | JML long / JMP long | Absolute Long | 4 | 4 | — |
| `$5D` | EOR addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$5E` | LSR addr, X | Absolute Indexed, X | 3 | 7 | +1 if m=0 |
| `$5F` | EOR long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |
| `$60` | RTS | Stack (return) | 1 | 6 | — |
| `$61` | ADC (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$62` | PER label | Stack (PC Relative Long) | 3 | 6 | — |
| `$63` | ADC sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$64` | STZ dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$65` | ADC dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$66` | ROR dp | Direct Page | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$67` | ADC [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$68` | PLA | Pull Accumulator | 1 | 4 | +1 if m=0 |
| `$69` | ADC #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$6A` | ROR | Accumulator | 1 | 2 | — |
| `$6B` | RTL | Stack (return long) | 1 | 6 | — |
| `$6C` | JMP (addr) | Absolute Indirect | 3 | 5 | — |
| `$6D` | ADC addr | Absolute | 3 | 4 | +1 if m=0 |
| `$6E` | ROR addr | Absolute | 3 | 6 | +1 if m=0 |
| `$6F` | ADC long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$70` | BVS near | Branch if Overflow Set | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$71` | ADC (dp), Y | DP Indirect Indexed, Y | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0, +1 if index crosses page boundary |
| `$72` | ADC (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$73` | ADC (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$74` | STZ dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$75` | ADC dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$76` | ROR dp, X | Direct Page Indexed, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$77` | ADC [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$78` | SEI | Set Interrupt Disable Flag | 1 | 2 | — |
| `$79` | ADC addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$7A` | PLY | Pull Index Register Y | 1 | 4 | +1 if x=0 |
| `$7B` | TDC | Transfer D to 16 bit A | 1 | 2 | — |
| `$7C` | JMP (addr, X) | Absolute Indexed Indirect, X | 3 | 6 | — |
| `$7D` | ADC addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$7E` | ROR addr, X | Absolute Indexed, X | 3 | 7 | +1 if m=0 |
| `$7F` | ADC long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |
| `$80` | BRA near | Branch Always | 2 | 3 | +1 if e=1 |
| `$81` | STA (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$82` | BRL label | Branch Always Long | 3 | 4 | — |
| `$83` | STA sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$84` | STY dp | Direct Page | 2 | 3 | +1 if x=0, +1 if D.l ≠ 0 |
| `$85` | STA dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$86` | STX dp | Direct Page | 2 | 3 | +1 if x=0, +1 if D.l ≠ 0 |
| `$87` | STA [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$88` | DEY | Implied | 1 | 2 | — |
| `$89` | BIT #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$8A` | TXA | Transfer X to A | 1 | 2 | — |
| `$8B` | PHB | Push Data Bank | 1 | 3 | — |
| `$8C` | STY addr | Absolute | 3 | 4 | +1 if x=0 |
| `$8D` | STA addr | Absolute | 3 | 4 | +1 if m=0 |
| `$8E` | STX addr | Absolute | 3 | 4 | +1 if x=0 |
| `$8F` | STA long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$90` | BCC near | Branch if Carry Clear | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$91` | STA (dp), Y | DP Indirect Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$92` | STA (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$93` | STA (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$94` | STY dp, X | Direct Page Indexed, X | 2 | 4 | +1 if x=0, +1 if D.l ≠ 0 |
| `$95` | STA dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$96` | STX dp, Y | Direct Page Indexed, Y | 2 | 4 | +1 if x=0, +1 if D.l ≠ 0 |
| `$97` | STA [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$98` | TYA | Transfer Y to A | 1 | 2 | — |
| `$99` | STA addr, Y | Absolute Indexed, Y | 3 | 5 | +1 if m=0 |
| `$9A` | TXS | Transfer X to S | 1 | 2 | — |
| `$9B` | TXY | Transfer X to Y | 1 | 2 | — |
| `$9C` | STZ addr | Absolute | 3 | 4 | +1 if m=0 |
| `$9D` | STA addr, X | Absolute Indexed, X | 3 | 5 | +1 if m=0 |
| `$9E` | STZ addr, X | Absolute Indexed, X | 3 | 5 | +1 if m=0 |
| `$9F` | STA long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |
| `$A0` | LDY #const | Immediate | 2 / 3 | 2 | +1 if x=0 |
| `$A1` | LDA (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$A2` | LDX #const | Immediate | 2 / 3 | 2 | +1 if x=0 |
| `$A3` | LDA sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$A4` | LDY dp | Direct Page | 2 | 3 | +1 if x=0, +1 if D.l ≠ 0 |
| `$A5` | LDA dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$A6` | LDX dp | Direct Page | 2 | 3 | +1 if x=0, +1 if D.l ≠ 0 |
| `$A7` | LDA [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$A8` | TAY | Transfer A to Y | 1 | 2 | — |
| `$A9` | LDA #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$AA` | TAX | Transfer A to X | 1 | 2 | — |
| `$AB` | PLB | Pull Data Bank | 1 | 4 | — |
| `$AC` | LDY addr | Absolute | 3 | 4 | +1 if x=0 |
| `$AD` | LDA addr | Absolute | 3 | 4 | +1 if m=0 |
| `$AE` | LDX addr | Absolute | 3 | 4 | +1 if x=0 |
| `$AF` | LDA long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$B0` | BCS near | Branch if Carry Set | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$B1` | LDA (dp), Y | DP Indirect Indexed, Y | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0, +1 if index crosses page boundary |
| `$B2` | LDA (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$B3` | LDA (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$B4` | LDY dp, X | Direct Page Indexed, X | 2 | 4 | +1 if x=0, +1 if D.l ≠ 0 |
| `$B5` | LDA dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$B6` | LDX dp, Y | Direct Page Indexed, Y | 2 | 4 | +1 if x=0, +1 if D.l ≠ 0 |
| `$B7` | LDA [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$B8` | CLV | Clear Overflow Flag | 1 | 2 | — |
| `$B9` | LDA addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$BA` | TSX | Transfer S to X | 1 | 2 | — |
| `$BB` | TYX | Transfer Y to X | 1 | 2 | — |
| `$BC` | LDY addr, X | Absolute Indexed, X | 3 | 4 | +1 if x=0, +1 if index crosses page boundary |
| `$BD` | LDA addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$BE` | LDX addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if x=0, +1 if index crosses page boundary |
| `$BF` | LDA long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |
| `$C0` | CPY #const | Immediate | 2 / 3 | 2 | +1 if x=0 |
| `$C1` | CMP (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$C2` | REP #const | Immediate | 2 | 3 | — |
| `$C3` | CMP sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$C4` | CPY dp | Direct Page | 2 | 3 | +1 if x=0, +1 if D.l ≠ 0 |
| `$C5` | CMP dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$C6` | DEC dp | Direct Page | 2 | 5 | +2 if m=0, +1 if D.l ≠ 0 |
| `$C7` | CMP [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$C8` | INY | Implied | 1 | 2 | — |
| `$C9` | CMP #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$CA` | DEX | Implied | 1 | 2 | — |
| `$CB` | WAI | Implied | 1 | 3 | additional cycles needed by interrupt handler to restart the processor |
| `$CC` | CPY addr | Absolute | 3 | 4 | +1 if x=0 |
| `$CD` | CMP addr | Absolute | 3 | 4 | +1 if m=0 |
| `$CE` | DEC addr | Absolute | 3 | 6 | +2 if m=0 |
| `$CF` | CMP long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$D0` | BNE near | Branch if Not Equal | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$D1` | CMP (dp), Y | DP Indirect Indexed, Y | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0, +1 if index crosses page boundary |
| `$D2` | CMP (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$D3` | CMP (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$D4` | PEI (dp) | Stack (DP Indirect) | 2 | 6 | +1 if D.l ≠ 0 |
| `$D5` | CMP dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$D6` | DEC dp, X | Direct Page Indexed, X | 2 | 6 | +2 if m=0, +1 if D.l ≠ 0 |
| `$D7` | CMP [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$D8` | CLD | Clear Decimal Flag | 1 | 2 | — |
| `$D9` | CMP addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$DA` | PHX | Push Index Register X | 1 | 3 | +1 if x=0 |
| `$DB` | STP | Implied | 1 | 3 | — |
| `$DC` | JML [addr] / JMP [addr] | Absolute Indirect Long | 3 | 6 | — |
| `$DD` | CMP addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$DE` | DEC addr, X | Absolute Indexed, X | 3 | 7 | +2 if m=0 |
| `$DF` | CMP long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |
| `$E0` | CPX #const | Immediate | 2 / 3 | 2 | +1 if x=0 |
| `$E1` | SBC (dp, X) | Direct Page Indirect, X | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$E2` | SEP #const | Immediate | 2 | 3 | — |
| `$E3` | SBC sr, S | Stack Relative | 2 | 4 | +1 if m=0 |
| `$E4` | CPX dp | Direct Page | 2 | 3 | +1 if x=0, +1 if D.l ≠ 0 |
| `$E5` | SBC dp | Direct Page | 2 | 3 | +1 if m=0, +1 if D.l ≠ 0 |
| `$E6` | INC dp | Direct Page | 2 | 5 | +2 if m=0, +1 if D.l ≠ 0 |
| `$E7` | SBC [dp] | Direct Page Indirect Long | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$E8` | INX | Implied | 1 | 2 | — |
| `$E9` | SBC #const | Immediate | 2 / 3 | 2 | +1 if m=0 |
| `$EA` | NOP | Implied | 1 | 2 | — |
| `$EB` | XBA | Implied | 1 | 3 | — |
| `$EC` | CPX addr | Absolute | 3 | 4 | +1 if x=0 |
| `$ED` | SBC addr | Absolute | 3 | 4 | +1 if m=0 |
| `$EE` | INC addr | Absolute | 3 | 6 | +2 if m=0 |
| `$EF` | SBC long | Absolute Long | 4 | 5 | +1 if m=0 |
| `$F0` | BEQ near | Branch if Equal | 2 | 2 | +1 if branch taken, +1 if e=1 |
| `$F1` | SBC (dp), Y | DP Indirect Indexed, Y | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0, +1 if index crosses page boundary |
| `$F2` | SBC (dp) | Direct Page Indirect | 2 | 5 | +1 if m=0, +1 if D.l ≠ 0 |
| `$F3` | SBC (sr, S), Y | SR Indirect Indexed, Y | 2 | 7 | +1 if m=0 |
| `$F4` | PEA addr | Stack (Absolute) | 3 | 5 | — |
| `$F5` | SBC dp, X | Direct Page Indexed, X | 2 | 4 | +1 if m=0, +1 if D.l ≠ 0 |
| `$F6` | INC dp, X | Direct Page Indexed, X | 2 | 6 | +2 if m=0, +1 if D.l ≠ 0 |
| `$F7` | SBC [dp], Y | DP Indirect Long Indexed, Y | 2 | 6 | +1 if m=0, +1 if D.l ≠ 0 |
| `$F8` | SED | Set Decimal Flag | 1 | 2 | — |
| `$F9` | SBC addr, Y | Absolute Indexed, Y | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$FA` | PLX | Pull Index Register X | 1 | 4 | +1 if x=0 |
| `$FB` | XCE | Implied | 1 | 2 | — |
| `$FC` | JSR (addr, X) | Absolute Indexed Indirect, X | 3 | 8 | — |
| `$FD` | SBC addr, X | Absolute Indexed, X | 3 | 4 | +1 if m=0, +1 if index crosses page boundary |
| `$FE` | INC addr, X | Absolute Indexed, X | 3 | 7 | +2 if m=0 |
| `$FF` | SBC long, X | Absolute Long Indexed, X | 4 | 5 | +1 if m=0 |

## Extraction method

Fetched the canonical markdown source with `curl` and parsed every table
(`Syntax | Addressing Mode | Opcode | Bytes | Cycles | Extra`) programmatically;
250 opcodes came from markdown tables and the remaining 6 (`$42`, `$4C`, `$5C`,
`$6C`, `$7C`, `$DC`) from the two raw-HTML `rowspan` tables, transcribed by hand.
All 256 opcode bytes are present, with no duplicates. Cell values are byte-for-byte
as published apart from: merging the two-syntax `rowspan` rows into `A / B`, and
substituting the placeholder text noted in the caveats for empty cells.
