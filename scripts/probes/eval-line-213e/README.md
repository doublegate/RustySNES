# `$213E` over-flag eval-line probe (T-CA-10)

Determines, empirically, **which display line the sprite range/time over-flags belong to**, so the
per-dot compositor's incremental range-evaluation cursor (over-flag dot-timing, `docs/adr/0014`) can
be anchored to match the MesenCE oracle instead of guessed — the kind of timing attribution the repo
warns must never be encoded unvalidated.

## How it works

`probe.s` is a 32 KiB LoROM ROM that:

- sets up **40 8×8 sprites all at Y=100** (X = 0,4,…,156, on-screen); the other 88 are parked
  off-screen (Y=$F0), so exactly those 40 are in range on scanlines 100–107 — well over the 32-sprite
  limit, forcing `range_over`;
- fires an **H-IRQ every scanline** at HTIME=256 (just after that line's sprite evaluation, but with
  a wide margin before the line ends at ~340 — at HTIME=300 the IRQ-service latency pushed the OPVCT
  latch to the V-counter increment, latching V+1 and skewing every reading by one line),
  latches the V-counter (`$2137` SLHV, then **`$213F` STAT78 to reset the OPHCT/OPVCT 2nd-read
  flip-flop** — without that reset the toggle alternates and corrupts odd-scanline samples), reads
  `$213E` (STAT77) and stores it to WRAM `$7E:1000 + V`.

Both emulators then read that array — RustySNES via `bus.peek_wram` (the `probe_213e` harness bin),
MesenCE via `emu.read(0x1000+s, emu.memType.snesWorkRam)` (`probe_mesen.lua`) — and report the first
scanline whose bit 6 (range over) / bit 7 (time over) reads set.

## Running

```bash
scripts/probes/eval-line-213e/run.sh      # builds the ROM (needs cc65), runs both RustySNES paths + MesenCE
```

## Finding (2026-07)

`range_over` first-observable scanline, same ROM, sampled at HTIME=256:

| build | scanline |
|---|---|
| **MesenCE (oracle)** | **100** |
| RustySNES per-dot compositor (with or without the incremental cursor) | **100** (matches) |
| RustySNES batch (shipped default) | 101 (one line late) |

**What the probe resolves — and what it doesn't.** At the probe's line granularity the per-dot
compositor already reads 100 (matching MesenCE); the **batch** model reads 101, one line late. So the
probe's reliable result is *per-dot matches MesenCE, batch is a line behind*. (An earlier run at
HTIME=300 reported per-dot=101 / batch=102, but that was the V-counter latch artifact this probe now
avoids — do not trust those numbers.)

The probe's V-counter sampling **cannot** resolve the finer, internal difference the incremental
over-flag cursor (`Ppu::pd_eval_over_flags`) makes: the pre-cursor per-dot path sets `range_over`
internally at `(scanline 101, dot 1)` — line start of the paint line — while the cursor sets it at
`(scanline 100, dot 66)`, i.e. one line *ahead* (`scan_y = self.v`, the next display line MesenCE
evaluates during the current one) and at the 33rd in-range sprite's dot (2 dots/sprite). That internal
`(100, 66)` matches MesenCE's `EvaluateNextLineSprites` exactly and is asserted directly by the unit
test `incremental_range_over_sets_on_next_line_at_the_33rd_sprite` — the real acceptance test for the
cursor. This probe validates the coarser line-level parity; the unit test validates the dot-level one.

(`time_over` reads unset here on both emulators — and correctly so: only the ≤32 in-range sprites are
tile-fetched, and 32 × one 8×8 tile = 32 tiles ≤ the 34-tile limit. Both `eval_objects_range` and the
incremental cursor stop counting tiles past the 32nd in-range sprite for the same reason, so this is
matching behaviour, not a bug — a wider sprite (`w/8 ≥ 2`) or more tiles per sprite is needed to trip
it.)
