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

### v0.7.0 "Resolution" — true hi-res output

The one item from `v0.5.0`'s carried-forward PPU gap list that's genuinely bounded, with no
external unknowns (no ROM sourcing, no open-ended root-causing): full 512-px hi-res (Modes 5/6)
output. Confirmed against ares' `DAC::run()` that hi-res is a dual-half-pixel
alternating-compositor-result trick (`above`/`below` at 2× the pixel rate), not a numeric-
precision tweak — the mechanism is already researched (`docs/ppu.md` §Hi-res color-math
precision); this rung implements the 512-wide output itself, closing the real feature gap that
was blocking the Bishoujo Janshi Suchie-Pai / Marvelous+SA-1 residual. Kept as its own minor
rather than folded into the accuracy-debt cluster below, matching this project's
one-theme-per-`v0.x.0` convention every prior rung has held to.

### Ongoing, opportunistic — the carried-forward accuracy-debt cluster (v0.x.y patches, not a gating rung)

Open-ended research items, not bounded deliverables — several have already carried forward
across `v0.3.0`→`v0.6.0` without closing, so none of them gates a numbered `v0.x.0` rung above.
Land each independently as a patch release whenever its investigation actually concludes,
re-running the full `--features test-roms` suite before landing anything, per this project's
established discipline:

- **Mid-scanline/HDMA-driven register timing + the Super FX/GSU regression** — a fix is
  designed, prototyped, and SA-1-verified-correct, but blocked on an unexplained regression
  across all 24 GSU golden tests (`docs/ppu.md` §Mid-scanline/HDMA-driven register timing).
  Resume design work on the `investigate/gsu-mid-scanline-interaction` branch (the fix's
  mechanism is fully described in `docs/ppu.md` §The prototype fix, reproducible from that
  write-up alone if the branch's own local state isn't available); next step is the
  access-level trace of GSU VRAM/CGRAM writes that doc's "What a future investigation needs"
  section calls for.
- **Open-bus-via-HDMA-latch** — an independent prototype for this fix hit the **identical**
  all-24-GSU-goldens failure signature as the item above (`docs/scheduler.md` §Open bus via
  DMA/HDMA). Two independent fixes hitting the same unexplained failure shape is a real signal
  this area of the master-clock tick sequence is fragile in a way not yet understood — treat
  both investigations as linked, not coincidental; a resolution to one may unlock the other.
- **SPC7110 boot gap** — needs a real 65C816 disassembler as prerequisite tooling before the
  crash-point trace can proceed (`docs/audit/spc7110-boot-crash-2026-07-08.md`'s own "suggested
  next steps"); building that disassembler is useful debugging infra beyond this one bug.
- **DRAM refresh (40 clocks/scanline)** — researched, not yet implemented; a real architectural
  timing tension needs resolving empirically before landing, not a simple port.
- **ST018 / S-RTC / PAL / ExLoROM real-ROM validation** — currently unit-test-only or
  formula-level only, blocked purely on sourcing a commercial dump for each. Keep the
  opportunistic posture already established across four prior releases; don't gate a rung on
  finding ROMs that may never become available.

### v0.8.0 "Instrumentation" — debugger, scripting/TAS, cheats

One coherent theme: tooling that inspects and manipulates emulator state, all sharing the same
memory-watch/introspection substrate. This is the first rung of the breadth pass — RustySNES's
own crate manifest already anticipated it (`crates/rustysnes-frontend/Cargo.toml`'s `debug-hooks`/
`scripting` flags, present since the frontend's first cut, "mirroring the RustyNES feature
surface so the chip-implementation phase can switch them on without restructuring the manifest").

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
- **Recurring gate, starting here:** a byte-identical-with-all-flags-off CI check (every new
  flag landed on this ladder must leave the default build unchanged) — re-verify after `v0.9.0`
  and `v1.0.0` too. Use explicit feature combos in CI, never `--all-features`.

### v0.9.0 "Community" — netplay, RetroAchievements

Both wire pre-existing stub crates against a named integration pattern; both are "connect
RustySNES to other players or an external service."

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

### v1.0.0 — desktop UX shell maturity, performance engineering, production cut

- **Fix `Board: Send` first.** `cargo check -p rustysnes-frontend --features emu-thread` fails
  today — `dyn Board` isn't `Send`, breaking `Arc<Mutex<EmuCore>>: Send` for the thread closure.
  No interior non-`Send` state exists in `rustysnes-cart`'s coprocessor code, so this really is
  the "one-word change" the Cargo.toml comment already diagnoses — verify it stays that simple,
  then wire the dedicated emulation thread behind `emu-thread`.
- **Desktop UX shell maturity** — the shell (menu bar, status bar, 4-tab Settings) already
  exists; this rung finishes it to RustyNES's bar: a thumbnail save-state manager (multi-slot,
  replacing today's single quick-save slot), the Settings Input tab's key-rebind grid (currently
  a literal `"TODO"` in source), light/dark/system themes, fullscreen, speed presets (25–300%),
  per-channel audio mutes, a first-run welcome modal, and a Performance panel with histograms.
- **New frame-time performance-regression CI gate** — RustyNES has one, RustySNES doesn't yet.
- **Save-state `FORMAT_VERSION` backward-compat fixture + regression test** — the real, small,
  still-open gap flagged above (v1.0.0 gate item 2, `v0.6.0`-era): only same-version round-trip
  is tested today, not actual old-format loadability. Lands here.
- **README.md rewrite** to RustyNES's README depth (structure/length/technical detail) — lands
  here, once the feature set it needs to document is actually complete.
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

## Post-v1.0 — Reach (deferred)

- **Libretro core**, a **shader/filter pipeline** (CRT/HQ2x), **HD texture packs** (wires the
  already-scaffolded `hd-pack` flag), the **fractional-timebase MAJOR refactor**
  (`docs/adr/0002`, only if hard residuals from the accuracy-debt cluster above actually warrant
  it), and any future **mobile/Android** target (no appetite assumed by default, unlike
  RustyNES's own Android build — don't inherit that scope blindly).

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
