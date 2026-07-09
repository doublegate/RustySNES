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
