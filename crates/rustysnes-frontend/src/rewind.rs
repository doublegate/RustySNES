//! Rewind + run-ahead — frontend-side orchestration built entirely on [`EmuCore::save_state`] /
//! [`EmuCore::load_state`] (`docs/adr/0006`).
//!
//! Pure frontend concerns: the core never knows either exists, matching `docs/adr/0004`'s
//! determinism boundary (rate control + run-ahead live here, never in core synthesis).
//!
//! Unlike `docs/frontend.md`'s original "ring of keyframes + deltas" sketch, [`RewindBuffer`]
//! stores full snapshots — delta-compression is a possible future memory optimization, not a
//! correctness requirement, and full snapshots are trivially correct to reason about (no
//! keyframe-chain replay logic to get wrong).

use crate::emu::EmuCore;

/// A bounded ring buffer of full save-state snapshots for rewind.
///
/// Snapshots are recorded every `interval_frames` real frames (not every frame) to bound
/// memory — a full snapshot includes VRAM/CGRAM/OAM/WRAM/ARAM and every coprocessor's state, so
/// recording every single frame would be wasteful for a feature whose whole point is coarse
/// time travel, not frame-perfect scrubbing. [`Self::step_back`] pops the most recent snapshot,
/// so each call rewinds by exactly one recorded interval.
#[derive(Debug)]
pub struct RewindBuffer {
    /// Maximum number of snapshots retained; the oldest is evicted once this is exceeded.
    capacity: usize,
    /// Record a snapshot only every this many calls to [`Self::record`] (minimum 1).
    interval_frames: u32,
    /// Frames elapsed since the last recorded snapshot.
    frames_since_snapshot: u32,
    /// The ring buffer itself, oldest-first (`VecDeque::push_back` / `pop_front` on overflow,
    /// `pop_back` to rewind).
    states: std::collections::VecDeque<Vec<u8>>,
}

impl RewindBuffer {
    /// Construct a rewind buffer holding at most `capacity` snapshots, recorded every
    /// `interval_frames` real frames. `capacity == 0` makes [`Self::record`] a permanent no-op
    /// (the additive-default-off posture: a zeroed config disables rewind entirely at zero
    /// cost). `interval_frames` is clamped to at least 1.
    #[must_use]
    pub fn new(capacity: usize, interval_frames: u32) -> Self {
        Self {
            capacity,
            interval_frames: interval_frames.max(1),
            frames_since_snapshot: 0,
            states: std::collections::VecDeque::new(),
        }
    }

    /// Whether this buffer ever records (i.e. `capacity > 0`).
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.capacity > 0
    }

    /// The number of snapshots currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Whether no snapshot is currently held (nothing to rewind to).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Discard every recorded snapshot. Call this on ROM load/close — a new cart invalidates any
    /// prior snapshot (they belong to a different ROM/format). Deliberately NOT called on
    /// Reset/Power-Cycle: rewinding past an accidental reset is a legitimate use case, so those
    /// discontinuities keep the buffer intact (see `app.rs`'s `MenuAction` dispatch).
    pub fn clear(&mut self) {
        self.states.clear();
        self.frames_since_snapshot = 0;
    }

    /// Call once per real (non-peek) frame, AFTER [`EmuCore::run_frame`]. Records a snapshot of
    /// `core`'s current state once every `interval_frames` calls, evicting the oldest snapshot
    /// first if already at `capacity`. No-op when `capacity == 0`.
    pub fn record(&mut self, core: &EmuCore) {
        if self.capacity == 0 {
            return;
        }
        // Saturating: a multi-year 60 FPS session could otherwise overflow this counter (panic in
        // debug, wrap in release). Self-healing at the saturation point: once pinned at
        // `u32::MAX`, the very next call is `>= interval_frames` (triggering a snapshot, which
        // resets the counter to 0), so this never permanently stops recording — worst case is one
        // extra snapshot taken slightly early.
        self.frames_since_snapshot = self.frames_since_snapshot.saturating_add(1);
        if self.frames_since_snapshot < self.interval_frames {
            return;
        }
        self.frames_since_snapshot = 0;
        if self.states.len() >= self.capacity {
            self.states.pop_front();
        }
        self.states.push_back(core.save_state());
    }

    /// Rewind by one recorded snapshot (i.e. `interval_frames` real frames), restoring `core` to
    /// it. Returns `true` if a snapshot was available and restored, `false` if the buffer was
    /// empty (nothing happens to `core` in that case). The restored snapshot is consumed (popped)
    /// — repeated calls step further back until the buffer is exhausted.
    pub fn step_back(&mut self, core: &mut EmuCore) -> bool {
        let Some(bytes) = self.states.pop_back() else {
            return false;
        };
        // A snapshot taken from `core` earlier in this same session always matches `core`'s
        // currently-loaded cart (same ROM, never swapped mid-buffer — `clear()` on ROM load
        // guarantees this), so a restore failure here would indicate a corrupted buffer entry,
        // not a legitimate user-facing error. Silently discard-on-failure would hide that; the
        // snapshot is already popped, so simply not applying it is the honest behavior.
        core.load_state(&bytes).is_ok()
    }
}

/// Run one displayed frame with `frames`-deep run-ahead.
///
/// Peeks `frames` frames into the future using `core`'s currently-latched input, captures that
/// peek's framebuffer for presentation, then rolls back and re-runs exactly ONE real frame so
/// `core`'s persisted state (and its audio — [`EmuCore::audio`] after this call is the real,
/// continuous stream, never peek audio) only ever advances by one frame per call, regardless of
/// `frames`.
///
/// This is pure re-simulation of the SAME deterministic core (`docs/adr/0004`): no injected
/// timing/RNG, just running the existing step function extra times and discarding the result.
/// `frames == 0` is the trivial case — run one real frame and return it directly, matching
/// [`EmuCore::run_frame`] plus a fresh framebuffer copy.
///
/// Returns the framebuffer to actually present (which may be `frames` frames "ahead" of what's
/// persisted) and its `(width, height)`.
///
/// # Panics
/// Panics if restoring `snapshot` (taken from `core` moments earlier, same session, same cart)
/// fails — that can only mean a genuine internal invariant violation, not a legitimate,
/// recoverable error; silently skipping the rollback would leave `core` incorrectly persisting
/// the deep-peeked state instead of advancing by exactly one frame.
#[must_use]
pub fn step_with_run_ahead(core: &mut EmuCore, frames: u32) -> (Vec<u8>, (u32, u32)) {
    if frames == 0 {
        core.run_frame();
        return (core.framebuffer().to_vec(), core.fb_dims());
    }

    let snapshot = core.save_state();
    for _ in 0..frames {
        core.run_frame();
    }
    let peek_framebuffer = core.framebuffer().to_vec();
    let peek_dims = core.fb_dims();

    // Roll back to the pre-peek state and re-run exactly one real frame: this is what actually
    // persists (and produces the continuous audio stream), regardless of how deep the peek ran.
    // `snapshot` was taken from THIS SAME `core` a few lines above (same session, same cart), so
    // a restore failure here can only mean a genuine internal invariant violation, not a
    // legitimate/recoverable error — silently skipping the rollback would leave `core` incorrectly
    // persisting the deep-peeked state (and its audio) instead of advancing by exactly one frame,
    // breaking this function's whole contract. Fail loudly instead of corrupting state quietly.
    core.load_state(&snapshot)
        .expect("run-ahead rollback: snapshot taken from this same core moments ago must restore");
    core.run_frame();

    (peek_framebuffer, peek_dims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Region;

    /// Build a minimal-but-valid LoROM ROM whose reset-vector program turns the display on
    /// (`$2100 = $0F`, clearing forced-blank), enables NMI-on-vblank (`$4200 = $81`), then spins
    /// forever. The NMI handler increments a WRAM counter and writes it into CGRAM entry 0 (the
    /// backdrop color, `$2121`/`$2122`) **once per frame** — deliberately tied to the NMI/vblank
    /// edge rather than to an in-loop instruction counter, since a fixed-cycle busy loop's
    /// position at a fixed-length frame boundary is otherwise exactly periodic (every frame
    /// samples the SAME phase of the loop), which would make the backdrop color never actually
    /// change from one `run_frame` to the next. Ties the composited framebuffer to a real
    /// once-per-frame signal, giving these tests an observable state signal to snapshot/compare.
    fn counter_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x8000];
        #[rustfmt::skip]
        let program: [u8; 32] = [
            0x78,                   // SEI
            0xA9, 0x0F,             // LDA #$0F
            0x8D, 0x00, 0x21,       // STA $2100      (INIDISP: force-blank off, brightness max)
            0xA9, 0x81,             // LDA #$81
            0x8D, 0x00, 0x42,       // STA $4200      (NMITIMEN: enable NMI on vblank)
            0x4C, 0x0B, 0x80,       // loop: JMP $800B  (spin; NMI interrupts this every vblank)
            // --- NMI handler, at rom offset 0x0E == CPU $800E ---
            0xEE, 0x10, 0x00,       // INC $0010      (WRAM frame counter)
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x21, 0x21,       // STA $2121      (CGADD = 0, backdrop — fixed constant)
            0xAD, 0x10, 0x00,       // LDA $0010      (reload the counter)
            0x8D, 0x22, 0x21,       // STA $2122      (CGDATA low byte)
            0x8D, 0x22, 0x21,       // STA $2122      (CGDATA high byte, same counter value)
            0x40,                   // RTI
        ];
        rom[..program.len()].copy_from_slice(&program);

        let h = 0x7FC0;
        rom[h..h + 21].copy_from_slice(b"REWIND TEST ROM      ");
        rom[h + 0x15] = 0x20; // LoROM, slow
        rom[h + 0x16] = 0x00; // no coprocessor, no RAM, no battery
        rom[h + 0x18] = 0x00; // RAM size 0
        rom[h + 0x19] = 0x01; // North America / NTSC
        let checksum: u16 = 0x1234;
        let complement = !checksum;
        rom[h + 0x1C..h + 0x1E].copy_from_slice(&complement.to_le_bytes());
        rom[h + 0x1E..h + 0x20].copy_from_slice(&checksum.to_le_bytes());
        rom[h + 0x3A..h + 0x3C].copy_from_slice(&0x800Eu16.to_le_bytes()); // NMI vector (emulation mode, $FFFA)
        rom[h + 0x3C..h + 0x3E].copy_from_slice(&0x8000u16.to_le_bytes()); // reset vector
        rom
    }

    fn booted_core() -> EmuCore {
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&counter_rom()).expect("test ROM should load");
        core
    }

    #[test]
    fn disabled_buffer_never_records() {
        let core = booted_core();
        let mut buf = RewindBuffer::new(0, 1);
        assert!(!buf.is_enabled());
        for _ in 0..10 {
            buf.record(&core);
        }
        assert!(buf.is_empty());
    }

    #[test]
    fn records_on_interval_and_evicts_oldest_past_capacity() {
        let mut core = booted_core();
        let mut buf = RewindBuffer::new(3, 2);
        for _ in 0..20 {
            core.run_frame();
            buf.record(&core);
        }
        // 20 frames / interval 2 = 10 snapshots taken, capacity 3 → only the newest 3 remain.
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn step_back_with_empty_buffer_returns_false_and_leaves_core_unchanged() {
        let mut core = booted_core();
        let mut buf = RewindBuffer::new(4, 1);
        for _ in 0..5 {
            core.run_frame();
        }
        let before = core.framebuffer().to_vec();
        assert!(!buf.step_back(&mut core));
        assert_eq!(core.framebuffer(), before.as_slice());
    }

    #[test]
    fn rewind_restores_an_earlier_frames_exact_state() {
        let mut core = booted_core();
        let mut buf = RewindBuffer::new(64, 1);

        core.run_frame();
        buf.record(&core);
        let snapshot_frame = core.framebuffer().to_vec();

        // Advance further; the counter program guarantees the backdrop color (and therefore the
        // framebuffer) has changed by now.
        for _ in 0..5 {
            core.run_frame();
            buf.record(&core);
        }
        assert_ne!(core.framebuffer(), snapshot_frame.as_slice());

        // 6 snapshots total are recorded (after frames 1..=6). `step_back` pops newest-first, so
        // 6 pops walks back through frames 6,5,4,3,2 and finally restores the frame-1 snapshot.
        for _ in 0..6 {
            assert!(buf.step_back(&mut core));
        }
        assert_eq!(core.framebuffer(), snapshot_frame.as_slice());
    }

    #[test]
    fn run_ahead_zero_frames_is_a_plain_run_frame() {
        let mut a = booted_core();
        let mut b = booted_core();
        a.run_frame();
        let (peeked, dims) = step_with_run_ahead(&mut b, 0);
        assert_eq!(peeked, a.framebuffer());
        assert_eq!(dims, a.fb_dims());
    }

    #[test]
    fn run_ahead_peek_matches_running_that_many_frames_directly() {
        const PEEK: u32 = 3;
        // Ground truth: run exactly `PEEK` frames directly on a reference core, no peeking
        // involved — `step_with_run_ahead`'s peek phase runs `frames` `run_frame()` calls
        // starting from the pre-call state, so it advances exactly `PEEK` frames too.
        let mut reference = booted_core();
        for _ in 0..PEEK {
            reference.run_frame();
        }
        let reference_frame = reference.framebuffer().to_vec();

        let mut core = booted_core();
        let (presented, _) = step_with_run_ahead(&mut core, PEEK);
        assert_eq!(presented, reference_frame);
    }

    #[test]
    fn run_ahead_only_persists_one_real_frame_per_call() {
        // After a single `step_with_run_ahead` call, the CORE's own actual (non-peeked) state
        // must match running exactly one real frame — proving the N peek frames were discarded,
        // not accidentally kept.
        let mut reference = booted_core();
        reference.run_frame();

        let mut core = booted_core();
        let _ = step_with_run_ahead(&mut core, 5);

        // A second `step_with_run_ahead(0)` (== a plain `run_frame`) from each core should now
        // walk them forward identically if their PERSISTED (non-peek) state already matched.
        let (from_reference, _) = step_with_run_ahead(&mut reference, 0);
        let (from_core, _) = step_with_run_ahead(&mut core, 0);
        assert_eq!(from_reference, from_core);
    }

    #[test]
    fn run_ahead_audio_is_the_real_stream_not_peek_audio() {
        // The S-DSP free-runs at 32 kHz regardless of whether the game writes it, so a real
        // frame always emits a fixed sample count. If run-ahead's discarded peek frames leaked
        // into `EmuCore::audio()` (accumulated or duplicated), a 4-frame peek would report ~5x
        // a plain frame's sample count instead of exactly 1x.
        let mut reference = booted_core();
        reference.run_frame();
        let reference_len = reference.audio().len();
        assert!(
            reference_len > 0,
            "the S-DSP should emit samples every frame"
        );

        let mut core = booted_core();
        let _ = step_with_run_ahead(&mut core, 4);
        assert_eq!(core.audio().len(), reference_len);
    }
}
