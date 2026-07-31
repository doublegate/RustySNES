//! Touch-to-peripheral mapping (`v1.30.0`).
//!
//! The FFI already exposes [`set_superscope`](crate::MobileCore::set_superscope) and
//! [`set_mouse`](crate::MobileCore::set_mouse), but both take input in units a touchscreen does
//! not have: the Super Scope wants a position in **SNES screen space**, and the Mouse wants a
//! **relative delta** in mouse counts. Getting from a finger to either is real arithmetic — the
//! letterboxed viewport, the aspect correction, and for the Mouse an accumulator that survives
//! sub-count movement.
//!
//! That arithmetic lives here rather than in the platform layer for one reason: otherwise it is
//! written twice, in Kotlin and in Swift, and the two drift. This crate is also the only place it
//! can be **tested** — neither mobile shell has a test harness, and this project has no macOS
//! toolchain at all. Keeping the platform layer to "forward the touch, forward the result" means a
//! bug here is caught by `cargo test` rather than by a user aiming half a screen off.

/// Where the emulated picture actually sits inside the view, in view pixels.
///
/// The shell computes this once per layout change: a letterboxed fit preserves aspect, so there is
/// dead space on two sides and a touch there is **not** a valid aim.
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Left edge of the picture within the view.
    pub x: f32,
    /// Top edge of the picture within the view.
    pub y: f32,
    /// Displayed width of the picture.
    pub width: f32,
    /// Displayed height of the picture.
    pub height: f32,
}

/// A touch mapped into SNES screen space.
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimPoint {
    /// SNES X, clamped to the active framebuffer width.
    pub x: i32,
    /// SNES Y, clamped to the active framebuffer height.
    pub y: i32,
    /// Whether the touch landed on the picture at all.
    ///
    /// A touch in the letterbox bars is reported `false` and clamped, rather than being silently
    /// snapped to an edge as though the user had aimed there. The Super Scope's off-screen state is
    /// meaningful to a game — several use it as "reload" — so the caller must be able to tell.
    pub on_screen: bool,
}

/// Whole mouse counts produced by one drag sample.
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseDelta {
    /// Horizontal counts to pass to [`set_mouse`](crate::MobileCore::set_mouse).
    pub dx: i32,
    /// Vertical counts.
    pub dy: i32,
}

/// Map a touch into SNES screen space, across the FFI.
///
/// Takes the framebuffer size as two scalars because `UniFFI` has no tuple type; the Rust-side
/// [`map_aim`] keeps the tuple its callers already have.
#[uniffi::export]
#[must_use]
pub fn map_touch_to_screen(
    view: Viewport,
    frame_width: u32,
    frame_height: u32,
    tx: f32,
    ty: f32,
) -> AimPoint {
    map_aim(view, (frame_width, frame_height), tx, ty)
}

/// Map a touch at `(tx, ty)` view pixels into SNES screen space.
///
/// `frame` is the **active** framebuffer size, which is not constant: hi-res modes double the
/// width, and overscan changes the height. Taking it as a parameter rather than assuming 256x224 is
/// what keeps aim correct when a game switches mode mid-scene.
#[must_use]
pub fn map_aim(view: Viewport, frame: (u32, u32), tx: f32, ty: f32) -> AimPoint {
    let (fw, fh) = (frame.0.max(1), frame.1.max(1));

    // A degenerate viewport (zero width or height) would divide by zero. It happens in practice —
    // Android reports a 0x0 surface between rotation and first layout — so it is handled rather
    // than assumed away.
    if view.width <= 0.0 || view.height <= 0.0 {
        return AimPoint {
            x: 0,
            y: 0,
            on_screen: false,
        };
    }

    let rel_x = (tx - view.x) / view.width;
    let rel_y = (ty - view.y) / view.height;
    let on_screen = (0.0..=1.0).contains(&rel_x) && (0.0..=1.0).contains(&rel_y);

    // Clamp before the cast: `as` on a float out of an integer's range saturates rather than
    // wrapping, but relying on that is exactly the kind of confident-wrong-number this project has
    // been bitten by before. Clamp explicitly and the cast is total.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let x = (rel_x * fw as f32).clamp(0.0, (fw - 1) as f32) as i32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let y = (rel_y * fh as f32).clamp(0.0, (fh - 1) as f32) as i32;

    AimPoint { x, y, on_screen }
}

/// Turns a drag into SNES Mouse counts.
///
/// The Mouse reports **relative** movement, and a slow drag moves fewer view pixels per frame than
/// one mouse count — so a naive `delta as i32` truncates every such frame to zero and the pointer
/// never moves at all. The residual is carried instead, which is what makes slow movement work.
#[derive(Debug, Default, Clone)]
pub struct MouseAccumulator {
    residual_x: f32,
    residual_y: f32,
    last: Option<(f32, f32)>,
}

impl MouseAccumulator {
    /// A fresh accumulator with no drag in progress.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a drag at `(tx, ty)`. The first sample produces no motion — there is nothing to
    /// subtract from yet, and reporting a delta against a stale origin is how a touch-down teleports
    /// the pointer.
    pub const fn begin(&mut self, tx: f32, ty: f32) {
        self.last = Some((tx, ty));
        self.residual_x = 0.0;
        self.residual_y = 0.0;
    }

    /// Continue a drag, returning whole mouse counts to feed
    /// [`set_mouse`](crate::MobileCore::set_mouse).
    ///
    /// `sensitivity` scales view pixels to mouse counts. Sub-count movement accumulates rather than
    /// being lost.
    pub fn drag(&mut self, tx: f32, ty: f32, sensitivity: f32) -> (i32, i32) {
        let Some((lx, ly)) = self.last else {
            // A drag with no `begin` is a lost touch-down, not a jump from the origin.
            self.begin(tx, ty);
            return (0, 0);
        };
        self.last = Some((tx, ty));

        // `mul_add` rather than `a + b * c`: one rounding instead of two, which matters because
        // the residual is fed back into itself every frame and a systematic bias would drift.
        self.residual_x = (tx - lx).mul_add(sensitivity, self.residual_x);
        self.residual_y = (ty - ly).mul_add(sensitivity, self.residual_y);

        let dx = self.residual_x.trunc();
        let dy = self.residual_y.trunc();
        self.residual_x -= dx;
        self.residual_y -= dy;

        #[allow(clippy::cast_possible_truncation)]
        (dx as i32, dy as i32)
    }

    /// End the drag. The next `drag` without a `begin` will not fabricate motion.
    pub const fn end(&mut self) {
        self.last = None;
        self.residual_x = 0.0;
        self.residual_y = 0.0;
    }
}

/// The [`MouseAccumulator`] as an FFI handle.
///
/// `UniFFI` exports methods taking `&self` only, so the mutable residual lives behind a `Mutex` on
/// the same reasoning as [`MobileCore`](crate::MobileCore): the shell drives this from whichever
/// thread delivers touches, and a poisoned lock is recovered from rather than propagated — a
/// dropped residual loses sub-pixel motion, which is not worth aborting a session over.
#[derive(uniffi::Object, Default)]
pub struct TouchMouse(std::sync::Mutex<MouseAccumulator>);

// Not inside the `#[uniffi::export]` block below: that macro exports every method it contains, and
// a `MutexGuard` has no FFI representation.
impl TouchMouse {
    /// The accumulator, recovering from a poisoned lock rather than propagating it.
    ///
    /// Poisoning means some other thread panicked mid-drag. The worst that state can be is a stale
    /// residual worth under one mouse count, so refusing to serve further touches would be a
    /// strictly worse outcome than continuing.
    fn get(&self) -> std::sync::MutexGuard<'_, MouseAccumulator> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[uniffi::export]
impl TouchMouse {
    /// A fresh handle with no drag in progress.
    #[uniffi::constructor]
    #[must_use]
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::default())
    }

    /// Touch down: start a drag at `(tx, ty)` view pixels.
    pub fn begin(&self, tx: f32, ty: f32) {
        self.get().begin(tx, ty);
    }

    /// Touch move: whole mouse counts to hand to
    /// [`set_mouse`](crate::MobileCore::set_mouse).
    pub fn drag(&self, tx: f32, ty: f32, sensitivity: f32) -> MouseDelta {
        let (dx, dy) = self.get().drag(tx, ty, sensitivity);
        MouseDelta { dx, dy }
    }

    /// Touch up.
    pub fn end(&self) {
        self.get().end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Viewport = Viewport {
        x: 40.0,
        y: 0.0,
        width: 320.0,
        height: 224.0,
    };

    #[test]
    fn the_centre_of_the_picture_maps_to_the_centre_of_the_screen() {
        let p = map_aim(VIEW, (256, 224), 40.0 + 160.0, 112.0);
        assert!(p.on_screen);
        assert_eq!((p.x, p.y), (128, 112));
    }

    #[test]
    fn a_touch_in_the_letterbox_is_reported_off_screen() {
        // x = 10 is left of the picture, which starts at 40. Several games treat the Super Scope's
        // off-screen state as "reload", so this must be distinguishable from aiming at the edge.
        let p = map_aim(VIEW, (256, 224), 10.0, 112.0);
        assert!(!p.on_screen, "a touch in the bar is not an aim");
        assert_eq!(p.x, 0, "and it still clamps to a usable coordinate");
    }

    #[test]
    fn the_far_corner_stays_inside_the_framebuffer() {
        // The bottom-right pixel is (255, 223), not (256, 224) — an off-by-one here indexes out of
        // the framebuffer on the consuming side.
        let p = map_aim(VIEW, (256, 224), 40.0 + 320.0, 224.0);
        assert_eq!((p.x, p.y), (255, 223));
    }

    #[test]
    fn a_vertically_letterboxed_view_subtracts_its_own_origin() {
        // Every other case here uses `view.y == 0`, which makes dropping the `- view.y` term
        // invisible: injecting `rel_y = ty / view.height` passes all of them. A portrait phone
        // holding a 4:3 picture is the common case and has bars on the TOP, not the sides, so this
        // is the orientation most users would actually hit.
        let v = Viewport {
            x: 0.0,
            y: 100.0,
            width: 256.0,
            height: 224.0,
        };
        let top = map_aim(v, (256, 224), 128.0, 100.0);
        assert_eq!(
            top.y, 0,
            "the picture's top edge is SNES row 0, not row 100"
        );
        assert!(top.on_screen);

        let above = map_aim(v, (256, 224), 128.0, 50.0);
        assert!(!above.on_screen, "a touch in the top bar is not an aim");

        let middle = map_aim(v, (256, 224), 128.0, 100.0 + 112.0);
        assert_eq!(middle.y, 112);
    }

    #[test]
    fn hi_res_doubles_the_x_mapping() {
        // The active framebuffer is not constant: a mode switch to 512-wide must move aim with it,
        // which is why `frame` is a parameter rather than a 256x224 assumption.
        let lo = map_aim(VIEW, (256, 224), 40.0 + 160.0, 112.0);
        let hi = map_aim(VIEW, (512, 224), 40.0 + 160.0, 112.0);
        assert_eq!(lo.x, 128);
        assert_eq!(
            hi.x, 256,
            "the same finger position is twice the X at hi-res"
        );
    }

    #[test]
    fn a_degenerate_viewport_does_not_divide_by_zero() {
        // Android reports a 0x0 surface between rotation and first layout.
        let v = Viewport {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        let p = map_aim(v, (256, 224), 10.0, 10.0);
        assert!(!p.on_screen);
        assert_eq!((p.x, p.y), (0, 0));
    }

    #[test]
    fn a_slow_drag_still_moves_the_mouse() {
        // The load-bearing one. At 0.5 counts per pixel a 1-pixel step is half a count, which a
        // plain `as i32` truncates to zero *every frame* — the pointer would never move at all.
        let mut m = MouseAccumulator::new();
        m.begin(100.0, 100.0);
        let mut total = 0;
        for i in 1..=8 {
            #[allow(clippy::cast_precision_loss)]
            let (dx, _) = m.drag(100.0 + i as f32, 100.0, 0.5);
            total += dx;
        }
        assert_eq!(total, 4, "eight 1px steps at 0.5 must yield four counts");
    }

    #[test]
    fn touch_down_does_not_teleport_the_pointer() {
        // Without a fresh origin the first sample is measured against the *previous* touch, and the
        // pointer jumps the whole distance between them — 290 counts here rather than 1.
        //
        // The origin is reset in two places, `begin` and `end`, so this pins the pair rather than
        // either one: injecting a no-op into only one of them leaves the other covering, and the
        // test still passes. Removing both fails it. That redundancy is deliberate — a shell that
        // drops a `begin` (Android delivers `ACTION_MOVE` without a preceding `ACTION_DOWN` after a
        // gesture-recognizer steal) is still safe.
        let mut m = MouseAccumulator::new();
        m.begin(10.0, 10.0);
        let _ = m.drag(20.0, 20.0, 1.0);
        m.end();
        m.begin(300.0, 300.0);
        assert_eq!(
            m.drag(301.0, 300.0, 1.0),
            (1, 0),
            "the new drag measures from its own start, not the old one"
        );
    }

    #[test]
    fn a_drag_without_a_begin_reports_no_motion() {
        let mut m = MouseAccumulator::new();
        assert_eq!(m.drag(50.0, 50.0, 1.0), (0, 0));
    }

    #[test]
    fn the_ffi_handle_carries_the_residual_between_calls() {
        // The whole point of `TouchMouse` being an object rather than a free function: the state
        // has to survive across FFI calls, because each touch event is a separate call from Kotlin.
        // A stateless wrapper would truncate every sample to zero and look like it worked.
        let m = TouchMouse::new();
        m.begin(100.0, 100.0);
        let mut total = 0;
        for i in 1..=8 {
            #[allow(clippy::cast_precision_loss)]
            let d = m.drag(100.0 + i as f32, 100.0, 0.5);
            total += d.dx;
        }
        assert_eq!(total, 4);
        m.end();
        assert_eq!(
            m.drag(500.0, 500.0, 1.0),
            MouseDelta { dx: 0, dy: 0 },
            "after `end` a stray move must not fabricate motion"
        );
    }

    #[test]
    fn the_exported_aim_mapping_agrees_with_the_rust_one() {
        // Two entry points for one calculation; a transposed argument in the FFI shim would be
        // invisible to every other test here.
        let a = map_aim(VIEW, (512, 224), 40.0 + 80.0, 56.0);
        let b = map_touch_to_screen(VIEW, 512, 224, 40.0 + 80.0, 56.0);
        assert_eq!(a, b);
        assert_eq!((b.x, b.y), (128, 56));
    }

    #[test]
    fn the_residual_never_grows_without_bound() {
        // Truncation leaves |residual| < 1 by construction; a sign error would let it drift.
        let mut m = MouseAccumulator::new();
        m.begin(0.0, 0.0);
        for i in 1..500 {
            #[allow(clippy::cast_precision_loss)]
            let _ = m.drag(i as f32 * 0.3, 0.0, 1.0);
        }
        assert!(m.residual_x.abs() < 1.0, "residual {}", m.residual_x);
    }
}
