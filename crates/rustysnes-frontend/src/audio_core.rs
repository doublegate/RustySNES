//! Console-agnostic audio building blocks, shared between the native cpal output path
//! (`audio.rs`) and the wasm `AudioWorklet` output path (`wasm_audio.rs`).
//!
//! The S-DSP source rate, the lock-free SPSC ring, the producer-side resampler, and the
//! dynamic-rate-control (DRC) servo all live here. This is the RustyNES audio path,
//! SNES-adapted: the S-DSP's native output is **32 kHz**
//! stereo, resampled by [`Resampler`] (producer-side linear interpolation) to the output device's
//! rate. The ring + DRC + resampler are console-agnostic; only the source rate + channel count
//! differ from RustyNES's NES equivalent.
//!
//! The DRC servo + resampler live in the FRONTEND (never the core's synthesis) — that is what
//! keeps the determinism contract intact (the core emits the same samples regardless of how the
//! frontend paces playback).

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// The S-DSP native output sample rate (stereo).
pub const SDSP_RATE: u32 = 32_000;

/// A simple lock-free single-producer/single-consumer ring of `f32` samples.
///
/// Samples are interleaved stereo. The producer (emulation thread, or the wasm main thread)
/// writes; the consumer (cpal callback) reads. Power-of-two capacity so the index wrap is a mask.
///
/// # Health instrumentation and the refill gate
///
/// Occupancy alone cannot distinguish "the ring is briefly low" from "the consumer has been
/// starved repeatedly", so the ring also counts [`Self::underruns`] (a pop with nothing queued)
/// and [`Self::overrun_dropped`] (a push onto a full ring). Both are monotonic counters a
/// diagnostics panel can sample; neither affects the audio path.
///
/// The **refill gate** exists because an underrun is not a one-off click: once the consumer
/// catches up to the producer it tends to stay caught up, emitting a silence sample every
/// callback and turning one dropout into continuous crackle. When the gate is armed
/// ([`Self::set_start_threshold`] with a non-zero value) the consumer feeds silence until that
/// many samples are queued, then plays; an underrun **re-arms** it, so the ring re-buffers once
/// instead of tearing repeatedly. The threshold defaults to `0` (gate disabled), which is exactly
/// the ungated behaviour every prior release had — callers opt in.
pub struct AudioRing {
    buf: Box<[f32]>,
    mask: usize,
    write: AtomicUsize,
    read: AtomicUsize,
    /// Pops that found the ring empty (consumer starved).
    underruns: AtomicU64,
    /// Pushes dropped because the ring was full (producer ahead of the device).
    overrun_dropped: AtomicU64,
    /// Whether playback is currently released (see the refill gate in the type docs).
    started: AtomicBool,
    /// Samples that must be queued before playback (re)starts; `0` disables the gate.
    start_threshold: AtomicUsize,
    /// Consumer-side hard mute (pause), independent of the refill gate.
    muted: AtomicBool,
}

impl AudioRing {
    /// Create a ring with capacity `2^pow2` samples (must be ≥ 8). Interleaved stereo, so the
    /// effective frame capacity is half the sample capacity.
    ///
    /// The refill gate starts **disabled** (threshold `0`); see [`Self::set_start_threshold`].
    #[must_use]
    pub fn new(pow2: u32) -> Self {
        let cap = 1usize << pow2.max(3);
        Self {
            buf: vec![0.0; cap].into_boxed_slice(),
            mask: cap - 1,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            underruns: AtomicU64::new(0),
            overrun_dropped: AtomicU64::new(0),
            started: AtomicBool::new(false),
            start_threshold: AtomicUsize::new(0),
            muted: AtomicBool::new(false),
        }
    }

    /// Arm the refill gate at `samples` queued (`0` disables it entirely).
    ///
    /// Setting a threshold also re-arms the gate, so the next pops feed silence until the ring has
    /// buffered that much — the correct behaviour when the latency target changes mid-session.
    pub fn set_start_threshold(&self, samples: usize) {
        self.start_threshold.store(samples, Ordering::Relaxed);
        self.started.store(false, Ordering::Release);
    }

    /// Consumer-side hard mute: pops return silence and do **not** count as underruns.
    ///
    /// This is the pause gate. Without it, a paused emulator produces nothing and every device
    /// callback would tally an underrun, burying the real starvation signal in noise.
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Release);
        if muted {
            // Re-arm so un-pausing refills rather than resuming into a nearly-empty ring.
            self.started.store(false, Ordering::Release);
        }
    }

    /// Pops that found the ring empty since the last [`Self::reset_health`].
    #[must_use]
    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Samples the producer had to drop because the ring was full, since the last
    /// [`Self::reset_health`].
    #[must_use]
    pub fn overrun_dropped(&self) -> u64 {
        self.overrun_dropped.load(Ordering::Relaxed)
    }

    /// Whether playback is currently released (the refill gate is satisfied).
    #[must_use]
    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }

    /// Zero both health counters (a diagnostics panel's "reset" button).
    pub fn reset_health(&self) {
        self.underruns.store(0, Ordering::Relaxed);
        self.overrun_dropped.store(0, Ordering::Relaxed);
    }

    /// The ring's total sample capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// The number of samples currently queued (producer-side estimate).
    #[must_use]
    pub fn occupancy(&self) -> usize {
        self.write
            .load(Ordering::Acquire)
            .wrapping_sub(self.read.load(Ordering::Acquire))
            & self.mask
    }

    /// Push one sample; drops it if the ring is full (a full ring means the consumer is behind —
    /// the DRC servo will correct the ratio). Returns whether it was stored.
    pub fn push(&self, sample: f32) -> bool {
        let w = self.write.load(Ordering::Relaxed);
        let next = (w + 1) & self.mask;
        if next == (self.read.load(Ordering::Acquire) & self.mask) {
            self.overrun_dropped.fetch_add(1, Ordering::Relaxed);
            return false; // full
        }
        // SAFETY: single producer; `w` is the only index we write, and `next != read` proves the
        // slot is free. The `&self.buf` aliasing is sound because the consumer only reads slots
        // behind `read`, which never overlaps `w`.
        unsafe {
            let slot = self.buf.as_ptr().add(w & self.mask).cast_mut();
            slot.write(sample);
        }
        self.write.store(next, Ordering::Release);
        true
    }

    /// Pop one sample, or `0.0` (silence) if muted, still refilling, or empty.
    ///
    /// An empty ring tallies an underrun and **re-arms the refill gate** (see the type docs): the
    /// consumer then feeds silence until the producer has rebuilt the buffer, which converts a
    /// continuous crackle into a single short gap.
    pub fn pop(&self) -> f32 {
        if self.muted.load(Ordering::Acquire) {
            return 0.0; // paused: silence, and deliberately not an underrun
        }
        let threshold = self.start_threshold.load(Ordering::Relaxed);
        if threshold > 0 && !self.started.load(Ordering::Acquire) {
            if self.occupancy() < threshold {
                return 0.0; // still refilling
            }
            self.started.store(true, Ordering::Release);
        }
        let r = self.read.load(Ordering::Relaxed);
        if (r & self.mask) == (self.write.load(Ordering::Acquire) & self.mask) {
            self.underruns.fetch_add(1, Ordering::Relaxed);
            if threshold > 0 {
                self.started.store(false, Ordering::Release);
            }
            return 0.0; // empty -> silence
        }
        let sample = self.buf[r & self.mask];
        self.read.store((r + 1) & self.mask, Ordering::Release);
        sample
    }
}

/// A producer-side linear resampler from the S-DSP's 32 kHz `i16` stereo stream to the output
/// device rate.
///
/// [`Self::process`] pushes interleaved `f32` L/R into an [`AudioRing`] (the native cpal path);
/// [`Self::process_into`] appends to a plain `Vec<f32>` instead (the wasm `AudioWorklet` path,
/// which crosses a `postMessage` boundary rather than sharing memory with its consumer). Both
/// share the same interpolation core. The dynamic-rate-control ratio nudges the step so the
/// consumer stays near its target occupancy (absorbing pacing jitter without changing the
/// deterministic source samples — the `docs/frontend.md` determinism boundary).
pub struct Resampler {
    /// Source advance per output sample, before the DRC nudge (`src_rate / dst_rate`).
    base_step: f64,
    /// Fractional source position within the current interpolation interval (`0.0..1.0`).
    frac: f64,
    /// Four-tap source history `[p0, p1, p2, p3]`. Output is interpolated across `[p1, p2]`, so
    /// the kernel can see one sample either side — which is what a cubic needs and a linear blend
    /// ignores. Holding a window costs exactly one source sample of delay (~31 µs at 32 kHz).
    hist: [(f32, f32); 4],
    /// Master volume in `0.0..=1.0`.
    volume: f32,
    /// Which interpolation kernel [`Self::resample`] applies.
    kernel: ResampleKernel,
    /// The graphic-equaliser stage (`v1.25.0`), applied to the interpolated output.
    ///
    /// Lives here rather than in the caller because this is where source samples become `f32` — an
    /// EQ applied to the 32 kHz `i16` input instead would be filtering at the wrong rate, and its
    /// band centres would land in the wrong place.
    eq: crate::eq::Equalizer,
}

/// The interpolation kernel a [`Resampler`] applies between source samples.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResampleKernel {
    /// Two-point linear blend — cheapest, and what every release before the RustyNES-parity pass
    /// used. Kept selectable so that older output can be reproduced exactly.
    Linear,
    /// Four-tap Catmull-Rom cubic (a Hermite spline with tangents from the neighbours), matching
    /// RustyNES's own resampler. Continuous in the first derivative across sample boundaries,
    /// which is audibly less harsh than a linear blend on the S-DSP's 32 kHz output — the
    /// aliasing a 2-point blend leaves behind is exactly what this removes. The default.
    #[default]
    Hermite,
}

impl ResampleKernel {
    /// Human-readable label for the Settings radio row.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Linear => "Linear (2-tap)",
            Self::Hermite => "Hermite (4-tap)",
        }
    }

    /// All kernels in display order — the single source of truth the Settings row iterates.
    #[must_use]
    pub const fn all() -> [Self; 2] {
        [Self::Linear, Self::Hermite]
    }
}

/// One axis of a Catmull-Rom cubic through `p1`/`p2`, with `p0`/`p3` supplying the tangents.
///
/// Standard uniform Catmull-Rom basis; `t` is the position within `[p1, p2]`.
fn catmull_rom(prev: f32, from: f32, to: f32, next: f32, t: f32) -> f32 {
    // Coefficients of the cubic in `t`, lowest order first.
    let c0 = 2.0 * from;
    let c1 = to - prev;
    let c2 = 2.0f32.mul_add(prev, -(5.0 * from)) + 4.0f32.mul_add(to, -next);
    let c3 = 3.0f32.mul_add(from - to, next - prev);
    0.5 * (c3.mul_add(t, c2).mul_add(t, c1).mul_add(t, c0))
}

impl Resampler {
    /// Build a resampler from the S-DSP rate to `dst_rate` (the output device rate), using the
    /// default ([`ResampleKernel::Hermite`]) kernel.
    #[must_use]
    pub fn new(dst_rate: u32, volume: f32) -> Self {
        Self::with_kernel(dst_rate, volume, ResampleKernel::default())
    }

    /// As [`Self::new`], choosing the interpolation kernel explicitly.
    #[must_use]
    pub fn with_kernel(dst_rate: u32, volume: f32, kernel: ResampleKernel) -> Self {
        let dst = f64::from(dst_rate.max(1));
        Self {
            base_step: f64::from(SDSP_RATE) / dst,
            frac: 0.0,
            hist: [(0.0, 0.0); 4],
            volume,
            kernel,
            eq: crate::eq::Equalizer::new(dst_rate, [0.0; crate::eq::BANDS], false),
        }
    }

    /// Update the equaliser (from the Settings sliders). Filter state is preserved, so moving a
    /// slider does not click.
    pub fn set_eq(&mut self, enabled: bool, gains_db: [f32; crate::eq::BANDS]) {
        self.eq.set_enabled(enabled);
        self.eq.set_gains(gains_db);
    }

    /// Whether the equaliser is currently altering samples.
    #[must_use]
    pub const fn eq_active(&self) -> bool {
        self.eq.is_active()
    }

    /// Swap the interpolation kernel (from the Settings row) without discarding ring state.
    pub const fn set_kernel(&mut self, kernel: ResampleKernel) {
        self.kernel = kernel;
    }

    /// Update the master volume (from the Settings slider).
    pub const fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Resample `input` (32 kHz `i16` stereo) into `ring` at the device rate, applying `drc` (a
    /// ratio near 1.0 from [`drc_ratio`]). One push per channel sample (interleaved L, R).
    // The `frac` while-loop emits one output sample per crossing of a source-sample interval — a
    // float accumulator is the natural form for a fractional resampler; `left`/`right` are the
    // intentionally-parallel stereo pair.
    #[allow(clippy::while_float, clippy::similar_names)]
    pub fn process(&mut self, input: &[(i16, i16)], drc: f64, ring: &AudioRing) {
        self.resample(input, drc, |l, r| {
            ring.push(l);
            ring.push(r);
        });
    }

    /// As [`Self::process`], but appends interleaved L, R samples to `out` instead of an
    /// [`AudioRing`] — the wasm `AudioWorklet` path, which hands samples across a `postMessage`
    /// boundary rather than sharing memory with the consumer.
    #[allow(clippy::while_float, clippy::similar_names)]
    pub fn process_into(&mut self, input: &[(i16, i16)], drc: f64, out: &mut Vec<f32>) {
        self.resample(input, drc, |l, r| {
            out.push(l);
            out.push(r);
        });
    }

    /// The shared interpolation core: emits interleaved L, R pairs to `emit` for every source
    /// sample the DRC-adjusted step crosses.
    #[allow(clippy::while_float, clippy::similar_names)]
    fn resample(&mut self, input: &[(i16, i16)], drc: f64, mut emit: impl FnMut(f32, f32)) {
        let step = (self.base_step * drc).max(1e-6);
        let vol = self.volume;
        for &(l, r) in input {
            let cur = (f32::from(l) / 32768.0 * vol, f32::from(r) / 32768.0 * vol);
            // Slide the 4-tap window: the newest sample becomes `p3`, and interpolation runs
            // across `[p1, p2]` so both neighbours a cubic needs are already in hand.
            self.hist[0] = self.hist[1];
            self.hist[1] = self.hist[2];
            self.hist[2] = self.hist[3];
            self.hist[3] = cur;
            let [p0, p1, p2, p3] = self.hist;
            while self.frac < 1.0 {
                #[allow(clippy::cast_possible_truncation)]
                let t = self.frac as f32;
                let (left, right) = match self.kernel {
                    ResampleKernel::Linear => (
                        (p2.0 - p1.0).mul_add(t, p1.0),
                        (p2.1 - p1.1).mul_add(t, p1.1),
                    ),
                    ResampleKernel::Hermite => (
                        catmull_rom(p0.0, p1.0, p2.0, p3.0, t),
                        catmull_rom(p0.1, p1.1, p2.1, p3.1, t),
                    ),
                };
                let (left, right) = self.eq.process(left, right);
                emit(left, right);
                self.frac += step;
            }
            self.frac -= 1.0;
        }
    }
}

/// The dynamic-rate-control servo: nudge the resample ratio toward a target ring occupancy.
///
/// Given the current ring occupancy vs. a target, return a small resample-ratio adjustment (a
/// fraction near 1.0) that nudges occupancy toward the target. A classic proportional controller,
/// clamped to avoid audible pitch wobble.
#[must_use]
pub fn drc_ratio(occupancy: usize, target: usize, capacity: usize) -> f64 {
    if capacity == 0 {
        return 1.0;
    }
    // Error normalized to [-1, 1] over the half-capacity around the target.
    // The cast precision loss is irrelevant: occupancy/target/capacity are small ring indices
    // (far below f64's 2^52 mantissa limit), and this is a coarse audio-pacing servo ratio.
    #[allow(clippy::cast_precision_loss)]
    let err = (occupancy as f64 - target as f64) / (capacity as f64 / 2.0);
    // Gentle proportional gain; clamp to ±0.5% so the pitch shift is inaudible.
    let adjust = (err * 0.005).clamp(-0.005, 0.005);
    1.0 + adjust
}

/// Interleaved-stereo sample count representing `latency_ms` of audio at `rate`.
///
/// The unit the ring counts is a single `f32` channel sample, so a stereo frame is two of them —
/// forgetting the factor of two is a 2× latency error, which is why this is a named helper rather
/// than an open-coded multiply at each call site.
#[must_use]
pub fn latency_samples(rate: u32, latency_ms: u32) -> usize {
    (usize::try_from(rate).unwrap_or(48_000) * usize::try_from(latency_ms).unwrap_or(0) / 1000) * 2
}

/// The dynamic-rate-control servo, targeting an explicit **latency setpoint** rather than the
/// ring's midpoint.
///
/// [`drc_ratio`] servos to half-capacity, which ties the achieved latency to whatever the buffer
/// happens to be sized at. Servoing to a setpoint instead makes latency the thing the user
/// configures and the buffer merely the headroom around it, which is what
/// [`crate::config::AudioConfig::latency_ms`] exposes. The error is normalised by the target (so
/// the response is proportional to how far off the setpoint is, in units of the setpoint) and the
/// correction is clamped to ±1% — wide enough to hold a setpoint against pacing jitter, far below
/// the ~2% where pitch drift becomes audible.
#[must_use]
pub fn drc_ratio_latency(occupancy: usize, target: usize, capacity: usize) -> f64 {
    if capacity == 0 || target == 0 {
        return 1.0;
    }
    // Ring indices are far below f64's 2^52 exact-integer limit; this is a coarse pacing ratio.
    #[allow(clippy::cast_precision_loss)]
    let err = (occupancy as f64 - target as f64) / target as f64;
    let adjust = (err * 0.01).clamp(-0.01, 0.01);
    1.0 + adjust
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_push_pop_roundtrip() {
        let ring = AudioRing::new(4); // 16 samples
        assert!(ring.push(0.5));
        assert!(ring.push(-0.25));
        assert!((ring.pop() - 0.5).abs() < 1e-6);
        assert!((ring.pop() - (-0.25)).abs() < 1e-6);
        // Empty -> silence. Exact-sentinel compare: `pop` returns a literal `0.0` on underrun.
        #[allow(clippy::float_cmp)]
        let silent = ring.pop() == 0.0;
        assert!(silent);
    }

    #[test]
    fn ring_reports_full() {
        let ring = AudioRing::new(3); // 8 samples, 7 usable before wrap collision
        let mut stored = 0;
        for _ in 0..16 {
            if ring.push(1.0) {
                stored += 1;
            }
        }
        assert!(stored <= ring.capacity());
        assert!(stored >= 1);
    }

    #[test]
    fn drc_nudges_toward_target() {
        let cap = 4096;
        let target = cap / 2;
        // Over-full -> ratio > 1 (consume faster).
        assert!(drc_ratio(target + 1000, target, cap) > 1.0);
        // Under-full -> ratio < 1 (consume slower).
        assert!(drc_ratio(target - 1000, target, cap) < 1.0);
        // At target -> ~1.0.
        assert!((drc_ratio(target, target, cap) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn latency_setpoint_servo_targets_the_setpoint_not_the_midpoint() {
        let cap = 8192;
        let target = 1536; // e.g. 16ms @ 48kHz stereo — far from cap/2
        assert!(
            drc_ratio_latency(target * 2, target, cap) > 1.0,
            "above setpoint: consume faster"
        );
        assert!(
            drc_ratio_latency(target / 2, target, cap) < 1.0,
            "below setpoint: consume slower"
        );
        assert!((drc_ratio_latency(target, target, cap) - 1.0).abs() < 1e-9);
        // The correction is bounded at 1% however far off the setpoint is.
        assert!(drc_ratio_latency(cap, target, cap) <= 1.01 + 1e-9);
        assert!(drc_ratio_latency(0, target, cap) >= 0.99 - 1e-9);
        // Degenerate inputs are inert rather than dividing by zero.
        assert!((drc_ratio_latency(100, 0, cap) - 1.0).abs() < 1e-9);
        assert!((drc_ratio_latency(100, target, 0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn latency_samples_counts_both_channels() {
        // 16 ms at 48 kHz is 768 frames, and the ring counts channel samples, so 1536.
        assert_eq!(latency_samples(48_000, 16), 1536);
        assert_eq!(latency_samples(48_000, 0), 0);
    }

    #[test]
    fn ring_counts_underruns_and_overruns() {
        let ring = AudioRing::new(3); // 8 samples; 7 usable
        for _ in 0..32 {
            ring.push(1.0);
        }
        assert!(
            ring.overrun_dropped() > 0,
            "a full ring must tally dropped pushes"
        );
        while ring.occupancy() > 0 {
            ring.pop();
        }
        let before = ring.underruns();
        ring.pop();
        assert_eq!(
            ring.underruns(),
            before + 1,
            "an empty pop must tally an underrun"
        );
        ring.reset_health();
        assert_eq!(ring.underruns(), 0);
        assert_eq!(ring.overrun_dropped(), 0);
    }

    #[test]
    fn refill_gate_is_off_by_default_and_holds_silence_until_threshold_when_armed() {
        // Default: ungated — one pushed sample is immediately playable (prior-release behaviour).
        let ring = AudioRing::new(6);
        ring.push(0.75);
        assert!((ring.pop() - 0.75).abs() < 1e-6);

        // Armed: silence until the threshold is met, then real samples.
        let gated = AudioRing::new(6);
        gated.set_start_threshold(8);
        for _ in 0..4 {
            gated.push(0.5);
        }
        #[allow(clippy::float_cmp)]
        let still_silent = gated.pop() == 0.0;
        assert!(
            still_silent,
            "below threshold the consumer must feed silence"
        );
        assert!(!gated.is_started());
        for _ in 0..8 {
            gated.push(0.5);
        }
        assert!((gated.pop() - 0.5).abs() < 1e-6, "threshold met: play");
        assert!(gated.is_started());
    }

    #[test]
    fn underrun_rearms_the_gate_and_mute_is_not_an_underrun() {
        let ring = AudioRing::new(4);
        ring.set_start_threshold(4);
        for _ in 0..6 {
            ring.push(0.25);
        }
        while ring.occupancy() > 0 {
            ring.pop();
        }
        ring.pop(); // underrun
        assert!(
            !ring.is_started(),
            "an underrun must re-arm the refill gate"
        );

        // A muted ring returns silence without tallying underruns — the pause gate.
        let quiet = AudioRing::new(4);
        quiet.push(1.0);
        quiet.set_muted(true);
        let before = quiet.underruns();
        #[allow(clippy::float_cmp)]
        let muted_silent = quiet.pop() == 0.0;
        assert!(muted_silent);
        assert_eq!(
            quiet.underruns(),
            before,
            "pause must not look like starvation"
        );
    }

    #[test]
    fn hermite_is_smooth_and_linear_stays_reproducible() {
        // A ramp is exactly representable by both kernels, so both must track it closely; this
        // pins that the 4-tap window is wired up (indices not transposed) rather than that cubic
        // and linear differ.
        let input: Vec<(i16, i16)> = (0..64).map(|i| (i * 400, i * 400)).collect();
        for kernel in ResampleKernel::all() {
            let mut r = Resampler::with_kernel(48_000, 1.0, kernel);
            let mut out = Vec::new();
            r.process_into(&input, 1.0, &mut out);
            assert!(!out.is_empty(), "{kernel:?} produced no output");
            // Monotone non-decreasing input must not make the output swing wildly out of range.
            for w in out.windows(2) {
                assert!(
                    (w[1] - w[0]).abs() < 0.2,
                    "{kernel:?} produced a discontinuity: {:?} -> {:?}",
                    w[0],
                    w[1]
                );
            }
        }
    }

    #[test]
    fn process_and_process_into_agree() {
        // `process` (into an AudioRing) and `process_into` (into a Vec) share the same
        // interpolation core — they must emit matching sample sequences (within float epsilon,
        // asserted below) for the same input, not merely similar ones.
        let input: Vec<(i16, i16)> = (0..64)
            .map(|i| (i * 100 - 3000, -(i * 50) + 1500))
            .collect();

        let ring = AudioRing::new(14); // plenty of headroom
        let mut r1 = Resampler::new(48_000, 1.0);
        r1.process(&input, 1.0, &ring);
        let mut from_ring = Vec::new();
        loop {
            let before = ring.occupancy();
            if before == 0 {
                break;
            }
            from_ring.push(ring.pop());
        }

        let mut r2 = Resampler::new(48_000, 1.0);
        let mut from_vec = Vec::new();
        r2.process_into(&input, 1.0, &mut from_vec);

        assert_eq!(from_ring.len(), from_vec.len());
        for (a, b) in from_ring.iter().zip(from_vec.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }
}
