# RustySNES Documentation Index

**RustySNES version:** `v1.5.0 "Bedrock"`

This index maps the `docs/` tree for RustySNES — the cycle-accurate SNES/Super Famicom emulator.
The single source of truth for accuracy pass counts, the coprocessor/board matrix, and version
policy is [`STATUS.md`](STATUS.md). This page also serves as the home page of the Material for
MkDocs handbook (`mkdocs.yml`, `v1.6.0 "Lighthouse"`) published at
`https://doublegate.github.io/RustySNES/docs/`.

---

## Subsystem specifications

The core "spec" docs — kept in sync with the code in the same PR as a change (docs-as-spec,
`docs/adr` isn't the place for subsystem behavior, these files are).

| Document | Subsystem |
|----------|-----------|
| [cpu.md](cpu.md) | 65C816 (5A22) CPU — opcodes, addressing modes, native/emulation mode, the master-clock-driven access-speed model |
| [ppu.md](ppu.md) | PPU1 (5C77) + PPU2 (5C78) — BG modes 0-7, Mode 7, OAM/sprites, color math, windows, the dot/scanline timeline |
| [apu.md](apu.md) | SPC700 (S-SMP) + S-DSP — the asynchronous clock domain, the integer-accumulator resync, cycle-exact SMP stepping |
| [scheduler.md](scheduler.md) | The master-clock lockstep scheduler (dot-resolution timing), DMA/HDMA bus-steal, H/V-IRQ, SA-1/GSU second-thread integration |
| [cart.md](cart.md) | LoROM/HiROM/ExHiROM/ExLoROM memory-map models + the full coprocessor/board matrix (chip-by-chip implementation notes) |
| [cartridge-format.md](cartridge-format.md) | SNES header parsing, chipset-byte coprocessor detection, region/mapping-mode scoring |
| [st018-arm-notes.md](st018-arm-notes.md) | The ARMv3 (ARM6-class) CPU core for ST018 — architecture, detection research, board-bus protocol, build order |
| [architecture.md](architecture.md) | Cross-cutting design (the `Bus` owns mutable state, one-directional crate graph, determinism contract) |
| [frontend.md](frontend.md) | The `rustysnes` desktop/wasm app (winit + wgpu + cpal + egui), audio engine, pacing, save-states, rewind, run-ahead |

## Cross-cutting references

| Document | Topic |
|----------|-------|
| [STATUS.md](STATUS.md) | **Single source of truth** — the accuracy dashboard, per-suite pass counts, coprocessor/board matrix, version policy |
| [accuracy-ledger.md](accuracy-ledger.md) | Every known approximation/divergence mapped to an explicit disposition (Remediated / No-stricter-oracle-available / Deferred / Out-of-scope) — the "why" companion to STATUS.md's pass-count dashboard (`v1.6.0`) |
| [testing-strategy.md](testing-strategy.md) | The testing layers; test ROMs and golden framebuffers as the spec |
| [rom-test-corpus.md](rom-test-corpus.md) | Which commercial/homebrew ROM would close each currently-open validation gap, and where (or whether) it's sourceable |
| [performance.md](performance.md) | Performance targets and rules |
| [benchmarks.md](benchmarks.md) | The reproducible benchmark record — actual measured numbers |
| [compatibility.md](compatibility.md) | ROM-format + coprocessor + per-game compatibility status |
| [libretro.md](libretro.md) | The `rustysnes-libretro` core — region-aware AV info, cheats, peripheral negotiation, firmware auto-resolution |
| [glossary.md](glossary.md) | SNES hardware + emulation terminology |

---

## Subdirectories

| Directory | Contents |
|-----------|----------|
| [adr/](adr/) | Architecture Decision Records (Michael Nygard format), `0001`–`0011` — the master-clock scheduler, the fractional-timebase-refactor deferral, the accuracy-tiering honesty gate, the determinism contract, the 65816 opcode-oracle license, the save-state format, the versioning/release-process adoption, ExLoROM's decode-formula sourcing, ST018's detection + catch-up architecture, the HD texture pack system, and CI branch-protection/`ci-success`. |
| [audit/](audit/) | Decision-rationale / open-investigation audit reports — longer-form than an ADR, for capturing *why* an investigation is where it is (root-cause trail, ruled-out hypotheses). Currently: the SPC7110 boot-crash investigation, and the fractional-timebase refactor go/no-go. |

## Related, outside `docs/`

| Location | Contents |
|----------|----------|
| [`../ref-docs/`](../ref-docs/) | Immutable primary research (never rewritten in place — corrections land as new dated supplemental files): the master research report, and per-subsystem PPU/APU/coprocessor research notes. |
| [`../ref-proj/`](../ref-proj/) | Gitignored study clones of reference emulators (ares, bsnes, Mesen2) — read for hardware behavior and board/timing data, never copied wholesale. |
| [`../to-dos/ROADMAP.md`](../to-dos/ROADMAP.md) | The phase spine (Phase 0 foundation → Phase 8 Reach, all complete) plus the ongoing RustyNES-parity ladder (`v1.5.0` onward) — the planning entry point. |
| [`../to-dos/VERSION-PLAN.md`](../to-dos/VERSION-PLAN.md) | The concrete `v0.x.0` → `v1.4.0` release ladder that reached production, and the `v1.5.0`-`v1.19.0` RustyNES-parity ladder now in progress. |
| [`../to-dos/LOCKSTEP-CHECKLIST.md`](../to-dos/LOCKSTEP-CHECKLIST.md) | The process for re-checking RustyNES's own continuing development before scoping each rung of the parity ladder. |
| [`../to-dos/phase-*/`](../to-dos) | Per-phase ticket breakdowns and sprint notes. |
| [`../CHANGELOG.md`](../CHANGELOG.md) | Release history — the annotated git tag body for each release is sourced from this file's dated sections. |

---

## External references

- [SNESdev Wiki](https://snes.nesdev.org/wiki/) — hardware specifications (registers, timing, the Errata page)
- [Super Famicom Development Wiki](https://wiki.superfamicom.org/) — registers, board database cross-reference
- [Fullsnes / Nocash SNES Specs](https://problemkaputt.de/fullsnes.htm) — the other primary hardware-timing reference
- [SingleStepTests](https://github.com/SingleStepTests) — the 65816/SPC700 per-opcode JSON oracle
- [gilyon/snes-tests](https://github.com/gilyon/snes-tests) · [undisbeliever/snes-test-roms](https://github.com/undisbeliever/snes-test-roms) — on-cart CPU / PPU-DMA-HDMA test ROMs
- [blargg's test ROMs](https://slack.net/~ant/) — the SPC/DSP audio accuracy suite

---

**Source of truth:** [STATUS.md](STATUS.md) · **Release history:** [`../CHANGELOG.md`](../CHANGELOG.md)
