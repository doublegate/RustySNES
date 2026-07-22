# RustySNES — Review Style Guide

Project-specific rules fed to the Antigravity reviewer. RustySNES is a cycle-accurate
SNES/Super Famicom emulator at the Mesen2 / ares / higan accuracy bar. Override the
path with the `STYLE_GUIDE` env var in the workflow if you move this file.

## Architecture contracts (never violate)

- **The Bus owns all mutable state** (`rustysnes-core::Bus`); chips borrow `&mut Bus`. Flag any
  chip that reaches around the Bus or holds a back-reference to a peer.
- **Timing is master-clock-driven** (21477270 Hz) through one lockstep scheduler; every chip
  advances on its divisor. Reject free-running or catch-up timing outside the scheduler.
- **The crate graph is strictly one-directional** — no chip crate depends on another;
  `rustysnes-core` ties them together. A new back-edge between peers is blocking.
- **Determinism is a hard contract:** same seed + ROM + input must yield bit-identical AV. The
  frontend owns rate control; nothing wall-clock, OS-RNG, or thread-scheduling-dependent belongs
  in the core.
- **Test ROMs / golden vectors are the spec.** A behavior change without the matching
  `docs/<chip>.md` update, or that re-blesses a golden without justification, is blocking.
- **Additive features are default-off**, so shipped / `no_std` / wasm builds stay byte-identical
  when a feature is off. Flag a default-on new feature.

## Priorities (in order)

1. Correctness and hardware accuracy (verified against a reference, not self-asserted).
2. Determinism and the ownership/timing contracts above.
3. Security: validate external input (ROM/save/network) at boundaries; never `unwrap`/panic on
   untrusted data; no secrets in code, logs, or errors.
4. Tests accompany behavior changes; accuracy-critical paths pin the failing test/ROM first.

## Conventions

- Conventional Commits (`feat|fix|docs|refactor|test|chore|perf|build|ci(scope): subject`),
  imperative, 72-char-or-shorter subject; one logical change per commit.
- Rust edition 2024, toolchain pinned 1.96; workspace lints `pedantic`+`nursery`+`missing_docs`,
  CI is `-D warnings` (every pub item needs a doc comment). Never `--all-features`.
- `unsafe` only in the frontend + FFI, each block with a `// SAFETY:` comment stating the
  invariant. Hot paths stay allocation-free.
- No emojis in code, comments, commits, or docs. Match surrounding style; smallest correct change.
- A chip change updates the chip code AND its `docs/<chip>.md` in the same PR (docs-as-spec).
- **Never commit commercial ROMs** — only derived screenshots/hashes.

## What to flag as BLOCKING

- A break in Bus ownership, master-clock timing, the one-directional crate graph, or determinism.
- Unvalidated external input (ROM/header/save bytes) reaching a sink, or `unwrap`/panic on it.
- A behavior change whose `docs/<chip>.md` (or golden vector) is left untouched.
- Hardcoded credentials, or a committed commercial ROM byte.
- Silent failure paths — swallowed errors, ignored return values, a stubbed-unimplemented path
  that reports success.
- A default-on additive feature, or a breaking save-state/format change outside a major release.

## What to keep as SUGGESTION / NITPICK

- Naming, structure, and readability.
- Missing tests for non-accuracy-critical paths.
- Performance ideas without a measurement (profile before optimizing; document before/after).
