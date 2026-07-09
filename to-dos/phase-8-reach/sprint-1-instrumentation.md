# Sprint 1 — Instrumentation

**Phase:** Phase 8 — Instrumentation + Community
**Sprint goal:** the debugger overlay, Lua scripting + TAS movie API, and cheat-code support
ship behind default-off feature flags, each byte-identical with the feature off.
**Estimated duration:** 3 weeks
**Release mapping:** `v0.8.0 "Instrumentation"` (`to-dos/VERSION-PLAN.md`)

## Tickets

### T-81-001 — Debugger overlay (65C816/PPU/APU/Cart panels)

**Description:** fill in `ui_shell.rs`'s already-wired debugger window (currently
`"TODO(impl-phase)"` placeholders in each of its 4 panel selectors) with real breakpoint and
memory-viewer functionality, behind the existing `debug-hooks` flag. Include SA-1/Super FX
coprocessor state in the Cart panel from day one — resolving `docs/frontend.md`'s open question
in the breadth-inclusive direction this whole ladder takes, not deferring it further.

**Landed in two PRs, not one** (scoping found during implementation, not before): PR A ships
the live state viewers for all 4 panels (pure read-only plumbing, no core changes beyond small
new accessors). PR B adds a minimal 65C816 disassembler + PC breakpoints + step/step-over/
step-into (frontend-only, using the existing `System::step_instruction()`). Read/write
watchpoints need a new `debug-hooks` feature on `rustysnes-core` itself + a `Bus`-level hook —
deferred to a separate follow-up ticket, T-81-001b, since it touches the hottest path in the
engine and deserves its own focused review.

**Acceptance criteria:**

- [x] 65C816 panel: register/flag view (PR A). Breakpoints (PC), step/step-over/step-into: PR B.
      Read/write watchpoints: T-81-001b (not this ticket).
- [x] PPU panel: VRAM (scrollable window) / CGRAM viewer, current scanline/dot, register state
      (PR A). OAM viewer not yet landed — small follow-up, same shape as the VRAM/CGRAM viewers.
- [x] APU panel: SPC700 PC/halt state, DSP voice state (PR A).
- [x] Cart panel: active board type + coprocessor register state (SA-1 second-CPU state via
      `System::sa1_regs`, Super FX/GSU state via `Board::debug_gsu_state`) (PR A).
- [x] With `debug-hooks` off, the build is byte-identical — the Debug menu entry itself is
      feature-gated, so `debugger_open` can never become `true` and the app never builds a
      snapshot (PR A; verified `cargo check`/`clippy`/`fmt` clean in both configs, full
      `--features test-roms` suite passes unchanged).

**Dependencies:** T-51-001 (the shell itself, already landed)
**Reference:** `docs/frontend.md` §Debugger overlay; `crates/rustysnes-frontend/src/ui_shell.rs`,
`debug_snapshot.rs`
**Estimated complexity:** L

---

### T-81-002 — Lua scripting + TAS movie API

**Description:** implement `rustysnes-script`'s full stated scope in one pass — Lua scripting
(a memory-read/write + frame-callback API) and TAS movie record/playback (a deterministic input
log format + save-state-at-frame-0 seeding, replay-verified bit-identical) — behind the existing
`scripting` flag. Both build on the existing `Bus::set_joypad`/save-state envelope; no new
architectural work needed.

**Acceptance criteria:**

- [ ] Lua scripts can read/write emulated memory and hook a per-frame callback.
- [ ] TAS movies record a deterministic input log; replaying a recorded movie against the same
      ROM + save-state-at-frame-0 produces a bit-identical framebuffer/audio trace.
- [ ] With `scripting` off, the build is byte-identical (CI gate).

**Dependencies:** T-31-004 (determinism exercised), `v0.2.0`'s save-state envelope
**Reference:** `docs/architecture.md` (determinism-contract fact citing "TAS replay" as a
designed-for use case); `docs/STATUS.md`'s `rustysnes-script` subsystem entry
**Estimated complexity:** L

---

### T-81-003 — Cheat-code support (Game Genie / Pro Action Replay SNES format)

**Description:** implement SNES Game Genie and Pro Action Replay cheat-code parsing + a
per-frame memory-patch application, behind a new `cheats` flag (no existing scaffold — the first
new flag added on this ladder, matching the existing naming convention). Grouped in this sprint
rather than with netplay/RetroAchievements: cheats are memory-watch/poke tooling, the same
substrate as the debugger's memory panel.

**Acceptance criteria:**

- [ ] Game Genie code parsing + decode to a RAM-address/value patch.
- [ ] Pro Action Replay code parsing + decode.
- [ ] Patches apply every frame without breaking the determinism contract when the feature is
      off (a cheat is host-applied external input, not a hardware behavior — model it that way).
- [ ] With `cheats` off, the build is byte-identical (CI gate).

**Dependencies:** none beyond the base memory-access surface
**Reference:** RustyNES's cheat-code feature (parity target); `docs/adr/0004`
**Estimated complexity:** M

---

### T-81-004 — The byte-identical CI gate (feature-off), extended

**Description:** extend the existing byte-identical-with-flags-off CI gate to cover the three
new flags this sprint adds (`debug-hooks`, `scripting`, `cheats`); run clippy per explicit
feature combo (never `--all-features`).

**Acceptance criteria:**

- [ ] The byte-identical gate passes with all three flags off.
- [ ] clippy runs each new feature combo explicitly.
- [ ] The gate is wired into the standard CI run, ready to extend again in Sprint 2.

**Dependencies:** T-81-001; T-81-002; T-81-003
**Reference:** `docs/testing-strategy.md`; `docs/STATUS.md` §version-policy
**Estimated complexity:** S

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] Every instrumentation feature is off by default + byte-identical when off.
- [ ] CHANGELOG.md updated.
