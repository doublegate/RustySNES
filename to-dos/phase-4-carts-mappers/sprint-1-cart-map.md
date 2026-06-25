# Sprint 1 — Memory map + header detection + the honesty gate

**Phase:** Phase 4 — Carts + coprocessors
**Sprint goal:** the cart memory model boots ROMs under LoROM/HiROM/ExHiROM with correct
auto-detection + SRAM/battery, and the Core/Curated/BestEffort honesty gate is live.
**Estimated duration:** 1–2 weeks

## Tickets

### T-41-001 — The `Cart` trait + LoROM / HiROM / ExHiROM map models

**Description:** implement the `Cart` trait and the three map models with their bank-wiring
(LoROM A15-skip, HiROM linear banks, ExHiROM split addressing) and mirroring.

**Acceptance criteria:**

- [ ] Each model maps addresses per `docs/cart.md` §memory-map-models.
- [ ] ExHiROM's $80–$FF / $00–$7D split is correct.
- [ ] DMA cannot cross a bank (consistent with Phase 2).

**Dependencies:** T-21-003
**Reference:** `docs/cart.md` §memory-map-models; `ref-docs/2026-06-24-coprocessors.md` §A
**Estimated complexity:** M

---

### T-41-002 — Header parser + the score-heuristic auto-detection

**Description:** parse the internal header ($FFC0–$FFDF) and score candidate offsets
($7FC0/$FFC0/$40FFC0) on checksum-complement, map-mode match, plausible sizes, and reset-vector
plausibility; strip the spurious 512-byte SMC copier header.

**Acceptance criteria:**

- [ ] The detected map mode + coprocessor family match the known-good database for the canonical
      set.
- [ ] Copier-header detection (`len % 1024 == 512`) strips correctly.
- [ ] A unit test per map mode with a hand-built minimal header.

**Dependencies:** T-41-001
**Reference:** `docs/cartridge-format.md` §auto-detection
**Estimated complexity:** M

---

### T-41-003 — SRAM / battery saves (deterministic)

**Description:** implement board-dependent SRAM windows + battery persistence with a
deterministic round-trip.

**Acceptance criteria:**

- [ ] LoROM + HiROM SRAM windows map per `docs/cart.md` §SRAM.
- [ ] A save → load → save round-trip is byte-identical.
- [ ] Battery flag honored from `$FFD6` low nibble.

**Dependencies:** T-41-002
**Reference:** `docs/cart.md`; `docs/adr/0004`
**Estimated complexity:** S

---

### T-41-004 — The Core/Curated/BestEffort honesty gate

**Description:** add the `CoprocessorTier` enum + a CI test that fails if any BestEffort board
contributes to the accuracy oracle / pass-gate; seed `docs/STATUS.md`'s matrix.

**Acceptance criteria:**

- [ ] `Cart::tier()` returns the board's tier.
- [ ] A CI test asserts no BestEffort board backs the oracle.
- [ ] `docs/STATUS.md` matrix reflects the tier of every board added.

**Dependencies:** T-41-001
**Reference:** `docs/adr/0003`; `docs/testing-strategy.md` §honesty-gate
**Estimated complexity:** S

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] The cart foundation is ready for the coprocessors (Sprints 2–3).
- [ ] CHANGELOG.md updated.
