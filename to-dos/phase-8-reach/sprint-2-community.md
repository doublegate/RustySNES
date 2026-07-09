# Sprint 2 — Community

**Phase:** Phase 8 — Instrumentation + Community
**Sprint goal:** rollback netplay and RetroAchievements ship behind default-off feature flags,
each byte-identical with the feature off, resting on the exercised determinism contract.
**Estimated duration:** 2 weeks
**Release mapping:** `v0.9.0 "Community"` (`to-dos/VERSION-PLAN.md`)

## Tickets

### T-82-001 — Netplay save-state cost benchmark (pre-work)

**Description:** before committing to the existing full-snapshot save-state design for rollback
netplay, benchmark `System::save_state()`/`load_state()` cost. `docs/benchmarks.md` currently has
only one number (steady-state frame time); `RewindBuffer` was designed for ~10 Hz capture, and
rollback netplay calls save/restore far more often. If the cost is too high for a real rollback
window, delta/incremental snapshots become necessary — a real design change beyond
`docs/adr/0006`'s "future memory optimization, not correctness requirement" framing.

**Acceptance criteria:**

- [ ] A new Criterion benchmark measures `save_state()`/`load_state()` cost across a
      no-coprocessor, a Curated (Super FX/SA-1), and a BestEffort sample.
- [ ] The result is recorded in `docs/benchmarks.md`.
- [ ] A go/no-go call on the full-snapshot design is made explicitly: either it's fast enough
      for the target rollback window, or a new ADR is opened describing the delta/incremental
      redesign needed before T-82-002 proceeds.

**Dependencies:** `v0.2.0`'s save-state envelope; `docs/adr/0006`
**Reference:** `docs/benchmarks.md`; `docs/adr/0006-save-state-format.md`
**Estimated complexity:** S

---

### T-82-002 — Rollback netplay (frontend-orchestrated)

**Description:** implement GGPO-style rollback netplay in `rustysnes-netplay` (UDP native +
WebRTC browser, 2+ players), orchestrated by the frontend against the deterministic core
(snapshot / restore / re-simulate). Behind a new `netplay` feature flag — unlike
`retroachievements`/`scripting`, no existing scaffold for this flag exists in
`crates/rustysnes-frontend/Cargo.toml` yet, so this ticket adds it; flag any obsolete/unused
netplay code skeletons found elsewhere in the codebase for removal rather than leaving them to
silently contradict this. Keep the netplay drive loop independent of `emu_thread.rs`'s
single-player pacer — a netplay session uses its own rollback-aware loop, never the generic
`emu-thread` path, avoiding a control-model conflict with `v1.0.0`'s dedicated-thread work.

**Acceptance criteria:**

- [ ] Rollback re-simulation is bit-identical (relies on `docs/adr/0004`).
- [ ] Native (UDP) + browser (WebRTC) transports work.
- [ ] Netplay sessions use their own drive loop, verified independent of `emu-thread`.
- [ ] With `netplay` off, the build is byte-identical (CI gate).

**Dependencies:** T-82-001 (go/no-go on the save-state design); T-51-003; T-31-004 (determinism
exercised)
**Reference:** `docs/frontend.md` §determinism-boundary; `docs/adr/0004`
**Estimated complexity:** L

---

### T-82-003 — RetroAchievements (opt-in, native FFI)

**Description:** implement opt-in RetroAchievements in `rustysnes-cheevos` (native FFI), with the
`RustySNES/<crate ver> rcheevos/<rcheevos ver>` HTTP User-Agent pattern. Default-off feature.

**Acceptance criteria:**

- [ ] RA auth + achievement processing work native (opt-in).
- [ ] The User-Agent leads with `RustySNES/` (a regression test guards it).
- [ ] With `retroachievements` off, the build is byte-identical; clippy runs the RA feature combo.

**Dependencies:** T-51-001
**Reference:** `docs/frontend.md`; the RustyNES RA User-Agent convention
**Estimated complexity:** M

---

### T-82-004 — The byte-identical CI gate (feature-off), extended again

**Description:** extend the byte-identical-with-flags-off CI gate (last extended in Sprint 1,
T-81-004) to cover netplay and RetroAchievements; run clippy per explicit feature combo (never
`--all-features`).

**Acceptance criteria:**

- [ ] The byte-identical gate passes with all Phase 8 features off (Sprint 1 + Sprint 2 combined).
- [ ] clippy runs each feature combo explicitly (the mutually-exclusive-backend trap avoided).
- [ ] The gate is wired into the standard CI run, ready for `v1.0.0`'s final re-verification.

**Dependencies:** T-82-002; T-82-003
**Reference:** `docs/testing-strategy.md`; `docs/STATUS.md` §version-policy
**Estimated complexity:** S

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] Every Phase 8 feature is off by default + byte-identical when off.
- [ ] CHANGELOG.md updated.
