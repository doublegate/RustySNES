# RustySNES `v1.28.0` "Plumbline"

**Released:** 2026-07-31 · **Tag commit:** `b2057b2` · **Previous release:** [`v1.27.0` "Tether"](https://github.com/doublegate/RustySNES/releases/tag/v1.27.0)

> The release that got the second reference emulator working. The AccuracySNES battery had **never**
> completed under Mesen2 — and the cause turned out to be one line in the harness, not anything in
> the cart or the emulator. With both references arbitrating, three findings that had been stuck at
> "no oracle can decide this" resolved in the same window.
>
> A plumb line is a reference you check against. This release fixed ours.

---

## Executive summary

| | |
|---|---|
| Commits | 15 (PRs #282–#297) |
| Diff | 43 files changed, +4,647 / −438 |
| AccuracySNES coverage | **344 → 347 of 443** (291 → 294 on-cart, 53 scene) |
| Battery under Mesen2 | **never completed → 335/335 status bytes, 1 failing test** |
| Scene goldens under Mesen2 | **0 → 53/53 matching** |
| Save-state format version | unchanged |

Five threads:

1. **The Mesen2 oracle is fixed** — and the fix retracted several conclusions that had been built on
   top of the broken instrument.
2. **The dot model gains the short and long scanlines** (`B2.02`, `B2.03`), which is where
   60.0988 Hz actually comes from.
3. **Three AccuracySNES rows land** — `A5.19` (`RTI` timing), `C7.05` (Range Over position),
   `C7.06` (Time Over position) — the last two of which required new runtime machinery that no row
   in this rung was supposed to need.
4. **Cross-check calibration** — per-row signatures replace a position-blind colour-set comparison,
   which immediately exposed a 23-row disagreement that had been invisible.
5. **Mobile CI arrives early.** Android and iOS workflow work slated for `v1.30.0` landed in this
   window; the CHANGELOG entries carry their `(v1.30.0)` labels from when they were written. The
   engineering is here; the store gate is not, and stays NOT GREENLIT.

---

## Headline — the Mesen2 oracle: one `emu.setInput` call too many

The battery had never run under MesenCE. It now completes: `magic='ACSN'`, `R_DONE=$A5`,
**335/335** status bytes written, and `crossval.sh` reports `Mesen2: 1 failing test(s)` where it
previously timed out.

### Root cause

In this MesenCE build's `--testRunner`, `emu.setInput`'s **port argument does not select a
controller** — indices 0, 1 and 2 all land on **controller 1** (verified through the cart's own
`V_PAD_HELD`, not through a register read). `mesen_crossval.lua` called it twice, so the second
call, intended for port 2, **overwrote port 1**: the cart saw `PAD2_CONTRACT` (`$60A0`) on
controller 1. `$60A0` contains no Start — and the pre-battery menu waits for Start. So the cart
booted, ran init, cleared `R_STATUS`, reached the menu, and sat there forever behind a static
picture that showed nothing.

**The scene half was the same class of bug.** `mesen_scenes.lua` held **no input contract at all** —
no `setInput` anywhere — so the cart never left its pre-battery menu there either, never ran the
battery, and never reached the scene loop that follows it. The symptom read as a scene-loop problem
and was an input problem. With one port-1 call added:

```text
snes9x: 53 scene(s) match, 0 unblessed, 0 mismatched
Mesen2: 53 scene(s) match, 0 unblessed, 0 mismatched
```

**Both halves of the oracle now arbitrate.** `docs/adr/0013` requires a golden be blessed only from
a render the references agree on, and for the first time both references produce one — which is the
gate the `v1.29.0` scene work had been waiting behind.

### What this retracted

Every recorded symptom followed from that one line, and several earlier conclusions were wrong
because of it:

- **The "completes 14 of 335 and stops at `A3.03`" figure was an all-zero status array misread.**
  No test ever ran. Two cart-side `A3.03` fixes were aimed at a layer that was never at fault.
- **"MesenCE headless is broken" was too broad.** It renders the entire undisbeliever corpus with
  **0 skipped captures**. Rendering worked; only the battery read did not — so the scene work was
  never blocked by whatever blocked the battery.
- **A "24 non-zero bytes" lead was an over-read** — those bytes sit at indices 336–359, past the end
  of a 335-entry array. Retracted.

The investigation is worth recording as method, because the path to the answer ran through four
refuted hypotheses, each killed by a measurement rather than by argument: a press-edge problem
(refuted — contract applied at frame 0, 30 and 120 all identical), a frame-budget problem (refuted —
4000 frames fails as 900 did), auto-joypad (refuted — `NMITIMEN` reads `$42`, bit 0 clear, and the
cart reads `$4016` manually anyway), and the Lua-to-emulator memory bridge (refuted — the bridge is
how the misread zero array was counted in the first place). Four probes are committed so none of it
is re-run.

**Stated, not hidden:** port 2 cannot be driven from Lua in this build, so rows depending on
`PAD2_CONTRACT` are **not** cross-validated by this runner. The in-repo harness and the snes9x
libretro driver both drive both ports, so those rows remain covered elsewhere.

What remains under Mesen2 is no longer an oracle failure but a **finding**: 1 failing test,
catalogue index 11 (`A2.10`).

---

## The dot model: short and long scanlines

Two changes with **opposite shapes**, which is why they are separate commits rather than one.

### `B2.02` — the short line

NTSC, progressive, field set, `V = 240` is **1360** master clocks rather than 1364. Under this
project's measured convention that means the two 6-clock dots (323, 327) are **not** long on that
line, leaving 340 dots of 4 — exactly the decomposition the references give.

The observable consequence is the one `B2.07` needs: the NTSC frame now **alternates 357,368 /
357,364** master clocks instead of being constant, which is where 60.0988 Hz comes from. Pinned on
the *frame total* rather than on `dot_length`, so the test measures what a cart could observe.

`Ppu::is_short_scanline` owns the predicate because all four inputs (region, interlace, field, V)
are the PPU's own; the Bus only turns it into clocks. "Every other frame" keys on the field flag,
following anomie's *"scanline `$f0` of every other frame (those with `$213f.7=1`)"*.

**One golden moved.** `hdmaen_latch_test`'s framebuffer hash changed
(`0xd518b7c9df2c9725` → `0x8f60351e0cdd8125`). An earlier draft of the entry claimed no golden
moved, on the strength of `cargo test --workspace` — **that command does not run the golden suite**,
which needs `--features test-roms` and otherwise self-skips entirely. CI caught it. Once the Mesen2
oracle was fixed the change could be arbitrated properly, and it is clean: with `B2.02` applied,
cross-validation is byte-identical to `main` — snes9x OK (14 known divergences), Mesen2 1 failing
test, and **53/53 scene goldens matching on both references**.

### `B2.03` — the long line

PAL, interlace on, field set, `V = 311` is **1368** master clocks and **341** dots. Its shape is the
opposite of the short line's: `B2.02` *substitutes* dot lengths, while this **appends a whole extra
4-clock dot** and leaves the two 6-clock dots alone (`339 × 4 + 2 × 6 = 1368`). It therefore moves
the H wrap (`Ppu::dots_this_line`) rather than the Bus's clock table, and the H counter reaches dot
340 on that line and on no other.

The H-IRQ comparator's upper bound now follows the same per-line count instead of the constant, so
an `HTIME` landing on dot 340 is a genuine match on the long line and stays suppressed elsewhere —
previously it could never match anywhere.

PAL interlaced frames alternate **425,568 / 425,572** clocks. The test enables interlace through
`SETINI $2133` bit 0 rather than poking the field, so it drives the path a game would, and asserts
the *set* of frame lengths rather than their order — which phase carries the extra 4 clocks depends
on the field's power-on value, and pinning that would pin an initial condition the row does not care
about. Both fail under their own injection.

**Still unmodelled, and stated as such:** the interlaced frame's extra *scanline* (263/313 rather
than 262/312). `B2.03` is reachable without it since `V = 311` is the last PAL line either way.

### The prerequisite: the field flag's doc contradicted the code

`Ppu::field` is `$213F` bit 7 and toggles at the end of **every** frame; its doc comment said
"toggles each frame when interlace is on", which describes neither the code nor the hardware. Only
the flag's *use* is interlace-conditional (picking the odd/even row).

That stale reading mattered beyond tidiness: it makes the short/long-scanline gate that keys on the
field look **unreachable in progressive mode** when it is not. Corrected in code, in `docs/ppu.md`,
and pinned by a test that ticks four progressive frames and requires bit 7 to alternate — injecting
the gate the old doc described (`if self.io.interlace`) makes it fail with a constant `[0,0,0,0]`.

**One measurement note worth keeping:** a frame-length probe must read `clock.master`, not sum what
it passes to `advance_master`. The DRAM-refresh reallocation advances the clock 40 per line beyond
the caller's amount, and counting the caller's side reports 346,888 for a 357,368-clock frame.

---

## Three AccuracySNES rows

### `A5.19` — `RTI` is 7 cycles native, 6 emulation

The extra native cycle is the PBR pull, an 8-clock WRAM read rather than a 6-clock internal cycle,
so eight iterations differ by 16 dots. `A5` on-cart coverage 12 → 13 of 15.

The RTIs are chained through **one** `rti` instruction: every return frame is built before
`measure_begin`, all returning to the `@spin` label that *is* the `rti`, except the first-pushed one
— pulled last — which returns to `@done`. The measured span therefore holds nothing but the RTIs,
with no loop counter or branch inside it.

**Three traps on the way**, all now recorded:

1. The 341-dot line wrap returned native **42** against emulation **76** — the native reading
   *smaller*, the arithmetic opposite of the assertion. Diagnosed by measuring at two repeat counts
   and reading the **slope**.
2. `rep` cannot clear `m` while `E = 1`, so `measure_result`'s 16-bit arithmetic silently ran 8-bit
   and pinned the reading at a constant regardless of repeat count.
3. The one that forced the design: `hv_begin` / `hv_end` run at the **caller's** register width, so
   the instrument's own overhead **does not cancel across a mode boundary** — every other `A5`
   differential compares two spans in the same mode. The `xce` pairs now sit *inside* the span, so
   both arms run the instrument natively and differ only in the mode the RTIs execute in.

Verified by injection at the site the row names: making native `RTI` advance `S` without paying the
bus cycle collapses the difference to exactly 0 and fails the row.

### `C7.05` — Range Over trips at the 33rd in-range sprite, `H = OAM.INDEX * 2`

**A fixed bracket would have been vacuous.** An H-IRQ is serviced ~22–27 dots after its `HTIME`, so
one tight enough to pin dot 65 sits inside the latency's own uncertainty, and one loose enough to be
safe passes for any set dot. Instead **both phases sample the same dot** and change only which OAM
entry is 33rd in range — index 32 (set dot 65, reads set) against index 72 (set dot 145, reads
clear) — so the flag has to *move with the index*. Phase B also re-reads in the same frame's vblank,
since "clear" is otherwise what a core that never sets the flag reports.

**The sample dot is measured, not inferred:** an H-IRQ handler runs **~93 dots** after its `HTIME`,
not the ~22–27 a raw trigger latency suggests, because the interrupt is only taken at an instruction
boundary and the trampoline, shim and prologue all precede the read.

**This needed new runtime machinery**, which no row in this rung was supposed to. `irq_trampoline`
is a bank-local `jmp (V_IRQ_VEC)`, so only bank-`$00` groups could install an IRQ handler; Groups
C–G, relocated out of bank `$00`, could not, and none ever had. New **`irq_far_shim`** plus the
24-bit `V_IRQ_VEC_FAR` at `$0058` fixes it for every relocated group — `jml` pushes nothing, so a
far handler still ends in a plain `rti`. Default behaviour is unchanged: the shim is opt-in and the
far vector defaults to `irq_stub`.

Three injections, each failing its own code. snes9x fails the row (`SNES9X_KNOWN_FAILURES` 12 → 13):
its Range Over is scanline-granular rather than per-sprite.

### `C7.06` — Time Over reads set by `V = OBJ.YLOC + 1, H = 0`

`C7` on-cart coverage 7 → 8 of 16.

**8×8 sprites can never reach Time Over.** The budget is 34 tiles per line but range evaluation stops
at the 33rd in-range sprite, so 8×8 caps at 32 — under the limit, permanently. Twenty 16×16 sprites
give 40 tiles while staying under the 32-sprite range limit, which also keeps the two flags
independently observable: the row asserts Time Over sets while Range Over stays **clear**, so a core
raising them together fails.

The bracket runs across the line boundary rather than within a line, since the asserted position is
fixed and there is no index to sweep — clear on the eval line `V = 100`, set on `V = 101`. Phase B
samples on line 101 deliberately: RustySNES raises the flag at `HBLANK_START_DOT` of the eval line,
*earlier* than the assertion requires, and pinning that dot would fail a core raising it at
`(101, 0)` exactly, which the assertion permits.

A **low-tile control** (the same 20 sprites at 8×8, 20 tiles) must read clear at phase B's sampling
point, or "set" would only have meant "sprites are present". Four injections, each failing its own
code. snes9x fails it (`SNES9X_KNOWN_FAILURES` 13 → 14): it reads Time Over set on `V = 100`,
flagging the overflow a line early because it evaluates and paints in one pass.

---

## Cross-check calibration, and the 23-row finding it exposed

`scripts/perdot_crossval.sh` compares **distinct-colour sets**, which are position-blind — so a
change that moves a band without introducing a colour reports `MATCH`. **That is the entire content
of a raster test.**

Both capture sides (`perdot_dump` and `perdot_capture.lua`) now optionally emit one token per row
(`PERDOT_ROWS=1`): the row's colour if uniform, `----` if mixed. Sweeping for the offset that
minimises row mismatches recovers **+7** on the real corpus, which **confirms the documented ~7-row
overscan offset by measurement** rather than by assumption.

What that immediately showed: for `hdmaen_latch_test` at offset +7, **23 of 232 rows mismatch
against MesenCE on `main` today**, before any change in this release. The existing framebuffer
golden therefore encodes a render the reference does **not** agree with — it is a **regression lock,
not an accuracy oracle**. The 23-row disagreement is a genuine pre-existing accuracy gap, invisible
to the colour-set gate that had been the only check on it, and it is filed as its own item rather
than folded into the `B2.02` change.

---

## Mobile engineering (the `v1.30.0` rung, landed early)

### Android CI, and the 16 KB page-alignment gate

`android/` had Gradle sources and a `cargo ndk` layout but **no workflow at all** — nothing built
it, so nothing could regress visibly. `.github/workflows/android.yml` now cross-builds for all four
ABIs, asserts every 64-bit `.so`'s `PT_LOAD` segments are 16 KB aligned, assembles a debug APK, and
checks the APK actually carries each ABI. The gate **fails closed** if the `.so` glob matches
nothing.

**It was written as a formality and immediately found four real defects:**

1. **4 KB-aligned `arm64-v8a` and `x86_64` libraries** — exactly what Play rejects. The gate's own
   comment asserted 16 KB was the NDK default from r27 onward, so the obvious reading was that the
   check was broken. It was not. Reproduced and bisected: with the **same** NDK r27c, `cargo-ndk`
   3.5.4 emits `0x1000` and 4.1.2 emits `0x4000`. **The alignment is decided by the linker
   invocation, not the NDK version** — and the workflow pinned `cargo-ndk` to `^3`. Fixed by passing
   `-C link-arg=-Wl,-z,max-page-size=16384` explicitly, so the result no longer depends on a tool
   default a version range cannot promise.
2. **The Android app could not be built from a clean checkout at all.**
   `android/gradle.properties` had never been committed, so `:app:checkDebugAarMetadata` fails with
   "contains AndroidX dependencies, but the `android.useAndroidX` property is not enabled" — every
   Android build this project had ever done ran on a machine that already had those settings in a
   user-level `~/.gradle/gradle.properties`. Now committed. The **Gradle wrapper is committed too**
   (pinning Gradle 8.10), so CI and a developer's machine build with the same Gradle.
3. **The gate was checking the wrong artifacts** (caught in review). The APK carries **three**
   libraries per ABI, all built by Gradle's own `cargoNdkBuild`, while the workflow's standalone
   pre-build produced only one and Gradle's task did not inherit the alignment flag. So the gate
   could pass on libraries that never ship while the ones that do ship were 4 KB aligned. Fixed on
   both sides, plus a second gate over the **assembled APK's** `lib/`.
4. **The alignment test itself was wrong, and so was a shipped dependency.** It compared each
   `PT_LOAD` Align to `0x4000` for *equality*, but the requirement is "aligned to **at least** 16 KB"
   — a divisibility property, and 64 KB satisfies it. Fixing that exposed a genuine one underneath,
   and finding it required measuring **per ABI**:

   | JNA version | arm64-v8a | x86_64 | armeabi-v7a | x86 |
   |---|---|---|---|---|
   | 5.15.0 | `0x10000` | `0x1000` | `0x1000` | `0x1000` |
   | 5.16.0+ | `0x4000` | `0x4000` | `0x4000` | `0x4000` |

   The pinned 5.15.0 satisfied the requirement on arm64 and **violated it on every other ABI**.
   Bumped to 5.17.0. Every library in the APK is now alignment-checked, not just this project's:
   Play's requirement is a property of the package, so a misaligned dependency fails the listing just
   as surely.

Deliberately **two actions only**, both already pinned in this repo — the NDK comes from the runner
image's own `sdkmanager` — rather than adding three third-party actions to the supply-chain surface
`v1.26.0` tightened.

### iOS CI now launches the app, not just links it

`ios.yml` boots a simulator, installs the built `.app`, launches it, and requires the process to
**still be alive eight seconds later** — asked of the simulator via `launchctl list`, because
`simctl launch` has already returned 0 by the time a launch-crash happens, so its exit code cannot
catch one.

Every iOS verification before this was compile/link-only, and a binary that links and then traps on
launch is indistinguishable from a working one in a build log. **What this does not prove is
emulation:** the app bundles no cartridge, so no ROM has run. `docs/mobile-readiness.md` states that
distinction rather than listing the whole runtime question as open.

The liveness check needed a fix before it was trustworthy: piping `launchctl list` straight into
`grep -q` made it report "it started and died" for an app that was running — `grep -q` exits at its
first match and closes the pipe, `launchctl` takes `SIGPIPE`, and `set -o pipefail` turns that into
the pipeline's status. The listing is now captured before it is searched.

### Touch-to-peripheral coordinate mapping

`rustysnes-mobile::touch`. The FFI already exposed `set_superscope` and `set_mouse`, but both take
units a touchscreen does not have. `map_touch_to_screen` maps a touch through the letterboxed
viewport, taking the **active** framebuffer size as a parameter so aim stays correct when a game
switches to a hi-res mode mid-scene, and reports a touch in the letterbox bars as
`on_screen: false` rather than snapping it to an edge (several games read the Scope's off-screen
state as "reload"). `TouchMouse` carries the sub-count residual across FFI calls — without which a
slow drag truncates to zero **every** frame and the pointer never moves at all.

In Rust rather than in the shells because otherwise it is written twice, in Kotlin and in Swift, and
the two drift — and this crate is the only place it can be tested.

### App Store §4.7 self-audit

Checked against the shipped tree rather than against intent. Two of the three risk classes are
clean: **no ROM acquisition path** and **no bundled copyrighted content**. The third has
**findings**: "Super Nintendo Entertainment System", "Super Famicom" and "Super Scope" appear in
user-facing strings. The app's own identity is clean, and naming the emulated hardware is ordinary
nominative use — but App Store review is conservative here. `docs/mobile-readiness.md` records a
**recommendation** to soften the two iOS-visible surfaces before submission, left for the maintainer
to accept or reject rather than applied unilaterally.

> This is the **authoritative** §4.7 audit for the project. A narrower mobile-shell supplement was
> added in `v1.30.0` and reaches a different trademark verdict *because it never looks outside the
> two shells*; where they differ, this one governs.

---

## Compatibility and upgrade notes

- **Save states:** format version unchanged.
- **Timing behaviour changed.** The NTSC frame now alternates 357,368 / 357,364 master clocks and
  PAL interlaced frames alternate 425,568 / 425,572. This is a correctness fix, but anything that
  assumed a constant frame length will see the alternation.
- **One framebuffer golden re-blessed:** `hdmaen_latch_test`,
  `0xd518b7c9df2c9725` → `0x8f60351e0cdd8125`, arbitrated by both references after the oracle fix.
  Its status is recorded honestly as a **regression lock, not an accuracy oracle** — it is 23 rows
  from MesenCE on `main` already.
- **`SNES9X_KNOWN_FAILURES` 12 → 14**, both from real snes9x sprite-flag divergences the new rows
  expose.
- **Android:** `android/gradle.properties` and the Gradle wrapper are now committed. A local build
  that previously relied on user-level `~/.gradle` settings now works from a clean checkout. JNA
  bumped 5.15.0 → 5.17.0 for 16 KB page alignment on non-arm64 ABIs.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                                  # 872 passed, 0 failed
cargo test --workspace --features test-roms             # the golden suites (NOT run by the line above)
cargo run -p accuracysnes-gen                           # never piped through `tail`
REF_PROJ=$PWD/ref-proj bash scripts/accuracysnes/crossval.sh
```

- Battery: 335/335 on-cart status bytes under both references.
- Scenes: **53/53 matching on snes9x and on Mesen2**, 0 unblessed, 0 mismatched.
- Every new row verified by injecting the bug **at the site the row names** and confirming *which*
  failure code fires.

## Included changes

| PR | Commit | Summary |
|---|---|---|
| #282 | `5f9f0d8` | `test(accuracysnes)`: A5.19 — RTI is 7 cycles native, 6 emulation |
| #283 | `f004f4e` | `docs(accuracysnes)`: record what each remaining v1.28.0 row actually needs |
| #284 | `cbdbb16` | `docs(accuracysnes)`: correct the C7.05/C7.06 verdict — no eval-line gap |
| #285 | `1b0020f` | `test(accuracysnes)`: C7.05 — Range Over trips at OAM.INDEX * 2 |
| #286 | `e194a38` | `test(accuracysnes)`: C7.06 — Time Over reads set by V = YLOC + 1 |
| #287 | `ee34709` | `fix(accuracysnes)`: diagnose the Mesen2 oracle; flag an unverified figure |
| #288 | `c20dd02` | `test(accuracysnes)`: harden A3.03's stack; correct the oracle root-cause claim |
| #289 | `10841d8` | `ci(android)`: build every ABI and gate on 16 KB page alignment |
| #290 | `a2a615b` | `ci(ios)`: launch the app in a simulator, not just link it |
| #291 | `e72de97` | `docs(mobile)`: complete the App Store 4.7 self-audit |
| #292 | `0e1aa13` | `feat(mobile)`: map touches to Super Scope and Mouse coordinates |
| #293 | `d200e3a` | `fix(ppu)`: the field flag toggles every frame, not only when interlaced |
| #294 | `23fb76a` | `feat(ppu)`: model the short scanline (B2.02) |
| #296 | `e41c9a5` | `fix(accuracysnes)`: the oracle's "14 of 335" does not reproduce |
| #297 | `b2057b2` | `feat(ppu)`: model the long scanline (B2.03) |

Full per-entry detail: [`CHANGELOG.md` → `[1.28.0]`](../../CHANGELOG.md).
