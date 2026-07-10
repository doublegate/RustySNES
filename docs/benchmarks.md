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

## `v0.8.0` pre-work — save-state cost (netplay rollback go/no-go, T-82-001)

**The question this answers:** rollback netplay (T-82-002) calls `System::save_state()`/
`load_state()` far more often than `RewindBuffer`'s ~10 Hz design point (`docs/adr/0006` frames
delta/incremental snapshots as "future memory optimization, not correctness requirement" — this
is the measurement that either confirms that framing holds for rollback's higher call rate, or
forces reopening it before T-82-002 starts).

| Board tier | ROM | `save_state()` | `load_state()` |
|---|---|---|---|
| No coprocessor | `undisbeliever/inidisp_hammer_0f00.sfc` (committed) | **107.87 µs** [107.54, 108.25] µs, 95% CI | **292.07 µs** [290.66, 293.45] µs, 95% CI |
| Curated (Super FX) | `Super Mario World 2: Yoshi's Island` (commercial, gitignored) | **107.73 µs** [107.21, 108.29] µs | **297.08 µs** [295.43, 298.97] µs |
| BestEffort (CX4) | `Mega Man X2` (commercial, gitignored) | **109.15 µs** [108.46, 109.89] µs | **295.53 µs** [294.21, 296.96] µs |

**Go/no-go call: GO — the existing full-snapshot design (`docs/adr/0006`) is fast enough for a
real rollback window; no delta/incremental redesign is needed before T-82-002 proceeds.**

**Reasoning:** all three board tiers cluster tightly (~108 µs save, ~295 µs load) regardless of
which coprocessor is active — save-state cost is dominated by the fixed-size buffers every board
carries (128 KiB WRAM, VRAM, CGRAM, OAM, 64 KiB ARAM), not coprocessor-specific state, so this
result generalizes to boards not directly measured here. Both numbers are small next to a single
frame's own execution cost (**3.27 ms**, the `v0.4.0` baseline above): a `save_state()` every
real frame (the naive worst case — snapshot-every-predicted-frame) costs **~0.65%** of the 16.64
ms NTSC frame budget, negligible overhead even at that call rate. A rollback event itself (rare —
only on misprediction) costs one `load_state()` (~0.3 ms) plus re-simulating the rolled-back
frames forward (`headless_frame`'s own ~3.27 ms/frame, ~5.1× headroom under real-time already
established) — the resimulation dominates cost, not the snapshot mechanism, so it is that
existing per-frame number (not save/load) that bounds how many frames a rollback window can
absorb within one real frame's budget. Save/load overhead itself is not the bottleneck.

### Reproduction

```bash
cargo bench -p rustysnes-core --bench save_state_cost
```

- **Benchmark:** `crates/rustysnes-core/benches/save_state_cost.rs` (Criterion 0.7, `harness =
  false`). The Curated/BestEffort tiers self-skip (print a message, register no bench) when
  their commercial ROM is absent — `tests/roms/external/commercial/` is gitignored
  (`docs/adr/0003`), present on this measuring machine, absent on a fresh clone/CI.
- **Workload:** 16 warm-up frames past each ROM's own boot sequence (matching
  `headless_frame.rs`), then Criterion's standard 100-sample steady-state collection of
  `save_state()` alone, then `load_state()` of that same blob.
- **Machine/toolchain:** same as the `v0.4.0` baseline above.
- **Captured:** 2026-07-09, on `v0.8.0 "Instrumentation"` (immediately after Sprint 1 landed),
  ahead of `v0.8.0 "Community"` Sprint 2 starting.

## Future measurements

Add a new dated section above this one for each future benchmark run that's worth recording
(a real optimization landing, a new hot-path suspect, or a periodic re-baseline) — never edit a
past section's numbers in place; a regression or improvement is a new row, so the history stays
auditable (the same "immutable reference corpus, corrections land as new dated entries" posture
`master-core`'s docs module already applies elsewhere in this project).
