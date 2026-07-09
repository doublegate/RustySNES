//! wasm audio output: `AudioWorkletNode` primary, `ScriptProcessorNode` fallback.
//!
//! `AudioWorkletNode` processing runs in a separate realm (`AudioWorkletGlobalScope`) that plain
//! wasm-bindgen code cannot call into directly, so the ring-buffer drain logic
//! ([`WORKLET_JS`]) is a small hand-written JS class, loaded via a `Blob:` object URL (avoids
//! shipping a second JS asset file for trunk to bundle) — the same shape RustyNES's own
//! `wasm_audio.rs` uses. No `SharedArrayBuffer` (GitHub Pages can't send COOP/COEP headers), so
//! resampled samples cross the realm boundary via `port.postMessage`, and the worklet reports its
//! ring occupancy back the same way every 2048 output frames for the DRC servo.
//!
//! `AudioWorklet::add_module` is asynchronous, so [`ensure_audio`] returns the device sample rate
//! immediately (needed synchronously to build the [`Resampler`]) and continues wiring the graph
//! in the background; [`push_samples`] silently drops samples pushed before the graph attaches
//! (a few frames' worth at most). If the worklet module fails to load or attach (older browser,
//! CSP restriction), [`ensure_audio`] falls back to a `ScriptProcessorNode`, whose
//! `onaudioprocess` fires as an ordinary main-thread callback and drains a plain [`AudioRing`]
//! directly — deprecated but fully functional in every browser RustySNES targets.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::{Array, Float32Array};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioContext, AudioProcessingEvent, AudioWorkletNode, Blob, BlobPropertyBag, MessageEvent,
    ScriptProcessorNode, Url,
};

use crate::audio_core::{AudioRing, Resampler, drc_ratio};

/// The `AudioWorkletProcessor` class, loaded via a `Blob:` URL. Maintains its own ring (sized to
/// ~0.5s of audio at the context's sample rate) fed by `port.onmessage`, drained one output frame
/// at a time by `process()`; reports `[occupancy, capacity]` back every 2048 output frames.
const WORKLET_JS: &str = r"
class RustySnesAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.cap = Math.max(8192, Math.floor(sampleRate * 0.5));
    this.ring = new Float32Array(this.cap);
    this.write = 0;
    this.read = 0;
    this.count = 0;
    this.reportCounter = 0;
    this.port.onmessage = (event) => {
      const data = event.data;
      for (let i = 0; i < data.length; i++) {
        if (this.count < this.cap) {
          this.ring[this.write] = data[i];
          this.write = (this.write + 1) % this.cap;
          this.count++;
        }
      }
    };
  }
  process(inputs, outputs) {
    const out = outputs[0];
    const left = out[0];
    const right = out.length > 1 ? out[1] : out[0];
    for (let i = 0; i < left.length; i++) {
      if (this.count >= 2) {
        left[i] = this.ring[this.read];
        this.read = (this.read + 1) % this.cap;
        this.count--;
        right[i] = this.ring[this.read];
        this.read = (this.read + 1) % this.cap;
        this.count--;
      } else {
        left[i] = 0;
        right[i] = 0;
      }
    }
    this.reportCounter += left.length;
    if (this.reportCounter >= 2048) {
      this.reportCounter = 0;
      this.port.postMessage([this.count, this.cap]);
    }
    return true;
  }
}
registerProcessor('rustysnes-audio', RustySnesAudioProcessor);
";

/// The registered `AudioWorkletProcessor` name (`registerProcessor`'s first argument in
/// [`WORKLET_JS`], and the second argument to `AudioWorkletNode::new`).
const WORKLET_NAME: &str = "rustysnes-audio";
/// `ScriptProcessorNode` buffer size in frames (a browser-mandated power-of-two, 256..=16384) —
/// only used by the fallback path.
const SCRIPT_PROCESSOR_BUFFER_FRAMES: u32 = 2048;
/// The `ScriptProcessorNode` fallback's staging ring capacity (`2^15` samples, ~0.5s stereo at a
/// 32 kHz source rate) — generous headroom since `onaudioprocess` drains only ~2048 frames at a
/// time.
const SCRIPT_PROCESSOR_RING_POW2: u32 = 15;

thread_local! {
    static STATE: RefCell<Option<AudioState>> = const { RefCell::new(None) };
}

/// Which output sink [`push_samples`] currently feeds.
enum Sink {
    /// Waiting on `AudioWorklet::add_module`/node construction to finish (or fail) in the
    /// background; samples pushed in this state are silently dropped.
    Pending,
    /// The primary path: resampled f32 pairs cross via `port.postMessage`; `occupancy` is
    /// refreshed by the worklet's periodic `[occupancy, capacity]` report.
    Worklet {
        node: AudioWorkletNode,
        occupancy: Rc<Cell<(usize, usize)>>,
    },
    /// The fallback path: `onaudioprocess` drains `ring` directly on the main thread.
    ScriptProcessor {
        _node: ScriptProcessorNode,
        ring: Rc<AudioRing>,
    },
}

/// The live audio graph + the producer-side resampler [`push_samples`] feeds it with.
struct AudioState {
    ctx: AudioContext,
    sink: Sink,
    resampler: Resampler,
}

/// Ensure the audio graph is built and running, returning the device sample rate.
///
/// **Must be called synchronously from within a user-gesture event handler** (a click or a file
/// pick) — browser autoplay policy blocks `AudioContext` playback started any other way. The
/// worklet module load is asynchronous and continues after this returns (see the module docs);
/// idempotent — a second call just returns the already-built graph's sample rate.
///
/// Returns `None` only if `AudioContext` construction itself fails outright (no audio subsystem
/// at all) — the caller should treat this as "run silently," not a hard error.
#[must_use]
pub fn ensure_audio(volume: f32) -> Option<u32> {
    if let Some(rate) = STATE.with(|s| s.borrow().as_ref().map(|s| sample_rate_of(&s.ctx))) {
        return Some(rate);
    }

    let ctx = AudioContext::new().ok()?;
    let _ = ctx.resume(); // gesture-critical synchronous call; the returned promise is best-effort
    let rate = sample_rate_of(&ctx);
    let resampler = Resampler::new(rate, volume);

    STATE.with(|s| {
        *s.borrow_mut() = Some(AudioState {
            ctx: ctx.clone(),
            sink: Sink::Pending,
            resampler,
        });
    });

    wasm_bindgen_futures::spawn_local(async move {
        if attach_worklet(&ctx).await.is_none() {
            attach_script_processor(&ctx);
        }
    });

    Some(rate)
}

/// Try to build + attach the `AudioWorkletNode` path. `None` on any failure (unsupported,
/// module load error, node construction error) — the caller falls back to `ScriptProcessorNode`.
// `JsFuture` (and everything built on `js_sys`/`web_sys` handles) is `!Send` by construction —
// wasm32-unknown-unknown is single-threaded, so `Send` is meaningless here; `spawn_local` (not
// `spawn`) is what actually runs this future, and it has no `Send` bound.
#[allow(clippy::future_not_send)]
async fn attach_worklet(ctx: &AudioContext) -> Option<()> {
    let worklet = ctx.audio_worklet().ok()?;
    let url = worklet_blob_url().ok()?;
    let promise = worklet.add_module(&url).ok()?;
    JsFuture::from(promise).await.ok()?;

    let node = AudioWorkletNode::new(ctx, WORKLET_NAME).ok()?;
    let port = node.port().ok()?;

    let occupancy = Rc::new(Cell::new((0usize, 1usize)));
    let cb_occupancy = Rc::clone(&occupancy);
    let closure: Closure<dyn FnMut(MessageEvent)> = Closure::new(move |event: MessageEvent| {
        if let Some((occ, cap)) = parse_occupancy_report(&event) {
            cb_occupancy.set((occ, cap));
        }
    });
    port.set_onmessage(Some(closure.as_ref().unchecked_ref()));
    closure.forget(); // outlives this fn; the node (kept in `AudioState`) owns the port

    node.connect_with_audio_node(&ctx.destination()).ok()?;

    STATE.with(|s| {
        if let Some(state) = s.borrow_mut().as_mut() {
            state.sink = Sink::Worklet { node, occupancy };
        }
    });
    Some(())
}

/// Parse the worklet's `port.postMessage([occupancy, capacity])` report.
fn parse_occupancy_report(event: &MessageEvent) -> Option<(usize, usize)> {
    let arr = Array::from(&event.data());
    let occ = arr.get(0).as_f64()?;
    let cap = arr.get(1).as_f64()?;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    Some((occ.max(0.0) as usize, cap.max(1.0) as usize))
}

/// Build the fallback `ScriptProcessorNode` path (main-thread `onaudioprocess`, plain
/// [`AudioRing`]) — used when [`attach_worklet`] fails for any reason.
fn attach_script_processor(ctx: &AudioContext) {
    let Ok(node) = ctx
        .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
            SCRIPT_PROCESSOR_BUFFER_FRAMES, 0, 2,
        )
    else {
        return;
    };

    let ring = Rc::new(AudioRing::new(SCRIPT_PROCESSOR_RING_POW2));
    let cb_ring = Rc::clone(&ring);
    let closure: Closure<dyn FnMut(AudioProcessingEvent)> =
        Closure::new(move |event: AudioProcessingEvent| drain_into_output(&cb_ring, &event));
    node.set_onaudioprocess(Some(closure.as_ref().unchecked_ref()));
    closure.forget(); // outlives this fn; the node (kept in `AudioState`) owns the callback

    if node.connect_with_audio_node(&ctx.destination()).is_err() {
        return;
    }

    STATE.with(|s| {
        if let Some(state) = s.borrow_mut().as_mut() {
            state.sink = Sink::ScriptProcessor { _node: node, ring };
        }
    });
}

/// `onaudioprocess`: fill `event`'s output buffer's two channels by draining `ring`, one L/R pair
/// per output frame (underrun -> silence, same as the native cpal callback).
fn drain_into_output(ring: &AudioRing, event: &AudioProcessingEvent) {
    let Ok(output) = event.output_buffer() else {
        return;
    };
    let len = output.length() as usize;
    let mut left = vec![0f32; len];
    let mut right = vec![0f32; len];
    for i in 0..len {
        left[i] = ring.pop();
        right[i] = ring.pop();
    }
    let _ = output.copy_to_channel(&left, 0);
    let _ = output.copy_to_channel(&right, 1);
}

/// Resample `samples` (32 kHz `i16` stereo, one emulated frame's worth of the S-DSP's native
/// output) into the active sink.
///
/// Applies the same dynamic-rate-control servo the native path uses. A no-op if [`ensure_audio`]
/// was never successfully called, or the graph hasn't attached yet.
pub fn push_samples(samples: &[(i16, i16)]) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let Some(state) = state.as_mut() else {
            return;
        };
        match &state.sink {
            Sink::Pending => {}
            Sink::Worklet { node, occupancy } => {
                let (occ, cap) = occupancy.get();
                let drc = drc_ratio(occ, cap / 2, cap);
                let mut buf = Vec::new();
                state.resampler.process_into(samples, drc, &mut buf);
                if let Ok(port) = node.port() {
                    let arr = Float32Array::from(buf.as_slice());
                    let _ = port.post_message(&arr);
                }
            }
            Sink::ScriptProcessor { ring, .. } => {
                let capacity = ring.capacity();
                let drc = drc_ratio(ring.occupancy(), capacity / 2, capacity);
                state.resampler.process(samples, drc, ring);
            }
        }
    });
}

/// Update the master volume (from a future Settings UI; unused by the MVP, wired for parity with
/// the native `Resampler::set_volume` call site).
pub fn set_volume(volume: f32) {
    STATE.with(|s| {
        if let Some(state) = s.borrow_mut().as_mut() {
            state.resampler.set_volume(volume);
        }
    });
}

// `AudioContext::sample_rate()` returns an `f32` (WebIDL `float`); real device rates are small
// positive integers (8 kHz..192 kHz), far below `f32`'s exact-integer range, so this narrowing is
// lossless in practice.
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn sample_rate_of(ctx: &AudioContext) -> u32 {
    ctx.sample_rate() as u32
}

/// Wrap [`WORKLET_JS`] in a `Blob:` object URL, avoiding shipping it as a second JS asset for
/// trunk to bundle.
fn worklet_blob_url() -> Result<String, JsValue> {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(WORKLET_JS));
    let opts = BlobPropertyBag::new();
    opts.set_type("application/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&parts, &opts)?;
    Url::create_object_url_with_blob(&blob)
}
