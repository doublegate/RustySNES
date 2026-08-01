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

**The first thing it settled: `A2.10` ("PEI does not page-wrap").** ares **passes** it (catalogue
index 11, status `$01`). With RustySNES and snes9x also passing, that is **3 against 1 with Mesen2
the outlier**, and the row comes off the "unexplained, needs a fourth opinion" list.

## The five rows ares disagrees with the cart about

**Three of the five are rows snes9x already fails**, with rationales in `crossval.sh`. That was not
obvious from the tally and it changes what each row means:

| row | ares code | who else fails it | so the split is |
|---|---:|---|---|
| `C7.05` | 1 | snes9x (code **2** — a different failure) | 2-vs-2, and the two dissenters disagree with each other |
| `C7.10` | 1 | snes9x | **2-vs-2** — RustySNES + Mesen2 against snes9x + ares |
| `F1.10` | 2 | snes9x | **2-vs-2** (but see the caveat below) |
| `E8.02` | 3 | nobody | ares alone |
| `E3.06` | 2 | nobody | ares alone — **but see below: probably a wrap, not a rate** |

So ares is *corroborating snes9x* on three rows rather than standing alone, and only `E8.02` and
`E3.06` are genuinely ares-only. **Nothing here is adjudicated** — this is the shape of the
disagreement, not a verdict on it.

**A correction to this file's first version:** it said `F1.10` was a `PAD2_CONTRACT` row and to
suspect this host's port detection first. That is wrong. `f1_require_contract` reads `$4016` only —
port 1 — and `F1.10` code 2 means "`$4212` read busy at the very start of the vblank line", which
does not involve controller state at all. The claim came from the Mesen2 known-failure grouping in
`crossval.sh`, which attributes *its* `F1.10` failure to the port-2 limitation and is itself now
flagged as doubtful there.

**`E3.06` is probably the row's fault, not ares'.** It compares timer 2's tick count against timer
0's over one interval, and `TnOUT` is four bits. The band is `8..15`, so it ends one tick short of
its own wrap — structurally, because timer 0 must tick at least once and one timer-0 period is eight
timer-2 periods. The row now records both counts: RustySNES reads timer 2 = **10**, ares reads
**0**, and 0 is what 16 reads as. ares' `Timer<128>`/`Timer<128>`/`Timer<16>` declarations are an
exactly correct 8:1 ratio, so the wrap reading is the likelier one. The row cannot distinguish the
two, and now says so.

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
