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
        if cap == 0 {
            return; // device not ready yet — matches health_pct's own guard.
        }
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

/// The names of every output device the default host currently exposes (`v1.25.0`).
///
/// Used by the Settings device picker. Enumeration can fail per-device on a host whose backend is
/// mid-reconfiguration, so a device whose name cannot be read is skipped rather than aborting the
/// whole list — a partial list is useful, an error is not.
#[must_use]
pub fn output_device_names() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices()
        .map(|devices| devices.map(|d| d.to_string()).collect())
        .unwrap_or_default()
}

/// Pick the output device matching `preferred` by name, falling back to the host default.
///
/// A remembered device legitimately disappears between sessions (a USB interface unplugged, a
/// Bluetooth headset off), so a name that no longer matches must fall back rather than leave the
/// emulator silent — the user would have no way to tell "device gone" from "audio broken".
fn pick_device(host: &cpal::Host, preferred: Option<&str>) -> Option<cpal::Device> {
    if let Some(want) = preferred
        && let Ok(mut devices) = host.output_devices()
        && let Some(found) = devices.find(|d| d.to_string() == want)
    {
        return Some(found);
    }
    host.default_output_device()
}

/// Build the output stream for one concrete sample format.
///
/// Generic over the device's sample type because a host may hand back `f32`, `i16`, or `u16`;
/// requesting `f32` unconditionally (as this did before `v1.25.0`) simply fails to open on a device
/// that offers only integer formats, which reads to the user as "this machine has no audio".
fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    ring: Arc<AudioRing>,
) -> Result<cpal::Stream, cpal::Error>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            // Drain the interleaved L/R ring into the device buffer; underrun -> silence.
            // Always pop an L/R pair to keep the ring stereo-aligned, then fan out to the device's
            // channel count (mono = average; >=2 = L,R with extras duplicated).
            for frame in data.chunks_mut(channels.max(1)) {
                let l = ring.pop();
                let r = ring.pop();
                if channels == 1 {
                    frame[0] = T::from_sample(0.5 * (l + r));
                } else {
                    frame[0] = T::from_sample(l);
                    frame[1] = T::from_sample(r);
                    for ch in frame.iter_mut().skip(2) {
                        *ch = T::from_sample(l);
                    }
                }
            }
        },
        |e| eprintln!("rustysnes audio stream error: {e}"),
        None,
    )
}

impl AudioOutput {
    /// Open the host default output device and start a stream draining `ring`.
    ///
    /// # Errors
    /// Returns [`AudioError`] when the host has no default output device, the config query fails,
    /// or the stream cannot be built/started.
    pub fn new(ring: Arc<AudioRing>) -> Result<Self, AudioError> {
        Self::with_device(ring, None)
    }

    /// As [`Self::new`], preferring the output device named `preferred` (`v1.25.0`).
    ///
    /// # Errors
    /// Returns [`AudioError`] when no usable device exists, the config query fails, the device's
    /// sample format is unsupported, or the stream cannot be built/started.
    pub fn with_device(ring: Arc<AudioRing>, preferred: Option<&str>) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = pick_device(&host, preferred).ok_or(AudioError::NoDevice)?;
        let supported = device
            .default_output_config()
            .map_err(|e| AudioError::Config(e.to_string()))?;
        // cpal 0.18: `SampleRate` is a `u32` alias; `sample_rate()` returns it directly.
        let sample_rate = supported.sample_rate();
        let channels = supported.channels() as usize;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let stream_ring = Arc::clone(&ring);

        let stream = match format {
            cpal::SampleFormat::F32 => build_stream::<f32>(&device, config, channels, stream_ring),
            cpal::SampleFormat::I16 => build_stream::<i16>(&device, config, channels, stream_ring),
            cpal::SampleFormat::U16 => build_stream::<u16>(&device, config, channels, stream_ring),
            other => {
                return Err(AudioError::Config(format!(
                    "unsupported device sample format {other:?}"
                )));
            }
        }
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
