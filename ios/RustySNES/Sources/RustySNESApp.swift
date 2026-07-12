import SwiftUI

/// `v1.16.0 "Beacon"` -- the iOS alpha, reusing `v1.15.0 "Sideload"`'s Android touch-UX design
/// and MVP scope exactly (P1 standard gamepad only via on-screen touch buttons, a file-picker ROM
/// load, no settings/post-filters yet). `v1.17.0 "Parity"` adds a single-slot Save State/Load
/// State pair, persisted to the app's Documents directory. See `docs/mobile-readiness.md` for
/// what's verified (the Rust side, for real: `cargo build --release --target aarch64-apple-ios`
/// succeeds and produces a genuine ARM64 Mach-O static library, and a real `xcodebuild` simulator
/// build passes on a `macos-latest` CI runner) versus what isn't (no on-device/simulator *run*
/// has ever happened, since this development environment has no Xcode/macOS toolchain).
@main
struct RustySNESApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
