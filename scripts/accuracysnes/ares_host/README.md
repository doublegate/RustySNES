# A headless ares host for AccuracySNES — builds and links, does **not** yet run

**Status: incomplete infrastructure, committed deliberately.** `build.sh` produces a linked binary;
running it against the cart dumps core during setup. What is finished is the part that was recorded
as the blocker, and what remains is a bounded debugging job rather than a feasibility question.

## Why this exists

Several findings are stuck at **2 versus 1** with no way to break the tie, because this project's
own provenance rule counts ares and bsnes as **one** reference:

- **`A2.10`** ("PEI does not page-wrap") — Mesen2 fails it, RustySNES and snes9x pass. Recorded as
  *not* settled precisely because a harness bug upstream of every implementation produces the same
  signature, and one did once (the `$F8`/`$F9` retraction).
- **OBJ/screen interlace field parity** — RustySNES draws one field's source rows, snes9x and
  Mesen2 the other. RustySNES's `row + field` and its `$213F` bit 7 are both *ares'*, so this is the
  bsnes/ares lineage against the other two rather than RustySNES alone.

Both name the same missing thing: **ares actually running the cart.** The recorded reason it had not
happened was that ares would have to be built and had no headless mode. The first half of that is
now known to be cheap; only the second half is real.

## What is established

| | |
|---|---|
| ares' SFC core builds standalone | **yes** — `-DARES_CORES=sfc`, ~76 targets, a couple of minutes |
| a headless host compiles against it | **yes** — `ares::Platform` is a small interface whose methods all have no-op defaults |
| it links | **yes** — see `build.sh` for the library set and the two non-obvious traps |
| the results block is reachable | **yes, by construction** — `ares::SuperFamicom::cpu.wram[0xF000 + n]` is the cart's `$7E:F000` |
| it runs the cart | **no** — dumps core during setup |

Two traps `build.sh` records because each cost a round:

- **`hiro` is not optional for a headless host.** `mia/mia.hpp` includes it, and its generated
  `resource/resource.hpp` does not exist until hiro has been built once.
- **`nall/main.hpp` must be included by the host translation unit.** It emits `::main` only when
  `NALL_MAIN_IMPL` is undefined, and nall's own `main.cpp.o` defines that. Omit the include and the
  link fails with a bare `undefined reference to 'main'` out of `crt1.o`, which reads like a missing
  object file rather than a missing shim.

## What is left

The crash is in setup, before any frame runs. The likely candidates, in order:

1. `mia::System::create("Super Famicom")->load()` needs a system pak that `mia` looks for under the
   home location — `setHomeLocation` here points at `~/.local/share/ares/`, which may not exist.
   desktop-ui populates it on first run.
2. The `Cartridge Slot` port is allocated with no argument; desktop-ui passes the medium and checks
   the returned node.
3. Controller ports are allocated by name `"Gamepad"`; the actual node name should be confirmed
   against `ares/sfc/controller/controller.cpp` rather than assumed.

Run it under a debugger and start at (1) — a null pak is the failure that would reach furthest
before dying.

## Usage, once it works

```bash
REF_PROJ=$PWD/ref-proj bash scripts/accuracysnes/ares_host/build.sh
/tmp/ares_host tests/roms/AccuracySNES/build/accuracysnes.sfc 900
```

Output is the same `magic` / `done` / `count` / `status N XX` shape the snes9x libretro host emits,
so `crossval.sh` can consume it as a third reference with a `ARES_KNOWN_FAILURES` constant beside
the existing two.
