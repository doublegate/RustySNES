# Changelog

All notable changes to RustySNES will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **RustySNES integrates a cycle-accurate emulation engine.** Modeled after its predecessor `RustyNES`, this emulator is built on a master-clock-precise, lockstep-scheduled core targeting the Mesen2/ares accuracy bar. The entries below document the engine-internal milestones as this core is built and hardened.

## [Unreleased]

### Added

- **AccuracySNES `C1.08` (OAM address taken over during active display) is now a scored row, the
  first coverage the per-dot compositor unblocks (294 -> 295 of the on-cart scoring total).** During
  active display the renderer drives the OAM address, so a `$2138` read returns the sprite-evaluation
  address (`eval_index << 2`), not the CPU-programmed one — behaviour the shipped per-dot compositor
  now models. The test reads `$2138` at a *controlled* low dot (an H+V IRQ armed at `(V=50, H=24)`
  plus `SEI`/`WAI`, which resumes inline with no dispatch latency), so `eval_index` is well under 32
  and the render address is provably below the programmed `$80` regardless of region or the code that
  runs before it — the old fixed-burn read landed at a region-dependent dot and could only record a
  variant. Retiered `Contested`/golden -> `Documented`/scored on nocash fullsnes and the SNESdev Wiki,
  and cross-validated: Mesen2 agrees (`GetOamAddress` returns the same render address), while snes9x
  and the batch compositor read back the programmed `$80` — a now-documented snes9x divergence.

- **AccuracySNES dossier `C3.04` (CGRAM taken over during active display) is now a scored row
  (295 -> 296), the CGRAM sibling of `C1.08`.** A `$2122` write during active display commits to the
  colour the PPU is drawing — its internal CGRAM address — not the CPU-programmed CGADD (ares/MesenCE
  `!CanAccessCgram` → `InternalCgramAddress`). With every main-screen layer off (`TM = 0`) the whole
  active line is the backdrop, palette 0, so the redirect target is a known 0. The test seeds colour 0
  and colour `$10` to distinct values, points CGADD at `$10`, and writes a third value at a controlled
  active-display dot (the same H+V IRQ + `SEI`/`WAI` sync as `C1.08`, so the write lands where the
  redirect is live rather than in h-blank — a fixed burn could not guarantee that across emulators):
  colour 0 must take the write and colour `$10` must stay its seed. Cross-validated — Mesen2 agrees,
  snes9x uses the programmed CGADD and fails (a now-documented divergence).

- **AccuracySNES `G1.16` (ExHiROM A23->A22 half-selection) via a third, two-half cartridge image
  (coverage 338 -> 339 of 443).** ExHiROM inverts address bit 23 into ROM offset bit 22 — banks
  `$80-$FF` (A23=1) select the first 4 MiB, `$00-$7D` (A23=0) the extra 4 MiB — so distinguishing the
  two halves requires a genuine >4 MiB image whose halves are distinct physical bytes (a smaller image
  would mirror them together). The generator now emits a third self-scoring image
  (`build/accuracysnes-exhirom.sfc`, `$410000` bytes; the `$FF` fill compresses to a few KiB in git):
  the shared `runtime.s` (again under `-D HIROM_BUILD`, which also drops the LoROM-only signature
  blocks and scene loop) linked with a new `exhirom.cfg` and `header-exhirom.s` (`$FFD5 = $25`). The
  load-bearing constraint is that ExHiROM maps bank `$00` (A23=0) into the *extra* half, so the runtime
  links at CPU `$00:8000` (where `$21xx/$42xx` still decode as MMIO) but its bytes sit at ROM
  `$408000`; the reset vector reads `$40FFFC`. The image plants two landmark bytes — `$A1` at ROM
  `$000000` (first half) and `$E2` at ROM `$400000` (extra half) — and `g1_16` reads them through
  `$C0:0000` and `$40:0000`, asserting the A23->A22 inversion selected different halves. Verified on
  RustySNES, snes9x, and Mesen2, with a non-vacuity injection in `ExHiRom::map` (forcing the linear
  region's high bit to 0 so both banks read the first half fails `g1_16` alone while the runtime — in
  the untouched windowed region — still boots). The coverage report now spans all three images'
  batteries.
- **AccuracySNES `G1.15` (HiROM decode) + `G1.17` (HiROM SRAM) via a parallel HiROM cartridge image
  (coverage 336 -> 338 of 443).** The generator now emits a second, minimal self-scoring image
  (`build/accuracysnes-hirom.sfc`, 64 KiB) alongside the LoROM battery: the shared `runtime.s`
  (assembled with `-D HIROM_BUILD` so its LoROM-only per-bank signature blocks are dropped) linked
  with a new `hirom.cfg` and `header-hirom.s` (`$FFD5 = $21`, 8 KiB battery SRAM). The load-bearing
  constraint is that the runtime links into the `$00:8000-$FFFF` window so `phk/plb` keeps `DBR` in a
  bank where MMIO decodes; a `$C0` linear low half holds the font and the position-independent test
  bodies. `g1_15` asserts the HiROM decode (the `$00`-window, `$C0` linear, and `$40-$7D` mirror all
  read the same ROM offset); `g1_17` asserts the `$20-$3F:$6000-$7FFF` SRAM window and its `$A0-$BF`
  mirror. Verified on RustySNES, snes9x, and Mesen2 (2/2 each), with per-test non-vacuity injections
  in `HiRom::map` (dropping the `$40-$7D` mirror fails `g1_15` alone; disabling SRAM fails `g1_17`
  alone). The coverage report now spans both images' batteries. ExHiROM (`G1.16`) landed next as a
  separate third image (above).
- **AccuracySNES `F1.03` — the shared `$4016` latch, unblocked by attaching Mesen2 port 2 (coverage
  335 -> 336 of 443).** One write to `$4016` bit 0 latches BOTH controller ports' shift registers.
  The row needs a second controller held at a mask disjoint from port 1's (`PAD2_CONTRACT = $60A0`
  vs `$9050`) so "port 2 latched" is distinguishable from "port 2 echoes port 1"; it had been
  withdrawn because Mesen2's headless `--testrunner` had no device in port 2. Fixed by passing
  `--snes.port2.type=SnesController` to every Mesen2 invocation in `scripts/accuracysnes/crossval.sh`
  (a generic config switch applied before the core config push, not persisted). All three runners now
  hold port 2; `f1_03()` latches once and reads both ports interleaved, asserting port 1 = `$9050`
  (guard) and port 2 = `$60A0`. Verified across the harness battery (327/327), Mesen2 (0 failing),
  and snes9x (7 known divergences, F1.03 not among them), both NTSC and PAL images, 53/53 scenes
  unchanged; the non-vacuity injection (hold port 2 at `$0000`) fails F1.03 alone.
- **AccuracySNES `G1.05` — power-on PPU registers reported as a golden (coverage 334 -> 335 of 443).**
  The dossier row says the PPU has no boot ROM and most of its registers start indeterminate.
  `capture_power_on` now samples the *readable* ones before `init_registers` writes the PPU into its
  known state — the Mode 7 multiply `$2134`-`$2136` (= M7A x M7B, both power-on-undefined) and the
  status registers `$213E`/`$213F` (STAT77/STAT78, whose low bits are the defined PPU1/PPU2 version
  and the rest indeterminate) — and `g1_05` reports them to the measurement channel, **never
  asserted** (identical in spirit to `G1.03`/`G1.20` for the CPU/APU side; a scored value would be
  meaningless because the row's whole content is that these are undefined and the reference cores
  need not agree). Battery 292/292 scoring (100.00%), golden 33 unscored; snes9x + Mesen2 cross-val
  agree with no new divergences.

- **AccuracySNES `E4.08` — the IPL boot ROM pokes DSP registers through a `$00F2` transfer (coverage
  333 -> 334 of 443).** The IPL's inner loop stores each uploaded byte with `MOV ($00)+Y, A`, so the
  transfer *destination* is an ordinary SPC700 address — and `$00F2`/`$00F3` are the memory-mapped
  DSPADDR/DSPDATA registers. Aiming a transfer at `$00F2` therefore lets the boot ROM itself write
  the DSP before any game code runs, a documented trick (fullsnes, APU / S-DSP) real sound drivers
  use to pre-load the DSP during upload. The test (reusing `E4.06`'s two-block `apu_upload_2block`)
  sends the pair `[$0F, $7E]` to `$00F2` — poking FIR echo-coefficient 0 with a distinctive value —
  then a verifier reached through the non-zero continue reads DSP register `$0F` back and restores it
  to `$00` so nothing leaks into a later echo test. A first phase sets `$0F` to a known baseline
  `$A5` before the poke and leaves DSPADDR selecting a *different* register (`$1F`), so the `$7E`
  assertion proves the poke *changed* `$0F` through **both** the DSPADDR (`$00F2`) and DSPDATA
  (`$00F3`) writes — a broken DSPADDR write would leave DSPADDR at `$1F` and land `$7E` in `$1F`,
  leaving `$0F` at `$A5` — rather than depending on power-on state or a conveniently pre-selected
  DSPADDR. Confirmed by injection: pointing the same transfer at plain ARAM (`$0250`) instead of
  `$00F2` leaves `$0F` at the `$A5` baseline and fails `E4.08` alone — the poke works precisely
  because it targets the DSP ports. Battery 292/292 scoring (100.00%); snes9x + Mesen2
  cross-validation agree with no new divergences.

- **AccuracySNES `E4.06` — the IPL boot ROM's multi-block continue (coverage 332 -> 333 of 443).**
  The IPL reads port 1 at every block boundary and its value is the whole decision: zero means "the
  address in ports 2/3 is an entry point, jump there", non-zero means "it is another block's
  destination, keep transferring". The single-block `apu_upload` only ever drives the two ends every
  upload must use — a non-zero at the opening kick and a zero at the close — so the middle case, a
  non-zero *close* that continues to a second block (the boot ROM's `BNE $FFF9`), was unexercised by
  the whole battery. A new runtime helper `apu_upload_2block` drives it: block A (three distinctive
  data bytes) is transferred to `$0250`, closed with a non-zero port 1 naming a second destination,
  block B (a verifier program) is transferred to `$0280` through that continue, and only the final
  close carries the zero that jumps into the verifier — which reads block A back and reports it. Every
  handshake wait stays bounded exactly as `apu_upload`'s do, so a core that mishandles the continue
  stands the test down (SKIP) rather than hanging the battery; `apu_upload` itself is untouched.
  Verified by injection both ways: forcing the boot ROM's `$FFF9` branch to always jump breaks the
  battery (the continue is load-bearing), and pointing the verifier at an address block A never wrote
  fails `E4.06` alone (the read-back assertion is real). Battery 291/291 scoring (100.00%), and
  snes9x + Mesen2 cross-validation agree with no new divergences — the multi-block continue is
  standard IPL behavior all three implement.

### Changed

- **The per-dot PPU compositor (`docs/adr/0014`, T-CA-10) is now the shipped default renderer.** The
  `per-dot-compositor` feature is on by default in `rustysnes-core` (propagating to the frontend and
  the test harness) and in `rustysnes-ppu`/`rustysnes-test-harness`; the batch compositor stays
  reachable, byte-identical to the pre-flip renders, via `--no-default-features` (the `no_std`/
  thumbv7em gate builds it that way, verified). The shipped emulator now composites one dot at a time
  with live registers, so a CGRAM/OAM access during active display hits the color/sprite-eval address
  the hardware is drawing rather than the CPU-programmed one. The AccuracySNES self-scoring battery is
  **294/294 both ways** (zero regression) and the shared framebuffer corpus is re-blessed to the
  per-dot values, each cross-validated against the MesenCE oracle: `inidisp_brightness_delay` and
  `inidisp_enable_display_mid_frame` move to their MesenCE-agreeing per-dot hashes. One documented gap
  remains — `undisbeliever/inidisp_forgot_to_force_blank` (a PPU access during active display without
  forced blank) renders `7fff` where MesenCE renders `7fc6`; it is pinned as a known per-dot gap
  pending Phase 4d (PPU access-during-render) rather than blessed. `C1.08` is declared region-dependent
  (its mid-render `$2138` read samples a dot-sensitive address the region's frame timing shifts).

### Fixed

- **AccuracySNES no longer regresses its OAM tests on a Select restart under the per-dot compositor.**
  The battery's standing precondition is forced blank (`INIDISP=$8F`), which is what lets the OAM port
  return the CPU-programmed address; the cold-boot path sets it at reset before `restart_entry`, but a
  Select restart re-entered with the menu's display ON and `init_registers` never touched INIDISP, so
  the C1.01-C1.05/C1.03b OAM tests re-ran mid-render — where the per-dot PPU correctly redirects OAM to
  the sprite-evaluation index and the read-back no longer matched the written value (6 tests `Pass` ->
  `Fail`). `run_all_tests` now establishes forced blank at its own entry so every entry path shares the
  precondition. A no-op under the batch compositor.
- **AccuracySNES `B4.12` (`$4211` read releases the IRQ latch) no longer depends on where the polling
  loop catches the flag.** A `$4211` read on the exact dot the flag is raised returns it set but does
  not clear it (the four-master-clock `/IRQ` hold that hardware and ares model, and RustySNES models
  correctly). B4.12 used one read to both detect and acknowledge, so if the poll happened to catch the
  hold dot the latch was never released and the follow-up read failed — a phase-dependent verdict that
  the region and any change to the code running before the test could flip (the per-dot C1.08 rewrite,
  which adds a few frames ahead of it, tripped it on the PAL image). It now takes an explicit second
  read, guaranteed past the one-dot hold, as the release; the verdict is region- and phase-independent
  and still agrees with Mesen2 and snes9x.

- **`$4210`/`$4211` now hold their flag for four master clocks after the edge (cycle-accuracy,
  Tier-1 T-CA-02 — completes the ticket).** RDNMI and TIMEUP previously cleared their flag on every
  read. Hardware holds `/NMI` and `/IRQ` across the VBlank/IRQ edge: a `$4210`/`$4211` read within
  the first four master clocks (one dot / one interrupt poll) after the flag is raised returns bit 7
  set but does **not** clear it (Terranigma depends on the RDNMI flag surviving such a read). Modeled
  as `Clock::rdnmi_hold`/`irq_hold`, set with the flag and consumed at the next dot in `tick_ppu_dot`
  so the window is exactly the dot the flag was raised on — matching ares `status.nmiHold`/`irqHold`
  ("hold for four cycles") and fullsnes. The hold is serialized (`FORMAT_VERSION` bumped 5 -> 6,
  `docs/adr/0006`) so a save inside the window restores identical read-clear behavior; old blobs fail
  loudly. Verified: new `rustysnes-core` unit test (held read does not clear, post-hold read does),
  AccuracySNES battery 292/292 with `B4.03`/`B4.04`/`B4.05` (the RDNMI read-clear/auto-clear tests)
  unregressed, the full test-roms harness green (49 golden-framebuffer + coprocessor tests unchanged
  — no NMI-timing frame shift), and the save round-trip suite green. This is the held-flag half of
  T-CA-02; the `$4210`/`$4211` open-bus bits landed earlier (#204).

- **Automatic joypad read is now a timed ~4224-clock operation with a busy flag, not an instant
  latch (cycle-accuracy, Tier-1 T-CA-01/03).** The auto-read previously completed the moment vblank
  started, `$4212` bit 0 (auto-joypad busy) was never set, and `$4218-$421F` held the new result
  immediately. It now models ares' `status.autoJoypadCounter` as a master-clock deadline: at vblank
  entry the controller state is snapshotted, `$4212` bit 0 reads **busy** for the next 4224 master
  clocks (33 steps x 128, ~3 scanlines), and the result publishes to `$4218-$421F` only at
  completion — so a read during the window still sees the previous frame's value, as on hardware. The
  busy window also fills `$4212` bits 1-5 with open bus. Verified: two new `rustysnes-core` unit
  tests (the busy window + deferred publish, and the open-bus bits), AccuracySNES battery 290/290,
  and snes9x + Mesen2 cross-validation unchanged (0 new divergences). The in-flight read's start
  snapshot and busy deadline are in the save state (`FORMAT_VERSION` bumped 4 -> 5, `docs/adr/0006`),
  so a save taken during the ~4224-clock window restores identical machine state; a round-trip test
  covers the mid-window case, and pre-5 blobs fail loudly (no silent misinterpretation).
- **`$4210` (RDNMI) and `$4211` (TIMEUP) now return open bus in their unused bits (cycle-accuracy,
  Tier-1).** Both registers previously returned `0` in the bits the hardware leaves floating —
  `$4210` bits 4-6 and `$4211` bits 0-6 — so a ROM that reads the full byte (rather than masking the
  flag) saw the wrong value. They now OR in the CPU open-bus / MDR (`self.open_bus`, the pre-read
  last-driven value) in exactly those positions, matching ares `CPU::readIO` (which writes only the
  flag and, for `$4210`, the version nibble, leaving the rest as the incoming open-bus data) and
  fullsnes. `$4210` keeps bit 7 = the read-clearing VBlank flag and bits 0-3 = CPU version 2; `$4211`
  keeps bit 7 = the read-clearing IRQ flag. First landed item of the Tier-1 cycle-accuracy
  remediation program (`to-dos/TIER1-CYCLE-ACCURACY.md`, T-CA-02). Verified: core + CPU unit suites
  green, AccuracySNES battery 290/290 (B4.03/B4.04/B4.05 RDNMI tests unchanged — they mask the flag).

- **OBJ interlace (`SETINI` $2133 bit 1) is now rendered, not just stored.** The bit was decoded and
  saved but never consulted, so a sprite drawn under OBJ interlace kept its full height where
  hardware halves it. The sprite renderer now ports ares `Object::onScanline`/`fetch`: the
  on-scanline height is `height >> obj_interlace` and the fetched row is `(row << 1) + field` (`-`
  when v-flipped), applied after the width-based V-flip — so a 16x32 sprite squishes into 16 lines,
  sampling every other row per interlace field. Found via AccuracySNES C7.12; with the port in place
  RustySNES matched Mesen2 exactly on a rendered 16x32 interlace scene, the 52 existing scenes and 29
  PPU unit tests are unregressed (interlace is off by default), and a new `rustysnes-ppu` unit test
  pins the 32→16 squish. The C7.12 scene itself can't be committed as a golden: snes9x renders the
  interlace field differently, so the three references don't agree (ADR 0013).

- **Menu navigation is now flicker-free.** Moving the cursor within the visible window rewrites only
  the `>` marker — two character writes inside the vblank — so the display never blanks. Only when
  the window actually scrolls (every visible name changes) does the full blank-and-redraw run. Before
  this, every keypress blanked the whole screen for a frame. `cursor_up`/`cursor_down` now flag the
  redraw as marker-only (`V_DIRTY = 2`) or full (`V_DIRTY = 1`), and record the previous cursor row so
  the old marker can be erased. Asserted: a within-window Down move keeps brightness at 15 and moves
  the `>`; a scroll past the window still does the full redraw.

- **Pressing Down (or any navigation key) blanked the results menu and hung the ROM.** The menu
  redraw blanks the screen, calls `draw_list`, then un-blanks — but `draw_list` returns with `A`
  16-bit while the assembler was still `.a8`, so `lda #$0F` assembled one byte where the CPU read
  two, ate the `sta INIDISP` opcode, and the un-blank never ran: the display stayed forced-blank and
  the instruction stream desynced. The headless test missed it because it read VRAM directly (intact
  even when the display is off); it now asserts `display_brightness` recovers. The cursor was moving
  correctly the whole time — only the display was dead.
- **The results menu is now drawn in green, not the last scene's leftover palette.** The font's "on"
  pixels index CGRAM colour 1, and nothing reloaded the palette after the rendered scenes overwrote
  CGRAM — so the menu inherited whatever colours the final scene left (orange, or a per-tile
  green/orange mix). `load_palette` now sets colour 1 to `$03E0` (green), and `restart_entry` reloads
  it after the rendered scenes overwrite CGRAM, before `draw_screen` runs.

- **The interactive menu is now navigable and self-correcting, after a real cartridge showed three
  more defects.** The D-pad blanked the screen and killed the ROM: `cursor_up`/`cursor_down`
  returned `A` 8-bit while `main_loop` continued `.a16`, so ca65 emitted a 16-bit `bit #PAD_DOWN`
  the CPU decoded 8-bit and ran the immediate's high byte as an opcode. The results list came out in
  two colours alternating per character, because the drawing routines write only `VMDATAL` and the
  tilemap word's high byte — the palette — held whatever the last scene left; `blank_rows` now
  clears full words. And the redraw ran during active display where writes are dropped; it now
  blanks the screen for the one frame it takes.
- **`V_MENU_MODE` was uninitialised, which hung the whole battery on snes9x.** `test_restore_target`
  reads it after every test to decide whether to tally or return to the menu, and snes9x fills WRAM
  with garbage where RustySNES zeroes it — so the first test bounced to the menu and the battery
  never finished. The in-repo harness could not catch it; cross-validation did. Same class as the
  cursor variables, now initialised beside them at reset.

- **The interactive menu never drew its test list, and its tallies read zero.** Three width bugs,
  compounding. `draw_str` returned with `A` 8-bit, so `draw_screen`'s `lda f:R_PASSED` loads
  following it read only a low byte. `draw_dec3` did `pha` with `A` 16-bit and `pla` with `A` 8-bit,
  leaking one byte of stack per digit, so the `plx` after it picked up a misaligned VRAM address and
  the tens and units were drawn wherever it pointed. And the tally columns (2, 9, 16, 23) did not
  match `str_tally`'s own placeholders (2, 8, 14, 21).
  Also fixed with them: `@blank_rest` in `draw_list` was an alias for `@done` and never blanked
  anything, so the rendered scenes' tilemap showed through behind every row — there is now a
  `blank_rows` helper the header and list share; and `V_CURSOR`/`V_SCROLL`/`V_DIRTY` were never
  initialised, which is harmless where WRAM powers up zeroed and is not on hardware, where `G1.07`
  records it as undefined. The menu now shows its title, a tally matching the results block, and the
  list with names, cursor and verdicts.
  It had been broken since it was written, because the menu is drawn by code **no gate observed**.
  It surfaced only when moving `CATALOG` forced a look at `draw_str`, and the obvious reading was
  that the move had broken it — the control that disproved that also proved the bugs pre-existing.

- **The PPU dot model was uniform and one dot too long (`T-06-A`).** A scanline is 1364 master
  clocks, and both `341 × 4` and `338 × 4 + 2 × 6` satisfy that — which is why the wrong one kept
  perfect frame timing while reporting an `OPHCT` value hardware never produces. fullsnes' *PPU
  H-Counter-Latch Quantities* histogram settles it by measurement: sampling `$2137` once per master
  clock across a line, dots 323 and 327 latch **six** times each and dot 340 **never**. bsnes, ares
  and Mesen2 all implement it; snes9x uses 322/326 and is the outlier. `DOTS_PER_LINE` is now 340
  and `rustysnes-core` carries `LONG_DOTS`/`dot_length`.
  Both long dots sit at `H ≥ 323` — past the visible window, past hblank's start at 274 and past
  `HDMA_RUN_DOT` — so dots `0..=322` kept their exact clock alignment and **nothing moved**: all 50
  blessed scenes, both `hdmaen_latch_test` goldens, the whole battery and `B4.16`'s recorded H-IRQ
  positions were identical before and after. The short (1360) and long (1368) lines remain
  unmodelled, and the H-IRQ comparator deliberately stays in the dot domain — converting it would
  shift IRQ timing and re-bless framebuffer goldens, which wants its own adjudication.

- **The automatic joypad read ignored the `$4016` latch line.** On hardware the read clocks the
  ports' shift registers, and while `$4016` bit 0 is held high those registers reload rather than
  shift — so all sixteen clocks return the first bit and the result is uniform, not merely stale.
  RustySNES copied the latched pad state regardless, so a driver that strobes `$4016` during vblank
  would corrupt the auto-read results on hardware and not here, which is the more dangerous
  direction. Found by AccuracySNES `F1.11`, and adjudicated by the references rather than by
  reading: Mesen2 corrupts (`$FFFF` with `B` held) and snes9x does not, so snes9x gains a seventh
  documented divergence.

- **`$4218`-`$421F` reported the live controller state rather than the automatic read's result.**
  `Bus::read_cpu_reg` returned `joypad[pad]` directly, with no result buffer and no `$4200` bit 0
  gate, so the registers tracked the pad continuously — software that disarms auto-read to poll
  `$4016` by hand would find the hardware's answer appearing underneath its own reads. Fixed by
  adding `joypad_auto`, copied at vblank entry and only while armed, with `$4218`-`$421F` reading
  the buffer; two unit tests in `bus.rs` pin it. Found by AccuracySNES `F1.07`, which could not
  detect it until the battery gained a host input contract: with nothing held, a correct buffer and
  a live passthrough both report `$0000` in every phase.

- **Fourteen measurement-channel slot collisions, and a build gate so there cannot be more.** The
  channel has no allocator — a slot is claimed by writing to it — so two tests picking the same
  number silently overwrite each other, and every reader of the earlier one starts reporting the
  later one's values *under the earlier one's labels*. A wrong number with a confident caption.

  Writing `E3.02` against slots 106/107 collided with `B3.01`, which surfaced it; a check added to
  the generator then found **twelve more that were already there**. Six were the trap the plan
  document already warned about — the opcode sweep owns slots 8-75 and is invisible to any
  `record(N` grep, so `B1.03`, `B1.04`, `B2.06` and `B4.07` had all been silently overwritten by
  it. The rest were `A5.10`/`A2.13` on 120, `A6.12`/`D1.08` on 122-123, and `D1.02` against both
  `B3.01` and `E1.07`.

  The corruption was real and visible once fixed: `B3.01` had been publishing a 2-dot excess
  between its shortest and longest sampled interval, because `D1.02` was overwriting one of the two
  values. With the slots separated it reads 65 and 65 — a perfectly flat sequence, which is the
  correct answer for a core that models no refresh pause.

  `dossier::check_slots` now fails the build on any duplicate, listing every clash at once together
  with the free slots, since fixing them one build at a time would be miserable. The channel is
  widened from 128 slots to 192 to make room, which touches the four places that know its size:
  `runtime.inc`, the generator, the harness and the libretro cross-validation host.

- **`$2137` latched the H/V counters unconditionally, ignoring the `$4201` bit 7 gate.** The
  software latch is wired to the same pin the light gun uses: superfamicom.org's register reference
  says reading `$2137` latches *"if bit 7 of `$4201` is set"* and that *"when bit a is 0, no latching
  can occur"*. RustySNES already modelled the falling-edge latch on that pin (`Bus::set_pio`) but
  latched on every `$2137` read regardless of the gate — correct for every ordinary program, which
  leaves the bit set, and wrong for anything that clears it.

  The gate now lives in the Bus, beside the `set_pio` edge handling, because the Bus is what owns
  the pin; the PPU gained only a read-only accessor for its open-bus latch, so the save-state format
  is untouched. Found by the new AccuracySNES `C3.10`, where snes9x and Mesen2 both gated it and
  RustySNES did not.

- **`JSR (a,X)` did not escape page 1 in emulation mode.** Its return-address pushes went through
  the page-1-confined stack path, so from `S = $0100` the second pushed byte wrapped to `$01FF` and
  corrupted the top of the stack page instead of escaping to `$00FF`. `JSL` already used the
  escaping path; `JSR (a,X)` is on the same escape list and did not.

  ares' `instructionCallIndexedIndirect` pushes with `pushN` and re-applies `S.h = $01` only at the
  instruction boundary, which is what RustySNES now does. Found by the new AccuracySNES `A3.08`.
  snes9x and Mesen2 both passed it while RustySNES failed, which is what pointed at a real defect
  rather than a broken test; the confirmation is ares' source, which pushes with `pushN` and
  re-applies `S.h = $01` only at the instruction boundary.

- **The coverage report counted unblessed scenes as coverage.** ADR 0013 rule 4 says an unblessed
  scene "is not yet evidence of anything" — it renders, and nothing has confirmed the picture is
  right — but `docs/accuracysnes-coverage.md` counted a scene the moment it existed. A scene could
  therefore claim an assertion by being written, which is precisely the gap the report exists to
  close for the battery. It now reads the golden file and counts only blessed scenes.

- **Sprite vertical flip is computed against the sprite's WIDTH, not its height** — and correcting a
  test is what found it. `c7-vflip-tall-halves` was selecting `OBJSEL` pair 3 with the size bit set,
  which is **32x32**: a square sprite, on which `C7.13`'s errata says nothing. Pointed at a real
  16x32 sprite, the scene split three ways — snes9x and Mesen2 agreed, RustySNES did not.

  On hardware each square half of a rectangular sprite flips *inside itself* and the halves do not
  swap places, which falls out of using the width. For square sprites the two are the same number,
  which is why nothing else in the corpus moved. RustySNES now matches both references bit-for-bit,
  and the re-blessed golden is that agreed value.

- **The gamepad's shift register was the button word itself, so a strobe never reloaded it.** The
  SNES pad is a *parallel-load* shift register: `$4016.0` high loads it from the button lines and
  low starts clocking, so a program may strobe and re-read as often as it likes within a frame and
  get the same answer each time. RustySNES shifted the button state in place — so the first manual
  read of a frame consumed the buttons, every later one returned all-ones, and a manual read also
  corrupted the auto-read result at `$4218-$421F`. **A frontend rewrites the button state every
  frame, which hid both**; a game that polls twice in one frame would not have been so lucky. The
  shift registers are now separate state, reloaded while the strobe is high *and* on its falling
  edge — a program that raises the strobe, changes the buttons, and lowers it must capture what is
  held at the fall, not at the rise — and a read taken while the strobe is still high returns the
  first bit without advancing, because a continuously reloading register never moves on. Saved as
  state (save-state `FORMAT_VERSION` 3 → 4, with the reason in `docs/adr/0006`).

- **`B4.12` asserted more than its citation.** It read `$4211` to acknowledge an IRQ and then read
  it again on the same scanline, expecting the latch released. But a V-only IRQ's comparator matches
  for the *whole* scanline, and while it matches a core is free to re-assert what the read released
  — so the second read said nothing about the release and everything about where in the line the
  polling loop happened to catch the flag. It began failing on Mesen2 when an unrelated change moved
  the battery's code by a few bytes. It now disarms `$4200` before looking again, which is the claim
  the dossier actually makes.

- **`E3.01` raced the timer it was reading.** Its two reads of `$FD` are about eight SPC700 cycles
  apart and a tick at that divider lands every 128, so a tick falling between them was uncommon
  rather than impossible — and when it did, the second read was non-zero for a reason that has
  nothing to do with whether the first one cleared it. It surfaced as Mesen2 failing the test on the
  PAL image only, after an unrelated change shifted the battery's timing. The timer is now stopped
  before the pair of reads.

- **The cross-validation gate could report a silent pass on zero rendered scenes.** The snes9x
  scene run has a frame budget that has to cover the battery *and* the scene loop after it, and the
  battery keeps growing; run short, the cart never reaches the scenes, the host reports none, and
  "0 mismatched" read as success. `check_scenes` now fails when it sees no scenes at all, and the
  budgets were raised.

- **SPC700 memory-access side effects (`E2`).** `E2.05`: direct-page indexing wraps *inside* the
  page, so `$FF + X` with `X = 2` reads `$01` and not `$0101` — a core computing a 16-bit sum reads
  from the wrong page entirely, which is silent until something lives there. `E2.01`: a store
  dummy-reads its destination, so `MOV $FD,A` **clears Timer 0** even though the counter is
  read-only — a trap for any driver that "initialises" the counters by writing them.

  `E2.01` first asserted the store's effect as a *difference* from a control reading, and that
  version failed on snes9x. The core was right and the assertion was weak: a core that does not
  clear leaves an arbitrary value there, and an arbitrary value lands inside a difference range
  often enough to pass or fail by luck. Asserting the post-store reading directly (empty, against a
  control that must have advanced) is both stronger and stable on all three.

- **First S-DSP behaviour tests, now that the DSP is reachable.** `E9.19`: writing `ENDX` clears
  it — any write, regardless of value — so a core modelling it as ordinary storage returns what was
  written, and a driver polling for sample-end sees "every voice finished" forever. The assertion
  is deliberately "not `$FF`" rather than "exactly `$00`": with no sample playing there is nothing
  to set the bits, so demanding zero would pass on a core that never implemented the register at
  all.

  Plus a global-register addressing test to sit beside the voice-register one. The two blocks are
  decoded from the same latch by different parts of the address, so a core that gets voices right
  and aliases the globals passes one and fails the other. Both write low-to-high and read back
  high-to-low, so returning the last value written cannot pass either.

- **The S-DSP is reachable, and the blocker was never the DSP.** `E3.11` (`$F2` bit 7 disables
  writing through `$F3`) and a foundational DSP register-addressing test both land, which unblocks
  `E5`-`E9` (~73 assertions).

  The cause was one bit. `E3.01` writes `$F1` to enable a timer, and `$F1` bit 7 also controls
  whether `$FFC0`-`$FFFF` reads as the IPL boot ROM or as RAM — so clearing it meant the release
  path's `JMP $FFC0` landed in zeroed RAM, the SMP wandered off, and **every APU upload after that
  test silently died**. It presented as "the DSP is unreachable" because the DSP tests happened to
  run later in the battery. `release_to_ipl` now re-maps the ROM before jumping, so a program that
  touches `$F1` for its own reasons cannot break the ones after it.

- **Correction: `E3.14` was briefly published as a Contested golden, and that was wrong.** The
  claim was that neither snes9x nor Mesen2 returns what was written to `$F8`/`$F9`, contradicting
  the documentation. All three return it correctly; the apparent failure was the IPL bug above.
  It is a Scored test again.

  Worth stating plainly because the reasoning that produced the false finding is otherwise sound:
  three-way agreement against a test really is this project's signature of a broken test. It is a
  good heuristic, not a proof — **a harness bug upstream of every implementation produces exactly
  the same signature**, and this one did.

- **The SPC700's I/O block: `E3.01` and `E3.14`, both scored.** `E3.01` pins that reading a timer
  counter returns four bits **and clears it** — `$FD`-`$FF` are not registers holding a value, they
  are counters a read consumes, and a core treating them as storage lets a driver double-count
  every tick it sees. The first read is only required to be non-zero, because how far the timer
  advanced depends on the delay loop's exact cost and asserting a count would be asserting the loop.

- **Three more SPC700 flag tests — `E1` is now 7 of 15.** `E1.04` (`DIV`'s H flag is a nibble
  comparison of the *inputs*, `(Y & 15) >= (X & 15)`, with nothing to do with any carry the
  division produces — the name is borrowed and the behaviour is not), `E1.05` (`DIV`'s V flag is
  bit 8 of the quotient, which is how a caller learns the byte it was handed is not the whole
  answer), and `E1.13` (`ADDW`'s H is the bit-11 carry, not the bit-3 one a reused 8-bit
  half-carry would give).

  Each checks both directions in one program, for the reason the earlier pair did: a flag that is
  never set passes any test that only looks for it being set.

- **Three more SPC700 tests, and the fix that makes more than one of them possible.** `E1.02`
  (`DIV YA,X` on its normal branch, the baseline every stranger `DIV` assertion deviates from),
  `E1.06` (the errata that `DIV` takes N and Z from the **quotient** alone — a zero quotient sets
  `Z` even with a non-zero remainder, and the reverse case is checked in the same program so a core
  that never sets `Z` cannot pass), and `E1.15` (`MOVW YA,dp` flags all sixteen bits, checked from
  both sides: `$0100` must not set `Z`, `$8000` must set `N`, and a core flagging the accumulator
  alone gets both wrong).

  **Every uploaded program now hands the APU back to the IPL when the cart releases it.** Once a
  program is running the boot ROM is not, so the next test's upload has nothing to handshake with.
  The first version ended in `BRA *`, and every APU test after the first silently timed out and
  then read the *previous* test's leftover port values — which looks exactly like a wrong answer
  rather than like a test that never ran. The cart now copies the results out, writes a release
  byte, and the program jumps to the IPL entry, which re-announces itself for the next upload.

- **Group E is unblocked: the APU is now reachable from the cart (T-04-E).** The SPC700 is a
  separate processor with its own RAM, and the only channel between it and the 65816 is four bytes
  — so nothing about it was testable at all. The cart now uploads a small SPC700 program through
  the IPL boot ROM's handshake (`apu_upload`), lets it run, and reads its answers back through
  those same four ports, which is exactly what a game's sound driver does at boot.

  The programs are assembled by a new `gen/src/spc.rs`, because `ca65` does not speak SPC700. It
  is deliberately minimal — one emitter per instruction a committed test actually uses, since an
  unexercised encoding is an unverified one, and a wrong byte in it would surface as an emulator
  disagreement rather than as an assembler bug.

  First test: `E1.01`, the errata that `MUL YA` takes N and Z from `Y` alone. With `Y = A = $10`
  the product is `$0100`, so `A` ends at `$00` and `Z` is nonetheless **clear**. Reading `PSW` at
  all is the recurring trick here — the SPC700 has no instruction for it, so `PUSH PSW`/`POP A`
  does the job, which only works because the results are captured first with the two moves that
  leave flags alone.

  **Every wait in the handshake is bounded.** The first version was not, and it hung the whole
  battery — reporting nothing about the other 149 tests, which is a far worse failure than one
  test standing down. `V_APU_STAGE` names the step that gave up, and a test whose APU never
  answers reports SKIP.

- **Sprites (`C7`) — three new scenes, bringing the blessed total to 41.** `C7.15` (sprite
  priority is OAM index alone, so the scene writes the two in the opposite order to the expected
  result — a core drawing in write order gets it exactly backwards), `C7.13` (the errata: V-flip
  on a tall sprite flips each 16x16 half *independently*, giving the same pixels in a different
  arrangement — which a hash separates and an eye might not), and `C7.14` (a 64-pixel sprite near
  the bottom wraps to the top rather than clipping).

  A new `scene_oam_reset` helper parks all 128 sprites at Y=224 first. OAM is 544 bytes of state
  nothing else clears, so a sprite scene would otherwise render its own sprites *plus* whatever the
  previous scene left — the same contamination the per-scene canvas rebuild exists to prevent, in
  another place it was not reaching.

- **The scroll-register write-twice latch (`C4`) — three new scenes, bringing the blessed total
  to 38.** These registers are
  write-only, so the latch is observable only through the picture. `C4.02`/`C4.03`: one `Prev`
  latch is shared across all four backgrounds *and* both axes, which shows up only when a register
  is written **once** and the next write goes somewhere else — so the scene writes one byte to
  `$210D` and one to `$210E` and reads the answer off the V scroll. A core with a per-register
  latch lands 128 rows away. `C4.01`: the H formula masks the previous byte's low three bits and
  takes them from the register instead, so `$47` then `$00` scrolls by `$40` — the asymmetry with
  the V formula, which keeps all eight bits of the same latch. `C4.04`/`C4.05`: `$210D` drives
  `M7HOFS` as well as `BG1HOFS`, through Mode 7's own 13-bit path.

- **Four more Group D tests: channel priority, indirect HDMA, and the HDMA working registers.**
  `D1.04` (a multi-channel start runs the lower channel first), `D2.05` (indirect mode fetches each
  transfer through a pointer), `D2.06` (`$4308/09` and `$430A` are live working state, so the
  counter holds the `$00` terminator once the table runs out), and `D1.03`, the DMA startup
  overhead, recorded as a golden vector.

  `D1.04` is observable only because both channels write to the same auto-incrementing port: the
  byte pair left in WRAM spells out the order the hardware chose, where timing alone could not.
  `D2.05` catches the specific failure of ignoring the indirect bit — such a core transfers the
  pointer bytes themselves, which look nothing like the data behind them.

  `D1.03` is a golden rather than a scored test because what it measures is an 8-clock startup plus
  an alignment cost that depends on where in the CPU's cycle the `$420B` write lands — exactly the
  part two implementations need not agree on to both be right. `B4.14` gets the same treatment for
  the same reason, and `D1.02` deliberately cancels it by measuring a length differential instead.

- **Group D continues: HDMA arrives, plus three more GP-DMA semantics.** `D1.05` (a byte count of
  zero means 65536), `D1.09`/`D1.15` (a WRAM source with `$2180` as the destination performs no
  write), and the first two HDMA tests — `D2.03` the line-count byte and `D2.04` the repeat flag.

  HDMA is the awkward part of the controller to observe, because it runs itself once per scanline
  with no CPU involvement. Pointing it at `$2180` solves that outright: `WMADD` auto-increments, so
  a whole frame of HDMA activity becomes a byte sequence the CPU can read back and check exactly —
  how many writes happened, in what order, and that they then stopped. The two tables differ only
  in bit 7 of their header bytes, so a core that ignores the repeat flag renders them identically
  and neither test alone would notice.

  `D1.05` is observed through **time**, not through the destination: 65536 bytes at 8 clocks each
  is ~384 scanlines, so the V counter lands far from where it started. That makes it frame-length
  dependent, so it measures the frame height and branches on what it measured — never on the region
  bit, whose position `B2.10` had to settle and which a frame-length test must not lean on.

- **Group D opens: seven general-purpose DMA tests (T-04-D).** Transfer modes 0 and 1, the byte
  counter reaching zero, both non-incrementing A-bus steps, the undocumented `$43xB` scratch latch,
  and the 8-clocks-per-byte rate as a length differential so startup overhead cancels. These are
  self-scoring on-cart — DMA moves bytes into memory the CPU can read back, which is why the group
  leads with behaviour rather than with timing.

  Three of the seven were **wrong on their first run and both emulators said so**, which is the
  signature of a broken test rather than a broken core. The interesting one: `$4300` bits 4-3 are a
  two-bit *field* (0 = increment, 1 = fixed, 2 = decrement, 3 = fixed), not two independent flags —
  the exact confusion `D1.07`/`D1.07b` exist to catch, made while writing them. They are declared
  as a split so neither can be deleted as a duplicate of the other.

- **Mode 7 scenes (`C11`, plus `C5.08`/`C10.05`/`C12.03`) — 35 scenes total.** Identity and
  rotate/scale transforms, all three screen-over modes, screen flip, EXTBG, direct colour, mosaic
  and windowing. A shared `scene_mode7_vram` helper lays down the tilemap and character data in one
  pass, which is possible precisely because Mode 7 interleaves them by byte (`C11.05`) — and is the
  cheapest way to be sure the two halves cannot drift apart.

  `scene_canvas` now fills **all 256** CGRAM entries rather than 128. An 8bpp or Mode 7 scene
  indexes the whole palette, so the upper half was being rendered through whatever the previous
  scene or test had left there — the same cross-scene contamination the per-scene canvas rebuild
  exists to prevent, in the one place the rebuild was not reaching.

- **Offset-per-tile scenes (`C6.01`-`C6.06`) plus a mode-2 control — 25 scenes total.** Mode 2's
  BG3 stops being a layer and becomes a table of per-column offsets; the scenes pin the enable bits
  (13 = BG1, 14 = BG2, as a *pair* — neither alone can tell a core that swapped them), mode 4's
  bit-15 H/V selector, that a V entry replaces the background's own scroll while an H entry keeps
  its low three bits, that each entry moves a whole tile column, and the errata that the leftmost
  tile is never affected. All three emulators agree bit-for-bit.

  Getting there took three rounds against scenes that rendered pictures no assertion could show,
  and none of them would have been caught by cross-validation — an unshowable scene hashes stably
  and the references agree with it. `scene_low_tiles` now picks tiles `$10`-`$1F` (a 4bpp tile
  spans two font glyphs and an 8bpp tile four, so anything lower lands on the blank ASCII control
  characters — which made mode 2 render empty while mode 4 looked fine) and varies the tile with
  the row as well as the column. The offsets moved from 64 to 100 rows, because 64 is a multiple of
  both 8 and 16 and so is invisible against a 16-tile cycle no matter what the map contains.

- **`C13.01`-`C13.06` are recorded as blocked, deliberately, rather than covered.** They are the
  INIDISP early-read artifacts — a one-dot display flash, a one-dot brightness step, a ~72-pixel
  brightness ramp — and they are blocked twice over. The compositor still paints a whole scanline
  from one register snapshot (v0.8.0 moved *when* a line is composited, not the granularity), so a
  sub-scanline effect cannot be rendered; and they are chip-revision-dependent (`C13.01` 3-chip,
  `C13.05` 1CHIP, gated by `C14.02`), so a golden would commit to one revision as though it were
  the behaviour. The second blocker survives fixing the first, and unlike a reference disagreement
  it would never announce itself. `B2.09`'s entry is corrected the same way.

- **Fifteen more rendered scenes (T-04-H) — 18 total, all cross-validated.** Mode 0 four-layer
  priority and palette segregation, Mode 3's 8bpp BG1, the tilemap flip bits, 16x16 tiles, colour
  math in subtract mode and at saturation, five window scenes (inclusive bounds, crossed bounds,
  inverted-empty, both-windows-disabled, XOR combination), screen-anchored mosaic, and direct
  colour. RustySNES, snes9x and Mesen2 agree bit-for-bit on every one — no divergence this time,
  which is the expected outcome now that the background-fetch and mosaic-anchoring bugs the first
  batch exposed are fixed, since both sit upstream of most of what these render.

  Three of them assert **equivalences** rather than numbers, and the harness gates on those
  separately: `c8-half-ignored-on-fixed-backdrop` must hash identically to `c8-fixed-colour-add`
  (CGADSUB's half bit is ignored when the subscreen is the fixed backdrop, `C8.03`), and
  `c8-window-left-gt-right-empty` must equal `c8-both-windows-disabled-empty` (crossed bounds and
  no enabled window are both *empty* masks, not full ones, `C8.05`/`C8.07`). An equivalence is the
  stronger statement: it survives a change to the canvas, and it catches a core that gets both
  scenes wrong the same way — which two independent hash comparisons cannot.

  Scene coverage now appears in `docs/accuracysnes-coverage.md` as its **own column**, never added
  into the on-cart figure: an on-cart result means the same thing on any emulator and on real
  hardware, a rendered scene needs a host holding the golden, and one number cannot mean both. A
  scene naming an assertion the dossier does not enumerate now fails the build, the same gate the
  battery already had.

**AccuracySNES totals, as of this section:** **220 tests — 208 scoring at 100.00%, 11 golden
vectors**, plus one region-dependent SKIP per image, and **50 rendered scenes** in the host
framebuffer-oracle tier. Dossier coverage is **178 of 443** on-cart plus **50** scene-only —
**228 of 443** in total, and **every group A-G now has shipped tests**
(`docs/accuracysnes-coverage.md`, regenerated with the ROM). The per-entry
"Battery now N" tallies below are each batch's state *as it landed*, kept as written rather than
rewritten to the current number — this line is the one to read.

- **AccuracySNES ships a PAL image, and it settled a contested assertion.** "This needs a PAL
  console" was only half true: a console's region fixes the timing, but which timing an emulator
  boots is decided by the cart header's country code. The generator now emits
  `build/accuracysnes-pal.sfc` by patching one header byte of the linked NTSC image and recomputing
  the checksum, so the two images are provably identical apart from the region — any behavioural
  difference between them is the region and cannot be anything else. `B2.04` (262 lines) and the new
  `B2.05` (312 lines) are mirrors, each standing down as SKIP on the machine it does not describe,
  and the skip predicate is the *measured* frame height rather than the region bit, because a
  frame-height test must not depend on the thing it is evidence for. **`B2.10` is settled**: the bit
  that moves between the two images is **bit 4**, so fullsnes is right and the SNESdev wiki's bit 3
  is wrong — settled by measurement rather than by picking a source.

- **`B4.14`: interrupt dispatch latency, measured (golden).** The dossier's claim — the poll occurs
  just before the final CPU cycle — is sub-cycle and not CPU-observable; the finest clock a cart can
  read is the H counter at four master clocks per dot, and reading it costs more than the interval.
  So the cart measures the consequence: with its own IRQ handler installed through a new
  RAM-indirect IRQ vector, it times handler entry while spinning on `NOP`s and again while spinning
  on `JSL`/`RTL`. **The three references split on the sign** — RustySNES +3 dots, snes9x +2, Mesen2
  −2 — which is exactly why it is recorded rather than scored. The
  libretro cross-validation host dumps the whole measurement channel so any golden timing vector is
  comparable across emulators.

- **The AccuracySNES framebuffer oracle is live (T-04-H, `docs/adr/0013` ratified).** Part of the
  PPU decides only what appears on screen, so a self-scoring cart cannot reach it — there is no path
  from rendered pixels back to the CPU. The cart now renders and the *host* judges: after the
  battery, a scene loop sets up PPU state, settles, publishes a scene ID, and holds it, and three
  independent hosts hash a fixed 256x224 region of canonical pixels and compare against
  `tests/golden/accuracysnes-scenes.tsv` — the in-repo harness, snes9x through the libretro host
  (`--scenes`) and Mesen2 through `mesen_scenes.lua`. Rendered results are reported in their own
  tier and **never** enter the on-cart pass rate, because a rendered scene lacks the property the
  rest of the battery has: that the identical image means the same thing everywhere. Per ADR 0013
  rule 4 a golden is blessed only from a cross-validated render, and `crossval.sh` now gates on
  that. Three scenes to start (mode 1 priority, fixed-colour math, mosaic); all three found real
  bugs, see Fixed.

- **AccuracySNES Group B, second batch — 6 tests.** `B1.03` an internal cycle costs 6 master clocks
  (isolated by differencing `XBA` against `NOP`, which differ by exactly one internal cycle and are
  both single-byte). `B1.04` DMA runs at a uniform rate **regardless of source region** — a
  differential between a `MEMSEL`-fast bank and a slow one, which catches the natural bug of reusing
  the CPU's speed map for DMA. `B4.09` an HV-IRQ requires **both** comparators to match, not either.
  Plus three golden vectors: the `$213F` region-bit encoding (the sources conflict on bit 3 vs
  bit 4), the interlace line count, and the H-IRQ position. Battery now **132 tests, 123 scoring,
  100.00%, 9 golden**.

- **AccuracySNES: the opcode sweep now covers every inline-measurable class (34 entries), plus
  `A9.03`.** The sweep adds direct page, absolute, absolute long, indexed, read-modify-write,
  stores and untaken branches to the implied/immediate/stack set. `A9.03` settles — or rather,
  records — WDC's single-vendor note (17) on emulation-mode read-modify-write, by probing `$2104`
  whose address counter auto-increments per write and so counts writes directly. **The emulators
  split three ways**: Mesen2 sees two writes (WDC's note holds), RustySNES and snes9x see one.
  Recorded as a golden vector rather than scored, since the one document asserting it is the one
  two other vendors decline to corroborate. Battery now **126 tests, 120 scoring, 100.00%,
  6 golden**.

- **AccuracySNES: the opcode cycle sweep runs (T-04-I) — 22 tests.** A safe-operand table and
  sandbox in `gen/src/tests/sweep.rs`, emitting one test per opcode so a failure names the
  instruction rather than the batch. Expectations are derived from `clocks = 6*cycles + 2*mem`
  against cycle counts the WDC, GTE and VLSI instruction-operation tables all agree on — no
  emulator anywhere in the chain. Covers the opcodes whose operands and safety are unambiguous:
  implied, immediate at `m=1`/`x=1`, and balanced push/pull pairs. Battery now **113 tests, 108
  scoring, 100.00%, 5 golden**.

- **A full-width measurement channel for AccuracySNES.** A test's verdict is one byte, which cannot
  carry a dot count — a value above 255 wraps and becomes indistinguishable from a real reading.
  Timing tests now write raw measurements to `$7E:E200` (64 `u16` slots) and the harness prints and
  sanity-checks them, bounding both the physical floor and the 341-dot scanline wrap. T-04-I's
  256-opcode sweep needs this regardless: it produces 256 numbers and the status array has nowhere
  to put them.

- **T-04-J: coverage is now measured instead of estimated.** `gen/src/dossier.rs` maps every cart
  test to the dossier assertion(s) it implements, because the two numbering schemes look identical
  and are not — cart `A1.04` is dossier `A1.06`. The generator refuses to build if a test is
  unmapped, if an assertion is claimed by two tests without a declared reason, or if a test maps to
  nothing without a justification; both failure modes were verified to actually fire. The mapping
  is emitted as a `dossier` column in `SOURCE_CATALOG.tsv` and re-checked by the harness against
  the committed artifact, and `docs/accuracysnes-coverage.md` is regenerated with the ROM.

- **The dossier's 23 prose sub-groups are now per-ID tables.** Content preserved verbatim, only
  restructured, plus `E10` which had been missed. The enumeration goes from 232 checkable
  assertions to **443** across all 43 sub-groups, so the coverage report is a *complete* statement:
  an assertion with no test is listed there by name. Previously coverage could only be reported for
  whichever assertions happened to sit in a table — which is exactly where an untested behaviour
  could hide. Current coverage: **79 of 443**.

- **The AccuracySNES research corpus is in the repository.** The 938-line hardware-behaviour and
  test-list design report that `docs/accuracysnes-research-dossier.md` distils was cited at a path
  under `~/.claude/`, outside version control. It is now
  `ref-docs/2026-07-19-accuracysnes-hardware-test-design.md`, under the immutable-corpus rules in
  `ref-docs/README.md`.

- **AccuracySNES opens Group B — the 5A22 (T-04-B, first batch, 9 tests).** Memory access speed:
  `MEMSEL` switching banks `$80`+ between 8 and 6 master clocks (measured through a long read so
  the timed access is the subject, while the measuring loop keeps running from always-slow bank
  `$00`), and the joypad ports being the slowest region on the bus at 12 clocks against CPU MMIO's
  6. `RDNMI` mechanics: bit 7 setting at vblank *independently of whether NMI is enabled* — the
  flag tracks the event, not the interrupt — and clearing on read, split into two tests because
  the failure modes are opposite. The multiply/divide unit: 8x8 unsigned multiply, 16/8 divide
  with the remainder sharing `RDMPY`, and divide-by-zero saturating to `$FFFF` with the dividend
  left as the remainder. Plus two golden vectors: the CPU revision nibble, and the **undefined**
  mul/div overlap, which the SNESdev Errata explicitly declines to define and which is therefore
  recorded rather than asserted.

- **`docs/accuracysnes-plan.md` — the AccuracySNES phase plan**, plus follow-on tickets
  **T-04-A**–**T-04-J** in `to-dos/ROADMAP.md`. Frames the ~235 remaining tests by *what blocks
  them* rather than by group: reachable now (Groups B, G, the rest of register-observable C, the
  rest of A), needs its own mechanism (D's research top-up, E's on-cart APU harness), cannot be
  fully self-scoring (F — a cart cannot press its own buttons), and needs a framebuffer oracle
  (the renderer-dependent rest of C, which would break the property that the same image runs on
  real hardware). Also records the constraints worth settling before the affected group starts.

- **AccuracySNES: three Group A gaps closed (T-04-A, first batch).** `A1.06` — `TCD`/`TDC` move
  all 16 bits regardless of `m`, so an 8-bit accumulator must not narrow a register that has no
  8-bit form. `A5.07` — read-modify-write `abs,X` pays a flat cost with no page-cross penalty,
  measured 8-deep against the same instruction without a cross. `A6.09` — `BRK` sets the `B` flag
  in the status byte it pushes, which in emulation mode is the *only* thing distinguishing a
  software `BRK` from a hardware IRQ arriving at the same `$FFFE`. Battery now **76 tests, 73
  scoring, 100.00%, 3 golden**. Battery after both batches: **85 tests, 80 scoring, 100.00%, 5 golden**.

- **AccuracySNES Group B continued (T-04-B): frame geometry and the IRQ timers — 4 tests.**
  `B2.04` — an NTSC frame is 262 lines, sampled by polling `OPVCT` from vblank until the counter
  wraps and keeping the maximum, so it measures the counter rather than trusting a constant.
  `B4.05` — `RDNMI` auto-clears at the end of vblank, the counterpart to `B4.04`'s clear-on-read.
  `B4.08` — a V-IRQ fires on the programmed scanline, armed with interrupts masked and observed by
  polling `$4211`, so it measures the comparator without depending on interrupt dispatch.
  `B4.12` — reading `$4211` releases the latch immediately, asserted with a second read *on the
  same scanline*. Battery now **90 tests, 85 scoring, 100.00%, 5 golden**.

- **AccuracySNES `A5.08` — the `A5.22` cycle spot checks, as a golden vector, and the measured
  blocker for T-04-I.** Converts cited cycle counts into measurable time via
  `clocks = 6*cycles + 2*mem` (`mem` = instruction length plus data/stack accesses) — the term a
  naive "cycles x constant" conversion misses, and why `NOP` and `LDA #imm`, both 2 cycles, do not
  cost the same. Written scored first; it failed on **all three** references at **different**
  sub-assertions (snes9x on `XBA`, RustySNES on `REP`), which is the signature of the references
  disagreeing with each other rather than of a broken test. It therefore reports a bitmask of which
  expectations matched — RustySNES `101`, snes9x `100` — and stays out of the pass rate. The
  consequence for the planned 256-opcode sweep is recorded in `docs/accuracysnes-plan.md`: the
  blocker is sourcing an **external** per-opcode timing table, not writing the sweep.

- **AccuracySNES: pre-`init_registers` power-on sampling (T-04-G prerequisite).**
  `capture_power_on` runs at the top of reset, before `init_registers` puts every register into a
  known state, and stashes what it samples in a documented WRAM capture block. Without it no
  power-on test can exist: the runtime deliberately erases exactly the state such a test wants.
  This unblocks all 18 Group G assertions. `B5.05` is the first consumer.

- **AccuracySNES `B5.05` — the multiply/divide power-on latches.** `$4202` powers up `$FF` and
  `$4204/05` `$FFFF`. Both are write-only, so the test observes the latch *through the unit it
  feeds*: start a multiply without writing `$4202` and the product is `$FF x N`.

- **The rendered-scene gate had become intermittently red, and the cause was the oracle, not the
  emulators.** Two separate faults, both surfaced by the battery growing longer:

  1. Mesen2'''s report block could run on more than one frame before `emu.stop` took effect, printing
     the whole scene list twice. It stayed hidden while only one frame elapsed; a longer battery
     made it two, and the duplicated list read as a scene mismatch rather than as a duplicated
     report.
  2. The capture window (4 settle frames, 4 published) was too tight once the phase between the
     cart'''s vblank polling and a host'''s frame callback shifted. "Take the second sighting"
     occasionally landed on a transition frame, and the gate failed on a different scene each run.
     Widened to 6 and 8, capturing the fourth — all 41 goldens are unchanged, which is the evidence
     that the window moved and the steady state did not.

  An intermittently-red gate is worse than a slow one, because it gets ignored.

- **A WRAM source with `$2180` as the DMA destination no longer writes.** It is a WRAM-to-WRAM
  transfer through the data port, and the hardware performs no write at all — the read still
  happens and the time is still spent. RustySNES copied the bytes, which looks right until a game
  relies on the no-op. snes9x passes `D1.09`; RustySNES did not.

  Worth noting for anyone touching this code: GP-DMA and HDMA have **separate** transfer paths
  (GP-DMA interleaves HDMA and accounts clocks per byte), so a rule belonging to the transfer
  itself has to be stated in both. Fixing only `transfer_unit` left the test still failing.

- **The DMA `$43xB` scratch latch is now modelled**, mirrored at `$43xF` and per-channel. It is
  undocumented storage the controller never reads, but it is CPU-visible: RustySNES returned 0 from
  both addresses while snes9x returned what was written, which is how `D1.10` found it. It is
  deliberately **not** added to the save state — ares and bsnes serialise theirs, but changing the
  `DMA0` section's length is a format-version decision (`docs/adr/0006`) and the latch has no
  effect on emulation; the reasoning is recorded rather than left implicit.

- **Mode 7 rendered one scanline low.** The identical off-by-one this release already fixed in the
  tiled backgrounds — `render_mode7` is a separate function and was missed. Nine of the ten new
  Mode 7 scenes moved on that one line, and all nine then matched both references exactly.

- **EXTBG replaced BG1 instead of adding a layer.** Mode 7 has one background; EXTBG splits it by
  the pixel's high bit into BG1 (full 8-bit palette) *and* BG2 (seven bits of colour, bit 7 as a
  priority selector). RustySNES rendered one or the other, so enabling EXTBG made BG1 vanish.

- **Mode 7 ignored mosaic entirely**, rendering identically with and without it — `render_mode7`
  had no mosaic handling at all. Quantised in screen space, matching the tiled path.

  All three were found by the framebuffer oracle and confirmed the same way: snes9x and Mesen2
  agreed bit-for-bit with each other and disagreed with RustySNES, which is the signature of a real
  bug rather than a broken test. The 29 undisbeliever goldens are unaffected (none use Mode 7).

- **Backgrounds were rendering one scanline low.** The first displayed line must show BG row
  `BGnVOFS + 1`, not row `BGnVOFS` — the vertical fetch runs a line ahead of the line it appears on.
  `render_bg` derived its BG row from `self.v - 1`, the same number as the framebuffer row; the two
  are deliberately different and now are. Found by the framebuffer oracle's first scene and
  confirmed against two independent references, snes9x and Mesen2, which agree bit-for-bit with the
  corrected output and disagreed with the old one.

- **Mosaic was anchored to the background instead of to the screen.** A mosaic block belongs to a
  fixed grid at the top of the picture, so scrolling moves content *through* the grid; RustySNES
  quantised the already-scrolled BG row, dragging the grid along with the scroll. Now quantised in
  screen space and converted back. Found by the same oracle, same two references.

  **Re-bless note.** These two fixes moved 25 of the 29
  `tests/golden/undisbeliever-framebuffer.tsv` entries, which is the expected consequence rather
  than a red flag: those goldens were blessed from our own output and so recorded the bug instead of
  catching it — the hazard `docs/scheduler.md` records from the `hdmaen_latch_test` re-bless.
  Independent evidence for the re-bless: hashing the same 29 third-party ROMs on snes9x through a
  new `--fb-after=N` mode of the libretro host, agreement went from **2/29 to 14/29**. The
  remaining 15 are the HDMA and S-CPU A-bus DMA glitch ROMs, which is separate work.

- **The multiply/divide latches now power up as `$FF` / `$FFFF`.** They were defaulting to zero.
  Asserted rather than merely recorded on two independent documentation lineages that agree with
  nothing contradicting them in nineteen years — anomie's `regs.txt` r1157 (*"$4202 holds the value
  $ff on power on and is unchanged on reset"*, in a document that marks its uncertain claims with
  `(?)` and marks neither of these) and nocash's fullsnes (`$4202`-`$4206` listed `(FFh)` at
  power-up) — and implemented by bsnes, ares and Mesen2. **snes9x diverges** (its
  `S9xSoftResetPPU` blanket-`memset`s `$4200-$42FF` to zero), which is a snes9x bug rather than
  counter-evidence; the divergence is declared explicitly in `scripts/accuracysnes/crossval.sh` so
  the cross-validation gate keeps its teeth instead of being weakened to unanimity. No hardware
  test ROM is known to verify this, and the provenance string says so.

- **`RDNMI` now auto-clears at the end of vblank.** The flag was cleared only by a read, so it
  stayed set through the whole active display and code polling `$4210` outside vblank saw a vblank
  that had already ended and acted a frame late. Found by AccuracySNES `B4.05`.

- **A V-only IRQ no longer re-asserts on every dot of its scanline.** With H-IRQ disabled the
  horizontal half of the comparator was treated as unconditionally matching, which made
  `V == VTIME` a *level* held across all 341 dots: acknowledging via `$4211` was undone a few dots
  later, and a V-only handler saw a storm instead of one interrupt per frame. The comparator is now
  sampled at a single dot (`VIRQ_TRIGGER_DOT`, the documented `H ~ 2.5`). ares reaches the same
  behaviour from the other direction — its `irqValid.raise(...)` is an *edge* detector. Found by
  AccuracySNES `B4.12`, with `B4.08` pinning the firing line.

### Added

- **`C5.14` — 2/4/8bpp bitplane layouts, as a rendered scene (`docs/adr/0013`).** A custom 4bpp tile
  fills the screen, its four bitplanes each carrying a distinct horizontal stripe: plane 0 -> colour 1
  (rows 0-1), plane 1 -> colour 2 (rows 2-3), plane 2 -> colour 4 (rows 4-5), plane 3 -> colour 8
  (rows 6-7). A 4bpp tile is two 2bpp halves — planes 0/1 in the first 16 bytes, planes 2/3 in the
  next 16 — so it reads out as four separately-coloured bands only if the decoder pairs the right byte
  offsets to the right planes. Font tiles cannot test this: they carry nothing in planes 2/3, so a
  core that mis-pairs the high half renders them identically; here a swapped pairing recolours the
  lower two bands. Verified by injecting a plane-pair swap into `read_planar` and watching the scene
  hash flip `0xf11def7a…` -> `0xf7ba0cdf…` while every other check stayed green. Blessed after all
  three cores agreed on `0xf11def7a…`, distinct from every existing golden. (The scene oracle is fixed
  at 256x224 non-hi-res, so it deliberately does not claim the neighbouring `C5.15` "modes 5/6 use
  16-px-wide tiles", which only a hi-res render could reach.)
  Coverage: 52 -> 53 rendered scenes (332/443 total; on-cart stays 279).
- **`C8.12` — CGWSEL force-main-black field, as a rendered scene (`docs/adr/0013`).** CGWSEL bits 7-6
  select where the main screen is forced black (never / outside / inside / always the colour window).
  The scene sets `01` (black outside the colour window at columns 64..191): BG1 is on everywhere, so
  the font shows only in that central band and the sides are blacked out by the output stage — not by
  clipping the layer. A core that reads the field inverted blacks the band instead, and one that ties
  force-black to colour math being enabled shows nothing black (no CGADSUB here). Blessed after all
  three cores agreed on `0x21a5b8a8…`, distinct from every existing golden.
  Coverage: 51 -> 52 rendered scenes (331/443 total; on-cart stays 279).
- **`C7.03` — sprite H-flip sliver order, as a rendered scene (`docs/adr/0013`).** A 16x32 sprite two
  8-pixel slivers wide (distinct font glyphs `$10`/`$11`) with the H-flip attribute set: H-flip swaps
  which sliver appears on the left and mirrors each glyph, but the slivers are still emitted
  left-to-right across the screen. A core that reverses the sliver *output* order, or mirrors the
  sprite as a whole without re-fetching per sliver, hashes differently. Blessed only after all three
  cores agreed on the pixels (`0x863f085b…`), and its hash is distinct from every existing golden.
  Coverage: 50 -> 51 rendered scenes (330/443 total; on-cart stays 279).
- **`D1.13` — the GP-DMA byte-count register decrements to zero (general-purpose DMA).** A DMA runs
  until its count reaches zero, so `$43x5/6` is spent by the end and reads `$0000`, not the programmed
  size. The test reads the count both **before** and **after** a four-byte mode-0 transfer and asserts
  the difference is exactly `4`: a paired control that a bare "reads `$0000`" assertion lacks, because
  a core that never exposes the count (`$43x5` reads a constant `$0000`/open bus) reads zero both times
  and would pass vacuously. The before-read holds the register to the programmed size and the delta
  proves it decremented — separating a working count register from one that never decrements (reads
  `$0004` both times) *and* from one that never exposes it. Verified both ways: suppressing the
  write-back in `run_gp`, and forcing `$43x5` to read constant zero — each makes D1.13 alone fail.
  Cross-validated on snes9x and Mesen2 (both read `$0004` before, `$0000` after).
  Coverage: 278 -> 279 on-cart assertion rows (329/443 with rendered scenes).
- **`E7.18` — `VxENVX` is the envelope shifted right four, bit 7 always clear (S-DSP).** The
  eleven-bit envelope exposes bits 10-4 through `ENVX`, so it tops out at `$7F`. Probed at direct gain
  `$40` — envelope `$400`, the value that separates every candidate shift at once: `>>4` reads `$40`,
  `>>5` reads `$20`, and `>>3` reads `$80` (bit 7 set, which eleven bits can never produce). Asserting
  exactly `$40` pins the shift at four and confirms bit 7 stays clear in one read, and it is the only
  ENVX test that catches a `>>3` shift — verified by injecting exactly that. Cross-validated on snes9x
  and Mesen2. Coverage: 277 -> 278 on-cart assertion rows (328/443 with rendered scenes).
- **`E5.13` — a voice never stops decoding BRR (S-DSP).** Key-off releases the envelope but the BRR
  decoder advances on the pitch clock regardless, so a released, silent voice still reaches its end
  blocks and re-sets `ENDX`. The voice loops a sample and is keyed off; slot 240 records `ENVX = $00`
  (the guard that it is genuinely silent, not merely a voice that never started), then `ENDX` is
  cleared and re-read at slot 241 — voice 0's bit reads set because the released voice kept looping.
  A core that halts a silent voice's decoder leaves it clear and fails; verified by injecting exactly
  that halt into `voice4`. The assert masks to voice 0's bit because other voices carry leftover
  `ENDX` bits from earlier tests. Cross-validated on snes9x and Mesen2.
  Coverage: 276 -> 277 on-cart assertion rows (327/443 with rendered scenes).
- **`E8.05` — KON is write-triggered and non-persistent (S-DSP key-on).** A voice is keyed on with
  `$4C` bit 0 and the register bit is deliberately **left set**; a correct DSP keys on once and, under
  direct gain, its envelope escapes the five-sample key-on delay and `ENVX` reads `$7F`. A core that
  re-consults the held bit every poll perpetually restarts the key-on delay, the envelope never leaves
  zero, and `ENVX` reads `$00`. The reading is self-guarding — `$7F` can only mean keyed-on-then-not-
  re-keyed. `ENDX` cannot serve here (end always jumps to the loop pointer and re-crosses the end
  block, so a merely-playing voice re-sets it regardless of a re-key). Verified by injecting a level-
  sensitive KON into `misc30` and watching the row fail; cross-validated on snes9x and Mesen2.
  Coverage: 275 -> 276 on-cart assertion rows (326/443 with rendered scenes).
- **Two more S-DSP envelope rows — `E7.13` and `E7.17` (S-DSP GAIN).** `E7.13` pins GAIN mode 7
  (bent-increase): the envelope climbs `+32`/sample below the internal `$600` break and `+8` above
  it, read back through `ENVX`; a core that ignores the break saturates at `$7F` instead of landing
  in the `$68`-`$7C` slow region. `E7.17` pins GAIN mode 4 (linear-decrease) clamping at zero on
  underflow: the voice is parked at full scale under direct gain (a guard slot records `$7F`), then
  switched to linear-decrease and driven through zero — `ENVX` must read exactly `$00`, where a core
  that wraps `-$20` instead of clamping reads near `$7E`. Both verified by injecting the bug (removing
  the `$600` break; wrapping the clamp) and watching the row fail; both cross-validated on snes9x and
  Mesen2. Coverage: 273 -> 275 on-cart assertion rows (325/443 with rendered scenes).
- **The results menu scrolls and can re-run tests and restart the battery.** Up/Down move the
  cursor and scroll the 26-row window; **A** re-runs the highlighted test through the same dispatch
  the batch uses, rewriting only its verdict; **Select** restarts the battery from `restart_entry`
  (after the power-on capture, so it does not re-read power-on registers as garbage). Select rather
  than Start because `PAD_CONTRACT` holds Start, so every menu action button avoids the four
  contract buttons. `F1.07` stands down as SKIP on a restart — its phase A needs the power-on value
  of `$4218`, which a soft restart cannot reproduce — gated on a new `V_RESTARTED` flag.
- **Group F skips when the host is not holding the input contract.** The six contract-dependent
  tests asserted against buttons nobody was holding when the cart ran outside the cross-validation
  harness — six FAILs on a real emulator or on hardware with an untouched pad. A test that depends
  on host configuration must detect its absence and SKIP; `f1_require_contract` is that guard.

- **A host input contract, unblocking Group F.** Every runner — the in-repo harness, the snes9x
  libretro driver, and the Mesen2 test-runner script — now holds controller 1 at `PAD_CONTRACT` =
  `$9050` (B + Start + X + R) for the whole run. Group F was otherwise almost entirely untestable:
  with nothing held, every controller observable the cart can reach is `$0000`, so a test of the
  read order or of what a disarmed auto-read preserves has nothing to distinguish from anything
  else. The value is chosen to use no d-pad (the post-battery menu scrolls on Up/Down), to put bits
  in both bytes (so a host reporting one half is visibly wrong), and to be asymmetric under bit
  reversal (so an LSB-first read cannot pass by accident). Documented in `runtime.inc`.

- **`F1.01` — is the manual read order `B Y Select Start Up Down Left Right A X L R`?** Sixteen bits
  clocked out of `$4016` MSB-first must equal the held mask. The order is what every hand-polling
  driver depends on, and a core with it wrong produces a game where the buttons are simply the wrong
  buttons — obvious to a player, invisible to a test that only checks whether *something* was
  pressed. The reads are open-coded rather than calling the runtime's `read_pad`, so what is
  asserted is the hardware's order and not the runtime's agreement with itself. Verified by making
  the shift register LSB-first, which fails it.

- **`f1_14`'s doc comment restored, and the module doc rewritten.** The `F1.07` withdrawal earlier
  in this session removed a span ending at the *last* line of the following function's doc block,
  leaving `f1_14` with a one-line `/// judged.` and its `$4213` explanation deleted — and it
  compiled, linted clean under `missing_docs`, and shipped in four commits before a stray `grep`
  surfaced it. The module doc had gone stale in the same way: it still said Group F could reach
  nothing depending on what is plugged in, and that `NMITIMEN` is zero for the whole battery, both
  of which the input contract changed.

- **The measurement channel widened to 512 slots**, and its index type from `u8` to `u16`. It stood
  at 240 with **25 free** against the 130-150 tests still to write — about ten tests of headroom, so
  this one was deliberate rather than forced by the collision gate. The type change is why it is not
  simply a larger number: 255 was the real ceiling, and `$7EE200` has room to `$7EF000` for 1,792.
  Widening touches four places (`runtime.inc`, the generator, the harness, the libretro
  cross-validator) plus the sweep's two slot blocks, and `runtime.inc` now says so.

- **The SPC700 program images now pack across two banks.** They total ~22 KiB over 106 programs and
  grow ~240 bytes per APU test, so one 32 KiB bank runs out partway through finishing Group E — and
  a segment cannot span a bank boundary. `apu_upload` already reads them through a 24-bit pointer,
  so which bank a program lands in is invisible to every test. Verified by forcing a split with a
  reduced budget and confirming the battery still passes 284/284 with programs in two banks; at the
  real budget the mechanism is inert, and inert machinery that has never run is worth nothing.
  **Deduplication was measured and rejected.** Across the 106 programs the longest common prefix is
  **0 bytes** — `data_first` puts each test's sample at the head, so programs diverge immediately —
  though 28 share a 128-byte prologue and a shared library would save ~4,864 bytes (22%). It was
  rejected on failure mode, not size: a library at a fixed APU RAM address that a later program
  overwrites yields a *wrong measurement* rather than a crash, on a cart whose purpose is to be
  believed about small numbers. Splitting removes the ceiling outright and risks nothing.

- **`CATALOG` moved out of bank `$00`** (to bank `$04`), which takes bank `$00` from 3,268 bytes
  free to **11,686**. It was the last movable thing there and it grows ~27 bytes per test, so it was
  the segment that would have hit the wall first.
  Every access to it was already long-addressed; what had to change was `draw_str`, which read
  length-prefixed strings with `lda a:0,y` — through `DBR`, so it could only ever see one bank. It
  now reads through a 24-bit `V_STR_PTR` and no longer cares where strings live, with the catalog's
  bank taken from `^_test_names` so it follows the segment automatically. The two header strings go
  through a `str_ptr_bank0` helper that says out loud which bank it assumes.

- **A gate on the interactive menu**, which until now was drawn by code *no check observed* — the
  battery reports through WRAM and scenes draw their own tilemaps. It asserts the title row renders
  after a full run. Getting it right took two corrections worth recording: `draw_screen` runs after
  `run_scenes`, not after the battery, so waiting on the battery's sentinel lands mid-scene-loop with
  the tilemap still holding a scene; and the run needs its own frame budget, larger than
  `MAX_FRAMES`, because it is the only check that waits for the whole cartridge.
  It also found a **pre-existing defect**, since fixed (see *Fixed*): `draw_list` did not render at
  all and the tally digits read `000` where the battery passed 284. With that fixed the gate now
  asserts a test name renders — which is what gives the catalog-bank branch of the new `draw_str` its
  runtime evidence — and that the rendered tally matches the results block it is drawn from, a
  comparison against the machine rather than a remembered number, so it never needs re-blessing.

- **The build now reports per-bank headroom and fails before a bank runs out.** A segment overflow
  is an `ld65` error with no warning beforehand, it lands mid-change rather than at a moment anyone
  chose, and it names a *segment* when the fix is always to move a different one — it has happened
  four times. The generator now parses its own map file after linking and prints how much room each
  bank has left, failing at **512 bytes free** so the build breaks while there is still space to
  land the change in hand and move something afterwards. Same reasoning as `dossier::check_slots`,
  which prints the free list rather than merely refusing.
  Two arithmetic mistakes in the gate itself, both caught by the numbers being obviously wrong:
  `(bank << 16) | 0x1_0000` for the bank end is only correct on *even* banks, and bank `$00`'s
  `HEADER`/`VECTORS` are pinned at the top by the hardware rather than stacked after the growing
  segments, so counting them as "last" made the bank read as full when what matters is the gap
  below them.

- **`APUDATA` given a bank to itself** (`$06`). The SPC700 program images grow at ~240 bytes per APU
  test and Group E holds the most uncovered rows left, so after `TESTSE` this is the next segment a
  single bank will not hold. `apu_upload` reads them through a 24-bit pointer, so the move costs
  nothing. Headroom after it: `$00` 3,268 B (~122 tests), `$02` 13,570 B (~63 E tests), `$06` 11,121
  B (~45 APU tests).

- **Banks re-packed, one segment per group where it matters.** `TESTSE` has bank `$02` to itself —
  it is the largest segment and Group E holds the most uncovered rows, so it is the one that will
  need splitting first — and `TESTSD` moved to bank `$05`. `G1.11`, which walks the whole cartridge
  byte by byte, now sweeps eight banks rather than four.
  **Groups A and B were *not* moved, and that is deliberate**: `A4.11`/`A4.12` aim an `(a,X)`
  indirect jump at the *program bank's* signature block to tell an in-bank wrap from a bank carry,
  and Group B's access-speed rows depend on where they execute. Relocating either would leave the
  tests passing while measuring something else. The build gate caught the attempt — `A4.01`'s
  `jmp ($12FF)` is its subject, not a call to the runtime — which is the gate working as designed.
  So bank `$00` keeps ~18 KiB of bodies it cannot shed, and the next lever there is `CATALOG` (8.5
  KiB), which every access already reaches long-addressed; the blocker is that `draw_str` reads name
  strings through `DBR`, so moving it needs the data bank managed around the menu's draw loop.

- **Every host's frame budget raised together.** The in-repo harness (600 → 1500),
  `mesen_scenes.lua` (2000 → 4000) and `libretro_crossval.c` (1200 → 2400) all bound the same run.
  `G1.11` walks the entire image, so doubling the cartridge doubled it — about 320 of the battery's
  431 frames are that one test — and Mesen2's scene run stopped reaching the scene loop. It reported
  **"0 scenes match"**, not a timeout, which reads as a mismatch in the gate's output and cost a
  cycle to diagnose; the constant now says to raise all three together and why.

- **The cartridge image is now 256 KiB (8 banks), grown from 128.** The reason is *not* total
  space: at the time of the change bank `$03` was two-thirds empty and the image had 34 KiB free
  across banks `$01`-`$03`. It is that **a segment cannot span a bank boundary in LoROM**, so each
  group's test bodies must fit inside one 32 KiB bank — and `TESTSE` was already 19 KiB with Group E
  holding by far the most uncovered dossier rows. Growing gives a group somewhere to split into when
  it outgrows a bank, which Group E will.
  Worth stating plainly because it is the opposite of the intuition: **bank `$00` is 32 KiB whatever
  the cart size**, since LoROM maps only its `$8000-$FFFF` and the vectors, header and everything
  the runtime reaches with a bank-local `jsr` must live there. The overflow that prompted this was
  in bank `$00`, and a bigger image would not have given it one byte. That is relieved by moving
  bodies out, never by a larger cart.
  Three places carry the size: `lorom.cfg`'s memory areas, the header's `$FFD7` byte (`$07` → `$08`)
  and the generator's `ROM_SIZE`. `G1.12` asserts the header's own size byte from inside the running
  image and was updated with them — a test that would have caught the change being made in two of
  the three places, which is what it is for.

- **Group D's test bodies moved out of bank `$00`.** `CATALOG` overflowed `ROM0` by 73 bytes, and
  since the catalog grows with the test *count* wherever the bodies live, the only way to make room
  for another entry is to relocate bodies. Group D followed Group C's route: 13 bank-local `jsr`s to
  runtime labels became `jsl`s to the existing `_far` wrappers, and the build gate rejected each
  remaining one until they were all gone — including two that no amount of editing the test files
  would have found, because `Asm::measure_begin`/`measure_end` emit the bank-local form from inside
  the DSL. Those gain `_far` variants; `measure_frame_height`, shared between Group B (still in bank
  `$00`) and Group D, now uses the far form unconditionally since it works from either.
  Relocation also exposed nine address immediates (`ldx #@data`, `sbc #@table`) that fitted while
  the group was in bank `$00` and are 24-bit values elsewhere — all now `.loword(…)`. Every Group D
  test passes unchanged afterwards, which is the point of checking.

- **`B1.05` attempted a third time and withdrawn.** Built exactly as the previous post-mortem
  prescribed — three bodies identical but for the address read, a counted loop, eight iterations so
  every span stays inside one scanline. Spans came out 312 / 314 / 323 dots for the 6-, 8- and
  12-clock regions, where the differences should be exactly 4 and 8 dots; they were **2 and 9**.
  The diagnosis is now specific and supersedes the previous two: the probes are right, the
  differential design is right, the sub-scanline bound is met, and the **instrument is
  dot-quantised while the signal is smaller than its noise**. `hv_begin`/`hv_end` difference the H
  counter, so a 16-clock difference is four dots only if every dot on both sides is four clocks —
  which `T-06-A` has since established is false — and only if both spans start at the same H, which
  a polling loop cannot arrange to better than its own ~5-dot granularity. More repeats would raise
  the signal and are blocked from both ends, since they cross a scanline. The row needs a
  clock-domain instrument; the cart has a dot-domain one. That is the same limit that marks `A5.20`
  `[NOT CART-MEASURABLE]` — one obstacle, not two.

- **`A5.16` — is `BRL` a flat 4 cycles?** It joins the opcode sweep, and is the one entry there
  answering a dossier row of its own rather than the composite `A5.01-08`. A *taken* branch is the
  sweep's excluded case because it moves `PC` — but `BRL` with a zero displacement falls through to
  the very next instruction, so it measures inline like any other opcode. 4 cycles / 3 accesses = 30
  clocks, against an 8-bit branch's +1 for being taken and, in emulation mode, +1 more for crossing
  a page; measuring exactly 4 *is* the "never penalised" claim, since a penalty would read as 5.
  Verified by adding an idle cycle to `op_brl`.
  The sweep's slot allocation needed fixing to take it: `slot_base = 8 + index * 2` ran past the end
  of its reserved 8-75 block at the 35th entry and landed on `B4.09`'s slot. `dossier::check_slots`
  caught it immediately, which is the second time that gate has paid for itself. The block is now
  named, documented and continued into a second one rather than being an unstated convention.

- **`B2.01` — does any dot above 339 exist?** The regression guard for `T-06-A`, and its design is
  the finding. Asserting "the largest H sampled is exactly 339" is **not portable**: which dots get
  sampled depends on the core's instruction timing, since the loop covers roughly every fifth dot
  and relies on its phase drifting between lines — and 1364 factors as `2² × 11 × 31`, so a loop
  period sharing a large factor with it covers a sparse lattice forever. Measured: RustySNES reaches
  339, Mesen2 338, snes9x 332, from three cores that agree about the dot count.
  So the assertion is one-sided — **no sample may exceed 339**. Reaching 340 proves an extra dot
  exists; failing to reach 339 proves nothing. That matches the defect, which can only ever show up
  as a reading too *high*. The lower guard is loose (300) and bounded only from below, because a
  `335..=339` guard would make the assertion unable to fire — a core reporting 340 would trip the
  guard instead, and the injection said exactly that before it was fixed. The loop also jitters, one
  iteration in two taking an extra `NOP`; without it the period was evidently a divisor of 1364 and
  the maximum came back 336.

- **`E8.02` — does key-on take five output samples to reach the envelope?** A `KON` write is held
  while the DSP reads the directory, fetches the first BRR block and primes the interpolator, which
  is why a driver that keys on and immediately reads `ENVX` sees zero. The observable is "how long
  until `ENVX` goes non-zero", polled through the register port — and that loop costs about half a
  sample, comparable to the thing being measured, so it is **subtracted**: one phase times the poll
  against an already-sounding voice, the other times it from a `KON`. Timer 2 at `T2DIV = 2` ticks
  once per output sample, so the difference counts samples directly: baseline 0, keyed-on 7 (the
  five-sample delay, the `KON` write, and the first sample on which `ENVX` climbs). Verified by
  setting the delay to zero, which fails it.
  Two things had to be got right first, and both were caught by a guard rather than by inspection.
  The release before the second key-on runs **254 samples**, not the ~96 the first version allowed —
  the envelope was still around `$40` when the poll began and it exited on its first pass, reporting
  a delay of one tick. `ENVX` is now asserted to be zero before the key-on. And `T2DIV = 1` was finer
  but put the reading at 15 of `TnOUT`'s 16 values, close enough to the wrap that the NTSC/PAL drift
  gate caught it failing on the PAL image alone. Resolution is worth nothing if it costs headroom.
  Adds `Spc::beq_back`, the zero-flag mirror of `bne_back`, for polls that wait for a register to
  *become* something.

- **`C11.08` — is the Mode 7 multiplier busy during active display?** A **golden vector**. The
  multiplier is not a unit sitting beside the PPU; it is *the* one the renderer transforms each
  pixel with, so a mid-frame read of `$2134`-`$2136` returns whatever step of the transform it has
  reached rather than the programmed `M7A × M7B` — which is why it is only usable as a
  general-purpose multiplier during blank. The blank read **is** asserted, and is not redundant with
  `C11.07`'s: it re-establishes the same fact in this test's different register state (Mode 7
  selected, BG1 on the main screen, matrix written in a different order).
  **All three cores report the programmed `$0200`** during render — measured, not assumed, and
  Mesen2 was the one worth checking since it models several intervals the other two do not.
  Producing an intermediate at all needs the sub-scanline Mode 7 pipeline that `C13.01`-`C13.06` are
  blocked on, so the row becomes scorable the same day `C1.08` does.

- **`C11.07` — do `$210D` and `$211B` share a write-twice latch?** PPU1 holds a **single** byte
  latch for its write-twice registers, so `$211B`-`$211E` (the Mode 7 matrix) and `$210D`/`$210E`
  (BG1 scroll) pass through the same one. A driver that writes `M7A`'s low byte, is interrupted by
  something touching BG1 scroll, and then writes `M7A`'s high byte gets the *interrupt's* byte in
  the low half — the dossier names an IRQ handler or an HDMA channel, both of which routinely write
  scroll registers. `M7A` is not readable, but `$2134`-`$2136` report `M7A × M7B`, so with `M7B`'s
  high byte at 2 the product is the multiplicand doubled and names it directly: `$0200` with the
  writes adjacent, `$03FE` with a `$210D` = `$FF` between them. The control is asserted exactly and
  the corruption only has to differ — `$03FE` is published but asserting it would be asserting
  *which* byte the interposed write leaves behind, where the row's claim is that the multiplicand is
  corrupted at all. Verified by dropping the latch update from the BG1HOFS path.

- **`F1.12` — when does the automatic read's result become valid?** A **golden vector**, because
  the sources conflict with each other: `F1.12` says valid by `V = $E3` (227), while `F1.09`'s
  4224-cycle duration and `F1.08`'s start window put the finish near line 228. `$4218` is sampled at
  `V` = 225, 227, 230 and 240; only the last is asserted, since four identical wrong values would
  otherwise read as a very stable answer.
  The measurement favours `F1.09`. RustySNES and snes9x report `$9050` throughout — they write the
  result in one step, so there is no interval to observe — but Mesen2 shows the fill: `$00` at 225,
  **`$82` at 227**, `$50` from 230. On the only core that models the interval, the result is *not*
  valid at the line `F1.12` names. Nothing asserts it either way; the numbers are published for
  whoever settles the conflict.

- **`F1.03` attempted and withdrawn.** The row — one write to `$4016` bit 0 latches both ports —
  needs a second controller held at a *different* mask, since with the same buttons on both, "port 2
  latched" and "port 2 is echoing port 1" are the same sixteen bits. The harness and the snes9x
  driver both handled the extended contract and the test passed on snes9x; **Mesen2 went from 0
  failing tests to 5**, losing port *1*'s input to the port-2 call. A `pcall` guard changed nothing
  (so the call is not raising) and reversing the order dropped it to 1 (so the calls are not
  independent) — with no device in port 2, Mesen2's headless runner lets the second call clobber the
  first's pending state. Withdrawn rather than shipped green on two references out of three;
  `runtime.inc` records what re-adding needs. This is the first Group F blocker about a *runner*
  rather than about the machine, and `F1.15`-`F1.22` need a much larger version of it.

- **`F1.11` — does holding the `$4016` latch corrupt the automatic read?** Two phases differing only
  in the latch line: the control reads `$9050`, the latched run must not. Both halves of the input
  contract are load-bearing — the control has to be exactly `$9050` or "phase B differs" could mean
  the poll never ran, and the corruption is only *visible* because a button is held, since with
  nothing pressed a correct uniform-`$0000` corruption and a correct `$0000` read are the same
  sixteen bits. What is asserted is "differs", not a particular corrupt value: which uniform value
  appears depends on how a core models a shift register reloaded while clocked, which no source
  pins down.

- **`F1.05` — is a standard pad's signature nibble `0000`?** The four bits after the twelve buttons
  identify what is plugged in — a mouse reports `0001`, an NTT Data keypad `0100` — so software
  distinguishes peripherals by reading a nibble rather than guessing from behaviour. The guard
  deliberately masks the nibble out and checks only bits 15-4 against the held mask: comparing the
  whole word would be strictly stronger and would make the nibble assertion unable to ever fire,
  which is the same vacuity this battery keeps finding, from the other direction. It was found that
  way too — the first injection failed the guard and left the named assertion untouched.

- **`F1.06` — is the first bit clocked out `B`, in bit 15?** Auto-read shifts the same sixteen bits
  a manual read would, in the same order; `F1.01` asserts that for the manual path, and a core could
  implement the two independently and have exactly one backwards. Bit 15 (`B`, held) and bit 14
  (`Y`, not held) are both checked, because bit 15 alone passes on a core reporting `$FF` for the
  high byte — which is what an unimplemented auto-read looks like where the line idles high.

- **Group F now settles past the auto-read window before reading `$4218`.** `wait_vblank_far`
  returns at the start of vblank, which is exactly when the automatic read begins, and it takes
  about three scanlines. Mesen2 clears the result registers when the read starts and fills them as
  it goes, so `F1.06` read `$4219` as `$00` there while passing on RustySNES and snes9x, which write
  the result in one step. Both are defensible models of an interval nobody observes directly, and
  the cart was reading inside it — `F1.12` says results are valid by `V = $E3`. The tests now burn
  about seven scanlines first.

- **`F1.07` — does `$4218`-`$421F` stop being written with `$4200` bit 0 clear?** Restored after its
  earlier withdrawal, which the contract makes possible: `$4218` reads `$0000` before auto-read is
  ever armed, `$9050` once armed, and `$9050` still after disarming. The guard is that the first two
  differ — without a change to preserve, "it did not change" is equally true of a core that never
  stopped polling, and the guard also catches the ordering hazard where an earlier test left
  auto-read armed. This test found the `$4218` defect above on its first run.

- **`E5.12` — where does a mid-note `SRCN` change land?** A running voice re-reads its directory
  entry's *loop* address every sample, so writing `SRCN` under a held note changes nothing until the
  current sample reaches a loop point — and what plays then is the new entry's **loop** address, not
  its start. A driver swapping instruments under a held note hears the change arrive late, and hears
  the tail of the new sample rather than its attack. Entry 1's two addresses point at *different*
  constant samples, which is the whole apparatus: with the usual arrangement, where an entry loops
  back to its own start, the row has no observable at all. Control `$6E`, changed `$1F`, with `$3F`
  reserved for the start address so the assertion distinguishes all three outcomes. Verified by
  making the directory read take the start address, which moves the reading to exactly `$3F`.
  The row's other clause — that a change landing *before* the voice has looped takes the start
  address — is left uncovered and said so in the doc comment. In the pipeline that clause is about
  the key-on delay, a window five output samples wide; the cart's only timing lever is a `DBNZ` loop
  with six-cycle granularity and an upload-dependent phase, so it is not reliably hittable.

- **`C7.04` — does a sprite at `X = $100` still consume a range slot?** The X field is nine bits
  signed, so `$100` is `-256`: an 8x8 sprite there sits a full screen width to the left and cannot
  contribute a pixel. Sprite evaluation does not care — it selects on Y alone, which is why parking
  unused sprites off-screen *horizontally* still loses them to Range Over and why the convention is
  to park them below the visible area instead. Two phases in the identical configuration, differing
  only in how many sprites share the line: 2 leaves Range Over clear, 40 sets it. The guard matters
  because a stuck flag is a real failure mode — `$213E` clears on read, so a core that clears at the
  wrong moment carries it in from an earlier test. Verified by making evaluation skip sprites at
  `X >= 256`, which fails the test. `setup_and_render` gains a `high_fill` byte so the whole high
  table can be put at `X = $100` rather than just the leading sprites.

- **`E9.15` — does the per-voice mix saturate after each addition?** Two voices each near the
  positive limit come out *at* the limit, not at their arithmetic sum reinterpreted as a negative
  number. The mix is not readable, so `EON` routes both voices into the echo buffer — ordinary APU
  RAM the program reads back — with `EVOL` and `EFB` at zero so what lands there is the summed voice
  output and nothing else. A constant `+7` nibble at shift 12 decodes to `$7000`: high enough that
  two exceed the limit, low enough that one does not. One voice reads `$6E`, two read `$7F`.
  The one-voice reading is asserted first and bounded on *both* sides — a single voice that was
  already silent, negative, or already saturated makes the two-voice reading uninterpretable.
  Verified by replacing the per-voice clamp with a 16-bit wrap, which leaves the one-voice reading
  untouched at `$6E` and flips the two-voice one to `$DC`.

- **`E6.09` — does the gaussian accumulator wrap or saturate?** Interpolation sums four weighted
  taps in a 16-bit intermediate, and the row says the second addition wraps. A constant `$8` nibble
  at shift 12 gives four identical maximally-negative taps, so interpolation has no shape to
  contribute: a saturating implementation returns that same large negative value, and only a
  wrapping one can produce a positive number from four negative inputs. `OUTX` reads `$7E`. No
  tolerance is needed — the two possibilities are on opposite sides of zero.
  The test was first written as `E6.08`, the `$801` BRR decode overflow, and **passed for the wrong
  reason**: injecting into the BRR clamp moved the reading not at all, while injecting into the
  gaussian accumulator flipped it to `$81`. The decoder handles the large negative sample correctly
  and the interpolator downstream is what inverts the sign. `E6.08` is consequently recorded as not
  coverable from this cart — interpolation sits between the decoder and every observable a cart can
  reach.

- **`F1.07` attempted and withdrawn.** The row says `$4218`-`$421F` are not written while `$4200`
  bit 0 is clear, and proving a non-update needs the register to first hold something an auto-read
  would not produce. It cannot here: `$4218` powers on as `$0000`, an unconnected port auto-reads as
  `$0000`, and holding `$4016` bit 0 high across the poll re-reads that same first bit. All three
  phases returned `$0000`, the guard fired, and the test was removed rather than shipped as one that
  passes vacuously. Not an emulator gap — the row needs a pad reporting a non-zero state, the same
  peripheral contract the rest of Group F needs. Removing it returned the ROM to checksum `$BD42`,
  byte-identical to the build before it.

- **`E6.11` — the four named BRR waveform vectors.** A **golden vector**: the row names four nibble
  patterns and asks what a decoder makes of each without stating an expected value for any of them,
  so the row's content *is* the measurement. All four run at shift 12, filter 0, so the reading is
  the decoder's own arithmetic rather than a filter's history. `OUTX` reads `$E1` / `$B8` / `$6F` /
  `$4F` for `79797979` / `77997799` / `77779999` / `7777CC44`; all four move when a scale error is
  injected into the BRR decode, which is what says they track the decoder.
  The verdict deliberately does not classify them. Gaussian interpolation runs over four consecutive
  samples, so `OUTX` depends on where in the pattern the DSP was when the cart looked — and the
  measurement bore that out: against snes9x the two slow patterns agree exactly while the two fast
  ones do not. A classifying verdict would have turned the sample phase into a cross-validation
  failure about nothing, which is what happened to `E8.07`. Adds `brr_block_pattern`, since two of
  the four patterns are not expressible as one repeated byte.

- **`E10.05` — what does a DSP soft reset leave behind?** A **golden vector**; the dossier marks the
  row `[CONFLICT]` and asks for one by name. Both sources agree `FLG` bit 7 makes the chip behave as
  `$E0` and force every voice into release, and that half **is** asserted: `ENVX` reads `$7F` before
  the reset and `$00` after. They contradict each other on what `ENDX` then reads — nocash `$FF`,
  anomie `$00` — and RustySNES and snes9x both report `$01`, the bit the sample's own loop had
  already set. Both preserve `ENDX` across the reset rather than forcing it either way, which is a
  third answer to a two-way question. Verified by removing the reset's effect on the envelope, which
  fails the test.
  The first version of the assertion was wrong in an instructive way and the plan doc records it:
  it waited a sample and a half and demanded `$00`, which tests *which* implementation a core chose
  — RustySNES zeroes on the spot, snes9x runs the ordinary release ramp — rather than whether it
  honours the bit at all. Both references failed it. The settle now gives a ramped release time to
  reach zero as well.

- **`E10.01` — is the DSP's output sample exactly 32 SPC cycles?** Release steps the envelope down
  by a fixed 8 per output sample, so a voice keyed off from `GAIN` `$7F` (envelope `$7F0` = 2032)
  takes a known 254 *samples* to go silent — and timing that with timer 0 converts it to *cycles*.
  At `T0DIV = 6` (768 cycles per tick) 8128 cycles read as 10 ticks, which is where the answer was
  put deliberately: `TnOUT` is four bits, and the competing periods land far away either side — 64
  cycles per sample gives 21 ticks wrapping to 5, and 16 gives 5 outright. Two guards, because two
  different things can be silently absent: `ENVX` is asserted at full scale before the key-off (a
  voice that never keyed on exits the poll immediately) and the tick count is asserted non-zero (a
  timer that never ran reads zero however long the ramp took). Verified by doubling the DSP's base
  clocks per tick, which fails the test.

- **The measurement channel widened from 192 to 240 slots.** `E10.01` was the first test to find it
  full — the collision gate reported no free slots rather than a clash, which is the same gate
  answering a different question. `$7EE200` has room to `$7EF000`, so this is a constant change in
  four places (`runtime.inc`, the generator, the harness, the libretro cross-validator).

- **`F1.02`'s guard rewritten for the input contract.** It ORed the sixteen data bits and asserted
  the result was zero — "nothing is pressed" — which the contract makes false. Its actual purpose is
  catching a core that returns 1 to every read, which an AND over the same bits does directly and
  without depending on what is held.

- **`E1.14` — does `XCN` cost five cycles?** Two 256-instruction blocks, one of `NOP` and one of
  `XCN`, both one byte, timed off timer 0 at `T0DIV = 1` (one tick per 128 SPC cycles). Everything
  but the per-instruction cost cancels between them: `NOP` reads 4 ticks (512 cycles), `XCN` reads
  10 (1280). 256 is the widest window in which both numbers are unambiguous — fewer and the
  difference disappears into quantisation, more and the `XCN` block overflows the **four-bit**
  `TnOUT`. A one-cycle error moves the reading by two ticks, so 4 or 6 cycles would read 8 or 12.
  Verified by dropping an idle cycle from the core's `XCN`, which fails the test. Adds `Spc::xcn`
  and `Spc::nop`.

- **`E8.03` — does `KON` restart a voice that is already playing?** It is not "start if stopped":
  every write of a set bit re-enters the key-on sequence, resetting the BRR pointer and zeroing the
  envelope, which is why a driver retriggering a held note hears it restart from silence. Two runs
  with `ADSR` attack rate 8 — slow enough that the climb is still in progress — read `ENVX` after 24
  delay blocks: untouched `$34`, retriggered one block earlier `$02`. Both halves are guarded, and
  for different reasons: the untouched run must be **high** or a core ignoring `KON` reads the same
  as one honouring it, and the retriggered run must be low **in absolute terms** rather than merely
  lower, since "lower" is also satisfied by an envelope that happens to be decaying. Verified by
  making key-on conditional on a zero envelope, which fails the test.

- **`C1.08` — is the OAM address destroyed during render?** A **golden vector**. Sprite evaluation
  drives the OAM address counter while the picture is being drawn, so a `$2138` read taken mid-frame
  returns the renderer's position rather than the address a driver programmed. The low OAM table is
  filled so that byte *n* holds *n*, which makes a reading name the address it came from instead of
  merely being "different". The blank read at the same address **is** asserted, as the guard: without
  it, an odd mid-render answer could equally mean the fill or the port never worked.
  The mid-render half is recorded because no source says which byte evaluation has reached at a
  given moment, and producing an answer at all needs the sub-scanline sprite pipeline that
  `C13.01`-`C13.06` are already blocked on. RustySNES and snes9x return the programmed address;
  Mesen2 models the counter. The row becomes scorable for free once the `C13` blocker lifts.

- **`E7.12` — does the decay→sustain boundary come from `VxGAIN`?** A **golden vector**. The dossier
  records an `[ERRATA]` that the comparison ending the decay phase reads its boundary from `VxGAIN`
  bits 7-5 rather than `VxADSR2`'s sustain level — even in `ADSR` mode, where `GAIN` should be
  ignored entirely.

  `E7.07` pins the other half: with `GAIN` untouched, the boundary sits exactly where `ADSR2` says.
  This holds `ADSR2` fixed at level 3 and moves `GAIN` instead, which is the only arrangement that
  can tell the two sources apart. **Both runs park at `$40` on RustySNES and on snes9x** — the
  boundary is `ADSR2`'s, and the erratum is unmodelled.

  Recorded rather than asserted: whether a core should model it is not something the dossier settles,
  and asserting either behaviour would be asserting an implementation choice. Verified by sourcing
  the boundary from `VxGAIN`, which moves the readings apart and reports variant 2 — so the test can
  see the erratum if a core ever implements it.

- **`E7.03` — the attack rate indexes the counter table at `a*2+1`.** `E7.04` covers the one
  exception, rate `$F`; this covers the other fifteen, where the field is doubled and offset before
  it reaches the table. A core using `a` verbatim indexes the slow half for every setting, so every
  attack is far too gradual while decay, sustain and release stay correct.

  Rates `$8` and `$C` (indices 17 and 25), both well away from `$F` so the two tests stay disjoint,
  read at `$08` and `$3A` mid-attack. Verified by indexing with `a` verbatim, which fails at code 1
  — the **guard**, not the comparison, and that is the more informative failure: it says the
  indexing is wrong rather than that two numbers disagreed.

  The measurement channel filled up again at 192 slots; this test uses gaps freed by earlier moves
  rather than widening it a second time.

- **`E7.07` — decay hands over to sustain on the `$100*(l+1)` boundary.** With the sustain rate at
  zero the envelope freezes where decay stops, so the parking level *is* the boundary and is
  readable directly. Levels 3 and 5 park at `$40` and `$60`; snes9x reports the same two values.

  **The row states its boundary twice and the two statements are not obviously the same.** It gives
  `$100*(l+1)` and also "compare `(E>>8) == SL`" — and reading only the comparison suggests the
  envelope parks *inside* the level's band, `$30`-`$3F` for level 3. This test was written against
  that reading and failed. Measured, it parks one band higher, exactly on `$100*(l+1)`: the
  comparison fires on the value before the decrement, so the envelope comes to rest *on* the
  boundary rather than below it. The row's first clause is the one that holds, and the doc now says
  which and why.

  No separate guard is needed here, unusually: the two bands do not overlap and neither touches the
  ends of the range, so a stuck envelope at `$00` or `$7F` fails both assertions rather than
  satisfying either. Verified by comparing `E>>4` instead of `E>>8`, which fails at code 1.

- **`E7.06` — the sustain rate indexes the counter table verbatim.** Decay adds 16 to its field
  (`E7.05`); sustain does not, and the difference is invisible everywhere except the bottom of the
  range. Sustain rate `0` indexes rate 0, which never fires, so a voice held at sustain never decays
  — under any offset that same setting becomes a real rate and every held note in a soundtrack
  fades.

  **`r = 0` is the discriminating case**, and comparing two fast rates would have settled nothing:
  `r = 31` clamps to the top of the table with or without an offset, so both models agree there.
  Run 1 is therefore asserted at *exactly* full scale — anything below means rate 0 fired. Run 2
  uses rate 31 purely as the anti-vacuity control, since a core ignoring the field entirely would
  park both runs at `$7F` and satisfy the first assertion alone.

  Rate 16 was the first choice for run 2 and only reached `$76`: sustain decay is exponential, so
  from full scale it barely moves at first. Verified by offsetting the sustain index by 16, which
  fails at code 1.

- **`E7.05` — the decay rate is indexed as `d*2+16`.** Not the raw field: the offset places the
  eight decay settings in the upper half of the counter table the other phases share. A core using
  `d` verbatim decays orders of magnitude too slowly, and the envelope barely moves where hardware
  would have crossed most of its range.

  Two voices differing only in the decay field (`0` → index 16, `7` → index 30), both attacking at
  rate `$F` and holding **sustain level 0** so the decay phase runs the whole way and never hands
  over to sustain — the same isolation `E7.09` needed, applied in reverse.

  The window took tuning and the guard is what showed it. Decay is exponential and `ENVX` is `E>>4`,
  so the top of the range compresses: at `settle: 0` neither rate had moved and the guard fired
  correctly; at `settle: 4` the slow rate read `$7D`, one step from the guard's own upper bound.
  `settle: 24` puts it at `$76` against `$10` — a gap of 102 with headroom at both ends. Verified by
  indexing the decay with `d` verbatim, which fails at code 1.

- **`E7.09` — the release rate is fixed and consults no register.** Release steps the envelope down
  by 8 every sample; the four `ADSR` rates cover attack, decay and sustain and none reaches the
  release phase, which is why a custom fade has to be built from `GAIN` instead — the `[ERRATA]` the
  row records. Two voices differing only in their sustain rate (`0` against `31`, as far apart as
  the table goes) are keyed off and read mid-ramp; equal readings mean release ignored the field.

  **The first version reported a difference that was entirely legitimate, and would have libelled a
  correct core.** With sustain level 7 the decay phase ends immediately at full scale, so the voice
  spends the whole pre-key-off interval in *sustain* — where the sustain rate decays it, exactly as
  it should. The two runs read `$67` and `$17`, and run 2 was simply starting its release from much
  lower down. The measurement was real; the inference would have been a defect filed against correct
  behaviour.

  Sustain level **0** fixes it: the decay phase never completes, sustain is never entered, the rate
  has nothing to act on, and both runs enter release from the same place. Verified by scaling the
  release step by the sustain rate, which fails it at code 2.

- **`E7.04` — attack rate `$F` steps every sample, by `+1024`.** Every other rate advances the
  envelope by 32 on a counter tick; `$F` does neither part of that, crossing the full `$7FF` range in
  two samples. A core folding it into the general `a*2+1` formula gets an attack that is merely
  fast, and every percussive instrument softens.

  Both runs park the envelope once it arrives — sustain level 7 with sustain rate 0, so decay ends
  immediately and rate 0 never fires — otherwise the reading depends on how far decay has since
  pulled it back.

  **The settle length is the load-bearing part, and the first version got it wrong.** With the
  default four settle blocks the test passed its own injection: over that long an interval `+32`
  every sample *also* reaches full scale, so the reading distinguished only "the rate index is
  large" and not the step size. Dropping to `settle: 0` puts the read about thirty samples after
  key-on — long enough for `+1024` to have crossed the range twice, far short of the sixty-four
  samples `+32` needs. The same injection now fails at code 1.

- **`E9.01` — the noise LFSR starts at `$4000`.** A wrong seed produces a different noise sequence
  from the first sample onward, which no amount of listening distinguishes from correct noise and
  which any bit-exact audio comparison fails immediately.

  Freezing the register is what makes the seed observable. The noise rate comes from `FLG` bits 0-4
  through the counter table the envelopes use, and **rate 0 never fires** (`E7.01`) — so with those
  bits clear the register is seeded and never advances, and a voice in noise mode outputs a direct
  function of the seed. The arithmetic checks out exactly: `$4000 << 1` is `$8000`, scaled by the
  direct gain's `$7F0` envelope gives `$8100`, and `VxOUTX` is that top byte. All three cores report
  `$81`. Verified by seeding `$0001` instead, which gives `$00` and fails at code 1.

  One thing the test explicitly does *not* claim: every program in this group leaves `FLG` at `$20`,
  so the noise rate is zero throughout the whole battery and the LFSR never advances anywhere. What
  is read is the **power-on** seed. RustySNES's `$6C` write sets the mute and reset flags without
  touching the shift register, so whether a *soft reset* re-seeds it is a separate question this
  test does not answer, and the doc says so rather than letting the `flg_reset` in its configuration
  imply otherwise.

- **`E5.01` — the BRR header is `ssssffle`.** The other three fields are already pinned:
  `E5.03`/`E5.05` decode through both filters, `E5.04` drives the shift into its invalid range,
  `E5.08`/`E5.09` separate the loop bit from the end bit. What none establishes is *where the shift
  lives* — and a core reading it from the wrong nibble decodes every sample at the wrong amplitude
  while still honouring the flags, which looks like a volume bug rather than a header bug.

  Two constant samples identical but for the shift nibble, `$8` then `$9`. A nibble decodes as
  `(nibble << shift) >> 1`, so the second must be twice the first — a claim about the field's
  *position*, since a core reading bits 3-0 sees zero in both headers and returns the same
  amplitude twice.

  **Written as an exact doubling it failed**: the readings are `$06` and `$0D`, and twice six is
  twelve. `VxOUTX` is the top eight bits of a fifteen-bit sample after gaussian interpolation, so
  the low-order rounding does not survive truncation — the amplitude doubles, the reported byte
  doubles to within one. The tolerance is two, still far tighter than the factor separating it from
  "the shift was ignored".

  The guard turned out to be load-bearing in the literal sense: injecting the wrong-nibble read
  makes both headers decode at shift 0, so both readings are `$00` — and zero doubles to zero.
  Without the non-zero check the test would have **passed** under exactly the bug it exists to
  catch; with it, the injection fails at code 1.

- **`E8.10` — `KOFF` and `KON` together silence a voice faster than `KOFF` alone.** `KOFF` starts
  the release ramp, which takes about eight milliseconds; `KON` zeroes the envelope outright, and
  since `KOFF` outranks it (`E8.04`) the attack never starts. The pair gets the zero without the
  ramp, which is why drivers use it to cut a voice dead. Read early — one delay block, not the
  twelve `E7.08` uses — so the ramp has started but not finished: `$68` for `KOFF` alone against
  `$00` for the pair.

  **The first version measured nothing, and the cause was a lesson already on record.** It wrote
  `KOFF`, `KON`, then `KON = 0` back to back, which makes the key-on a ten-cycle pulse — and
  `KON`/`KOFF` are sampled every second output sample, so the poll missed it entirely. Both runs
  read within one of each other. That is the same mechanism `E8.07` is a whole test about, applied
  by accident to my own setup. `Voice` gained a `post` list so the clear happens *after* the settle,
  holding the bit across a poll and still leaving the register tidy.

  Verified by skipping the envelope zeroing while `KOFF` is asserted, which fails it at code 2. Two
  earlier injection attempts changed nothing — the first checked the wrong state (`KON` sets the
  mode to Attack before the delay runs, so a "not Release" guard never fires). An injection that
  does not move the verdict says nothing about the test until you find out which of the two is
  wrong.

- **`E9.13` — the echo FIR filters the two channels independently.** Each has its own eight-sample
  history and accumulator; only the `FIRx` coefficients are shared. A core keeping one history turns
  every echo to mono the moment feedback is on — and that is invisible to any test driving both
  channels equally.

  Feedback is what makes this about the *filter*. `E9.05` already shows an all-left voice writes
  zeroes into the right half of the buffer, so the input mix is separate; but with `EFB = 0` the
  buffer never reaches the FIR at all. This enables feedback and one non-zero tap, writing every
  coefficient explicitly — the DSP is shared between programs, and a leftover tap set inside a
  feedback loop is the difference between a filter and an oscillator.

  **It was first written asserting the right channel is exactly zero, and all three cores returned
  `$FFFE`** — minus two. Three implementations agreeing is a wrong expectation, not a shared defect,
  and the magnitudes agreed: the left sits near `$0900` and the right is nine bits below it, which
  is arithmetic residue rather than a channel leaking. The assertion is now about orders of
  magnitude — the left substantial, the right within one high-byte step of silence either side.
  Verified by sharing one history between the channels, which lifts the right to `$024A` and fails
  it at code 3.

- **`E9.05` — the echo buffer stores four bytes per entry, left channel first.** `L` low, `L` high,
  `R` low, `R` high. `E9.12` already pins what goes *in* each sample; this pins the layout around
  them, which is what a driver reading the buffer back has to know.

  With a voice at equal volume on both sides the four bytes are two identical pairs, and a core
  writing two bytes per entry — or writing right-then-left — produces the same buffer. So the voice
  plays at full volume on the **left only**: bytes 0-1 must be written and not the `$FF` marker,
  bytes 2-3 must be written *as zero*. A two-byte-entry core leaves the marker in 2-3; a
  right-then-left core puts the zeroes in 0-1.

  Verified by inverting the channel-to-offset mapping in the DSP, which fails it at code 2. **The
  first injection attempt proved nothing**: swapping which channel is written at echo29 versus
  echo30 leaves each one addressing its own offset, so the buffer is unchanged. Noticing that the
  verdict had not moved is what led to the right injection — the address arithmetic, not the call
  order.

- **`E9.03` — `VxPITCH` does not affect the noise generator's frequency.** A voice in noise mode
  takes its samples from the global LFSR, whose step rate comes from `FLG` bits 0-4 and nothing
  else. The voice's pitch still drives its sample pointer — which is why `E9.04` can show a noise
  voice decoding BRR at the same time — but it has no bearing on how fast the noise advances. A core
  running the LFSR off the voice's pitch counter gives every noise voice a timbre that depends on
  the note it was keyed at.

  Two runs differing in exactly one register, `$1000` against `$2000`, read at the same point. Both
  return `$81`, so the pitch did not reach the noise generator. Verified by perturbing the noise
  rate with voice 0's pitch, which fails it at code 2.

  Two things make the comparison mean something. **The LFSR is global and survives between
  programs**, so without a reset the second run would start from wherever the first left it and the
  readings would differ for a reason unrelated to pitch — both programs now pulse `FLG` bit 7 first,
  which re-seeds it to `$4000` (the new `Voice::flg_reset`). And **two silences are also equal**, so
  the output is asserted non-zero before the two are compared; without that, a muted or unkeyed
  voice would pass the test having compared nothing.

- **`G1.07` — what WRAM powers up holding.** The row is marked `[UNDEFINED]` and asks for a golden
  vector by name, and the measurement is about as complete a demonstration of that as a row can get
  — **one core per variant**:

  | core | `$7F8000` | `$7F8020` | `$7F8040` | variant |
  |---|---|---|---|---|
  | RustySNES | `$00` | `$00` | `$00` | 1 — uniformly zero |
  | snes9x | `$55` | `$55` | `$55` | 2 — uniform, non-zero |
  | Mesen2 | `$E8` | `$47` | `$2C` | 3 — randomised, different every run |

  Three bytes rather than one, spaced 32 apart, so a *pattern* is distinguishable from a uniform
  fill — a single byte cannot tell `$00` everywhere from `$00` here and something else nearby, and
  "32 of one value then 32 of another" is exactly the shape `E4.11`'s row documents for APU RAM.
  They are read from bank `$7F` above `$8000`, which the battery never writes; most of WRAM would
  say more about the battery than the console.

  It also explains `G1.20`, which reads WRAM through `$2180` at an indeterminate address and got
  `$00`/`$55`/`$33`: the `$33` was one of Mesen2's random bytes, not a third fill convention. With
  `E4.11` doing the same for APU RAM, both memories a core must invent a power-on state for are now
  recorded.

- **`E2.04` — `DBNZ dp,rel` is a read-modify-write.** The access pattern is invisible on ordinary
  RAM, where read-decrement-write leaves exactly what a bare decrement would. It becomes visible on
  a target whose *read* has a side effect, and the SPC700 has three: the timer counters at
  `$FD`-`$FF` are read-to-clear. Pointed at one, `DBNZ`'s read clears it whatever the arithmetic
  then does; a core implementing `DBNZ` as a plain decrement leaves the count in place.

  The control is the same interval timed without the `DBNZ`, and it doubles as the drain — reading
  the counter is what clears it, so phase 2 starts from a known zero for free. The displacement is
  `0` so the branch falls through either way; `DBNZ`'s branch is not what this is about. Verified by
  removing the read from the instruction, which fails it at code 2.

- **`F1.14` — `$4213` reads the `$4201` output latch back.** `$4201` drives controller port 1's
  IOBIT pin from bit 6 and port 2's from bit 7; `$4213` reads those pins. The port is
  open-collector, so a device can pull a pin low but nothing pulls one high — with nothing driving
  them, the value read is the value written. A standard pad drives neither, which is why this needs
  no peripheral contract: the assertion is about the latch and its read-back path.

  Three values, chosen so a stuck bit cannot hide: `$FF` and `$00` catch a core returning a constant
  either way, and `$55` catches one returning "all bits the same" — a mask, a boolean, or the two
  IOBIT pins smeared across the byte. All three cores return all three values exactly.

  `$4201` is restored to `$FF` **before anything is asserted**, and that is load-bearing rather than
  tidy: bit 7 gates the `$2137` counter latch (`C3.10`) that a dozen later tests depend on, and a
  failure exits through `test_restore`, which deliberately does not touch `$4201`. Leaving it at
  `$00` would turn one failure into a cascade.

  It is complementary to `G1.19` rather than overlapping: injecting "`$4213` always reads `$FF`"
  fails `F1.14` at code 2 while `G1.19` — which only checks `$FF` at power-on — still passes.

- **`F1.04` — `$4016` bits 7-2 are CPU open bus**, and it is Group F's first scored row that needed
  no peripheral contract. Only bits 1 and 0 carry controller data; the rest of the byte is driven by
  nothing and reads back as whatever the CPU last left on the bus.

  The claim is checked without settling any open-bus model: the register is read twice, once as
  `lda $4016` (`AD 16 40`) and once as `lda f:$004016` (`AF 16 40 00`), so the last byte fetched
  before each data cycle differs. A core whose bits 7-2 are open bus returns the operand byte each
  time; one that manufactures them returns the same value twice. **All three cores return exactly
  `$41` and `$01`**, so this is `Corroborated` and scored rather than recorded. Verified by
  hardcoding the bits to zero, which fails it at code 1.

  **It also narrows Group F's blocker.** `docs/accuracysnes-plan.md` recorded the whole group as
  waiting on a documented peripheral contract, because the cart cannot tell "no controller" from
  "pad past bit 16". That is true of bits 1 and 0 — and only of them. This test masks them off, and
  the same move reaches the group's other undriven-bit and mechanism rows. The contract is still
  needed for the read order, port 2's seventeenth bit, the auto-read registers and the exotic
  peripherals; it is not needed for roughly half the group.

- **`G1.20` — the registers power-on leaves indeterminate.** A **golden vector by instruction**:
  `G1.03` lists `APUIOn`, `WMDATA`, `WMADD*`, `JOYSER`, `HDMAEN`, `MDMAEN` and `JOY1-4` as undefined
  and says to *report, never assert*. Half of them are write-only and have nothing to report at all,
  which is a property of the bus rather than a gap in the test; the rest are sampled in
  `capture_power_on` before `init_registers` writes over them.

  The row earns its place on one byte. `$2180` reads WRAM through the port at whatever indeterminate
  address `WMADD` held, so it reports each core's **WRAM power-on fill** — and the three
  implementations disagree completely: RustySNES `$00`, snes9x `$55`, Mesen2 `$33`. None is wrong;
  `G1.07` says no canonical fill exists, and this is the measurement behind that claim.

  All four APU ports read `$00` on all three cores, so none has run the IPL's announcement by the
  time the cart's reset handler gets there. Every other test hides that by waiting for the
  announcement first.

- **`D2.09` — enabling HDMA outside vblank transfers from an uninitialised channel.** A **golden
  vector**. HDMA initialises every enabled channel once per frame at `V = 0`, reloading `A2An` from
  `A1Tn` and fetching the first line-count byte. Enabling a channel after that does not run the
  init; it simply joins the per-line transfers using whatever `A2An`/`NLTRn` still hold. Marked
  `[ERRATA]`, and what the stale pointer contains depends on where the previous frame stopped, which
  no source specifies — so the test records rather than asserts. RustySNES reports variant 1:
  nothing transferred.

  Phase 1 is the control: the identical channel armed during vblank, running a full frame, and its
  first written byte is **asserted** to be the table's first data byte. Without that, "phase 2 wrote
  something odd" could equally mean the table, destination or channel programming was wrong. The
  table is eight one-line entries carrying `$11`-`$88`, so a landing-page byte names the entry it
  came from.

  The variants describe the observation and not a mechanism, deliberately: making the `$420C` write
  run the per-frame init — the obvious "no erratum" implementation — produced variant **3**, ten
  bytes starting `$C2`, not the variant 2 that guess would have predicted.

  It also landed on a second shared-scratch hazard. Its first landing page was `$0D`, which `D1.14`
  points `WMADD` at — and because `D1.14` names it through `$2182` rather than as an address
  literal, no grep for `$7E0D00` finds it. `D1.14` began failing the moment this test was added. The
  pages are now `$13`/`$14`, taken from well outside the range anything else uses; WRAM scratch has
  the same no-allocator problem the measurement channel has, without the gate.

- **`E8.07` — a `KOFF` pulse shorter than the poll interval is never seen.** `KOFF` is sampled every
  second output sample, not acted on at the instant it is written, so `$FF` followed a few cycles
  later by `$00` collapses into one poll that reads `$00` and nothing is released. A core applying
  `KOFF` on the write releases every voice on the `$FF` and cannot take it back — release is a state
  the envelope has entered, not a level held on a register.

  **It is the pair to `E7.08`**, which writes a single `KOFF` and asserts the envelope reaches zero.
  Together they bracket the mechanism: the first shows key-off works, the second that it is sampled
  rather than edge-triggered. Either alone is weak — "the envelope is still `$7F`" would be
  satisfied by a core whose key-off never worked at all. Verified by making `KOFF` act on the write:
  `E8.07` fails at code 1 while `E7.08` still passes.

  **The pulse has to be genuinely short, and the first version was not.** Written as two ordinary
  `dsp_write`s the values sit about twelve SPC cycles apart, and that failed on Mesen2's **PAL**
  image while passing on its NTSC one — the SPC is synchronised to a CPU clock that differs by
  region, so the same sequence spans a different fraction of the poll interval. A test about the DSP
  that changes answer with the video standard is measuring the harness. It now emits one `$F2`
  select and two `$F3` stores, about five cycles apart: robust across all four core/region
  combinations, and a truer statement of a row that is specifically about a short pulse.

- **`E5.10` — BRR decoding keeps running for a released voice.** Key-off starts the release ramp;
  it does not stop the decoder, which goes on reading blocks, following loop points and setting
  `ENDX`. A core that treats key-off as "switch this voice off" gets the audible result right — the
  envelope reaches zero either way — and the state wrong, so a driver watching `ENDX` for a released
  voice's sample to wrap waits forever.

  **It is the distinction `E7.08` documents that it cannot make.** That test keys off the same voice
  and asserts the envelope reaches zero, noting: *"the one thing it cannot distinguish is a core
  that stops the voice outright on key-off instead of releasing it, since both end at zero."* This
  one uses the decoder instead of the envelope, because the decoder is the part that keeps running.
  Verified by halting decode for released voices in the DSP: `E5.10` fails at code 2 while `E7.08`
  still passes — the gap, demonstrated.

  Two details make the reading mean what it claims. `ENDX` is cleared **at** key-off rather than
  before it, because the single-block looping sample sets `ENDX` within a few samples of key-on, so
  a reading taken at the end would otherwise say nothing about what happened after the release.
  And `ENVX` is asserted zero first: a core whose key-off did nothing would leave the decoder
  running for the ordinary reason and pass the `ENDX` check without having been tested.

- **`E1.07` — `DIV YA,X` is valid only while the quotient fits in nine bits.** `V` is quotient bit 8,
  so `A` carries 0-255 and `V` the 256-511 range; ask for 512 and there is nowhere to put the
  answer. Marked `[ERRATA]` because the hardware does not wrap or saturate in the obvious way — it
  silently switches to `E1.03`'s overflow algorithm and both halves of the result go wrong together.

  Two divisions with the same divisor, one step apart: `$03FE / 2` (quotient 511) returns `A = $FF`,
  `Y = $00`, and `$0400 / 2` (quotient 512) returns `A = $FF`, `Y = $02`. **`A` is `$FF` in both**,
  which is the trap — a test checking only the quotient sees the same byte on either side of the
  boundary and concludes nothing happened. The remainder is what moves: 1024 / 2 leaves none, and
  the hardware reports 2.

  The negative is pinned in both bytes: a core computing `YA / X` and `YA % X` and truncating
  returns `$00`/`$00` for the second division, so the failure cannot be a rounding difference. The
  first division is the control — same instruction, same divisor, one step below the boundary — so a
  core failing it has a broken `DIV` rather than a boundary bug. Verified by deleting the overflow
  branch from the SPC700 core: `E1.07` fails at code 3 and `E1.03` alongside it, while `E1.02`'s
  normal branch keeps passing.

- **`E4.11` — what pattern does APU RAM power up holding?** A **golden vector**: the dossier records
  a repeating `32x$00, 32x$FF` fill and marks it chip-dependent and informational, and the three
  cores do three different things, none of them that.

  | core | `$8000` | `$8020` | `$8040` | variant |
  |---|---|---|---|---|
  | RustySNES | `$00` | `$00` | `$00` | 1 — uniformly zero |
  | snes9x | `$00` | `$00` | `$00` | 1 — uniformly zero |
  | Mesen2 | *random* | *random* | *random* | 3 — neither |

  **Mesen2 randomises APU RAM**: four consecutive runs returned `$62`, `$18`, `$F2`, `$85` at
  `$8000`. Its bytes here are therefore not reproducible, which is the finding rather than a defect
  in the measurement — and is why this is golden. A scored test would flap on Mesen2 every run.

  Addresses are one per half-period of the documented 64-byte cycle so the pattern would be
  unmistakable, and they sit at `$8000`, clear of both the `$0200` upload area and the `$3000` echo
  buffer the `E9` tests use. Verified by injecting the documented pattern into RustySNES's power-on
  fill: the test reports variant 2, and — worth noting — the rest of the battery stayed at 250/250,
  so nothing else depends on ARAM booting zeroed.

- **`E4.03` — the IPL boot ROM zero-fills APU RAM `$0000-$00EF`.** The first version of this test
  could not fail, and finding out why is most of what it is worth reporting.

  Written the obvious way — upload a program, check the zero page is zero, and place it first in the
  group so nothing else could have dirtied it — it passed on every core, and on two of the three it
  proved nothing: **RustySNES and snes9x both boot APU RAM as all-zero**, so a core that never ran
  the fill produces an identical reading there. An armed-ness probe at `$0420`, outside the filled
  range, read `$00` on both and confirmed the assertions were unfalsifiable. (`E4.11` below later
  measured this properly and found Mesen2 *randomises* APU RAM, where the original test would in
  fact have discriminated — but a test that only works on one of three cores is not one this
  battery can score.)

  The fix came from reading `release_to_ipl`: it jumps to **`$FFC0`**, the IPL's *reset* entry, and
  the zero-fill is the first thing there — so the fill runs again before every upload. The way to
  make the test falsifiable was therefore not to run before anything else, but to dirty the range
  deliberately and go back through the IPL. It now uploads two programs: one fills `$02-$EF` with
  `$FF` and releases, the second sweeps and reports. Verified by NOPing the `MOV (X),A` in the IPL
  ROM's fill loop — both halves come back `$FF` and the test fails. Being self-arming, it also no
  longer depends on its position in the group.

  `$00`/`$01` are excluded and reported instead of asserted: they are the IPL's own transfer-
  destination pointer, measured as `$01 = $02`, the high byte of the `$0200` upload address.

  It also surfaced a latent coupling: **`E4.04` depends on the preceding program having overwritten
  port 1.** It polls port 1 for the boot ROM's `$BB` and then asserts port 0 reads `$AA`, which is
  only sound if a stale `$BB` is not already sitting there — otherwise the poll matches instantly
  and port 0 is read while the SMP is still working through the fill. Every other program happens to
  write `PORT1` with a result; the first version of `E4.03` did not, and `E4.04` failed immediately
  after it. `E4.03` now reports `$01` there, restoring the invariant deliberately rather than by
  accident.

- **`G1.19` — the documented power-on register state, for the parts a cartridge can observe.**
  `$4200-$420D` are write-only, so "what did reset leave here" cannot be answered by reading them
  back; each register needs its own indirect channel. Two more are now sampled in
  `capture_power_on`, the pre-`init_registers` hook `B5.05` and `D1.11` already use, because
  `init_registers` deliberately overwrites the whole block:

  - `$4201 = $FF`, observed through `$4213` (RDIO), which reflects its output pins.
  - `HTIME`/`VTIME` `= $1FF`, observed by arming both timers on the untouched comparators and
    watching three frames produce no interrupt at all. 511 is past both the 340-dot line and the
    262/312-line frame, so a correct machine can never match — while every likely wrong answer is
    reachable and fires almost immediately: `$0000` matches on the first line of every frame, and an
    8-bit truncation gives 255, a real dot and a real line.

  Verified by injecting each: powering the timers up at `$0000` fails at code 2, powering WRIO up at
  `$00` fails at code 1. The row's remaining registers stay where their mechanism already lives —
  `$4202` and `$4204/05` with `B5.05`'s multiply/divide probes, `$420D` with `B1.01`'s access
  timing. `$4200 = $00` is explicitly **not** claimed: the probe enables the timer interrupts
  itself, so a machine that powered up with them already enabled is indistinguishable.

- **`C3.10` / `C3.11` — `$2137`, split into its two independent claims.** The dossier row makes two
  at once, and they turned out to have opposite verdicts, so they became separate tests (declared in
  `SPLITS`).

  `C3.10` asserts the **gating**, and is scored: snes9x and Mesen2 both honour it and RustySNES was
  fixed to match. Reading the latched counters needs care — `$213C`/`$213D` return *latched* values,
  so the battery's usual helper (which pokes `$2137` first) is both the action under test and an
  infinite loop once latching is off; this test reads them directly and waits on `$4212` instead.
  `$4201` is restored *before* any assertion, because a failure exits through `test_restore`, which
  does not touch it — a cleared bit 7 there would break the counter latch for every later test.

  `C3.11` records **which open bus** `$2137` presents, and is golden: snes9x and RustySNES return
  PPU1's latch, Mesen2 returns the CPU's. Both are physically reasonable and no source decides.
  Making the two distinguishable took a deliberate step — planting `$5A` in PPU1's latch via `$2138`
  also leaves `$5A` as the CPU's last read, so both models predict the same byte and the test would
  have passed everywhere while proving nothing. A WRAM read in between separates them.

  Mesen2's actual answer is `$21`, not the `$A5` planted: its `lda $2137` puts its own operand on
  the bus first. That was **measured after an earlier draft predicted `$A5` from reading Mesen2's
  source** — which is why the raw byte is recorded alongside the variant.

- **`C2.09` — a VRAM read returns the latch, and only the trigger register refills it.** The read
  port is a latch, not a window onto memory: `$2139`/`$213A` hand back what the latch holds, and the
  register selected by `VMAIN` bit 7 refills it **from the address it is still on** before stepping.
  Two words seeded `$1234`/`$ABCD` make every wrong answer name itself — reading the non-trigger
  register twice must return the same byte, and the byte after the trigger must still come from the
  first word.

  **The test's first expectation was wrong, and all three cores said so.** It was written expecting
  the post-trigger read to come from the *following* word, reasoning that refilling before the step
  would re-read one word forever. Every core rejected it identically — the signature of a wrong
  expectation, not a wrong core — and the reference source settles it: bsnes and ares both
  `latch = readVRAM()` and only then `vramAddress += vramIncrementSize`. That ordering is precisely
  what produces the documented one-word prefetch lag, and it is what the dossier's "return latch →
  refill latch → increment" says. The assertion now pins that direction, so it catches the
  inversion instead of demanding it — verified by inverting the two statements in the PPU, which
  fails at code 4 and passes again once restored.

- **`C9.05` — does enabling overscan mid-vblank re-close the VRAM window?** A **golden vector**,
  because the references split: RustySNES and snes9x drop a write issued just after the toggle,
  Mesen2 lets it land. One reference each way is `Contested` under ADR 0003, and the dossier's own
  repro cannot break the tie — it read-modify-writes the **write-only** `$2133`, so which bits moved
  is unknowable from it, and the row does not fix which direction the height changed.

  **The first version of this test was vacuous, and cross-validation certified it.** The battery
  runs its tests under forced blank, which opens the VRAM port unconditionally regardless of line or
  screen height. The test never released it, so both writes landed on all three cores — and that
  read as unanimous agreement that the window stays open. It was unanimous agreement about nothing:
  no write could be dropped by anything, and the real split was completely hidden. A vacuous test
  passes identically everywhere, which is exactly what agreement looks like.

  What catches it is a guard that proves the mechanism is armed. The test now releases forced blank
  and writes a third word from the middle of active display, requiring *that* write to be dropped;
  if it lands, the test reports itself unarmed rather than reporting a result. With the guard in
  place the same cart, unchanged in every other respect, immediately produced the opposite reading —
  which is the whole argument for the guard.

- **`B4.11` — no IRQ triggers for dot 153 on the last scanline of a frame.** A **golden vector**.
  superfamicom.org's timing page states the exception outright and gives no mechanism; its timing
  text derives from fullsnes, so what reads like two sources is one, and no test ROM verifies it.
  ares, bsnes, Mesen2 and snes9x were each searched for the exception and **none of the four
  implements it** — which matches the dossier's own list of behaviours with no public test-ROM
  coverage. All three cross-validated cores report the interrupt firing.

  Recorded rather than scored, and RustySNES was deliberately *not* changed to match. Honouring a
  single-source claim no hardware test confirms would make this core the only one suppressing the
  interrupt, and the cost of being wrong is a missing interrupt in real games — the expensive
  direction. The recording covers the row honestly and is the evidence that would justify the
  change if someone confirms it on hardware.

  Two controls run alongside the reading and both must fire: dot 153 one line earlier, and dot 100
  on the same last line. Without them a core that cannot raise an HV-IRQ at all would report
  "suppressed" for free, so an inconclusive run gets its own variant rather than being folded in
  with a real observation. The last line is measured, not assumed — it is 261 on NTSC and 311 on
  PAL, and `B2.10` establishes the region bit is too contested to branch on.

  Verified by implementing the exception in the PPU comparator: the test moved to variant 2 with
  both controls still firing, and back to variant 1 once removed.

- **`B4.13` — `HTIME`/`VTIME` are nine bits, and the surplus range is inert.** Both registers accept
  values up to 511 while the counters they are compared against stop at 339 dots and 261 (NTSC) or
  311 (PAL) lines. The assertion is that a value past the end simply never matches.

  "Nothing happened" is the weakest kind of observation, so two things make it mean something. Each
  half is preceded by a **positive control** at an in-range value armed through the same path, so a
  broken timer fails the control rather than passing the silence. And both plausible wrong answers
  are loud: a core keeping only the low eight bits arms at 144, one reducing modulo the line length
  arms at 59, and each fires on nearly every line. The wait is frame-counted rather than a spin, so
  "never fires" is a finite result instead of a battery timeout.

  Verified by injecting each masking bug in turn — dropping `$4208` bit 8 fails at code 2, dropping
  `$420A` bit 8 fails at code 4 — and passing again once restored.

  **This found a snes9x defect.** snes9x fires an H-IRQ at `HTIME = 400`. Its register write keeps
  all nine bits; the fault is downstream, where the beam position becomes an absolute cycle within
  the line and nothing checks that the result exceeds the line length. 400 lands at 1600 cycles
  against a 1364-cycle line, so rather than being rejected as unreachable it carries into the next
  line and fires near dot 59 — the modulo-reduced answer the test's own failure message names.
  Mesen2 and RustySNES both agree with the cart, so it is recorded as a sixth known snes9x
  divergence rather than chased.

- **`B3.01` — the DRAM refresh pause, probed by the tight H-counter loop `B3.03` names.** A **golden
  vector**, and it closes all three `B3` rows at once: the pause's size (`B3.01`), where it falls
  (`B3.02`), and that a tight H-counter loop is what makes it visible (`B3.03`).

  The cart samples the full 9-bit H counter four times inside one scanline and differences the
  readings. A core that models the 5A22's per-line refresh shows one interval about ten dots longer
  than the others; a core that models none shows a flat sequence. **snes9x reports 64/75 dots — an
  11-dot excess starting at dot 80 — and RustySNES reports 63/65, flat.**

  Recorded rather than scored, deliberately. `docs/accuracy-ledger.md` scopes refresh out of
  RustySNES on the measurement that frame length is *already* the correct ~357,368 clocks without
  one, so adding a stall would make frame length wrong; and ares' own source says its refresh
  pattern is technically incorrect and only right on average. A reference that disclaims itself is
  not an oracle. The numbers go to the measurement channel and the verdict only names the shape.

  The probe was verified by injecting a synthetic 40-clock per-line stall into the emulator: the
  test flipped from variant 1 to variant 2 with an 11-dot excess in exactly the interval spanning
  the injected dot, and back again when the injection was removed. Without that check "flat" would
  have been indistinguishable from a probe too coarse to see anything.

  Two things it deliberately does not claim. Its resolution is one loop iteration — about 60 dots —
  so it brackets the pause rather than confirming `B3.02`'s multiple-of-8 rule. And a first version
  that stored only the low byte of H had its window run past dot 255, where the wrap looked exactly
  like a large pause; the full 9-bit reading turns that into a decreasing sample, which the test
  reports as an invalid measurement instead of as evidence.

- **`C14.03` — `$213E` bit 5, PPU1's master/slave mode pin.** A **golden vector**, never scored:
  the bit reports a board wiring input rather than emulator state, so a cart cannot distinguish
  "models the pin" from "returns zero here and always would". Recorded as a variant, the same call
  as `C14.01`/`C14.02` beside it. All three cores report variant 1 (clear). Isolating the bit also
  keeps it decoupled from `$213E`'s time-over, range-over and version bits, which *are* asserted
  elsewhere.

- **`A5.09`/`A5.10` — the `+1 m` and `+1 x` width penalties.** A memory operation costs one extra
  8-clock access when its operand class is 16-bit, measured as a differential over 16 repeats (32
  dots) with the *other* width bit held constant, so a core deriving one width from the other fails
  the one it gets wrong. Unlike `A5.20` these are measurable: both spans stay inside one scanline,
  clear of the long dots and the line-length approximation behind `T-06-A`.

  Their first draft recorded into slots 20-25 and read back the opcode sweep's baseline spans — the
  sweep computes `slot_base = 8 + index * 2` and owns slots 8-75, which no `record(...)` literal
  reveals. The channel's claimed ranges are now documented in `docs/accuracysnes-plan.md`.

- **`A3.06` — `(d,S),Y` escapes page 1 for its pointer read and bank-carries for its data read.**
  Two independent claims, so the test seeds a **distinct wrong answer for each**: `$5A` when both
  are right, `$99` when the pointer escaped but the bank carry was masked to 16 bits, and `$77` when
  the pointer read was confined to page 1. The failure message names which one broke, rather than
  reporting "not `$5A`".

  `DBR` is loaded before `S` is moved, because the `PHA`/`PLB` pair that sets it would otherwise
  push through the very page boundary under test. **`A3` is now complete.**

- **`A3.08` — `JSR (a,X)` escapes page 1.** The companion to `A3.07` and not a duplicate: `JSL`
  pushes three bytes and `JSR (a,X)` two, so they cross the page-1 floor from different alignments
  through different opcodes, and a core special-casing the escape per instruction can get one right
  and the other wrong. RustySNES did exactly that — see *Fixed* above.

  The canary at `$01FF` is the discriminator. The subroutine rebuilds the stack and jumps back
  rather than returning, for the reason `A3.07` records: after an escaping push `S` is below
  `$0100` and emulation mode forces the stack's high byte back to `$01`, leaving no return address
  to pull. No claim is made about which bank the pointer is read from, since banks `$00-$3F` alias
  the same WRAM below `$2000` and such a claim would be unfalsifiable.

- **Three Group A mode/addressing assertions (`A1.08`, `A1.09`, `A8.05`).** All four are
  `Documented`-tier and all four pass on RustySNES and on both cross-validation references.

  - `A1.08` — `CLC; XCE` entered with carry clear is a **no-op**, and the test checks the machine is
    genuinely untouched rather than just the flag: register widths and the full 16 bits of `X`/`Y`.
    A core that treats every `XCE` as a mode transition and re-initialises on it passes a
    flag-only check and corrupts the machine here.
  - `A1.09` — `REP #$30` cannot clear `m`/`x` while `E = 1`. The width bits are read **after**
    returning to native mode, because in emulation `P` bits 4 and 5 are `B` and unused, so the
    obvious in-emulation check reads a register that does not carry the answer.
  - `A8.05` — `MVN` wraps `X` inside the source bank and advances `Y` in the destination bank
    independently, with `Y` started away from its own wrap so one shared counter could not explain
    both results.

  Dossier coverage moves **241 → 244 of 443**.

- **Five more Group A addressing and mode assertions (`A1.10`, `A2.12`, `A4.07`, `A4.09`,
  `A4.10`).** `A1`, `A7` and `A9` are now complete. `A4` is not: two further tests were withdrawn
  on review (below), reopening `A4.04`/`A4.05`.

  - `A1.10` — `PLP` cannot clear `m`/`x` while `E = 1`, the third of the three paths the dossier
    requires to behave identically. Not redundant with the `REP` path (`A1.09`): a core that
    implements the emulation-mode pin as a mask inside `REP`/`SEP`, rather than as a property of
    `P`, passes that one and fails this.
  - `A2.12` — `[dp],Y` takes its bank from the pointer's third byte and carries out of it, ignoring
    `DBR`. Both candidate addresses are seeded so a failure says which way the core went.
  - `A4.07` — `JML [a]` uses the full 24-bit destination. The target bank is `$80`, which mirrors
    bank `$00` in this LoROM image, so a bank-ignoring core runs the same instructions and does not
    crash; `PHK` afterwards is what separates them.
  - `A4.09` — `PC` wraps inside its bank on an operand fetch. The instruction stream is assembled
    as data into WRAM and jumped to, because bank `$00`'s boundary is occupied by the vector table.
  - `A4.10` — **golden vector**, never scored: where a branch lands when its target crosses a bank
    boundary. Upstream marks the relative addressing modes `r`/`rl` *"XXX: untested"*, so no source
    vouches for the row. All three cores currently report variant 1 (the wrap). Both candidate
    landing sites are seeded with a jump home so either answer returns.

  Dossier coverage moves **244 → 255 of 443**.

  Also `A8.06` — in emulation mode `E = 1` forces `x = 1`, so the block-move offsets are 8-bit and
  confined to `$00xx`: an offset stepping past `$FF` wraps inside page 0 rather than advancing to
  `$0100`. The count is loaded before the mode switch, since the full 16-bit `C` cannot be written
  once `E = 1` but survives `XCE` unchanged. All three candidate source addresses are seeded
  distinctly, so the destination says which behaviour occurred.

  Two further tests (`A4.06`/`A4.08`, the `(a,X)` in-bank pointer wrap) were written and then
  **withdrawn on review**: banks `$00-$3F` mirror the same WRAM below `$2000`, so the wrapped and
  carried pointer addresses were literally the same bytes and every implementation passed. A
  discriminating test needs a ROM-resident pointer, which is a linker-layout change; recorded in
  `docs/accuracysnes-plan.md` and reopened in `T-04-A`.

- **`MVN`'s machine encoding puts the destination bank first (`A8.01`).** `MVN $00,$7E` assembles to
  `54 7E 00` — the reverse of how the mnemonic reads. Assemblers hide it, so a core written against
  the mnemonic rather than the opcode table copies in the wrong direction with nothing in the source
  looking wrong.

  The three bytes are emitted **by hand** rather than through `mvn`, because going through the
  assembler would test ca65's operand convention instead of the core's decoding. Both readings are
  made to land somewhere seeded: decoded correctly the move copies bank `$00`'s signature, decoded
  swapped it copies a byte planted at `$7E:8005`, and the two land on the same destination byte
  through the low-WRAM mirror. So the value found there says which way the operands were read, and
  a third seeded value distinguishes "did not move at all".

- **Emulation mode uses its own vector table (`A6.02`).** `COP` goes through `$FFF4`, sixteen bytes
  from the native `$FFE4`, and a core that keeps one set of vectors — or picks the table from
  something other than the E flag — lands in the wrong handler. Nothing about that is visible in
  ordinary code, since a game's `COP` handler is usually the same routine either way.

  **The cart could not see it either.** Both vectors pointed at the same trampoline, so a core taking
  the native table in emulation mode ran the same handler and passed. The runtime now gives the
  emulation `COP` vector its own trampoline and its own RAM pointer, which is what makes the two
  distinguishable at all. Both handlers are installed and live, so taking the wrong table is a wrong
  answer rather than a hang, and the failure code says which vector was used.

  `BRK` deliberately does not get the same treatment: in emulation, `$FFFE` is shared between `IRQ`
  and `BRK`, so a pointer behind it could not mean "the BRK handler" unambiguously, and splitting it
  would invent a distinction the machine does not have.

- **`(dp,X)`'s pointer fetch wraps inside bank `$00`, and `N`/`Z` are valid in decimal (`A2.09`,
  `A7.04`).** The pointer fetch is a separate piece of code in most cores and is the one place the
  `d,X` no-bank-crossing rule is easy to forget, because it happens before the mode looks like an
  indexed access at all. `D = $FFFF` + `X = $8000` + operand `$06` sums to `$18005`, and both
  candidate pointers — bank `$00`'s signature and bank `$01`'s — are aimed at bytes the test plants,
  so a core that crosses the bank reads a specific wrong value rather than garbage.

  The decimal-flag row is the 6502's behaviour the 65C02 fixed: `N` and `Z` describing the binary sum
  while the accumulator holds the decimal one. Each reading is chosen where the two answers differ —
  `$99 + $01` is `$00` decimal and `$9A` binary, so `Z` separates them; `$79 + $79` is `$58` and
  `$F2`, so `N` does. On an input where the two results share a sign and a zero-ness the flag models
  agree, and a reading taken there would assert nothing.

- **`TCS`/`TXS` set no flags, and `ORA [d]` reaches through a 24-bit pointer (`A1.09`, `A9.03`).**
  The stack pointer is not data, so moving a value into it does not describe that value — and a core
  routing every transfer through one flag-setting helper gets that wrong in a way nothing crashes
  on. Both instructions are handed the value the stack pointer already holds, so `S` is written with
  what was in it: a native-mode `TXS` is a full 16-bit write, and a test that put anything else there
  would be pushing its own `PHP` into ROM. `N` and `Z` are both checked, and both are planted at the
  value a flag-setting transfer would have to change — `$1FFF` has bit 15 clear, so checking `Z`
  alone would have been satisfied by a core that wrongly updates `N`. `TXA` moving `$8000` is the
  control, without which a core that sets no transfer flags at all passes.

  `ORA [d]` is the addressing mode most likely to be implemented as its 16-bit sibling with the data
  bank glued on, because for a pointer in bank `$00` the two are identical. This image is 128 KiB so
  they are not: the same pointer with `$01` in its third byte reads bank `$01`'s signature where a
  `(d)`-style fetch reads bank `$00`'s. A third reading with `$0F` already in the accumulator is
  what makes it an assertion about `ORA` rather than about a load — the first two start from zero,
  where the two instructions are indistinguishable.

- **Timer 2 counts eight times faster than timer 0 (`E3.06`), and `TEST` bit 0 halts timer 0
  (`E3.08`).** The first is a ratio rather than two measurements: both timers run over one interval,
  started by a single write and stopped by another, so whatever that interval was, `T2` must show
  about eight times what `T0` does. It catches the obvious mistake — one clock rate for all three
  timers — which no other timer test on this cart can see, because `E3.01`, `E3.05` and `E2.01` all
  use `T0` alone and a uniform-rate core passes every one of them.

  `E3.08` runs one interval twice with nothing changed but the halt bit: frozen, then running. The
  second half is what stops the first from being satisfied by a timer that never started. The bit
  halts all three timers on hardware; the test enables and reads only timer 0, and claims only that.

  **A fifth snes9x divergence**, and it is the fourth one's twin: `apu/bapu/smp/memory.cpp` has no
  `case 0xf0` at all, so the whole `TEST` register is discarded. `E3.10` already found that through
  bit 1 (the RAM write enable); `E3.08` finds it through bit 0. ares implements it explicitly and
  Mesen2 agrees with the cart. Declared with the citation in `scripts/accuracysnes/crossval.sh`.

- **Pitch scales the sample rate, and `$2000` is an octave above `$1000` (`E6.02`).** The first
  assertion from the S-DSP's pitch block, and the first on this cart to measure a *rate*. A single
  reading of `ENDX` cannot do that — it says "finished" or "not finished", which bounds a rate on one
  side only — so three tests play the same 384-sample voice and bound it on both: at `$1000` it has
  not finished after six waits and has after sixteen, and at `$2000` the same six waits are enough.

  Each pitch is read twice, at waits either side of where it finishes, because one reading bounds a
  rate on one side only. The result is two windows — `$1000` consumes 24-64 samples per wait,
  `$2000` consumes 64-128 — which both contain the documented rates, do not overlap, and cannot both
  be satisfied by a core that ignores the pitch register.

  The four waits were **found by bisection rather than calculated** — the first attempt placed them
  by arithmetic and the voice had already finished — and then deliberately moved *away* from the
  boundaries they found, so no verdict sits close to a finishing point. What the windows do not
  establish is that the factor is exactly two: a core scaling by 1.5 fits both, and excluding it
  would mean bracketing between adjacent waits, where every verdict is a hair's breadth from
  flipping. `docs/accuracysnes-plan.md` records the trade. All four tests agree on snes9x and
  Mesen2, on both images.

- **The power-on state is now reachable, and Group G reports out of it (`G1.02`, `G1.04`, `G1.08`,
  `G1.09`).** The battery runs long after reset, through a runtime that deliberately puts every PPU
  and CPU register into a known state — so until now the whole power-on half of Group G was
  unreachable: any test that ran in the normal battery was measuring the runtime, not the machine.

  What makes it reachable is a snapshot taken *before* that initialisation, extended here to cover
  three more facts. `XCE` exchanges C and E, so the first `clc`/`xce` of `reset` leaves the boot-time
  emulation flag in the carry for exactly one instruction; the runtime catches it there. `$4210` and
  `$4211` are read-to-clear, so their reset values are visible exactly once and only to whoever reads
  first, which is now the capture routine rather than a vblank poll.

  On top of that snapshot: `G1.04` asserts the CPU booted in **emulation mode** and that the word
  LoROM exposes at `$00FFFC` points at code beginning with `SEI`; `G1.02` asserts neither interrupt
  flag was already pending at reset. `G1.09` needed no new test — the existing CPU-revision golden
  vector *is* that assertion, and had simply never been mapped to it.

- **Reading a write-only register returns the CPU's open bus (`G1.08`).** `$4200` is read twice
  through two addressing modes whose last operand byte differs: absolute leaves the address's high
  byte on the bus (`$42`), long leaves the bank byte (`$00`). Same register, two answers — which is
  what makes the assertion about the bus rather than the register. A core returning a constant
  (`$00`, `$FF`, a stale value) gets at most one of the two right.

  The B bus is deliberately left out: reads of write-only PPU registers return the *PPU's* MDR, a
  different latch on the far side of that bus, and which one a given address exposes is a question
  this assertion does not settle.

- **`MOV dp,dp` is exempt from the store dummy-read (`E2.02`).** `E2.01` establishes the rule — a
  store reads its destination first, visible against a timer counter because reading one empties it.
  `$FA` is one of the two opcodes the rule does not apply to, so the same store through it leaves
  the counter alone.

  The two tests are **the same measurement with one instruction changed**, which is what makes this
  an assertion about `$FA` rather than about timers: a core that applies the dummy read uniformly
  passes `E2.01` and fails here, and one that omits it everywhere does the reverse. Timer 1 runs
  alongside as the vacuity guard, because otherwise "timer 0 still holds a count" and "the timers
  never started" are the same reading.

- **`MOVW dp,YA` dummy-reads its low byte only (`E2.03`).** Stores read their destination before
  writing it (`E2.01`), and a sixteen-bit store might reasonably do that twice. It does not. Pointed
  at the timer counters — where a read is *destructive* — the difference is directly visible: `$FD`
  comes back empty and `$FE`, one address higher and written by the very same instruction, still
  holds its count.

  Both timers are stopped before the instruction runs, because the counters are four bits and the
  reads are a handful of cycles apart. And `$FE`'s reading doubles as the vacuity guard: a timer
  that never advanced would leave both counters empty and make the `$FD` assertion meaningless.

- **`PSW.P` moves the direct page, for a store and for a read-modify-write (`E2.06`).** The bit is
  easy to implement for the obvious loads and stores and forget for the rest. A driver that sets `P`
  to keep its variables clear of the zero page then finds half its accesses going elsewhere, and the
  failure looks like memory corruption rather than an addressing bug.

  Two kinds of access, because one proves less than it looks: a `MOV` store, and an `INC dp`, which
  reads through `P`, modifies, and writes back through it. A core that resolves the page once at
  decode passes both; one that resolves it separately for the read and the write can fail the second
  while passing the first. The `[aa]+Y` pointer fetch the dossier also names is **not** covered.

  Both pages are seeded with different values first, so "it went to `$0120`" and "it went to
  `$0020`" are distinguishable answers rather than one answer and one absence — and the page-0
  assertion catches a core that writes *both*.

- **`DAS` reads the inverted sense of `C` and `H` (`E1.09`).** `DAA` adjusts when a flag is *set*;
  `DAS` adjusts when one is *clear*. A core that copies `DAA`'s conditions and merely flips the
  addition to a subtraction adjusts in exactly the wrong cases — invisible on the values anyone
  would pick by eye, wrong on almost everything else.

  Two runs of the same value differing only in `H`: with `H` set nothing happens to `$15`, with `H`
  clear it becomes `$0F`. `C` is set in both so the first condition stays out of the way, and `$15`
  trips neither of `DAS`'s value tests — so every difference between the two answers is the flag.
  Setting `H` needs an `ADC` with a nibble carry because nothing sets it directly, and clearing it
  needs `CLRV` (`E1.12`) because nothing else clears it either.

- **`DIV YA,X`'s overflow branch computes something else entirely (`E1.03`).** When `Y >= X << 1`
  the quotient will not fit in eight bits, and the instruction does not saturate: it produces
  `A = 255 - (YA - (X << 9)) / (256 - X)` and `Y = X + (YA - (X << 9)) % (256 - X)` — what the
  hardware's restoring-division loop leaves behind when it runs off the end. Games hit this, because
  the condition is `Y` against `X` rather than anything about the dividend.

  `YA = $4000, X = $10` gives `A = $DD` and `Y = $30`. The true quotient's low byte would be `$00`
  and a clamp would be `$FF` — neither is anywhere near `$DD`, which is what makes these
  discriminating numbers rather than a coincidence.

  **The duplicate-assertion gate rejected the first version of the mapping**, which claimed `E1.05`
  (`DIV`'s `V` flag) as well. That assertion already has its own test; the flag is still checked
  here as a supporting condition, but the coverage claim belongs where the dedicated test is.

- **Echo samples are stored with their bottom bit masked off (`E9.12`).** Each is written as a
  16-bit value ANDed with `$FFFE`, so the low byte's bit 0 is always zero whatever the mixer
  produced. A core that stores the sample verbatim leaves odd values in the buffer, and a driver
  that reads the buffer back — some do, to fade an echo tail by hand — sees numbers the hardware
  cannot produce.

  **The marker is `$FF`, which makes one assertion do two jobs.** Bit 0 of `$FF` is set, so a zero
  there afterwards proves both that a write happened *and* that what was written is even. An even
  marker would have left "nothing was written" and "an even value was written" indistinguishable —
  the same trap `E5.04` avoids by pairing with `E5.03`, solved here with one byte instead of a
  second test.

- **`EDL = 0` is a four-byte buffer, not the absence of one (`E9.06`).** The natural reading of a
  length of zero is that echo is off, or that the buffer is empty. It is neither: the DSP writes one
  sample's worth — four bytes — at the buffer's start, and does it again next sample. A core that
  treats zero as "skip the write" leaves the buffer alone; one that treats it as full size walks off
  across whatever follows `ESA`, which on a real driver is its own code.

  The test paints eight bytes and reads two back: byte 0 must have been overwritten, byte 4 must
  not. That **pair** separates "wrote four bytes" from both wrong answers — a core that skipped the
  write fails on byte 0, one that wrote further fails on byte 4. It is also the assertion `E9.10`
  quietly depends on, now stated in its own right.

- **`FLG` bit 5 stops the DSP *writing* the echo buffer, and nothing else (`E9.10`).** It is usually
  described as "echo disable", and it is not: the DSP goes on reading the buffer and feeding it
  through the FIR, it simply stops writing anything back. A driver that clears the buffer once and
  sets the bit gets silence; one that sets the bit over a buffer full of noise gets that noise
  **forever**, because the same samples circulate unchanged. A core that treats the bit as "echo
  off" produces silence in both cases and sounds fine until a game does the second thing.

  The test asks APU RAM rather than the ear: it paints a marker over the buffer's first bytes,
  waits, and reads them back — with writes disabled the marker survives, with writes enabled it is
  replaced by the zero the mixer is producing. `EDL = 0` is the smallest buffer, four bytes
  rewritten every sample (`E9.06`), which is what makes a short wait enough and puts the write
  exactly where the marker is.

- **Bit 7 of a CGRAM read is PPU2's open bus, not a sixteenth stored bit (`C3.03`).** A palette
  entry stores fifteen bits, so what comes back in the second `$213B` read's top bit is whatever
  PPU2 last drove — and the *first* read of the pair is what drove it. The bit therefore mirrors the
  low byte's bit 7.

  Two entries make that unambiguous: `$FFFF` stores as `$7FFF` with a low byte of `$FF`, and `$7F00`
  keeps a low byte of `$00`. Both hold the same fifteen real bits in the high byte, so any
  difference between the two readings is the open-bus bit and can be nothing else. A core returning
  a stored zero there gets the same answer for both.

- **`$213F` bit 7 is a field flag, and the test says so in both directions (`C3.09`).** It toggles
  once per frame and holds for the whole of one, so two readings a frame apart must *differ* and two
  readings two frames apart must *agree*. A core that toggles it per scanline, or on every read,
  sails through the first check and fails the second — which is why both are there.

  Third assertion to be built on `frame_step`: the battery is force-blanked throughout, and without
  rendering a frame there is no frame boundary to cross.

- **Lifting forced blank mid-frame closes the VRAM window on that write (`C2.12`).** Forced blank
  is what makes VRAM writable during the active display period, and the moment it is lifted the port
  stops accepting writes — on the same instruction, not at the next scanline. A core that closes the
  window lazily lets a handful of writes through after the program has turned the screen back on,
  which is the classic way for a tile to arrive corrupted in exactly one frame out of many.

  **Two wrong versions of the wait loop came first, and both are about lines rather than about the
  window.** The first fired the moment `$4212` said "not vblank" — but line 0 is a blanking line
  where VRAM is legitimately open, so it measured line 0 on one core and line 1 on another, and
  RustySNES was "wrong" for a reason that had nothing to do with the assertion. The second watched
  the V counter's **low byte** for 8-199, which on a 312-line PAL frame also matches lines 264-311:
  vblank, where the port is open. From 64 the alias would need line 320, which does not exist.

- **The sprite overflow flags clear at the end of vblank, and forced blank is not that event
  (`C7.09`).** `$213E` bit 6 latches when more than 32 sprites are in range on a scanline and is
  cleared once per frame, as rendering begins. A program that blanks the screen and reads the flag
  still sees the last frame's verdict — which is what makes the flag usable at all, since a driver
  reads it during vblank.

  **Both** flags, not one: 34 sprites of 16x16 exceed the 32-sprite range limit *and*, at two
  slivers each, the 34-sliver limit, so bit 6 and bit 7 latch together. Three readings, because each
  is meaningless alone: after a frame with those sprites the flags are **set**; after parking every
  sprite *without* rendering they are **still set**; after one more rendered frame with nothing in
  range they are **clear**. A core that clears them on any `$2100` write passes the first and fails
  the second; one that never clears them passes both and fails the third.

  The test also clears the OAM high table and sets `OBJSEL` explicitly, because `C1.03b` leaves
  `$AA` in the high table's first byte — size bits for sprites 0-3. Inheriting a large size would
  let a parked sprite reach back into the picture and make this test depend on the order the battery
  happens to run in.

  The parked sprites sit at `Y = 240`, not 224, and that is a finding rather than a detail: the
  visible height is not fixed. An overscan display shows 239 lines, and Mesen2's PAL run failed the
  third reading because sprites parked at 224 were still in range there — the flag it was asked to
  have cleared had been set again.

- **The battery can render a frame now (`frame_step`), and `C1.07` is the first assertion to need
  it.** The battery runs under forced blank throughout, which is what makes VRAM, OAM and CGRAM
  freely accessible — and it is why a whole class of assertions was unreachable: the ones about
  things that only happen when rendering *starts*. `C1.07` is the OAM address reload on `$2100`
  bit 7's falling edge, and a straight-line version of it returned the walked-to byte on all three
  emulators, because the transition only *arms* the reload and the next rendered frame applies it.

  `frame_step` clears blank from inside vblank, waits for rendering to begin and then for the
  following vblank, and blanks again before returning — which is also what makes the `$2138` read
  afterwards safe, since an OAM read during active display is unreliable (`C1.08`). Anchoring both
  ends matters: clearing blank mid-scanline would resume rendering somewhere unrepeatable.

  It is available to the rest of that class: `C7.09` (the sprite range/time-over flags clear at the
  end of vblank but not during forced blank) and `C9.05` (the mid-frame overscan hazard) are next.

- **A sprite's name-select bit reaches a different part of VRAM (`C7.11`).** Two sprites with the
  *same tile number*, one with the attribute bit set: the character address gains
  `(NameSelect + 1) << 12`, so the second reads from `$1000` words on — where the font never
  reached — and draws nothing. A core that ignores the bit draws two identical glyphs. Blank is a
  legible answer here only because the other sprite is not.

  **A `C7.12` scene (OBJ interlace on a tall sprite) was attempted and produced three different
  hashes on three emulators** — the only three-way split any scene has produced. That is not three
  cores disagreeing about interlace: the picture *alternates every frame*, and the capture protocol
  (hash the fourth sighting of an eight-frame window) does not pin frame parity, so each host lands
  on whichever field its own counter happens to be on. Publishing a scene only on a known field is a
  change to `run_scenes` rather than to a scene, and it would unblock the interlace half of `C9` as
  well. Written up in `docs/accuracysnes-plan.md`.

- **`VxOUTX` is sampled before the per-voice volume (`E7.16`).** A voice turned all the way down
  still reports the same `OUTX` it reported at full volume, because the register sits after the
  envelope and before `VxVOLL`/`VxVOLR`. A core that reads it off the mixer's input returns zero,
  and a driver using `OUTX` to watch a sound's progress loses it the moment the music fades that
  channel out. Its control is `E5.03` — the same voice with the volume left at `$7F`, reading the
  same band.

  **An `E8.03` test was written and dropped.** The dossier row says a `KON` "clears `ENDX` even when
  suppressed", and the obvious reading is the `KOFF`+`KON` pair `E8.04` already uses. All three
  emulators leave `ENDX` set — three failing identically is a broken test, not three broken cores,
  so "suppressed" there is not `KOFF`. The likeliest candidate is the key-on *collapse* cases, where
  two `KON` writes land inside the same 16 kHz polling window and one is dropped; that is a
  suppression internal to the DSP's scheduling rather than one a program asks for, and it needs the
  collapse cases built first. Parked with the measurement in `docs/accuracysnes-plan.md`.

- **`KOFF` outranks `KON`, and mute is downstream of `VxOUTX` (`E8.04`, `E9.17`).** The two key
  registers are not symmetric: `KON` is a write-triggered *edge* that starts a voice once, while
  `KOFF` is a *level* the DSP consults every time it looks. So a driver that sets `KOFF` and then
  writes `KON` without clearing it first gets silence — a real and confusing way to lose a note.
  `E8.04` writes both back to back with nothing between them and asserts the envelope reaches zero;
  its two controls are already in the battery (`E7.10`, no `KOFF` at all, reads `$7F`; `E7.08`,
  `KOFF` alone, reads `$00`).

  `E9.17`: `FLG`'s mute bit silences the *mixer*, so everything upstream carries on — the envelope
  steps, the sample decodes, and `VxOUTX` still reads what it read before. A core that implements
  mute by zeroing the voices makes `VxOUTX` go quiet too, and a driver watching it to decide when a
  sound effect has finished waits forever.

  `Voice::late` now takes a *slice* of register writes rather than one, because a test about two
  registers written together depends on nothing running between them.

- **Two PPU port rules that only a program can see (`C1.03`, `C3.07`).** `C1.03`: the OAM *high*
  table commits every byte as it is written — the pairing rule that buffers a lone byte belongs to
  the low table only. A core that applies it everywhere loses every odd write to the high table,
  which is where the X bit 8 and size bits live, so sprites go missing or change size depending on
  how a driver happened to batch its writes. `C3.07`: `OPHCT` and `OPVCT` have **independent** read
  flipflops, so reading `$213C` says nothing about what `$213D` returns next; with one shared
  flipflop a driver that reads H then V gets a vertical position of 0 or 1 for the whole frame.

  `C3.07` latches once and never re-latches, so both `$213D` reads sample the same frozen number
  and the comparison is byte-exact rather than approximate. Its retry loop is the vacuity guard:
  `OPVCT`'s high byte is a single bit, so the test only separates the two behaviours while the low
  byte is something other than 0 or 1, and it waits for a scanline where that holds.

- **Mode 4's two layers (`C5.05`).** BG1 is 8bpp and BG2 is 2bpp — three bits of colour depth
  apart, from the same tilemap — and BG3 exists only as the offset-per-tile table, left inert here
  so the scene is about the mode's layers rather than about OPT. A core that reuses mode 3's depths,
  or mode 2's layer set, renders both layers at the wrong depth.

- **A BG's extra tilemap screen is placed to the right when it is 64 wide (`C5.12`), and the canvas
  had to grow a marker before that could be seen.** `scene_second_screen` fills the screen after the
  canvas with tiles `$20`-`$2F` at a flat palette 5, varying with the column and nothing else — the
  canvas draws 64 glyphs from `$21` upward with a row-derived palette, so the tile numbers overlap
  and the two are still nothing alike as pictures — so a 64x32 map scrolled 256 pixels renders
  something no other scene renders. Without it the canvas's
  own horizontal period divides 256 and the scroll is invisible, which is what the first attempt
  rendered: a picture hash-identical to the plain canvas.

  A second attempt then wrote the scroll to `$210F`, which is BG2HOFS rather than BG1HOFS, and
  produced another stable, three-way-agreed hash identical to an existing scene's. **Two wrong
  scenes in a row, both caught by the same check** — comparing the new hash against the committed
  goldens — and neither by anything else.

- **Mode 7 reads neither `BG1SC` nor `BG1NBA` (`C5.13`), declared as an equivalence.** The scene
  points both registers at nonsense and must render exactly what `c11-mode7-identity` renders —
  Mode 7 has its own fixed VRAM layout, byte-interleaved with characters at `$0000`, and consults
  neither. Its hash duplicating another scene's is the *assertion* here rather than a warning, which
  is the difference between a declared equivalence and the accidental collisions that caught two
  broken scenes this week. An equivalence also survives a change to the shared Mode 7 canvas, where
  a second committed hash would not.

- **Pure black is unreachable through direct colour (`C12.02`).** Pixel value 0 is transparent in
  every mode, direct colour included, so the backdrop shows where a naive RGB decode would render
  black. The scene sets a loud red backdrop so a transparent pixel is legible as one.

- **Mode 7's 13-bit origin mask, after two dead ends (`C11.02`).** `ORG.X` is
  `(M7HOFS - M7X) AND NOT $1C00`, and making that visible took getting two things right at once.
  **Screen-over must be TRANSPARENT**: `$1C00` is `7 * $400`, so the mask only removes multiples of
  1024, and a wrapping 1024x1024 map removes those anyway — the rule is invisible by construction
  while screen-over is wrap. And **`M7HOFS` is thirteen bits *signed***: `$1C40` has bit 12 set and
  is therefore negative, the origin is off the map to the left with or without the mask, and the
  scene renders a blank screen either way. `$0C40` is the same low bits with bit 12 clear: masked it
  is 64 and on the map, unmasked it is 3136 and off it, so the two possibilities are a picture and a
  blank screen.

  **Both dead ends were caught by the hash colliding with an existing scene's** — the first with
  `c11-mode7-identity`, the second with `c8-window-inverted-empty-is-full` (all backdrop). Three
  emulators agreeing, a stable hash and a plausible name are not evidence that a scene shows
  anything; checking a new hash against the committed goldens is.

- **Sprite colour math applies to palettes 4-7 and to nothing else (`C8.01`).** A scene with two
  identical sprites side by side, one in palette 2 and one in palette 6, colour math enabled for
  OBJ against the fixed colour: only the palette-6 sprite blends. It is an errata rather than a rule
  anyone would guess, and a core that applies the maths to every sprite blends both — a picture that
  looks perfectly reasonable until it is compared with one that is right.

- **The undocumented sprite size pairs, as two scenes (`C7.10`).** `OBJSEL` pairs 6 and 7 are the
  only ones whose members are not square — 16x32/32x64 and 16x32/32x32 — and no official document
  lists them. The two scenes place one sprite of each size side by side and differ from each other
  by a single `OBJSEL` bit: the large member's height, 64 against 32. A core that stops its size
  table at 5, or repeats an earlier pair to fill the gap, draws squares.

- **Group G opens: the cartridge itself (`G1.10`, `G1.12`, `G1.14`).** The one group whose subject
  is not a chip. `G1.10`: the header's checksum and complement must XOR to `$FFFF` — the pair every
  emulator uses to *find* a header at all. `G1.12`: a LoROM header sits at `$00:FFC0`, and its
  map-mode and ROM-size bytes say which image it belongs to. `G1.14`: LoROM decodes a bank as
  `((bank & $7F) << 15) | (addr & $7FFF)`, which is two claims — banks `$80`+ mirror `$00`+, and
  each bank maps its own **32 KiB**. The per-bank signature bytes this image has carried since the
  beginning exist for the second one, and it is why the cart is 128 KiB rather than the minimum 32.

  `G1.11` then adds up **every byte of the image** — all 131,072 — and compares the total against
  the checksum in the header. What that really tests is the memory map: reaching every byte means
  walking all four banks through the LoROM formula, so a decode that mirrors a bank, drops one, or
  gets the stride wrong produces a different total. `G1.14` proves the formula on three sample
  bytes; this proves it on all of them. It does **not** validate the checksum algorithm, and the
  test says so: the generator computes it the same way, so a shared misunderstanding would agree
  with itself.

  These assert what every other test silently depends on. Every assertion in every other group runs
  out of a ROM addressed through that formula, so if the formula were wrong the failures would
  appear anywhere but here — and a mapping bug that happens to be self-consistent produces a battery
  that passes and a commercial ROM that does not boot.

- **Group F opens with one test, and it found a defect the frontend could not (`F1.02`).** A
  standard pad returns 1 once its sixteen data bits are gone, which is how software tells a pad from
  a multitap or a mouse. The test checks the sixteen data bits first as a vacuity guard — with
  nothing held they must all be 0 — so a core that returns 1 to everything fails there instead.

- **Four more 65816 addressing rules (`A2`, `A3`), and a fourth snes9x divergence.** `A3.07`:
  `JSL` escapes page 1 — its third push lands at `$00FF`, not wrapped to `$01FF`. `A3.09`: so does
  `PHD`. `A2.07`: `(dp),Y` carries into the *next bank* once the pointer is loaded, so `$FFFF + 2`
  reads `$7F:0001` and not `$7E:0001`. `A2.10`: `PEI` reads its pointer straight through the page
  boundary at `E=1` with `DL = $00`, because it is a "new" instruction — the same old-versus-new
  split as `PLD` against `PLY`, on the fetch side rather than the stack side.

  **`A2.10` fails on snes9x.** `DirectIndirectE1` in `cpuaddr.h` fetches the pointer with
  `Registers.DL ? WRAP_BANK : WRAP_PAGE`, applying the old-instruction wrap rule, and `PEI` shares
  that helper with the genuinely old `(d),Y` modes. snes9x's own comment in `OpD4E1` — "PEI is a new
  instruction, and so doesn't respect the emu-mode stack bounds" — shows it distinguishes
  new-instruction behaviour for the *stack* but not for the fetch. Mesen2 and RustySNES agree with
  the cart.

  **`A3.07` does not exercise `RTL`, and that is a measurement rather than an omission.** The first
  version called `JSL` and let the subroutine return; RustySNES, snes9x and Mesen2 all *hung*. Three
  implementations failing identically is the signature of a broken test: after the escaping pushes
  `S` is `$00FE`, emulation mode forces the stack's high byte back to `$01` at the next instruction,
  and there is no return address left to pull. The subroutine now rebuilds the stack and jumps back
  instead of returning.

- **The dispatch table is 24-bit, so a test body can live in any bank.** Bank `$00` filled up twice
  in a week and the workarounds were running out: the SPC700 images moved to bank `$01`, then the
  font followed, and what remained was the one thing that could not move — every test body, because
  `_test_entries` held 16-bit addresses. It now holds `.faraddr` entries, `call_indirect` is a
  `JMP [abs]`, and every test exits with `jml test_restore`. **Group E's bodies moved to bank
  `$02`**, which leaves bank `$00` with about 7 KB free and group E with a bank of its own.

  Group E is the group that can move: its tests reach the runtime's variables through long
  addressing and the APU ports through `$21xx`, which LoROM mirrors into every bank. What it could
  *not* keep was `jsr apu_upload` — a bank-local `jsr` from bank `$02` lands at the same 16-bit
  address *in bank `$02`*, which is not a subroutine but whatever bytes are there. Every test in the
  group reported a garbage verdict until it became `jsl apu_upload_far`. The generator now rejects a
  bank-local `jsr`, or a `jmp` to anything but a cheap local, in an out-of-bank body: the failure is
  silent and total, so it is a build error rather than a comment.

- **The SPC700's vector table, and the port-latch strobes (`E2`, `E3`).** `E2.08`: `TCALL n`
  vectors through `[$FFDE - n*2]`, counting *down* from the top of the address space — a stride and
  a direction that are both easy to get backwards, and a driver using `TCALL` for its dispatch table
  lands somewhere arbitrary if either is. `E2.09`: `BRK` has no vector of its own; it shares
  `TCALL 0`'s, so installing one handler installs both. `E3.03`: `$F1` bit 5 is a *strobe*
  that clears a CPU-to-APU input latch, so a driver can drop a stale command without a second write;
  a core that ignores it leaves a command the driver believed it had discarded sitting in the port.
  Only the immediate clear is asserted, and only for port 3 — port 2's latch holds `$00` here, which
  is indistinguishable from cleared, and the non-persistence half needs a mid-program cart-to-APU
  handshake the upload mechanism does not have. The test's doc comment says which two thirds of the
  dossier row it does not reach.

  Both vector tests plant the *right* handler at the slot under test and a different one either
  side, so a miscounted vector produces a **wrong answer** rather than a hang. That distinction
  matters: a test whose only failure mode is the timeout reports SKIP, which says the APU did not
  answer rather than that it answered wrongly.

  **`E2.09` broke `E4.02` on its first run, and the coupling is real.** `BRK` sets the `B` flag and
  nothing on the SPC700 clears it short of a `POP PSW`, so the handler left `B` set for the whole
  rest of the battery — and `E4.02`, which reads the register state the IPL hands over, saw `$1A`
  where it expects `$0A`. The handlers now restore `PSW` before handing back. A test that changes
  processor state every later test can see has to put it back.

- **Three SPC700 I/O registers (`E3`), and a third snes9x divergence.** `E3.04`: the boot ROM is an
  *overlay*, not a region — a store to `$FFC0` reaches the RAM underneath whether or not the ROM is
  mapped over it, and is simply invisible until the overlay is switched off. An emulator treating
  that range as read-only while mapped loses a driver's data with no error anywhere. `E3.05`:
  `TnDIV = $00` selects a divider of **256**, the slowest setting, not zero and not one; read as a
  literal zero a driver's tempo is wrong by two orders of magnitude. `E3.10`: `TEST` bit 1 gates
  every write into APU RAM, and with it clear stores execute, take their cycles, and change nothing.

  **`E3.10` fails on snes9x, and the citation is unusually clean:** `SMP::mmio_write` in
  `apu/bapu/smp/memory.cpp` has no `case 0xf0` at all, so writes to the `TEST` register fall through
  the switch and are discarded. Mesen2 and RustySNES both implement it. No game depends on it —
  which is exactly why it is the kind of register an emulator leaves out and a test ROM finds. It
  joins `B5.05` and `A5.S17` as a recorded reference divergence with a source citation, not a
  lowered bar.

- **The font moved out of bank `$00`.** Bank `$00` filled up again — this time with test *bodies*,
  which cannot move: the dispatch table holds 16-bit entry points. The font can, because the runtime
  already reads it with long addressing. It is declared after `BANK1` in the linker config because
  ld65 fills a memory area in declaration order and `BANK1` has a fixed start at the bottom of it.

- **Four SPC700 instructions whose behaviour contradicts their names (`E1`, `E2`).** `E1.12`:
  `CLRV` clears the half-carry as well as the overflow flag, and nothing else on the SPC700 clears
  `H` — so a decimal routine using it to prepare for `DAA` depends on the undocumented half.
  `E1.08`: `DAA` applies two adjustments, and `$9A` trips both, wrapping to `$00` with carry set.
  `E1.10`: `TSET1` reports `N`/`Z` from a *comparison* of `A` against the target's old value, not
  from the result — visible when the operands are equal and the result is not zero, where the
  hardware says "equal" and a result-based core says "not zero". `E2.07`: a `CALL` pushes the
  address it will return to, not that address minus one; a core copying the 65816's convention
  returns into the middle of the following instruction.

  `E2.07`'s subroutine never returns. Popping the pushed bytes is the only way to see them, and
  having seen them there is nothing left to return with — so it reports and ends the program. Its
  expected value is computed from the program's own layout rather than written down.

- **The boot ROM itself (`E4`).** The cart has used the IPL handshake since Group E existed;
  these are the first tests *of* it. `E4.01` walks all 64 bytes of `$FFC0`-`$FFFF` and reports both
  their sum and a position-weighted rolling value — the sum alone would accept any permutation,
  which is precisely the mistake a hand-transcribed listing makes. `E4.04`: an idle boot ROM
  announces `$BBAA`, the one piece of APU state a game can check before it has uploaded anything.
  `E4.02`: the IPL hands a program `A`/`X`/`Y` zero and a defined `PSW`.

  `E4.04` polls the **second** byte and asserts the first, which makes it an ordering claim as well.
  The boot ROM stores `$AA` to port 0 and then `$BB` to port 1, two separate instructions, so once
  `$BB` is visible `$AA` must already be. The first version did it the other way round and landed in
  the gap between the two stores; snes9x failed it, correctly. A driver that polls for `$AA` and then
  trusts port 1 has the same bug.

  **`E4.02` found the documented handoff state to be incomplete.** RustySNES, snes9x and Mesen2 all
  hand over `PSW = $0A`, not the listed `$02`: `Z` as described, plus `H` left set by the boot ROM's
  own arithmetic. Three independent implementations agreeing that a listing is incomplete is worth
  recording — but not worth scoring against, since asserting `$0A` would be grading a measured value
  with a citation that says something else. The test asserts the documented bits (`Z` set, `N`/`V`/
  `I`/`C` clear) and publishes the whole byte to the measurement channel, which exists so a number
  can be reported without being scored.

- **A BRR filter and an envelope floor (`E5.05`, `E7.14`).** `E5.05`: filter 1 is a recurrence,
  not a scale factor — it keeps most of the previous output and adds the new sample, so a constant
  input converges on a fixed point an order of magnitude above itself. A core that ignores the
  filter field reports `E5.03`'s single-digit answer and fails by an enormous margin. `E7.14`: a
  linear-decrease GAIN applied to a freshly keyed-on voice underflows on its very first step, and
  the hardware holds it at zero; a core that lets the eleven-bit envelope wrap turns silence into
  maximum volume.

  **`E5.06` — the fifteen-bit wrap — was attempted and is not reachable this way, and the attempt
  is worth more than the test would have been.** The constant-input trick the other BRR tests rely
  on works because a non-overflowing filter converges on a fixed point: the output stops changing,
  so it does not matter which sample the cart catches. Wrapping destroys exactly that property —
  the output becomes a sawtooth cycling through the whole range, and `VxOUTX` reports wherever it
  happens to be. The two reference emulators returned `$E1` and `$D0` from the same image, agreeing
  only that it was negative, and that agreement was luck. The rule this leaves behind is in
  `docs/accuracysnes-plan.md`: **an `OUTX` assertion is only valid where the output is provably
  stationary**, and every committed one now says so in its own comment.

- **BRR decoding arithmetic, as three tests that are each other's controls (`E5`).** All three play
  a sample whose every nibble is identical, so filter 0 decodes the same value every time and the
  output is a constant the cart can read without racing the sample clock. `E5.02`: nibbles are
  signed, so a sample of `$8` nibbles must come out *negative* — read as unsigned it becomes a DC
  offset and a wrong waveform rather than silence. `E5.03`: the same shift with `+7` nibbles must
  come out positive and non-zero. `E5.04`: shift 13 is not a shift at all but a documented special
  case that discards the magnitude and keeps the sign, so the same `+7` nibbles must collapse to
  zero — a core applying it literally produces an enormous sample where the hardware produces
  silence.

  Zero is also what a voice that never started looks like, which is exactly why `E5.03` is in the
  set rather than folded into the others.

- **Two more things a playing voice makes visible (`E9`).** `E9.18`: `FLG`'s reset bit is not a
  gentle stop — it behaves as `KOFF = $FF` with the envelopes forced to zero, so a voice held at a
  direct gain nothing else would move reads zero afterwards. `E9.04`: a voice switched to noise
  *still decodes its BRR sample underneath*, so an end-without-loop block silences it anyway; a
  driver parking a noise voice on whatever sample address happens to be there gets silence at an
  unpredictable moment, and a core that skips decoding for noise voices never reproduces it.

- **The envelope generator, four ways (`E7`).** With a voice playing, the envelope is finally
  observable: `E7.15` attacks at rate `$F` to full scale and reads exactly `$7F`, which is the one
  assertion that pins `VxENVX` as `E >> 4` of an eleven-bit envelope — a core carrying a full byte,
  or shifting by three, reports `$FF` or `$FE` and is otherwise indistinguishable. `E7.11` and
  `E7.01` are a pair on custom GAIN: linear increase at rate `$1F` climbs from zero to full scale,
  and the same ramp at rate 0 does not move at all, because the rate table's first entry means
  "never" rather than "as fast as possible". `E7.08` key-offs a voice held at full direct gain and
  finds the envelope at zero, which only the release path can do.

  The pairs are load-bearing. On its own, "the envelope did not move" is also what a core with no
  GAIN ramps reports, and what a voice that never started reports.

- **The SPC700 program images moved out of bank `$00`.** They are pure data read through a 24-bit
  pointer, they are several hundred bytes each, and bank `$00` — which also holds the runtime, the
  font, every test body and the catalog — ran out. They now link into an `APUDATA` segment in bank
  `$01`, which is what makes the rest of Group E affordable at all.

- **The S-DSP plays a sample, and five assertions follow from that (`E5`, `E7`).** Everything the
  cart could previously say about the DSP was said by writing a register and reading it back, which
  proves the address latch decodes and nothing else. These upload a program that plants a BRR
  sample and a sample directory in APU RAM, points a voice at it, keys it on, and reports what the
  DSP says afterwards: `ENDX` sets when a block carrying the end flag is decoded (`E5.09`); the loop
  bit alone does not set it, because code 2 is an ordinary block (`E5.08`); end-without-loop forces
  release with a zero envelope even against a direct GAIN that nothing else would move (`E5.07`); a
  directory entry is at `DIR*$100 + SRCN*4`, checked through entry 1 with entry 0 pointed at silence
  so a wrong stride has somewhere defined to land (`E5.11`); and a direct GAIN *is* the envelope, so
  `VxENVX` reads back the byte written (`E7.10`).

  Two of those pairs are the point. `E5.07` and `E7.10` differ by one header bit and assert opposite
  values from the same read, which separates "the envelope works" from "the envelope is stuck at
  what was written". `E5.08` and `E5.09` differ by one header bit and assert opposite `ENDX` values,
  which is what keeps `E5.08` from also passing on a voice that never started.

- **A settle before reading `ENDX` (`E9.19`), and the reason is a hazard the dossier already
  enumerates.** `E9.19` wrote `$FF` to `ENDX` and read it straight back. `ENDX`, `OUTX` and `ENVX`
  are written back from an internal buffer once per sample, and a CPU write landing one or two
  clocks before that writeback is lost (`E7.17`) — so the read was racing a documented window, and
  its answer depended on which DSP clock the write happened to land on. It was not hypothetical: a
  single extra byte elsewhere in the battery moved the write into the window and flipped the result
  on snes9x at PAL timing while leaving NTSC alone. The test now waits a few samples and asserts the
  same thing about the same write, minus the coin flip.

- **`E2.05` poisons the address a wrong answer would read.** As shipped it asserted only that
  `$0101` — where a 16-bit `dp+X` sum would land — did not happen to hold the same marker as the
  correct address, which is a weaker claim that quietly depends on APU RAM's power-on state. It now
  writes a third value there first.

### Changed

- **`E7.09` and `E8.07` no longer drift when anything ahead of them in the battery moves.** Adding
  `C1.08` — a PPU test — shifted the DSP poll phase for every APU test after it, turning `E7.09`
  from pass to fail and making `E8.07` report a different variant on the PAL image than on the NTSC
  one. `E7.09` compared two separately-uploaded `ENVX` readings for exact equality; two uploads'
  key-off-to-read windows can land one DSP sample apart, so it now allows the one sample (±8 at the
  fixed release step) and still fails when a rate-consulting release is injected. `E8.07`'s variant
  was a phase sample by construction — the row's own subject is that a short `KOFF` pulse is seen or
  missed depending where the poll falls — so it now reports only that the measurement was taken and
  lets its recorded reading speak. The NTSC/PAL drift gate is what caught the second one.

- **`hdmaen_latch_test` / `hdmaen_latch_test_2` golden framebuffers re-blessed** as a direct,
  intended consequence of the V-IRQ fix: both ROMs gate their `STA $420C` on a V-only IRQ, so
  firing once per frame rather than on every dot changes which dot the write lands on and hence the
  banding realization. Legitimate only because these goldens are regression snapshots of our own
  deterministic output — undisbeliever documents the ROM as *not stable* on real hardware — and
  because the change is externally corroborated (ares' edge detector; Mesen2 and snes9x both pass
  `B4.08`/`B4.12`, which RustySNES failed). Isolated by reverting the IRQ change alone and
  confirming the goldens returned. Rationale recorded in `docs/scheduler.md` §H/V-IRQ.

- **`E8.07` is now a golden vector, because its outcome is phase-dependent.** It shipped scored, and
  should not have. The `KOFF` pulse is about five SPC cycles wide against a poll interval of about
  sixty-four, so a poll falls *inside* it roughly one time in twelve — and which way that goes
  depends on where the DSP happens to be when the test runs. Adding an unrelated test earlier in the
  battery shifted the phase and the assertion failed, with nothing about the emulator having
  changed.

  The row states its outcome flatly, but the mechanism it describes is the one `E8.05` and `E8.06`
  hedge as *"usually"* — and a claim that is usually true is not one a battery can score. It now
  records the envelope and names the two shapes. `E7.08` remains the counterpart that separates "a
  poll saw the `$FF`" from "the core acts on the write".

## [1.20.0] "Aperture" - 2026-07-15

Phase A of the new UI/UX-parity ladder: brings the wasm demo's menus and the desktop frontend's
peripheral/overscan/inspection controls up from placeholder or dormant to actually functional,
closing several gaps found in a systematic audit against RustyNES's own frontend.

### Fixed

- **Wasm demo: `Cheats`/`Debugger overlay` menu items now real, not placeholders** (Phase A of the
  new UI/UX-parity ladder). `.github/workflows/web.yml`'s `trunk build` gained
  `--features cheats,debug-hooks` — both are pure computation with zero wasm-incompatible
  dependencies (confirmed via a real `cargo check --target wasm32-unknown-unknown` and a full
  local `trunk build` reproducing the exact CI command), and had simply never been added to the
  deployed demo's feature set, not excluded for any architectural reason. The hosted demo's
  Tools → Cheats and Debug → Debugger overlay menu items now show their real controls instead of
  a `(rebuild with --features ...)` label. Verified: the built demo's gzip size (2.96 MiB) stays
  well under the 5 MiB budget gate (2.04 MiB headroom), and compile-time `#[cfg]` proof —
  building with these features on means the sibling `#[cfg(not(feature = "..."))]` placeholder
  branches are provably absent from this binary. `scripting`/`netplay`/`retroachievements` remain
  genuinely unavailable on wasm (`mlua`/native sockets/`rcheevos` FFI are not wasm-portable) —
  their placeholders are honest, not touched by this fix; see `docs/frontend.md`'s "hosted demo
  page" section for the full disposition and `to-dos/ROADMAP.md`/the approved UI/UX-parity plan
  for what fixing those three would actually require.

- **A real, separate finding surfaced while scoping the above**: `docs/frontend.md`'s own "Status"
  line claimed controller port 1 had "keyboard + gilrs gamepad" input, but `gilrs::Gilrs` is never
  actually instantiated anywhere in `rustysnes-frontend` — confirmed via `input::gamepad_button`
  (the gilrs-button-name mapping function) having zero callers. Port 1 is keyboard-only today.
  Corrected the doc; wiring real gamepad support is a genuinely separate, larger prerequisite
  (a live `Gilrs` instance + per-frame event polling), not something silently expanded into this
  fix — it's also what blocks Super Multitap sub-pad 1-3 host input specifically, tracked
  separately in the UI/UX-parity plan's backlog.

### Added

- **Live host-input capture for Mouse/Super Scope** (Phase A.2 of the UI/UX-parity ladder, new
  `crate::peripherals`). `config.port2_peripheral`'s Settings selector already wired the emulated
  hardware correctly since `v0.9.0`; now `egui::Context`'s pointer state actually drives it once
  per frame — `EmuCore::set_mouse` from pointer delta + left/right buttons, `EmuCore::set_superscope`
  from an absolute aim position mapped through the present path's own letterbox transform
  (`Gfx::letterbox_scale`, exposed `pub(crate)` for this reuse rather than re-derived) into SNES
  pixel space, with trigger/cursor/turbo on left/right/middle mouse buttons. Mirrors
  `rustysnes-libretro`'s own already-verified `poll_port_input` translation. Portable to wasm on
  purpose (no `target_arch` gate) — both the pointer API and the `EmuCore` calls are already
  platform-agnostic, so the hosted demo gets this too. 5 real unit tests cover the pure
  coordinate-mapping math directly (centered/corner/pillarboxed/off-window cases), not just
  "compiles."

- **View → Hide Overscan** (Phase A.3 of the UI/UX-parity ladder). Crops the trailing "overscan"
  scanlines a real 4:3 CRT wouldn't reliably show — the SNES's own `SETINI` register extends the
  standard 224-line display to 239 lines (`rustysnes_ppu`); the new `app.rs`'s `crop_overscan`
  crops exactly that extra 15-line extension back off, once per frame, after every other buffer
  transform (HD-pack compositing, run-ahead, the `emu-thread` build's `PresentBuffer` handoff)
  has already settled on the bytes actually being presented. Crops a FRACTION (`15/239`) of the
  current height rather than a fixed pixel count, so it stays exact under an HD-pack integer
  upscale too. Presentation-only, additive, `false` by default — byte-identical to every prior
  release when unchanged. 3 real unit tests cover native resolution, an HD-pack-scaled
  resolution, and that the kept bytes are untouched.

- Debug → ROM Info panel: a read-only CRC32/SHA-256/header decode of the loaded cart
  (`crates/rustysnes-frontend/src/debugger/rom_info_panel.rs`), captured once per ROM load/close
  rather than every frame. `rustysnes_cart::header::Header` gained a decoded `title: String` field
  along the way.

## [1.19.0] "Afterburner" - 2026-07-15

Fifteenth release of the RustyNES-parity roadmap: an optional PGO/BOLT pipeline for the
shipping `rustysnes` binary, deliberately last per the plan (after mobile-specific hot-path
work landed, so the profile isn't invalidated).

### Added

- **PGO/BOLT pipeline** (Mobile-track-adjacent, deliberately last per the RustyNES-parity
  roadmap): `scripts/pgo/run.sh` instruments, trains against the committed permissive ROM corpus
  (via a new `crates/rustysnes-test-harness/src/bin/pgo_trainer.rs` binary — the `gilyon`
  CPU-instruction suite plus a handful of `undisbeliever` HDMA-glitch/INIDISP-hammer ROMs, chosen
  for control-flow breadth beyond the single steady-state `headless_frame` bench ROM), and
  rebuilds the shipping `rustysnes` binary with the merged profile. New
  `.github/workflows/pgo.yml`: `workflow_dispatch` + release-tag push only (never the PR gate —
  an instrument+train+rebuild cycle is far too slow for that). Promotion requires **both** a
  measured `>3%` Criterion speedup over the plain release build **and** a byte-identical re-run
  of the full `--features test-roms` oracle under the PGO-merged profile, citing
  `docs/adr/0004`'s determinism contract — never promotes on speed alone. An optional Linux-only
  BOLT post-link stage chains onto an already-promoted PGO binary, best-effort.
- Fixed a real, latent CI gap found while building this: `rust-toolchain.toml` didn't list
  `llvm-tools-preview`, and `dtolnay/rust-toolchain` silently ignores the `rust-setup` composite
  action's own `components:` input whenever a `rust-toolchain.toml` file exists in the repo (the
  same behavior already found and fixed for `ios.yml` in `v1.16.0`) — without this, `cargo-pgo`'s
  `.profraw`/`.profdata` merging would have silently failed to find the component on a fresh CI
  runner. Added `llvm-tools-preview` directly to `rust-toolchain.toml`, the actual effective
  source of truth.
- **Verified for real in this development environment**: the full instrument → train (5 committed
  ROMs) → optimized-rebuild pipeline produces a genuine, running `rustysnes` binary, and the
  determinism oracle (`cargo pgo optimize test`) passes cleanly under the PGO-merged profile. The
  local A/B speedup did not clear the `>3%` promotion bar on a short local training run (as
  documented honestly in `docs/performance.md` — this is an expected, not a failure, state; a
  short/narrow local run isn't representative of CI's real `3600`-frame training sweep).

### Fixed

- **A real bug in `pgo.yml`'s BOLT stage, found in PR review**: re-invoking the whole
  `scripts/pgo/run.sh` between `cargo pgo bolt build` and `cargo pgo bolt optimize` ran a
  separate, non-BOLT PGO cycle that never fed BOLT's profile data and could clobber the
  bolt-instrumented binary with an unrelated plain-PGO one. Fixed per `cargo-pgo`'s own
  documented BOLT+PGO combined workflow: `--with-pgo` on both `cargo pgo bolt build` and
  `cargo pgo bolt optimize`, with the erroneous `run.sh` re-invocation removed. Real BOLT profile
  gathering (running the actual GUI frontend binary against a workload) stays out of scope —
  this project's frontend has no headless CLI mode — so the fix deliberately uses `cargo-pgo`'s
  own documented profile-less BOLT fallback instead.

## [1.18.0] "Dormant" - 2026-07-14

Fourteenth release of the RustyNES-parity roadmap: Mobile Phase 5, monetization scaffolding.

### Added

- **`rustysnes-monetization`** (Mobile Phase 5): a new, standalone UniFFI crate providing a
  dormant entitlement/ad-pacing policy scaffold — `check_entitlement`, `default_ad_pacing_policy`,
  `should_show_ad`. **Never a dependency of the deterministic core** (no `rustysnes-core`/`-cpu`/
  `-ppu`/`-apu`/`-cart` dependency in either direction) and, unlike RustyNES's own already-shipped
  module, every concrete pricing/pacing number here is an explicit placeholder default, not a
  committed figure — the real store-launch decision stays with `docs/mobile-readiness.md`'s
  standing "Mobile Phase 6" gate. Pure functions only, host-injected `now_unix_secs` timestamps
  (matching `docs/adr/0004`'s determinism-discipline convention), 5 unit tests covering the
  ad-pacing session/interval/clock-rollback logic.
- **Wired into both mobile shells as an inert dependency**: compiled in, called once at startup,
  logged only, no real store SDK calls, no paywall/UI shown. **Verified for real on Android**:
  rebuilt via a real Gradle build (native `.so` cross-compiled for both ABIs via `cargo ndk`, a
  second, separate `uniffiBindgen`-style task generating this crate's own Kotlin bindings
  alongside `rustysnes-mobile`'s existing ones), installed on the real AVD, launched, and confirmed
  via `logcat`: `monetization scaffold (dormant): unlocked=true minIntervalSecs=300
  sessionsBeforeFirstAd=3`, with the app remaining alive with no crash afterward. **iOS**:
  `scripts/build-ios-xcframework.sh` gained a third crate to build/package, merged with
  `rustysnes-mobile` into one combined `RustysnesFFI.xcframework` (a real macOS CI run caught two
  separate per-crate xcframeworks colliding: `xcodebuild` copies every "library"+`-headers`
  xcframework's headers into one directory shared across the whole target, and two xcframeworks
  each contributing a same-named `module.modulemap` there is a genuine "Multiple commands
  produce" build failure — fixed by `libtool -static`-merging both crates' `.a`s and combining
  their modulemaps into one umbrella module); `ios/project.yml`'s dependency list updated to
  match. The Rust side's `staticlib`/`rlib` outputs cross-compile for real in this development
  environment (matching `rustysnes-ios`'s own precedent), but the `cdylib` output the
  bindgen step needs only links with a real Apple toolchain, so the full pipeline is
  compile-verified via `ios.yml`'s real macOS CI build only, matching this platform's standing
  "scaffolded-only" disposition since `v1.16.0`.

## [1.17.0] "Parity" - 2026-07-12

Thirteenth release of the RustyNES-parity roadmap: Mobile Phase 4, hardening.

### Added

- **Save State / Load State on both mobile shells** (Mobile Phase 4): a
  single save-state slot on Android (`android/.../MainActivity.kt`, persisted to app-private
  internal storage) and iOS (`ios/.../EmulatorViewModel.swift`, persisted to the app's Documents
  directory), both calling `MobileCore.saveState`/`loadState` — already covered by
  `rustysnes-mobile`'s own host-side round-trip/garbage-rejection unit tests since `v1.14.0`, not
  new Rust logic. Multi-slot UI is `v1.17.1+` polish, matching how the mobile track's own
  touch-UX/save-state UI were themselves deferred from `v1.15.0`.
  **Verified for real on Android**: rebuilt, reinstalled, and re-tested on the real AVD — saved
  mid-run, let the emulator advance further, tapped Load State, and confirmed via `adb run-as`
  that a real, correctly-sized (~497KB) save-state blob was written to disk and `loadState`
  returned with no exception logged (the visual test-ROM counter itself converges to a fixed
  "Success" state too quickly after the load to serve as an unambiguous rewind indicator with
  this specific ROM, so the file-existence + no-exception evidence is the actual verification
  signal here, on top of the already-tested Rust-level round-trip logic). **iOS**: written and
  compile-verified via `ios.yml`'s real macOS CI build; no on-device/simulator run, matching
  `v1.16.0`'s own standing disposition for this whole platform.
- Bumped the Android `versionName` (`android/app/build.gradle.kts`) to `1.17.0` — found to have
  been left at `1.15.0` through both the `v1.15.0` and `v1.16.0` releases; fixed alongside iOS's
  `project.yml` `MARKETING_VERSION`, which already got this treatment correctly in `v1.16.0`.

### Fixed

- **A real, pre-existing, already-shipped Android crash**: a native `SIGSEGV` inside
  `AudioTrack::write` → `AudioTrack::releaseBuffer` (null pointer dereference), reproducible
  on the real AVD by simply loading a ROM and letting it run for ~10+ seconds — present since
  `v1.15.0` and never caught before because prior verification passes never ran the app that
  long. Root cause: the frame loop's own audio path allocated a fresh `ShortArray` every ~16ms
  via `ShortArray(size) { audio[it] }` (converting `MobileCore.drainAudio()`'s boxed
  `List<Short>` every frame), enough sustained allocation/GC pressure at 60 FPS to disrupt the
  native `AudioTrack` buffer's timing and trigger the crash. Fixed by reusing a persistent,
  only-ever-grown `ShortArray` scratch buffer across frames instead. Re-verified on the real AVD:
  stable through 45+ seconds of continuous run plus a full save/load-state cycle, zero crashes.
  (A prior `v1.15.0` PR review actually flagged this exact allocation pattern as a hot-path perf
  nit and it was reasoned-rejected as "real but perf-only" — this rung found that disposition was
  wrong: it's a real correctness/stability bug, not just a perf nit.)
- `startFrameLoop` is now idempotent (a no-op if a loop is already active) rather than
  unconditionally cancelling and restarting — found while investigating the crash above:
  `attachSurface` calls it unconditionally whenever `surfaceCreated` fires with a ROM already
  loaded, and `Job.cancel()` is cooperative (the old coroutine keeps running until its next
  suspension point), so two coroutines could briefly write to the same `AudioTrack`
  concurrently. This alone did not reproduce the crash above in isolation, but it's a real
  latent hazard worth closing regardless.

### Honestly re-scoped (not silently dropped)

`v1.17.0 "Parity"` was originally planned to also include RetroAchievements wiring into
`rustysnes-mobile`, an `mlua` `send`-feature migration, and direct-IP/LAN netplay on both mobile
shells (see `to-dos/VERSION-PLAN.md`'s prior entry for this rung). All three were investigated and
found not to fit a discrete, honestly-verifiable change at this rung:

- **RetroAchievements**: `rustysnes-cheevos`'s `RaClient` API is callback-based (`begin_login_*`/
  `begin_load_game` take `on_done` closures for async HTTP completion), which doesn't map onto
  UniFFI's synchronous call model without real bridging design work, and wiring it in would also
  require cross-compiling `rcheevos`'s vendored C library for two additional native target sets
  (Android NDK ABIs it doesn't yet target, and iOS device/simulator triples) — real, substantial
  engineering, not a scoped addition. Deferred to a later mobile-track rung once that bridging
  design exists.
- **`mlua` `send`-feature migration**: the roadmap's own text gated this on "if Lua/TAS-on-mobile
  is greenlit" — no such greenlight has been given; neither mobile shell has any scripting
  surface to migrate for.
- **Direct-IP/LAN netplay on both shells**: a large, net-new UI surface (room/IP entry, in-game
  connection-state handling) on top of the background/foreground lifecycle work both shells
  already carry — attempting it now would mean writing more untested-in-this-sandbox Swift/Kotlin
  than this rung's own verification capacity could actually back up, especially on iOS where
  nothing has ever been run, only built.

## [1.16.0] "Beacon" - 2026-07-12

Twelfth release of the RustyNES-parity roadmap: Mobile Phase 3, the iOS alpha.

### Added

- **New crate `rustysnes-ios`** (Mobile Phase 3): a presentation-only `wgpu`-on-`CAMetalLayer`
  host with no emulation logic of its own — the same shape `rustysnes-android` (`v1.15.0`) already
  proved, just a plain C-ABI FFI surface (declared in `ios/RustySNES/Bridging-Header.h`) instead
  of JNI, since Swift's C interop needs no JNI-style boilerplate. Reuses
  `rustysnes-gfx-shaders::BLIT_WGSL` verbatim for the unfiltered blit pass.
  **Verified for real**: `cargo build --release --target aarch64-apple-ios` (and
  `aarch64-apple-ios-sim`) genuinely succeeds in this project's Linux development environment with
  no Xcode/macOS SDK installed — a `staticlib` only needs the downloaded `rust-std` component for
  the target, deferring the link against Apple's frameworks to Xcode's own final link step;
  confirmed via `file` that the produced `librustysnes_ios.a` contains a real
  `Mach-O 64-bit arm64 object`. `cargo clippy`/`cargo test` also pass cleanly against the plain
  host target (unlike `rustysnes-android`, this crate needs no CI workspace exclusion).
- **New `ios/` SwiftUI shell source**: mirrors `v1.15.0`'s Android Compose shell's exact MVP scope
  and architecture — a file-picker ROM load, on-screen touch d-pad/face buttons for the standard
  SNES gamepad (P1 only), `AVAudioEngine` playback of `rustysnes-mobile`'s `drainAudio`, and the
  same background/foreground pause-resume lifecycle handling `rustysnes-android`'s PR review found
  real bugs around (applied here from the start, not left to be rediscovered). Project structure
  is an `XcodeGen` YAML spec (`ios/project.yml`), not a hand-authored `.xcodeproj` — a plain-text
  spec can be written and reviewed correctly without Xcode, where a subtly-malformed binary
  project file would only reveal itself the first time someone opened it.
- **New `.github/workflows/ios.yml`**: builds the `.xcframework` artifacts
  (`scripts/build-ios-xcframework.sh`) and the generated UniFFI Swift bindings, then a real,
  unsigned `xcodebuild` simulator build on a `macos-latest` runner — this is the ONLY place in the
  project that exercises a real Xcode/Swift toolchain, since this development environment has
  none. A ~60-day refresh cron catches Xcode/Swift toolchain drift on GitHub's runner image even
  when nothing in `crates/rustysnes-ios`/`ios/` itself has changed. TestFlight upload is
  implemented as an explicit no-op, gated on distribution-signing secrets that don't exist yet
  (skip, not fail).
- **The `ios.yml` build genuinely passes** on a real `macos-latest` runner, after fixing four real
  bugs this Swift/Xcode code's first-ever real compiler pass and PR review actually found: a
  missing `x86_64-apple-ios` simulator slice (the xcframeworks only had `arm64`, but a
  `generic/platform=iOS Simulator` destination wants both, regardless of the runner's own CPU —
  fixed via a `lipo`-merged universal simulator library), an `AVAudioPlayerNode.scheduleBuffer`
  `async` overload missing its `await`, a real `DispatchQueue.main.async`-induced race between
  `surfaceCreated` and `surfaceDestroyed` (fixed by calling `surfaceCreated` synchronously), and a
  missing `AVAudioSession` category/activation (iOS produces no audible output without it, unlike
  Android/desktop). Also switched the audio buffer format from interleaved to non-interleaved
  Int16 to sidestep a real, plausible `int16ChannelData`-for-interleaved-formats correctness risk
  a reviewer flagged that this sandbox has no way to verify at runtime.

### Honestly unverified (unlike everything above, which is genuinely tested)

- **No on-device or simulator *run* has happened** — only a build. `ios.yml`'s `xcodebuild build`
  proves every `.swift` file compiles against a real Swift compiler and links against the real
  `.xcframework` artifacts, but no ROM has ever actually booted on this platform.
- No App Store §4.7 self-audit, no TestFlight upload, no real distribution signing.

## [1.15.0] "Sideload" - 2026-07-12

Eleventh release of the RustyNES-parity roadmap: Mobile Phase 2, a real Android alpha.

### Added

- **New crate `rustysnes-android`** (Mobile Phase 2, `v1.15.0 "Sideload"`): a presentation-only
  JNI/`wgpu`-on-`Surface` host with no emulation logic of its own — receives
  `(RGBA8 framebuffer bytes, width, height)` from the Kotlin shell once per frame and blits it via
  `rustysnes-gfx-shaders::BLIT_WGSL` (the same unfiltered shader `rustysnes-frontend::gfx` uses;
  the `Crt`/`Hqx`/`Xbrz` post-filters are a documented follow-up, not wired here yet). Explicit
  Vulkan-first/GLES-fallback backend selection, matching `rustysnes-frontend`'s own non-ambiguous
  wasm backend choice.
- **New `android/` Gradle project**: a minimal native Kotlin Compose shell — a Storage-Access-
  Framework ROM picker, on-screen touch d-pad/face buttons for the standard SNES gamepad (P1
  only), and `AudioTrack`-streamed audio playback of `rustysnes-mobile`'s `drainAudio`. Wired via
  custom Gradle tasks (`cargoNdkBuild`, per-ABI `copyCargoLibs*`, `uniffiBindgen`) that build both
  native crates and regenerate the UniFFI Kotlin bindings on every build, so they can never drift
  from the Rust source they're generated from.
- **Verified for real, not just claimed**: built, installed, and launched on a real Android
  emulator (API 34, x86_64) — the app displays with no crash, and loading a real test ROM through
  the SAF picker shows the emulator actually running (live, advancing framebuffer output, not a
  static frame) with zero errors in `logcat`.

### Fixed

- **A real wgpu-on-Android-Surface initialization bug**, found only by actually running the app
  on-device (not caught by `cargo ndk check`/`clippy`, which don't exercise runtime surface
  creation): `SurfaceTargetUnsafe::from_window()` in wgpu 29 unconditionally sets
  `raw_display_handle: None` (it only requires `HasWindowHandle`), and `wgpu-core`'s
  `create_surface` hard-errors whenever both the per-surface and the `InstanceDescriptor::display`
  handles are `None`. Switched to `SurfaceTargetUnsafe::from_display_and_window`, which forwards
  the marker-only `RawDisplayHandle::Android` value Android's `HasDisplayHandle` impl already
  supplies.
- **A real emulator-only crash**: `InstanceFlags::default()`'s debug-build `DEBUG`+`VALIDATION`
  flags crashed the AVD's SwiftShader software Vulkan renderer outright (a SPIR-V debug-info
  emission the software rasterizer can't handle, taking the whole emulator process down). Real
  hardware Vulkan drivers don't hit this path; disabled both flags explicitly since real devices
  ship a hardware driver, not a software one.
- **A premature `ANativeWindow` release**: `Renderer` now keeps a cloned `NativeWindow` handle
  alive for its own lifetime, fixing a real use-after-free-adjacent bug where the window's
  refcount could be released while the `wgpu::Surface` built from it was still in use (found in
  PR review).
- **A per-frame allocation on the render hot path**: `nativePresentFrame` now copies into a
  reused scratch buffer via `get_byte_array_region` instead of `convert_byte_array`, which
  always allocated a fresh `Vec` every frame.
- **A `u32` overflow ordering bug** in `Renderer::present`'s bounds check, and a genuine
  `AudioTrack` cross-thread visibility bug (`@Volatile` was missing), both found in PR review.
- **ROM loading off the main thread**: `MainActivity.loadRom` now runs on
  `lifecycleScope.launch(Dispatchers.IO)` — previously ran synchronously on the UI thread, a
  real ANR risk for larger ROMs.
- **The frame loop and audio no longer keep running while backgrounded**: both now pause on
  `onPause`/surface-destroyed and resume on `onResume`/surface-reattach if a ROM is loaded —
  previously kept spinning (and could keep playing audio) after the surface became invalid.

### Deferred (honestly scoped, matching the "Minimal real MVP now" decision for this rung)

- Mouse-mode trackpad, Super Scope drag-reticle, and Multitap pass-and-play seat switcher (net-new
  SNES-specific touch UX with no RustyNES precedent) — `v1.15.1+`.
- Save-state UI, settings screen, `Crt`/`Hqx`/`Xbrz` post-filter wiring, frame-pacing/vsync-synced
  render loop (currently a fixed ~60 Hz sleep-paced coroutine) — `v1.15.1+`.
- `.github/workflows/android.yml` (NDK cross-build CI, UniFFI Kotlin smoke test, 16KB ELF
  page-alignment check, dormant Play-flavor Gradle split) — `v1.15.1+`.
- A checked-in `./gradlew` wrapper (this environment used the locally cached Gradle 8.11
  distribution directly) — `v1.15.1+`.

## [1.14.0] "Foundry" - 2026-07-12

Tenth release of the RustyNES-parity roadmap: Mobile Phase 1, the UniFFI bridge foundations.

### Added

- **New crate `rustysnes-mobile`** (Mobile Phase 1): a `UniFFI` bridge
  generating Kotlin (Android) and Swift (iOS) bindings over `rustysnes_core::facade::EmuCore` —
  the same facade the desktop frontend and `rustysnes-libretro` already drive the emulator
  through. MVP surface: ROM load/close, `run_frame`, the peripheral setters (Gamepad/Mouse/Super
  Scope/Multitap), framebuffer + per-frame audio access, save/load state, reset/power-cycle.
  Verified for real: a genuine `cargo ndk` cross-compile to `arm64-v8a` produced an actual ARM64
  `.so` (confirmed via `file`), and `uniffi-bindgen` generated real, correctly-shaped Kotlin and
  Swift bindings from the compiled library.
- **`no_std` CI gate expanded to a per-crate matrix**: `rustysnes-{cpu,ppu,apu,cart,core}` each
  now build standalone against `thumbv7em-none-eabihf --no-default-features`, replacing the prior
  single aggregate-only `rustysnes-core` build.
- **The mobile/Android+iOS "no appetite" default from `v1.0.0` is formally reversed** — new
  `docs/adr/0012-mobile-platform-target.md` records the decision, new `docs/mobile-readiness.md`
  is the living status page.

### Deferred (honestly scoped, not silently dropped)

- HD-pack consumption, cheats, rewind/run-ahead, netplay, `RetroAchievements`, and Lua/TAS
  scripting are all out of `rustysnes-mobile`'s MVP surface — real, separate frontend concerns
  layered on top of `EmuCore` in the desktop build too, not re-invented here.
- No real Android app, emulator run, or touch UX yet — `v1.15.0 "Sideload"`'s scope.
- No iOS build/link/run at all — this development environment has no macOS/Xcode toolchain.
  `v1.16.0 "Beacon"`'s `rustysnes-ios` crate and SwiftUI shell will be written and Rust-side
  compile-checked, but the real Xcode verification needs the project owner's own Mac.

## [1.13.0] "Vantage" - 2026-07-12

Ninth release of the RustyNES-parity roadmap: two accessibility theme variants, plus an honest
re-scoping of the other two originally-planned items.

### Added

- **Two accessibility theme variants**: `AppTheme::HighContrast` (a
  near-black/near-white theme pushing every foreground/background pair past WCAG 2.1 AA, most
  past AAA) and `AppTheme::Colorblind` (interactive accents drawn from the Okabe-Ito palette,
  mutually distinguishable under the most common red-green color-vision deficiencies), both
  additive after the original `Light`/`Dark`/`System` trio and both regression-tested against the
  stock dark theme so a builder that forgot to override a `Visuals` field can't silently ship an
  indistinguishable theme.

### Deferred (honestly scoped, not silently dropped)

- A keyboard-only-navigation audit across every UI surface added since `v1.7.0` was investigated
  and found to be a manual-walkthrough task, not a discrete code fix (egui's own default Tab
  order is used everywhere; nothing here is broken, but nothing has been walked and confirmed
  either) — tracked as an open item in `docs/frontend.md`'s Theme section rather than converted
  into a hollow "audit passed" claim.

### Corrected (a stale plan premise, not new work)

- `to-dos/VERSION-PLAN.md`'s `v1.13.0` entry described "a save-state versioned-migration
  regression fixture" as "the one real save-state gap found." Investigating it found the premise
  was stale: `System::load_state` was always designed to fail loudly on an older-format blob, by
  deliberate choice recorded since the `FORMAT_VERSION` `2`/`3` bumps — never to gracefully
  migrate one — and a regression fixture proving exactly that behavior
  (`save_state_backward_compat.rs`'s `old_format_version_blob_fails_loudly_not_silently`) has
  existed since `v0.7.0`. No code changed here; this closes the item as verified-non-issue. See
  `docs/frontend.md`'s "Save-states, rewind, run-ahead" section for the full explanation.

## [1.12.0] "Refraction" - 2026-07-12

Eighth release of the RustyNES-parity roadmap: a third post-filter, and a shader-source crate
extraction that sets up the mobile track.

### Added

- **A third presentation post-filter, `PostFilter::Xbrz`**: a single-pass, context-aware
  corner-rounding blend — an xBRZ-style *approximation* of the algorithm's corner rule (not a
  literal multi-pass xBRZ port). It blends the same 2x2 corner `PostFilter::Hqx` does, but reads
  a wider 4x4 neighborhood and only commits to the full diagonal pull when the outward context
  agrees the edge is a genuine corner, not isolated-pixel noise. One strength slider
  (`config.video.xbrz_strength`, default `0.6`), selectable from Settings → Video and the View →
  Post-filter submenu, same as `Crt`/`Hqx`.
- **New `rustysnes-gfx-shaders` crate**: the `BLIT_WGSL`/`CRT_WGSL`/`HQX_WGSL` shader sources
  moved out of `rustysnes-frontend::gfx`, byte-identical, alongside the new `XBRZ_WGSL` — so the
  planned `rustysnes-mobile` bridge (`v1.14.0 "Foundry"`) can reuse the exact shader strings
  without depending on this crate's winit/egui/cpal shell. `#![no_std]`, verified against the
  existing `thumbv7em-none-eabihf` no_std CI gate.

### Deferred (honestly scoped, not silently dropped)

- RetroArch `.slangp`/`.cgp` shader-preset import and a composite/RF post-pass approximating
  SNES analog-out characteristics remain out of scope, unrevisited from `v1.2.0`'s original call
  (not a new finding this release). See `to-dos/VERSION-PLAN.md`'s `v1.12.0` section.

## [1.11.0] "Podium" - 2026-07-12

Seventh release of the RustyNES-parity roadmap: RetroAchievements never loaded a game.

### Fixed

- **RetroAchievements never actually loaded a game.** No code path ever called
  `RaClient::begin_load_game` — login worked, `CheevosState::do_frame` ran every
  emulated frame, and `AchievementTriggered` events were wired all the way to
  status-bar toasts, but with no game ever identified/loaded into `rc_client`,
  there was no achievement set to evaluate memory against, so achievements
  could never actually trigger. `CheevosState::load_game`/`unload_game` now
  wrap the missing calls, invoked from `app.rs`'s `MenuAction::OpenRom`/
  `CloseRom` handlers (a no-op unless a user is logged in); a `poll()`-drained
  toast surfaces success/failure so the fix is observably verifiable, not just
  type-checked. This is the actual prerequisite bug blocking hardcore mode,
  leaderboards, and rich presence from meaning anything — found while scoping
  those features for this release.

### Deferred (honestly scoped, not silently dropped)

- Splitting a new `rustysnes-ra` session/UI crate out of `frontend/src/cheevos.rs`'s
  informal state, hardcore mode gating rewind/save-load/cheats/TAS, and
  leaderboard/rich-presence UI are all real, substantial features that were
  meaningless without the game-load fix above landing first. Pushed to a
  later, explicitly-scoped release. See `to-dos/VERSION-PLAN.md`'s `v1.11.0`
  section.
- A ROM loaded via the CLI at startup, followed by a *later* login through the
  Tools window, is not retroactively announced to `rc_client` — the common
  path (launch, log in, then open a ROM via the File menu) is unaffected. See
  `cheevos.rs`'s module doc.

## [1.10.0] "Atelier" - 2026-07-12

Sixth release of the RustyNES-parity roadmap: HD-pack `emu-thread` wiring.

### Fixed

- **HD texture packs (`v1.3.0`) were never wired into the `emu-thread` build** —
  `app.rs`'s synchronous render path composited an active pack via
  `hd_compositor::composite` before its own `drop(emu)`, but the threaded build's
  `emu_thread::drive_one` had no equivalent step, so a threaded build with a pack
  selected silently rendered the native (uncomposited) framebuffer
  (`docs/frontend.md`'s documented scope cut, closed here). `drive_one`'s
  plain-frame (run-ahead-disabled) branch now composites before publishing to
  `PresentBuffer`; the common no-pack-active case stays exactly as fast as before
  (a cheap `hd_pack_name()` `&self` pre-check, no extra allocation) since the real
  compositing cost only applies once a pack is actually selected. Found in review
  (#90): the lock is now released before the `PresentBuffer` copy in this branch
  too, matching the run-ahead branch's existing `drop(emu)`-before-publish pattern.

### Deferred (honestly scoped, not silently dropped)

- The in-app HD-pack **Builder GUI** (browsing the live `TileTag` stream,
  assigning replacement PNGs, writing `pack.toml` + assets) needs a new
  core-side "reconstruct RGBA pixels for a given tile hash" API that doesn't
  exist yet — the tile-identity hash doesn't reverse to a VRAM location, so
  authoring support is a genuinely separate, substantial piece of work from the
  `emu-thread` wiring fix above. Pushed to a later, explicitly-scoped release.
  See `to-dos/VERSION-PLAN.md`'s `v1.10.0` section.
- **HD-pack compositing is deliberately NOT applied to `emu-thread`'s run-ahead
  branch.** Found in review (#90): `step_with_run_ahead`'s returned frame is a
  PEEKED frame (captured, then rolled back so `emu`'s persisted state only
  advances by one real frame), but `EmuCore::hd_pack_composite_inputs` reads
  `Ppu::tile_tags()` from `emu`'s CURRENT (post-rollback) state — a different
  frame than the peeked bytes. Compositing with a mismatched `(fb, tags)` pair
  would silently apply replacement tiles keyed to the wrong frame, corrupting
  the picture rather than just showing native art, so this rung skips
  compositing there instead. **The same desync already exists, unfixed, in
  `app.rs`'s synchronous render path** (pre-existing since run-ahead and
  HD-pack were first combined; not introduced by this release, not touched by
  it either) — both are tracked together as a `v1.10.x`/later follow-up in
  `to-dos/VERSION-PLAN.md`. Run-ahead and HD-pack are each off by default, so
  this only affects the narrow case where a user has both features enabled
  simultaneously.

## [1.9.0] "Marionette" - 2026-07-12

Fifth release of the RustyNES-parity roadmap: Lua scripting bus-widening.

### Added

- **Lua scripting: full-bus reads** — `rustysnes-script`'s `emu.read(addr)` now reaches
  [`Bus::peek`] (the full 24-bit bus: WRAM, cart ROM/SRAM; I/O register space still reads back as
  `0`, `Bus::peek`'s own documented behavior, matching the debugger's Memory panel), widened from
  [`Bus::peek_wram`] (WRAM-only). `emu.write(addr, val)` stays scoped to
  [`Bus::poke_wram`] (WRAM only) — a side-effect-free "poke" has no clean semantic for register
  space (a real PPU/APU/DMA register write has hardware side effects a silent poke can't model
  without either faking them or breaking the determinism contract), so widening reads while
  keeping writes WRAM-scoped is a deliberate, asymmetric choice, not an oversight.

### Deferred (honestly scoped, not silently dropped)

- A wasm `piccolo` Lua backend (scripting is currently native-only, `mlua`) and TAStudio-style
  piano-roll movie editing are both substantial standalone efforts, comparable in size to a full
  release rung on their own — pushed to a later, explicitly-scoped release rather than folded into
  this one. See `to-dos/VERSION-PLAN.md`'s `v1.9.0` section.

## [1.8.0] "Tracepoint" - 2026-07-12

Fourth release of the RustyNES-parity roadmap: debugger depth II.

### Added

- **Memory Compare panel** — captures a baseline snapshot of the Memory panel's current window
  and diffs it against the live window on every frame, showing only the rows that changed
  (`before -> after` hex). Flags a mismatch instead of a misleading diff if the window has
  scrolled since the baseline was captured (no scroll control exists yet — same gap the Memory
  panel itself carries).
- **Docs panel** — an in-app SNES-terminology glossary (`docs/glossary.md`, embedded via
  `include_str!`, ~3KB) for quick lookup mid-session, plus a link to the full `MkDocs` handbook
  (`v1.6.0 "Lighthouse"`). Deliberately scoped to the glossary alone, not the full 10-50KB
  subsystem-spec docs, to keep wasm size impact negligible (verified: +2KB gzip, still ~2.05 MiB
  under the 5 MiB budget).

### Deferred (honestly scoped, not silently dropped)

- A call-stack view, an instruction/event trace buffer, and an inline 65816 assembler all need
  new core-side instrumentation (tracking call/return events or recording a trace log as they
  happen — not inferable from a point-in-time memory snapshot the way this rung's two panels are).
  A larger cross-crate change than this rung's frontend-only scope; tracked as follow-up work.
- A dedicated per-coprocessor-type register panel (DSP-2/4, S-DD1, CX4, OBC1, ST018, S-RTC beyond
  the SA-1/GSU state the existing Cart panel already shows) needs new `Board`-trait debug-state
  accessors — also deferred.

## [1.7.1] - 2026-07-12

Patch release: a single user-reported bugfix, no new scope.

### Fixed

- **The wasm demo's canvas rendered at a smaller, fixed 2x scale instead of the 3x
  `INITIAL_SCALE` native launches at.** `App::create_window` special-cased `wasm32` to a hardcoded
  `(512.0, 448.0)` `LogicalSize`, deferring to `web/index.html`'s own CSS (`512x448`) — but
  winit's web backend actually resizes the attached `<canvas>` to match the requested inner size,
  overriding that CSS regardless. RustyNES's own `create_window` requests `NES_W * INITIAL_SCALE`
  unconditionally (native and wasm alike), which is why its own demo already rendered at 3x — a
  user comparing the two demos side by side noticed the size difference. `web/index.html`'s CSS
  updated to `896x728` (the new pre-JS fallback appearance, matching the actual chrome-padded 3x
  size) so there's no flash of the old size before winit applies the real one. Found in review
  (#82): the fallback CSS's fixed height paired with `max-width: 96vw` would have distorted the
  canvas on narrow viewports — switched to `aspect-ratio` + `height: auto` instead.

## [1.7.0] "Telemetry" - 2026-07-12

Third release of the RustyNES-parity roadmap.

### Added

- **Debugger foundation** — the 4-panel debugger overlay (previously inline in `ui_shell.rs`,
  ~600 lines) moved into a dedicated `debugger/` module (`mod.rs` + `cpu_panel.rs`/`ppu_panel.rs`/
  `apu_panel.rs`/`cart_panel.rs`/`watch_panel.rs`) — a pure structural extraction, zero behavior
  change, that later debugger-depth rungs (`v1.8.0` onward) plug new panels into. `lib.rs`'s
  stale "the deep debugger panels are still TODO stubs" doc comment corrected (the panels have
  existed since `v0.8.0`; this rung gives them a real module).
- **Memory panel** — the Watch panel (renamed "Memory/Watch" in the panel selector) gained a
  read-only hex dump of a 512-byte window of WRAM/cart space (`DebugSnapshot::memory_window`,
  read via the same non-intrusive `Bus::peek` the disassembler already uses; I/O register space
  reads back as `00` rather than a live register value — `Bus::peek` intentionally doesn't model
  registers, so this is a memory dump, not a register viewer). Fixed at `$7E0000` (WRAM bank 0)
  by default — no UI scroll control yet (`EmuCore::set_debug_memory_scroll` exists for a future
  one to call), the same honestly-tracked gap the existing VRAM viewer already carries. Write
  support and a RAM-search tool are explicitly **not** included in this rung — deferred, not
  overclaimed.

### Fixed

- **The workspace version was stuck at `1.4.0`** since `v1.5.0` — `env!("CARGO_PKG_VERSION")`
  feeds the egui Help window's version label and the CLI's `--version` output (including on the
  deployed GitHub Pages wasm demo), so both silently under-reported the running version through
  the `v1.5.0`/`v1.6.0` releases. Every prior `chore(release)` commit back to `v0.7.0` bumped
  `[workspace.package] version` (and each crate's own pinned `version` field) as part of the
  closeout — a step that isn't spelled out in `docs/adr/0007`'s decision list and got missed
  starting at `v1.5.0`. Reported by a user testing the live demo; bumped to `1.7.0` across the
  workspace and all 11 non-workspace-inherited crates, and this ceremony gap is now called out
  explicitly in `to-dos/VERSION-PLAN.md`'s standing release checklist.
- **Findings from review (#80)**: the memory panel's doc comment, panel label, and CHANGELOG
  entry all overclaimed "full 24-bit CPU bus" without noting that `Bus::peek` returns `0` for
  I/O register space — corrected in all three places.

## [1.6.0] "Lighthouse" - 2026-07-11

Second release of the RustyNES-parity roadmap.

### Added

- **Documentation site + PWA + accuracy ledger** — a Material for MkDocs handbook (`mkdocs.yml`)
  is now published at `https://doublegate.github.io/RustySNES/docs/`, alongside the wasm demo
  (`/`) and rustdoc (`/api/`), replacing `pages.yml` with a combined `web.yml` that also enforces
  the existing `<5MiB` gzip wasm size budget (`scripts/wasm_size_budget.sh`) on every PR. The wasm
  demo gained PWA/offline support (`manifest.webmanifest`, a stale-while-revalidate `sw.js`
  service worker, a real `icon.svg`). New `docs/accuracy-ledger.md` maps every known
  approximation/divergence to an explicit disposition (Remediated / No-stricter-oracle-available /
  Deferred / Out-of-scope), the "why" companion to `docs/STATUS.md`'s pass-count dashboard.
  `docs/DOCUMENTATION_INDEX.md` refreshed (was still stamped `v0.4.0`, referenced a nonexistent
  `SALVAGE_MANIFEST.md`).

### Fixed (caught in PR review, #78)

- `web.yml`'s `cancel-in-progress` was inverted relative to its own comment — PR size-budget
  checks could be cancelled mid-flight while stale `main` pushes weren't; corrected to cancel on
  `push` only.
- `sw.js`'s fetch handler could resolve `respondWith()` to `undefined` on a truly offline first
  visit (network fails and nothing is cached yet); now falls back to a synthetic `503` response.

## [1.5.0] "Bedrock" - 2026-07-11

First release of the RustyNES-parity roadmap: closes the gap between this project's own
feature/UX/accuracy maturity and its sibling NES emulator RustyNES, tracked in lockstep rather
than a frozen snapshot. This rung is CI safety net only — see `to-dos/VERSION-PLAN.md`'s
"RustyNES-parity ladder" section for the full `v1.5.0`-`v1.19.0` plan.

### Added

- **CI safety net** — `cargo test --workspace` now runs on every PR/push to `main` (new
  `test-light` job), not only on a tagged release. A new `changes`/`setup` job pair computes a
  light-vs-full run mode per push (mirroring RustyNES's own pattern), and `full-test`/`no_std`/
  `bench` now also run on every push to `main` (previously tag-only), plus a weekly drift-net cron
  and manual dispatch. A new `ci-success` job is the one stable required-check name; branch
  protection on `main` now requires it. See `docs/adr/0011`.
- A shared `.github/actions/rust-setup` composite action factors the pinned toolchain version and
  cache-key convention out of `ci.yml`/`pages.yml` into one place.
- `to-dos/LOCKSTEP-CHECKLIST.md` — the process for re-checking RustyNES's own continuing
  development before scoping each subsequent rung in the parity ladder.

### Fixed (caught in PR review, #76)

- The `rust-setup` composite action pinned `dtolnay/rust-toolchain@master` (a floating ref);
  changed to `@1.96`, matching what `ci.yml`'s jobs already used before this release.
- The composite action's Linux frontend dependency list was missing `libxkbcommon-x11-dev`
  (present in `CONTRIBUTING.md`'s documented list but never actually installed by the old inline
  per-job steps this action replaces) — added, and `CONTRIBUTING.md` reconciled to match exactly.

## [1.4.0] "Convergence" - 2026-07-11

### Added

- **Window Size presets** (native only) — View → Window Size offers 1x/2x/3x/4x (100%-400%) of
  the SNES native resolution, matching RustyNES; the app now launches at 3x by default instead of
  a fixed 512x448 window.
- **Libretro peripheral negotiation** — `rustysnes-libretro` now offers Mouse (both ports) and
  Super Multitap / Super Scope (port 2) via `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO`, mirroring
  bsnes's own libretro core's per-port device menu.
- **`emu-thread` mechanical re-sync** — cheats, watchpoints, breakpoints, port2-peripheral
  selection, and per-voice audio mutes now apply in the threaded build too (previously only the
  synchronous drive path saw these changes).
- **`emu-thread` run-ahead + netplay-aware pause** — run-ahead now runs on the emu thread via
  `crate::rewind::step_with_run_ahead`, only when actually configured (matching the synchronous
  path's own `run_ahead > 0` branch, avoiding an avoidable per-frame allocation in the common
  disabled case — caught in PR review); netplay now actually functions under `emu-thread` (its
  `NetplayState::drive` call was previously dead code there, so netplay was silently
  non-functional in threaded builds), pausing the emu thread TOCTOU-safely via a new
  `EmuControl::netplay_paused` flag re-checked under the shared `EmuCore` lock. `PresentBuffer`
  now carries the framebuffer's `(width, height)` alongside its bytes, and the present path tracks
  the dims that actually match its staging buffer (`Active::present_dims`) rather than the emu's
  live (possibly-moved-on) resolution, so a run-ahead-peeked frame can never publish bytes for one
  resolution against dims from another (also caught in review). `emu-thread` is now clippy- and
  test-gated in CI for the first time (previously referenced only in a comment). Movies, Lua
  scripting, RetroAchievements, and rewind-recording remain intentionally unported to `emu-thread`
  — confirmed via RustyNES's own reference implementation, which doesn't port these to its thread
  either.

### Fixed

- **Fullscreen crash on monitors wider/taller than 2048px** — `Gfx` requested
  `wgpu::Limits::downlevel_webgl2_defaults()` unconditionally on every target, capping
  `max_texture_dimension_2d` at 2048 even on native GPUs that support far more. Fullscreening on
  e.g. a 3440x1368 ultrawide made `Surface::configure` receive an out-of-range request and
  panic/abort (`wgpu::Surface::configure` has no recoverable error path here). Native now requests
  `downlevel_defaults()` and both targets call `.using_resolution(adapter.limits())`, raising the
  floor preset to match the real adapter; the granted limit is tracked at runtime and enforced
  everywhere the old hardcoded 2048 constant was.
- **Open bus during DMA/HDMA transfers** (the "Speedy Gonzales stage 6-1" mechanism) — DMA/HDMA
  reads now update the open-bus latch, matching real hardware; writes deliberately do not, per a
  direct cross-check against ares' and bsnes' own `CPU::Channel` DMA implementation.
  `superfx_boots_live_and_deterministic`'s 24 golden hashes were re-blessed with that citation
  trail as justification — see `docs/scheduler.md` §Open bus via DMA/HDMA.

## [1.3.0] "Palimpsest" - 2026-07-11

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (455 tests,
44 suites) plus the full `--features test-roms` ROM-oracle battery (28 tests, 17 suites via
`-p rustysnes-test-harness --release`) are green; no golden hash changed. fmt/clippy clean
across every feature combination this feature touches (default, `hd-pack`, `full`,
`emu-thread,hd-pack`, plus the pre-existing `debug-hooks`/`scripting`/`cheats`/`netplay`/
`retroachievements` lanes); `no_std` build clean; both wasm32 frontends (`wasm-winit` default,
`wasm-canvas`) build clean via real `trunk build --release` runs; `rustysnes-libretro` builds
clean.

### Added

- **HD texture packs** (`hd-pack` feature, off by default) — replace individual 8×8 tiles with
  higher-resolution PNG art while the accuracy-critical core stays completely pack-agnostic
  (`docs/adr/0010`). `rustysnes-ppu` computes a palette-inclusive XXH3-64 tile-identity hash
  (`hdtag::hash_tile`, hashed into a fixed stack buffer — no heap allocation on the rendering hot
  path) and records it per composited pixel into a write-only `Ppu::tile_tags()` side-buffer,
  populated only when `Ppu::set_hd_pack_tagging(true)` is on; leaving it at its default `false`
  is proven byte-identical to every prior release
  (`hd_pack_tagging_toggle_does_not_alter_framebuffer_output`), and the whole mechanism compiles
  out entirely — not just runtime-disabled — when the `hd-pack` feature is off. The frontend owns
  everything pack-specific: a versioned `pack.toml` manifest + PNG loader (`crate::hd_pack`,
  pure-Rust `png` decode, path-traversal-safe, duplicate-hash-rejecting), a pure CPU compositor
  (`crate::hd_compositor::composite`, fully unit-testable without a GPU adapter), a Settings →
  Video pack selector (dynamic `ComboBox`, populated per-ROM via the same SHA-256 identity
  save-states already use), and `config.video.hd_pack_name` persistence with automatic
  re-selection on ROM load. The compositor is wired into the live wgpu present path: `Gfx`'s
  previously fixed `MAX_W × MAX_H` streaming texture now grows on demand
  (`Gfx::ensure_texture_capacity`, capped at this device's actual downlevel-WebGL2
  `max_texture_dimension_2d`) to fit the composited output at a fixed 2× upscale, with the
  no-pack-active path staying pixel-identical to before (the texture never grows past its
  original allocation unless a pack is actually active). Not yet built, honestly tracked:
  a user-configurable upscale factor (fixed at 2× for now) and `emu-thread`-build compositing
  (that build's framebuffer arrives via a lock-free handoff with no equivalent `TileTag` channel
  yet). See `docs/ppu.md` §HD texture pack `TileTag` recording hook, `docs/frontend.md` §HD
  texture packs, and `docs/adr/0010`.

**Process note:** all three feature PRs (#66, #67, #68) went through the full branch → CI →
automated bot review → fix → reply → resolve → green → squash-merge ceremony. Real findings
addressed along the way: a heap allocation on the PPU rendering hot path, a path-traversal
vulnerability in the pack loader, a memory-pre-allocation DoS vector sized off an untrusted PNG
header, an integer-overflow risk in the compositor's coordinate math, a stale-tile-tags bug when
tagging was toggled off mid-session, an active-pack-cleared-on-a-failed-ROM-load bug, a
redundant per-frame ROM re-hash, and a texture-capacity cap that was enforced by the caller but
not the function itself. Two Gemini suggestions were investigated and found to not actually
compile as proposed (a borrow-checker conflict in the Settings pack-selector closure) — verified
by trying them, not just trusting the diff, and documented with the reasoning inline.

**Files changed:** 15 files across 3 PRs — `crates/rustysnes-ppu/src/hdtag.rs` (new),
`crates/rustysnes-ppu/src/{lib,render}.rs`, `crates/rustysnes-frontend/src/hd_pack.rs` (new),
`crates/rustysnes-frontend/src/hd_compositor.rs` (new), `crates/rustysnes-frontend/src/{emu,
gfx,app,config,ui_shell,cli,save_states}.rs`, `crates/rustysnes-core/Cargo.toml` (new `hd-pack`
feature propagation), `docs/{ppu,frontend}.md`, `docs/adr/0010-hd-texture-pack-system.md` (new).

**Testing evidence:** `cargo test --workspace` (455 tests, 44 suites), `cargo test -p
rustysnes-test-harness --features test-roms --release` (28 tests, 17 suites, zero regressions),
`cargo clippy --workspace --all-targets -- -D warnings` across every feature combination this
work touches, `cargo fmt --all --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
--no-deps` (both default and `--features hd-pack`), the `no_std` gate, two real `trunk build
--release` runs (`wasm-winit` default + `wasm-canvas`), and `cargo build -p rustysnes-libretro`.
Manual verification: real headless (`xvfb-run`) launches of the native binary against a staged
ROM — with no pack configured (unaffected path), with a real generated pack at the default 2×
scale, and with the scale temporarily forced to 3× specifically to exercise the texture-growth
path — all ran clean with no panics or wgpu validation errors.

## [1.2.0] "Phosphor" - 2026-07-11

### Changed

- **Relocated the `EmuCore` embedding facade from `rustysnes-frontend` into `rustysnes-core`**
  (a new `facade` module, `std`-only) — a libretro core or any other headless embedder can now
  depend on `rustysnes-core` alone instead of the winit/wgpu/cpal/egui-heavy frontend crate.
  `rustysnes-frontend::emu::EmuCore` is now a thin wrapper adding only the debugger-only fields
  (breakpoints, single-step, VRAM viewer scroll) on top of the relocated facade. Zero behavior
  change: every pure-facade method is a one-line delegation, verified by the unchanged frontend
  test suite, the full ROM-oracle battery, and the `no_std` CI job (the acid test that the new
  `#[cfg(feature = "std")]` gate actually removes the facade from the `thumbv7em` build). Also
  fixes a determinism-seed-discarding bug found in review: `load_rom`/`power_cycle`/`close_rom`
  rebuilt `System::new(0)` on every call, silently ignoring the caller's seed. See
  `docs/architecture.md` §3/§6 and `docs/frontend.md`.

### Added

- **`rustysnes-libretro`: a libretro core.** A thin C-ABI wrapper over
  `rustysnes_core::facade::EmuCore`, loadable by RetroArch or any other libretro-compatible
  frontend — region-aware NTSC/PAL geometry+timing, the S-DSP's real 32 kHz output rate,
  coprocessor firmware auto-resolution from the frontend's system directory, Game Genie/Pro
  Action Replay cheat support, and raw WRAM/VRAM/SRAM memory-map pointers for RetroArch's own
  SRAM autosave and RetroAchievements/cheat tooling. Peripheral negotiation (Mouse/Super
  Scope/Multitap via `RETRO_DEVICE_SUBCLASS`) is a documented follow-up, not yet wired. New
  additive `Bus::wram`/`wram_mut`, `Ppu::vram`/`vram_mut`, `Cart::sram_mut` accessors support it.
  See `docs/libretro.md`.
- **CRT/HQx presentation post-filters** (Settings → Video / View → Post-filter). `PostFilter::Crt`
  adds scanlines + an RGB aperture-grille mask (each with its own strength slider); `PostFilter::Hqx`
  adds a single-pass, edge-directed diagonal blend (an HQ2x-style approximation, not a literal
  lookup-table port) that softens staircase edges on flat-color pixel art. `PostFilter::None`
  (default) is the pre-existing direct blit, kept byte-for-byte unchanged — `Gfx::present`'s `None`
  arm calls the same unmodified `Gfx::blit` rather than a re-derived equivalent. Verified via
  `naga` WGSL-validity tests for both new shaders plus a real headless `xvfb-run` launch of all
  three filter states against a live wgpu adapter (zero errors, no panics). See
  `docs/frontend.md` §Presentation post-filters.

## [1.1.0] "Latchkey" - 2026-07-11

### Fixed

- **`SuperFxBoard::map`'s Game-Pak-RAM-ownership open-bus gap** — a CPU/DMA read of Game Pak RAM
  while the GSU owned the RAM bus always returned a hardcoded `0`, bypassing `Cart::read24`'s
  generic open-bus fallback (the same mechanism the SPC7110 investigation added) entirely, since
  `map()` classified this case as `Sram` rather than `Open`. Now correctly threads the real
  last-driven bus byte through. Verified independently: zero regressions across the full
  `--features test-roms` battery (all 27 suites) with this fix alone. Writes are unaffected
  (`Cart::write24` never consults `map()`). See `docs/scheduler.md` §Open bus via DMA/HDMA.

### Added

- **`emu-thread` (opt-in feature): real audio output + a proper pause/ROM-loaded/speed
  lifecycle.** The dedicated emulation thread now has its own `AudioProducer` (pushed once per
  produced frame, closing the "silent thread" gap) and an `EmuControl` lifecycle block (a
  thread-owned `Pacer` that tracks live speed-preset changes, plus a pause/ROM-loaded idle gate)
  instead of an independent, uncontrollable pacing loop — and a lock-free `PresentBuffer`
  triple-buffer handoff so the present path never blocks on the emu mutex for the framebuffer
  copy. Native builds now also carry an `EventLoopProxy<AppEvent>` (previously wasm32-only) so the
  thread can ping the winit loop (`AppEvent::EmuFrame`) after every produced frame. Still not full
  parity with the synchronous drive: cheats/watchpoints/breakpoints/port2-peripheral/voice-mutes
  sync, run-ahead, rewind recording, TAS movies, Lua scripting, netplay-aware pause, and
  RetroAchievements are not yet ported into the thread's loop — each needs a new
  shared-mutable-state design rather than a mechanical port, and stays a documented follow-up
  (`crates/rustysnes-frontend/src/emu_thread.rs`'s own module doc has the exact list). Verified
  via the unit suite plus a real headless `xvfb-run` launch against a staged commercial ROM (no
  panics over several seconds of runtime).

### Investigated (research, no code landed)

- **Open-bus-via-DMA-latch** (the "Speedy Gonzales stage 6-1" mechanism) — the naive fix (update
  `Bus::open_bus` on every DMA-driven access) still breaks all 24 Super FX/GSU golden hashes even
  after the `SuperFxBoard::map` fix above. Substantially narrowed this pass: ruled out the
  `$4016`/`$4017` joypad-read open-bus blend, the generic CPU-side open-bus-fallback arms, and
  `VideoBus::cart_read` (confirmed dead code, never actually called) as the mechanism. Confirmed a
  real, reproducible CPU-control-flow divergence (a spin-loop signature) exists, but the exact
  first diverging instruction wasn't isolated before this pass's budget was spent. Still
  documented-not-landed; see `docs/scheduler.md` §Open bus via DMA/HDMA for the full trail.
- **DRAM refresh (40 clocks/scanline)** — empirically measured (500 steady-state frames × 3
  unrelated ROMs): the current CPU-driven master-clock model already reproduces the correct
  357,368-clock NTSC frame length to within natural instruction-boundary quantization noise
  (average gap within a fraction of a clock of zero). Implementing the originally-planned
  additive stall would inflate every frame by ~10,480 clocks — a large, clearly-wrong regression
  against this now-confirmed-correct baseline. Concluded NOT to implement it as originally
  planned; see `docs/scheduler.md` §DRAM refresh for the full methodology and the two open
  hypotheses for what a correct future implementation would need.
- **Fractional-timebase refactor go/no-go** (`docs/adr/0002`) — assessed the refactor's own gate
  ("residuals that only sub-cycle resolution can close") against every currently-named accuracy
  residual. None qualify — each is a ROM-sourcing gap, a coprocessor-board scope gap, or a
  bug/validation question answerable within the existing whole-master-clock-tick model.
  Recommendation: do not start the refactor. See
  `docs/audit/fractional-timebase-go-no-go-2026-07-11.md`.

## [1.0.1] "Aftertouch" - 2026-07-11

**Versioning note:** both items below are additive and off-by-default/opt-in in effect (existing
behavior is unchanged unless the user mutes a voice or presses a hotkey), which this project's own
SemVer convention (`master-core` module 10: "ship additive, off-by-default changes as MINOR") would
normally ship as `v1.1.0`. This release ships as **`v1.0.1`** instead, per explicit user instruction
overriding that convention for this cut specifically.

### Added

- **Per-voice audio mute** (Settings → Audio, 8 checkboxes, `config.audio.voice_mutes`) — a
  frontend/debug convenience with **no real S-DSP hardware register behind it** (real hardware
  only has the whole-mix `FLG.6` mute bit); gates `Dsp::voice_output`, the single point strictly
  downstream of BRR decode/envelope/pitch computation, so muting cannot perturb any
  ROM-observable register (`OUTX`/`ENVX`/`ENDX`) or envelope timing. Re-synced once per real frame
  (`Bus::set_voice_mutes`), excluded from save-states (same "frontend convenience state, re-synced
  unconditionally, not part of the deterministic core" pattern as cheats/watchpoints/breakpoints).
  All unmuted by default — byte-identical to every prior release. See `docs/apu.md` §Per-voice mute.
- **Global keyboard hotkeys** — every system/emulation action used to be menu-bar-only
  (`rustysnes help hotkeys` said so explicitly; this is now corrected). A fixed, non-rebindable
  hotkey table now works anywhere the window has focus: `Escape`=Quit, `F1`=Save State, `F2`=Reset,
  `F3`=Power Cycle, `F4`=Load State, `F5`=Rewind, `F9`=Save States… window, `F11`=Fullscreen,
  `F12`=Open ROM, `Space`=Pause/Resume, `` ` ``=Toggle Debugger overlay (feature-gated:
  `debug-hooks`, mirrors the Debug menu's own gating — no second way to reach a surface the
  default build never vets). Checked on the key-down edge only, never on OS auto-repeat, and
  suppressed while an egui widget (e.g. a Settings text field) has keyboard focus. The key-map
  avoids every default P1 gameplay binding. See `docs/frontend.md` §Global hotkeys.

## [1.0.0] "Zenith" - 2026-07-10

The production cut: `Board: Send` (unblocking the dedicated `emu-thread` feature to
compile/test/lint for the first time), the five desktop-UX-shell-maturity items, a CI frame-time
performance-regression gate, a `cargo full-build`/`full-run` alias pair, an enhanced native CLI,
a full README rewrite, and a GitHub Pages demo-page polish pass. `docs/frontend.md` documents
every item below in depth.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`, 27 suites — the 65C816 per-opcode oracle, SPC700 oracle, gilyon on-cart
CPU, undisbeliever PPU/DMA/HDMA golden framebuffers, blargg `spc_*`, DSP-1/Super FX/SA-1
commercial-ROM validation, save-state determinism across all three board tiers, and the
save-state `FORMAT_VERSION` backward-compat fixture) is green; no golden hash changed. `no_std`
and both wasm32 frontends (`wasm-winit`, `wasm-canvas`) build clean; a real `trunk build` produced
a genuine ~7.4 MB wasm bundle (not an empty stub), confirming the Pages demo deploy will succeed.

### Added

- **`Board: Send`** — `rustysnes_cart::Board` now requires `Send` (the RustyNES `Mapper: Send`
  rule), the one change needed to make `Arc<Mutex<EmuCore>>: Send` for the `emu-thread` closure.
  Every existing board/coprocessor implementation compiled clean with no further changes needed.
  `emu-thread` now compiles, tests, and lints clean for the first time, but stays off-by-default:
  its loop has no audio output yet and doesn't drive cheats/watchpoints/breakpoints/scripting/
  movies/rewind/run-ahead/RetroAchievements (a real feature-parity gap vs. RustyNES's own mature
  `emu_thread.rs`, documented rather than silently promoted to default).
- **Input rebind grid** (`ui_shell.rs`, `input.rs`) — Settings → Input now has a working per-button
  P1 key-rebind grid; clicking "Rebind" arms a capture that intercepts the next physical key press
  (via `App::window_event`) instead of latching it as gameplay input. Persists to `config.toml`.
- **Thumbnail Save States manager** (`save_states.rs`) — a new disk-backed, 10-slot,
  thumbnail-previewed Save States window (Emulation → Save States…), additive alongside the
  existing RAM-only quick-save slot. Slots are keyed per-ROM by SHA-256
  (`rustysnes_core::movie::hash_rom`) under the platform data directory; each slot file wraps an
  UNMODIFIED `EmuCore::save_state()` blob in a small frontend-only header carrying a
  nearest-neighbor-downsampled thumbnail — no `rustysnes-savestate` `FORMAT_VERSION` bump needed.
- **Themes** (`config.rs`, `ui_shell.rs`) — Light/Dark/System, applied via `egui::Visuals`, live in
  Settings → System; a change-guard (`Active::applied_theme`) re-themes only on an actual change.
- **Speed presets** (`ui_shell.rs`, `pacing.rs`) — 25%/50%/75%/100%/150%/200%/300% presets in a new
  Emulation → Speed submenu, live-reconfiguring `Pacer`'s target rate (`Pacer::set_rate`) and
  scaling the audio resampler's DRC ratio so alt-speed audio pitch-shifts instead of over/
  underrunning the ring. Transient session state, never persisted — always launches at 100%.
- **Performance panel** (`ui_shell.rs`) — View → Performance panel: FPS, speed, frame time, audio
  ring health, and a rolling ~2-second frame-time sparkline (hand-drawn via `Painter::line`, no
  new dependency).
- **Fullscreen toggle** and **first-run welcome modal** (`ui_shell.rs`, `app.rs`, `config.rs`) —
  View → Fullscreen (borderless, the same change-guard pattern as theme/present-mode); a one-time
  orientation window shown on the very first launch (`config.first_run_seen`).
- **Frame-time performance-regression CI gate** — `.github/workflows/ci.yml`'s `bench` job +
  `scripts/bench_regression_check.sh`, ported from RustyNES's own pattern: runs
  `headless_frame_steady_state` on release-tag pushes, asserting the steady-state mean stays under
  an absolute 10 ms/frame ceiling (deliberately non-flaky — an absolute ceiling, not a tight
  %-regression check, since shared CI runners are too noisy for the latter).
- **`cargo full-build` / `cargo full-run`** (`.cargo/config.toml`, ported from RustyNES) — one
  command builds/runs the maximal native binary via a new `full` feature aggregating every native
  opt-in flag (`debug-hooks`, `scripting`, `cheats`, `netplay`, `retroachievements`, `hd-pack`);
  `emu-thread` is deliberately excluded (not yet feature-complete, and combining it with
  `scripting` specifically fails to compile under `-D warnings` today).
- **Enhanced native CLI** (`cli.rs`) — expanded from 4 to 9 help topics (`controls`, `hotkeys`,
  `gamepad`, `features`, `coprocessors`, `config`, `scripting`, `netplay`, `about`), replacing
  stale v0.1.0-scaffold-era text with accurate `v0.9.0`-era content; added `long_about` and a
  richer `--help` footer.
- **README.md rewrite** to match RustyNES's structural depth (Overview, Why, Feature highlights,
  Crates & Architecture, Quick Start, Desktop UX, Default Controls, Compatibility and Accuracy,
  Performance, Platform Support, Documentation, Current Release, Roadmap, Contributing, License,
  Acknowledgments) — accurately describing RustySNES's own `v0.9.0`/`v1.0.0`-in-progress state,
  not copied from RustyNES's own far more mature `v2.0.4` content.
- **Hosted demo page polish** (`crates/rustysnes-frontend/web/index.html`) — a visible title, a
  keyboard-controls + feature-parity hint (including an honest disclosure that the Save States
  manager has no filesystem to persist to in the browser), an inline-SVG favicon, and
  `theme-color`/description meta tags. Deliberately not ported from RustyNES's own page: the
  touch-controls overlay, PWA manifest/service worker, browser-Lua panel, and `?settings=`
  share-link — none of those features exist in RustySNES.

### Known gaps, tracked not hidden

- Per-channel audio mutes did not land in this pass — needs its own scoped follow-up (S-DSP
  per-voice model research) rather than being rushed to hit this list. (The save-state
  `FORMAT_VERSION` backward-compat fixture + regression test, once thought still open, turned out
  to already be landed — `tests/golden/savestate-v1-gilyon.bin` +
  `tests/save_state_backward_compat.rs`, from `v0.7.0 "Resolution"`.)
- RustySNES does not yet have global keyboard hotkeys (Reset/Power-Cycle/Pause/Save-States/Speed/
  Fullscreen are all menu-bar-only today) — `rustysnes help hotkeys` states this plainly.

## [0.9.0] "Threshold" - 2026-07-10

Closes out Phase 7's last open exit criterion (niche peripherals) and Phase 8's one remaining
ticket half (T-81-001 PR B), and resolves the previously carried-forward SPC7110 boot
investigation — the last loose ends before the `v1.0.0` production push
(`to-dos/VERSION-PLAN.md`'s `v1.0.0` gate is next: `Board: Send` + desktop UX shell maturity are
what remain there). `v0.9.0` had never been used before this — earlier drafts once mislabeled a
different rung with it before it was corrected to `v0.8.0` — so this genuinely picks up right
after `v0.8.0 "Community"`.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`, 17 suites) is green; no golden hash changed.

### Added

- **SNES niche peripherals: Mouse, Super Scope, and Super Multitap** (`rustysnes_core::controller`) — Phase 7's last open exit criterion, a real 2-bit-per-clock (`data1`/`data2`) serial-shift-register protocol per controller port, ported from ares' `sfc/controller/{mouse,super-scope,super-multitap}`, not a stub. `Bus::set_port_device` selects the peripheral per port (default: `Gamepad`, byte-identical to every prior release); `Bus::set_mouse`/`set_superscope`/`set_multitap_pad` feed host input once per frame, matching `set_joypad`'s own convention. Also added: WRIO (`$4201`/`$4213`) IOBIT register plumbing (previously entirely unimplemented) and `Ppu::latch_hv_counters` (the PPU H/V-counter latch a Super Scope's light sensor drives via IOBIT's falling edge — the same mechanism `$2137`'s software latch already used, now shared). Save-stated as real controller-port hardware state (`FORMAT_VERSION` 2→3, `docs/adr/0006`); 14 unit tests cover the shift-register/edge-detection protocols directly. The frontend gained a Settings → Input control to select controller port 2's peripheral (`config.port2_peripheral`); live host-input capture (a real mouse pointer driving Super Scope aim/Mouse deltas, extra gamepads for Multitap sub-pads) is a follow-up frontend task, not yet wired (`docs/frontend.md` §Peripherals).
- **65C816 disassembly view + PC breakpoints + step/step-over/step-into** (T-81-001 PR B, the debugger overlay ticket's remaining half after PR A's live-state panels and T-81-001b's watchpoints) — entirely frontend-side (`emu.rs`): `EmuCore::disassembly_window` walks `rustysnes_cpu::disasm::disassemble_one` forward from PC, tracking `REP`/`SEP` so later instructions' `M`/`X`-dependent operand lengths decode correctly across a width change; `EmuCore::set_breakpoints` (re-synced every frame like cheats/watchpoints) is checked once per instruction boundary via the existing `System::step_instruction()`, changing `run_frame`'s behavior only when at least one breakpoint is armed (empty list = the exact prior fast path — full `--features test-roms` suite re-verified unchanged). Step Into/Step Over only act while paused; Step Over runs a `JSR`/`JSL` to completion via the disassembler's own mnemonic check, bounded so a non-returning subroutine can't hang the debugger. One new `rustysnes-core` API: `Bus::peek` — a genuinely side-effect-free read (unlike `CpuBus::read24`, never touches the open-bus latch or watchpoints), added because the debugger's own disassembly reads must not perturb the emulated hardware state they're inspecting. 10 new unit tests.

### Fixed

- **SPC7110's boot-crash "gap" was never an emulation bug — the local test ROM is a fan-translation, not the original cartridge.** A follow-up investigation traced the exact instruction the CPU derails on (`JSL $4FFB80`, first found in `v0.8.0`) and found the path to it is one unconditional chain of subroutine calls with no branch and no SPC7110 register touched anywhere upstream — ruling out a wrong-branch bug directly. That raised the question no prior session had asked: is this actually the commercial ROM? It is not. Three independent checks confirm it: the local dump's SHA256 doesn't match `ref-proj/ares`'s own database entry for this exact board (`SHVC-LDH3C-01`, used by no other title); its header checksum only self-validates against the file's non-standard 7 MiB size, not the real cartridge's documented 5 MiB; and a public nesdev.org forum thread on this exact fan-translation documents that it adds a 1 MiB "Expansion ROM" region mapped at CPU banks `$40-$4F` — precisely the bank the derailing `JSL` targets — that exists only in the patch, on no real hardware. RustySNES's `$40-$7D`-unmapped fix (`v0.8.0`) is correct for the real cartridge; it was never meant to (and shouldn't) implement a fan-patch-only memory region. This reclassifies SPC7110 from "open boot-crash bug" to "correctly implemented, blocked on sourcing a genuine original-cartridge dump" (sha256 `69d06a3f3a4f3ba769541fe94e92b42142e423e9f0924eab97865b2d826ec82d`) — full evidence chain in `docs/audit/spc7110-boot-crash-2026-07-08.md`, cross-referenced from `docs/STATUS.md`, `docs/cart.md`, and `docs/rom-test-corpus.md`.

## [0.8.0] "Community" - 2026-07-10

Sprint 2 of Phase 8 Reach: GGPO-style rollback netplay, native RetroAchievements support, and the
extended byte-identical-with-flags-off CI gate, alongside a follow-up debugger pass (65C816
read/write watchpoints, a minimal disassembler) and a continued SPC7110 boot investigation that
found and fixed four real bugs — including a systemic cart-layer open-bus fix that benefits every
board — plus a new per-mapper/coprocessor ROM test-corpus inventory doc. `v0.9.0` was never used;
this release picks up directly from `v0.7.0`.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`) is green; SPC7110 still does not reach a bootable screen (tracked
honestly, not claimed fixed — `docs/adr/0003`).

### Added

- **65C816 read/write watchpoints — `v0.8.0`, T-81-001b.** A new `debug-hooks` feature on
  `rustysnes-core` itself (previously the flag only existed as a frontend UI gate) adds
  `rustysnes_core::watchpoint`: an armed address list checked in `CpuBus::read24`/`write24` (an
  `is_empty()` fast path keeps the accuracy-critical Bus read/write path free when nothing is
  armed), recording up to 256 hits (a ring, oldest dropped first) per poll. Mirrors the existing
  `cheats` feature's architecture exactly (`Bus::set_watchpoints`/`take_watchpoint_hits`, synced
  once per real frame). The frontend's debugger overlay gained a Watch panel (address + R/W/RW
  entry, an armed list with remove buttons, and a scrollable hit log) — `debug-hooks` on the
  frontend crate now also forwards to `rustysnes-core/debug-hooks`. Never part of save-state (host
  debug tooling, not emulated state — `docs/adr/0004`).

- **A minimal 65C816 disassembler — `rustysnes_cpu::disasm`, `v0.8.0`.** Decode-only, not wired
  into execution: `disassemble_one` takes a byte-peek closure and returns a human-readable
  `"MNEMONIC operand"` string plus instruction length, covering the full standard 256-opcode WDC
  65C816 map (11 unit tests, including a full-opcode-table decode sweep). Built for the frontend's
  debugger overlay and for ad hoc instruction-level tracing (used immediately below).

- **Three real SPC7110 addressing/timing bugs found and fixed, and the boot-completion gap
  substantially narrowed and precisely relocated — `v0.8.0`.** Reading `ref-proj/ares`'s
  `sfc/coprocessor/spc7110/` directly confirmed real hardware's SPC7110 runs as its own cothread
  at the master-clock rate, deferring a `$4806` DCU-begin-transfer / `$4825` multiply / `$4827`
  divide trigger by one tick rather than completing it synchronously within the register write.
  Ported faithfully (`Spc7110Board::coprocessor_tick`, unit-tested including a new deferral-proof
  test) — a real, independently-verified accuracy fix. Using the new watchpoint hook to trace the
  one committed SPC7110 title's boot (Far East of Eden Zero) confirmed this fix does **not** close
  the previously-tracked boot-completion gap: those triggers are never actually written during
  this boot's crash path. The new disassembler then found the earlier "stall loop" framing was
  itself incomplete — the CPU spends most of its time in a real, coherent VRAM-upload loop (bank
  `$4F`) — until it hits a literal jump into a bank that, per `ref-proj/ares`'s own board database
  (`board: SHVC-LDH3C-01`, the exact board this title uses), should be entirely unmapped. Fixed
  two more real bugs found this way: the `$40-$7D` range was wrongly treated as a `$C0-FF` mirror
  (an earlier session's claim, never actually checked against the database); and the DROM buffer
  was 2 MiB oversized (the committed 7 MiB dump vs. the real 5 MiB of physical chip content),
  corrupting the `bus_mirror` fold length for any high DROM offset. All three fixes independently
  verified against ares' authoritative source, 9/9 `spc7110` unit tests plus the full workspace
  suite green — but none of them close the gap: with the mapping now correct, the game's own PROM
  code still jumps into that (now-correctly-unmapped) space, meaning real hardware must diverge
  from this emulation even earlier, in a stretch of boot code not yet traced. Full trail and next
  steps in `docs/audit/spc7110-boot-crash-2026-07-08.md`; `docs/cart.md`/`docs/STATUS.md`'s
  SPC7110 entries updated to match — still not claimed boot-validated (`docs/adr/0003`).

- **A real, systemic open-bus bug found and fixed — `rustysnes-cart`, `v0.8.0`.** Continuing the
  SPC7110 investigation past the `JSL $4FFB80` dead end (rather than stopping at it) exposed that
  the cart layer's open-bus fallback was itself wrong, independent of any board-specific logic:
  `Board::read24`'s `MappedAddr::Open` case (and every board's own override, SPC7110's included)
  returned a hardcoded `0` instead of the real bus open-bus latch. Checking ares' actual bus-read
  plumbing (`sfc/cpu/memory.cpp`, `sfc/memory/inline.hpp`) confirms real hardware's open bus
  echoes back the CPU's own MDR (the last byte actually driven on the data bus) — a `0` fetch is
  `BRK`, so this emulator's cart-space open bus reliably BRK-storms on any wild jump into unmapped
  cart space, where real hardware often keeps running (harmlessly or not). Fixed: `Cart::read24`
  (`crates/rustysnes-cart/src/lib.rs`) now takes the caller's open-bus byte as a parameter and
  echoes it back for a genuinely `MappedAddr::Open` address, exactly mirroring ares'
  `Bus::read(address, data)` — both of `rustysnes-core`'s call sites (`Bus::cart_read_raw`,
  `CartView::cart_read`) now thread their own `open_bus` field through. Benefits every board, not
  only SPC7110: re-tracing Far East of Eden Zero's `JSL $4FFB80` dead end with this fix in place
  now shows a stable, harmless open-bus spin loop instead of the previous BRK/RTI oscillation —
  more honestly modeled, though it still doesn't close the boot gap (full trail in
  `docs/audit/spc7110-boot-crash-2026-07-08.md`). **Two golden vectors re-blessed as an intentional
  consequence** (`docs/adr/0003`'s honesty gate: update a golden only on a reviewed, understood
  behavior change, never silently): `tests/golden/sa1-framebuffer.tsv` (SD F-1 Grand Prix's sampled
  frame now matches the same "SA-1 not yet live" hash most other titles already shared) and
  `tests/golden/superfx-framebuffer.tsv` (24 of the Krom `FillPoly`/`PlotLine`/`PlotPixel` draw-
  primitive ROMs, whose setup code touches cart open bus). Both suites' non-hash assertions
  (detection, liveness, substantial-bitmap-plotted) held throughout — only the informational
  determinism-drift hash moved, and only for titles/ROMs that actually exercise open bus.

- **`docs/rom-test-corpus.md` — a per-mapper/coprocessor/test-category ROM inventory.** Catalogs,
  for every mapper (LoROM/HiROM/ExHiROM) and coprocessor (DSP-1..4, Super FX, SA-1, S-DD1, OBC1,
  CX4, ST010/ST011/ST018, S-RTC, SPC7110), the best available test ROM and its concrete
  availability: committed corpus, gitignored external corpus, present in the local Dropbox ROM
  collection, or genuinely unavailable — including PAL and hi-res-specific golden-boot gaps that
  stay honestly open for lack of a suitable dump anywhere on this machine.

- **The byte-identical-with-flags-off CI gate, extended for Sprint 2's two new flags —
  `v0.8.0 "Community"`, T-82-004.** `.github/workflows/ci.yml`'s `lint` job now clippys
  `netplay` and `retroachievements` individually (alongside Sprint 1's `debug-hooks`/
  `scripting`/`cheats`) and combined (`debug-hooks,scripting,cheats,netplay,retroachievements`)
  — still never `--all-features`, since `wasm-winit`/`wasm-canvas` stay mutually exclusive. The
  existing `--no-default-features --features wasm-winit,help-tui` flags-off guard needed no
  change (its value is exactly that it stays a fixed, named regression lock regardless of how
  many optional flags accumulate around it) and passes with all five Phase 8 flags compiled out.
  `full-test`'s Linux-only combined-feature behavioral run (ahead of every tagged release) is
  extended to the same five-flag combo — `retroachievements` vendors and compiles `rcheevos` via
  `cc`, real cross-platform build surface `lint` never exercises, the same category `scripting`'s
  vendored `mlua` already established the Linux-only scoping for.

- **GGPO-style rollback netplay — `v0.8.0 "Community"`, T-82-002.** A new `rustysnes-netplay`
  crate implements two-player rollback netcode, ported from RustyNES's own proven
  `rustynes-netplay::session::RollbackSession` shape (the N-player mesh/Roster/spectator/NAT-
  traversal breadth RustyNES also carries is deliberately NOT ported — out of this ticket's
  stated scope, and the SNES core itself only has two physical controller ports, no multitap
  emulation, so 2 players is the core's own real ceiling, not an arbitrary cut).
  - **The rollback loop**: every real frame, predict the remote player's input (repeat its last
    known value), run the frame, and keep a checkpoint (a full `System::save_state()` snapshot)
    at the last confirmed frame. A contradicted prediction restores the checkpoint and
    re-simulates forward with corrected input. The checkpoint itself advances as confirmation
    catches up (bounding resimulation distance instead of always replaying from frame 0), and a
    periodic desync checksum is computed only from state that's already fully settled — an
    earlier draft computed it from possibly-still-predicted "live" state, which raced an
    eventual correction and produced a false-positive desync between two peers that were, in
    fact, converging correctly; fixed before landing.
  - **Reliability**: a dropped `Input` packet is resent every `advance()` call until the remote
    peer's cumulative `InputAck` catches up — an earlier draft had no resend path at all, which
    permanently stalled a session the first time a single packet was lost under any non-zero
    packet-loss condition; fixed before landing (caught by the adverse-conditions determinism
    test, not just reasoned about).
  - **Proof, not assertion**: `tests/determinism.rs` drives two sessions over a seeded,
    deterministic `MemoryTransport` — one run under ideal (zero-latency) conditions, one under
    real synthetic latency + jitter + 10% packet loss — and asserts both sessions' per-frame
    framebuffer hash sequence matches a fresh, no-rollback reference run exactly, frame for
    frame, under both conditions.
  - **Transports**: `udp.rs`'s `UdpTransport` is a real `std::net::UdpSocket`, proven by a
    genuine OS-level loopback round-trip test. `webrtc.rs`'s `WebRtcTransport` wraps a
    `web_sys::RtcDataChannel`, wasm32-clippy-verified against the real API. **Honest scope
    note**: the frontend's UI wiring is native/UDP only this pass — the browser-side SDP
    offer/answer/ICE negotiation glue needed to actually establish a `RtcDataChannel` is a
    genuinely separate scope of async signaling work, not half-wired in.
  - **Frontend integration**: a new `netplay` feature (native-only) adds a Tools → Netplay…
    window (local/peer `host:port`, a P1/P2 slot picker, Connect/Disconnect) and a
    `NetplayState`. `Active::render`'s per-frame loop dispatches to `NetplayState::drive`
    (which calls `RollbackSession::advance` directly on `System`) via an early `continue` that
    skips the entire single-player `apply_frame_input`/cheats/rewind/script/`run_frame` path for
    that iteration whenever a session is connected — netplay's own drive loop, verified
    independent of `emu-thread`, never both driving the same `System`. A new
    `EmuCore::present_current_frame` splits `run_frame`'s framebuffer-decode/audio-drain half
    out on its own, since `RollbackSession::advance` drives the core crate's `System` directly
    (not this frontend's `EmuCore`) and only the session's own settled result — not each
    internal resimulation pass — should ever reach the screen. **Known limitation, shared with
    rollback netplay generally, not specific to this implementation**: video always reflects
    the corrected state cleanly, but audio already sent to a real output device during a
    since-corrected misprediction can't be "unplayed" — a rollback event may audibly glitch,
    the same accepted artifact GGPO-family netcode has elsewhere.
  - With `netplay` off, the crate's frontend wiring compiles out entirely (`rustysnes-netplay`
    itself stays an always-compiled workspace member, same precedent as `rustysnes-script`); full
    default-feature workspace build/test/clippy/fmt/doc verified unaffected.
  - **Hardening from review, before merge**: an untrusted `Input`/`Checksum` message's `frame`
    index is now bounds-checked before it can grow `history` (an unbounded value could otherwise
    force an arbitrarily large allocation); the pending-remote-checksum queue is capped rather
    than growing without bound; nothing from the remote peer is acted on before its `Sync`
    handshake has verified the ROM hash + protocol version (`ingest`/`advance` both gate on it);
    a misprediction-detection condition that referenced a predicted slot's `confirmed` flag —
    always `false` for a genuine prediction, so it never actually fired — was corrected (the
    underlying resimulation was already correct via the `confirmation_advanced` path, proven by
    the passing determinism tests either way; only the public `AdvanceOutcome::rolled_back` flag
    was misreporting); `settle_if_confirmed`'s duplicate `sys.save_state()` call was collapsed to
    one (reused for both the checkpoint and the checksum hash); `SessionConfig::input_delay` —
    documented but never read — is now wired into `add_local_input`, proven against a
    delay-aware reference test; and `predict_remotes`'s O(frame) backward scan was replaced with
    an O(1) read (frames are predicted in strictly increasing order, so the previous frame's
    slot already holds the correct last-known value by induction).

- **RetroAchievements (opt-in, native FFI) — `v0.8.0 "Community"`, T-82-003.** A new
  `rustysnes-cheevos` crate wraps the vendored `rcheevos` `rc_client` C API (MIT-licensed,
  vendored verbatim from RustyNES's own `rustynes-cheevos/vendor/rcheevos` copy — confirmed
  byte-identical via `diff -rq`, matching `rustysnes-script`'s already-established
  vendoring-under-`docs/adr/0003` precedent), native-only (`#![cfg(not(target_arch =
  "wasm32"))]`; the vendored C library needs a C toolchain + `std`, and this pass has no
  browser-side HTTP worker model for RA server calls).
  - **The FFI boundary**: hand-written `extern "C"` declarations (not bindgen output) transcribed
    from the vendored headers, with every `#[repr(C)]` struct's layout pinned against the ACTUAL
    C `sizeof` via a `static_asserts.c` translation unit's `rc_cheevos_sizeof_*()` accessors (not
    numbers hardcoded from one build host) — a future vendor bump that changes a struct layout
    fails loudly at build time, on every platform, not just the one it was written on.
  - **Callback bridging**: `rc_client`'s three C callbacks (read-memory, server-call,
    event-handler) are bridged to safe Rust via thread-local raw pointers installed by RAII
    guards for exactly the duration of one `rc_client_*` call (`ReadGuard`/`TransportGuard`);
    async completions (login/load-game) bridge through a boxed `FnOnce` passed as the C API's
    opaque `callback_userdata`. HTTP itself runs on a dedicated worker thread owning a `ureq`
    agent — the `server_call` trampoline only enqueues a job and returns immediately, never
    blocking the emulator thread; `RaClient::poll_http_completions` drains finished exchanges
    and invokes rcheevos' completion callbacks back on the calling (render) thread.
  - **SNES memory mapping, verified not guessed**: `ra_addr_to_snes` maps RA's flat address space
    to the SNES CPU bus by reading the ACTUAL `RetroAchievements/RASnes9x` integration source
    (`win32/RetroAchievements.cpp`'s `RA_InstallMemoryBank(0, ByteReader, ByteWriter, 0x20000)`,
    whose `ByteReader` returns `Memory.RAM[nOffs % 0x20000]`) rather than assuming a mapping:
    RA flat `0x000000..0x01FFFF` (128 KiB) identity-maps to WRAM `$7E0000..$7FFFFF`. Cartridge
    SRAM (RASnes9x's bank 1) is an honest, documented scope cut — most SNES achievement sets
    target WRAM; a follow-up can add the SRAM bank once a set that needs it surfaces.
  - **User-Agent identification**: `RA_USER_AGENT` leads with `RustySNES/<crate version>` (the
    token RA allowlists a client by) followed by a canonical `rcheevos/<version>` clause parsed
    from the vendored `rc_version.h` at build time (`build.rs`'s `emit_rcheevos_version`) — a
    regression test (`ra_user_agent_identifies_rustysnes_with_versions`) guards both the leading
    name and the version clauses' presence.
  - **Frontend integration**: a new `retroachievements` feature (native-only) adds a Tools →
    RetroAchievements… login window (username/password, a Log in/Log out button) and a
    `CheevosState` (`crates/rustysnes-frontend/src/cheevos.rs`) that creates the `rc_client`
    lazily on first login attempt, bridges the async login completion through a shared
    `Rc<RefCell<Option<Result<...>>>>` slot (the completion closure must be `'static` and so
    can't hold `&mut CheevosState` directly), and drives one `rc_client` frame per emulated frame
    (`CheevosState::do_frame`, reading WRAM through the same `Bus::peek_wram` the
    debugger/scripting integrations already use — read-only, no new mutation path).
    Achievement-unlock events surface as status-bar toast messages. **Honest scope notes**: not
    wired into the netplay `drive` path (a `RollbackSession`-driven `System` and achievement
    tracking interacting — e.g. resimulation re-triggering rc_client frames — is a separate,
    deferred concern); no leaderboard/rich-presence UI panel yet (the `RaClient` API already
    exposes both).
  - With `retroachievements` off, `rustysnes-cheevos` never enters the frontend's dependency
    graph (`dep:rustysnes-cheevos`) and every wiring site is feature-gated; full default-feature
    workspace build/test/clippy/fmt/doc verified unaffected.

- **Netplay save-state cost benchmark + rollback go/no-go call — `v0.8.0 "Community"`,
  T-82-001.** A new Criterion benchmark (`crates/rustysnes-core/benches/save_state_cost.rs`)
  measures `System::save_state()`/`load_state()` cost across three board tiers (no-coprocessor,
  Curated Super FX, BestEffort CX4) — pre-work before T-82-002's rollback netplay, which calls
  save/restore far more often than `RewindBuffer`'s ~10 Hz design point. Result: **GO** — all
  three tiers cluster tightly (~108 µs save, ~295 µs load) regardless of which coprocessor is
  active (cost is dominated by the fixed-size WRAM/VRAM/CGRAM/OAM/ARAM buffers every board
  carries, not coprocessor state), and both numbers are negligible next to a single frame's own
  ~3.27 ms execution cost (the `v0.4.0` baseline) — the existing full-snapshot design
  (`docs/adr/0006`) is fast enough for a real rollback window; no delta/incremental redesign is
  needed before T-82-002 proceeds. The Curated/BestEffort benchmarks self-skip when their
  commercial ROM is absent (gitignored corpus, `docs/adr/0003`), matching
  `commercial_screenshots.rs`'s own convention. Full write-up in `docs/benchmarks.md`.

- **The byte-identical-with-flags-off CI gate, extended for Sprint 1's three new flags —
  `v0.8.0 "Instrumentation"`, T-81-004.** `.github/workflows/ci.yml`'s `lint` job (runs on
  every PR/push to `main`) now clippys `debug-hooks`, `scripting`, and `cheats` individually
  and combined (`debug-hooks,scripting,cheats`) — never `--all-features`, since `wasm-winit`/
  `wasm-canvas` are mutually exclusive and an all-features build wouldn't even make sense.
  Also adds an explicit `--no-default-features --features wasm-winit,help-tui` clippy step: a
  named, protected flags-off regression guard, distinct from (if currently redundant with)
  plain default-feature clippy — it stays correct even if a future change ever folds one of the
  new flags into `default` without updating this line too. `full-test` (the exhaustive 3-OS,
  release-tag-gated battery) additionally runs `cargo test -p rustysnes-frontend --features
  debug-hooks,scripting,cheats` on Linux ahead of every tagged release, for real behavioral
  coverage of the combined flags, not just clippy — scoped to Linux only, since `scripting`
  vendors and compiles Lua 5.4 via `mlua`'s C source, and validating that specifically on
  macOS/Windows is a genuinely separate question from "is the gate wired up," out of this
  ticket's scope. Closes out `v0.8.0 "Instrumentation"` Sprint 1 (T-81-001 through T-81-006 all
  landed).

- **Game Genie / Pro Action Replay cheat-code support — `v0.8.0 "Instrumentation"`, T-81-003.**
  A new `rustysnes_core::cheat` module decodes SNES Game Genie (`XXXX-XXXX`, 9 characters
  including the dash, the 16-character alphabet `DF4709156BC8A23E`) and Pro Action Replay (8 hex
  digits, `AAAAAADD` — 6 hex-digit address, 2 hex-digit value, no scrambling) codes into a plain
  24-bit CPU-bus `(address, value)` patch. Ported from bsnes's `CheatEditor::decodeSNES`
  (`ref-proj/bsnes/bsnes/target-bsnes/tools/cheat-editor.cpp`) and cross-checked bit-for-bit
  against Mesen2's independent `CheatManager::ConvertFromSnesGameGenie`/
  `ConvertFromSnesProActionReplay` (`ref-proj/Mesen2/Core/Shared/CheatManager.cpp`) — both
  decoders compute an identical address and value for any given code. Unit tests decode real
  commercial codes drawn from Mesen2's shipped cheat database
  (`ref-proj/Mesen2/UI/Dependencies/Internal/CheatDb.Snes.json`) as an external-oracle check, not
  self-asserted values. Neither SNES format supports a compare byte, and neither needs LoROM/
  HiROM bank translation in the decoder — that stays the Bus's job. **A decoded patch is applied
  as a `Bus::read24` CPU-read intercept (`Bus::set_cheats`), not a WRAM poke**: like NES's own
  Game Genie, real SNES Game Genie/Pro Action Replay hardware is a pass-through cart that
  intercepts cartridge-ROM reads — the review-caught test vectors above (`$02B1DD`, `$00993D`)
  are themselves ROM addresses, so a `Bus::poke_wram`-only application (the initial design) would
  have silently done nothing for virtually every real Game Genie code. `Bus::read24` checks the
  installed patch list once per CPU-visible read (empty in every build that never calls
  `set_cheats`, costing one branch when inactive) and substitutes a matching patch's value; the
  underlying ROM/RAM byte itself is never modified. A cheat is host-applied external input
  (`docs/adr/0004`), not emulated hardware — with the new `cheats` feature off, or no entries
  enabled, nothing here executes and the determinism contract is untouched. A new Tools →
  Cheats… window (native and `wasm32` both — unlike `scripting`'s `mlua`, cheat decoding is pure
  computation with no platform constraint) lets a user type a code, see it decoded (or a
  parse-error message), enable/disable it, and remove it; the enabled set is re-installed into
  `Bus` every real frame (`crate::cheats::sync`). In-memory only for this pass — no per-ROM disk
  persistence yet, matching the frontend's own quick-save slot's current in-memory-only maturity
  level (a `RustyNES`-style per-ROM-SHA256 TOML file is a natural follow-up once save-states
  themselves persist to disk). With `cheats` off, the crate's cheat-list/UI code compiles out
  entirely (the decode module itself stays unconditional in `rustysnes-core`, same as the
  `movie` module) — full default-feature workspace build/test/clippy/fmt/doc verified
  unaffected.

- **Sandboxed Lua scripting + TAS movie record/playback — `v0.8.0 "Instrumentation"`, T-81-002.**
  Fills in the previously-empty `rustysnes-script` crate stub with both halves of its stated
  scope in one pass, behind the existing `scripting` flag.
  - **Lua scripting**: `ScriptEngine` wraps `mlua` 0.12 (vendored Lua 5.4, `["lua54", "vendored"]`
    — deliberately NOT `"send"`, since the `MaybeSend` bounds it imposes on `set_hook`/
    `create_function` are incompatible with the `Rc<Cell<_>>`/`Rc<RefCell<_>>` internal state this
    engine uses, and `ScriptEngine` never needs to cross threads: `emu-thread` is off by default
    and not yet functional, since `rustysnes-cart::Board` isn't `Send` yet). Scripts run in a
    hard sandbox: only `TABLE`/`STRING`/`MATH`/`COROUTINE` stdlibs are loaded, and `load`/
    `loadfile`/`dofile`/`loadstring`/`collectgarbage`/`require`/`package`/`io`/`os`/`debug` are
    explicitly nilled as belt-and-suspenders on top of the stdlib allowlist (verified: a unit
    test asserts `io.open`, `os.execute`, and `require('os')` all fail). A per-frame instruction
    budget (`Lua::set_hook` on `every_nth_instruction`, default 1,000,000) interrupts a runaway
    script loop rather than hanging the frontend (verified with a real `while true do end`
    script). `emu.read`/`emu.write` operate on WRAM only (new `Bus::joypad`/`Bus::poke_wram`
    accessors), bound via `Lua::scope` for the exact duration of one `on_frame` call so the `&mut
    Bus` borrow never escapes into the persistent Lua state (no `Rc<RefCell<Bus>>` needed).
    `emu.onFrame(fn)` registers a per-frame callback; `print` is redirected into an internal log
    drained by the frontend rather than going to stdout.
  - **TAS movies**: a new `rustysnes_core::movie` module (no_std, no Lua/frontend coupling —
    matching RustyNES's own crate boundary) defines the on-disk format (`RSNESMOV` magic, format
    version, region, a u64 determinism seed, the ROM's SHA-256, a start-point kind byte
    (power-on or an embedded save-state blob), then a raw `p1(u16)+p2(u16)` stream, one pair per
    frame), built on the existing `rustysnes_savestate` tag/section framing. `MovieRecorder`
    captures inputs frame-by-frame; `MoviePlayer` owns its `Movie` outright (not a borrow — a
    `MoviePlayer<'a>` design was rejected during implementation since the frontend needs to hold
    one across many real frames, which a borrow can't do without a self-referential lifetime) and
    exposes `next_frame() -> Option<FrameInput>` as pure data. **A real ordering bug was caught
    and fixed before it shipped**: `MoviePlayer` was originally going to call `Bus::set_joypad`
    directly, but `EmuCore::run_frame()` already re-applies its own retained `self.pads` array to
    `Bus::set_joypad` on every call — a direct write from the player would have silently raced
    with (and lost to) that reapplication depending on call order, so `next_frame` returns data
    only and the frontend applies it through `EmuCore::set_pad` instead. A new `System::seed()`
    accessor lets `Movie::seek_to_start` reject a power-on movie replayed against a `System` built
    with the wrong seed before any replay happens, rather than silently producing a diverged
    trace. While a movie is recording or playing, `ScriptEngine::set_writes_locked` makes
    `emu.write` a silent no-op, so a loaded script can never perturb a deterministic run it
    doesn't own.
  - **Frontend wiring**: a new Tools-menu set of actions (Load Script, Start/Stop Movie
    Recording, Load & Play Movie, Stop Movie Playback) in `ui_shell.rs`/`app.rs`, all gated
    `#[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]` (not just `feature =
    "scripting"` alone — `mlua`'s vendored Lua VM needs a C compiler + `std`, unavailable on
    `wasm32`; `rustysnes-script` is an optional dependency only under the native
    `target.'cfg(...)'` dependency table). `rustysnes-script` is declared `optional = true` there.
  - **Verified**: 5 new `rustysnes-script` unit tests (frame-callback invocation, WRAM
    read/write round-trip, writes-locked no-op, runaway-loop interruption, sandbox-escape
    rejection) and 8 new `rustysnes-core::movie` unit tests (format round-trip with both start
    kinds, malformed/truncated input rejection, ROM-hash verification, seed-mismatch rejection,
    recorder/player round-trip), all passing. A new `movie_determinism.rs` integration test
    records 40 frames of VARYING synthetic input (not a static pad — a movie that never
    exercises input divergence would pass trivially even with a broken recorder) against the
    committed `cputest-basic.sfc` ROM, round-trips the movie through its real on-disk byte
    format, replays it against a completely fresh `System`, and asserts per-frame framebuffer
    hashes and total audio hash are byte-identical between the original recording and the
    replay — the acceptance criterion, proven, not asserted. With `scripting` off, the build is
    unaffected (the crate and all its code paths are compiled out entirely).

- **The live wasm demo now runs the full native shell — `wasm-winit` unification (T-81-006).**
  `app.rs`'s `ApplicationHandler` is now ONE implementation shared by native and `wasm32` (a new
  `wasm_winit.rs` entry point, ported from RustyNES's own, not invented), with internal
  `#[cfg(target_arch = "wasm32")]` branches for the genuinely-async wgpu init (`Gfx::new_async`
  delivered back via a new `AppEvent::GfxReady` through an `EventLoopProxy`, since
  `pollster::block_on` cannot block on `wasm32` — native drives the same core synchronously),
  browser ROM loading (`AppEvent::RomLoaded` from the page's `<input id="rom-input">`, replacing
  `rfd`'s native file dialog), and audio (`wasm_audio` instead of `cpal`/`AudioOutput`). The
  window attaches to the existing `<canvas id="snes-canvas">` rather than a detached one
  (`WindowAttributesExtWebSys::with_canvas`). `Gfx` now probes `navigator.gpu`'s mere presence to
  pick `wgpu::Backends::BROWSER_WEBGPU` or `::GL` and commits to exactly one before ever touching
  the canvas — a `<canvas>` can only bind one context type for its lifetime, and a WebGPU
  `create_surface` call poisons it for a subsequent GL attempt regardless of whether
  `request_adapter` later succeeds, so a sequential try-then-fallback on the same element can
  never actually reach its own fallback. The WebGL2 (`Backend::Gl`) path also needed its own
  color-space fix: unlike WebGPU/native, its surface can't present to a real sRGB default
  framebuffer, so wgpu-hal adds an extra encode at present time that (combined with GL's own
  automatic sRGB write) breaks the sRGB round-trip and washes out the palette — fixed by keeping
  the GL backend entirely in the UNORM domain (non-sRGB surface + framebuffer texture), matching
  `wasm-canvas`'s byte-exact output. `wasm-winit` is now the crate's default wasm feature
  (`wasm-canvas`, T-81-005, remains independently selectable and fully functional — re-verified
  end-to-end with no regression after this change). **Verified with a real headless-browser
  load** (Playwright/Chromium): the WebGL2 fallback path renders correctly — a full-page
  screenshot after loading a real committed test ROM shows the egui menu bar, the status bar
  (`LoROM | Ntsc | 60.0 fps | ROM loaded`), and the actual emulated framebuffer, not a blank
  canvas (`getImageData`-based pixel counting, T-81-005's method, reads back empty on a
  WebGL/WebGPU canvas whose drawing buffer isn't preserved across presents —
  `page.screenshot()`, reading the browser's own compositor output, is what actually proved
  this). **Honest gap:** this sandbox's headless Chromium exposes `navigator.gpu` but returns
  "no compatible wgpu adapter" for a real WebGPU request despite several software-Vulkan
  launch-flag attempts — the WebGPU path shares the same `Gfx::new_async` core the verified GL
  path uses and its backend-selection/color-space reasoning is grounded in real prior hardware
  testing, but a live screenshot specifically on WebGPU is not claimed here; real-browser
  verification with actual WebGPU support is still owed as a follow-up.

- **Debugger overlay: live CPU/PPU/APU/Cart state viewers — `v0.8.0 "Instrumentation"`,
  T-81-001 (PR A of 2).** `ui_shell.rs`'s debugger window (menu entry, panel selector) has
  existed since the frontend's first cut but every panel was a literal `"TODO(impl-phase)"`
  label. This lands the state-viewer half: a new `DebugSnapshot` (mirroring `ShellInfo`'s
  own copy-out-under-the-brief-lock pattern — the shell's non-negotiable rule that egui never
  touches the emu lock directly) shows real 65C816 registers/flags, key PPU registers + the
  dot/scanline timeline + a scrollable VRAM window + full CGRAM, SPC700 PC/halt state + all 8
  S-DSP voices' key registers, and the active board name plus (when loaded) SA-1's second-CPU
  registers or the Super FX/GSU register file — resolving `docs/frontend.md`'s open question in
  the breadth-inclusive direction this whole ladder takes. New small read-only accessors added
  to `rustysnes-ppu` (`bg_mode`/`display_brightness`), `rustysnes-core` (`System::sa1_regs`),
  and a new `Board::debug_gsu_state` default-no-op trait hook (overridden by `SuperFxBoard`) —
  all read-only, no new mutation paths, zero risk to the 0-diff CPU/SPC700 oracles (verified:
  the full `--features test-roms` suite passes unchanged). The Debug menu entry that opens the
  overlay is gated behind the `debug-hooks` feature (default off) — without it, the debugger
  can never open, so the app never builds a snapshot and the default build's emulation output is
  untouched. **Deferred to T-81-006, not this pass:** the 65C816 disassembler + breakpoints/
  step controls (needs `System::step_instruction()`-driven stepping, not core changes) and
  read/write watchpoints (needs a new `debug-hooks` feature on `rustysnes-core` itself + a
  `Bus`-level hook — scoped as its own separate, focused change, T-81-001b, since it touches the
  hottest path in the engine).

- **The live Pages demo actually renders now: the `wasm-canvas` MVP (T-81-005).** Replaced
  `crates/rustysnes-frontend/src/wasm.rs`'s `v0.1.0` scaffold stub (panic hook + one log line,
  never rendered anything) with a real canvas-2D frontend ported from RustyNES's proven shape: a
  `CanvasRenderingContext2d.putImageData` blit of the existing RGBA8 framebuffer, a
  `requestAnimationFrame` loop paced by a new shared `pacing::Pacer` (extracted from `app.rs`,
  now used natively AND on wasm so a 144 Hz display doesn't run emulation 2.4x too fast), keyboard
  input via DOM `keydown`/`keyup` (reusing `input::KeyBindings` unchanged), and ROM loading via
  `<input type="file">`. Audio is a new `wasm_audio.rs`: `AudioWorkletNode` primary with a
  `ScriptProcessorNode` fallback, reusing the native DRC/resampler core verbatim (extracted into a
  new target-agnostic `audio_core.rs` specifically for this reuse, not reimplemented). No
  `wgpu`/`egui` yet — that unification is `wasm-winit`/T-81-006, not yet landed; `wasm-canvas` is
  the crate's default wasm feature for now so the live Pages build actually picks it up.
  **Found and fixed a second, deeper, pre-existing bug while verifying this with a real
  headless-browser load (Playwright/Chromium — not just an HTTP-status check, the exact gap that
  let the stub ship unnoticed since `v0.1.0`):** `web/index.html`'s trunk directive
  (`data-bin="rustysnes" data-type="main"`) built the `[[bin]]` (`main.rs`, whose wasm32 arm is an
  empty `fn main() {}` that never references the lib), not the `[lib]` cdylib — so the actual
  `#[wasm_bindgen(start)]` entry point got dead-code-eliminated entirely regardless of what code
  `wasm.rs` contained; the built `.wasm` was confirmed to be only ~14 KB with zero emulator code
  linked in. Fixed to `data-target-name="rustysnes_frontend"` (the same pattern RustyNES's own
  working `index.html` uses). `pages.yml`'s `RUSTFLAGS="-C target-feature=-reference-types"` also
  had to be removed — it broke wasm-bindgen's externref table generation once the demo actually
  linked in real `Closure`-based code; it had been a silent no-op until now because there was no
  real code for it to break. Verified end-to-end: a real committed test ROM loaded through the
  live `#rom-input` in headless Chromium produced a canvas with 28672/57344 non-black pixels and
  zero console errors. **Honest gap:** audio was verified to construct without throwing, but
  headless automation cannot conclusively prove audible output through the browser's real
  autoplay-gesture semantics — manual verification in a real browser is still owed.

### Changed

- **Folded the real wasm frontend build into `v0.8.0 "Instrumentation"`'s scope, per explicit
  direction.** The user compared RustySNES's live Pages demo against RustyNES's working one and
  found it renders a blank page — root-caused to `crates/rustysnes-frontend/src/wasm.rs` being
  an explicitly-labeled scaffold stub since `v0.1.0` (installs a panic hook, logs one message,
  returns — never builds the app or creates a canvas). Every prior "wasm demo is live"
  verification (`v0.1.0`-`v0.6.0`) checked only HTTP-level liveness, never that the app actually
  renders. Scoped as two stages ported from RustyNES's own proven shape (`wasm.rs`/
  `wasm_winit.rs`, confirmed by reading the source directly): a `wasm-canvas` MVP first (canvas-2D
  blit, no `wgpu`/`egui`, ships a real working demo fast), then `wasm-winit` unification (routes
  wasm through the same `App` native uses — requires un-gating `app.rs`/`audio.rs` from their
  current `wasm32` exclusion, a real architectural gap, not just plumbing).
  `to-dos/VERSION-PLAN.md`'s `v0.8.0` section, `to-dos/phase-8-reach/overview.md`, and
  `sprint-1-instrumentation.md` (two new tickets, T-81-005/T-81-006) updated accordingly.

### Fixed

- **Mid-scanline/HDMA-driven register timing — `v0.8.0 "Community"`.** `Ppu::tick_dot` now
  composites each scanline at `RENDER_DOT` (dot 276) instead of end-of-scanline (dot 340) —
  matching real hardware's per-pixel active-region timing, so a per-line HDMA-driven register
  write during line `V` only becomes visible starting `V+1`, not on `V` itself (`docs/ppu.md`
  §Mid-scanline/HDMA-driven register timing has the full mechanism + verification). This fix was
  designed and SA-1-verified correct months ago but blocked on an apparently-unrelated Super
  FX/GSU golden regression; what actually unblocked it was finding a separate, previously
  undiscovered bug in `Bus::advance_master`'s HDMA run-check — it read the PPU's dot counter
  *after* `tick_ppu_dot()` had already incremented it, so the HDMA-run condition matched a whole
  4-master-clock dot-window early, putting HDMA back ahead of render for the same line (the exact
  ordering the fix exists to prevent). Fixed by capturing the dot value before the tick and gating
  the HDMA-run check on the exact sub-tick that advanced it. Re-verified against the full
  `--features test-roms` golden suite: SA-1's `SD F-1 Grand Prix` golden changed to the
  pixel-exact predicted hardware-correct value; 15 of the `undisbeliever` HDMA-timing-focused
  micro-tests (`hdma-*`, `hdmaen_latch_test*`, `scpu-a-dma-bug-*`) and 24 of the Super FX/GSU
  goldens changed too — every change independently row-level-verified (not blindly re-blessed):
  the Super FX/GSU corpus's functional invariants (GSU liveness, plot-pipeline completion,
  determinism) are unaffected, and the pixel-level diffs are small, bounded, and localized
  (a couple of rows shifted per ROM, not a chaotic break) — see `docs/ppu.md` for the full
  row-by-row analysis.

- **`wasm.rs` (`wasm-canvas`): fixed a real, currently-broken build — `CanvasRenderingContext2d::put_image_data`'s dx/dy arguments must be `f64`, not `i32`.** Found live: this shipped broken in T-81-005's merge and silently failed the `wasm-canvas` build path from `main` (confirmed via the actual `pages.yml` deploy run for that merge, which failed at this exact line) — masked locally by a stray, untracked `.cargo/config.toml` left over from a different sibling project, whose `--cfg=web_sys_unstable_apis` rustflag switches `web-sys` to the *other* `put_image_data` overload (`i32` args, gated behind that unstable cfg), so local builds compiled while CI's genuinely clean environment did not. `wasm-canvas` is not the default wasm feature since T-81-006 landed (`wasm-winit` is), so this didn't affect the live demo, but it's a real defect in a still-supported, independently-selectable build path. Re-verified against an environment with that stray rustflag neutralized (`RUSTFLAGS=""`, matching CI): both `cargo clippy` and a real `trunk build` + headless-browser load now succeed genuinely, not just locally.

- **`crates/rustysnes-frontend/web/index.html`: added the missing link to `/api/` on the live
  wasm demo page.** Found live by the user comparing against RustyNES's Pages deployment (which
  has `<a href="api/">API documentation</a>` in its own footer) — RustySNES's demo page had no
  path to the API docs at all short of manually typing `/api/` into the URL bar. Added a small
  footer mirroring RustyNES's own pattern (a GitHub repo link + an API docs link), opening in a
  new tab so navigating to it doesn't kill the running wasm instance's emulation state.

## [0.7.0] "Resolution" - 2026-07-09

Implements true 512-px hi-res (Modes 5/6) output, the one bounded item left on `v0.5.0`'s
carried-forward PPU residual list, and rewrites the `v0.7.0`→`v1.0.0` release ladder to
front-load breadth into the `v1.0.0` gate rather than deferring it post-1.0, matching what
RustyNES actually shipped in its own v1.0.0. Also fixes a live `/api/` 404 on the Pages
deployment and a real shell-injection-style bug in `release-auto.yml` found on its first live
run. See `to-dos/VERSION-PLAN.md` for the full ladder this release opens.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`) is green; no currently-passing golden ROM enters hi-res mode, so the
non-hires compositor path is untouched and byte-identical to before.

This release's substantive work landed across PRs #44 and #45, each independently reviewed by
Gemini + Copilot (including two AI-reviewer suggestions investigated and rejected with
primary-source citations against ares' `dac.cpp` — see PR #45's review threads), human-reviewed,
and adjudicated before merge; this release-closeout PR (#46) is the final bookkeeping step,
matching the same convention every prior release-closeout PR (`v0.5.0`'s #35, `v0.6.0`'s #40) has
used.

### Added

- **True 512-px hi-res (Modes 5/6) output.** `rustysnes-ppu`'s DAC now
  emits two output columns per PPU pixel clock in hi-res, mirroring ares' `PPU::DAC::run()`/
  `above()`/`below()` (`ref-proj/ares/ares/sfc/ppu/dac.cpp`, read as primary source, not
  paraphrased from an earlier research summary that had undersold the real complexity here): the
  "odd" column is exactly today's unchanged main-screen color-math result; the "even" column is
  the subscreen's own color, math'd with the operand roles swapped, gated by state from the
  **previous** pixel clock's above-pass — a genuine one-pixel-clock-delayed hardware pipeline
  stage (verified precisely enough to know column 0 of every scanline is transparent by
  construction, matching the documented hardware fact, not a coincidence). The non-hires path is
  byte-for-byte the pre-existing code, unchanged; a frame's output width is latched once at its
  first visible scanline rather than re-checked per line, a deliberate, documented
  simplification. Bumped the save-state `FORMAT_VERSION` `1`→`2` (the framebuffer's backing
  storage growing to hi-res capacity is a real byte-layout change) — its first real bump, closing
  the `v1.0.0` gate's previously-flagged backward-compat-fixture gap early: a committed real
  `FORMAT_VERSION=1` blob (`tests/golden/savestate-v1-gilyon.bin`) plus a regression test proving
  the version mismatch fails loudly (a real `SaveStateError`, not silent corruption), not a
  synthetic one. Also corrected an overclaim in `docs/adr/0006-save-state-format.md`'s
  versioning-policy paragraph (that minor format bumps stay backward-loadable — not actually
  implemented; `load_state()` only ever rejects strictly-newer versions). Wired
  `crates/rustysnes-frontend/src/emu.rs` to query the PPU's actual active width instead of a
  hardcoded 256; the wgpu texture/present pipeline needed no changes (already allocated at hi-res
  capacity with a live UV sub-rect scale). Two new unit tests hand-construct synthetic scanlines
  to isolate the one-column-delay mechanism precisely, independent of full BG/tilemap setup. The
  full `--features test-roms` suite passes unchanged (no currently-passing golden ROM enters
  hi-res mode). **Real-title validation not achieved, honestly tracked as open, not claimed:**
  neither locally-available named hi-res-motivating title confirms the mechanism against actual
  game content — Marvelous — Mouhitotsu no Takarajima (SA-1) never entered hi-res in a
  1200-frame headless run, and Bishoujo Janshi Suchie-Pai has no local dump; an `ares`
  reference-screenshot comparison was attempted and abandoned (no working GUI display in this
  environment). `tests/golden/sa1-framebuffer.tsv` is not re-blessed — Marvelous's hash is
  unaffected by this change.

### Changed

- **The `v0.7.0`→`v1.0.0` release ladder is rewritten to front-load breadth into the 1.0 gate,
  matching what RustyNES actually shipped in its own v1.0.0.** The `v0.1.0`-`v0.6.0` ladder
  treated `v1.0.0` as an accuracy + stability gate with Phase 8 (netplay, RetroAchievements, TAS,
  scripting, a debugger, cheats) deferred to named post-1.0 minors — a deliberate correction away
  from an even earlier draft that had folded that breadth into 1.0. This reverses course a second
  time: RustyNES front-loaded nearly all of that breadth into its own v1.0.0 rather than
  deferring it, so matching that bar means it lands before RustySNES's production cut too, not
  after. New ladder: `v0.7.0 "Resolution"` (true 512-px hi-res Modes 5/6 output, the one bounded
  item left on the accuracy-debt list), an ongoing opportunistic `v0.x.y`-patch cluster for the
  rest of that list (mid-scanline/GSU, open-bus-via-HDMA-latch, SPC7110, DRAM refresh, ROM-dump-
  gated validation — none of it gates a numbered rung), `v0.8.0 "Instrumentation"` (debugger
  overlay, Lua scripting + TAS movie API, cheat-code support), `v0.8.0 "Community"` (rollback
  netplay, RetroAchievements), then `v1.0.0` (desktop UX shell maturity, a new frame-time
  performance-regression CI gate, the `README.md` rewrite, the production cut).
  `to-dos/VERSION-PLAN.md`, `to-dos/ROADMAP.md`, and `to-dos/phase-8-reach/overview.md` (plus its
  sprint files, renumbered: Sprint 1 = Instrumentation/`v0.8.0`, Sprint 2 = Community/`v0.8.0`,
  replacing the old netplay+RA-only Sprint 1) are rewritten together so all three planning
  documents agree.

### Fixed

- **`pages.yml`: fixed the live `/api/` rustdoc landing page 404 (found live, not in CI).**
  `v0.6.0`'s CHANGELOG entry claimed "the co-deployed rustdoc site (`/api/`) is reachable too" —
  true for any specific crate's page (`/api/rustysnes_core/index.html` returns `200`), but `/api/`
  itself 404'd: `cargo doc --workspace` writes one directory per crate and no top-level
  `index.html`, so the bare `/api/` path had nothing to serve. CI never caught this because
  `pages.yml` only asserts the build/deploy steps succeed, not that the resulting site's URLs
  actually resolve. Fixed by generating a redirect `_site/api/index.html` pointing at
  `rustysnes_core/index.html`, mirroring RustyNES's own already-working pattern
  (`../RustyNES/.github/workflows/web.yml`).

- **`release-auto.yml`: fixed a real shell-injection-style bug found on its first live run.**
  Step outputs containing a literal `"` (e.g. a CHANGELOG header like
  `## [0.6.0] "Shippable" - 2026-07-08`) were interpolated directly into `run:` script text —
  GitHub Actions substitutes `${{ steps.X.outputs.Y }}` as raw text *before* the shell parses the
  script, so an embedded quote silently closed the surrounding `header="..."` string early and
  corrupted the value instead of erroring loudly. This is exactly what happened on `v0.6.0`'s own
  first automated release: the title landed as `v0.6.0` instead of `v0.6.0 "Shippable"` (the tag
  annotation itself, sourced from a file rather than a raw interpolation, was unaffected and
  correct). Fixed by routing every step-output value used inside a `run:` block through `env:`
  instead of direct interpolation, which passes real argv/environment data immune to this whole
  class of bug — including a second, not-yet-exercised instance in the `gh release create --title`
  call that would have hit the identical corruption the next time a title actually contained a
  quoted theme name. Corrected the already-published `v0.6.0` release title retroactively via
  `gh release edit`.

### Investigated (not landed)

- **Mid-scanline/HDMA-driven register timing: a fix is designed, prototyped, and verified
  correct for the CPU/HDMA-driven case — but NOT landed, blocked by a second regression the same
  change causes in Super FX/GSU rendering.** The confirmed off-by-one-line compositor bug
  (`docs/ppu.md` §Mid-scanline/HDMA-driven register timing) has a prototype fix: `rustysnes-ppu`
  would composite each line at a new `RENDER_DOT` constant (`= 276` — the same dot number HDMA's
  own per-line run fires at, but sequenced strictly before it within that master-clock tick's
  execution order) instead of at dot 340 (`end_of_scanline`), matching real hardware's per-pixel
  timing. Prototyping and running the full `--features test-roms` suite confirmed this mechanism
  is correct for CPU/HDMA-driven register changes: all 29 `undisbeliever` goldens held, and the
  one golden it legitimately changes (SA-1's `SD F-1 Grand Prix`) was independently confirmed a
  real accuracy improvement, not blindly accepted — diffing pre-/post-prototype framebuffers
  row-by-row found 159/239 rows differed, and testing those against the fix's predicted "shifted
  one line later" signature matched 232/237 checkable rows (97.9%; 237 = 239 minus the 2 boundary
  rows a one-line-shift comparison can't reach) with zero unexplained outliers. **But the same
  prototype broke all 24 Super FX/GSU golden tests** with a diff pattern that does *not* fit that
  mechanism (a color bar shifted 4 rows in the opposite direction on one ROM; 7 genuine outliers
  on another) — the identical failure signature an earlier, unrelated investigation this cycle
  (open-bus-via-HDMA-latch) also hit and correctly did not land. Working hypothesis (not
  confirmed): the GSU coprocessor's host-synced VRAM writes are sampled at a different point in
  their own progress once the render trigger moves earlier in the master-clock tick — needs an
  access-level trace to confirm. Reverted; full mechanism, both verifications, and what a future
  investigation needs are documented in `docs/ppu.md` for whoever picks this up next.

## [0.6.0] "Shippable" - 2026-07-08

Closes out the release-engineering and doc-parity half of "match RustyNES's level" that isn't
about emulation accuracy — the part `v0.5.0 "Fidelity"` deliberately left for this rung, per
`to-dos/VERSION-PLAN.md`'s own ladder. Every checklist item lands: `release.yml` exercised
end-to-end with checksummed assets (first proven live by `v0.5.0`'s own build), `security.yml`
(`cargo audit` + `cargo deny check`), the `lint` job now also gates `cargo doc`,
`docs/DOCUMENTATION_INDEX.md`, `docs/benchmarks.md` + a real Criterion benchmark, `docs/audit/`,
3 ADR backfills (9 total, up from 6), and — the item this rung adds on top of what `v0.5.0`
already pulled forward — automated release-cutting (`release-auto.yml`), directly addressing the
recurring manual-release-ceremony bottleneck the `v0.5.0` cut itself ran into. Also verified the
wasm/Pages demo deploy is genuinely live (not just CI-green): the trunk-built `index.html`,
wasm-bindgen JS loader, `.wasm` binary, and co-deployed rustdoc all return HTTP 200 with correct
content-types at `https://doublegate.github.io/RustySNES/`.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`), the `no_std` gate, and `RUSTDOCFLAGS="-D warnings" cargo doc` are all
green.

This release landed across PRs #36-39, each independently reviewed by Gemini + Copilot,
human-reviewed, and adjudicated before merge.

## [0.5.0] "Fidelity" - 2026-07-08

Closes out the accuracy-pass-rate dashboard RustySNES previously lacked (`docs/STATUS.md`'s new
"Accuracy dashboard" section, RustyNES's AccuracyCoin-equivalent) and works the full named
hardware-gotcha regression list this release's goal called for: every item is now either fixed
(a real, previously-undocumented doc/code drift in HDMA dot-phase timing), correctly reclassified
as an intentional non-goal with primary-source justification (`$4203`/`$4206`, the
"DMA/HDMA-collision crash quirk"), or honestly researched-and-deferred with a full mechanism
write-up and regression evidence for whoever picks it up next (open-bus-via-HDMA-latch, DRAM
refresh, mid-scanline/HDMA-driven register timing, hi-res color-math precision). Two of those
deferrals surfaced genuine findings worth flagging for `v0.6.0`+: a real, previously-unknown
off-by-one-line compositor bug (documented in `docs/ppu.md`, not yet fixed — touches the hottest
code path in the engine with no dedicated test ROM yet), and a confirmed real regression (a
prototype open-bus fix broke all 24 Super FX golden hashes) that correctly stopped an
unverified change from landing. Also pulls forward several `v0.6.0 "Shippable"` release-engineering
items opportunistically (a `security.yml` CI gate, checksummed release assets, a real Criterion
benchmark, `docs/DOCUMENTATION_INDEX.md`, `docs/audit/`, 3 ADR backfills) since they were
low-risk, self-contained wins ready ahead of schedule. See `to-dos/VERSION-PLAN.md`.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (339 tests
across 39 suites, including `--features test-roms`), the `no_std` gate, and
`RUSTDOCFLAGS="-D warnings" cargo doc` are all green. The new `security.yml` gate (`cargo audit`

- `cargo deny check`) is also green, its first real end-to-end run.

This release landed across PRs #33-34, each independently reviewed by Gemini + Copilot,
human-reviewed, and adjudicated before merge.

### Added

- **Mid-scanline/HDMA-driven register timing + hi-res color-math precision: researched — `v0.5.0`
  "Fidelity" work.** Confirmed a genuine, previously-undocumented off-by-one-line compositor bug
  against ares' per-pixel reference model (`ppu/main.cpp`'s active-pixel rendering runs strictly
  *before* HDMA's per-line service point): RustySNES's end-of-line compositor applies a line `V`
  HDMA-driven register write to line `V` itself, when real hardware only ever observes it starting
  line `V+1` (the mechanism behind Air Strike Patrol's BG3 raster scroll and any similar
  HDMA-driven per-line effect). Corrected two related overclaims this same investigation surfaced
  (`docs/ppu.md`'s "single-split-per-line effects work" claim, and both that doc's and
  `crates/rustysnes-ppu/src/lib.rs`'s claim that the per-scanline compositor is unconditionally
  "bit-identical" to a per-dot renderer). Not fixed this pass — the change touches the hottest
  code path in the engine (every frame, all 29 currently-passing goldens) with no dedicated test
  ROM yet to verify a fix against; full mechanism and what a fix needs documented in
  `docs/ppu.md` §Mid-scanline/HDMA-driven register timing. Separately confirmed hi-res
  color-math precision (Bishoujo Janshi Suchie-Pai / Marvelous+SA-1) is blocked entirely on
  512-wide hi-res output not existing yet (ares' `DAC::run()` shows hi-res is a dual-half-pixel
  alternating-compositor-result trick, not a numeric-precision tweak) — a real feature gap, not
  this pass's scope; full mechanism in `docs/ppu.md` §Hi-res color-math precision. No `.rs` code
  changed beyond doc comments; full workspace + `--features test-roms` suites verified unaffected.

- **The "DMA/HDMA-collision crash quirk": researched and reclassified — `v0.5.0` "Fidelity"
  work.** The SNESdev errata page's DMA section bundles three distinct behaviors under this
  vague label: a version-1-5A22-only crash and a version-2-5A22-only silent-DMA-failure bug
  (both chip-revision defects compliant commercial ROMs are written to avoid, not reproduced as
  a crash by any mainstream reference emulator), plus a version-agnostic silent whole-frame HDMA
  failure that's well-defined but has no known commercial title or committed test ROM depending
  on it — no oracle exists to verify an implementation against, and the sibling open-bus
  investigation (below) already demonstrated this exact class of change carries real regression
  risk even when the documented mechanism is correct. A fourth item on the same errata list
  (A-bus address restrictions) turned out to already be correctly implemented, as is the general
  "HDMA preempts GP-DMA" priority ordering — the well-defined half of what "collision" could have
  meant was never actually a gap. Full citation and per-sub-case reasoning in `docs/scheduler.md`.

- **`security.yml` CI gate — `v0.6.0` "Shippable" work, pulled forward.** A new dedicated
  workflow runs `cargo audit` and `cargo deny check` on every `main`/PR push touching non-doc
  paths, plus a weekly schedule so a newly-published advisory against an unchanged dependency is
  still caught. Added `deny.toml`, built from RustySNES's own `cargo deny list` output (not
  copied from RustyNES's config) — independently confirms the same winit/egui/wgpu dependency
  chain trips the identical 3 RUSTSEC advisories RustyNES already documented (`ttf-parser`
  unmaintained via winit's Wayland decoration stack; `quick-xml`'s two advisories, reachable only
  through `wayland-scanner`'s compile-time XML parsing of trusted vendored protocol files, never
  runtime input). Suppressed in `deny.toml` + the new `.cargo/audit.toml` with the full
  rationale, after explicit review and approval.

- **Checksummed release assets (SHA-256) — `v0.6.0` "Shippable" work, pulled forward.**
  `.github/workflows/release.yml` gained a `Checksum` step that emits a detached `<archive>.sha256`
  alongside each platform's packaged binary archive, portable across the three runner shells
  (tries `sha256sum`, falls back to `shasum -a 256`, since GNU coreutils' `sha256sum` is absent on
  macOS runners and Perl's `shasum` isn't guaranteed on Windows' Git-Bash `PATH`); the upload step
  now attaches both files. Not yet exercised end-to-end against a real tag.

- **`docs/benchmarks.md` + a real Criterion benchmark — `v0.6.0` "Shippable" work, pulled
  forward.** The first-ever measured performance number on this codebase:
  `crates/rustysnes-core/benches/headless_frame.rs` (Criterion 0.7) measures headless full-frame
  throughput against a real committed test ROM (`tests/roms/undisbeliever/inidisp_hammer_0f00.sfc`,
  chosen for no coprocessor/DMA-heavy content so the measurement isolates the base
  CPU+PPU+scheduler cost). Result: **3.27 ms/frame** steady state, against `docs/performance.md`'s
  ≤~2ms target — real-time headroom is fine (~5.1× at NTSC's 16.64ms/frame budget), but the
  target itself isn't met yet. Documented honestly as a baseline to measure future optimization
  against, not a claim of having hit the target.

- **`docs/DOCUMENTATION_INDEX.md` — `v0.6.0` "Shippable" work, pulled forward.** The full
  documentation map (subsystem specs, ADRs, testing strategy, `ref-docs`/`ref-proj`/`to-dos`
  cross-references, external hardware-reference links), matching RustyNES's own index and linked
  from the README.

- **`$4203`/`$4206` multiply/divide overlap: researched and correctly reclassified — `v0.5.0`
  "Fidelity" work.** The 65816 hardware-gotcha list named this as an open item;
  research against SNESdev's own Errata page shows starting a new multiply/divide while a
  previous one's 8-cycle latency hasn't elapsed produces genuinely **undefined** `RDMPY`/`RDDIV`
  output — no canonical corrupted value is documented anywhere to port, and fabricating one would
  violate the determinism contract's spirit (`docs/adr/0004`). `MulDiv`'s doc comment now cites
  the errata directly and explains why this stays a documented non-goal rather than an open gap.
  Added a regression test locking in the well-defined case real hardware *does* document (MPYA
  is a stable latch; a fresh `$4203` write alone starts another multiply against whatever it
  already holds, no `$4202` rewrite needed).

- **ADR backfill: 3 new ADRs, `v0.5.0` "Fidelity" / `v0.6.0` "Shippable" work.**
  `docs/adr/0007` (the versioning/release-process adoption itself — the named `v0.x.0` ladder,
  the tag-body-is-the-release-note convention), `docs/adr/0008` (why the ExLoROM decode formula
  is sourced from bsnes's runtime board database rather than extrapolated from LoROM or the
  header-detection heuristic), and `docs/adr/0009` (ST018's title-match detection method, kept
  consistent with the rest of the `$F`-nibble coprocessor family rather than reading the
  `$xFBF` byte other customs are known-unreliable against; and the `Board::coprocessor_tick`
  catch-up architecture chosen over the SA-1 second-CPU hooks, since ST018's ARM core is
  self-contained in `rustysnes-cart` unlike SA-1's second 65C816). Also adds implementation
  guidance for the still-unstarted DRAM-refresh hardware-gotcha fix to `docs/scheduler.md`,
  surfacing a real architectural tension this project's CPU-driven master clock has with real
  hardware's independent video-timing generator that needs resolving empirically (against the
  full golden-framebuffer suite) before that fix lands, not assumed safe up front.

- **`docs/audit/` — `v0.6.0` "Shippable" work, pulled forward.** A new decision-rationale /
  open-investigation directory (modeled on RustyNES's own `docs/audit/`), seeded with the full
  SPC7110 boot-crash trail: the `v0.4.0`-landed `bus_mirror` addressing fix (confirmed root
  cause #1) and the still-open gap (root cause #2, narrowed to two candidate hypotheses, not
  yet fixed) that keeps Far East of Eden Zero from booting to real content. Also fixed two
  remaining "Sharp RTC-4513" naming errors (`docs/cart.md`, `coproc::sharprtc`'s module/struct
  docs) — the standalone Sharp S-RTC has no established "4513" part number anywhere; that number
  belongs only to the different Epson chip paired with SPC7110.

- **`docs/STATUS.md`: an accuracy dashboard — `v0.5.0` "Fidelity" work.** RustySNES
  has no single monolithic oracle ROM the way RustyNES's AccuracyCoin does (an early skeleton for
  exactly that approach, `rustysnes-test-harness::accuracy_battery`, ticket T-04, was never
  implemented and is superseded, not a competing source of truth), so rather than force the
  composed multi-layer battery into one misleading summed fraction (a 5.12M-case CPU oracle
  would swamp a 4-ROM audio suite), a new "Accuracy dashboard" table tracks each layer's own
  status — the CPU per-opcode oracle (0-diff against its chosen reference; one documented
  inter-reference divergence, not a bug, `docs/adr/0002`), the SPC700 per-opcode oracle (0-diff,
  100.00%), on-cart CPU, PPU/DMA golden framebuffer, audio boot+run, Core/Curated coprocessors
  (honesty-gate green, 3/3), and BestEffort coprocessors split into real-title-validated (6/9)
  vs unit-test-only (3/9) — always
  current, reaffirmed every release from here on, plus a named-residuals line so known gaps stay
  visible instead of buried in prose.

- **Nintendo Aging/Controller/SNES Test Program ROMs: researched, reclassified as a stretch
  goal — `v0.5.0` "Fidelity" work.** These Nintendo factory-diagnostic cartridges are real and
  individually preserved (Internet Archive, SNES Central, The Cutting Room Floor) but carry the
  same copyright status as the commercial ROMs this project already gates behind
  `--features commercial-roms`. Checked whether RustyNES pursued an NES equivalent as precedent:
  it did not — its AccuracyCoin is one third-party homebrew ROM, not a Nintendo-authored factory
  diagnostic, so this checklist item's original premise didn't hold. Deferred to a later release
  as a stretch goal rather than pursued this release.

## [0.4.0] "Completion" - 2026-07-08

Closes out Phase 7's BestEffort coprocessor/board matrix: a full ARMv3 (ARM6-class) CPU core for
ST018 (Hayazashi Nidan Morita Shogi 2), a standalone Sharp S-RTC board (Daikaijuu Monogatari
II), and a confirmed, fixed SPC7110 addressing bug (materially improved boot progress, one
narrowed-but-still-open gap honestly documented, not silently claimed fixed). See
`to-dos/VERSION-PLAN.md`.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`), the `no_std` gate, and `RUSTDOCFLAGS="-D warnings" cargo doc` are all
green.

This release landed across PRs #24-30, each independently reviewed by Gemini + Copilot and
adjudicated before merge.

### Added

- **ST018: the SNES-side board wrapper — `v0.4.0 "Completion"` is done**
  (`coproc::armv3::board::St018Board`). Steps 9+10 of the ARMv3 core build order
  (`docs/st018-arm-notes.md`), the final piece: `Coprocessor::St018` (new header enum variant),
  detected via a title match on the confirmed real cart, *Hayazashi Nidan Morita Shogi 2*
  (`NIDAN MORITASHOGI2`) — an earlier investigation wrongly assumed this chip was Star Ocean's,
  which uses S-DD1 only, no ARM coprocessor. Driven by `Board::coprocessor_tick` (the existing
  GSU/Super FX host-sync hook, fired once per master-clock unit) rather than the SA-1 second-CPU
  hooks: unlike SA-1's second 65C816, this ARM core is entirely self-contained within
  `rustysnes-cart` already, so `St018Board` owns and steps it directly with no `rustysnes-core`
  changes needed. A single combined `0x28000`-byte firmware dump splits into 128 KiB PRG ROM +
  32 KiB data ROM; the `$3800`/`$3802`/`$3804` handshake registers over the whole `$3000-$3FFF`
  window; 16 KiB work RAM; full save-state coverage (register file, pipeline, handshake state —
  never the firmware bytes themselves). Ported a genuine fidelity nuance found while wiring this
  up: the reference's `PowerOn(forReset)` only preserves the ARM's cycle counter across a TRUE
  reset (the SNES-side `$3804` `1->0` edge), not at construction/firmware-load — a bug a test
  caught by assuming cycle-preservation applied everywhere. 9 new tests. Closes out `v0.4.0`'s
  full coprocessor/board matrix (`docs/STATUS.md`).

- **ST018: multiply, multiply-long, and single data swap — the ARMv3 instruction set is complete**
  (`coproc::armv3::cpu`). Step 8 of the ARMv3 core build
  order (`docs/st018-arm-notes.md`). `MUL`/`MLA`/`UMULL`/`UMLAL`/`SMULL`/`SMLAL`: a deliberate
  fidelity tradeoff over the reference's cycle-exact `GbaCpuMultiply` circuit simulation (Booth's
  algorithm with an empirically-reverse-engineered correction table, built for GBA test-ROM
  precision) — this port instead computes the mathematically correct widened result directly and
  idles for the ARM ARM's own *documented* early-termination cycle count (1/2/3/4 cycles by how
  many of Rs's top bytes are all-0/all-1), leaving the multiply C flag deliberately unchanged
  (real hardware's value there is implementation-defined/meaningless and isn't simulated). `SWP`/
  `SWPB`: atomic read-then-idle-then-write at one address, with `rm==15` writing `R15+4`. 7 new
  tests. This closes out the full ARMv3 instruction set — `Cpu::step` no longer panics on any
  opcode category.

- **ST018: LDR/STR and LDM/STM (single/block data transfer)**
  (`coproc::armv3::cpu`). Steps 6+7 of the ARMv3 core
  build order (`docs/st018-arm-notes.md`). `LDR`/`STR`: immediate and shifted-register offsets,
  pre/post-indexed addressing (post-indexed always writes back, even without the explicit W bit;
  a load into the same register as the base never writes back), and the real ARM6-class quirk
  where storing R15 stores address+12 instead of the usual address+8. `LDM`/`STM`: the empty-
  register-list glitch (only R15 transfers, but the address advances as if all 16 did), the
  load/store write-back timing asymmetry, and the S-bit's dual role (temporary User-bank access
  during the transfer, or — when loading with R15 in the list — a full CPSR-from-SPSR restore
  after the transfer, the `LDM ... {..., pc}^` exception-return idiom). 7 new tests.

- **ST018: data processing, branch, MSR/MRS, and exception entry**
  (`coproc::armv3::cpu`). Steps 4+5 of the ARMv3 core
  build order (`docs/st018-arm-notes.md`). All 16 data-processing ALU ops (both immediate and
  shifted-register operand forms, including the register-specified-shift `+4`-on-top-of-`+8` R15
  exposure quirk) and the implicit `MOVS PC, ...`-restores-CPSR-from-SPSR exception-return
  behavior; `B`/`BL` (`LR = R15-4`, not `R15`, since R15 is already pipeline-advanced); masked
  `MSR` writes and `MRS` reads; and exception entry for `SWI`/undefined-instruction traps. The
  opcode-category decoder mirrors the reference `InitArmOpTable`'s exact construction-order
  priority (sparse Multiply/MultiplyLong/SingleDataSwap/SoftwareInterrupt carve-outs win over the
  broader ranges they overlap) without needing a real 4096-entry lookup table. 11 new tests,
  including a full `SWI`-then-`MOVS PC,LR` round trip proving CPSR survives a real mode change
  (User → Supervisor → User).

- **ST018: the ARM register file, mode-switch banking, and the 3-stage pipeline**
  (`coproc::armv3::regs`). Steps 2+3 of the ARMv3 core
  build order (`docs/st018-arm-notes.md`). Register banking ports real ARM hardware exactly:
  `R8-R12` shared across every mode except FIQ (which gets a fully private bank), `R13`/`R14`
  banked separately per mode including a distinct User-mode bank, and per-mode SPSR routing —
  proven by round-trip tests for each banking rule. The pipeline model is the entire mechanism
  behind ARM's well-known "PC reads as address+8" quirk (no `+8` constant exists anywhere in this
  port; it falls out of the 3-stage Fetch/Decode/Execute timing itself) — proven by dedicated
  tests asserting the exact R15 value observed at power-on, steady-state stepping, and across a
  taken branch, since every later instruction's correctness depends on this being right first
  (`crates/rustysnes-cart/src/coproc/armv3.rs` split into a directory module: `primitives.rs` +
  `regs.rs`, 14 new tests). Instruction decode/execute and the board wrapper remain.

- **ST018 foundation: the ARMv3 barrel shifter, condition codes, and ALU core**
  (`coproc::armv3`). The first increment of a full
  ARMv3 (ARM6-class) CPU core for ST018 (Hayazashi Nidan Morita Shogi 2's LLE coprocessor,
  not Star Ocean's -- Star Ocean uses S-DD1 only) — clean-room port of Mesen2's
  `ArmV3Cpu` (chosen over ares' generic ARM7TDMI-based `armdsp`, a Thumb-capable superset the
  real pre-Thumb ST018 chip never needed). Ports only the pure, state-free primitives every ARM
  instruction depends on: `LSL`/`LSR`/`ASR`/`ROR`/`RRX` (every documented `shift ≥ 32` boundary
  case), the 4-bit condition-code checker, and the `ADD`/`SUB`/logical-op flag formulas — each
  verified against the ARM Architecture Reference Manual's own truth tables (12 new tests).
  Deliberately NOT wired to any board yet: instruction decode, the register file + mode banking,
  and the 3-stage pipeline (whose exact timing implicitly produces ARM's "PC reads as address+8"
  quirk) remain, sequenced in that order per `docs/st018-arm-notes.md` — a from-scratch ARM core
  is comparable in scope to the 65C816 core, not a small register-file port.

- **Standalone S-RTC board** (`coproc::sharprtc::SharpRtcBoard`).
  A standalone Sharp S-RTC real-time clock (Daikaijuu Monogatari II, ExHiROM) — a different
  chip/protocol from the Epson RTC-4513 already paired with SPC7110: a 2-register (`$2800`/
  `$2801`) handshake over a 13-slot decimal clock file (second/minute/hour/day/month/year + an
  auto-computed weekday) through a `Ready -> Command -> Read`/`Write` state machine. Seeded to a
  fixed epoch, never wall-clock-advanced (`docs/adr/0004`'s determinism contract, matching
  `EpsonRtc`'s existing posture). No commercial dump exists in the local corpus — unit-test-level
  coverage only, header detection is a best-effort title match, matching the existing CX4/SPC7110
  disambiguation pattern (`docs/adr/0003`).

### Fixed

- **SPC7110 boot-crash root cause: a real DROM/PROM address-mirroring bug found and fixed.**
  `datarom_read`/`mcurom_read` folded out-of-range data-ROM offsets with a plain `offset % len`;
  real hardware (ares `Bus::mirror`) instead repeatedly strips the largest power-of-two block
  that keeps the address in range, which only coincides with modulo when the buffer size is
  itself a power of two — Far East of Eden Zero's 6 MiB DROM is not. A register-selected read
  past the physical chip size but inside the addressable window silently returned the wrong
  byte. Ported the real algorithm (`spc7110::bus_mirror`) into every PROM/DROM lookup. This
  pushed the previously-observed wild-PC excursion from ~20-30 frames into boot to ~90+ frames,
  and it now self-recovers via a BRK/RTI loop rather than crashing outright — real, measurable
  progress, though the CPU still does not reach a bootable screen: it eventually `RTI`s (from
  genuine PROM code) into a WRAM location that's confirmed entirely unpopulated, a separate,
  still-open issue documented in `docs/cart.md` §SPC7110 and `docs/STATUS.md`'s coprocessor
  matrix rather than silently claimed fixed.

- **`release.yml` built platform binaries but never attached them to the GitHub release.**
  `v0.1.0`, `v0.2.0`, and `v0.3.0` all shipped with zero release assets — the workflow ran
  `cargo build --release` per platform and stopped there. Added a packaging step (tar.gz on
  Linux/macOS, zip on Windows, each bundling the binary + `README.md`/`LICENSE-MIT`/
  `LICENSE-APACHE`) and an upload step (`gh release upload`, self-healing via `gh release
  create` if the release doesn't exist yet) so every future tag automatically attaches its
  build artifacts. Backfilled `tar.gz`/`zip` archives onto the existing `v0.1.0`/`v0.2.0`/
  `v0.3.0` releases by hand to close the retroactive gap.

## [0.3.0] "Continuum" - 2026-07-08

Rewind, run-ahead, PAL region auto-detection, and the ExLoROM memory-map model — the frontend
orchestration layer built on `v0.2.0`'s save-state primitive, plus the remaining `Phase 7`
memory-map coverage. See `to-dos/VERSION-PLAN.md`.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`), the `no_std` gate, the wasm32 build check, and `RUSTDOCFLAGS="-D
warnings" cargo doc` are all green.

### Added

- **PAL region auto-detection.** `Bus::sync_region_from_cart` reads the cart header's
  destination-code byte and reconfigures the PPU's line-count/status-bit timeline at
  `System::reset()` — the 50 Hz/312-line table already existed in the scheduler, but nothing
  previously wired the header's own region detection into the running machine, so PAL carts
  silently ran at NTSC timing. Proven end-to-end by a synthetic-header test that runs a full
  frame and asserts it completes at the correct 312-line count, not just that the region flag
  flips. Only the PPU's line-count/status-bit timeline is region-dependent in the core; the
  differing NTSC/PAL master-clock rate is a frontend/real-world-pacing concern (`docs/adr/0004`).
  **Still open:** no real PAL ROM exists in the local corpus, so this has no golden-ROM-boot
  validation yet.
- **ExLoROM memory-map model.** `MapMode::ExLoRom` + the `ExLoRom` board: header detection at
  `$40_7FC0` and an A23-inverted, LoROM-windowed ROM decode (`high | ((bank & 0x7F) << 15) |
  (addr & 0x7FFF)`) for >4 MiB titles that keep LoROM's 32 KiB bank windowing instead of
  switching to HiROM's linear banks. ExLoROM has no dedicated `$FFD5` mode value — ares/bsnes
  both document it as unofficial — so the decode formula is sourced directly from bsnes's own
  *runtime* board database (`board: EXLOROM`/`EXLOROM-RAM`,
  `target-bsnes/resource/system/boards.bml`), decoded against bsnes's `Bus::reduce` bit-packing
  algorithm, rather than guessed from the header-detection heuristic alone. See `docs/cart.md`
  §ExLoROM for the full provenance chain. **Still open:** no real ExLoROM ROM (commercial or
  homebrew) exists in the local corpus, so this board has only formula-level unit-test coverage,
  not golden-framebuffer validation.
- **Rewind.** `rustysnes-frontend::rewind::RewindBuffer` — a bounded ring buffer of FULL
  `EmuCore::save_state` snapshots, recorded every `config.rewind.interval_frames` real frames
  (default 6, ~10 Hz) up to `config.rewind.capacity` entries, oldest evicted first. Simpler than
  `docs/frontend.md`'s original "keyframes + deltas" sketch — delta-compression is a possible
  future memory optimization, not a correctness requirement. Wired into the synchronous
  frame-drive loop (`app.rs`) + a new Emulation → Rewind menu item; **`capacity: 0` is the
  shipped default**, making recording a permanent no-op (e.g. `capacity: 300` at the default
  6-frame interval would give ≈30s of NTSC rewind, but that's an example config, not what
  ships). Snapshots are discarded on ROM load/close (a new cart invalidates any prior snapshot),
  NOT on Reset/Power-Cycle
  (rewinding past an accidental reset is a legitimate use case).
- **Run-ahead.** `rustysnes-frontend::rewind::step_with_run_ahead` — peeks `config.run_ahead.frames`
  frames ahead each displayed frame using the currently-latched input, presents that peek's
  video, then rolls back and re-runs exactly ONE real frame — so persisted state (and audio, the
  continuous stream; peek audio is never played) only ever advances by one frame per call,
  regardless of peek depth. Wired into the frame-drive loop; `frames: 0` (the shipped default)
  degrades to a plain `run_frame`. Both rewind and run-ahead are pure re-simulation of the SAME
  deterministic core (`docs/adr/0004`) — no injected timing/RNG — and are proven by tests that
  hand-assemble a tiny 65C816 program (an NMI handler incrementing a WRAM counter into the CGRAM
  backdrop color) to get a real, observable per-frame state signal rather than a synthetic
  fingerprint; a naive in-loop instruction counter turned out to be exactly periodic at a fixed
  video-frame boundary, which is what motivated tying the counter to the NMI/vblank edge instead.
- **Quick-save/load.** The previously-stubbed Emulation → Save State / Load State menu items now
  actually call `EmuCore::save_state`/`load_state` against a single in-memory slot
  (`Active::quick_save`), completing the `docs/frontend.md` "not yet implemented" TODO left over
  from before `v0.2.0`'s save-state format landed.
- **`EmuCore::save_state`/`load_state`.** Thin wrappers around `System::save_state`/`load_state`
  (`docs/adr/0006`) that additionally re-render the framebuffer and clear the audio FIFO on load
  (a state load jumps time discontinuously) — the shared primitive rewind, run-ahead, and
  quick-save all build on.

### Fixed

- **`release.yml`'s Linux build was broken.** The tag-triggered release workflow never installed
  the Linux system dependencies (`libxkbcommon-dev`/`libwayland-dev`/`libasound2-dev`/
  `libudev-dev`/`libx11-dev`/`libxcursor-dev`/`libxrandr-dev`/`libxi-dev`) that `ci.yml`/
  `pages.yml` already do, so `cargo build --release -p rustysnes-frontend` failed immediately at
  `libudev-sys`'s `pkg-config` build step on every `ubuntu-latest` release build — caught when
  the `v0.2.0` tag push actually exercised this workflow for the first time. Added the same
  install step `ci.yml` uses, gated to the Linux matrix leg.

### Changed

- **CI now runs the full verification battery only on release-tag pushes.** `ci.yml`'s single
  `test` job (3-OS matrix, both `cargo test` invocations, the doc-warnings gate) ran on every
  single push and PR — expensive CI minutes for what's usually mid-review iteration, not a
  release candidate. Split into `lint` (fmt --check + clippy -D warnings, Linux only, every
  push/PR) and `full-test` + `no_std` (the complete battery), the latter two gated to `v*` tag
  pushes only, matching `release.yml`'s existing tag-only trigger.
- **Further CI/CD cost reductions.** Concurrency groups (`cancel-in-progress: true`) on
  `ci.yml`, `pages.yml`, and `release.yml` — a new push to the same PR/branch/ref now cancels the
  already-stale in-flight run. `Swatinem/rust-cache` in every job that runs cargo, caching
  `~/.cargo/registry`, `~/.cargo/git`, and `target/`. Dropped the `wasm32-unknown-unknown`/
  `thumbv7em-none-eabihf` toolchain-target installs from `ci.yml`'s `lint`/`full-test` jobs
  (neither ever cross-compiles — pure unused setup cost). Trimmed `full-test`'s `cargo fmt
  --all --check` to a single matrix leg (formatting is platform-independent). Replaced
  `pages.yml`'s `cargo install trunk --locked` (compiled trunk + its whole dependency tree from
  source on every `main` push) with a prebuilt-binary download via `taiki-e/install-action`.

## [0.2.0] "Persistence" - 2026-07-02

A versioned, deterministic core-wide snapshot format — the prerequisite every downstream Reach
feature (rewind/run-ahead in `v0.3.0`, netplay rollback in `v1.2.0`, TAS replay in `v1.4.0`)
builds on. See `to-dos/VERSION-PLAN.md` and `to-dos/phase-5-frontend/sprint-2-save-states.md`.

**Oracle/golden suites: all held, no regressions.** The full workspace test suite (including
`--features test-roms`) is green; the round-trip determinism test additionally proves the new
save-state format bit-identical across a no-coprocessor ROM, a `Curated` Super FX ROM, and a
`BestEffort` commercial coprocessor ROM.

### Added

- **Save-state foundation (parts 1-9 of N — the complete `v0.2.0` scope).** New
  `rustysnes-savestate` leaf crate: `SaveWriter`
  (an allocation-free append-only builder with primitive writers + a `section(tag, body)` helper
  for nested, self-describing sections — writes directly into the parent buffer with a length
  placeholder patched in place, not a throwaway nested `Vec` per section) and `SaveReader` (a
  bounds-checked cursor with the mirror-image readers, returning `Result` instead of panicking on
  truncated/corrupt input) — the wire-format primitives `docs/adr/0006-save-state-format.md`
  specifies. `Board::save_state`/`load_state` hooks added to `rustysnes-cart`, default no-op
  (correct, not just convenient, for the base LoROM/HiROM/ExHiROM boards, which carry no extra
  coprocessor state). Implemented for `Obc1Board` (its 3-field cursor, with untrusted-input
  validation — an out-of-range value is rejected rather than risking a later panic, and a section
  with unconsumed trailing bytes is rejected too), `Dsp1Board`, and `NecDspVariantBoard` (which
  covers DSP-2/DSP-4/ST010 for free via the shared `Upd77c25` engine's own `save_state`/
  `load_state` — every register + the 2048-word data RAM, deliberately excluding firmware, which
  is never embedded in a save-state, and the host-access debugger counter; the pointer registers
  `pc`/`rp`/`dp`/`sp` are masked to their revision-correct widths on load rather than trusted
  verbatim, since they're used as unchecked array indices elsewhere in the engine). Extended to
  `Cx4Board` (its `Hg51b` core's full register file, IO block, both cached 256-word program
  pages, the 3 KiB data RAM, and the 8-deep call stack — the 3 KiB data-ROM constant table,
  `cx4.rom`, is firmware and stays excluded) and `Sdd1Board` (the MMC bank registers, the
  snooped-DMA shadow state, and the `Decompressor`'s full mid-stream entropy-decoder state — input
  cursor, all 8 Golomb bit generators, the probability-estimation module's 32-entry context
  table, the context model, and the output-logic register triple — so a save-state landing
  mid-DMA-transfer resumes the decompression correctly instead of desyncing the stream; a
  `ContextInfo::status` out of `EVOLUTION_TABLE`'s range is rejected as invalid rather than
  masked, since it's a semantic state-machine index, not a hardware register width, while
  `current_bitplane` IS masked, being a genuine 3-bit hardware quantity). Extended further to
  `SuperFxBoard` (its `Gsu` core's full register file, status/control fields, both bus-buffer
  latches, the opcode cache, the plot pixel cache, and the in-flight per-access checkpoint
  queue — the latter matters because master-clock-interleaved `Gsu::tick` execution, unlike the
  run-to-completion `run_until_stopped` path, can leave a `Go` burst genuinely mid-flight at any
  save point; a claimed checkpoint-queue length or cursor beyond what real execution could ever
  produce is rejected as invalid, not trusted) and `Sa1Board` (the full `$2200-$23FF` register
  file, the 2 KiB I-RAM, the H/V timer counters, and the character-conversion DMA staging flags —
  BW-RAM stays excluded, captured separately via the existing `Board::sram` path). This
  completes save-state coverage for every coprocessor board (T-52-002): `Spc7110Board` (every
  DCU/data-port/ALU/memory-control register, the `dcu_tile` scratch buffer — `dcu_offset` masked
  `& 31` since it indexes it directly — and its `Decompressor`'s mid-tile range-coder state,
  including its 5x15 context table; a prediction index outside the 53-entry `EVOLUTION` table is
  rejected, `bpp` is validated against the only values a real `1 << mode` can produce
  (`{0,1,2,4}`), and `bits` is bounded `0..=8` — all guard against the same
  corrupted-save-state-triggers-a-shift/index-panic class of bug already fixed on the other
  boards) and its paired `EpsonRtc` (every clock field + the 4-state handshake machine; an
  out-of-range state discriminant is rejected as invalid, the same enum-constraint posture
  `Obc1Board` already applies to its own cursor fields). `#![no_std]` holds throughout; 13 new
  round-trip/validation tests across `rustysnes-cart` (`obc1.rs` ×3, `dsp1.rs` ×1,
  `necdsp_variant.rs` ×1, `upd77c25.rs` ×1, `cx4.rs` ×1, `sdd1.rs` ×1, `superfx.rs` ×1,
  `sa1.rs` ×1, `epsonrtc.rs` ×2, `spc7110.rs` ×1). T-52-002's board-coverage acceptance
  criterion is now fully met. T-52-003 (the wider core snapshot) begins here: `Cpu::save_state`/
  `load_state` (the full 65C816 register file, the `WAI`/`STP` latches, and the cumulative cycle
  counter into a `"CPU0"` section) and `Ppu::save_state`/`load_state` (VRAM/CGRAM/OAM, the full
  register file — including the six-layer window unit — the write latches, the dot/scanline
  timeline, the interrupt/frame poll state, `region`, and the composited framebuffer into a
  `"PPU0"` section; an out-of-range `region` discriminant is rejected as invalid). Neither engine
  has an array index whose valid range is narrower than its storage type once the existing
  regs.rs/lib.rs masking at each *use* site is accounted for (`cgram_address` is a `u8` matching
  the 256-entry `cgram` exactly; `oam_address`/VRAM offsets are masked at every access site, not
  trusted verbatim there either), so neither `load_state` needed additional range validation for
  memory safety. 3 new round-trip/validation tests (`rustysnes-cpu` ×1, `rustysnes-ppu` ×2).
  T-52-003 completes here with `Apu` (`rustysnes-apu` now depends on `rustysnes-savestate`
  too): `Spc700::save_state`/`load_state` (the SPC700 register file + `STOP`/`SLEEP` latches),
  `Dsp::save_state`/`load_state` (the 128-byte register mirror, all 8 voices, the shared
  main-volume/echo/noise/BRR/latch/clock sub-units, the 32-step micro-sequence phase, and the
  queued output-sample FIFO — a voice's `envelope_mode` discriminant outside `EnvMode`'s four
  variants is rejected, and a FIFO length beyond the live FIFO's own `AUDIO_FIFO_CAP` bound is
  rejected too, since neither could arise from real execution; the Gaussian interpolation table
  is NOT written — it's a pure compile-time-derived constant, identical on every fresh `Dsp`),
  and `Apu::save_state`/`load_state` (ARAM, the `$00F0-$00FF` register file, the three timers,
  the DSP sample counter, and the in-flight instruction micro-op plan — the SPC700 analogue of
  the GSU's `pending_clocks`/`pending_idx`, needed because `Apu::advance_smp_cycle`'s
  sub-instruction lockstep can leave an instruction genuinely mid-drain at any save point; a
  claimed plan length beyond `MAX_SAVED_PLAN_LEN` is rejected (mirroring the GSU's validation);
  a step's `base_clocks` outside `{1, 2}` (the only values `record`/`record_next_instruction`
  ever produce) is rejected; `plan_pos` beyond the restored plan's length is rejected; and
  `plan_sub` inconsistent with `plan_pos` (nonzero past the plan's end, or `>=` the step at
  `plan_pos`'s own `base_clocks`) is rejected too — either would let `advance_smp_cycle` commit a
  deferred port write at the wrong cycle on resume. The 64-byte IPL boot ROM is never written (a
  fixed public-domain constant, identical on every SNES). Every voice's `index` (masked `&
  0x70`, the 8 legal voice-register bases — found in review: an unmasked value `>= 0x80` would
  index `registers[128]` out of bounds via `index | 0x09`) and `buffer_offset` (masked `% 12`,
  the ring-buffer's own size — a second use site the initial pass missed lacks the `%12` wrap
  `gaussian_interpolate` applies) are masked on load; `Echo::history_offset` and a timer's
  `stage3` are masked too. 2 new round-trip/validation tests (`rustysnes-apu` ×2).
  T-52-003 completes here with `System::save_state()`/`load_state()` (`rustysnes-core` now
  depends on `rustysnes-savestate` too) — the versioned envelope: a 4-byte magic (`b"RSNS"`) + a
  `u16` format version `System::load_state` rejects if newer than this build understands
  (`SaveStateError::UnsupportedVersion`), or if the leading bytes aren't the magic at all
  (`SaveStateError::BadMagic`), wrapping `Cpu`, the whole `Bus` (`Ppu`/`Apu`/the new `Dma`/
  `Clock`/`MulDiv` save-states + WRAM, plus — if a cart is loaded — its coprocessor state and
  battery SRAM), and the SA-1 second CPU + its master-clock catch-up accounting when present. A
  save-state's cart/SA-1 presence, and a restored SRAM image's length, are cross-checked against
  the target `System`'s own installed state on load and rejected on mismatch rather than
  silently corrupted — restoring a cart-carrying save-state requires the caller to have already
  loaded the SAME ROM first, the same "never embed a ROM/firmware byte" posture every
  coprocessor's firmware already follows. 6 new round-trip/validation tests (`rustysnes-core`:
  `dma.rs` ×1, `scheduler.rs` ×3 covering the no-cart round trip plus bad-magic and
  newer-format-version rejection). **T-52-003 is now fully complete** — every subsystem
  (`Cpu`/`Ppu`/`Apu`/`Bus`/`Cart`) round-trips its exact state through one versioned envelope.
  Closes with **T-52-004, the round-trip determinism test that is this format's actual spec**
  (`crates/rustysnes-test-harness/tests/save_state_determinism.rs`): boot a ROM, run 30 frames,
  snapshot, restore the snapshot onto a SEPARATE freshly-booted `System` (the same ROM loaded
  fresh — a save-state never embeds a ROM byte), run 30 more frames on both the original
  (continuing uninterrupted) and the restored system, and assert the framebuffer + queued audio
  samples are byte-identical between the two. Green across a no-coprocessor ROM (the committed
  gilyon `cputest-basic.sfc`, always present), a `Curated`-tier Super FX Krom ROM, and a
  `BestEffort`-tier commercial coprocessor ROM (the latter two self-skip when the gitignored
  external corpus is absent, matching every other on-cart test in this suite). **`docs/adr/0006`
  is now `Accepted`** — the save-state format is a stable public contract every post-`v1.0.0`
  Reach feature (netplay rollback, TAS replay) can build on. This closes out the `v0.2.0
  "Persistence"` sprint in full.

## [0.1.0] "Foundation" - 2026-07-02

The first tagged release. Everything below accumulated across CPU, PPU/scheduler, APU, cart/
coprocessor, and frontend development before any release was ever cut — this tag closes that
gap; see `to-dos/VERSION-PLAN.md` for why and for the release ladder going forward.

**Oracle/golden suites: all held, no regressions.** 65816 SingleStepTests 0-diff (state+cycles,
5,119,999/5,120,000 gated on license), SPC700 SingleStepTests 0-diff, gilyon on-cart CPU suite
1107/1107 "Success", undisbeliever PPU/DMA/HDMA golden 29/29, blargg `spc_smp`/`spc_timer`/
`spc_mem_access_times` literal `PASSED TESTS`, `spc_dsp6` known residual (reported honestly,
see the S-DSP entry below).

### Added

- **Zip-archive ROM loading** (`rustysnes-frontend`): `EmuCore::load_rom` now sniffs the local-
  file-header magic and transparently extracts the first `.sfc`/`.smc`/`.fig`/`.swc` entry from a
  `.zip`-wrapped ROM before header detection — the common distribution format for commercial ROM
  dumps. Pure in-memory (a `Cursor` over the already-loaded byte slice, `deflate`-only via a
  pure-Rust `flate2` backend), so it works identically on native and the `wasm32-unknown-unknown`
  target with no system zlib dependency. A plain unwrapped `.sfc`/`.smc` file still passes through
  unchanged. Note: the wasm/GitHub Pages build's in-browser file-loading UI itself is still a
  bootstrap scaffold (see `docs/STATUS.md`) — this lands the extraction logic every future loading
  path (native today, the browser UI once it exists) shares, not a browser-side feature yet.
- **Phase 7 — BestEffort coprocessors: OBC1, DSP-2, DSP-4, ST010, CX4, S-DD1, SPC7110.**
  - **OBC1** (`coproc::obc1`): dedicated 8 KiB RAM behind a reprogrammable cursor register.
    Validated against real Metal Combat: Falcon's Revenge.
  - **DSP-2 / DSP-4 / ST010** (`coproc::necdsp_variant`): reuse the DSP-1 µPD77C25/µPD96050 LLE
    engine, title-detected. DSP-4 needed a DSP-1-style half-window DR/SR split instead of the
    generic bit-0 split (found via a real Top Gear 3000 boot-time hardware check). Validated
    against real Dungeon Master, Top Gear 3000, and F1 ROC II.
  - **CX4** (`coproc::hg51b` + `coproc::cx4`): a clean-room Hitachi HG51B S169 core (sequential
    mask/value opcode decode transcribed from ares' `pattern(...)` strings) — no chip dump for the
    program (runs from cart ROM), only a 3 KiB data-ROM constant table. Fixed a real bug where
    pending DMA/cache work triggered while the chip was halted never ran. Validated against real
    Mega Man X2 and X3.
  - **S-DD1** (`coproc::sdd1`): a Golomb-code + adaptive-binary-probability decompressor streamed
    during fixed-address DMA via a new `Board::notify_dma_channel` hook (`rustysnes-core::Dma`
    owns the DMA registers directly, so the cart needs an explicit snoop). Fixed a real `u8`
    shift-by-8 overflow in the codeword reader (well-defined in the original C++ via implicit int
    promotion; a genuine bug once ported literally to Rust). Validated against real Star Ocean and
    Street Fighter Alpha 2.
  - **SPC7110** (`coproc::spc7110`) + a paired **Epson RTC-4513** (`coproc::epsonrtc`, seeded to a
    fixed epoch to preserve the determinism contract): decompression unit, data-port unit, ALU, and
    a 4×1 MiB bankable memory-control unit. Fixed two header-detection bugs uncovered while wiring
    it up (the title string is "TENGAI MAKYO", not "…MAKYOU"; the `$F`-custom chipset-nibble gate
    wrongly excluded RTC carts' `$F9` byte) and added a `$40-$7D` HiROM-style ROM mirror. Does not
    yet boot to real content on its one available ROM (Far East of Eden Zero) — implemented but
    unvalidated, tracked as a known gap.

- **Phase 5 — Playable native frontend (`rustysnes-frontend`).** The always-on egui shell is now a
  working SNES emulator: a real commercial ROM boots in a window with picture, sound, and control.
  - **Video:** `EmuCore` decodes the PPU's 256×(224|239) 15-bit BGR555 framebuffer to RGBA8 each
    frame and uploads it to the wgpu streaming texture; the blit now samples only the live sub-rect
    and letterboxes to the 4:3 SNES display aspect via a small uniform (the prior skeleton sampled
    the whole oversized texture). The stale "PPU produces no pixels / cleared frame" path in
    `emu.rs` is replaced with the real present path.
  - **Audio:** a new additive S-DSP output FIFO (`Apu::drain_audio`, captured at the DAC-latch point
    in `dsp::echo27`) feeds a producer-side linear resampler (32 kHz → cpal device rate, DRC-paced)
    into the lock-free ring; the cpal callback now emits true stereo. The FIFO is pure
    instrumentation over already-emitted samples, so the deterministic audio contract is unchanged.
  - **Input:** keyboard (default SNES map) + gilrs gamepad late-latch into `Bus::set_joypad` for P1
    and P2.
  - **Cartridge UX:** ROM load resolves coprocessor firmware (DSP-1.. / CX4) from beside the ROM /
    a `firmware/` dir and auto-loads a `<rom>.srm` battery save; **Reset**, **Power-Cycle**, and
    **Pause** are wired to the core; a missing firmware dump surfaces a clear "supply it" message
    (the `docs/adr/0003` honesty posture).
  - **Dependency stack refreshed to the latest mutually-compatible tier:** egui / egui-wgpu /
    egui-winit **0.35**, wgpu **29**, winit **0.30** (winit 0.31 is beta-only and egui-winit 0.35
    pins to 0.30 — winit is the gating crate), directories **6**, wasm-bindgen **0.2.126** /
    web-sys · js-sys **0.3.103** / wasm-bindgen-futures **0.4.76**. Native **and**
    `wasm32-unknown-unknown` both build.
  - **Validation:** a `playable_smoke` integration test drives a staged commercial ROM through the
    same `EmuCore` path the GUI uses and asserts a structured (non-blank) frame **and** a non-silent
    audio stream (Super Mario World: 256×224 picture + 63,975 samples over 120 frames); it skips
    cleanly when no ROM is staged. The native binary was also launched headless under xvfb (clean
    init + run, no panic).
  - **Deferred:** save-states / rewind / run-ahead (need a core-wide deterministic snapshot across
    the `Board` trait + APU/Bus/System) and the full wasm browser frontend (the wasm entry point is
    a compiling bootstrap scaffold).

- **Phase 4 — SA-1 (second 65C816 + ASIC) coprocessor:** a clean-room
  `no_std`/`forbid(unsafe_code)` port of the SA-1 system (`rustysnes-cart::coproc::sa1::Sa1Board`,
  from ares' `sfc/coprocessor/sa1`, ISC) — the `$2200–$23FF` register file (SA-1 control/reset, the
  bidirectional S-CPU↔SA-1 IRQ/NMI/message lines + the S-CPU NMI/IRQ vector redirect), the Super-MMC
  ROM banking (CXB/DXB/EXB/FXB), BW-RAM (the shared battery RAM with the `$2224` 8 KiB S-CPU window,
  the `$40–$4F` linear image, the SA-1 2/4 bpp bitmap + linear projections, and the SWEN/CWEN/BWPA
  write-protect), 2 KiB I-RAM (SIWP/CIWP protect), the arithmetic unit (signed multiply / unsigned
  divide / cumulative-sum sigma), the variable-length bit unit, the H/V timer, and the normal +
  type-1/type-2 character-conversion DMA. The **second 65C816** is instantiated and stepped in
  `rustysnes-core` (the one-directional crate graph keeps the CPU core out of the cart crate): the
  scheduler owns an optional `sa1_cpu`, wires a `Sa1Bus` adapter to the new `Board` second-CPU hooks
  (`has_second_cpu` / `second_cpu_read|write` / `second_cpu_running` / `second_cpu_take_reset` /
  `second_cpu_poll_nmi|irq` / `second_cpu_tick`), and advances it in deterministic master-clock
  catch-up — so the SA-1 runs in parallel **without perturbing the main CPU** (the `cpu_oracle`
  stays 0-diff; SA-1 stepping is gated to SA-1 carts). `Board::irq_pending()` is now ORed into the
  bus IRQ line (the documented wiring), so the SA-1→S-CPU IRQ reaches the main CPU. `board::select`
  routes `Coprocessor::Sa1` (no chip dump — the SA-1 program is in cart ROM). Tier stays
  **Curated** and joins the honesty oracle set. Validated by the new `sa1_oncart` harness gate (18
  commercial SA-1 carts: detection + S-CPU↔SA-1 register traffic for all 18, an aggregate
  "the SA-1 CPU executed millions of cycles" liveness floor — Super Mario RPG, both Kirby titles,
  PGA Tour 96, Power Rangers Zeo, … — and a deterministic golden framebuffer) plus board unit tests.
- **Phase 4 — Super FX / GSU (Argonaut RISC) coprocessor:** a clean-room
  `no_std`/`forbid(unsafe_code)` port of the GSU core (`rustysnes-cart::coproc::gsu`) — the full
  Argonaut RISC: R0–R15 (R15 = PC) with the FROM/TO/WITH register-select prefixes, the
  ALT1/ALT2/ALT3 composite-mode machine, the ALU + signed/unsigned `mult`/`umult` and the
  `fmult`/`lmult` multiplier, the ROM buffer (ROMBR:R14 + busy/latency) and RAM buffer
  (RAMBR/RAMADDR + deferred-write latency), the 256-byte/32-line opcode cache, the 1-instruction
  pipeline that gives the GSU its branch delay slot, and the PLOT/RPIX pixel-plot pipeline (the
  two-deep pixel cache, the color/cmode logic with dither/freeze-high/high-nibble/transparent, and
  the SCBR/SCMR screen-base + 2/4/8 bpp character-format addressing) + the SFR status flags. Added
  `SuperFxBoard` (`coproc::superfx`): it owns the cart ROM + the Game Pak RAM (the GSU plot
  bitmap), decodes the LoROM Super FX CPU map (the `$3000–$32FF` register/cache window, the LoROM +
  linear ROM windows, the `$70–$71` + `$6000–$7FFF` RAM windows), and arbitrates the shared
  ROM/RAM bus (the snooze-vector / open-bus model while the GSU owns the bus). Unlike the DSP
  family there is **no chip-ROM dump** — the GSU program lives in the cartridge ROM — so the board
  is functional the moment the cart loads. Host↔GSU sync reuses the DSP-1 idea: the board runs the
  GSU **to completion the instant the CPU sets the Go flag** (`Gsu::run_until_stopped`, capped),
  byte-exact and deterministic with **no free-running core-scheduler tick**. `board::select` routes
  `Coprocessor::SuperFx` (the base board is never built — Super FX re-decodes its own map). New
  harness gate `superfx_oncart` (feature `test-roms`, self-skips when ROMs absent): boots the 58
  Krom GSU test ROMs (2/4/8 bpp PlotPixel/PlotLine/FillPoly + the per-instruction `GSUTest` suite)
  on the full System and asserts SuperFx detection, that the GSU actually executed its program out
  of cart ROM, that the `FillPoly` suites plot a substantial bitmap into the Game Pak RAM (the
  whole plot pipeline end-to-end at the cart boundary, PPU-independent), and a committed
  deterministic golden framebuffer. `mapper_tier_honesty` adds `SuperFx` to the oracle set and
  stays green (Super FX is the second `Core/Curated` coprocessor backing the oracle). Engine unit
  tests cover a hand-assembled `ibt`/`stop` program through the full host-sync path + the board
  ROM/RAM/register decode.
- **Phase 4 — DSP-1 (NEC µPD77C25) + the shared NEC DSP engine:** a clean-room
  `no_std`/`forbid(unsafe_code)` port of the NEC µPD77C25 / µPD96050 LLE core
  (`rustysnes-cart::coproc::upd77c25`) — the full DSP instruction set (OP/RT/JP/LD, the K×L
  signed-multiplier pipeline, dual accumulators + 6-flag condition sets, the 16-deep stack,
  program/data ROM + data RAM, and the DR/SR/DP host ports), revision-parameterized so one engine
  backs DSP-1/2/3/4 + ST010/011 (six chips). Added the `Dsp1Board` (`coproc::dsp1`) wrapping a base
  LoROM/HiROM board and intercepting the DR/SR window, with the snes9x/ares-equivalent window
  selection (HiROM `$00–$1F:$6000–$7FFF`; LoROM ≤1 MiB `$30–$3F:$8000–$FFFF`; LoROM >1 MiB
  `$60–$6F:$0000–$7FFF`). Header detection now decodes the coprocessor from the `$FFD6` chipset
  byte, and `board::select` routes `Coprocessor::Dsp` to the DSP-1 board. New
  `Cart::install_coprocessor_firmware` + `Board::load_firmware` hook: the µPD77C25 is **inert until
  the user supplies the `dsp1.rom` / `dsp1b.rom` chip dump** (gitignored, never committed —
  `docs/adr/0003`), never silently degraded. Host↔chip sync is the RQM-handshake catch-up
  (`run_until_rqm`) — byte-exact at the bus boundary and fully deterministic, no core-scheduler
  hook. New harness gate `dsp1_oncart` (feature `test-roms`, self-skips when ROMs/firmware absent):
  boots Super Mario Kart / Pilotwings / Super Bases Loaded 2 / Aim for the Ace on the full System,
  asserts DSP detection, the RQM-handshake access count on both the LoROM and HiROM windows, a
  committed deterministic golden framebuffer, and the firmware-differential (the Mode-7 titles
  render differently with the chip installed). Engine unit tests cover the decode/ALU/multiplier
  via a hand-assembled synthetic firmware. The `mapper_tier_honesty` gate stays green (DSP-1 is the
  first real `Core/Curated` coprocessor backing the oracle).
- **Phase 3 — Audio (SPC700 + S-DSP + ARAM):** a clean-room SPC700 (S-SMP) core driving the
  SingleStepTests/spc700 oracle to **0-diff — 100% state + cycle count over all 256 opcodes**
  (12,800 committed-sample tests in-tree; 256,000 in the full external tier). Full 256-opcode
  set with every addressing mode, `MUL`/`DIV`, the word ops, `DAA`/`DAS`, the bit-manipulation
  family, and `STP`/`SLEEP`. Added the S-DSP (8 voices, BRR decode, 4-point Gaussian
  interpolation, ADSR/GAIN envelopes, noise + PMON, KON/KOFF/ENDX edges, the 8-tap echo FIR +
  feedback, MVOL/EVOL, 32 kHz stereo mix), the 64 KiB ARAM, the three timers, the four
  `$2140-$2143` communication-port latches, and the IPL boot ROM. New `Apu` API
  (`tick`/`step_instruction`/`run_cycles`/`cpu_read_port`/`cpu_write_port`/`sample`) for the
  core to wire the bus ports + async resync. New oracle `tests/spc700_oracle.rs` (gated behind
  `test-roms`, self-skips when data absent). `#![no_std]` + `forbid(unsafe_code)`; bare-metal
  `thumbv7em-none-eabihf` build green. See `docs/apu.md` §Implementation status.
- **Phase 3 — APU↔machine integration + the deterministic async resync (T-31-002/T-31-003):**
  wired the four `$2140-$2143` CPU↔APU ports through the real `Apu` (`cpu_read_port` /
  `cpu_write_port` — the one-way latches; removed the dead `apu_ports` latch array), so the CPU's
  IPL upload handshake now reaches the SPC700. The SPC700/S-DSP advance in **integer-accumulator
  lockstep** with the 21.477 MHz master clock at the exact reduced ratio `68_352 / 715_909`
  (= `(apuFrequency/12) / NTSC-master`, no floating point — determinism, `docs/adr/0004`), so a
  CPU port read observes every SMP write up to that master instant. Corrected the SMP internal
  timebase to the ares base clock (`apuFrequency/12 ≈ 2.05 MHz`, `SMP_WAIT = 2` base clocks per
  access matching ares `cycleWaitStates[0]`, S-DSP one 32 kHz sample every 64 base clocks),
  replacing the earlier 1-unit/access approximation that ran the timers + DSP off-rate. New
  `Apu::advance_smp_cycle` (port-preserving lockstep advance) + `smp_pc`/`smp_stopped` debug
  accessors. Added `tests/blargg_spc.rs` (gated behind `test-roms`, self-skips when the
  external-tier ROMs are absent): the four blargg `spc_*` ROMs **boot, drive the IPL upload
  handshake to completion, and execute the uploaded SPC700 program bit-deterministically**
  (framebuffer + ARAM + ports hashed identical across runs) against the committed baseline
  `tests/golden/blargg-spc.tsv`.
- **Phase 3 — cycle-exact SMP↔CPU lockstep (T-31-004):** `Apu::advance_smp_cycle` now releases
  **exactly one SMP base clock per call** by draining a recorded micro-op timeline of the in-flight
  SPC700 instruction (one entry per bus access, with each SMP→CPU port write **deferred to the
  precise base cycle** its access completes). The new `RecordingSmpBus` runs the *unchanged*
  `Spc700::step` and applies every side effect byte-for-byte as the per-instruction `SmpBus` does —
  so the SPC700 oracle stays **0-diff (100%)** — while emitting the timeline. This is the
  ares/bsnes cooperative-thread interleaving achieved single-threaded (no coroutines, so save-state
  / netplay stay bit-deterministic). With it, **all four blargg `spc_*` ROMs now boot, upload, run,
  and stream their detailed result grids** (previously all stalled at "Running tests:");
  `tests/blargg_spc.rs` was upgraded to **decode and report the real on-screen verdict** (blargg's
  BG-tilemap header at `$0400` + result grid at `$0800`), keeping — not weakening — the
  deterministic + baseline-hash assertion (baselines re-blessed for the new timing). A literal
  blargg PASS is **still not earned**: every ROM streams its grid and reports **Failed 02**
  (`spc_smp` after the CPU-Instructions + CPU-Timing opcode grid; `spc_dsp6` after the Echo +
  Envelope list; `spc_timer` / `spc_mem_access_times` likewise). The residual is a sub-cycle
  interleave skew intrinsic to the **CPU-leading** clock model: ares/bsnes use a *symmetric*
  cooperative-thread model (either chip may lead, the other catches up at its port access), which
  would require a CPU↔SMP bus-master inversion out of scope for an APU-only change. Documented
  honestly in `docs/apu.md` §cycle-exact / `docs/scheduler.md` / `docs/STATUS.md` — reported, not
  faked. **(Superseded — see "Fixed: SPC700 timer clocking phase" below: the `spc_smp`/`spc_timer`/
  `spc_mem_access_times` residual was the recording-bus write phase, not a clock-model asymmetry, and
  all three now reach a literal PASS.)**
- **Phase 3 — cycle-accurate (cycle-stepped) S-DSP (T-31-005):** decomposed the S-DSP's monolithic
  per-sample `voice_pipeline` into the nine per-voice steps (`voice1..voice9`, with `voice3a/b/c`),
  the echo path (`echo22..echo30`), and `misc27..misc30`, scheduled on the **32-entry ares phase
  table** (`sfc/dsp/dsp.cpp::main`, ISC) via a new `Dsp::tick` (one phase per call; voices
  interleaved, voice 0 wrapping the sample boundary, DAC latched at phase 27 / `echo27`). The
  integration (`Apu` `step_instruction` + `RecordingSmpBus::record`) now drives the DSP **one tick
  per 2 SMP base clocks** (32 ticks = one 32 kHz sample) instead of a whole sample per 64 clocks, so
  an SMP instruction that reads a DSP register (`$F3`) mid-execution sees **cycle-correct
  sub-sample** OUTX/ENVX/ENDX/envelope state. `Dsp::run_sample` is retained as the batched
  `32 × tick` wrapper; a new guard test (`run_sample_equals_32_ticks_with_brr_content`) asserts the
  batched vs one-at-a-time drives are bit-identical (sample stream + ARAM) on real BRR/echo content.
  **Empirical outcome:** the cycle-accurate DSP *isolated* the residual blargg gap — `spc_smp` /
  `spc_timer` / `spc_mem_access_times` are now **byte-for-byte identical** to the per-sample build
  (DSP granularity was provably **not** their blocker; their residual is the CPU-leading clock-model
  asymmetry above), while `spc_dsp6` (the DSP-register-reading member) changed — more Echo/Envelope
  timing resolves — but still reports **Failed 02**. SPC700
  oracle stays **0-diff**, undisbeliever framebuffer golden **29/29**, all DSP unit + APU
  integration tests green, `#![no_std]` + `forbid(unsafe_code)` preserved. See `docs/apu.md`
  §cycle-accurate DSP. **(Superseded for the three timer-mechanism ROMs — see "Fixed: SPC700 timer
  clocking phase" below: `spc_smp`/`spc_timer`/`spc_mem_access_times` now reach a literal PASS;
  `spc_dsp6` remains Failed 02 on the S-DSP residual.)**
- Initial workspace scaffold (cycle-accurate emulator architecture, ported from RustyNES).
- Seeded `tests/roms/` with the permissive corpora — gilyon (MIT), undisbeliever (MIT/Zlib),
  and a deterministic SingleStepTests/spc700 (MIT) sample — plus the gitignored `external/`
  tier (65816, full spc700, 240p, Krom, blargg-spc). Curated the commercial-ROM coverage
  manifest (`tests/roms/commercial-corpus.json`) to popularity-weighted, genre-diverse beloved
  titles (metadata + SHA-256 only; no ROM bytes committed).
- **Phase 2 — cartridge base-mapper memory model: real SNES internal-header detection
  (copier-prefix strip + scored `$7FC0`/`$FFC0`/`$40FFC0` candidate selection on
  checksum+complement, map-mode, reset-vector, and printable-title heuristics) and working
  LoROM / HiROM / ExHiROM address decode backed by owned `rom`/`sram` storage.** `Cart::load`
  builds the board with the stripped ROM bytes + a zeroed header-sized SRAM; `read24`/`write24`
  route through the decode over real memory (ROM read-only, SRAM read/write, hardware-accurate
  non-power-of-two ROM mirroring). Added `save_sram`/`load_sram` battery accessors. Coprocessors
  remain stubs (Phase 4).
- **Phase 2 — dual-chip PPU (PPU1 5C77 + PPU2 5C78):** VRAM/CGRAM/OAM + the full `$2100-$213F`
  register file (with the BG-offset / Mode-7 / scroll write latches, VMAIN remap + increment,
  CGRAM/OAM/VRAM read-prefetch quirks, MPYL/M/H multiply, SLHV/OPHCT/OPVCT, STAT77/78); BG modes
  0-7 tile fetch (2/4/8 bpp, per-mode priority, 16×16 tiles, mosaic); Mode 7 affine
  (matrix/center/wrap/flip, EXTBG); the 128-sprite OAM pipeline (32-sprite range / 34-tile time
  limits, reverse-order fetch); color math (add/sub/half, fixed/sub addend, direct color); and
  windows (OR/AND/XOR/XNOR). Per-scanline compositor; mid-line raster + hi-res 512 deferred.
- **Phase 2 — master-clock lockstep scheduler + bus + DMA/HDMA:** the master clock advances
  through the CPU's bus accesses on the **6/8/12 region access-speed map** (ares `CPU::wait`,
  `$420D` MEMSEL FastROM), stepping the PPU dot clock (4 master/dot) + the SPC accumulator in
  lockstep. Full 24-bit memory decode (WRAM + low-mirror, PPU/APU B-bus, controllers, the
  CPU registers `$4200-$421F` incl. the multiply/divide unit, the DMA registers, cart routing).
  The 8-channel **GP-DMA** (CPU-halt, 8 transfer modes) and **HDMA** (per-line budget, indirect
  tables, mode lengths `{1,2,2,4,4,4,2,4}`) clean-room from ares `dma.cpp`. NMI + the RDNMI
  VBlank flag + the H/V-IRQ comparator. The `System` boots a cart from its reset vector and runs
  deterministic frames (an NTSC frame ≈357,374 master clocks).
- **Phase 2 — verified on-cart:** gilyon `cputest-basic.sfc` boots and reports "Success" (all
  1107 65C816 tests; `tests/gilyon_oncart.rs`) — closing Phase 1's deferred on-cart criterion;
  the 29 undisbeliever PPU/DMA/HDMA ROMs render bit-deterministic golden framebuffers matching
  `tests/golden/undisbeliever-framebuffer.tsv` (`tests/undisbeliever_golden.rs`).
- **Phase 1 — CPU + golden oracle: the WDC 65C816 core passes the SingleStepTests/65816
  per-opcode oracle to 0-diff** on architectural state **and** per-instruction cycle count
  (5,119,999 / 5,120,000 = 100.00% across all 512 opcode files × 10,000 tests, native +
  emulation). All 256 opcodes × addressing modes, `REP`/`SEP` width changes, and `XCE`
  emulation/native transitions verified.

### Fixed

- **Dot-accurate HDMA servicing + H-IRQ / interrupt latency ⇒ `hdmaen_latch_test` now bands.** Three
  coupled accuracy fixes, all traced to ares source, turn undisbeliever's `hdmaen_latch_test` from a
  flat per-line alternation into the banded HDMAEN-vs-latch crossing hardware shows:
  - **HDMA is serviced at its dot-accurate position, not the scanline boundary.** Per ares
    `sfc/cpu/timing.cpp`, HDMA now runs a once-per-frame **setup** at V=0 and a per-visible-line
    **run** at **hcounter 1104 = dot 276** (`HDMA_RUN_DOT`). Servicing at that exact dot latches a
    mid-line `STA $420C` on the hardware-correct scanline. (`superfx-framebuffer.tsv`
    `GSU2BPP256x192PlotLine` re-blessed for the re-timed GSU concurrency — Star Fox fly-in ship +
    planet verified still rendering; `sa1-framebuffer.tsv` `SD F-1 Grand Prix` re-blessed, same boot
    screen confirmed against HEAD, positions shifted by the IRQ delay below.)
  - **Hardware NMI/IRQ open with two internal cycles, not one** (`CPU::service_interrupt`). The WDC
    sequence is 2 internal + pushes + 2 vector fetches; the path is hardware IRQ/NMI only, so BRK/COP
    keep their oracle-validated counts (5.12M-test CPU oracle still 100%).
  - **The H-IRQ comparator lags `HTIME` by 4 dots** (`HIRQ_TRIGGER_DELAY`, PPU `check_hv_irq`),
    modelling the counter→interrupt communication delay ares encodes as `hcounter(10) ==
    (HTIME+1)<<2` (fire at dot `HTIME + 3.5`). Together these push the IRQ-gated `$420C` write-drift
    up across the dot-1104 latch, producing the crossing.

  **Determinism caveat (honesty gate):** undisbeliever documents `hdmaen_latch_test.sfc` as *not a
  stable test* — its exact bands differ every power-cycle on real hardware. RustySNES is
  deterministic, so it produces one fixed realization; the re-blessed golden is a regression snapshot
  of that, **not** a byte-match to ares. What is portable and spec-accurate is the *mechanism*, now
  present. `docs/scheduler.md` §§DMA/HDMA, H/V-IRQ; `docs/cpu.md`; `docs/ppu.md` updated.
- **65C816 memory-access timing is now cycle-exact (ares `CPU::read`/`write` phasing).** A CPU cycle
  used to perform its bus access *first* and advance the master clock *after*, so a register write
  landed a full cycle (6/8/12 master clocks) too early relative to the PPU/HDMA. The CPU now asks the
  bus for the access cost (`Bus::access_cycles`, ares `wait`) and sequences the advance
  (`Bus::advance`, ares `step`) around the access: a **write** advances the whole cycle then stores
  (lands at the cycle end), a **read** advances cost−4, reads, then advances 4 (lands four clocks
  before the end). Instruction cycle *counts* are unchanged (the CPU-timing tables still pass); only
  the sub-cycle phase at which each access becomes visible to the PPU/HDMA moves to the hardware-exact
  instant. `superfx-framebuffer.tsv` re-blessed for the re-phased GSU concurrency (Star Fox fly-in
  ship + planet verified still rendering); undisbeliever stays 29/29. (The `hdmaen_latch` banding
  this note previously deferred is resolved by the dot-accurate HDMA + IRQ-latency entry below.)
- **Star Fox fly-in now renders correctly (Super FX) — ship and planet.** Four coupled fixes across
  the DMA/HDMA, PPU, cart, and CPU paths, all validated against ares:
  - **HDMA during GP-DMA (missing ship segment).** `Bus::run_gp_dma` takes the `Dma` out of the bus,
    so while a framebuffer GP-DMA ran, HDMA (Star Fox's per-line force-blank) was dormant and the
    DMA's tail lines dropped. The taken `Dma` now drives HDMA itself at scanline crossings via
    `Dma::service_hdma_line` / `service_hdma_during_gp` (new `DmaBus` scanline hooks); a
    frame-crossing framebuffer DMA no longer drops writes.
  - **HDMA setup/reset faithfulness.** `hdma_setup` sets `hdma_do_transfer` for every channel before
    the enable-check and `service_hdma_line` runs `hdma_reset` unconditionally at frame start
    (matching ares `Channel::hdmaSetup` / `timing.cpp`), so a channel enabled mid-frame reactivates.
  - **Mode-2 offset-per-tile (missing planet).** The planet is a mode-2 OPT BG2 layer, not a GSU
    object; `render_bg` ignored OPT so its columns never scrolled in. Implemented mode-2/4/6 OPT
    (`bg3_opt_tile` + per-column `world_x`/`world_y` override), transcribed from ares
    `background.cpp` — a general accuracy improvement for any OPT-using game.
  - **Super FX CPU→Game Pak RAM writes are unconditional.** `SuperFxBoard::write24` no longer gates
    RAM writes behind GSU ownership (reads still return open bus), matching ares `CPURAM::write`.
  - **65C816 `WAI` wakes on any asserted interrupt line** regardless of the `I` flag (WDC datasheet);
    a masked-IRQ `SEI; WAI` sync primitive no longer hangs.
  - Goldens re-blessed for the intentional behavior change: `superfx-framebuffer.tsv` (Super FX
    corpus now plots structured framebuffers) and the two `hdmaen_latch` entries in
    `undisbeliever-framebuffer.tsv` (HDMA now executes instead of a blank screen). undisbeliever
    stays 29/29; `superfx_oncart` passes. Exact `hdmaen_latch` band-parity with ares additionally
    needs cycle-exact 65C816 write timing and is tracked separately.
- **PPU color math — subscreen-backdrop addend is the fixed color (washed/black backgrounds).**
  When "add subscreen" (CGWSEL $2130 bit 1) is enabled but the subscreen pixel at a column is the
  backdrop (no opaque sub-layer wrote it), the color-math addend must be **COLDATA's fixed color**,
  not CGRAM[0]. `compose_dac` (`crates/rustysnes-ppu/src/render.rs`) used `layer_color(&bp)`, which
  returns CGRAM[0] (black) for a transparent subscreen pixel, so SMW's blue title sky (painted by
  the fixed color over a black main backdrop) rendered **black**. Now the addend falls back to the
  fixed color when the subscreen is transparent, and the half is suppressed for that pixel —
  matching ares `DAC::above` (`io.blendMode && math.transparent ⇒ addend = fixedColor()`). Verified
  by an ares (ISC) framebuffer pixel-diff on Super Mario World: the title sky now reads the fixed
  color (BGR555 `0x7393` ⇒ light blue) instead of `0x0000`.
- **PPU background palette-group offset (washed multi-palette BG art).** A BG tilemap entry's 3-bit
  palette group (bits 12–10) was fetched but dropped from the CGRAM index, collapsing every BG tile
  onto palette group 0 — the SMW logo and brick border rendered as flat grey/cream instead of their
  per-letter colors. `render_bg` now computes `paletteBase + (group << bpp) + color` (masked to a
  byte; 8bpp ignores the group; `paletteBase = id<<5` only in Mode 0), per ares `background.cpp`.
  The ares pixel-diff confirms the title logo/border colors now match. **undisbeliever golden stays
  29/29** (no re-bless — none of those ROMs exercise a hash-affecting non-zero BG palette group or
  subscreen-backdrop math); the PPU stays `#![no_std]` + `forbid(unsafe_code)`.
- **Frontend pacing — emulation ran at the display refresh, not the region rate (~2–3× too fast).**
  The synchronous (default) drive stepped exactly one emulated frame per winit `RedrawRequested`,
  i.e. once per display vsync, so a 144 Hz monitor ran the emulator 2.4× too fast. The present path
  now drives emulation from a wall-clock **fixed-timestep accumulator** (`app::Pacer`): `run_frame`
  runs only once `1 / region.frame_rate()` seconds of real time have accrued (NTSC 60.0988 /
  PAL 50.0070 Hz), the latest framebuffer is presented in between, catch-up after a stall is capped
  to avoid a spiral of death, and the present mode now governs **only** vsync/tearing. Unit-tested
  to hold ~60 fps across 30/60/75/144/240 Hz present rates (`pacing_tracks_region_rate_not_present_rate`).
- **Frontend FPS counter always read `0.0`.** `ShellInfo::fps` was hardcoded to `0.0`; the new
  `Pacer` measures the emulated-frame rate over a 0.5 s window and feeds the status bar.
- **Frontend Settings → Video present-mode toggle did nothing.** The radio wrote
  `config.video.present_mode` but the wgpu surface was only configured once at startup. The present
  path now detects the change and calls the new `Gfx::set_present_mode`, which re-validates against
  the surface's supported modes (falling back to `Fifo`) and reconfigures the live surface.
- **S-DSP GAIN mode-7 threshold (blargg `spc_dsp6` literal PASS, T-31-007):** `Dsp::envelope_run`
  compared the voice's internal envelope latch (`env_internal`) against the bent/two-slope
  `GAIN`-increase threshold `0x600` with a **signed** test, where blargg `SPC_DSP`
  (`(unsigned) hidden_env >= 0x600`) and ares (`(u32) _envelope >= 0x600`) use an **unsigned** one.
  The latch is the pre-clamp envelope; a preceding `GAIN` *decrease* mode (4 linear / 5 exponential)
  can leave it **negative**, and the unsigned reinterpretation makes that trip the reduced `+0x08`
  slope — a signed compare misses it and over-increments by `+0x20`. This was the sole divergence
  behind `spc_dsp6`'s `Envelope/gain $E0 threshold` → **"Failed 02"**. Cast the latch to `u32` for
  the comparison (`crates/rustysnes-apu/src/dsp.rs`), matching both references; the rest of the
  envelope path was already bit-identical to ares (verified by an all-`GAIN`-value differential).
  **`spc_dsp6` now reaches blargg's literal `PASSED TESTS`** (rendered at `$0800` row 30 near frame
  8.8k), so **all four blargg `spc_*` ROMs are now asserted to PASS** in `tests/blargg_spc.rs`
  (`screen_text` widened to the full 32×32 nametable, `VERDICT_FRAMES` raised to 12000). The quirk
  fires only deep in the Envelope suite, so no ROM's 120-frame boot hash moves (baseline TSV
  unchanged). undisbeliever golden stays 29/29, SPC700 oracle **0-diff**; `#![no_std]` +
  `forbid(unsafe_code)` preserved. See `docs/apu.md` §DSP GAIN mode-7 threshold.
- **SPC700 timer clocking phase (blargg `spc_*` literal PASS, T-31-006):** `RecordingSmpBus::write`
  — the bus the integrated machine drives through `Apu::advance_smp_cycle` — applied the write side
  effect (`$F0` global-enable / `$F1` enable / `$FA-$FC` target / the store) **before** advancing the
  SMP timebase and clocking the three timers. ares (`SMP::step`) and Mesen2 (`Spc::Write` →
  `IncCycleCount` first) clock the timers **before** the store, and our own per-instruction
  `SmpBus::write` already did so — but the recording bus was reversed, shifting the timer phase by
  **one access** on every timer-register write (e.g. arming `target` was observed *before* the
  arming cycle's own clock instead of after, so `TnOUT` lagged hardware by an off-by-one in the stage
  accumulation). Reordered `record()` (timebase + timer clock) to run first, then the store + IO
  decode (the deferred SMP→CPU port latch still rides that access's micro-op, so the CPU↔SMP
  handshake timing is unchanged). With the phase corrected, **`spc_smp`, `spc_timer`, and
  `spc_mem_access_times` reach blargg's literal `PASSED TESTS`** — `tests/blargg_spc.rs` now
  **asserts** the literal PASS (no longer determinism-only reporting); their re-blessed baselines are
  in `tests/golden/blargg-spc.tsv`. `spc_dsp6` is **unchanged** by the fix (its observable state is
  byte-identical) and still reports **Failed 02** on a separate S-DSP echo/envelope residual,
  reported honestly. This supersedes the earlier "literal PASS pending a CPU↔SMP bus-master
  inversion" conclusion above — the residual was the recording-bus write phase, not a clock-model
  asymmetry. SPC700 oracle stays **0-diff** (it replays against a flat, timer-less bus);
  `#![no_std]` + `forbid(unsafe_code)` preserved. See `docs/apu.md` §timer phase.
- `MVN`/`MVP` (block move): address now uses the full 16-bit `X`/`Y` regardless of index width
  and the increment respects the `X` flag (8-bit keeps the high byte); the `A.w` loop test is a
  post-decrement (ares `instructionBlockMove`). The oracle harness re-steps these looping
  instructions to the recorded cycle budget.
- `JSL`/`RTL`: stack access uses the full-16-bit `S` "new" push/pull (`pushN`/`pullN`) so it no
  longer corrupts `S` on an emulation-mode page wrap; the page-1 confinement is re-applied at
  the instruction boundary (ares `CallLong`/`instructionReturnLong`).
