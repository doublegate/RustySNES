# ROM-seeding runbook (T-01-004 / T-01-005)

The concrete steps behind Phase-0 ticket **T-01-004** (seed the permissive corpora + stand up
the external tier) and **T-01-005** (65816 license, resolved in `docs/adr/0005`). Authoritative
licensing source: `docs/testing-strategy.md` §licensing. **Rule of thumb: only permissive
corpora are committed; everything unlicensed / copyleft / Nintendo lives in the gitignored
`tests/roms/external/` tier.**

## Target layout

```text
tests/
  roms/
    README.md                 # committed — the license ledger (already present)
    gilyon/                   # committed (MIT)   — CPU+SPC .sfc + golden tests*.txt
      LICENSE
    undisbeliever/            # committed (Zlib)  — PPU/DMA/HDMA hardware-behavior .sfc
      LICENSE
    spc700-singlestep/        # committed (MIT)   — SAMPLED subset of the per-opcode JSON
      LICENSE  SAMPLING.md     #                    (full set is multi-GB → sample, don't vendor whole)
    external/                 # GITIGNORED — entire tier, never committed
      65816-singlestep/        #   no license   — local CPU oracle cross-check (ADR 0005)
      spc700-singlestep-full/  #   MIT but huge — full set, local only
      blargg-spc/              #   unstated     — cycle-accurate SPC/DSP reference
      240p/                    #   GPLv2        — run-only video/overscan
      krom/                    #   no license   — PeterLemon/SNES broad reference + PNGs
      commercial/              #   Nintendo     — the local legal dump (see corpus plan)
  golden/                     # committed — framebuffer/audio hashes + insta .snap baselines
    .gitkeep
```

## Committed (permissive) tier — the steps

For each: clone, copy ONLY the ROM/table files (not the upstream `.git`), preserve the upstream
`LICENSE` verbatim alongside, and add a one-line row to `tests/roms/README.md`.

1. **gilyon/snes-tests (MIT)** → `tests/roms/gilyon/`
   - Source: `github.com/gilyon/snes-tests`. Copy the built `.sfc` ROMs **and** their golden
     `tests*.txt` result tables (the harness asserts decoded output against these).
   - Verify: `git -C <clone> show HEAD:LICENSE | head` is MIT before copying; vendor the LICENSE.

2. **undisbeliever/snes-test-roms (Zlib)** → `tests/roms/undisbeliever/`
   - Source: `github.com/undisbeliever/snes-test-roms`. Copy the PPU / DMA / HDMA / hardware-glitch
     `.sfc` outputs. Zlib is permissive — vendor the LICENSE.

3. **SingleStepTests/spc700 (MIT)** → `tests/roms/spc700-singlestep/`
   - Source: the `SingleStepTests/spc700` (TomHarte ProcessorTests) JSON. The **full** set is
     multi-GB — do **not** vendor it whole. Commit a **deterministic sampled subset** (e.g. first
     N tests per opcode, fixed seed) sufficient for CI signal; keep the full set in
     `tests/roms/external/spc700-singlestep-full/` for exhaustive local runs.
   - Record the sampling rule in `spc700-singlestep/SAMPLING.md` so the subset is reproducible.

## External (gitignored) tier — the steps

Create the dir and gitignore the whole subtree; fetch each corpus locally as needed. Nothing
here is committed except artifacts they *produce* (screenshots, `.snap`, hashes) which land in
`tests/golden/`.

- **65816-singlestep/** — `SingleStepTests/65816`. No license (ADR 0005): local cross-validation
  reference only; the committed CPU oracle is self-generated. Never committed, never a CI dep.
- **blargg-spc/** — blargg `spc_*` (`spc_dsp6`, `mem_access`, `spc`, `timer`). Unstated license.
- **240p/** — 240p Test Suite (SNES), GPLv2 — clone/run-only, never vendored.
- **krom/** — PeterLemon/SNES (Krom) broad CPU/PPU/SPC/DSP/GSU ROMs + reference PNGs. No license.
- **commercial/** — the local legal dump (the Dropbox No-Intro set). See the commercial-ROM
  corpus plan; only the derived manifest + screenshots/`.snap` are committed.

## .gitignore additions

Add (root `.gitignore` already has `/tests/roms/external/`; confirm and extend):

```gitignore
/tests/roms/external/
# belt-and-suspenders against accidental commercial/ROM commits:
*.sfc
*.smc
!tests/roms/gilyon/**
!tests/roms/undisbeliever/**
```

(The negations keep the permissive committed `.sfc` files trackable while blocking stray ROMs
anywhere else. spc700 JSON is `.json`, not `.sfc`, so it is unaffected.)

## How the harness loads each tier (the loader contract — T-01-006)

- **Committed on-cart ROMs (Layer 3):** the harness enumerates `tests/roms/{gilyon,undisbeliever}/`,
  boots each `.sfc` via `runner::run_until_complete(system, budget)`, and asserts
  `TestResult::Passed` (or compares the decoded WRAM result string to the gilyon `tests*.txt`
  golden table). Behind the **`test-roms`** feature.
- **Per-opcode JSON oracle (Layer 2):** the JSON runner reads `tests/roms/spc700-singlestep/`
  (committed sample) by default; the full sets in `external/` are picked up when present. Each
  test sets the initial CPU/bus state, single-steps one opcode, and diffs final state **and the
  cycle-by-cycle bus trace** against the JSON; first mismatch fails. The 65816 oracle of record
  is the self-generated set (ADR 0005); the external upstream set is the cross-validation path.
- **Commercial / external boards:** behind the **`commercial-roms`** feature; reads
  `tests/roms/external/commercial/` and the committed corpus manifest. Absent ROMs → the suite
  reports skipped, never failed (`/accuracy-battery` skill enforces "skipped ≠ pass").
- **CI reality:** contributors have only the committed permissive tier. CI runs `test-roms` (it
  has those ROMs); `commercial-roms` runs only where the dump is staged (locally / a private
  runner), so CI never depends on non-redistributable data.

## Done when

- gilyon + undisbeliever + spc700-sample committed with LICENSE files; `tests/roms/README.md`
  ledger updated; `tests/roms/external/` gitignored and present.
- `cargo test --workspace --features test-roms` enumerates the committed corpora (0 real passes
  is fine at scaffold — the chips don't run yet; the point is the loader wiring stands up).
- No unlicensed/copyleft/Nintendo bytes tracked by git (`git ls-files | grep -iE '\.s[fm]c$'`
  shows only the gilyon/undisbeliever permissive ROMs).
