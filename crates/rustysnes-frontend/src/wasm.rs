//! The `wasm-canvas` entry point (`#[wasm_bindgen(start)]`): a lightweight canvas-2D embed,
//! ported from RustyNES's proven `wasm.rs` shape (not invented from scratch).
//!
//! No `wgpu`/`egui` — a direct `CanvasRenderingContext2d.putImageData` blit of the existing
//! RGBA8 framebuffer (`emu::EmuCore::framebuffer`, already produced for the native wgpu texture;
//! no PPU/core changes needed), a `requestAnimationFrame` loop paced by the same
//! [`crate::pacing::Pacer`] the native synchronous drive uses, keyboard input via DOM
//! `keydown`/`keyup` events (reusing [`crate::input::KeyBindings`] unchanged — its default binds
//! are already stored as `KeyboardEvent.code` strings), and ROM loading via `<input
//! type="file">`. This stage proves a real, visible, playable demo exists fast, without needing
//! `app.rs`/`audio.rs` un-gated for `wasm32` yet — that unification is `wasm-winit` (T-81-006).
//!
//! The two wasm frontends (`wasm-winit` full / `wasm-canvas` lightweight embed) each provide a
//! unique `#[wasm_bindgen(start)]`; exactly one is compiled per the active feature.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;
use web_sys::{
    CanvasRenderingContext2d, Document, Event, FileReader, HtmlCanvasElement, HtmlInputElement,
    ImageData, KeyboardEvent, ProgressEvent,
};

use crate::config::Region;
use crate::emu::EmuCore;
use crate::input::{Buttons, KeyBindings};
use crate::pacing::Pacer;
use crate::wasm_audio;

/// The wasm-canvas frontend's whole live state: the emulator, the latched P1 pad, the keyboard
/// binding table, and the wall-clock pacer (so a 144 Hz display doesn't run emulation 2.4x too
/// fast — `pacing.rs`'s module doc). wasm32 is single-threaded, so a plain (non-atomic)
/// `thread_local`/`RefCell` is sufficient; every access here is synchronous DOM-callback code.
struct Emu {
    core: EmuCore,
    pad1: Buttons,
    keys: KeyBindings,
    pacer: Pacer,
}

thread_local! {
    static EMU: RefCell<Emu> = RefCell::new(Emu {
        core: EmuCore::new(0, Region::Ntsc),
        pad1: Buttons::default(),
        keys: KeyBindings::default(),
        pacer: Pacer::new(Region::Ntsc.frame_rate()),
    });
}

/// The wasm entry point. `trunk` calls this on module load.
///
/// # Errors
/// Returns a [`JsValue`] if the page is missing the `#snes-canvas` / `#rom-input` elements the
/// embed needs, or the 2D rendering context can't be obtained.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let canvas: HtmlCanvasElement = document
        .get_element_by_id("snes-canvas")
        .ok_or_else(|| JsValue::from_str("missing #snes-canvas"))?
        .dyn_into()?;
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("2d context unavailable"))?
        .dyn_into()?;

    install_rom_loader(&document)?;
    install_keyboard_handlers(&document);
    start_raf_loop(canvas, ctx);

    web_sys::console::log_1(&"RustySNES wasm-canvas: ready".into());
    Ok(())
}

/// Wire the `#rom-input` file picker: on change, read the selected file as bytes and load it.
/// [`wasm_audio::ensure_audio`] is called synchronously here (NOT in the async `FileReader`
/// callback below) specifically because a file-picker `change` event is itself a user gesture —
/// the one browser-autoplay-policy-satisfying moment available in this flow.
fn install_rom_loader(document: &Document) -> Result<(), JsValue> {
    let input: HtmlInputElement = document
        .get_element_by_id("rom-input")
        .ok_or_else(|| JsValue::from_str("missing #rom-input"))?
        .dyn_into()?;

    let closure: Closure<dyn FnMut(Event)> = Closure::new(move |event: Event| {
        let _ = wasm_audio::ensure_audio(1.0);
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
            load_rom_file(&file);
        }
    });
    input.set_onchange(Some(closure.as_ref().unchecked_ref()));
    closure.forget(); // outlives this fn; the input element owns the callback for the page's life

    Ok(())
}

/// A `FileReader` paired with the `onload` `Closure` wired to it.
type RomReaderState = (FileReader, Closure<dyn FnMut(ProgressEvent)>);

thread_local! {
    /// A single [`RomReaderState`], built once and reused for every ROM load. `load_rom_file` can
    /// be called many times in one session (the user picking several ROMs in turn); building +
    /// `forget()`-ing a fresh `Closure` each time would leak one closure per load for the page's
    /// lifetime.
    static ROM_READER: RefCell<Option<RomReaderState>> = const { RefCell::new(None) };
}

/// Read `file`'s bytes asynchronously and hand them to [`EmuCore::load_rom`].
fn load_rom_file(file: &web_sys::File) {
    ROM_READER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            let Ok(reader) = FileReader::new() else {
                return;
            };
            let cb_reader = reader.clone();
            let onload: Closure<dyn FnMut(ProgressEvent)> =
                Closure::new(move |_event: ProgressEvent| {
                    let Ok(result) = cb_reader.result() else {
                        return;
                    };
                    let bytes = js_sys::Uint8Array::new(&result).to_vec();
                    EMU.with(|emu| {
                        let mut emu = emu.borrow_mut();
                        match emu.core.load_rom(&bytes) {
                            Ok(()) => web_sys::console::log_1(&"RustySNES: ROM loaded".into()),
                            Err(e) => web_sys::console::log_1(
                                &format!("RustySNES: ROM load failed: {e:?}").into(),
                            ),
                        }
                    });
                });
            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            *slot = Some((reader, onload));
        }
        if let Some((reader, _onload)) = slot.as_ref() {
            let _ = reader.read_as_array_buffer(file);
        }
    });
}

/// Wire `keydown`/`keyup` on `document`, translating `KeyboardEvent.code` through the shared
/// [`KeyBindings`] table (unchanged from native — its keys are already `code` strings).
fn install_keyboard_handlers(document: &Document) {
    let down: Closure<dyn FnMut(KeyboardEvent)> = Closure::new(move |event: KeyboardEvent| {
        EMU.with(|emu| {
            let mut emu = emu.borrow_mut();
            if let Some(button) = emu.keys.button_for(&event.code()) {
                emu.pad1.set(button, true);
                event.prevent_default();
            }
        });
    });
    document.set_onkeydown(Some(down.as_ref().unchecked_ref()));
    down.forget();

    let up: Closure<dyn FnMut(KeyboardEvent)> = Closure::new(move |event: KeyboardEvent| {
        EMU.with(|emu| {
            let mut emu = emu.borrow_mut();
            if let Some(button) = emu.keys.button_for(&event.code()) {
                emu.pad1.set(button, false);
                event.prevent_default();
            }
        });
    });
    document.set_onkeyup(Some(up.as_ref().unchecked_ref()));
    up.forget();
}

/// The self-rescheduling rAF closure's shared cell (it must reference itself to call
/// `request_animation_frame` again from inside its own body).
type RafClosure = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// Start the self-rescheduling `requestAnimationFrame` loop: each callback lets [`Pacer`] decide
/// how many emulated frames (0..=cap) this real tick earns, runs them, drains audio for each, and
/// blits only the LAST frame's framebuffer to the canvas (matching the native present path's
/// "present the latest framebuffer in between" behavior).
fn start_raf_loop(canvas: HtmlCanvasElement, ctx: CanvasRenderingContext2d) {
    let f: RafClosure = Rc::new(RefCell::new(None));
    let g = Rc::clone(&f);

    *g.borrow_mut() = Some(Closure::new(move || {
        EMU.with(|emu| {
            let mut emu = emu.borrow_mut();
            let pad1 = emu.pad1;
            emu.core.set_pad(0, pad1);
            let frames = emu.pacer.tick();
            for _ in 0..frames {
                emu.core.run_frame();
                wasm_audio::push_samples(emu.core.audio());
            }
            if frames > 0 && emu.core.rom_loaded() {
                present(&canvas, &ctx, &emu.core);
            }
        });
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

/// Resize the canvas to the framebuffer's active dims (tracks hi-res mode changes,
/// `docs/ppu.md` §Hi-res) and blit the current frame.
fn present(canvas: &HtmlCanvasElement, ctx: &CanvasRenderingContext2d, core: &EmuCore) {
    let (w, h) = core.fb_dims();
    if canvas.width() != w {
        canvas.set_width(w);
    }
    if canvas.height() != h {
        canvas.set_height(h);
    }
    let fb = core.framebuffer();
    if let Ok(image) = ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(fb), w, h)
    {
        let _ = ctx.put_image_data(&image, 0, 0);
    }
}

/// `window.requestAnimationFrame`, panicking only if `window` itself is unavailable (impossible
/// once [`start`] has already run — it required `window()` to succeed).
fn request_animation_frame(closure: &Closure<dyn FnMut()>) {
    let _ = web_sys::window()
        .expect("window available (start() already required it)")
        .request_animation_frame(closure.as_ref().unchecked_ref());
}
