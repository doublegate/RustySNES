//! `rustysnes-frontend` — the RustySNES frontend library, shared between the native
//! `[[bin]]` (`src/main.rs`) and the wasm32 `cdylib` that `trunk` consumes for the browser.
//!
//! This is the lift-and-adapt of the RustyNES `winit + wgpu + cpal + egui` shell, renamed
//! and swapped for the SNES: a 256x224/239 (512x448 hi-res) framebuffer, the 15-bit BGR555
//! palette decode, and the SNES controller map (B / Y / Select / Start / D-pad / A / X / L /
//! R). The shell runs **every frame** — it is always on, not a bare window. Menu
//! interactions return a [`MenuAction`](ui_shell::MenuAction) dispatched *after* the egui
//! pass; the present path copies the framebuffer under a brief lock and never holds the emu
//! lock inside the egui closure. The frontend owns rate control + run-ahead — NEVER the core
//! (the determinism contract).
//!
//! Phase 5 status: PLAYABLE on native. The chip stack is complete, so the present path decodes
//! the real PPU framebuffer, the S-DSP audio drives the cpal stream, and keyboard/gamepad input
//! reaches the controllers. Save-states, rewind, and run-ahead (`rewind` module) are implemented
//! and config-driven (off by default). The deep debugger panels are still TODO stubs.
//!
//! `v0.8.0 "Instrumentation"`: the `wasm32` build is PLAYABLE too. `wasm-winit` (T-81-006,
//! default) routes the browser through the SAME `App`/`ApplicationHandler`/`Gfx`/egui shell
//! native uses (`app.rs`, `wasm_winit.rs`) — the wgpu render pipeline and the egui debugger
//! overlay work on the web exactly as they do natively. `wasm-canvas` (T-81-005) is a lighter
//! canvas-2D-only fallback embed, still independently functional and selectable.
//
// TODO(v-next): after the second/third Rusty<System>, the console-agnostic shell wants to be
// a shared `rusty-frontend-core` crate parameterized over a `Console` trait (framebuffer
// dims, input map, debugger-panel set). See `frontend_reuse.md` and the ROADMAP. Do NOT
// block v0.1 on it — lift-and-adapt first, factor later.

#![warn(missing_docs)]
// The frontend is the ONE crate (besides any FFI crate) permitted `unsafe` per the architecture
// — the lock-free audio ring (`audio.rs`) needs it. Every block carries a `// SAFETY:` comment;
// the chip stack stays `#![forbid(unsafe_code)]`.
#![allow(unsafe_code)]

pub mod audio_core;
pub mod config;
pub mod debug_snapshot;
pub mod emu;
pub mod gfx;
pub mod input;
pub(crate) mod pacing;
pub mod rewind;
pub mod ui_shell;

// The always-on egui App shell + the run loop — shared by native and `wasm-winit` (T-81-006);
// internal `#[cfg(target_arch = "wasm32")]` branches handle the split (async wgpu init, browser
// ROM loading, `wasm_audio` instead of cpal). `wasm-canvas` does not use this module at all.
pub mod app;
// The native cpal audio device glue. `wasm32` uses `wasm_audio` instead (see its module doc).
#[cfg(not(target_arch = "wasm32"))]
pub mod audio;
#[cfg(all(not(target_arch = "wasm32"), feature = "emu-thread"))]
pub mod emu_thread;

// Native CLI (clap 4) + the structured help-topic registry + the ratatui help TUI.
// Native-only: a browser tab has no terminal.
#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
#[cfg(all(not(target_arch = "wasm32"), feature = "help-tui"))]
pub mod help_tui;

// The `wasm-canvas` entry point (`#[wasm_bindgen(start)]`). Gated to wasm so it's absent from
// native rustdoc; named here as a code span rather than an intra-doc link. Feature-gated (not
// just target-gated) so it coexists with `wasm_winit` without a duplicate `#[wasm_bindgen(start)]`
// — "exactly one wasm frontend is compiled" per both modules' own docs.
#[cfg(all(target_arch = "wasm32", feature = "wasm-canvas"))]
pub mod wasm;
// The `wasm-winit` entry point (`#[wasm_bindgen(start)]`) — T-81-006, the default wasm feature.
#[cfg(all(target_arch = "wasm32", feature = "wasm-winit"))]
pub mod wasm_winit;
// wasm audio output (`AudioWorkletNode`/`ScriptProcessorNode`), shared by both wasm frontends.
#[cfg(target_arch = "wasm32")]
pub mod wasm_audio;

/// The native NTSC frame rate (the wall-clock pacing target for the produce loop).
pub const FRAME_RATE_NTSC: f64 = 60.098_8;
/// The PAL frame rate (region-switchable; the pacing matrix reads it from config).
pub const FRAME_RATE_PAL: f64 = 50.006_98;
