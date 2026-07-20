# WDC W65C816S datasheet — citation and usage terms

**This is a citation, not an extract.** The datasheet's tables are *not* reproduced here, and must
not be. See "Copyright" below.

## The document

| Field | Value |
|---|---|
| Title | W65C816S 8/16–bit Microprocessor |
| Publisher | The Western Design Center, Inc. |
| Author (PDF metadata) | William D. Mensch Jr. |
| Dated | 13 March 2024 (page footers and PDF `creationDate`; WDC prints no revision number) |
| URL | <https://www.westerndesigncenter.com/wdc/documentation/w65c816s.pdf> |
| Retrieved | 2026-07-20 |
| Size / MD5 | 1,532,025 bytes · `89419b51b51d6cb7aac6aa0a5b726536` |
| Form | Born-digital, 55 pages, full text layer (not a scan) |

## Copyright

Verbatim from the document:

> Copyright (C) 1981-2024 by The Western Design Center, Inc. All rights reserved, including the
> right of reproduction in whole or in part in any form.

A search of all 55 pages for `permission`, `reproduc`, `license`, `distribut` and `trademark` found
**no permissive carve-out**. The datasheet's "provided gratuitously and without liability" language
is a *liability* disclaimer, not a copyright grant.

### What this means here

| | |
|---|---|
| Vendor the PDF into this repo | **No** |
| Vendor a verbatim or near-verbatim transcription of its tables | **No** |
| Cite it, and derive values from it | **Yes** |

**Project policy**, not a legal opinion: this repository does not reproduce the datasheet's tables
in whole or in part. Where timing data is needed, it is written independently — measured, or
described in our own words and structure — and carries the citation above. If a question arises
about whether a particular derived artifact is distinguishable enough from WDC's expression, treat
that as a question for the project owner rather than something to resolve in a commit message.

(The reasoning behind the policy is the ordinary observation that a processor's cycle behaviour is
something one can also determine by measurement. How far that goes is jurisdiction-dependent and is
deliberately not something this file adjudicates.)

An extraction was produced while sourcing this and deliberately **kept outside the repository**.

## Why this document, and not the alternatives

AccuracySNES needs a per-opcode timing oracle that is independent of any emulator, because
`A5.08` measured the three reference emulators disagreeing with each other on instruction timing —
so none of them can adjudicate. undisbeliever's table
(`2026-07-20-undisbeliever-65816-timing.md`) turned out to cite *higan*'s `wdc65816` among its
sources, which is the same lineage as bsnes and ares; scoring against it would reproduce the
circularity at one remove. This datasheet is written by the chip's designer, and no emulator
appears anywhere in its chain.

## What it gives that nothing else does

**Table 5-7 "Instruction Operation"** is cycle-by-cycle bus activity per addressing mode, and it
carries the **VDA** and **VPA** columns — physical output pins, not an author's interpretation.
§7.5 states:

> VDA and VPA should be used to qualify all memory cycles. Note that when VDA and VPA are both
> low, invalid addresses may be generated.

So a cycle's classification is manufacturer-defined rather than inferred:

```text
VDA = 0 and VPA = 0  ->  internal (no bus access)
otherwise, RWB = 1   ->  read
           RWB = 0   ->  write
```

That is exactly the split AccuracySNES needs. A 65816 cycle is 6, 8 or 12 SNES master clocks
depending on the region touched, so a cycle *count* alone cannot be converted into measurable time:

```text
clocks = 8*mem + 6*internal,  cycles = mem + internal   =>   clocks = 6*cycles + 2*mem
```

Table 5-7's Address Bus column goes further still — it names *which* address each cycle touches, so
per-cycle speed classes can be assigned instead of assuming a uniform 8 clocks.

**Table 5-4** is the opcode matrix: mnemonic, addressing-mode symbol, cycles and bytes for all 256
opcodes. **Table 5-6** is the addressing-mode symbol table; **5-2/5-3** are the vector tables.

## Limits — read before treating this as sufficient

- **This is the W65C816S, not Ricoh's 5A22.** The SNES CPU adds the memory-speed map, DMA/HDMA
  stalls and the per-scanline DRAM refresh. None of that is in this document, and all of it affects
  observed timing. A second layer is required for the SNES overlay; anomie's `timing.txt` is the
  candidate (hardware-measurement-derived rather than emulator-derived — though emulators were
  built *from* it, so emulator agreement with it proves nothing).
- **A datasheet describes intended design, not silicon errata.** Where the two could differ —
  Note 4's `abs,x` page-cross rule is the obvious case — prefer measurement over the document.
- **Eyes & Lichty's *Programming the 65816* is WDC-endorsed**, so it corroborates this datasheet but
  is *not* independent of it.
