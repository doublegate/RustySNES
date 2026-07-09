//! The `wasm-winit` entry point (`#[wasm_bindgen(start)]`) — T-81-006, the default wasm build.
//!
//! Unlike the lightweight canvas-2D embed (`wasm-canvas`, in `wasm.rs`), this routes the browser
//! through the SAME winit `App`/`ApplicationHandler<AppEvent>` the native desktop binary uses
//! (`app.rs`), so the wgpu render pipeline and the egui debugger overlay work on the web exactly
//! as they do natively — ported from RustyNES's own `wasm_winit.rs` (confirmed by reading its
//! source directly, not inferred).
//!
//! ## Lifecycle
//!
//! 1. [`start`] (`#[wasm_bindgen(start)]`) fires when the `.wasm` loads. It installs the panic
//!    hook and calls [`crate::app::App::run_wasm`], which builds the typed `EventLoop<AppEvent>`,
//!    wires an [`crate::app::App::new_empty`] with the event-loop proxy, and `spawn_app`s it
//!    (non-blocking).
//! 2. winit's `resumed` creates the canvas-backed window and spawns the async `Gfx::new_async`;
//!    when it resolves it sends `AppEvent::GfxReady` back through the proxy (`app.rs`).
//! 3. `start` also wires the `<input type="file" id="rom-input">` ROM picker (the same element
//!    `wasm-canvas` uses): on selection it calls [`crate::wasm_audio::ensure_audio`]
//!    synchronously — the reliable user-gesture point for `AudioContext.resume()`, since the
//!    later async `FileReader.onload` may fall outside the gesture window — then reads the bytes
//!    and sends `AppEvent::RomLoaded` through the proxy, which the `App` turns into a running
//!    `EmuCore`.

use std::cell::RefCell;

use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;
use web_sys::{Event, FileReader, HtmlInputElement};
use winit::event_loop::EventLoopProxy;

use crate::app::{App, AppEvent};
use crate::config::Config;

/// A `FileReader` paired with the `onload` `Closure` wired to it.
type RomReaderState = (FileReader, Closure<dyn FnMut()>);

thread_local! {
    /// A single [`RomReaderState`], built once and reused for every ROM load — the same
    /// leak-avoidance pattern `wasm.rs` (`wasm-canvas`) uses. The user's `<input>` `change`
    /// handler can fire many times in one session (picking several ROMs in turn); building +
    /// `forget()`-ing a fresh `Closure` on every pick would leak one closure per load for the
    /// page's lifetime.
    static ROM_READER: RefCell<Option<RomReaderState>> = const { RefCell::new(None) };
}

/// The wasm entry point. `trunk` calls this on module load.
///
/// # Errors
/// Returns a [`JsValue`] if the `#rom-input` element can't be located.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"RustySNES wasm-winit: booting".into());

    // Build + spawn the winit App; keep the proxy to feed it ROM bytes.
    let proxy = App::run_wasm(Config::default());

    let document = web_sys::window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let rom_input: HtmlInputElement = document
        .get_element_by_id("rom-input")
        .ok_or_else(|| JsValue::from_str("missing #rom-input"))?
        .dyn_into()?;

    let on_change = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        // Gesture-critical, still within the file-picker `change` event's own user activation:
        // NOT deferred into the async `FileReader.onload` callback below.
        let _ = crate::wasm_audio::ensure_audio(1.0);
        let Some(target) = event.target() else {
            return;
        };
        let Ok(input) = target.dyn_into::<HtmlInputElement>() else {
            return;
        };
        let Some(files) = input.files() else {
            return;
        };
        if let Some(file) = files.get(0) {
            load_rom_file(&file, &proxy);
        }
    });
    rom_input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget(); // outlives this fn; the input element owns the callback for the page's life

    web_sys::console::log_1(&"RustySNES wasm-winit: armed — load a ROM to begin".into());
    Ok(())
}

/// The largest file `load_rom_file` will read — real SNES ROMs (even with a coprocessor firmware
/// dump concatenated in some ad-hoc distributions) top out at a few MiB; this is a generous
/// ceiling against a user accidentally selecting an unrelated large file (a video, a disk image)
/// and triggering an unbounded `Vec<u8>` allocation on the wasm heap.
const MAX_ROM_FILE_BYTES: f64 = 32.0 * 1024.0 * 1024.0;

/// Read `file`'s bytes asynchronously and deliver them as an [`AppEvent::RomLoaded`] via `proxy`.
fn load_rom_file(file: &web_sys::File, proxy: &EventLoopProxy<AppEvent>) {
    if file.size() > MAX_ROM_FILE_BYTES {
        web_sys::console::error_1(
            &format!(
                "RustySNES: selected file ({:.1} MiB) exceeds the 32 MiB ROM size limit",
                file.size() / (1024.0 * 1024.0)
            )
            .into(),
        );
        return;
    }
    ROM_READER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            let Ok(reader) = FileReader::new() else {
                return;
            };
            let cb_reader = reader.clone();
            let cb_proxy = proxy.clone();
            let on_load: Closure<dyn FnMut()> = Closure::new(move || {
                let Ok(buffer) = cb_reader.result() else {
                    return;
                };
                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                web_sys::console::log_1(
                    &format!("RustySNES: ROM selected ({} bytes)", bytes.len()).into(),
                );
                let _ = cb_proxy.send_event(AppEvent::RomLoaded(bytes));
            });
            reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
            *slot = Some((reader, on_load));
        }
        if let Some((reader, _onload)) = slot.as_ref()
            && let Err(e) = reader.read_as_array_buffer(file)
        {
            web_sys::console::error_1(
                &format!("RustySNES: FileReader.readAsArrayBuffer failed: {e:?}").into(),
            );
        }
    });
}
