# spc700-singlestep — sampling rule

This directory holds a **deterministic sampled subset** of the
[SingleStepTests/spc700](https://github.com/SingleStepTests/spc700) per-opcode
JSON oracle (TomHarte ProcessorTests format), license **MIT** (see `LICENSE`).

## Why a subset

The full upstream `v1/` set is **256 opcode files × 1000 tests = 256,000 tests
(~80 MB)** — too large to commit into the MIT/Apache source tree for CI signal.
The full set is kept locally (gitignored) at
`tests/roms/external/spc700-singlestep-full/` for exhaustive runs.

## The rule (reproducible)

For each opcode file `v1/XX.json` (XX = `00`..`ff`, the full 8-bit opcode space):

- Take the **first 50 tests** in upstream array order — `data[:50]`.
- Upstream order is fixed in the repo, so "first 50" is fully deterministic; no
  RNG, no seed needed.
- Re-serialized compactly (`separators=(",",":")`) to minimize committed bytes;
  the per-test object contents (`name`, `initial`, `final`, `cycles`) are
  byte-for-byte the upstream values, unmodified.

Committed subset: **256 files × 50 = 12,800 tests (~4.5 MB)**.

## Regenerate

```bash
# with the full upstream set present at tests/roms/external/spc700-singlestep-full/
python3 - <<'PY'
import json, glob, os
SRC = "tests/roms/external/spc700-singlestep-full"
DST = "tests/roms/spc700-singlestep/v1"
N = 50
os.makedirs(DST, exist_ok=True)
for f in sorted(glob.glob(os.path.join(SRC, "*.json"))):
    data = json.load(open(f))
    json.dump(data[:N], open(os.path.join(DST, os.path.basename(f)), "w"),
              separators=(",", ":"))
PY
```

The JSON runner reads this committed sample by default; if the full
`external/spc700-singlestep-full/` set is present it is picked up for the
exhaustive local run (per the loader contract in
`to-dos/phase-0-foundation/rom-seeding-runbook.md`).
