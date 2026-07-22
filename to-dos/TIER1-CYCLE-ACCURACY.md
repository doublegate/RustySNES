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
| T-CA-10 | **Per-dot PPU compositor** | per-scanline; mid-line register writes, offset-per-tile, interlace, live `frame_hires` all wrong at dot resolution | `docs/adr/0014`, `ppu.md` | [~] ADR 0014 written. **Phase 1 landed:** extracted `compose_pixel` (the per-pixel DAC entry point Phase 4 drives per-dot) from `compose_dac`'s inline loop, threading the hi-res carry via `DacCarry` — **bit-identical** (undisbeliever 29 + 53 scenes unchanged). Remaining: Phase 2 (BG per-dot drain), Phase 3 (14-dot fetch-ahead + in-render CGRAM/OAM latch — first behavior change), Phase 4 (wire to `tick_dot` per-dot). |
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
the project docs state each approximation is *exact for the results games/tests actually observe*,
and the determinism contract holds. `docs/scheduler.md:435` (SA-1: "approximate catch-up … exact for
the register/arithmetic/DMA results games observe"), `docs/cpu.md:200` (WAI/STP wake-edge "approx",
but AccuracySNES `A6.11`/`A6.12` already pass), `docs/st018-arm-notes.md` (ST018 cycle timing
deliberately not gated). Changing these speculatively — no red test to turn green — risks regressing
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
