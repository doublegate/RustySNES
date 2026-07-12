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
        let scale = window?.screen.scale ?? UIScreen.main.scale
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
        let scale = UIScreen.main.scale
        view.contentScaleFactor = scale
        // `bounds` is not yet meaningful on the same run-loop turn `makeUIView` returns (SwiftUI
        // hasn't laid the view out yet) -- deferring to the next run-loop turn, after which
        // `layoutSubviews` will also have fired at least once, matches how `layoutSubviews`
        // itself measures pixel size (mirrors the pattern `MainActivity.kt`'s
        // `SurfaceHolder.Callback.surfaceCreated` avoids needing, since Android hands the real
        // size directly in that callback).
        let uiView = Unmanaged.passUnretained(view).toOpaque()
        DispatchQueue.main.async {
            let pixelSize = CGSize(
                width: max(view.bounds.width * scale, 1),
                height: max(view.bounds.height * scale, 1)
            )
            NativeRenderer.surfaceCreated(
                uiView: uiView,
                width: UInt32(pixelSize.width),
                height: UInt32(pixelSize.height)
            )
        }
        return view
    }

    func updateUIView(_ uiView: MetalBackedView, context: Context) {}

    static func dismantleUIView(_ uiView: MetalBackedView, coordinator: ()) {
        NativeRenderer.surfaceDestroyed()
    }
}
