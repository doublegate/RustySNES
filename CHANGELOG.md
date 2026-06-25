# Changelog

All notable changes to RustySNES are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Initial workspace scaffold (cycle-accurate emulator architecture, ported from RustyNES).
- Seeded `tests/roms/` with the permissive corpora — gilyon (MIT), undisbeliever (MIT/Zlib),
  and a deterministic SingleStepTests/spc700 (MIT) sample — plus the gitignored `external/`
  tier (65816, full spc700, 240p, Krom, blargg-spc). Curated the commercial-ROM coverage
  manifest (`tests/roms/commercial-corpus.json`) to popularity-weighted, genre-diverse beloved
  titles (metadata + SHA-256 only; no ROM bytes committed).
- **Phase 2 — cartridge base-mapper memory model: real SNES internal-header detection
  (copier-prefix strip + scored `$7FC0`/`$FFC0`/`$40FFC0` candidate selection on
  checksum+complement, map-mode, reset-vector, and printable-title heuristics) and working
  LoROM / HiROM / ExHiROM address decode backed by owned `rom`/`sram` storage.** `Cart::load`
  builds the board with the stripped ROM bytes + a zeroed header-sized SRAM; `read24`/`write24`
  route through the decode over real memory (ROM read-only, SRAM read/write, hardware-accurate
  non-power-of-two ROM mirroring). Added `save_sram`/`load_sram` battery accessors. Coprocessors
  remain stubs (Phase 4).
- **Phase 2 — dual-chip PPU (PPU1 5C77 + PPU2 5C78):** VRAM/CGRAM/OAM + the full `$2100-$213F`
  register file (with the BG-offset / Mode-7 / scroll write latches, VMAIN remap + increment,
  CGRAM/OAM/VRAM read-prefetch quirks, MPYL/M/H multiply, SLHV/OPHCT/OPVCT, STAT77/78); BG modes
  0-7 tile fetch (2/4/8 bpp, per-mode priority, 16×16 tiles, mosaic); Mode 7 affine
  (matrix/center/wrap/flip, EXTBG); the 128-sprite OAM pipeline (32-sprite range / 34-tile time
  limits, reverse-order fetch); color math (add/sub/half, fixed/sub addend, direct color); and
  windows (OR/AND/XOR/XNOR). Per-scanline compositor; mid-line raster + hi-res 512 deferred.
- **Phase 2 — master-clock lockstep scheduler + bus + DMA/HDMA:** the master clock advances
  through the CPU's bus accesses on the **6/8/12 region access-speed map** (ares `CPU::wait`,
  `$420D` MEMSEL FastROM), stepping the PPU dot clock (4 master/dot) + the SPC accumulator in
  lockstep. Full 24-bit memory decode (WRAM + low-mirror, PPU/APU B-bus, controllers, the
  CPU registers `$4200-$421F` incl. the multiply/divide unit, the DMA registers, cart routing).
  The 8-channel **GP-DMA** (CPU-halt, 8 transfer modes) and **HDMA** (per-line budget, indirect
  tables, mode lengths `{1,2,2,4,4,4,2,4}`) clean-room from ares `dma.cpp`. NMI + the RDNMI
  VBlank flag + the H/V-IRQ comparator. The `System` boots a cart from its reset vector and runs
  deterministic frames (an NTSC frame ≈357,374 master clocks).
- **Phase 2 — verified on-cart:** gilyon `cputest-basic.sfc` boots and reports "Success" (all
  1107 65C816 tests; `tests/gilyon_oncart.rs`) — closing Phase 1's deferred on-cart criterion;
  the 29 undisbeliever PPU/DMA/HDMA ROMs render bit-deterministic golden framebuffers matching
  `tests/golden/undisbeliever-framebuffer.tsv` (`tests/undisbeliever_golden.rs`).
- **Phase 1 — CPU + golden oracle: the WDC 65C816 core passes the SingleStepTests/65816
  per-opcode oracle to 0-diff** on architectural state **and** per-instruction cycle count
  (5,119,999 / 5,120,000 = 100.00% across all 512 opcode files × 10,000 tests, native +
  emulation). All 256 opcodes × addressing modes, `REP`/`SEP` width changes, and `XCE`
  emulation/native transitions verified.

### Fixed

- `MVN`/`MVP` (block move): address now uses the full 16-bit `X`/`Y` regardless of index width
  and the increment respects the `X` flag (8-bit keeps the high byte); the `A.w` loop test is a
  post-decrement (ares `instructionBlockMove`). The oracle harness re-steps these looping
  instructions to the recorded cycle budget.
- `JSL`/`RTL`: stack access uses the full-16-bit `S` "new" push/pull (`pushN`/`pullN`) so it no
  longer corrupts `S` on an emulation-mode page wrap; the page-1 confinement is re-applied at
  the instruction boundary (ares `CallLong`/`instructionReturnLong`).
