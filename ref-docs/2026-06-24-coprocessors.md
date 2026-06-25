# RustySNES — Cartridge Memory Map & Coprocessors Supplemental Research

**Generated:** 2026-06-24 · Immutable supplement to `research-report.md`. Do not edit; new findings → new dated file.

Full per-chip detail for the cart memory-map models and the coprocessor family. Every nontrivial
claim is cited.

---

## A. Memory map / cart models

### LoROM ($20)
32 KiB ROM windows in the **upper half** ($8000–$FFFF) of each bank. CPU A0–A14 wire directly;
**A15 is skipped** (cart port pin not connected); CPU A16–A21 → ROM A15–A20. Without SRAM, the
lower halves of banks $40–$7D / $C0–$FF mirror the upper half. Max 4 MiB. **Header at ROM offset
$007FC0.** ([SNESdev Memory map](https://snes.nesdev.org/wiki/Memory_map))

### HiROM ($21)
**64 KiB linear banks** (direct A0–A21) — full ROM visible at $C0–$FF ($0000–$FFFF/bank); data
crosses bank boundaries freely. The $8000–$FFFF half is also visible at $00–$3F/$80–$BF. Max 4 MiB.
**Header at $00FFC0.** ([SNESdev Memory map](https://snes.nesdev.org/wiki/Memory_map))

### ExHiROM ($25) / ExLoROM
ExHiROM extends past 4 MiB (up to ~8 MiB): $80–$FF = first 4 MiB, $00–$7D = extra ~4 MiB.
**Header at $40FFC0.** (Tales of Phantasia, Star Ocean.) ExLoROM is the analogous LoROM extension
(mostly homebrew/flashcart). ([SNESdev Memory map](https://snes.nesdev.org/wiki/Memory_map))

### Internal header ($FFC0–$FFDF)
21-byte title; **$FFD5** map-mode+speed (`001smmmm`: bit7 0=Slow/1=Fast, bits3–0 map mode $0 LoROM
/$1 HiROM /$5 ExHiROM); **$FFD6** chipset (low nibble feature: $0 ROM, $1 +RAM, $2 +RAM+battery;
high nibble coprocessor family: $0x DSP, $1x GSU, $2x OBC1, $3x SA-1, $Ex Other, $Fx Custom);
$FFD7 ROM size (1<<N KiB), $FFD8 RAM size, $FFD9 region, $FFDC/$FFDE checksum complement + checksum
(sum to $FFFF). ([SNESdev ROM header](https://snes.nesdev.org/wiki/ROM_header), [SnesLab](https://sneslab.net/wiki/SNES_ROM_Header))

### Auto-detection (header score heuristic)
Score the candidate header at $7FC0 / $FFC0 / $40FFC0 on: (a) complement+checksum = $FFFF, (b)
map-mode byte matching its location, (c) plausible size/region bytes, (d) **reset-vector
plausibility** (≥$8000; first opcode a likely boot instruction sei/clc/sec/stz/jmp/jml, not
brk/cop/stp/wdm). Highest score wins. ([SNESdev ROM header](https://snes.nesdev.org/wiki/ROM_header))

### SRAM mapping
Board-dependent — **no single canonical table.** LoROM SRAM typically banks $70–$7D/$F0–$FF
$0000–$7FFF; HiROM SRAM typically banks $20–$3F/$A0–$BF $6000–$7FFF. Battery flagged in $FFD6 low
nibble ($2). ([SNESdev Memory map](https://snes.nesdev.org/wiki/Memory_map))

## B. Coprocessors

> **Emulation-approach key:** higan/ares are fully cycle-accurate; bsnes partially. The NEC DSP
> family, ST01x, ST018 (ARM), and CX4 use **LLE** (run the dumped chip program ROM) — these need
> the user to supply chip ROMs. Super FX and SA-1 run their programs from cart ROM (no chip dump).
> ([bsnes.org — State of Emulation IV](https://bsnes.org/articles/state-of-emulation-4/))

### NEC DSP family — DSP-1/1A/1B, DSP-2, DSP-3, DSP-4
- **Chip:** pre-programmed **NEC µPD77C25** (CMOS µPD7725), ~7.6–8 MHz (Wikipedia ~7.6, emulator
  configs 8 — record the range). Memory-mapped DR/SR command ports + on-chip program/data ROM/RAM.
  ([Fullsnes](https://problemkaputt.de/fullsnes.htm), [SNESdev DSP-1](https://snes.nesdev.org/wiki/DSP-1))
- **Function/games:** Mode-7 math (3D projection/rotation). **DSP-1 = 15+ games** (Super Mario Kart,
  Pilotwings…); **DSP-2** = Dungeon Master; **DSP-3** = SD Gundam GX; **DSP-4** = Top Gear 3000.
  ([Wikipedia — SNES enhancement chips](https://en.wikipedia.org/wiki/List_of_Super_NES_enhancement_chips))
- **Tier:** DSP-1 = **Core/Curated**; DSP-2/3/4 = BestEffort (single-game, share the µPD77C25 LLE
  core — implement once).

### Super FX / GSU (Mario Chip 1, GSU-1, GSU-2)
16-bit Argonaut RISC; renders geometry by plotting into bitmap RAM. **10.74 MHz (mclk/2) or
21.47 MHz (mclk/1)** (Mario Chip 1 = 10.74 only). 32/64 KB cart RAM shared with the SNES CPU (not
simultaneous; arbitrated). ~8 games (Star Fox, Yoshi's Island/GSU-2, Doom, Stunt Race FX…).
Cycle-accurate, program in cart ROM. **Tier: Core/Curated.** ([Wikipedia — Super FX](https://en.wikipedia.org/wiki/Super_FX))

### SA-1
A second **65C816 @ 10.74 MHz** — the most complex coprocessor. Registers $2200–$230E; on-chip
I-RAM $3000–$37FF; shared BW-RAM (8-bit half-speed; SA-1 stalls 1 cycle/BW-RAM access). Built-in
DMA incl. **Character Conversion DMA** + arithmetic unit. **~35 games** (Super Mario RPG, Kirby
Super Star…). Cycle-accurate, program in cart ROM. **Tier: Core/Curated.** ([snescentral SA-1](https://snescentral.com/chips.php?chiptype=SA-1), [Wikipedia](https://en.wikipedia.org/wiki/List_of_Super_NES_enhancement_chips))

### S-DD1
Nintendo ASIC, **lossless decompression** (Ricoh ABS entropy decoder) during DMA. **2 games** (Star
Ocean, Street Fighter Alpha 2; ExHiROM). Algorithm fully emulated. **Tier: BestEffort.** ([snescentral S-DD1](https://snescentral.com/chips.php?chiptype=S-DD1))

### SPC7110
Hudson **decompression** chip; one config adds an **RTC-4513** real-time clock. **3 games** (Tengai
Makyou Zero +RTC, Momotarou Dentetsu Happy, Super Power League 4). Decompression emulated; RTC via
**frozen** host time (determinism caveat). **Tier: BestEffort.** ([snescentral SPC7110](https://snescentral.com/chips.php?chiptype=SPC7110))

### CX4 (Capcom)
**Hitachi HG51B169 DSP @ 20 MHz**, 3 KB on-chip RAM (shared, not simultaneous) + 3 KB data ROM;
trig for 3D wireframes + sprite scale/rotate + OAM management. **2 games** (Mega Man X2, X3). LLE in
ares/bsnes. **Tier: BestEffort/Curated.** ([Super Famicom Dev — CX4](https://wiki.superfamicom.org/capcom-cx4-hitachi-hg51b169))

### OBC1
Simple ASIC that builds sprite tables in RAM → DMA'd to OAM. **1 game** (Metal Combat). Trivial HLE.
**Tier: BestEffort.** ([SnesLab OBC-1](https://sneslab.net/wiki/OBC-1))

### ST010 / ST011 / ST018 (SETA)
- **ST010/ST011 = NEC µPD96050** (µPD77C25-compatible successor; battery-backed data RAM).
  Registers $600000–$67FFFF; battery RAM $680000–$6FFFFF. ST010 ≈10 MHz (F1 ROC II), ST011 ≈15 MHz
  (Hayazashi Nidan Morita Shougi). **Share the µPD96050 LLE core** with the DSP family.
- **ST018 = ARMv3 32-bit @ ~21.44 MHz** (Hayazashi Nidan Morita Shougi 2) — the costliest (full ARM
  LLE core).
- **Tier: BestEffort** all three; ST010/011 ride the shared NEC core, ST018 needs a separate ARM
  core. ([Fullsnes](https://problemkaputt.de/fullsnes.htm), [SnesLab ST018](https://sneslab.net/wiki/Seta_ST018), [Wikipedia](https://en.wikipedia.org/wiki/List_of_Super_NES_enhancement_chips))

### S-RTC
Cartridge-bus real-time clock. **1 game** (Daikaijuu Monogatari II). Trivial HLE backed by **frozen**
host time (determinism caveat). **Tier: BestEffort.** (Distinct from the RTC-4513 used via SPC7110.)
([snescentral S-RTC](https://snescentral.com/chips.php?chiptype=S-RTC))

## C. Shared-core / tiering cheat-sheet

| Chip | Core | Clock | ~Games | Shares core? | Emu | Tier |
|---|---|---|---|---|---|---|
| DSP-1/1A/1B | µPD77C25 | ~7.6–8 MHz | 15+ | µPD77C25 family | LLE (prog ROM) | **Core/Curated** |
| DSP-2/3/4 | µPD77C25 | ~8 MHz | 1 each | µPD77C25 | LLE | BestEffort (shared) |
| Super FX GSU-1/2 | Argonaut RISC | 10.74/21.47 MHz | ~8 | no | cycle-accurate (cart ROM) | **Core/Curated** |
| SA-1 | 65C816 | 10.74 MHz | ~35 | (65C816) | cycle-accurate (cart ROM) | **Core/Curated** |
| S-DD1 | Nintendo ASIC | — | 2 | no | algorithm-exact | BestEffort |
| SPC7110 (+RTC-4513) | Hudson ASIC | — | 3 | no | algorithm + frozen RTC | BestEffort |
| CX4 | Hitachi HG51B169 | 20 MHz | 2 | no | LLE (prog ROM) | BestEffort/Curated |
| OBC1 | simple ASIC | — | 1 | no | HLE | BestEffort |
| ST010/ST011 | µPD96050 | ~10/15 MHz | 1 each | µPD96050 (≈77C25) | LLE (shared) | BestEffort (shared) |
| ST018 | ARMv3 | ~21.44 MHz | 1 | no | LLE ARM core | BestEffort |
| S-RTC | Epson RTC | — | 1 | no | HLE + frozen time | BestEffort |

**Key leverage:** one **µPD77C25/µPD96050 LLE core** covers DSP-1/2/3/4 **and** ST010/011 — six
chips, one engine. Super FX + SA-1 run from cart ROM (no chip dump). RTC chips are the determinism
hazard — seed/freeze them.

### Flagged discrepancies
- DSP clock cited as ~7.6 MHz (Wikipedia) vs 8 MHz (emulator configs) — record the range; the LLE
  core's correctness is gated by test ROMs, not the nominal clock.
- Per-chip game counts vary slightly by source (SA-1 "35", Super FX "~8") — approximate.
- The jsgroth.dev coprocessor series (best secondary synthesis) is bot-gated; its specifics here are
  search-corroborated against the wikis, not directly fetched.
