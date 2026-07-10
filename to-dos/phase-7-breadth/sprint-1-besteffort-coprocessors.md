# Sprint 1 — BestEffort coprocessors via the shared core

**Phase:** Phase 7 — Breadth
**Sprint goal:** the BestEffort NEC-DSP siblings + ST010/011 ride the shared µPD77C25/96050
engine, and the simple ASICs (OBC1, S-RTC) land — all honesty-gated.
**Estimated duration:** 1–2 weeks
**Status: complete** (DSP-2/DSP-4/ST010/OBC1/S-RTC implemented and honesty-gated; DSP-3/ST011
explicitly deferred — no verified board/window entry, not a chip-ROM-sourcing gap like the other
residuals on this ladder).

## Tickets

### T-71-001 — DSP-2 / DSP-3 / DSP-4 on the shared µPD77C25 core

**Description:** drive each chip's program/data ROM through the existing shared LLE engine; tier
each BestEffort.

**Acceptance criteria:**

- [x] DSP-2 and DSP-4 each boot their single game (Dungeon Master, Top Gear 3000) to real
      gameplay content — `NecDspVariantBoard` reuses the DSP-1 µPD7725 LLE engine, title-detected
      (`Variant::detect`); DSP-4 needed a DSP-1-style half-boundary split instead of DSP-2's
      generic bit-0 DR/SR split (found + fixed against a real Top Gear 3000 boot-time hardware
      check).
- [x] DSP-2 and DSP-4 tier BestEffort; the honesty gate stays green.
- [ ] **DSP-3 explicitly deferred, not silently dropped:** no verified board/window entry exists
      to pin an implementation against (`necdsp_variant.rs`) — tracked as a named residual in
      `docs/STATUS.md`. Not blocked on a chip-ROM dump like the ST01x/OBC1 items; blocked on
      finding a documented DSP-3 cartridge memory map.
- [x] `docs/STATUS.md` matrix updated.

**Dependencies:** T-41-002 (the shared core; the DSP-1 sprint)
**Reference:** `docs/cart.md` §coprocessor-families; `docs/adr/0003`
**Estimated complexity:** M

---

### T-71-002 — ST010 / ST011 on the shared µPD96050 core

**Description:** map the ST01x register / battery-RAM windows ($600000–$67FFFF / $680000–
$6FFFFF) and run them through the shared core (µPD96050 ≈ 77C25 successor).

**Acceptance criteria:**

- [x] ST010 boots its single game (F1 ROC II) to real gameplay content — same
      `NecDspVariantBoard`, µPD96050 DR/SR bit-0 split + the DP battery data-RAM window.
- [ ] **ST011 explicitly deferred, same reason as DSP-3 above:** no verified board/window entry
      wired (`necdsp_variant.rs`) — a named residual in `docs/STATUS.md`, not a silently-dropped
      chip.
- [x] Battery RAM round-trips deterministically (ST010).
- [x] ST010 tiered BestEffort.

**Dependencies:** T-71-001
**Reference:** `ref-docs/2026-06-24-coprocessors.md` §B (ST010/011)
**Estimated complexity:** M

---

### T-71-003 — OBC1 (HLE) + S-RTC (frozen time)

**Description:** implement OBC1 (sprite-table builder → OAM DMA, trivial HLE) and S-RTC (HLE
backed by frozen/seeded time, never host wall-clock).

**Acceptance criteria:**

- [x] OBC1's single game (Metal Combat: Falcon's Revenge) boots to a real gameplay cinematic —
      dedicated 8 KiB RAM, a reprogrammable cursor over 4-byte slots + a packed status byte.
- [x] S-RTC reads frozen/seeded time; a determinism test passes (`docs/adr/0004`) —
      `coproc::sharprtc::SharpRtcBoard`, unit-test-level coverage (no commercial dump in the local
      corpus for Daikaijuu Monogatari II, its named title; tracked in `docs/rom-test-corpus.md`).
- [x] Both tiered BestEffort.

**Dependencies:** T-41-004
**Reference:** `docs/cart.md`; `docs/adr/0004`
**Estimated complexity:** S

---

## Sprint review checklist

- [x] All tickets checked off or explicitly deferred (with reason): T-71-001 (DSP-2/DSP-4 done,
      DSP-3 deferred — no board), T-71-002 (ST010 done, ST011 deferred — no board), T-71-003
      (done).
- [x] The shared-core economy is realized (one µPD77C25/µPD96050 engine covers DSP-1/2/3/4 +
      ST010/011 — six chips, one engine, per `docs/STATUS.md`); the honesty gate stays green.
- [x] CHANGELOG.md updated (across `v0.4.0`'s coprocessor-completion entries).
