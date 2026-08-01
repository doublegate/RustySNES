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

New information, and **not yet adjudicated** — the cart, snes9x and Mesen2 all pass these:

| idx | row | code |
|---:|---|---:|
| 116 | `C7.05` | 1 |
| 120 | `C7.10` | 1 |
| 265 | `E8.02` | 3 |
| 276 | `E3.06` | 2 |
| 288 | `F1.10` | 2 |

**Treat `F1.10` as suspect-of-this-host first.** It is a `PAD2_CONTRACT` row, and this host's port
detection (`port->name().find("2")` on the button's grandparent) is *assumed* to work, not verified.
Check that before concluding anything about ares.

The standing heuristic — three implementations agreeing usually means a broken test, one disagreeing
usually means a real bug — cuts an unfamiliar way here: it is **ares** alone, on rows the other three
pass.

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
