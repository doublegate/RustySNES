//! `v1.1.0` — decoupled triple-buffer framebuffer handoff (emu-thread feature parity).
//!
//! Ported from RustyNES's `present_buffer.rs` (same author, near-verbatim shape — the
//! triple-buffer SPSC handoff itself has no NES/SNES-specific content). The present (winit)
//! thread must upload the most-recently-produced SNES framebuffer every redraw; copying it out of
//! `EmuCore::framebuffer()` under the emu mutex would serialize the present against the emulation
//! thread's whole `run_frame` (which the `emu-thread` feature exists specifically to decouple).
//!
//! This triple buffer moves the framebuffer copy OFF the emu mutex onto a dedicated handoff that
//! is **never held across emulation work** — only for the brief RGBA8 memcpy of a publish/take.
//! The producer writes into the slot neither thread currently reads (`back`), publishes it (swap
//! `back`<->`ready`), and the consumer swaps the freshest slot into its own hand
//! (`front`<->`ready`). The present thread can block at most for one framebuffer copy, never for
//! a full `run_frame`.
//!
//! ## Determinism
//!
//! This is a pure presentation-path optimization: it moves *where* the already-produced,
//! deterministic framebuffer bytes are copied (off the emu lock, onto the handoff). It changes no
//! emulated state or audio. The bytes published are exactly `EmuCore::framebuffer()` when
//! run-ahead is off; with run-ahead on (post-`v1.3.0`), they're the deeper *peeked* frame
//! `crate::rewind::step_with_run_ahead` produces instead — still a pure function of the same
//! deterministic `System` state (peek-then-restore, never a second persisted branch), so the
//! determinism contract (same seed + ROM + input => bit-identical FB + audio) is unaffected
//! either way; only *which* already-deterministic frame gets presented changes.
//!
//! ## SPSC contract + synchronization
//!
//! Exactly ONE producer (the emu thread) and ONE consumer (the winit present path). The index
//! word's three 2-bit fields (`back` / `ready` / `front`) are always a permutation of `{0,1,2}`,
//! so the producer and consumer touch disjoint slots. A small dedicated `Mutex` guards the index
//! word + the `has_new` flag + the slot bytes AND dims together so a publish and a take stay
//! consistent without `unsafe`; that mutex is held only for the brief copy. A cheap `Relaxed`
//! `has_new` pre-check lets the consumer early-out without taking the mutex when nothing is new.
//! Post-`v1.3.0`: each slot's `(width, height)` travels WITH its bytes (see [`Slots::dims`]) —
//! run-ahead can publish a peeked frame whose dims differ from `EmuCore::fb_dims()`'s
//! currently-persisted-state reading (a hi-res-mode-toggle-mid-peek edge case), so the consumer
//! must never re-query dims separately from the bytes it's about to upload.
//!
//! ## SNES-specific adaptation
//!
//! Unlike RustyNES's fixed 256x240 framebuffer, RustySNES's framebuffer is variable-size
//! (hi-res Modes 5/6 double the width/height) — `FB_LEN` (see [`PresentBuffer::fb_len`]) is
//! therefore only an informational worst-case sizing hint (the actual slot `Vec`s resize
//! dynamically via `clear`+`extend_from_slice` on every publish/take, so a smaller
//! native-resolution frame is never zero-padded to the
//! hi-res worst case).

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Worst-case SNES framebuffer size in bytes (512x448 hi-res x RGBA8) — an informational sizing
/// hint only (see module doc); the actual slots resize to whatever `publish` is given.
const FB_LEN: usize = 512 * 448 * 4;

/// The three reusable byte buffers behind the handoff.
struct Slots {
    bufs: [Vec<u8>; 3],
    /// The `(width, height)` each `bufs` slot's bytes decode as — post-`v1.3.0` (run-ahead
    /// support): a run-ahead-peeked frame's dims can differ from `EmuCore::fb_dims()`'s
    /// currently-persisted-state reading in a hi-res-mode-toggle-mid-peek edge case, so dims
    /// travel WITH the bytes through this same slot (one lock acquisition, always a consistent
    /// pair) rather than the consumer re-querying `EmuCore::fb_dims()` separately, which could
    /// observe a size that doesn't match the bytes it's about to upload.
    dims: [(u32, u32); 3],
    /// `front | (ready << 2) | (back << 4)` slot ids; mutated only by the producer's `publish`
    /// and the consumer's `take_into` via a swap that keeps the three fields a permutation of
    /// `{0,1,2}`. Stored here (a plain `u8` under the `slots` mutex) rather than as an atomic on
    /// `PresentBuffer`, because it is only ever read or written while the lock is held.
    index: u8,
}

/// Triple buffer for the SNES framebuffer handoff.
///
/// Producer = emu thread, consumer = winit present path; guarded by a small dedicated mutex held
/// only for the brief publish/take copy — never across emulation work.
///
/// The packed index word holds three slot ids (0..=2): `front` (the consumer's scratch), `ready`
/// (the freshest published frame), and `back` (the producer's scratch). `has_new` flags whether
/// `ready` holds a frame the consumer has not yet taken.
pub struct PresentBuffer {
    /// Set by `publish`, cleared by `take_into`: whether `ready` is fresh.
    has_new: AtomicBool,
    /// Published-frame count (diagnostic only — proves the producer is live).
    generation: AtomicUsize,
    slots: Mutex<Slots>,
}

impl PresentBuffer {
    /// Initial packed index: front = 0, ready = 1, back = 2.
    const INIT_INDEX: u8 = Self::pack(0, 1, 2);

    /// New, empty handoff (all three slots zero-length until the first publish sizes them).
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            has_new: AtomicBool::new(false),
            generation: AtomicUsize::new(0),
            slots: Mutex::new(Slots {
                bufs: [Vec::new(), Vec::new(), Vec::new()],
                dims: [(0, 0), (0, 0), (0, 0)],
                index: Self::INIT_INDEX,
            }),
        })
    }

    const fn front(idx: u8) -> usize {
        (idx & 0b11) as usize
    }
    const fn ready(idx: u8) -> usize {
        ((idx >> 2) & 0b11) as usize
    }
    const fn back(idx: u8) -> usize {
        ((idx >> 4) & 0b11) as usize
    }
    #[allow(clippy::cast_possible_truncation)] // each id is 0..=2.
    const fn pack(front: usize, ready: usize, back: usize) -> u8 {
        (front as u8) | ((ready as u8) << 2) | ((back as u8) << 4)
    }

    /// Producer: copy `frame` (and its `dims`) into the back slot and publish it (swap
    /// back<->ready). The consumer's `front` slot is never touched here, so a concurrent take
    /// only contends for the brief copy window.
    ///
    /// `frame` is normally the deterministic `EmuCore::framebuffer()` bytes (with `dims` =
    /// `EmuCore::fb_dims()`); post-`v1.3.0`, a run-ahead-peeked frame's own (bytes, dims) pair is
    /// published instead when run-ahead is active, so the two always travel together.
    ///
    /// # Panics
    /// Panics only if the internal slots mutex is poisoned (a prior panic while the lock was
    /// held elsewhere) — not a condition this module's own code can trigger.
    pub fn publish(&self, frame: &[u8], dims: (u32, u32)) {
        let mut slots = self.slots.lock().expect("present buffer slots");
        let idx = slots.index;
        let back = Self::back(idx);
        {
            let buf = &mut slots.bufs[back];
            buf.clear();
            buf.extend_from_slice(frame);
            slots.dims[back] = dims;
        }
        // Swap back -> ready (front unchanged) and arm `has_new` — all under the same slots
        // lock, so the index move, the slot write, and the fresh-flag stay consistent for the
        // consumer (which also takes the lock before reading any of them).
        let front = Self::front(idx);
        let ready = Self::ready(idx);
        slots.index = Self::pack(front, back, ready);
        self.has_new.store(true, Ordering::Relaxed);
        drop(slots);
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Consumer: if a new frame was published since the last call, swap it into the front slot
    /// and copy it into `out` (the present-staging buffer the GPU uploads). Returns the frame's
    /// `(width, height)` when `out` was refreshed with a new frame, `None` when there was
    /// nothing new (the caller keeps the previously presented `out` — the display simply
    /// re-presents it). The returned dims are always the exact pair `publish` was called with
    /// for these bytes (never a separately-queried `EmuCore::fb_dims()`, which could disagree
    /// with a run-ahead-peeked frame's own dims — see the module doc).
    ///
    /// # Panics
    /// Panics only if the internal slots mutex is poisoned (a prior panic while the lock was
    /// held elsewhere) — not a condition this module's own code can trigger.
    pub fn take_into(&self, out: &mut Vec<u8>) -> Option<(u32, u32)> {
        // Cheap pre-check off the lock; the authoritative check is under it.
        if !self.has_new.load(Ordering::Relaxed) {
            return None;
        }
        let mut slots = self.slots.lock().expect("present buffer slots");
        if !self.has_new.swap(false, Ordering::Relaxed) {
            return None;
        }
        let idx = slots.index;
        let front = Self::front(idx);
        let ready = Self::ready(idx);
        let back = Self::back(idx);
        // Swap front <-> ready so the producer's next `publish` reuses our old front as its
        // back, and we now own the freshest frame in `ready`.
        slots.index = Self::pack(ready, front, back);
        out.clear();
        out.extend_from_slice(&slots.bufs[ready]);
        Some(slots.dims[ready])
    }

    /// True once at least one frame has been published (so the present path can distinguish
    /// "no ROM / not yet produced" from "have a frame").
    #[must_use]
    pub fn has_published(&self) -> bool {
        self.generation.load(Ordering::Relaxed) > 0
    }

    /// Reset to the empty (no-frame) state on a ROM load / power cycle so the next present shows
    /// a black frame until the first new frame arrives.
    ///
    /// # Panics
    /// Panics only if the internal slots mutex is poisoned (a prior panic while the lock was
    /// held elsewhere) — not a condition this module's own code can trigger.
    pub fn reset(&self) {
        self.has_new.store(false, Ordering::Release);
        self.generation.store(0, Ordering::Relaxed);
        let mut slots = self.slots.lock().expect("present buffer slots");
        slots.index = Self::INIT_INDEX;
        for b in &mut slots.bufs {
            b.clear();
        }
    }

    /// Worst-case SNES framebuffer byte length (for the no-ROM black frame) — see module doc for
    /// why this is only a sizing hint, not load-bearing.
    #[must_use]
    pub const fn fb_len() -> usize {
        FB_LEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_then_take_roundtrips() {
        let pb = PresentBuffer::new();
        assert!(!pb.has_published());
        let frame = vec![0xABu8; 16];
        pb.publish(&frame, (4, 4));
        assert!(pb.has_published());
        let mut out = Vec::new();
        assert_eq!(pb.take_into(&mut out), Some((4, 4)));
        assert_eq!(out, frame);
        // No new frame -> take returns None and leaves `out` intact.
        assert_eq!(pb.take_into(&mut out), None);
        assert_eq!(out, frame);
    }

    #[test]
    fn latest_publish_wins_when_producer_outruns_consumer() {
        let pb = PresentBuffer::new();
        for i in 0..5u8 {
            pb.publish(&[i; 8], (u32::from(i), u32::from(i)));
        }
        // The consumer only ever sees the freshest frame (older ones were overwritten in the
        // back slot before a take) — the intended "drop stale frames" behavior under wall-clock
        // pacing.
        let mut out = Vec::new();
        assert_eq!(pb.take_into(&mut out), Some((4, 4)));
        assert_eq!(out, vec![4u8; 8]);
    }

    #[test]
    fn slots_never_alias_under_interleaving() {
        let pb = PresentBuffer::new();
        let mut out = Vec::new();
        for i in 0..50u8 {
            pb.publish(&[i; 4], (1, 1));
            if i % 3 == 0 {
                let _ = pb.take_into(&mut out);
            }
            let idx = pb.slots.lock().unwrap().index;
            let ids = [
                PresentBuffer::front(idx),
                PresentBuffer::ready(idx),
                PresentBuffer::back(idx),
            ];
            let mut seen = [false; 3];
            for s in ids {
                assert!(s < 3, "slot id out of range: {s}");
                assert!(!seen[s], "slot {s} aliased");
                seen[s] = true;
            }
        }
    }

    #[test]
    fn reset_clears_published_state() {
        let pb = PresentBuffer::new();
        pb.publish(&[1u8; 8], (1, 1));
        assert!(pb.has_published());
        pb.reset();
        assert!(!pb.has_published());
        let mut out = Vec::new();
        assert_eq!(pb.take_into(&mut out), None);
    }

    #[test]
    fn fb_len_matches_hires_worst_case() {
        assert_eq!(PresentBuffer::fb_len(), 512 * 448 * 4);
    }

    #[test]
    fn concurrent_producer_consumer_no_torn_frame() {
        use std::thread;
        let pb = PresentBuffer::new();
        let prod = Arc::clone(&pb);
        let done = Arc::new(AtomicBool::new(false));
        let done_p = Arc::clone(&done);
        let h = thread::spawn(move || {
            for i in 0..10_000u32 {
                #[allow(clippy::cast_possible_truncation)]
                prod.publish(&[(i & 0xFF) as u8; 64], (64, 1));
            }
            done_p.store(true, Ordering::Release);
        });
        // Consume until the producer is done AND the buffer is drained. This is
        // scheduling-independent (no fixed poll budget that a loaded CI runner could exhaust
        // before the producer is scheduled), so `taken > 0` is deterministic — the buffer
        // always retains the latest published frame, and the post-`done` drain catches it.
        let mut out = Vec::new();
        let mut taken = 0u32;
        loop {
            if pb.take_into(&mut out).is_some() {
                taken += 1;
                // Every taken frame is a full 64-byte uniform buffer (no torn read across
                // slots).
                assert_eq!(out.len(), 64);
                let v = out[0];
                assert!(out.iter().all(|&b| b == v), "torn frame");
            } else if done.load(Ordering::Acquire) {
                // Producer finished, so at most ONE final frame remains (the buffer only
                // retains the latest publish). Take it once, count it, then stop.
                if pb.take_into(&mut out).is_some() {
                    taken += 1;
                    assert_eq!(out.len(), 64);
                    let v = out[0];
                    assert!(out.iter().all(|&b| b == v), "torn frame");
                }
                break;
            } else {
                std::hint::spin_loop();
            }
        }
        h.join().unwrap();
        assert!(taken > 0, "consumer never observed a frame");
    }
}
