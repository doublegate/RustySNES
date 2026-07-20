# Changelog

All notable changes to RustySNES will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **RustySNES integrates a cycle-accurate emulation engine.** Modeled after its predecessor `RustyNES`, this emulator is built on a master-clock-precise, lockstep-scheduled core targeting the Mesen2/ares accuracy bar. The entries below document the engine-internal milestones as this core is built and hardened.

## [Unreleased]

### Added

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

### Changed

- **`hdmaen_latch_test` / `hdmaen_latch_test_2` golden framebuffers re-blessed** as a direct,
  intended consequence of the V-IRQ fix: both ROMs gate their `STA $420C` on a V-only IRQ, so
  firing once per frame rather than on every dot changes which dot the write lands on and hence the
  banding realization. Legitimate only because these goldens are regression snapshots of our own
  deterministic output — undisbeliever documents the ROM as *not stable* on real hardware — and
  because the change is externally corroborated (ares' edge detector; Mesen2 and snes9x both pass
  `B4.08`/`B4.12`, which RustySNES failed). Isolated by reverting the IRQ change alone and
  confirming the goldens returned. Rationale recorded in `docs/scheduler.md` §H/V-IRQ.

### Fixed

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

### Fixed

- **A real, separate finding surfaced while scoping the above**: `docs/frontend.md`'s own "Status"
  line claimed controller port 1 had "keyboard + gilrs gamepad" input, but `gilrs::Gilrs` is never
  actually instantiated anywhere in `rustysnes-frontend` — confirmed via `input::gamepad_button`
  (the gilrs-button-name mapping function) having zero callers. Port 1 is keyboard-only today.
  Corrected the doc; wiring real gamepad support is a genuinely separate, larger prerequisite
  (a live `Gilrs` instance + per-frame event polling), not something silently expanded into this
  fix — it's also what blocks Super Multitap sub-pad 1-3 host input specifically, tracked
  separately in the UI/UX-parity plan's backlog.

### Added

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
