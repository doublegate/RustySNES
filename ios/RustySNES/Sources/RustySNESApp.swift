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
    init() {
        logMonetizationScaffold()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

/// `v1.18.0 "Dormant"` -- an inert call into `rustysnes-monetization`: compiled in and invoked
/// once at startup, logged only, never gating any UI or behavior. No real store SDK is wired up;
/// both functions are dormant placeholders (see that crate's own module doc) pending the
/// `docs/mobile-readiness.md` "Mobile Phase 6" store-launch decision. Mirrors
/// `MainActivity.kt`'s `logMonetizationScaffold` exactly.
private func logMonetizationScaffold() {
    // `max(0.0, ...)` (found in review): a negative `timeIntervalSince1970` (device clock set
    // before 1970) would otherwise trap on the `UInt64` cast, crashing the app at startup over a
    // log-only scaffold call.
    let nowUnixSecs = UInt64(max(0.0, Date().timeIntervalSince1970))
    let entitlement = checkEntitlement(nowUnixSecs: nowUnixSecs)
    let pacing = defaultAdPacingPolicy()
    print(
        "RustySNES: monetization scaffold (dormant): unlocked=\(entitlement.unlocked) "
            + "minIntervalSecs=\(pacing.minIntervalSecs) "
            + "sessionsBeforeFirstAd=\(pacing.sessionsBeforeFirstAd)"
    )
}
