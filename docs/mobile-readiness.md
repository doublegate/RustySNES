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

## Not yet verified / explicitly deferred

- **No real Android app or emulator run yet.** `v1.15.0 "Sideload"` is where the actual Kotlin
  Compose shell, JNI host, `SurfaceView`-backed `wgpu` surface, and AAudio sink land. An AVD
  exists in this environment (`Pixel_8.avd`) but currently fails to load (`Google pixel_8 no
  longer exists as a device` — a stale device-definition mismatch, not an emulator-infrastructure
  problem); fixing that is a `v1.15.0` task, not blocking this rung.
- **No iOS build/link/run at all.** This development environment has no macOS/Xcode toolchain.
  `v1.16.0 "Beacon"`'s `rustysnes-ios` crate and SwiftUI shell will be written and Rust-side
  compile-checked wherever `cargo build --target aarch64-apple-ios` (or the simulator target)
  succeeds without needing Xcode itself, but the real build/link/run/on-device or
  on-simulator verification needs the project owner's own Mac — this will be flagged explicitly
  at that point, not silently claimed as done.
- **No touch UX yet** — the Mouse-mode trackpad, Super Scope drag-reticle, and Multitap
  pass-and-play seat switcher are net-new SNES-specific UI with no RustyNES desktop precedent to
  port; `v1.15.0`'s own scope.
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
