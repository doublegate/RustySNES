//! The greenzone: cached machine states along a TAS timeline (`v1.25.0`, T-FP-G2).
//!
//! Without it, seeking to frame 30,000 means emulating 30,000 frames — several seconds of waiting
//! for every backwards step, which makes frame-by-frame editing unusable. With it, a seek restores
//! the nearest cached state and replays the handful of frames after it.
//!
//! # Why it reuses the rewind compression
//!
//! A greenzone of raw save-states is enormous: 30,000 frames at a few hundred kilobytes is
//! gigabytes. But TAS states have exactly the property [`crate::delta`] exploits — successive
//! states differ in a few hundred bytes — so the same XOR-delta + RLE chain applies unchanged. This
//! is why T-FP-F was sequenced before T-FP-G2 rather than the reverse.
//!
//! # Why entries are sparse
//!
//! A state is kept every [`Interval`] frames, not every frame. The cost of replaying up to
//! `interval - 1` frames from a cached state is microseconds; the cost of storing every frame is
//! the whole problem. The interval is the only tuning knob and it trades memory against seek time
//! directly.
//!
//! # Invalidation is the whole correctness story
//!
//! Emulation is deterministic, so a state is a function of the initial state and every input up to
//! it. Editing frame N makes every cached state from N onward describe a machine that no longer
//! exists. [`Greenzone::invalidate_from`] discards them, and the piano roll returns exactly the
//! frame to pass it. Keeping a stale state would resume from a timeline that was never played —
//! silently, and with a divergence that surfaces much later as an unexplainable desync.

use crate::delta::Chain;

/// How often a state is cached, in frames.
///
/// 30 frames is half a second at 60 Hz: a seek replays at most 29 frames (well under a millisecond)
/// and the chain holds 2% as many entries as a per-frame cache would.
pub type Interval = usize;

/// The default caching interval.
pub const DEFAULT_INTERVAL: Interval = 30;

/// Keyframes within the compression chain.
///
/// Chosen so a full reconstruct replays at most this many deltas. Larger stores less; smaller seeks
/// faster. 8 keeps both costs small at greenzone sizes.
///
/// **Capped at the chain's capacity by [`Greenzone::new`]**, and that is a correctness requirement,
/// not tuning: the chain evicts from the front, so if the interval exceeds the capacity the
/// keyframe is always the first thing evicted and every remaining delta becomes undecodable. A test
/// caught exactly that with a capacity of 4 against this interval of 8.
const KEYFRAME_INTERVAL: usize = 8;

/// Cached machine states along a timeline.
pub struct Greenzone {
    /// The compressed states, oldest first.
    chain: Chain,
    /// The frame number each chain entry corresponds to, in the same order.
    ///
    /// Held alongside rather than inside the chain because [`crate::delta::Chain`] is a generic
    /// byte-sequence store with no notion of what a state *is* — keeping the mapping here is what
    /// lets that stay true.
    frames: Vec<usize>,
    /// Frames between cached states.
    interval: Interval,
}

impl Greenzone {
    /// A greenzone holding at most `capacity` states, caching every `interval` frames.
    ///
    /// `interval` is clamped to at least 1: a zero would mean caching every frame *and* dividing by
    /// zero when deciding whether to.
    #[must_use]
    pub fn new(capacity: usize, interval: Interval) -> Self {
        // See `KEYFRAME_INTERVAL`: an interval larger than the capacity guarantees the keyframe is
        // evicted before the deltas that depend on it.
        let keyframes = KEYFRAME_INTERVAL.min(capacity.max(1));
        Self {
            chain: Chain::new(capacity, keyframes),
            frames: Vec::new(),
            interval: interval.max(1),
        }
    }

    /// Cached states.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether nothing is cached.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Bytes the cached states occupy — what the panel reports so the cost is visible.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.chain.bytes()
    }

    /// The highest frame with a cached state, if any.
    #[must_use]
    pub fn newest_frame(&self) -> Option<usize> {
        self.frames.last().copied()
    }

    /// Whether `frame` is one this greenzone would cache.
    #[must_use]
    pub const fn is_checkpoint(&self, frame: usize) -> bool {
        frame.is_multiple_of(self.interval)
    }

    /// Discard everything.
    pub fn clear(&mut self) {
        self.chain.clear();
        self.frames.clear();
    }

    /// Offer a state for `frame`.
    ///
    /// Ignored unless `frame` is a checkpoint and is strictly after the newest cached one. The
    /// second condition matters: the chain is a *sequence* whose deltas depend on their
    /// predecessors, so appending an out-of-order state would produce a delta against the wrong
    /// base — and the reconstruction would be silently wrong rather than failing.
    pub fn offer(&mut self, frame: usize, state: &[u8]) {
        if !self.is_checkpoint(frame) {
            return;
        }
        if self.newest_frame().is_some_and(|newest| frame <= newest) {
            return;
        }
        self.chain.push(state);
        self.frames.push(frame);
        // The chain enforces its own capacity by evicting the front, so the frame list must drop
        // the same entries or the two would disagree about which state is which — the seek would
        // then restore the right bytes at the wrong frame.
        while self.frames.len() > self.chain.len() {
            let _ = self.frames.remove(0);
        }
    }

    /// The nearest cached frame at or before `frame`, if any.
    #[must_use]
    pub fn nearest_at_or_before(&self, frame: usize) -> Option<usize> {
        self.frames.iter().rev().find(|f| **f <= frame).copied()
    }

    /// Restore the nearest cached state at or before `frame`, returning `(frame, state)`.
    ///
    /// **Consumes** every cached state after the one returned, because the chain can only be read
    /// from its end. That is not a limitation worth hiding: after a seek the states beyond the
    /// target are about to be recomputed anyway, since the reason to seek backwards in a TAS is to
    /// change something.
    ///
    /// If the chain refuses to reconstruct — which it does rather than returning a wrong state —
    /// the entry is dropped and the seek fails. Reporting no state is correct; reporting a state
    /// that is nearly right would resume from a machine that never existed.
    #[must_use]
    pub fn seek(&mut self, frame: usize) -> Option<(usize, Vec<u8>)> {
        let target = self.nearest_at_or_before(frame)?;
        loop {
            let newest = self.newest_frame()?;
            let Some(state) = self.chain.pop() else {
                // Unrecoverable: drop the whole list, since every remaining entry depends on a
                // base that is now gone. Leaving them would report cached states that cannot be
                // restored, which is worse than reporting none.
                self.clear();
                return None;
            };
            let _ = self.frames.pop();
            if newest == target {
                return Some((target, state));
            }
        }
    }

    /// Discard every cached state at or after `frame`.
    ///
    /// Called with the frame a piano-roll edit returned. At-or-after, not after: the state *at*
    /// frame N is computed from the input *at* frame N, so an edit to N invalidates N itself.
    pub fn invalidate_from(&mut self, frame: usize) {
        while self.newest_frame().is_some_and(|newest| newest >= frame) {
            let _ = self.chain.pop();
            let _ = self.frames.pop();
        }
    }
}

impl core::fmt::Debug for Greenzone {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Greenzone")
            .field("states", &self.frames.len())
            .field("bytes", &self.chain.bytes())
            .field("interval", &self.interval)
            .field("newest_frame", &self.newest_frame())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_INTERVAL, Greenzone};

    /// A deterministic pseudo-state that differs a little per frame, like a real one.
    fn state(frame: usize) -> Vec<u8> {
        let mut v = vec![0xAAu8; 4096];
        let idx = (frame * 7) % 4000;
        v[idx] = u8::try_from(frame % 251).unwrap_or(0);
        v
    }

    #[test]
    fn only_checkpoints_are_cached() {
        let mut gz = Greenzone::new(64, 10);
        for frame in 0..25 {
            gz.offer(frame, &state(frame));
        }
        // 0, 10, 20 — three checkpoints in 0..25.
        assert_eq!(gz.len(), 3);
        assert_eq!(gz.newest_frame(), Some(20));
        assert!(gz.is_checkpoint(30) && !gz.is_checkpoint(31));
    }

    /// A seek returns the nearest cached state at or before the target, so the caller replays only
    /// the frames between.
    #[test]
    fn a_seek_returns_the_nearest_earlier_checkpoint() {
        let mut gz = Greenzone::new(64, 10);
        for frame in 0..=50 {
            gz.offer(frame, &state(frame));
        }
        assert_eq!(gz.nearest_at_or_before(37), Some(30));
        let (frame, restored) = gz.seek(37).expect("seek");
        assert_eq!(frame, 30);
        assert_eq!(restored, state(30), "the state must be exact");
        // Everything after the target is consumed — it is about to be recomputed anyway.
        assert_eq!(gz.newest_frame(), Some(20));
    }

    /// Seeking before the first checkpoint has no answer, and must say so rather than returning
    /// frame 0's state for a frame that precedes it.
    #[test]
    fn seeking_before_the_first_state_returns_none() {
        let mut gz = Greenzone::new(8, 10);
        gz.offer(20, &state(20));
        gz.offer(30, &state(30));
        assert!(gz.nearest_at_or_before(19).is_none());
        assert!(gz.seek(5).is_none());
        assert_eq!(gz.len(), 2, "a failed seek consumes nothing");
    }

    /// The whole correctness story: editing frame N discards every state from N onward, because
    /// they describe a machine that no longer exists.
    #[test]
    fn invalidation_discards_from_the_edited_frame_inclusive() {
        let mut gz = Greenzone::new(64, 10);
        for frame in 0..=60 {
            gz.offer(frame, &state(frame));
        }
        assert_eq!(gz.newest_frame(), Some(60));
        gz.invalidate_from(30);
        assert_eq!(
            gz.newest_frame(),
            Some(20),
            "frame 30 itself must go — its state is computed FROM frame 30's input"
        );
        // What survives is still exact.
        let (frame, restored) = gz.seek(25).expect("seek");
        assert_eq!(frame, 20);
        assert_eq!(restored, state(20));
    }

    #[test]
    fn invalidating_before_everything_empties_it() {
        let mut gz = Greenzone::new(16, 10);
        for frame in 0..=30 {
            gz.offer(frame, &state(frame));
        }
        gz.invalidate_from(0);
        assert!(gz.is_empty());
        assert!(gz.seek(100).is_none());
        assert_eq!(gz.bytes(), 0);
    }

    /// The chain and the frame list must never disagree about which state is which, or a seek
    /// restores the right bytes at the wrong frame.
    #[test]
    fn eviction_keeps_the_frame_list_in_step() {
        let mut gz = Greenzone::new(4, 10);
        for frame in 0..=200 {
            gz.offer(frame, &state(frame));
        }
        assert_eq!(gz.len(), 4, "capacity is enforced");
        // Whatever survived must restore exactly, at the frame it claims.
        while let Some(newest) = gz.newest_frame() {
            let (frame, restored) = gz.seek(newest).expect("seek to the newest");
            assert_eq!(frame, newest);
            assert_eq!(restored, state(frame), "frame {frame} restored wrongly");
        }
    }

    /// An out-of-order offer would delta against the wrong base and reconstruct silently wrongly.
    #[test]
    fn out_of_order_offers_are_ignored() {
        let mut gz = Greenzone::new(16, 10);
        gz.offer(30, &state(30));
        gz.offer(20, &state(20));
        gz.offer(30, &state(0)); // a repeat, with different bytes
        assert_eq!(gz.len(), 1);
        assert_eq!(gz.newest_frame(), Some(30));
        let (_, restored) = gz.seek(30).expect("seek");
        assert_eq!(restored, state(30), "the original state, not the repeat");
    }

    /// The reason T-FP-F was sequenced first: TAS states compress the same way rewind states do.
    #[test]
    fn similar_states_compress_far_below_their_raw_size() {
        let mut gz = Greenzone::new(128, 1);
        let mut raw = 0usize;
        for frame in 0..128 {
            let s = state(frame);
            raw += s.len();
            gz.offer(frame, &s);
        }
        assert_eq!(gz.len(), 128);
        assert!(
            gz.bytes() < raw / 4,
            "greenzone used {} of {raw} raw bytes",
            gz.bytes()
        );
    }

    /// A zero interval must not mean "divide by zero when deciding whether to cache".
    #[test]
    fn a_zero_interval_is_clamped_to_every_frame() {
        let mut gz = Greenzone::new(8, 0);
        for frame in 0..5 {
            gz.offer(frame, &state(frame));
        }
        assert_eq!(gz.len(), 5);
        assert!(gz.is_checkpoint(1), "every frame is a checkpoint at 1");
    }

    #[test]
    fn the_default_interval_is_half_a_second() {
        let gz = Greenzone::new(8, DEFAULT_INTERVAL);
        assert!(gz.is_checkpoint(0) && gz.is_checkpoint(30) && !gz.is_checkpoint(15));
        assert!(gz.is_empty());
        // Debug output names the numbers someone tuning this needs.
        let text = format!("{gz:?}");
        assert!(
            text.contains("interval") && text.contains("bytes"),
            "{text}"
        );
    }
}
