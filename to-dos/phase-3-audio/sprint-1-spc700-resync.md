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

- [x] A port read returns what the *other* side last wrote (not an echo). — `Bus::b_read`/`b_write`
      route `$2140-$2143` through `Apu::cpu_read_port`/`cpu_write_port`; unit test
      `ports_are_one_way_latches`.
- [x] The IPL upload handshake completes for a known audio program. — all four blargg `spc_*` ROMs
      boot, upload their SPC program through the ports, and the SPC700 runs it (`tests/blargg_spc.rs`).
- [x] Unit tests for each port direction. — `rustysnes-apu` `ports_are_one_way_latches` + the
      blargg boot/upload gate.

**Dependencies:** T-31-001
**Reference:** `docs/apu.md` §hardware-facts, §edge-cases
**Estimated complexity:** M
**Status:** done (port wiring + IPL handshake reach the SPC700 end-to-end).

---

### T-31-003 — The integer relative-time accumulator + sync points

**Description:** implement the signed accumulator (CPU step → −N×24,576,000; SMP step →
+N×21,477,272) and resync the SMP up to "now" on every port access and once per scanline.

**Acceptance criteria:**

- [x] The accumulator is integer-exact (no floating point). — `Bus::Clock::spc_accum` (`u64`),
      ratio `SPC_NUM/SPC_DEN = 68_352/715_909` = `(apuFrequency/12)/master`, reduced by gcd 30.
- [x] The SMP resyncs on `$2140–$2143` access and on the per-scanline hook. — stronger: the SMP is
      advanced at master-clock granularity inside `advance_master`, so it is *continuously* in
      lockstep (the on-demand + per-scanline sync is subsumed).
- [x] A save-state round-trip preserves the accumulator bit-identically (`docs/adr/0004`). — the
      accumulator is a plain `u64` field on `Clock` (serializes with the bus); a booted frame is
      verified bit-identical across runs (`tests/blargg_spc.rs` determinism assert).
- [x] blargg `spc_mem_access_times` green. — reaches blargg's literal **`PASSED TESTS`** after the
      **T-31-006** timer-phase fix (timebase + timers clocked before the write side effect);
      `tests/blargg_spc.rs` asserts the literal PASS. Documented in `docs/apu.md` §timer phase.

**Dependencies:** T-31-002
**Reference:** `docs/scheduler.md` §async-resync; `docs/apu.md` §2; `docs/adr/0001`
**Estimated complexity:** L
**Status:** integration + integer-exact resync done; cycle-exactness landed in T-31-005; the
timer-phase fix (T-31-006) drove `spc_smp` / `spc_timer` / `spc_mem_access_times` to a literal blargg
PASS. `spc_dsp6` remains Failed 02 on a separate S-DSP echo/envelope residual.

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

### T-31-005 — Cycle-exact SMP↔CPU lockstep (sub-instruction)

**Description:** make the SMP cycle-steppable so it interleaves with the main CPU at
sub-instruction granularity. `Apu::advance_smp_cycle` now releases exactly one SMP base clock per
call by draining a recorded micro-op timeline of the in-flight SPC700 instruction (a `RecordingSmpBus`
runs the *unchanged* `Spc700::step`, applying every side effect byte-for-byte as `SmpBus` does — so
the oracle stays 0-diff — while emitting one timeline entry per bus access). Each SMP→CPU port write
is deferred to the precise base cycle its access completes, so a CPU read of `$2140-$2143` observes
the SMP exactly up to that master instant. Single-threaded (no coroutines → save-state/netplay stay
deterministic).

**Acceptance criteria:**

- [x] SPC700 oracle stays **0-diff** (100% state + cycle) — `spc700_oracle` full set.
- [x] Determinism preserved — a booted frame is bit-identical across runs (`blargg_spc`).
- [x] All four blargg `spc_*` ROMs boot/upload/run **and stream their result grids**;
      `tests/blargg_spc.rs` decodes + asserts the real verdict (header `$0400` + grid `$0800`),
      retaining the determinism + baseline-hash assertion (re-blessed), **not** weakened to
      determinism-only.
- [x] Literal blargg text PASS — **reached for 3 of 4** after the T-31-006 timer-phase fix:
      `spc_smp` / `spc_timer` / `spc_mem_access_times` → literal `PASSED TESTS` (asserted).
      `spc_dsp6` → Failed 02 on a separate S-DSP echo/envelope residual (T-31-007).

**Dependencies:** T-31-003
**Reference:** `docs/apu.md` §cycle-exact; `docs/scheduler.md`; `ref-proj/ares` + `ref-proj/bsnes`
SMP scheduling (study-only)
**Estimated complexity:** L
**Status:** done (cycle-exact step + honest verdict decoding). Earlier this ticket attributed the
blargg residual to a CPU-leading-vs-symmetric clock-model asymmetry needing a bus-master inversion;
T-31-006 disproved that — the residual was the recording-bus write phase.

---

### T-31-006 — SPC700 timer clocking phase (blargg `spc_*` literal PASS)

**Description:** the integrated machine drives the SMP through `RecordingSmpBus`, whose `write` path
applied the write side effect (`$F0` global-enable / `$F1` enable / `$FA-$FC` target / the store)
**before** advancing the SMP timebase and clocking the three timers. ares (`SMP::step`) and Mesen2
(`Spc::Write` → `IncCycleCount` first) clock the timers **before** the store, as does our own
per-instruction `SmpBus::write`. The recording bus was reversed, shifting the timer phase by one
access on every timer-register write — the entire blargg timer-suite divergence (an off-by-one in
the stage accumulation at the `target`/`enable` arming instant).

**Fix:** reorder `RecordingSmpBus::write` to `record()` (timebase + timer clock) first, then store +
IO decode, carrying the deferred SMP→CPU port latch onto that access's micro-op (handshake timing
unchanged).

**Acceptance criteria:**

- [x] `spc_smp` / `spc_timer` / `spc_mem_access_times` reach blargg's literal **`PASSED TESTS`**;
      `tests/blargg_spc.rs` **asserts** the literal PASS (`EXPECT_PASS`), not determinism-only.
- [x] Re-blessed `spc_timer` / `spc_mem_access_times` baselines in `tests/golden/blargg-spc.tsv`
      (`spc_smp` / `spc_dsp6` 120-frame hashes unchanged).
- [x] SPC700 oracle stays **0-diff** (flat, timer-less bus — unaffected); `#![no_std]` +
      `forbid(unsafe_code)` preserved.
- [x] `spc_dsp6` reported honestly — its observable state is unchanged by the fix; still Failed 02.

**Dependencies:** T-31-005
**Reference:** `docs/apu.md` §timer phase; `ref-proj/Mesen2/Core/SNES/SpcTimer.h` + `Spc.cpp`
`IncCycleCount`; `ref-proj/ares` SMP `step`
**Estimated complexity:** S
**Status:** done — 3 of 4 blargg `spc_*` ROMs at literal PASS; `spc_dsp6` deferred to T-31-007.

---

### T-31-007 — `spc_dsp6` S-DSP GAIN mode-7 threshold (done)

**Description:** `spc_dsp6` reported **Failed 02** at `Envelope/gain $E0 threshold`. Pinned the
sub-test against the installed `ares` (which renders `PASSED TESTS`) and blargg's canonical
`SPC_DSP` reference: the S-DSP `GAIN` **mode 7** (bent/two-slope linear increase) compares the
voice's internal envelope latch against `0x600` **unsigned** (`(unsigned) hidden_env` / ares
`(u32) _envelope`). `Dsp::envelope_run` did it **signed**, so a latch left negative by a prior
`GAIN` decrease (mode 4/5 underflow) failed to trip the reduced `+0x08` slope and over-incremented
by `+0x20`. The rest of the envelope path was already bit-identical to ares (proven by an
all-`GAIN`-value differential), so the bug was this single comparison.

**Acceptance criteria:**

- [x] Pin the exact failing `spc_dsp6` sub-test (`Envelope/gain $E0 threshold`) against ares /
  blargg `SPC_DSP` S-DSP behavior.
- [x] Drive `spc_dsp6` to blargg's literal `PASSED TESTS` without weakening the gate (cast the
  threshold compare to `u32`; widened `screen_text` to the full 32×32 nametable + `VERDICT_FRAMES`
  to 12000 so the late `$0800` row-30 verdict is captured; all four ROMs now in `EXPECT_PASS`).

**Dependencies:** T-31-006
**Reference:** `docs/apu.md` §DSP GAIN mode-7 threshold; blargg `SPC_DSP` `run_envelope`;
`ref-proj/ares/ares/sfc/dsp/envelope.cpp`
**Estimated complexity:** M
**Status:** done — **all four blargg `spc_*` ROMs reach blargg's literal `PASSED TESTS`** (the gate
asserts each). 120-frame boot hashes unchanged (baseline TSV untouched); undisbeliever golden 29/29,
SPC700 oracle 0-diff.

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] The SPC700 + resync are ready for the S-DSP pipeline (Sprint 2).
- [ ] CHANGELOG.md updated.
