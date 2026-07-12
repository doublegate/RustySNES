import Foundation

/// Thin Swift wrapper over `rustysnes-ios`'s plain C-ABI FFI (declared in `Bridging-Header.h`),
/// mirroring `android/.../NativeRenderer.kt`'s role exactly -- same four-call lifecycle, just a
/// static-lib C boundary instead of JNI.
enum NativeRenderer {
    static func surfaceCreated(uiView: UnsafeMutableRawPointer, width: UInt32, height: UInt32) {
        rustysnes_ios_surface_created(uiView, width, height)
    }

    static func surfaceChanged(width: UInt32, height: UInt32) {
        rustysnes_ios_surface_changed(width, height)
    }

    static func surfaceDestroyed() {
        rustysnes_ios_surface_destroyed()
    }

    /// `rgba` must be a `width * height * 4`-byte RGBA8 framebuffer, matching
    /// `MobileCore.framebuffer()`'s own contract.
    static func presentFrame(rgba: Data, width: UInt32, height: UInt32) {
        rgba.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) in
            guard let base = buffer.bindMemory(to: UInt8.self).baseAddress else { return }
            rustysnes_ios_present_frame(base, buffer.count, width, height)
        }
    }
}
