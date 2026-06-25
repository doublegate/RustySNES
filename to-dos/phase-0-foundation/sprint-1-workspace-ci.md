# Sprint 1 — Workspace, CI, lints, test-harness skeleton

**Phase:** Phase 0 — Foundation
**Sprint goal:** the scaffold compiles green across the CI matrix, the lints + `no_std` gate
pass, the permissive test ROMs are seeded, and the test-harness oracle scaffolding is ready.
**Estimated duration:** 1–2 weeks

## Tickets

### T-01-001 — Workspace compiles + CI matrix green

**Description:** ensure the ten-crate workspace builds and the CI runs `check` / `build` /
`test` on stable across Linux/macOS/Windows plus the 1.96 MSRV pin on Linux.

**Acceptance criteria:**

- [ ] `cargo check --workspace` and `cargo build --workspace` green locally.
- [ ] CI matrix (Linux/macOS/Windows + MSRV 1.96) green on the stubs.
- [ ] Linux system-dep install step documented for the frontend (libxkbcommon / wayland /
      alsa-lib / systemd-libs).

**Dependencies:** none
**Reference:** `docs/architecture.md` §crate-inventory; `Cargo.toml`
**Estimated complexity:** S

---

### T-01-002 — Lint + doc gates

**Description:** wire `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` into CI and the
pre-commit hook (markdownlint pinned v0.39.0 already present).

**Acceptance criteria:**

- [ ] All three gates green in CI.
- [ ] `pre-commit run --all-files` green (markdownlint + fmt).
- [ ] No `--all-features` in any gate (mutually-exclusive script backends — the RustyNES trap).

**Dependencies:** T-01-001
**Reference:** `docs/testing-strategy.md`; `.pre-commit-config.yaml`
**Estimated complexity:** S

---

### T-01-003 — `no_std` cross-compile gate for the chip crates

**Description:** confirm `rustysnes-{cpu,ppu,apu,cart,core}` build `--no-default-features`
against `core` + `alloc` only (a bare-metal target), and add a CI job for it.

**Acceptance criteria:**

- [ ] `cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features` green.
- [ ] The chip crates carry `#![no_std]` + `extern crate alloc;`.
- [ ] CI runs the `no_std` job.

**Dependencies:** T-01-001
**Reference:** `docs/architecture.md` §crate-inventory
**Estimated complexity:** M

---

### T-01-004 — Seed `tests/roms/` with the permissive corpora + the external tier

**Description:** commit gilyon/snes-tests (MIT) and undisbeliever/snes-test-roms (Zlib) under
`tests/roms/`; create the gitignored `tests/roms/external/` tier for the unlicensed/copyleft
corpora; record each corpus's license in `tests/roms/README.md`.

**Acceptance criteria:**

- [ ] gilyon + undisbeliever ROMs committed with their LICENSE files.
- [ ] `tests/roms/external/` gitignored; `.gitignore` updated.
- [ ] `tests/roms/README.md` lists every corpus + license + commit/external posture.
- [ ] No unlicensed/copyleft/Nintendo ROMs committed.

**Dependencies:** T-01-001
**Reference:** `docs/testing-strategy.md` §licensing
**Estimated complexity:** S

---

### T-01-005 — Decide + document the 65816 JSON oracle license policy

**Description:** the SingleStepTests/65816 set ships no LICENSE. Decide between (a) gitignored
external tier, (b) securing permission, or (c) self-generating equivalent JSON from a validated
core. Record the decision so Phase 1 can gate its primary oracle.

**Acceptance criteria:**

- [ ] The decision is recorded in `docs/testing-strategy.md` §licensing "Open questions".
- [ ] If (a): the 65816 set lands only in `tests/roms/external/` (gitignored).
- [ ] `docs/STATUS.md` reflects the chosen posture in the suite table.

**Dependencies:** T-01-004
**Reference:** `docs/testing-strategy.md`; `ref-docs/research-report.md` "Open questions" #1
**Estimated complexity:** S

---

### T-01-006 — Test-harness skeleton (golden differ + JSON-oracle runner + `run_until_complete`)

**Description:** stand up `rustysnes-test-harness` with a no-op `run_until_complete(rom,
max_frames)`, a JSON per-opcode oracle runner stub (parses the SingleStepTests format), and a
golden-state differ that reports the first mismatch.

**Acceptance criteria:**

- [ ] The harness compiles and its stub tests pass.
- [ ] The JSON-oracle runner parses one sample opcode file (gated behind the external tier).
- [ ] `run_until_complete` + the differ have signatures Phase 1 can fill in.

**Dependencies:** T-01-005
**Reference:** `docs/testing-strategy.md` Layers 2–3
**Estimated complexity:** M

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] `docs/STATUS.md` still shows all-zero counts but a wired matrix.
- [ ] CHANGELOG.md updated for the scaffold milestone.
