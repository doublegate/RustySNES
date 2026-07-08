# RustySNES — Roadmap

Entry point for project planning. Each phase below links to its overview; each phase contains
sprints; each sprint contains tickets with stable IDs `T-PS-NNN` (P = phase, S = sprint).
Reference ticket IDs in commit messages. `docs/STATUS.md` is the authoritative current-state
record; this file frames the phase line.

## Status

- **Current phase:** Phases 0–3 **complete** (CPU oracle 0-diff, scheduler + video, audio
  0-diff). Phase 4 (Core/Curated coprocessors: DSP-1, Super FX, SA-1) **complete**. Phase 7
  (BestEffort coprocessors) **mostly complete**: DSP-2/DSP-4/ST010/S-DD1/CX4/OBC1 implemented +
  validated against real commercial ROMs; SPC7110 implemented but not yet booting to real
  content; ST018 and standalone S-RTC not started; PAL region auto-detection is implemented and
  validated end-to-end (`Bus::sync_region_from_cart`; no golden-ROM-boot proof yet — no PAL ROM
  in the local corpus); ExLoROM is implemented (decode formula sourced from bsnes's own runtime
  board database; no golden-ROM-boot proof — no ExLoROM ROM in the local corpus). Phase 5
  (frontend) **partially complete**: the
  native+wasm shell is playable (video/audio/input/ROM-load wired), save-states are **fully
  implemented** (`v0.2.0 "Persistence"`, `docs/adr/0006` — every subsystem round-trips its exact
  state through one versioned envelope, proven by a round-trip determinism test), and rewind +
  run-ahead (`v0.3.0 "Continuum"`, `crate::rewind` — a bounded ring buffer of full snapshots +
  N-frame peek-and-discard, both config-driven and off by default) are now **fully implemented**
  — the frontend orchestration Phase 8 (netplay, TAS movies) will build on. Phase 6 (accuracy
  push) and Phase 8
  (netplay/RetroAchievements/scripting — all three crates are still 1-line stubs) have not
  started. See `docs/STATUS.md` for the authoritative per-subsystem table this line summarizes.
- **Release:** `v0.1.0 "Foundation"` and `v0.2.0 "Persistence"` are tagged and released on
  GitHub, establishing the real release cadence `to-dos/VERSION-PLAN.md` defines — read it
  alongside this file; it maps the phases above onto a concrete, named `v0.x.0` → `v1.0.0`
  ladder with release-cut criteria per rung. `v0.3.0 "Continuum"` (rewind, run-ahead, PAL/
  ExLoROM completion) is **complete** — all four line items (rewind, run-ahead, PAL auto-detect,
  ExLoROM) have landed. Ready to tag.

## The phase spine

The order is chosen so each layer rests on a verified one below it (the cycle-accurate-emulator
build spine).

### Phase 0 — Foundation ✅ complete

**Goal:** the Cargo workspace + one-directional crate skeletons compile; CI green on stubs;
`tests/roms/` seeded with the permissive suites; the test-harness skeleton stands up.
**Exit:** `cargo check --workspace` + `cargo test --workspace` (stubs) green in CI.
→ [overview](phase-0-foundation/overview.md)

### Phase 1 — CPU + golden oracle ✅ complete

**Goal:** the 65C816 core passes the SingleStepTests/65816 per-opcode oracle (every opcode ×
addressing mode, 8/16-bit, native + emulation) and the gilyon CPU ROMs.
**Exit:** CPU per-opcode oracle 0-diff (gated on the 65816 license); gilyon CPU tables green.
→ [overview](phase-1-cpu-golden-log/overview.md)

### Phase 2 — Scheduler + video ✅ complete (mid-line raster deferred to Phase 6)

**Goal:** the master-clock lockstep scheduler (the 6/8/12 access map + 1360/1364/1368 lines)
and the PPU to a stable rendered frame; the PPU/DMA/HDMA test ROMs; a deterministic golden
framebuffer.
**Exit:** undisbeliever PPU/DMA/HDMA suite green; a deterministic golden framebuffer for a
known ROM.
→ [overview](phase-2-scheduler-video/overview.md)

### Phase 3 — Audio (SPC700 + S-DSP + the async resync) ✅ complete

**Goal:** the SPC700, S-DSP, ARAM, and the integer-accumulator async resync; the audio oracle.
**Exit:** SingleStepTests/spc700 0-diff; blargg `spc_*` green to the achievable bar;
deterministic golden audio.
→ [overview](phase-3-audio/overview.md)

### Phase 4 — Carts + coprocessors (Core tier first) ✅ complete

**Goal:** the LoROM/HiROM/ExHiROM memory model + header detect, then the Core/Curated
coprocessors (DSP-1 via the shared µPD77C25 core, Super FX, SA-1). Tier + honesty gate from
the first board.
**Exit:** the map models + Core/Curated coprocessors boot + pass their tests; honesty gate
green (`docs/adr/0003`).
→ [overview](phase-4-carts-mappers/overview.md)

### Phase 5 — Frontend 🚧 partial — the shell is playable; save-states/rewind/run-ahead landed; the full wasm frontend (Sprint 4) remains

**Goal:** the always-on egui shell (menu/status/Settings + debugger panels), the audio ring +
pacing, gamepads, save-states, rewind, run-ahead, the wasm build.
**Exit:** playable native + wasm; the frontend determinism path intact.
**Release mapping:** the playable shell shipped inside the retroactive `v0.1.0` tag
(`to-dos/VERSION-PLAN.md`); save-states shipped in `v0.2.0`; rewind/run-ahead shipped in
`v0.3.0`; the full wasm frontend (Sprint 4) remains.
→ [overview](phase-5-frontend/overview.md)

### Phase 6 — Accuracy to target 🚧 not started as a dedicated push

**Goal:** drive the composed two-layer accuracy battery to ≥90% (100% the goal); identify the
hard-tier residuals and decide which defer to the fractional-timebase refactor. Un-defer the
Phase-2 mid-line-raster gap here.
**Exit:** accuracy battery at target; residuals documented + deferred, not point-fixed
(`docs/adr/0002`).
**Release mapping:** `v0.5.0` (`to-dos/VERSION-PLAN.md`).
→ [overview](phase-6-accuracy-to-100/overview.md)

### Phase 7 — Breadth 🚧 mostly complete

**Goal:** the remaining BestEffort coprocessors + niche peripherals; region timing as data.
**Exit:** the full coprocessor / board matrix in `docs/STATUS.md`.
**Status:** DSP-2/DSP-4/ST010/S-DD1/CX4/OBC1 done + validated; SPC7110 implemented but not
booting; ST018 and standalone S-RTC not started; PAL region auto-detection and ExLoROM are both
implemented (each with a documented, honest validation gap — no PAL ROM and no ExLoROM ROM
exist in the local corpus, so neither has golden-framebuffer proof).
**Release mapping:** the done work shipped inside `v0.1.0`; PAL auto-detect and ExLoROM landed
inside `v0.3.0 "Continuum"` alongside rewind/run-ahead (all four line items complete); the
remainder is a PAL/ExLoROM golden-boot proof if a real ROM ever surfaces (`v0.3.x`, opportunistic
— not gating) and `v0.4.0` (SPC7110 fix, ST018, standalone S-RTC).
→ [overview](phase-7-breadth/overview.md)

### Phase 8 — Reach (additive, off-by-default) 🚧 not started (all three crates are 1-line stubs)

**Goal:** rollback netplay, RetroAchievements, TAS movies, Lua scripting, a shader ecosystem —
each behind a default-off feature, each byte-identical with the feature off.
**Exit:** features ship; shipped/native/no_std/wasm byte-identical.
**Release mapping:** entirely post-`v1.0.0` — see `to-dos/VERSION-PLAN.md`'s `v1.1.0`+ ladder.
This phase does not gate `v1.0.0` (see "Milestones" below).
→ [overview](phase-8-reach/overview.md)

## Milestones beyond the phases

- **v1.0.0 — production cut.** Deliberately **not** gated on Phase 8 (feature breadth) —
  gated on: the accuracy battery holding its Phase-6 target with no regressions, a **stable**
  save-state/core API (Phase 5), and a genuinely shippable multi-platform app (the release
  matrix + wasm/Pages, both exercised end-to-end for the first time in `v0.6.0`). README /
  CHANGELOG / docs / STATUS in sync. See `to-dos/VERSION-PLAN.md` for the full rationale (it
  mirrors how RustyNES gated its own v1.0.0 — accuracy + API stability + shippability, not
  mapper/feature count).
- **Post-v1.0 — Phase 8 ships as named, themed minors** (`v1.1.0` scripting/debugger,
  `v1.2.0` netplay, `v1.3.0` RetroAchievements, `v1.4.0` TAS movies, `v1.5.0`+ shaders/cheats/
  Libretro) — see `to-dos/VERSION-PLAN.md`.
- **Beyond that — the fractional-timebase refactor (`docs/adr/0002`).** *Only if* the hard-tier
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
