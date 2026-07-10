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
| **ExLoROM** | — (unofficial, no dedicated value) | LoROM extension for >4 MiB titles (mostly homebrew / flashcart) | `$407FC0` | ~8 MiB |

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
| **ExLoROM** | every bank, `$8000–$FFFF` | `high \| ((bank & 0x7F) << 15) \| (addr & 0x7FFF)`, where `high = (bank & 0x80 != 0) ? 0 : (1<<22)` | banks $70–$7D / $F0–$FF, `$0000–$7FFF` (as LoROM) | as LoROM |

The ExHiROM/ExLoROM `high` bit is A23-inverted: banks $80–$FF (A23=1) select the first 4 MiB;
banks $00–$7D (A23=0) select the extra 4 MiB. ROM offsets are folded to `rom_size` by the
`mirror` helper (clean-room port of ares `Bus::mirror`): power-of-two sizes mask, non-power-of-
two sizes split the largest power-of-two block linear + mirror the remainder. SRAM size is
`if $FFD8 == 0 { 0 } else { 0x400 << $FFD8 }`; ROM and open-bus regions are read-only.

**ExLoROM provenance.** Unlike LoROM/HiROM/ExHiROM, ExLoROM has no dedicated `$FFD5` mode
value — ares/bsnes both document it as unofficial (`ref-proj/ares/mia/medium/super-famicom.cpp`:
"ExLoROM mode is unofficial, and lacks a mapping mode value"; real carts often report plain
LoROM's `$20` there). The decode formula above is not a guess from the header-detection
heuristic — it's sourced directly from bsnes's own *runtime* board database
(`ref-proj/bsnes/bsnes/target-bsnes/resource/system/boards.bml`, `board: EXLOROM` /
`EXLOROM-RAM`: `map address=00-7d:8000-ffff mask=0x808000 base=0x400000` / `map
address=80-ff:8000-ffff mask=0x808000 base=0x000000`), decoded against bsnes's `Bus::reduce`
bit-packing algorithm (`sfc/memory/memory.cpp`) — which, for that mask, is exactly the LoROM
packed offset `((bank & 0x7F) << 15) | (addr & 0x7FFF)` with the same A23-inverted 4 MiB
half-select ExHiROM already uses. **No real ExLoROM ROM (commercial or homebrew) exists in this
project's local corpus**, so this board has no golden-framebuffer validation — only the
formula-level unit tests in `board.rs` (`docs/adr/0003`'s honesty gate: this is flagged, not
silently presented as hardware-proven).

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
| ST018 | ARMv3 | ~21.44 MHz | 1 | no | LLE ARM core | BestEffort (implemented, `coproc::armv3`) |
| S-RTC | Sharp S-RTC | — | 1 | no | HLE + frozen time | BestEffort |

### Key leverage — the shared NEC core

One **µPD77C25 / µPD96050 LLE engine** covers **DSP-1/2/3/4 and ST010/011 — six chips, one
engine**. Implement it once in `rustysnes-cart` and drive each chip's program/data ROM through
it. This is the single biggest economy in the coprocessor breadth phase.

### The µPD77C25 / µPD96050 engine (implemented — `crate::coproc::upd77c25`)

A clean-room port of ares' `uPD96050` component (ISC), parameterized by `Revision` (`Upd7725` =
DSP-1..4, 2 K×24 program + 1 K×16 data ROM; `Upd96050` = ST010/011, 16 K×24 + 2 K×16). The full
NEC DSP instruction set is decoded from the 24-bit word (OP / RT / JP / LD): the K×L signed
multiplier pipeline, the dual accumulators + 6-flag condition sets, the 16-deep call stack, the
program/data ROM + data RAM, and the DR / SR / DP host ports. Registers wrap at the revision's
PC/RP/DP widths.

**Host synchronization (the only cross-clock coupling):** the chip free-runs on its ~7.6 MHz
oscillator and hand-shakes the CPU solely through the **RQM** ("request for master") status bit —
DSP-1 games always poll `SR.rqm`, never a wall-clock cycle count. The engine therefore advances to
its next parked (RQM-set) state after every host DR access (`run_until_rqm`, capped). This keeps
the bus boundary byte-exact and fully deterministic (`docs/adr/0004`) without a free-running
per-master-clock tick, and needs no core-scheduler hook.

### The GSU core + the Super FX board (implemented — `crate::coproc::gsu` + `crate::coproc::superfx`)

Phase 4's second Core/Curated coprocessor. Unlike the NEC DSP family there is **no chip-ROM
dump** — the GSU program lives in the cartridge ROM the user already owns — so the board is
functional the moment a Super FX cart loads (`docs/adr/0003`: never silently degraded, and here
nothing to degrade).

**The GSU core (`coproc::gsu::Gsu`)** is a clean-room port of ares' `GSU` + `SuperFX` components
(ISC). It implements the full Argonaut RISC: R0–R15 (R15 = PC) with the FROM/TO/WITH source/dest
register-select prefixes; the ALT1/ALT2/ALT3 composite-mode machine that re-skins each opcode
(e.g. `add`→`adc`→`add #N`→`adc #N`); the ALU + signed/unsigned `mult`/`umult` + the
`fmult`/`lmult` 16×16 multiplier; the ROM buffer (ROMBR:R14 with `R`-flag busy + latency) and the
RAM buffer (RAMBR/RAMADDR with the deferred-write latency); the 256-byte / 32-line opcode cache
(`CACHE`, `cbr`, the `$3100–$32FF` cache window); the **1-instruction pipeline** that gives the
GSU its branch delay slot (`peekpipe`/`pipe`); the PLOT/RPIX pixel-plot pipeline with the two-deep
pixel cache, the `color`/`cmode` colour logic (dither / freeze-high / high-nibble / transparent),
and the SCBR/SCMR screen-base + 2/4/8 bpp character-format addressing; and the SFR status flags
(Z/CY/S/OV, **Go**, R, ALT1/2, B, IRQ).

**Host-sync (the only cross-clock coupling).** The GSU is started by the CPU writing R15's high
byte at `$301F`, which sets **Go** and begins execution at `(PBR:R15)`; the chip free-runs until
`STOP` clears Go (and, unless CFGR masks it, raises the cart IRQ), and software polls SFR for Go.
Because Go is the only observable coupling — exactly the RQM role the DSP-1 uses — the board runs
the GSU to completion the instant Go is set (`Gsu::run_until_stopped`, capped against a runaway
program). This is byte-exact and fully deterministic (`docs/adr/0004`) and needs **no
free-running core-scheduler tick** — the same economy as the DSP-1 `run_until_rqm`.

**The Super FX board (`coproc::superfx::SuperFxBoard`)** owns the ROM (shared, read-only) and the
Game Pak RAM (the GSU plot bitmap, sized from the header clamped to a 64 KiB minimum, power-of-two
masked), intercepts the GSU register window, and decodes the LoROM Super FX CPU map:

| Region (banks : addr)            | Target                              |
|----------------------------------|-------------------------------------|
| `$00–$3F,$80–$BF : $3000–$32FF`  | GSU registers + opcode-cache window |
| `$00–$3F,$80–$BF : $8000–$FFFF`  | Game Pak ROM (LoROM windows)        |
| `$40–$5F,$C0–$DF : $0000–$FFFF`  | Game Pak ROM (linear)               |
| `$70–$71,$F0–$F1 : $0000–$FFFF`  | Game Pak RAM (the plot bitmap)      |
| `$00–$3F,$80–$BF : $6000–$7FFF`  | Game Pak RAM low window (8 KiB)     |

**Bus arbitration (not simultaneous — edge case #3).** While Go is set the GSU owns whichever of
ROM/RAM its SCMR `RON`/`RAN` bits grant; a CPU ROM read then returns the hardware "snooze vector"
(ares `CPUROM::read`) and a CPU RAM read returns open bus. Run-to-completion-on-Go serialises this
naturally; the checks are kept for fidelity. The GSU and the CPU share the **same** ROM/RAM bytes,
so a GSU plot into `$70:xxxx` is exactly what the CPU then DMAs to VRAM.

`Coprocessor::SuperFx` routes through `board::select` to this board (the base board is never
built — Super FX re-decodes the map itself). Tier stays **Curated** and is in the honesty
oracle set. Validated by the `superfx_oncart` harness gate (58 Krom GSU ROMs: detection +
GSU-executed liveness + a `FillPoly`-into-RAM plot-pipeline assertion + deterministic golden;
see the test plan) plus engine unit tests (a hand-assembled `ibt`/`stop` program through the full
host-sync path, plus the per-instruction Krom `GSUTest` suite booted on the System).

### The SA-1 system + the second CPU (implemented — `crate::coproc::sa1` + `rustysnes-core`)

Phase 4's third Core/Curated coprocessor and the most complex: the **SA-1** is a second WDC
65C816 @ ~10.74 MHz (master clock / 2) plus a support ASIC. Like Super FX it carries **no chip-ROM
dump** — the SA-1 program lives in the cartridge ROM — so the board is functional the moment an
SA-1 cart loads (`docs/adr/0003`).

**Why it spans two crates.** The one-directional crate graph forbids `rustysnes-cart` from
depending on `rustysnes-cpu`. So `coproc::sa1::Sa1Board` owns the entire SA-1 **system** state and
exposes the SA-1 CPU's *memory view + control lines* through the `Board` second-CPU hooks
(`has_second_cpu` / `second_cpu_read` / `second_cpu_write` / `second_cpu_running` /
`second_cpu_take_reset` / `second_cpu_poll_nmi` / `second_cpu_poll_irq` / `second_cpu_tick`);
`rustysnes-core` owns the second `rustysnes_cpu::Cpu` and steps it (see `docs/scheduler.md` §SA-1).
The board is a clean-room port of ares' `sfc/coprocessor/sa1` (ISC).

**The SA-1 system (`coproc::sa1`)** implements: the `$2200–$23FF` register file (SA-1 control/reset,
the bidirectional S-CPU↔SA-1 IRQ/NMI/message lines, the S-CPU NMI/IRQ vector redirect SNV/SIV); the
**Super-MMC** ROM banking (CXB/DXB/EXB/FXB — four selectable 1 MiB blocks projected into the LoROM
`$8000–$FFFF` windows and the HiROM `$C0–$FF` banks); **BW-RAM** (shared battery RAM, the `$2224`
8 KiB-block S-CPU window + the `$40–$4F` linear image + the SA-1 `$60–$6F` 2/4 bpp **bitmap** and
`$40–$5F` linear projections, with the SWEN/CWEN/BWPA write-protect); **I-RAM** (2 KiB internal,
SIWP/CIWP per-256-byte write-protect); the **arithmetic unit** (`$2250–$2254`: signed multiply /
unsigned divide / cumulative-sum sigma with the 40-bit accumulator + overflow); the
**variable-length bit** processor (`$2258–$225B`, `$230C/$230D`); the **H/V timer** (the linear /
HV counter that raises the SA-1 timer IRQ); and the **DMA** unit (normal ROM/BW-RAM/I-RAM transfer
plus the type-1 and type-2 **character-conversion** DMA that transcodes linear BW-RAM ↔ planar
I-RAM).

S-CPU (main) memory map handled by `Board::read24`/`write24`:

| Region (banks : addr)            | Target                                |
|----------------------------------|---------------------------------------|
| `$00–$3F,$80–$BF : $2200–$23FF`  | SA-1 registers (S-CPU side)           |
| `$00–$3F,$80–$BF : $3000–$37FF`  | I-RAM (2 KiB)                         |
| `$00–$3F,$80–$BF : $6000–$7FFF`  | BW-RAM (8 KiB block, `$2224` BMAPS)  |
| `$00–$3F,$80–$BF : $8000–$FFFF`  | ROM (Super-MMC blocks C/D)           |
| `$40–$4F : $0000–$FFFF`          | BW-RAM (linear)                      |
| `$C0–$FF : $0000–$FFFF`          | ROM (Super-MMC blocks)               |

**The reset/interrupt handshake.** The SA-1 powers up held in reset (RESB asserted). The S-CPU
programs the SA-1 reset vector (CRV) + Super-MMC banks, then clears RESB (`$2200`) — the board
latches a reset edge that `rustysnes-core` consumes to reset the second CPU (its reset/NMI/IRQ
vector fetches are redirected to CRV/CNV/CIV inside `second_cpu_read`, since the SA-1 uses its own
vectors, not the ROM `$FFEx` vectors). The SA-1→S-CPU IRQ is the board's `irq_pending()`, ORed into
the main bus IRQ line; the S-CPU→SA-1 IRQ/NMI drive the second CPU's `poll_irq`/`poll_nmi`.

`Coprocessor::Sa1` routes through `board::select` to this board (the base board is never built —
SA-1 owns its own Super-MMC decode). Tier stays **Curated** and is in the honesty oracle set.
Validated by the `sa1_oncart` harness gate (18 staged commercial SA-1 carts: per-ROM SA-1 detection
+ S-CPU↔SA-1 register traffic, an aggregate "the SA-1 CPU executed millions of cycles" liveness
floor — Super Mario RPG, both Kirby titles, PGA Tour 96, Power Rangers Zeo, … — and a deterministic
golden framebuffer) plus board unit tests (decode regions, reset+vector handshake, arithmetic unit,
I-RAM/BW-RAM round-trips, Super-MMC ROM windows).

### DSP-1 board mapping + the firmware requirement (`crate::coproc::dsp1`)

`Dsp1Board` wraps a base LoROM/HiROM board (ROM + SRAM decode delegated) and intercepts only the
DR/SR window. There is **no canonical per-game window table**; the board picks the de-facto window
from map mode + ROM size — the heuristic snes9x/bsnes use absent a cartridge DB, which coincides
with every ares DSP-1 board definition:

| Map mode / size     | DSP window (banks : addr)        | DR / SR split |
|---------------------|----------------------------------|---------------|
| HiROM               | `$00–$1F,$80–$9F : $6000–$7FFF`   | DR `$6xxx`, SR `$7xxx` |
| LoROM, ROM ≤ 1 MiB  | `$30–$3F,$B0–$BF : $8000–$FFFF`   | DR `$8000–$BFFF`, SR `$C000–$FFFF` |
| LoROM, ROM > 1 MiB  | `$60–$6F,$E0–$EF : $0000–$7FFF`   | DR `$0000–$3FFF`, SR `$4000–$7FFF` |

**Firmware is user-supplied, never committed** (`docs/adr/0003`, edge case #2). The µPD77C25 runs a
fixed 8 KiB chip-ROM dump (`dsp1.rom` or the revised `dsp1b.rom`): the program ROM (2048 LE 24-bit
words) followed by the data ROM (1024 LE 16-bit words). Place it at the **gitignored**
`tests/roms/external/firmware/dsp1*.rom` and install it via
`Cart::install_coprocessor_firmware(&bytes)`. **Absent the dump the board is inert** — SR/DR read
as open bus, the game wedges on its first DSP poll — it is never silently degraded.

### Per-chip notes (the load-bearing ones)

- **DSP-1** (`Core/Curated`): NEC µPD77C25, Mode-7 3D math; 15+ games (Super Mario Kart,
  Pilotwings); memory-mapped DR/SR command ports.
- **Super FX / GSU** (`Core/Curated`, **implemented** — `coproc::gsu` + `coproc::superfx`):
  Argonaut RISC plotting into bitmap RAM; 10.74 MHz (Mario Chip 1) or 21.47 MHz (CLSR); 32/64/128
  KB cart RAM arbitrated with the SNES CPU (not simultaneous, the snooze-vector/open-bus model);
  runs its program from cart ROM (no chip dump); host-synced on the Go flag; Star Fox, Yoshi's
  Island (GSU-2), Doom. See "The GSU core + the Super FX board" above.
- **SA-1** (`Core/Curated`): a second 65C816 @ 10.74 MHz — the most complex coprocessor.
  Registers $2200–$230E; I-RAM $3000–$37FF; shared BW-RAM (8-bit half-speed, 1-cycle stall per
  access); Character-Conversion DMA + arithmetic unit; ~35 games (Super Mario RPG, Kirby Super
  Star). Reuses the 65C816 core from `rustysnes-cpu`.
- **RTC chips** (S-RTC, SPC7110's RTC-4513): the **determinism hazard** — HLE backed by
  **frozen / seeded** host time, never live wall-clock (`docs/adr/0004`). The RTC-4513
  (`coproc::epsonrtc::EpsonRtc`) is implemented as a 3-register (`$4840` chip-select/`$4841`
  data/`$4842` ready) handshake over a 16-nibble register file, seeded to an all-zero epoch and
  never advanced except by explicit register writes.
- **DSP-2 / DSP-4** (`BestEffort`, **implemented** — `coproc::necdsp_variant`): the same
  µPD77C25 LLE engine as DSP-1, title-detected and wired via `NecDspVariantBoard`. DSP-2 uses the
  generic bit-0 DR/SR split; DSP-4 needed a DSP-1-style half-window-boundary split instead (found
  by tracing a real Top Gear 3000 boot-time hardware check that expects both bytes of a 16-bit
  compare to come from the same port). Validated against real Dungeon Master / Top Gear 3000.
- **ST010 / ST011** (`BestEffort`, **implemented** — `coproc::necdsp_variant`): the µPD96050 LLE
  engine (also `coproc::upd77c25`), bit-0 DR/SR split + the DP battery data-RAM window. Validated
  against real F1 ROC II.
- **S-DD1** (`BestEffort`, **implemented** — `coproc::sdd1`): a Golomb-code + adaptive-binary-
  probability decompressor that streams during a fixed-address DMA transfer (a new
  `Board::notify_dma_channel` hook lets the cart snoop `$43n2-$43n6` DMA-register writes, since
  `rustysnes-core::Dma` owns those registers directly). No chip dump — decompresses the cart's own
  ROM. Validated against real Star Ocean / Street Fighter Alpha 2.
- **CX4** (`BestEffort`/`Curated`, **implemented** — `coproc::hg51b` + `coproc::cx4`): a
  clean-room Hitachi HG51B S169 core (sequential mask/value opcode decode transcribed from ares'
  `pattern(...)` strings). No chip dump for the program (runs from cart ROM); only a 3 KiB data-ROM
  constant table (`cx4.rom`) needs external supply. Validated against real Mega Man X2 / X3.
- **OBC1** (`BestEffort`, **implemented** — `coproc::obc1`): dedicated 8 KiB RAM behind a
  reprogrammable cursor register. Validated against real Metal Combat: Falcon's Revenge.
- **SPC7110** (`BestEffort`, **implemented, boot-crash root cause partially fixed, still not
  booting to real content** — `coproc::spc7110`): a decompression unit (Hudson adaptive binary
  range coder over 1/2/4bpp planes), data-port unit, ALU, and memory-control unit (four
  independently-bankable 1 MiB data-ROM windows). Paired with the RTC-4513 above on its one
  commercial title, Far East of Eden Zero. Cartridge geometry note: unlike every other
  coprocessor here, SPC7110 carts physically carry a separate small PROM (program) chip plus a
  much larger DROM (data) chip, concatenated in a raw dump; `coproc::spc7110::select` guesses the
  split (1 MiB PROM) from Far East of Eden Zero's documented physical geometry — there is no
  header field or generic formula that recovers this split for an arbitrary SPC7110 title.
  **Confirmed and fixed (`v0.4.0`):** `datarom_read`/`mcurom_read`'s PROM/DROM lookups used a
  plain `offset % len` fold; real hardware (ares `Bus::mirror`, `sfc/memory/inline.hpp`) instead
  repeatedly strips the largest power-of-two block that keeps the address in range — the two
  agree only when the buffer size is itself a power of two, which Far East of Eden Zero's 6 MiB
  DROM (`7 MiB image − 1 MiB PROM`) is NOT. A register-selected read past the physical chip size
  but inside the addressable window (`r4830`-`r4833` select up to 8 MiB) silently returned the
  WRONG byte, corrupting whatever data-ROM-resident table the game read through it. Ported the
  real `Bus::mirror` algorithm (`spc7110::bus_mirror`) and applied it to every PROM/DROM lookup;
  the wild-PC excursion this caused moved from ~20-30 frames into boot (BRK-storming into
  unmapped low banks, per the original diagnostic) to ~90+ frames, and it now self-recovers via a
  BRK/RTI oscillation instead of a permanent crash. **`v0.8.0`:** ported ares' SPC7110 cothread
  timing exactly — the DCU-begin-transfer (`$4806`)/multiply (`$4825`)/divide (`$4827`) triggers
  are deferred one master-clock tick (`dcu_pending`/`mul_pending`/`div_pending`, consumed in a new
  `coprocessor_tick` override), not completed synchronously within the register write — a real,
  independently-verified accuracy fix (9/9 unit tests), but watchpoint-based tracing
  (`T-81-001b`) confirmed it does **not** fix the boot gap: those triggers are never written at
  all during this boot's crash path. **Still open, substantially narrowed:** the same watchpoint
  trace shows `$7E0800-08FF` (containing the crashing `RTI`'s `$0848` target) is written exactly
  once at reset and never again across 60 real seconds of boot; no SPC7110 register is ever
  touched again either; and holding Start the whole time changes nothing. A new
  `rustysnes_cpu::disasm` disassembler + branch trace then found the `$00:F416`/`$20:20xx`
  framing was itself incomplete: the CPU actually spends most of its time in a real, coherent
  VRAM-upload loop in bank `$4F` (`STA $2118`/`$2116`), not stalled — until it hits a literal
  `JSL $4FFB80` (confirmed present in the raw dump, not a read artifact). Bank `$4F` is in
  `$40-$7D`, which two more real bugs (found by cross-checking `ref-proj/ares`'s own board
  database, `board: SHVC-LDH3C-01`, the exact board this title uses) turned out to mishandle:
  **(1)** `$40-$7D` should be unmapped (`MappedAddr::Open`), not a `$C0-FF` mirror — an earlier
  session's claim otherwise was never checked against this database; fixed in `read24`/`map`.
  **(2)** the DROM buffer was 2 MiB oversized — the committed dump is 7 MiB but the real physical
  chips total 5 MiB (1 MiB PROM + 4 MiB DROM per the same database), so `select` was treating 2
  MiB of trailing dump padding as real DROM and feeding `bus_mirror` the wrong fold length; fixed
  by slicing exactly `PROM_SIZE + DROM_SIZE`. Both fixes are independently verified. A fourth,
  systemic bug found alongside these: the cart layer's open-bus fallback returned a hardcoded `0`
  for `MappedAddr::Open` instead of echoing the Bus's real open-bus latch (ares' `Bus::read(address,
  data)` pattern) — fixed via `Cart::read24` now taking the caller's open-bus byte as a parameter
  (`rustysnes-cart/src/lib.rs`), benefiting every board, not just SPC7110. With this fix, the `JSL
  $4FFB80` dead end now lands on a stable, harmless open-bus spin loop (`AND $3D3D,X` echoing the
  last-latched byte forever) instead of a deterministic `BRK` — a more honestly-modeled failure,
  but still not a fix: **the real question is now precisely located, not closed:** a shipped
  commercial title cannot legitimately execute a jump into unmapped space on real hardware, so this
  emulation must diverge from real hardware *earlier* than this `JSL` — tracked as future SPC7110
  work, not silently claimed fixed (`docs/adr/0003`; full trail in
  `docs/audit/spc7110-boot-crash-2026-07-08.md`).
- **S-RTC** (`BestEffort`, **implemented** — `coproc::sharprtc`): a standalone Sharp S-RTC
  real-time clock (Daikaijuu Monogatari II, an ExHiROM title; ares board
  `EXHIROM-RAM-SHARPRTC`). A DIFFERENT chip/protocol from SPC7110's paired Epson RTC-4513 despite
  the similar name: a 2-register (`$2800` data, `$2801` unused) handshake that walks a 13-slot
  decimal clock file (second/minute/hour/day/month/year + an auto-computed weekday) through a
  `Ready -> Command -> Read`/`Write` state machine driven by magic values written to `$2800`
  (`$0D`=enter read, `$0E`=enter command, then `$00`=write / `$04`=reset-to-epoch as the command
  byte). Wraps a base `ExHiRom` board (`SharpRtcBoard::new`); ROM/SRAM delegate to it unchanged.
  Like the Epson RTC-4513, this port seeds a fixed epoch and never advances the clock other than
  via explicit register writes (`docs/adr/0004`'s determinism contract). No commercial Daikaijuu
  Monogatari II dump exists in this project's local corpus, so this board has unit-test-level
  coverage only, not golden-framebuffer validation (`docs/adr/0003`); header detection is a
  best-effort title match (`"DAIKAIJUU MONOGATARI"` / `"DAIKAIJU MONOGATARI"`), the same posture
  already carried openly for CX4/SPC7110's own `$F`-nibble disambiguation.
- **ST018** (`BestEffort`, **implemented** — `coproc::armv3`): a full ARMv3 (ARM6-class,
  pre-Thumb) CPU core, comparable in scope to `rustysnes-cpu`'s 65C816, not a small register-file
  port like this project's other BestEffort coprocessors. Clean-room port of Mesen2's `ArmV3Cpu`
  (`Core/SNES/Coprocessors/ST018/`), chosen over ares' `sfc/coprocessor/armdsp` (which instead
  wraps ares' generic shared ARM7TDMI component — a full ARM+Thumb superset the real ARMv3-class
  ST018 chip, predating Thumb, never needed). Built bottom-up: the barrel shifter/condition-codes/
  ALU core, the register file + mode banking, the 3-stage pipeline (whose exact timing implicitly
  produces ARM's well-known "PC reads as address+8" quirk), the full instruction set (data
  processing, branch, MSR/MRS, exception entry, `LDR`/`STR`, `LDM`/`STM`, multiply/multiply-long,
  `SWP`/`SWPB`), and finally the SNES-side board wrapper (`St018Board`) — driven by
  `Board::coprocessor_tick` (the same host-sync hook GSU/Super FX use) rather than the SA-1
  second-CPU hooks, since this ARM core (unlike SA-1's second 65C816) is entirely self-contained
  within `rustysnes-cart` and doesn't cross the one-directional crate graph. `Coprocessor::St018`
  is detected via a title match on the confirmed real cart, *Hayazashi Nidan Morita Shogi 2*
  (`NIDAN MORITASHOGI2`) — an earlier version of this doc wrongly assumed Star Ocean, which uses
  S-DD1 only, no ARM coprocessor; no commercial dump exists in this project's local corpus to
  verify the exact title string against, the same honesty gap already carried openly for the
  other title-matched `$F`-nibble customs. See `docs/st018-arm-notes.md` for the full
  architecture notes, detection research, and build order.

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
- **Super FX (`superfx_oncart`, feature `test-roms`):** boots the staged Krom GSU test ROMs
  (`tests/roms/external/krom/CHIP/GSU/`, CC0/homebrew, gitignored) — the `2/4/8 bpp`
  PlotPixel/PlotLine/FillPoly demos + the per-instruction `GSUTest` suite — on the full System and
  asserts (a) `Coprocessor::SuperFx` detection, (b) the GSU actually executed its program out of
  cart ROM (non-zero coprocessor-activity count — only possible if the `$3000–$32FF` window is
  mapped right *and* the host-sync run path works), (c) the `FillPoly` suites plot a substantial
  bitmap into the Game Pak RAM (read back via `Board::sram`), proving the whole plot pipeline —
  opcode-cache fetch, the `getbl`/`getbh` ROM-buffer scan-table reads, `ldw`/`stw` RAM, and the
  PLOT pixel-cache → character-format flush — end-to-end at the cart boundary (PPU-independent),
  and (d) a deterministic committed golden framebuffer hash. A 4 bpp `FillPoly` polygon also
  reaches the framebuffer; full PPU BG-mode coverage for 2/8 bpp is a PPU concern. The GSU
  instruction set is additionally exercised by the `GSUTest` per-opcode ROMs.
- **DSP-1 (`dsp1_oncart`, feature `test-roms`):** boots the staged DSP-1 dumps on the full System
  with the user-supplied (gitignored) `dsp1*.rom`, asserting (a) `Coprocessor::Dsp` detection, (b)
  a non-zero RQM-handshake access count on **both** the LoROM (Pilotwings) and HiROM (Super Mario
  Kart) windows — only possible if the window is mapped right *and* the µPD77C25 returns RQM, (c)
  a committed deterministic golden framebuffer hash, and (d) the firmware-differential (the Mode-7
  titles render differently with the chip installed). Engine decode/ALU/multiplier are unit-tested
  against a hand-assembled synthetic firmware (no copyrighted bytes).

## Open questions

- Per-board SRAM / coprocessor bus windows (no canonical table) — Phase 4 build-out.
- DSP nominal clock range (~7.6–8 MHz) — gated by test ROMs, not the number
  (`ref-docs/2026-06-24-coprocessors.md` "Flagged discrepancies").
