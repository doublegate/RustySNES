# Salvage Manifest — RustySNES

Record of files rescued from a volatile `/tmp` into the project tree (tmp-salvage skill).

## 2026-07-22 — curated salvage (timing references only)

A full `/tmp` dry-run surfaced 150 candidates (2.6 GB pile), but verification against the repo
showed every STRONG match was already committed (`ref-docs/fullsnes/`, the AccuracySNES harness,
`scripts/accuracysnes/mesen_crossval.lua`), superseded (staged CHANGELOG/ROADMAP drafts), third-party
study-clone material (deliberately gitignored), another project's files (RustyN64 / RustyNES-the-NES /
AccuracyCoin), or regenerable bulk (traces, screenshots, `.bak`/`.mut`/`.preedit` snapshots). The only
project-relevant material not already preserved was two publicly-available third-party timing
references. Salvaged those; skipped everything else.

| Source | Destination | Note |
|---|---|---|
| `/tmp/rustysnes-research/anomie_timing.txt` | `ref-docs/2026-07-22-anomie-snes-timing.md` | Vendored verbatim under provenance header; reference-only |
| `/tmp/rustysnes-research/nesdev_timing.txt` | `ref-docs/2026-07-22-nesdev-snes-timing.md` | Vendored verbatim; CC BY-SA 4.0, reference-only |

`ref-docs/README.md` index updated with both entries. Nothing else moved.
