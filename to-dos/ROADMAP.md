# RustySNES — Roadmap

Entry point for project planning. Each phase below links to its overview; each phase contains
sprints; each sprint contains tickets with stable IDs `T-PS-NNN` (P = phase, S = sprint).
Reference ticket IDs in commit messages. `docs/STATUS.md` is the authoritative current-state
record; this file frames the phase line.

## Status

- **Current phase:** Phase 2 — Scheduler + video **complete**. The master-clock lockstep
  scheduler + bus + DMA/HDMA + dual-chip PPU + base cart mappers boot and run real ROMs: gilyon
  `cputest-basic` → "Success" (all 1107 CPU tests), undisbeliever PPU/DMA/HDMA → 29/29
  bit-deterministic golden framebuffers. Phases 0–1 done (65C816 oracle 0-diff,
  5,119,999/5,120,000). **Next: Phase 3 — Audio (SPC700 + S-DSP).**
- **Release:** v0.1.0 — CPU oracle 0-diff; PPU/scheduler/cart boot real ROMs; APU + coprocessors
  not started.

## The phase spine

The order is chosen so each layer rests on a verified one below it (the cycle-accurate-emulator
build spine).

### Phase 0 — Foundation

**Goal:** the Cargo workspace + one-directional crate skeletons compile; CI green on stubs;
`tests/roms/` seeded with the permissive suites; the test-harness skeleton stands up.
**Exit:** `cargo check --workspace` + `cargo test --workspace` (stubs) green in CI.
→ [overview](phase-0-foundation/overview.md)

### Phase 1 — CPU + golden oracle

**Goal:** the 65C816 core passes the SingleStepTests/65816 per-opcode oracle (every opcode ×
addressing mode, 8/16-bit, native + emulation) and the gilyon CPU ROMs.
**Exit:** CPU per-opcode oracle 0-diff (gated on the 65816 license); gilyon CPU tables green.
→ [overview](phase-1-cpu-golden-log/overview.md)

### Phase 2 — Scheduler + video

**Goal:** the master-clock lockstep scheduler (the 6/8/12 access map + 1360/1364/1368 lines)
and the PPU to a stable rendered frame; the PPU/DMA/HDMA test ROMs; a deterministic golden
framebuffer.
**Exit:** undisbeliever PPU/DMA/HDMA suite green; a deterministic golden framebuffer for a
known ROM.
→ [overview](phase-2-scheduler-video/overview.md)

### Phase 3 — Audio (SPC700 + S-DSP + the async resync)

**Goal:** the SPC700, S-DSP, ARAM, and the integer-accumulator async resync; the audio oracle.
**Exit:** SingleStepTests/spc700 0-diff; blargg `spc_*` green to the achievable bar;
deterministic golden audio.
→ [overview](phase-3-audio/overview.md)

### Phase 4 — Carts + coprocessors (Core tier first)

**Goal:** the LoROM/HiROM/ExHiROM memory model + header detect, then the Core/Curated
coprocessors (DSP-1 via the shared µPD77C25 core, Super FX, SA-1). Tier + honesty gate from
the first board.
**Exit:** the map models + Core/Curated coprocessors boot + pass their tests; honesty gate
green (`docs/adr/0003`).
→ [overview](phase-4-carts-mappers/overview.md)

### Phase 5 — Frontend

**Goal:** the always-on egui shell (menu/status/Settings + debugger panels), the audio ring +
pacing, gamepads, save-states, rewind, run-ahead, the wasm build.
**Exit:** playable native + wasm; the frontend determinism path intact.
→ [overview](phase-5-frontend/overview.md)

### Phase 6 — Accuracy to target

**Goal:** drive the composed two-layer accuracy battery to ≥90% (100% the goal); identify the
hard-tier residuals and decide which defer to the fractional-timebase refactor.
**Exit:** accuracy battery at target; residuals documented + deferred, not point-fixed
(`docs/adr/0002`).
→ [overview](phase-6-accuracy-to-100/overview.md)

### Phase 7 — Breadth

**Goal:** the remaining BestEffort coprocessors + niche peripherals; region timing as data.
**Exit:** the full coprocessor / board matrix in `docs/STATUS.md`.
→ [overview](phase-7-breadth/overview.md)

### Phase 8 — Reach (additive, off-by-default)

**Goal:** rollback netplay, RetroAchievements, TAS movies, Lua scripting, a shader ecosystem —
each behind a default-off feature, each byte-identical with the feature off.
**Exit:** features ship; shipped/native/no_std/wasm byte-identical.
→ [overview](phase-8-reach/overview.md)

## Milestones beyond the phases

- **v1.0.0 — production cut.** All of the above; README / CHANGELOG / docs / STATUS in sync;
  the release matrix (cross-platform binaries) + Pages (wasm demo + rustdoc) green.
- **Beyond v1.0 — the fractional-timebase refactor (`docs/adr/0002`).** *Only if* the hard-tier
  residuals warrant it: the one-clock + every-cycle-bus-access collapse (a fractional master
  clock with a φ1/φ2 split). **The one release expected to break byte-identity / save-state
  compatibility.** Do NOT conflate it with "the master clock already exists (the Phase-0
  scheduler)" — the RustyNES versioning trap.

## Cross-phase dependencies

- Phase 2 (scheduler) depends on Phase 1 (the CPU drives the scheduler's access-speed query).
- Phase 3 (audio resync) depends on Phase 2's scheduler (the once-per-scanline forced sync).
- Phase 4's SA-1 reuses the Phase-1 65C816 core (a second instance).
- Phase 6 (accuracy) depends on Phases 1–4 being feature-complete enough to run the full
  battery.
- Phase 8 (netplay / TAS) depends on the determinism contract (`docs/adr/0004`) being
  exercised in Phase 5.

## Open questions blocking planning

- **The 65816 JSON oracle ships no license** — secure permission, gitignore it, or
  self-generate equivalent JSON. This blocks gating Phase 1's primary oracle in CI
  (`docs/testing-strategy.md` §licensing; `ref-docs/research-report.md` "Open questions" #1).
- Per-board SRAM / coprocessor bus windows have no canonical table — built incrementally in
  Phase 4 from the cartridge database + ares board definitions.
