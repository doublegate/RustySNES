# Contributing

Thanks for your interest in contributing.

## Development setup

RustySNES is a pure-Rust workspace.

- Install [rustup](https://rustup.rs).
- The toolchain is pinned in `rust-toolchain.toml` (Rust 1.96, edition 2024);
  `rustup` auto-installs it, including the `wasm32-unknown-unknown` and
  `thumbv7em-none-eabihf` targets the gates exercise.
- `cargo check --workspace` to verify the workspace compiles.
- `cargo test --workspace` to run the unit and integration tests.
- `cargo test --workspace --features test-roms` to add the test-ROM oracle.

On Linux, the frontend crate pulls in the wgpu / winit / cpal system deps:

```bash
# Debian / Ubuntu
sudo apt-get install -y libxkbcommon-dev libwayland-dev libxkbcommon-x11-dev libasound2-dev libudev-dev
# Arch / CachyOS
sudo pacman -S --needed libxkbcommon wayland alsa-lib systemd-libs
```

## Workflow

1. Pick a ticket from `to-dos/` (or open an issue first if your work
   isn't already represented there).
2. Create a branch: `<type>/<short-description>` (e.g.,
   `feat/cpu-immediate-addressing`, `fix/ppu-scroll-wrap`).
3. Make changes. Keep commits focused.
4. Run the local quality gate before pushing.
5. Open a PR. Reference the ticket(s) and any relevant `docs/` files.

## Quality gate

Before opening a PR, ensure every gate below is green:

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features`
      passes (the chip stack stays `no_std`)
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passes
- [ ] New public items have rustdoc (`missing_docs` is a workspace lint)
- [ ] User-visible changes are noted in `CHANGELOG.md` under `[Unreleased]`

Never run `cargo clippy --all-features`: the `scripting` (native mlua) and
`script-wasm` (wasm piccolo) backends are mutually exclusive, so the feature
set cannot resolve. Use the explicit per-feature jobs instead.

## Documentation expectations

- New subsystems get a doc in `docs/`.
- Architecture-affecting changes update `docs/architecture.md`.
- A chip-behavior change touches both the chip code and the chip's
  `docs/<subsystem>.md` — they drift apart easily; don't let them.
- User-visible changes are noted in `CHANGELOG.md` under `[Unreleased]`.
- Ticket completion is reflected in the relevant `to-dos/` sprint file.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org):
`<type>(<scope>): <subject>`.

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `build`,
`ci`. Keep the imperative subject at or under 72 characters; an optional body
explains the why (not the what — the diff shows the what). No emojis in code,
comments, or commits (project policy).

## Code review

- One reviewer minimum; two for changes to `docs/architecture.md` or
  cross-subsystem refactors.
- Reviewers focus on correctness, design, and adherence to the relevant
  `docs/` specification.
- Discussion is preferred over deferral; if a comment can't be resolved
  in review, file a follow-up ticket explicitly.
