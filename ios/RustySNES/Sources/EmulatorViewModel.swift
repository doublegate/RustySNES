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
        // Interleaved Int16 stereo, matching `MobileCore.drainAudio()`'s own `[Int16]` layout
        // (LRLRLR...) and the S-DSP's native output rate -- the same convention
        // `rustysnes-android`'s `AudioTrack` setup and `rustysnes-frontend::audio`'s resampler
        // target both already use.
        guard
            let format = AVAudioFormat(
                commonFormat: .pcmFormatInt16,
                sampleRate: 32_000,
                channels: 2,
                interleaved: true
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
        samples.withUnsafeBufferPointer { source in
            guard let base = source.baseAddress else { return }
            channelData[0].update(from: base, count: samples.count)
        }
        player.scheduleBuffer(buffer)
    }
}
