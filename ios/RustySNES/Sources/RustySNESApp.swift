import SwiftUI

/// `v1.16.0 "Beacon"` -- the iOS alpha, reusing `v1.15.0 "Sideload"`'s Android touch-UX design
/// and MVP scope exactly (P1 standard gamepad only via on-screen touch buttons, a file-picker ROM
/// load, no save-state UI/settings/post-filters yet). See `docs/mobile-readiness.md` for what's
/// verified (the Rust side, for real: `cargo build --release --target aarch64-apple-ios`
/// succeeds and produces a genuine ARM64 Mach-O static library) versus what isn't (everything in
/// this `ios/` directory -- no Swift compiler has ever run over it, since this development
/// environment has no Xcode/macOS toolchain).
@main
struct RustySNESApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
