//! Native cpal audio output.
//!
//! The emulation thread (or the synchronous present path) fills an [`AudioRing`] via a
//! [`Resampler`]; the cpal callback drains it. The ring/resampler/DRC servo themselves are
//! console-agnostic and shared with the wasm `AudioWorklet` path — see [`crate::audio_core`] for
//! that shared core; this module is only the cpal device glue.

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};

pub use crate::audio_core::{AudioRing, Resampler, SDSP_RATE, drc_ratio};

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
    /// Build a `Send` producer half (`v1.1.0`, `emu-thread` feature parity) sharing this output's
    /// ring, with its own independent [`Resampler`] instance (resampler state — `frac`/`last` —
    /// must not be shared across producers; there is only ever one real producer at a time, but
    /// giving the thread its own instance avoids any doubt).
    #[must_use]
    pub fn make_producer(&self, volume: f32) -> AudioProducer {
        AudioProducer {
            ring: Arc::clone(&self.ring),
            resampler: Resampler::new(self.sample_rate, volume),
        }
    }
}

/// `v1.1.0` — the `Send` producer half of an [`AudioOutput`], for the emulation thread to own.
///
/// Bundles exactly what the synchronous present path already does inline (a [`Resampler`] +
/// the shared [`AudioRing`]) into one type the emu thread can push through once per produced
/// frame, closing the emu-thread build's "no audio output" gap — see `crate::emu_thread`'s module
/// doc. `AudioRing`'s only interior mutability is atomics, and `Resampler` is plain `f32`/`f64`
/// state, so this is `Send` with no unsafe of its own.
pub struct AudioProducer {
    ring: Arc<AudioRing>,
    resampler: Resampler,
}

impl AudioProducer {
    /// Update the master volume (from the Settings slider, re-synced each produced frame the same
    /// way the synchronous path already re-syncs it every present).
    pub const fn set_volume(&mut self, volume: f32) {
        self.resampler.set_volume(volume);
    }

    /// Resample `samples` (32 kHz `i16` stereo from `EmuCore::audio()`) into the ring, applying
    /// the same dynamic-rate-control + `speed`-multiplier math the synchronous present path uses
    /// (`app.rs`'s render loop) so alt-speed audio pitch-shifts identically either way.
    pub fn push(&mut self, samples: &[(i16, i16)], speed: f32) {
        if samples.is_empty() {
            return;
        }
        let cap = self.ring.capacity();
        let ratio = drc_ratio(self.ring.occupancy(), cap / 2, cap) * f64::from(speed);
        self.resampler.process(samples, ratio, &self.ring);
    }

    /// The audio ring's occupancy as a percentage of its capacity (the Performance panel's
    /// "audio health" gauge) — mirrors the synchronous path's own computation exactly.
    #[must_use]
    pub fn health_pct(&self) -> f32 {
        let cap = self.ring.capacity();
        if cap == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            (self.ring.occupancy() as f32 / cap as f32) * 100.0
        }
    }
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
                    // Drain the interleaved L/R ring into the device buffer; underrun -> silence.
                    // Always pop an L/R pair to keep the ring stereo-aligned, then fan out to the
                    // device's channel count (mono = average; >=2 = L,R with extras duplicated).
                    for frame in data.chunks_mut(channels.max(1)) {
                        let l = stream_ring.pop();
                        let r = stream_ring.pop();
                        if channels == 1 {
                            frame[0] = 0.5 * (l + r);
                        } else {
                            frame[0] = l;
                            frame[1] = r;
                            for ch in frame.iter_mut().skip(2) {
                                *ch = l;
                            }
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
