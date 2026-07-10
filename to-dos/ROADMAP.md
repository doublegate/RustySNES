# RustySNES — Roadmap

Entry point for project planning. Each phase below links to its overview; each phase contains
sprints; each sprint contains tickets with stable IDs `T-PS-NNN` (P = phase, S = sprint).
Reference ticket IDs in commit messages. `docs/STATUS.md` is the authoritative current-state
record; this file frames the phase line.

## Status

- **Current phase:** Phases 0–3 **complete** (CPU oracle 0-diff, scheduler + video, audio
  0-diff). Phase 4 (Core/Curated coprocessors: DSP-1, Super FX, SA-1) **complete**. Phase 7
  (BestEffort coprocessors) **mostly complete**: DSP-2/DSP-4/ST010/S-DD1/CX4/OBC1/standalone
  S-RTC implemented + validated (S-RTC unit-tested only, no commercial dump available); SPC7110
  implemented with a confirmed, fixed addressing bug that materially improved but did not fully
  resolve its boot crash (`docs/cart.md` §SPC7110); ST018 is now implemented (unit-tested only,
  no commercial dump available); PAL region auto-detection
  is implemented and
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
  push) **dashboard + triage complete, fixes carried forward** (see the Phase 6 section below —
  the accuracy-pass-rate dashboard is done and every named hardware-gotcha item is triaged with
  evidence, but the confirmed mid-line-raster fix and the accuracy-percentage push itself remain
  open). Phase 8 Sprint 1 (`v0.8.0 "Instrumentation"`) is **complete**: the debugger overlay
  (T-81-001, live-state panels landed; a 65C816 disassembler + PC breakpoints/step controls
  remain an open follow-up despite an earlier sprint-doc checkbox claiming otherwise — no such
  code exists yet, corrected here rather than left stale; read/write watchpoints deferred to a
  follow-up, T-81-001b, which landed post-Sprint-2 in `v0.8.0` — a new `debug-hooks` feature on
  `rustysnes-core` itself + a `Bus`-level hook + the debugger's Watch panel), sandboxed Lua
  scripting + TAS movie record/playback
  (T-81-002, `rustysnes-script` + `rustysnes_core::movie`), Game Genie/Pro Action Replay
  cheat codes (T-81-003, `rustysnes_core::cheat` + a `Bus::read24` intercept), the extended
  byte-identical-with-flags-off CI gate (T-81-004), and the full wasm frontend (T-81-005
  `wasm-canvas`, T-81-006 `wasm-winit` unification) have all landed. Sprint 2
  (`v0.8.0 "Community"`: netplay, RetroAchievements) is **in scope for the `v0.8.0` rung gating
  `v1.0.0`**, not deferred post-1.0 (see "Milestones beyond the phases" below — this reverses the
  prior post-1.0 framing, matching what RustyNES actually shipped in its own v1.0.0), but has not
  started. See `docs/STATUS.md` for the authoritative per-subsystem table this line summarizes.
- **Release:** `v0.1.0 "Foundation"`, `v0.2.0 "Persistence"`, `v0.3.0 "Continuum"` (rewind,
  run-ahead, PAL auto-detect, ExLoROM), `v0.4.0 "Completion"` (SPC7110 addressing fix, ST018,
  standalone S-RTC), `v0.5.0 "Fidelity"` (the accuracy-pass-rate dashboard + the full named
  hardware-gotcha regression list — every item fixed, correctly reclassified as an intentional
  non-goal, or honestly researched-and-deferred with a mechanism write-up), `v0.6.0 "Shippable"`
  (release engineering + doc parity — `security.yml`, checksummed release assets, automated
  release-cutting via `release-auto.yml`, the `lint` job's `cargo doc` gate, the documentation
  index, benchmarks, audit trail, and ADR backfill), and `v0.7.0 "Resolution"` (true 512-px
  hi-res Modes 5/6 output, a genuine one-pixel-clock-delayed DAC pipeline verified against ares'
  primary source; the save-state `FORMAT_VERSION`'s first real bump, closing the `v1.0.0` gate's
  backward-compat-fixture item early) are all tagged and released on GitHub, establishing the
  real release cadence `to-dos/VERSION-PLAN.md` defines — read it alongside this file; it maps
  the phases above onto a concrete, named `v0.x.0` → `v1.0.0` ladder with release-cut criteria
  per rung, rewritten with `v0.7.0` to front-load Phase 8 breadth into the `v1.0.0` gate rather
  than deferring it post-1.0 (matching what RustyNES actually shipped in its own v1.0.0 — see
  `to-dos/VERSION-PLAN.md`'s "Second reversal"). The mid-scanline/HDMA-driven register timing fix
  and the open-bus-via-HDMA-latch investigation remain open, carried forward as an ongoing,
  opportunistic `v0.x.y`-patch cluster rather than gating a numbered rung (`to-dos/VERSION-PLAN.md`).
  `v0.8.0 "Instrumentation"` (debugger, scripting/TAS, cheats) is next.

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

### Phase 6 — Accuracy to target ✅ dashboard + triage complete, fixes carried forward

**Goal:** drive the composed two-layer accuracy battery to ≥90% (100% the goal); identify the
hard-tier residuals and decide which defer to the fractional-timebase refactor. Un-defer the
Phase-2 mid-line-raster gap here.
**Status:** the accuracy-pass-rate dashboard is done (`docs/STATUS.md` §Accuracy dashboard) and
every named hardware-gotcha item has been triaged with evidence — fixed (a real HDMA dot-phase
doc/code drift), correctly reclassified as an intentional non-goal (`$4203`/`$4206`, the
"DMA/HDMA-collision crash quirk"), or honestly researched-and-deferred (DRAM refresh,
open-bus-via-HDMA-latch, hi-res color-math precision). The Phase-2 mid-line-raster gap is
**confirmed real, a fix is designed and prototyped, but NOT landed**: the prototype
(`rustysnes-ppu` compositing each line at the hardware-correct `RENDER_DOT` instead of the
line's end) is independently verified correct for the CPU/HDMA-driven case (SA-1's
`SD F-1 Grand Prix` golden change, pixel-verified as a real improvement), but the same change
breaks all 24 Super FX/GSU golden tests for reasons not yet understood — the identical failure
signature the sibling open-bus-via-HDMA-latch investigation also hit and correctly did not land
(`docs/ppu.md` §Mid-scanline/HDMA-driven register timing has the full mechanism, both
verifications, and what a future investigation needs).
**Exit:** accuracy battery at target; residuals documented + deferred, not point-fixed
(`docs/adr/0002`). **Not yet met** — see Status above.
**Release mapping:** `v0.5.0` (`to-dos/VERSION-PLAN.md`), triage complete; the one bounded
residual (true hi-res Modes 5/6 output) closes in `v0.7.0 "Resolution"`; the rest (mid-scanline/
GSU, open-bus-via-HDMA-latch, SPC7110, DRAM refresh, ROM-dump-gated validation) carries forward
as an ongoing, opportunistic `v0.x.y`-patch cluster, not a gating rung.
→ [overview](phase-6-accuracy-to-100/overview.md)

### Phase 7 — Breadth 🚧 mostly complete

**Goal:** the remaining BestEffort coprocessors + niche peripherals; region timing as data.
**Exit:** the full coprocessor / board matrix in `docs/STATUS.md`.
**Status:** DSP-2/DSP-4/ST010/S-DD1/CX4/OBC1/standalone S-RTC done + validated (S-RTC unit-tested
only — no commercial dump available); SPC7110 implemented, a confirmed addressing bug fixed
(materially improved, still not booting to real content — `docs/cart.md` §SPC7110); ST018 is
now implemented (unit-tested only, no commercial dump available); PAL region auto-detection and ExLoROM are both implemented (each with a documented,
honest validation gap — no PAL ROM and no ExLoROM ROM exist in the local corpus, so neither has
golden-framebuffer proof).
**Release mapping:** the done work shipped inside `v0.1.0`; PAL auto-detect and ExLoROM landed
inside `v0.3.0 "Continuum"` alongside rewind/run-ahead (all four line items complete); standalone
S-RTC, the SPC7110 addressing fix, and ST018 all land inside `v0.4.0 "Completion"` (now
complete); the PAL/ExLoROM golden-boot proof remains opportunistic (`v0.3.x`, not gating) if a
real ROM ever surfaces.
→ [overview](phase-7-breadth/overview.md)

### Phase 8 — Instrumentation + Community (additive, off-by-default) 🚧 Sprint 1 complete, Sprint 2 not started

**Goal:** debugger overlay, Lua scripting + TAS movies, cheat-code support, rollback netplay,
and RetroAchievements — each behind a default-off feature, each byte-identical with the feature
off. **As of this update, this phase gates `v1.0.0`** (reversed from the earlier post-1.0
framing — see "Second reversal" in `to-dos/VERSION-PLAN.md`'s intro): RustyNES front-loaded this
exact breadth into its own v1.0.0 rather than deferring it, and matching that bar means Phase 8
lands before the production cut, not after it. A shader ecosystem/Libretro core remain
post-`v1.0.0` Reach — RustyNES doesn't have HD texture packs either, so `hd-pack` stays
deliberately out of the parity target.
**Status:** Sprint 1 (`v0.8.0 "Instrumentation"`) is done — the debugger overlay,
`rustysnes-script` (Lua scripting + TAS movies), `rustysnes_core::cheat` (Game Genie/Pro Action Replay),
the extended byte-identical-with-flags-off CI gate, and the full wasm frontend all landed
(T-81-001 through T-81-006). Sprint 2 (`v0.8.0 "Community"`: netplay, RetroAchievements) has not
started; `rustysnes-netplay`/`rustysnes-cheevos` are still 1-line stubs.
**Exit:** features ship; shipped/native/no_std/wasm byte-identical with every new flag off
(the byte-identical-with-all-flags-off CI gate, added starting `v0.8.0` and re-verified through
`v0.8.0`/`v1.0.0`).
**Release mapping:** `v0.8.0 "Instrumentation"` (debugger, scripting/TAS, cheats) then
`v0.8.0 "Community"` (netplay, RetroAchievements) — see `to-dos/VERSION-PLAN.md` for the full
per-item breakdown, including the `Board: Send`/`emu-thread` prerequisite and the netplay
save-state-cost pre-work.
→ [overview](phase-8-reach/overview.md)

## Milestones beyond the phases

- **v0.7.0 "Resolution".** True 512-px hi-res (Modes 5/6) output — the one bounded item left on
  Phase 6's residual list; the rest of that list (mid-scanline/GSU, open-bus-via-HDMA-latch,
  SPC7110, DRAM refresh, ST018/S-RTC/PAL/ExLoROM real-ROM validation) stays an ongoing,
  opportunistic `v0.x.y`-patch cluster, not a gating rung — see `to-dos/VERSION-PLAN.md`.
- **v0.8.0 "Instrumentation" / v0.8.0 "Community" — Phase 8, gating `v1.0.0`.** See the Phase 8
  section above.
- **v1.0.0 — production cut.** Gated on: the accuracy battery holding its Phase-6 target with no
  regressions; a **stable, backward-compat-fixture-proven** save-state/core API (Phase 5); the
  full Phase 8 breadth landed and byte-identical with flags off; a genuinely shippable
  multi-platform app (the release matrix + wasm/Pages, both exercised end-to-end since `v0.6.0`)
  plus a new frame-time performance-regression CI gate; a desktop UX shell at RustyNES's
  maturity bar (thumbnail save-state manager, input rebinding, themes, speed presets, a
  Performance panel, the dedicated `emu-thread`); the README rewrite; README / CHANGELOG / docs /
  STATUS in sync. See `to-dos/VERSION-PLAN.md` for the full rationale and per-item detail.
- **Beyond that — Reach (deferred):** a Libretro core, a shader/filter pipeline (CRT/HQ2x), HD
  texture packs (`hd-pack`), and any future mobile/Android target (no appetite assumed by
  default) — see `to-dos/VERSION-PLAN.md`'s "Post-v1.0 — Reach".
- **Further beyond — the fractional-timebase refactor (`docs/adr/0002`).** *Only if* the
  hard-tier residuals warrant it: the one-clock + every-cycle-bus-access collapse (a fractional
  master clock with a φ1/φ2 split). **The one release expected to break byte-identity /
  save-state compatibility.** Do NOT conflate it with "the master clock already exists (the
  Phase-0 scheduler)" — the RustyNES versioning trap.

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
