# AccuracySNES — Phase Plan

**Status:** live planning document. `docs/STATUS.md` remains the authoritative record of what is
*done*; this file frames what is *left* and what blocks it. The per-test enumeration it draws on is
`docs/accuracysnes-research-dossier.md` §5; the cartridge itself is `tests/roms/AccuracySNES/`.

AccuracySNES closed ticket **T-04**. The follow-on tickets minted here are **T-04-A** through
**T-04-J**, one per remaining work unit, and are listed in `to-dos/ROADMAP.md`. (T-04-J is done.)

---

## 1. Where the battery stands

| | |
|---|---|
| Tests | **85** (80 scoring + 5 golden vectors) |
| Pass rate | **100.00%**, floor enforced at 1.00 by `tests/accuracysnes.rs` |
| Cross-validated | RustySNES, Mesen2, snes9x — all agree, 0 failures |
| Groups shipped | **A** (65C816 CPU, 46 tests) · **C** partial (PPU, 30 tests) · **B** partial (5A22, 9 tests) |
| Defects found in this emulator | **2** — see §5 |

Phase A shipped Group A. Phase B has so far shipped the register-observable half of Group C — the
OAM/VRAM/CGRAM port mechanics, the H/V counters, the two open-bus latches, the version nibbles, the
Mode 7 multiply, the sprite over-flags, the VRAM access window, and the overscan vblank boundary —
plus a first T-04-A batch closing three Group A gaps (`TCD`/`TDC` width, flat RMW `abs,X`, the `B`
flag in the status byte `BRK` pushes).

## 2. Coverage against the enumeration

| Group | Scope | Enumerated | Done | Left |
|---|---|---:|---:|---:|
| **A** | 65C816 CPU | ~55 | 46 | ~10-15 |
| **B** | 5A22 bus, clock, timing | ~30 | 9 | ~21 |
| **C** | S-PPU1 / S-PPU2 | ~85 | 30 | ~55 |
| **D** | DMA / HDMA | ~35 | 0 | ~35 |
| **E** | SPC700 + S-DSP | ~75 | 0 | ~75 |
| **F** | Input | ~22 | 0 | ~22 |
| **G** | Power-on / reset / cartridge | ~18 | 0 | ~18 |
| | | **~320** | **85** | **~235** |

**These are test counts. For assertion coverage, read `docs/accuracysnes-coverage.md`** — it is
regenerated with the ROM from the map in `gen/src/dossier.rs` and currently reports **79 of 443**
enumerated assertion rows covered. That file is now a *complete* statement: every sub-group of the
dossier's Part V is enumerated, so an assertion with no test is listed there by name.

One test routinely carries several assertions with distinct failure codes, so test counts and
assertion counts do not track each other. And cart IDs are not dossier IDs — cart `A1.04` is
dossier `A1.06`. Never read coverage off an ID number; that mistake has already cost rework (§4).

## 3. What blocks the remaining work

The useful axis is not "which group" but "what does this need that we do not have". Four of the
five buckets below need nothing new.

### Bucket 1 — reachable now, no new machinery (~85 tests)

Everything scoreable from a register read, using primitives already built and proven: the
`hv_begin`/`hv_end` H-counter measurement pair, and the release-forced-blank + double-`wait_vblank`
pattern the C7 sprite tests established.

- **T-04-B · Group B (~21 left of ~30)** — **started.** The first batch shipped access speed
  (`MEMSEL`/FastROM, the 12-clock joypad ports), the `RDNMI` flag mechanics, the CPU revision
  nibble, and the multiply/divide unit including the undefined mul/div overlap as a golden vector.
  Left: the scanline-geometry assertions (`B2`, which need frame-length and short-scanline totals
  rather than within-line deltas), DRAM refresh (`B3` — and note `docs/accuracy-ledger.md` and the
  dossier actively disagree on scope, so those tests must probe *position*, not aggregate frame
  length), and the IRQ-timing half of `B4`, which needs the IRQ armed and acknowledged around a
  measurement.
- **T-04-C · rest of register-observable Group C (~20)** — `C1.07`/`C1.08` (the `$2100` 1→0 reload
  trigger, address destroyed during render), the 9- and 10-bit `VMAIN` remap rotations,
  CGRAM-during-render, counter-flipflop independence, `C7.04`–`C7.09` sprite flag set positions,
  `C9.05` overscan vblank deferral, `C11.07`/`C11.08` MPY latch corruption and MPY-during-render.
- **T-04-G · Group G (~18)** — power-on / reset state. Mostly **golden vectors**: hardware does not
  define much of it, so the honest output is a recorded observation, not an assertion. See §4 for
  the ordering constraint that makes this harder than it looks.
- **T-04-A · rest of Group A (38 uncovered assertions — see the coverage report)** — the `A5` spot checks
  (`BRL` flat 4, `BRK` 8/7, `RTI` 7/6, `MVN`/`MVP` 7 per byte,
  `PHD`/`PLD`/`PEA`/`PEI`/`PER`/`REP`/`SEP`/`XBA`), the `+1 m` / `+1 x` sweeps, the E-gated branch
  page-cross penalty, `A1.09` (`TCS`/`TXS` set no flags while other transfers set N/Z), and the
  `A6` gaps (`RTI` mode matching, `WAI` resume and wake latency). The earlier "~12 left" figure was
  wrong — it came from reading ID numbers, and the measured number is considerably larger.

### Bucket 2 — needs its own mechanism (~30 tests)

- **T-04-D · Group D, DMA/HDMA (~35)** — observable through memory and timing, so no oracle is
  needed, **but the enumeration is thin**. The dossier's DMA/HDMA research sub-agent never returned
  during the original pass, so `D1`/`D2` are sketched rather than pinned to sources. This needs a
  research top-up *first*; writing tests against an under-sourced enumeration is how `Novel`-tier
  assertions get smuggled into the pass rate.
- **T-04-E · Group E, SPC700 + S-DSP (~75)** — needs an on-cart APU test harness: upload SPC700
  code through the IPL boot handshake, run it, read results back through `$2140`–`$2143`. Well
  understood (blargg's SPC suite works exactly this way) but it is a real subsystem, not a batch of
  tests. Audio *output* verification stays out of scope by design — the APU is exercised through
  its register and timing side effects.

### Bucket 3 — cannot be fully self-scoring (~22 tests)

- **T-04-F · Group F, input (~22)** — a cartridge cannot press its own buttons. The auto-read
  register mechanics (`$4016`/`$4017`, `$4218`-`$421F`, the `$4212` bit-0 read race) are reachable
  on-cart, but actual button semantics need the host harness driving input, which means those tests
  do not run on real hardware unaided. Split the group accordingly rather than pretending the whole
  thing is portable.

### Bucket 4 — needs a framebuffer oracle (~35 tests)

- **T-04-H · the renderer-dependent rest of Group C** — backgrounds and modes (`C5`), offset-per-tile
  (`C6`), colour math and windows (`C8`), mosaic (`C10`), direct colour (`C12`), the hi-res and
  interlace cases (most of `C9`), and the `C13.01`–`C13.06` INIDISP early-read artifacts.

  These decide only what appears on screen, so **they cannot be self-scored at all**. Scoring them
  means comparing pixels, which breaks the property that makes this cartridge worth having: that
  the identical image runs unmodified on other emulators and on real hardware. Any design here must
  be explicit that these are *host-harness-only* tests, kept out of the on-cart pass rate, and
  reported separately. This is a decision to take deliberately, not a batch of work to schedule.

## 4. Constraints to decide before starting the affected group

**Group G has an ordering problem.** The runtime's `init_registers` deliberately puts every PPU
register `$2101`–`$2133` and every CPU register `$4200`–`$420D` into a known state before any test
runs — precisely because hardware does not. Power-on tests placed in the normal battery would
therefore measure *our runtime*, not the machine. They have to sample before `init_registers`,
which means special-casing them in the boot path and stashing the observations for the battery to
report later.

**Group F splits the portability property.** See bucket 3.

**Real hardware remains the honest ceiling.** Every result so far is three emulators agreeing, and
two of those are not fully independent — a full diff of ares' and bsnes' `wdc65816` cores shows
only type renames, so that lineage is one opinion. Mesen2 is the genuine second. A flash-cart run
would convert "three emulators agree" into "hardware says so", and nothing else will. Until then
the `Corroborated` tier means exactly what `docs/adr/0003` says it means and no more.

**Cart IDs and dossier IDs are different numbering schemes.** *(Resolved by T-04-J — kept here
because the reasoning still governs how to read coverage.)* This is not a theoretical risk — it
caused real rework. The cart's `A1.04` is the dossier's
`A1.06`, the cart's `A2.05` is the dossier's `A2.06`, and the cart's `A3.05` is the dossier's
`A3.10`, because the cart numbers tests sequentially per sub-group while the dossier numbers
assertions. Reading coverage off the ID numbers therefore reports gaps that do not exist: a batch
of seven "remaining Group A" tests was written against that assumption and **four turned out to
duplicate existing tests** under different IDs, caught only by eye when the regenerated catalog
put the old and new names side by side.

**T-04-J landed and fixed it.** `gen/src/dossier.rs` maps every cart test to the assertion(s) it
implements; the generator refuses to build if a test is unmapped, if an assertion is claimed by two
tests without a declared reason, or if a test maps to nothing without a justification. The mapping
is emitted as a `dossier` column in `SOURCE_CATALOG.tsv` and re-checked by the harness against the
committed artifact. Both failure modes were verified to actually fire.

The same ticket also converted the dossier's 23 **prose** sub-groups into per-ID tables (content
preserved verbatim, only restructured), taking the enumeration from 232 checkable assertions to
**443** across all 43 sub-groups. Before that, coverage could only be reported for whichever
assertions happened to sit in a table, and the rest were guesses — which is precisely where an
untested behaviour could hide indefinitely.

**The 256-opcode cycle sweep is deliberately not in T-04-A.** The dossier's `A5.01`–`A5.08` call
for measuring every opcode at `m=1,x=1,e=0,DL=$00`. That needs a safe-operand table (opcode length,
whether it branches, whether it writes somewhere harmful) and a scratch sandbox that survives 256
arbitrary instructions. It is its own piece of engineering and gets its own ticket rather than
being bolted onto a batch. **`STP` is excluded outright** — it halts the CPU until reset, so a
battery that executes it never reports.

## 5. Defects this cartridge has found

Recorded because it is the only real measure of whether the battery is worth its cost.

| Test | Defect | Fixed in |
|---|---|---|
| `C13.03` | `write_reg` opened with an unconditional `ppu1_mdr = val`, so a write to a *PPU2* register (`$2121`/`$2122`) clobbered *PPU1*'s open-bus latch — two physically separate latches behaving as one | #118 |
| `C1.06` | `oam_address` was only ever reloaded by a `$2102`/`$2103` write, so it never recovered from wherever sprite evaluation left it; an address a game programmed did not survive a frame | #119 |

Both were found the same way: the test failed on RustySNES while **both** references passed it.
The inverse pattern — a test failing identically on all three — has twice meant a broken test
(`C7.02`'s `OBJSEL` size field, `C13.03`'s unseeded OAM word), and is treated as such on sight.

## 6. Suggested order

1. **T-04-A** — finish Group A. Small, closes a group out.
2. **T-04-B** — Group B. Largest reachable block, reuses proven primitives.
3. **T-04-C** — the rest of register-observable Group C.
4. **T-04-D** — DMA/HDMA research top-up, then the tests.
5. **T-04-G** — power-on golden vectors, once the boot-path ordering is settled.
6. **T-04-E** — the APU harness, as its own phase.
7. **T-04-F** — input, after deciding the on-cart/host split.
8. **T-04-H** — only if the framebuffer-oracle decision is taken.
