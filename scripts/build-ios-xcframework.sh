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

echo "==> Building rustysnes-mobile + rustysnes-ios + rustysnes-monetization for aarch64-apple-ios (device)"
cargo build --release --target aarch64-apple-ios \
  -p rustysnes-mobile -p rustysnes-ios -p rustysnes-monetization \
  --manifest-path "$repo_root/Cargo.toml"

# Both simulator architectures, not just arm64-apple-ios-sim -- a `generic/platform=iOS
# Simulator` xcodebuild destination (used by `ios.yml`'s CI build) is architecture-unconstrained
# and expects the simulator slice to cover every arch Xcode's default `ARCHS` lists for the
# Simulator SDK, which includes x86_64 alongside arm64 regardless of the host Mac's own CPU.
# Found for real on a real (Apple Silicon) macOS CI runner: `xcodebuild` reported both
# xcframeworks here "missing architecture(s) required by this target (x86_64)" when only
# `aarch64-apple-ios-sim` had been built.
echo "==> Building rustysnes-mobile + rustysnes-ios + rustysnes-monetization for aarch64-apple-ios-sim (simulator, Apple Silicon)"
cargo build --release --target aarch64-apple-ios-sim \
  -p rustysnes-mobile -p rustysnes-ios -p rustysnes-monetization \
  --manifest-path "$repo_root/Cargo.toml"

echo "==> Building rustysnes-mobile + rustysnes-ios + rustysnes-monetization for x86_64-apple-ios (simulator, Intel)"
cargo build --release --target x86_64-apple-ios \
  -p rustysnes-mobile -p rustysnes-ios -p rustysnes-monetization \
  --manifest-path "$repo_root/Cargo.toml"

device_dir="$repo_root/target/aarch64-apple-ios/release"
sim_arm64_dir="$repo_root/target/aarch64-apple-ios-sim/release"
sim_x86_64_dir="$repo_root/target/x86_64-apple-ios/release"

# One universal (`lipo`-merged) simulator library per crate, combining both simulator
# architectures into the single library `xcodebuild -create-xcframework` expects per
# platform+environment slice (device and simulator are separate slices; each slice's own binary
# can itself be multi-arch).
sim_universal_dir="$out_dir/sim-universal"
mkdir -p "$sim_universal_dir"
for crate in rustysnes_mobile rustysnes_ios rustysnes_monetization; do
  lipo -create \
    "$sim_arm64_dir/lib${crate}.a" \
    "$sim_x86_64_dir/lib${crate}.a" \
    -output "$sim_universal_dir/lib${crate}.a"
done
sim_dir="$sim_universal_dir"

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

echo "==> Generating UniFFI Swift bindings for rustysnes-monetization"
# Same shape as rustysnes-mobile's bindgen step above -- a distinct crate/dylib/namespace, so it
# needs its own bindgen invocation (`v1.18.0 "Dormant"`).
monetization_bindings_dir="$out_dir/generated-monetization"
mkdir -p "$monetization_bindings_dir"
cargo run -p rustysnes-monetization --features bindgen --bin uniffi-bindgen \
  --manifest-path "$repo_root/Cargo.toml" -- \
  generate --library "$device_dir/librustysnes_monetization.dylib" --language swift \
  --out-dir "$monetization_bindings_dir" --no-format
mv "$monetization_bindings_dir/rustysnes_monetization.swift" \
  "$repo_root/ios/RustySNES/Sources/Generated-RustysnesMonetization.swift"

# `rustysnes-mobile` and `rustysnes-monetization` are packaged into ONE combined
# `RustysnesFFI.xcframework`, not two separate ones -- found on a real macOS CI run: a
# "library"+`-headers` xcframework (as opposed to a `.framework` bundle) has its headers copied by
# `xcodebuild` into one directory shared across every such xcframework linked into the target
# (`$(BUILT_PRODUCTS_DIR)/include/`), and BOTH crates' UniFFI-generated modulemap gets renamed to
# the same literal filename Clang requires for auto-discovery (`module.modulemap`) -- two
# xcframeworks each contributing a same-named file to that one shared directory is a genuine
# "Multiple commands produce '.../include/module.modulemap'" `xcodebuild` failure, not a
# hypothetical one. Merging both crates' static libraries (via `libtool -static`) and headers
# (one combined `module.modulemap` declaring both crates' modules) into a single xcframework
# avoids the collision entirely -- matching the same umbrella-module pattern other multi-crate
# UniFFI Swift/Xcode integrations use.
echo "==> Packaging combined RustysnesFFI.xcframework (rustysnes-mobile + rustysnes-monetization)"
ffi_headers="$out_dir/ffi-headers"
mkdir -p "$ffi_headers"
cp "$bindings_dir/rustysnes_mobileFFI.h" "$ffi_headers/"
cp "$monetization_bindings_dir/rustysnes_monetizationFFI.h" "$ffi_headers/"
cat "$bindings_dir/rustysnes_mobileFFI.modulemap" \
  "$monetization_bindings_dir/rustysnes_monetizationFFI.modulemap" \
  > "$ffi_headers/module.modulemap"

ffi_device_lib="$out_dir/librustysnes_ffi-device.a"
ffi_sim_lib="$out_dir/librustysnes_ffi-sim.a"
libtool -static -o "$ffi_device_lib" \
  "$device_dir/librustysnes_mobile.a" "$device_dir/librustysnes_monetization.a"
libtool -static -o "$ffi_sim_lib" \
  "$sim_dir/librustysnes_mobile.a" "$sim_dir/librustysnes_monetization.a"

xcodebuild -create-xcframework \
  -library "$ffi_device_lib" -headers "$ffi_headers" \
  -library "$ffi_sim_lib" -headers "$ffi_headers" \
  -output "$out_dir/RustysnesFFI.xcframework"

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
