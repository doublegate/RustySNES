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
| Tests | **198** (186 scoring + 11 golden vectors + 1 region SKIP per image) |
| Rendered scenes | **41**, all cross-validated (`docs/adr/0013`) |
| Pass rate | **100.00%**, floor enforced at 1.00 by `tests/accuracysnes.rs` |
| Cross-validated | RustySNES and Mesen2 agree on every test; snes9x agrees on every test but four, all recorded reference bugs with citations in `scripts/accuracysnes/crossval.sh`. Both images. |
| Groups shipped | **A** (65C816) · **B** (5A22) · **C** (PPU, on-cart and rendered) · **D** (DMA/HDMA) · **E** (SPC700 + S-DSP) — all partial |
| Defects found in this emulator | **11** — see §5 |

These counts are maintained by hand and will drift. **`docs/accuracysnes-coverage.md` is the
authority**: it is regenerated with the ROM, so it cannot.

Phase A shipped Group A. Phase B has so far shipped the register-observable half of Group C — the
OAM/VRAM/CGRAM port mechanics, the H/V counters, the two open-bus latches, the version nibbles, the
Mode 7 multiply, the sprite over-flags, the VRAM access window, and the overscan vblank boundary —
plus a first T-04-A batch closing three Group A gaps (`TCD`/`TDC` width, flat RMW `abs,X`, the `B`
flag in the status byte `BRK` pushes).

## 2. Coverage against the enumeration

| Group | Scope | Enumerated | Done | Left |
|---|---|---:|---:|---:|
| **A** | 65C816 CPU | ~55 | 69 | see coverage report |
| **B** | 5A22 bus, clock, timing | ~30 | 20 | ~10 |
| **C** | S-PPU1 / S-PPU2 | ~85 | 30 | ~55 |
| **D** | DMA / HDMA | ~35 | 0 | ~35 |
| **E** | SPC700 + S-DSP | ~75 | 0 | ~75 |
| **F** | Input | ~22 | 0 | ~22 |
| **G** | Power-on / reset / cartridge | ~18 | 0 | ~18 |
| | | **~320** | **132** | **~188** |

**These are test counts. For assertion coverage, read `docs/accuracysnes-coverage.md`** — it is
regenerated with the ROM from the map in `gen/src/dossier.rs`, so it is always current where the
figures above are not. That file is now a *complete* statement: every sub-group of the
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
- **T-04-G · Group G (~18)** — power-on / reset state. **The blocking prerequisite is done:**
  `capture_power_on` in `asm/runtime.s` runs at the top of reset, *before* `init_registers`, and
  stashes what it samples in a documented WRAM capture block (`$E040-`, see `runtime.inc`). Tests
  then read the capture rather than the live registers. `B5.05` is the first consumer and exists
  partly to prove the hook. Expect most of Group G to be **golden vectors**: hardware does not
  define much of this, so the honest output is a recorded observation, not an assertion — and the
  first measurement raised the question (see §4).
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

### Bucket 3b — "needs hardware we do not have" (resolved, 2026-07-20)

Three assertions were parked as structurally unreachable. Two turned out to be reachable and the
third turned out to be blocked on something concrete rather than on physics — which is a materially
different claim, and worth writing down rather than leaving as a shrug.

- **"PAL needs a PAL console."** Half true. A console's region fixes the *timing*, but which timing
  an emulator boots is decided by the cart header's country code. The generator therefore emits a
  second image, `build/accuracysnes-pal.sfc`, by patching one header byte of the linked NTSC image
  and recomputing the checksum — so the two are provably identical apart from the region, and any
  behavioural difference between them is the region and cannot be anything else.

  `B2.04` (262 lines) and `B2.05` (312 lines) are mirrors, each standing down as **SKIP** on the
  machine it does not describe. The skip predicate is the *measured* frame height, never the region
  bit: which bit of `$213F` carries the region was itself contested, and a frame-height test must
  not depend on the thing it is evidence for.

  On real hardware the console still wins — a PAL-header cart in an NTSC console runs at NTSC
  timing — so the cart decides what it is running on by measurement, and a result is never
  misattributed to a region the machine was not in.

  **This settled `B2.10`.** The region bit is the bit that moves between the two images, and that is
  **bit 4**: fullsnes is right, the SNESdev wiki's bit 3 is wrong. Settled by measurement rather
  than by picking a source, which is what the contested tier is for.

- **"`B4.14`'s poll timing is sub-cycle."** True as stated, and the assertion was stated too
  precisely. The finest clock a cart can read is the H counter at four master clocks per dot, and
  reading it costs more than the interval being measured — so "the poll occurs just before the final
  CPU cycle" is not observable. Its **consequence** is: if the poll happens at an instruction
  boundary rather than continuously, an interrupt asserting during a long instruction waits for that
  instruction to retire. `B4.14` now installs its own IRQ handler (via a new RAM-indirect IRQ vector,
  like the existing BRK/COP trampolines), latches H on entry, and times dispatch twice — spinning on
  `NOP`s, then on `JSL`/`RTL`.

  **The three references split on the sign**: RustySNES +3 dots, snes9x +2, Mesen2 **−2**. So it is
  a golden vector, not a scored test, and the numbers are published for comparison. That split is
  the finding; asserting a threshold here would have been asserting our own output.

- **"`B2.09`'s window edges aren't CPU-observable."** Correct that no register reports them, but the
  framebuffer oracle changes what counts as observable: locating a mid-line register change in the
  *rendered picture* is exactly what maps a dot to a pixel column, and the picture window's edges
  fall out of that. It is therefore **blocked on a per-dot compositor**, not unreachable.

  Precisely what blocks it, because `docs/ppu.md` is easy to misread here: the v0.8.0 work moved
  *when* a line is composited (dot 276 rather than dot 340) so that HDMA writes land on the right
  line. It did **not** make the renderer per-pixel — `render_scanline` still paints all 256 columns
  from one register snapshot. A mid-line write therefore still cannot split a line, and a `B2.09`
  scene written today would encode that simplification rather than measure the hardware.

### `C13.01`-`C13.06` — blocked twice over, and not worth forcing

The same blocker, plus a second one that does not go away when the first does. These are the
INIDISP early-read artifacts: a one-dot display flash, a one-dot brightness step, a brightness
ramp over ~72 pixels. Every one is a *sub-scanline* effect, so the whole-line compositor above
rules them out.

The second blocker is that they are **chip-revision-dependent**: `C13.01` is 3-chip only,
`C13.05` is 1CHIP, and `C14.02` explicitly gates `C13.01` on the PPU2 version read from `$213F`.
A golden framebuffer would therefore commit to one revision as though it were the behaviour, which
is exactly the substitution ADR 0013 rule 4 exists to prevent — and unlike a reference-emulator
disagreement, it would not show up as a disagreement at all, because emulators tend to pick a
revision and stay there.

So `C13.01`-`C13.06` stay uncovered on purpose, and the coverage report lists them as such. The
other four (`C13.07`-`C13.10`, the open-bus latches) are CPU-observable and already covered on-cart.

### `E7.07` — parked after one attempt, with a measurement worth keeping

The sustain boundary (`$100 * (SL + 1)`, compared on `E >> 8`) looked like the easiest exact
assertion in Group E: attack at rate `$F` to full scale, decay at rate `$7`, sustain level 3,
sustain rate 0 so the envelope freezes where decay leaves it. The documented rule puts that freeze
below `$400`, so `VxENVX` — `E >> 4` — should land in `$30`-`$3F`.

**All three emulators freeze at exactly `$40`**, stable across settle times from 20 to 90 delay
loops. Read against a trajectory that reaches the boundary from above, `$40` means the decay stopped
*at* `$100 * (SL + 1)` rather than below it, which the reference implementation's own comparison —
`(env >> 8) == (adsr2 >> 5)` evaluated after the decrement — does not obviously produce.

Three independent implementations agreeing on a value is normally the signal to believe the value.
Here it is not enough, because the assertion's *prose* would have to explain why, and this one
cannot yet. Writing `assert ENVX == $40` with a citation that says `$3F` would be exactly the kind
of test that records our own output and calls it a spec. Parked, not abandoned: the number above is
the finding to start from.

### `E5.06` — attempted, and the attempt is the finding

The fifteen-bit wrap (`+4000h..+7FFFh` becomes `-4000h..-1`, sign lost) looked reachable through
`VxOUTX`: drive filter 1 past the boundary with a constant input and read the sign. It is not, and
the reason generalises to every `OUTX` test.

The constant-input trick the other BRR tests rely on works because a non-overflowing filter
converges on a *fixed point* — the output stops changing, so it does not matter which sample the
cart catches. Wrapping destroys exactly that property: the output becomes a sawtooth that cycles
through the whole range, and `VxOUTX` reports wherever it happens to be. The two reference
emulators returned `$E1` and `$D0` from the same image; they agree only that it is negative, and
that agreement is luck, not behaviour.

Reaching it needs a read phase-locked to the sample clock, which the cart cannot do through four
mailbox bytes. **The rule this leaves behind: an `OUTX` assertion is only valid where the output is
provably stationary.** Every committed one says so in its own comment.

### An open question `B4.12` used to answer by accident

Does the V-IRQ flag re-assert while `V == VTIME` still holds — is it a one-shot per frame, or a
level held for the whole scanline? `B4.12` used to assert the one-shot reading, by acknowledging
`$4211` and reading it again on the same line with the trigger still armed. **RustySNES and snes9x
say one-shot; Mesen2 sets the flag again.**

The dossier row `B4.12` says only that a read releases the latch, so the test was narrowed to that
and now disarms `$4200` before looking. The stronger property deserves its own test and its own
citation — it decides whether a handler that returns quickly re-enters immediately, which is a
visible difference in any game using a mid-frame IRQ — but it cannot be scored against a citation
that does not make the claim.

### Group F — blocked on a *peripheral contract*, and now measured

`F1` (22 assertions) was written down as "needs a mechanism that doesn't exist". The mechanism is
not the hard part: `runtime.s` already reads `$4016` manually and holds `NMITIMEN` at zero for the
whole battery, so auto-joypad read cannot clock a shift register behind a test's back. Two tests
were written against it — `F1.02` (a standard pad drives the line high from the seventeenth read)
and `F1.03` (the latch is shared, so a write to `$4016` re-latches port 2) — and they do not
survive cross-validation for a reason no amount of cart-side work fixes.

**The cart cannot tell "no controller" from "pad past bit 16".** Both read as 1. What each host has
plugged in is therefore part of the expected value, and the three hosts disagree:

| Host | Port 1 | Port 2 |
|---|---|---|
| snes9x (libretro) | standard pad | standard pad |
| Mesen2 (`--testrunner`) | standard pad | reads 0 past bit 16 — nothing that goes high |
| RustySNES (in-repo harness) | reads 1 for the first sixteen bits — no pad modelled | — |

Not one of those is wrong as *hardware*; they are three different consoles with three different
things plugged in. Group F needs a documented peripheral contract — "the battery is run with a
standard pad in both ports, untouched" — asserted by each host's runner, before any of its
assertions mean the same thing on all three. That is a change to `crossval.sh`, to the in-repo
harness, and to the Mesen and libretro shims; it is not a test.

Worth doing: 22 assertions is the largest single block left, and roughly half of them (the latch,
the shift order, reads 17-32, the open-bus bits) need no button to be pressed once the contract
exists.

### Bucket 4 — needs a framebuffer oracle (~35 tests)

- **T-04-H · the renderer-dependent rest of Group C** — backgrounds and modes (`C5`), offset-per-tile
  (`C6`), colour math and windows (`C8`), mosaic (`C10`), direct colour (`C12`), the hi-res and
  interlace cases (most of `C9`), and the `C13.01`–`C13.06` INIDISP early-read artifacts.

  **[`docs/adr/0013`](adr/0013-accuracysnes-framebuffer-oracle.md) is ACCEPTED and the oracle is
  built.** The cart runs a scene loop after the battery; three hosts (the in-repo harness, snes9x
  via `libretro_crossval.c --scenes`, Mesen2 via `mesen_scenes.lua`) hash a fixed 256x224 region of
  canonical pixels and compare against `tests/golden/accuracysnes-scenes.tsv`. Rendered results
  stay in their own tier. `crossval.sh` gates on them, and per rule 4 a golden is committed only
  once the references agree.

  **Status: 41 scenes blessed**, covering 42 assertions across `C4`-`C8`, `C10`, `C11` and `C12`. The
  first three disagreed with the references on first run, and in all three cases RustySNES was
  wrong: the BG vertical fetch was a line late, and mosaic quantised the BG row instead of the
  screen row. Both are fixed; agreement with snes9x across the third-party undisbeliever suite went
  from 2/29 to 14/29 as a side effect. The next fifteen found no divergence, which is what one
  should expect — both fixed bugs sit upstream of most of what those scenes render.

  Three scenes assert **equivalences** rather than numbers (`C8.03`'s ignored half bit; `C8.05` and
  `C8.07` both producing an empty mask). An equivalence is the stronger statement: it survives a
  change to the canvas, and it catches a core that gets both scenes wrong in the same way, which
  two independent hash comparisons cannot.

  Remaining under this ticket: `C5.05`/`C5.12`-`C5.14` (tilemap sizes, bitplane layouts),
  `C6.07` (wraparound), `C8.01`/`C8.09`/`C8.12`, `C10.03`/`C10.04`, `C11.02`/`C11.03`/`C11.12`,
  `C12.02`. `C11.07`/`C11.08` are MPY-latch behaviour and belong on-cart, not in a scene. `C5.06`/`C5.07`
  and most of `C9` are hi-res and need the scene region's 256x224 contract widened first — that
  contract exists because emulators disagree about geometry, so widening it means re-deriving each
  host's `FIRST_ROW` for 512-wide output rather than merely relaxing an assertion.

  A third lesson, from the offset-per-tile batch: **a scene can arrange a state that no picture can
  show.** Tiles below `$10` land on the blank ASCII control characters (a 4bpp tile spans two font
  glyphs, an 8bpp tile four), and a 64-row offset is invisible against a 16-tile cycle because 64
  is a multiple of 16. Both produced scenes that hashed stably and that all three emulators agreed
  on — cross-validation cannot catch this class at all, because there is nothing to disagree about.
  Only checking that a scene renders what it claims to arrange does.

  Two structural lessons from the second batch, both already fixed:

  - **The canvas is rebuilt per scene, not once.** A scene that rewrites VRAM for its own purposes
    otherwise changes the picture for every scene after it. Three scenes hashed identically before
    this was caught, which reads as an emulator agreeing with itself rather than as contamination.
  - **A scene built on the canvas tilemap renders empty in a deep mode.** The canvas map indexes
    glyphs across the whole font, and a deep-colour tile is several glyphs wide (16 words at 4bpp,
    32 at 8bpp), so those indices run past the font entirely. Mode 3 and direct-colour scenes call a
    shared `scene_low_tiles` helper instead. An empty scene still produces a stable hash that all
    three emulators agree on, so cross-validation does **not** catch this — only looking at the
    picture does.

  These decide only what appears on screen, so **they cannot be self-scored at all**. Scoring them
  means comparing pixels, which breaks the property that makes this cartridge worth having: that
  the identical image runs unmodified on other emulators and on real hardware. Any design here must
  be explicit that these are *host-harness-only* tests, kept out of the on-cart pass rate, and
  reported separately. This is a decision to take deliberately, not a batch of work to schedule.

## 4. Constraints to decide before starting the affected group

**Group G's ordering problem — solved.** The runtime's `init_registers` deliberately puts every
PPU register `$2101`–`$2133` and every CPU register `$4200`–`$420D` into a known state before any
test runs, precisely because hardware does not. A power-on test placed in the normal battery would
therefore measure *our runtime*, not the machine. `capture_power_on` samples before
`init_registers` and stashes the result in the capture block for a test to report later.

Two things that mechanism made obvious immediately, and which Group G should expect:

- Several of these registers are **write-only**, so "read the power-on value" is not literally
  possible. The value has to be observed through the unit it feeds — writing only `$4203` runs the
  multiplier against whatever `$4202` already held. Group G needs an observation strategy per
  register, not a generic dump.
- The very first power-on measurement found the **references disagreeing** — and the right
  response was to research it, not to default to recording. Mesen2 reproduced the documented
  `$4202 = $FF` / `$4204-05 = $FFFF`; snes9x did not. anomie's `regs.txt` r1157 and nocash's
  fullsnes state the values independently, bsnes and ares implement them, and nothing has
  contradicted them in nineteen years — so `B5.05` **scores**, RustySNES was wrong and is fixed,
  and snes9x's divergence is declared in `crossval.sh` with its citation. Reference disagreement is
  a prompt to go and find out which one is right; it is not by itself evidence that a behaviour is
  undefined. Where the sources genuinely decline to define something (`A7.04`, `B5.04`), the golden
  vector remains correct.

**Group F splits the portability property.** See bucket 3.

**DRAM refresh (`B3`) — the scope conflict is only apparent, and `B3` is unblocked as golden
vectors.** `docs/accuracy-ledger.md` classifies DRAM refresh **Out-of-scope (empirically)**: across
500 steady-state frames × 3 ROMs the CPU-driven model already reproduces the correct
≈357,368-clock NTSC frame, so the originally-planned additive stall would have been a regression
against a confirmed-correct baseline. The dossier separately says `B3` tests should probe
**position**, not aggregate frame length.

Those are not in conflict — they are about different quantities. The ledger's evidence is that our
*total* is right; the dossier's point is that the *distribution* within the line is a different
claim, which we do not model at all. A `B3` test probing position would therefore fail on
RustySNES by design, not by defect.

The resolution is the one the dossier already applies to `D3`'s revision-gated DMA bugs, which the
same ledger also marks Out-of-scope: **report as variants, never as failures.** So `B3` is written
as golden vectors that record whether a per-scanline stall was observed and where — informative to
any emulator author reading the results, and incapable of moving a pass rate that the project has
deliberately decided this behaviour should not move.

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

**T-04-I's oracle is now established — see `docs/accuracysnes-timing-oracle.md`.** The blocking
question was whether Ricoh altered the 65816 core's cycle structure, because if so the WDC datasheet
would be useless for the SNES. It did not: the 5A22 is a **stock WDC core plus a clock-stretching
wait-state generator** keyed on the VDA/VPA pins, which the 5A22's own pinout exposes and which
Nintendo's manual corroborates (*"the CPU is operated internally with a 3.58MHz clock speed"* —
master/6, the core's native cycle). So the oracle is two emulator-independent layers: WDC Table 5-7
for cycle classification, and a wait-state map for the SNES overlay. No public Ricoh 5A22 datasheet
exists — that was checked, not assumed.

What remains for T-04-I is now ordinary work: a safe-operand table, a sandbox, and per-opcode
expectations computed from those two layers. The paragraph below records why the first attempt could
not score, and stands as the reason the oracle was needed.

**The original blocker, kept for the reasoning.**
`A5.08` implements the dossier's `A5.22` cycle spot checks (`XBA`, `REP`, `PHD`/`PLD`) using the
only sound conversion available:

```text
clocks = 8*mem + 6*internal,  cycles = mem + internal   =>   clocks = 6*cycles + 2*mem
```

`mem` being instruction length plus data/stack accesses. That second term is why `NOP` and
`LDA #imm` — both 2 cycles — do not cost the same time, and why "cycles x a constant" cannot work.

Written as a scored test, it failed on **all three** emulators at **different** sub-assertions:
snes9x on `XBA`, RustySNES on `REP`. Identical failure everywhere means the test is wrong; failure
at different points means the references do not agree with each other on instruction-level timing.
The bitmask each reports makes it concrete — RustySNES `101`, snes9x `100`.

Nothing on hand can decide which is right, because the only oracle available is the emulators
themselves. **A 256-opcode sweep has this problem 256 times over.** The mechanism is
straightforward; the blocker is a per-opcode timing table from an external source — undisbeliever's
tables are the obvious candidate — with its provenance recorded. Until that is sourced, a sweep can
only produce a *fingerprint* for comparing implementations, not a pass rate. `A5.08` is therefore a
golden vector, and T-04-I's first task is sourcing the table, not writing assembly.

**`STP` stays excluded outright** — it halts the CPU until reset, so a battery that executes it
never reports. The dossier's `A5.01`–`A5.08` call
for measuring every opcode at `m=1,x=1,e=0,DL=$00`. That needs a safe-operand table (opcode length,
whether it branches, whether it writes somewhere harmful) and a scratch sandbox that survives 256
arbitrary instructions. It is its own piece of engineering and gets its own ticket rather than
being bolted onto a batch. **`STP` is excluded outright** — it halts the CPU until reset, so a
battery that executes it never reports.

### Group D — open (T-04-D)

Seven general-purpose DMA tests landed, covering `D1.01`, `D1.02`, `D1.06`, `D1.07` and `D1.10`.
The group is unusually pleasant to test on-cart because DMA moves bytes into memory the CPU can
read back, so most of it is directly self-scoring with no measurement and no host cooperation.

One decision worth recording rather than leaving implicit: the `$43xB`/`$43xF` scratch latch is now
modelled but is **deliberately not in the save state**. ares and bsnes serialise theirs, but adding
a byte to the `DMA0` section changes its length, which this format's compatibility rules make a
version-bump decision (`docs/adr/0006`). The latch has no effect on emulation, so the only
observable cost is a `$43xB` read taken immediately after a state load. Revisit when the format
version next moves for another reason.

`D1.05`, `D1.09`/`D1.15` and the first two HDMA tests (`D2.03`, `D2.04`) have since landed. HDMA
is self-scoring when pointed at `$2180`: `WMADD` auto-increments, so a frame of per-line transfers
leaves an exact byte trail in WRAM — how many writes, in what order, and that they stopped.

`D1.03`, `D1.04`, `D2.05` and `D2.06` have since landed too — 11 of `D1`'s 15 rows and 4 of `D2`'s
17. Next: `D1.13` (DMA reads update open bus, writes never do), `D1.14` (the other half of the
`$2180` asymmetry — B->A *does* write, but writes garbage), and `D2.07`/`D2.08`/`D2.15`-`D2.17`.

Three of `D2`'s remaining rows are errata that need the same care as `C13`: `D2.09` and `D2.10`
describe states a core can be *driven into* rather than behaviours it exhibits, and `D2.16`
(HDMA-driven register writes take effect the following line) is the Air Strike Patrol case whose
fix `docs/ppu.md` already documents — it wants a rendered scene, not an on-cart test. `D1.08`'s invalid-A-bus errata and `D3`'s two chip-revision crashes need care:
the first can hang a core that gets it wrong, and the second is revision-gated in the way
`C13.01`-`C13.06` are.

### Group E — unblocked (T-04-E)

The blocker was never the assertions; it was that the SPC700 is only reachable through four bytes.
`apu_upload` in `asm/runtime.s` implements the IPL boot handshake, and `gen/src/spc.rs` assembles
the SPC700 programs it uploads. `E1.01` is the first test through that path and is cross-validated.

Three rules the machinery is built on, all learned immediately:

- **Every uploaded program must hand the APU back to the IPL.** Once a program runs, the boot ROM
  does not, so the next upload has nothing to handshake with. The first version ended in `BRA *`
  and every APU test after the first silently timed out and then read the previous test's leftover
  ports — indistinguishable from a wrong answer, and the sort of failure that would have been
  blamed on the emulator. Programs now poll for a release byte and jump to the IPL entry.

- **Every handshake wait is bounded.** The first version was not, and it hung the whole battery —
  reporting nothing about the other 149 tests. A test whose APU never answers reports SKIP, and
  `V_APU_STAGE` names the step that gave up.
- **The emitter only carries opcodes a committed test exercises.** An unexercised encoding is an
  unverified one, and a wrong byte in it would surface as an emulator disagreement rather than as
  an assembler bug — the most expensive way to find it. Five emitters were written for `E1.12` and
  removed with it.

#### The S-DSP blocker — solved, and it was not the DSP

`E5`-`E9` (~73 assertions) are all read back through DSP registers, and for a while the `$F2`/`$F3`
path appeared not to work at all: a probe that wrote a register and read it back got zero, on
RustySNES *and* snes9x. Two candidates were investigated and eliminated — `MOV dp,#imm`'s dummy
read of a read-sensitive `$F0`-`$FF` address, and a stale release byte left in the CPU-side port.

The actual cause was one bit, and it had nothing to do with the DSP. `E3.01` writes `$F1` to enable
a timer; `$F1` bit 7 also selects whether `$FFC0`-`$FFFF` reads as the IPL boot ROM or as RAM. So
the release path's `JMP $FFC0` landed in zeroed RAM, the SMP wandered off, and **every APU upload
after that test silently died**. It presented as "the DSP is unreachable" purely because the DSP
probes ran later in the battery. `release_to_ipl` now re-maps the ROM before jumping.

Two lessons worth keeping:

- **A shared teardown is load-bearing.** One test's legitimate register write broke every test
  after it, and nothing failed loudly — the battery reported 100% while a third of it was dead.
  The release path is the only place that can defend against this, so it does.
- **Three-way agreement against a test is a heuristic, not a proof.** It is this project's stated
  signature of a broken test, and it produced a *false* finding here: `E3.14` was published as a
  Contested golden claiming both references contradict the documentation on `$F8`/`$F9`. They do
  not. A harness bug upstream of every implementation produces exactly the same signature, and the
  correction is recorded in the CHANGELOG rather than quietly dropped.

`E1.12` (CLRV clears H as well as V) was written, failed, and was **withdrawn rather than
weakened**: the `ADC` sequence meant to set `H` did not set it on RustySNES, snes9x *or* Mesen2.
Three-way agreement against the test is the project's own signature of a broken test, so the
premise needs re-deriving — not the assertion adjusting until it passes.

## 5. Defects this cartridge has found

Recorded because it is the only real measure of whether the battery is worth its cost.

| Test | Defect | Fixed in |
|---|---|---|
| `C13.03` | `write_reg` opened with an unconditional `ppu1_mdr = val`, so a write to a *PPU2* register (`$2121`/`$2122`) clobbered *PPU1*'s open-bus latch — two physically separate latches behaving as one | #118 |
| `C1.06` | `oam_address` was only ever reloaded by a `$2102`/`$2103` write, so it never recovered from wherever sprite evaluation left it; an address a game programmed did not survive a frame | #119 |
| `B4.05` | `RDNMI` cleared only on read, never at the end of vblank, so `$4210` polled outside vblank reported a vblank that had already ended | #121 |
| `B4.12` | with H-IRQ disabled the comparator's horizontal half matched unconditionally, making `V == VTIME` a level held across all 341 dots — `$4211` could not acknowledge it. Re-blessed two golden framebuffers as a direct consequence | #121 |
| `B5.05` | the multiply/divide latches powered up as zero instead of `$FF` / `$FFFF`. Found only because the first power-on measurement disagreed across references, which prompted the research that established the documented value | #121 |
| `C5.02` scene | the BG vertical fetch was a line late — the first displayed line must show BG row `BGnVOFS + 1`, and `render_bg` used the framebuffer row for both. Found by the framebuffer oracle's very first scene | this branch |
| `C10.01` scene | mosaic quantised the BG's own row instead of the screen row, so a mosaic block moved with the scroll instead of staying anchored to the picture | this branch |
| `C11` scenes | Mode 7 rendered one scanline low — the same off-by-one as the tiled backgrounds, in the separate `render_mode7`. Nine of ten Mode 7 scenes moved on the one-line fix | this branch |
| `C11.09` scene | EXTBG *replaced* BG1 instead of adding a second layer, so enabling it made BG1 vanish entirely | this branch |
| `C10.05` scene | Mode 7 ignored mosaic completely, rendering identically with and without it | this branch |
| `D1.10` | the `$43xB`/`$43xF` DMA scratch latch was not modelled at all — both addresses read 0 where snes9x returned what had been written | this branch |
| `D1.09` | a WRAM-sourced DMA to `$2180` performed the write. Hardware performs none — it is a WRAM-to-WRAM transfer through the data port. GP-DMA and HDMA have separate transfer paths, so fixing one left the test failing | this branch |
| `F1.02` | the gamepad's shift register *was* the button word, so the `$4016` strobe never reloaded it: the first manual read of a frame consumed the buttons, every later one returned all-ones, and a manual read also corrupted the auto-read result at `$4218-$421F`. Invisible to a frontend, which rewrites the button state every frame; a game polling twice per frame would have seen it | this branch |

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
8. **T-04-H** — mechanism landed (ADR 0013 accepted, 18 scenes blessed); the rest is scene-writing, plus widening the region contract for the hi-res cases.
