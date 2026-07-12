# Mobile readiness

Tracks the RustyNES-parity roadmap's mobile track (`v1.14.0 "Foundry"` through `v1.18.0
"Dormant"`) — what exists, what's verified, and what's still needed before a real store
submission. See `docs/adr/0012-mobile-platform-target.md` for the platform-target decision
itself; this doc is the living status page, not the decision record.

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

## Not yet verified / explicitly deferred

- **No Mouse/Super Scope/Multitap touch UX yet** — net-new SNES-specific UI with no RustyNES
  desktop precedent to port; deferred to `v1.15.1+` under the "minimal real MVP now" scope chosen
  for this rung (P1 standard gamepad only, in-app ROM picker, blit-only rendering, no save-state
  UI or settings screen).
- **No `android.yml` CI workflow yet** — NDK cross-build, UniFFI Kotlin smoke test, 16KB ELF
  page-alignment check, dormant Play-flavor Gradle split — `v1.15.1+`.
- **No checked-in `./gradlew` wrapper yet** — this environment used its locally cached Gradle
  8.11 distribution directly; a proper wrapper should still be generated/committed for
  reproducibility — `v1.15.1+`.
- **No iOS build/link/run at all.** This development environment has no macOS/Xcode toolchain.
  `v1.16.0 "Beacon"`'s `rustysnes-ios` crate and SwiftUI shell will be written and Rust-side
  compile-checked wherever `cargo build --target aarch64-apple-ios` (or the simulator target)
  succeeds without needing Xcode itself, but the real build/link/run/on-device or
  on-simulator verification needs the project owner's own Mac — this will be flagged explicitly
  at that point, not silently claimed as done.
- **No store-submission readiness assessment yet** — that's the standing "Mobile Phase 6"
  go/no-go gate in `to-dos/ROADMAP.md`, deliberately not tied to a fixed version.

## Risk context (not re-litigated here, only cited)

RustyNES's own mobile bridge absorbed nine patch releases (`v2.0.1`-`v2.0.9` "Harbor")
re-porting after a scheduler rewrite broke its save-state/movie format. RustySNES carries the
same NAMED risk (`docs/adr/0002`, the fractional-timebase refactor) — already assessed and found
**not currently warranted** at `v1.1.0`
(`docs/audit/fractional-timebase-go-no-go-2026-07-11.md`), with zero save-state-format churn
since (`FORMAT_VERSION` last bumped `v0.9.0`, five stable minors plus the entire
`v1.5.0`-`v1.13.0` ladder shipped since). The mobile track does not need to wait on this; it's a
fact informing risk, not an open decision.
