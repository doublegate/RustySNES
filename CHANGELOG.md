# Changelog

All notable changes to RustySNES will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **RustySNES integrates a cycle-accurate emulation engine.** Modeled after its predecessor `RustyNES`, this emulator is built on a master-clock-precise, lockstep-scheduled core targeting the Mesen2/ares accuracy bar. The entries below document the engine-internal milestones as this core is built and hardened.

## [Unreleased]

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

This release landed across PRs #44 and #45, each independently reviewed by Gemini + Copilot
(including two AI-reviewer suggestions investigated and rejected with primary-source citations
against ares' `dac.cpp` — see PR #45's review threads), human-reviewed, and adjudicated before
merge.

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
  overlay, Lua scripting + TAS movie API, cheat-code support), `v0.9.0 "Community"` (rollback
  netplay, RetroAchievements), then `v1.0.0` (desktop UX shell maturity, a new frame-time
  performance-regression CI gate, the `README.md` rewrite, the production cut).
  `to-dos/VERSION-PLAN.md`, `to-dos/ROADMAP.md`, and `to-dos/phase-8-reach/overview.md` (plus its
  sprint files, renumbered: Sprint 1 = Instrumentation/`v0.8.0`, Sprint 2 = Community/`v0.9.0`,
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
+ `cargo deny check`) is also green, its first real end-to-end run.

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
