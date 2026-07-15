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
        PeripheralInputKind::Mouse => sync_mouse(egui_ctx, gfx, emu),
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
///
/// `egui`'s delta is in logical *points*, not the physical pixels `gfx.config.{width,height}`
/// (`size_in_pixels`) use, and a raw point delta would also scale with the window's own
/// size/present scale (3x window scale = 3x-too-fast mouse) — found in review (PR #114, both
/// bots independently). Converts to physical pixels via `pixels_per_point`, then rescales through
/// the same letterbox transform [`sync_superscope`] uses, into SNES pixel space, so sensitivity
/// stays constant regardless of window size, present scale, or host display DPI.
fn sync_mouse(egui_ctx: &egui::Context, gfx: &Gfx, emu: &mut EmuCore) {
    let (delta_points, left, right) = egui_ctx.input(|i| {
        (
            i.pointer.delta(),
            i.pointer.button_down(egui::PointerButton::Primary),
            i.pointer.button_down(egui::PointerButton::Secondary),
        )
    });
    let ppp = egui_ctx.pixels_per_point();
    let delta_px = (delta_points.x * ppp, delta_points.y * ppp);
    let (dx, dy) = scale_delta_to_snes(
        (gfx.config.width, gfx.config.height),
        gfx.letterbox_scale(),
        delta_px,
        emu.fb_dims(),
    );
    #[allow(clippy::cast_possible_truncation)]
    emu.set_mouse(0, dx.round() as i32, dy.round() as i32, left, right);
}

/// Super Scope: absolute aim position (mapped from window pixels through the letterbox transform
/// into SNES `0..256`/`0..239` screen space) plus trigger/cursor/turbo. No fourth mouse button
/// exists to drive `scope::PAUSE`, so it stays unset here — matches this module's own "host
/// mouse input only" scope; a future keyboard-bound Pause is a natural, separate follow-up.
fn sync_superscope(egui_ctx: &egui::Context, gfx: &Gfx, emu: &mut EmuCore) {
    let (pointer_pt, left, right, middle) = egui_ctx.input(|i| {
        (
            i.pointer.latest_pos(),
            i.pointer.button_down(egui::PointerButton::Primary),
            i.pointer.button_down(egui::PointerButton::Secondary),
            i.pointer.button_down(egui::PointerButton::Middle),
        )
    });
    let pixels_per_point = egui_ctx.pixels_per_point();
    // `SuperScopeState::set_input` always clamps/offscreen-checks against the SNES's BASE
    // 256-wide, non-interlaced-height screen space, unconditionally — never the game's current
    // (possibly pixel-doubled) video mode — so this maps into `logical_snes_dims`, not the raw
    // (possibly hi-res/interlaced) `emu.fb_dims()` (found in review, PR #114, Copilot).
    let target_dims = logical_snes_dims(emu.fb_dims());
    // Matches `SuperScopeState::set_input`'s own `-16` fringe convention (one step further out to
    // stay unambiguously off-screen after its `-16..=dimension+16` clamp) — same sentinel
    // `rustysnes-libretro`'s own `poll_port_input` uses for an off-screen lightgun read.
    let (x, y) = pointer_pt
        .and_then(|p| {
            let px = (p.x * pixels_per_point, p.y * pixels_per_point);
            pointer_to_snes_pixel(gfx, px, target_dims)
        })
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

/// Derive the SNES's fixed BASE screen dims (`256` wide, `224`/`239` tall) from the PPU's actual
/// `fb_dims`, halving any axis the current video mode has pixel-doubled: hi-res mode doubles
/// width to `512` (`rustysnes_core::facade::SNES_W_HIRES`), interlace doubles height (up to
/// `rustysnes_core::facade::SNES_H_HIRES`, `448`). Super Scope games are always base-resolution
/// in practice, but this stays correct even if the PPU happens to be in a doubled mode for
/// unrelated reasons — see [`sync_superscope`]'s own doc for why the core needs this space, not
/// the raw framebuffer's.
const fn logical_snes_dims(fb_dims: (u32, u32)) -> (u32, u32) {
    let w = if fb_dims.0 > 256 {
        fb_dims.0 / 2
    } else {
        fb_dims.0
    };
    let h = if fb_dims.1 > 239 {
        fb_dims.1 / 2
    } else {
        fb_dims.1
    };
    (w, h)
}

/// Map a host pointer position (window/surface PHYSICAL pixels — already `pixels_per_point`-
/// scaled by the caller) through `Gfx::letterbox_scale` into `target_dims`'s pixel space, or
/// `None` if the pointer falls outside the letterboxed game image (over the black bars, or
/// off-window). Thin wrapper around [`pointer_to_snes_pixel_pure`] — pulls the live window size +
/// letterbox scale out of `gfx`, which needs a real wgpu device to construct and so can't be
/// exercised directly by a unit test (`gfx.rs`'s own `letterbox_scale_matches_known_cases` test
/// hits the same constraint).
fn pointer_to_snes_pixel(
    gfx: &Gfx,
    pointer_px: (f32, f32),
    target_dims: (u32, u32),
) -> Option<(i32, i32)> {
    pointer_to_snes_pixel_pure(
        (gfx.config.width, gfx.config.height),
        gfx.letterbox_scale(),
        pointer_px,
        target_dims,
    )
}

/// Scale a relative pointer-motion delta (window/surface PHYSICAL pixels) into SNES pixel space,
/// through the same letterbox image-rect `pointer_to_snes_pixel_pure` maps absolute positions
/// through — a delta only needs the rect's SIZE (`image_w`/`image_h`), not its origin, since
/// motion is translation-invariant.
#[allow(clippy::cast_precision_loss)]
fn scale_delta_to_snes(
    win_dims: (u32, u32),
    letterbox_scale: (f32, f32),
    delta_px: (f32, f32),
    fb_dims: (u32, u32),
) -> (f32, f32) {
    let win_w = win_dims.0.max(1) as f32;
    let win_h = win_dims.1.max(1) as f32;
    let (scale_x, scale_y) = letterbox_scale;
    let image_w = (win_w * scale_x).max(1.0);
    let image_h = (win_h * scale_y).max(1.0);
    (
        delta_px.0 * (fb_dims.0 as f32 / image_w),
        delta_px.1 * (fb_dims.1 as f32 / image_h),
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
    target_dims: (u32, u32),
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
        (norm_x * target_dims.0 as f32) as i32,
        (norm_y * target_dims.1 as f32) as i32,
    ))
}

#[cfg(test)]
mod tests {
    use super::{logical_snes_dims, pointer_to_snes_pixel_pure, scale_delta_to_snes};

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

    #[test]
    fn logical_dims_passes_through_base_resolution_unchanged() {
        assert_eq!(logical_snes_dims((256, 224)), (256, 224));
        assert_eq!(logical_snes_dims((256, 239)), (256, 239));
    }

    #[test]
    fn logical_dims_halves_hires_width() {
        assert_eq!(logical_snes_dims((512, 224)), (256, 224));
    }

    #[test]
    fn logical_dims_halves_interlaced_height() {
        assert_eq!(logical_snes_dims((256, 448)), (256, 224));
        assert_eq!(logical_snes_dims((512, 448)), (256, 224));
    }

    #[test]
    fn delta_scale_is_identity_at_1to1_no_letterbox() {
        // An 800x600 window with no letterboxing, 4:3 SNES fb: no rescale needed at these dims
        // (image_w == 800, fb_dims.0 == 256, so a delta only rescales by 256/800 -- verify the
        // ratio, not literal identity, since fb space is smaller than window space here).
        let (dx, dy) = scale_delta_to_snes((800, 600), (1.0, 1.0), (10.0, 10.0), SNES_FB);
        assert!((dx - 3.2).abs() < 1e-4); // 10 * 256/800
        assert!((dy - 3.7333).abs() < 1e-3); // 10 * 224/600
    }

    #[test]
    fn delta_scale_shrinks_with_a_larger_window_at_the_same_letterbox() {
        // Same letterbox fraction, a 2x larger window -> half the delta for the same physical
        // pointer motion (the core bug this fix addresses: 3x window scale felt 3x too fast).
        let small = scale_delta_to_snes((800, 600), (1.0, 1.0), (10.0, 10.0), SNES_FB);
        let large = scale_delta_to_snes((1600, 1200), (1.0, 1.0), (10.0, 10.0), SNES_FB);
        assert!(large.0.mul_add(-2.0, small.0).abs() < 1e-3);
        assert!(large.1.mul_add(-2.0, small.1).abs() < 1e-3);
    }
}
