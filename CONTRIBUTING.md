# Contributing

Thanks for your interest in contributing.

## Development setup

<FILL — language-appropriate. Examples:

### Rust
- Install [rustup](https://rustup.rs).
- Toolchain is pinned in `rust-toolchain.toml`; `rustup` will auto-install.
- `cargo check` to verify the workspace compiles.
- `cargo test` to run tests.

### Python
- Python 3.11+ recommended.
- `pip install -e ".[dev]"` to install with dev dependencies.
- `pytest` to run tests.

### C / C++
- CMake 3.20+, a C++20 compiler.
- `cmake -B build -DCMAKE_BUILD_TYPE=Debug && cmake --build build`.
- `ctest --test-dir build` to run tests.
>

## Workflow

1. Pick a ticket from `to-dos/` (or open an issue first if your work
   isn't already represented there).
2. Create a branch: `<type>/<short-description>` (e.g.,
   `feat/cpu-immediate-addressing`, `fix/ppu-scroll-wrap`).
3. Make changes. Keep commits focused.
4. Run the local quality gate before pushing.
5. Open a PR. Reference the ticket(s) and any relevant `docs/` files.

## Quality gate

Before opening a PR, ensure:

<FILL — language-appropriate. Examples:

### Rust
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] New public items have rustdoc

### Python
- [ ] `ruff check` passes
- [ ] `ruff format --check` passes
- [ ] `mypy src` passes
- [ ] `pytest` passes
- [ ] New public functions have type hints and docstrings

### C / C++
- [ ] `clang-format --dry-run --Werror src/**/*.{c,cpp,h}` passes
- [ ] Build is clean with `-Wall -Wextra -Wpedantic`
- [ ] `ctest` passes
>

## Documentation expectations

- New subsystems get a doc in `docs/`.
- Architecture-affecting changes update `docs/architecture.md`.
- User-visible changes are noted in `CHANGELOG.md` under `[Unreleased]`.
- Ticket completion is reflected in the relevant `to-dos/` sprint file.

## Commit messages

<FILL — pick a convention. Common options:

# Conventional Commits:
Use [Conventional Commits](https://www.conventionalcommits.org):
`<type>(<scope>): <subject>`

Types: feat, fix, docs, refactor, test, chore, perf, build, ci.

# Or imperative-mood:
Imperative subject ≤72 chars, blank line, then optional body
explaining the why (not the what — the diff shows the what).
>

## Code review

- One reviewer minimum; two for changes to `docs/architecture.md` or
  cross-subsystem refactors.
- Reviewers focus on correctness, design, and adherence to the relevant
  `docs/` specification.
- Discussion is preferred over deferral; if a comment can't be resolved
  in review, file a follow-up ticket explicitly.
