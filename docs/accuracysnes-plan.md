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
| Tests | **235** (223 scoring + 11 golden vectors + 1 region SKIP per image) — *tests, not assertions; see the note below the table* |
| Rendered scenes | **50**, all cross-validated (`docs/adr/0013`) |
| Pass rate | **100.00%**, floor enforced at 1.00 by `tests/accuracysnes.rs` |
| Cross-validated | RustySNES and Mesen2 agree on every test; snes9x agrees on every test but five, all recorded reference bugs with citations in `scripts/accuracysnes/crossval.sh`. Both images. |
| Groups shipped | **A** (65C816) · **B** (5A22) · **C** (PPU, on-cart and rendered) · **D** (DMA/HDMA) · **E** (SPC700 + S-DSP) · **F** (controller ports) · **G** (cartridge/memory map) — all seven, all partial |
| Defects found in this emulator | **12** — see §5 |

These counts are maintained by hand and will drift. **`docs/accuracysnes-coverage.md` is the
authority**: it is regenerated with the ROM, so it cannot.

**The test count and the assertion count are different numbers and neither bounds the other.** One
test routinely carries several assertions with distinct failure codes, and several tests routinely
share one enumerated assertion (`E6.02` is four tests for one row). Reading the coverage figure off
the test count, or the reverse, has now been done once by a human and once by a review bot in the
same fortnight, so it is written here as well as in §2.

Phase A shipped Group A. Phase B has so far shipped the register-observable half of Group C — the
OAM/VRAM/CGRAM port mechanics, the H/V counters, the two open-bus latches, the version nibbles, the
Mode 7 multiply, the sprite over-flags, the VRAM access window, and the overscan vblank boundary —
plus a first T-04-A batch closing three Group A gaps (`TCD`/`TDC` width, flat RMW `abs,X`, the `B`
flag in the status byte `BRK` pushes).

## 2. Coverage against the enumeration

| Group | Scope | Enumerated | Done | Left |
|---|---|---:|---:|---:|
| **A** | 65C816 CPU | ~55 | 86 | see coverage report |
| **B** | 5A22 bus, clock, timing | ~30 | 22 | ~8 |
| **C** | S-PPU1 / S-PPU2 | ~85 | 30 | ~55 |
| **D** | DMA / HDMA | ~35 | 15 | ~20 |
| **E** | SPC700 + S-DSP | ~75 | 44 | ~31 |
| **F** | Input | ~22 | 1 | ~21 |
| **G** | Power-on / reset / cartridge | ~18 | 4 | ~14 |
| | | **~320** | **202** | **~118** |

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
- **T-04-G · Group G (10 uncovered)** — power-on / reset state. The mechanism is done and now has
  four consumers: `capture_power_on` in `asm/runtime.s` runs at the top of reset, *before*
  `init_registers`, and stashes what it samples in a documented WRAM capture block (`$E040-`, see
  `runtime.inc`); tests read the capture rather than the live registers. It grew two additions with
  `G1.02`/`G1.04` — the carry `XCE` leaves at the very top of `reset` (the boot-time emulation flag,
  readable for exactly one instruction) and the first reads of the read-to-clear `$4210`/`$4211`.

  What is left divides cleanly. **Genuinely undefined and therefore golden at best**: `G1.03`
  (APUIOn, WMDATA, JOYSER, HDMAEN and the rest — the dossier says report, never assert), `G1.05`
  (most PPU registers start unknown), `G1.07` (the WRAM fill, which bsnes randomises by setting).
  **Needs a second image**: `G1.15`/`G1.16` (HiROM and ExHiROM decode), `G1.17` (SRAM mapping, which
  this cart's header does not declare), `G1.18` (the copier header, which requires a file 512 bytes
  longer), and the non-power-of-two half of `G1.11`. **Needs a soft reset the harness cannot
  currently issue**: `G1.06` (PPU state survives cartridge `/RESET`). `G1.01` is write-only
  registers whose power-on values no instruction can read back, and `G1.13` is a `[CONFLICT]` on a
  FastROM bit this SlowROM image cannot exercise either way.
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

### `C5.12` — solved by giving the canvas a second screen worth looking at

A 64x32 BG places its extra screen to the right of the first, and the obvious scene is to scroll 256
pixels and look at it. That scene renders **the plain canvas**: its hash equalled
`c8-window-left-gt-right-empty`, which is BG1 fully visible.

`scene_canvas`'s tilemap repeats horizontally with a period that divides 256 pixels, so a 256-pixel
scroll is not observable no matter which screen it lands in. The same trap as the Mode 7 mask and
the 64-row offset before it: **a scene can only show a difference the canvas is capable of
expressing.**

That is what `scene_second_screen` now does: it fills `MAP_BASE + $400` with tiles `$20`-`$2F` at a
flat palette 5, varying with the column and nothing else. The canvas draws 64 glyphs from `$21`
upward with a row-derived palette, so the tile numbers overlap and the two are still nothing alike
as pictures — landing in the second screen renders something no other scene renders. The shipped scene scrolls a 64x32 map 256 pixels and must show the marker.

A second wrong version wrote the scroll to `$210F` — BG2HOFS, not BG1HOFS (`$210D`) — and produced
another stable, three-way-agreed hash identical to an existing scene's. Two wrong scenes in a row,
both caught by the same check and neither by anything else.

### `C11.02` — solved, and the two dead ends are the lesson

The origin rule is `ORG.X = (M7HOFS - M7X) AND NOT $1C00`, with `$1C00` put back when the
difference is negative. A scene was written for it and **rendered a picture identical to
`c11-mode7-identity`** — same hash, on all three emulators. That is the useful part.

`$1C00` is `7 * $400`, so the mask only ever clears **multiples of 1024**, and a Mode 7 map is
1024x1024 and wraps. Every value the mask removes is a value the wrap removes anyway, which makes
the rule invisible by construction whenever screen-over is set to wrap. Getting at it means setting
`M7SEL`'s screen-over field to transparent or char-0 so that being *outside* 0-1023 is visible,
and choosing a difference whose masked and unmasked forms fall on opposite sides of that boundary.

That is what the shipped scene does. Getting there also required noticing that `M7HOFS` is thirteen
bits **signed**, so `$1C40` — the obvious "large offset" — is *negative*, puts the origin off the map
to the left with or without the mask, and renders a blank screen either way. `$0C40` is the same low
bits with bit 12 clear.

Both dead ends looked like working tests: three emulators agreeing, a stable hash, a plausible name.
Each was caught only because its hash equalled an existing scene's — the first `c11-mode7-identity`,
the second `c8-window-inverted-empty-is-full`. **Check a new scene's hash against the committed
goldens before blessing it.**

### `C1.07` — solved by `frame_step`, and the primitive is now available

**Solved.** The runtime has a `frame_step` helper: it clears forced blank from inside vblank, waits
for rendering to begin and then for the following vblank, and blanks again before returning. `C1.07`
uses it and passes on all three emulators.

What follows is the finding that led there, kept because it is the reason the primitive exists.

`$2100` bit 7 going 1→0 reloads the OAM address from `$2102`/`$2103`. A test wrote the two values
back to back, moved the internal pointer off the programmed address first, and read `$2138`
straight afterwards. **All three emulators returned the walked-to byte, not the reloaded one.**

Three failing identically is a broken test. The reload is evidently tied to rendering *starting* —
the transition arms it, and it takes effect at the next visible scanline — not to the write
retiring. The battery runs the whole time under forced blank, so nothing in the test ever crossed
that boundary.

Reaching it means letting a frame actually render between the transition and the read, then
re-blanking before touching `$2138` (an OAM read during active display is unreliable — `C1.08` is
the assertion that says so). That is what `frame_step` does, and it is available to any other
assertion that needs rendering to have happened: `C7.09` (the sprite range/time-over flags clear at
the end of vblank but not during forced blank) and `C9.05` (the mid-frame overscan hazard) are the
next two.

### Interlace scenes need frame-parity control, which the scene protocol does not have

`C7.12` (a 16x32 sprite under OBJ interlace renders as 16x16) was written as a scene and produced
**three different hashes on three emulators** — the only three-way split any scene has produced.
RustySNES rendered the non-interlaced picture exactly; snes9x and Mesen2 each rendered something
else, and something else from each other.

That is not three cores disagreeing about interlace. It is the scene asking a question whose answer
alternates every frame: OBJ interlace draws alternate fields, so the picture depends on the *parity
of the captured frame*, and the protocol (hash the fourth sighting of an eight-frame published
window) does not pin parity. Each host lands on whichever field its own frame counter happens to be
on.

Any `C9`/`C7.12` interlace assertion needs the cart to publish a scene only on a known field —
a change to `run_scenes`, not to a scene. Worth doing: it unblocks the interlace half of `C9`
(`C9.03`, `C9.06`) as well as `C7.12`.

### `E8.03` — "clears `ENDX` even when suppressed" does not mean suppressed by `KOFF`

The dossier row reads: *"KON restarts even if playing, zeroing the envelope, and clears ENDX even
when suppressed."* The obvious test is to write `KOFF` and `KON` together — the arrangement `E8.04`
already uses — and assert that `ENDX` comes back clear even though the voice never starts.

**All three emulators leave `ENDX` set.** Three failing identically is the signature of a broken
test rather than three broken cores, so the reading is wrong: whatever "suppressed" means there, it
is not `KOFF`. The likeliest candidate is the key-on *collapse* cases (`E8.05`, `E8.06`), where two
`KON` writes land inside the same 16 kHz polling window and one of them is dropped — a suppression
internal to the DSP's own scheduling rather than one the program asks for.

Reaching that needs the collapse cases first, which are probabilistic on hardware and are their own
piece of work. Parked with the measurement: `ENDX` reads 1 on RustySNES, snes9x and Mesen2 alike
after a `KOFF`+`KON` pair.

### `E6.02` — a rate needs four assertions, and the exact factor needs eight

`ENDX` reports "finished" or "not finished", which bounds a rate on one side only, so a rate
measurement costs two assertions per pitch. `E6.02`-`E6.02d` spend those four and bracket the two
rates to **24-64** and **64-128** samples per wait — windows that contain the documented 48 and 96,
that do not overlap, and that no core ignoring the pitch register can satisfy.

**What they deliberately do not establish is that the factor is two.** A core scaling by 1.5 fits
both windows. Excluding it means bracketing each rate between *adjacent* waits, which is where
bisection actually puts them (`$1000` between the seventh and eighth, `$2000` between the fourth
and fifth) — but shipping that would be four assertions with roughly a tenth of the elapsed time in
hand each, and a tenth is inside the range that has already broken APU tests here twice when an
unrelated edit moved the battery's code. A longer sample buys finer granularity in wait units and
buys it linearly, so a factor-of-ten margin improvement means a factor-of-ten longer sample and a
battery that spends a visible fraction of a second on one row.

The trade is explicit: **a tighter claim is a thinner margin, and this cartridge would rather state
a wider window it can defend on three emulators than a precise one that flips when a test above it
grows a line.** If the exact factor becomes worth having, the way in is a longer sample, not a
narrower wait.

### `C3.05` — attempted twice, parked, and the second attempt is the lesson

`$2137` is supposed to latch the H/V counters only while `$4201` bit 7 is set: `WRIO` bit 7 drives
pin 6 of controller port 2, the counters latch on that line's falling edge, and reading `$2137`
pulls it low only if software left it high. SNESdev, fullsnes and anomie's `regs.txt` say so
independently.

**The first attempt measured the wrong thing.** It raised `$4201`, cleared the latch flag by reading
`$213F`, then dropped `$4201` and read `$2137`. But dropping `$4201` *is itself* the falling edge the
counters latch on, so the flag afterwards could not distinguish the write from the read. Every
reference reported "latched while disabled", and the write-up nearly shipped as *"documented and
implemented by nobody"*.

**Reordering it inverted the result.** With `$4201` lowered first and the flag cleared afterwards —
so nothing touches `$4201` between the clear and the `$2137` read — all three references report the
opposite: not latched while disabled, latched while enabled, exactly as documented.

**And that reading does not survive checking either.** A direct probe of this emulator's own `Bus`
(construct, `write24($4201, $00)`, read `$213F`, read `$2137`, read `$213F`) returns bit 6 **set**,
matching RustySNES's own source comment — *"gated by the CPU's I/O-enable in HW; we latch always"* —
and contradicting what the cart measures through the same code. The cart and a direct probe of the
same emulator disagree about the same register, and until that is explained neither number is worth
shipping.

So nothing is asserted and nothing is recorded. What is known:

* the ordering of the `$4201` write relative to the flag-clearing `$213F` read changes the answer,
  which means any future test has to fix that ordering explicitly and say why;
* RustySNES latches unconditionally at the `Bus` level, whatever the cart reports;
* an on-cart probe of this needs a **measurement slot that is genuinely free**. The first probe used
  slot 112, which another test already owns, so the value read back belonged to that test — the
  hazard `runtime.inc` warns about, encountered live.

The next attempt should start from a scratch build that dumps `$213F` before and after each step
into slots verified unused, rather than from a folded variant that hides where the discrepancy is.

### `A5.20` — MVN's per-byte cost does not measure, and the number is interesting

> **Read the conclusion first.** The `A5.20` **cart test** is withdrawn; no test ships. The
> dossier *assertion* is untouched and still counts in the 443-row denominator — it is simply
> **uncovered**, and annotated there as not cart-measurable. Everything from here to
> *"Implementing that golden vector disproved the paragraph above"* is a **superseded narrative**,
> kept because the wrong turns are the instructive part and the next person should not have to
> rediscover them. It is **not normative**: in particular the intermediate claims that the
> divergence is real, that it is too large to be an instrument artifact, and that RustySNES's
> 52-clock decomposition is thereby supported, are all **disproved further down**. The binding
> statements are the final bullets and `to-dos/ROADMAP.md` **T-06-A**.

`MVN` should cost **7 cycles per byte moved**. It is the one timing row where being wrong is
unbounded rather than fixed: a block move is a loop inside a single opcode, so a per-byte error of
one cycle is one cycle out *per byte*, and a 64 KiB clear diverges by most of a frame.

The natural measurement is a difference between two moves — sixteen bytes against eight — so that
everything not per-byte (opcode fetch, operands, register setup, the measurement's own overhead)
cancels.

Three units are in play and it is worth pinning each. The documented figure is **7 CPU cycles per
byte moved**, which decomposes into **2 memory cycles** (the source read and the destination write)
and **5 internal cycles**. Converting to master clocks: a memory access in an 8-clock region such as
WRAM costs 8, an internal cycle always costs 6, so one byte is `2*8 + 5*6` = **46 master clocks**.
The H counter reads **dots**, at 4 master clocks each, so a byte is 11.5 dots and the eight extra
bytes should cost `8 * 46 / 4` = **92 dots**.

**Measured: 13 dots**, on RustySNES and snes9x alike — 1.6 dots per byte, or **6.5 master clocks per
byte** against the 46 the model predicts. Six master clocks is one internal cycle, so the measured
figure is close to *one CPU cycle per byte* where the documentation says seven.

Two readings, and the work is to tell them apart:

* both cores genuinely under-charge `MVN`, which would be a significant shared timing defect and
  exactly what this row exists to catch; or
* the H-counter harness cannot measure a **single long instruction**. Every other `A5` test measures
  eight short instructions between `measure_begin` and `measure_end`; this is the first to put one
  multi-hundred-clock instruction inside that window, and the latch mechanics it depends on are the
  same `$2137`/`$213F` machinery that `C3.05` above could not pin down either.

**The second is investigated first, and the reason is asymmetry rather than intuition.** The
harness is *one* instrument, this cart's own code, shared by both measurements — so an instrument
error is common-mode by construction and needs only one mistake. The two cores are separate
implementations, so a shared timing defect needs the same mistake made twice. That makes the
instrument the cheaper explanation, but it does not make it the true one: the two cores are not
fully independent either, since both were written against the same published cycle tables, and a
shared source can produce a shared error just as a shared instrument can.

So the order was a triage order, not a verdict — and the triage has since been run. **It is the
instrument.**

The calibration measured two `MVN`s and two `NOP` spins through the ordinary `measure_begin` /
`measure_end` pair:

| what | measured | predicted |
|---|---:|---:|
| `MVN`, 8 bytes | 326 dots | 92 + overhead |
| `MVN`, 32 bytes | 327 dots | 368 + overhead |
| 32 `NOP`s | 287 dots | 96 + overhead |
| 64 `NOP`s | **58 dots** | 192 + overhead |

Twenty-four extra moved bytes cost one dot, and **sixty-four `NOP`s measure less than thirty-two of
them**. That last line is the proof, and it needs no model of the SNES at all: the instrument is not
monotonic in the work it measures, so it is out of range. `hv_read_raw` returns the **9-bit H
counter**, which wraps every scanline, and a measurement that overruns simply comes back small —
nothing in the result says "out of range", which is why the first `MVN` reading looked like a
finding instead of an artifact.

Consequences worth carrying forward:

* **The `A5` tests that exist are unaffected**, and it is worth stating the invariant that makes
  them safe rather than a count. Each measures a short unrolled run — fifteen of them repeat an
  instruction eight times and three repeat one sixteen times — and the property that matters is that
  the *window* stays well below the 341-dot wrap, not the number of instructions in it. Sixteen
  two-cycle instructions is 48 dots; the largest of them is nowhere near the cap. Anything added
  later has to be checked against the wrap, not against these counts: **the safe quantity is the
  measured span, and there is no guard on it.**
* **The wider instrument now exists and is validated.** `hv_begin_wide` / `hv_end_wide` in
  `runtime.s` latch H and V together and count dots from the top of the field, so a span may cross
  line boundaries. Getting there took four separate bugs, and every one of them produced
  plausible-looking numbers rather than an error:

  1. **Two latches instead of one.** H from one `$2137` and V from a second, tens of cycles later,
     so a line boundary between them threw the composite off by a whole line. One `$2137` latches
     both; `$213C`/`$213D` then read them out through independent flipflops (`C3.07`).
  2. **DBR-dependent reads.** `MVN` leaves `DBR` = its destination bank, so `lda $213F` after a
     block move reads WRAM at `$7E:213F`, not the PPU. The counters came back as whatever bytes
     were there and the line-countdown ran thousands of times — an 8-byte move reported 11,464
     dots. The latch is long-addressed throughout now.
  3. **No V window, and no overrun check.** A span starting near the end of a field let V wrap to
     zero, making `V1 - V0` hugely negative and running the line-countdown sixty-five thousand
     times. `hv_begin_wide` now waits for `V < 150`, which keeps the span inside active display
     where every line really is 341 dots — but a start window bounds only the *start*, so
     `hv_end_wide` also refuses any span whose V went backwards or which crossed more than
     `MAX_SPAN_LINES`, returning `$FFFF`. That is the point of the whole exercise applied to the
     instrument itself: **an out-of-range measurement must not come back looking like data.**
  4. **`A` was not preserved.** `MVN` takes its byte count in `A`. Clobbering it made 8-, 32- and
     64-byte moves all measure the *same* instruction — three identical numbers across two
     rebuilds, which read as "the instrument is saturating" rather than "the operand is wrong".

  Validated against a known quantity before being trusted: `NOP` spins come back linear at **3.5
  dots each**, exactly one 8-clock ROM fetch plus one 6-clock internal cycle.
* **`A5.20` is measurable but not yet settled.** Through the fixed instrument, `MVN` costs
  **13.42 and 13.39 dots per byte** across two independent size pairings — agreeing to 0.2%, where
  the narrow instrument had produced 326 and 327 dots for moves differing by 24 bytes. snes9x and
  Mesen2 both put the 120-byte difference at exactly **1610 dots**.

  **RustySNES does not**, and the gap is the reason no assertion is shipped yet. Its `block_move`
  re-fetches the opcode and both operand bytes per byte (the ares model), giving 5 memory accesses
  at 8 clocks plus 2 internal at 6 = 52 clocks = 13.0 dots. The references sit at 13.42, about
  **1.7 clocks a byte higher** — which is not a whole cycle, and a discrepancy smaller than one
  cycle is exactly what this instrument's remaining approximation (not every line is 341 dots)
  could also produce over a five-line span.

  **That experiment has been run, and the divergence is real.** Measuring 8-byte and 32-byte moves —
  a 24-byte difference spanning about one line, where the approximation is worth at most a dot:

  | | 8 bytes | 32 bytes | delta (24 bytes) | per byte |
  |---|---:|---:|---:|---:|
  | RustySNES | 390 | 702 | **312 dots** | 13.00 dots = **52.0 clocks** |
  | snes9x | 400 | 732 | **332 dots** | 13.83 dots = **55.3 clocks** |

  Twenty dots over 24 bytes is an order of magnitude more than the instrument can account for, so
  this is not an artifact. Two things fall out of the numbers besides:

  * **RustySNES measures 52.0 clocks a byte**, and its `block_move` implements exactly that —
    opcode and two operand bytes re-fetched from SlowROM, one WRAM read, one WRAM write (5 × 8)
    plus two internal cycles (2 × 6). The agreement is within the measurement's ±1 dot, which over
    24 bytes is ±0.17 clocks a byte, not "to the clock"; what it supports is that the core is
    implementing one particular decomposition of "7 cycles" rather than drifting.
  * **The cross-core gap scales with the byte count**, which is itself evidence that it is
    per-byte rather than fixed: 10 dots at 8 bytes, 30 at 32. (That is not an independent check on
    the difference method — subtraction removes an additive term by construction — but a constant
    implementation difference would have shown the same gap at both sizes, and it does not.)

  **What is still open is which core is right**, and it is now a documentary question rather than an
  experimental one. snes9x's 55.3 clocks a byte is not a whole number of cycles under any obvious
  decomposition: 54 would be six 8-clock accesses plus one internal, 56 would be seven 8-clock
  accesses. The next step is to establish from the WDC tables and the SNES clock model which memory
  accesses `MVN` actually makes per byte. **That has been checked, and the answer is that the sources
  do not say** — the third of the three possible outcomes, and the one that decides what this row
  can be.

  undisbeliever's table, the corpus's primary timing source, leaves the `Cycles` cell for `$54` and
  `$44` **empty**; the entire timing statement is `7 per byte moved` in the `Extra` column
  (`ref-docs/2026-07-20-undisbeliever-65816-timing.md`, note 2). Nothing in the frozen corpus
  decomposes those seven into memory cycles and internal cycles, and on this machine that
  distinction is 2 master clocks apiece — the difference between every candidate reading:

  | decomposition | clocks/byte | dots/byte |
  |---|---:|---:|
  | 4 memory + 3 internal | 50 | 12.50 |
  | **5 memory + 2 internal** | **52** | **13.00** |
  | 6 memory + 1 internal | 54 | 13.50 |
  | 7 memory + 0 internal | 56 | 14.00 |

  Measured: RustySNES **13.00 dots a byte (52.0 clocks)**, snes9x **13.83 dots (55.3 clocks)**.
  RustySNES lands exactly on an integral decomposition — five accesses (opcode plus two operand
  bytes re-fetched, one source read, one destination write) and two internal cycles.

  snes9x lands on none of them, and the measurement is precise enough to say so — but only if the
  units are kept straight, because the buckets above are 12 dots apart in the 24-byte difference and
  the uncertainty is quoted per byte. The instrument is good to **±2 dots over the whole 24-byte
  difference**, which is **±0.083 dots a byte**, i.e. **±0.33 clocks a byte**. So snes9x's slope is
  confined to **13.75-13.92 dots a byte (54.97-55.63 clocks)**, an interval containing neither 13.50
  dots (54 clocks) nor 14.00 (56).

  So the documentary step does not adjudicate the divergence; it establishes that **the row is
  under-determined by the sources**, which under the provenance rules makes it a golden vector
  rather than a scored assertion.

  **Implementing that golden vector disproved the paragraph above, and most of this section with
  it.** The test was written — wide instrument, 8- and 32-byte moves, differenced, classified into
  a decomposition bucket — and then measured at three code alignments. The measured slope moves
  when code *before* the measurement changes:

  | alignment | RustySNES | snes9x | Mesen2 |
  |---|---:|---:|---:|
  | A — source region left uninitialised | 312 | 332 | 312 |
  | B — source filled first | 312 | 322 | 322 |
  | C — B plus three `NOP`s before the span | 312 | 322 | 322 |

  Three things follow, and they retire the rest of this entry:

  * **The 20-dot gap was not stable.** Alignment A is the reading every conclusion above was built
    on, and it does not reproduce. snes9x moved 10 dots and Mesen2 moved 10 the other way, purely
    from filling a WRAM region and adding three `NOP`s. The claim that "twenty dots is an order of
    magnitude more than the instrument can account for, so this is not an artifact" is **wrong**;
    the instrument accounts for it easily.
  * **The stable reading is 2-vs-1 the other way.** At both reproducible alignments the two
    independent references agree at **322** and RustySNES sits alone at **312**. By this
    repository's own diagnostic rule that is the signature of a RustySNES defect, not a reference
    one — the opposite of what alignment A suggested.
  * **The dot domain cannot answer the question.** RustySNES advances the H counter uniformly at
    4 clocks a dot (`crates/rustysnes-core/src/bus.rs`: *"long-dot remainder folded into the
    1364/1360/1368 line"*), so its dot count is exactly clocks/4 and is alignment-independent by
    construction. The references model the per-dot irregularity, so their dot counts are not a
    fixed multiple of clocks and wobble with where a span starts. A cartridge can only read dots,
    so **this instrument cannot measure a clock-domain quantity in a core that models long dots** —
    and the buckets, which assume 4 clocks a dot, are meaningless for exactly the cores being
    compared. Classified naively, both references' 322 lands in the "6 memory + 1 internal" bucket,
    which is an artifact of the conversion and not a claim anyone should publish.

  `A5.20` is therefore **withdrawn, not shipped** — the same outcome as `C3.05` and for the same
  reason: a test that cannot distinguish "the core is wrong" from "the instrument is wrong" asserts
  nothing. What it leaves behind is worth more than the row would have been:

  * **A candidate RustySNES defect with a mechanism**: dot lengths are uniform where hardware and
    both references make dots 323/327 irregular. That is a PPU/bus timing gap, not a `MVN` gap, and
    it would explain the whole two-week saga — the block-move instruction may never have been the
    subject. Worth a ticket in its own right.
  * **ares decomposes the seven cycles** where the documentation does not.
    `instructionBlockMove8`/`16` are two operand `fetch`es, one `read`, one `write`, two `idle`s
    and the opcode re-fetched each iteration (`PC.w -= 3`) — **5 bus accesses + 2 internal**,
    i.e. 52 clocks, which is what RustySNES implements. So the sources' silence is fillable from
    implementations, and a future clock-domain test would have something to assert against.
  * **A method note**: any future timing row must be measured at two or more code alignments before
    it is believed. One alignment produced a confident, reproducible, and entirely wrong answer
    three times running here.

* The earlier `MVN` "finding" is retired regardless: 13 dots was the difference of two wrapped
  readings.

Nothing is shipped either way. A test that cannot distinguish "the core is wrong" from "the
instrument is wrong" asserts nothing.

### `A4.06` / `A4.08` — written, shipped, and withdrawn on review: the mirror hides the bug

Two tests asserted that `JMP (a,X)` and `JSR (a,X)` form their pointer address **within one bank**:
`jmp ($FFFE,x)` with `X = $1002`, pointer seeded at `$00:1000`, landing site reached only if the
sum wrapped rather than carrying into bank `$01`.

**They cannot fail.** `crates/rustysnes-core/src/bus.rs` maps banks `$00-$3F` (and `$80-$BF`) below
`$2000` to the same 8 KiB of WRAM. `$00:1000` and `$01:1000` are therefore *the same bytes*, so a
core that carried the pointer bank read the identical pointer and landed in the identical place.
Both tests asserted only that indexed-indirect jumps work at all, which `A4.01`/`A4.02` already
cover.

This is the failure mode the review instructions name — a test that does not distinguish the
behaviour it claims from the broken alternative — and it survived local runs, both cross-validation
references and both images, because *every* implementation passes it. Cross-validation cannot catch
a test that is vacuous; only reading it can.

**What a real test needs.** The wrapped and carried addresses must land in memory that actually
differs between banks `$00` and `$01`. Below `$2000` is shared WRAM and `$2000-$7FFF` is I/O and
open bus, mirrored identically across `$00-$3F` — so the only discriminating region is
`$8000-$FFFF`, which is ROM, and different ROM in each bank. That means the pointer has to be
**ROM-resident and placed at a known address at link time**, not written into WRAM at run time. It
is a linker-layout change, which is why the tests are withdrawn rather than patched in place.

`A4.04` and `A4.05` are reopened in `T-04-A` with that note attached.

### `A5.09`/`A5.10` — written, passing, and held back: the measurement channel does not reconcile

The `+1 m` and `+1 x` width penalties are the natural next timing rows, and unlike `A5.20` they are
measurable in principle: both spans stay inside one scanline, so the narrow instrument never reaches
the long dots at `H >= 323` or the line-length approximation behind `T-06-A`.

Both tests were written, both passed on RustySNES, and `A5.09` and `A5.10` both passed on snes9x
too. **They are still not shipped**, because the raw measurement channel says something that cannot
be reconciled with the assertions:

* `A5.10` asserts its difference is `32 +/- 2` and passes, while the slot it records that same
  difference into reads **12**.
* Slots 8 through 28 all read an identical **203**, which looks like a fill pattern rather than a
  set of independent measurements.
* Slots 0-6 read exactly `A5.08`'s documented values (24, 32, 76 for the three differentials), so
  the host's slot mapping is demonstrably correct — the anomaly is not an off-by-N in the reader.

An assertion that passes while its own recorded number disagrees with it is the `A5.20` failure mode
with the polarity reversed: there, a coherent-looking number was wrong; here, a passing test is
accompanied by an incoherent one. Either the `record` calls are not writing where they are read
from, or the assertion is passing for a reason other than the one claimed. Shipping before knowing
which would put a number in the pass rate that nothing has actually verified.

**What to check first when this is picked up**: whether `record` and the stash address `$7E0096`
survive `measure_result`'s `rep #$30` as expected; whether any earlier test writes into
`$7EE210-$7EE23A`; and whether the 203s predate the battery (dump the channel before the first test
runs). The tests themselves are straightforward and can be restored from this PR's history once the
channel is trusted.

### `A8.06` — deferred: the battery has no interrupt infrastructure, by design

`A8.06` ("`MVN` is interruptible mid-block — NMI + `RTI` resumes correctly") is the last unclaimed
Group A row that is not a timing measurement, and it is deferred rather than blocked.

The obstacle is a deliberate property of the runtime: **the battery runs with interrupts off.**
`runtime.s` disables NMI/IRQ at init (`stz NMITIMEN`) and detects VBlank by polling `$4212` bit 7,
precisely so that no test's timing can be perturbed by an interrupt it did not ask for. There is no
NMI vector wiring, no handler, and no save/restore of interrupt state around a test.

Implementing `A8.06` therefore means adding interrupt infrastructure to the runtime — installing a
native NMI handler, enabling NMI for the duration of one test, arranging for it to fire *inside* a
block move, and restoring the disabled state afterwards without leaving a window where a later test
can be interrupted. That is a runtime change with a blast radius across all 243 tests, in service of
a row the dossier already marks **UNVERIFIED** ("undocumented upstream"), which means the payoff is
a golden vector rather than a scored assertion.

The right sequencing is to take it *with* the other interrupt-dependent rows (`A6.11`/`A6.12`
`WAI` behaviour, which need the same machinery) as a single deliberate piece of work, not as a
bolt-on to close one row.

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
| `C7.13` scene | sprite vertical flip was computed against the sprite's HEIGHT. Hardware uses its WIDTH, so each square half of a rectangular sprite flips inside itself and the halves do not swap. Identical for square sprites, which is why it survived — and why it took *correcting* a scene that was quietly using a 32x32 sprite to find it | this branch |
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
