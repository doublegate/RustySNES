import SwiftUI
import UniformTypeIdentifiers

/// Canonical SNES auto-joypad bit layout (`rustysnes_core::controller::Button::mask`), ported
/// here as plain constants -- matches `android/.../MainActivity.kt`'s `SnesButton` object
/// exactly, including the comment about why these aren't derived some other way (there's no
/// Swift-side equivalent enum of its own; `setPad` takes a raw bitmask, matching the desktop
/// frontend's own wire format).
enum SnesButton {
    static let b: UInt16 = 1 << 15
    static let y: UInt16 = 1 << 14
    static let select: UInt16 = 1 << 13
    static let start: UInt16 = 1 << 12
    static let up: UInt16 = 1 << 11
    static let down: UInt16 = 1 << 10
    static let left: UInt16 = 1 << 9
    static let right: UInt16 = 1 << 8
    static let a: UInt16 = 1 << 7
    static let x: UInt16 = 1 << 6
    static let l: UInt16 = 1 << 5
    static let r: UInt16 = 1 << 4
}

struct ContentView: View {
    @StateObject private var viewModel = EmulatorViewModel()
    @Environment(\.scenePhase) private var scenePhase
    @State private var isPickingRom = false
    @State private var heldMask: UInt16 = 0

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button("Open ROM") { isPickingRom = true }
                    .padding()
                Spacer()
            }
            .background(Color(white: 0.98))

            ZStack {
                Color.black
                MetalHostView()
                    .onAppear { viewModel.surfaceAttached() }
                    .onDisappear { viewModel.stopFrameLoop() }

                TouchControls(onButton: setBit)
            }
        }
        .fileImporter(
            isPresented: $isPickingRom,
            allowedContentTypes: [.data],
            onCompletion: { result in
                if case let .success(url) = result {
                    viewModel.loadRom(url: url)
                }
            }
        )
        .onChange(of: scenePhase) { _, newPhase in
            switch newPhase {
            case .active:
                viewModel.sceneBecameActive()
            case .background, .inactive:
                viewModel.stopFrameLoop()
            @unknown default:
                break
            }
        }
    }

    private func setBit(_ bit: UInt16, pressed: Bool) {
        heldMask = pressed ? (heldMask | bit) : (heldMask & ~bit)
        viewModel.core.setPad(player: 0, buttons: heldMask)
    }
}

private struct TouchControls: View {
    let onButton: (UInt16, Bool) -> Void

    var body: some View {
        HStack {
            DPad(onButton: onButton)
            Spacer()
            FaceButtons(onButton: onButton)
        }
        .padding()
    }
}

private struct DPad: View {
    let onButton: (UInt16, Bool) -> Void

    var body: some View {
        VStack(spacing: 4) {
            TouchButton(label: "^", bit: SnesButton.up, onButton: onButton)
            HStack(spacing: 4) {
                TouchButton(label: "<", bit: SnesButton.left, onButton: onButton)
                TouchButton(label: ">", bit: SnesButton.right, onButton: onButton)
            }
            TouchButton(label: "v", bit: SnesButton.down, onButton: onButton)
        }
    }
}

private struct FaceButtons: View {
    let onButton: (UInt16, Bool) -> Void

    var body: some View {
        VStack(spacing: 4) {
            HStack(spacing: 4) {
                TouchButton(label: "Y", bit: SnesButton.y, onButton: onButton)
                TouchButton(label: "X", bit: SnesButton.x, onButton: onButton)
            }
            HStack(spacing: 4) {
                TouchButton(label: "B", bit: SnesButton.b, onButton: onButton)
                TouchButton(label: "A", bit: SnesButton.a, onButton: onButton)
            }
            HStack(spacing: 4) {
                TouchButton(label: "Select", bit: SnesButton.select, onButton: onButton)
                TouchButton(label: "Start", bit: SnesButton.start, onButton: onButton)
            }
        }
    }
}

/// A press-and-hold touch target: sets the bit when the touch begins, clears it when it ends --
/// deliberately `DragGesture(minimumDistance: 0)`, not `.onTapGesture` (whose action only fires
/// on a completed tap, unsuitable for a game pad that needs to know "is this button currently
/// held"), matching `MainActivity.kt`'s `TouchButton` and its identical rationale for not using a
/// plain `Button`.
private struct TouchButton: View {
    let label: String
    let bit: UInt16
    let onButton: (UInt16, Bool) -> Void

    @GestureState private var isPressed = false

    var body: some View {
        Text(label)
            .frame(width: 56, height: 56)
            .background(Color(white: 0.9))
            .gesture(
                DragGesture(minimumDistance: 0)
                    .updating($isPressed) { _, state, _ in state = true }
            )
            .onChange(of: isPressed) { _, pressed in
                onButton(bit, pressed)
            }
    }
}
