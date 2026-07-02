# Sprint 2 — Save-states (v0.2.0 "Persistence")

**Phase:** Phase 5 — Frontend
**Sprint goal:** a versioned, deterministic core-wide snapshot format
(`docs/adr/0006-save-state-format.md`) and `System::save_state()`/`load_state()`, proven by a
round-trip determinism test. This is the prerequisite for rewind/run-ahead (Sprint 3), netplay
(Phase 8 `v1.2.0`), and TAS movies (Phase 8 `v1.4.0`) — none of those build until this lands.
**Estimated duration:** 1–2 weeks
**Release:** `v0.2.0 "Persistence"` (`to-dos/VERSION-PLAN.md`)
**Progress:** `rustysnes-savestate` (the `SaveWriter`/`SaveReader`/`SaveStateError` wire-format
primitives, `docs/adr/0006`) landed as a new leaf crate; `Board::save_state`/`load_state` hooks
added with a default no-op (covers `LoRom`/`HiRom`/`ExHiRom` for free) and proven end-to-end on
`Obc1Board` (a round-trip unit test + the no_std gate both green). Remaining boards, `Cpu`/`Ppu`/
`Apu`, and the `System`-level envelope are still open — see T-52-002/003/004 below.

## Tickets

### T-52-001 — Accept ADR 0006 (the save-state format decision)

**Description:** `docs/adr/0006-save-state-format.md` is drafted `Proposed`; review it against
the actual `Board`/`Bus`/`Cpu`/`Apu`/`Ppu` state each subsystem carries today, adjust if a real
field-layout surprise turns up during T-52-002/003, and flip its status to `Accepted` once the
format is implemented and the round-trip test (T-52-004) is green.

**Acceptance criteria:**

- [ ] ADR 0006 status is `Accepted`.
- [ ] Any deviation from the ADR's drafted format is folded back into the ADR text (the ADR
      stays the source of truth for the format, not just a historical proposal).

**Dependencies:** none
**Reference:** `docs/adr/0006-save-state-format.md`
**Estimated complexity:** S

---

### T-52-002 — Per-board `save_state`/`load_state` on the `Board` trait

**Description:** add `fn save_state(&self, w: &mut dyn SaveWriter)` / `fn load_state(&mut self,
r: &mut dyn SaveReader) -> Result<(), SaveStateError>` to `crate::board::Board`
(`crates/rustysnes-cart/src/board.rs`), with a default no-op impl for boards with no extra state
beyond ROM/SRAM (which the cart-level snapshot already covers via `Board::sram`). Implement it
for every coprocessor board that carries register-file state: `Dsp1Board`, `NecDspVariantBoard`,
`Obc1Board`, `Cx4Board` (+ `Hg51b`'s register file/cache/DMA state), `Sdd1Board` (+ its
`Decompressor`), `Spc7110Board` (+ its `Decompressor` and the paired `EpsonRtc`), `SuperFxBoard`
(+ `Gsu`), `Sa1Board`. `SaveWriter`/`SaveReader` are `#![no_std]`-compatible cursors over
`&mut [u8]` / `&[u8]` (no `std::io`), per ADR 0006's "no serde/reflection" decision.

**Acceptance criteria:**

- [ ] Every board with non-ROM/SRAM state round-trips its exact register file through
      `save_state`/`load_state` (a per-board unit test: mutate a few registers, save, zero the
      board, load, assert equality).
- [ ] The default no-op impl compiles and is exercised for at least one ROM-only board (`LoRom`
      with no coprocessor).
- [ ] `#![no_std]` holds: `cargo build -p rustysnes-cart --no-default-features` (the workspace's
      existing no_std gate) still passes.

**Dependencies:** T-52-001
**Reference:** `docs/adr/0006-save-state-format.md`, `docs/cart.md`
**Estimated complexity:** L

---

### T-52-003 — `System::save_state()`/`load_state()` (the versioned envelope)

**Description:** in `rustysnes-core`, implement the format header (magic, format version,
crate-version string) + length-prefixed sections wrapping `Cpu`, the `Bus`-owned state (WRAM,
DMA, `Clock`), `Ppu`, `Apu`, and `Cart`/`Board` (via T-52-002's per-board hooks). Replace the
`Unsupported` stubs `ref-proj/RUSTYMU-INTEGRATION.md` documents for
`System::save_state()`/`load_state()` with real implementations. `load_state()` rejects an
unrecognized major format version with a typed error (never silently truncates/zero-fills, per
ADR 0006).

**Acceptance criteria:**

- [ ] `System::save_state() -> Vec<u8>` (or the no_std-compatible equivalent) and
      `System::load_state(&mut self, &[u8]) -> Result<(), SaveStateError>` both exist and are
      exercised end-to-end on a booted commercial ROM.
- [ ] An unrecognized/corrupt format is rejected with a typed error, not a panic or silent
      partial load.
- [ ] `#![no_std]` gate holds for `rustysnes-core` too.

**Dependencies:** T-52-002
**Reference:** `docs/adr/0006-save-state-format.md`, `ref-proj/RUSTYMU-INTEGRATION.md`
**Estimated complexity:** L

---

### T-52-004 — The round-trip determinism test (the spec, per ADR 0006)

**Description:** extend the existing determinism-contract test pattern (`docs/adr/0004`) with:
boot a commercial ROM, run N frames, save-state, fork the system, run N more frames on BOTH the
original (continuing) and a fresh system loaded from the save-state, assert byte-identical
framebuffer + audio between the two at every subsequent frame. Run across a representative
sample: a Core/Curated ROM (DSP-1 or Super FX), a BestEffort coprocessor ROM (e.g. S-DD1 or
CX4), and a no-coprocessor ROM.

**Acceptance criteria:**

- [ ] The round-trip test is green for all three sampled ROM categories.
- [ ] The test lives in `crates/rustysnes-test-harness/tests/` alongside the existing
      determinism/golden-framebuffer tests, following their existing structure.
- [ ] CHANGELOG.md `[Unreleased]` gets the `v0.2.0` entry once this lands.

**Dependencies:** T-52-003
**Reference:** `docs/adr/0004-determinism-contract.md`, `docs/adr/0006-save-state-format.md`
**Estimated complexity:** M

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] ADR 0006 flipped to `Accepted`.
- [ ] `docs/STATUS.md`'s frontend row updated: save-states no longer "deferred."
- [ ] CHANGELOG.md `[Unreleased]` describes the format + the round-trip proof.
- [ ] `to-dos/VERSION-PLAN.md`'s `v0.2.0` entry checked off; `v0.3.0 "Continuum"` (rewind/
      run-ahead, Sprint 3) opened next.
