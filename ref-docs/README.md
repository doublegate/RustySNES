# ref-docs — RustySNES

The IMMUTABLE research corpus for Super Nintendo Entertainment System / Super Famicom. After the research step this directory is
frozen: new findings go in NEW dated files, never edits. Put the deep research report here
(`research-report.md`) plus the console's hardware/dev-wiki extracts and dated supplements.

## Contents

- `research-report.md` — the original deep research report.
- `2026-06-24-{apu,ppu,coprocessors}.md` — dated subsystem extracts.
- `2026-07-20-wdc-w65c816s-citation.md` — **citation only, deliberately not an extract.** The WDC
  W65C816S datasheet is the intended timing oracle (Table 5-7's VDA/VPA pin columns give a
  manufacturer-defined read/write/internal split per cycle), but its copyright reserves *"the right
  of reproduction in whole or in part in any form"* with no permissive carve-out. Facts derived
  from it may be re-expressed in this repo with the citation; its tables may not be reproduced.
- `2026-07-20-undisbeliever-65816-timing.md` — undisbeliever's 65816 instruction timing table,
  all 256 opcodes, vendored verbatim. **CC BY-SA 4.0** (the only file here that is not
  permissively licensed) with attribution intact — do not transcribe its values into code, which
  would arguably create adapted material inheriting ShareAlike. **Reference only, never a scoring
  oracle:** it is a compilation partly derived from higan's `wdc65816`, so scoring against it would
  reintroduce the emulator circularity AccuracySNES exists to avoid. Its own header states this.
- `2026-07-19-accuracysnes-hardware-test-design.md` — the hardware-behaviour and test-list design
  corpus behind **AccuracySNES**: the SNESdev errata list, a behaviour-to-game motivation table,
  the full per-assertion test enumeration for groups A-G, and a survey of the existing SNES
  test-ROM landscape. Distilled into `docs/accuracysnes-research-dossier.md`; the cartridge it
  designs is `tests/roms/AccuracySNES/`.
- `2026-07-22-anomie-snes-timing.md` — Anomie's SNES Timing Doc (rev 1126, 2007), vendored
  verbatim. Third-party community documentation with no explicit permissive licence: **reference
  only**, cite rather than reproduce as our own, and never a scoring oracle (see its header).
- `2026-07-22-nesdev-snes-timing.md` — the NESdev Wiki "Timing" (SNESdev) page, vendored verbatim.
  **CC BY-SA 4.0** with attribution intact — like the undisbeliever entry, do not transcribe its
  values into code (adapted-material/ShareAlike risk). **Reference only, never a scoring oracle.**
