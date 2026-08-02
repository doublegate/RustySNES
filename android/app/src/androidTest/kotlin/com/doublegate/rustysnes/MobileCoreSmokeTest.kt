package com.doublegate.rustysnes

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
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

    /** AccuracySNES's HiROM image, packaged as a test asset by Gradle's `copyTestRom`. */
    private fun testRom(): ByteArray =
        InstrumentationRegistry.getInstrumentation().context.assets
            .open("accuracysnes-hirom.sfc").use { it.readBytes() }

    /**
     * A **real cart boots on the device**, and only then is the audio buffer asserted.
     *
     * This test took two wrong turns, both of the same kind. It first asserted that two successive
     * `drainAudio()` calls return equal counts, claiming to pin the documented non-destructive
     * contract; then that the buffer length is even, claiming to pin interleaved stereo. A no-ROM
     * frame produces **no audio at all** (measured on the host: `first=0 second=0`), so the first
     * passed on `0 == 0` and the second on `0 % 2 == 0`. Neither could fail for the reason it named.
     *
     * The fix is not a better assertion, it is a cart. `docs/mobile-readiness.md` records that no
     * ROM had ever actually booted on a device or simulator; loading one here closes that and makes
     * every assertion below capable of failing.
     */
    @Test
    fun a_real_cart_boots_and_produces_audio() {
        val core = MobileCore(MobileRegion.NTSC)
        core.loadRom(testRom())
        assertTrue("the cart did not load", core.romLoaded())

        // Eight frames for margin, not because eight are needed: measured on the host, a booted
        // AccuracySNES cart emits 1066 interleaved samples from the FIRST frame. Stating that
        // rather than a plausible "the APU needs the IPL handshake first" — which the measurement
        // disproves — keeps the comment something a reader can rely on.
        repeat(8) { core.runFrame() }

        val audio = core.drainAudio()
        assertTrue("a booted cart produced no audio at all", audio.isNotEmpty())
        assertEquals(
            "drainAudio must return interleaved stereo, so its length is always even",
            0,
            audio.size % 2,
        )
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
