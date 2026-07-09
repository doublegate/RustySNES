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

use std::sync::atomic::{AtomicUsize, Ordering};

/// The S-DSP native output sample rate (stereo).
pub const SDSP_RATE: u32 = 32_000;

/// A simple lock-free single-producer/single-consumer ring of `f32` samples.
///
/// Samples are interleaved stereo. The producer (emulation thread, or the wasm main thread)
/// writes; the consumer (cpal callback) reads. Power-of-two capacity so the index wrap is a mask.
pub struct AudioRing {
    buf: Box<[f32]>,
    mask: usize,
    write: AtomicUsize,
    read: AtomicUsize,
}

impl AudioRing {
    /// Create a ring with capacity `2^pow2` samples (must be ≥ 8). Interleaved stereo, so the
    /// effective frame capacity is half the sample capacity.
    #[must_use]
    pub fn new(pow2: u32) -> Self {
        let cap = 1usize << pow2.max(3);
        Self {
            buf: vec![0.0; cap].into_boxed_slice(),
            mask: cap - 1,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
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

    /// Pop one sample, or `0.0` (silence) if the ring is empty (an underrun the DRC corrects).
    pub fn pop(&self) -> f32 {
        let r = self.read.load(Ordering::Relaxed);
        if (r & self.mask) == (self.write.load(Ordering::Acquire) & self.mask) {
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
    /// Fractional source position within the current `[last, cur]` interval (`0.0..1.0`).
    frac: f64,
    /// The previous source sample, the left endpoint of the interpolation interval.
    last: (f32, f32),
    /// Master volume in `0.0..=1.0`.
    volume: f32,
}

impl Resampler {
    /// Build a resampler from the S-DSP rate to `dst_rate` (the output device rate).
    #[must_use]
    pub fn new(dst_rate: u32, volume: f32) -> Self {
        let dst = f64::from(dst_rate.max(1));
        Self {
            base_step: f64::from(SDSP_RATE) / dst,
            frac: 0.0,
            last: (0.0, 0.0),
            volume,
        }
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
            while self.frac < 1.0 {
                #[allow(clippy::cast_possible_truncation)]
                let t = self.frac as f32;
                let left = (cur.0 - self.last.0).mul_add(t, self.last.0);
                let right = (cur.1 - self.last.1).mul_add(t, self.last.1);
                emit(left, right);
                self.frac += step;
            }
            self.frac -= 1.0;
            self.last = cur;
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
    fn process_and_process_into_agree() {
        // `process` (into an AudioRing) and `process_into` (into a Vec) share the same
        // interpolation core — they must emit byte-identical sample sequences for the same input.
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
