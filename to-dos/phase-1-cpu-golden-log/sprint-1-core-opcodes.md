# Sprint 1 — Register file, addressing modes, official opcodes

**Phase:** Phase 1 — CPU + golden oracle
**Sprint goal:** the 65C816 instruction core executes the official opcode set across every
addressing mode in both register widths, with the `CpuBus` carrying access speeds back to the
scheduler.
**Estimated duration:** 2 weeks

## Tickets

### T-11-001 — Register file + status flags + the `CpuBus` trait

**Description:** implement A/X/Y/S/D/DBR/PBR/PC/P + the hidden E flag, the M/X width selection,
and the `CpuBus { read, write, io }` trait whose impls advance the master clock by the region
access speed.

**Acceptance criteria:**

- [x] Register file + flag set with width-aware A/X/Y access. *(`regs.rs`: `a_val`/`set_a`/
      `x_val`/`set_x`/… + `Status`; oracle exercises every width path.)*
- [x] `CpuBus` trait defined; a test double records access (addr, kind, speed). *(implemented as
      `Bus { read24, write24, on_cpu_cycle, poll_nmi, poll_irq }`; the oracle's `TestBus` is the
      access-recording double.)*
- [x] Unit tests for flag set/clear + width selection. *(`cpu/src/tests.rs` + the per-opcode
      oracle.)*

**Dependencies:** T-01-006
**Reference:** `docs/cpu.md` §registers; `docs/scheduler.md` §access-speed-map
**Estimated complexity:** M

---

### T-11-002 — Addressing modes

**Description:** implement all 65C816 addressing modes (immediate, direct page +X/+Y, absolute
+long +indexed, indirect + long-indirect, stack-relative + SR-indirect-indexed) with effective-
address computation honoring D, DBR, and PBR.

**Acceptance criteria:**

- [x] Every addressing mode computes the correct 24-bit effective address. *(`resolve()` covers
      all modes; oracle RAM-diff is 100%.)*
- [x] Bank-wrap + direct-page-wrap edge cases covered by unit tests. *(emulation `DL==0`
      page-lock + bank-0 operand wrap; oracle exercises them.)*
- [x] Page-cross + direct-page-misalignment penalties surfaced to the cycle counter. *(cycle
      column 100%.)*

**Dependencies:** T-11-001
**Reference:** `docs/cpu.md` §timing; `ref-docs/research-report.md` §4
**Estimated complexity:** L

---

### T-11-003 — Load/store/transfer + ALU instruction families

**Description:** implement LDA/STA/LDX/.../ADC/SBC/AND/ORA/EOR/CMP/INC/DEC/shifts/rotates +
the transfer instructions, all width-aware.

**Acceptance criteria:**

- [x] All listed families implemented across their addressing modes. *(LDA/STA/…/ADC/SBC/AND/
      ORA/EOR/CMP/INC/DEC/shifts/rotates/transfers; oracle passes each opcode file.)*
- [x] Decimal-mode (D flag) ADC/SBC correct. *(digit-wise BCD ported from bsnes; the `69/E9`
      decimal oracle paths pass.)*
- [x] Unit tests per family in both 8- and 16-bit widths. *(via the oracle's M/X-width coverage
      + `tests.rs`.)*

**Dependencies:** T-11-002
**Reference:** `docs/cpu.md`; `ref-docs/research-report.md` §4
**Estimated complexity:** L

---

### T-11-004 — Branches, jumps, stack, and mode-control instructions

**Description:** implement Bcc/BRA/BRL, JMP/JML/JSR/JSL/RTS/RTL/RTI, PHA/PLA/... stack ops, and
`REP`/`SEP`/`XCE`/`CLC`/`SEC` (the mode-control path), plus BRK/COP and the vectors.

**Acceptance criteria:**

- [x] `CLC : XCE` enters native mode; `XCE` exchanges E with C; RESET forces emulation mode.
- [x] `REP`/`SEP` change M/X and the next instruction reads the new width.
- [x] Emulation-mode stack stays in page 1; native uses full 16-bit S. *(two push/pull
      disciplines; JSL/RTL fixed to the full-16-bit `pushN`/`pullN` path.)*
- [x] Vector dispatch (RESET/NMI/IRQ/BRK/COP/ABORT) uses the right emulation/native table.
      *(ABORT modelled minimally; the rest oracle-verified.)*

**Dependencies:** T-11-003
**Reference:** `docs/cpu.md` §emulation-vs-native, §vectors
**Estimated complexity:** M

---

### T-11-005 — Cycle counter + first oracle smoke-run

**Description:** wire the per-cycle cost (Σ access speeds + 6×internal cycles + the variable
penalties) and run a small slice of the SingleStepTests/65816 oracle to validate the harness
plumbing end-to-end.

**Acceptance criteria:**

- [ ] The cycle counter matches hand-computed costs for a representative opcode set.
- [ ] One opcode's JSON file 0-diffs on state + per-cycle bus activity through the harness.
- [ ] Failing cases report the first mismatched cycle (not just "FAIL").

**Dependencies:** T-11-004; T-01-005 (oracle license)
**Reference:** `docs/testing-strategy.md` Layer 2; `docs/cpu.md` §timing
**Estimated complexity:** M

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] The oracle slice runs (Sprint 2 drives it to full 0-diff).
- [ ] CHANGELOG.md updated.
