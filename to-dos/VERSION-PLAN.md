# RustySNES — Version Plan (v0.1.0 to v1.0.0, and beyond)

This document is the release-cut map: it takes `to-dos/ROADMAP.md`'s phase spine and sequences
it into concrete, named, tagged releases — matching the depth and process RustyNES used to reach
its own v1.0.0. It replaces an earlier version of this document that described a v0.2.0-v0.8.0
skeleton whose scope had, in practice, already shipped inside the still-untagged `v0.1.0` before
any of those tags were ever cut — see "Why this document was rewritten" below.

## Why this document was rewritten

As of this rewrite, **RustySNES has never cut a single GitHub release.** `git tag -l` returns
nothing. Every subsystem that exists today — the 65C816 CPU (0-diff oracle), the PPU/scheduler,
SPC700+S-DSP audio (0-diff), eight validated coprocessors (DSP-1, DSP-2, DSP-4, ST010, Super FX,
SA-1, CX4, OBC1, S-DD1), and a playable native+wasm frontend — has accumulated inside one
perpetual `CHANGELOG.md` `[Unreleased]` section. The earlier draft of this plan assumed a linear
v0.2.0 (audio) → v0.3.0 (NEC DSP) → ... → v0.8.0 (Reach) progression; reality moved faster than
that draft and diverged from its ordering, so the plan is reset here against what's actually
built (`docs/STATUS.md` is the ground truth this ladder is checked against at every rung).

**Second reversal (this update, post-`v0.6.0`):** the ladder that shipped `v0.1.0`-`v0.6.0`
treated `v1.0.0` as an accuracy + stability gate with Phase 8 breadth (netplay, RetroAchievements,
TAS, scripting, a debugger, cheats) deferred to named post-1.0 minors — itself a deliberate
correction away from an even earlier draft that had folded that breadth into the 1.0 gate. This
update reverses course a second time, per explicit direction: RustyNES's own v1.0.0 front-loaded
nearly all of that breadth rather than deferring it, and matching that bar means RustySNES's
`v0.7.0`→`v1.0.0` span needs to do the same. `to-dos/ROADMAP.md` and
`to-dos/phase-8-reach/overview.md` are being rewritten in the same change so all three planning
documents, plus `CHANGELOG.md` and `docs/STATUS.md`, agree on one consistent story rather than
three.

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

### v0.5.0 "Fidelity" — the accuracy push — **RELEASED 2026-07-08**

**Goal:** build the accuracy-pass-rate dashboard RustySNES currently lacks (RustyNES's
AccuracyCoin-equivalent). See `to-dos/phase-6-accuracy-to-100/`.

- A named hardware-gotcha regression suite, each a targeted test: DRAM refresh (40 clocks/
  scanline — **researched, not yet implemented**, `docs/scheduler.md` §DRAM refresh — a real
  architectural tension needs resolving empirically before landing, not a simple port). **Already
  implemented, docs were stale (fixed this pass):** HDMA mid-scanline placement — `Bus::advance_master`
  already fires HDMA's per-line run at the hardware-correct dot 276 (`HDMA_RUN_DOT`, hcounter 1104,
  not the scanline boundary, per ares `sfc/cpu/timing.cpp`), proven by the committed
  `hdmaen_latch_test`/`hdmaen_latch_test_2` goldens (`docs/scheduler.md` §DMA/HDMA bus-steal); the
  "Implementation status" section further down the same doc had drifted out of sync with this and
  wrongly still described it as a scanline-boundary trigger + deferred work — corrected.
  **Researched, prototyped, regressed, deferred:** open-bus-via-HDMA-latch (the "Speedy Gonzales
  stage 6-1" case) — the mechanism is confirmed against two independent primary sources (SNESdev
  wiki + byuu/Near's original mGBA writeup): `Bus::open_bus` is currently updated only by direct
  CPU accesses, never by a DMA/HDMA-driven byte, which is wrong versus real hardware. A prototype
  making `DmaBus for Bus`'s `read_a`/`write_a`/`read_b`/`write_b` update `open_bus` on every
  transfer (mirroring `CpuBus::read24`/`write24`) passed the full base workspace suite but broke
  all 24 `superfx_boots_live_and_deterministic` golden hashes (confirmed caused by this exact
  change, not pre-existing) for a reason not yet root-caused — very likely something in how
  Super FX/GSU games' GP-DMA ROM→GSU-RAM transfers interact with the change. Not landed; see
  `docs/scheduler.md` §Open bus via DMA/HDMA for the full mechanism and what a future
  investigation needs (an access-level trace, mirroring `docs/audit/spc7110-boot-crash-2026-07-08.md`'s
  approach for the SPC7110 gap). **Designed, prototyped, SA-1-verified, BLOCKED (not landed):**
  true mid-scanline/mid-dot writes (the "Air Strike Patrol BG3 scroll" case) — confirmed against
  ares' per-pixel reference model (`ppu/main.cpp`'s `cycleRenderPixel()` only runs for hcounter
  `[56, 1078]`, strictly *before* HDMA's per-line run at hcounter 1104) that RustySNES's
  end-of-line compositor has a genuine, systemic off-by-one-line bug: an HDMA-driven per-line
  register write during line `V` is meant to take effect starting line `V+1`, but the compositor
  (reading live register state at dot 340, after line `V`'s own HDMA already ran) applies it to
  line `V` itself. A prototype fix (`rustysnes-ppu` gains a public `RENDER_DOT` constant, `= 276`,
  the PPU's own video-timing fact that `rustysnes-core`'s `HDMA_RUN_DOT` is defined equal to;
  `Ppu::tick_dot` composites each line at `RENDER_DOT` instead of dot 340 — the same dot number
  HDMA's own per-line run fires at, but sequenced strictly before it within that master-clock
  tick's execution order, since the HDMA-service check runs after the PPU-dot call returns — no
  DMA/HDMA knowledge leaked into the PPU crate) is independently verified CORRECT for the
  CPU/HDMA-driven case: SA-1's `SD F-1 Grand Prix` golden hash change was confirmed a real
  accuracy improvement by diffing pre-/post-fix framebuffers row-by-row — 159/239 rows differed,
  and testing those against the fix's predicted "shifted one line later" signature matched
  232/237 checkable rows (97.9%; 237 = 239 minus the 2 boundary rows a one-line-shift comparison
  can't reach) with zero unexplained outliers. **But the same change breaks all 24 Super FX/GSU
  golden tests** with a
  diff pattern that does NOT fit that same mechanism (a color bar shifted 4 rows in the *opposite*
  direction on one ROM; 7 genuine outliers on another) — the identical failure signature the
  sibling open-bus-via-HDMA-latch investigation (above) also hit and correctly did not land.
  Working hypothesis: the GSU coprocessor's host-synced VRAM writes (`Board::coprocessor_tick`,
  stepped from the same `advance_master` loop the PPU dot-tick runs from) are sampled at a
  different point in their own progress once the render trigger moves from dot 340 to dot 276 —
  not confirmed, needs an access-level trace. Not landed; full mechanism, both verifications, and
  what a future investigation needs in `docs/ppu.md` §Mid-scanline/HDMA-driven register timing.
  **Researched,
  deferred (blocked on a larger feature, not a precision nuance):** hi-res color-math precision
  (Bishoujo Janshi Suchie-Pai / Marvelous+SA-1) — confirmed against ares' `DAC::run()` that hi-res
  is a dual-half-pixel output trick (alternating `above`/`below` compositor results at 2× the
  pixel rate), which RustySNES cannot model at all yet since Modes 5/6 don't emit 512-wide output
  in the first place (a real feature gap, not a numeric-precision tweak) — full mechanism in
  `docs/ppu.md` §Hi-res color-math precision. **Researched and
  reclassified:** the 65816 `$4203`/`$4206` overlapping-multiply/divide case (SNESdev's own
  errata documents this as producing genuinely *undefined* RDMPY/RDDIV output — no canonical
  "corrupted" value exists to port, and inventing one would violate `docs/adr/0004`'s
  determinism-contract spirit of not fabricating behavior real hardware itself doesn't define
  one way). This is correctly a **documented, intentional non-goal**, not an open implementation
  item — `crates/rustysnes-core/src/bus.rs`'s `MulDiv` doc comment cites the errata directly.
  **Also researched and reclassified:** the "DMA/HDMA-collision crash quirk" — the SNESdev errata
  page's DMA section actually bundles three distinct behaviors under that vague umbrella: a
  version-1-5A22-only crash and a version-2-5A22-only silent-DMA-failure bug (both chip-revision
  defects compliant commercial ROMs are written to avoid, not reproduced as a crash by any
  mainstream reference emulator), plus a version-agnostic silent whole-frame HDMA failure that IS
  well-defined but has no known commercial title or committed test ROM depending on it either way
  — no oracle exists to verify an implementation against, and the sibling open-bus investigation
  (above) just demonstrated this exact class of change carries real regression risk even when the
  mechanism is correct. A fourth item on the same errata list (A-bus address restrictions) is
  already correctly implemented (`DmaBus for Bus`'s blocked-address branches,
  `crates/rustysnes-core/src/bus.rs`), as is the general "HDMA preempts GP-DMA" priority ordering
  (`run_gp`'s `service_hdma_during_gp`, `crates/rustysnes-core/src/dma.rs`) — the well-defined
  half of what "collision" could have meant was never actually a gap. Full citation and
  per-sub-case reasoning in `docs/scheduler.md` §The "DMA/HDMA-collision crash quirk".
- Track the composed accuracy battery's pass rate as a literal, always-current dashboard number
  in `docs/STATUS.md`, cited in every release from here on — the same treatment RustyNES gives
  its own AccuracyCoin score (currently `139/141` shipped-default, `141/141` behind a default-off
  feature flag per its own "bake, then promote" gate — worth mirroring that exact pattern:
  ship honest partial coverage default-on, land experimental fixes default-off until proven
  against a broad commercial-ROM byte-identity oracle, then promote in a dedicated point release).
- **Researched: obtainable, but reclassified as a `commercial-roms`-gated stretch goal, not
  pursued this release.** The Nintendo Aging/Controller/SNES Test Program ROMs are real and
  dumped — *Super Famicom Aging Program Ver. 1.00* and *Controller Test Program (Japan)* are
  both individually preserved on the Internet Archive, and a further factory/QA test-program
  archive (from tukuyomi's now-offline SNES emulation site) is mirrored at SNES Central; the
  *NTF 2.5 Test Cartridge* (Nintendo World Class Service's own diagnostic, per The Cutting Room
  Floor) is a third, separate artifact. All are Nintendo's own copyrighted internal software —
  same legal status as the commercial ROMs this project already validates coprocessors against
  (`docs/STATUS.md`'s BestEffort real-title-validated rows), so integrating one would follow the
  exact same pattern: local-only under `tests/roms/external/` (gitignored), gated behind
  `--features commercial-roms`, never committed. Checked whether RustyNES pursued an NES
  equivalent (an "Aging"/factory-diagnostic cartridge, as opposed to its actual AccuracyCoin
  approach — one comprehensive third-party homebrew ROM) as precedent for this being worth the
  effort: it did not — no reference to an NES factory/aging diagnostic cartridge anywhere in
  RustyNES's docs, CHANGELOG, or to-dos. This item's original phrasing assumed a RustyNES
  precedent that doesn't actually exist. Deferred to a later release as a stretch goal (would add
  a genuinely independent, Nintendo-authored oracle layer beyond this project's existing
  third-party homebrew suites), not pursued in `v0.5.0`.
- Anything needing the fractional-timebase refactor (`docs/adr/0002`) is explicitly deferred
  and documented in the residual ledger, never point-fixed (that ADR's own rule).

### v0.6.0 "Shippable" — release engineering + doc parity — **RELEASED 2026-07-08**

**Goal:** bring CI and docs up to RustyNES's depth — the part of "match RustyNES's level" that
isn't about emulation accuracy.

- [x] Actually exercise `.github/workflows/release.yml` end-to-end: the multi-platform build
      matrix (linux-gnu/macOS/Windows) now packages each platform's binary + README/LICENSE into
      a `tar.gz`/`zip` archive and attaches it to the tag's GitHub release (self-healing —
      creates a minimal release first if the agent-authored `gh release create` ceremony step
      hasn't run yet). Landed retroactively on `v0.1.0`/`v0.2.0`/`v0.3.0` (backfilled after the
      fact, since none of the first three tags had attached artifacts) ahead of this rung, since
      it was a real user-facing gap, not deferred work. wasm→Pages deploy (`pages.yml`) has been
      exercised on every `main` push since `v0.1.0`; verified live (not just "CI job green") at
      `https://doublegate.github.io/RustySNES/` — the trunk-built `index.html`, wasm-bindgen JS
      loader, and `.wasm` binary all return `200` with correct content-types, and the co-deployed
      rustdoc site (`/api/`) is reachable too. Both halves of `v1.0.0`'s gate item 3 ("native
      binaries + a wasm demo") are now confirmed genuinely live, not merely assumed from a
      passing workflow run.
- [x] Checksummed assets (SHA-256): `release.yml` gained a `Checksum` step (portable across the
      three runner shells — tries `sha256sum`, falls back to `shasum -a 256`, since neither tool
      alone is guaranteed present on every one of Linux/macOS/Windows) that emits a detached
      `<archive>.sha256` alongside each platform archive; the upload step now attaches both.
      Not yet exercised end-to-end against a real tag (next tag push / `workflow_dispatch`
      backfill will be the first live proof, mirroring how the artifact-attachment fix itself
      was only proven real by `v0.4.0`).
- [x] `security.yml`: `cargo audit` + `cargo deny check` jobs, gated on `main`/PR pushes
      touching non-doc paths + a Monday 00:00 UTC `schedule` + `workflow_dispatch`, mirroring
      RustyNES's structure. `deny.toml` was built from RustySNES's own `cargo deny list` output
      (not copied from RustyNES's), independently confirming the identical winit/egui/wgpu
      dependency chain trips the same 3 RUSTSEC IDs RustyNES already documented
      (RUSTSEC-2026-0192 `ttf-parser` unmaintained; -0194/-0195 `quick-xml`, reachable only via
      `wayland-scanner`'s compile-time proc-macro parsing trusted vendored XML, never runtime
      input) — suppressed in `deny.toml` + `.cargo/audit.toml` with the full rationale, after
      explicit user review and approval. RustySNES now has 4 workflows (`ci.yml`, `pages.yml`,
      `release.yml`, `security.yml`) against RustyNES's 8 — the mobile/PGO workflows (no mobile
      target here) are the remaining, deliberately-not-mirrored gap.
- [x] The dedicated `lint`-job-with-rustdoc extension: `ci.yml`'s `lint` job (every PR/push to
      `main`) now runs `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` alongside
      fmt+clippy — cheap locally (~4s), so it belongs on every PR rather than being reserved for
      the tag-only `full-test` job, and catches broken intra-doc links / rustdoc-specific
      warnings clippy's own lints don't cover.
- [x] `docs/DOCUMENTATION_INDEX.md` — the full documentation map (subsystem specs, ADRs, testing
      strategy, external references), linked from the README, matching RustyNES's own index.
- [x] `docs/benchmarks.md` + a real Criterion benchmark
      (`crates/rustysnes-core/benches/headless_frame.rs`) — the first-ever measured number on
      this codebase: **3.27 ms/frame** steady state against `docs/performance.md`'s ≤~2ms target
      (real-time headroom is fine at ~5.1×, but the target itself isn't met yet — an honest
      baseline, not a claim of hitting it).
      Establishes the number every future optimization pass is measured against.
- [x] `docs/audit/` — a decision-rationale / open-investigation directory (RustyNES's pattern),
      seeded with the full SPC7110 boot-crash trail (`docs/audit/spc7110-boot-crash-2026-07-08.md`).
- [x] Automated release-cutting (`.github/workflows/release-auto.yml`): mirrors RustyNES's
      `release-auto.yml` pattern (fires on the `CI` workflow completing successfully on `main`,
      invokes `release.yml` via `workflow_call` since a bot-pushed tag doesn't trigger
      `on: push: tags`) adapted to this project's own conventions rather than copied — since
      RustySNES's crate `Cargo.toml` versions stay pinned at `0.1.0` (nothing here publishes to
      crates.io), the trigger signal is `CHANGELOG.md`'s own structure (an empty `[Unreleased]`
      immediately followed by a real `## [X.Y.Z] "Name" - date` heading means that version was
      just closed out and is ready to tag) rather than a Cargo.toml version bump, and it creates
      a real ANNOTATED tag (`git tag -a -F <notes>`) sourced directly from the CHANGELOG section
      (`docs/adr/0007`'s tag-body-is-the-release-note convention), not a separate
      maintainer-authored notes file. Idempotent (a no-op once the version's tag already exists).
      Directly closes the recurring manual-release-ceremony bottleneck this ladder's own v0.5.0
      cut ran into.
- [x] ADR backfill for cross-cutting decisions made along this ladder. Save-state format was
      already covered (`docs/adr/0006`); the three real gaps are now filled: `docs/adr/0007`
      (the versioning/release-process adoption itself — this document + the tag-body-is-the-
      release-note convention), `docs/adr/0008` (the ExLoROM decode-formula sourcing decision),
      and `docs/adr/0009` (ST018's title-match detection method + `Board::coprocessor_tick`-not-
      second-CPU-hooks architectural choice) — 9 ADRs now, growing toward RustyNES's 30 as new
      decisions land, documented as they're made rather than retroactively.
- `to-dos/ROADMAP.md` and this file updated to reflect the ladder's actual progress at every
  release, not left stale (the mistake this rewrite is fixing).

### v0.7.0 "Resolution" — true hi-res output — **RELEASED 2026-07-09**

The one item from `v0.5.0`'s carried-forward PPU gap list that's genuinely bounded, with no
external unknowns (no ROM sourcing, no open-ended root-causing): full 512-px hi-res (Modes 5/6)
output. Kept as its own minor rather than folded into the accuracy-debt cluster below, matching
this project's one-theme-per-`v0.x.0` convention every prior rung has held to.

- [x] **Mechanism, verified against `ref-proj/ares/ares/sfc/ppu/dac.cpp`'s primary source (not a
      paraphrase):** hi-res is a dual-column DAC output stage, not a numeric-precision tweak —
      each PPU pixel clock emits an "odd" column (today's unchanged main-screen color-math
      result) and an "even" column (the subscreen's own color, math'd with the operand roles
      swapped). The even column is gated by, and blends against, state from the **previous**
      pixel clock's above-pass — a genuine one-pixel-clock-delayed hardware pipeline stage, found
      by reading the primary source directly rather than trusting the earlier research summary
      that scoped this as a simple parallel extension. Full derivation in `docs/ppu.md` §Hi-res
      (Modes 5/6) color-math precision.
- [x] **Implementation:** `crates/rustysnes-ppu/src/lib.rs` (`MAX_SCREEN_WIDTH`, an
      always-hi-res-capacity backing framebuffer with a resolution-sized `framebuffer()` slice,
      `is_hires()`/`frame_hires()`/`visible_width()`) + `render.rs` (`compose_dac`'s new
      below-color pass, threading the one-column-delayed state). The non-hires path is
      byte-for-byte the pre-existing code, unchanged — a hi-res frame's output width is latched
      once per frame (at row 0) rather than re-checked per scanline, a deliberate, documented
      simplification (`docs/ppu.md`).
- [x] **Save-state `FORMAT_VERSION` 1→2** — the framebuffer's backing storage growing to hi-res
      capacity is a real byte-layout change to the `PPU0` section. The `v1.0.0` gate's
      previously-flagged backward-compat-fixture gap (a committed old-format blob + a regression
      test proving the mismatch fails loudly, not silently) closes here, ahead of schedule:
      `tests/golden/savestate-v1-gilyon.bin` (real `FORMAT_VERSION=1` output, captured from the
      pre-bump code) + `crates/rustysnes-test-harness/tests/save_state_backward_compat.rs`.
      `docs/adr/0006-save-state-format.md` also corrected an overclaim its versioning-policy
      paragraph had made (that minor bumps stay backward-loadable — not actually implemented).
- [x] **Frontend:** `crates/rustysnes-frontend/src/emu.rs`'s `render_framebuffer()` now queries
      `visible_width()` instead of a hardcoded 256; the wgpu texture/present pipeline needed no
      changes (`gfx.rs` already allocated at hi-res capacity with a live UV sub-rect scale).
- [x] **Verification:** two new unit tests hand-construct `Pixel` rows to isolate the mechanism
      precisely (the column-0 transparency boundary; the one-column delay itself, proven by
      varying only column 0's state and observing column 1's below-output change while its
      above-output and its own input stay fixed). The full `--features test-roms` suite —
      including `sa1_oncart` — passes unchanged, since no currently-passing golden ROM enters
      hi-res mode. **Real-title validation not achieved, honestly tracked as open:** Marvelous —
      Mouhitotsu no Takarajima (SA-1, the named local hi-res-motivating title) never entered
      hi-res in a 1200-frame headless run; Bishoujo Janshi Suchie-Pai (the other named title) has
      no local dump; an `ares` reference-screenshot comparison was attempted and abandoned — no
      working GUI display was available in this environment to drive it. `tests/golden/
      sa1-framebuffer.tsv` is **not** re-blessed (Marvelous's hash is unaffected).

### Ongoing, opportunistic — the carried-forward accuracy-debt cluster (v0.x.y patches, not a gating rung)

Open-ended research items, not bounded deliverables — several have already carried forward
across `v0.3.0`→`v0.6.0` without closing, so none of them gates a numbered `v0.x.0` rung above.
Land each independently as a patch release whenever its investigation actually concludes,
re-running the full `--features test-roms` suite before landing anything, per this project's
established discipline:

- ~~**Mid-scanline/HDMA-driven register timing + the Super FX/GSU regression**~~ **Landed,
  `v0.8.0`.** The blocking regression was root-caused to a SEPARATE bug (`Bus::advance_master`
  reading `self.ppu.dot()` after it had already been incremented, not the render-timing fix
  itself) — see `docs/ppu.md` §Mid-scanline/HDMA-driven register timing for the full mechanism,
  the SA-1 golden re-bless, and the row-level-verified 24-hash Super FX/GSU golden re-bless.
- **Open-bus-via-HDMA-latch** — still open; NOT resolved by the mid-scanline fix above (a
  different bug, confirmed by re-testing the original open-bus-via-DMA prototype against the
  now-fixed tree — it still hits the identical all-24-GSU-goldens regression). `docs/scheduler.md`
  §Open bus via DMA/HDMA has the full mechanism (the Speedy Gonzales stage 6-1 "Holy Grail" case)
  and what a future investigation needs: an access-level trace of GSU VRAM/CGRAM writes correlated
  against the failing DMA transfers, the same technique the SPC7110 item below now has tooling for.
- ~~**SPC7110 boot gap**~~ **Resolved, `v0.8.0` — was a ROM-identity issue, not an emulation bug.**
  The real 65C816 disassembler this item's own prerequisite called for landed (`rustysnes_cpu::
  disasm`, alongside T-81-001b's watchpoints); using it, the investigation found and fixed three
  more real bugs (bank `$40-$7D` wrongly mirrored, the DROM buffer 2 MiB oversized, a systemic
  cart-layer open-bus fallback bug) and confirmed the crash-point trace was itself framing the
  problem wrong (the CPU is mid-VRAM-upload-loop, not stalled). A full instruction trace then
  showed the path to the derailing `JSL $4FFB80` is one unconditional call chain with no branch
  and no SPC7110 register touched anywhere upstream — ruling out a wrong-branch bug and raising
  the real question: is this the right ROM? It is not. Three independent checks (a SHA256
  mismatch against `ref-proj/ares`'s database entry for this exact board; a header checksum that
  only validates against the file's non-standard 7 MiB size, not the real cartridge's 5 MiB; a
  public nesdev.org thread documenting this exact fan-translation's own memory map) confirm the
  local dump is the English translation patch, which adds a 1 MiB "Expansion ROM" at banks
  `$40-$4F` — precisely the bank the `JSL` targets — that exists only in the patch, never on real
  hardware. Full evidence chain: `docs/audit/spc7110-boot-crash-2026-07-08.md`. **Now a
  ROM-sourcing gap** (a genuine original-cartridge dump, sha256 `69d06a3f3a4f3ba769541fe94e92b421
  42e423e9f0924eab97865b2d826ec82d`, would very likely let it boot cleanly — none of the fixes
  above are fan-translation-specific), tracked in `docs/rom-test-corpus.md` alongside the other
  ROM-sourcing-blocked items below, not carried forward here as an open bug.
- **DRAM refresh (40 clocks/scanline)** — researched, not yet implemented; a real architectural
  timing tension needs resolving empirically before landing, not a simple port. See
  `docs/scheduler.md` §DRAM refresh for the exact empirical test to run before implementing.
- **ST018 / S-RTC / PAL / ExLoROM real-ROM validation** — currently unit-test-only or
  formula-level only, blocked purely on sourcing a commercial dump for each. `docs/rom-test-corpus.md`
  (added `v0.8.0`) is now the single source of truth for which ROM would close each gap and
  whether it's in the local corpus, the Dropbox ROMs locations, or unavailable entirely. Keep the
  opportunistic posture already established across four prior releases; don't gate a rung on
  finding ROMs that may never become available.

### v0.8.0 "Instrumentation" — debugger, scripting/TAS, cheats, the real wasm frontend

**Shipped together with "Community" below as one tag, `v0.8.0 "Community"` — RELEASED
2026-07-10.** Both sprints' scope, plus the mid-scanline/HDMA-driven register timing fix and a
continued SPC7110 investigation, landed and released as a single `v0.8.0` (`v0.9.0` was never
used anywhere on GitHub, so this picks up directly from `v0.7.0`; see `CHANGELOG.md`). This
section is kept as the original per-sprint planning record; the "Community" header below carries
the actual release note.

One coherent theme: tooling that inspects and manipulates emulator state, all sharing the same
memory-watch/introspection substrate — plus the wasm frontend build, folded in here per explicit
direction after the live demo page was found rendering blank (see below). This is the first rung
of the breadth pass — RustySNES's own crate manifest already anticipated the tooling half
(`crates/rustysnes-frontend/Cargo.toml`'s `debug-hooks`/`scripting` flags, present since the
frontend's first cut, "mirroring the RustyNES feature surface so the chip-implementation phase
can switch them on without restructuring the manifest").

- **Debugger overlay** — fills in `ui_shell.rs`'s already-wired 65C816/PPU/APU/Cart panels
  (currently `"TODO(impl-phase)"` placeholders) behind the existing `debug-hooks` flag; the
  shell itself (menu entry, window, panel selection) already exists, so this is instrumentation
  work, not a from-scratch UI build. Includes SA-1/Super FX coprocessor state in the Cart panel
  from day one — resolving `docs/frontend.md`'s open question the same direction this whole
  ladder does: breadth-inclusive, not a further-deferred add-on.
- **Scripting + TAS** — wires `rustysnes-script`'s full stated scope in one pass (its own
  `docs/STATUS.md` entry already describes it as "Lua scripting / TAS API," so both land
  together here rather than pairing TAS with netplay). Wires the existing `scripting` flag. TAS
  needs a deterministic input log format plus save-state-at-frame-0 seeding — both build
  directly on the existing `Bus::set_joypad`/save-state envelope, no new architectural work.
- **Cheat-code support** (Game Genie / Pro Action Replay SNES format) — has zero existing
  scaffold (no stub crate, no feature flag, unlike every other item on this ladder) and is
  fundamentally memory-watch/poke tooling, the same substrate as the debugger's memory panel —
  grouped here rather than with netplay/RetroAchievements for that reason. Adds a new `cheats`
  feature flag matching the existing naming convention.
- **The real wasm frontend** — `crates/rustysnes-frontend/src/wasm.rs` has been an explicitly-
  labeled scaffold stub since `v0.1.0` (installs a panic hook, logs one message, returns — never
  builds the `App`, creates a wgpu canvas surface, or drives a render loop), making the live
  `https://doublegate.github.io/RustySNES/` demo a blank page despite every prior "wasm demo is
  live" CHANGELOG claim (`v0.1.0`-`v0.6.0`) being true only at the HTTP level (200 status,
  correct content-types) — found live by direct comparison against RustyNES's own working wasm
  deployment, not by CI (`pages.yml` only asserts the build/deploy steps succeed, never that the
  resulting app actually renders). `rustysnes-frontend/Cargo.toml`'s `wasm-winit` (default) /
  `wasm-canvas` feature split already anticipated exactly the two-stage build RustyNES itself
  used to reach a working wasm frontend (confirmed by reading RustyNES's `wasm.rs`/`wasm_winit.rs`
  source directly, not inferred): a `wasm-canvas` MVP first (a `CanvasRenderingContext2d`
  `putImageData` blit of the RGBA8 framebuffer + a `requestAnimationFrame` loop + keyboard via DOM
  events + ROM load via `<input type="file">` — no `wgpu`/`egui`, ~500 lines in RustyNES's
  `wasm.rs`), then `wasm-winit` unification as a larger follow-up (routes wasm through the *same*
  `App`/`ApplicationHandler` native already uses, via
  `winit::platform::web::EventLoopExtWebSys::spawn_app` + an `EventLoopProxy` delivering
  `RomLoaded`/`GfxReady`-style events in from JS — RustyNES's own `app.rs` states "the `ApplicationHandler` impl serves both
  native and wasm32," proving this reuse is architecturally sound, not aspirational). **Real,
  confirmed gap, not just plumbing:** RustySNES's `app.rs` and `audio.rs` are currently
  `#[cfg(not(target_arch = "wasm32"))]` — entirely excluded from the wasm build today, not merely
  unused; `gfx.rs` has zero `wasm32`/`web_sys`/canvas references yet. Un-gating and adapting them
  (swapping `cpal` for Web Audio behind a conditional path, gating out native-only deps like
  `gilrs`/`directories`) is the same real work RustyNES needed for its own `wasm_winit.rs`
  follow-up, not a small addition. Minimal wasm-canvas-MVP subsystem list, RustyNES file
  (line count) → RustySNES port target: canvas-2D entry (`wasm.rs`, 538) → replaces the current
  stub; audio (`wasm_audio.rs`, 456) → AudioWorklet primary + ScriptProcessorNode fallback, no
  `SharedArrayBuffer` since GitHub Pages can't send COOP/COEP headers, reuses the same DRC/
  resampler logic already in native `audio.rs`; gamepad (`wasm_gamepad.rs`, 64) → trivial,
  `navigator.getGamepads()` polled from JS + one bridge fn; file I/O (`wasm_io.rs`, 345) → the
  `<input type="file">` + generic save-file-with-fallback helper. **Explicitly out of scope for
  this rung** (RustyNES's own Phase-8-equivalent reach features, needed only once RustySNES's
  native equivalents exist, not as a wasm-specific prerequisite): `wasm_idb.rs` (IndexedDB
  save-state persistence — native save-states already work, can defer), `wasm_save_states.rs`,
  `wasm_touch.rs` (mobile touch overlay), `wasm_lobby.rs`/`wasm_netplay.rs` (netplay lobby UI —
  belongs with `v0.8.0`'s netplay work instead), `wasm_script.rs`/`wasm_cheevos.rs` (wire once
  this rung's own native scripting/`v0.8.0`'s cheevos land), `wasm_share.rs` (settings
  share-links).
- **Recurring gate, starting here:** a byte-identical-with-all-flags-off CI check (every new
  flag landed on this ladder must leave the default build unchanged) — re-verify after `v0.8.0`
  and `v1.0.0` too. Use explicit feature combos in CI, never `--all-features`.

### v0.8.0 "Community" — netplay, RetroAchievements — **RELEASED 2026-07-10**

Both wire pre-existing stub crates against a named integration pattern; both are "connect
RustySNES to other players or an external service." Released as one tag together with
"Instrumentation" above, plus the mid-scanline/HDMA fix and a continued SPC7110 investigation —
see `CHANGELOG.md`'s `[0.8.0]` entry for the full release note.

- **Rollback netplay** — wires the `rustysnes-netplay` stub crate. **Pre-work required before
  committing to the existing full-snapshot save-state design:** benchmark
  `System::save_state()`/`load_state()` cost — `docs/benchmarks.md` has only one number today
  (steady-state frame time), and `RewindBuffer` was designed for ~10 Hz capture while rollback
  netplay calls save/restore far more often. If full-snapshot cost is too high for a real
  rollback window, delta/incremental snapshots become necessary — a real design change beyond
  `docs/adr/0006`'s explicit "future memory optimization, not correctness requirement" framing
  (a call made for rewind's occasional-capture case, not netplay's every-frame one); write a new
  ADR if this triggers. **Architecture note:** `emu_thread.rs`'s pacing model is single-player-
  only by its own doc comment; netplay's rollback drive loop is frontend-orchestrated
  resimulation, a different control model. Keep the two mutually exclusive by session type (a
  netplay session uses its own rollback-aware loop, not the generic `emu-thread` pacer) rather
  than unifying them — this sidesteps any ordering conflict with `emu-thread` landing later, in
  `v1.0.0`.
- **RetroAchievements** — wires the `rustysnes-cheevos` stub crate + the existing
  `retroachievements` flag, wrapping `rcheevos` via a `read_memory` callback +
  `rc_client_do_frame()` called every frame including fast-forwarded ones.
- **Recurring gate:** re-verify the byte-identical-with-flags-off CI check.

### v0.9.0 "Threshold" — closing the last Phase 7/8 residuals — **RELEASED 2026-07-10**

Not an originally-planned rung on this ladder — it emerged from finishing the carried-forward
items still open after `v0.8.0`: Phase 7's one remaining exit criterion, Phase 8's one remaining
ticket half, and the SPC7110 investigation's resolution. Named for what it is: the last loose
ends closed out before the `v1.0.0` production cut below.

- **Niche peripherals (multitap, mouse, Super Scope)** — Phase 7's final exit criterion, closed.
  `rustysnes_core::controller`: a real 2-bit-per-clock (`data1`/`data2`) serial-shift-register
  protocol per controller port, ported from ares' `sfc/controller/{mouse,super-scope,
  super-multitap}`, not a stub. `Bus::set_port_device` selects the peripheral (default:
  `Gamepad`, byte-identical to every prior release); `Bus::set_mouse`/`set_superscope`/
  `set_multitap_pad` feed host input once per frame. New WRIO (`$4201`/`$4213`) IOBIT register
  plumbing (previously unimplemented) and `Ppu::latch_hv_counters` (the Super Scope beam-latch
  mechanism). Save-stated as real hardware state — `FORMAT_VERSION` 2→3 (`docs/adr/0006`). The
  frontend gained a Settings → Input control to select port 2's peripheral; live host-input
  capture (a real mouse pointer, extra gamepads) is a tracked frontend follow-up, not yet wired.
- **T-81-001 PR B: 65C816 disassembly view + PC breakpoints + step/step-over/step-into** — Phase
  8's last open ticket half, closed. Entirely frontend-side (`emu.rs`), built on T-81-001b's
  existing `rustysnes_cpu::disasm` engine. `EmuCore::set_breakpoints` (re-synced every frame like
  cheats/watchpoints) is checked once per instruction boundary via the existing
  `System::step_instruction()` — a real behavior change to `run_frame` only when at least one
  breakpoint is armed (empty list = the exact prior fast path). One new `rustysnes-core` API,
  `Bus::peek` — a genuinely side-effect-free read (unlike `CpuBus::read24`, never touches the
  open-bus latch or trips watchpoints), needed because the debugger's own reads must not perturb
  the emulated hardware state they're inspecting.
- **SPC7110 boot-crash gap resolved (was never an emulation bug)** — the local test ROM turned
  out to be the English fan-translation, not the original cartridge: a SHA256 mismatch against
  `ref-proj/ares`'s own database, a checksum-size inconsistency, and a public forum thread on the
  patch's own non-standard memory map all confirm this independently. The patch adds a 1 MiB
  "Expansion ROM" at banks `$40-$4F` — precisely the bank the previously-traced derailing `JSL`
  targets — that exists only in the patch, never on real hardware. Every prior addressing/timing
  fix (`v0.4.0`-`v0.8.0`) stands as a genuine accuracy improvement; only the ROM being tested
  against was the wrong artifact. Now a ROM-sourcing gap (`docs/rom-test-corpus.md`), not an open
  bug — full evidence chain in `docs/audit/spc7110-boot-crash-2026-07-08.md`.
- **24 new unit tests**; full `--features test-roms` suite (17 suites) re-verified unchanged;
  fmt/clippy clean across all feature combinations; `no_std` and doc-build gates clean.
- See `CHANGELOG.md`'s `[0.9.0]` entry for the full release note.

### v1.0.0 — desktop UX shell maturity, performance engineering, production cut

- **Fix `Board: Send` first — DONE.** `dyn Board` is now `Send` (one-word change, confirmed —
  every existing board/coprocessor implementation compiled clean with no further changes). The
  dedicated emulation thread (`emu-thread`) compiles/tests/lints clean for the first time, but
  stays off-by-default: its loop has no audio output and doesn't yet drive cheats/watchpoints/
  breakpoints/scripting/movies/rewind/run-ahead/RetroAchievements (a real feature-parity gap vs.
  RustyNES's own mature `emu_thread.rs`, documented in `crates/rustysnes-frontend/Cargo.toml`'s
  `emu-thread` comment and `docs/frontend.md` — not silently claimed as done).
- **Desktop UX shell maturity — DONE** (the thumbnail save-state manager, key-rebind grid,
  themes, fullscreen, speed presets, welcome modal, and Performance panel with a frame-time
  sparkline all landed; `docs/frontend.md` documents each). Per-channel audio mutes did not land
  — needed its own S-DSP per-voice model research, scoped separately rather than rushed; landed
  in `v1.0.1` below, alongside global keyboard hotkeys.
- **New frame-time performance-regression CI gate — DONE** (`.github/workflows/ci.yml`'s `bench`
  job + `scripts/bench_regression_check.sh`, mirroring RustyNES's own pattern; see
  `docs/performance.md`/`docs/benchmarks.md`).
- **Save-state `FORMAT_VERSION` backward-compat fixture + regression test — ALREADY DONE.**
  Turned out to have landed earlier than this rung, in `v0.7.0 "Resolution"`'s `FORMAT_VERSION`
  1→2 bump: `tests/golden/savestate-v1-gilyon.bin` (a genuine pre-`v0.7.0` blob, not hand-crafted)
  alongside `tests/save_state_backward_compat.rs` (asserts an old-format blob fails loudly with a
  real `SaveStateError`, never a panic or silent corruption). This bullet had been carried forward
  as still-open by mistake; verified landed and green during the `v1.0.0` re-verification pass.
- **README.md rewrite — DONE**, to RustyNES's structural depth (Overview, Why, Feature
  highlights, Crates & Architecture, Quick Start, Desktop UX, Compatibility and Accuracy,
  Performance, Platform Support, Documentation, Current Release, Roadmap, Contributing, License,
  Acknowledgments) — describing RustySNES's own actual `v0.9.0`/`v1.0.0`-in-progress state, not
  RustyNES's own far more mature `v2.0.4` content.
- **Enhanced native CLI + `cargo full-build`/`full-run` — DONE.** `cli.rs` grew from 4 to 9 help
  topics (accurate content, no stale scaffold-era claims); `full` (`crates/rustysnes-frontend/
  Cargo.toml`) aggregates every native opt-in feature except the not-yet-complete `emu-thread`;
  `.cargo/config.toml` adds the `full-build`/`full-run` aliases, ported from RustyNES.
- **Explicitly deferred, not part of the parity bar:** Super Scope / multitap / mouse
  peripherals (no RustyNES analogue — NES has nothing comparable, so parity doesn't require
  modeling these; `docs/frontend.md` already notes them as stubbed) and HD texture packs (the
  `hd-pack` flag exists in the manifest already, but RustyNES itself doesn't have this feature,
  so it sits outside the parity target).
- **Final integration:** sync `docs/STATUS.md` + `to-dos/ROADMAP.md` + this document, run the
  full regression gate (every oracle/golden suite + every feature-flag combination + the
  `no_std` gate + the wasm build), then cut the tag.

Gate, updated from the earlier accuracy-only framing: the accuracy battery holds its
`v0.5.0`-established target with zero regressions; the save-state/core API is stable AND
backward-compat-fixture-proven (item above); a genuinely shippable multi-platform app with the
full breadth pass (debugger, scripting/TAS, cheats, netplay, RetroAchievements) landed and
byte-identical-with-flags-off; green CI including `no_std` + wasm + the new perf-regression
gate; README/CHANGELOG/`docs/`/`docs/STATUS.md` fully in sync.

### v1.0.1 — the two items deferred out of v1.0.0

- **Per-voice (per-channel) audio mute — DONE.** Settings → Audio grew 8 checkboxes
  (`config.audio.voice_mutes`), re-synced once per real frame via `Bus::set_voice_mutes` (the
  same "frontend/debug convenience state, re-synced unconditionally, excluded from save-states"
  pattern already used for cheats/watchpoints/breakpoints/port2_peripheral). Real S-DSP hardware
  has no per-voice mute register (only the whole-mix `FLG.6` bit) — this gates `Dsp::voice_output`,
  the point strictly downstream of BRR decode/envelope/pitch computation, so it cannot perturb any
  ROM-observable register (`OUTX`/`ENVX`/`ENDX`) or envelope timing. All unmuted by default —
  byte-identical to every prior release. See `docs/apu.md` §Per-voice mute.
- **Global keyboard hotkeys — DONE.** Every system/emulation action was menu-bar-only
  (`rustysnes help hotkeys` said so explicitly, now corrected). A fixed, non-rebindable table now
  works window-wide: `Escape`=Quit, `F1`=Save State, `F2`=Reset, `F3`=Power Cycle, `F4`=Load State,
  `F5`=Rewind, `F9`=Save States… window, `F11`=Fullscreen, `F12`=Open ROM, `Space`=Pause/Resume,
  `` ` ``=Toggle Debugger overlay (feature-gated: `debug-hooks`, mirroring the Debug menu's own
  gating — no second way to reach a surface the default build never vets). Key-down edge only,
  never on OS auto-repeat, suppressed while an egui widget has keyboard focus (so e.g. `Space`
  doesn't also insert a character into a Settings text field). The key-map avoids every default
  P1 gameplay binding. See `docs/frontend.md` §Global hotkeys.
- **Versioning note:** both items are additive/off-by-default in effect, which this project's own
  convention ("ship additive changes as MINOR") would normally cut as `v1.1.0`. Shipped as
  `v1.0.1` instead per explicit project-owner instruction overriding that convention for this one
  release — see `CHANGELOG.md`.
- **Regression gate:** full workspace suite (default + `debug-hooks`), the full clippy matrix
  (default / flags-off / `full` / `emu-thread` / `debug-hooks`), the `--features test-roms`
  27-suite accuracy/oracle battery, and the `no_std` build all green with zero regressions.

### v1.1.0 — Reach-phase accuracy research + emu-thread's biggest gaps

- **`SuperFxBoard::map`'s Game-Pak-RAM-ownership open-bus gap — FIXED.** A CPU/DMA read of Game
  Pak RAM while the GSU owned the RAM bus always returned a hardcoded `0` instead of the real
  last-driven bus byte, since `map()` classified that case as `Sram` rather than `Open`, bypassing
  `Cart::read24`'s generic open-bus fallback entirely. Zero regressions across the full
  `--features test-roms` battery with this fix alone. See `docs/scheduler.md` §Open bus via
  DMA/HDMA.
- **`emu-thread`'s two biggest gaps — DONE.** Real audio output (a thread-owned `AudioProducer`,
  pushed once per produced frame) and a proper pause/ROM-loaded/speed lifecycle (`EmuControl`
  driving a thread-owned `Pacer`) plus a `PresentBuffer` lock-free framebuffer handoff. Verified
  via the unit suite plus a real headless `xvfb-run` launch against a staged commercial ROM.
  **Not done:** cheats/watchpoints/breakpoints/port2-peripheral/voice-mutes sync, run-ahead,
  rewind recording, TAS movies, Lua scripting, netplay-aware pause, RetroAchievements — each needs
  a new shared-mutable-state design rather than a mechanical port; `emu_thread.rs`'s own module
  doc tracks the exact remaining list. `emu-thread`+`scripting`'s documented `-D warnings`
  dead-code conflict is unresolved (unchanged from `v1.0.0`).
- **Open-bus-via-DMA-latch — investigated, still open.** The naive DMA-open-bus-update fix still
  breaks all 24 Super FX/GSU golden hashes even after the `SuperFxBoard::map` fix above (confirmed
  a genuinely separate mechanism). Ruled out the `$4016`/`$4017` joypad-read blend, the generic
  CPU-side open-bus-fallback arms, and `VideoBus::cart_read` (dead code, never called). Confirmed
  a real, reproducible CPU-control-flow divergence exists but didn't isolate the exact first
  diverging instruction. See `docs/scheduler.md` §Open bus via DMA/HDMA for the full trail.
- **DRAM refresh — empirically measured, NOT implemented.** 500 steady-state frames × 3 unrelated
  ROMs show the current CPU-driven model already reproduces the correct 357,368-clock NTSC frame
  length (average gap within a fraction of a clock of zero) — the originally-planned additive
  40-clocks/scanline stall would have inflated every frame by ~10,480 clocks, a clear regression
  against this now-confirmed-correct baseline. See `docs/scheduler.md` §DRAM refresh.
- **Fractional-timebase refactor go/no-go — assessed: not warranted.** Every currently-named
  accuracy residual is a ROM-sourcing gap, a coprocessor-board scope gap, or a bug/validation
  question answerable within the existing whole-master-clock-tick model — none require sub-cycle
  resolution. See `docs/audit/fractional-timebase-go-no-go-2026-07-11.md` and `docs/adr/0002`'s
  status addendum.
- **ROM-sourcing research** — documented concrete legitimate leads (No-Intro/Archive.org verified
  dumps, homebrew SA-1 test-ROM repos) for every currently-open ROM-sourcing gap, without staging
  any actual ROM. See `docs/rom-test-corpus.md`.
- **Regression gate:** full workspace suite (default + `emu-thread` + `debug-hooks` +
  `emu-thread,debug-hooks`), the full clippy matrix (default / flags-off / `full` / `emu-thread` /
  `debug-hooks` / `emu-thread,debug-hooks`), the `--features test-roms` 27-suite accuracy/oracle
  battery, the `no_std` build, the `RUSTDOCFLAGS="-D warnings" cargo doc` gate (fixed two
  pre-existing broken intra-doc links from `v1.0.1`'s per-voice-mute work along the way), and both
  wasm32 frontends all green with zero regressions.

### v1.2.0 "Phosphor" — Libretro core + CRT/HQ2x shader pipeline — **RELEASED 2026-07-11**

- **`EmuCore` facade relocation — DONE.** The pure emulation-core facade (`load_rom`/`reset`/
  `power_cycle`/`run_frame`/`present_current_frame`/`framebuffer`/`audio`/`save_state`/
  `load_state`/the `set_*` peripheral feeds) moved from `rustysnes-frontend::emu` into a new,
  `std`-only `rustysnes_core::facade` module — a libretro core or any other headless embedder
  depends on `rustysnes-core` alone. `rustysnes-frontend::emu::EmuCore` is now a thin wrapper
  keeping only the debugger-only fields. Zero behavior change; also fixed a determinism-seed-
  discarding bug found in review (`load_rom`/`power_cycle`/`close_rom` rebuilt `System::new(0)`
  instead of preserving the constructor's seed). See `docs/architecture.md` §3/§6.
- **`rustysnes-libretro` — DONE.** A libretro core wrapping the relocated facade: region-aware
  NTSC/PAL geometry+timing (corrected via `RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO` on the first
  `on_run` after a ROM loads, once the cart header's region byte is known), the S-DSP's real
  32 kHz sample rate, coprocessor firmware auto-resolution from the frontend's system directory,
  Game Genie/Pro Action Replay cheat support (`on_cheat_set`/`on_cheat_reset`), and raw WRAM/
  VRAM/SRAM memory-map pointers (`get_memory_data`/`get_memory_size`) for RetroArch's own SRAM
  autosave and RetroAchievements/cheat tooling. New additive `Bus::wram`/`wram_mut`,
  `Ppu::vram`/`vram_mut`, `Cart::sram_mut` accessors support it. Peripheral negotiation (Mouse/
  Super Scope/Multitap via `RETRO_DEVICE_SUBCLASS`) was a documented follow-up at the time, not
  yet wired — **landed in the post-`v1.3.0` patch cluster** (see below). See `docs/libretro.md`.
- **CRT/HQx presentation post-filter pipeline — DONE.** A `PostFilter` enum (`None`/`Crt`/`Hqx`,
  Settings → Video radio row + per-filter strength sliders, plus a View → Post-filter menu
  submenu). `Crt` adds scanlines (a parabolic per-source-row brightness profile) + an RGB
  aperture-grille mask (fixed-pitch phosphor-triad tint), each independently strength-slidered.
  `Hqx` adds a single-pass, edge-directed diagonal blend (a 2xSaI/Eagle-family diagonal-similarity
  heuristic) — an HQ2x-**style** approximation, not a literal lookup-table port, matching this
  project's fixed-resolution architecture. `PostFilter::None` (default) is the pre-existing direct
  blit kept byte-for-byte unchanged: `Gfx::present`'s `None` arm calls the SAME unmodified
  `Gfx::blit`, not a re-derived equivalent. `Gfx::letterbox_scale` was extracted out of `blit`'s
  own inline math (a pure, behavior-preserving refactor, regression-tested against hand-computed
  cases) so both filter passes share the identical letterbox convention. Shaders are inline
  `const &str` WGSL in `gfx.rs`, matching the existing `BLIT_WGSL` convention — deliberately not
  split into a separate shader crate (no second consumer to justify it). Verified: `naga`
  WGSL-parse+validate tests for both new shaders, plus a real headless `xvfb-run` launch of the
  native binary against a staged ROM with each of `None`/`Crt`/`Hqx` set in `config.toml` — all
  three ran clean (zero stderr, no panics) against a real wgpu adapter. No golden-screenshot
  regression harness exists in this project (the existing `commercial_screenshots.rs` captures the
  raw core framebuffer, entirely upstream of this render path), so the `None`-path-unchanged
  guarantee is structural (same function, not a pixel-diff proof). Not built (documented scope
  cuts): RustyNES's NTSC composite-signal simulation, `.slangp` shader-preset loading, and overscan
  cropping (a separate, pre-existing `TODO`). See `docs/frontend.md` §Presentation post-filters.
- **Regression gate (facade + libretro + shader pipeline):** full workspace suite (455 tests),
  the full clippy matrix (default / flags-off / `full` / `emu-thread`), the `--features test-roms`
  accuracy/oracle battery, the `no_std` build, the doc-warnings gate, and both wasm32 frontends
  all green with zero regressions; the `rustysnes-libretro` crate builds/links clean as both
  `cdylib` and `staticlib` with every required libretro C-ABI symbol confirmed exported.

### v1.3.0 "Palimpsest" — HD texture packs — **RELEASED 2026-07-11**

- **Tile-identity hashing + the `TileTag` recording hook — DONE.** `rustysnes-ppu::hdtag::
  hash_tile` computes a palette-inclusive XXH3-64 hash (a 1-byte `TileClass` discriminant + bpp +
  the tile's raw pre-flip VRAM words + its resolved `2^bpp`-color CGRAM palette) into a fixed
  642-byte stack buffer — no heap allocation on the rendering hot path. `Ppu::set_hd_pack_tagging`
  (off by default) gates a write-only `Ppu::tile_tags()` side-buffer, indexed exactly like
  `Ppu::framebuffer()`, populated by three small companion helpers in `render_bg`/`render_mode7`/
  `render_objects` reusing address/bpp/palette values already resolved by each path's normal
  fetch. Proven byte-identical to every prior release both structurally (the whole mechanism is
  `#[cfg(feature = "hd-pack")]`-gated out of existence, not just runtime-disabled) and at the
  value level (`hd_pack_tagging_toggle_does_not_alter_framebuffer_output`); turning tagging off
  also clears stale tags from the last tagged frame. Never part of `save_state`/`load_state` —
  the same host/frontend-convenience carve-out as cheats/watchpoints/voice-mutes. See
  `docs/ppu.md` §HD texture pack `TileTag` recording hook.
- **Frontend loader + pure CPU compositor — DONE.** `crate::hd_pack`: a versioned `pack.toml`
  TOML manifest, a PNG decoder (pure-Rust `png` crate, normalizes any source color type/bit
  depth), path-traversal-safe image-path resolution, duplicate-tile-hash rejection, and per-ROM
  discovery mirroring `save_states.rs`'s directory convention. `crate::hd_compositor::composite`:
  a pure function (framebuffer + tags + pack tiles in, a new RGBA8 buffer out) with no wgpu/
  `EmuCore` dependency, fully unit-testable without a GPU adapter — each 8×8 output cell is
  sampled once and either replaced by its matching pack tile (mirrored per `hflip`/`vflip`) or
  nearest-neighbor-upscaled from its native color.
- **Settings UI + config + CLI docs + ADR 0010 — DONE.** `EmuCore` gained pack management
  (`available_hd_packs`/`hd_pack_name`/`set_hd_pack`, with `load_rom`/`close_rom` clearing a
  stale pack and `power_cycle` re-enabling tagging on the freshly reconstructed `Ppu`). Settings →
  Video gained a dynamic pack `ComboBox`; `VideoConfig` gained `hd_pack_name: Option<String>`
  (additive, default `None`), auto-reselected after loading a ROM. `docs/adr/0010` documents the
  four load-bearing decisions (the hashing scheme, the write-only off-by-default `TileTag` hook,
  the core-stays-pack-agnostic split, the versioned manifest) and the honest trade-offs.
- **Final integration — DONE.** The compositor is wired into the live wgpu present path:
  `Gfx`'s streaming texture, previously a fixed `MAX_W × MAX_H` allocation, now grows on demand
  (`Gfx::ensure_texture_capacity`, capped at this device's actual downlevel-WebGL2
  `max_texture_dimension_2d`) to fit the composited output at a fixed 2× upscale (not yet
  user-configurable); `blit`/`present`'s UV math divides by the texture's current actual size
  rather than the `MAX_W`/`MAX_H` constants, so it stays correct after a grow. The no-pack-active
  path is unaffected — the texture never grows past its original allocation. Not wired for the
  `emu-thread` build (its framebuffer arrives via a lock-free handoff with no equivalent `TileTag`
  channel yet). Verified via real headless (`xvfb-run`) launches: no pack configured, a real
  generated pack at the default 2× scale, and the same pack with scale temporarily forced to 3×
  specifically to exercise the texture-growth path — all three ran clean with no panics or wgpu
  validation errors.
- **Regression gate:** full workspace suite (455 tests, 44 suites), the full clippy matrix
  (default / `hd-pack` / `full` / `emu-thread,hd-pack` / the pre-existing `debug-hooks`/
  `scripting`/`cheats`/`netplay`/`retroachievements` lanes), the `--features test-roms`
  accuracy/oracle battery (28 tests, 17 suites), the `no_std` build, the doc-warnings gate, both
  wasm32 frontends, and `rustysnes-libretro` all green with zero regressions.

### v1.4.0 "Convergence" — closing the post-v1.3.0 patch cluster — **RELEASED 2026-07-11**

- **Fullscreen crash on monitors wider/taller than 2048px — FIXED.** `Gfx` requested
  `wgpu::Limits::downlevel_webgl2_defaults()` unconditionally on every target, capping
  `max_texture_dimension_2d` at 2048 even on native GPUs that support far more; fullscreening on a
  wide monitor (e.g. 3440x1368) sent `Surface::configure` an out-of-range request, which
  panics/aborts (no recoverable error path there). Native now requests `downlevel_defaults()` and
  both targets call `.using_resolution(adapter.limits())`; the granted limit is tracked at
  runtime (`Gfx::max_texture_dim`) and enforced everywhere the old hardcoded constant was.
  Confirmed via a standalone wgpu diagnostic against the real adapter available in dev (an NVIDIA
  RTX 3090, `max_texture_dimension_2d = 32768`): the old code only ever granted 2048 (matching the
  exact reported crash numbers), the new code grants the full adapter limit.
- **Window Size presets (1x-4x, RustyNES parity) — DONE.** Native-only View → Window Size menu,
  `MenuAction::SetWindowScale`; the app now launches at 3x by default (`INITIAL_SCALE`), matching
  RustyNES. `chrome_padded_size` derives width from the scaled height via `Gfx`'s own
  `TARGET_ASPECT` (4:3) and `Config::Region::active_height()` rather than hardcoding `SNES_W`/
  `SNES_H_NTSC` directly — two real bugs (a letterbox-squeeze aspect mismatch, a PAL-height
  mismatch) caught by automated code review and fixed before merge.
- **Libretro peripheral negotiation — DONE.** `rustysnes-libretro` now declares
  `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO` (Mouse + Super Multitap + Super Scope on port 2, Mouse
  only on port 1 — mirroring bsnes's own libretro core's per-port device menu exactly,
  `ref-proj/bsnes/bsnes/target-libretro/libretro.cpp`) and polls the matching libretro input API
  per port each `on_run` (`RETRO_DEVICE_MOUSE`, `RETRO_DEVICE_LIGHTGUN`'s absolute screen
  coordinates + trigger/cursor/turbo/pause, and Multitap's four sub-pads via libretro ports
  `[1, 4]`, also bsnes' own precedent). Closes the one gap `v1.2.0`'s own libretro core landing
  left open. See `docs/libretro.md`.
- **Open-bus-via-DMA-latch (the "Speedy Gonzales stage 6-1" case) — FIXED.** Two prior
  investigation passes (`v1.1.0` and an earlier pass this cluster) isolated the exact divergence
  but couldn't determine which of two candidate fixes (if either) matched real hardware, since
  neither had an independent oracle for the specific accumulated value. Cross-checking directly
  against ares' AND bsnes' `CPU::Channel::readA`/`readB`/`writeA`/`writeB` (`ref-proj/ares/ares/
  sfc/cpu/dma.cpp`, `ref-proj/bsnes/bsnes/sfc/cpu/dma.cpp` — logically identical) established the
  precise rule: DMA/HDMA reads update `open_bus`, writes never do. `DmaBus for Bus`'s `read_a`/
  `read_b` now update `open_bus`; `write_a`/`write_b` do not; `read_a`'s forbidden-range branch
  also now sets `open_bus` to a hard `0` matching ares/bsnes exactly. `superfx_boots_live_and_
  deterministic`'s 24 golden hashes were re-blessed with this citation trail as justification —
  every other assertion in that test (coprocessor detection, GSU liveness, the FillPoly plot
  threshold, cross-run determinism) is unaffected. Full workspace suite + the full
  `--features test-roms` battery (28 tests, 17 suites) both green. See `docs/scheduler.md` §Open
  bus via DMA/HDMA for the complete investigation and fix.
- **`emu-thread` mechanical re-sync (cheats/watchpoints/breakpoints/port2-peripheral/
  voice-mutes) — DONE.** These were previously synced only on the synchronous drive path,
  silently never applied at all in the threaded build. Turned out to be a genuinely mechanical
  port after all (contrary to `v1.1.0`'s own framing): `EmuCore` is the same `Arc<Mutex<...>>`
  both the winit thread and the emu thread share, so re-syncing from `render`'s existing brief
  lock — once per present, before the emu thread's next `run_frame()` — is sufficient; none of
  this needs to run ON the emu thread itself. See `emu_thread.rs`'s own module doc.
- **`emu-thread` run-ahead + netplay-aware pause — DONE.** Run-ahead: `drive_one` takes the same
  `run_ahead > 0` branch the synchronous path (`app.rs`) already does — `crate::rewind::
  step_with_run_ahead` is only called when run-ahead is actually configured; otherwise it
  publishes straight from the borrowed framebuffer slice, exactly matching this function's
  pre-run-ahead behavior with zero extra allocation. (An earlier revision called the helper
  unconditionally; Copilot's PR review caught that its own `frames == 0` fast path still does an
  avoidable `framebuffer().to_vec()` copy beyond the plain `run_frame()` it replaced — a real
  per-frame cost regression in the common disabled case, fixed before merge.) Netplay:
  `NetplayState::drive` was previously dead code under `emu-thread` (buried inside a
  `#[cfg(not(feature = "emu-thread"))]` block) — netplay was silently completely non-functional in
  threaded builds before this fix. It now runs once per present from the winit thread (matching
  `NetplayState::drive`'s own "drive one real frame" contract), while a new
  `EmuControl::netplay_paused` flag idles the emu thread; the flag is set by the winit thread and
  re-checked by the emu thread under the same `EmuCore` mutex in `drive_one`, so there's no TOCTOU
  race with the netplay rollback session claiming the `System`. `PresentBuffer` was extended to
  carry `(width, height)` alongside its bytes (`publish`/`take_into` signatures changed) — this
  was previously safe only because every published frame was exactly `emu.framebuffer()`'s current
  dims; run-ahead's peeked frame can differ across a hi-res-mode-toggle-mid-peek edge case, so
  dims must travel with the bytes through the same slot to avoid a bytes/dims size mismatch on the
  GPU-upload path. A second review finding (Gemini) caught a related gap: the present path's own
  `dims` fallback (used when `take_into` returns nothing new) still read the live `emu.fb_dims()`,
  which can have moved on to a new resolution since the last frame actually taken from
  `PresentBuffer` — mismatching the OLD-resolution bytes still sitting in `present_staging`. Fixed
  via a tracked `Active::present_dims` field, updated only when `take_into` returns `Some`, used as
  the sole fallback instead. Two previously-real CI gaps closed alongside this: `emu-thread` was
  never actually clippy-gated at all (only referenced in a comment), and its own unit tests were
  compile-checked via clippy but never executed — both now covered (`emu-thread` and
  `emu-thread,netplay` clippy in `lint`; `emu-thread,netplay` tests in `full-test`). Movies/
  scripting/RetroAchievements/rewind-recording remain unported — but reclassified as an
  intentional, permanent architecture boundary (confirmed by directly reading RustyNES's own
  mature 914-line `emu_thread.rs`, which doesn't port any of these to its thread either), not a
  parity gap. A live headless launch exercising a real netplay session under `emu-thread` was NOT
  re-verified this pass (this sandbox's headless GUI automation hangs regardless of feature combo
  — a previously recorded limitation); flagged honestly rather than claimed. See `emu_thread.rs`'s
  own module doc and `docs/frontend.md`.
- **Full pre-release gate — GREEN.** `cargo fmt --all --check`; `cargo clippy --workspace
  --all-targets -- -D warnings`; per-feature clippy across debug-hooks/scripting/cheats/netplay/
  retroachievements/emu-thread/`emu-thread,netplay`/full/hd-pack; `cargo test --workspace`; the
  full `--features test-roms` ROM-oracle battery (28 tests, 17 suites, zero regressions);
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; the `no_std` build; both wasm32
  frontends (`wasm-winit` via a real `trunk build --release`, `wasm-canvas` via `cargo check
  --target wasm32-unknown-unknown`); `rustysnes-libretro`; `cargo deny check`; and `cargo audit`
  (zero advisories). The frame-time bench gate was not re-run this pass (unaffected by this
  cluster's frontend-only changes).

## Post-v1.0 — Reach (deferred)

- **Libretro core**, the **CRT/HQx shader/filter pipeline**, and **HD texture packs** landed in
  `v1.2.0`/`v1.3.0` above. Still deferred: the **fractional-timebase MAJOR refactor**
  (`docs/adr/0002`, only if hard residuals from the accuracy-debt cluster above actually warrant
  it). The **mobile/Android + iOS target** — previously "no appetite assumed by default" here —
  is now explicitly IN SCOPE as of `v1.14.0 "Foundry"`; see `docs/adr/0012-mobile-platform-
  target.md` for the reversal decision and `docs/mobile-readiness.md` for the living status page.

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
- **RustyNES-lockstep check:** run `to-dos/LOCKSTEP-CHECKLIST.md` once at the start of scoping
  each release in the RustyNES-parity ladder below, so drift against RustyNES's own continuing
  development gets caught and folded in (or explicitly deferred) before it accumulates, rather
  than only at a periodic re-audit.
- **Version bump:** every `chore(release)` closeout PR bumps `[workspace.package] version` in the
  root `Cargo.toml` *and* each crate's own pinned `version` field (every crate except
  `rustysnes-libretro`, which uses `version.workspace = true`) to the release being cut, then
  runs `cargo check --workspace` to regenerate `Cargo.lock`. This was true of every release back
  to `v0.7.0` but wasn't written down here, so `v1.5.0`/`v1.6.0` missed it — `env!(
  "CARGO_PKG_VERSION")` feeds the Help window and `--version`, so both under-reported the running
  version (including on the live GitHub Pages demo) until caught and fixed in `v1.7.0`.

## RustyNES-parity ladder (`v1.5.0` onward)

A second, parallel ladder theme, distinct from the phase-spine ladder above: closing the gap
between RustySNES and its sibling NES emulator RustyNES (`../RustyNES`, currently released
`v2.1.5 "Fathom"`, `v2.1.6 "Expansion Audio"` in progress). Gap analysis, scope decisions
(full mobile parity in scope; dormant monetization scaffolding in scope; lockstep tracking over a
frozen snapshot), and full per-release detail live in the roadmap plan this section summarizes;
this ladder is the durable, versioned record — update it at every release like every other rung
in this document, not left to drift stale.

Two premise corrections drive this ladder: (1) RustyNES is not "at v2.2.0" — that's an unscoped
label for its ongoing post-`v2.1.5` arc, with its real next milestone (a joint Android/iOS store
launch) targeting `v2.3.0`, already deferred twice; this ladder reaches RustyNES's *current*
maturity bar, tracked in lockstep, not a fixed number. (2) The mobile track's biggest named risk —
this project's own fractional-timebase refactor (`docs/adr/0002`) — is **already closed**: assessed
"not warranted" at `v1.1.0` (`docs/audit/fractional-timebase-go-no-go-2026-07-11.md`), with five
stable minors (`v1.0.0`–`v1.4.0`) since shipped with zero save-state-format churn. The mobile rungs
below don't need to wait on this; it's a fact to cite, not a decision to make.

Save-states, rewind, run-ahead, cheats, netplay, native RetroAchievements (login + unlock toasts),
Libretro, CRT/HQx filters, and Mouse/Super Scope/Multitap peripherals are already at parity with
RustyNES and are explicitly NOT rebuilt anywhere in this ladder.

### `v1.5.0 "Bedrock"` — CI safety net — **RELEASED 2026-07-11**

The highest-leverage fix, done first: PR-time CI never ran `cargo test` (`full-test`/`no_std`/
`bench` were tag-only), relying entirely on `CONTRIBUTING.md`'s manual pre-push checklist as the
correctness gate. See `docs/adr/0011`.

- [x] `.github/actions/rust-setup` composite action; `ci.yml`/`pages.yml` migrated onto it.
- [x] `ci.yml` restructured: a `changes` (`dorny/paths-filter`) + `setup` job computing a
      light/full/skip mode per run; a new `test-light` job (`cargo test --workspace`, Linux-only,
      debug, cached) runs on every PR/push with code changes; `full-test`/`no_std`/`bench` widened
      from tag-only to `push to main OR tag OR weekly cron OR manual dispatch`.
- [x] `ci-success` summary job — the one stable required-check name.
- [x] `docs/adr/0011-branch-protection-and-ci-success-gate.md`.
- [x] `CONTRIBUTING.md`'s Quality gate section notes which checks CI now also enforces.
- [x] Branch protection on `main` requiring `ci-success` — enabled after PR #76 merged and the
      check ran green.
- [x] `chore(release): v1.5.0 "Bedrock"` closeout PR.

**Released 2026-07-11.** PR #76: three real findings from `copilot-pull-request-reviewer`
(a floating `dtolnay/rust-toolchain@master` ref, a missing `libxkbcommon-x11-dev` package, an
inaccurate comment), each fixed in its own follow-up commit and adjudicated inline before
squash-merge. `gemini-code-assist` hit its daily quota limit and did not review this PR — noted,
not blocked on (quota resets are out of this project's control; the ceremony proceeds with
whichever bots actually respond).

### `v1.6.0 "Lighthouse"` — docs site, PWA, accuracy-ledger — **RELEASED 2026-07-11**

MkDocs handbook over the existing `docs/` tree; a combined `web.yml` replacing `pages.yml` (wasm
demo at `/`, rustdoc at `/api/`, MkDocs at `/docs/`, a `<5MiB` gzip size-budget gate); a
`manifest.webmanifest`/`sw.js` PWA/offline pass for the wasm demo; `docs/accuracy-ledger.md`
extracting `docs/STATUS.md`'s "Accuracy dashboard"/"Named residuals" content into RustyNES's
per-item disposition format. Standing practice from here on: every future release-closeout PR adds
a docs page and updates the ledger if a disposition changed.

- [x] `mkdocs.yml` (Material for MkDocs) over the existing `docs/` tree, curated nav.
- [x] `.github/workflows/web.yml` replacing `pages.yml` — demo at `/`, rustdoc at `/api/`, MkDocs
      at `/docs/`, `scripts/wasm_size_budget.sh` (`<5MiB` gzip) gating both PR and push.
- [x] `manifest.webmanifest` + `icon.svg` + a stale-while-revalidate `sw.js` service worker for
      the wasm demo, wired into `index.html`.
- [x] `docs/accuracy-ledger.md` — every known residual mapped to a disposition; one item honestly
      flagged as **not yet implemented** rather than overclaimed (a SNES-appropriate Holy-Mapperel
      analog for the 3 unit-test-only coprocessor boards — no oracle-gated release scopes it yet).
- [x] `docs/DOCUMENTATION_INDEX.md` refreshed (stale `v0.4.0` stamp, a dead `SALVAGE_MANIFEST.md`
      link, a stale ADR count) and cross-linked to the new ledger + lockstep checklist.
- [x] `chore(release): v1.6.0 "Lighthouse"` closeout PR.

**Released 2026-07-11.** PR #78: two real bugs from `copilot-pull-request-reviewer` (inverted
`web.yml` concurrency `cancel-in-progress` logic; a `sw.js` fetch handler that could resolve
`respondWith()` to `undefined` on a truly offline first visit), each fixed in a follow-up commit
and adjudicated inline. The `web.yml` `build demo + docs` job — the first real exercise of the
MkDocs/trunk/size-budget pipeline — passed clean. `gemini-code-assist` remained quota-limited.

### `v1.7.0 "Telemetry"` / `v1.8.0 "Tracepoint"` — debugger foundation + depth

Extracts the current 4-panel inline debugger (`ui_shell.rs`, `lib.rs`'s own doc comment: "the deep
debugger panels are still TODO stubs") into a real `debugger/` module — a memory panel, conditional
(R/W/X) watchpoints, dedicated CPU/PPU/APU panels (`v1.7.0`), then callstack/step-controls, an
inline 65816 assembler, a memory-compare panel, a SNES-specific coprocessor state panel (no NES
analog), and an in-app doc browser (`v1.8.0`). Every later rung's new panels plug into this
scaffold — sequenced first among the desktop feature rungs for that reason.

#### `v1.7.0 "Telemetry"` — **RELEASED 2026-07-12**

- [x] Extracted the debugger overlay out of `ui_shell.rs` into `crates/rustysnes-frontend/src/
      debugger/{mod.rs,cpu_panel.rs,ppu_panel.rs,apu_panel.rs,cart_panel.rs,watch_panel.rs}` — a
      pure structural move (`render_debugger`/`render_watch_panel` stay `impl ShellState` methods,
      split across files via Rust's cross-file impl support; `render_cpu_panel` etc. became free
      `fn render(...)` per submodule), zero behavior change, verified by the full local quality
      gate (fmt, clippy across 4 feature lanes, the doc-warnings gate, `no_std`, and the full
      workspace test suite — 61+ frontend tests, all green — run locally before pushing).
- [x] `lib.rs`'s stale "the deep debugger panels are still TODO stubs" doc comment corrected.
- [x] Memory panel: `DebugSnapshot::memory_window` (512 bytes, `MEMORY_WINDOW_LEN`), read via
      `Bus::peek` in `EmuCore::debug_snapshot` (mirrors the existing VRAM-window pattern exactly);
      a read-only hex dump added to the renamed "Memory/Watch" panel tab.
- **Honestly scoped down from the original plan:** RAM search (multi-frame value narrowing) and
  write-capable hex editing are **not** included — a real memory *viewer* landed, not a memory
  *editor*. Conditional watchpoints remain R/W/RW as before (no execute/`X` watchpoints — that
  needs a new `rustysnes-core::watchpoint::WatchKind` variant + `Bus` hook, a cross-crate change
  out of this rung's scope). Both tracked as open follow-ups, not silently dropped.

#### `v1.7.1` — **RELEASED 2026-07-12** (patch, no new scope)

A user comparing the live RustySNES and RustyNES GitHub Pages demos side by side reported
RustySNES's canvas rendering visibly smaller. Root cause: `App::create_window` special-cased
`wasm32` to a hardcoded `512x448` size, assuming `web/index.html`'s CSS controlled the actual
rendered size — it doesn't; winit's web backend resizes the attached `<canvas>` to match the
requested inner size regardless. RustyNES's own `create_window` requests `NES_W * INITIAL_SCALE`
unconditionally (confirmed by reading its source), which is why its own demo already rendered at
3x. Fixed by having `wasm32` share the same `chrome_padded_size(INITIAL_SCALE, region)` call
native uses; `web/index.html`'s CSS updated (`aspect-ratio` + `height: auto`, after a review
finding that a fixed-height fallback would distort on narrow viewports). Verified via `cargo
check` on both native and `wasm32-unknown-unknown`, a real `trunk build --release`, and the full
`rustysnes-frontend` test suite — no browser available in this environment to visually confirm
the on-screen render, flagged honestly rather than assumed.

#### `v1.8.0 "Tracepoint"` — **RELEASED 2026-07-12**

- [x] `debugger/memory_compare_panel.rs` — captures a baseline of the Memory panel's live window,
      diffs it against the current window every frame, shows only changed rows
      (`before -> after`). Flags a start-address mismatch instead of a misleading diff if the
      window scrolled since capture.
- [x] `debugger/doc_panel.rs` — an in-app SNES-terminology glossary (`docs/glossary.md`, embedded
      via `include_str!`) + a link to the `MkDocs` handbook. Deliberately scoped to the glossary
      alone (~3KB) rather than the full subsystem-spec tree (10-50KB each) to keep wasm size
      impact negligible — verified via a real `trunk build --release` + the size-budget script:
      +2KB gzip, ~2.05 MiB of the 5 MiB budget still free.
- [x] New `DebugPanel::MemCompare`/`DebugPanel::Doc` variants + panel-selector buttons wired into
      `debugger/mod.rs`; new `ShellState::memcmp_baseline` field.
- **Honestly scoped down from the original plan:** a call-stack view, an instruction/event trace
  buffer, and an inline 65816 assembler are **not** included — all three need new core-side
  instrumentation (tracking call/return events or recording a trace log as they happen, not
  inferable from a point-in-time memory snapshot the way this rung's two panels are), a larger
  cross-crate change than this rung's frontend-only scope. A dedicated per-coprocessor-type
  register panel (beyond the SA-1/GSU state the existing Cart panel already shows) needs new
  `Board`-trait debug-state accessors — also deferred. All tracked as open follow-ups.
- [x] `chore(release): v1.8.0 "Tracepoint"` closeout PR (version bump + dated `CHANGELOG.md`
      entry — caught in PR review: an earlier draft of the feature PR dated the `CHANGELOG.md`
      section and marked this rung `RELEASED` before the version bump had actually happened, same
      "everything lands in the release-closeout PR, not the feature PR" pattern every prior rung
      in this ladder already follows).
- **Regression gate:** full local quality gate run before pushing (`fmt`, `clippy` across the
  default/`debug-hooks`/`full` feature lanes, the doc-warnings gate, `wasm32-unknown-unknown`
  compile, `cargo test -p rustysnes-frontend` — 61 tests, all green) — this sandbox has the exact
  pinned `1.96` toolchain, so these run for real, not just left to CI.

### `v1.9.0 "Marionette"` — Lua scripting bus-widening — **RELEASED 2026-07-12**

Widens `rustysnes-script`'s `emu.read` from `Bus::peek_wram` (WRAM-only) to `Bus::peek` (the full
24-bit bus — WRAM, cart ROM/SRAM; I/O register space still reads back as `0`, matching the
debugger's own Memory panel posture). `emu.write` stays deliberately scoped to WRAM only — a
side-effect-free "poke" has no clean semantic for register space (a real PPU/APU/DMA register
write has hardware side effects a silent poke can't model without either faking them or breaking
the determinism contract), so widening reads and keeping writes WRAM-scoped is an asymmetric
choice, not an oversight.

**Deferred (honestly scoped, not silently dropped):** a wasm `piccolo` Lua backend (scripting is
currently native-only, `mlua`) and TAStudio-style piano-roll movie editing on top of the existing
`rustysnes_core::movie` record/playback primitives are both substantial standalone efforts — each
comparable in size to a full rung of this ladder on its own — and are pushed to a later,
explicitly-scoped release rather than folded into this one. `v1.11.0`'s RetroAchievements
hardcore-mode gate still sequences after `v1.9.0` as planned: hardcore only needs to gate the
*existing* movie record/playback primitives, not a TAStudio UI.

### `v1.10.0 "Atelier"` — HD-pack `emu-thread` wiring — **RELEASED 2026-07-12**

Wires HD-pack compositing into `emu_thread::drive_one` for the first time — the synchronous
render path composited an active pack via `hd_compositor::composite` before its own `drop(emu)`
since `v1.3.0`, but the threaded build had no equivalent step, so a threaded build with a pack
selected silently rendered the native, uncomposited framebuffer. `drive_one`'s plain-frame
(run-ahead-disabled) branch now composites before publishing to `PresentBuffer`; the common
no-pack-active case stays allocation-free via a cheap `hd_pack_name()` pre-check. Found in review
(#90): the lock is released before the `PresentBuffer` copy in this branch too, matching the
run-ahead branch's existing pattern.

**Deferred (honestly scoped, not silently dropped):**

- Turning HD-pack support from consumer-only into an in-app **authoring tool** — browsing the
  live `TileTag` stream, assigning replacement PNGs, writing `pack.toml` + assets — needs a new
  core-side "reconstruct RGBA pixels for a given tile hash" API that doesn't exist yet (the
  tile-identity hash doesn't reverse to a VRAM location). A genuinely separate, substantial piece
  of work from the `emu-thread` wiring fix above; pushed to a later, explicitly-scoped release.
- **Run-ahead + HD-pack combined is a known, tracked gap, not silently broken.** Found in review
  (#90): `step_with_run_ahead`'s returned frame is a PEEKED frame (captured, then rolled back so
  `emu`'s persisted state only advances by one real frame), but
  `EmuCore::hd_pack_composite_inputs` reads `Ppu::tile_tags()` from `emu`'s CURRENT (post-rollback)
  state — a different frame than the peeked bytes. `drive_one`'s run-ahead branch deliberately
  skips compositing rather than apply replacement tiles keyed to the wrong frame (which would
  silently corrupt the picture). **The same desync already exists, unfixed, in `app.rs`'s
  synchronous render path** — pre-existing since run-ahead and HD-pack were first combined, not
  introduced by this release and not touched by it either. A real fix needs
  `step_with_run_ahead` itself to capture the peeked `TileTag` buffer alongside `fb` (before the
  rollback), shared by both call sites — a follow-up for a later `v1.10.x` patch or a subsequent
  rung, not blocking this one (run-ahead and HD-pack are each off by default; the narrow
  intersection of both enabled together is the only affected case).

### `v1.11.0 "Podium"` — RetroAchievements: the game was never actually loaded — **RELEASED 2026-07-12**

Found during scoping, not in the original plan: **no code path ever called
`RaClient::begin_load_game`.** Login worked, `CheevosState::do_frame` ran every emulated frame,
and `AchievementTriggered` events were wired all the way to status-bar toasts — but with no game
ever identified/loaded into `rc_client`, there was no achievement set loaded to evaluate
memory against, so achievements could never actually trigger. This is the actual prerequisite
bug blocking hardcore mode, leaderboards, and rich presence from meaning anything at all, so
fixing it is this rung's real deliverable: `CheevosState::load_game`/`unload_game` now wrap
`RaClient::begin_load_game`/`unload_game`, called from `app.rs`'s `MenuAction::OpenRom`/`CloseRom`
handlers (a no-op unless a user is logged in). A `poll()`-drained toast surfaces success/failure
so the fix is observably verifiable end-to-end, not just type-checked.

**Deferred (honestly scoped, not silently dropped):**

- Splitting a new `rustysnes-ra` session/UI crate out of `frontend/src/cheevos.rs`'s informal
  state (`rustysnes-cheevos` stays FFI-only) — a pure refactor with no functional value on its
  own; folding it into a later rung that actually needs the crate boundary (e.g. a leaderboard
  panel) avoids churn now.
- Hardcore mode gating rewind/save-load/cheats/TAS, and surfacing leaderboards/rich-presence in a
  Tools window — both real, substantial UI/gating features that are meaningless without a loaded
  game in the first place (this rung's fix). Pushed to a later, explicitly-scoped release.
- A known scope note on the fix itself: a ROM loaded via the CLI at startup, followed by a *later*
  login through the Tools window, is not retroactively announced to `rc_client` (login happens
  after the ROM-change wiring already ran once at startup). The common path (launch, log in, then
  open a ROM via the File menu) is unaffected. See `cheevos.rs`'s module doc.

### `v1.12.0 "Refraction"` — shader/NTSC ladder depth — **RELEASED 2026-07-12**

Delivered: a third `PostFilter::Xbrz` variant (a single-pass, context-aware corner-rounding
blend — an xBRZ-*style* approximation of the algorithm's corner rule, gating `Hqx`'s 2x2 corner
blend by a wider 4x4-neighborhood confidence check, not a literal multi-pass xBRZ port); the new
`rustysnes-gfx-shaders` crate (extracts `gfx.rs`'s inline WGSL byte-identically, verified via a
script diff against the pre-extraction source — later reused as-is by the mobile track,
`v1.14.0`).

**Deferred** (unrevisited from `v1.2.0`'s original scope call, not a new finding this release):
RetroArch `.slangp`/`.cgp` shader-preset import, and the optional composite/RF post-pass
explicitly scoped as *not* a port of RustyNES's NES-specific dot-crawl ladder (SNES has no
equivalent dot-clock-subsampling artifact to exploit). A real xBRZ implementation (the literal
multi-pass algorithm, not this release's single-pass GPU approximation) was considered and
deferred — the single-pass shader approximation was judged the honestly-testable, right-sized
scope for a presentation-only, non-accuracy-critical feature.

### `v1.13.0 "Vantage"` — accessibility/theming + save-state polish — **RELEASED 2026-07-12**

Delivered: `AppTheme::HighContrast` (WCAG AA/AAA dark theme) + `AppTheme::Colorblind`
(Okabe-Ito-accented dark theme), both additive and regression-tested against the stock dark
theme.

**Honestly re-scoped, not silently dropped**: the other two originally-planned items here were
investigated and neither fit a discrete code change. The "save-state versioned-migration
regression fixture... the one real save-state gap found" premise was stale —
`System::load_state` was always designed to fail loudly on an older-format blob (never to
migrate one), by deliberate choice recorded since the `FORMAT_VERSION` `2`/`3` bumps, and a
regression fixture proving exactly that has existed since `v0.7.0`
(`save_state_backward_compat.rs`). Closed as verified-non-issue; the 10-slot/thumbnail Save
States manager itself is already at full parity. The keyboard-only-navigation audit was found to
be a manual-walkthrough task, not a bug with a discrete fix (egui's own default Tab order is used
everywhere; nothing custom, nothing known-broken, but nothing walked/confirmed either) —
documented as an explicit open item in `docs/frontend.md`'s Theme section rather than converted
into a hollow "audit passed" claim. See `docs/frontend.md` for the full explanation of both.

**Decision-doc rung (small, precedes the mobile track):** reverses this document's own "Post-v1.0
— Reach (deferred)" no-mobile-appetite line and `docs/frontend.md`'s no-gfx-shaders-crate
rationale; stands up `docs/mobile-readiness.md`; a new ADR records the mobile-platform-target
decision, citing that `docs/adr/0002`'s gate is already closed favorably.

### `v1.14.0 "Foundry"` — Mobile Phase 1: bridge foundations — **RELEASED 2026-07-12**

Delivered: new crate `rustysnes-mobile`, a `UniFFI` bridge over `rustysnes_core::facade::EmuCore`
(the same `std`-only facade the desktop frontend and `rustysnes-libretro` already drive the
emulator through — `Board: Send` since `v1.0.0` and the chip-stack crates' `#![no_std]`+`alloc`
posture were both already proven, not new work here). MVP surface: ROM load/close, `run_frame`,
the peripheral setters (Gamepad/Mouse/Super Scope/Multitap), framebuffer + per-frame audio
access, save/load state, reset/power-cycle — 7 host-side unit tests. Verified for real, not just
claimed: a genuine `cargo ndk` cross-compile to `arm64-v8a` produced an actual ARM64 `.so`
(confirmed via `file`), and `uniffi-bindgen` generated real, correctly-shaped Kotlin AND Swift
bindings (inspected for
correct method names/types/`throws`/`@Throws`) from the compiled library. The per-crate `no_std`
CI matrix (`rustysnes-{cpu,ppu,apu,cart,core}` each building standalone, not only transitively
through `rustysnes-core`) also landed here. New `docs/adr/0012-mobile-platform-target.md` +
`docs/mobile-readiness.md` (the living status page); this document's own "Post-v1.0 — Reach"
no-mobile-appetite line is reversed above.

**Honestly scoped**: this development environment has a real Android SDK/NDK but no macOS/Xcode
toolchain, so Android-side work (`v1.15.0`) can be genuinely built/tested here going forward,
while iOS-side work (`v1.16.0`) will be written and Rust-side compile-checked but needs the
project owner's own Mac for a real Xcode build/link/run — see `docs/mobile-readiness.md`.

### `v1.15.0 "Sideload"` — Mobile Phase 2: Android alpha — **RELEASED 2026-07-12**

Delivered: new crate `rustysnes-android`, a presentation-only JNI/`wgpu`-on-`Surface` host (no
emulation logic — the Kotlin shell drives `rustysnes-mobile`'s `MobileCore` directly and hands
this crate exactly `(RGBA8 bytes, width, height)` per frame). New `android/` Gradle project: a
minimal native Kotlin Compose shell (Storage-Access-Framework ROM picker, touch d-pad/face
buttons for the standard SNES gamepad, `AudioTrack` playback), wired to the native crates via
custom `cargoNdkBuild`/`copyCargoLibs*`/`uniffiBindgen` Gradle tasks. Verified for real, not just
claimed: built, installed, and launched on a real Android emulator; a committed permissive test
ROM (`tests/roms/gilyon/cputest/cputest-basic.sfc`) booted through the actual SAF picker and ran
live (advancing framebuffer output across successive screenshots); the background/foreground
lifecycle was exercised (`KEYCODE_HOME` then relaunch) and confirmed to pause/resume correctly
with zero `logcat` errors throughout.

Two real, on-device-only `wgpu` bugs were found and fixed this way — neither reproduces without an
actual `Surface` and driver, so neither was catchable by `cargo ndk check`/clippy:
`SurfaceTargetUnsafe::from_window()` hard-codes a missing display handle (switched to
`from_display_and_window`), and the debug-build default `InstanceFlags` crashed the AVD's
SwiftShader software Vulkan renderer outright (disabled explicitly). A follow-up PR-review pass
then found and fixed a premature `ANativeWindow` release (a real use-after-free-adjacent bug), a
per-frame allocation on the render hot path, a `u32` overflow ordering bug, an `AudioTrack`
cross-thread visibility bug, main-thread ROM loading (an ANR risk), and the frame loop/audio not
pausing while backgrounded — see `CHANGELOG.md`'s `v1.15.0` entry for the full detail on each.

**Honestly scoped ("minimal real MVP now"), deferred to `v1.15.1+`**: Mouse-mode trackpad, Super
Scope drag-reticle, and Multitap pass-and-play seat switcher (net-new SNES-specific touch UX with
no RustyNES precedent); save-state UI; settings screen; `Crt`/`Hqx`/`Xbrz` post-filter wiring;
frame-pacing/vsync-synced render loop (currently a fixed ~60 Hz sleep-paced coroutine);
`.github/workflows/android.yml`; a checked-in `./gradlew` wrapper.

### `v1.16.0 "Beacon"` — Mobile Phase 3: iOS alpha — **RELEASED 2026-07-12**

Delivered: new crate `rustysnes-ios`, a presentation-only `wgpu`-on-`CAMetalLayer` host mirroring
`rustysnes-android`'s (`v1.15.0`) architecture exactly, with a plain C-ABI FFI surface instead of
JNI. New `ios/` SwiftUI shell reusing `v1.15.0`'s Android Compose shell's exact MVP scope and
(now-hardened) lifecycle handling. `ios/project.yml` is an `XcodeGen` YAML spec, not a
hand-authored `.xcodeproj`, avoiding a binary project file this environment could never verify.
New `.github/workflows/ios.yml` builds the `.xcframework` artifacts and runs a real, unsigned
`xcodebuild` simulator build on a `macos-latest` runner — the only place in the project with an
actual Xcode/Swift toolchain.

Verified for real, not just claimed: `cargo build --release --target aarch64-apple-ios` (and the
simulator target) genuinely succeeds in this Linux sandbox with no Xcode/macOS SDK installed
(confirmed via `file` that `librustysnes_ios.a` is a real `Mach-O 64-bit arm64 object`), and
`ios.yml`'s real macOS build genuinely passes — after fixing four real bugs that job's own output
found (a missing executable bit, `dtolnay/rust-toolchain` silently ignoring its inputs when a
`rust-toolchain.toml` is present, a missing `x86_64-apple-ios` simulator slice, and a real Swift
`async`/`await` compile error) and, in a follow-up PR-review pass, three more real runtime/
lifecycle bugs (a `DispatchQueue.main.async`-induced surface-lifecycle race, a missing
`AVAudioSession` activation, and a switch from interleaved to non-interleaved audio buffers to
sidestep a real correctness risk this sandbox can't verify at runtime).

**Honestly unverified**: no on-device or simulator *run* has happened (only a build) — no ROM has
ever actually booted on this platform. No TestFlight upload, no App Store §4.7 self-audit, no real
distribution signing.

### `v1.17.0 "Parity"` — Mobile Phase 4: hardening — **RELEASED 2026-07-12**

Delivered: a single-slot Save State/Load State pair on both mobile shells, calling
`MobileCore.saveState`/`loadState` (already covered by that crate's own host-side round-trip/
garbage-rejection unit tests since `v1.14.0` — no new Rust logic). Verified for real on Android:
rebuilt, reinstalled, and re-tested on the real AVD, confirming via `adb run-as` that a real,
correctly-sized save-state blob round-trips to disk. iOS: written and compile-verified only via
`ios.yml`'s real macOS CI build, matching `v1.16.0`'s standing disposition for that platform.

While re-verifying the Android save-state UI for real, found and fixed a real, pre-existing,
already-shipped native crash present since `v1.15.0`: a `SIGSEGV` in `AudioTrack::write`,
reproducible just by loading a ROM and letting it run for ~10+ seconds — never caught before
because no prior verification pass ran that long. Root cause: per-frame `ShortArray`
allocation/GC churn in the audio path disrupting the native `AudioTrack` buffer's timing; fixed
by reusing a persistent scratch buffer. Also closed a related latent race (`startFrameLoop` made
idempotent). Re-verified stable through 45+ seconds of continuous run plus a full
save/load-state cycle.

**Honestly re-scoped, not silently dropped**: RetroAchievements wiring, an `mlua` `send`-feature
migration, and direct-IP/LAN netplay were all investigated and found not to fit a discrete,
honestly-verifiable change at this rung (see `CHANGELOG.md`'s `v1.17.0` entry for the full
reasoning per item); all three remain on the roadmap for a later mobile-track rung.

### `v1.18.0 "Dormant"` — Mobile Phase 5: monetization scaffolding — **RELEASED 2026-07-14**

Delivered: a new, standalone `rustysnes-monetization` UniFFI crate — dormant entitlement/
ad-pacing policy scaffold (`check_entitlement`, `default_ad_pacing_policy`, `should_show_ad`),
never a dependency of the deterministic core, every concrete pricing/pacing number an explicit
placeholder (unlike RustyNES's own already-committed figure). Wired into both mobile shells as
an inert dependency — compiled in, called once at startup, logged only, no real store SDK, no
UI. Verified for real on Android: rebuilt via a real Gradle build, installed on the real AVD,
launched, and confirmed via `logcat` that the dormant scaffold logs correctly with no crash.
iOS: the Rust `staticlib`/`rlib` outputs cross-compile for real in this development environment;
the `cdylib`/xcframework packaging pipeline needs a real Apple toolchain, so it's
compile-verified via `ios.yml`'s real macOS CI build only — which caught a genuine
`xcodebuild` "Multiple commands produce" failure from two per-crate xcframeworks each
contributing a same-named `module.modulemap` to a shared build-products directory, fixed by
merging `rustysnes-mobile` and `rustysnes-monetization` into one combined
`RustysnesFFI.xcframework`.

A store-launch decision (Play + App Store submission, monetization activation) remains an
explicit maintainer go/no-go against `docs/mobile-readiness.md`, not a numbered rung —
mirroring RustyNES's own still-pending, twice-deferred launch.

### `v1.19.0 "Afterburner"` — PGO/BOLT pipeline — **RELEASED 2026-07-15**

Delivered: `scripts/pgo/run.sh` (instrument → train against the committed permissive ROM corpus
via a new `pgo_trainer` binary → optimized rebuild of the shipping `rustysnes` binary) and
`.github/workflows/pgo.yml` (`workflow_dispatch` + release-tag push only — never the PR gate).
Promotion requires both a `>3%` Criterion speedup over the plain release build **and** a
byte-identical `--features test-roms` re-run under the PGO profile (cites `docs/adr/0004`'s
determinism contract — never promotes on speed alone), deferred until this rung so the profile
isn't invalidated by mobile-specific hot-path work still landing. An optional Linux-only BOLT
post-link stage chains onto an already-promoted PGO binary, best-effort.

Also fixed a real, latent CI gap found while building this: `rust-toolchain.toml` was missing
`llvm-tools-preview`, and `dtolnay/rust-toolchain` silently ignores the `rust-setup` composite
action's own `components:` input whenever a `rust-toolchain.toml` file exists (the same class of
bug already found and fixed for `ios.yml` in `v1.16.0`).

Verified for real in this development environment: the full instrument → train → optimized-
rebuild pipeline produces a genuine, running `rustysnes` binary, and the determinism oracle
passes cleanly under the PGO-merged profile. A PR review caught a real structural bug in the
BOLT stage (re-invoking the whole training script mid-stage instead of chaining `--with-pgo`
onto the already-gathered PGO profile, which could have clobbered the bolt-instrumented binary)
— fixed per `cargo-pgo`'s own documented BOLT+PGO workflow.

**On version numbers:** RustySNES's own SemVer stays independent of RustyNES's — this ladder
reaches RustyNES's current maturity bar at RustySNES's own `v1.19.0`, not a literal "`v2.2.0`."
If the store-launch decision above is ever greenlit, that's the natural point to consider a
`v2.0.0` MAJOR bump (a platform-scope-expanding, non-backward-compatible milestone, matching this
document's own MAJOR-bump rule) — decided then, via the lockstep checklist, not pre-committed here.

### `v1.20.0 "Aperture"` — UI/UX-parity ladder, Phase A — **RELEASED 2026-07-15**

A new ladder, separate from the just-closed RustyNES-parity one: a systematic audit of
RustySNES's menus/settings/debugger against RustyNES's own frontend found the GitHub Pages wasm
demo showing literal `(rebuild with --features X)` placeholders for two features
(`cheats`/`debug-hooks`) that were never actually excluded for any architectural reason, plus a
desktop peripheral-input gap (the Settings Mouse/Super Scope selector wired the emulated hardware
but nothing ever captured host pointer input) and two small, named catch-up items already on
record (`to-dos/ROADMAP.md`'s "Milestones beyond the phases", `to-dos/LOCKSTEP-CHECKLIST.md`'s
2026-07-15 entry).

Delivered: `.github/workflows/web.yml`'s `trunk build` gained `--features cheats,debug-hooks`,
making the wasm demo's Tools → Cheats and Debug → Debugger overlay menu items real; a new
`crate::peripherals` module feeds `egui::Context`'s pointer state into `EmuCore::set_mouse`/
`set_superscope` once per frame, mapped through the same letterbox transform the present path
already uses (real bugs found and fixed in review: mouse deltas needed `pixels_per_point` +
letterbox rescaling to stay window-size/DPI-independent, and Super Scope needed to map into the
SNES's fixed base 256×224/239 screen space rather than the PPU's raw, possibly pixel-doubled
`fb_dims`); a View → Hide Overscan toggle crops the SNES's `SETINI`-extended 239-line display
back to 224 by a height FRACTION (stays exact under HD-pack's own integer upscale), applied
against the finalized presented `dims` (a real desync bug found and fixed in review — an earlier
pre-captured flag could read a stale resolution across a run-ahead/`emu-thread` PresentBuffer
handoff); and a new Debug → ROM Info panel (CRC32/SHA-256/header decode of the loaded cart,
captured once per successful load rather than every frame) closes the ROM-Info-panel catch-up
item, gained a decoded `title: String` field on `rustysnes_cart::header::Header` along the way,
and — per a real review finding — only hashes the ROM when `debug-hooks` is on and the load
actually succeeded, not on every attempt.

Also fixed a real, separate finding surfaced while scoping the peripheral-input work:
`docs/frontend.md`'s own "Status" line claimed controller port 1 had "keyboard + gilrs gamepad"
input, but `gilrs::Gilrs` is never actually instantiated anywhere in `rustysnes-frontend` — port
1 is keyboard-only today. Corrected the doc; real gamepad support (and the Super Multitap
sub-pad host input it blocks) is a genuinely separate, larger prerequisite, tracked in the
UI/UX-parity plan's backlog, not silently expanded into this rung.

Verified for real: a local `trunk build --release` reproducing the exact CI command (gzip size
2.96 MiB, well under the 5 MiB budget); the full local regression gate (fmt/test/clippy across
every feature combination including `wasm32-unknown-unknown`/doc-warnings-as-errors) green on
every PR; every `gemini-code-assist`/`copilot-pull-request-reviewer` finding adjudicated and its
GitHub thread resolved before merge, including two independently-confirmed real bugs (the mouse/
Super Scope scaling issues) and a real presented-frame desync (the overscan flag). Phases B and C
of the UI/UX-parity plan (in-app Help docs, deeper debugger panels, wasm Lua scripting via
`piccolo`, browser netplay lobby, browser RetroAchievements, i18n) remain scoped but not started —
each is its own future rung, sized like a small roadmap phase on its own.
