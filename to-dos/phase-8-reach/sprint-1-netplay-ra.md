# Sprint 1 — Netplay + RetroAchievements

**Phase:** Phase 8 — Reach
**Sprint goal:** rollback netplay and RetroAchievements ship behind default-off feature flags,
each byte-identical with the feature off, resting on the exercised determinism contract.
**Estimated duration:** 2 weeks

## Tickets

### T-81-001 — Rollback netplay (frontend-orchestrated)

**Description:** implement GGPO-style rollback netplay in `rustysnes-netplay` (UDP native +
WebRTC browser, 2+ players), orchestrated by the frontend against the deterministic core
(snapshot / restore / re-simulate). Behind a default-off feature.

**Acceptance criteria:**

- [ ] Rollback re-simulation is bit-identical (relies on `docs/adr/0004`).
- [ ] Native (UDP) + browser (WebRTC) transports work.
- [ ] With the feature off, the build is byte-identical (CI gate).

**Dependencies:** T-51-003; T-31-004 (determinism exercised)
**Reference:** `docs/frontend.md` §determinism-boundary; `docs/adr/0004`
**Estimated complexity:** L

---

### T-81-002 — RetroAchievements (opt-in, native FFI)

**Description:** implement opt-in RetroAchievements in `rustysnes-cheevos` (native FFI), with the
`RustySNES/<crate ver> rcheevos/<rcheevos ver>` HTTP User-Agent pattern. Default-off feature.

**Acceptance criteria:**

- [ ] RA auth + achievement processing work native (opt-in).
- [ ] The User-Agent leads with `RustySNES/` (a regression test guards it).
- [ ] With the feature off, the build is byte-identical; clippy runs the RA feature combo.

**Dependencies:** T-51-001
**Reference:** `docs/frontend.md`; the RustyNES RA User-Agent convention
**Estimated complexity:** M

---

### T-81-003 — The byte-identical CI gate (feature-off)

**Description:** add a CI gate asserting that with all reach features off, the shipped / native /
no_std / wasm builds are byte-identical to the pre-reach baseline; run clippy per explicit
feature combo (never `--all-features`).

**Acceptance criteria:**

- [ ] The byte-identical gate passes with features off.
- [ ] clippy runs each feature combo explicitly (the mutually-exclusive-backend trap avoided).
- [ ] The gate is wired into the standard CI run.

**Dependencies:** T-81-001; T-81-002
**Reference:** `docs/testing-strategy.md`; `docs/STATUS.md` §version-policy
**Estimated complexity:** S

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] Every reach feature is off by default + byte-identical when off.
- [ ] CHANGELOG.md updated.
