# ref-docs — RustySNES

The IMMUTABLE research corpus for Super Nintendo Entertainment System / Super Famicom. After the research step this directory is
frozen: new findings go in NEW dated files, never edits. Put the deep research report here
(`research-report.md`) plus the console's hardware/dev-wiki extracts and dated supplements.

## Contents

- `research-report.md` — the original deep research report.
- `2026-06-24-{apu,ppu,coprocessors}.md` — dated subsystem extracts.
- `2026-07-19-accuracysnes-hardware-test-design.md` — the hardware-behaviour and test-list design
  corpus behind **AccuracySNES**: the SNESdev errata list, a behaviour-to-game motivation table,
  the full per-assertion test enumeration for groups A-G, and a survey of the existing SNES
  test-ROM landscape. Distilled into `docs/accuracysnes-research-dossier.md`; the cartridge it
  designs is `tests/roms/AccuracySNES/`.
