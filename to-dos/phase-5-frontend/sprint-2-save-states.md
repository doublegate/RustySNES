# Sprint 2 — Save-states (v0.2.0 "Persistence")

**Phase:** Phase 5 — Frontend
**Sprint goal:** a versioned, deterministic core-wide snapshot format
(`docs/adr/0006-save-state-format.md`) and `System::save_state()`/`load_state()`, proven by a
round-trip determinism test. This is the prerequisite for rewind/run-ahead (Sprint 3), netplay
(Phase 8 `v0.8.0`), and TAS movies (Phase 8 `v0.8.0`) — none of those build until this lands.
**Estimated duration:** 1–2 weeks
**Release:** `v0.2.0 "Persistence"` (`to-dos/VERSION-PLAN.md`)
**Progress:** `rustysnes-savestate` (the `SaveWriter`/`SaveReader`/`SaveStateError` wire-format
primitives, `docs/adr/0006`) landed as a new leaf crate, including an allocation-free nested-
section writer (found in review — the naive version allocated a `Vec` per section, a real
concern given rewind/run-ahead will create/restore save-states repeatedly per frame).
`Board::save_state`/`load_state` hooks added with a default no-op (covers `LoRom`/`HiRom`/
`ExHiRom` for free), implemented for `Obc1Board`, `Dsp1Board`, `NecDspVariantBoard` (which
covers DSP-1/DSP-2/DSP-4/ST010 via the shared `Upd77c25` engine's own `save_state`/`load_state`),
`Cx4Board` (its `Hg51b` core's full register/IO/cache/stack state), and `Sdd1Board` (its MMC
registers, DMA-snoop shadow state, and `Decompressor`'s full mid-stream entropy-decoder state).
Untrusted-input validation is load-bearing here, not decorative: `Obc1Board::load_state` rejects
an out-of-range cursor (a real finding — an unvalidated value would panic on the next register
access) and any section with unconsumed trailing bytes; `Upd77c25::load_state` masks (not
rejects) its pointer registers to their revision-correct hardware widths (a bot-flagged finding
on both PR #4 and #5 — see CHANGELOG); `Decompressor::load_state` rejects an out-of-range PEM
context status (a semantic state-machine index) while masking `current_bitplane` (a genuine
3-bit hardware quantity) — the same width-mask-vs-semantic-reject distinction applied
consistently across every board implemented so far. Extended to `SuperFxBoard` (its `Gsu`
core's full state, including the in-flight per-access checkpoint queue that
master-clock-interleaved `tick`-driven execution can leave mid-flight at any save point — a
claimed queue length/cursor beyond what real execution could ever produce is rejected, not
trusted) and `Sa1Board` (the full register file, I-RAM, H/V timer, and DMA staging flags;
BW-RAM stays excluded, captured separately via `Board::sram`). Completed with `Spc7110Board`
(every DCU/data-port/ALU/memory-control register + `dcu_tile`, with `dcu_offset` masked since it
indexes it directly) and its `Decompressor` (a prediction index outside `EVOLUTION`'s range is
rejected, `bpp`/`bits` are bounded to the only values real execution ever produces) and paired
`EpsonRtc` (an out-of-range handshake-state discriminant is rejected) — **T-52-002's
board-coverage acceptance criterion is now fully met, every coprocessor board round-trips its
state**. T-52-003 completed its per-subsystem half: `Cpu` (the full 65C816 register file +
`WAI`/`STP` latches + cycle counter), `Ppu` (VRAM/CGRAM/OAM, the full register file including
the window unit, write latches, the dot/scanline timeline, interrupt/frame polls, `region`, and
the framebuffer), and `Apu` (`Spc700` + `Dsp` + ARAM + the `$00F0-$00FF` register file + timers +
the in-flight instruction micro-op plan — the SPC700 analogue of the GSU's
`pending_clocks`/`pending_idx`) all round-trip their exact state now. T-52-003 completes with
`System::save_state()`/`load_state()` — the versioned envelope (4-byte magic `b"RSNS"` + `u16`
format version) wrapping `Cpu`, the whole `Bus` (`Ppu`/`Apu`/`Dma`/`Clock`/`MulDiv`/WRAM, plus the
cart's coprocessor state + battery SRAM when a cart is loaded), and the SA-1 second CPU + its
master-clock catch-up accounting when present. A save-state's cart/SA-1 presence is
cross-checked against the target `System`'s own installed state on load (restoring a
cart-carrying save-state requires the caller to have already loaded the SAME ROM first — no ROM
byte is ever embedded, the same posture every coprocessor's firmware already follows) — a
mismatch is rejected, not silently corrupted. **T-52-003 is now fully complete.** Remaining: the
round-trip determinism test — see T-52-004 below.

## Tickets

### T-52-001 — Accept ADR 0006 (the save-state format decision)

**Description:** `docs/adr/0006-save-state-format.md` is drafted `Proposed`; review it against
the actual `Board`/`Bus`/`Cpu`/`Apu`/`Ppu` state each subsystem carries today, adjust if a real
field-layout surprise turns up during T-52-002/003, and flip its status to `Accepted` once the
format is implemented and the round-trip test (T-52-004) is green.

**Acceptance criteria:**

- [x] ADR 0006 status is `Accepted`.
- [x] Any deviation from the ADR's drafted format is folded back into the ADR text (the ADR
      stays the source of truth for the format, not just a historical proposal) — the header
      shape (magic + format version only, no crate-version string) was corrected during review.

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

- [x] Every board with non-ROM/SRAM state round-trips its exact register file through
      `save_state`/`load_state` (a per-board unit test: mutate a few registers, save, zero the
      board, load, assert equality).
- [x] The default no-op impl compiles and is exercised for at least one ROM-only board (`LoRom`
      with no coprocessor).
- [x] `#![no_std]` holds: `cargo build -p rustysnes-cart --no-default-features` (the workspace's
      existing no_std gate) still passes.

**Dependencies:** T-52-001
**Reference:** `docs/adr/0006-save-state-format.md`, `docs/cart.md`
**Estimated complexity:** L

---

### T-52-003 — `System::save_state()`/`load_state()` (the versioned envelope)

**Description:** in `rustysnes-core`, implement the format header (a 4-byte magic + `u16` format
version) + length-prefixed sections wrapping `Cpu`, the `Bus`-owned state (WRAM,
DMA, `Clock`), `Ppu`, `Apu`, and `Cart`/`Board` (via T-52-002's per-board hooks). Replace the
`Unsupported` stubs `ref-proj/RUSTYMU-INTEGRATION.md` documents for
`System::save_state()`/`load_state()` with real implementations. `load_state()` rejects an
unrecognized major format version with a typed error (never silently truncates/zero-fills, per
ADR 0006).

**Acceptance criteria:**

- [x] `System::save_state() -> Vec<u8>` (or the no_std-compatible equivalent) and
      `System::load_state(&mut self, &[u8]) -> Result<(), SaveStateError>` both exist. Exercised
      end-to-end on a booted commercial ROM is T-52-004's job (the determinism test itself IS
      that exercise); this ticket's own tests cover the no-cart path plus magic/version
      rejection.
- [x] An unrecognized/corrupt format is rejected with a typed error, not a panic or silent
      partial load (`bad_magic_is_rejected_not_panicked_on`,
      `newer_format_version_is_rejected_not_panicked_on`).
- [x] `#![no_std]` gate holds for `rustysnes-core` too.

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

- [x] The round-trip test is green for all three sampled ROM categories
      (`crates/rustysnes-test-harness/tests/save_state_determinism.rs`: a no-coprocessor ROM
      (committed gilyon `cputest-basic.sfc`, always present), a `Curated` Super FX Krom ROM, and
      a `BestEffort` commercial coprocessor ROM — the latter two self-skip when the gitignored
      external corpus is absent, matching every other on-cart test in this suite).
- [x] The test lives in `crates/rustysnes-test-harness/tests/` alongside the existing
      determinism/golden-framebuffer tests, following their existing structure.
- [x] CHANGELOG.md `[Unreleased]` gets the `v0.2.0` entry once this lands.

**Dependencies:** T-52-003
**Reference:** `docs/adr/0004-determinism-contract.md`, `docs/adr/0006-save-state-format.md`
**Estimated complexity:** M

---

## Sprint review checklist

- [x] All tickets checked off or explicitly deferred (with reason) — T-52-001 through T-52-004
      all fully complete.
- [x] ADR 0006 flipped to `Accepted`.
- [x] `docs/STATUS.md`'s frontend row updated: save-states no longer "deferred."
- [x] CHANGELOG.md now has a real `[0.2.0] "Persistence"` release section describing the format
      + the round-trip proof (renamed from `[Unreleased]`, with a fresh empty `[Unreleased]`
      above it).
- [x] `to-dos/VERSION-PLAN.md`'s `v0.2.0` entry checked off; `v0.3.0 "Continuum"` (rewind/
      run-ahead, Sprint 3) is next up.
