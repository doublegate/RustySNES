# Mobile readiness

Tracks the RustyNES-parity roadmap's mobile track (`v1.14.0 "Foundry"` through `v1.18.0
"Dormant"`) — what exists, what's verified, and what's still needed before a real store
submission. See `docs/adr/0012-mobile-platform-target.md` for the platform-target decision
itself; this doc is the living status page, not the decision record.

**The RustyNES-parity roadmap itself closed at `v1.19.0 "Afterburner"`** (2026-07-15) — every
planned rung, mobile and non-mobile alike, has shipped. The only thing this doc still tracks as
open is the standing **Mobile Phase 6** store-launch gate below; see `docs/STATUS.md`'s "Current
release" pointer and `to-dos/VERSION-PLAN.md` for the closed-out ladder itself.

## Architecture

`crates/rustysnes-mobile` is a thin `UniFFI` bridge over `rustysnes_core::facade::EmuCore` — the
same facade `rustysnes-frontend` and `rustysnes-libretro` already drive the emulator through. It
adds no new emulation logic; every method is a direct, FFI-safe wrapper (see the crate's own
module doc for the full rationale and threading model).

**Surface (`v1.14.0`):** `MobileCore::new`, `load_rom`/`close_rom`/`rom_loaded`,
`reset`/`power_cycle`, `run_frame`, `framebuffer`/`frame_size`, `drain_audio`, `set_pad`/
`set_port_device`/`set_mouse`/`set_superscope`/`set_multitap_pad`, `save_state`/`load_state`.
`drain_audio` is **non-destructive** — it returns the current frame's buffered samples, not a
FIFO/pop-style drain; call it exactly once per `run_frame` (which clears and refills the buffer
at its own start), mirroring `EmuCore::audio`'s own contract that every existing consumer
(`rustysnes-frontend`'s synchronous and `emu-thread` render paths alike) already relies on.

**Deliberately NOT in scope yet** (honest deferral, not silent gaps — every one of these is a
real, separate concern layered on top of `EmuCore` in the desktop build too, not something this
bridge needs to re-invent to reach a playable MVP):

- HD-pack texture-pack consumption
- Cheats, rewind, run-ahead
- Netplay
- `RetroAchievements`
- Lua/TAS scripting

**Mobile UI**: a native Compose (Android) / SwiftUI (iOS) shell over a raw `wgpu` surface — not
an attempt to run `egui` with touch input (`docs/adr/0012`'s decision). `rustysnes-gfx-shaders`
(`v1.12.0`) exists specifically so this shell can reuse `BLIT_WGSL`/`CRT_WGSL`/`HQX_WGSL`/
`XBRZ_WGSL` verbatim without pulling in `rustysnes-frontend`'s winit/egui/cpal dependency graph.

**Android (`v1.15.0 "Sideload"`)**: `crates/rustysnes-android` is a presentation-only JNI/`wgpu`
host — it owns no emulation state. The Kotlin shell (`android/`) drives `rustysnes-mobile`'s
`MobileCore` directly through its own UniFFI bindings and hands `rustysnes-android` exactly
`(RGBA8 framebuffer bytes, width, height)` once per frame via
`Java_com_doublegate_rustysnes_NativeRenderer_nativePresentFrame`, mirroring
`rustysnes-frontend`'s renderer/emulation-core separation across a JNI boundary instead of an
in-process crate boundary. `BLIT_WGSL` only (unfiltered) for this MVP — the `Crt`/`Hqx`/`Xbrz`
post-filters aren't wired here yet.

**iOS (`v1.16.0 "Beacon"`)**: `crates/rustysnes-ios` is the same shape as `rustysnes-android`,
adapted for the platform — a presentation-only `wgpu`-on-`CAMetalLayer` host with no emulation
state, exposing a plain C-ABI FFI (`ios/RustySNES/Bridging-Header.h`) instead of JNI. The SwiftUI
shell (`ios/`) drives `MobileCore` through UniFFI-generated Swift bindings and hands
`rustysnes-ios` exactly `(RGBA8 framebuffer bytes, width, height)` per frame via
`rustysnes_ios_present_frame`. `ios/project.yml` is an `XcodeGen` spec, not a hand-authored
`.xcodeproj` — see "Verified so far (`v1.16.0`)" below for why.

## Verified so far (`v1.14.0`)

- `cargo build`/`cargo test -p rustysnes-mobile` on the host: 7 unit tests covering ROM load
  (success + empty-image rejection), framebuffer sizing after a real frame, save-state
  round-trip, save-state garbage rejection, and every peripheral setter not panicking without a
  loaded ROM.
- `uniffi-bindgen generate --library <compiled .so> --language kotlin` produces real, correctly
  named Kotlin (`loadRom`, `runFrame`, `frameSize`, `drainAudio`, `setPortDevice`, ... — every
  method present, correct types, `@Throws` on the fallible ones).
- The same, for `--language swift` — correct `func` signatures, `Data` for byte buffers, `throws`
  on the fallible methods.
- A **real cross-compile to `arm64-v8a`** via `cargo ndk -t arm64-v8a build -p rustysnes-mobile`
  against this environment's actual NDK (r29), producing a genuine
  `ELF 64-bit LSB shared object, ARM aarch64 ... for Android 21, built by NDK r29` — confirmed via
  `file`, not just a successful exit code.
- Per-crate `no_std` CI matrix (`rustysnes-{cpu,ppu,apu,cart,core}` each build standalone against
  `thumbv7em-none-eabihf --no-default-features`), replacing the prior single aggregate-only job.

## Verified so far (`v1.15.0`)

A working `RustySNES_Test` AVD (Pixel 7 profile, `android-34/google_apis_playstore/x86_64`) was
created fresh in this environment (the pre-existing `Pixel_8.avd` mentioned in the `v1.14.0`
entry above was left untouched, not fixed — a new AVD sidestepped its stale device-definition
mismatch without touching whatever state that one held).

- `cargo ndk -t arm64-v8a -t x86_64 build/clippy -p rustysnes-android -p rustysnes-mobile`:
  real cross-compiles for both ABIs, zero clippy warnings (`-D warnings`), `cargo fmt --check`
  clean.
- The full Gradle build (`:app:assembleDebug`, wired through `cargoNdkBuild` +
  per-ABI `copyCargoLibs*` + `uniffiBindgen`) produces a real, installable debug APK.
- **Installed and launched on the real AVD**: `adb install` succeeded, the app launched with no
  crash, and a pulled screenshot confirmed the Compose UI (ROM picker button, d-pad, face
  buttons) rendering correctly over a live `wgpu` `SurfaceView` — not a placeholder.
- **A real ROM booted and ran**: pushed a committed permissive test ROM
  (`tests/roms/gilyon/cputest/cputest-basic.sfc`) to the device, drove the Storage-Access-
  Framework picker via `adb`/`uiautomator`, and confirmed via successive screenshots that the
  framebuffer is live and advancing (`Test number: 0185` → `Test number: 0452`, "Success"
  progressing) — genuine per-frame rendering through the fixed `wgpu` pipeline, not a static
  image. Zero errors in `logcat` throughout.
- Two real, on-device-only bugs were found and fixed this way (not caught by any host-side
  `check`/`clippy`/unit test, since neither reproduces without an actual `Surface` and a real —
  even software — Vulkan/GLES driver): the `SurfaceTargetUnsafe::from_window` missing-display-
  handle error, and the SwiftShader-crashing default `InstanceFlags`. See `CHANGELOG.md`'s
  `[Unreleased]` entry for the technical detail on both.

## Verified so far (`v1.16.0`)

- `cargo build --release --target aarch64-apple-ios -p rustysnes-ios` and the same for
  `aarch64-apple-ios-sim`: real cross-compiles, both succeeding in this Linux sandbox with no
  Xcode/macOS SDK installed — confirmed possible because a `staticlib` target only needs the
  downloaded `rust-std` component (no link against Apple's frameworks happens until Xcode's own
  final link step, which this environment never reaches). Confirmed via `file` that the produced
  `librustysnes_ios.a` contains a real `Mach-O 64-bit arm64 object`, not a stub.
- `cargo clippy --target aarch64-apple-ios --all-targets -p rustysnes-ios -- -D warnings` and
  `cargo fmt --check`: clean for both the device and simulator targets.
- Unlike `rustysnes-android`, this crate ALSO type-checks, lints, and its one host-runnable test
  (`blit_wgsl_validates`) passes cleanly against the plain Linux host target — `raw-window-handle`
  and `wgpu`'s `UiKit`/Metal types are portable at the type level even off-Apple platforms, so
  `cargo test -p rustysnes-ios` genuinely runs and passes here, and this crate needs no CI
  workspace exclusion (`cargo test --workspace`/`cargo clippy --workspace` already cover it).
- `oslog` (the natural `android_logger` analog) was tried and dropped: its build script shells
  out to `xcrun` to locate the iOS SDK, which doesn't exist in this sandbox — confirmed by
  actually hitting `failed to find tool "xcrun"`. No logger is installed by this crate yet.
- **`.github/workflows/ios.yml`'s real `macos-latest` build genuinely passes**, after 4 rounds of
  real fixes driven entirely by that job's own output (not guessed): a missing executable bit on
  `scripts/build-ios-xcframework.sh`; `dtolnay/rust-toolchain` silently ignoring its
  `toolchain:`/`targets:` inputs whenever a `rust-toolchain.toml` is present (fixed via an explicit
  `rustup target add` step); a missing `x86_64-apple-ios` simulator slice (`generic/platform=iOS
  Simulator` wants both simulator architectures regardless of the runner's own CPU, fixed via a
  `lipo`-merged universal simulator library); and one real Swift compile error
  (`AVAudioPlayerNode.scheduleBuffer`'s `async` overload needing `await`). A follow-up PR-review
  pass then found and fixed a real `DispatchQueue.main.async`-induced surface-lifecycle race, a
  missing `AVAudioSession` activation (iOS is audibly silent without it), and switched the audio
  buffer format from interleaved to non-interleaved Int16 to sidestep a real, plausible
  `int16ChannelData` correctness risk this sandbox can't verify at runtime. See `CHANGELOG.md`'s
  `v1.16.0` entry for the full detail.

## Verified so far (`v1.17.0`)

- **Save State / Load State on both shells** — a single slot, calling `MobileCore.saveState`/
  `loadState` (already covered by that crate's own host-side round-trip/garbage-rejection unit
  tests since `v1.14.0`). **Android**: rebuilt, reinstalled, and re-tested on the real AVD —
  saved mid-run, let the emulator advance, tapped Load State, and confirmed via `adb run-as` that
  a real, correctly-sized (~497KB) save-state blob was written to app-private storage and
  `loadState` returned with no exception logged. The visual test-ROM counter converges to a fixed
  terminal state too quickly after the load to serve as an unambiguous rewind indicator with this
  specific ROM, so the file-existence + no-exception evidence, layered on the already-tested
  Rust-level round-trip logic, is the real verification signal here. **iOS**: written and
  compile-verified via `ios.yml`'s real macOS CI build; no on-device/simulator run, matching
  `v1.16.0`'s standing disposition for this whole platform.
- Fixed a real, pre-existing gap: the Android `versionName` had been left at `1.15.0` through
  both the `v1.15.0` and `v1.16.0` releases (iOS's `MARKETING_VERSION` already got this right in
  `v1.16.0`). Both now correctly read `1.17.0`.
- **Found and fixed a real, pre-existing, already-shipped Android crash** (present since
  `v1.15.0`): a native `SIGSEGV` in `AudioTrack::write`, reproducible just by loading a ROM and
  letting it run for ~10+ seconds — never caught before because no prior verification pass ran
  the app that long. Root cause: per-frame `ShortArray` allocation/GC churn in the audio path
  disrupting the native `AudioTrack` buffer's timing; fixed by reusing a persistent scratch
  buffer. Re-verified stable through 45+ seconds of continuous run plus a full save/load-state
  cycle. See `CHANGELOG.md`'s `[Unreleased]` entry for the full technical detail, including the
  earlier `v1.15.0` PR review comment that flagged this same code as "perf-only" — this rung
  found that disposition was wrong.

### Honestly re-scoped this rung (not silently dropped)

RetroAchievements wiring, an `mlua` `send`-feature migration, and direct-IP/LAN netplay were all
originally planned for this rung and were investigated, not silently skipped: `rustysnes-cheevos`'s
`RaClient` is callback-based (async HTTP completions via `on_done` closures), which doesn't map
onto UniFFI's synchronous call model without real bridging design work, and would also need
cross-compiling `rcheevos`'s vendored C library for Android NDK ABIs and iOS device/simulator
triples it doesn't currently target — genuine engineering, not a scoped addition. The `mlua`
migration was explicitly gated on "Lua/TAS-on-mobile being greenlit," which hasn't happened.
Netplay is a large, net-new UI surface neither shell has any precedent for. See `CHANGELOG.md`'s
`[Unreleased]` entry for the full reasoning; all three remain on the roadmap for a later rung.

## Verified so far (`v1.18.0`)

- **`rustysnes-monetization`** (Mobile Phase 5): a new, standalone UniFFI crate — dormant
  entitlement/ad-pacing policy scaffold, never a dependency of the deterministic core, every
  concrete pricing/pacing number an explicit placeholder (unlike RustyNES's own already-committed
  figure). Fully verified on host: `cargo test -p rustysnes-monetization` (5/5 passing),
  `cargo clippy -p rustysnes-monetization --all-targets -- -D warnings` (clean),
  `RUSTDOCFLAGS="-D warnings" cargo doc -p rustysnes-monetization --no-deps` (clean),
  `cargo fmt --check -p rustysnes-monetization` (clean).
- **Wired into both mobile shells as an inert dependency** — compiled in, called once at startup,
  logged only, no real store SDK, no UI. **Android**: `build.gradle.kts`'s `cargoNdkBuild` task now
  builds all three native crates and a second, separate `uniffiBindgenMonetization` task generates
  this crate's own Kotlin bindings (its own UniFFI namespace, so it can't share `rustysnes-mobile`'s
  generated-sources directory). Rebuilt via a real Gradle build against the locally cached Gradle
  8.13 distribution (same no-`gradlew`-wrapper disposition already tracked below), installed on
  the real AVD, launched, and confirmed via `logcat`:
  `monetization scaffold (dormant): unlocked=true minIntervalSecs=300 sessionsBeforeFirstAd=3`;
  the app process stayed alive afterward (no crash). **iOS**: `scripts/build-ios-xcframework.sh`
  gained a third crate to build/package, merged with `rustysnes-mobile` into one combined
  `RustysnesFFI.xcframework` rather than two separate per-crate ones — a real macOS CI run caught
  a genuine `xcodebuild` "Multiple commands produce '.../include/module.modulemap'" failure: a
  "library"+`-headers` xcframework has its headers copied into one directory shared across every
  such xcframework linked into the target, so two xcframeworks each contributing a same-named
  `module.modulemap` collided. Fixed by `libtool -static`-merging both crates' `.a`s per platform
  slice and combining their modulemaps into one umbrella module before packaging a single
  xcframework; `ios/project.yml`'s dependency list updated to match. The Rust side's
  `staticlib`/`rlib` outputs for `aarch64-apple-ios` cross-compile for
  real in this development environment (confirmed: identical to `rustysnes-ios`'s own already-
  established precedent). The `cdylib` output the bindgen/xcframework packaging step needs does
  NOT link here — confirmed this is a pre-existing sandbox limitation, not something this rung
  introduced (an identical `cc: error: unrecognized command-line option '-target'` failure
  reproduces for `rustysnes-mobile`'s own pre-existing `cdylib` build in isolation, with or without
  `rustysnes-monetization` in the same invocation) — so the full xcframework/`xcodebuild` pipeline
  is compile-verified via `ios.yml`'s real macOS CI build only, matching this platform's standing
  "scaffolded-only" disposition since `v1.16.0`.

## Not yet verified / explicitly deferred

- **No Mouse/Super Scope/Multitap touch UX yet** — net-new SNES-specific UI with no RustyNES
  desktop precedent to port; deferred under the "minimal real MVP now" scope chosen for the
  Android rung and reused as-is for iOS (P1 standard gamepad only, in-app ROM picker, blit-only
  rendering, no settings screen).
- **No `android.yml` CI workflow yet** — NDK cross-build, UniFFI Kotlin smoke test, 16KB ELF
  page-alignment check, dormant Play-flavor Gradle split — `v1.15.1+`.
- **No checked-in `./gradlew` wrapper yet** — this environment used its locally cached Gradle
  8.11 distribution directly; a proper wrapper should still be generated/committed for
  reproducibility — `v1.15.1+`.
- **No on-device or simulator *run* has happened — only a build.** This development environment
  has no macOS/Xcode toolchain at all, so nothing here can be run interactively;
  `.github/workflows/ios.yml`'s `macos-latest` job (real `xcodegen generate` + unsigned
  `xcodebuild` simulator build) is the only real verification this Swift/Xcode code has ever had,
  and it now genuinely passes (see "Verified so far" above) — but a passing build proves the code
  compiles and links, not that it behaves correctly at runtime (no ROM has ever actually booted
  here).
- **No TestFlight upload, no App Store §4.7 self-audit, no real distribution signing** — the
  `ios.yml` step exists but is an explicit no-op pending the project owner provisioning real
  signing secrets.
- **No store-submission readiness assessment yet** — see "Mobile Phase 6 — store-launch gate
  status" below for the full gate criteria and current disposition.

## Mobile Phase 6 — store-launch gate status

**STATUS: NOT GREENLIT.** No store-submission readiness assessment has occurred. This is an
explicit maintainer go/no-go decision against this document, reviewed and re-decided each time
the mobile track advances — not an automatic outcome of shipping `v1.14.0`-`v1.18.0`, and not
tied to any fixed RustySNES version number. It mirrors RustyNES's own real precedent: that
project's analogous launch decision has already been deferred twice (`v2.1.0` → `v2.2.0` →
`v2.3.0`), so a "the mobile track exists" milestone being reached is explicitly NOT the same
thing as "ready to submit."

**What's done (the prerequisite mobile track, `v1.14.0`-`v1.18.0`, all shipped and verified —
see "Verified so far" above for the per-release detail):** the `rustysnes-mobile` UniFFI bridge,
a real Android alpha with save/load-state and a real, found-and-fixed native audio crash, a real
iOS alpha compile-verified via macOS CI, and the dormant `rustysnes-monetization` scaffold wired
into both shells as an inert, non-gating call.

**What would still need to happen before Phase 6 could even be considered** (not started; listed
here so a future go/no-go review has a concrete checklist, not a vague "more polish"):

- **Android:**
  - Mouse/Super Scope/Multitap touch UX (currently P1 standard gamepad only)
  - `.github/workflows/android.yml` — NDK cross-build CI, UniFFI Kotlin smoke test, 16KB ELF
    page-alignment check (a real Play Store requirement on current API levels)
  - A committed `./gradlew` wrapper (this environment has only ever used a locally cached Gradle
    distribution directly)
  - Play Store's own submission requirements: a Data Safety form, target API level compliance
    check, a signed release (not debug) build, a dormant-vs-live Play Billing decision for
    `rustysnes-monetization` if monetization is ever actually activated
- **iOS:**
  - A real on-device or simulator *run* — every verification so far is compile/link-only (this
    development environment has no macOS/Xcode toolchain at all); `ios.yml`'s passing
    `xcodebuild` build proves the code compiles, not that it behaves correctly at runtime
  - TestFlight distribution signing secrets provisioned (the `ios.yml` upload step is currently
    an explicit no-op pending these)
  - The App Store §4.7 self-audit (user-sourced ROM import only, no Nintendo trademark exposure
    in Super Scope/peripheral naming or art) — flagged as still-fresh in `v1.16.0`'s scope, not
    yet formally re-confirmed against the shipped UI
- **Both platforms:** the store-submission readiness assessment itself — an explicit, written
  maintainer review against this checklist, not an implicit "it built, so it's ready" inference.

**How to apply:** if a future session is asked to "get mobile ready to ship" or similar, treat
this section as the actual scope — don't re-litigate whether the mobile *track* exists (it does,
fully) and don't assume "the roadmap is done" implies "Phase 6 is greenlit" (it explicitly does
not). Update this section's checklist and status line as items above get addressed, and only
change the STATUS line itself on an explicit maintainer decision.

## Risk context (not re-litigated here, only cited)

RustyNES's own mobile bridge absorbed nine patch releases (`v2.0.1`-`v2.0.9` "Harbor")
re-porting after a scheduler rewrite broke its save-state/movie format. RustySNES carries the
same NAMED risk (`docs/adr/0002`, the fractional-timebase refactor) — already assessed and found
**not currently warranted** at `v1.1.0`
(`docs/audit/fractional-timebase-go-no-go-2026-07-11.md`), with zero save-state-format churn
since (`FORMAT_VERSION` last bumped `v0.9.0`, five stable minors plus the entire
`v1.5.0`-`v1.13.0` ladder shipped since). The mobile track does not need to wait on this; it's a
fact informing risk, not an open decision.
