# A headless ares host for AccuracySNES — the third opinion

**Working.** It runs the battery and reports the results block:

```
$ REF_PROJ=$PWD/ref-proj bash scripts/accuracysnes/ares_host/build.sh
$ /tmp/ares_host tests/roms/AccuracySNES/build/accuracysnes.sfc 900
ACCURACYSNES-BEGIN
magic ACSN
done a5
count 338
passed 299
failed 5
skipped 1
golden 33
status 0 01
...
```

Reproducible: two runs give the identical tally.

## Why it exists

Several findings sat at **2 versus 1** with no way to break the tie, because this project's
provenance rule counts ares and bsnes as **one** reference — so "RustySNES and snes9x against
Mesen2" is only 2-vs-1 if ares is not already on RustySNES's side, and nobody could check.

**The row it was built to settle did not need settling.** `A2.10` was recorded as "Mesen2 fails it,
RustySNES and snes9x pass". Measured: ares passes it, **Mesen2 passes it**, RustySNES passes it, and
**snes9x fails it** — where it is documented as the first entry in `SNES9X_KNOWN_FAILURES`. The
original reading took a catalogue index off one host's output and attributed it to another. Recorded
here because it is this host's stated motivation, and the motivation was wrong.

**What it did earn:** the first bug AccuracySNES has found in a *reference* emulator — see below.

## The five rows ares disagrees with the cart about

**Three of the five are rows snes9x already fails**, with rationales in `crossval.sh`. That was not
obvious from the tally and it changes what each row means:

| row | ares code | who else fails it | so the split is |
|---|---:|---|---|
| `C7.05` | 1 | snes9x (code **2** — a different failure) | 2-vs-2, and the two dissenters disagree with each other |
| `C7.10` | 1 | snes9x | **2-vs-2** — RustySNES + Mesen2 against snes9x + ares |
| `F1.10` | 2 | snes9x | **2-vs-2** (but see the caveat below) |
| `E8.02` | 3 | nobody | ares alone — **same cause as `E3.06`, see below** |
| `E3.06` | 2 | nobody | ares alone — **an ares bug, found by this cart** |

So ares is *corroborating snes9x* on three rows rather than standing alone, and only `E8.02` and
`E3.06` are genuinely ares-only. **Nothing here is adjudicated** — this is the shape of the
disagreement, not a verdict on it.

**A correction to this file's first version:** it said `F1.10` was a `PAD2_CONTRACT` row and to
suspect this host's port detection first. That is wrong. `f1_require_contract` reads `$4016` only —
port 1 — and `F1.10` code 2 means "`$4212` read busy at the very start of the vblank line", which
does not involve controller state at all. The claim came from the Mesen2 known-failure grouping in
`crossval.sh`, which attributes *its* `F1.10` failure to the port-2 limitation and is itself now
flagged as doubtful there.

## The two ares-only rows share one cause, and it is an ares bug

`E3.06` and `E8.02` both read **0** from every timer-2 slot they record. One mechanism explains
both, and it is visible in ares' source. `ares/sfc/smp/io.cpp`, the `$F1` (CONTROL) handler:

```cpp
if(timer0.enable.raise(data.bit(0)))  { timer0.stage2 = 0; timer0.stage3 = 0; }
if(timer1.enable.raise(data.bit(1)))  { timer1.stage2 = 0; timer1.stage3 = 0; }
if(!timer2.enable.raise(data.bit(2))) { timer2.stage2 = 0; timer2.stage3 = 0; }
//  ^ negated, and only here
```

`enable.raise(x)` is true on a 0→1 transition. Timers 0 and 1 reset their counters **on** a raise,
which is the documented behaviour. Timer 2 resets on **anything except** a raise — so every later
`$F1` write clears it, including the write that *stops* the timer.

Both rows do exactly that: enable timer 2 via `$F1`, run an interval, write `$F1` again to stop it,
then read `$FF`. In ares the stopping write has already zeroed `T2OUT`.

**ares is internally inconsistent here**, which is the strongest evidence available that this is a
stray `!` rather than intent: its own timers 0 and 1 do the un-negated version, and RustySNES,
snes9x and Mesen2 all treat the three identically (`rustysnes-apu/src/lib.rs`, the `0x01` arm,
resets `stage2`/`stage3` only when `raised`).

**This is the first bug in a reference emulator that AccuracySNES has found**, and it took the third
opinion to see: with only snes9x and Mesen2, both rows passed everywhere and there was nothing to
investigate.

## The row that is still weak regardless

**`E3.06` also has no headroom of its own**, and that is worth fixing independently of ares. It compares timer 2's tick count against timer
0's over one interval, and `TnOUT` is four bits. The band is `8..15`, so it ends one tick short of
its own wrap — structurally, because timer 0 must tick at least once and one timer-0 period is eight
timer-2 periods. The row now records both counts: RustySNES reads timer 2 = **10**, ares reads
**0**, and 0 is what 16 reads as. ares' `Timer<128>`/`Timer<128>`/`Timer<16>` declarations are an
exactly correct 8:1 ratio, so the wrap reading is the likelier one. The row cannot distinguish the
two, and now says so.

## `F1.10`'s ares verdict is not reproducible, and is excluded from the gate

**Measured 2026-08-01.** At the commit before `E9.09` landed, ares failed `F1.10` on eight runs out
of eight, code `04`. Adding `E9.09` — an APU row with nothing to do with controller ports — made it
read `01` on two runs of five and `04` on the other three, same binary, same image.

The row samples `$4212` right at the vblank edge, so where the cart happens to be decides it, and
anything added ahead of Group F moves that. Mesen2 showed the same thing from the other side: its
`F1.10` verdict flipped to **passing** when `E3.06` was rewritten. Two of three references now
demonstrate that this row's cross-host verdict carries no information.

`crossval.sh` therefore excludes `F1.10` from `ARES_KNOWN_FAILURES` **by name**, resolved through
`SOURCE_CATALOG.tsv` at run time — the catalogue index moves whenever a test is added ahead of it,
which is exactly the circumstance that exposed the problem. The constant is 3, and the count is
reproducible across runs again.

RustySNES passes the row because of a deliberate fix, and that stands. But pinning its sampling
point is the only thing that would make any reference's verdict on it mean anything.

The paragraph below is the reading that this supersedes, kept because it is the claim that was
published and it should be visible that it was withdrawn rather than quietly edited away.

**`F1.10` is the interesting one, and it is now measured.** fullsnes says the automatic read begins
~dot 32.5–95.5 of the first vblank line rather than at the vblank edge. snes9x fails the row
(documented instant-latch), ares fails it, and `mesen_failing_set_probe.lua` confirms **Mesen2 fails
it too** — the failing set read at `R_DONE` is exactly `F1.03` and `F1.10`, identical across four
runs. So **`F1.10` is 1-vs-3 with RustySNES passing alone**, on a row it passes only because of a
deliberate fix.

That is an acceptable place for a first-party accuracy cart to be — being right where the references
are wrong is the point of having one — but it is stated rather than left to be mistaken for
consensus. If the fullsnes citation ever turns out to be misread, this row is where it will show.

## Three setup steps that are not optional, each of which cost a debugging round

All three fail as a **segfault inside `System::load`**, with a backtrace pointing at memory setup
rather than at what is actually missing.

1. **`ares::Memory::FixedAllocator::get()` before anything touches a core.** `Bus::reset()`
   allocates its page tables from that bump allocator. desktop-ui does this on its first line.
2. **`ares::SuperFamicom::option("Pixel Accuracy", "true")` before `load`.** `PPUBase::implementation`
   is null until `setAccurate` picks one of the two PPUs, and `Bus::reset()` calls `ppu.map()` →
   `implementation->map()`. `"true"` selects the **accurate** PPU, the only one worth
   cross-validating against.
3. **`nall/main.hpp` included by this translation unit.** It emits `::main` only when
   `NALL_MAIN_IMPL` is undefined, and nall's own `main.cpp.o` defines that. Omit it and the *link*
   fails with a bare `undefined reference to 'main'` from `crt1.o`, which reads like a missing object
   file rather than a missing shim.

Plus one build fact: **`hiro` is not optional even headless** — `mia/mia.hpp` includes it, and its
generated `resource/resource.hpp` does not exist until `ninja hiro` has run once.

`ptrace` is denied in this sandbox, so gdb cannot attach. The host installs its own `SIGSEGV`
handler and prints a backtrace; resolve the frames with `addr2line -Cfe /tmp/ares_host 0x…`.

## Results-block offsets

From `asm/runtime.inc`, and easy to get wrong by one field: `R_COUNT` is `+$06`, `R_PASSED` is
`+$0A`. Reading the latter as the former reported "count 299" for a 338-test battery, which looks
like a truncated run rather than a misread field.

## Not yet wired into `crossval.sh`

Deliberately. Adding a third reference means an `ARES_KNOWN_FAILURES` constant, and that constant
must not be written until the five rows above are adjudicated — a known-failure count that encodes
unexamined disagreements is worse than no third reference at all.
