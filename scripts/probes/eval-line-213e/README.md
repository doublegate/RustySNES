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

## Baseline finding (2026-07)

`range_over` first-observable scanline, same ROM:

| build | scanline |
|---|---|
| **MesenCE (oracle)** | **100** |
| RustySNES per-dot, before the incremental cursor | 101 (one line late) |
| RustySNES per-dot, **with** the incremental cursor | **100** (matches MesenCE) |
| RustySNES batch (shipped) | 101 (one line late) |

**Conclusion:** MesenCE evaluates scanline *L*'s over-condition during scanline *L*; RustySNES's
`eval_objects_range` evaluates `scan_y = self.v-1` during `self.v`, one line late. The incremental
over-flag cursor must evaluate `scan_y = self.v` (the *next* display line's sprites, one line ahead of
the paint's `scan_y = self.v-1`) and set `range_over` at the dot the 33rd in-range sprite is found. That cursor
is `Ppu::pd_eval_over_flags`; with it the per-dot build reads **100** here (re-run to confirm).

(Also surfaced: `time_over` never sets for many 8×8 sprites — `eval_objects_range`'s range loop
`break`s at the 33rd sprite before `tile_count += w/8`, capping the count at 32; fix when the
incremental tile-fetch/`time_over` phase lands.)
