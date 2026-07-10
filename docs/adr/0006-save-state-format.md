# ADR 0006 — The save-state binary format + versioning policy

## Status

Accepted (`v0.2.0 "Persistence"`, `to-dos/VERSION-PLAN.md`).

**Progress:** fully implemented and proven. The wire-format primitives (`rustysnes-savestate`:
`SaveWriter`/`SaveReader`/`SaveStateError`), the `Board::save_state`/`load_state` trait hooks
(every coprocessor board), `Cpu`/`Ppu`/`Apu`, and `System::save_state()`/`load_state()` — the
versioned envelope (4-byte magic `b"RSNS"` + `u16` format version, wrapping `Cpu`, the whole
`Bus` including the cart's coprocessor state and battery SRAM, and the SA-1 second CPU when
present) — are all landed. The round-trip determinism test that is this format's actual spec
(`crates/rustysnes-test-harness/tests/save_state_determinism.rs`, T-52-004) is green across a
no-coprocessor ROM, a `Curated` Super FX ROM, and a `BestEffort` commercial-coprocessor ROM.

## Context

Save-states are the prerequisite for rewind, run-ahead, netplay rollback, and TAS movie replay
(`to-dos/VERSION-PLAN.md`'s `v0.2.0`-`v0.8.0` ladder) — every one of those features snapshots
and restores machine state, so the format decided here becomes a long-lived public contract the
moment `v1.0.0` declares the save-state/core API stable. Getting the shape right once, rather
than iterating on a de-facto format under later features' time pressure, is why this is an ADR
and not just an implementation detail.

The state to snapshot spans `rustysnes-core::Bus` (WRAM, PPU, APU/ARAM, DMA, the master-clock
`Clock`), `rustysnes-cart::Cart`/`Board` (every coprocessor's register file — each board already
carries its state as plain struct fields per the `#![no_std]` architecture, so this is a
serialize-the-struct exercise, not new state-tracking machinery), and the CPU register file.
The determinism contract (`docs/adr/0004`) is what makes this tractable at all: a save-state is
exactly "the pure function's argument tuple `(seed, ROM-derived-state, input-so-far)` collapsed
to its current value," and round-trip fidelity is provable by re-running from a restored state
and diffing the deterministic output against a fork that kept running uninterrupted.

## Decision

- **Format:** a versioned, tagged binary blob — a fixed header (a 4-byte magic, `b"RSNS"`, plus a
  `u16` format version) followed by one length-prefixed section per top-level component (`Cpu`,
  `Bus` core fields, `Ppu`, `Apu`, `Cart`/`Board` — the board's own `Debug`
  impl already enumerates its exact fields, giving a natural per-board serialization surface).
  Sections are ordered and length-prefixed specifically so an unknown/newer section from a
  future format version can be skipped rather than corrupting the whole load.
- **No `serde`/reflection magic**: every `Board` implementation writes an explicit
  `save_state(&self, w: &mut impl Write)` / `load_state(&mut self, r: &mut impl Read)` pair (or
  the `#![no_std]`-compatible equivalent — a `&mut [u8]` cursor, not `std::io`), mirroring the
  project's existing "explicit, no derive-magic" style for the `Board` trait itself
  (`docs/cart.md`). This keeps the no_std/wasm targets byte-identical to native (no macro
  expansion divergence) and keeps each board's format change local to that board's file.
- **Versioning/compat policy:** bump the format `u16` whenever any section's byte layout
  changes (a board adds/removes/reorders a field, a new coprocessor needs a new section kind).
  `load_state()` rejects a save whose major format version it doesn't recognize with a typed
  error (never silently truncates/zero-fills — the same honesty-gate posture `docs/adr/0003`
  already applies to coprocessor accuracy).
  **Correction (`v0.7.0`, the format's first real bump — see "Bump log" below): the
  "minor bumps stay backward-loadable" claim this paragraph originally made was aspirational, not
  actually implemented, and has been removed.** `load_state()` only ever checks `found >
  FORMAT_VERSION` (rejects strictly-newer blobs); it does not special-case `found <
  FORMAT_VERSION` at all — it always parses using the CURRENT code's section layout, regardless of
  what version number an older blob declares. In practice this means a section byte-layout change
  (the only thing `FORMAT_VERSION` is meant to track) makes an older blob fail to load — cleanly,
  as a real `SaveStateError` (`Truncated`/`UnexpectedTag`, since sections are length-prefixed and
  the mismatch surfaces locally), never a silent misread — but NOT gracefully in the sense of
  actually restoring old state. Real per-version section migration (skip/adapt an older section's
  bytes into the current in-memory shape) is a genuinely bigger feature, not implemented, and not
  planned unless a concrete need for it emerges. The bump's actual, verified job today is
  narrower than originally claimed: it's a required signal (so the version number itself changes
  whenever a layout does, catching a developer who forgets to bump it) plus a guarantee that the
  failure mode is loud, not silent corruption — proven by
  `crates/rustysnes-test-harness/tests/save_state_backward_compat.rs`.

### Bump log

- **`1` → `2` (`v0.7.0 "Resolution"`):** the `Ppu`'s `PPU0` section grew — the framebuffer's
  backing storage is now always allocated at hi-res capacity (512×239 words, up from 256×239) to
  support true hi-res (Modes 5/6) output, and a new `frame_hires` bool was added
  (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision). `tests/golden/savestate-v1-gilyon.bin`
  is the real `FORMAT_VERSION = 1` fixture (captured from the pre-bump code against the committed
  gilyon `cputest-basic.sfc`) the regression test above loads to prove the mismatch fails loudly.
- **The round-trip determinism test is the spec**: save → run N frames on a cloned/forked
  system → load the save into the original → run the same N frames → assert byte-identical
  framebuffer + audio output between the two. This extends `docs/adr/0004`'s existing
  determinism-test pattern rather than inventing a new verification method.
- **Rate control, rewind, and run-ahead stay in the frontend** (already decided by
  `docs/adr/0004`) — this ADR only fixes the wire format `System::save_state()`/`load_state()`
  produce/consume; the ring-buffer-of-snapshots orchestration for rewind, and the resimulate-
  and-discard orchestration for run-ahead, are frontend-crate concerns built on top of it.

## Consequences

- (+) A stable, inspectable, versioned format that every post-`v1.0.0` Reach feature (netplay,
  TAS movies) can depend on without re-deriving its own snapshot mechanism.
- (+) Per-board `save_state`/`load_state` methods stay colocated with each board's own register
  fields, so a coprocessor implementer touches one file, not a central serialization registry.
- (+) The honesty-gate posture (reject unknown formats loudly) matches this project's existing
  house style rather than introducing a new failure mode class.
- (−) No `serde` means more boilerplate per board than a `#[derive(Serialize)]` would — accepted
  because it keeps `#![no_std]` targets clean and keeps each board's format change auditable in
  isolation, matching the trade-off already made for the `Board` trait itself.
- (−) A future format-breaking change (a `MAJOR` version per `to-dos/VERSION-PLAN.md`'s
  versioning rule) invalidates old save-states. Acceptable and already anticipated: the
  fractional-timebase refactor (`docs/adr/0002`), if it ever lands, is explicitly documented as
  "the one release expected to break byte-identity / save-state compatibility."
