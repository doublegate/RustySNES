# Glossary — SNES

Domain terms for the Super Nintendo Entertainment System / Super Famicom, as used across
RustySNES docs. Derived from `ref-docs/research-report.md` and its supplements.

## Chips and packages

- **5A22** — Ricoh package wrapping the WDC 65C816 main CPU; adds DMA/HDMA, multiply/divide,
  IRQ timers, joypad auto-read. (`docs/cpu.md`)
- **65C816 / WDC 65C816** — the 16-bit main CPU core; emulation + native modes.
- **PPU1 / 5C77** — sprites / OAM + the BG/sprite rendering pipeline + Mode-7 multiply.
- **PPU2 / 5C78** — CGRAM / color math / output + the H/V counters and latch. (`docs/ppu.md`)
- **SPC700 / S-SMP** — the Sony 8-bit audio CPU, ~1.024 MHz, on its own resonator.
- **S-DSP** — the wavetable synthesizer: 8 voices, BRR, Gaussian interpolation, 32 kHz stereo.
- **ARAM** — the 64 KiB audio RAM shared by SPC700 and S-DSP. (`docs/apu.md`)

## Timing

- **Master clock** — 21.477270 MHz NTSC / 21.281370 MHz PAL; the single timebase the scheduler
  advances. (`docs/scheduler.md`)
- **Dot** — nominally 4 master clocks; 341 dots / normal scanline (RustySNES convention).
- **Short / long scanline** — 1360 / 1368 master clocks vs the normal 1364.
- **MEMSEL ($420D.0)** — the FastROM bit: 0 = SlowROM (8-cycle WS2 ROM), 1 = FastROM (6-cycle).
- **WS1 / WS2** — ROM access "windows": WS1 = $00–$3F $8000–$FFFF (always 8 cycles), WS2 =
  $80–$FF $8000–$FFFF (6 or 8 by MEMSEL).
- **FastROM / SlowROM** — 3.58 MHz vs 2.68 MHz effective CPU rate for WS2 ROM.

## CPU state

- **E flag** — emulation-mode flag; `XCE` exchanges it with carry. RESET sets E (6502 mode).
- **M / X flags** — accumulator/memory width and index width (8 vs 16 bit) in native mode.
- **D / DBR / PBR** — direct-page, data-bank, program-bank registers (24-bit addressing).

## Video

- **BG mode 0–7** — background layer configurations; Mode 7 is the affine rotate/scale mode.
- **Mode 7** — single 128×128-tile affine layer; matrix M7A–M7D (8.8 fixed-point).
- **OAM** — 544-byte object attribute memory for 128 sprites (low table 512 + high table 32).
- **Range Over / Time Over** — the 32-sprites-per-line / 34-tiles-per-line limits (STAT77).
- **CGRAM** — 256-color palette RAM, 15-bit BGR. **VRAM** — 64 KiB = 32 K words.
- **HDMA** — horizontal-blank DMA: per-line register writes for raster effects.
- **EXTBG** — Mode-7 high-bit-as-priority extended-BG mode (SETINI bit 6).

## Cart

- **LoROM / HiROM / ExHiROM / ExLoROM** — the cart memory-map models. (`docs/cart.md`)
- **Coprocessor** — an in-cart enhancement chip (DSP-1..4, Super FX/GSU, SA-1, S-DD1, SPC7110,
  CX4, OBC1, ST01x/ST018, S-RTC); a "mapper-equivalent."
- **µPD77C25 / µPD96050** — the NEC DSP cores; one LLE engine covers six chips.
- **LLE / HLE** — low-level (run the dumped chip ROM) vs high-level (model the algorithm)
  emulation.

## Process

- **Tier (Core / Curated / BestEffort)** — the accuracy honesty tiers; BestEffort never backs
  the oracle. (`docs/adr/0003`)
- **The async-resync** — the integer relative-time accumulator that keeps the SPC700 timeline
  coherent with the main CPU at the four ports. (`docs/apu.md`)
- **Determinism contract** — same seed + ROM + input ⇒ bit-identical AV. (`docs/adr/0004`)
- **The fractional-timebase refactor** — the future "v2.0"-style sub-cycle scheduler rewrite.
  (`docs/adr/0002`)
