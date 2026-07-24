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
- **`3` → `4` (AccuracySNES `F1.02`):** `crate::bus`'s `BUS0` section grew by two `u16` — the
  gamepad **shift registers**, which used to share storage with the button state. A manual `$4016`
  read shifted the button word itself, so a program that strobed twice in one frame read all-ones
  the second time and a manual read corrupted the auto-read result at `$4218-$421F`. Both were
  invisible to a frontend that rewrites the button state every frame, which is why the bug survived
  until a test ROM strobed twice with nobody watching. The registers are saved for the same reason
  `Bus::joypad` already was: real, CPU-observable controller-port state.
- **`2` → `3` (`v0.9.0`, Phase 7 niche peripherals):** `crate::bus`'s `BUS0` section grew — a new
  WRIO (`$4201`/`$4213`) `pio` byte plus each controller port's `crate::controller::PortState`
  (which peripheral is attached — Mouse/Super Scope/Super Multitap — plus that device's own
  runtime shift-register/latch state, saved for the same reason `Bus::joypad` already was: it's
  real, CPU-observable controller-port hardware state, not host debug tooling). Reuses the exact
  `1`→`2` mechanism above; no new fixture was added since the existing `savestate-v1-gilyon.bin`
  regression already proves the general "an older blob's section fails to parse under a newer,
  larger section layout" contract this bump also relies on.
- **`4` → `5` (Tier-1 T-CA-01/03):** `crate::bus`'s `BUS0` section grew — the in-flight automatic
  joypad read's start snapshot (`joypad_auto_pending`, two `u16`) and busy deadline
  (`auto_joypad_busy_until`, one `u64`). The automatic read is now a timed ~4224-clock operation
  whose `$4212` bit 0 busy state and deferred `$4218-$421F` publish are CPU-observable, so they are
  saved for the same reason `Bus::joypad` already was — real, CPU-observable machine state — rather
  than reset on load (the earlier iteration left them out, mirroring the `$43xB` scratch latch, but a
  save mid-window would then have dropped an observable in-flight read). Reuses the same
  older-blob-fails-loudly mechanism (`savestate-v1-gilyon.bin` proves it); a mid-window round-trip
  unit test proves the busy state and deferred snapshot survive save/load exactly.
- **`5` → `6` (Tier-1 T-CA-02):** `crate::bus`'s `BUS0` section grew by two bytes — the RDNMI/TIMEUP
  hold flags (`rdnmi_hold`/`irq_hold`, one `bool` each). For four master clocks (one dot) after a
  VBlank/IRQ edge the hardware holds `/NMI` and `/IRQ`, so a `$4210`/`$4211` read in that window
  returns bit 7 set without clearing it. The hold is CPU-observable machine state (a save taken
  inside the window must restore the same read-clear behavior), so it is serialized for the same
  reason the flags themselves are. Same older-blob-fails-loudly guarantee.
- **`6` → `7` (T-CA-10 Phase 4b):** `rustysnes_ppu`'s `PPU0` section grew by one byte — the OAM
  sprite-evaluation seed (`pd_oam_eval_seed`). During a rendering scanline the sprite evaluator owns
  the OAM address bus, so a `$2104` write is redirected to the evaluator's index (the Uniracers
  in-render OAM quirk); that index is seeded at line start and, with OAM priority rotation, diverges
  from `OAMADDR` after the redirected writes advance it. It therefore cannot be re-derived on load
  (unlike the other per-dot compositor state, which is transient and re-fetched per line) and must
  persist for a mid-line save to restore identical machine state — the same reason MesenCE serializes
  `_oamEvaluationIndex`. The byte is written **unconditionally** (0 when the `per-dot-compositor`
  feature is off) so the `PPU0` layout is identical across feature builds. Same older-blob-fails-loudly
  guarantee.
- **`7` → `8` (T-CA-10 Phase 4b over-flag cursor):** `PPU0` grew by one more byte — `pd_over_eval_seed`,
  the line-start priority-rotation seed the sprite over-flag (STAT77 `$213E`) timing evaluates from.
  The over-flag set-dots (the dots at which Range/Time Over trip) are themselves transient — re-derived
  per line, and re-derived again on load — but they depend on the line-start OAM evaluation order,
  which with priority rotation diverges from `OAMADDR` after redirected active-display `$2104` writes.
  If the recompute-on-load keyed off the live `oam_address`, a mid-line save/load on a rotated line
  would shift `$213E` timing (a determinism break); keying off this persisted line-start seed instead
  restores identical timing. Same reasoning as `pd_oam_eval_seed`, applied to the over-flag eval (which
  runs one line ahead of the paint and covers line 0). Regression-locked by
  `over_flag_timing_survives_mid_line_save_load_with_diverged_oamaddr`. Same older-blob-fails-loudly
  guarantee.
- **`8` → `9` (auto-read start timing):** `rustysnes_core`'s `BUS0` section grew by eight bytes —
  `auto_joypad_start_at`, the `clock.master` instant an armed automatic joypad read is scheduled to
  begin. Hardware does not start the read at the VBlank edge but ~dot 32.5-95.5 into the first VBlank
  line (`AUTO_JOYPAD_START_DELAY`; AccuracySNES `F1.08`/`F1.10`), so `$4212` bit 0 reads not-busy for
  that window and the NMI-entry race is observable. A save taken between the edge and the start must
  restore the pending deadline, or the read never begins on load and `$4212`/`$4218-$421F` desync.
  Regression-locked by `auto_joypad_pending_start_survives_save_load`. Same older-blob-fails-loudly
  guarantee.
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
