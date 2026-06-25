//! The wasm32 entry point (`#[wasm_bindgen(start)]`).
//!
//! v0.1.0: a minimal bootstrap that installs the panic hook and logs. The full browser frontend
//! (the same winit + wgpu + egui `App` as native, rendering to a `<canvas>`, driven by
//! `requestAnimationFrame`, with the `AudioWorklet` output path) is a TODO for the implementation
//! phase — see the RustyNES `wasm.rs` / `wasm_winit.rs` / `wasm_audio.rs` for the shape to lift.
//!
//! The two wasm frontends (`wasm-winit` full / `wasm-canvas` lightweight embed) each provide a
//! unique `#[wasm_bindgen(start)]`; exactly one is compiled per the active feature.

use wasm_bindgen::prelude::*;

/// The wasm entry point. `trunk` calls this on module load.
///
/// TODO(impl-phase): build the `App` (the native module shares the shell), create the wgpu
/// surface from the `<canvas>`, and drive the run loop with `requestAnimationFrame`.
///
/// # Errors
/// Returns a [`JsValue`] if browser bootstrap fails (none in the v0.1.0 scaffold, which only
/// installs the panic hook and logs).
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"RustySNES wasm (scaffold) — frontend bootstrap TODO".into());
    Ok(())
}
