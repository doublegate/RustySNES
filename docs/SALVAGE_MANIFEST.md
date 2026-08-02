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

## 2026-08-02T13:58:17 — agent scratch, curated

Moved **40** one-off probe/debug scripts out of this project's agent scratch tree into `salvaged/scripts/`.

Curation applied on top of `--no-weak`: dropped ROM-derived captures (`smw_*.ppm`), regenerable
AccuracySNES scene captures (`s9.scene*.bin`), ares/NDK build detritus (`*.cmake`, `CMakeFiles/`,
CMake compiler-ID probes), and 6 files whose names are already tracked in the repo.

- `salvaged/scripts/a5_18-parked.patch`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/a5_18-parked.patch`
- `salvaged/scripts/a5_19-wip.patch`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/a5_19-wip.patch`
- `salvaged/scripts/atdone.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/atdone.lua`
- `salvaged/scripts/base-agy.sh`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/base-agy.sh`
- `salvaged/scripts/capture_with_input.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/capture_with_input.lua`
- `salvaged/scripts/doctor_scan.py`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/951bcba8-82dc-4ed4-8363-b59d0791cee6/scratchpad/doctor_scan.py`
- `salvaged/scripts/e506.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/e506.rs`
- `salvaged/scripts/e801.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/e801.lua`
- `salvaged/scripts/e801.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/e801.rs`
- `salvaged/scripts/e806.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/e806.rs`
- `salvaged/scripts/e902.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/e902.rs`
- `salvaged/scripts/edge_probe.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/edge_probe.lua`
- `salvaged/scripts/fails.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/fails.lua`
- `salvaged/scripts/final.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/final.lua`
- `salvaged/scripts/fixed_crossval.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/fixed_crossval.lua`
- `salvaged/scripts/fixed_scenes.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/fixed_scenes.lua`
- `salvaged/scripts/gh.py`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/951bcba8-82dc-4ed4-8363-b59d0791cee6/scratchpad/gh.py`
- `salvaged/scripts/held_1based.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/held_1based.lua`
- `salvaged/scripts/held_probe.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/held_probe.lua`
- `salvaged/scripts/idx_probe.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/idx_probe.lua`
- `salvaged/scripts/inspect_rom.py`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/inspect_rom.py`
- `salvaged/scripts/liveness.fixed.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/liveness.fixed.rs`
- `salvaged/scripts/lrcv.c`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/lrcv.c`
- `salvaged/scripts/m2list.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/m2list.lua`
- `salvaged/scripts/meas.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/meas.lua`
- `salvaged/scripts/new_rtt_tests.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/new_rtt_tests.rs`
- `salvaged/scripts/old_rtt_tests.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/old_rtt_tests.rs`
- `salvaged/scripts/one.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/one.lua`
- `salvaged/scripts/pad_probe.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/pad_probe.lua`
- `salvaged/scripts/pc.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/pc.lua`
- `salvaged/scripts/pinexact_hash_baseline.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/pinexact_hash_baseline.rs`
- `salvaged/scripts/pr260-fixes.patch`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/pr260-fixes.patch`
- `salvaged/scripts/probe.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/probe.rs`
- `salvaged/scripts/resolve.py`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/resolve.py`
- `salvaged/scripts/run-guardtest.sh`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/run-guardtest.sh`
- `salvaged/scripts/run-nightlytest.sh`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/run-nightlytest.sh`
- `salvaged/scripts/shift.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/shift.rs`
- `salvaged/scripts/spc_probe.rs`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/spc_probe.rs`
- `salvaged/scripts/wire_i18n.py`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/wire_i18n.py`
- `salvaged/scripts/wram_probe.lua`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/wram_probe.lua`


## 2026-08-02T14:01:40 — agent scratch, docs (hand-picked)

Moved **5** files to `salvaged/docs/`. The other ~54 `docs/` candidates were dropped:
~45 PR bodies and bot-review adjudications for merged PRs, three working copies of the repo's own
CHANGELOG (~1.4 MB), release notes for v1.22-v1.25 already in `CHANGELOG.md` and on GitHub
Releases, and the dry-run output of this salvage itself.

Note two of these five turned out to be bot-review adjudications rather than durable prose
(`adjudication.md`, `c258.md`); kept anyway, since `salvaged/` is gitignored and the cost is nil.

- `salvaged/docs/adjudication.md`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/adjudication.md`
- `salvaged/docs/c258.md`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/c258.md`
- `salvaged/docs/netplay.md`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/split/netplay.md`
- `salvaged/docs/plan_section.md`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/plan_section.md`
- `salvaged/docs/stack-tips.txt`  <- `/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/fc11bae0-ecd6-457a-ac1b-be930be88017/scratchpad/stack-tips.txt`

