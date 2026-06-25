# Phase 0 — Foundation

## Goal

The Cargo workspace and the one-directional crate skeletons compile; CI is green on the stubs;
`tests/roms/` is seeded with the permissively-licensed test corpora; the test-harness skeleton
(golden-log differ + JSON-oracle runner + `run_until_complete`) stands up. No real emulation
yet — but everything builds and the test scaffolding is ready to receive the CPU work.

## Exit criteria

- [ ] `cargo check --workspace` and `cargo build --workspace` green.
- [ ] `cargo test --workspace` (stub tests) green in CI on Linux/macOS/Windows + the 1.96 MSRV pin.
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
      `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` green.
- [ ] `rustysnes-core` re-exports the (stub) chip types; the crate graph is one-directional
      (`docs/architecture.md` §3).
- [ ] The chip crates build `--no-default-features` for a `no_std` target.
- [ ] gilyon (MIT) + undisbeliever (Zlib) ROMs committed under `tests/roms/`; the gitignored
      `tests/roms/external/` tier exists for the unlicensed/copyleft corpora.
- [ ] The test-harness skeleton compiles with a no-op `run_until_complete` + JSON-oracle stub.
- [ ] All sprints in this phase complete.

## Scope

In-scope:

- Workspace + the ten crate skeletons (already scaffolded), CI, lints, `no_std` gate.
- Test-ROM seeding (permissive only) + the `tests/roms/external/` gitignore tier.
- The test-harness skeleton.

Out-of-scope (later phases):

- Any chip behavior (Phases 1–3), the cart memory model (Phase 4), the frontend (Phase 5).

## Sprints

- [Sprint 1 — Workspace, CI, lints, test-harness skeleton](sprint-1-workspace-ci.md) — make the
  scaffold green and the oracle scaffolding ready.

## Dependencies

None — this is the first phase. The crate skeletons are scaffolded by the crate-skeletons agent.

## Risks

- **The 65816 JSON oracle license** (no LICENSE) blocks committing it — resolve the policy now
  (gitignore vs self-generate) so Phase 1 isn't blocked. Detect: CI can't find the oracle.
  Mitigate: decide in T-01-005.
- **`no_std` drift** — a chip crate accidentally pulling `std`. Detect: the `no_std` CI job.
  Mitigate: gate it from day one.

## Reference docs

- [docs/architecture.md](../../docs/architecture.md) — the crate graph + invariants.
- [docs/testing-strategy.md](../../docs/testing-strategy.md) — the oracle + licensing tiers.
- [docs/STATUS.md](../../docs/STATUS.md) — the matrix this phase leaves all-zero but wired.
