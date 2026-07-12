package com.doublegate.rustysnes

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.net.Uri
import android.os.Bundle
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import uniffi.rustysnes_mobile.MobileCore
import uniffi.rustysnes_mobile.MobileRegion
import uniffi.rustysnes_monetization.checkEntitlement
import uniffi.rustysnes_monetization.defaultAdPacingPolicy

/**
 * `v1.15.0 "Sideload"` -- the minimal, real Android alpha MVP: a [SurfaceView] rendered via
 * `rustysnes-android`'s wgpu pipeline, a Storage-Access-Framework ROM picker, [AudioTrack]
 * streaming playback of [MobileCore.drainAudio], and on-screen touch buttons for the standard
 * SNES gamepad (P1 only). `v1.17.0 "Parity"` adds a single-slot Save State / Load State pair,
 * persisted to app-private internal storage -- multi-slot UI is `v1.17.1+` polish, matching how
 * the touch-UX/save-state UI were themselves deferred from `v1.15.0`. See
 * `docs/mobile-readiness.md` for what's still deliberately deferred (Mouse/Super Scope/Multitap
 * touch UX, settings).
 */
class MainActivity : ComponentActivity() {
    private val core = MobileCore(MobileRegion.NTSC)
    private var frameLoopJob: Job? = null

    // @Volatile: written on the main thread (setUpAudioTrack/onDestroy/stopFrameLoop) but read
    // and written from the frame loop's Dispatchers.Default background thread too -- without
    // this, a write on one thread is not guaranteed to be visible to a read on the other (found
    // in review).
    @Volatile
    private var audioTrack: AudioTrack? = null

    // Reused across frames instead of allocating a fresh `ShortArray` every ~16ms (via a
    // `ShortArray(size) { ... }` lambda constructor) -- only ever grown, never shrunk. Only
    // touched from the frame loop's single background coroutine, so no synchronization needed.
    private var audioScratch = ShortArray(0)

    private val pickRom =
        registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri: Uri? ->
            uri?.let(::loadRom)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        logMonetizationScaffold()
        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    EmulatorScreen(
                        core = core,
                        onOpenRom = { pickRom.launch(arrayOf("*/*")) },
                        onSaveState = ::saveState,
                        onLoadState = ::loadState,
                        onSurfaceReady = { holder -> attachSurface(holder) },
                        onSurfaceGone = {
                            NativeRenderer.nativeSurfaceDestroyed()
                            stopFrameLoop()
                        },
                    )
                }
            }
        }
    }

    // `v1.18.0 "Dormant"` -- an inert call into `rustysnes-monetization`: compiled in and invoked
    // once at startup, logged only, never gating any UI or behavior. No real store SDK is wired
    // up; both functions are dormant placeholders (see that crate's own module doc) pending the
    // `docs/mobile-readiness.md` "Mobile Phase 6" store-launch decision.
    private fun logMonetizationScaffold() {
        val entitlement = checkEntitlement((System.currentTimeMillis() / 1000).toULong())
        val pacing = defaultAdPacingPolicy()
        android.util.Log.d(
            "RustySNES",
            "monetization scaffold (dormant): unlocked=${entitlement.unlocked} " +
                "minIntervalSecs=${pacing.minIntervalSecs} " +
                "sessionsBeforeFirstAd=${pacing.sessionsBeforeFirstAd}",
        )
    }

    // Single save-state slot, persisted to app-private internal storage (`filesDir`, no
    // permissions needed) -- multi-slot UI is `v1.17.1+` polish, matching the "minimal real MVP"
    // scope this mobile track has followed since `v1.15.0`. `AtomicFile`, not a plain `File`
    // (found in review): a direct write left `save.state` at real risk of being left partially
    // written (app killed mid-write, or a load racing a save) and then failing to load --
    // `AtomicFile` writes to a temp file and renames it into place on `finishWrite`, so a reader
    // only ever sees the previous complete state or the new complete state, never a partial one.
    private val saveStateFile get() = android.util.AtomicFile(java.io.File(filesDir, "save.state"))

    private fun saveState() {
        if (!core.romLoaded()) return
        lifecycleScope.launch(Dispatchers.IO) {
            // `core.saveState()` does not throw -- `MobileCore::save_state` returns a plain
            // `Vec<u8>` in Rust, not a `Result` (serializing an already-loaded, valid, in-memory
            // core cannot fail the way deserializing untrusted bytes can), so only the file I/O
            // below needs a catch.
            val state = core.saveState()
            val file = saveStateFile
            val stream = file.startWrite()
            try {
                stream.write(state)
                file.finishWrite(stream)
            } catch (e: java.io.IOException) {
                file.failWrite(stream)
                android.util.Log.e("RustySNES", "saveState write failed: ${e.message}")
            }
        }
    }

    private fun loadState() {
        if (!core.romLoaded()) return
        lifecycleScope.launch(Dispatchers.IO) {
            // `exists()` moved onto this background dispatcher (found in review) -- it's
            // synchronous disk I/O, same ANR-avoidance reasoning as everything else here.
            val file = saveStateFile
            if (!file.baseFile.exists()) return@launch
            val blob = try {
                file.readFully()
            } catch (e: java.io.IOException) {
                android.util.Log.e("RustySNES", "loadState read failed: ${e.message}")
                return@launch
            }
            try {
                core.loadState(blob)
            } catch (e: uniffi.rustysnes_mobile.MobileException) {
                // A corrupt/foreign save blob must not crash the app -- same disposition as
                // loadRom's identical bad-input handling.
                android.util.Log.e("RustySNES", "loadState failed: ${e.message}")
            }
        }
    }

    // Reads the ROM file and calls into `core.loadRom` off the main thread -- both are I/O/CPU
    // work that can stall the UI thread long enough to trigger an ANR for large ROM files (found
    // in review). `lifecycleScope` (from `androidx.lifecycle:lifecycle-runtime-ktx`, already a
    // dependency) ties this coroutine to the activity's own lifecycle so it's cancelled
    // automatically if the activity is destroyed mid-load.
    private fun loadRom(uri: Uri) {
        lifecycleScope.launch(Dispatchers.IO) {
            val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return@launch
            try {
                core.loadRom(bytes)
            } catch (e: uniffi.rustysnes_mobile.MobileException) {
                // A bad/unrecognized ROM pick must not crash the app -- surfacing this properly (a
                // toast/dialog) is deferred alongside the rest of the settings/error-UI polish this
                // MVP intentionally skips; logging is the honest minimum for now.
                android.util.Log.e("RustySNES", "loadRom failed: ${e.message}")
                return@launch
            }
            startFrameLoop()
        }
    }

    private fun attachSurface(holder: SurfaceHolder) {
        val frame = holder.surfaceFrame
        NativeRenderer.nativeSurfaceCreated(holder.surface, frame.width(), frame.height())
        // Re-attaching after the surface was previously torn down (background/rotation) while a
        // ROM was already loaded -- resume where `stopFrameLoop` paused, instead of requiring the
        // user to re-open the ROM picker (found in review, paired with `stopFrameLoop` below).
        if (core.romLoaded()) {
            startFrameLoop()
        }
    }

    // Paired with `attachSurface`'s resume above: stops burning CPU/battery (and, worse, playing
    // audio) once the surface is gone and `nativePresentFrame` would be a silent no-op anyway
    // (found in review).
    private fun stopFrameLoop() {
        frameLoopJob?.cancel()
        frameLoopJob = null
        audioTrack?.pause()
    }

    /// One background coroutine driving `run_frame` -> present -> audio at a fixed ~60 Hz pace --
    /// deliberately simple (a sleep-paced loop, not `Choreographer`-synced) for this MVP; frame
    /// pacing/vsync-sync polish is a documented `v1.15.1+` follow-up, not attempted here.
    ///
    /// A genuine, real crash (a native SIGSEGV in `AudioTrack::write`) was found in review here:
    /// `frameLoopJob?.cancel()` is cooperative, not immediate -- the old coroutine keeps running
    /// until its next suspension point (`delay(16)`) actually observes the cancellation, so a
    /// call site that invokes `startFrameLoop` while a loop is already active (`attachSurface`
    /// previously did, unconditionally, whenever `surfaceCreated` fired again mid-game) could
    /// leave two coroutines briefly writing to the same `AudioTrack` concurrently, corrupting its
    /// native buffer state. Now idempotent: a no-op if a loop is already running, since `core`
    /// (the same `MobileCore` instance) just keeps advancing whatever ROM is currently loaded --
    /// there was never a real need to tear down and recreate the loop/`AudioTrack` on a redundant
    /// call.
    private fun startFrameLoop() {
        if (frameLoopJob?.isActive == true) return
        setUpAudioTrack()
        // `setUpAudioTrack` is a no-op past the first call (an existing, possibly `stopFrameLoop`-
        // paused, track is reused) -- `play()` un-pauses it either way; harmless to call again on
        // an already-playing track.
        audioTrack?.play()
        frameLoopJob = CoroutineScope(Dispatchers.Default).launch {
            while (true) {
                core.runFrame()
                val size = core.frameSize()
                NativeRenderer.nativePresentFrame(core.framebuffer(), size.width.toInt(), size.height.toInt())
                val audio = core.drainAudio()
                if (audio.isNotEmpty()) {
                    if (audioScratch.size < audio.size) {
                        audioScratch = ShortArray(audio.size)
                    }
                    for (i in audio.indices) {
                        audioScratch[i] = audio[i]
                    }
                    audioTrack?.write(audioScratch, 0, audio.size)
                }
                kotlinx.coroutines.delay(16)
            }
        }
    }

    private fun setUpAudioTrack() {
        if (audioTrack != null) return
        // The S-DSP's native output rate -- matches `rustysnes-frontend::audio`'s own resampler
        // target convention (32 kHz native, resampled elsewhere on desktop; here `AudioTrack`
        // itself handles any device-side resampling, so no explicit resampler is needed for this
        // MVP).
        val sampleRate = 32_000
        val minBuf = AudioTrack.getMinBufferSize(
            sampleRate,
            AudioFormat.CHANNEL_OUT_STEREO,
            AudioFormat.ENCODING_PCM_16BIT,
        )
        audioTrack = AudioTrack(
            AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_GAME)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build(),
            AudioFormat.Builder()
                .setSampleRate(sampleRate)
                .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .build(),
            minBuf.coerceAtLeast(4096),
            AudioTrack.MODE_STREAM,
            AudioManagerSessionIdGenerate(),
        )
        audioTrack?.play()
    }

    // `AudioTrack`'s constructor takes a session ID; `AudioManager.AUDIO_SESSION_ID_GENERATE`
    // (0) requests a fresh one -- named as a tiny helper only so the call site above reads
    // clearly without an inline magic-number comment.
    private fun AudioManagerSessionIdGenerate(): Int = 0

    // A lifecycle-level safety net for `stopFrameLoop` above -- `SurfaceView`'s own
    // `surfaceDestroyed` callback does not fire in every backgrounding path on every OEM skin
    // (found in review); `onPause` always fires, so this is the more reliable stop point.
    override fun onPause() {
        super.onPause()
        stopFrameLoop()
    }

    // Paired with `onPause` above: on a device/OEM skin where the `SurfaceView`'s surface
    // survives backgrounding (so `attachSurface`'s own resume path in `surfaceCreated` never
    // fires), this is the fallback that un-freezes the game on foreground return.
    override fun onResume() {
        super.onResume()
        if (core.romLoaded() && frameLoopJob == null) {
            startFrameLoop()
        }
    }

    override fun onDestroy() {
        stopFrameLoop()
        audioTrack?.release()
        core.close()
        super.onDestroy()
    }
}

/// Canonical SNES auto-joypad bit layout (`rustysnes_core::controller::Button::mask`, ported
/// here as plain constants since the Kotlin side has no equivalent enum of its own -- `set_pad`
/// takes a raw bitmask, matching the desktop frontend's own wire format exactly).
private object SnesButton {
    // Not `const` -- `.toUShort()` isn't a compile-time-constant expression in Kotlin (found by
    // actually trying it: "Const 'val' initializer should be a constant value").
    val B: UShort = (1 shl 15).toUShort()
    val Y: UShort = (1 shl 14).toUShort()
    val SELECT: UShort = (1 shl 13).toUShort()
    val START: UShort = (1 shl 12).toUShort()
    val UP: UShort = (1 shl 11).toUShort()
    val DOWN: UShort = (1 shl 10).toUShort()
    val LEFT: UShort = (1 shl 9).toUShort()
    val RIGHT: UShort = (1 shl 8).toUShort()
    val A: UShort = (1 shl 7).toUShort()
    val X: UShort = (1 shl 6).toUShort()
    val L: UShort = (1 shl 5).toUShort()
    val R: UShort = (1 shl 4).toUShort()
}

@Composable
private fun EmulatorScreen(
    core: MobileCore,
    onOpenRom: () -> Unit,
    onSaveState: () -> Unit,
    onLoadState: () -> Unit,
    onSurfaceReady: (SurfaceHolder) -> Unit,
    onSurfaceGone: () -> Unit,
) {
    var heldMask by remember { mutableStateOf(0) }

    fun setBit(bit: UShort, pressed: Boolean) {
        heldMask = if (pressed) heldMask or bit.toInt() else heldMask and bit.toInt().inv()
        core.setPad(0u, heldMask.toUShort())
    }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(modifier = Modifier.padding(8.dp)) {
            Button(onClick = onOpenRom) { Text("Open ROM") }
            Button(onClick = onSaveState, modifier = Modifier.padding(start = 8.dp)) {
                Text("Save State")
            }
            Button(onClick = onLoadState, modifier = Modifier.padding(start = 8.dp)) {
                Text("Load State")
            }
        }
        Box(modifier = Modifier.fillMaxSize()) {
            AndroidView(
                modifier = Modifier.fillMaxSize(),
                factory = { context ->
                    SurfaceView(context).apply {
                        holder.addCallback(object : SurfaceHolder.Callback {
                            override fun surfaceCreated(holder: SurfaceHolder) = onSurfaceReady(holder)
                            override fun surfaceChanged(
                                holder: SurfaceHolder,
                                format: Int,
                                width: Int,
                                height: Int,
                            ) = NativeRenderer.nativeSurfaceChanged(width, height)
                            override fun surfaceDestroyed(holder: SurfaceHolder) = onSurfaceGone()
                        })
                    }
                },
            )
            TouchControls(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(16.dp),
                onButton = ::setBit,
            )
        }
    }
}

@Composable
private fun TouchControls(modifier: Modifier = Modifier, onButton: (UShort, Boolean) -> Unit) {
    Row(
        modifier = modifier.fillMaxSize(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        DPad(onButton = onButton)
        FaceButtons(onButton = onButton)
    }
}

@Composable
private fun DPad(onButton: (UShort, Boolean) -> Unit) {
    Column {
        TouchButton("^", SnesButton.UP, onButton)
        Row {
            TouchButton("<", SnesButton.LEFT, onButton)
            TouchButton(">", SnesButton.RIGHT, onButton)
        }
        TouchButton("v", SnesButton.DOWN, onButton)
    }
}

@Composable
private fun FaceButtons(onButton: (UShort, Boolean) -> Unit) {
    Column {
        Row {
            TouchButton("Y", SnesButton.Y, onButton)
            TouchButton("X", SnesButton.X, onButton)
        }
        Row {
            TouchButton("B", SnesButton.B, onButton)
            TouchButton("A", SnesButton.A, onButton)
        }
        Row {
            TouchButton("Select", SnesButton.SELECT, onButton)
            TouchButton("Start", SnesButton.START, onButton)
        }
    }
}

/// A press-and-hold touch target: sets the bit on pointer-down, clears it on pointer-up/cancel --
/// deliberately not a Compose `Button` (whose `onClick` only fires on release, unsuitable for a
/// game pad that needs to know "is this button currently held").
@Composable
private fun TouchButton(label: String, bit: UShort, onButton: (UShort, Boolean) -> Unit) {
    Box(
        modifier = Modifier
            .padding(4.dp)
            .size(56.dp)
            .background(MaterialTheme.colorScheme.secondaryContainer)
            .pointerInput(bit) {
                detectTapGestures(
                    onPress = {
                        onButton(bit, true)
                        tryAwaitRelease()
                        onButton(bit, false)
                    },
                )
            },
        contentAlignment = Alignment.Center,
    ) {
        Text(label)
    }
}
