//! SNES controller input: the 12-button standard-pad bitfield, the keyboard default binds, and
//! (native) the gilrs gamepad auto-bind.
//!
//! The SNES standard controller is a 16-bit serial shift register; the auto-joypad read latches
//! 12 buttons in this canonical bit order (the order `$4218/$4219` returns, MSB first):
//!
//! `B Y Select Start Up Down Left Right A X L R` (then 4 unused / signature bits).
//!
//! [`Buttons`] packs those 12 into a `u16`; the core's controller model (TODO) consumes it. This
//! is the SNES swap of the RustyNES 8-button NES map — same shell plumbing (late-latched,
//! lock-free `SharedInput`), different button set + bit order.
//!
//! Default keyboard binds (P1): D-pad = arrows; **A = X**, **B = Z**, **X = S**, **Y = A**,
//! **L = Q**, **R = W**, Select = `RShift`, Start = Enter. (P2 defaults are a TODO; the config
//! schema already carries a second binding table.)

#![allow(clippy::cast_possible_truncation)]

use serde::{Deserialize, Serialize};

/// A SNES standard-pad button. Discriminants are the canonical auto-joypad bit positions
/// (bit 15 = B down to bit 4 = R; the low 4 bits are unused signature bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Button {
    /// B (bit 15).
    B,
    /// Y (bit 14).
    Y,
    /// Select (bit 13).
    Select,
    /// Start (bit 12).
    Start,
    /// D-pad Up (bit 11).
    Up,
    /// D-pad Down (bit 10).
    Down,
    /// D-pad Left (bit 9).
    Left,
    /// D-pad Right (bit 8).
    Right,
    /// A (bit 7).
    A,
    /// X (bit 6).
    X,
    /// L shoulder (bit 5).
    L,
    /// R shoulder (bit 4).
    R,
}

impl Button {
    /// The bit mask this button occupies in the [`Buttons`] `u16` (canonical auto-joypad order).
    #[must_use]
    pub const fn mask(self) -> u16 {
        let bit = match self {
            Self::B => 15,
            Self::Y => 14,
            Self::Select => 13,
            Self::Start => 12,
            Self::Up => 11,
            Self::Down => 10,
            Self::Left => 9,
            Self::Right => 8,
            Self::A => 7,
            Self::X => 6,
            Self::L => 5,
            Self::R => 4,
        };
        1u16 << bit
    }

    /// Every button, for iteration (binding tables, gamepad mapping).
    pub const ALL: [Self; 12] = [
        Self::B,
        Self::Y,
        Self::Select,
        Self::Start,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::A,
        Self::X,
        Self::L,
        Self::R,
    ];
}

/// The packed 12-button state for one controller (canonical auto-joypad bit order). A set bit
/// means pressed. The core's controller model consumes this each polled frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Buttons(pub u16);

impl Buttons {
    /// Set or clear `button`.
    pub const fn set(&mut self, button: Button, pressed: bool) {
        if pressed {
            self.0 |= button.mask();
        } else {
            self.0 &= !button.mask();
        }
    }

    /// Whether `button` is currently pressed.
    #[must_use]
    pub const fn is_pressed(self, button: Button) -> bool {
        self.0 & button.mask() != 0
    }

    /// Suppress simultaneous opposing D-pad directions (the SNES auto-joypad masks these; many
    /// games misbehave otherwise). Up+Down and Left+Right cancel to neither.
    #[must_use]
    pub const fn sanitize_dpad(mut self) -> Self {
        if self.is_pressed(Button::Up) && self.is_pressed(Button::Down) {
            self.set(Button::Up, false);
            self.set(Button::Down, false);
        }
        if self.is_pressed(Button::Left) && self.is_pressed(Button::Right) {
            self.set(Button::Left, false);
            self.set(Button::Right, false);
        }
        self
    }
}

/// A keyboard binding for one player: which physical key drives each button.
///
/// Keys are stored as the winit `KeyCode` debug name (a string) so the config TOML is
/// human-editable; resolved back at load. For the v0.1 skeleton the lookup is a linear scan
/// (12 entries) — fast enough; a perfect-hash is a v-next nicety.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    /// `(KeyCode-name, Button)` pairs. The default is the P1 layout in the module docs.
    pub binds: Vec<(String, Button)>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        // P1 defaults: D-pad = arrows; A=X B=Z X=S Y=A L=Q R=W Select=RShift Start=Enter.
        let binds = vec![
            ("ArrowUp".into(), Button::Up),
            ("ArrowDown".into(), Button::Down),
            ("ArrowLeft".into(), Button::Left),
            ("ArrowRight".into(), Button::Right),
            ("KeyX".into(), Button::A),
            ("KeyZ".into(), Button::B),
            ("KeyS".into(), Button::X),
            ("KeyA".into(), Button::Y),
            ("KeyQ".into(), Button::L),
            ("KeyW".into(), Button::R),
            ("ShiftRight".into(), Button::Select),
            ("Enter".into(), Button::Start),
        ];
        Self { binds }
    }
}

impl KeyBindings {
    /// Resolve a winit `KeyCode` debug-name (e.g. `"ArrowUp"`, `"KeyZ"`) to the button it's
    /// bound to, if any. The frontend's window handler converts the live key event to its name
    /// via `format!("{key:?}")` and calls this.
    #[must_use]
    pub fn button_for(&self, key_name: &str) -> Option<Button> {
        self.binds
            .iter()
            .find(|(name, _)| name == key_name)
            .map(|(_, b)| *b)
    }

    /// Rebind `button` to `key_name` (the Settings → Input rebind grid). Removes any prior bind
    /// for the same physical key (so one key never drives two buttons) and any prior bind for
    /// `button` (so each button keeps exactly one key) before adding the new pair.
    pub fn rebind(&mut self, key_name: String, button: Button) {
        self.binds
            .retain(|(name, b)| *name != key_name && *b != button);
        self.binds.push((key_name, button));
    }
}

/// Map an Xbox-style gamepad button name to a SNES button (auto-bind to P1).
///
/// Mirrors the RustyNES gilrs auto-bind: face buttons + D-pad + shoulders + Start/Back. The SNES
/// diamond is rotated vs. Xbox: SNES B/A (bottom/right) ~ Xbox A/B; SNES Y/X (left/top) ~ Xbox
/// X/Y.
///
/// `gilrs_button` is the gilrs [`gilrs::Button`] debug name; returns `None` for unmapped inputs.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn gamepad_button(gilrs_button: &str) -> Option<Button> {
    Some(match gilrs_button {
        "South" => Button::B, // Xbox A (bottom) -> SNES B (bottom)
        "East" => Button::A,  // Xbox B (right)  -> SNES A (right)
        "West" => Button::Y,  // Xbox X (left)   -> SNES Y (left)
        "North" => Button::X, // Xbox Y (top)    -> SNES X (top)
        "LeftTrigger" => Button::L,
        "RightTrigger" => Button::R,
        "Start" => Button::Start,
        "Select" => Button::Select,
        "DPadUp" => Button::Up,
        "DPadDown" => Button::Down,
        "DPadLeft" => Button::Left,
        "DPadRight" => Button::Right,
        _ => return None,
    })
}

/// Apply autofire ("turbo") to a button word (`v1.25.0`).
///
/// `turbo_mask` selects which buttons pulse, `period` is the full on/off cycle in frames, and
/// `frame` is a monotonically increasing frame counter. Buttons in the mask are **released** during
/// the off half of the cycle; buttons the player is not holding are never pressed, because this only
/// ever clears bits. That direction matters: an implementation that *set* bits would make an
/// autofire button fire while untouched.
///
/// This runs where host input is sampled, so the core still receives an ordinary button stream and
/// the determinism contract is untouched — the same boundary the DRC servo and post-filters respect.
#[must_use]
pub const fn apply_turbo(
    mut buttons: Buttons,
    turbo_mask: u16,
    period: u32,
    frame: u64,
) -> Buttons {
    if turbo_mask == 0 {
        return buttons;
    }
    // A period below 2 cannot pulse at all (the off half would be empty); callers clamp, and this
    // guards anyway so a bad value degrades to "held" rather than dividing by zero.
    let period = if period < 2 { 2 } else { period };
    let half = (period / 2) as u64;
    let on_phase = frame % (period as u64) < half;
    if !on_phase {
        buttons.0 &= !turbo_mask;
    }
    buttons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turbo_pulses_held_buttons_and_never_presses_unheld_ones() {
        let mask = Button::B.mask() | Button::Y.mask();
        // A held button alternates across the cycle.
        let held = Buttons(mask);
        let states: Vec<bool> = (0..8)
            .map(|f| apply_turbo(held, mask, 8, f).is_pressed(Button::B))
            .collect();
        assert!(states.iter().any(|s| *s), "must be pressed some frames");
        assert!(states.iter().any(|s| !*s), "must be released some frames");
        // An unheld button is never synthesised into a press.
        let idle = Buttons::default();
        for f in 0..32 {
            assert_eq!(
                apply_turbo(idle, mask, 8, f).0,
                0,
                "frame {f} invented a press"
            );
        }
        // Buttons outside the mask are untouched even during the off phase.
        let other = Buttons(Button::A.mask() | mask);
        for f in 0..32 {
            assert!(
                apply_turbo(other, mask, 8, f).is_pressed(Button::A),
                "frame {f} disturbed a non-turbo button"
            );
        }
        // An empty mask is a pure pass-through.
        assert_eq!(apply_turbo(other, 0, 8, 3).0, other.0);
        // A degenerate period must not divide by zero or hold permanently-off.
        let pulses: Vec<bool> = (0..4)
            .map(|f| apply_turbo(held, mask, 0, f).is_pressed(Button::B))
            .collect();
        assert!(pulses.iter().any(|p| *p) && pulses.iter().any(|p| !*p));
    }

    #[test]
    fn masks_are_unique_and_in_high_12_bits() {
        let mut seen = 0u16;
        for b in Button::ALL {
            let m = b.mask();
            assert_eq!(seen & m, 0, "duplicate mask for {b:?}");
            seen |= m;
        }
        // 12 buttons occupy bits 4..=15; the low 4 (signature) bits are clear.
        assert_eq!(seen, 0xFFF0);
    }

    #[test]
    fn dpad_opposing_cancels() {
        let mut b = Buttons::default();
        b.set(Button::Left, true);
        b.set(Button::Right, true);
        let s = b.sanitize_dpad();
        assert!(!s.is_pressed(Button::Left) && !s.is_pressed(Button::Right));
    }

    #[test]
    fn default_binds_cover_all_buttons() {
        let kb = KeyBindings::default();
        for b in Button::ALL {
            assert!(
                kb.binds.iter().any(|(_, bound)| *bound == b),
                "missing default bind for {b:?}"
            );
        }
        assert_eq!(kb.button_for("KeyZ"), Some(Button::B));
        assert_eq!(kb.button_for("Nonexistent"), None);
    }

    #[test]
    fn rebind_replaces_prior_key_and_prior_button_binds() {
        let mut kb = KeyBindings::default();
        // KeyX was A's default; rebind A to KeyP instead.
        kb.rebind("KeyP".into(), Button::A);
        assert_eq!(kb.button_for("KeyP"), Some(Button::A));
        assert_eq!(kb.button_for("KeyX"), None, "the old A bind must be gone");
        assert_eq!(kb.binds.iter().filter(|(_, b)| *b == Button::A).count(), 1);

        // Stealing a key already bound to another button (KeyZ was B's default) must clear that
        // button's old bind rather than leaving two buttons pointing at the same key.
        kb.rebind("KeyZ".into(), Button::Start);
        assert_eq!(kb.button_for("KeyZ"), Some(Button::Start));
        assert!(
            kb.binds.iter().all(|(_, b)| *b != Button::B),
            "B must be left unbound, not still claiming KeyZ"
        );
    }
}
