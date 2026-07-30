//! `TAStudio`: a frame-by-frame input editor (`v1.25.0`, T-FP-G2).
//!
//! A TAS is written by editing a **timeline**, not by playing. The piano roll is that timeline: one
//! row per frame, one column per button, and editing any cell invalidates everything after it —
//! which is precisely what makes a greenzone (a cache of machine states along the timeline)
//! necessary rather than a nicety.
//!
//! # The three pieces
//!
//! - [`PianoRoll`] — the input timeline and its edits. Pure data; no emulator, no GPU.
//! - [`greenzone::Greenzone`] — cached machine states, so seeking backwards is instant instead of
//!   replaying from frame zero.
//! - `tastudio_panel` — the grid UI.
//!
//! # Why editing invalidates forward
//!
//! Emulation is deterministic: a state is a function of the initial state and every input up to it.
//! Change frame 500's input and every cached state from 501 on describes a machine that no longer
//! exists. Keeping them would silently resume from a timeline that was never played — the same
//! failure the rewind buffer's bit-identical contract guards against, one level up.
//!
//! This module therefore treats invalidation as the *default* consequence of an edit, and the tests
//! assert it: a piano-roll edit that did not invalidate would be a correctness bug wearing the
//! costume of a performance optimisation.

pub mod greenzone;
pub mod panel;

pub use panel::TasState;

use crate::input::{Button, Buttons};

/// The maximum frame a timeline can address.
///
/// ~9 hours at 60 Hz. Far past any real TAS, and bounded so a corrupt seek or an arithmetic slip
/// cannot ask for an allocation the size of memory.
pub const MAX_FRAMES: usize = 2_000_000;

/// Which controller a column belongs to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Player {
    /// Controller 1.
    #[default]
    One,
    /// Controller 2.
    Two,
}

/// One editable cell: a frame, a player, and a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Frame index.
    pub frame: usize,
    /// Which controller.
    pub player: Player,
    /// Which button.
    pub button: Button,
}

/// The input timeline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PianoRoll {
    /// Per-frame pad words for controller 1.
    p1: Vec<u16>,
    /// Per-frame pad words for controller 2.
    p2: Vec<u16>,
}

impl PianoRoll {
    /// An empty timeline.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Frames in the timeline.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.p1.len()
    }

    /// Whether the timeline is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.p1.is_empty()
    }

    /// Grow the timeline to at least `frames`, padding with empty input.
    ///
    /// Both players grow together, so the two vectors can never disagree about the timeline's
    /// length — a mismatch would make a read past one but not the other silently return zero for a
    /// frame that exists.
    pub fn ensure_len(&mut self, frames: usize) {
        let target = frames.min(MAX_FRAMES);
        if self.p1.len() < target {
            self.p1.resize(target, 0);
            self.p2.resize(target, 0);
        }
    }

    /// The pad words at `frame`, or empty input past the end.
    ///
    /// Past the end reads as *no input pressed* rather than as an error: a TAS being extended plays
    /// into unwritten frames constantly, and that is what an unwritten frame means.
    #[must_use]
    pub fn at(&self, frame: usize) -> (Buttons, Buttons) {
        (
            Buttons(self.p1.get(frame).copied().unwrap_or(0)),
            Buttons(self.p2.get(frame).copied().unwrap_or(0)),
        )
    }

    /// Whether a cell is pressed.
    #[must_use]
    pub fn get(&self, cell: Cell) -> bool {
        let (p1, p2) = self.at(cell.frame);
        match cell.player {
            Player::One => p1.is_pressed(cell.button),
            Player::Two => p2.is_pressed(cell.button),
        }
    }

    /// Set a cell, growing the timeline if needed.
    ///
    /// Returns the frame from which cached state is now invalid — always the edited frame itself,
    /// because the machine state *at* frame N is computed from the input *at* frame N. Returning
    /// `N + 1` would leave one stale state that is exactly the one about to be resumed from.
    pub fn set(&mut self, cell: Cell, pressed: bool) -> usize {
        if cell.frame >= MAX_FRAMES {
            return cell.frame;
        }
        self.ensure_len(cell.frame + 1);
        let word = match cell.player {
            Player::One => &mut self.p1[cell.frame],
            Player::Two => &mut self.p2[cell.frame],
        };
        let mut buttons = Buttons(*word);
        buttons.set(cell.button, pressed);
        *word = buttons.0;
        cell.frame
    }

    /// Toggle a cell. Returns the invalidation frame, as [`Self::set`] does.
    pub fn toggle(&mut self, cell: Cell) -> usize {
        let now = self.get(cell);
        self.set(cell, !now)
    }

    /// Write both players' words at `frame` — how a live-play recording fills the timeline.
    pub fn record(&mut self, frame: usize, p1: Buttons, p2: Buttons) -> usize {
        if frame >= MAX_FRAMES {
            return frame;
        }
        self.ensure_len(frame + 1);
        self.p1[frame] = p1.0;
        self.p2[frame] = p2.0;
        frame
    }

    /// Insert `count` blank frames at `frame`, shifting everything after it later.
    ///
    /// The classic TAS edit — adding a wait. Returns the invalidation frame.
    pub fn insert_frames(&mut self, frame: usize, count: usize) -> usize {
        if count == 0 || frame > self.len() {
            return frame;
        }
        let count = count.min(MAX_FRAMES.saturating_sub(self.len()));
        for _ in 0..count {
            self.p1.insert(frame, 0);
            self.p2.insert(frame, 0);
        }
        frame
    }

    /// Delete `count` frames at `frame`, shifting everything after it earlier.
    pub fn delete_frames(&mut self, frame: usize, count: usize) -> usize {
        if count == 0 || frame >= self.len() {
            return frame;
        }
        let end = (frame + count).min(self.len());
        let _ = self.p1.drain(frame..end);
        let _ = self.p2.drain(frame..end);
        frame
    }

    /// Copy `count` frames starting at `frame`.
    #[must_use]
    pub fn copy(&self, frame: usize, count: usize) -> Vec<(u16, u16)> {
        let end = (frame + count).min(self.len());
        if frame >= self.len() {
            return Vec::new();
        }
        (frame..end).map(|i| (self.p1[i], self.p2[i])).collect()
    }

    /// Overwrite frames starting at `frame` with a clipboard.
    pub fn paste(&mut self, frame: usize, clip: &[(u16, u16)]) -> usize {
        if clip.is_empty() {
            return frame;
        }
        self.ensure_len(frame + clip.len());
        for (i, (a, b)) in clip.iter().enumerate() {
            let Some(slot) = self.p1.get_mut(frame + i) else {
                break;
            };
            *slot = *a;
            self.p2[frame + i] = *b;
        }
        frame
    }

    /// The whole timeline as `(p1, p2)` pairs, for saving to a movie.
    #[must_use]
    pub fn frames(&self) -> Vec<(u16, u16)> {
        self.p1
            .iter()
            .copied()
            .zip(self.p2.iter().copied())
            .collect()
    }

    /// Replace the timeline wholesale, from a loaded movie.
    pub fn load_frames(&mut self, frames: &[(u16, u16)]) {
        self.p1 = frames.iter().map(|(a, _)| *a).take(MAX_FRAMES).collect();
        self.p2 = frames.iter().map(|(_, b)| *b).take(MAX_FRAMES).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::{Button, Cell, MAX_FRAMES, PianoRoll, Player};

    fn cell(frame: usize, button: Button) -> Cell {
        Cell {
            frame,
            player: Player::One,
            button,
        }
    }

    #[test]
    fn setting_and_reading_a_cell_round_trips() {
        let mut roll = PianoRoll::new();
        assert!(roll.is_empty());
        roll.set(cell(10, Button::A), true);
        assert!(roll.get(cell(10, Button::A)));
        assert!(!roll.get(cell(10, Button::B)));
        assert!(!roll.get(cell(9, Button::A)));
        assert_eq!(roll.len(), 11, "the timeline grew to hold frame 10");
        assert!(roll.toggle(cell(10, Button::A)) == 10);
        assert!(!roll.get(cell(10, Button::A)));
    }

    /// The two players' vectors must never disagree about the timeline's length, or a read past
    /// one but not the other silently returns zero for a frame that exists.
    #[test]
    fn both_players_grow_together() {
        let mut roll = PianoRoll::new();
        roll.set(
            Cell {
                frame: 50,
                player: Player::Two,
                button: Button::Start,
            },
            true,
        );
        assert_eq!(roll.len(), 51);
        let (p1, p2) = roll.at(50);
        assert_eq!(p1.0, 0);
        assert!(p2.is_pressed(Button::Start));
    }

    /// An edit invalidates from the edited frame **itself**, not the one after: the machine state
    /// at frame N is computed from the input at frame N, so returning N+1 would leave exactly the
    /// stale state about to be resumed from.
    #[test]
    fn an_edit_invalidates_from_its_own_frame() {
        let mut roll = PianoRoll::new();
        assert_eq!(roll.set(cell(100, Button::B), true), 100);
        assert_eq!(roll.toggle(cell(42, Button::Y)), 42);
        assert_eq!(
            roll.record(7, crate::input::Buttons(1), crate::input::Buttons(2)),
            7
        );
    }

    /// Reading past the end is "nothing pressed", not an error: a TAS being extended plays into
    /// unwritten frames constantly, and that is what an unwritten frame means.
    #[test]
    fn reading_past_the_end_is_empty_input() {
        let roll = PianoRoll::new();
        let (p1, p2) = roll.at(999_999);
        assert_eq!(p1.0, 0);
        assert_eq!(p2.0, 0);
        assert!(!roll.get(cell(999_999, Button::A)));
    }

    /// Inserting a wait is the classic TAS edit — everything after must shift, unchanged.
    #[test]
    fn inserting_frames_shifts_later_input() {
        let mut roll = PianoRoll::new();
        roll.set(cell(0, Button::A), true);
        roll.set(cell(1, Button::B), true);
        roll.set(cell(2, Button::X), true);
        assert_eq!(roll.insert_frames(1, 2), 1);
        assert_eq!(roll.len(), 5);
        assert!(roll.get(cell(0, Button::A)), "before the insert, unchanged");
        assert!(!roll.get(cell(1, Button::B)), "frame 1 is now blank");
        assert!(!roll.get(cell(2, Button::B)));
        assert!(roll.get(cell(3, Button::B)), "shifted by two");
        assert!(roll.get(cell(4, Button::X)));
    }

    #[test]
    fn deleting_frames_pulls_later_input_earlier() {
        let mut roll = PianoRoll::new();
        roll.set(cell(0, Button::A), true);
        roll.set(cell(1, Button::B), true);
        roll.set(cell(2, Button::X), true);
        roll.set(cell(3, Button::Y), true);
        assert_eq!(roll.delete_frames(1, 2), 1);
        assert_eq!(roll.len(), 2);
        assert!(roll.get(cell(0, Button::A)));
        assert!(roll.get(cell(1, Button::Y)), "frame 3 became frame 1");
    }

    /// Deleting past the end truncates rather than panicking on the range.
    #[test]
    fn deleting_past_the_end_is_bounded() {
        let mut roll = PianoRoll::new();
        roll.set(cell(2, Button::A), true);
        assert_eq!(roll.delete_frames(1, 1000), 1);
        assert_eq!(roll.len(), 1);
        // Deleting at or past the end changes nothing.
        assert_eq!(roll.delete_frames(50, 10), 50);
        assert_eq!(roll.len(), 1);
        assert_eq!(
            roll.insert_frames(0, 0),
            0,
            "a zero-count insert is a no-op"
        );
    }

    #[test]
    fn copy_and_paste_move_a_span() {
        let mut roll = PianoRoll::new();
        roll.set(cell(0, Button::A), true);
        roll.set(cell(1, Button::B), true);
        let clip = roll.copy(0, 2);
        assert_eq!(clip.len(), 2);
        assert_eq!(roll.paste(10, &clip), 10);
        assert!(roll.get(cell(10, Button::A)));
        assert!(roll.get(cell(11, Button::B)));
        assert_eq!(roll.len(), 12);
        // Copying past the end yields nothing rather than panicking.
        assert!(roll.copy(1000, 4).is_empty());
        assert_eq!(roll.paste(5, &[]), 5, "an empty paste is a no-op");
    }

    /// The whole timeline round-trips through the movie representation.
    #[test]
    fn frames_round_trip_through_save_and_load() {
        let mut roll = PianoRoll::new();
        roll.set(cell(0, Button::A), true);
        roll.set(
            Cell {
                frame: 1,
                player: Player::Two,
                button: Button::Start,
            },
            true,
        );
        let saved = roll.frames();
        assert_eq!(saved.len(), 2);
        let mut restored = PianoRoll::new();
        restored.load_frames(&saved);
        assert_eq!(restored, roll);
    }

    /// A corrupt seek or an arithmetic slip must not ask for an allocation the size of memory.
    #[test]
    fn the_timeline_is_bounded() {
        let mut roll = PianoRoll::new();
        assert_eq!(
            roll.set(cell(MAX_FRAMES + 10, Button::A), true),
            MAX_FRAMES + 10
        );
        assert!(roll.is_empty(), "an out-of-range edit allocates nothing");
        roll.ensure_len(usize::MAX);
        assert_eq!(roll.len(), MAX_FRAMES);
    }
}
