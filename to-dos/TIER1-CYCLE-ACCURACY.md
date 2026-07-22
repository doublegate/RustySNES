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

**Status legend:** ☐ not started · ◐ in progress · ☑ landed · ⊘ blocked (reason)

---

## Group A — well-defined, localized, low regression-risk (do first)

| # | Ticket | Gap | Source | Approach |
|---|---|---|---|---|
| T-CA-01 | `$4212` bit 0 auto-joypad busy | unimplemented; only bits 7/6 modelled | `bus.rs:722`, fullsnes | Set bit 0 during the auto-read window (V=225.. for ~4224 clocks); clears when the read completes. Ties to Group F auto-joypad rows. |
| T-CA-02 | `$4210` RDNMI held-flag + open-bus | lacks the ~4-cycle held-flag (Terranigma) quirk + open-bus bits 4-6 | `bus.rs`, fullsnes | ◐ **Open-bus bits landed** (`$4210` bits 4-6, and `$4211` bits 0-6, now return the MDR — matches ares/fullsnes; battery 290/290). **Remaining:** the ~4-cycle held-flag quirk (flag reads set for a few cycles into the read) for both `$4210` and `$4211` (fullsnes: the IRQ/NMI condition true at read time returns bit7=1 without clearing, lasting 4-8 master cycles). |
| T-CA-03 | Auto-joypad read timing window | entirely unmodeled (`no auto_joypad symbol`) | `bus.rs:730`, fullsnes/anomie | Model the automatic read starting at vblank (~V=225, H≈32.5) and taking ~4224 master clocks, latching `$4218-$421F` at the end; `$4016/$4017` manual read blocked meanwhile. AccuracySNES Group F `F1.12` region. |
| T-CA-04 | SMP wait-state divider | glitchy `{2,4,10,20}` collapsed to `SMP_WAIT=2` | `apu/lib.rs:53` | Restore the per-region external-access wait-state table so SMP external (`$00F0-`) access timing matches; validate against `spc_mem_access_times`. |

## Group B — medium, subsystem-localized

| # | Ticket | Gap | Source | Approach |
|---|---|---|---|---|
| T-CA-05 | S-DSP literal 32-tick interleave | batched once per output sample | `apu.md:333` | Drive the existing `misc27/28`/per-voice/`echo22..30` sub-steps across the literal 32 DSP ticks with shadow latches; observable DAC + state must stay ares-identical for static input. Validate against `spc_dsp6`. |
| T-CA-06 | `STP`/`WAI` wake-edge timing | approximate | `cpu.md:194` | Model `WAI` resuming on the exact interrupt-poll edge and `STP` halting the master clock until reset at the correct cycle; AccuracySNES `A6.11`/`A6.12` region. |
| T-CA-07 | ABORT + mid-RMW interrupt injection | not modelled | `cpu.md:195` | Model interrupt injection at a sub-instruction (mid-RMW) boundary; ABORT vectors are unused on the 5A22 (dossier:741) so scope is the interrupt-timing half. |
| T-CA-08 | SA-1 timing | approximate | `scheduler.md:435` | Tighten the SA-1 second-CPU step/IRQ timing against ares `sfc/sa1`; re-verify the SA-1 golden (`SD F-1 Grand Prix`). |
| T-CA-09 | ST018 ARM cycle-count | simplified early-termination approximation | `st018-arm-notes.md:119` | Replace the early-termination approximation with the ARM-ARM documented cycle counts; unit-test-only (no commercial dump). |

## Group C — large / blocked (major efforts)

| # | Ticket | Gap | Source | Status |
|---|---|---|---|---|
| T-CA-10 | **Per-dot PPU compositor** | per-scanline; mid-line register writes, offset-per-tile, interlace, live `frame_hires` all wrong at dot resolution | `docs/adr/0014`, `ppu.md` | ◐ ADR 0014 written (design + ares blueprint + phased regression-safe rollout). Multi-phase implementation. |
| T-CA-11 | 65816 cycle-by-cycle bus trace | cycle counts are per-instruction tallies, access **order** not pin-validated | `cpu.md:186`, timing-oracle | ☐ Large: model per-cycle bus access (address driven each internal cycle) so open-bus/DMA-interaction is exact. |
| T-CA-12 | Open-bus-via-HDMA-latch | correct fix breaks 24 GSU goldens, root cause unknown | `accuracy-ledger.md`, `scheduler.md` | ⊘ Blocked on an access-level trace of GSU VRAM/CGRAM writes vs the failing DMA transfers. |

---

## Sequencing

Group A → B → C by tractability, but C's compositor (T-CA-10) proceeds in parallel as its own
phased effort (ADR 0014). Each Group A/B ticket is a self-contained PR. Land order chosen so
low-risk accuracy wins land first and the golden corpus is exercised repeatedly before the large
rewrites. T-CA-12 stays blocked until its investigation is scheduled.

## Progress log

- 2026-07-22: program opened; ADR 0014 (per-dot compositor) written; Group A/B tickets scoped.
- 2026-07-22: **T-CA-02 (partial) landed** — `$4210`/`$4211` open-bus bits (researched vs fullsnes +
  ares `CPU::readIO`). Battery 290/290, core+CPU unit suites green. Held-flag quirk + the
  `$4212`/auto-joypad busy-timing cluster (T-CA-01/03) remain next.
