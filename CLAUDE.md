# CLAUDE.md

Guidance for Claude Code working in RustySNES.

## What this is
RustySNES is a cycle-accurate Super Nintendo Entertainment System / Super Famicom emulator in Rust at the Mesen2 / ares / higan bar.
Architecture (the load-bearing facts — read `docs/architecture.md`):

- **The timing master is master clock** @ 21477270 Hz; a
  lockstep scheduler advances it one unit/tick and every other chip on its divisor.
- **The Bus owns everything mutable** (`rustysnes-core::Bus`); the CPU borrows `&mut Bus`.
- **The crate graph is one-directional**; no chip crate depends on another; `rustysnes-core` ties them.
- **Board logic lives in the cart crate** (default-no-op trait hooks).
- **Determinism is a hard contract** (seed+ROM+input ⇒ bit-identical AV; frontend owns rate control).
- **Test ROMs are the spec**; pin the failing ROM first, then implement.
- **Additive features are default-off** so shipped/native/no_std/wasm stay byte-identical.

## Where things live
- `crates/rustysnes-cpu/` — WDC 65C816 (cpu)
- `crates/rustysnes-ppu/` — PPU1 (5C77) + PPU2 (5C78) (video)
- `crates/rustysnes-apu/` — SPC700 + S-DSP (audio)
- `crates/rustysnes-cart/` — LoROM/HiROM/ExHiROM + coprocessors (cart)
- `crates/rustysnes-core/` — Bus + scheduler · `crates/rustysnes-frontend/` — egui shell (binary `rustysnes`)
- `crates/rustysnes-test-harness/` — the accuracy oracle
- `docs/` — the spec (update in the same PR as code); `docs/STATUS.md` = single source of truth;
  `docs/adr/` — ADRs. `ref-docs/` — immutable research. `ref-proj/` — study clones (gitignored).
- `to-dos/ROADMAP.md` — planning entry point; tickets `T-PS-NNN`.

## Build / test / lint
```bash
cargo check --workspace && cargo test --workspace
cargo test --workspace --features test-roms
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings   # + per-feature jobs; NEVER --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features   # no_std gate
```

## Conventions
Conventional Commits; chip change touches the chip code AND its `docs/<chip>.md`; user-visible
changes go in `CHANGELOG [Unreleased]`; hot paths allocation-free; `unsafe` only in frontend +
FFI with `// SAFETY:`; never commit commercial ROMs; never `--all-features`. Start clean at
v0.1.0 — RustyNES "v2.0 / engine-lineage" anchors are NOT this project's releases.
