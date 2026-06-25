# Compatibility — RustySNES

**References:** `docs/STATUS.md` (the authoritative board matrix);
`ref-docs/research-report.md` "Scope and goals"; `ref-docs/2026-06-24-coprocessors.md`.

This doc records *which hardware revisions, regions, and software families are in scope* and
known incompatibilities. The live per-board / per-suite matrix lives in `docs/STATUS.md` — this
file is the prose context for it.

## In scope (v0.1+ accuracy target)

Per `ref-docs/research-report.md` "Scope and goals":

- **Regions:** NTSC and PAL (and Dendy-equivalent timing) **as data, not a build fork** — the
  master-clock frequency + scanline-length variants are region constants
  (`docs/scheduler.md`).
- **CPU:** WDC 65C816 (5A22), emulation + native modes, the full DMA/HDMA controller,
  multiply / divide units, H/V-IRQ timers, NMI, joypad auto-read.
- **Video:** PPU1 + PPU2, BG modes 0–7 (incl. Mode 7 affine), the 128-sprite OAM model,
  CGRAM/VRAM, color math / windows, the dot-clock timeline.
- **Audio:** SPC700 + S-DSP + 64 KiB ARAM, 8 voices, BRR, the async clock domain.
- **Carts:** LoROM / HiROM / ExHiROM (+ ExLoROM), header auto-detection, SRAM / battery saves.
- **Coprocessors:** the families in `docs/cart.md`, tiered Core / Curated / BestEffort.

## Coprocessor tiers (the compatibility shape)

Per `ref-docs/2026-06-24-coprocessors.md` §C and `docs/adr/0003`:

- **Core / Curated (verified, back the oracle):** DSP-1, Super FX / GSU-1/2, SA-1.
- **BestEffort (functional, never inflate the accuracy number):** DSP-2/3/4, S-DD1, SPC7110,
  CX4, OBC1, ST010/011, ST018, S-RTC.
- **Chip-ROM-dump dependence:** the LLE chips (DSP family, ST01x, CX4, ST018) need the user to
  supply dumped chip program ROMs — feature-gated with a loud honesty caveat; without the dump
  the board is non-functional.

## Out of scope initially (documented, deferred / BestEffort)

Per `ref-docs/research-report.md` "Scope":

- Satellaview (BS-X) modem, Sufami Turbo, Super Game Boy pass-through emulation.
- SNES Mouse / Super Scope niche peripherals beyond a stub.
- Exotic flash-cart mappers.

## Known determinism caveats

- **SPC resonator drift (±0.5%)** is excluded from the deterministic core; a real game that
  relies on per-console drift will play deterministically but not bit-match a specific physical
  unit (`docs/apu.md` §determinism-caveat).
- **RTC chips** (S-RTC, SPC7110's RTC-4513) read frozen / seeded time, not host wall-clock
  (`docs/adr/0004`).

## Open questions

- The precise commercial-title compatibility list fills in during Phases 4 / 6 / 7 as boards
  land; track it in `docs/STATUS.md`, not here.
