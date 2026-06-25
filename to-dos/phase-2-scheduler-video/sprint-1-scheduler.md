# Sprint 1 — The master-clock scheduler + DMA/HDMA

**Phase:** Phase 2 — Scheduler + video
**Sprint goal:** the master-clock lockstep scheduler runs the CPU on its variable access map and
re-derives the PPU/HDMA/IRQ phases; the DMA/HDMA cycle-steal model is in place.
**Estimated duration:** 2 weeks

## Tickets

### T-21-001 — The master-clock scheduler + the access-speed map

**Description:** implement `Bus` advancing the 21.477270 MHz master clock; each CPU access
queries the 6/8/12 region map (incl. `$420D` MEMSEL / FastROM) and advances that many ticks;
re-derive the PPU dot phase after each.

**Acceptance criteria:**

- [ ] The access-speed map matches `docs/scheduler.md` for every region incl. MEMSEL.
- [ ] Effective CPU frequencies (3.58/2.68/1.79 MHz) verified by a cycle-count test.
- [ ] The scheduler is lockstep (not catch-up); a unit test proves a mid-step PPU advance.

**Dependencies:** T-11-005
**Reference:** `docs/scheduler.md` §access-speed-map, §divisor-table; `docs/adr/0001`
**Estimated complexity:** L

---

### T-21-002 — Scanline-length variants + WRAM refresh

**Description:** model the 1364/1360/1368-clock scanline variants (NTSC short / PAL long), the
262/312 line counts (+1 interlaced), and the 40-clock WRAM-refresh CPU stall.

**Acceptance criteria:**

- [ ] Per-frame master-clock totals match (~357,368 NTSC / ~425,568 PAL).
- [ ] Region is data, not a build fork.
- [ ] The WRAM-refresh stall fires ~536 clocks into each line for 40 clocks.

**Dependencies:** T-21-001
**Reference:** `docs/scheduler.md` §video-timing
**Estimated complexity:** M

---

### T-21-003 — GP-DMA (CPU halt) + the 8-channel registers

**Description:** implement the 8 DMA channels ($43n0–$43nA), `MDMAEN $420B`, the 8 transfer
patterns, and the full-CPU-halt model (transfer fires mid following-instruction; cannot cross a
bank; 8 clk/byte + overhead).

**Acceptance criteria:**

- [ ] All 8 transfer patterns correct.
- [ ] The CPU is fully halted for the transfer duration; cost matches the budget.
- [ ] A bank-cross is rejected per hardware.

**Dependencies:** T-21-001
**Reference:** `docs/scheduler.md` §DMA/HDMA; `docs/ppu.md` §DMA-HDMA
**Estimated complexity:** M

---

### T-21-004 — HDMA (per-line budget, preemption)

**Description:** implement HDMA (`HDMAEN $420C`) firing at H≈$116, the per-line cycle budget
(~18 overhead + 8/direct channel + 8/byte; indirect 24/channel), and HDMA preempting GP-DMA.

**Acceptance criteria:**

- [ ] HDMA fires at H≈$116; the per-line budget matches `docs/scheduler.md` (≤466 clk worst case).
- [ ] HDMA preempts an in-flight GP-DMA.
- [ ] undisbeliever HDMA-timing ROM green.

**Dependencies:** T-21-003
**Reference:** `docs/scheduler.md` §DMA/HDMA
**Estimated complexity:** L

---

### T-21-005 — H/V-IRQ + NMI + the counter latch

**Description:** raise NMI at VBlank start (V=225 / V=240 overscan), an IRQ at the programmed
H/V counter ($4207–$420A), and implement the SLHV $2137 latch read-back via $213C/$213D.

**Acceptance criteria:**

- [ ] NMI + H/V-IRQ fire at the correct master-clock phase.
- [ ] The H/V latch + read-back-twice + $213F clear behavior is correct.
- [ ] A mid-frame raster-IRQ test ROM behaves.

**Dependencies:** T-21-002
**Reference:** `docs/scheduler.md` §H/V-IRQ; `docs/ppu.md` §dot-clock-timeline
**Estimated complexity:** M

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] The scheduler is ready for the PPU pixel pipeline (Sprint 2).
- [ ] CHANGELOG.md updated.
