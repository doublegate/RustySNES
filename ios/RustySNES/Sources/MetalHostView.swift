import SwiftUI
import UIKit
import QuartzCore

/// A `UIView` whose backing `CALayer` is a `CAMetalLayer` -- the iOS analog of
/// `android/.../MainActivity.kt`'s `SurfaceView`. `rustysnes-ios`'s `wgpu` surface is built
/// directly from this view's own pointer (see `NativeRenderer`/`Bridging-Header.h`), matching
/// `raw_window_handle::UiKitWindowHandle`'s documented contract.
final class MetalBackedView: UIView {
    override static var layerClass: AnyClass { CAMetalLayer.self }

    private var lastPixelSize: CGSize = .zero

    override func layoutSubviews() {
        super.layoutSubviews()
        // `traitCollection.displayScale`, not `UIScreen.main`/`window?.screen` (deprecated since
        // iOS 15/16 -- found in review) -- the modern, non-deprecated way to read the scale of
        // whichever screen this view is currently on.
        let scale = traitCollection.displayScale
        let pixelSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        guard pixelSize != lastPixelSize, pixelSize.width > 0, pixelSize.height > 0 else { return }
        lastPixelSize = pixelSize
        (layer as? CAMetalLayer)?.drawableSize = pixelSize
        NativeRenderer.surfaceChanged(
            width: UInt32(pixelSize.width),
            height: UInt32(pixelSize.height)
        )
    }
}

/// Bridges [`MetalBackedView`] into SwiftUI, driving `rustysnes-ios`'s
/// created/changed/destroyed lifecycle from `UIViewRepresentable`'s own
/// `makeUIView`/`layoutSubviews`/`dismantleUIView` -- exactly the lifecycle pairing
/// `rustysnes-ios/src/lib.rs`'s module doc documents as the safety contract this Swift side must
/// uphold (the view must stay alive between `makeUIView` and `dismantleUIView`, which
/// `UIViewRepresentable` already guarantees).
struct MetalHostView: UIViewRepresentable {
    func makeUIView(context: Context) -> MetalBackedView {
        let view = MetalBackedView()
        view.backgroundColor = .black
        // SwiftUI's own environment value, not `UIScreen.main` (deprecated since iOS 15/16,
        // found in review) -- `context.environment.displayScale` is available before the view
        // has been laid out or added to a window, unlike `traitCollection`/`window?.screen`.
        view.contentScaleFactor = context.environment.displayScale
        // Called SYNCHRONOUSLY, not deferred via `DispatchQueue.main.async` (a real race found
        // in review): an async defer could let `dismantleUIView`/`surfaceDestroyed` run first if
        // the view is torn down before the block executes, leaving Rust holding a pointer to a
        // deallocated view. `bounds` isn't meaningful yet at this point (SwiftUI hasn't laid the
        // view out), so this passes a safe 1x1 placeholder; `layoutSubviews` sends the real size
        // via `surfaceChanged` moments later, once it's known.
        let uiView = Unmanaged.passUnretained(view).toOpaque()
        NativeRenderer.surfaceCreated(uiView: uiView, width: 1, height: 1)
        return view
    }

    func updateUIView(_ uiView: MetalBackedView, context: Context) {}

    static func dismantleUIView(_ uiView: MetalBackedView, coordinator: ()) {
        NativeRenderer.surfaceDestroyed()
    }
}
