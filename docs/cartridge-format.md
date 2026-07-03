# Cartridge format — the internal header + auto-detection — RustySNES

**References:** `ref-docs/research-report.md` §6; `ref-docs/2026-06-24-coprocessors.md` §A;
`docs/cart.md` (the memory-map models that consume this). Cited inline: SNESdev ROM header,
SnesLab SNES ROM Header.

This doc is the SPEC, not history. The header parser is `rustysnes-cart`'s entry point: it
decides LoROM vs HiROM vs ExHiROM, the coprocessor family, and the ROM/RAM/region sizes.

## The internal header ($FFC0–$FFDF)

Located at a map-mode-dependent ROM offset (`docs/cart.md` §memory-map-models):
**$007FC0** (LoROM), **$00FFC0** (HiROM), **$40FFC0** (ExHiROM), **$407FC0** (ExLoROM). Per
`ref-docs/research-report.md` §6 and `ref-docs/2026-06-24-coprocessors.md` §A:

| Offset | Field | Meaning |
|---|---|---|
| $FFC0–$FFD4 | Title | 21-byte ASCII game title |
| **$FFD5** | Map-mode + speed | `001smmmm`: bit7 `0=Slow / 1=Fast`; low nibble map mode `$0`=LoROM, `$1`=HiROM, `$5`=ExHiROM; ExLoROM has no dedicated value and typically reports `$0` (LoROM) — disambiguated by header *offset* alone |
| **$FFD6** | Chipset | low nibble feature (`$0`=ROM, `$1`=+RAM, `$2`=+RAM+battery); high nibble coprocessor family (`$0x`=DSP, `$1x`=GSU/Super FX, `$2x`=OBC1, `$3x`=SA-1, `$Ex`=Other, `$Fx`=Custom) |
| $FFD7 | ROM size | `1 << N` KiB |
| $FFD8 | RAM size | `1 << N` KiB (SRAM) |
| $FFD9 | Region | NTSC / PAL / region code |
| $FFDC | Checksum complement | |
| $FFDE | Checksum | complement + checksum sum to **$FFFF** |

The native/emulation interrupt vectors immediately follow ($FFE0–$FFFF), including the reset
vector used by the detection heuristic.

## Auto-detection (the score heuristic)

Per `ref-docs/2026-06-24-coprocessors.md` §A "Auto-detection": score the candidate header at
**$7FC0 / $FFC0 / $40FFC0 / $407FC0**, take the highest, on:

1. **complement + checksum == $FFFF** (the strongest signal),
2. the **map-mode byte matches its location** (a $20 header found at $7FC0, etc.),
3. **plausible size / region bytes**,
4. **reset-vector plausibility:** ≥ $8000, and the first opcode a likely boot instruction
   (`sei` / `clc` / `sec` / `stz` / `jmp` / `jml`), **not** `brk` / `cop` / `stp` / `wdm`.

Cross-validate fields against SnesLab where SNESdev is ambiguous
(`ref-docs/research-report.md` §6).

## Notes

- A copier header (the spurious 512-byte SMC prefix) must be detected and stripped before
  scoring (`romlen % 1024 == 512`).
- The chipset high nibble selects which coprocessor `rustysnes-cart` instantiates
  (`docs/cart.md` §coprocessor-families); RTC presence implies the determinism freeze
  (`docs/adr/0004`).

## Test plan

- Round-trip the header parser over the gilyon / undisbeliever corpora and the canonical
  commercial set; the detected map mode + coprocessor must match the known-good database.
- A unit test per map mode with a hand-built minimal header.

## Open questions

- A few homebrew / flashcart ExLoROM headers do not follow the heuristic cleanly — fall back
  to a size + reset-vector vote (Phase 4).
- ExLoROM detection/decode is implemented (`docs/cart.md` §ExLoROM) but has **no real-ROM
  validation** — no commercial or homebrew ExLoROM dump exists in this project's local corpus.
  If one surfaces, add it to the header-detection round-trip corpus above.
