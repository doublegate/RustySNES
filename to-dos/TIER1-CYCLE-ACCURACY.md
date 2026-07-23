# Tier-1 Cycle-Accuracy Remediation Program

**Goal:** remediate every **Tier-1** (ROM-observable) cycle-accuracy gap catalogued in
`docs/accuracy-ledger.md` + the dossier §Part IX, bringing RustySNES to true SNES-hardware matching.
Opened 2026-07-22 at coverage baseline: 332/443 AccuracySNES, accuracy ledger current as of `v1.9.0`.

**Method (every ticket):** research the exact hardware behavior against a primary source
(fullsnes / anomie / SNESdev wiki) AND the reference cores in `ref-proj/` (ares/bsnes/Mesen2) →
implement clean-room → cross-validate against the full golden corpus (undisbeliever 29, coprocessor
goldens, 53 AccuracySNES scenes, commercial screenshots, both AccuracySNES cross-val references) →
any golden that legitimately changes is re-blessed **only** from a render the references agree on
(never blind) → land as its own PR with the accuracy-ledger row moved to **Remediated**. Determinism
(`docs/adr/0004`) and the allocation-free hot path are invariants throughout.

**Status legend:** `[ ]` not started · `[~]` in progress · `[x]` landed · `[BLOCKED]` blocked (reason)

---

## Group A — well-defined, localized, low regression-risk (do first)

| # | Ticket | Gap | Source | Approach |
|---|---|---|---|---|
| T-CA-01 | `$4212` bit 0 auto-joypad busy | unimplemented; only bits 7/6 modelled | `bus.rs`, fullsnes | **[x] Landed** — `$4212` bit 0 reads busy for the 4224-clock auto-read window (ares deadline model), + open bus in bits 1-5. Battery 290/290; cross-val unchanged. |
| T-CA-02 | `$4210` RDNMI held-flag + open-bus | lacks the ~4-cycle held-flag (Terranigma) quirk + open-bus bits 4-6 | `bus.rs`, fullsnes | **[x] Landed** — open-bus bits (`$4210` 4-6, `$4211` 0-6) + the four-master-clock **held-flag**: a `$4210`/`$4211` read within one dot of the VBlank/IRQ edge returns bit 7 set without clearing it (ares `nmiHold`/`irqHold`; `Clock::rdnmi_hold`/`irq_hold`, consumed next dot in `tick_ppu_dot`, serialized `FORMAT_VERSION` 6). Battery 292/292 (B4.03/04/05 unregressed), new core unit test, save round-trip green. |
| T-CA-03 | Auto-joypad read timing window | entirely unmodeled (`no auto_joypad symbol`) | `bus.rs`, fullsnes/anomie | **[x] Landed** — the read now spans 4224 master clocks from vblank entry and publishes `$4218-$421F` only at completion (deferred from the start snapshot). Battery 290/290; cross-val unchanged. (`$4016/$4017` manual-read blocking during the window is a finer refinement not yet added.) |
| T-CA-04 | SMP wait-state divider | glitchy `{2,4,10,20}` collapsed to `SMP_WAIT=2` | `apu/lib.rs:53` | Restore the per-region external-access wait-state table so SMP external (`$00F0-`) access timing matches; validate against `spc_mem_access_times`. |

## Group B — medium, subsystem-localized

| # | Ticket | Gap | Source | Approach |
|---|---|---|---|---|
| T-CA-05 | S-DSP literal 32-tick interleave | batched once per output sample | `apu.md:333` | Drive the existing `misc27/28`/per-voice/`echo22..30` sub-steps across the literal 32 DSP ticks with shadow latches; observable DAC + state must stay ares-identical for static input. Validate against `spc_dsp6`. |
| T-CA-06 | `STP`/`WAI` wake-edge timing | approximate | `cpu.md:194` | Model `WAI` resuming on the exact interrupt-poll edge and `STP` halting the master clock until reset at the correct cycle; AccuracySNES `A6.11`/`A6.12` region. |
| T-CA-07 | ABORT + mid-RMW interrupt injection | not modelled | `cpu.md:195` | Model interrupt injection at a sub-instruction (mid-RMW) boundary; ABORT vectors are unused on the 5A22 (dossier:741) so scope is the interrupt-timing half. |
| T-CA-08 | SA-1 timing | approximate | `scheduler.md:435` | Tighten the SA-1 second-CPU step/IRQ timing against ares `sfc/sa1`; re-verify the SA-1 golden (`SD F-1 Grand Prix`). |
| T-CA-09 | ST018 ARM cycle-count | simplified early-termination approximation | `st018-arm-notes.md:119` | **[x] No change needed (re-scoped 2026-07-22).** `multiply_cycles` (`coproc/armv3/cpu.rs`) ALREADY implements the ARM ARM's documented early-termination rule exactly (1 cycle if `Rs` bits 31-8 are uniform, 2 if 31-16, 3 if 31-24, else 4). `docs/st018-arm-notes.md` is authoritative that this documented rule — NOT the reverse-engineered Booth's-exact `GbaCpuMultiply` derivation — is the intended target, because **nothing in the determinism contract or accuracy oracle exercises ST018 cycle timing** (unlike the 65C816/PPU/APU, which do). Result bits are exact (games depend on them); idle-cycle precision beyond the documented rule is deliberately out of scope. The ticket's premise (a "simplified" count below the documented rule) does not match the code. |

## Group C — large / blocked (major efforts)

| # | Ticket | Gap | Source | Status |
|---|---|---|---|---|
| T-CA-10 | **Per-dot PPU compositor** | per-scanline; mid-line register writes, offset-per-tile, interlace, live `frame_hires` all wrong at dot resolution | `docs/adr/0014`, `ppu.md` | [~] ADR 0014 written. **Phase 1 landed (#205):** extracted `compose_pixel`/`DacCarry` (the per-pixel DAC entry point) from `compose_dac`'s inline loop — bit-identical. **Phase 2 landed (#210):** split `render_bg` into a FETCH pass (`[Pixel;256]` line buffer) + a DRAIN pass — bit-identical. **Phase 3 (NEXT — first behaviour change) scoped 2026-07-23, see the plan block + TRAPS below.** Then Phase 4 (live BG fetch / mid-line scroll), Phase 5 (sprites + hi-res two-sub-pixel), Phase 6 (flip default after full-corpus re-bless). |

### T-CA-10 Phase 3 — concrete implementation plan + traps (scoped 2026-07-23)

**Goal of Phase 3:** the first behaviour change — the in-render CGRAM/OAM/counter latch (unblocks
dossier **C3.04** "CGRAM access during active display hits the color *currently being drawn*",
**C11.08** MPY-per-pixel, **C1.08** `$2138` mid-frame). This *requires* driving the composite live
per-dot, because the latch VALUE is the palette index of the dot being drawn right now — the batch,
which composites the whole line at `RENDER_DOT`=276, cannot supply it (confirmed by diffing the code
2026-07-23). Reference: ares `sfc/ppu/io.cpp:48-51` (CGRAM write → `latch.cgramAddress` when
`!displayDisable && 0<vcounter<vdisp && 88<=hcounter<1096`), `:32-42` (OAM → `latch.oamAddress`,
`!displayDisable && vcounter<vdisp`, no hcounter bound); latch set at the DAC/sprite read sites
(`dac.cpp:159`, `object.cpp`).

**Shape:** add a default-off `per-dot-compositor` cargo feature (rustysnes-ppu; wire a frontend
passthrough). Under flag-ON, in `tick_dot`: FETCH the line (backdrop+BG+sprites into feature-gated
`Ppu` line buffers) once, then COMPOSE one column per dot via the existing `compose_pixel`, tracking
`latch_cgram_address`. Skip the batch `render_scanline` call at `RENDER_DOT`. **Milestone before any
behaviour: flag-ON byte-identical to batch across the FULL corpus** (undisbeliever 29 + 53 scenes +
coprocessor goldens + commercial screenshots); only THEN wire the CGRAM/OAM write-redirect.

**TRAPS identified 2026-07-23 (each will silently shift goldens or break determinism if missed):**
1. **Sprite over-flag timing.** `render_objects` (`render.rs:803`) is `&mut self` and sets the
   `$213E` range/time over-flags. Moving the fetch off `RENDER_DOT` to line-start re-times when those
   CPU-observable flags appear (AccuracySNES C7.x reads them). Either keep sprite eval at its current
   dot or account for the shift explicitly — do not move it blindly.
2. **HDMA-ordering dot window.** The batch renders line V at `RENDER_DOT`=276, and line V's own HDMA
   runs strictly AFTER (the off-by-one-line design, `tick_dot` doc + `docs/ppu.md`). Every per-dot
   compose MUST finish by dot 276 so it still reads pre-line-V-HDMA state; mapping column `x` to a
   dot `>276` reads post-HDMA state for the last column(s) and shifts the line edge. Suggested map:
   column `x` composes at dot `276-255+x` (= 21..276), fetch at ~dot 20 — but that is `<ACTIVE_DOT_START`
   (22), which re-raises trap 1. Resolve the exact window against ares' `hcounter` (`x=(hcounter-56)>>2`,
   plan's `hcounter=h*4`) before coding.
3. **Save-state of the transient per-dot line buffers.** `pd_above`/`pd_below`/`pd_carry` are live
   only during a line; a mid-line save/restore under flag-ON would lose them (the batch has no such
   per-line state). `latch_cgram_address` is CPU-observable and MUST be serialized (FORMAT_VERSION
   bump). Either serialize the line buffers too or document/guarantee save points are at VBlank.
4. **`Pixel` is private to `render.rs`** — to store line buffers on `Ppu` (lib.rs) either make
   `Pixel` `pub(crate)` or keep the buffers in a `render`-module struct field.

**Validate LOCALLY** (fast loop): `--test undisbeliever_golden` + `--test accuracysnes_scenes` +
`rustysnes-ppu --lib`, then the coprocessor oncart tests + `crossval.sh` before the PR. Re-bless any
legitimately-shifted golden (mid-line-write ROMs) ONLY from a reference-agreeing render (ADR 0013);
the coprocessor goldens are usable local gates again post-#216 ([[coprocessor-golden-staleness-rootcause]]).
Keep flag-OFF byte-identical throughout (default builds unchanged).

### T-CA-10 Phase 4 — status + remaining plan (2026-07-23; branch `feat/per-dot-compositor-cgram-latch`, draft #218)

**KEY ASSET — `ref-proj/MesenCE`** (cloned + built this session; env has .NET 10 SDK + SDL2 + clang, `make -j`,
binary `bin/linux-x64/Release/Mesen`, run `SDL_VIDEODRIVER=offscreen SDL_AUDIODRIVER=dummy Mesen --testRunner
<lua> <rom>`; Lua `emu.getScreenBuffer()` = 0xRRGGBB of the RENDERED frame, top rows are overscan-black so
compare distinct picture-color SETS not row-0). It is the authoritative Phase-4 blueprint AND the exact-frame
oracle. Phase-4 driving loop = `Core/SNES/SnesPpu.cpp:RenderScanline()` (928-976): three incremental cursors
advancing with the dot clock — sprite-eval `_spriteEvalStart..End` (935 `EvaluateNextLineSprites`), BG-fetch
`_fetchBgStart..End` (944 `FetchTileData`), draw `_drawStartX..EndX` (`_drawEndX=min(hPos-22,255)`); reset per
line at `ProcessEndOfScanline:443`. CGRAM redirect = `:2216` (`InternalCgramAddress`); OAM redirect = `:2006`
+`:1768` (`GetOamAddress`→`_oamEvaluationIndex`/`_oamTimeIndex` + high-table quirk, "needed for Uniracers").

**4a DONE (commits 7689b92, 391ac7c):** the DRAW cursor. `Ppu::pd_render_to_dot` (each `tick_dot`) composites
the line one column at a time up to the DAC column with LIVE registers, finishing by `RENDER_DOT` (pre-HDMA);
`pd_fetch_line` builds the line once at its start (always, even under force-blank — a real bug fixed in 391ac7c).
The exact CGRAM latch `internal_cgram_address` (= last-drawn palette) replaces the removed on-demand resolver.
Feature-gated `Ppu` fields are transient (not save-stated; re-fetched per line). **VERIFIED: feature-OFF
byte-identical (29 tests); feature-ON byte-identical on 26/29 undisbeliever (static); clippy+fmt clean both.**

**Open at 4a:** the 3 mid-line INIDISP undisbeliever ROMs shift but don't match MesenCE (`inidisp_forgot` renders
`7fff` vs MesenCE `7fc6`; the BATCH renders `34e6`, so even the old model was wrong for it). Findings: that ROM
does ZERO `$2122` writes (redirect irrelevant) and its artifact is PPU-access-during-active-display (VRAM/OAM),
a deep quirk = **Phase 4d**. MesenCE applies INIDISP live per-segment with NO write-delay (so brightness_delay is
not a simple systematic fix). These are undisbeliever, NOT AccuracySNES scored rows — they gate the corpus
re-bless, not coverage. **DON'T chase one ROM; do the systematic phases:**

- **4b — incremental sprite-eval cursor.** Split `render_objects` range/time eval (sets `$213E` over-flags) from
  paint; run the eval incrementally over dots per MesenCE `EvaluateNextLineSprites`. Fixes over-flag DOT timing
  (needed for AccuracySNES C7.x) and mid-line OAM changes. Milestone: framebuffer byte-identical + over-flag reads.
- **4c — incremental BG fetch-ahead.** Run `render_bg`'s FETCH incrementally over `_fetchBgStart..End` (≤ dot 263,
  draw ≤ 255) so a mid-line scroll write only reaches not-yet-fetched columns. Unblocks mid-line scroll rasters.
- **4d — deep mid-line access + hi-res/interlace.** PPU VRAM/OAM/CGRAM access-during-render redirect/drop model
  (the inidisp_forgot class), MPY-during-render (C11.08), `$2138` mid-frame (C1.08), two-sub-pixel hi-res +
  interlace at dot resolution. Re-bless the legitimately-shifted goldens (incl. the 3 INIDISP ROMs) vs MesenCE.
- **Item 3 — OAM redirect (C7.16 Uniracers), integrated with 4b's sprite-eval cursor** (`oam_write_target` in
  regs.rs mirroring the CGRAM one, using the eval index; port the high-table-also-written quirk).
- **Phase 6 — flip default** after 4b-4d+OAM land and the full corpus re-blesses vs MesenCE/ares; then add the
  AccuracySNES SCORED rows (C3.04, C11.08, C1.08, C7.16, hi-res) → coverage climbs past 339/443.

Determinism/save-state: serialize any CPU-observable new cursor state at the point mid-line saves become possible;
keep transient line buffers re-derived. Every phase: byte-identical milestone → save-state round-trip → determinism
→ clippy/fmt both feature states → MesenCE frame agreement for any re-blessed golden.

| T-CA-11 | 65816 cycle-by-cycle bus trace | cycle counts are per-instruction tallies, access **order** not pin-validated | `cpu.md:186`, timing-oracle | [ ] Large: model per-cycle bus access (address driven each internal cycle) so open-bus/DMA-interaction is exact. |
| T-CA-12 | Open-bus-via-HDMA-latch | correct fix breaks 24 GSU goldens, root cause unknown | `accuracy-ledger.md`, `scheduler.md` | [BLOCKED] Blocked on an access-level trace of GSU VRAM/CGRAM writes vs the failing DMA transfers. |

---

## Sequencing

Group A → B → C by tractability, but C's compositor (T-CA-10) proceeds in parallel as its own
phased effort (ADR 0014). Each Group A/B ticket is a self-contained PR. Land order chosen so
low-risk accuracy wins land first and the golden corpus is exercised repeatedly before the large
rewrites. T-CA-12 stays blocked until its investigation is scheduled.

### Reassessment after landing T-CA-01/02/03 and resolving T-CA-09 (2026-07-22)

Investigating the remaining Group A/B tickets against this program's own **test-as-spec discipline —
pin a failing oracle (a red test/vector) FIRST, then implement only until it passes** (the Method
note at the top of this file) — surfaced a pattern: **T-CA-04, T-CA-05, T-CA-06, T-CA-07, T-CA-08
have no failing oracle** —
no test or vector in the suite currently distinguishes these approximations from exact hardware, and
the project docs describe each as *exact for the results games/tests actually observe* (a claim, not
yet an independently-pinned fact — the table above still records each as an open gap):
`docs/scheduler.md:435` (SA-1: "approximate catch-up … exact for the register/arithmetic/DMA results
games observe"), `docs/cpu.md:200` (WAI/STP wake-edge "approx", but AccuracySNES `A6.11`/`A6.12`
already pass), `docs/st018-arm-notes.md` (ST018 cycle timing deliberately not gated). Changing these
speculatively — no red test to turn green — risks regressing
CPU/DSP/coprocessor timing for **no ROM-observable benefit**, which the pin-a-failing-oracle-first
discipline exists to prevent. They should each wait for a concrete failing vector (a game or a stricter test that
actually diverges) rather than being remediated blind. **The genuine remaining Tier-1 work with a
real ROM-observable payoff is T-CA-10 (the per-dot compositor)** — it unblocks the hi-res scene
cluster (~15-20 AccuracySNES rows) and mid-line register-write accuracy — plus T-CA-11 (large) if an
open-bus/DMA-order edge case ever needs it. T-CA-12 stays blocked.

## Progress log

- 2026-07-22: program opened; ADR 0014 (per-dot compositor) written; Group A/B tickets scoped.
- 2026-07-22: **T-CA-02 (partial) landed** — `$4210`/`$4211` open-bus bits (researched vs fullsnes +
  ares `CPU::readIO`). Battery 290/290, core+CPU unit suites green. Held-flag quirk remains.
- 2026-07-22: **T-CA-01 + T-CA-03 landed** — `$4212` auto-joypad busy flag + the timed 4224-clock
  auto-read (ares `status.autoJoypadCounter` as a master-clock deadline; result deferred to
  completion), + `$4212` open-bus bits 1-5. Two new unit tests, battery 290/290, snes9x + Mesen2
  cross-val unchanged. (Landed in #204 with T-CA-02 open-bus.)
- 2026-07-22: **T-CA-10 Phase 1 landed** — extracted `compose_pixel`/`DacCarry` from `compose_dac`
  (the per-pixel entry point Phase 4 will drive per-dot), verified bit-identical (undisbeliever 29 +
  53 scenes, ppu unit tests 29/29). Foundational refactor; no behavior change. Next: Phase 2-4.
- 2026-07-22: **T-CA-02 fully landed** (PR #208) — the four-master-clock RDNMI/TIMEUP **held-flag** (a
  `$4210`/`$4211` read within one dot of the VBlank/IRQ edge returns bit 7 set without clearing it;
  ares `nmiHold`/`irqHold`, Terranigma). `Clock::rdnmi_hold`/`irq_hold`, set with the flag, consumed
  next dot in `tick_ppu_dot`, serialized (`FORMAT_VERSION` 5→6). New core unit tests (read behavior +
  a lifecycle test driving `tick_ppu_dot` across the vblank edge); AccuracySNES battery 292/292
  (B4.03/04/05 unregressed); full test-roms harness green (49 golden framebuffers + coprocessor
  unchanged — no NMI-timing frame shift); save round-trip green. Ticket closed.
- 2026-07-22: **T-CA-09 resolved as no-change-needed** — `multiply_cycles` already implements the ARM
  ARM documented early-termination rule exactly, which `docs/st018-arm-notes.md` establishes as the
  intended target (further precision deliberately out of scope; nothing exercises ST018 cycle timing).
- 2026-07-22: **Group A/B remainder reassessed** — T-CA-04/05/06/07/08 have no failing oracle (each
  approximation is documented as exact for observed results); deferred (pin a failing oracle first) rather than
  changed blind. The genuine remaining ROM-observable Tier-1 work is T-CA-10 (per-dot compositor).
- 2026-07-23: **T-CA-10 Phase 2 recorded as landed (#210)** — `render_bg` fetch/drain split,
  bit-identical. And **Phase 3 fully scoped** (see the plan block under the T-CA-10 row): the in-render
  CGRAM/OAM latch (C3.04/C11.08/C1.08) is confirmed to REQUIRE the live per-dot composite (the batch
  composites the whole line at `RENDER_DOT`, so it cannot supply "the color currently being drawn").
  Three determinism/timing TRAPS identified before writing any code — sprite over-flag timing, the
  HDMA-ordering dot window, and save-state of the transient per-dot line buffers — plus the `Pixel`
  visibility detail. Phase 3 is the first behaviour change and the emulator's most determinism-critical
  code; it is queued as its own focused PR (branch `feat/per-dot-compositor-*`) with a flag-ON
  byte-identical milestone before the CGRAM redirect is wired. No code landed this session (the
  scoping + trap-identification is the deliverable); the baseline was restored first (coprocessor
  goldens re-blessed, #216) so Phase 3's legitimate golden shifts will be cleanly isolatable.
