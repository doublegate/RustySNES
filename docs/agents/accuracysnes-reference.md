# AccuracySNES agent reference

Moved verbatim from the AGENTS.md project block: AccuracySNES working rules, cartridge layout, and the Group F / Group E contracts.

## AccuracySNES (the first-party accuracy cartridge)

Two artifacts from one build: `build/accuracysnes.sfc` and `build/accuracysnes-pal.sfc`, which
differ in **one header byte plus the checksum** — so any behavioural difference between them is the
video region and can be nothing else. Region-dependent tests key on the *measured* frame height,
never on the region bit (whose position `$213F` bit 4 was itself settled by diffing the two images).

**Two tiers, deliberately never summed.** The battery is self-scoring: the cart decides pass/fail
on-cart and the host supplies no expected values, so a result means the same thing on any emulator
and on a flash cart. Rendered **scenes** (`docs/adr/0013`) cover the parts of the PPU that only
decide what appears on screen — the cart renders, the host hashes a fixed 256x224 region of
canonical pixels and compares against `tests/golden/accuracysnes-scenes.tsv`. A scene needs a host
holding the golden, so scene results stay in their own column in the coverage report and out of the
pass rate. Per ADR 0013 a golden is blessed **only** from a render the reference emulators agree on.

Working rules that have each already cost a debugging session:

- The generator emits `.a8`/`.a16` from every `sep`/`rep`; ca65 tracks immediate width from the
  directives, not from the instruction. A miss assembles a 2-byte immediate after `sep #$20` and
  desynchronises everything after it. Assembly helpers are **width-neutral** (`php`/`plp`).
- Each scene starts from a rebuilt canvas *and* re-run `init_registers`. Build the canvas once and
  the first scene to touch VRAM silently changes the picture for every scene after it.
- A scene can arrange a state **no picture can show**, and cross-validation cannot catch that class
  at all — an unshowable scene hashes stably and every emulator agrees with it. Two instances, both
  now handled by `scene_low_tiles`: a tile below `$10` covers only ASCII 0-31 (a 4bpp tile spans two
  font glyphs, an 8bpp tile four), which are blank; and a vertical offset that is a multiple of 16
  is invisible against a 16-tile cycle. Check that a scene renders what it claims to arrange.
- `STZ` has no long-addressing form; `cop #$00` is rejected by ca65 2.19 (emit `.byte $02,$00`);
  menu labels are capped at 24 columns.
- **Never hand-write a verdict byte.** Use the assertion helpers even when the condition is not an
  equality — `assert_a16_range` expresses "must not be X" fine. A hand-written `sta V_TEST_RESULT`
  puts a code in the ROM that the generated `ERROR_CODES.md` cannot know about, so the table stops
  being the complete account of failure bytes that it exists to be. Got wrong twice.
- **A guard must not subsume the assertion it protects.** `F1.05` first guarded on the whole
  auto-read word and then asserted the signature nibble within it — arithmetically the guard could
  never pass while the assertion failed, so the assertion was dead. Mask the guard down to the part
  the assertion does not cover, and confirm by injecting the named bug and checking *which* code
  fails.
- **Inject at the site the test names, not at any site that moves the reading.** `E6.08` (BRR
  15-bit decode overflow) passed on a reading produced entirely by the *gaussian* accumulator one
  row away: injecting into the BRR clamp moved nothing, injecting into the interpolator flipped it.
  Two real 16-bit overflows, one row apart, same observable sign flip. If the injection at the named
  site does not move the verdict, the attribution is wrong even when the test passes.
- **A verdict that encodes a timing phase will change for reasons unrelated to its subject.** Adding
  a PPU test shifted the DSP poll phase and broke two APU tests that had not changed. If the phase
  *is* the subject, record it and score nothing (`E8.07`); if it is not, widen the comparison by one
  sampling interval (`E7.09`). The NTSC/PAL drift gate is the tripwire for this, not only a region
  check.
- **When a source says when a result becomes valid, waiting for it is part of the measurement.**
  `wait_vblank_far` returns at vblank start, which is exactly when the automatic joypad read begins;
  Group F burns ~7 scanlines before reading `$4218` because the read takes ~3 and `F1.12` says
  results are valid by `V = $E3`. Same shape as `E10.05`, where "forced into release" is not
  "zeroed on the spot".
- **The measurement channel (`MEAS_BASE`, 240 slots) has no allocator**, and `dossier::check_slots`
  fails the build on a collision — it prints the free list, and prints an empty one when the channel
  is full (widen it; `$7EE200` has room to `$7EF000`). The opcode sweep owns 8-75 invisibly to grep.
- Three emulators failing **identically** usually means a broken test; RustySNES failing **alone**
  means a real bug. Both have happened repeatedly — check which before investigating. But the first
  is a **heuristic, not a proof**: a harness bug upstream of every implementation produces the same
  signature, and one did — it cost a published finding that had to be retracted (see the `$F8`/`$F9`
  correction in the CHANGELOG).

### Cartridge layout — what can move, what cannot, and why

A 256 KiB LoROM image, 8 banks. The generator prints per-bank headroom on every build and **fails at
512 bytes free**, so an overflow arrives as a warning with room to act rather than as an `ld65` error
mid-change. It has overflowed four times without that gate.

| bank | holds |
|---|---|
| `$00` | runtime, `RODATA`, and Groups **A + B** bodies — pinned, see below |
| `$01` | `FONT` · `$02` `TESTSE` · `$03` `TESTSC`+`TESTSG`/F · `$04` `CATALOG` · `$05` `TESTSD` |
| `$06`/`$07` | `APUDATA`, packed across two segments |

Three facts that are not obvious and each cost a wrong turn:

- **Bank `$00` is 32 KiB whatever the cart size.** LoROM maps only its `$8000-$FFFF`, and the
  vectors, header and everything reached by a bank-local `jsr` live there. Growing the image does
  not give it a byte; only moving bodies out does (`emit.rs`'s `OUT_OF_BANK`).
- **A segment cannot span a bank boundary.** That, not total space, is why the image is 256 KiB:
  `TESTSE` will outgrow one bank before Group E is finished, and needs somewhere to split into.
- **Groups A and B must NOT be relocated.** `A4.11`/`A4.12` aim an `(a,X)` indirect jump at the
  *program bank's* signature block to tell an in-bank wrap from a bank carry, and Group B's
  access-speed rows depend on where they execute. Moving either leaves the tests green while
  measuring something else. The build gate rejects it, and `emit.rs` says so.

A relocated group reaches the runtime only through the `_far` wrappers (`wait_vblank_far`,
`hv_begin_far`, …) — and note that `Asm::measure_begin`/`measure_end` emit the bank-local form from
*inside the DSL*, so grepping the test files for `jsr` does not find them; use the `_far` variants.
Address immediates (`ldx #@data`) need `.loword(…)` once a group leaves bank `$00`.

The measurement channel is 512 slots (`u16`) at `$7EE200`, with room to `$7EF000` for 1,792.
`dossier::check_slots` fails the build on a collision and prints the free list — an *empty* list
means the channel is full, which is the same gate answering a different question.

### Group F — the host input contract

Group F is untestable without one. With nothing plugged in and no button held, **every controller
observable the cart can reach is `$0000`** — the manual shift register, the auto-read results, the
power-on state — so a test of the read order, of the signature nibble, or of what a disarmed
auto-read preserves has nothing to distinguish from anything else. `F1.07` was written and withdrawn
for exactly this before the contract existed.

**Every runner holds both controllers for the whole run**: the in-repo harness (`set_joypad`), the
snes9x libretro driver (`input_state`), and the Mesen2 script (`emu.setInput` inside an
`inputPolled` callback, which Mesen2's docs require). The masks are declared in `runtime.inc`:

- `PAD_CONTRACT = $9050` — B + Start + X + R on controller 1.
- `PAD2_CONTRACT = $60A0` — Y + Select + A + L on controller 2.

Each is chosen against the same three requirements, and all three have caught something: **no
d-pad** (the post-battery menu scrolls on Up/Down), **bits in both bytes** (so a host reporting one
half is visibly wrong rather than accidentally right), and **asymmetric under bit reversal** (so an
LSB-first read cannot pass by accident). The two masks additionally **share no set bit**, which is
what makes "port 2 latched" distinguishable from "port 2 is echoing port 1".

Adding the contract found two RustySNES defects immediately: `$4218`-`$421F` reported the live pad
rather than the automatic read's result, and the read ignored the `$4016` latch line. Both were
invisible with nothing held, because both wrong models produce `$0000` in every phase.

`F1.07` must run **before** anything else arms auto-read — its first phase reads `$4218` while it
has never been written. Its guard catches a violation rather than hiding it, which is how the
ordering was found.

### Group E — the APU, reached through four bytes

The SPC700 is a separate processor with its own RAM; the only channel is `$2140`-`$2143`. The cart
uploads a small SPC700 program through the IPL boot handshake (`apu_upload` in `asm/runtime.s`),
lets it run, and reads its answers back. `gen/src/spc.rs` assembles those programs — `ca65` does not
speak SPC700. Verify any new opcode encoding against
`crates/rustysnes-apu/src/spc700_exec.rs`'s dispatch table.

- **Every program must hand the APU back to the IPL** (`release_to_ipl`), and that path **re-maps
  the IPL ROM first**. `$F1` bit 7 selects whether `$FFC0`+ reads as the boot ROM or as RAM, so a
  test that writes `$F1` for its own reasons (enabling a timer, say) leaves `JMP $FFC0` landing in
  dead RAM — and then *every upload after it silently fails* while the battery still reports 100%.
- **Every handshake wait is bounded**, and a test whose APU never answers reports SKIP. An
  unbounded wait hangs the whole battery and reports nothing about any other test.
- The emitter carries only opcodes a committed test exercises. An unexercised encoding is an
  unverified one, and a wrong byte in it surfaces as an emulator disagreement rather than as an
  assembler bug.
