# Scheduler — RustySNES

**References:** `ref-docs/research-report.md` §§1, 2, 3, 5; `ref-docs/2026-06-24-ppu.md` §§4–5;
`ref-docs/2026-06-24-apu.md` §2; `docs/adr/0001`, `docs/adr/0002`.

## Purpose

The scheduler is the timebase that every chip phase derives from. It owns the single
**21.477270 MHz NTSC** (PAL **21.281370 MHz**) master clock and advances the CPU, PPU, DMA,
and HDMA in lockstep on it, while the SPC700 / S-DSP run in a **second, asynchronous clock
domain** resynced on demand. This is the central architectural choice (`docs/adr/0001`) and
the reason mid-instruction events work without per-quirk patches.

## The master-clock model (not pure dot-lockstep)

The NES CPU is always PPU÷3, so RustyNES gets away with integer dot-lockstep. The SNES
**cannot**: per `ref-docs/research-report.md` §1, a CPU cycle is **6, 8, or 12 master clocks
depending on the address region accessed and the FastROM bit `$420D.0`**, and per
`ref-docs/2026-06-24-ppu.md` §4 the dot and scanline lengths vary. So we model the **master
clock directly** and re-derive every chip's phase from it — a fractional-master-clock
timebase, which is why `docs/adr/0002`'s refactor is designed in from day one rather than
retrofitted.

`tick()` advances the master clock; the CPU bus returns the access *speed* (6/8/12) for each
memory cycle, the scheduler advances that many master ticks, then re-evaluates the PPU dot,
HDMA, and IRQ-timer phases. This is lockstep, not catch-up.

## The memory-access-speed map (master clocks per CPU access)

Per `ref-docs/research-report.md` §1 (sources: Fullsnes, SNESdev Timing, Super Famicom Dev
wiki):

| Address region | Banks | Speed | Cycles | Notes |
|---|---|---|---|---|
| System / WRAM mirror `$0000–$1FFF` | $00–$3F, $80–$BF | Slow | **8** | low 8 KiB WRAM mirror |
| PPU/APU/B-bus I/O `$2100–$21FF` | $00–$3F, $80–$BF | Fast | **6** | PPU1/PPU2 + APU ports |
| Old-style joypad `$4016/$4017` | $00–$3F, $80–$BF | XSlow | **12** | controller serial ports |
| CPU/DMA registers `$4200–$5FFF` | $00–$3F, $80–$BF | Fast | **6** | DMA, NMITIMEN, MEMSEL |
| Expansion `$6000–$7FFF` | $00–$3F, $80–$BF | Slow | **8** | cart-dependent |
| ROM `$8000–$FFFF` (WS1) | $00–$3F | Slow | **8** | always 8 |
| ROM `$8000–$FFFF` (WS2) | $80–$BF, $C0–$FF | MEMSEL | **6 or 8** | 0=8 (Slow), 1=6 (Fast) |
| WRAM `$7E0000–$7FFFFF` | $7E–$7F | Slow | **8** | full 128 KiB work RAM |
| Internal cycle (no bus access) | — | Fast | **6** | always 6 |

Resulting effective CPU frequencies: 6 → **3.58 MHz**, 8 → **2.68 MHz**, 12 → **1.79 MHz**.
`MEMSEL $420D` bit 0: `0 = SlowROM (2.68 MHz)`, `1 = FastROM (3.58 MHz)` for WS2 ROM
($80–$FF).

## The divisor table (the scheduler's core constants)

Per `ref-docs/research-report.md` §1:

| Chip | Advances per | Rate | Notes |
|---|---|---|---|
| Master clock | 1 tick | 21.477270 MHz | the finest practical quantum |
| 65C816 (5A22) | 6 / 8 / 12 master clocks | 3.58 / 2.68 / 1.79 MHz | **variable per access** (map above) |
| PPU dot | **4 master clocks** (nominal) | ~5.37 MHz | with long-dot exceptions (below) |
| SPC700 (S-SMP) | **separate ~1.024 MHz** | ~1.024 MHz | **asynchronous** — own resonator (§async) |
| S-DSP | 24.576 MHz resonator ÷768 | 32000 Hz sample | 1 stereo sample / 768 resonator cycles |

The SPC700 is **not** a master-clock divisor — it is a second clock domain. The two source
frequencies the accumulator math needs are **24,576,000 Hz** (SMP resonator) and
**21,477,272 Hz** (main master).

## Video timing — dots, scanlines, frames

Per `ref-docs/research-report.md` §2 and `ref-docs/2026-06-24-ppu.md` §4:

- **Normal scanline = 1364 master clocks = 341 dots.** The invariant is
  **1364 = 336×4 + 4×5** (SNESdev equivalently counts 340 dots with dots 323 and 327 being
  6 clocks). **Pick one numbering convention and document it** (see "Convention" below).
- **Short scanline = 1360 clocks / 340 dots:** NTSC non-interlace, V=240 of alternate frames.
- **Long scanline = 1368 clocks / 341 dots:** PAL interlace, field=1, V=311.
- **Lines/frame: 262 (NTSC) / 312 (PAL)** non-interlace; +1 interlaced (263/313). Last
  VBlank line: 261 (NTSC) / 311 (PAL). Per-frame master clocks ≈ 357,368 (NTSC) / 425,568
  (PAL).
- **WRAM refresh:** the CPU is **paused for 40 master clocks** beginning ~536 clocks into
  each scanline (DRAM refresh) — model this as a fixed per-line CPU stall.

### Convention (binding)

RustySNES counts **340 dots, numbered `0..=339`**, of which **323 and 327 are 6 master clocks** and
the rest 4: `338 × 4 + 2 × 6 = 1364`. `rustysnes-core`'s `LONG_DOTS`/`dot_length` own the
distribution and `rustysnes-ppu`'s `DOTS_PER_LINE` just counts. Code, comments and `docs/ppu.md`
use this convention exclusively.

**This replaced a uniform `341 × 4` model (T-06-A).** Both reach 1364, which is why the old one kept
perfect frame timing while reporting an `OPHCT` value — dot 340 — that hardware never produces.
fullsnes' *PPU H-Counter-Latch Quantities* histogram is the oracle: sampling `$2137` once per master
clock across a line, dots 323 and 327 latch six times, dot 340 never. bsnes, ares and Mesen2 all
implement it; snes9x uses 322/326 and is the outlier. AccuracySNES `B2.01` is the regression guard.

Both long dots sit at `H ≥ 323`, past the visible window (22-277), past hblank's start at 274 and
past `HDMA_RUN_DOT` (276), so dots `0..=322` kept their exact previous clock alignment — no rendered
pixel and no HDMA transfer moved, and all 50 blessed scenes and both `hdmaen_latch_test` goldens
were unchanged by the switch. The two source descriptions are the same silicon
(`ref-docs/2026-06-24-ppu.md` "Note on a flagged discrepancy").

**Still unmodelled**, and unchanged by this: the short line (NTSC, non-interlace, field 1, V=240 —
1360 clocks, all 340 dots at 4) and the long line (PAL interlace, field 1, V=311 — 1368 clocks, 341
dots, distribution unknown upstream). `B2.02` and `B2.03` remain uncovered.

## DMA / HDMA bus-steal

Per `ref-docs/research-report.md` §5 and `ref-docs/2026-06-24-ppu.md` §5:

- **GP-DMA** (`MDMAEN $420B`): **8 master clocks per byte** (regardless of FastROM),
  +8 cycles / channel, +12–24 cycles whole-transfer alignment. **The CPU is fully halted**
  until all transfers finish; the transfer fires "in the middle of the following
  instruction." Cannot cross a bank. Model as a **CPU stall inserted at the MDMAEN write**.
- **HDMA** (`HDMAEN $420C`): serviced at two dot-accurate points per ares `sfc/cpu/timing.cpp`
  — a once-per-frame **setup** at V=0 (table reset + reload) and a per-visible-line **run** at
  **hcounter 1104 = dot 276** (`HDMA_RUN_DOT`), *not* at the scanline boundary. Costs: **~18
  cycles overhead** when any channel is active, **+8 cycles per active direct channel per
  scanline**, **+8 cycles / byte**; **indirect channels cost 24 cycles** (16-cycle pointer
  load). Worst case ~466 cycles / scanline (8 channels, indirect, 4 bytes). **HDMA preempts
  GP-DMA.** Servicing at the exact dot (rather than the line boundary) is what latches a
  mid-line `$420C` write on the hardware-correct scanline — the banded HDMAEN-vs-latch crossing
  of undisbeliever's `hdmaen_latch_test` (see §H/V-IRQ for the write-timing half of that race).

### Open bus via DMA/HDMA (the "Speedy Gonzales stage 6-1" case) — FIXED

**Status (post-`v1.3.0`): landed.** Cross-checking the exact divergence (below) directly against
ares' AND bsnes' `CPU::Channel::readA`/`readB`/`writeA`/`writeB` (`ref-proj/ares/ares/sfc/cpu/
dma.cpp`, `ref-proj/bsnes/bsnes/sfc/cpu/dma.cpp` — the two files are logically identical, only
differing in template/type-alias style) shows the real rule is **DMA/HDMA reads update the
open-bus latch; DMA/HDMA writes never do** — a third combination distinct from both previously-
tried prototypes ("update on all four A/B read/write methods" and "update on the B-bus side
only"), and for a straight-copy DMA channel (this investigation's exact failing case) it happens
to predict the identical accumulated value either of those two already did, since a straight
copy's read-value always equals its write-value. This is strong, direct, independent confirmation
that the "regression" was actually a correction: **`DmaBus for Bus`'s `read_a`/`read_b` now update
`open_bus`; `write_a`/`write_b` do not.** `read_a`'s A-bus "forbidden range" branch (CPU/DMA-I/O-
register-shadowed addresses) also now matches ares/bsnes exactly — it sets `open_bus` to a hard
`0` (not "leave it unchanged", the prior behavior) — though this branch has zero coverage in the
current corpus either way (confirmed by the earlier investigation pass), so this piece is
unverified-by-test, only by direct source citation.

**The independent-oracle citation, precisely.** `CPU::Channel::readA`: `cpu.r.mdr = validA(address)
? bus.read(address, cpu.r.mdr) : (n8)0x00;` — reads always update `mdr` (this project's
`open_bus`), on both the mapped and forbidden-range paths. `writeA`/`writeB`: `bus.write(...)`
only, `mdr` is never touched. `Bus::read`'s own default (unmapped) reader —
`reader[0] = [](n24, n8 data) -> n8 { return data; };` — is the open-bus echo mechanism itself: an
unmapped read returns exactly the fallback byte it was handed, i.e. whatever was last legitimately
read/written onto the bus. This is the same "any device driving the bus updates what a later
unmapped read observes" principle the SNESdev wiki / mgba blog describe at the mechanism level,
now confirmed at the exact-value level for DMA specifically.

**Regression gate.** `cargo test --workspace` (zero regressions, unaffected — this only touches
`DmaBus for Bus`, exercised solely by DMA/HDMA-driven accesses) and the full `--features
test-roms` battery (28 tests, 17 suites): 27 of 28 unaffected; `superfx_boots_live_and_deterministic`
re-blessed (`BLESS_SUPERFX=1`) — all 24 golden hashes changed (expected: every Krom GSU ROM shares
the same boot/fade library routine, `tests/roms/external/krom/LIB/SNES_GSU.INC`, that exercises
this exact overrun), while every OTHER assertion in that same test — cart/coprocessor detection,
GSU liveness (`accesses > 0`), the FillPoly plot-bitmap threshold, and cross-run determinism — is
unaffected and still passes, both before and after re-blessing. `cargo clippy --workspace
--all-targets`, `cargo fmt --all --check`, the `no_std` gate, and `RUSTDOCFLAGS="-D warnings"
cargo doc --workspace --no-deps` all clean.

**A real, independent fix landed along the way: `SuperFxBoard::map`'s RAM-ownership gap.**
`SuperFxBoard::map(addr24)`'s `Region::Ram` arm unconditionally returned `MappedAddr::Sram`, even
while the GSU owned Game Pak RAM (`gsu.owns_ram() == true`) — so `Cart::read24`'s generic
open-bus fallback (the same mechanism the SPC7110 investigation added,
`docs/audit/spc7110-boot-crash-2026-07-08.md`) never triggered for this case, and
`SuperFxBoard::read24`'s own hardcoded `return 0;` always won regardless of the bus's actual
last-driven byte — wrong for real open bus the same way every other bare-`0` fallback in this
codebase was wrong before the SPC7110 fix. **Fix:** `map()` now returns `MappedAddr::Open` for
`Region::Ram` specifically when `gsu.owns_ram()` is true, letting the existing generic fallback
thread the real `open_bus` value through; `SuperFxBoard::read24`'s own hardcoded-0 arm is
unreachable through the normal `Cart::read24`-mediated path now (kept only as a sane standalone
default for direct `Board::read24` callers, e.g. this module's own unit tests). Verified
independently: **zero regressions** across the full `--features test-roms` battery (all 27
suites, including `superfx_oncart` itself) with this fix alone, before the DMA-open-bus prototype
is applied on top. Writes are unaffected (`Cart::write24` never consults `map()`, so
`write24`'s own Ram arm — which posts unconditionally, even while the GSU owns RAM — is
untouched).

**Also landed this pass, prerequisite tooling:** the `debug-hooks` watchpoint hook
(`Bus::note_bus_access`) now fires on `DmaBus`'s A-bus/B-bus accesses too, not just
`CpuBus::read24`/`write24` — DMA/HDMA-driven accesses were previously completely invisible to the
watchpoint tooling, which is exactly the instrumentation the investigation below needed.

The open-bus latch (`Bus::open_bus`, the MDR) is a physical property of the SNES's shared data
bus, not a CPU-only concept: **any** device that drives a byte onto the bus updates what a later
unmapped read observes, whether that device is the CPU or the DMA/HDMA controller. Currently
`open_bus` is updated only by direct CPU bus accesses (`CpuBus::read24`/`write24`); a DMA/HDMA
byte transfer never touches it — silently wrong versus real hardware.

This is the documented mechanism behind Speedy Gonzales in Los Gatos Bandidos' stage 6-1 softlock
(a "Holy Grail" emulation bug, unsolved by any emulator until 2010): the game polls an unmapped
address (`$225C`) in a tight loop, expecting to read back `$18` — the last byte of the CPU's own
load instruction, echoed twice (low then high byte) to form the address `$1818`. If an HDMA
transfer fires between the loop's instruction fetch and its execution, the HDMA-driven byte
re-latches the bus first; when that byte happens to be `0`, the loop reads back `$0000` instead
of `$1818`, which satisfies the exit condition and breaks the softlock. Sources: the SNESdev
wiki's [Open bus](https://snes.nesdev.org/wiki/Open_bus) page and mGBA's ["Holy Grail" Bugs in
Emulation, Part 2](https://mgba.io/2017/07/31/holy-grail-bugs-2/) (byuu/Near's original
writeup of the fix, "Adding OpenBus (the MDR) writing to S9xSetPPU, which didn't have it before,
and is used in HDMA, fixes the game").

**The prototype and what it broke.** Making `DmaBus for Bus`'s `read_a`/`write_a`/`read_b`/
`write_b` set `open_bus` on every successful transfer (mirroring `CpuBus::read24`/`write24`
exactly, with the blocked-address branches — the A-bus cannot reach the B-bus or the CPU/DMA I/O
registers, ares `validA` — deliberately left untouched since nothing is actually driven onto the
bus there) passed the full base workspace suite (`cargo test --workspace`, zero regressions) but
broke every one of `superfx_boots_live_and_deterministic`'s 24 golden hashes
(`crates/rustysnes-test-harness/tests/superfx_oncart.rs`) — confirmed to be caused by this exact
change (the same test passes cleanly on the unmodified tree). This still reproduces byte-for-byte
identically **after** the `SuperFxBoard::map` RAM-ownership fix above (the two are not the same
bug — fixing the RAM-ownership gap has zero measurable effect on this regression's output hashes).

**What was ruled out this pass** (each via direct instrumentation, not inference — a temporary
`Bus::note_bus_access`-adjacent trace + a runtime toggle field let both the unmodified and
prototype-patched code paths run in the same test binary and be diffed instruction-for-instruction;
the instrumentation itself was not landed, per this project's "build it, use it, delete it"
diagnostic discipline):

- **The `$4016`/`$4017` joypad-read open-bus blend** (`read_cpu_reg`'s `(self.open_bus & mask) |
  ...` arms) — only 1 total access across a 30-frame run of the smallest failing ROM
  (`GSU2BPP256x128PlotPixel.sfc`), no divergence at that single access. Not the mechanism.
- **The generic open-bus-fallback arms** in `b_read`/`read_cpu_reg` (`_ => self.open_bus`) — zero
  hits across the same 30-frame run. Not the mechanism.
- **`CartView`/`VideoBus::cart_read`** (the PPU's cart-mediated read hook, which also threads
  `open_bus` through) — confirmed dead code: `cart_read` has no call site anywhere in
  `rustysnes-ppu`'s actual rendering path (`render.rs`) today. Not the mechanism.
- **The A-bus "forbidden range" short-circuit** in `read_a` (`return self.open_bus` for
  CPU/DMA-I/O-register-shadowed addresses) — zero hits across the full 58-ROM Krom GSU corpus.
  Not the mechanism.

**The exact divergence, isolated this pass.** A per-instruction trace (PC + every byte each
instruction's bus accesses — CPU and DMA/HDMA alike — read or wrote, tagged with a monotonic
instruction index) comparing the unmodified tree against the DMA-open-bus prototype step-by-step
from reset — exactly the finer, `docs/audit/spc7110-boot-crash-2026-07-08.md`-style tool the prior
pass's own "concrete next step" called for, built as temporary `Bus`-internal instrumentation
(a runtime trace-log + a runtime prototype toggle so both code paths run in the same test binary,
deleted again before this update landed, per this project's "build it, use it, delete it"
discipline) — found the first byte-for-byte divergence at **CPU instruction #5873** of
`GSU2BPP256x128PlotPixel.sfc`'s boot, reproducibly, in well under a second once compiled:

- The diverging instruction is `STA $420B` at `$7E:8254` (opcode `$8D`), which sets `MDMAEN = $03`
  — arming GP-DMA channels 0 and 1. Both trees fetch and execute this identically; the divergence
  is not in CPU control flow at all, it is *inside the DMA burst this one instruction triggers*
  (146,215 total bus accesses charged to this single instruction, since our scheduler — correctly,
  matching real hardware's channel-0-then-channel-1 sequential GP-DMA servicing — runs each
  enabled channel to completion before starting the next).
- Channel 0 (source bank `$70`, Game Pak RAM → dest `$2118`/`$2119`, VMDATAL/VMDATAH) transfers the
  GSU's plotted bitmap into VRAM; this half is byte-identical between the two trees (real cart data
  is unaffected by the open-bus latch's internal bookkeeping).
- Channel 1 is a fixed-target-DMA screen-fade idiom: source = an incrementing WRAM buffer (bank
  `$00`, starting near `$1FFD`), dest = a *fixed* `$2100` (`INIDISP` — display brightness/forced
  blank), i.e. this channel deliberately drives a gradient table into INIDISP once per transferred
  byte for a fade effect. **The WRAM gradient table is shorter than the configured transfer count**:
  at access index 92976 within the burst, channel 1's source address crosses from `$00:1FFF` (the
  last real WRAM low-mirror byte) into `$00:2000` — a genuinely unmapped address in real hardware's
  own memory map (not `$2100-$213F` PPU registers, not `$2140-$217F` APU ports, not the cart's ROM/
  RAM window at all) — and keeps incrementing through it for the rest of the transfer. This overrun
  past the buffer's real content, reading open bus for the remainder of the fade, is very likely
  intentional/tolerated in the source ROM (`INIDISP` only meaningfully uses 5 bits, so garbage tail
  bytes barely perturb the fade visually) rather than a bug in this project's own DMA channel/count
  decode — the two trees agree on every real byte transferred both before and after this point;
  only the *open-bus-fallback* byte at `$00:2000` (and everything after it, once contaminated)
  differs.
- The unmodified tree returns `$03` there (`Cart::read24`'s generic open-bus fallback echoing
  whatever `open_bus` was before this whole GP-DMA burst started — the `STA`'s own operand/value,
  stale for the burst's entire duration). The prototype returns `$00` — the value the fallback
  echoes back is whatever `open_bus` was most recently set to *by an earlier byte in this same
  burst* (channel 1's own prior WRAM→`$2100` transfers), which is exactly the documented mechanism
  working as intended. This is what corrupts (relative to the currently-blessed goldens) the tail
  of the fade sequence and, through it, VRAM/framebuffer content — explaining why all 24
  `superfx_oncart` goldens move together: every ROM in the Krom GSU corpus shares this same
  boot/fade library routine (`tests/roms/external/krom/LIB/SNES_GSU.INC`), so all 24 hit the
  identical pattern.

**A second candidate fix was tried this pass and also fails, for a structural reason.** Since the
real Speedy-Gonzales fix byuu/Near describes is specifically about *HDMA writes* re-latching the
bus (`S9xSetPPU`, a register-write path), a narrower prototype was tried: update `open_bus` only on
the B-bus side (`read_b`/`write_b`), leaving A-bus DMA reads/writes (`read_a`/`write_a`, which cover
WRAM and cart-space DMA transfers) untouched. **This reproduces the identical divergence at the
identical instruction/access index.** The reason: channel 1's own fixed-target writes to `$2100`
(the B-bus side) already update `open_bus` on every transferred byte *before* the burst reaches the
`$00:2000` overrun — so by the time a later A-bus read in the *same* burst hits the open-bus
fallback, the latch has already been contaminated by this burst's own prior B-bus writes, with or
without the A-bus side also updating it. There is no way to scope the fix by bus direction alone;
any policy that makes *any* DMA-driven byte within this burst update `open_bus` changes what the
burst's own later open-bus read observes, because the write and the read that observes it are both
inside the one continuous transfer the CPU triggered.

**What actually broke the tie between the two failed shapes and the fix.** Both previously-tried
shapes (full DMA-bus update, B-bus-only update) update `open_bus` on some write path, which this
investigation's own trace already showed is unnecessary for this specific channel (a straight
copy makes read-value and write-value identical, so the *numeric outcome* was never the problem —
both already matched what real hardware would show at this exact access). The missing piece
was independent confirmation, not a different formula: reading ares'/bsnes' actual `dma.cpp`
directly (rather than reasoning from the mechanism-level SNESdev wiki / mgba blog sources alone)
supplied the specific-value oracle this investigation lacked, converging on "reads update, writes
don't" as the precisely-scoped, hardware-matching rule — which was implemented, verified
regression-clean, and the 24 dependent goldens re-blessed with this citation trail as their
justification (`docs/adr/0003`'s honesty-gate posture: a golden change is fine once cross-checked
against an independent reference, never blessed blind).

(b), the byte-level GP-DMA channel-arbitration-order question, was not separately re-verified this
pass — it turned out not to matter for resolving *this* divergence, since the fix's correctness
follows from the per-transfer read/write open-bus rule alone, independent of channel ordering. It
remains a standing, lower-priority question if a *future* two-simultaneous-channel timing
discrepancy is ever found.

This precise, content-dependent cycle theft is the second reason (after the variable CPU
cycle) the scheduler must be master-clock resolution.

### The "DMA/HDMA-collision crash quirk" — researched, reclassified as a non-goal

**Status: researched against the primary source; the umbrella label bundles three distinct real
behaviors, none of which should be modeled as a hard emulated crash. One sub-case (A-bus address
restrictions) is already correctly implemented. Not an open implementation item.**

Per the SNESdev wiki's [Errata](https://snes.nesdev.org/wiki/Errata) page (§S-CPU (5A22) → DMA),
three distinct DMA/HDMA-interaction defects are documented, and the vague "collision crash
quirk" label this project's own planning docs used bundles all of them under one name:

1. **Version-1 5A22 ("S-CPU") crash: "the chip can crash if DMA finishes right before HDMA
   happens."** A genuine silicon defect specific to the FIRST hardware revision only (later
   `S-CPU-A`, `S-CPU-B`, and the 1-CHIP SNES are unaffected). The errata page frames this as a
   pitfall for *game developers* to avoid ("generally only a problem for games that want to use
   DMA to clear WRAM or copy data from a coprocessor to WRAM... during rendering"), not a
   behavior players or emulators need to reproduce — a compliant commercial ROM is written to
   never trigger it. No mainstream accurate emulator (ares/bsnes/Mesen2 — confirmed by reading
   `ref-proj/ares/ares/sfc/cpu/timing.cpp`'s `dmaEdge()`, which implements only the well-defined
   general HDMA-preempts-DMA *priority* ordering, not a crash) treats this as anything but
   undefined/out-of-scope, matching this project's own precedent for the `$4203`/`$4206`
   overlapping-multiply case (`MulDiv`'s doc comment, `crates/rustysnes-core/src/bus.rs`):
   fabricating a specific "crash" behavior for a chip-revision-specific defect no compliant ROM
   is meant to hit would manufacture behavior real hardware doesn't universally define, violating
   `docs/adr/0004`'s determinism-contract spirit.
2. **Version-2 5A22 ("S-CPU-A") DMA-fails-silently bug**: "a recent HDMA transfer to/from
   INIDISP (meaning `BBADn` is set to `$00`) can make a DMA transfer fail" — the transfer
   silently does nothing, leaving `DASnL`/`DASnH` unchanged instead of zeroed. Also
   version-specific (does NOT affect rev-1, `S-CPU-B`, or the 1-CHIP SNES) and requires an
   unusual configuration (an HDMA channel targeting `INIDISP`, `$2100`, a display-control
   register no ordinary graphics/audio HDMA use case would pick) that has a documented
   developer-side workaround (`BBADn = $ff` + transfer pattern 1). No known commercial title or
   committed test ROM in this project's corpus exercises it.
3. **Version-agnostic silent HDMA failure**: "HDMA can fail if a DMA transfer ends when HDMA
   starts (just after the start of scanline 0) and the previous value read by DMA is `0`" — when
   this triggers, "the HDMA channel stops at the start of scanline 0 and there are no H-Blank
   transfers for an entire frame." Unlike the two chip-revision-specific items above, the errata
   page does not scope this one to a particular 5A22 version, and it is not a crash — a real,
   well-defined-enough silent misbehavior that in principle COULD be modeled. It is not landed
   here because: it requires a coincidental data condition (the last DMA-read byte happens to be
   exactly `$00` at the exact scanline-0 boundary) with no known commercial game or committed
   test ROM that depends on it either triggering or being absent, so there is no oracle to verify
   an implementation against — and the sibling open-bus-via-HDMA-latch investigation just
   demonstrated (above) that this exact class of change (touching DMA/HDMA/open-bus interaction
   inside `Bus::advance_master`) carries real, non-obvious regression risk to currently-passing
   golden suites even when the documented mechanism is correct. If a future test ROM or
   commercial title is found to depend on this, it becomes a concrete, verifiable ticket; until
   then it stays documented, not implemented, per this project's own regression-risk discipline.

**Already correctly implemented, not part of this non-goal**: the errata page's fourth DMA item —
A-bus addresses that cannot reach the B-bus, the CPU/DMA I/O registers, or (for `WMDATA`) another
WRAM location — is already modeled in `DmaBus for Bus`'s `read_a`/`write_a` blocked-address
branches (`crates/rustysnes-core/src/bus.rs`, referenced in the "Open bus via DMA/HDMA" section
above) and in the general **"HDMA preempts GP-DMA"** priority ordering (`run_gp`'s
`service_hdma_during_gp` calls, `crates/rustysnes-core/src/dma.rs`) — the well-defined half of
what "collision" could have meant is not a gap at all.

## H/V-IRQ and NMI

The 5A22 raises NMI at VBlank start (V=225, or V=240 in overscan) and an IRQ at a programmed
H and/or V counter position (`$4207–$420A`), enabling mid-frame raster effects
(`ref-docs/research-report.md` §2). The H/V counters are latched by reading SLHV `$2137` and
read back from `$213C`/`$213D`. These fire off the master-clock phase, not the CPU cycle.

The horizontal comparator asserts the IRQ **`HIRQ_TRIGGER_DELAY` (4) dots after** the programmed
`HTIME`, modelling the hardware communication delay between the counter unit and the CPU's
interrupt logic (ares `sfc/cpu/irq.cpp`: `hcounter(10) == io.htime` with `io.htime` stored as
`(HTIME+1) << 2` clocks ⇒ the IRQ fires at hcounter `HTIME*4 + 14` = dot `HTIME + 3.5`). This
delay lands an IRQ-gated register write (e.g. `hdmaen_latch_test`'s `STA $420C` after `WAI`) on
the hardware-correct dot; without it the write drifts ~3–4 dots early and — against the fixed
dot-1104 HDMA latch — collapses the test's banded crossing into a uniform per-line alternation.

**V-only IRQ (`$4200` bit 5 without bit 4) is sampled at one dot, not held across the line.**
The comparator fires at `V = VTIME, H = VIRQ_TRIGGER_DOT (2)` — the dossier's documented `H ~ 2.5`
rounded to the nearest whole dot. Modelling the horizontal half as unconditionally true when H-IRQ
is disabled made `V == VTIME` a *level* that re-raised the IRQ on all 341 dots of the target line,
so acknowledging via `$4211` was undone a few dots later and a V-only handler saw a storm rather
than one interrupt per frame. ares reaches the same place from the other direction: its
`irqValid.raise(...)` (`sfc/cpu/irq.cpp:26-30`) is an *edge* detector, so a level condition raises
once. Found by AccuracySNES **B4.12**; **B4.08** pins the firing line.

> **Golden re-bless, this change.** `hdmaen_latch_test` and `hdmaen_latch_test_2` moved
> (`0x47870388220f3725` → `0x60dd903f56753725`, `0xdce49c12e5402f25` → `0x1a189dc89e5f4525`) and
> were deliberately re-blessed. Both ROMs gate their `STA $420C` on a V-only IRQ, so firing once
> per frame instead of on every dot of the line changes which dot the write lands on and therefore
> the banding realization. That is legitimate here *only* because these goldens are regression
> snapshots of our own deterministic output — see the note below — and because the change is
> corroborated externally (ares' edge detector; Mesen2 and snes9x both pass B4.08/B4.12, which
> RustySNES failed before the fix). A golden that tracked an external oracle would mean the
> opposite: that the change was wrong.

---

> **On `hdmaen_latch_test` (ROM 1) determinism.** undisbeliever documents `hdmaen_latch_test.sfc`
> as *not a stable test* — its exact bands differ on every power-cycle on real hardware, because
> the HDMAEN-write-vs-latch race turns on the sub-cycle CPU/DMA phase at power-on. RustySNES is
> deterministic (seed+ROM ⇒ fixed output), so it produces one fixed realization of the banding;
> the committed golden is a regression snapshot of *that*, not a byte-match to any other emulator.
> What is spec-accurate and portable is the **mechanism**: dot-1104 HDMA latch + `HTIME+3.5` IRQ
> assertion ⇒ the write-drift straddles the latch ⇒ a banded crossing rather than flat alternation.

## SA-1 — the second CPU (Phase 4)

The SA-1 coprocessor is a **second WDC 65C816** clocked at master / 2 (~10.74 MHz), so each SA-1
CPU cycle is **2 master clocks**. The crate graph forbids `rustysnes-cart` from depending on
`rustysnes-cpu`, so the SA-1 *system* state lives in `coproc::sa1::Sa1Board` (the cart) while the
scheduler (`rustysnes-core`) owns the second `rustysnes_cpu::Cpu` and drives it through the
`Board` second-CPU hooks (`docs/cart.md` §SA-1).

**The stepping model is deterministic catch-up, not a free-running thread.** `System` holds an
optional `sa1_cpu`, instantiated in `reset()` iff the installed cart `has_second_cpu()`. After every
main-CPU instruction (and the HDMA/DMA bus-steals inside the run loop), `run_sa1` measures the
master clock the *untouched* main CPU has elapsed since the last call and converts it to an SA-1
cycle budget (`Δmaster / 2`). It then steps the second CPU — against a thin `Sa1Bus` adapter that
routes `read24`/`write24` to `Board::second_cpu_{read,write}` — until the budget is spent, charging
each instruction's returned cycle count (×2) to the SA-1 H/V timer via `second_cpu_tick`. Because
the budget is a pure function of `bus.clock.master` (which is a pure function of the deterministic
main CPU), **installing and stepping the second CPU never perturbs the main CPU's behaviour or the
existing scheduler timing** — the `cpu_oracle` stays bit-identical, and the SA-1 only runs for SA-1
carts (gated by `sa1_cpu.is_some()`).

**Control + interrupts.** The SA-1 powers up held in reset (RESB). When the S-CPU clears RESB the
board latches a reset edge that `run_sa1` consumes to reset the second CPU (its reset/NMI/IRQ vector
fetches are redirected inside the board to the SA-1's own CRV/CNV/CIV vectors). While the SA-1 is
held in reset or asleep (`second_cpu_running()` is false) the budget drains into the timer only.
The SA-1→S-CPU IRQ is `Board::irq_pending()`, ORed into the main bus IRQ line in `poll_irq`; the
S-CPU→SA-1 IRQ/NMI feed the second CPU's `poll_irq`/`poll_nmi`. The SA-1 timing is approximate
catch-up (not sub-instruction lockstep with the main CPU's bus accesses), which is exact for the
register/arithmetic/DMA results games observe and keeps the contract fully deterministic.

## The SPC700 async resync (the accuracy crux)

Per `ref-docs/2026-06-24-apu.md` §2: the SPC700 / S-DSP run on their own ~1.024 MHz timebase
(24.576 MHz resonator). RustySNES tracks "how far ahead is the CPU vs the SMP" with a single
**signed integer relative-time accumulator**: when the CPU steps N of its clocks, subtract
N × 24,576,000; when the SMP steps N, add N × 21,477,272 (or the equivalent reduced rational
ratio). No floating point, so the counter is exact. The bus resyncs the SMP up to "now":

1. on **every CPU access to `$2140–$2143`** (and SMP access to `$F4–$F7`), and
2. **once per scanline** (to bound audio latency).

Between syncs the SMP may run arbitrarily far ahead as long as neither side touches the
ports. This is the higan/bsnes cooperative-threaded technique (`docs/adr/0001`,
`ref-docs/research-report.md` §3) implemented single-threaded so save-states / netplay stay
bit-deterministic. Resonator drift is **deliberately not modeled** in the deterministic core
(see `docs/adr/0004`).

### Implementation (Phase 3 — T-31-003)

The accumulator lives in `Bus::Clock::spc_accum` (a `u64`) and is stepped inside
`Bus::advance_master` — the same per-master-tick loop that drives the PPU dot clock — so the SMP
advances in **true lockstep**, not catch-up:

```text
spc_accum += SPC_NUM;                       // SPC_NUM = 68_352
while spc_accum >= SPC_DEN {                 // SPC_DEN = 715_909
    spc_accum -= SPC_DEN;
    apu.advance_smp_cycle();                 // release one SMP *base* clock
}
```

`68_352 / 715_909` is the exact rational `(apuFrequency / 12) / 21_477_270` reduced by gcd = 30,
where `apuFrequency = 32040 × 768 = 24_606_720` Hz (ares) and the SMP runs at `apuFrequency / 12 ≈
2.05 MHz` (a normal SMP access = `SMP_WAIT` = 2 base clocks → the ~1.025 MHz effective opcode rate;
`docs/apu.md`). Integer-only, so the SPC domain is bit-deterministic (`docs/adr/0004`).

Because the SMP is advanced at master-clock granularity, by the time the CPU reads `$2140-$2143`
the SMP has already been clocked up to that exact master instant — `Bus::b_read`/`b_write` route
those four ports straight through `Apu::cpu_read_port` / `Apu::cpu_write_port` (the dead
`apu_ports` latch array is gone). The "forced per-scanline sync" the model above describes is
therefore **subsumed by the continuous lockstep** (the SMP is never arbitrarily ahead), which is
stronger than the latency-bounded on-demand sync and stays fully deterministic.

**Cycle-exact SMP step (T-31-004):** `advance_smp_cycle` now releases **exactly one SMP base clock
per call** by draining a recorded micro-op timeline of the in-flight instruction (one entry per
SPC700 bus access), committing each SMP→CPU port write at the precise base cycle its access
completes — rather than running a whole instruction at the budget boundary. So a CPU read of
`$2140-$2143` observes the SMP exactly up to that master instant and no further, the cooperative-
thread interleaving achieved single-threaded (full derivation in `docs/apu.md` §cycle-exact). This
got all four blargg `spc_*` ROMs to **stream their result text** (decoded + asserted by
`tests/blargg_spc.rs`). The **timer-phase fix** (T-31-006) then drove `spc_smp` / `spc_timer` /
`spc_mem_access_times` to blargg's **literal PASS** — the residual was the recording bus clocking the
SPC700 timer *after* the write side effect instead of before (ares/Mesen2 clock it first), not a
CPU-leading-vs-symmetric clock-model asymmetry as earlier believed (`docs/apu.md` §timer phase).
`spc_dsp6` remains Failed 02 on a separate S-DSP echo/envelope residual.

## Test plan

- The variable-cycle map: verify against the SingleStepTests/65816 per-cycle bus traces
  (each opcode JSON carries cycle-by-cycle bus activity).
- DMA/HDMA timing: undisbeliever/snes-test-roms HDMA-timing and mid-frame ROMs; the cycle
  budget must match within the test's tolerance.
- The SPC resync: blargg `spc_mem_access_times` + the IPL-boot handshake; gilyon SPC tables.
- Scanline-length variants: a deterministic golden framebuffer for a known ROM at each region.

## Implementation status (Phase 2)

The scheduler lives in `rustysnes-core` as the `Bus` (the master-clock phase + memory decode +
DMA/HDMA) plus the `System` run loop (`scheduler.rs`):

- **The clock is CPU-driven.** Each `CpuBus::read24`/`write24` stashes the region access speed
  (`Bus::access_speed`, the ares `CPU::wait` map above), and the following `on_cpu_cycle` advances
  the master clock by it — internal CPU cycles default to 6. `advance_master` steps the PPU dot
  clock (4 master/dot) and the SPC accumulator in-line, so it is true lockstep. A steady-state
  booted NTSC frame measures 357,368 master clocks on average (spec 357,368 exactly), within
  ±20-40 clocks of natural instruction-boundary quantization noise per individual frame — measured
  across 500 frames × 3 unrelated ROMs, `v1.1.0`; see §DRAM refresh for the full methodology and
  why this rules out an additive refresh stall.
- **DMA/HDMA** is `dma.rs` (clean-room from ares `dma.cpp`): GP-DMA halts the CPU and charges
  `8`/byte; HDMA runs per visible scanline with the per-mode lengths `{1,2,2,4,4,4,2,4}`, indirect
  pointers, and the line counter. `Bus::advance_master` fires HDMA's per-frame setup at V=0 and
  its per-visible-line run at the hardware-correct **dot 276** (`HDMA_RUN_DOT`, hcounter 1104 —
  not the scanline boundary), matching ares `sfc/cpu/timing.cpp`; this dot-accurate phase is what
  latches a mid-line `$420C` write on the correct scanline (§DMA/HDMA bus-steal above), proven by
  the committed `hdmaen_latch_test`/`hdmaen_latch_test_2` goldens. **Sub-tick precision, `v0.8.0`:**
  the run-check must observe the exact master-clock sub-tick whose *pre-tick* dot value is 276
  (the sub-tick that advances the counter *from* 276 to 277), not merely "the dot currently reads
  276" (`self.ppu.dot()` read *after* `tick_ppu_dot()` had already incremented it matched the dot
  a whole 4-master-clock window early) — see `docs/ppu.md` §Mid-scanline/HDMA-driven register
  timing for the fix and the goldens it required re-blessing (`hdmaen_latch_test`/
  `hdmaen_latch_test_2` among them).
- **NMI / IRQ:** the RDNMI (`$4210`) VBlank flag sets at VBlank **regardless** of the NMITIMEN
  enable (so VBlank-poll loops like gilyon's work); the NMI *interrupt* and the H/V-IRQ comparator
  (pushed to the PPU each dot) fire only when enabled. Both flag registers return **open bus** (the
  CPU MDR, `self.open_bus` — the pre-read last-driven value) in the bits hardware leaves floating:
  `$4210` bit 7 = the read-clearing VBlank flag, bits 4-6 = open bus, bits 0-3 = CPU version 2;
  `$4211` (TIMEUP) bit 7 = the read-clearing IRQ flag, bits 0-6 = open bus. A ROM that reads the
  whole byte instead of masking the flag therefore sees the last bus value in those positions, as on
  hardware (ares `CPU::readIO`, fullsnes). Tier-1 remediation T-CA-02 (`to-dos/TIER1-CYCLE-ACCURACY.md`).
- **Automatic joypad read (`$4200` bit 0):** a *timed* ~4224-master-clock operation, not an instant
  latch. At vblank entry (while armed) the controller state is snapshotted; `$4212` bit 0 reads
  **busy** for the next `AUTO_JOYPAD_CLOCKS` = 33 x 128 = 4224 clocks (ares `status.autoJoypadCounter`
  as a master-clock deadline), and the result publishes to `$4218-$421F` only at completion — a read
  during the window still holds the previous frame's value. `$4212` also returns open bus in bits
  1-5. The busy deadline self-settles each dot, and the in-flight snapshot + deadline **are** in the
  save state (`FORMAT_VERSION` 5, `docs/adr/0006`), so a save taken mid-window restores an identical
  machine state. Tier-1 remediation T-CA-01/03.
- **Deferred refinements** (no committed ROM depends on them yet): the 40-clock DRAM-refresh CPU
  stall (researched, not yet implemented — see §DRAM refresh above) and the PAL-frame
  master-clock cycle-check.

### DRAM refresh — empirically measured, NOT implemented (`v1.1.0` conclusion: adding it would regress)

**Status: the mechanism is well-understood (ares' model, below), but this pass's empirical
measurement — which the original `v0.5.0` note explicitly required *before* implementing the
stall — shows there is no gap for a 40-clocks/scanline stall to fill. Implementing it as
literally described would introduce a genuine, large, wrong regression against the
now-confirmed-correct current baseline. Deliberately not implemented.**

ares' model (`sfc/cpu/timing.cpp`/`cpu.hpp`): `CPU::step` checks `hcounter() >=
dramRefreshPosition` on every tick; the first time it trips each scanline it runs 5 iterations of
`step(6)` (refresh active) + `step(2)` (refresh inactive) + `aluEdge()` — 40 clocks total.
`dramRefreshPosition` is recomputed at the start of each scanline as `530 + 8 - dmaCounter()`
(`io.version == 2`, the revision essentially every commercial cart uses; `dmaCounter()` is
`counter.cpu & 7`, a phase-alignment term averaging out where in the CPU's own 8-clock DMA-divider
cycle the scanline happened to start).

**The architectural complication, not present in ares': the master clock here is CPU-driven**
(`Bus::advance_master` only advances because a `CpuBus` access charged it some cost — see "The
clock is CPU-driven" above), whereas real hardware's PPU dot/scanline timing is generated by an
**independent, fixed-rate** clock: a booted frame is *always* exactly 357,368 (NTSC) master clocks
long regardless of what the CPU does. On real hardware, DRAM refresh doesn't add clocks to the
frame — it's the CPU losing 40 clocks *out of* the frame's already-fixed budget to a stall, exactly
like a slow ROM access. The naive port of this (charge an unconditional `advance_master(40)` once
per scanline, mirroring how HDMA already charges its own per-line cost) assumed the current
CPU-driven total *undershoots* 357,368 by roughly that much, and that adding the stall would close
the gap.

**The empirical measurement (this pass) shows the opposite: there is no gap to close.** A
throwaway diagnostic (`cargo test`-only, not committed) ran `System::run_frame()` for 500
steady-state frames (past the first, DMA-heavy boot frame) across three independent, unrelated
test ROMs (`tests/roms/gilyon/cputest/cputest-basic.sfc`, `cputest-full.sfc`, and
`tests/roms/gilyon/spctest/spctest.sfc`), recording `Clock::master`'s per-frame delta against the
357,368-clock NTSC spec:

| ROM | avg gap (clocks/frame) | min | max |
|---|---|---|---|
| `cputest-basic.sfc` | −0.056 | −36 | +26 |
| `cputest-full.sfc` | −0.056 | −24 | +22 |
| `spctest.sfc` | +0.004 | −40 | +32 |

The average gap across all three is **within a fraction of a single clock of exactly zero** — the
current CPU-driven model already reproduces the fixed 357,368-clock NTSC frame length to within
natural instruction-boundary quantization noise (±20-40 clocks, the size of whichever single
instruction happens to straddle the frame/VBlank boundary at the moment `run_frame()`'s internal
check fires — expected and benign, not a timing bug, and it averages to ~0 over many frames exactly
as quantization noise should). This directly contradicts the `≈357,374` figure this document
previously cited as the working assumption — that number was very likely a single-frame (or very
small sample) measurement that happened to land on the high side of this same ±20-40 clock noise
band, not a persistent bias.

**Conclusion: implementing the naive 40-clocks/scanline stall now would be wrong.** Charging an
additional, unconditional `advance_master(40)` once per scanline on top of a model that already
averages to the correct 357,368-clock total would inflate every frame by ~40×262≈10,480 clocks —
a large, obviously-wrong overshoot against the now-empirically-confirmed-correct baseline, not a
missing-clocks fix. This means one of two things is true, and distinguishing them is real future
work, not assumed here: either (a) the existing per-opcode/per-access cost table (`Bus::access_speed`,
ported from ares' own `CPU::wait` map) already implicitly absorbs DRAM refresh's real-hardware
effect through however its costs were originally calibrated, and no further change is needed at
all; or (b) refresh really does need to be modeled explicitly, but only by *reallocating* an
equivalent ~40 clocks/scanline out of the existing cost table (finding and trimming ~0.15
clocks/access somewhere across the ~262 accesses/scanline a typical frame makes) rather than
*adding* a new stall on top — a materially harder, more invasive change than the original
"just add a stall" plan assumed, and not undertaken in this pass given the empirical result
removes the urgency (the current model is already correct at the whole-frame-length level).

**For whoever revisits this:** re-run this same measurement methodology (steady-state
`run_frame()` deltas averaged over hundreds of frames, multiple unrelated ROMs, outlier-filtered
to exclude boot-frame DMA spikes) before ever re-attempting the stall — it is cheap, decisive, and
this pass's result should be treated as the current baseline to compare against, not re-derived
from scratch. If a future change to the per-opcode cost model (e.g. closing the CPU oracle's one
remaining `e1.e` residual, `docs/STATUS.md`) shifts this average measurably away from zero, that
would be the actual evidence needed to justify option (b) above.

## Open questions

- Exact per-opcode master-clock breakdown for rarer addressing modes — a verify-against-the-
  oracle item, gated on securing the 65816 JSON license (`ref-docs/research-report.md`
  "Open questions" #1; `docs/testing-strategy.md` §licensing).
