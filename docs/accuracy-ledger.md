# Accuracy Ledger

`v1.6.0 "Lighthouse"`. This document maps every currently-known approximation, divergence, or
validation gap in RustySNES to an explicit disposition. `docs/STATUS.md`'s "Accuracy dashboard"
stays the pass-count scoreboard — always current, reaffirmed every release; this ledger is its
companion, answering *why* each residual is where it is, in the same disposition taxonomy
RustyNES's own `docs/accuracy-ledger.md` uses:

- **Remediated** — was a real gap or bug; fixed and verified.
- **No-stricter-oracle-available** — behavior is implemented; no independent test-ROM, golden
  framebuffer, or commercial title exists locally to validate it further (a ROM-sourcing gap, not
  an open bug).
- **Deferred** — researched, sometimes prototyped, not landed; a specific reason blocks it.
- **Out-of-scope** — an intentional non-goal (hardware behavior that's genuinely undefined, or a
  documented, deliberate simplification).

Every row cites the subsystem doc with the full mechanism and evidence trail — this ledger is the
index, not a duplicate of that reasoning.

## AccuracySNES residuals

| Item | Disposition | Detail |
|---|---|---|
| Decimal-mode `V` flag (`A7.04`) | **Out-of-scope** (golden vector) | Hardware does not define `V` after a decimal `ADC`. ares, bsnes, and Mesen2 all compute it with the identical binary-overflow formula evaluated *before* the BCD `+$60` correction — agreement by shared convention, not authority. The test records the observed bit as a variant and never scores. |
| Real-hardware validation | **Deferred** | AccuracySNES is designed to run on a real SNES and has not been. Cross-validated headlessly against Mesen2 and snes9x (both agree, 0 failures), and its expected values reviewed against ares/bsnes/Mesen2 source — but no silicon. This is the honest ceiling on the battery's authority. |
| ares/bsnes counted as one reference | **Out-of-scope** (methodology note) | A full diff of their `wdc65816` cores shows only type renames; ares' 65816 is a lineal descendant of bsnes'. The `Corroborated` tier therefore means "the bsnes/ares lineage and Mesen2 agree" — two implementations, not three. Recorded so the tier is not over-read. |
| Groups B-G | **Deferred** | Phase A ships Group A (65816 CPU) only. The remaining ~290 tests are enumerated per-test in `docs/accuracysnes-research-dossier.md` §5. |

## Core/SPC oracle residuals

| Item | Disposition | Detail |
|---|---|---|
| 65816 `e1.e` (`SBC (dp,X)`, emulation mode) | **Out-of-scope** | A single documented inter-reference divergence in the bsnes `readDirectX` `DL!=0` high-byte-wrap case that the rest of SingleStepTests/65816 does not model. 5,119,999/5,120,000 — 0-diff against the reference behavior every other test vector agrees on. `docs/adr/0002`. |
| `$4203`/`$4206` overlapping multiply/divide read | **Out-of-scope** | SNESdev's own errata documents genuinely *undefined* RDMPY/RDDIV output here — no canonical "corrupted" value exists to port; inventing one would violate the determinism-contract spirit (`docs/adr/0004`) of not fabricating behavior real hardware itself doesn't define. `crates/rustysnes-core/src/bus.rs`'s `MulDiv` doc comment cites the errata directly. |
| "DMA/HDMA-collision crash quirk" | **Out-of-scope** (mostly) / **Remediated** (the well-defined parts) | The SNESdev errata bundles three behaviors: two are chip-revision defects (5A22 v1/v2) compliant commercial ROMs avoid, not reproduced by mainstream reference emulators; the third (a version-agnostic silent whole-frame HDMA failure) is well-defined but has no known title or test ROM to verify against. The well-defined sub-cases that DO apply — A-bus address restrictions, HDMA-preempts-GP-DMA priority — are already correctly implemented. `docs/scheduler.md` §The "DMA/HDMA-collision crash quirk". |

## PPU/timing residuals

| Item | Disposition | Detail |
|---|---|---|
| HDMA mid-scanline placement | **Remediated** | `Bus::advance_master` fires HDMA's per-line run at the hardware-correct dot 276, proven by committed goldens. `docs/scheduler.md` §DMA/HDMA bus-steal. |
| Mid-scanline/HDMA-driven register timing (the "Air Strike Patrol BG3 scroll" case) | **Remediated** (`v0.8.0`) | The PPU composites each line at `RENDER_DOT` (dot 276) so a per-line HDMA register write only becomes visible starting the following line, matching real hardware. Verified against SA-1's `SD F-1 Grand Prix` (159/239 rows changed, 232/237 checkable rows matching the predicted signature) and all 24 Super FX/GSU goldens, each independently row-level-verified. `docs/ppu.md` §Mid-scanline/HDMA-driven register timing. |
| Open-bus-via-HDMA-latch (the "Speedy Gonzales stage 6-1" case) | **Deferred** | Confirmed via two independent primary sources that `Bus::open_bus` should update on DMA/HDMA-driven byte transfers, not just direct CPU accesses — but the naive fix breaks all 24 Super FX/GSU goldens for a reason not yet root-caused. Blocked on an access-level trace of GSU VRAM/CGRAM writes correlated against the failing DMA transfers. `docs/scheduler.md` §Open bus via DMA/HDMA. |
| Open-bus-via-DMA-latch (the "Speedy Gonzales stage 6-1" DMA case) | **Remediated** (`v1.4.0`) | Cross-checked directly against ares' and bsnes' `CPU::Channel` implementations: DMA/HDMA reads update `open_bus`, writes never do. Golden hashes re-blessed with the citation trail. `docs/scheduler.md` §Open bus via DMA/HDMA. |
| DRAM refresh (40 clocks/scanline) | **Out-of-scope** (empirically) | Measured across 500 steady-state frames × 3 ROMs: the current CPU-driven model already reproduces the correct ≈357,368-clock NTSC frame length. The originally-planned additive stall would have been a regression against this confirmed-correct baseline. `docs/scheduler.md` §DRAM refresh. |
| Hi-res (Modes 5/6) color-math precision | **Remediated**, real-title validation **No-stricter-oracle-available** | The dual-column, one-pixel-clock-delayed DAC mechanism is implemented and unit/non-regression-verified (`v0.7.0`), mirroring ares' `PPU::DAC`. Neither named motivating title has confirmed it against real hi-res content: Bishoujo Janshi Suchie-Pai has no local dump; Marvelous/SA-1 is dumped but never observed entering hi-res in a 1200-frame headless run. `docs/ppu.md` §Hi-res (Modes 5/6) color-math precision. |
| PAL region auto-detection | **Remediated**, golden-boot proof **No-stricter-oracle-available** | `Bus::sync_region_from_cart` correctly reconfigures line count/status from the header at reset, proven end-to-end (a full 312-line PAL frame completes). No PAL ROM in the local corpus for a golden-framebuffer proof. `docs/rom-test-corpus.md`. |
| ExLoROM memory map | **Remediated**, golden-boot proof **No-stricter-oracle-available** | Decode formula sourced directly from bsnes's own runtime board database (`docs/adr/0008`), formula-level unit-tested. No ExLoROM ROM in the local corpus. |

## Coprocessor residuals (honesty gate: `docs/adr/0003`)

| Chip | Disposition | Detail |
|---|---|---|
| DSP-1, Super FX/GSU, SA-1 (Core/Curated) | **Remediated** | Oracle-gated, honesty-gate green — 3/3. |
| DSP-2, DSP-4, ST010, S-DD1, CX4, OBC1 (BestEffort) | **Remediated** | 6/9 BestEffort boards real-title validated against actual commercial gameplay content. |
| DSP-3, ST011 | **Out-of-scope** | No verified board/window entry exists to pin against — a named residual, not a ROM-sourcing gap (there's nothing to source; the hardware protocol itself isn't established). `necdsp_variant.rs`. |
| SPC7110 | **No-stricter-oracle-available** | Implemented, multiple real addressing/open-bus bugs found and fixed through `v0.8.0`/`v0.9.0`. The remaining boot gap was root-caused to a ROM-identity issue, not an emulation bug: the local dump is an English fan-translation with a patch-only 1 MiB "Expansion ROM" region no real cartridge has (SHA256 mismatch, non-standard file size, a public forum thread documenting the patch's memory map — three independent confirmations). A genuine original-cartridge dump (sha256 `69d06a3f3a4f3ba769541fe94e92b42142e423e9f0924eab97865b2d826ec82d`) is the ROM-sourcing gap now tracked in `docs/rom-test-corpus.md`. `docs/audit/spc7110-boot-crash-2026-07-08.md`. |
| ST018 | **No-stricter-oracle-available** | Full ARMv3 core implemented (`v0.4.0`), unit-test-level coverage only — no commercial dump in the local corpus. |
| S-RTC (standalone) | **No-stricter-oracle-available** | Implemented (`v0.4.0`), unit-test-level coverage only — no commercial dump in the local corpus. |

## Frontend/determinism boundary notes

| Item | Disposition | Detail |
|---|---|---|
| `emu-thread`: movies, Lua scripting, RetroAchievements, rewind-recording | **Out-of-scope** (intentional architecture boundary) | Confirmed by directly reading RustyNES's own mature `emu_thread.rs`, which doesn't port any of these to its thread either — reclassified from "remaining gap" to permanent boundary in `v1.4.0`. |
| Per-voice audio mute (`v1.0.1`) | **Out-of-scope** (by design) | Real S-DSP hardware has no per-voice mute register (only the whole-mix `FLG.6` bit); this gates `Dsp::voice_output`, strictly downstream of BRR decode/envelope/pitch computation, so it can never perturb any ROM-observable register or envelope timing. A host/debug convenience, not modeled hardware. `docs/apu.md` §Per-voice mute. |
| Shader/post-filter presentation passes (CRT/HQx, `v1.2.0`; the planned composite/RF pass, `v1.10.0`+) | **Out-of-scope** (by design) | Display-only, downstream of the golden framebuffer — never included in any determinism-contract hash or golden-framebuffer comparison. `docs/frontend.md` §Presentation post-filters. |

## Regression-net coverage

RustyNES's Holy Mapperel is a large-mapper-number-space regression net with no direct SNES
equivalent — SNES's coprocessor/board space is small enough that `docs/STATUS.md`'s tier matrix
already tracks every known chip family individually, and every Core/Curated/BestEffort board
already has its own dedicated oracle or golden-framebuffer test where a commercial dump exists.
The one gap: the 3 unit-test-only boards (SPC7110, ST018, S-RTC, all **No-stricter-oracle-
available** above) have no boot-level regression coverage at all today, since they have no
commercial dump to boot against. A future, SNES-appropriate analog to Holy Mapperel — a per-board
boot-smoke net using synthetic/homebrew fixtures rather than commercial dumps — is not yet
implemented; tracked as a candidate future addition to `crates/rustysnes-test-harness`, not
claimed done here.

**Source of truth:** [STATUS.md](STATUS.md) (pass-count dashboard) · **Cross-linked from:**
[DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md)
