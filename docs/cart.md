# Cartridge — memory map + coprocessors — RustySNES

**References:** `ref-docs/2026-06-24-coprocessors.md` (the primary source for this doc);
`ref-docs/research-report.md` §§6–7; `docs/cartridge-format.md` (the header bytes);
`docs/adr/0003` (tiering honesty gate); `docs/adr/0004` (RTC determinism). Cited inline:
SNESdev Memory map / ROM header, SnesLab, Fullsnes, bsnes State-of-Emulation IV.

This doc is the SPEC, not history — update it in the same PR as the code. Pin behavior
against the test ROMs first.

## Purpose

The cart crate (`rustysnes-cart`) owns the ROM/SRAM memory model **and** the coprocessor
families — each coprocessor is a "mapper-equivalent" with its own bus window and clock
(`docs/architecture.md` §4). It exposes a `Cart` trait with default-no-op hooks so the CPU
and PPU never special-case a board.

## Memory-map models

Per `ref-docs/2026-06-24-coprocessors.md` §A (SNESdev Memory map):

| Model | $FFD5 | Layout | Header offset | Max |
|---|---|---|---|---|
| **LoROM** | $20 | 32 KiB windows in $8000–$FFFF of each bank; A15 skipped; A16–A21 → ROM A15–A20 | `$007FC0` | 4 MiB |
| **HiROM** | $21 | 64 KiB linear banks, full ROM at $C0–$FF; data crosses banks freely | `$00FFC0` | 4 MiB |
| **ExHiROM** | $25 | >4 MiB (≤~8 MiB): $80–$FF = first 4 MiB, $00–$7D = extra; *Tales of Phantasia*, *Star Ocean* | `$40FFC0` | ~8 MiB |
| **ExLoROM** | — | LoROM extension (mostly homebrew / flashcart) | — | — |

**SRAM mapping is board-dependent — no single canonical table.** LoROM SRAM typically banks
$70–$7D/$F0–$FF $0000–$7FFF; HiROM SRAM typically banks $20–$3F/$A0–$BF $6000–$7FFF. Battery
is flagged in `$FFD6` low nibble ($2). Build per-board windows from the cartridge database +
ares board definitions during Phase 4 (`ref-docs/research-report.md` "Open questions" #3).

### Phase-2 base-board decode (implemented in `board.rs`)

Base LoROM/HiROM/ExHiROM now decode against real `rom: Box<[u8]>` + zeroed `sram: Box<[u8]>`
storage. The `(bank, addr)` → backing-store math (`bank = addr24 >> 16`, `addr = addr24 & 0xFFFF`):

| Model | ROM region(s) | ROM offset formula | SRAM window | SRAM index |
|---|---|---|---|---|
| **LoROM** | every bank, `$8000–$FFFF` | `((bank & 0x7F) << 15) \| (addr & 0x7FFF)` | banks $70–$7D / $F0–$FF, `$0000–$7FFF` | `(lo-0x70)*0x8000 + addr`, `% sram_size` |
| **HiROM** | $40–$7D / $C0–$FF full 64 KiB; $00–$3F / $80–$BF `$8000–$FFFF` | `((bank & 0x3F) << 16) \| addr` | banks $20–$3F / $A0–$BF, `$6000–$7FFF` | `(lo-0x20)*0x2000 + (addr-0x6000)`, `% sram_size` |
| **ExHiROM** | same regions as HiROM | `high \| ((bank & 0x3F) << 16) \| addr`, where `high = (bank & 0x80 != 0) ? 0 : (1<<22)` | banks $20–$3F (low half), `$6000–$7FFF` | as HiROM |

The ExHiROM `high` bit is A23-inverted: banks $80–$FF (A23=1) select the first 4 MiB; banks
$00–$7D (A23=0) select the extra 4 MiB. ROM offsets are folded to `rom_size` by the `mirror`
helper (clean-room port of ares `Bus::mirror`): power-of-two sizes mask, non-power-of-two
sizes split the largest power-of-two block linear + mirror the remainder. SRAM size is
`if $FFD8 == 0 { 0 } else { 0x400 << $FFD8 }`; ROM and open-bus regions are read-only.

## Coprocessor families

Per `ref-docs/2026-06-24-coprocessors.md` §§B–C. **Emulation-approach key:** the NEC DSP
family / ST01x / ST018 / CX4 use **LLE** (run the dumped chip program ROM — the user supplies
it); Super FX and SA-1 run their program from cart ROM (no chip dump).

| Chip | Core | Clock | ~Games | Shares core? | Emu | Tier |
|---|---|---|---|---|---|---|
| DSP-1/1A/1B | µPD77C25 | ~7.6–8 MHz | 15+ | µPD77C25 family | LLE (prog ROM) | **Core/Curated** |
| DSP-2/3/4 | µPD77C25 | ~8 MHz | 1 each | µPD77C25 | LLE | BestEffort (shared) |
| Super FX / GSU-1/2 | Argonaut RISC | 10.74 / 21.47 MHz | ~8 | no | cycle-accurate (cart ROM) | **Core/Curated** |
| SA-1 | 65C816 | 10.74 MHz | ~35 | (65C816) | cycle-accurate (cart ROM) | **Core/Curated** |
| S-DD1 | Nintendo ASIC | — | 2 | no | algorithm-exact | BestEffort |
| SPC7110 (+RTC-4513) | Hudson ASIC | — | 3 | no | algorithm + frozen RTC | BestEffort |
| CX4 | Hitachi HG51B169 | 20 MHz | 2 | no | LLE (prog ROM) | BestEffort/Curated |
| OBC1 | simple ASIC | — | 1 | no | HLE | BestEffort |
| ST010 / ST011 | µPD96050 | ~10 / 15 MHz | 1 each | µPD96050 (≈77C25) | LLE (shared) | BestEffort (shared) |
| ST018 | ARMv3 | ~21.44 MHz | 1 | no | LLE ARM core | BestEffort |
| S-RTC | Epson RTC | — | 1 | no | HLE + frozen time | BestEffort |

### Key leverage — the shared NEC core

One **µPD77C25 / µPD96050 LLE engine** covers **DSP-1/2/3/4 and ST010/011 — six chips, one
engine**. Implement it once in `rustysnes-cart` and drive each chip's program/data ROM through
it. This is the single biggest economy in the coprocessor breadth phase.

### Per-chip notes (the load-bearing ones)

- **DSP-1** (`Core/Curated`): NEC µPD77C25, Mode-7 3D math; 15+ games (Super Mario Kart,
  Pilotwings); memory-mapped DR/SR command ports.
- **Super FX / GSU** (`Core/Curated`): Argonaut RISC plotting into bitmap RAM; 10.74 MHz
  (Mario Chip 1) or 21.47 MHz; 32/64 KB cart RAM arbitrated with the SNES CPU (not
  simultaneous); Star Fox, Yoshi's Island (GSU-2), Doom.
- **SA-1** (`Core/Curated`): a second 65C816 @ 10.74 MHz — the most complex coprocessor.
  Registers $2200–$230E; I-RAM $3000–$37FF; shared BW-RAM (8-bit half-speed, 1-cycle stall per
  access); Character-Conversion DMA + arithmetic unit; ~35 games (Super Mario RPG, Kirby Super
  Star). Reuses the 65C816 core from `rustysnes-cpu`.
- **RTC chips** (S-RTC, SPC7110's RTC-4513): the **determinism hazard** — HLE backed by
  **frozen / seeded** host time, never live wall-clock (`docs/adr/0004`).

## Header detection

The internal header ($FFC0–$FFDF) and the score heuristic live in `docs/cartridge-format.md`.
The cart crate scores the candidate header at $7FC0 / $FFC0 / $40FFC0 and picks the highest;
the `$FFD6` high nibble selects the coprocessor family.

## Interfaces (sketch)

```rust
// rustysnes-cart
pub trait Cart {
    fn read(&mut self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, value: u8);
    /// Coprocessors that tick on the master clock advance here (default no-op).
    fn tick(&mut self, master_cycles: u32) {}
    fn sram(&self) -> &[u8];          // for battery save
    fn tier(&self) -> CoprocessorTier; // Core | Curated | BestEffort (honesty gate)
}
```

## Edge cases and gotchas

1. **DMA cannot cross a bank** — relevant to LoROM/HiROM bank wiring
   (`ref-docs/2026-06-24-ppu.md` §5).
2. **Chip-ROM-dump dependence** (DSP/ST01x/CX4/ST018) must be feature-gated with an honesty
   caveat — without the dump the board is non-functional, and it never backs the oracle
   (`docs/adr/0003`).
3. **Super FX / SA-1 RAM arbitration** is not simultaneous; model the access stalls.
4. **ExHiROM split addressing** ($80–$FF first 4 MiB, $00–$7D extra) is the only >4 MiB case.
5. **RTC freeze** — see `docs/adr/0004`.

## Test plan

- **Memory map / header:** gilyon + undisbeliever ROMs boot under each map model; auto-detect
  picks the right one for the canonical commercial set.
- **Coprocessors:** Krom/PeterLemon GSU ROMs (reference-only); commercial dumps booted locally
  with committed screenshots / `.snap` only (never the ROM — `tests/roms/external/` is
  gitignored). Tier each board and assert the honesty gate (`docs/adr/0003`).

## Open questions

- Per-board SRAM / coprocessor bus windows (no canonical table) — Phase 4 build-out.
- DSP nominal clock range (~7.6–8 MHz) — gated by test ROMs, not the number
  (`ref-docs/2026-06-24-coprocessors.md` "Flagged discrepancies").
