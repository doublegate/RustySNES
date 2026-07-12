import Foundation
import AVFoundation
import Combine

/// Owns `MobileCore` and the frame/audio loop -- the iOS analog of `MainActivity.kt`. Mirrors
/// that class's now-fixed lifecycle handling exactly (ROM loading off the main thread, the frame
/// loop pausing/resuming with the surface and app lifecycle, not just its happy path): those
/// fixes came from real, on-device bugs found in `rustysnes-android`'s PR review, and are applied
/// here from the start rather than left to be rediscovered — though, per this crate's own
/// verification-status note, none of this Swift code has actually been compiled or run, since
/// this development environment has no Xcode/macOS toolchain.
@MainActor
final class EmulatorViewModel: ObservableObject {
    let core = MobileCore(region: .ntsc)

    private var frameLoopTask: Task<Void, Never>?
    private var audioEngine: AVAudioEngine?
    private var audioPlayer: AVAudioPlayerNode?
    private var audioFormat: AVAudioFormat?

    /// Reads `url` and calls `core.loadRom` off the main thread — both are I/O/CPU work that
    /// could otherwise stall the UI (the same real ANR-class risk `MainActivity.kt`'s equivalent
    /// fix addressed on Android).
    func loadRom(url: URL) {
        Task.detached(priority: .userInitiated) { [core] in
            let accessed = url.startAccessingSecurityScopedResource()
            defer {
                if accessed { url.stopAccessingSecurityScopedResource() }
            }
            guard let data = try? Data(contentsOf: url) else {
                print("RustySNES: failed to read ROM at \(url)")
                return
            }
            do {
                try core.loadRom(rom: data)
            } catch {
                // A bad/unrecognized ROM pick must not crash the app -- surfacing this properly
                // (an alert) is deferred alongside the rest of the settings/error-UI polish this
                // MVP intentionally skips, matching `MainActivity.kt`'s identical disposition;
                // logging is the honest minimum for now.
                print("RustySNES: loadRom failed: \(error)")
                return
            }
            await self.startFrameLoop()
        }
    }

    /// Call from `MetalHostView`'s surface-created path — resumes if a ROM is already loaded
    /// (e.g. re-attaching after backgrounding), matching `MainActivity.kt`'s `attachSurface`.
    func surfaceAttached() {
        if core.romLoaded() {
            startFrameLoop()
        }
    }

    /// Call from `MetalHostView`'s surface-destroyed path AND from `scenePhase` going
    /// `.background`/`.inactive` — matches `MainActivity.kt`'s `stopFrameLoop`, called from both
    /// `onSurfaceGone` and the `onPause` lifecycle safety net for the same reason: the surface
    /// isn't guaranteed to be torn down on every backgrounding path.
    func stopFrameLoop() {
        frameLoopTask?.cancel()
        frameLoopTask = nil
        audioPlayer?.pause()
    }

    /// Call from `scenePhase` going `.active` — matches `MainActivity.kt`'s `onResume` fallback,
    /// for the case where the Metal view survives backgrounding and `surfaceAttached` never
    /// re-fires.
    func sceneBecameActive() {
        if core.romLoaded(), frameLoopTask == nil {
            startFrameLoop()
        }
    }

    private func startFrameLoop() {
        frameLoopTask?.cancel()
        setUpAudioIfNeeded()
        audioPlayer?.play()
        frameLoopTask = Task.detached(priority: .userInitiated) { [core, weak self] in
            while !Task.isCancelled {
                core.runFrame()
                let size = core.frameSize()
                let rgba = core.framebuffer()
                NativeRenderer.presentFrame(rgba: rgba, width: size.width, height: size.height)
                let audio = core.drainAudio()
                if !audio.isEmpty {
                    await self?.playAudio(samples: audio)
                }
                // Deliberately simple (a fixed ~60 Hz sleep-paced loop, not display-link-synced)
                // for this MVP, matching `MainActivity.kt`'s identical, explicitly-documented
                // trade-off — frame-pacing/vsync-sync polish is a `v1.16.1+` follow-up.
                try? await Task.sleep(nanoseconds: 16_000_000)
            }
        }
    }

    private func setUpAudioIfNeeded() {
        guard audioEngine == nil else { return }
        // On iOS (unlike Android/desktop), an `AVAudioEngine` produces no audible output without
        // first configuring and activating the shared `AVAudioSession` for playback -- found in
        // review; this is a real runtime-behavior gap the build-only CI check can't catch (no
        // on-device/simulator run has ever happened, only a compile).
        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playback, mode: .default)
            try session.setActive(true)
        } catch {
            print("RustySNES: AVAudioSession setup failed: \(error)")
            return
        }
        // NOT interleaved -- `AVAudioPCMBuffer.int16ChannelData` is documented to return the
        // per-channel buffer pointers for a non-interleaved format; for an interleaved format
        // its behavior is a real, plausible correctness risk this sandbox cannot verify at
        // runtime (found in review, and not something a build-only CI check would catch either).
        // Non-interleaved sidesteps the ambiguity entirely: each channel gets its own
        // unambiguous buffer, filled by indexed writes below (matching `MobileCore.drainAudio()`'s
        // `[Int16]` LRLRLR layout, just deinterleaved on the way in instead of on the way out).
        guard
            let format = AVAudioFormat(
                commonFormat: .pcmFormatInt16,
                sampleRate: 32_000,
                channels: 2,
                interleaved: false
            )
        else {
            print("RustySNES: failed to construct the audio format")
            return
        }
        let engine = AVAudioEngine()
        let player = AVAudioPlayerNode()
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        do {
            try engine.start()
        } catch {
            print("RustySNES: AVAudioEngine.start failed: \(error)")
            return
        }
        audioEngine = engine
        audioPlayer = player
        audioFormat = format
    }

    private func playAudio(samples: [Int16]) async {
        guard let format = audioFormat, let player = audioPlayer else { return }
        let frameCount = AVAudioFrameCount(samples.count / 2)
        guard
            frameCount > 0,
            let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frameCount)
        else { return }
        buffer.frameLength = frameCount
        guard let channelData = buffer.int16ChannelData else { return }
        for i in 0..<Int(frameCount) {
            channelData[0][i] = samples[i * 2]
            channelData[1][i] = samples[i * 2 + 1]
        }
        // `AVAudioPlayerNode.scheduleBuffer(_:)` has an `async` overload (in addition to the
        // completion-handler one) -- `playAudio` already runs in an async context, so Swift's
        // overload resolution picks that one, requiring `await` (a real compile error found by
        // this PR's own CI, the first real Swift compiler pass over this file).
        await player.scheduleBuffer(buffer)
    }
}
