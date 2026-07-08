# Copilot Cloud Agent Instructions for RustySNES

## What this repository is

RustySNES is a cycle-accurate SNES/SFC emulator in Rust. Accuracy and determinism are hard requirements, not best-effort goals. `docs/STATUS.md` is the authoritative current-state source.

## Read these first (in order)

1. `docs/architecture.md` (global invariants)
2. `docs/STATUS.md` (current subsystem state)
3. `CONTRIBUTING.md` (quality gate and workflow)
4. `docs/<subsystem>.md` for the area you are changing
5. `docs/adr/*.md` when your change touches architecture/policy decisions

## Load-bearing rules (do not violate)

- Master-clock lockstep timing model is fundamental.
- `rustysnes-core::Bus` owns mutable machine state; CPU borrows `&mut Bus`.
- Keep crate dependencies one-directional; chip crates do not depend on each other.
- Determinism is mandatory (seed + ROM + input → bit-identical output).
- Test ROMs are the behavioral spec; if docs disagree with passing ROM behavior, update docs.
- Additive features remain default-off so native/no_std/wasm outputs stay byte-identical in default configurations.

## Workspace map

- Core chips: `crates/rustysnes-{cpu,ppu,apu,cart,core}/`
- Frontend: `crates/rustysnes-frontend/` (binary: `rustysnes`)
- Accuracy harness: `crates/rustysnes-test-harness/`
- Future/additive crates: `crates/rustysnes-{netplay,cheevos,script}/`
- Specs and design docs: `docs/`
- Planning/tickets: `to-dos/`

## Preferred change workflow

1. Find the failing behavior/test first (especially ROM/oracle failures).
2. Make minimal, local changes in the owning crate.
3. Update matching docs in the same PR when behavior/architecture changes.
4. Run the smallest relevant verification early, then full gate before finalizing.

## Validation commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --features test-roms
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features
```

Wasm build:

```bash
cd crates/rustysnes-frontend/web
trunk build --release
```

## Common errors and workarounds

- **Linux frontend build/link errors (missing X11/Wayland/ALSA/udev libs).**  
  Install system deps first, matching `CONTRIBUTING.md`'s canonical list exactly (don't diverge
  from it here): `libxkbcommon-dev libwayland-dev libxkbcommon-x11-dev libasound2-dev libudev-dev`.

- **`cargo clippy --all-features` fails due to feature incompatibility.**  
  Do **not** use `--all-features`; use `cargo clippy --workspace --all-targets -- -D warnings` (and explicit feature jobs when needed).

- **`trunk build` / Pages wasm build fails from `wasm-bindgen` mismatch.**  
  Keep `crates/rustysnes-frontend/web/Trunk.toml` `[tools].wasm_bindgen` equal to the `wasm-bindgen` version resolved in `Cargo.lock`.

- **`cargo test --workspace --features test-roms` appears to skip parts of the suite.**  
  This is expected when gitignored external ROM corpora are absent; stage external ROM assets locally when those tests are required.

## Repo-specific safety and policy notes

- Never commit commercial ROMs or copyleft/unlicensed ROM corpora into this dual-licensed MIT/Apache-2.0 repository.
- Keep `unsafe` confined to existing allowed areas (frontend/FFI) and document each `unsafe` block with `// SAFETY:`.
- For chip behavior changes, update both code and the corresponding `docs/<chip>.md`.
