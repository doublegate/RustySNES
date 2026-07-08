# Benchmarks — RustySNES

The reproducible benchmark record — actual measured numbers, distinct from `docs/performance.md`
(targets and rules). Landed as part of `v0.6.0 "Shippable"`'s CI/docs-depth push
(`to-dos/VERSION-PLAN.md`), mirroring RustyNES's own `docs/benchmarks.md`.

## `v0.4.0` baseline — headless frame throughput

**First measurement taken against this codebase — establishes the baseline every future change
is compared to, not a claim of having hit the target yet.**

| Metric | Result |
|---|---|
| Headless frame time (steady state) | **3.27 ms** [3.2678, 3.2782] ms, 95% CI |
| Target (`docs/performance.md`) | ≤ ~2 ms |
| Gap | **~1.6×** over target |
| Real-time budget (NTSC, 60.0988 Hz) | 16.64 ms/frame |
| Headroom vs. real-time | ~5.1× |

**Honest reading:** the headless core is well within real-time (real-time playback is not at
risk), but the `docs/performance.md` ≤2ms target — set to leave slack for present + run-ahead +
netplay rollback layered on top in the frontend — is not yet met. This is the first-ever
measurement on this codebase (no prior baseline existed to regress against), so there is no A/B
story yet; that starts now, with this number as the anchor.

### Reproduction

```bash
cargo bench -p rustysnes-core --bench headless_frame
```

- **Benchmark:** `crates/rustysnes-core/benches/headless_frame.rs` (Criterion 0.7, `harness =
  false`).
- **Workload:** `tests/roms/undisbeliever/inidisp_hammer_0f00.sfc` (Zlib-licensed, committed —
  chosen for having no coprocessor/DMA-heavy content, so the measurement isolates the base
  CPU+PPU+scheduler cost rather than a specific board's own overhead). 16 warm-up frames past the
  ROM's own boot sequence, then Criterion's standard 100-sample steady-state collection.
- **Machine:** 20-thread x86_64 (Linux, CachyOS kernel `7.1.3-1-cachyos`), `rustc 1.96.1`,
  workspace `release` profile (`opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic =
  "abort"`).
- **Captured:** 2026-07-08, on `v0.4.0 "Completion"` + the in-progress `v0.5.0`/`v0.6.0`
  documentation work.

### Where the cost concentrates (from `docs/performance.md`'s already-identified hot paths)

Not yet profiled in this pass (`perf record` against this same headless replay is the natural
next step, `docs/performance.md` §Profiling plan) — the four candidates already named there (the
CPU bus access-speed dispatch, PPU per-dot pixel emission, the SPC700 resync accumulator, DMA/HDMA
byte loops) haven't been individually measured yet. This benchmark establishes the end-to-end
number the next optimization pass is measured against; per `docs/performance.md`'s own rule
("never optimize without a Criterion baseline"), that baseline is now this document.

## Future measurements

Add a new dated section above this one for each future benchmark run that's worth recording
(a real optimization landing, a new hot-path suspect, or a periodic re-baseline) — never edit a
past section's numbers in place; a regression or improvement is a new row, so the history stays
auditable (the same "immutable reference corpus, corrections land as new dated entries" posture
`master-core`'s docs module already applies elsewhere in this project).
