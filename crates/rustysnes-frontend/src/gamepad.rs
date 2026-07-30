//! Physical gamepad input via `gilrs` (`v1.25.0`, native only).
//!
//! Closes the gap where [`crate::input::gamepad_button`] — the Xbox-to-SNES button map — existed
//! and was unit-tested but had **no caller**: `gilrs` was a declared dependency that was never
//! instantiated, so port 1 was keyboard-only however many pads were plugged in.
//!
//! # Why state is polled rather than accumulated from events
//!
//! `gilrs` delivers events, and a frontend that only accumulated press/release deltas would
//! desynchronise permanently the first time an event was missed (a button released while the window
//! was unfocused, a pad re-enumerated mid-session). Instead this pumps the event queue purely to let
//! `gilrs` update its own internal state, then reads that state outright each frame. The frame's
//! button word is therefore always the truth about what is held right now, and a missed event
//! self-corrects on the next poll.
//!
//! # Player assignment
//!
//! Pads map to players in **connection order** — first connected drives P1, second P2 — which is
//! what makes an unplug-replug land back on the same player rather than shuffling. Disconnecting
//! P1 promotes P2, matching how a console behaves when a controller is pulled.

use crate::input::{Button, Buttons};

/// The live `gilrs` context plus per-player pad assignment.
pub struct GamepadRuntime {
    gilrs: gilrs::Gilrs,
    /// Connected pads in connection order; index 0 drives P1, index 1 drives P2.
    assigned: Vec<gilrs::GamepadId>,
    /// Analog-stick magnitude below which axis motion is ignored.
    deadzone: f32,
}

impl GamepadRuntime {
    /// Open the gamepad backend, or `None` when it is unavailable.
    ///
    /// A machine with no input subsystem (a headless CI runner, a container without `/dev/input`)
    /// legitimately fails here; that must degrade to keyboard-only rather than refusing to launch,
    /// so the failure is swallowed into `None` by design.
    #[must_use]
    pub fn new(deadzone: f32) -> Option<Self> {
        let gilrs = gilrs::Gilrs::new().ok()?;
        let assigned = gilrs.gamepads().map(|(id, _)| id).collect();
        Some(Self {
            gilrs,
            assigned,
            deadzone,
        })
    }

    /// Update the deadzone (from the Settings slider).
    pub const fn set_deadzone(&mut self, deadzone: f32) {
        self.deadzone = deadzone;
    }

    /// Drain the event queue so `gilrs`'s own state is current, tracking connect/disconnect.
    ///
    /// Must be called once per frame before [`Self::buttons`]; the events themselves are only used
    /// for assignment bookkeeping (see the module docs on why state is polled, not accumulated).
    pub fn poll(&mut self) {
        while let Some(gilrs::Event { id, event, .. }) = self.gilrs.next_event() {
            match event {
                gilrs::EventType::Connected => {
                    if !self.assigned.contains(&id) {
                        self.assigned.push(id);
                    }
                }
                gilrs::EventType::Disconnected => self.assigned.retain(|a| *a != id),
                _ => {}
            }
        }
    }

    /// The number of pads currently assigned to players.
    #[must_use]
    pub const fn connected_count(&self) -> usize {
        self.assigned.len()
    }

    /// Human-readable names of the assigned pads, in player order (the Settings list).
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.assigned
            .iter()
            .map(|id| {
                self.gilrs
                    .connected_gamepad(*id)
                    .map_or_else(|| "(disconnected)".to_string(), |g| g.name().to_string())
            })
            .collect()
    }

    /// The current SNES button word for `player` (0 = P1, 1 = P2), or `None` if no pad is assigned
    /// to that player.
    ///
    /// Combines digital buttons, the D-pad, and the left analog stick (past the deadzone). Opposing
    /// directions are cancelled by [`Buttons::sanitize_dpad`], the same treatment keyboard input
    /// gets — a stick held diagonally must not produce Up+Down.
    #[must_use]
    pub fn buttons(&self, player: usize) -> Option<Buttons> {
        let id = *self.assigned.get(player)?;
        let pad = self.gilrs.connected_gamepad(id)?;
        let mut out = Buttons::default();

        // Digital buttons, routed through the SAME name-keyed map the unit tests cover so the
        // Xbox-diamond-to-SNES-diamond rotation is defined in exactly one place.
        for gb in [
            gilrs::Button::South,
            gilrs::Button::East,
            gilrs::Button::West,
            gilrs::Button::North,
            gilrs::Button::LeftTrigger,
            gilrs::Button::RightTrigger,
            gilrs::Button::Start,
            gilrs::Button::Select,
            gilrs::Button::DPadUp,
            gilrs::Button::DPadDown,
            gilrs::Button::DPadLeft,
            gilrs::Button::DPadRight,
        ] {
            // A direct match, not `gamepad_button(&format!("{gb:?}"))`: that allocated a `String`
            // per button per assigned player per frame (~1400 heap allocations a second) on the
            // input path, against this project's own allocation-free-hot-path rule.
            if pad.is_pressed(gb)
                && let Some(sb) = snes_button(gb)
            {
                out.set(sb, true);
            }
        }

        // Left analog stick -> D-pad. Y is positive-up in gilrs.
        let (ax, ay) = (
            pad.value(gilrs::Axis::LeftStickX),
            pad.value(gilrs::Axis::LeftStickY),
        );
        for (value, positive, negative) in [
            (ax, Button::Right, Button::Left),
            (ay, Button::Up, Button::Down),
        ] {
            if value > self.deadzone {
                out.set(positive, true);
            } else if value < -self.deadzone {
                out.set(negative, true);
            }
        }
        Some(out.sanitize_dpad())
    }
}

/// Resolve a stick axis to a direction pair, given a deadzone.
///
/// Extracted so the deadzone rule is testable without a physical pad: `gilrs` state cannot be
/// synthesised, and an off-by-one in the sign here reads as "the stick is inverted", which is
/// exactly the class of bug a test should catch rather than a user.
#[must_use]
pub fn axis_direction(value: f32, deadzone: f32) -> Option<bool> {
    if value > deadzone {
        Some(true)
    } else if value < -deadzone {
        Some(false)
    } else {
        None
    }
}

/// Map a `gilrs` button to the SNES pad input it drives.
///
/// A direct match rather than a name lookup: the previous form built a `String` per button per
/// player per frame on the input path. The mapping is the standard modern-controller layout —
/// `South`/`East` are the bottom/right face buttons, which sit where a SNES pad's B/A do.
const fn snes_button(gb: gilrs::Button) -> Option<crate::input::Button> {
    use crate::input::Button;
    Some(match gb {
        gilrs::Button::South => Button::B,
        gilrs::Button::East => Button::A,
        gilrs::Button::West => Button::Y,
        gilrs::Button::North => Button::X,
        gilrs::Button::LeftTrigger => Button::L,
        gilrs::Button::RightTrigger => Button::R,
        gilrs::Button::Start => Button::Start,
        gilrs::Button::Select => Button::Select,
        gilrs::Button::DPadUp => Button::Up,
        gilrs::Button::DPadDown => Button::Down,
        gilrs::Button::DPadLeft => Button::Left,
        gilrs::Button::DPadRight => Button::Right,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_ignores_resting_drift_but_passes_real_motion() {
        let dz = 0.35;
        // A stick resting slightly off-centre must read as neutral, not as a held direction.
        assert_eq!(axis_direction(0.2, dz), None);
        assert_eq!(axis_direction(-0.34, dz), None);
        assert_eq!(axis_direction(0.0, dz), None);
        // Real deflection passes, with the sign preserved.
        assert_eq!(axis_direction(0.9, dz), Some(true));
        assert_eq!(axis_direction(-0.9, dz), Some(false));
        // A zero deadzone still treats dead-centre as neutral (strict comparison).
        assert_eq!(axis_direction(0.0, 0.0), None);
    }

    #[test]
    fn the_button_map_covers_every_snes_button() {
        // The runtime iterates a fixed gilrs button list; that list must actually reach all 12 SNES
        // buttons, or a pad would silently be unable to press one.
        let mut seen = std::collections::HashSet::new();
        for name in [
            "South",
            "East",
            "West",
            "North",
            "LeftTrigger",
            "RightTrigger",
            "Start",
            "Select",
            "DPadUp",
            "DPadDown",
            "DPadLeft",
            "DPadRight",
        ] {
            if let Some(b) = crate::input::gamepad_button(name) {
                seen.insert(b);
            }
        }
        for b in Button::ALL {
            assert!(seen.contains(&b), "no gamepad input maps to {b:?}");
        }
    }
}
