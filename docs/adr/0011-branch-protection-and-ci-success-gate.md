# ADR 0011 — Branch protection on a single `ci-success` required check

## Status

Accepted (`v1.5.0 "Bedrock"`).

## Context

`ci.yml`'s `lint` job ran on every PR/push to `main` (fmt + clippy + doc build), but
`full-test`/`no_std`/`bench` — the jobs that actually run `cargo test` — were gated
`if: startsWith(github.ref, 'refs/tags/v')`, i.e. tag pushes only. `main` itself had no branch
protection (`repos/DoubleGate/RustySNES/branches/main/protection` returns 404): nothing
programmatically prevented a PR whose tests fail from squash-merging. The compensating control was
entirely manual — `CONTRIBUTING.md`'s "Quality gate" checklist, including `cargo test --workspace`,
run by hand before opening a PR. This was a deliberate, documented cost tradeoff (a release-mode
3-OS matrix on every PR push is expensive, and the frame-time bench gate is a ceiling that only
needs to hold at release), not an oversight — but it means the only thing standing between a
broken `main` and a real correctness regression was trust that the checklist was actually run
every time, by every contributor (human or agent), which is exactly the kind of gap this project's
own honesty-gate discipline (`docs/adr/0003`) exists to close in the emulation-accuracy domain and
had not yet been applied to its own CI.

RustyNES solved the same problem with a `changes`/`setup` job pair that computes a light-vs-full
mode per run, plus every job downstream keyed off one required check name rather than the raw job
list (raw job lists are fragile to require directly once any of them are conditionally skipped —
GitHub treats a required-but-skipped check as permanently blocking).

## Decision

- Add a `changes` job (`dorny/paths-filter`) and a `setup` job that computes one `mode` output —
  `full` on a release-tag push, a push to `main`, the weekly cron, or a manual dispatch; `light` on
  a PR/feature-branch push that touched code; `skip` on a doc-only diff.
- Add a new `test-light` job: `cargo test --workspace`, Linux-only, debug-mode, cached — the
  actual gap this ADR closes. Runs whenever `mode != skip`, independent of `full`/`light`, so it
  fires on every PR/push with code changes, not just tagged releases.
- Widen `full-test`/`no_std`/`bench`'s trigger from tag-only to `mode == full` (tag push, push to
  `main`, weekly cron, manual dispatch) — `main` itself now gets the complete battery on every
  push, not only at the next tag.
- Add a `ci-success` job (`if: always()`, depends on every job above) that fails iff any dependency
  resolved to anything other than `success`/`skipped`. This is the one check name branch protection
  requires — individual jobs stay conditionally skippable without breaking the required-check
  contract.
- Repo-settings action (not a code change — GitHub branch-protection rules aren't tracked in git):
  the maintainer enables branch protection on `main` requiring `ci-success`, once this PR merges
  and the check has run at least once so GitHub can see it exists.
- Extract a `.github/actions/rust-setup` composite action (toolchain, cache, optional Linux
  frontend deps as inputs) and migrate `ci.yml`/`pages.yml` onto it, so the pinned `1.96` toolchain
  version and cache-key convention live in one place instead of being repeated across every job.
  `security.yml` is not migrated — it never installs a Rust toolchain at all (its two jobs only run
  prebuilt `cargo-audit`/`cargo-deny` binaries against `Cargo.lock`), so there's nothing for the
  composite action to factor out there.

## Consequences

- (+) A `cargo test` failure can no longer merge to `main` silently — closes the real gap between
  "the checklist says to run this" and "CI actually enforces it," matching RustyNES's own posture.
- (+) `main` gets the full 3-OS/`no_std`/bench battery on every push, not only retroactively at the
  next tag — a regression is caught within one push instead of accumulating until release day.
- (+) One required-check name (`ci-success`) survives future job additions/removals/reordering
  without a branch-protection-rule edit each time.
- (−) PR/push CI now runs one more job (`test-light`) than before; kept cheap deliberately (Linux,
  debug mode, cached) to preserve the original fast-iteration intent `ci.yml`'s own prior comments
  argued for.
- (−) `main`-push CI now costs the same as a tag push (the full 3-OS matrix), a real increase in
  minutes spent per merge — accepted because it converts "a regression sits on `main` until the
  next tag" into "a regression is visible within one CI run of landing," which is the actual goal.
- (−) Branch protection is a manual, one-time repo setting outside version control; if the
  `ci-success` job is ever renamed, the branch-protection rule must be updated to match by hand.
