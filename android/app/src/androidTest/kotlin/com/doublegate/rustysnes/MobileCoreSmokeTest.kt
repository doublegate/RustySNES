package com.doublegate.rustysnes

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.rustysnes_mobile.MobileCore
import uniffi.rustysnes_mobile.MobileRegion

/**
 * The UniFFI smoke test: proves the generated Kotlin bindings actually **load and call** the native
 * library on a real Android runtime.
 *
 * `assembleDebug` already proves the bindings *compile* against the shell — `MainActivity` calls
 * `MobileCore` directly, so a bindgen output that drifted from the Rust API fails the Kotlin
 * compile. What a build cannot prove is that `System.loadLibrary` finds the `.so` for the device's
 * ABI, that JNA's mapping matches the symbols in it, and that a call marshals across and returns.
 * Those are runtime facts, and this project has already shipped one native Android crash that a
 * build could not have caught.
 *
 * Deliberately ROM-free. The app takes ROMs only from the user's document picker
 * (`docs/app-store-4-7-self-audit.md`), so there is no ROM to open here and no need for one: every
 * assertion below is about the *bridge*, not about emulation, which the workspace's own test suite
 * covers far better than an emulator can.
 */
@RunWith(AndroidJUnit4::class)
class MobileCoreSmokeTest {
    /** Constructing the core loads the library and crosses the FFI boundary once. */
    @Test
    fun the_native_library_loads_and_a_core_can_be_constructed() {
        val core = MobileCore(MobileRegion.NTSC)
        assertFalse("a freshly constructed core must report no ROM loaded", core.romLoaded())
    }

    /**
     * A frame with no ROM loaded still has to return a correctly sized framebuffer. This is the
     * assertion that would catch a marshalling error: a wrong length, or a returned buffer that
     * does not survive the crossing, shows up here and nowhere in a build.
     */
    @Test
    fun a_frame_runs_and_returns_a_framebuffer_of_the_declared_size() {
        val core = MobileCore(MobileRegion.NTSC)
        core.runFrame()

        val size = core.frameSize()
        assertTrue("frame width must be positive, got ${size.width}", size.width > 0u)
        assertTrue("frame height must be positive, got ${size.height}", size.height > 0u)

        val fb = core.framebuffer()
        assertEquals(
            "the framebuffer length must be width * height * 4 (RGBA8)",
            (size.width * size.height * 4u).toInt(),
            fb.size,
        )
    }

    /**
     * `drainAudio` is documented as non-destructive — it returns the current frame's buffered
     * samples rather than popping a FIFO — so calling it twice for one frame returns the same
     * count. Pinning that here is what stops the contract drifting under a shell that calls it once
     * per `runFrame` and would not notice.
     */
    @Test
    fun drain_audio_is_non_destructive_within_a_frame() {
        val core = MobileCore(MobileRegion.NTSC)
        core.runFrame()
        val first = core.drainAudio().size
        val second = core.drainAudio().size
        assertEquals("drainAudio must not consume the buffer", first, second)
    }

    /** Reset and power-cycle are the two lifecycle calls the shell makes; both must cross safely. */
    @Test
    fun the_lifecycle_calls_cross_the_boundary() {
        val core = MobileCore(MobileRegion.NTSC)
        core.reset()
        core.powerCycle()
        assertFalse("no ROM was ever loaded", core.romLoaded())
    }
}
