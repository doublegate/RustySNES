package com.doublegate.rustysnes

import android.view.Surface

/**
 * JNI declarations for `rustysnes-android`'s wgpu-on-[Surface] renderer.
 *
 * These four methods mirror [android.view.SurfaceHolder.Callback]'s own lifecycle exactly (see
 * `rustysnes-android`'s own module doc for why) plus one present call, driven once per emulated
 * frame from [MainActivity]'s render loop. Loading `librustysnes_android.so` here is separate
 * from loading `librustysnes_mobile.so` -- the UniFFI-generated bindings for the latter load
 * themselves lazily on first use via JNA, not `System.loadLibrary`.
 */
object NativeRenderer {
    init {
        System.loadLibrary("rustysnes_android")
    }

    external fun nativeSurfaceCreated(surface: Surface, width: Int, height: Int)

    external fun nativeSurfaceChanged(width: Int, height: Int)

    external fun nativeSurfaceDestroyed()

    external fun nativePresentFrame(rgba: ByteArray, width: Int, height: Int)
}
