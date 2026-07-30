//! An on-screen controller (`v1.25.0`, T-FP-G1).
//!
//! Two audiences, one implementation:
//!
//! - **Touch devices** (the `wasm32` build on a phone or tablet) where there is no keyboard at all
//!   and the emulator is otherwise unplayable.
//! - **Anyone**, as a visual input display — the standard way to see what a TAS movie or a netplay
//!   peer is actually pressing.
//!
//! # Why the layout is computed, not drawn
//!
//! [`Layout::hit`] answers "which button is at this point" from the same geometry the renderer
//! draws from. A renderer with its own hardcoded rectangles and a separate hit-test table drift
//! apart the moment either is edited, and the symptom — a button that highlights but does not
//! press — is maddening to diagnose. One source of geometry means they cannot disagree.
//!
//! The layout is resolution-independent: every rectangle is a fraction of the pad's own box, so it
//! scales to any surface without a second set of constants.

use crate::input::{Button, Buttons};

/// A normalised rectangle within the pad's box, `0.0..=1.0` on both axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Whether `(px, py)` — also normalised — falls inside.
    #[must_use]
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// One on-screen button: its area and the pad input it sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadButton {
    /// The button's label.
    pub label: &'static str,
    /// Where it sits.
    pub rect: Rect,
    /// Which pad input it presses.
    ///
    /// The existing [`Button`] enum rather than a raw mask: the bit layout is already defined
    /// once, in `input.rs`'s canonical auto-joypad order, and a second copy here would be one more
    /// place to get it wrong.
    pub button: Button,
}

/// The full on-screen pad layout.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Every button, in draw order.
    pub buttons: Vec<PadButton>,
}

impl Default for Layout {
    fn default() -> Self {
        Self::snes()
    }
}

impl Layout {
    /// The standard SNES pad: d-pad left, face buttons right, shoulders on top, Select/Start centre.
    ///
    /// Proportions follow the real controller closely enough to be recognisable, with the d-pad and
    /// face clusters deliberately **oversized** relative to the shoulder buttons: on a touch screen
    /// the directional and action controls are pressed constantly and the shoulders rarely, so
    /// equal-sized targets would make the common case harder for no benefit.
    #[must_use]
    pub fn snes() -> Self {
        // A helper keeps the table readable; the numbers are fractions of the pad's own box.
        let b = |label, x, y, w, h, button| PadButton {
            label,
            rect: Rect { x, y, w, h },
            button,
        };
        Self {
            buttons: vec![
                // D-pad, as a plus. The centre is deliberately left empty rather than mapped to a
                // diagonal: `Buttons::sanitize_dpad` already rejects opposing pairs, and a
                // touch-diagonal that produces a rejected pair reads as an unresponsive control.
                b("Up", 0.10, 0.26, 0.12, 0.16, Button::Up),
                b("Left", 0.00, 0.44, 0.09, 0.16, Button::Left),
                b("Right", 0.23, 0.44, 0.09, 0.16, Button::Right),
                b("Down", 0.10, 0.62, 0.12, 0.16, Button::Down),
                // Face cluster, in the SNES diamond: X top, Y left, A right, B bottom.
                // The rows are separated by a visible gap rather than sharing an edge: abutting
                // rectangles are ambiguous at the boundary under float rounding, and a
                // `no_two_buttons_overlap` test caught exactly that here.
                b("X", 0.80, 0.26, 0.11, 0.16, Button::X),
                b("Y", 0.66, 0.44, 0.11, 0.16, Button::Y),
                b("A", 0.88, 0.44, 0.11, 0.16, Button::A),
                b("B", 0.80, 0.62, 0.11, 0.16, Button::B),
                // Shoulders.
                b("L", 0.04, 0.05, 0.18, 0.14, Button::L),
                b("R", 0.78, 0.05, 0.18, 0.14, Button::R),
                // Centre pair.
                b("Select", 0.36, 0.70, 0.13, 0.13, Button::Select),
                b("Start", 0.51, 0.70, 0.13, 0.13, Button::Start),
            ],
        }
    }

    /// The button at a normalised point, if any.
    ///
    /// Returns the **first** match in draw order, so overlapping rectangles resolve the same way
    /// they render — which is the only behaviour that cannot surprise someone looking at the pad.
    #[must_use]
    pub fn hit(&self, px: f32, py: f32) -> Option<&PadButton> {
        self.buttons.iter().find(|b| b.rect.contains(px, py))
    }

    /// The pad word for a set of simultaneously-touched points.
    ///
    /// A touch screen reports several points at once, and a player holding a direction while
    /// tapping a face button is the normal case — resolving one point at a time would drop one of
    /// them every frame.
    #[must_use]
    pub fn buttons_for(&self, points: &[(f32, f32)]) -> Buttons {
        let mut word = 0u16;
        for (px, py) in points {
            if let Some(hit) = self.hit(*px, *py) {
                word |= hit.button.mask();
            }
        }
        // Sanitised here rather than by the caller: a two-thumb touch genuinely can land Left and
        // Right at once, and the SNES pad cannot report that.
        Buttons(word).sanitize_dpad()
    }
}

#[cfg(test)]
mod tests {
    use super::Layout;
    use crate::input::Button;

    #[test]
    fn every_button_is_reachable_at_its_own_centre() {
        let layout = Layout::snes();
        assert_eq!(layout.buttons.len(), 12, "the SNES pad has 12 inputs");
        for button in &layout.buttons {
            let cx = button.rect.x + button.rect.w / 2.0;
            let cy = button.rect.y + button.rect.h / 2.0;
            let hit = layout.hit(cx, cy).expect("its own centre must hit it");
            assert_eq!(
                hit.label, button.label,
                "{} centre hit {} instead",
                button.label, hit.label
            );
        }
    }

    /// The hit test and the renderer share one geometry, so a button cannot highlight without
    /// pressing. This is the property that guarantees it: no two rectangles overlap.
    #[test]
    fn no_two_buttons_overlap() {
        let layout = Layout::snes();
        for (i, a) in layout.buttons.iter().enumerate() {
            for b in layout.buttons.iter().skip(i + 1) {
                let disjoint = a.rect.x + a.rect.w <= b.rect.x
                    || b.rect.x + b.rect.w <= a.rect.x
                    || a.rect.y + a.rect.h <= b.rect.y
                    || b.rect.y + b.rect.h <= a.rect.y;
                assert!(disjoint, "{} overlaps {}", a.label, b.label);
            }
        }
    }

    /// Every rectangle stays inside the pad's box, or part of a button is undrawable and
    /// unpressable.
    #[test]
    fn every_rect_is_inside_the_unit_box() {
        for button in &Layout::snes().buttons {
            let r = button.rect;
            assert!(r.x >= 0.0 && r.y >= 0.0, "{} starts outside", button.label);
            assert!(
                r.x + r.w <= 1.0 && r.y + r.h <= 1.0,
                "{} extends past the box",
                button.label
            );
            assert!(r.w > 0.0 && r.h > 0.0, "{} is degenerate", button.label);
        }
    }

    /// Every pad bit is represented exactly once — a missing one is an input the touch player
    /// simply cannot make.
    #[test]
    fn every_pad_bit_appears_exactly_once() {
        let layout = Layout::snes();
        for button in Button::ALL {
            let count = layout.buttons.iter().filter(|b| b.button == button).count();
            assert_eq!(count, 1, "{button:?} appears {count} times");
        }
    }

    /// A touch screen reports several points at once, and holding a direction while tapping a face
    /// button is the normal case — one-at-a-time resolution would drop one every frame.
    #[test]
    fn simultaneous_touches_combine() {
        let layout = Layout::snes();
        let centre = |label: &str| {
            let b = layout
                .buttons
                .iter()
                .find(|b| b.label == label)
                .expect("button");
            (b.rect.x + b.rect.w / 2.0, b.rect.y + b.rect.h / 2.0)
        };
        let word = layout.buttons_for(&[centre("Right"), centre("B"), centre("Start")]);
        assert!(word.is_pressed(Button::Right));
        assert!(word.is_pressed(Button::B));
        assert!(word.is_pressed(Button::Start));
        assert!(!word.is_pressed(Button::Left));
    }

    /// A two-thumb touch genuinely can land Left and Right at once; the SNES pad cannot report
    /// that, so it is sanitised here rather than left for the caller to remember.
    #[test]
    fn opposing_directions_are_sanitised() {
        let layout = Layout::snes();
        let centre = |label: &str| {
            let b = layout
                .buttons
                .iter()
                .find(|b| b.label == label)
                .expect("button");
            (b.rect.x + b.rect.w / 2.0, b.rect.y + b.rect.h / 2.0)
        };
        let word = layout.buttons_for(&[centre("Left"), centre("Right")]);
        assert!(
            !word.is_pressed(Button::Left) && !word.is_pressed(Button::Right),
            "an opposing pair must not reach the console"
        );
        let word = layout.buttons_for(&[centre("Up"), centre("Down")]);
        assert!(!word.is_pressed(Button::Up) && !word.is_pressed(Button::Down));
    }

    /// A touch outside every button presses nothing, rather than the nearest thing.
    #[test]
    fn a_miss_presses_nothing() {
        let layout = Layout::snes();
        assert!(layout.hit(0.5, 0.02).is_none(), "the gap between shoulders");
        assert!(layout.hit(0.16, 0.57).is_none(), "the d-pad's empty centre");
        assert_eq!(layout.buttons_for(&[(0.5, 0.02)]).0, 0);
        assert_eq!(layout.buttons_for(&[]).0, 0);
    }
}
