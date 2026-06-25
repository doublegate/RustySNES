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

- [x] `cargo check --workspace` and `cargo build --workspace` green locally.
- [x] CI matrix (Linux/macOS/Windows + MSRV 1.96) green on the stubs. *(toolchain pinned `@1.96`
      = the MSRV; the three-OS matrix is in `.github/workflows/ci.yml`.)*
- [x] Linux system-dep install step documented for the frontend (libxkbcommon / wayland /
      alsa-lib / systemd-libs). *(documented in `CONTRIBUTING.md`; CI now installs them in an
      `apt-get` step before building.)*

**Dependencies:** none
**Reference:** `docs/architecture.md` §crate-inventory; `Cargo.toml`
**Estimated complexity:** S

---

### T-01-002 — Lint + doc gates

**Description:** wire `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` into CI and the
pre-commit hook (markdownlint pinned v0.39.0 already present).

**Acceptance criteria:**

- [x] All three gates green in CI. *(fmt, clippy `-D warnings`, rustdoc `-D warnings` all pass
      locally across the workspace and per-feature `test-roms` / `commercial-roms`.)*
- [x] `pre-commit run --all-files` green (markdownlint + fmt). *(`.pre-commit-config.yaml` runs
      markdownlint + `cargo fmt --all --check`; fmt verified clean.)*
- [x] No `--all-features` in any gate (mutually-exclusive script backends — the RustyNES trap).

**Dependencies:** T-01-001
**Reference:** `docs/testing-strategy.md`; `.pre-commit-config.yaml`
**Estimated complexity:** S

---

### T-01-003 — `no_std` cross-compile gate for the chip crates

**Description:** confirm `rustysnes-{cpu,ppu,apu,cart,core}` build `--no-default-features`
against `core` + `alloc` only (a bare-metal target), and add a CI job for it.

**Acceptance criteria:**

- [x] `cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features` green.
- [x] The chip crates carry `#![no_std]` + `extern crate alloc;`.
- [x] CI runs the `no_std` job. *(`no_std` job present in `ci.yml`.)*

**Dependencies:** T-01-001
**Reference:** `docs/architecture.md` §crate-inventory
**Estimated complexity:** M

---

### T-01-004 — Seed `tests/roms/` with the permissive corpora + the external tier

**Description:** commit gilyon/snes-tests (MIT) and undisbeliever/snes-test-roms (Zlib) under
`tests/roms/`; create the gitignored `tests/roms/external/` tier for the unlicensed/copyleft
corpora; record each corpus's license in `tests/roms/README.md`.

**Acceptance criteria:**

- [x] gilyon + undisbeliever ROMs committed with their LICENSE files. *(+ a deterministic
      spc700 sample; see `rom-seeding-runbook.md`.)*
- [x] `tests/roms/external/` gitignored; `.gitignore` updated. *(65816 / full-spc700 / 240p /
      Krom / blargg-spc / commercial all under the gitignored tier.)*
- [x] `tests/roms/README.md` lists every corpus + license + commit/external posture.
- [x] No unlicensed/copyleft/Nintendo ROMs committed. *(gate verified: only gilyon/undisbeliever
      `.sfc` are trackable.)*

**Dependencies:** T-01-001
**Reference:** `docs/testing-strategy.md` §licensing; **step-by-step:**
[`rom-seeding-runbook.md`](rom-seeding-runbook.md)
**Estimated complexity:** S

---

### T-01-005 — Decide + document the 65816 JSON oracle license policy

**Description:** the SingleStepTests/65816 set ships no LICENSE. Decide between (a) gitignored
external tier, (b) securing permission, or (c) self-generating equivalent JSON from a validated
core. Record the decision so Phase 1 can gate its primary oracle. **Decided in
[`docs/adr/0005`](../../docs/adr/0005-65816-opcode-oracle-license.md): hybrid (c)+(a).**

**Acceptance criteria:**

- [x] The decision is recorded — `docs/adr/0005` + `docs/testing-strategy.md` §"Open questions".
- [x] If (a): the 65816 set lands only in `tests/roms/external/` (gitignored). *(staged at
      `tests/roms/external/65816-singlestep/`; gate-verified not trackable.)*
- [x] `docs/STATUS.md` reflects the chosen posture in the suite table. *(65816 row now records
      5,119,999/5,120,000 with the external cross-check posture.)*

**Dependencies:** T-01-004
**Reference:** `docs/testing-strategy.md`; `ref-docs/research-report.md` "Open questions" #1
**Estimated complexity:** S

---

### T-01-006 — Test-harness skeleton (golden differ + JSON-oracle runner + `run_until_complete`)

**Description:** stand up `rustysnes-test-harness` with a no-op `run_until_complete(rom,
max_frames)`, a JSON per-opcode oracle runner stub (parses the SingleStepTests format), and a
golden-state differ that reports the first mismatch.

**Acceptance criteria:**

- [x] The harness compiles and its stub tests pass.
- [x] The JSON-oracle runner parses one sample opcode file (gated behind the external tier).
      *(now a full per-opcode oracle: `tests/cpu_oracle.rs` replays all 512 files and diffs
      state + RAM + cycle count.)*
- [x] `run_until_complete` + the differ have signatures Phase 1 can fill in. *(`runner.rs`
      `run_until_complete` signature stands; on-cart WRAM-sentinel decode is TODO(T-04) — needs
      a bootable `System`, Phase 2/4.)*

**Dependencies:** T-01-005
**Reference:** `docs/testing-strategy.md` Layers 2–3
**Estimated complexity:** M

---

## Sprint review checklist

- [x] All tickets checked off or explicitly deferred (with reason). *(T-01-001…006 done; the
      gilyon on-cart decode in T-01-006 is deferred to T-04 — it needs a bootable `System`.)*
- [x] `docs/STATUS.md` updated: the matrix is wired; counts stay 0 except the CPU 65816 oracle,
      which Phase 1 brought to 5,119,999/5,120,000.
- [x] CHANGELOG.md updated for the scaffold + ROM-seeding + Phase-1 CPU milestone.
