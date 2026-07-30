//! GPU-side pass timing via `wgpu` timestamp queries (`v1.25.0`, T-FP-B, `gpu-timing` feature).
//!
//! The Performance panel's `produce_ms`/`present_ms` are both **CPU** measurements. On a GPU-bound
//! machine they can look perfectly healthy while the display still stutters, because the cost is
//! entirely in passes the CPU only ever *submits*. This closes that blind spot by bracketing the
//! frame's command encoder with two GPU timestamps.
//!
//! # Why the readback is pipelined, not awaited
//!
//! A timestamp is only readable once the GPU has actually executed the commands, so reading it in
//! the frame that wrote it means blocking the render thread on the GPU — which would make the
//! measurement itself the largest thing being measured. Instead the result is picked up one or more
//! frames later ([`GpuTimer::poll`]), and the panel shows a reading that is a couple of frames old.
//! For a rolling distribution that is exactly as useful and costs nothing.
//!
//! # Honesty
//!
//! [`GpuTimer::new`] returns `None` when the device did not grant `TIMESTAMP_QUERY` — the common
//! case on older adapters and on WebGL. Nothing here fabricates a number in that case; the panel
//! says the capability is missing. See `crate::perf_panel`.

#![cfg(feature = "gpu-timing")]

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// Two queries: one before the frame's passes, one after.
const QUERY_COUNT: u32 = 2;
/// Bytes the resolve produces: `QUERY_COUNT` × `u64`.
const RESOLVE_BYTES: u64 = QUERY_COUNT as u64 * 8;

/// Readback-slot state, shared with the `map_async` callback.
///
/// An `AtomicU8` rather than a channel because the callback may run on a wgpu-internal thread and
/// the only thing that needs communicating is which of three states the slot is in.
mod slot {
    /// No query in flight; the buffer is free to record into.
    pub const IDLE: u8 = 0;
    /// Commands submitted, mapping requested, result not yet available.
    pub const PENDING: u8 = 1;
    /// The callback fired successfully; the buffer is mapped and readable.
    pub const READY: u8 = 2;
    /// The callback reported an error; the slot must be reset without reading.
    pub const FAILED: u8 = 3;
}

/// A two-timestamp GPU timer with a pipelined readback.
pub struct GpuTimer {
    query_set: wgpu::QuerySet,
    /// `resolve_query_set` destination (`QUERY_RESOLVE | COPY_SRC`) — cannot be map-readable.
    resolve: wgpu::Buffer,
    /// The map-readable copy of `resolve` (`MAP_READ | COPY_DST`).
    readback: wgpu::Buffer,
    /// Nanoseconds per timestamp tick, from `Queue::get_timestamp_period`.
    period_ns: f32,
    state: Arc<AtomicU8>,
}

impl GpuTimer {
    /// Build a timer, or `None` when the device lacks the required features.
    ///
    /// Both features are needed: `TIMESTAMP_QUERY` for the query set itself, and
    /// `TIMESTAMP_QUERY_INSIDE_ENCODERS` for `CommandEncoder::write_timestamp` (without it only
    /// render-pass begin/end timestamps are permitted, which would not bracket the whole frame).
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        let features = device.features();
        if !features.contains(wgpu::Features::TIMESTAMP_QUERY)
            || !features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            return None;
        }
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("rustysnes-gpu-timer"),
            ty: wgpu::QueryType::Timestamp,
            count: QUERY_COUNT,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rustysnes-gpu-timer-resolve"),
            size: RESOLVE_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rustysnes-gpu-timer-readback"),
            size: RESOLVE_BYTES,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Some(Self {
            query_set,
            resolve,
            readback,
            period_ns: queue.get_timestamp_period(),
            state: Arc::new(AtomicU8::new(slot::IDLE)),
        })
    }

    /// Whether a new frame can be recorded into the timer this present.
    ///
    /// `false` while a previous result is still in flight — recording anyway would overwrite the
    /// resolve buffer the pending map is reading, so a busy frame is simply not timed rather than
    /// timed wrongly. Missing occasional samples is fine for a distribution; a corrupted one is not.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.state.load(Ordering::Acquire) == slot::IDLE
    }

    /// Write the opening timestamp. Call before the frame's passes are encoded.
    pub fn begin(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.query_set, 0);
    }

    /// Write the closing timestamp and queue the resolve + readback copy.
    ///
    /// Call after the last pass is encoded and before `submit`; the caller must then call
    /// [`Self::after_submit`] so the map is requested against the submission that just happened.
    pub fn end(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.query_set, 1);
        encoder.resolve_query_set(&self.query_set, 0..QUERY_COUNT, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, RESOLVE_BYTES);
    }

    /// Request the readback map. Call immediately after `Queue::submit`.
    pub fn after_submit(&self) {
        self.state.store(slot::PENDING, Ordering::Release);
        let state = Arc::clone(&self.state);
        self.readback
            .map_async(wgpu::MapMode::Read, .., move |res| {
                // A failed map is recorded, not ignored: the slot must still be released or the timer
                // would wedge in PENDING and silently stop producing samples forever.
                state.store(
                    if res.is_ok() {
                        slot::READY
                    } else {
                        slot::FAILED
                    },
                    Ordering::Release,
                );
            });
    }

    /// Collect a finished measurement, in milliseconds, if one is ready.
    ///
    /// Non-blocking: polls the device once without waiting, so a result that is not ready yet is
    /// simply picked up on a later present.
    #[must_use]
    pub fn poll(&self, device: &wgpu::Device) -> Option<f32> {
        match self.state.load(Ordering::Acquire) {
            slot::PENDING => {
                // A single non-blocking check. Callbacks only fire from inside a poll, so without
                // this the map would never complete on a backend the surface present does not
                // implicitly poll.
                let _ = device.poll(wgpu::PollType::Poll);
                None
            }
            slot::READY => {
                let ms = self.read_mapped();
                self.readback.unmap();
                self.state.store(slot::IDLE, Ordering::Release);
                ms
            }
            slot::FAILED => {
                self.state.store(slot::IDLE, Ordering::Release);
                None
            }
            _ => None,
        }
    }

    /// Read the two mapped timestamps and convert their delta to milliseconds.
    fn read_mapped(&self) -> Option<f32> {
        let view = self.readback.slice(..).get_mapped_range();
        let bytes: &[u8] = &view;
        if u64::try_from(bytes.len()).is_ok_and(|len| len < RESOLVE_BYTES) {
            return None;
        }
        let start = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let end = u64::from_le_bytes(bytes[8..16].try_into().ok()?);
        drop(view);
        Some(ticks_to_ms(start, end, self.period_ns))
    }
}

/// Convert a timestamp-tick delta to milliseconds at `period_ns` nanoseconds per tick.
///
/// Returns `0.0` for a non-increasing pair rather than wrapping: a driver may report an unordered
/// or identical pair (a pass the GPU coalesced away, a reset counter), and a huge bogus reading
/// would poison every percentile in the window for the next ~5 seconds — which is precisely the
/// display the metric exists to make trustworthy.
#[must_use]
pub fn ticks_to_ms(start: u64, end: u64, period_ns: f32) -> f32 {
    let Some(delta) = end.checked_sub(start) else {
        return 0.0;
    };
    // Widen to f64 BEFORE multiplying. An f32 has 24 bits of mantissa, so a delta past ~16.7M
    // ticks — about 16.7 ms on a 1 ns/tick timer, i.e. exactly the frame times this metric exists
    // to measure — starts quantising in the cast itself, before any arithmetic. f64 carries the
    // whole u64 range that matters here; the narrowing back to f32 happens on a millisecond value
    // of order 10, where f32 has precision to spare.
    #[allow(clippy::cast_precision_loss)]
    let ms = delta as f64 * f64::from(period_ns) / 1.0e6;
    #[allow(clippy::cast_possible_truncation)]
    let ms = ms as f32;
    ms
}

#[cfg(test)]
mod tests {
    use super::ticks_to_ms;

    /// The conversion is ticks × period / 1e6. A 1 ms pass on a 1 ns/tick timer is 1,000,000 ticks.
    #[test]
    fn converts_ticks_to_milliseconds() {
        assert!((ticks_to_ms(0, 1_000_000, 1.0) - 1.0).abs() < 1e-4);
        assert!((ticks_to_ms(500, 1_000_500, 1.0) - 1.0).abs() < 1e-4);
        // A period other than 1 ns/tick scales linearly (NVIDIA reports ~1.0, AMD ~2.5-40).
        assert!((ticks_to_ms(0, 400_000, 2.5) - 1.0).abs() < 1e-4);
    }

    /// The tick delta is widened before it is multiplied. An `f32` mantissa is 24 bits, so past
    /// ~16.7M ticks — about 16.7 ms on a 1 ns/tick timer, i.e. squarely inside the range this
    /// metric reports — a `delta as f32` quantises in the cast itself. Each of these deltas is one
    /// tick above a power of two beyond 2^24; under the old cast the odd tick vanished and
    /// consecutive readings collapsed onto the same value.
    #[test]
    fn large_deltas_do_not_quantise_in_the_cast() {
        for exp in 25_u32..40 {
            let base = 1_u64 << exp;
            let a = ticks_to_ms(0, base, 1.0);
            let b = ticks_to_ms(0, base + (base >> 10), 1.0);
            assert!(
                b > a,
                "2^{exp} and 2^{exp}+2^{} must not read identically ({a} vs {b})",
                exp - 10
            );
        }
        // And the value itself stays right at a size where f32 alone would have drifted.
        let ms = ticks_to_ms(0, 33_333_333, 1.0);
        assert!((ms - 33.333_332).abs() < 0.001, "got {ms}");
    }

    /// A non-increasing pair reads as zero rather than as an enormous wrapped delta that would
    /// poison every percentile in the window.
    #[test]
    fn non_increasing_pairs_read_as_zero() {
        assert!((ticks_to_ms(1_000, 999, 1.0)).abs() < 1e-6);
        assert!((ticks_to_ms(42, 42, 1.0)).abs() < 1e-6);
        assert!((ticks_to_ms(u64::MAX, 0, 1.0)).abs() < 1e-6);
    }
}
