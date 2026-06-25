# Sprint 1 — BestEffort coprocessors via the shared core

**Phase:** Phase 7 — Breadth
**Sprint goal:** the BestEffort NEC-DSP siblings + ST010/011 ride the shared µPD77C25/96050
engine, and the simple ASICs (OBC1, S-RTC) land — all honesty-gated.
**Estimated duration:** 1–2 weeks

## Tickets

### T-71-001 — DSP-2 / DSP-3 / DSP-4 on the shared µPD77C25 core

**Description:** drive each chip's program/data ROM through the existing shared LLE engine; tier
each BestEffort.

**Acceptance criteria:**

- [ ] Each boots its single game (with the user-supplied chip ROM) to a reference screenshot.
- [ ] All three tier BestEffort; the honesty gate stays green.
- [ ] `docs/STATUS.md` matrix updated.

**Dependencies:** T-41-002 (the shared core; the DSP-1 sprint)
**Reference:** `docs/cart.md` §coprocessor-families; `docs/adr/0003`
**Estimated complexity:** M

---

### T-71-002 — ST010 / ST011 on the shared µPD96050 core

**Description:** map the ST01x register / battery-RAM windows ($600000–$67FFFF / $680000–
$6FFFFF) and run them through the shared core (µPD96050 ≈ 77C25 successor).

**Acceptance criteria:**

- [ ] Both boot their single game to a reference screenshot.
- [ ] Battery RAM round-trips deterministically.
- [ ] Tiered BestEffort.

**Dependencies:** T-71-001
**Reference:** `ref-docs/2026-06-24-coprocessors.md` §B (ST010/011)
**Estimated complexity:** M

---

### T-71-003 — OBC1 (HLE) + S-RTC (frozen time)

**Description:** implement OBC1 (sprite-table builder → OAM DMA, trivial HLE) and S-RTC (HLE
backed by frozen/seeded time, never host wall-clock).

**Acceptance criteria:**

- [ ] OBC1's single game boots to a reference screenshot.
- [ ] S-RTC reads frozen/seeded time; a determinism test passes (`docs/adr/0004`).
- [ ] Both tiered BestEffort.

**Dependencies:** T-41-004
**Reference:** `docs/cart.md`; `docs/adr/0004`
**Estimated complexity:** S

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] The shared-core economy is realized; the honesty gate stays green.
- [ ] CHANGELOG.md updated.
