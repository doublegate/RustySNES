# RustySNES `v1.29.0` "Triangulate"

**Released:** 2026-08-02 · **Tag commit:** `368cc9f` · **Previous release:** [`v1.28.0` "Plumbline"](https://github.com/doublegate/RustySNES/releases/tag/v1.28.0)

> The largest accuracy release the project has cut. It adds a **third independent reference
> emulator** — and with three opinions available, findings that had sat at two-versus-one for weeks
> resolved, two real engine defects surfaced, several published claims were retracted on
> measurement, and **AccuracySNES found a bug in a reference emulator for the first time**.
>
> Triangulation: three references, and the position they fix.

---

## Executive summary

| | |
|---|---|
| Commits | 34 (PRs #298–#331) |
| Diff | 49 files changed, +11,727 / −1,244 |
| AccuracySNES coverage | **347 → 361 of 443** |
| — on-cart | 294 → **304** |
| — rendered scene | 53 → **55** |
| — host tier (new) | — → **2** |
| Battery | 100% on-cart throughout |
| References cross-validating | **2 → 3** (snes9x, Mesen2, **ares**) |
| Save-state format version | unchanged |

The three tiers are **never summed into one figure**, and this release adds the third. An on-cart
verdict means the same thing on any emulator and on real hardware; a rendered scene needs a host
holding the golden; a **host-side** cover is this project testing its own code — the one thing
AccuracySNES exists to stop being the only evidence. The host tier is admitted **only** where the
cart physically cannot observe the assertion, with the reason stated per row.

### Two engine defects, both found by the cart

1. **The SMP wait states were parsed, saved, restored — and read by nothing.** The eighth instance
   of this project's dead-config defect class, and the first found by the cart rather than by grep.
2. **The APU ran 0.92% slow on PAL** because its clock divisor was pinned to the NTSC master rate.

### One reference defect

**ares' `$F1` handler negates the timer-2 counter reset**, so every later `$F1` write zeroes
`T2OUT` — including the write that stops the timer. See below; it took a third opinion to see at
all.

---

## The third reference: a headless ares host

Several findings sat at **two-versus-one with no tiebreaker**, because this project's provenance
rule counts ares and bsnes as *one* reference — so "RustySNES and snes9x against Mesen2" is only
2-vs-1 if ares is not already on RustySNES's side, and nobody could check.
`scripts/accuracysnes/ares_host/` now can:

```text
magic ACSN / done a5 / count 338 / passed 299 / failed 5 / skipped 1 / golden 33
```

Reproducible across runs. It is wired into `crossval.sh` as
`cross-validation: 3 reference(s) agree with the cart`, **opt-in and skipping cleanly** — the host is
a C++ link against ares' static libraries and takes minutes, so `crossval.sh` looks for a pre-built
binary at `$ARES_HOST` and prints a build hint when it is absent. Verified in both states.

**Three setup steps turned out to be mandatory, and each cost a round**, because all three fail as
the *same* segfault inside `System::load` with a backtrace pointing at memory setup rather than at
what is missing:

1. `ares::Memory::FixedAllocator::get()` before anything touches a core (`Bus::reset` allocates
   from it);
2. `ares::SuperFamicom::option("Pixel Accuracy", "true")` before `load` (`PPUBase::implementation`
   is null until `setAccurate` picks a PPU, and `Bus::reset` calls `ppu.map()`);
3. `nall/main.hpp` included by the host translation unit, without which the *link* fails with a bare
   `undefined reference to 'main'` from `crt1.o`.

**One gate bug caught on the way, worth recording for its shape.** Counting the failing rows with
`("0x" $3) % 2 == 0` reported **337 failures**. POSIX awk does not parse `"0x01"`, so the expression
is `0 % 2` for every byte and every non-skipped row counted as a failure. *A gate reporting
catastrophe out of a parsing bug is a specific kind of dangerous*, so the fix matches on the last hex
digit and says why in a comment.

### The ares bug

`E3.06` and `E8.02` were the two divergences the third opinion could not attribute to a known snes9x
failure, and both read **0** from every timer-2 slot they record. One mechanism explains both, and
it is visible in `ares/sfc/smp/io.cpp`'s `$F1` (CONTROL) handler:

```cpp
if(timer0.enable.raise(data.bit(0)))  { timer0.stage2 = 0; timer0.stage3 = 0; }
if(timer1.enable.raise(data.bit(1)))  { timer1.stage2 = 0; timer1.stage3 = 0; }
if(!timer2.enable.raise(data.bit(2))) { timer2.stage2 = 0; timer2.stage3 = 0; }
//  ^ negated, and only here
```

`raise()` is true on a 0→1 transition, so timers 0 and 1 reset their counters **on** a raise — the
documented behaviour. Timer 2 resets on **anything except** a raise, so every later `$F1` write
clears it, *including the write that stops the timer*. Both rows enable timer 2, run an interval,
write `$F1` again to stop it, then read `$FF` — and in ares that stopping write has already zeroed
`T2OUT`.

**ares is internally inconsistent**, which is the strongest available evidence that this is a stray
`!` rather than intent: its own timers 0 and 1 do the un-negated version, and RustySNES, snes9x and
Mesen2 all treat the three identically.

**It took the third opinion to see.** With only snes9x and Mesen2, both rows passed everywhere and
there was nothing to investigate — which is the case for having built the ares host, made concrete.

### Retracted in the same window

The ares host was built to settle `A2.10` — and **`A2.10` never needed settling.** ares passes it;
so does Mesen2 (measured with `mesen_failing_set_probe.lua`, failing set `F1.03` + `F1.10`, identical
on six runs). The host that fails `A2.10` is **snes9x**, where it is documented as the *first* entry
in `SNES9X_KNOWN_FAILURES`. The original reading took an index off one host's output and attributed
it to another. The host still earned itself by finding the ares bug.

`F1.10` was published as "1-vs-3 with RustySNES passing alone" and **that is retracted too**, on a
sharper mechanism: rewriting `E3.06` — an *unrelated APU row* — made Mesen2 **pass** `F1.10`. Three
identical runs before, three after. The rewrite changed one uploaded program's length, which moved
the cart's execution phase, which moved when `F1.10` samples `$4212` relative to the vblank edge.
Mesen2's verdict on that row encodes **where the cart happens to be**, not only what Mesen2 models.

> **This is the same trap three times in one release** — `E8.01`'s two rejected drafts, the scene
> field gate, and now `F1.10`. A verdict that encodes an uncontrolled phase looks stable until
> something unrelated moves it.

---

## Two engine defects

### The SMP wait states were implemented nowhere

`$F0` bits 4-5 and 6-7 select a clock divider for the SMP, nominally `{2, 4, 8, 16}` — but 8 and 16
are glitchy on real silicon, and **the CPU consumes 10 and 20 clocks per opcode cycle while the
timers still advance by 8 and 16.** ares and bsnes carry the same comment (`sfc/smp/timing.cpp`):
*"the timers are not affected by this and advance by their expected values."* Two tables, and the gap
between them is the row.

RustySNES parsed both selectors into `Io::external_wait` / `Io::internal_wait`, saved them, restored
them — and **nothing downstream ever read either one.**

`rustysnes-apu` now carries `SMP_CYCLE_WAIT = [2,4,10,20]` for the CPU (and hence for the recorded
micro-op plan and the S-DSP catch-up, which are real base clocks) against
`SMP_TIMER_WAIT = [2,4,8,16]` for the timers alone, with ares' `SMP::wait` address classification:
idle cycles, `$00F0-$00FF` and a mapped IPL ROM take the *internal* selector, everything else the
external one.

**At the reset selector both tables read `SMP_WAIT`**, so every program that leaves `$F0` alone —
which is every commercial driver — is byte-identical to before.

`E3.09` reads the gap as a ratio the program can see. Over a fixed 48-pass poll loop, timer 0 ticks
**4×** as often at selector 2 as at selector 0. **Both wrong models were injected and both fail:** no
wait states at all reads **1×**, and charging the CPU's glitchy 10 to the timers as well reads **5×**.
So the row separates the two ways of *having* the feature, not merely its absence.

### The APU ran 0.92% slow on PAL

`Bus::advance_master` converts master ticks to SMP base clocks through a fixed rational, and the
denominator was `715_909` — **the NTSC master clock — in both regions.** The APU runs from its own
24.576 MHz crystal, so its rate is the same on NTSC and PAL while the master clock's is not; holding
the whole *ratio* fixed therefore made the APU scale with the video clock. On PAL that is
`21_281_370 × 68352/715909 = 2_031_850` base Hz against hardware's `2_050_560`.

**Both references disagree with the old behaviour, from opposite directions.** ares region-sets
`cpuFrequency` and never region-sets `apuFrequency` at all; snes9x carries two explicit ratios,
`15664/328125` and `34176/709379`, which both work out to an APU rate of exactly **1,025,280 Hz** and
differ *only* in the master-clock denominator — `709_379 × 30` is `21_281_370`, which is where the
new constant comes from.

The divisor is now chosen from the PPU's region **at the point of use** rather than cached in
`Clock`: the accumulator is already serialized, and a second field agreeing with it is one more
thing that can disagree across a region change or a state restore. A stale doc comment had asserted
the opposite (`sync_region_from_cart`: "nothing else in the core depends on which oscillator
frequency a real console would use") and is corrected.

**NTSC output is byte-identical.** `the_apu_rate_is_region_independent` asserts the two divisors
differ by exactly the ratio of the two master clocks, observed through emitted DSP samples so it
needs no new counter.

---

## The dot model: the H-IRQ comparator moves into the clock domain (`T-06-A`)

`HIRQ_TRIGGER_DELAY = 4` was a *dot-domain rounding* of ares' `hcounter(10) == (HTIME+1)<<2` — exact
only while every dot is four clocks, which stopped being true in `v1.28.0` when dots 323 and 327
became six. The match is now computed where it actually happens: clock `4·HTIME + 14`
(`hirq_match_clock`), mapped to the first dot boundary at or after it (`hirq_trigger_dot`).

**Below the long dots the two agree exactly**, because `4·HTIME + 14` is never a multiple of 4 and
the next boundary is `HTIME + 4`. They diverge only for `HTIME` **321..=337**, where the six-clock
dots have displaced every later boundary — the old constant fired up to a whole dot late, and at
`HTIME = 336` suppressed an IRQ that does fire. `HTIME = 337` lands on dot 340's boundary, which
exists only on the long line, so it is honoured there and suppressed elsewhere.

This is the change the plan recorded as *"attempted and reverted because it moves
`hdmaen_latch_test_2`'s golden"*. **It does not, this time:** no framebuffer golden moved, the
undisbeliever suite passes unchanged, and cross-validation is byte-identical.

**`B4.16` is a weaker guard than its own doc claimed.** Measured either side of the change, *both* of
its readings are unchanged — including the `HTIME = 330` one, whose trigger dot moved 334 → 333. The
CPU takes an IRQ at an instruction boundary, so the handler-entry dot quantises to the spin loop's
instruction length and a one-dot shift is absorbed. `B4.16` can say "nothing regressed"; it cannot
say "the change took effect". A unit test does that, sweeping every `HTIME`.

`LONG_DOTS` and the per-dot clock count also move to `rustysnes-ppu`, which owns the dot model;
`rustysnes-core`'s scheduler delegates rather than keeping a second copy.

---

## Two full-instruction-set sweeps

### `E2.10` — all 256 SPC700 opcodes, timed on-cart

The cart measures how long every opcode the SPC700 can execute in a straight line actually takes, and
compares it on-cart against the cycle count **fullsnes** documents. The host supplies no expected
values; it reads back three bytes — opcodes measured, opcodes disagreeing, and the first one that
did.

**How one opcode is timed.** `T2OUT` steps once every 16 opcode cycles, far too coarse for a single
instruction — so the sweep never times one. It times a block of six copies, sixteen times over,
against the same block built from `NOP`. Everything around the copies cancels in the difference,
leaving **six ticks per cycle** against a quantisation of ±1 on each side.

**Branches are measured taken.** A relative branch with a displacement of *zero* lands on the
following instruction, which is where a not-taken branch would have gone anyway — so a block of six
runs straight through whichever way each goes, and the arm can arrange the taken path.

**The table is built from rules, and the build fails if the rules do not tile the map.** fullsnes
documents the opcode map *as* rules, and `spc_opcodes.rs` follows them; a slot filled twice or left
empty is a build failure, not a shipped hole. The operand kind is **recorded at construction** rather
than derived from the opcode byte afterwards — the first draft derived it from the low nibble, which
is nearly right and hides at least four traps.

**Twenty-five opcodes are excluded, by name and with reasons** — the absolute jumps and calls, the
vectored calls (`TCALL`/`PCALL`/`BRK`, whose vectors are in the IPL ROM every Group E program keeps
mapped), the returns, and `SLEEP`/`STOP`, for which fullsnes itself gives the cycle count as `?`.
**The count of what was measured is asserted on-cart at 231**, so a sweep covering a different set
fails rather than quietly reporting no disagreements.

**Verified by injection, twice.** One idle cycle added to `XCN` → exactly one disagreement, first-bad
`$9F`. One idle cycle removed from the taken relative branch → exactly nine, first-bad `$10` — the
eight conditional branches plus `BRA`, which is also the proof that the taken-path prologues really
do take the branch in all eight conditions.

Two things had to be measured rather than reasoned about: the results page was first placed at
`$0900` and the uploaded image reached `$0948`, so the sweep overwrote its own driver as it recorded
(a build-time assertion now rejects that layout); and the shared bounded APU wait is a fraction of a
frame, right for every other Group E program and far too short for this one, which runs about
**42 frames**.

### `A6.15` — all 256 65C816 opcodes defined, only `STP` hangs

The row executes each of the **241 straight-line opcodes** in a WRAM sandbox and counts three
outcomes against the length **Table 5-4 of the WDC W65C816S datasheet** documents: returned where it
should, returned late, or did not return.

**The sandbox terminator cannot be a return.** `TXS` and `TCS` move the stack pointer — with `x = 1`
a `TXS` puts it in page zero — so the return address is no longer where an `RTS` would pop it from.
Control comes back through a `JMP`, and because a `JMP` is three bytes, the addresses are chosen so
its **own operand bytes are harmless one-byte instructions**: `$AAAA` (`TAX`) for the clean exit and
`$B8B8` (`CLV`) for the overshoot one. An opcode that consumes one byte too many therefore executes a
register transfer and walks into a `NOP` fill, instead of executing half an address.

**The watchdog takes two strikes.** `runtime.s` already carried an NMI trampoline with a settable
vector; one strike would be wrong, because NMI fires once per vblank and across 241 sandbox runs it
will eventually land inside a *healthy* one.

**Four opcodes are dangerous even when correct**, and the preamble handles each rather than excluding
it: `MVN`/`MVP` move `A + 1` bytes (so `A = 0`); `XCE` flips to emulation mode only if `C` is set (so
`CLC` first); `TXS`/`TCS` are why the exits restore `SP` from WRAM; `SED` and `PLP` are why they
re-establish `m`, `x`, `d` and `c` rather than trusting what came back.

**Verified by injection, twice.** `WDM` made three-byte produced exactly one LATE with first-bad
`$42`; `TRB dp` made to jam produced exactly one NO-RETURN with first-bad `$14`, the watchdog
rescuing the battery. Picking that second injection is not free — `SED` and `CLC` both hang the cart
*before* `A6.15` runs, because the runtime and earlier Group A rows execute them. `TRB` is executed
nowhere else on the cart.

#### The "ares `PLA` divergence" was our bug

The `A6.15` NMI handler ran with whatever data bank the sandbox left — `$7E` — so `lda $4210` read a
WRAM byte and the NMI was never acknowledged. Three hosts happened never to land an NMI where it
showed; **ares did**, and the row reported `PLA` (`$68`) overshooting on ares alone, stably, with
every other opcode agreeing.

**That reads exactly like a reference bug, and it was ours.** The chain that settled it is the
reusable part — three *eliminating* experiments beat many source-reading guesses:

1. A diagnostic replacing the terminator with `INX` fill showed **both** ares and RustySNES resuming
   at the correct offset → `PLA`'s length was never in question.
2. Sweeping only `$68` **passed** on ares → the failure needed the full sweep's elapsed time.
3. Disarming the watchdog made **all four hosts agree** → the watchdog was the subject.

Long addressing (`lda f:$004210`) is DBR-independent and fixes it. The handler now also preserves
`A` — `RTI` restores `P` and `PC` but not the accumulator, and an interrupt that silently rewrites
`A` is not transparent to what it interrupted.

---

## Scenes: a per-scene extraction rule, and what it found

Hi-res scenes were parked behind "widen the capture region". Measuring showed that framing was
**wrong** (`docs/adr/0013`, 2026-08-02 supplement): the hosts do not agree on the *shape* of a hi-res
frame — snes9x emits `512x224`, Mesen2 `512x478` because it line-doubles — so what is needed is not
a bigger hash but **a rule for reducing whatever a host emits to the canonical 256×224 sample**.

`Scene` gains an `extract` field (`Direct` | `HiResEven`), emitted as `build/scenes.tsv`'s fourth
column so the rule travels with the scene rather than being compiled into any host. All three hosts
honour it, and **a host meeting a rule it does not implement rejects the scene** rather than falling
back to `Direct` — falling back would silently hash the left half of a hi-res picture.

**A related latent bug, found first:** both scene hosts' width tests were *lower* bounds
(`w < SCENE_W`; `#buf < …`), which catch a frame that is too narrow and miss one that is too **wide**
— and too wide is the case that actually happens. The Lua script would walk a 512-wide buffer with a
stride of 256, hashing a diagonal slice; the C host would hash the leftmost 256 columns. **A golden
blessed from either would be stable, reproducible and wrong.** Both are exact now.

### The Mode-5 divergence, published and then retracted

The first hi-res scene diverged immediately, with the two references agreeing bit-for-bit against
RustySNES — this project's signature for a real defect. **That was published from two references
without consulting the third, which is exactly the mistake this project's own notes warn about.**

Diffing the **pixels rather than the hashes**: the divergence is **one column** — column 0 of the
extracted sample, the first pixel of the 512-wide picture — on all 224 rows and nothing else. 224
differing pixels of 57,344.

ares' `sfc/ppu/dac.cpp` settles it *against* the original conclusion: `scanline()` seeds
`math.above.colorEnable = false` under the comment *"the first hires pixel of each scanline is
transparent // note: exact value initializations are not confirmed on hardware"*, and `below()`
returns black in that state. RustySNES does the same thing. **The split is RustySNES + ares against
snes9x + Mesen2, on a value ares itself flags as unverified.**

So there is **no defect to fix**. `C5.15` became coverage the other way: excluding the one undefined
pixel made the rest of Mode 5 blessable, and the extraction infrastructure is vindicated either way —
two hosts with different geometries produced identical hashes for the other 57,120 pixels.

### The scene field gate: necessary, not sufficient

`run_scenes` now sets the scene ID only on frames whose `$213F` bit 7 is set, so every sighting the
host counts is the same **cart-side** field. `SCENE_FRAMES` grew 8 → 12 accordingly.

An accompanying claim that this collapsed `C7.12`'s three-way emulator split into a two-way one was
**retracted**: running the *identical ROM* twice under Mesen2 gives two different hashes — exactly
the two field-parity outcomes, alternating run to run. RustySNES and snes9x are each stable; Mesen2
is not. **The missing half is host-side**: what is not pinned is which *rendered frame* a host
associates with the `R_SCENE` value it read. The interlace scene was **withdrawn**, not left
unblessed — a scene reporting a different hash each run is noise in the gate output.

**`C11.03` replaced it one-for-one** and is the better row: a Mode 7 scene with `M7A` and `M7B` both
`$0101`, deliberately not round numbers, so the discarded part is `line MOD 64` — a different amount
on every line. Every other Mode 7 scene uses round matrix values, which hide the mask completely.
Blessed at a hash **all three** references produce.

---

## Other rows that landed

| Row | Subject | Note |
|---|---|---|
| `B2.07`/`B2.08` | the frame rate, measured against the APU's crystal | only became a real measurement once the APU stopped scaling with the video clock — an APU that speeds up exactly when frames get shorter cannot measure frame length |
| `A5.18` | `BRK` is 8 cycles native, 7 emulation | nothing returns, which is what isolates `BRK` from `RTI`'s own mode-dependent cost |
| `E3.13` | a write to `$00F0-$00FF` lands in the RAM shadow too | needs a second reader that skips the register decode — the S-DSP is one |
| `E9.09` | the echo write pointer wraps at 16 bits, over page zero | `ESA = $F9` ends the buffer *exactly* at `$FFFF`, so the wrapped part is page zero and precisely page zero |
| `E8.01` | `KON` is examined at 16 kHz | measures writes the DSP **never sees**, after two drafts that measured their own timing |
| `E9.02` | noise output is bipolar | scores the *transition*, `$81` → `$3F` |
| `E5.06` | BRR wraps at 15 bits | the first assertion was **vacuous** — injecting the named bug made it pass *harder* (64 negatives of 64, against the correct decoder's 32), because the gaussian interpolator, not the decoder, was supplying the sign |
| `E3.06` | timer 2 ticks ~8× timer 0 | rewritten twice: poll-and-accumulate to escape `TnOUT`'s four-bit ceiling, then **poll both timers**, because a single end-of-interval read measures a *phase* — it broke on the battery's **second** run and not its first |
| `G1.06`, `G1.18` | soft reset leaves the PPU alone; the copier prefix | the new host tier |
| `C11.12` | Mode 7 scroll-offset latch timing | **unauthorable, verified two ways** — the subject appears twice in `ref-docs/` and neither is a behavioural statement, and no reference models a distinct timing. Enumerated and scored as uncoverable, not chased |

---

## Compatibility and upgrade notes

- **Save states:** format version unchanged.
- **PAL audio timing changed.** The APU no longer scales with the video clock; PAL SMP base rate
  moves 2,031,850 → 2,050,560 Hz (+0.92%). **NTSC output is byte-identical.** PAL recordings or
  goldens produced before this release will differ in audio timing.
- **SMP wait states are now honoured.** At the reset selector both tables read `SMP_WAIT`, so every
  driver that leaves `$F0` alone is byte-identical to before. A driver that writes `$F0` will now see
  the modelled clock divider.
- **H-IRQ trigger positions change for `HTIME` 321..=337 only.** Below the long dots the new
  computation agrees with the old constant exactly. No framebuffer golden moved.
- **`ARES_KNOWN_FAILURES = 4`, `MESEN2_KNOWN_FAILURES = 1`, `SNES9X_KNOWN_FAILURES = 14`**, each with
  per-row rationale.
- **`c7-obj-interlace-halves-height` withdrawn** as a scene; `c11-mode7-16-mask` added.
- **`ref-proj/ares` is optional.** `crossval.sh` skips the ares block cleanly when `$ARES_HOST` is
  absent.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --features test-roms
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features
cargo run -p accuracysnes-gen
REF_PROJ=$PWD/ref-proj bash scripts/accuracysnes/crossval.sh
bash scripts/accuracysnes/ares_host/build.sh    # optional third reference
```

- Battery **100% on-cart** at every commit in the window.
- **Three references cross-validating**, each divergence adjudicated by row rather than counted.
- Every new row verified by injecting the bug **at the site the row names** and confirming *which*
  failure code fires — and in the cases where the injection did **not** move the verdict
  (`E5.06`, `E6.08`), the attribution was treated as wrong even though the row passed.
- Every golden blessed **only** from a render the references agree on (ADR 0013), never from this
  project's own output.

## Included changes

| PR | Commit | Summary |
|---|---|---|
| #298 | `2e5c2ec` | three Group E rows — E8.01, E9.02, E5.06 (T-04-E) |
| #299 | `e7fcca2` | publish scenes on a known field (T-04-H) |
| #300 | `6c09eb6` | derive the H-IRQ dot from the clock, not a constant (T-06-A) |
| #301 | `ba1ea3a` | record why E8.06 is not reachable from the cart |
| #302 | `390848e` | cover C11.03, and retract the interlace-gate claim |
| #303 | `5482eb3` | a headless ares host — builds and links, does not yet run |
| #304 | `6d43f87` | the ares host runs the battery |
| #305 | `3c677c2` | adjudicate ares' divergences; two corrections to my own claims |
| #306 | `1480f23` | measure Mesen2's failing set |
| #307 | `122ef84` | E3.06 records the counts it compares |
| #308 | `121f666` | AccuracySNES found an ares bug — `$F1`'s timer-2 reset is inverted |
| #309 | `d75a8ed` | ares as a third reference — and retract the A2.10 finding |
| #310 | `08361ad` | E3.06 polls and accumulates; retract F1.10's "1-vs-3" |
| #311 | `396d22f` | sync coverage figures to the generated report |
| #312 | `fe3ea97` | E9.09 echo wraps into page zero; fix the ares gate |
| #313 | `48d2842` | implement the SMP wait states; E3.09 measures them |
| #314 | `8f19133` | survey what is left in Group E, per row |
| #315 | `43e5968` | A5.18 — BRK is 8 cycles native, 7 in emulation |
| #316 | `f195314` | the APU clock is region-independent; it ran 0.92% slow on PAL |
| #317 | `ddee9b0` | B2.07/B2.08 — the frame rate, measured against the APU |
| #318 | `f574551` | the copier-strip rule is % 32768, not % 1024 |
| #319 | `f3ad15e` | a third coverage tier for on-cart-impossible assertions |
| #320 | `2d40e09` | both scene hosts accepted an out-of-contract geometry |
| #321 | `bd75d9c` | ADR-0013: hi-res needs an extraction rule, not a wider region |
| #322 | `fd8ad04` | per-scene capture extraction, and the Mode-5 divergence it found |
| #323 | `7f56cc3` | retract the Mode-5 defect claim — it is a 2-vs-2 |
| #324 | `e186ab6` | bless C5.15 — exclude the one undefined hi-res pixel |
| #325 | `195092c` | C10.04 is blocked by the same exclusion that unblocked C5.15 |
| #326 | `24fc120` | interlace needs a rule `extract` cannot express |
| #327 | `97dba58` | identify the inidisp gap's exact model difference |
| #328 | `b167fe9` | C11.12 is unauthorable — verified, not assumed |
| #329 | `37f6f5c` | E3.06 polls both timers, not just timer 2 |
| #330 | `c6406de` | the full 256-opcode SPC700 cycle sweep (E2.10) |
| #331 | `368cc9f` | A6.15 — all 256 opcodes defined, only STP hangs |

Full per-entry detail: [`CHANGELOG.md` → `[1.29.0]`](../../CHANGELOG.md).
