//! Performance telemetry primitives (`v1.25.0`, T-FP-A).
//!
//! Ported from RustyNES's own `perf.rs`. Today the frontend surfaces a single smoothed FPS float
//! (`pacing.rs`), which cannot answer the question that actually matters when frames hitch: **how
//! bad is the tail?** A mean of 16.6 ms hides a p99 of 40 ms completely, and a p99 of 40 ms is the
//! difference between "smooth" and "visibly stuttering" — so this records distributions, not
//! averages.
//!
//! # Why fixed-capacity ring buffers and exact percentiles
//!
//! Each metric keeps the last [`WINDOW`] samples in a fixed array and sorts a copy on demand. That
//! is deliberately not a streaming estimator (P², t-digest): at this window size an exact sort costs
//! microseconds and only runs when a panel is open, whereas an estimator would be approximate
//! *and* need its own correctness tests. Fixed capacity also means **no allocation in the hot path**
//! — `push` is a store and an index bump, which is what lets it be called every frame
//! unconditionally.
//!
//! This module is pure data: it never touches the emulator, so it cannot perturb what it measures
//! beyond the timestamp the caller already took.

/// How many samples each metric retains (~5 s at 60 Hz).
///
/// Long enough that a p99 means something (it is the worst of ~300 frames, not of 10), short enough
/// that the display reflects *now* rather than a hitch from a minute ago.
pub const WINDOW: usize = 300;

/// A fixed-capacity ring of `f32` samples with exact order statistics.
#[derive(Debug, Clone)]
pub struct Metric {
    samples: [f32; WINDOW],
    /// Where the next sample lands.
    head: usize,
    /// How many slots are populated (`<= WINDOW`), so a partly-filled window is not read as zeros.
    len: usize,
    /// Monotonic count of every sample ever pushed, including evicted ones.
    total: u64,
}

impl Default for Metric {
    fn default() -> Self {
        Self::new()
    }
}

impl Metric {
    /// An empty metric.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            samples: [0.0; WINDOW],
            head: 0,
            len: 0,
            total: 0,
        }
    }

    /// Record one sample, evicting the oldest once the window is full.
    ///
    /// Allocation-free and branch-light: this is called once per frame from the present path.
    pub const fn push(&mut self, value: f32) {
        self.samples[self.head] = value;
        self.head = (self.head + 1) % WINDOW;
        if self.len < WINDOW {
            self.len += 1;
        }
        self.total = self.total.wrapping_add(1);
    }

    /// Populated sample count (`0..=WINDOW`).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether no samples have been recorded yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total samples ever pushed, including evicted ones.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    /// Forget every sample (a panel's "reset" button).
    pub const fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
        self.total = 0;
    }

    /// The most recently pushed sample, or `None` when empty.
    #[must_use]
    pub const fn last(&self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        // `head` points at the NEXT slot, so the newest sample is the one behind it (wrapping).
        let idx = if self.head == 0 {
            WINDOW - 1
        } else {
            self.head - 1
        };
        Some(self.samples[idx])
    }

    /// Arithmetic mean of the window, or `None` when empty.
    #[must_use]
    pub fn mean(&self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        let sum: f32 = self.samples[..self.len].iter().sum();
        #[allow(clippy::cast_precision_loss)]
        Some(sum / self.len as f32)
    }

    /// The window's samples, oldest first — for a sparkline.
    ///
    /// Returns a `Vec` because the ring wraps; a caller wanting zero allocation should read
    /// [`Self::last`] per frame instead. Only called while a panel is open.
    #[must_use]
    pub fn ordered(&self) -> Vec<f32> {
        if self.len < WINDOW {
            return self.samples[..self.len].to_vec();
        }
        let mut out = Vec::with_capacity(WINDOW);
        out.extend_from_slice(&self.samples[self.head..]);
        out.extend_from_slice(&self.samples[..self.head]);
        out
    }

    /// A sorted copy of the populated window (ascending), for order statistics.
    ///
    /// `total_cmp` rather than `partial_cmp().unwrap()`: a NaN sample (a timer that produced one, a
    /// division by a zero frame count) must sort deterministically instead of panicking inside the
    /// comparator and taking down the render thread.
    fn sorted(&self) -> Vec<f32> {
        let mut v = self.samples[..self.len].to_vec();
        v.sort_unstable_by(f32::total_cmp);
        v
    }

    /// The `q`-quantile (`0.0..=1.0`) by nearest-rank, or `None` when empty or `q` is not finite.
    ///
    /// Nearest-rank (not interpolated) because these are latency samples: a reported p99 should be
    /// a value that genuinely occurred, not an average of two frames that never happened.
    ///
    /// A non-finite `q` is rejected rather than clamped: `f32::clamp` propagates `NaN`, and the
    /// float-to-int cast below then saturates it to index 0, so the function would silently answer
    /// "the minimum sample" to a question that had no meaning. `None` says so instead.
    #[must_use]
    pub fn quantile(&self, q: f32) -> Option<f32> {
        if self.len == 0 || !q.is_finite() {
            return None;
        }
        let sorted = self.sorted();
        let q = q.clamp(0.0, 1.0);
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let idx = ((q * (self.len - 1) as f32).round() as usize).min(self.len - 1);
        Some(sorted[idx])
    }

    /// Median (p50).
    #[must_use]
    pub fn p50(&self) -> Option<f32> {
        self.quantile(0.50)
    }

    /// 95th percentile.
    #[must_use]
    pub fn p95(&self) -> Option<f32> {
        self.quantile(0.95)
    }

    /// 99th percentile — the tail that decides whether motion looks smooth.
    #[must_use]
    pub fn p99(&self) -> Option<f32> {
        self.quantile(0.99)
    }

    /// Largest sample in the window.
    #[must_use]
    pub fn max(&self) -> Option<f32> {
        self.quantile(1.0)
    }

    /// Smallest sample in the window.
    #[must_use]
    pub fn min(&self) -> Option<f32> {
        self.quantile(0.0)
    }

    /// A one-line `p50/p95/p99/max` summary, or `"—"` when empty.
    #[must_use]
    pub fn summary(&self) -> String {
        match (self.p50(), self.p95(), self.p99(), self.max()) {
            (Some(a), Some(b), Some(c), Some(d)) => {
                format!("p50 {a:.2} · p95 {b:.2} · p99 {c:.2} · max {d:.2}")
            }
            _ => "—".to_string(),
        }
    }
}

/// Every metric the frontend tracks, plus the counters that are not distributions.
///
/// Produced-vs-presented is tracked as two separate metrics on purpose: they diverge exactly when
/// something is wrong (a present that drew a duplicate frame, a catch-up burst producing several
/// emulated frames for one present), and a single combined number hides that.
#[derive(Debug, Clone, Default)]
pub struct PerfStats {
    /// Wall-clock milliseconds spent producing emulated frame(s) for one present.
    pub produce_ms: Metric,
    /// Wall-clock milliseconds spent on the whole present (upload + passes + egui + submit).
    pub present_ms: Metric,
    /// GPU milliseconds for the render passes, when the `gpu-timing` feature resolved a timestamp.
    pub gpu_ms: Metric,
    /// Audio ring occupancy as a percentage, sampled once per present.
    pub audio_pct: Metric,
    /// Emulated frames produced (may exceed presents during a catch-up burst).
    pub frames_produced: u64,
    /// Presents performed.
    pub frames_presented: u64,
}

impl PerfStats {
    /// A fresh, empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one present's worth of measurements.
    ///
    /// `produce_ms` is `None` when nothing was emulated this present (paused, or zero frames earned
    /// this tick) — recording a 0.0 there would drag every percentile toward zero and make an idle
    /// emulator look impossibly fast, which is precisely the reading that would hide a real problem.
    pub const fn record_present(
        &mut self,
        produce_ms: Option<f32>,
        present_ms: f32,
        audio_pct: Option<f32>,
        produced_frames: u32,
    ) {
        if let Some(ms) = produce_ms {
            self.produce_ms.push(ms);
        }
        self.present_ms.push(present_ms);
        if let Some(pct) = audio_pct {
            self.audio_pct.push(pct);
        }
        self.frames_produced = self.frames_produced.wrapping_add(produced_frames as u64);
        self.frames_presented = self.frames_presented.wrapping_add(1);
    }

    /// Record a resolved GPU pass duration (the `gpu-timing` feature).
    pub const fn record_gpu(&mut self, gpu_ms: f32) {
        self.gpu_ms.push(gpu_ms);
    }

    /// Reset every metric and counter.
    pub const fn reset(&mut self) {
        self.produce_ms.clear();
        self.present_ms.clear();
        self.gpu_ms.clear();
        self.audio_pct.clear();
        self.frames_produced = 0;
        self.frames_presented = 0;
    }

    /// Whether emulation produced more frames than were presented (a catch-up burst happened).
    #[must_use]
    pub const fn is_catching_up(&self) -> bool {
        self.frames_produced > self.frames_presented
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_metric_reports_nothing_rather_than_zero() {
        // Every accessor must distinguish "no data" from "measured zero"; a 0.0 default would make
        // an idle emulator look impossibly fast and hide a real regression.
        let m = Metric::new();
        assert!(m.is_empty() && m.total() == 0);
        assert!(m.last().is_none());
        assert!(m.mean().is_none());
        assert!(m.p50().is_none() && m.p95().is_none() && m.p99().is_none());
        assert!(m.min().is_none() && m.max().is_none());
        assert_eq!(m.summary(), "—");
        assert!(m.ordered().is_empty());
    }

    #[test]
    fn percentiles_are_exact_and_nearest_rank() {
        let mut m = Metric::new();
        for v in 1..=100u16 {
            m.push(f32::from(v));
        }
        assert_eq!(m.len(), 100);
        assert!((m.min().expect("min") - 1.0).abs() < 1e-6);
        assert!((m.max().expect("max") - 100.0).abs() < 1e-6);
        // Nearest-rank: every reported value must be one that genuinely occurred.
        for q in [0.0f32, 0.25, 0.5, 0.95, 0.99, 1.0] {
            let v = m.quantile(q).expect("quantile");
            assert!(
                (v - v.round()).abs() < 1e-6,
                "q={q} produced {v}, which is not an observed integer sample"
            );
        }
        // Monotonic in q.
        let (p50, p95, p99, max) = (
            m.p50().expect("p50"),
            m.p95().expect("p95"),
            m.p99().expect("p99"),
            m.max().expect("max"),
        );
        assert!(
            p50 <= p95 && p95 <= p99 && p99 <= max,
            "{p50} {p95} {p99} {max}"
        );
        // Out-of-range quantiles clamp rather than panic or index out of bounds.
        assert!((m.quantile(-5.0).expect("clamped low") - 1.0).abs() < 1e-6);
        assert!((m.quantile(5.0).expect("clamped high") - 100.0).abs() < 1e-6);
    }

    #[test]
    fn a_mean_hides_the_tail_that_percentiles_expose() {
        // The reason this module exists: 299 good frames plus one 40 ms hitch is a ~16.7 ms mean
        // (indistinguishable from perfect) but a clearly bad max.
        let mut m = Metric::new();
        for _ in 0..(WINDOW - 1) {
            m.push(16.6);
        }
        m.push(40.0);
        let mean = m.mean().expect("mean");
        assert!(mean < 16.7, "mean {mean} should look almost perfect");
        assert!(
            (m.max().expect("max") - 40.0).abs() < 1e-6,
            "the hitch must still be visible"
        );
    }

    #[test]
    fn ring_evicts_oldest_and_ordered_is_chronological() {
        let mut m = Metric::new();
        // Overfill by half a window so wrap-around is exercised.
        let total = WINDOW + WINDOW / 2;
        for v in 0..u16::try_from(total).expect("window fits u16") {
            m.push(f32::from(v));
        }
        assert_eq!(m.len(), WINDOW, "must cap at the window size");
        assert_eq!(m.total(), total as u64, "total counts evicted samples too");
        let ordered = m.ordered();
        assert_eq!(ordered.len(), WINDOW);
        // Oldest surviving sample first, newest last.
        let oldest = f32::from(u16::try_from(total - WINDOW).expect("fits u16"));
        let newest = f32::from(u16::try_from(total - 1).expect("fits u16"));
        assert!((ordered[0] - oldest).abs() < 1e-6, "got {}", ordered[0]);
        assert!((ordered[WINDOW - 1] - newest).abs() < 1e-6);
        assert!((m.last().expect("last") - newest).abs() < 1e-6);
        // Strictly increasing, i.e. genuinely chronological rather than rotated.
        for w in ordered.windows(2) {
            assert!(
                w[1] > w[0],
                "ordered() is not chronological: {:?} -> {:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn a_partly_filled_window_is_not_padded_with_zeros() {
        // Reading the whole array would make p50 of three 20 ms frames come out as 0.
        let mut m = Metric::new();
        for _ in 0..3 {
            m.push(20.0);
        }
        assert_eq!(m.len(), 3);
        assert!((m.p50().expect("p50") - 20.0).abs() < 1e-6);
        assert!((m.min().expect("min") - 20.0).abs() < 1e-6);
        assert_eq!(m.ordered().len(), 3);
    }

    #[test]
    fn nan_samples_sort_deterministically_instead_of_panicking() {
        // A timer can produce NaN; `total_cmp` must keep the comparator total so percentiles do not
        // panic on the render thread.
        let mut m = Metric::new();
        m.push(1.0);
        m.push(f32::NAN);
        m.push(2.0);
        assert!(m.p50().is_some());
        assert!(m.max().is_some());
        assert_eq!(m.len(), 3);
    }

    /// NOTE: this asserts the *sink* honours a `None` produce sample. It passed while the caller
    /// in `app.rs` was still handing it `Some(stale)` on a zero-frame present — the invariant has
    /// two halves and only one of them is reachable from here. The caller-side half is documented
    /// on `Active::produce_sample_ms`, which exists precisely so the sticky `last_frame_time_ms`
    /// (wanted by the status bar and the run-ahead throttle) never reaches this function.
    #[test]
    fn idle_presents_do_not_pollute_the_produce_distribution() {
        let mut s = PerfStats::new();
        // Three real frames, then two idle presents (paused): produce_ms must hold only the real
        // three, while present_ms counts all five.
        for _ in 0..3 {
            s.record_present(Some(12.0), 2.0, Some(50.0), 1);
        }
        for _ in 0..2 {
            s.record_present(None, 1.0, None, 0);
        }
        assert_eq!(
            s.produce_ms.len(),
            3,
            "idle presents must not enter produce_ms"
        );
        assert_eq!(s.present_ms.len(), 5);
        assert_eq!(s.audio_pct.len(), 3);
        assert!((s.produce_ms.p50().expect("p50") - 12.0).abs() < 1e-6);
        assert_eq!(s.frames_produced, 3);
        assert_eq!(s.frames_presented, 5);
        assert!(!s.is_catching_up());
    }

    #[test]
    fn catch_up_bursts_are_visible_in_produced_vs_presented() {
        let mut s = PerfStats::new();
        // One present that emulated three frames — the divergence a single combined metric hides.
        s.record_present(Some(30.0), 3.0, Some(60.0), 3);
        assert_eq!(s.frames_produced, 3);
        assert_eq!(s.frames_presented, 1);
        assert!(s.is_catching_up());
        s.reset();
        assert_eq!(s.frames_produced, 0);
        assert!(s.produce_ms.is_empty() && s.present_ms.is_empty());
        assert!(!s.is_catching_up());
    }

    /// A non-finite quantile is rejected rather than clamped: `f32::clamp` propagates `NaN` and the
    /// float-to-int cast saturates it to 0, so the old code answered "the minimum sample" to a
    /// meaningless question.
    #[test]
    fn a_non_finite_quantile_is_none_not_the_minimum_sample() {
        let mut m = Metric::new();
        for v in [9.0, 1.0, 5.0] {
            m.push(v);
        }
        assert_eq!(m.quantile(0.0), Some(1.0));
        assert_eq!(m.quantile(f32::NAN), None);
        assert_eq!(m.quantile(f32::INFINITY), None);
        assert_eq!(m.quantile(f32::NEG_INFINITY), None);
    }

    #[test]
    fn gpu_metric_is_independent_of_the_cpu_ones() {
        let mut s = PerfStats::new();
        s.record_present(Some(10.0), 2.0, None, 1);
        assert!(s.gpu_ms.is_empty(), "no GPU sample unless one was resolved");
        s.record_gpu(1.5);
        assert_eq!(s.gpu_ms.len(), 1);
        assert!((s.gpu_ms.last().expect("last") - 1.5).abs() < 1e-6);
    }
}
