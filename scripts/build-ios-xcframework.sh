#!/usr/bin/env bash
# Builds rustysnes-mobile and rustysnes-ios for both the iOS device and simulator targets, then
# packages each into an .xcframework Xcode can link directly. Mirrors
# android/app/build.gradle.kts's cargoNdkBuild + uniffiBindgen tasks: same two crates, same
# "build for the real target(s), then wrap for the platform's own tooling" shape, just Apple's
# xcodebuild instead of Gradle.
#
# REQUIRES a real macOS + Xcode install (xcrun/xcodebuild) -- this only ever runs for real on
# .github/workflows/ios.yml's macos-latest runner or the project owner's own Mac. It is NOT
# runnable in the Linux sandbox this crate's Rust source was otherwise developed and verified in
# (confirmed: `cargo build --release --target aarch64-apple-ios` succeeds there without Xcode,
# but `xcodebuild -create-xcframework` obviously cannot).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$repo_root/ios/Frameworks"
rm -rf "$out_dir"
mkdir -p "$out_dir"

echo "==> Building rustysnes-mobile + rustysnes-ios for aarch64-apple-ios (device)"
cargo build --release --target aarch64-apple-ios \
  -p rustysnes-mobile -p rustysnes-ios \
  --manifest-path "$repo_root/Cargo.toml"

echo "==> Building rustysnes-mobile + rustysnes-ios for aarch64-apple-ios-sim (simulator)"
cargo build --release --target aarch64-apple-ios-sim \
  -p rustysnes-mobile -p rustysnes-ios \
  --manifest-path "$repo_root/Cargo.toml"

device_dir="$repo_root/target/aarch64-apple-ios/release"
sim_dir="$repo_root/target/aarch64-apple-ios-sim/release"

echo "==> Generating UniFFI Swift bindings for rustysnes-mobile"
# `.dylib` (the `cdylib` crate-type output), not `.a` -- matches `android/app/build.gradle.kts`'s
# `uniffiBindgen` task, which introspects the analogous `librustysnes_mobile.so`. This dylib is
# only ever used transiently as `uniffi-bindgen`'s codegen input, never shipped in the app itself
# (the xcframework built below embeds the `.a` staticlib instead).
bindings_dir="$out_dir/generated"
mkdir -p "$bindings_dir"
cargo run -p rustysnes-mobile --features bindgen --bin uniffi-bindgen \
  --manifest-path "$repo_root/Cargo.toml" -- \
  generate --library "$device_dir/librustysnes_mobile.dylib" --language swift \
  --out-dir "$bindings_dir" --no-format
# `rustysnes_mobile.swift` is a plain Swift source file (not a binary artifact) -- it gets added
# directly to the Xcode target as a source file (see `ios/project.yml`), not embedded in the
# xcframework below. Only the compiled staticlib + its generated C header/modulemap go into the
# xcframework.
mv "$bindings_dir/rustysnes_mobile.swift" "$repo_root/ios/RustySNES/Sources/Generated-RustysnesMobile.swift"

echo "==> Packaging RustysnesMobileFFI.xcframework"
mobile_headers="$bindings_dir/mobile-headers"
mkdir -p "$mobile_headers"
cp "$bindings_dir/rustysnes_mobileFFI.h" "$mobile_headers/"
cp "$bindings_dir/rustysnes_mobileFFI.modulemap" "$mobile_headers/module.modulemap"
xcodebuild -create-xcframework \
  -library "$device_dir/librustysnes_mobile.a" -headers "$mobile_headers" \
  -library "$sim_dir/librustysnes_mobile.a" -headers "$mobile_headers" \
  -output "$out_dir/RustysnesMobileFFI.xcframework"

echo "==> Packaging RustysnesIOS.xcframework"
# Library-only (no -headers) -- rustysnes-ios's small, hand-declared FFI surface is exposed to
# Swift via `ios/RustySNES/Bridging-Header.h` (a `SWIFT_OBJC_BRIDGING_HEADER` build setting in
# `project.yml`), not a Clang module. Wrapping it in a *second* module here too would give the
# same four C symbols two different importable names/paths into Swift for no benefit -- the
# xcframework format is used purely as Xcode's standard multi-platform packaging for the compiled
# binary itself.
xcodebuild -create-xcframework \
  -library "$device_dir/librustysnes_ios.a" \
  -library "$sim_dir/librustysnes_ios.a" \
  -output "$out_dir/RustysnesIOS.xcframework"

echo "==> Done. Frameworks at $out_dir"
