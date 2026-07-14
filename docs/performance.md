# Performance — RustySNES

**References:** `ref-docs/research-report.md` "Principal engineering challenges";
`docs/scheduler.md`; `docs/architecture.md`.

## Targets

- **Headless core:** ≤ ~2 ms per emulated frame (the RustyNES headless budget) so the frontend
  has slack for present + run-ahead + netplay rollback. The SNES does more per frame than the
  NES (two CPUs, the variable cycle, HDMA), so treat 2 ms as a target to defend, not a given.
- **Real-time:** sustain 60.0988 Hz (NTSC) / 50.0070 Hz (PAL) frame pacing in the frontend
  without underruns on the lock-free audio ring.

## Hot paths (where the cost concentrates)

Per `ref-docs/research-report.md` §§1–5, the per-cycle cost is dominated by:

1. **The CPU bus access dispatch** — every CPU cycle queries the region map for its 6/8/12
   speed (`docs/scheduler.md` §access-speed-map). Keep this a branch-light table lookup, not a
   match cascade.
2. **PPU pixel emission** — the per-dot background / sprite / color-math pipeline at 341 dots
   × 262 lines. This is the analog of RustyNES's "PPU pixel emission" hot path.
3. **The SPC700 resync accumulator** — cheap per step, but the per-port-access and
   per-scanline sync points are on the hot path (`docs/apu.md` §async-resync). Keep the
   accumulator arithmetic integer and inline.
4. **DMA/HDMA byte loops** — content-dependent; the per-line HDMA budget (≤466 clk) must not
   allocate.

## Rules (from `docs/architecture.md` + CLAUDE.md conventions)

- Hot paths (`Cpu::step`, `Ppu::tick`, the resync, mapper register access): **no allocations**,
  prefer fixed arrays, profile (`cargo bench` + `perf record`) **before** adding abstractions.
- Measure first: never optimize without a Criterion baseline; gate any "perf" change on a
  ≥ measurable Criterion delta + byte-identical output (the determinism contract,
  `docs/adr/0004`).
- `release` profile is `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` (see
  `Cargo.toml`).

## Profiling plan

- `cargo bench -p rustysnes-cpu` / `-ppu` / `-apu` / `-core` (Criterion) per crate. The
  integration-level headless-frame benchmark is landed
  (`crates/rustysnes-core/benches/headless_frame.rs`) — see `docs/benchmarks.md` for the actual
  measured number and how to reproduce it. Per-crate hot-path benchmarks (CPU dispatch, PPU
  per-dot emission, the SPC700 resync) are not yet split out.
- `perf record` on a headless replay of a known ROM for the integration hot path — not yet run;
  `docs/benchmarks.md`'s current baseline is Criterion wall-clock only, no flamegraph yet.
- **Landed (`v1.0.0`):** a frame-time regression gate in CI (`.github/workflows/ci.yml`'s `bench`
  job, `scripts/bench_regression_check.sh`), mirroring the RustyNES pattern — runs
  `headless_frame` on release-tag pushes and asserts the steady-state mean stays under an
  absolute 10 ms/frame ceiling (~60% of the 16.64 ms NTSC deadline, ~3x the measured `v0.4.0`
  baseline). An absolute ceiling, not a tight %-regression check, deliberately — shared CI
  runners vary by tens of percent run-to-run, so a percentage gate would flake; use local
  Criterion `--save-baseline`/`--baseline` comparisons (the script's own header comment) for a
  tighter before/after read.
- **Landed (`v1.19.0 "Afterburner"`):** an optional PGO/BOLT pipeline for the shipping `rustysnes`
  binary — `scripts/pgo/run.sh` (instrument → train against the committed permissive ROM corpus
  via `crates/rustysnes-test-harness/src/bin/pgo_trainer.rs` → optimized rebuild) plus
  `.github/workflows/pgo.yml` (`workflow_dispatch` + release-tag push; **never on the PR gate** —
  an instrument+train+rebuild cycle is far too slow for that). Promotion requires **both**: a
  measured `> 3%` `headless_frame_steady_state` Criterion speedup over the plain release build on
  the same runner, **and** a byte-identical re-run of the full `--features test-roms` oracle under
  the PGO-merged profile (cites `docs/adr/0004`'s determinism contract — PGO changes inlining/code
  layout, never proves correctness by assertion). An optional Linux-only BOLT post-link stage
  chains onto an already-promoted PGO binary, best-effort (skips cleanly if `llvm-bolt` is
  unavailable on the runner). Verified for real in this development environment: the full
  instrument → train (60 frames × the 5-ROM committed corpus) → optimized-rebuild pipeline
  produces a genuine, running `rustysnes` binary, and `cargo pgo optimize test -- --workspace
  --release --features test-roms` (scoped to `rustysnes-test-harness` locally) passes cleanly
  under the PGO-merged profile — but the A/B speedup on that short local run did **not** clear the
  `>3%` bar (a `headless_frame_steady_state` mean of ~3.65 ms either way), so promotion is
  correctly gated off locally; whether CI's real `3600`-frame training run clears the bar on a
  release tag is left to `pgo.yml`'s own gate to decide honestly, not asserted here.

## Open questions

- Whether the variable-cycle dispatch wants a precomputed per-bank speed table (256 banks ×
  region) vs a computed predicate — benchmark in Phase 1.
- Whether the SPC resync's once-per-scanline forced sync is frequent enough to bound latency
  without hurting throughput — tune in Phase 3.
