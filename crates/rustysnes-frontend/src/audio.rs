//! Audio output: a lock-free SPSC ring plus a dynamic-rate-control (DRC) servo.
//!
//! The emulation thread fills the ring and the cpal callback drains it; the DRC servo nudges the
//! resample ratio toward a target ring occupancy so audio neither starves nor overflows.
//!
//! This is the RustyNES audio path, SNES-adapted: the S-DSP's native output is **32 kHz**
//! stereo, resampled (in a future `resampler` stage) to the cpal device rate (commonly 48 kHz).
//! The ring + DRC are console-agnostic; only the source rate + channel count differ.
//!
//! The DRC servo + resampler live in the FRONTEND (never the core's synthesis) — that is what
//! keeps the determinism contract intact (the core emits the same samples regardless of how the
//! frontend paces playback).
//!
//! v0.1.0: the cpal stream is wired and plays silence (the APU is a skeleton). The ring + DRC
//! math are real and tested.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};

/// The S-DSP native output sample rate (stereo).
pub const SDSP_RATE: u32 = 32_000;

/// A simple lock-free single-producer/single-consumer ring of `f32` samples.
///
/// Samples are interleaved stereo. The producer (emulation thread) writes; the consumer (cpal
/// callback) reads. Power-of-two capacity so the index wrap is a mask.
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

/// The live cpal output stream + its ring (kept alive for the program's duration).
pub struct AudioOutput {
    /// The shared ring the emulation thread fills.
    pub ring: Arc<AudioRing>,
    /// The device output sample rate (the resample target).
    pub sample_rate: u32,
    // The stream must outlive its callback; keep it owned here. `Mutex` only to make
    // `AudioOutput` `Send` for the app struct — the stream itself is never re-locked.
    _stream: Mutex<cpal::Stream>,
}

impl AudioOutput {
    /// Open the default output device and start a stereo f32 stream draining `ring`. Returns an
    /// [`AudioError`] if no device/config is available.
    ///
    /// # Errors
    /// Returns [`AudioError`] when the host has no default output device, the config query fails,
    /// or the stream cannot be built/started.
    pub fn new(ring: Arc<AudioRing>) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
        let supported = device
            .default_output_config()
            .map_err(|e| AudioError::Config(e.to_string()))?;
        // cpal 0.18: `SampleRate` is a `u32` alias; `sample_rate()` returns it directly.
        let sample_rate = supported.sample_rate();
        let channels = supported.channels() as usize;
        let config: cpal::StreamConfig = supported.into();
        let stream_ring = Arc::clone(&ring);

        let err_fn = |e| eprintln!("rustysnes audio stream error: {e}");
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| {
                    // Drain the ring into the device buffer; underrun -> silence (ring.pop).
                    for frame in data.chunks_mut(channels.max(1)) {
                        let s = stream_ring.pop();
                        for ch in frame.iter_mut() {
                            *ch = s;
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::Build(e.to_string()))?;
        stream
            .play()
            .map_err(|e| AudioError::Build(e.to_string()))?;

        Ok(Self {
            ring,
            sample_rate,
            _stream: Mutex::new(stream),
        })
    }
}

/// Audio initialization failures.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// No default output device.
    #[error("no default audio output device")]
    NoDevice,
    /// Device config query failed.
    #[error("audio device config error: {0}")]
    Config(String),
    /// Stream build/start failed.
    #[error("audio stream build error: {0}")]
    Build(String),
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
}
