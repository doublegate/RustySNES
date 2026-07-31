//! Desync diagnostics — observational telemetry for a [`RollbackSession`].
//!
//! [`RollbackSession`]: crate::session::RollbackSession
//!
//! Peers exchange a periodic confirmed-frame state checksum
//! ([`NetMessage::Checksum`](crate::message::NetMessage::Checksum)). Before `v1.27.0` the **first**
//! mismatch was a fatal [`NetplayError::Desync`](crate::session::NetplayError::Desync) and the
//! frontend tore the session down on it. That is too eager: a burst-reordered pair of `Checksum`
//! messages can momentarily disagree before the deferred comparison pass reconciles them, so a
//! transient network event ended a healthy game.
//!
//! This module records every comparison — matching ones too — and folds them into one graded
//! [`DesyncStatus`] with a hysteresis threshold, so the frontend can distinguish "one odd frame"
//! from "these two machines have genuinely diverged".
//!
//! **Purely observational.** It only ever *reads* values the session already computed (the
//! canonical gameplay digest and the peer's reported digest) and stores copies; it never feeds back
//! into the rollback algorithm, the checksum exchange, or the emulator. Deleting it would leave
//! every produced frame, checksum, and rollback byte-identical, so it cannot perturb the
//! determinism contract (`docs/adr/0004`). It holds no wall-clock either — that lives in the
//! connection layer — so a `RollbackSession` stays seeded and reproducible.

use std::collections::VecDeque;

/// The graded desync verdict derived from the recent comparison history.
///
/// This is the **single desync surface** the frontend renders. Folding the raw counters into one
/// enum here rather than in the UI means the hysteresis rule lives next to the data it guards, and
/// a second consumer (a headless log, a test) cannot re-derive it differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesyncStatus {
    /// Every comparison so far matched — the peers are in lockstep.
    InSync,
    /// At least one frame has mismatched, but the current consecutive run is below the confirm
    /// threshold: either a transient (a later match reset the run, leaving a sticky historical
    /// mismatch) or a run still building toward a confirmed desync.
    Suspect {
        /// The current consecutive-mismatch run (`0` if the last compare matched).
        consecutive: u32,
        /// The earliest frame that ever diverged.
        first_desync_frame: u32,
    },
    /// The run has reached the confirm threshold: a real, sustained divergence.
    ///
    /// **Sticky once entered.** A rollback desync is unrecoverable — the peers cannot re-converge
    /// without a full state resync — so the verdict never silently downgrades back to
    /// [`Suspect`](Self::Suspect) on a later stray match. A surface that flapped between "desynced"
    /// and "fine" would train the user to ignore it.
    Desynced {
        /// The earliest frame that diverged.
        first_desync_frame: u32,
    },
}

/// One recorded confirmed-frame checksum comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrcCompare {
    /// The confirmed frame whose checksums were compared.
    pub frame: u32,
    /// This peer's canonical (combined) gameplay digest for the frame.
    pub local: u64,
    /// The remote peer's reported digest for the frame.
    pub remote: u64,
    /// `true` if `local == remote` — in sync at this frame.
    pub matched: bool,
    /// `true` if the framebuffer-only hashes matched.
    ///
    /// This is what makes a mismatch *diagnosable* rather than just alarming, and it costs nothing
    /// because `NetMessage::Checksum` already carries both hashes:
    ///
    /// * `!matched` but `same_framebuffer` — the picture agrees and only the cumulative cycle term
    ///   diverged, so the bug is in timing.
    /// * `!matched` and `!same_framebuffer` — the rendered picture itself diverged, so the bug is
    ///   in state.
    ///
    /// Always `true` for a matched compare.
    pub same_framebuffer: bool,
}

/// A rolling, allocation-bounded record of recent confirmed-frame checksum comparisons, plus the
/// derived desync status.
///
/// The history is a fixed-capacity ring ([`Self::CAPACITY`]); the first-desync frame and the
/// mismatch counters are **sticky scalars that survive eviction from the ring**, so a session that
/// has been running for an hour still reports the frame where it first diverged rather than
/// forgetting it once 64 newer comparisons arrive.
#[derive(Clone, Debug)]
pub struct DesyncDiagnostics {
    /// Bounded ring of the most recent comparisons (oldest first).
    history: VecDeque<CrcCompare>,
    /// Total comparisons recorded across all time — not just those still in the ring.
    total: u64,
    /// Total mismatched comparisons recorded across all time.
    mismatches: u64,
    /// The earliest frame whose checksums disagreed, if any has yet.
    first_desync_frame: Option<u32>,
    /// Consecutive mismatches ending at the most recent comparison (reset to `0` by any match).
    consecutive_mismatches: u32,
    /// Peak consecutive-mismatch run ever observed. Drives the sticky [`DesyncStatus::Desynced`]
    /// verdict: once the run has *ever* reached [`Self::desync_threshold`] the session is treated
    /// as confirmed-desynced even if a later stray match resets the live run.
    peak_consecutive: u32,
    /// How many consecutive mismatches confirm a real desync (the hysteresis).
    desync_threshold: u32,
    /// The most recent comparison, for the local-vs-remote readout.
    last: Option<CrcCompare>,
}

impl Default for DesyncDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl DesyncDiagnostics {
    /// Maximum comparisons retained in the rolling history ring.
    ///
    /// At the default 30-frame checksum interval (~2 per second) this is ~32 seconds of history —
    /// long enough to see the shape of a divergence, short enough to stay a fixed allocation.
    pub const CAPACITY: usize = 64;

    /// Default hysteresis: how many *consecutive* mismatching comparisons confirm a real desync.
    ///
    /// A checksum is exchanged only every `checksum_interval` frames and covers a *confirmed*
    /// frame, so a legitimate one-off mismatch is nearly impossible on a correct implementation —
    /// but a burst-reordered pair of `Checksum` messages can momentarily disagree before the
    /// deferred comparison pass reconciles them. Requiring **3** in a row (~1.5 s at the default
    /// interval) rejects that transient while still declaring a genuine divergence promptly.
    pub const DEFAULT_DESYNC_THRESHOLD: u32 = 3;

    /// A fresh, empty record with the default confirm threshold.
    ///
    /// `const` so `RollbackSession::new` — which is itself `const fn` — can hold one without
    /// giving that up.
    #[must_use]
    pub const fn new() -> Self {
        Self::with_threshold(Self::DEFAULT_DESYNC_THRESHOLD)
    }

    /// A fresh, empty record with an explicit confirm threshold.
    ///
    /// A threshold of `0` is treated as `1` (the very first mismatch confirms) rather than as
    /// "confirm immediately and always", which is what a literal `>= 0` comparison would mean —
    /// that would report [`DesyncStatus::Desynced`] on a session that has never mismatched.
    #[must_use]
    pub const fn with_threshold(desync_threshold: u32) -> Self {
        Self {
            history: VecDeque::new(),
            total: 0,
            mismatches: 0,
            first_desync_frame: None,
            consecutive_mismatches: 0,
            peak_consecutive: 0,
            // Spelled out rather than `.max(1)`: `Ord::max` is not `const`, and this constructor
            // must be so the session's own `const fn new` survives.
            desync_threshold: if desync_threshold == 0 {
                1
            } else {
                desync_threshold
            },
            last: None,
        }
    }

    /// Record one confirmed-frame checksum comparison.
    ///
    /// `local`/`remote` are the combined gameplay digests; `local_fb`/`remote_fb` are the
    /// framebuffer-only hashes, used only to classify a mismatch. The session compares each
    /// confirmed frame at most once, so no frame is double-counted.
    pub fn record(&mut self, frame: u32, local: u64, remote: u64, local_fb: u64, remote_fb: u64) {
        let matched = local == remote;
        let entry = CrcCompare {
            frame,
            local,
            remote,
            matched,
            same_framebuffer: local_fb == remote_fb,
        };
        self.total = self.total.saturating_add(1);
        if matched {
            self.consecutive_mismatches = 0;
        } else {
            self.mismatches = self.mismatches.saturating_add(1);
            self.consecutive_mismatches = self.consecutive_mismatches.saturating_add(1);
            self.peak_consecutive = self.peak_consecutive.max(self.consecutive_mismatches);
            // `min`, not "first one seen": comparisons are matched by frame number out of a pair of
            // pending queues, so they can be recorded out of order. The EARLIEST diverging frame is
            // the one worth reporting — it is where a bisect would start.
            self.first_desync_frame = Some(self.first_desync_frame.map_or(frame, |f| f.min(frame)));
        }
        if self.history.len() == Self::CAPACITY {
            self.history.pop_front();
        }
        self.history.push_back(entry);
        self.last = Some(entry);
    }

    /// `true` if no mismatch has ever been recorded.
    #[must_use]
    pub const fn in_sync(&self) -> bool {
        self.first_desync_frame.is_none()
    }

    /// The graded [`DesyncStatus`] verdict — the frontend's single desync surface.
    #[must_use]
    pub const fn status(&self) -> DesyncStatus {
        match self.first_desync_frame {
            None => DesyncStatus::InSync,
            Some(first) => {
                // Keyed on the PEAK run, not the live one: once the run has ever reached the
                // threshold the session is confirmed desynced, and a later stray match must not
                // downgrade it (see `DesyncStatus::Desynced`).
                if self.peak_consecutive >= self.desync_threshold {
                    DesyncStatus::Desynced {
                        first_desync_frame: first,
                    }
                } else {
                    DesyncStatus::Suspect {
                        consecutive: self.consecutive_mismatches,
                        first_desync_frame: first,
                    }
                }
            }
        }
    }

    /// `true` once the run has ever reached the confirm threshold — i.e. [`Self::status`] is
    /// [`DesyncStatus::Desynced`].
    #[must_use]
    pub const fn is_desynced(&self) -> bool {
        matches!(self.status(), DesyncStatus::Desynced { .. })
    }

    /// The confirm threshold in effect.
    #[must_use]
    pub const fn desync_threshold(&self) -> u32 {
        self.desync_threshold
    }

    /// Peak consecutive-mismatch run ever observed (survives a later match).
    #[must_use]
    pub const fn peak_consecutive_mismatches(&self) -> u32 {
        self.peak_consecutive
    }

    /// The earliest frame whose checksums disagreed, if any.
    #[must_use]
    pub const fn first_desync_frame(&self) -> Option<u32> {
        self.first_desync_frame
    }

    /// Consecutive mismatches ending at the most recent comparison.
    #[must_use]
    pub const fn consecutive_mismatches(&self) -> u32 {
        self.consecutive_mismatches
    }

    /// Total comparisons recorded across the whole session.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    /// Total mismatched comparisons recorded across the whole session.
    #[must_use]
    pub const fn mismatches(&self) -> u64 {
        self.mismatches
    }

    /// The most recent comparison, if any.
    #[must_use]
    pub const fn last(&self) -> Option<CrcCompare> {
        self.last
    }

    /// The rolling history, oldest first.
    #[must_use]
    pub fn history(&self) -> impl ExactSizeIterator<Item = &CrcCompare> {
        self.history.iter()
    }

    /// Number of entries currently in the rolling history ring.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Record a run of matching compares starting at `frame`.
    fn matches(d: &mut DesyncDiagnostics, frame: u32, n: u32) {
        for i in 0..n {
            d.record(frame + i, 0xAAAA, 0xAAAA, 0xBBBB, 0xBBBB);
        }
    }

    /// Record a run of mismatching compares starting at `frame`.
    fn mismatches_run(d: &mut DesyncDiagnostics, frame: u32, n: u32) {
        for i in 0..n {
            d.record(frame + i, 0xAAAA, 0xCCCC, 0xBBBB, 0xDDDD);
        }
    }

    #[test]
    fn fresh_is_in_sync() {
        let d = DesyncDiagnostics::new();
        assert!(d.in_sync());
        assert_eq!(d.status(), DesyncStatus::InSync);
        assert_eq!(d.first_desync_frame(), None);
        assert_eq!(d.total(), 0);
        assert_eq!(d.last(), None);
        assert_eq!(d.history_len(), 0);
    }

    #[test]
    fn matching_compares_stay_in_sync() {
        let mut d = DesyncDiagnostics::new();
        matches(&mut d, 0, 10);
        assert!(d.in_sync());
        assert_eq!(d.status(), DesyncStatus::InSync);
        assert_eq!(d.total(), 10);
        assert_eq!(d.mismatches(), 0);
    }

    #[test]
    fn one_transient_mismatch_is_suspect_not_desynced() {
        // THE POINT OF THE WHOLE MODULE. Before this, one mismatch was fatal and the frontend
        // disconnected on it. A burst-reordered `Checksum` pair produces exactly this shape.
        let mut d = DesyncDiagnostics::new();
        matches(&mut d, 0, 5);
        mismatches_run(&mut d, 5, 1);
        matches(&mut d, 6, 5);

        assert!(
            !d.is_desynced(),
            "a single transient must not confirm a desync"
        );
        assert_eq!(
            d.status(),
            DesyncStatus::Suspect {
                consecutive: 0,        // the later matches reset the live run...
                first_desync_frame: 5, // ...but the historical fact is sticky
            }
        );
    }

    #[test]
    fn status_applies_hysteresis_then_confirms_and_sticks() {
        let mut d = DesyncDiagnostics::new();
        assert_eq!(d.desync_threshold(), 3);

        mismatches_run(&mut d, 10, 1);
        assert!(matches!(
            d.status(),
            DesyncStatus::Suspect { consecutive: 1, .. }
        ));
        mismatches_run(&mut d, 11, 1);
        assert!(matches!(
            d.status(),
            DesyncStatus::Suspect { consecutive: 2, .. }
        ));

        // Third in a row crosses the threshold.
        mismatches_run(&mut d, 12, 1);
        assert_eq!(
            d.status(),
            DesyncStatus::Desynced {
                first_desync_frame: 10
            }
        );

        // ...and a later match must NOT downgrade it. A rollback desync is unrecoverable, so a
        // surface that flapped back to "fine" would be lying.
        matches(&mut d, 13, 20);
        assert_eq!(d.consecutive_mismatches(), 0, "the live run does reset");
        assert_eq!(
            d.status(),
            DesyncStatus::Desynced {
                first_desync_frame: 10
            },
            "but the verdict is sticky"
        );
    }

    #[test]
    fn first_desync_frame_is_the_earliest_even_recorded_out_of_order() {
        // Comparisons are matched by frame number out of two pending queues, so they can arrive
        // out of order. The earliest diverging frame is the one a bisect would start from.
        let mut d = DesyncDiagnostics::new();
        mismatches_run(&mut d, 90, 1);
        mismatches_run(&mut d, 40, 1);
        assert_eq!(d.first_desync_frame(), Some(40));
    }

    #[test]
    fn framebuffer_hash_classifies_the_kind_of_mismatch() {
        let mut d = DesyncDiagnostics::new();
        // Same picture, different combined digest -> a timing divergence.
        d.record(1, 0x1111, 0x2222, 0xFFFF, 0xFFFF);
        let e = d.last().expect("recorded");
        assert!(!e.matched);
        assert!(e.same_framebuffer, "same picture => timing bug, not state");

        // Different picture -> a state divergence.
        d.record(2, 0x1111, 0x2222, 0xFFFF, 0xEEEE);
        let e = d.last().expect("recorded");
        assert!(!e.matched);
        assert!(!e.same_framebuffer, "different picture => state bug");
    }

    #[test]
    fn history_is_bounded_but_the_scalars_survive_eviction() {
        let mut d = DesyncDiagnostics::new();
        mismatches_run(&mut d, 0, 1);
        let overfill = u32::try_from(DesyncDiagnostics::CAPACITY).expect("capacity fits a u32") * 2;
        matches(&mut d, 1, overfill);

        assert_eq!(
            d.history_len(),
            DesyncDiagnostics::CAPACITY,
            "the ring must stay a fixed allocation"
        );
        assert_eq!(
            d.first_desync_frame(),
            Some(0),
            "the diverging frame must survive falling out of the ring — a long session would \
             otherwise forget where it broke"
        );
        assert_eq!(d.mismatches(), 1);
        assert_eq!(d.peak_consecutive_mismatches(), 1);
    }

    #[test]
    fn a_threshold_of_zero_is_treated_as_one() {
        // Not as "confirm always": a literal `peak >= 0` would report Desynced on a session that
        // has never mismatched at all.
        let mut d = DesyncDiagnostics::with_threshold(0);
        assert_eq!(d.desync_threshold(), 1);
        assert_eq!(d.status(), DesyncStatus::InSync, "nothing recorded yet");

        matches(&mut d, 0, 3);
        assert_eq!(d.status(), DesyncStatus::InSync, "all matched");

        mismatches_run(&mut d, 3, 1);
        assert!(
            d.is_desynced(),
            "with threshold 1 the first mismatch confirms"
        );
    }

    #[test]
    fn history_is_ordered_oldest_first() {
        let mut d = DesyncDiagnostics::new();
        matches(&mut d, 7, 3);
        let frames: Vec<u32> = d.history().map(|c| c.frame).collect();
        assert_eq!(frames, vec![7, 8, 9]);
    }
}
