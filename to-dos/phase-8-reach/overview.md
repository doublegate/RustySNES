# Phase 8 — Reach (additive, off-by-default)

## Goal

Add the reach features — rollback netplay, RetroAchievements, TAS movies + a piano-roll editor,
Lua scripting, and a shader / filter ecosystem — each **behind a default-off feature flag** and
each **proven byte-identical with the feature off**, so the shipped / native / `no_std` / wasm
builds stay byte-identical and the accuracy number is unaffected. This is the v1.0.0-completing
phase.

## Exit criteria

- [ ] Rollback netplay (frontend-orchestrated against the deterministic core, native + browser).
- [ ] RetroAchievements (opt-in, native FFI, the `RustySNES/<ver> rcheevos/<ver>` User-Agent
      pattern).
- [ ] TAS movie record / play + a piano-roll editor; deterministic replay.
- [ ] Lua scripting / TAS API.
- [ ] A composable shader / filter ecosystem.
- [ ] Every feature off by default; with all off, builds are byte-identical (a CI gate proves
      it).
- [ ] All sprints complete; v1.0.0 cut prerequisites met.

## Scope

In-scope:

- The five reach feature families, each default-off and byte-identical-when-off.
- The v1.0.0 cut: README / CHANGELOG / docs / STATUS in sync; release matrix + Pages green.

Out-of-scope:

- The fractional-timebase refactor (`docs/adr/0002`) — strictly beyond v1.0.

## Sprints

- [Sprint 1 — Netplay + RetroAchievements](sprint-1-netplay-ra.md) — the determinism-dependent
  reach features first.
- Sprint 2 — TAS movies + piano-roll + Lua.
  **Status:** stub — refine when Sprint 1 is ~complete.
- Sprint 3 — Shaders + the v1.0.0 cut.
  **Status:** stub.

## Dependencies

Phase 5 (the frontend + the exercised determinism contract); Phases 1–4 feature-complete; Phase
6 accuracy at target.

## Risks

- **Byte-identity drift** — a default-off feature accidentally changing the shipped path. Detect:
  the byte-identical CI gate. Mitigate: gate every feature, run the gate per feature combo (the
  RustyNES `--all-features` trap — use explicit combos, never `--all-features`).
- **Netplay determinism** — any hidden non-determinism breaks rollback. Mitigate: the
  determinism contract (`docs/adr/0004`) is the precondition.

## Reference docs

- [docs/frontend.md](../../docs/frontend.md) — the determinism boundary the reach features rest
  on.
- [docs/adr/0004](../../docs/adr/0004-determinism-contract.md) — the rollback/replay precondition.
- [docs/STATUS.md](../../docs/STATUS.md) — version policy + the v1.0.0 cut record.
