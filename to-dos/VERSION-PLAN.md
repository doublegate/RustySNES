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

### v0.3.0 "Continuum" — rewind, run-ahead, PAL, ExLoROM — **RELEASED 2026-07-08**

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

### v0.4.0 "Completion" — finish the coprocessor/board matrix — **RELEASED 2026-07-08**

All three line items landed: standalone S-RTC and ST018 fully implemented; SPC7110's addressing
bug found and fixed (the boot-crash gap that remains is honestly tracked, not a blocker — see the
SPC7110 entry below and `docs/cart.md` §SPC7110).

Closes Phase 7's exit criterion ("the full coprocessor/board matrix in `docs/STATUS.md`").

- [x] **Standalone S-RTC.** `coproc::sharprtc::SharpRtcBoard` — a standalone Sharp S-RTC
      (Daikaijuu Monogatari II, ExHiROM), distinct chip/protocol from SPC7110's paired Epson
      RTC-4513. Unit-tested; no commercial dump in the local corpus, so not golden-framebuffer
      validated (`docs/adr/0003`).
- [~] **SPC7110 — real progress, not yet fully validated.** Found + fixed a genuine addressing
      bug: `datarom_read`/`mcurom_read` used a plain `offset % len` fold where real hardware
      (ares `Bus::mirror`) uses a block-mirror algorithm that only coincides with modulo when
      the buffer size is a power of two — Far East of Eden Zero's 6 MiB DROM is not. This moved
      the wild-PC excursion from ~20-30 frames into boot to ~90+ frames (now a self-recovering
      BRK/RTI loop, not a permanent crash). Root cause of the REMAINING failure is narrowed —
      the CPU eventually `RTI`s from genuine PROM code into a WRAM address confirmed entirely
      unpopulated — but not fixed; needs a proper disassembler + symbol trace, out of scope for
      this pass. See `docs/cart.md` §SPC7110 / `docs/STATUS.md` for the full diagnostic trail.
- [x] **ST018** (ARMv3 LLE) — a full ARMv3 (ARM6-class) CPU core, clean-room ported from Mesen2's
      `ArmV3Cpu` (barrel shifter/ALU, mode-banked register file, 3-stage pipeline, the complete
      instruction set) + `St018Board` (firmware loading, the `$3800`/`$3802`/`$3804` handshake,
      driven by `Board::coprocessor_tick` rather than the SA-1 second-CPU hooks since this core
      is self-contained in `rustysnes-cart`). Detected via title match on the confirmed real
      cart, Hayazashi Nidan Morita Shogi 2 (an earlier investigation wrongly assumed Star Ocean).
      No commercial dump in the local corpus — unit-test-level coverage only.

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
  its own AccuracyCoin score (currently `139/141` shipped-default, `141/141` behind a default-off
  feature flag per its own "bake, then promote" gate — worth mirroring that exact pattern:
  ship honest partial coverage default-on, land experimental fixes default-off until proven
  against a broad commercial-ROM byte-identity oracle, then promote in a dedicated point release).
- Pursue the Nintendo Aging/Controller/SNES Test Program ROMs if obtainable, as an independent
  oracle layer.
- Anything needing the fractional-timebase refactor (`docs/adr/0002`) is explicitly deferred
  and documented in the residual ledger, never point-fixed (that ADR's own rule).

### v0.6.0 "Shippable" — release engineering + doc parity

**Goal:** bring CI and docs up to RustyNES's depth — the part of "match RustyNES's level" that
isn't about emulation accuracy.

- [x] Actually exercise `.github/workflows/release.yml` end-to-end: the multi-platform build
      matrix (linux-gnu/macOS/Windows) now packages each platform's binary + README/LICENSE into
      a `tar.gz`/`zip` archive and attaches it to the tag's GitHub release (self-healing —
      creates a minimal release first if the agent-authored `gh release create` ceremony step
      hasn't run yet). Landed retroactively on `v0.1.0`/`v0.2.0`/`v0.3.0` (backfilled after the
      fact, since none of the first three tags had attached artifacts) ahead of this rung, since
      it was a real user-facing gap, not deferred work. wasm→Pages deploy (`pages.yml`) was
      already exercised on every `main` push since `v0.1.0`.
- Add checksummed assets (SHA-256) to the release archives — the current packaging step doesn't
  emit them yet (deferred here, not urgent enough to block anything).
- Add a dedicated `security.yml` (`cargo audit` + `cargo deny`, on a schedule + every PR touching
  `Cargo.lock`) and a single `lint` job running fmt+clippy+rustdoc all `-D warnings` as one gate,
  mirroring RustyNES's 8-workflow `.github/workflows/` structure (`ci.yml`, `security.yml`,
  `release.yml`, `release-auto.yml`, `web.yml`, plus its mobile/PGO workflows this project has no
  mobile-target reason to copy) — RustySNES currently has 3 (`ci.yml`, `pages.yml`, `release.yml`)
  and workspace lints are `warn`, not enforced as `-D warnings` at the attribute level (CI's own
  `-- -D warnings` command-line flag is the actual gate today; keep that, add the dedicated job).
- New docs: `docs/benchmarks.md` (a results doc recording actual measured numbers — distinct
  from the existing `docs/performance.md`, which is targets/rules), `docs/DOCUMENTATION_INDEX.md`,
  a `docs/audit/` directory for dense investigation write-ups (RustyNES's pattern for campaigns
  like the SPC7110 boot-crash trace this project still owes, `docs/cart.md` §SPC7110).
- ADR backfill for cross-cutting decisions made along this ladder that don't yet have one. Save-
  state format is already covered (`docs/adr/0006`); still missing: the versioning/release-
  process adoption itself (this document + the tag-body-is-the-release-note convention), the
  ExLoROM decode-formula sourcing decision (`docs/cart.md` §ExLoROM), and ST018's detection
  method + `Board::coprocessor_tick`-not-second-CPU-hooks architectural choice
  (`docs/st018-arm-notes.md`) — grow past today's 6 ADRs toward RustyNES's 30, documenting
  decisions as they're made rather than retroactively.
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
