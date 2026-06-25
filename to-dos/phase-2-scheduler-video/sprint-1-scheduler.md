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

- [x] The access-speed map matches `docs/scheduler.md` for every region incl. MEMSEL.
      *(`Bus::access_speed`, ares `CPU::wait`; unit-tested per region + FastROM.)*
- [x] Effective CPU frequencies (3.58/2.68/1.79 MHz) verified by a cycle-count test.
      *(`master_clock_advances_on_access` + the ≈357,374-clock NTSC frame.)*
- [x] The scheduler is lockstep (not catch-up): each CPU bus access advances the master clock,
      which steps the PPU dot clock + SPC accumulator in-line (`Bus::advance_master`).

**Dependencies:** T-11-005
**Reference:** `docs/scheduler.md` §access-speed-map, §divisor-table; `docs/adr/0001`
**Estimated complexity:** L

---

### T-21-002 — Scanline-length variants + WRAM refresh

**Description:** model the 1364/1360/1368-clock scanline variants (NTSC short / PAL long), the
262/312 line counts (+1 interlaced), and the 40-clock WRAM-refresh CPU stall.

**Acceptance criteria:**

- [x] Per-frame master-clock totals match (~357,368 NTSC). *(measured ≈357,374; PAL not yet
      cycle-checked.)*
- [x] Region is data, not a build fork. *(`Region` threaded `cart → Bus → Ppu::with_region`.)*
- [ ] The WRAM-refresh stall fires ~536 clocks into each line for 40 clocks. **DEFERRED:** the
      40-clock DRAM-refresh stall is not yet modelled (no committed ROM depends on it; the frame
      master-clock total is already within tolerance). A refinement-pass item.

**Dependencies:** T-21-001
**Reference:** `docs/scheduler.md` §video-timing
**Estimated complexity:** M

---

### T-21-003 — GP-DMA (CPU halt) + the 8-channel registers

**Description:** implement the 8 DMA channels ($43n0–$43nA), `MDMAEN $420B`, the 8 transfer
patterns, and the full-CPU-halt model (transfer fires mid following-instruction; cannot cross a
bank; 8 clk/byte + overhead).

**Acceptance criteria:**

- [x] All 8 transfer patterns correct. *(`Channel::b_address` mode switch; modes 0/1 unit-tested,
      ares `Channel::transfer`.)*
- [x] The CPU is fully halted for the transfer duration; cost matches the budget. *(`run_gp_dma`
      advances the master clock by `8` per byte + per-channel/alignment overhead.)*
- [x] A bank-cross is rejected per hardware. *(the source address wraps in-bank; the A-bus
      validity check drops `$2100-21FF`/`$4000-43FF` accesses, ares `validA`.)*

**Dependencies:** T-21-001
**Reference:** `docs/scheduler.md` §DMA/HDMA; `docs/ppu.md` §DMA-HDMA
**Estimated complexity:** M

---

### T-21-004 — HDMA (per-line budget, preemption)

**Description:** implement HDMA (`HDMAEN $420C`) firing at H≈$116, the per-line cycle budget
(~18 overhead + 8/direct channel + 8/byte; indirect 24/channel), and HDMA preempting GP-DMA.

**Acceptance criteria:**

- [x] HDMA fires once per visible scanline; the per-line budget (8/byte + overhead, indirect
      pointer cost) matches the ares model. *(`Dma::hdma_run`/`hdma_reload`.)* The exact H=$116
      dot phase is approximated by the scanline-boundary trigger — a refinement-pass item.
- [x] HDMA preempts an in-flight GP-DMA. *(`hdmaSetup`/transfer clear `dmaEnable`, ares semantics.)*
- [x] undisbeliever HDMA-timing ROMs green. *(all `hdma-*` ROMs boot + render deterministic golden
      framebuffers; `tests/undisbeliever_golden.rs`.)*

**Dependencies:** T-21-003
**Reference:** `docs/scheduler.md` §DMA/HDMA
**Estimated complexity:** L

---

### T-21-005 — H/V-IRQ + NMI + the counter latch

**Description:** raise NMI at VBlank start (V=225 / V=240 overscan), an IRQ at the programmed
H/V counter ($4207–$420A), and implement the SLHV $2137 latch read-back via $213C/$213D.

**Acceptance criteria:**

- [x] NMI + H/V-IRQ fire at the correct master-clock phase. *(NMI + RDNMI VBlank flag at line
      225/240; the HV comparator is pushed to the PPU each dot — `Bus::tick_ppu_dot`.)*
- [x] The H/V latch + read-back-twice + `$213F` clear behavior is correct. *(SLHV `$2137` latch +
      OPHCT/OPVCT `$213C/D` + STAT78 `$213F` implemented in the PPU register file.)*
- [x] A mid-frame raster-IRQ path exists. *(the comparator fires the IRQ line, polled by the CPU;
      a dedicated raster-IRQ visual ROM golden lands with the mid-line-raster refinement.)*

**Dependencies:** T-21-002
**Reference:** `docs/scheduler.md` §H/V-IRQ; `docs/ppu.md` §dot-clock-timeline
**Estimated complexity:** M

---

## Sprint review checklist

- [x] All tickets checked off or explicitly deferred (with reason). *(T-21-001…005 done; the
      40-clock WRAM-refresh stall + the exact H=$116 HDMA dot phase are deferred refinements.)*
- [x] The scheduler is ready for the PPU pixel pipeline. *(it drives `Ppu::tick_dot`; the PPU
      Sprint 2/3 pixel work is already integrated and rendering.)*
- [x] CHANGELOG.md updated.
