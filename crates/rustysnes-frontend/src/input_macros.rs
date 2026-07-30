//! Recorded input macros (`v1.25.0`, T-FP-G1).
//!
//! A macro is a short recorded sequence of pad words replayed on a hotkey — a fighting-game input,
//! a menu sequence, a frame-perfect trick. It is deliberately **not** a TAS movie: a movie owns the
//! whole session from a known start point and is the deterministic-replay artefact, while a macro
//! is injected into live play and makes no determinism claim at all.
//!
//! That distinction is why they are separate types rather than one with a flag. A movie's
//! guarantees (seed, ROM hash, start point) are exactly what a macro cannot have, and a single type
//! would either weaken the movie's contract or make the macro claim one it does not meet.
//!
//! # Playback semantics
//!
//! A macro is applied **on top of** live input, OR-ed rather than replacing it. Replacing would mean
//! a held direction drops the instant a macro starts, which is the opposite of what a
//! motion-plus-button macro is for.

use crate::input::Buttons;

/// The longest macro that can be recorded.
///
/// ~8 seconds at 60 Hz. Long enough for any real input sequence, short enough that a recording
/// left running by accident does not grow without bound — which is the actual failure mode, since
/// a macro is started by a hotkey and stopping it is easy to forget.
pub const MAX_FRAMES: usize = 512;

/// How many macro slots exist.
pub const SLOTS: usize = 4;

/// One recorded macro.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Macro {
    /// The recorded pad words, one per frame.
    pub frames: Vec<u16>,
    /// A user-visible name.
    pub name: String,
}

impl Macro {
    /// Frames recorded.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether nothing is recorded.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Trim leading and trailing empty frames.
    ///
    /// A recording started and stopped by hand always has dead frames at both ends — the time
    /// between pressing the hotkey and pressing the first button. Replaying those makes a macro
    /// feel laggy for a reason nobody would guess at.
    pub fn trim(&mut self) {
        while self.frames.first() == Some(&0) {
            let _ = self.frames.remove(0);
        }
        while self.frames.last() == Some(&0) {
            let _ = self.frames.pop();
        }
    }
}

/// What the macro system is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Nothing.
    #[default]
    Idle,
    /// Recording into a slot.
    Recording {
        /// Which slot.
        slot: usize,
    },
    /// Replaying a slot from a position.
    Playing {
        /// Which slot.
        slot: usize,
        /// The next frame index to emit.
        position: usize,
    },
}

/// The macro recorder/player.
#[derive(Debug, Clone, Default)]
pub struct Macros {
    /// The slots.
    pub slots: [Macro; SLOTS],
    /// Current mode.
    pub mode: Mode,
}

impl Macros {
    /// A fresh, empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin recording into `slot`, discarding whatever it held.
    ///
    /// Out-of-range slots are ignored rather than panicking: a slot index reaches here from a
    /// hotkey binding, and a stray one must not take down the emulator.
    pub fn start_recording(&mut self, slot: usize) {
        if slot >= SLOTS {
            return;
        }
        self.slots[slot].frames.clear();
        self.mode = Mode::Recording { slot };
    }

    /// Stop recording (or playing) and trim the recorded slot.
    pub fn stop(&mut self) {
        if let Mode::Recording { slot } = self.mode
            && slot < SLOTS
        {
            self.slots[slot].trim();
        }
        self.mode = Mode::Idle;
    }

    /// Begin replaying `slot` from its start.
    ///
    /// An empty slot leaves the mode idle rather than entering a playback that emits nothing —
    /// otherwise "playing" would be indistinguishable from "playing silence".
    pub const fn start_playing(&mut self, slot: usize) {
        if slot >= SLOTS || self.slots[slot].is_empty() {
            return;
        }
        self.mode = Mode::Playing { slot, position: 0 };
    }

    /// Whether a recording or playback is in progress.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !matches!(self.mode, Mode::Idle)
    }

    /// Process one frame: record `live`, or OR a replayed frame onto it.
    ///
    /// Returns the pad word to actually use. Recording stops on its own at [`MAX_FRAMES`], and
    /// playback stops at the end of the slot — a macro that silently kept running past its end
    /// would hold whatever its last frame was forever.
    pub fn step(&mut self, live: Buttons) -> Buttons {
        match self.mode {
            Mode::Idle => live,
            Mode::Recording { slot } => {
                if slot < SLOTS {
                    self.slots[slot].frames.push(live.0);
                    if self.slots[slot].frames.len() >= MAX_FRAMES {
                        self.stop();
                    }
                }
                live
            }
            Mode::Playing { slot, position } => {
                let Some(recorded) = self.slots.get(slot).and_then(|m| m.frames.get(position))
                else {
                    self.mode = Mode::Idle;
                    return live;
                };
                let next = position + 1;
                if next >= self.slots[slot].frames.len() {
                    self.mode = Mode::Idle;
                } else {
                    self.mode = Mode::Playing {
                        slot,
                        position: next,
                    };
                }
                // OR-ed onto live input, not replacing it: replacing would drop a held direction
                // the instant a macro starts, which is the opposite of what a motion-plus-button
                // macro is for. `sanitize_dpad` then resolves any opposing pair the combination
                // produced.
                Buttons(live.0 | recorded).sanitize_dpad()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_FRAMES, Macros, Mode, SLOTS};
    use crate::input::{Button, Buttons};

    fn pad(buttons: &[Button]) -> Buttons {
        let mut b = Buttons::default();
        for button in buttons {
            b.set(*button, true);
        }
        b
    }

    #[test]
    fn records_and_replays_a_sequence() {
        let mut m = Macros::new();
        m.start_recording(0);
        let sequence = [
            pad(&[Button::Right]),
            pad(&[Button::Right, Button::B]),
            pad(&[Button::B]),
        ];
        for input in sequence {
            // While recording, live input passes through untouched.
            assert_eq!(m.step(input), input);
        }
        m.stop();
        assert_eq!(m.slots[0].len(), 3);
        assert!(!m.is_active());

        m.start_playing(0);
        for expected in sequence {
            let out = m.step(Buttons::default());
            assert_eq!(out.0, expected.0);
        }
        // Playback ends on its own rather than holding the last frame forever.
        assert!(!m.is_active());
    }

    /// A macro is OR-ed onto live input. Replacing would drop a held direction the instant it
    /// starts — the opposite of what a motion-plus-button macro is for.
    #[test]
    fn playback_combines_with_live_input() {
        let mut m = Macros::new();
        m.start_recording(1);
        m.step(pad(&[Button::B]));
        m.stop();

        m.start_playing(1);
        let out = m.step(pad(&[Button::Right]));
        assert!(out.is_pressed(Button::B), "the macro's button");
        assert!(out.is_pressed(Button::Right), "the live direction survives");
    }

    /// A macro pressing Left while the player holds Right cannot reach the console as both.
    #[test]
    fn a_conflicting_combination_is_sanitised() {
        let mut m = Macros::new();
        m.start_recording(0);
        m.step(pad(&[Button::Left]));
        m.stop();
        m.start_playing(0);
        let out = m.step(pad(&[Button::Right]));
        assert!(
            !out.is_pressed(Button::Left) && !out.is_pressed(Button::Right),
            "an opposing pair must not reach the console"
        );
    }

    /// A recording started and stopped by hand always has dead frames at both ends; replaying them
    /// makes a macro feel laggy for a reason nobody would guess at.
    #[test]
    fn recording_trims_dead_frames_at_both_ends() {
        let mut m = Macros::new();
        m.start_recording(0);
        m.step(Buttons::default());
        m.step(Buttons::default());
        m.step(pad(&[Button::A]));
        m.step(Buttons::default());
        m.step(pad(&[Button::A]));
        m.step(Buttons::default());
        m.stop();
        assert_eq!(m.slots[0].len(), 3, "leading and trailing zeros trimmed");
        assert_eq!(m.slots[0].frames[0], pad(&[Button::A]).0);
        assert_eq!(m.slots[0].frames[1], 0, "an internal gap is preserved");
        assert_eq!(m.slots[0].frames[2], pad(&[Button::A]).0);
    }

    /// A recording left running by accident must not grow without bound — the actual failure mode,
    /// since stopping is a hotkey it is easy to forget.
    #[test]
    fn recording_stops_itself_at_the_cap() {
        let mut m = Macros::new();
        m.start_recording(0);
        for _ in 0..(MAX_FRAMES + 100) {
            m.step(pad(&[Button::A]));
        }
        assert!(!m.is_active(), "recording must stop itself");
        assert_eq!(m.slots[0].len(), MAX_FRAMES);
    }

    /// Playing an empty slot must not enter a mode that emits nothing — "playing" would be
    /// indistinguishable from "playing silence".
    #[test]
    fn playing_an_empty_slot_is_a_no_op() {
        let mut m = Macros::new();
        m.start_playing(0);
        assert!(!m.is_active());
        assert_eq!(m.mode, Mode::Idle);
    }

    /// A stray slot index reaches here from a hotkey binding and must not panic.
    #[test]
    fn out_of_range_slots_are_ignored() {
        let mut m = Macros::new();
        m.start_recording(SLOTS + 5);
        assert!(!m.is_active());
        m.start_playing(usize::MAX);
        assert!(!m.is_active());
        // And a step while idle is a pass-through.
        let live = pad(&[Button::Start]);
        assert_eq!(m.step(live), live);
    }

    /// Re-recording a slot discards what it held rather than appending.
    #[test]
    fn re_recording_replaces_a_slot() {
        let mut m = Macros::new();
        m.start_recording(2);
        for _ in 0..5 {
            m.step(pad(&[Button::A]));
        }
        m.stop();
        assert_eq!(m.slots[2].len(), 5);
        m.start_recording(2);
        m.step(pad(&[Button::B]));
        m.stop();
        assert_eq!(m.slots[2].len(), 1);
        assert_eq!(m.slots[2].frames[0], pad(&[Button::B]).0);
    }
}
