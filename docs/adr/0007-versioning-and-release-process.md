# ADR 0007 — The `v0.x.0` → `v1.0.0` versioning ladder and release process

## Status

Accepted (retroactively — `v0.1.0` through `v0.4.0` already shipped under this policy before it
was written down as its own ADR; `to-dos/VERSION-PLAN.md` is where it lives day-to-day, this ADR
records why the policy exists and won't change without a new one).

## Context

RustySNES had built most of an accurate SNES emulator (CPU/PPU/APU at oracle parity, eight
validated coprocessors, a playable frontend) before it had ever cut a GitHub release — every
milestone sat inside one perpetual `CHANGELOG.md` `[Unreleased]` section, and
`to-dos/VERSION-PLAN.md`'s own skeleton (an earlier draft) described work that was already
shipped. This is the same process gap RustyNES had already solved for the NES side of this
workspace: a real,
named, git-tagged release ladder with dense, technically detailed tag annotations, cut on a
regular cadence rather than left to accumulate indefinitely.

## Decision

- **Named `v0.x.0` minors, each one coherent theme** (`v0.1.0 "Foundation"`, `v0.2.0
  "Persistence"`, `v0.3.0 "Continuum"`, `v0.4.0 "Completion"`, ...), sequenced toward `v1.0.0` per
  `to-dos/VERSION-PLAN.md`'s ladder. `v0.x.y` patches are reserved for fixes discovered after an
  `x` release ships, never new scope.
- **The annotated git tag body IS the release note** — dense technical prose grouped by area,
  always closing with an explicit accuracy-regression statement ("oracle/golden suites: all held,
  no regressions"), matching RustyNES's own practice and this project's existing `CHANGELOG.md`
  entry voice. `gh release create` uses the same text as the tag body, not a separate summary.
- **`CHANGELOG.md`'s `[Unreleased]` section gets restructured into a dated `## [x.y.z] "Name" -
  date` section at cut time** — every entry that landed since the last release, grouped under
  `### Added`/`### Fixed`/etc., matching Keep a Changelog. A fresh empty `[Unreleased]` heading is
  left in place immediately afterward so the next cycle's entries have somewhere to land.
- **Every release goes through the same PR-based increment workflow** as regular feature work
  (branch → implement → verify → PR → Gemini + Copilot review → adjudicate findings against
  primary sources, not blind compliance → merge), including the release-notes-closeout commit
  itself and the version-ladder-refresh commit — release-process changes are not exempted from
  review just because they're "just docs."
- **`v1.0.0`'s gate is accuracy + a stable save-state/core API + a shippable app — deliberately
  NOT feature-count completeness** (matching RustyNES's actual v1.0 gate, not an earlier internal
  draft that conflated breadth with 1.0). Breadth items (scripting, netplay, RetroAchievements,
  TAS, shaders) are explicitly deferred to named post-1.0 minors, each additive/default-off,
  each reaffirming the accuracy gate never regressed — RustyNES's own post-1.0 pattern.
- **`to-dos/ROADMAP.md` and `to-dos/VERSION-PLAN.md` are updated at every release**, not left
  stale — the exact failure mode this ADR's own context section describes.

## Consequences

- (+) A real, auditable release history exists going forward, with install-ready binaries
  attached to every tag (see the `release.yml` artifact-attachment fix landed alongside `v0.4.0`).
- (+) The tag-body-as-release-note convention means release notes are never a separate,
  lower-effort afterthought — they're written at the same technical depth as the CHANGELOG
  entries that feed them, because they're the same prose.
- (+) Deferring breadth out of the `v1.0.0` gate keeps the 1.0 cut achievable on a realistic
  timeline instead of chasing an ever-growing feature list.
- (−) Every release now carries real ceremony (CHANGELOG restructure, docs sync, a dedicated PR,
  a tag, a GitHub release) — a standing cost accepted because the alternative (the pre-`v0.1.0`
  state) was worse: real, shipped work invisible behind an unreleased tag indefinitely.
- (−) `v0.x.y` patch releases need their own discipline (this ADR reserves them for fixes only)
  to avoid the ladder drifting back into "everything piles into Unreleased."
