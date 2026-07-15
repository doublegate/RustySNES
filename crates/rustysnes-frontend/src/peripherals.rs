//! Host-input capture for controller port 2's non-`Gamepad` peripherals (`v1.20.0`).
//!
//! Closes the gap `docs/frontend.md`'s "Peripherals" section named: `config.port2_peripheral`
//! already selects the emulated hardware correctly (`app.rs`'s `set_port_device` re-sync), but
//! nothing fed a real host mouse pointer to [`EmuCore::set_mouse`]/[`EmuCore::set_superscope`]
//! until now.
//!
//! Reuses `Gfx::letterbox_scale` — the SAME clip-space fraction [`Gfx::blit`]/[`Gfx::present`]
//! use to fit the 4:3 SNES image inside the window — rather than re-deriving the letterbox math a
//! second time and risking the two implementations drifting apart. Mirrors
//! `rustysnes-libretro`'s own `poll_port_input` (`crates/rustysnes-libretro/src/lib.rs`): the
//! button-bit mapping and the `(-20, -20)` off-screen sentinel for Super Scope match that crate's
//! existing, already-verified translation exactly, just fed from `egui::Context`'s pointer state
//! instead of libretro's `get_input_state` callback.
//!
//! Portable to wasm on purpose (no `target_arch` gate) — egui's pointer API and
//! [`EmuCore::set_mouse`]/[`EmuCore::set_superscope`] are both already platform-agnostic, so this
//! closes the same gap in the hosted demo too, not just the native build.
//!
//! Super Multitap sub-pads 1-3 are NOT wired here — real host gamepad polling would be the input
//! source for those, but `gilrs::Gilrs` is never actually instantiated anywhere in this crate
//! today (confirmed: `input::gamepad_button` is a real, correct button-name mapping with zero
//! callers) — even controller port 1's OWN gamepad support is unwired, keyboard-only. Wiring
//! Multitap host input is blocked on that separate, larger prerequisite, not a small addition on
//! top of this module; see `docs/frontend.md`'s "Peripherals" section for the honest disposition.

use rustysnes_core::controller::scope;

use crate::emu::EmuCore;
use crate::gfx::Gfx;

/// Feed one frame's worth of host pointer input to `emu`'s port-2 peripheral.
///
/// A no-op for Gamepad/Multitap — Multitap's own host-input source is a separate, unimplemented
/// prerequisite, see the module doc.
pub fn sync(egui_ctx: &egui::Context, gfx: &Gfx, emu: &mut EmuCore, device: PeripheralInputKind) {
    match device {
        PeripheralInputKind::Mouse => sync_mouse(egui_ctx, emu),
        PeripheralInputKind::SuperScope => sync_superscope(egui_ctx, gfx, emu),
        PeripheralInputKind::Other => {}
    }
}

/// Which port-2 peripherals this module actually captures host input for.
///
/// Deliberately NOT [`rustysnes_core::controller::PortDevice`] itself, so a future core
/// peripheral addition can't silently fall through this `match` unnoticed (a new `PortDevice`
/// variant requires an explicit decision here, via [`crate::config::PeripheralKind`]'s own
/// exhaustive `match` in `to_core`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeripheralInputKind {
    /// SNES Mouse — relative pointer motion + left/right buttons.
    Mouse,
    /// Super Scope light gun — absolute pointer position + trigger/cursor/turbo buttons.
    SuperScope,
    /// Gamepad or Multitap — no host pointer input to capture.
    Other,
}

impl From<crate::config::PeripheralKind> for PeripheralInputKind {
    fn from(kind: crate::config::PeripheralKind) -> Self {
        match kind {
            crate::config::PeripheralKind::Mouse => Self::Mouse,
            crate::config::PeripheralKind::SuperScope => Self::SuperScope,
            crate::config::PeripheralKind::Gamepad | crate::config::PeripheralKind::Multitap => {
                Self::Other
            }
        }
    }
}

/// SNES Mouse: relative motion since last frame (real hardware reports deltas, not an absolute
/// position) plus the primary/secondary button state. `egui::PointerState::delta` already
/// accumulates exactly that between egui passes, so no manual last-position tracking is needed.
fn sync_mouse(egui_ctx: &egui::Context, emu: &mut EmuCore) {
    let (delta, left, right) = egui_ctx.input(|i| {
        (
            i.pointer.delta(),
            i.pointer.button_down(egui::PointerButton::Primary),
            i.pointer.button_down(egui::PointerButton::Secondary),
        )
    });
    #[allow(clippy::cast_possible_truncation)]
    emu.set_mouse(
        0,
        delta.x.round() as i32,
        delta.y.round() as i32,
        left,
        right,
    );
}

/// Super Scope: absolute aim position (mapped from window pixels through the letterbox transform
/// into SNES `0..fb_w`/`0..fb_h` pixel space) plus trigger/cursor/turbo. No fourth mouse button
/// exists to drive `scope::PAUSE`, so it stays unset here — matches this module's own "host
/// mouse input only" scope; a future keyboard-bound Pause is a natural, separate follow-up.
fn sync_superscope(egui_ctx: &egui::Context, gfx: &Gfx, emu: &mut EmuCore) {
    let (pointer, left, right, middle) = egui_ctx.input(|i| {
        (
            i.pointer.latest_pos(),
            i.pointer.button_down(egui::PointerButton::Primary),
            i.pointer.button_down(egui::PointerButton::Secondary),
            i.pointer.button_down(egui::PointerButton::Middle),
        )
    });
    let fb_dims = emu.fb_dims();
    // Matches `SuperScopeState::set_input`'s own `-16` fringe convention (one step further out to
    // stay unambiguously off-screen after its `-16..=dimension+16` clamp) — same sentinel
    // `rustysnes-libretro`'s own `poll_port_input` uses for an off-screen lightgun read.
    let (x, y) = pointer
        .and_then(|p| pointer_to_snes_pixel(gfx, (p.x, p.y), fb_dims))
        .unwrap_or((-20, -20));
    let mut buttons = 0u8;
    if left {
        buttons |= scope::TRIGGER;
    }
    if right {
        buttons |= scope::CURSOR;
    }
    if middle {
        buttons |= scope::TURBO;
    }
    emu.set_superscope(0, x, y, buttons);
}

/// Map a host pointer position (window/surface pixels) through `Gfx::letterbox_scale` into
/// SNES pixel space `(0..fb_w, 0..fb_h)`, or `None` if the pointer falls outside the letterboxed
/// game image (over the black bars, or off-window). Thin wrapper around
/// [`pointer_to_snes_pixel_pure`] — pulls the live window size + letterbox scale out of `gfx`,
/// which needs a real wgpu device to construct and so can't be exercised directly by a unit test
/// (`gfx.rs`'s own `letterbox_scale_matches_known_cases` test hits the same constraint).
fn pointer_to_snes_pixel(
    gfx: &Gfx,
    pointer_px: (f32, f32),
    fb_dims: (u32, u32),
) -> Option<(i32, i32)> {
    pointer_to_snes_pixel_pure(
        (gfx.config.width, gfx.config.height),
        gfx.letterbox_scale(),
        pointer_px,
        fb_dims,
    )
}

/// The actual coordinate-mapping math, `Gfx`-free so it's directly unit-testable: window pixels
/// -> the letterboxed game image's on-screen rect -> normalized `0.0..1.0` -> SNES pixel space.
/// `None` when `pointer_px` falls outside that rect (over the black bars, or off-window).
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn pointer_to_snes_pixel_pure(
    win_dims: (u32, u32),
    letterbox_scale: (f32, f32),
    pointer_px: (f32, f32),
    fb_dims: (u32, u32),
) -> Option<(i32, i32)> {
    let win_w = win_dims.0.max(1) as f32;
    let win_h = win_dims.1.max(1) as f32;
    let (scale_x, scale_y) = letterbox_scale;
    let image_w = win_w * scale_x;
    let image_h = win_h * scale_y;
    let origin_x = (win_w - image_w) / 2.0;
    let origin_y = (win_h - image_h) / 2.0;
    let (px, py) = pointer_px;
    if px < origin_x || px >= origin_x + image_w || py < origin_y || py >= origin_y + image_h {
        return None;
    }
    let norm_x = (px - origin_x) / image_w;
    let norm_y = (py - origin_y) / image_h;
    Some((
        (norm_x * fb_dims.0 as f32) as i32,
        (norm_y * fb_dims.1 as f32) as i32,
    ))
}

#[cfg(test)]
mod tests {
    use super::pointer_to_snes_pixel_pure;

    const SNES_FB: (u32, u32) = (256, 224);

    #[test]
    fn center_of_a_4_3_window_maps_to_center_of_the_framebuffer() {
        // No letterboxing (scale (1.0, 1.0)) on an exact-4:3 800x600 window.
        let got = pointer_to_snes_pixel_pure((800, 600), (1.0, 1.0), (400.0, 300.0), SNES_FB);
        assert_eq!(got, Some((128, 112)));
    }

    #[test]
    fn top_left_corner_maps_to_origin() {
        let got = pointer_to_snes_pixel_pure((800, 600), (1.0, 1.0), (0.0, 0.0), SNES_FB);
        assert_eq!(got, Some((0, 0)));
    }

    #[test]
    fn pillarboxed_window_rejects_a_pointer_over_the_black_bars() {
        // A 1920x1080 (16:9) window pillarboxed to 4:3 has scale_x < 1.0 -- a pointer pinned to
        // the far-left window edge lands in the left black bar, outside the letterboxed image.
        let (scale_x, scale_y) = ((4.0 / 3.0) / (1920.0 / 1080.0), 1.0);
        let got =
            pointer_to_snes_pixel_pure((1920, 1080), (scale_x, scale_y), (0.0, 540.0), SNES_FB);
        assert_eq!(got, None);
    }

    #[test]
    fn pillarboxed_window_accepts_a_pointer_inside_the_letterboxed_image() {
        let (scale_x, scale_y) = ((4.0 / 3.0) / (1920.0 / 1080.0), 1.0);
        // Dead center of a pillarboxed window is still dead center of the game image.
        let got =
            pointer_to_snes_pixel_pure((1920, 1080), (scale_x, scale_y), (960.0, 540.0), SNES_FB);
        assert_eq!(got, Some((128, 112)));
    }

    #[test]
    fn a_pointer_past_the_window_edge_is_rejected() {
        let got = pointer_to_snes_pixel_pure((800, 600), (1.0, 1.0), (800.0, 300.0), SNES_FB);
        assert_eq!(got, None);
    }
}
