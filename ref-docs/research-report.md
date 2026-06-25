# RustySNES — Deep Research Report

**Generated:** 2026-06-24
**Mode:** Autonomous
**Source count:** ~30 primary (hardware/dev wikis, datasheets, emulator-author design notes, test-ROM repos), ~8 secondary
**Subject:** Super Nintendo Entertainment System / Super Famicom (SNES / SFC), cycle-accurate emulation in Rust

> This file is the **immutable research corpus** for RustySNES. After this step `ref-docs/`
> is frozen — later findings go in NEW dated files (e.g. `ref-docs/2026-06-24-coprocessors.md`),
> never edits to this file. It is the SNES analog of RustyNES's `ref-docs/research-report.md`
> and the source from which `docs/scheduler.md`, `docs/cpu-65816.md`, `docs/ppu.md`,
> `docs/apu.md`, `docs/cartridge.md`, and the crate graph are derived.

---

## Executive summary

The Super Nintendo (Super Famicom in Japan) is a 16-bit console built around a **WDC 65C816**
main CPU (in Nintendo's Ricoh **5A22** package), a **two-chip PPU** (Ricoh **5C77** = PPU1 +
**5C78** = PPU2), and an **independent audio subsystem** — the Sony **SPC700** (S-SMP) 8-bit CPU
plus the **S-DSP** wavetable synthesizer and **64 KiB of audio RAM (ARAM)** — that runs on its
own crystal, **asynchronous** to the rest of the machine. Everything in the main system is driven
from a single **21.477270 MHz NTSC master clock** (PAL: 21.281370 MHz), and the central accuracy
challenge of an SNES emulator is the same as the NES's, but harder: the machine has **two CPUs in
two clock domains**, the main CPU's cycle length is **variable** (6, 8, or 12 master clocks
depending on the memory region and the FastROM bit), and the PPU's dot/scanline lengths vary
(1360/1364/1368 master clocks per line). A faithful core must therefore model the **master clock
directly** and derive every chip's phase from it — a fractional-master-clock timebase, not a
fixed "1 dot = N CPU steps" lockstep.

The hardest single problem is keeping the **asynchronous SPC700 timeline coherent** with the main
CPU timeline at the **four communication ports** ($2140–$2143 on the CPU side / $F4–$F7 on the SMP
side). The reference accuracy emulators (higan/bsnes/ares, by Near/byuu) solve this with a
**cooperative-threaded scheduler**: each processor is a coroutine that runs ahead on a shared
relative-time counter, and `synchronize()` resumes the other processor up to the current time
**only when a cross-processor access occurs** (plus a forced sync once per scanline to bound audio
latency). This report establishes the master clock, the divisor table, the async-resync model,
the cart/coprocessor map, the test-ROM oracle, and the reference-emulator/license matrix that drive
the RustySNES scheduler and crate design.

The state of the art is well-mapped: ares (now **ISC-licensed**, hence vendor-friendly), bsnes,
and Mesen-S/Mesen2 (both **GPLv3**) are all cycle-accurate; a small but real **Rust-native**
ecosystem exists (twvd/**siena**, nat-rix/**rsnes**) for direct idiomatic study. The test-ROM
corpus is mature: per-instruction JSON suites for the 65816 **and** SPC700 (SingleStepTests),
Krom/PeterLemon CPU/PPU/SPC ROMs, blargg's cycle-accurate SPC/DSP suite, and the 240p Test Suite.

---

## Scope and goals

**In scope** (the v0.1+ accuracy target):

- WDC 65C816 main CPU (Ricoh 5A22): emulation + native modes, variable-width registers, the
  built-in DMA/HDMA controller, multiply/divide units, H/V-IRQ timers, NMI, joypad auto-read.
- PPU1 (5C77) + PPU2 (5C78): BG modes 0–7 (incl. Mode 7 affine), the 128-sprite OAM model,
  CGRAM/VRAM, color math/windows, the dot-clock timeline and H/V counters.
- SPC700 (S-SMP) + S-DSP + 64 KiB ARAM: 8 voices, BRR samples, Gaussian interpolation, the
  three timers, the four communication ports, and the **asynchronous clock domain**.
- The master-clock scheduler with the **6/8/12-cycle memory-speed map** and the
  1360/1364/1368-clock scanline variants; NTSC and PAL region timing **as data**.
- Cartridge memory map (LoROM / HiROM / ExHiROM / ExLoROM), the internal header + auto-detection
  heuristic, SRAM/battery saves, and the coprocessor family (DSP-1/2/3/4, Super FX/GSU, SA-1,
  S-DD1, SPC7110, CX4, OBC1, ST010/011/018, S-RTC).
- A determinism contract: same seed + ROM + input ⇒ bit-identical framebuffer + audio.

**Out of scope (initially):** the Satellaview (BS-X) modem, the Sufami Turbo, Super Game Boy
pass-through emulation, the SNES Mouse/Super Scope niche peripherals beyond a stub, and exotic
flash-cart mappers. These are documented here for completeness but tiered BestEffort/deferred.

**Success criteria for an implementation built from this report:**

1. The 65816 core passes the SingleStepTests/65816 JSON suite and Krom/gilyon CPU ROMs.
2. The SPC700 core passes SingleStepTests/SPC700 and blargg's `spc_*` suite (the cycle-accurate
   SPC/DSP oracle); the DSP passes `spc_dsp6`.
3. The PPU renders the 240p Test Suite (SNES) patterns correctly and matches reference
   framebuffers for the canonical commercial test set.
4. A bit-identical-AV determinism contract holds across save-state round-trip and replay.

---

## Background and context

The SNES launched in 1990 (JP) / 1991 (NA). Architecturally it is a "PPU is the timing master"
machine like the NES, but with a **decoupled audio computer** bolted on. Three facts dominate the
design and are the reason an SNES core is materially harder than an NES core:

1. **One master clock, many divisors, and a *variable* CPU cycle.** The NES CPU is always
   PPU÷3. The SNES CPU's cycle is 6, 8, or 12 master clocks depending on *what address it is
   touching* and the FastROM bit. You cannot pre-compute "CPU runs every N dots."
2. **Two asynchronous clock domains.** The SPC700/S-DSP run from a separate ~24.576 MHz resonator
   (÷24 → 1.024 MHz SPC700), unrelated to the 21.477 MHz master. Real consoles drift; this is a
   documented hardware-TAS-desync source.
3. **Sub-instruction visibility everywhere.** Mid-scanline scroll/Mode-7/color-math writes,
   HDMA firing at a precise dot, IRQ at an exact H/V position, and DMA stealing a precise,
   content-dependent cycle count all require dot-resolution (ideally master-clock-resolution)
   scheduling.

The SNES dev community's primary references are the **SNESdev wiki** ([snes.nesdev.org/wiki](https://snes.nesdev.org/wiki/Timing)),
the **Super Famicom Development wiki** ([wiki.superfamicom.org](https://wiki.superfamicom.org/timing)),
**Anomie's** register/timing docs (largely folded into the SNESdev wiki), and **nocash's
Fullsnes** ([problemkaputt.de/fullsnes.htm](https://problemkaputt.de/fullsnes.htm)). The
authoritative emulator-design writing is **byuu/Near's** articles on
[bsnes.org](https://bsnes.org/articles/cooperative-threading/) and
[byuu.net](https://byuu.net/design/schedulers/).

---

## Technical deep-dive

### 1. Clock topology — master clock + divisor table (the scheduler spine)

**Master clock frequencies:**

| Region | Master clock | Derivation | Source |
|---|---|---|---|
| NTSC | **21.477270 MHz** (= 945/44 MHz, "6× chroma", theoretically 1.89e9/88 Hz) | 6× the NTSC color burst | [SNESdev Timing](https://snes.nesdev.org/wiki/Timing) |
| PAL | **21.281370 MHz** (4.8× chroma) | 17.734475 MHz (4× chroma) × 6/5 | [SNESdev Timing](https://snes.nesdev.org/wiki/Timing) |

**65816 CPU cycle = 6, 8, or 12 master clocks**, selected by the address region and bit 0 of
`$420D` (MEMSEL / FastROM). "A CPU internal operation (an IO cycle) takes 6 master cycles. A
memory access cycle takes 6, 8, or 12 master cycles, depending on the memory region accessed and
bit 0 of CPU register $420D." ([Super Famicom Dev wiki — Timing](https://wiki.superfamicom.org/timing))

**The memory-access-speed map (master clocks per CPU access):**

| Address region | Banks | Speed | Cycles | Notes |
|---|---|---|---|---|
| System/WRAM mirror `$0000–$1FFF` | $00–$3F, $80–$BF | Slow | **8** | low 8 KiB WRAM mirror |
| PPU/APU/B-bus I/O `$2100–$21FF` | $00–$3F, $80–$BF | Fast | **6** | PPU1/PPU2 + APU ports |
| Old-style joypad `$4016/$4017` | $00–$3F, $80–$BF | **XSlow** | **12** | controller serial ports |
| CPU/DMA registers `$4200–$5FFF` | $00–$3F, $80–$BF | Fast | **6** | DMA, NMITIMEN, MEMSEL, etc. |
| Expansion `$6000–$7FFF` | $00–$3F, $80–$BF | Slow | **8** | cart-dependent |
| ROM `$8000–$FFFF` (WS1) | $00–$3F | Slow | **8** | always 8 |
| ROM `$8000–$FFFF` (WS2) | $80–$BF, $C0–$FF | **6 or 8** | MEMSEL: 0 = 8 (SlowROM), 1 = 6 (FastROM) |
| WRAM `$7E0000–$7FFFFF` | $7E–$7F | Slow | **8** | full 128 KiB work RAM |
| Internal cycles (no bus access) | — | Fast | **6** | always 6 |

Sources: [Fullsnes (nocash)](https://problemkaputt.de/fullsnes.htm), [SNESdev Timing](https://snes.nesdev.org/wiki/Timing),
[Super Famicom Dev wiki — Timing](https://wiki.superfamicom.org/timing). Resulting effective CPU
frequencies: 6 clocks → **3.58 MHz** (21.47727/6), 8 → **2.68 MHz** (/8), 12 → **1.79 MHz** (/12).

**MEMSEL `$420D` bit 0:** `0 = 2.68 MHz (SlowROM)`, `1 = 3.58 MHz (FastROM)` for WS2 ROM
($80–$FF). ([Fullsnes](https://problemkaputt.de/fullsnes.htm))

**Master-clock divisor table (the scheduler's core constants):**

| Chip | Advances per | Rate | Notes |
|---|---|---|---|
| Master clock | 1 tick | 21.477270 MHz | the finest practical quantum |
| 65816 (5A22) | 6 / 8 / 12 master clocks | 3.58 / 2.68 / 1.79 MHz | **variable per access** (see map) |
| PPU dot | **4 master clocks** (nominal) | ~5.37 MHz | with long-dot exceptions (below) |
| SPC700 (S-SMP) | **separate ~1.024 MHz clock** | ~1.024 MHz | **asynchronous** — own resonator |
| S-DSP | 3.072 MHz internal (24.576 MHz resonator ÷8) | 32000 Hz sample | 1 stereo sample / 768 resonator cycles |

The SPC700 is **not** a master-clock divisor — it is a second clock domain (see §3). For the
emulator, the practical relative-time constants are the two source frequencies, **24,576,000 Hz
(SMP resonator)** and **21,477,272 Hz (main master)** — see the resync model in §3.

### 2. Video timing — dots, scanlines, frames

- **Normal scanline = 1364 master clocks = 341 dot cycles** (340 dots of 4 clocks + the long
  dots). Equivalently SNESdev counts **340 dots (0–339)** where dots 323 and 327 are 6 master
  clocks instead of 4. Both descriptions are the same silicon — **pick one numbering convention
  and document it**; the invariant is **1364 = 336×4 + 4×5** (or 338×4 + 2×6).
  ([SNESdev Timing](https://snes.nesdev.org/wiki/Timing), [Super Famicom Dev wiki](https://wiki.superfamicom.org/timing))
- **Short scanline (1360 clocks / 340 dots):** NTSC, non-interlace, scanline V=240 ($F0) of every
  other frame (those with $213F.7=1). ([SNESdev Timing](https://snes.nesdev.org/wiki/Timing))
- **Long scanline (1368 clocks / 341 dots):** PAL, interlace on, field=1, V=311.
  ([SNESdev Timing](https://snes.nesdev.org/wiki/Timing))
- **Scanlines/frame:** **262 (NTSC) / 312 (PAL)** non-interlace; interlace adds a line on alternate
  fields → **263 / 313**. Last VBlank line: **261 (NTSC) / 311 (PAL)**.
  ([SNESdev Timing](https://snes.nesdev.org/wiki/Timing))
- **Per-frame master clocks (NTSC):** ~357,368; (PAL) ~425,568. ([SNESdev Timing](https://snes.nesdev.org/wiki/Timing))
- **WRAM refresh:** the CPU is **paused for 40 master clocks** beginning ~536 clocks into each
  scanline (DRAM refresh). ([SNESdev Timing](https://snes.nesdev.org/wiki/Timing))
- **H/V-IRQ + NMI:** the 5A22 raises NMI at VBlank start (V=225, or V=240 in overscan) and can
  raise an IRQ at a programmed H and/or V counter position (`$4207–$420A`), enabling mid-frame
  raster effects. The H/V counters are latched by reading `$2137` (SLHV) and read back from
  `$213C`/`$213D`.

The full PPU register/timeline detail (BG modes, OAM, CGRAM/VRAM, color math, DMA/HDMA) is in
**[ref-docs/2026-06-24-ppu.md](2026-06-24-ppu.md)**.

### 3. The SPC700 / S-DSP audio subsystem and the async-resync model (the accuracy crux)

**Hardware facts:**

- **SPC700 clock = 1.024 MHz**, "independent from the rest of the SNES, and may drift slightly
  with temperature." It is generated from a **24.576 MHz ceramic resonator** (the same resonator
  clocks the S-DSP). ([SNESdev S-SMP](https://snes.nesdev.org/wiki/S-SMP))
- **Resonator tolerance ±0.5% (5000 ppm).** Measured per-console S-DSP sample rates span
  **32036–32152 Hz** (nominal 32000 Hz), warming slightly with temperature. The variable
  SPC700/S-CPU communication delays plus this drift are a documented cause of **TAS desyncs on
  real hardware**. ([undisbeliever — S-SMP clock measurements](https://undisbeliever.net/blog/20250313-smpspeed.html))
- **S-DSP:** 8 voices, **BRR-compressed samples** decoded with **4-point Gaussian interpolation**,
  32 kHz 16-bit stereo, "1 stereo sample every 768 resonator cycles." ([SNESdev S-SMP](https://snes.nesdev.org/wiki/S-SMP))
- **64 KiB ARAM** from two 32K×8 PSRAM chips, time-shared "1 S-SMP access for every 2nd S-DSP
  access." ([SNESdev S-SMP](https://snes.nesdev.org/wiki/S-SMP))
- **Three timers:** two at 8 kHz, one at 64 kHz. ([SNESdev S-SMP](https://snes.nesdev.org/wiki/S-SMP))
- **Four communication ports:** `$F4–$F7` on the SMP side ↔ `$2140–$2143` on the CPU side. "8
  stored values, each is a one-way communication written from one side, and readable only from the
  other side" — each port is two latches (CPU→SMP and SMP→CPU). ([SNESdev S-SMP](https://snes.nesdev.org/wiki/S-SMP))

**The async-resync model (one paragraph):** Because the SPC700 runs on its own crystal, a faithful
emulator runs the SPC700/S-DSP as a **separate timeline** and only forces it into lockstep with the
main CPU **when the two actually communicate** — i.e. when the CPU reads or writes one of the four
ports `$2140–$2143` (or vice versa). higan/bsnes/ares (Near/byuu) implement this with a
**cooperative-threaded scheduler**: each processor is a coroutine (libco) holding its own clock;
when the CPU steps N of its clocks it **subtracts N×24,576,000** from a shared relative-time
counter, and when the SMP steps N it **adds N×21,477,272** — the two source frequencies as scaling
factors so the single signed counter expresses "how far ahead is the CPU vs the SMP" exactly,
without floating point. `Thread::synchronize(other)` resumes `other` until its clock catches up to
the caller's **before** any cross-processor access, so the SMP can run arbitrarily far ahead of the
CPU as long as neither touches the ports; a sync is also **forced once per scanline** to keep audio
latency bounded. ([byuu.net — Designing a scheduler](https://byuu.net/design/schedulers/),
[bsnes.org — Cooperative threading](https://bsnes.org/articles/cooperative-threading/))

For RustySNES, the architecturally faithful equivalent is a **master-clock scheduler** where the
CPU/PPU/HDMA advance in lockstep on the 21.477 MHz timebase, and the SPC700/S-DSP advance on their
own 1.024 MHz timebase with a **relative-time accumulator** (the ×24576000 / ×21477272 trick, or
the equivalent rational ratio) that the bus uses to resync the SMP up to "now" on every
`$2140–$2143` access and once per scanline. This preserves the determinism contract (the relative
counter is integer/exact, no host RNG or wall-clock leakage) while modeling the genuine asynchrony.

Full SPC700/S-DSP register and BRR detail is in **[ref-docs/2026-06-24-apu.md](2026-06-24-apu.md)**.

### 4. The 65816 main CPU (Ricoh 5A22)

- **Core:** WDC 65C816, 16-bit, in Nintendo's Ricoh 5A22 wrapper. The 5A22 adds multiply/divide
  registers, the DMA/HDMA hardware, NMI/IRQ timers, and joypad auto-read.
  ([SNESdev S-CPU / 65C816](https://snes.nesdev.org/wiki/S-CPU))
- **Emulation vs native mode (the E flag):** powers on in **6502 emulation mode** (behaves as a
  65C02 with NMOS cycle counts); A and index registers locked to 8 bits. Code does `CLC : XCE` to
  enter **native mode**, where the **M** (accumulator/memory width) and **X** (index width) status
  bits select 8- or 16-bit registers via `REP`/`SEP`.
  ([Super Famicom Dev wiki — 65816 reference](https://wiki.superfamicom.org/65816-reference),
  [WDC 65C816 — Wikipedia](https://en.wikipedia.org/wiki/WDC_65C816))
- **Registers:** A (accumulator), X/Y (index), S (16-bit stack), **D** (16-bit direct page),
  **DBR** (data bank), **PBR** (program bank), P (status with M/X flags). 24-bit address space
  (16 MiB) via the bank registers. ([Super Famicom Dev wiki — 65816 reference](https://wiki.superfamicom.org/65816-reference))
- **Variable instruction cycle counts:** +1 cycle if m=0 (16-bit memory/accumulator), +1 if the
  low byte of D is non-zero (direct-page misalignment), +1 if an indexed access crosses a page
  boundary. ([Super Famicom Dev wiki — 65816 reference](https://wiki.superfamicom.org/65816-reference))
- **Vectors:** RESET/NMI/IRQ/BRK/COP/ABORT, with separate emulation-mode and native-mode vector
  tables at the top of bank 0.

Full register/opcode/timing detail (every opcode's cycle formula) belongs in `docs/cpu-65816.md`,
derived from the [Super Famicom Dev wiki 65816 reference](https://wiki.superfamicom.org/65816-reference)
and [undisbeliever's opcode reference](https://undisbeliever.net/snesdev/65816-opcodes.html).

### 5. DMA and HDMA

The 5A22 contains **8 DMA channels** (registers `$43n0–$43nA`, n = 0–7), enabling general-purpose
DMA (`MDMAEN $420B`) and per-scanline HDMA (`HDMAEN $420C`).

- **GP-DMA:** **8 master clocks per byte** (regardless of FastROM), +8 cycles/channel overhead,
  +12–24 cycles whole-transfer alignment. **The CPU is fully halted** until all transfers finish;
  the transfer actually fires "in the middle of the following instruction." Cannot cross a bank.
  ([SNESdev DMA registers](https://snes.nesdev.org/wiki/DMA_registers),
  [Super Famicom Dev wiki — DMA & HDMA](https://wiki.superfamicom.org/dma-and-hdma))
- **HDMA:** fires at ~H=$116 each visible line; **~18 cycles overhead** while any channel active,
  **+8 cycles per active direct channel per scanline**, **+8 cycles/byte**; **indirect channels
  cost 24 cycles** (16-cycle pointer load). Worst case ~466 cycles/scanline (8 channels, indirect,
  4 bytes). **HDMA preempts GP-DMA.** ([Super Famicom Dev wiki — DMA & HDMA](https://wiki.superfamicom.org/dma-and-hdma))

This precise, content-dependent cycle theft is a key reason the scheduler must be master-clock
resolution. Full register/transfer-pattern detail in [ref-docs/2026-06-24-ppu.md](2026-06-24-ppu.md).

### 6. Cartridge memory map and the header

- **LoROM ($20):** 32 KiB ROM windows in the upper half ($8000–$FFFF) of each bank; header at ROM
  offset `$007FC0`. Max 4 MiB.
- **HiROM ($21):** full 64 KiB linear banks at $C0–$FF; header at `$00FFC0`. Max 4 MiB.
- **ExHiROM ($25):** >4 MiB (up to ~8 MiB); header at `$40FFC0`. (Tales of Phantasia, Star Ocean.)
  **ExLoROM** is the analogous LoROM extension (mostly homebrew/flashcart).
- **Internal header** at `$FFC0–$FFDF`: 21-byte title, **map-mode+speed byte $FFD5**, **chipset
  byte $FFD6**, ROM size $FFD7 (1<<N KiB), RAM size $FFD8, region $FFD9, checksum+complement
  ($FFDC/$FFDE, summing to $FFFF).
- **Auto-detection (header score heuristic):** score the candidate header at $7FC0 / $FFC0 /
  $40FFC0 on (a) complement+checksum = $FFFF, (b) map-mode byte matching the location, (c)
  plausible size/region bytes, (d) reset-vector plausibility (≥$8000; first opcode a likely boot
  instruction sei/clc/sec/stz/jmp/jml, not brk/cop/stp/wdm). Highest score wins.

Sources: [SNESdev Memory map](https://snes.nesdev.org/wiki/Memory_map),
[SNESdev ROM header](https://snes.nesdev.org/wiki/ROM_header),
[SnesLab SNES ROM Header](https://sneslab.net/wiki/SNES_ROM_Header).

### 7. Coprocessors

Each coprocessor is effectively a "mapper-equivalent" with its own bus behavior. Full per-chip
detail (bus windows, clocks, games, emulation approach) is in
**[ref-docs/2026-06-24-coprocessors.md](2026-06-24-coprocessors.md)**. Summary tier table:

| Chip | Core | ~Games | Shared core? | Emu approach | Suggested tier |
|---|---|---|---|---|---|
| DSP-1/1A/1B | NEC µPD77C25 | 15+ | µPD77C25 family | LLE (needs prog ROM) | **Core/Curated** |
| DSP-2 / DSP-3 / DSP-4 | µPD77C25 | 1 each | µPD77C25 | LLE | BestEffort (shared) |
| Super FX / GSU-1/2 | Argonaut RISC | ~8 | no | cycle-accurate (ROM in cart) | **Core/Curated** |
| SA-1 | 65C816 @ 10.74 MHz | ~35 | (65C816) | cycle-accurate (ROM in cart) | **Core/Curated** |
| S-DD1 | Nintendo ASIC | 2 | no | algorithm-exact | BestEffort |
| SPC7110 (+RTC-4513) | Hudson ASIC | 3 | no | algorithm + frozen RTC | BestEffort |
| CX4 | Hitachi HG51B169 | 2 | no | LLE | BestEffort/Curated |
| OBC1 | simple ASIC | 1 | no | HLE | BestEffort |
| ST010 / ST011 | NEC µPD96050 | 1 each | µPD96050 (≈77C25) | LLE (shares DSP core) | BestEffort (shared) |
| ST018 | ARMv3 32-bit | 1 | no | LLE ARM core (costly) | BestEffort |
| S-RTC | Epson RTC | 1 | no | HLE + frozen time | BestEffort |

**Key leverage:** one **µPD77C25/µPD96050 LLE core** covers DSP-1/2/3/4 **and** ST010/011 (six
chips, one engine). Super FX and SA-1 run their programs from cart ROM (no chip-ROM dump). The RTC
chips (S-RTC, RTC-4513) are the **determinism hazard** — seed/freeze them, never read host wall
time into the core.

---

## State of the art / prior art (reference emulators)

**Accuracy ranking (cycle-accurate tier first):**

| Emulator | Accuracy | Language | License (verified 2026-06) | Study posture |
|---|---|---|---|---|
| **ares** (SNES core descends from bsnes/higan) | Reference cycle-accurate, LLE coprocessors | C++ | **ISC** (was CC BY-NC-ND historically) | **Vendor-OK** (permissive) — but verify the cloned revision's license |
| **bsnes** | Cycle-accurate ancestor; the accuracy bar | C++ | **GPLv3** | **Clone-only**, clean-room |
| **Mesen-S / Mesen2** | Cycle-accurate; best debugger; the only emulator that passed *all* of blargg's SPC/DSP tests | C++/C# | **GPLv3** | **Clone-only**, clean-room |
| **higan** | bsnes successor / ares predecessor | C++ | GPLv3 | Clone-only (mostly superseded by ares) |
| **Snes9x** | Compatibility-focused, faster, less accurate | C++ | **Non-commercial** (custom, not GPL) | **Clone-only** (license forbids commercial reuse) |
| **siena** (twvd) | Claims cycle-accurate 65816 + SPC700; DSP-1/SuperFX/partial SA-1; scanline-threaded | **Rust** | Unspecified (treat as clone-only until verified) | **Direct idiomatic study** (Rust) |
| **rsnes** (nat-rix) | WIP; 65816 + SPC700 + coprocessors | **Rust** | verify | Direct idiomatic study |

Sources: [ares repo (ISC)](https://github.com/ares-emulator/ares),
[Mesen2 (GPLv3)](https://github.com/SourMesen/Mesen2),
[Snes9x (non-commercial)](https://emulation.gametechwiki.com/index.php/Snes9x),
[siena (Rust)](https://github.com/twvd/siena), [rsnes (Rust)](https://github.com/nat-rix/rsnes),
[ares license note (Emulation General Wiki)](https://emulation.gametechwiki.com/index.php/Ares).

**Recommendation:** mine **ares** (ISC, vendor-safe, the accuracy reference) as the primary
behavioral + code reference; use **Mesen-S** as the debugger-grade behavioral oracle (it passes the
hardest SPC/DSP suite); study **siena**/**rsnes** for Rust idioms and the crate-graph port. Treat
bsnes/Mesen2/Snes9x as **clone-only / clean-room** (copyleft or non-commercial). The clean-room
boundary: re-implement from the **test ROMs + this `ref-docs/` corpus**, not from copyleft source
lines.

---

## Standards / test-ROM corpora (the accuracy oracle)

**All licenses below were verified live (2026-06-24) by reading the actual repo LICENSE files via
`gh api`, not assumed.** The licensing is heterogeneous and has real gotchas — note especially that
the SingleStepTests **65816** set ships **NO license** while its **spc700** sibling is MIT.

| Corpus | Covers | Form | License (verified) | Role |
|---|---|---|---|---|
| **[SingleStepTests/65816](https://github.com/SingleStepTests/65816)** | 65816 per-opcode, all addr modes, 8/16-bit, native+emulation, **with cycle-by-cycle bus-pin trace** (20k tests/opcode, 512 JSON files) | JSON | **NONE** (no LICENSE; `gh api …/license` → 404) ⚠️ | **Primary CPU oracle** — but license-gate it (gitignored external tier or generate your own) |
| **[SingleStepTests/spc700](https://github.com/SingleStepTests/spc700)** | SPC700 per-opcode, golden-state + bus activity | JSON | **MIT** ✅ | **Primary SPC oracle** (cleanly licensed) |
| **[gilyon/snes-tests](https://github.com/gilyon/snes-tests)** (Krom) | 65C816 + SPC-700 (all opcodes except STP/WAI · SLEEP/STOP), all addr modes, emulation+native, wrapping; ships golden `tests*.txt` tables | `.sfc` ROMs (runnable headless) | **MIT** ✅ | **The committable nestest-equivalent layer** (CPU + SPC, with golden tables) |
| **[undisbeliever/snes-test-roms](https://github.com/undisbeliever/snes-test-roms)** | HDMA timing, force-blank mid-frame, mid-scanline VRAM, OAM dropout, auto-joypad, multiply-in-flight, hardware-glitch tests | `.sfc` ROMs (mostly visual) | **Zlib** ✅ | **Committable PPU/DMA/HDMA hardware-behavior layer** |
| **[PeterLemon/SNES](https://github.com/PeterLemon/SNES)** (Krom) | CPU / PPU (Mode7/HDMA/window/blend) / SPC / DSP / GSU / bank-mode / MSU-1 | `.sfc` + ref `.png` (screenshot-diff) | **NONE** (no LICENSE; 404) ⚠️ | reference-only, do not commit/vendor |
| **blargg `spc_*`** (spc_dsp6, spc_mem_access_times, spc_spc, spc_timer) | **Cycle-accurate SPC700 + S-DSP** (first cycle-accurate DSP tests; per-opcode hash) | `.sfc` ROMs | unstated (dev tools) ⚠️ | **The SPC/DSP accuracy oracle** (Mesen-S passes all) — external tier |
| **[blargg `snes_spc` library](https://github.com/blarggs-audio-libraries/snes_spc)** | de-facto cycle-accurate S-DSP reference model | C library (`.spc`→`.wav`) | **LGPL-2.1** ⚠️ | external **audio-comparison oracle only**, never vendor |
| **[240pTestSuite (SNES)](https://github.com/ArtemioUrbina/240pTestSuite)** | video / 240p / overscan calibration patterns (human-eyeball) | `.sfc` | **GPL-2.0-or-later** ⚠️ | clone/run, **do not vendor**; PPU/video verification |
| **TASVideos SNES accuracy tests** (Nintendo service ROMs: Aging Test, Cx4/SPC7110 check) | WRAM/DRAM/VRAM/DMA/OAM/CGRAM/multiply/HV-timer, deterministic pass/fail | `.sfc` | **Nintendo-copyrighted** ✗ | reference-only, **not redistributable** |

**The "AccuracyCoin-equivalent" oracle choice (two-layer):** RustyNES uses a single 139-test
AccuracyCoin battery. The SNES has **no single canonical battery** — and no Nintendulator-style
textual golden CPU log exists for the 65816 — so the recommended oracle is **composed**:

1. **Primary CPU oracle:** **SingleStepTests `65816` + `spc700`** JSON (per-opcode, bus-accurate,
   both CPUs) — the direct analog of the NES SingleStepTests RustyNES already trusts. **License
   snag:** `spc700` is MIT (clean); **`65816` ships no LICENSE** — obtain explicit permission, keep
   it in a gitignored external tier, or generate equivalent JSON from a validated core.
2. **Committable on-cart system layer:** **gilyon/snes-tests (MIT)** — pass/fail `.sfc` for both
   CPUs with golden `tests*.txt`; commit it. + **undisbeliever/snes-test-roms (Zlib)** for
   PPU/DMA/HDMA hardware behavior.
3. **Audio:** **blargg `spc_*`** ROMs (the cycle-accurate SPC/DSP gate; `spc_dsp6` hardest) and
   output comparison vs **blargg `snes_spc`** — both **external/reference tier** (unstated / LGPL).
4. **Video/integration:** the 240p Test Suite (GPLv2, run-only) + Krom PeterLemon (no-license,
   reference-only) for screenshot-diff.

Because **Mesen-S is the only emulator that passed all of blargg's SPC/DSP tests**, matching that
suite is the concrete accuracy bar. Model the in-repo harness on **Mesen2's `RecordedRomTest`**
(per-frame screenshot-hash baseline+replay — study the design, don't copy the GPLv3 code).
Mirror RustyNES's existing commercial-ROM policy: commit only the permissive corpora (gilyon MIT,
undisbeliever Zlib); keep unlicensed/copyleft/Nintendo ROMs in a gitignored `tests/roms/external/`.
([byuu — blargg's SPC test ROMs](https://forums.nesdev.org/viewtopic.php?t=18005),
[gilyon/snes-tests](https://github.com/gilyon/snes-tests),
[Mesen2 RecordedRomTest](https://github.com/SourMesen/Mesen2/blob/master/Core/Shared/RecordedRomTest.h))

---

## Principal engineering challenges

1. **Variable-cycle CPU on a fractional master clock.** The 6/8/12-cycle access map means the CPU
   cannot be a fixed master-clock divisor. *Mitigation:* the bus returns the access speed for each
   address; the scheduler advances the master clock by that many ticks per CPU cycle and re-derives
   the PPU/HDMA phase. This is the same direction as RustyNES's future "Timebase" rewrite — start
   there, don't retrofit a dot-lockstep model.
2. **The asynchronous SPC700 resync.** (See §3.) *Failure modes:* audio desync, missed
   handshakes during the boot IPL upload, TAS non-determinism. *Mitigation:* the integer
   relative-time accumulator + sync-on-port-access + once-per-scanline forced sync; never let host
   time leak in.
3. **HDMA / DMA cycle theft.** Content-dependent, channel-dependent, fires at a precise dot, and
   HDMA preempts GP-DMA. *Mitigation:* model DMA as a CPU stall inserted at the MDMAEN write; model
   HDMA as a per-line budget evaluated at H≈$116.
4. **Mode 7 + mid-scanline writes + sprite over/time limits.** Sub-instruction PPU visibility
   (scroll/Mode-7/CGRAM mid-line, the 32-sprite/34-tile limits, the latch counters). *Mitigation:*
   dot-resolution PPU rendering with the master-clock scheduler so writes land at the exact dot.
5. **Coprocessor breadth + the DSP LLE core.** Twelve+ chips, but six share the NEC DSP core.
   *Mitigation:* implement the µPD77C25/96050 LLE engine once; tier the rest; gate chip-ROM-dump
   dependence behind honesty flags (Core/Curated/BestEffort).
6. **Determinism vs RTC chips and the drifting SPC resonator.** *Mitigation:* seed/freeze all
   clocks; the SPC domain uses a fixed nominal 1.024 MHz in the deterministic core (document that
   real-hardware drift is intentionally not modeled in the deterministic path).

---

## Architecture options (surfaced, not decided — a Phase 3 call)

1. **Single-threaded master-clock lockstep (RustyNES-style, extended).** One scheduler advances the
   master clock; CPU/PPU/HDMA step on their divisors; the SPC700 domain steps via the relative-time
   accumulator, resynced on port access. *Pro:* deterministic, debuggable, matches the RustyNES
   architecture the project is modeled on. *Con:* the variable CPU cycle + async SPC need careful
   accumulator math. **Recommended default** — it is the faithful + deterministic choice and mirrors
   the reference repo.
2. **Cooperative-threaded coroutines (higan/bsnes-style).** Each chip a libco-style coroutine;
   sync-on-access. *Pro:* the proven accuracy model; minimal sync points on the hot path. *Con:*
   coroutines fit Rust awkwardly (would need a state-machine or `generator`/async transform);
   harder to make bit-deterministic for save-states/netplay. *Use as the conceptual model, not the
   literal implementation.*
3. **Catch-up / lazy sync (Snes9x-style).** Run the CPU freely, catch the PPU/APU up at sync
   points. *Pro:* fast. *Con:* loses sub-instruction accuracy — **rejected** for the accuracy bar.

The recommended path: option 1 (single-threaded master-clock), borrowing option 2's
**relative-time-accumulator + sync-on-access** technique for the SPC700 domain.

---

## External dependencies and integrations

- **Frontend (mirrors RustyNES):** winit + wgpu + cpal + egui — pure Rust, permissive.
- **Coprocessor chip ROMs:** the NEC DSP / ST01x / CX4 / ST018 LLE cores require the user to supply
  dumped chip program ROMs (not distributable) — gate behind a feature + honesty caveat.
- **Test ROMs (license-verified):** commit only the permissive corpora — **gilyon/snes-tests (MIT)**
  + **undisbeliever/snes-test-roms (Zlib)** + **SingleStepTests/spc700 (MIT)**. The
  **SingleStepTests/65816 set ships NO license** — keep it in the gitignored external tier (or
  generate equivalent JSON). PeterLemon (no license), blargg `spc_*` (unstated), blargg `snes_spc`
  (LGPL-2.1), 240p Suite (GPLv2), and the Nintendo service ROMs are **external/reference-only —
  do not vendor** into the MIT/Apache tree.
- **Reference emulators:** **ares (ISC, vendor-safe study)** primary — the one top-accuracy core you
  may legally adapt; **siena / rsnes / ness / r-snes (Rust)** for idiom study; bsnes / higan / Mesen2
  (GPL-3.0) and Snes9x (non-commercial) are **clone-only / clean-room**.

---

## Open questions

1. **Exact 65816 per-opcode master-clock breakdown** for the rarer addressing modes — resolve
   against SingleStepTests bus traces during implementation (the JSON includes per-cycle bus
   activity, so this is a verify-against-the-oracle item, not a research gap). **Caveat:** the
   65816 JSON set has no license — secure permission or generate equivalent JSON before relying on
   it in CI.
2. **SPC700 instruction cycle edge cases** (the `STP`/`SLEEP` and a few timer-edge behaviors) —
   blargg's `spc_*` + SingleStepTests/SPC700 settle these empirically.
3. **Per-board SRAM/coprocessor bus windows** — board-dependent, no single canonical table; build
   a per-board map from the cartridge database + ares's board definitions.
4. **DSP clock** cited as ~7.6 MHz (Wikipedia) vs 8 MHz (emulator configs) — record the range; the
   LLE core's correctness is gated by the test ROMs, not the nominal clock.
5. **Whether to model SPC resonator drift** at all in the deterministic core (default: no — fixed
   1.024 MHz; offer it only as a non-deterministic "hardware-accurate audio" toggle).

---

## Source manifest

### Tier 1 / Tier 2 — primary hardware & emulator-author references

1. [SNESdev wiki — Timing](https://snes.nesdev.org/wiki/Timing) — master clock, CPU cycle speeds, scanline/frame structure, short/long lines.
2. [SNESdev wiki — S-SMP](https://snes.nesdev.org/wiki/S-SMP) — SPC700 clock, async domain, 4 ports, S-DSP, ARAM, BRR, timers.
3. [SNESdev wiki — Memory map](https://snes.nesdev.org/wiki/Memory_map) — LoROM/HiROM/ExHiROM, address-pin wiring, mirroring, SRAM.
4. [SNESdev wiki — ROM header](https://snes.nesdev.org/wiki/ROM_header) — header fields, map-mode/chipset bytes, checksum, auto-detection heuristic.
5. [SNESdev wiki — S-CPU / 65C816](https://snes.nesdev.org/wiki/S-CPU) — the 5A22, 65816 core.
6. [SNESdev wiki — PPU registers](https://snes.nesdev.org/wiki/PPU_registers) — BGMODE, M7A–D, OBSEL, CGRAM/VRAM, color math, windows, counters.
7. [SNESdev wiki — Backgrounds](https://snes.nesdev.org/wiki/Backgrounds) — BG mode bpp/layer table, Mode 7, hi-res, offset-per-tile.
8. [SNESdev wiki — Sprites](https://snes.nesdev.org/wiki/Sprites) — 128-sprite OAM, size pairs, 32/34 limits, priority/draw order.
9. [SNESdev wiki — DMA registers](https://snes.nesdev.org/wiki/DMA_registers) — 8 channels, $43nx, transfer patterns, MDMAEN/HDMAEN.
10. [SNESdev wiki — Mode 7 transform](https://snes.nesdev.org/wiki/Mode_7_transform) — affine matrix math.
11. [Super Famicom Dev wiki — Timing](https://wiki.superfamicom.org/timing) — CPU IO/access cycle counts, $420D MEMSEL, scanline length.
12. [Super Famicom Dev wiki — DMA & HDMA](https://wiki.superfamicom.org/dma-and-hdma) — exact DMA/HDMA cycle costs, preemption.
13. [Super Famicom Dev wiki — 65816 reference](https://wiki.superfamicom.org/65816-reference) — register set, M/X flags, cycle penalties, modes.
14. [nocash Fullsnes](https://problemkaputt.de/fullsnes.htm) — comprehensive: memory-speed map, PPU chip split, BRR, registers (note: direct fetch 403s; mirrored via patrickjohnston.org).
15. [byuu.net — Designing a cooperative-threaded scheduler](https://byuu.net/design/schedulers/) — relative-time counter (×24576000 / ×21477272), synchronize().
16. [bsnes.org — Cooperative threading](https://bsnes.org/articles/cooperative-threading/) — libco coroutine model, sync granularity.
17. [bsnes.org — The State of Emulation, Part IV (Near)](https://bsnes.org/articles/state-of-emulation-4/) — LLE coprocessor list, accuracy posture.
18. [undisbeliever — S-SMP clock speed measurements](https://undisbeliever.net/blog/20250313-smpspeed.html) — measured SPC/DSP rates, ±0.5% resonator drift, TAS-desync implication.
19. [higan/sfc/cpu/timing.cpp (byuu/higan)](https://github.com/byuu/higan/blob/master/higan/sfc/cpu/timing.cpp) — reference CPU step/sync implementation.
20. [WDC 65C816 — Wikipedia](https://en.wikipedia.org/wiki/WDC_65C816) — emulation/native mode, E flag, reset behavior.
21. [undisbeliever — 65816 opcodes](https://undisbeliever.net/snesdev/65816-opcodes.html) — opcode cycle reference.
22. [SnesLab — SNES ROM Header](https://sneslab.net/wiki/SNES_ROM_Header) — header cross-validation.
23. [Wikipedia — List of Super NES enhancement chips](https://en.wikipedia.org/wiki/List_of_Super_NES_enhancement_chips) — coprocessor inventory, game counts.

### Test ROMs / corpora (licenses verified via `gh api` 2026-06-24)

24. [SingleStepTests/65816](https://github.com/SingleStepTests/65816) — per-opcode JSON + bus-pin trace (primary CPU oracle); **NO LICENSE (404) — license-gate it.**
25. [SingleStepTests/spc700](https://github.com/SingleStepTests/spc700) — per-opcode JSON (primary SPC oracle); **MIT.**
26. [gilyon/snes-tests](https://github.com/gilyon/snes-tests) — Krom 65816 + SPC700 ROMs + golden tables (committable nestest-equivalent); **MIT.**
27. [undisbeliever/snes-test-roms](https://github.com/undisbeliever/snes-test-roms) — PPU/DMA/HDMA + hardware-glitch behavior; **Zlib.**
28. [PeterLemon/SNES](https://github.com/PeterLemon/SNES) — broad CPU/PPU/SPC/DSP/GSU/MSU-1 ROMs + ref PNGs; **NO LICENSE — reference-only.**
29. [blargg's SPC test ROMs (NESDev forum, byuu recovery)](https://forums.nesdev.org/viewtopic.php?t=18005) — spc_dsp6/spc_mem_access_times/spc_spc/spc_timer (cycle-accurate SPC/DSP oracle); unstated — external tier.
30. [blargg `snes_spc` library](https://github.com/blarggs-audio-libraries/snes_spc) — cycle-accurate S-DSP audio reference; **LGPL-2.1** (external comparison only).
31. [240pTestSuite](https://github.com/ArtemioUrbina/240pTestSuite) — video/240p calibration; **GPL-2.0-or-later** (run-only).

### Reference emulators (license-verified via `gh api` 2026-06-24)

32. [ares (ISC)](https://github.com/ares-emulator/ares) — accuracy reference, **vendor-safe** (ISC); GitHub reports NOASSERTION only due to bundled deps — actual LICENSE is verbatim ISC.
33. [bsnes (GPL-3.0)](https://github.com/bsnes-emu/bsnes) — accuracy ancestor; clone-only.
34. [Mesen2 (GPL-3.0)](https://github.com/SourMesen/Mesen2) / [Mesen-S](https://github.com/SourMesen/Mesen-S) — debugger-grade; passes blargg SPC/DSP; study its [`RecordedRomTest`](https://github.com/SourMesen/Mesen2/blob/master/Core/Shared/RecordedRomTest.h) harness.
35. [higan (GPL-3.0)](https://github.com/higan-emu/higan) — ares predecessor; clone-only (prefer ares).
36. [Snes9x (custom non-commercial)](https://github.com/snes9xgit/snes9x) — NOT GPL; clone-only.
37. [siena (Rust)](https://github.com/twvd/siena) — cycle-accurate 65816/SPC700 Rust reference (license unspecified — clone-only until verified).
38. [rsnes (Rust, MIT)](https://github.com/nat-rix/rsnes) / [ness (Rust, no license)](https://github.com/kelpsyberry/ness) / [r-snes (Rust, MIT)](https://github.com/r-snes/r-snes) — Rust idiom study (no mature accuracy-focused Rust SNES core exists — the field is open).
39. [ares — Emulation General Wiki](https://emulation.gametechwiki.com/index.php/Ares) — ISC license confirmation + history.

### Tier 3 — supplementary (pointers, not load-bearing)

35. [nesdev forum — "What 65816 test analogous to nestest?"](https://forums.nesdev.org/viewtopic.php?t=18446)
36. [nesdev forum — Cooperative-threaded scheduler design](https://forums.nesdev.org/viewtopic.php?f=23&t=18988)
37. [jsgroth.dev — SNES coprocessors blog series](https://jsgroth.dev/blog/posts/snes-coprocessors-part-1/) (bot-gated; corroborated via search).
38. [Fabien Sanglard — SNES PPU architecture](https://fabiensanglard.net/snes_ppus_why/) — secondary overview.

**Source-quality note:** the corpus is primary-heavy. The two non-fetchable primaries —
Fullsnes (HTTP 403 on direct fetch) and the jsgroth.dev coprocessor series (Anubis bot-gate) —
were captured via search excerpts and **cross-validated against the SNESdev / Super Famicom wikis**,
which independently corroborate every fact used from them. The 340-vs-341 dot convention and the
~7.6-vs-8 MHz DSP clock are explicitly flagged as source-convention/range discrepancies rather than
resolved silently.
