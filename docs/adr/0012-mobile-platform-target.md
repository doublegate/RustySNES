# ADR 0012 — Mobile platform target (Android + iOS via a `UniFFI` bridge)

## Status

Accepted (`v1.14.0 "Foundry"`).

## Context

`to-dos/VERSION-PLAN.md`'s "Post-v1.0 — Reach (deferred)" section recorded, since `v1.0.0`: "any
future mobile/Android target (no appetite assumed by default, unlike RustyNES's own Android
build — don't inherit that scope blindly)." That default has now been explicitly reversed by the
project owner as part of scoping the RustyNES-parity roadmap's `v1.14.0`-`v1.18.0` mobile track.

Two prerequisites this decision depends on were already in place before this ADR, not something
landing alongside it:

- `rustysnes_cart::Board: Send` since `v1.0.0` (the same trait bound `emu-thread` already relies
  on) — a mobile host driving the emulator from a dedicated render/audio thread needs the same
  guarantee `emu-thread` already proved out on desktop.
- Every chip crate (`rustysnes-{cpu,ppu,apu,cart}`) has been `#![no_std]` + `alloc` since before
  `v1.0.0`, proven continuously by the `no_std` CI gate against `thumbv7em-none-eabihf`.

`rustysnes_core::facade::EmuCore` (the same facade `rustysnes-frontend` and `rustysnes-libretro`
already drive the emulator through) is `std`-only (needs `zip` archive extraction for
`.zip`-wrapped ROMs). This is not a blocker: Android's (`aarch64-linux-android`,
`x86_64-linux-android`) and iOS's (`aarch64-apple-ios`, `aarch64-apple-ios-sim`) Rust targets are
`std`-supporting Tier 2/3 targets, not bare-metal — unlike the `thumbv7em-none-eabihf` gate the
chip crates alone are held to.

RustyNES's own mobile bridge had to absorb nine patch releases (`v2.0.1`-`v2.0.9` "Harbor")
re-porting after its `v2.0.0` scheduler rewrite broke the save-state/movie format — the same class
of risk this project's own `docs/adr/0002` (the fractional-timebase refactor) names. That refactor
was assessed and found **not currently warranted** at `v1.1.0`
(`docs/audit/fractional-timebase-go-no-go-2026-07-11.md`), and five stable minors since then
(`v1.0.0`-`v1.4.0`) plus the entire RustyNES-parity ladder (`v1.5.0`-`v1.13.0`) have shipped with
zero save-state-format churn (`FORMAT_VERSION` last bumped at `v0.9.0`). This is cited here as a
fact informing the mobile track's risk profile, not a new decision made by this ADR.

## Decision

- **Bridge technology: `UniFFI`**, generating both Kotlin (Android) and Swift (iOS) bindings from
  one Rust source of truth (`crates/rustysnes-mobile`), rather than hand-written JNI +
  `swift-bridge`/raw C FFI maintained twice. `UniFFI`'s proc-macro export style
  (`#[uniffi::export]`, `uniffi::setup_scaffolding!()`) needs no `.udl` interface-definition file
  to hand-maintain in parallel with the Rust source.
- **The bridge wraps `EmuCore`, adds no new emulation logic.** `rustysnes-mobile` is a thin FFI
  adapter over the exact same facade the desktop frontend and the libretro core already use — the
  same "one emulation core, multiple embeddings" shape this project has followed since `v1.2.0`
  relocated `EmuCore` out of the frontend crate specifically to enable this.
- **Mobile UI is a native Compose/SwiftUI shell over a raw `wgpu` surface** (`v1.15.0`/`v1.16.0`),
  not an attempt to run `egui` with touch input. `rustysnes-gfx-shaders` (`v1.12.0`) was extracted
  specifically so this shell can reuse the exact `BLIT_WGSL`/`CRT_WGSL`/`HQX_WGSL`/`XBRZ_WGSL`
  shader strings without depending on `rustysnes-frontend`'s winit/egui/cpal dependency graph.
- **`no_std` CI gate expanded to a per-crate matrix** (`v1.14.0`) — each chip crate now builds
  standalone against `thumbv7em-none-eabihf --no-default-features`, not only transitively through
  `rustysnes-core`'s own build. This was already implied by each crate's own `#![no_std]`
  posture; the gate now actually proves it per-crate instead of only in aggregate.
- **Android-first, iOS honestly scaffolded-but-untested where the toolchain is unavailable.** This
  development environment has a real Android SDK/NDK (confirmed: NDK r29, both the
  `aarch64-linux-android` and `x86_64-linux-android` Rust targets, `cargo-ndk`) but no macOS/Xcode
  toolchain. `v1.14.0`'s
  Rust bridge and its generated Kotlin AND Swift bindings are both verified to actually generate
  correctly (real `cargo ndk` cross-compiles producing a genuine ARM64 `.so`, real
  `uniffi-bindgen`-generated `.kt`/`.swift` source inspected for correct method signatures). The
  iOS-specific `rustysnes-ios` crate and SwiftUI shell (`v1.16.0 "Beacon"`) will be written and
  will compile-check via `cargo build --target aarch64-apple-ios` where cross-compilation permits,
  but the actual Xcode build/link/run step needs the project owner's own Mac to verify — flagged
  explicitly at that point, not silently claimed as done.

## Consequences

- The mobile track (`v1.14.0`-`v1.18.0`) is now in scope, reversing the `v1.0.0`-era default.
  `to-dos/VERSION-PLAN.md`'s "Post-v1.0 — Reach (deferred)" no-mobile-appetite line is corrected
  in the same change as this ADR.
- `rustysnes-mobile` becomes a new permanent crate in the workspace graph, depending only on
  `rustysnes-core` (default/`std` features) and `rustysnes-cart` — no new dependency of any
  existing crate ON `rustysnes-mobile` (strictly a leaf, matching this project's one-directional
  dependency-graph rule).
- Real Android on-device/emulator verification is possible in this environment going forward
  (an AVD exists, though it needs a working device definition — a `v1.15.0` concern, not this
  ADR's). Real iOS verification is not, and is explicitly out of this environment's reach for the
  duration of the mobile track unless that changes.
- This does NOT reopen `docs/adr/0002`'s fractional-timebase question — the mobile track's own
  risk (a scheduler rewrite breaking save-state/movie compatibility) is judged already retired by
  that ADR's `v1.1.0` "not currently warranted" finding, cited here, not re-litigated.
