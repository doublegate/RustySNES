# Sprint 1 — SPC700 core + the async resync

**Phase:** Phase 3 — Audio
**Sprint goal:** the SPC700 executes its opcode set to oracle 0-diff, and the integer
relative-time accumulator keeps it coherent with the main CPU at the four ports + the
once-per-scanline forced sync, including the IPL boot handshake.
**Estimated duration:** 2 weeks

## Tickets

### T-31-001 — SPC700 instruction core

**Description:** implement the SPC700 register set, addressing modes, and opcode set (all but
SLEEP/STOP edge cases), running on its own ~1.024 MHz timebase + ARAM.

**Acceptance criteria:**

- [ ] SingleStepTests/spc700 (MIT) 0-diffs on state + bus activity.
- [ ] gilyon SPC golden `tests*.txt` tables match (except SLEEP/STOP).
- [ ] The 3 timers (two @ 8 kHz, one @ 64 kHz) tick on the SMP timebase.

**Dependencies:** T-21-002 (the scheduler's scanline hook)
**Reference:** `docs/apu.md` §hardware-facts; `docs/testing-strategy.md`
**Estimated complexity:** L

---

### T-31-002 — The four communication ports + the IPL boot handshake

**Description:** implement the `$2140–$2143` ↔ `$F4–$F7` port latches (each two one-way
latches) and the 64-byte IPL ROM boot handshake that uploads the audio program.

**Acceptance criteria:**

- [ ] A port read returns what the *other* side last wrote (not an echo).
- [ ] The IPL upload handshake completes for a known audio program.
- [ ] Unit tests for each port direction.

**Dependencies:** T-31-001
**Reference:** `docs/apu.md` §hardware-facts, §edge-cases
**Estimated complexity:** M

---

### T-31-003 — The integer relative-time accumulator + sync points

**Description:** implement the signed accumulator (CPU step → −N×24,576,000; SMP step →
+N×21,477,272) and resync the SMP up to "now" on every port access and once per scanline.

**Acceptance criteria:**

- [ ] The accumulator is integer-exact (no floating point).
- [ ] The SMP resyncs on `$2140–$2143` access and on the per-scanline hook.
- [ ] A save-state round-trip preserves the accumulator bit-identically (`docs/adr/0004`).
- [ ] blargg `spc_mem_access_times` green.

**Dependencies:** T-31-002
**Reference:** `docs/scheduler.md` §async-resync; `docs/apu.md` §2; `docs/adr/0001`
**Estimated complexity:** L

---

### T-31-004 — Determinism guard (no drift, no host time)

**Description:** ensure the SPC domain uses a fixed nominal 1.024 MHz in the deterministic core
and that no host wall-clock / RNG leaks in; stage the optional drift toggle as a frontend-only,
off-by-default setting.

**Acceptance criteria:**

- [ ] The deterministic core has zero host-time / OS-RNG references.
- [ ] A seed+ROM+input replay is bit-identical on the audio stream.
- [ ] The drift toggle (if stubbed) lives outside the core.

**Dependencies:** T-31-003
**Reference:** `docs/adr/0004`; `docs/apu.md` §determinism-caveat
**Estimated complexity:** S

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] The SPC700 + resync are ready for the S-DSP pipeline (Sprint 2).
- [ ] CHANGELOG.md updated.
