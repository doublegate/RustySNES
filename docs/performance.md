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

- `cargo bench -p rustysnes-cpu` / `-ppu` / `-apu` / `-core` (Criterion) per crate.
- `perf record` on a headless replay of a known ROM for the integration hot path.
- A perf-capture regression gate in CI once the core is real (mirrors the RustyNES pattern).

## Open questions

- Whether the variable-cycle dispatch wants a precomputed per-bank speed table (256 banks ×
  region) vs a computed predicate — benchmark in Phase 1.
- Whether the SPC resync's once-per-scanline forced sync is frequent enough to bound latency
  without hurting throughput — tune in Phase 3.
