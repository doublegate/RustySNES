# RustySNES — Version Plan (v0.1.0 to v1.0.0, and beyond)

This document is the release-cut map: it takes `to-dos/ROADMAP.md`'s phase spine and sequences
it into concrete, named, tagged releases — matching the depth and process RustyNES used to reach
its own v1.0.0. It replaces an earlier version of this document that described a v0.2.0-v0.9.0
skeleton whose scope had, in practice, already shipped inside the still-untagged `v0.1.0` before
any of those tags were ever cut — see "Why this document was rewritten" below.

## Why this document was rewritten

As of this rewrite, **RustySNES has never cut a single GitHub release.** `git tag -l` returns
nothing. Every subsystem that exists today — the 65C816 CPU (0-diff oracle), the PPU/scheduler,
SPC700+S-DSP audio (0-diff), eight validated coprocessors (DSP-1, DSP-2, DSP-4, ST010, Super FX,
SA-1, CX4, OBC1, S-DD1), and a playable native+wasm frontend — has accumulated inside one
perpetual `CHANGELOG.md` `[Unreleased]` section. The earlier draft of this plan assumed a linear
v0.2.0 (audio) → v0.3.0 (NEC DSP) → ... → v0.9.0 (Reach) progression; reality moved faster than
that draft and diverged from its ordering, so the plan is reset here against what's actually
built (`docs/STATUS.md` is the ground truth this ladder is checked against at every rung).

## Versioning rule

- **`v0.x.0`** (minor) = new scope — a phase-spine chunk or a themed feature set. Additive,
  default-off where the codebase's existing convention requires it (`docs/STATUS.md`'s version
  policy note).
- **`v0.x.y`** (patch) = same-minor bugfixes / accuracy fixes / dependency bumps only. No new
  scope. Cut as needed after any `v0.x.0`, at any point before the next minor.
- **`v1.0.0`** and beyond follow the same rule at the next digit.
- A `MAJOR` bump (`v2.0.0`) is reserved for a public-API or save-state-format break — the
  fractional-timebase refactor (`docs/adr/0002`) is the only currently-anticipated candidate,
  and only *if* Phase 6's accuracy triage concludes it's warranted.
- **Every tag is annotated, and the annotation IS the release note** — dense technical prose
  grouped by area (CPU / PPU / APU / cart / frontend / etc.), closing with an explicit
  accuracy-regression statement ("oracle/golden suites: N/N held"). This mirrors both
  RustyNES's actual practice and this project's own `CHANGELOG.md` voice already in use — the
  writing bar doesn't need to move, only the habit of actually cutting the tag.
- Each release is **named** (RustyNES: "Curator", "Bedrock", "Fidelity", ...) — pick a name that
  captures the release's one coherent theme.

## The ladder

### v0.1.0 "Foundation" — cut immediately, retroactively

Not new work. Closes the process gap described above.

1. Reorganize `CHANGELOG.md`'s current `[Unreleased]` content into a real `## [0.1.0] - <date>`
   section, grouped by area: CPU oracle (Phase 1), scheduler/PPU (Phase 2), audio (Phase 3),
   coprocessors Core+BestEffort tier (Phase 4/7 — DSP-1/2/4, ST010, Super FX, SA-1, CX4, OBC1,
   S-DD1), frontend (Phase 5's playable shell).
2. Tag `v0.1.0` (annotated, the CHANGELOG section content as the tag body) and cut the GitHub
   release. This is the template every later tag repeats.

**Known, honestly-carried gap going into v0.1.0:** SPC7110 is implemented but does not boot to
real content; ST018 and standalone S-RTC are not started; save-states/rewind/run-ahead don't
exist; `rustysnes-netplay`/`rustysnes-cheevos`/`rustysnes-script` are empty stubs. None of this
blocks v0.1.0 — it's exactly the honest "not started" status `docs/STATUS.md` already carries,
and v0.1.0's job is only to stop hiding the substantial *finished* work behind an unreleased tag.

### v0.2.0 "Persistence" — save-states — **RELEASED 2026-07-02**

**Goal:** the prerequisite for rewind, run-ahead, netplay, and TAS movies — all four build on
this. See `to-dos/phase-5-frontend/sprint-2-save-states.md` for the ticket breakdown.

- [x] A versioned, deterministic snapshot format across `Bus` (WRAM, PPU, APU/ARAM, DMA, clock),
      `Cart`/`Board` (every coprocessor's register state), and the CPU registers.
- [x] `rustysnes-core::System::save_state()`/`load_state()` — a 4-byte magic + `u16` format
      version envelope, replacing the `Unsupported` stubs `ref-proj/RUSTYMU-INTEGRATION.md`
      documented.
- [x] A round-trip determinism test: save → restore onto a fresh `System` → run N frames on
      both the original (continuing) and the restored system → byte-identical framebuffer +
      audio, across a no-coprocessor / `Curated` / `BestEffort` sample (extends the existing
      determinism-contract test pattern, `docs/adr/0004`).
- [x] New ADR: `docs/adr/0006-save-state-format.md` (the save-state binary format +
      versioning/compatibility policy), status `Accepted`.

### v0.3.0 "Continuum" — rewind, run-ahead, PAL, ExLoROM

- [x] Rewind: `crate::rewind::RewindBuffer` — a host-owned ring buffer of FULL save-state
      snapshots (frontend crate, not core — matches the existing architectural boundary that
      rate control lives in the frontend, `docs/adr/0004`), recorded every `interval_frames`
      real frames, capacity-bounded, oldest evicted first. Simpler than the original "keyframes +
      deltas" sketch in `docs/frontend.md`; delta-compression is a future memory optimization,
      not a correctness requirement. Wired into `app.rs`'s frame-drive loop + an Emulation →
      Rewind menu item; config-driven and off by default (`capacity: 0`).
- [x] Run-ahead: `crate::rewind::step_with_run_ahead` — N-frame resimulate-and-discard using the
      same save-state primitive: peeks N frames ahead for the presented video, rolls back, then
      re-runs exactly one real frame so persisted state (and audio, the continuous stream) only
      ever advances by one frame per call. Wired into `app.rs`'s frame-drive loop; config-driven
      and off by default (`frames: 0`). Both proven by `rewind.rs` tests that hand-assemble a
      tiny 65C816 program (NMI-driven WRAM counter → CGRAM write) for a real, observable
      per-frame state signal — not a synthetic fingerprint.
- [x] PAL region auto-detection: `Bus::sync_region_from_cart` reads the cart header's
      destination-code byte at `System::reset()` and reconfigures the PPU's line count/status bit
      (the 50 Hz/312-line table already existed in the scheduler per `docs/scheduler.md`; nothing
      previously wired the header's own PAL detection into the running machine). Proven
      end-to-end by `rustysnes-core::scheduler`'s `pal_cart_auto_detects_pal_region_on_reset`
      (a full 312-line frame actually completes, not just the region flag flipping).
      **Still open:** real-ROM-boot + golden-framebuffer validation against an actual PAL
      cartridge — no PAL ROM exists in the local test corpus yet, so this is honestly tracked as
      remaining, not silently claimed done.
- [x] ExLoROM memory-map model: `MapMode::ExLoRom` + the `ExLoRom` board (`board.rs`), header
      detection at `$40_7FC0` (`header.rs`). The decode formula is sourced directly from
      bsnes's runtime board database (`board: EXLOROM`/`EXLOROM-RAM`,
      `target-bsnes/resource/system/boards.bml`), not guessed from the header-detection
      heuristic alone — see `docs/cart.md` §ExLoROM for the full provenance chain. **Still
      open:** no real ExLoROM ROM (commercial or homebrew) exists in the local corpus, so this
      board has only formula-level unit-test coverage, not golden-framebuffer validation.

### v0.4.0 "Completion" — finish the coprocessor/board matrix

Closes Phase 7's exit criterion ("the full coprocessor/board matrix in `docs/STATUS.md`").

- **SPC7110 fully validated.** Resume from `coprocessor-phase7-status.md` (session memory): the
  CPU currently runs into unmapped memory ~20-30 frames into boot on Far East of Eden Zero
  regardless of PROM/DROM split tried. Add a 65816 disassembler to `rustysnes-cpu` (useful
  beyond this one bug) to get a real trace at the crash point; pursue the SPC7110 "Check
  Program" factory-diagnostic test ROM as a better oracle than a full commercial boot.
- **ST018** (ARMv3 LLE) — the one BestEffort coprocessor with no work started; a new ARM core
  following the clean-room-port pattern already used for `hg51b` (CX4) / `upd77c25` (DSP).
- **Standalone S-RTC** — `coproc::epsonrtc::EpsonRtc` already exists (built for SPC7110's
  paired RTC); wire it as its own board for S-RTC-only carts.

### v0.5.0 "Fidelity" — the accuracy push

**Goal:** build the accuracy-pass-rate dashboard RustySNES currently lacks (RustyNES's
AccuracyCoin-equivalent). See `to-dos/phase-6-accuracy-to-100/`.

- A named hardware-gotcha regression suite, each a targeted test: DRAM refresh (40 clocks/
  scanline — confirm modeled, not just documented), HDMA mid-scanline placement, the DMA/HDMA-
  collision crash quirk, open-bus-via-HDMA-latch (the "Speedy Gonzales stage 6-1" case), true
  mid-scanline/mid-dot writes (the "Air Strike Patrol BG3 scroll" case — this un-defers Phase
  2's flagged "mid-line raster deferred" gap), hi-res color-math precision (Bishoujo Janshi
  Suchie-Pai / Marvelous+SA-1), and the 65816 `$4203` double-write multiplier edge case.
- Track the composed accuracy battery's pass rate as a literal, always-current dashboard number
  in `docs/STATUS.md`, cited in every release from here on — the same treatment RustyNES gives
  "AccuracyCoin 139/139."
- Pursue the Nintendo Aging/Controller/SNES Test Program ROMs if obtainable, as an independent
  oracle layer.
- Anything needing the fractional-timebase refactor (`docs/adr/0002`) is explicitly deferred
  and documented in the residual ledger, never point-fixed (that ADR's own rule).

### v0.6.0 "Shippable" — release engineering + doc parity

**Goal:** bring CI and docs up to RustyNES's depth — the part of "match RustyNES's level" that
isn't about emulation accuracy.

- Actually exercise `.github/workflows/release.yml` end-to-end (it exists but, with zero tags
  ever cut, has never really run): the multi-platform build matrix (linux-gnu/macOS/Windows),
  release archives, wasm→Pages deploy.
- Add a `cargo audit`/`cargo deny` CI gate and a single `lint` job running fmt+clippy+rustdoc
  all `-D warnings`, mirroring RustyNES's `ci.yml` structure (RustySNES's workspace lints are
  currently `warn`, not enforced as `-D warnings` at the attribute level).
- New docs: `docs/benchmarks.md` (a results doc recording actual measured numbers — distinct
  from the existing `docs/performance.md`, which is targets/rules), `docs/DOCUMENTATION_INDEX.md`.
- ADR backfill for cross-cutting decisions made along this ladder that don't yet have one
  (save-state format from v0.2.0, this versioning-process adoption itself) — grow past today's
  5 ADRs toward RustyNES's 27, documenting decisions as they're made rather than retroactively.
- `to-dos/ROADMAP.md` and this file updated to reflect the ladder's actual progress at every
  release, not left stale (the mistake this rewrite is fixing).

### v1.0.0 — production cut

**Gate — deliberately not feature-count completeness** (matching RustyNES's actual v1.0 gate,
not RustySNES's earlier v0.9.0-skeleton draft which conflated breadth with 1.0):

1. The accuracy battery holds its v0.5.0-established target with zero regressions.
2. The save-state/core API (v0.2.0) is **stable** — the contract every post-1.0 Reach feature
   below relies on not breaking.
3. A genuinely shippable multi-platform app: native binaries + a wasm demo, both produced by
   the now-proven `v0.6.0` release pipeline.
4. Green CI including the `no_std` gate and the wasm build.
5. README / CHANGELOG / `docs/` / `docs/STATUS.md` fully in sync.

Phase 8 (netplay, RetroAchievements, TAS, scripting, shaders) is explicitly **not** part of
this gate — it ships after, as named minors, exactly as RustyNES deferred its own post-1.0
breadth.

## Post-v1.0 — Reach (Phase 8, themed minors, additive/default-off)

Each is one coherent theme, always reaffirms the accuracy gate never regressed (the RustyNES
pattern). Ordered so each unlocks the next: scripting/debugger first (smallest closed-scope
lift, most useful for accuracy work already done); netplay/TAS next (share the save-state +
determinism foundation); RetroAchievements/shaders/cheats/Libretro last (polish, least
accuracy-adjacent).

- **v1.1.0** — Lua scripting (`rustysnes-script`, currently a 1-line stub) + a first debugger
  pass (breakpoints, memory viewer — Mesen2-class tooling, the ambition already on record).
- **v1.2.0** — Rollback netplay (`rustysnes-netplay`, currently a 1-line stub) — built directly
  on v0.2.0's save-state format and the now-stable determinism contract.
- **v1.3.0** — RetroAchievements (`rustysnes-cheevos`, currently a 1-line stub) — wrap the
  `rcheevos` C library: a `read_memory` callback + `rc_client_do_frame()` called every frame,
  including fast-forwarded ones.
- **v1.4.0** — TAS movie recording/playback (deterministic input log + save-state-at-frame-0,
  same foundation as netplay).
- **v1.5.0+** — shader/filter pipeline (CRT/HQ2x), cheat-code support (Game Genie/Pro Action
  Replay SNES format), a Libretro core. A stretch tail, not committed scope up front. Unlike
  RustyNES's Android/iOS builds, **no mobile target is assumed** here unless there's real
  appetite once the desktop+wasm app is solid — don't inherit that scope by default.

## Standards adopted for every release from v0.1.0 onward

- **Commits:** Conventional Commits (the existing house rule) plus naming the concrete
  mechanism in the body, not just the feature — RustyNES's practice of citing the exact
  scheduler ratio / exact tap count / exact register formula, referencing the `T-PS-NNN` ticket.
- **Release notes:** the annotated tag body IS the release note, written at the same technical
  depth this project's `CHANGELOG.md` entries already use.
- **Continuous research:** as each rung above is executed, re-consult `ref-proj/{ares,bsnes,
  Mesen2}` and do targeted external research for that rung's specific hardware behavior or
  integration pattern — the sourcing in this document (test-ROM names, hardware gotchas, the
  `rcheevos` pattern) is a starting point to re-verify against current sources at
  implementation time, not a final citation.
