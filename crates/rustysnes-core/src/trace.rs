//! Instruction trace, control-flow events, and a memory-access heat map (`v1.25.0`, T-FP-C1).
//!
//! The third rung of the same opt-in debugger observability [`crate::watchpoint`] established, and
//! written to the same contract: feature-gated on `debug-hooks` (this crate compiles the module out
//! entirely otherwise), **never** part of a save state ([`docs/adr/0004`]), and costing exactly one
//! `bool` test per hook when nothing is being recorded.
//!
//! Where a watchpoint answers "who touched *this* address?", these answer the three questions a
//! watchpoint structurally cannot:
//!
//! - **What ran?** — [`TraceState::record_step`], a bounded ring of executed instructions with the
//!   full register file at the moment of execution.
//! - **How did it get here?** — [`TraceState::record_event`], a call/return/interrupt log, which is
//!   what a call-stack view is reconstructed from. A point-in-time snapshot cannot show this: the
//!   stack holds return addresses but not *when* or *why* each frame was pushed.
//! - **What is hot?** — [`TraceState::note_access`], per-address read/write counts over WRAM, which
//!   is what turns a hex dump into a heat map.
//!
//! # Why recording is separately armed, not implied by `debug-hooks`
//!
//! A watchpoint list is naturally empty until the user arms one, so `is_empty()` is a sufficient
//! gate. These have no such natural empty state — a trace records *everything* by nature — so each
//! carries an explicit `enabled` flag that starts `false`. The heat map's backing allocation is
//! made on the first enable and never on a build that leaves it off, which is why it can afford to
//! be a direct-indexed array rather than a map.
//!
//! [`docs/adr/0004`]: https://github.com/doublegate/RustySNES/blob/main/docs/adr/0004-save-state-format.md

use alloc::vec;
use alloc::vec::Vec;
use core::mem;

/// Instructions retained in the trace ring.
///
/// ~0.1 s of 65C816 execution at typical rates. Long enough to cover the approach to a breakpoint
/// (which is what a trace is read for), short enough that the fixed allocation is ~150 KiB rather
/// than growing without bound while a game runs for an hour behind an open debugger.
pub const MAX_TRACE: usize = 4096;

/// Control-flow events retained.
///
/// Far fewer than instructions — a call/return/interrupt is a small fraction of executed opcodes —
/// so a smaller ring covers proportionally much more wall time.
pub const MAX_EVENTS: usize = 1024;

/// WRAM bytes the heat map covers (`$7E0000-$7FFFFF`).
///
/// Only WRAM: it is the region a debugger actually watches for churn, it is a single contiguous
/// direct-indexable space, and covering the whole 24-bit bus instead would need 16 M counters for
/// a range that is mostly unmapped.
pub const HEATMAP_LEN: usize = 128 * 1024;

/// One executed instruction, captured before it ran.
///
/// The register file is recorded *pre*-execution deliberately: a trace is read to answer "what was
/// the machine holding when it decided to do that", and post-execution state is already the next
/// row's pre-execution state, so recording it after would lose the first row's inputs entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceEntry {
    /// `PBR:PC` (24-bit) of the instruction.
    pub pbr_pc: u32,
    /// The opcode byte at `pbr_pc`.
    pub opcode: u8,
    /// Accumulator (16-bit; the high byte is meaningless in 8-bit mode but is what the register
    /// physically holds, which is what a debugger must show).
    pub a: u16,
    /// X index register.
    pub x: u16,
    /// Y index register.
    pub y: u16,
    /// Stack pointer.
    pub sp: u16,
    /// Direct page register.
    pub dp: u16,
    /// Processor status byte.
    pub p: u8,
    /// Data bank register.
    pub db: u8,
    /// Emulation-mode flag.
    pub emulation: bool,
}

/// What kind of control-flow transfer an event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A subroutine call (`JSR`/`JSL`).
    Call,
    /// A subroutine return (`RTS`/`RTL`).
    Return,
    /// A non-maskable interrupt was taken.
    Nmi,
    /// A maskable interrupt was taken.
    Irq,
    /// A software break (`BRK`).
    Brk,
    /// A coprocessor trap (`COP`).
    Cop,
    /// An interrupt return (`RTI`).
    Rti,
}

/// One recorded control-flow event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceEvent {
    /// What happened.
    pub kind: EventKind,
    /// `PBR:PC` of the instruction (or the interrupted instruction) that caused it.
    pub from: u32,
    /// Where control went — the callee, the return address, or the vector target.
    pub to: u32,
    /// The call depth **after** this event, so a viewer can indent without replaying the log.
    pub depth: u16,
}

impl EventKind {
    /// Whether this event pushes a frame (deepens the call stack).
    const fn is_enter(self) -> bool {
        matches!(
            self,
            Self::Call | Self::Nmi | Self::Irq | Self::Brk | Self::Cop
        )
    }

    /// Whether this event pops a frame.
    const fn is_leave(self) -> bool {
        matches!(self, Self::Return | Self::Rti)
    }
}

/// Read and write counts for one address.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccessCount {
    /// Times the address was read.
    pub reads: u32,
    /// Times the address was written.
    pub writes: u32,
}

impl AccessCount {
    /// Total accesses, saturating rather than wrapping — a counter that wraps to zero after a long
    /// session would read as "never touched", the exact opposite of the truth.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.reads.saturating_add(self.writes)
    }

    /// Whether this address was ever accessed.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.reads == 0 && self.writes == 0
    }
}

/// The trace ring, event log, and access heat map — owned by [`crate::Bus`]/[`crate::System`].
#[derive(Debug, Default)]
pub struct TraceState {
    tracing: bool,
    counting: bool,
    entries: Vec<TraceEntry>,
    /// Where the next entry lands once `entries` reaches [`MAX_TRACE`].
    head: usize,
    events: Vec<TraceEvent>,
    events_head: usize,
    depth: u16,
    /// Lazily allocated on the first enable via [`Self::set_counting`]; empty otherwise, so a build
    /// that never enables counting pays nothing at all.
    heatmap: Vec<AccessCount>,
}

impl TraceState {
    /// Whether instruction tracing is armed.
    #[must_use]
    pub const fn is_tracing(&self) -> bool {
        self.tracing
    }

    /// Whether access counting is armed.
    #[must_use]
    pub const fn is_counting(&self) -> bool {
        self.counting
    }

    /// Arm or disarm instruction + event tracing.
    ///
    /// Disarming keeps whatever was already recorded, so "stop and look at what just happened" is
    /// the natural flow rather than one that discards the evidence at the moment of interest.
    pub const fn set_tracing(&mut self, on: bool) {
        self.tracing = on;
    }

    /// Arm or disarm access counting, allocating the heat map on the first enable.
    pub fn set_counting(&mut self, on: bool) {
        self.counting = on;
        if on && self.heatmap.is_empty() {
            self.heatmap = vec![AccessCount::default(); HEATMAP_LEN];
        }
    }

    /// Record one executed instruction. A no-op unless tracing is armed.
    pub fn record_step(&mut self, entry: TraceEntry) {
        if !self.tracing {
            return;
        }
        if self.entries.len() < MAX_TRACE {
            self.entries.push(entry);
        } else {
            self.entries[self.head] = entry;
            self.head = (self.head + 1) % MAX_TRACE;
        }
    }

    /// Record one control-flow event and update the call depth. A no-op unless tracing is armed.
    ///
    /// Depth saturates at both ends: a trace armed mid-execution starts inside an unknown number of
    /// frames, so the first `Return` it sees has no matching `Call` and must not wrap to 65535.
    pub fn record_event(&mut self, kind: EventKind, from: u32, to: u32) {
        if !self.tracing {
            return;
        }
        if kind.is_enter() {
            self.depth = self.depth.saturating_add(1);
        } else if kind.is_leave() {
            self.depth = self.depth.saturating_sub(1);
        }
        let event = TraceEvent {
            kind,
            from,
            to,
            depth: self.depth,
        };
        if self.events.len() < MAX_EVENTS {
            self.events.push(event);
        } else {
            self.events[self.events_head] = event;
            self.events_head = (self.events_head + 1) % MAX_EVENTS;
        }
    }

    /// Count one bus access. A no-op unless counting is armed or the address is outside WRAM.
    pub fn note_access(&mut self, addr24: u32, is_write: bool) {
        if !self.counting {
            return;
        }
        let Some(idx) = heatmap_index(addr24) else {
            return;
        };
        let Some(slot) = self.heatmap.get_mut(idx) else {
            return;
        };
        // Saturating: see `AccessCount::total`. A wrapped counter reads as "cold" when it is the
        // hottest address in the program.
        if is_write {
            slot.writes = slot.writes.saturating_add(1);
        } else {
            slot.reads = slot.reads.saturating_add(1);
        }
    }

    /// The trace ring in execution order, oldest first.
    #[must_use]
    pub fn entries(&self) -> Vec<TraceEntry> {
        rotate(&self.entries, self.head, MAX_TRACE)
    }

    /// The event log in occurrence order, oldest first.
    #[must_use]
    pub fn events(&self) -> Vec<TraceEvent> {
        rotate(&self.events, self.events_head, MAX_EVENTS)
    }

    /// The current call depth (frames entered and not yet left since tracing was armed).
    #[must_use]
    pub const fn depth(&self) -> u16 {
        self.depth
    }

    /// Instructions currently held in the trace ring.
    ///
    /// Exposed separately from [`Self::entries`] because a caller that only wants to *show* how
    /// full the ring is must not pay for the copy that reading it out costs.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the trace ring holds nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Control-flow events currently held in the event log.
    #[must_use]
    pub const fn event_len(&self) -> usize {
        self.events.len()
    }

    /// Access counts for `len` bytes starting at `addr24`, for the memory viewer's heat column.
    ///
    /// Addresses outside WRAM (and every address when counting has never been enabled) read as
    /// zero, which is the truth: nothing was counted there.
    #[must_use]
    pub fn counts(&self, addr24: u32, len: usize) -> Vec<AccessCount> {
        (0..len)
            .map(|i| {
                let offset = u32::try_from(i).unwrap_or(u32::MAX);
                heatmap_index(addr24.wrapping_add(offset))
                    .and_then(|idx| self.heatmap.get(idx).copied())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// The largest total count in the heat map, for scaling a display. `0` when nothing counted.
    #[must_use]
    pub fn peak_count(&self) -> u32 {
        self.heatmap.iter().map(|c| c.total()).max().unwrap_or(0)
    }

    /// Discard the trace ring and event log, keeping the armed flags.
    pub fn clear_trace(&mut self) {
        let _ = mem::take(&mut self.entries);
        let _ = mem::take(&mut self.events);
        self.head = 0;
        self.events_head = 0;
        self.depth = 0;
    }

    /// Zero every access count, keeping the allocation (so re-arming does not re-allocate).
    pub fn clear_counts(&mut self) {
        self.heatmap.fill(AccessCount::default());
    }
}

/// Map a 24-bit bus address to its heat-map slot, or `None` when it is not WRAM.
///
/// Covers both WRAM windows the SNES exposes: the full `$7E0000-$7FFFFF` linear space and the
/// `$0000-$1FFF` low-RAM mirror in banks `$00-3F`/`$80-BF`, which alias the SAME bytes. Counting
/// them as one slot each is the point — a game that writes low RAM through the mirror and reads it
/// through `$7E` is touching one address, and two separate counters would show two cold ones.
#[must_use]
pub const fn heatmap_index(addr24: u32) -> Option<usize> {
    let bank = (addr24 >> 16) & 0xFF;
    let offset = addr24 & 0xFFFF;
    match bank {
        0x7E..=0x7F => Some((addr24 & 0x1_FFFF) as usize),
        0x00..=0x3F | 0x80..=0xBF if offset < 0x2000 => Some((offset & 0x1FFF) as usize),
        _ => None,
    }
}

/// Return a ring's contents oldest-first.
///
/// Before the ring is full, `head` is 0 and the storage is already chronological; once full, the
/// oldest element is at `head`.
fn rotate<T: Copy>(buf: &[T], head: usize, cap: usize) -> Vec<T> {
    if buf.len() < cap {
        return buf.to_vec();
    }
    let mut out = Vec::with_capacity(cap);
    out.extend_from_slice(&buf[head..]);
    out.extend_from_slice(&buf[..head]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(pc: u32) -> TraceEntry {
        TraceEntry {
            pbr_pc: pc,
            opcode: 0xEA,
            a: 0,
            x: 0,
            y: 0,
            sp: 0x01FF,
            dp: 0,
            p: 0x34,
            db: 0,
            emulation: false,
        }
    }

    /// Nothing is recorded until tracing is armed — the whole point of a separate flag, since a
    /// trace has no natural "empty until the user asks" state the way a watchpoint list does.
    #[test]
    fn disarmed_state_records_nothing() {
        let mut st = TraceState::default();
        assert!(!st.is_tracing() && !st.is_counting());
        st.record_step(entry(0x00_8000));
        st.record_event(EventKind::Call, 0x00_8000, 0x00_9000);
        st.note_access(0x7E_0000, true);
        assert!(st.entries().is_empty());
        assert!(st.events().is_empty());
        assert_eq!(st.peak_count(), 0);
        assert!(st.counts(0x7E_0000, 4).iter().all(|c| c.is_zero()));
    }

    /// The ring keeps the most recent `MAX_TRACE` instructions in execution order.
    #[test]
    fn trace_ring_evicts_oldest_and_stays_chronological() {
        let mut st = TraceState::default();
        st.set_tracing(true);
        let total = u32::try_from(MAX_TRACE + MAX_TRACE / 2).expect("fits u32");
        let cap = u32::try_from(MAX_TRACE).expect("fits u32");
        for i in 0..total {
            st.record_step(entry(i));
        }
        let out = st.entries();
        assert_eq!(out.len(), MAX_TRACE);
        assert_eq!(out[0].pbr_pc, total - cap);
        assert_eq!(out[MAX_TRACE - 1].pbr_pc, total - 1);
        for w in out.windows(2) {
            assert!(w[1].pbr_pc > w[0].pbr_pc, "ring is not chronological");
        }
    }

    /// Disarming keeps the evidence — "stop and look at what just happened" must not discard it.
    #[test]
    fn disarming_preserves_what_was_recorded() {
        let mut st = TraceState::default();
        st.set_tracing(true);
        st.record_step(entry(0x00_8000));
        st.set_tracing(false);
        st.record_step(entry(0x00_8001));
        let out = st.entries();
        assert_eq!(out.len(), 1, "the post-disarm step must not be recorded");
        assert_eq!(out[0].pbr_pc, 0x00_8000);
        st.clear_trace();
        assert!(st.entries().is_empty());
    }

    /// Depth tracks enter/leave and is what a call-stack view indents by.
    #[test]
    fn events_track_call_depth() {
        let mut st = TraceState::default();
        st.set_tracing(true);
        st.record_event(EventKind::Call, 0x00_8000, 0x00_9000);
        st.record_event(EventKind::Call, 0x00_9000, 0x00_A000);
        st.record_event(EventKind::Nmi, 0x00_A000, 0x00_FF00);
        assert_eq!(st.depth(), 3);
        st.record_event(EventKind::Rti, 0x00_FF10, 0x00_A000);
        st.record_event(EventKind::Return, 0x00_A010, 0x00_9000);
        assert_eq!(st.depth(), 1);
        let events = st.events();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].depth, 1);
        assert_eq!(events[2].depth, 3);
        assert_eq!(events[4].depth, 1);
    }

    /// Arming mid-execution means the first `Return` has no matching `Call`; depth must floor at
    /// zero rather than wrap to 65535 and make every later row look impossibly deep.
    #[test]
    fn depth_saturates_instead_of_wrapping() {
        let mut st = TraceState::default();
        st.set_tracing(true);
        st.record_event(EventKind::Return, 0x00_8000, 0x00_7000);
        assert_eq!(st.depth(), 0);
        st.record_event(EventKind::Rti, 0x00_8000, 0x00_7000);
        assert_eq!(st.depth(), 0);
    }

    /// The low-RAM mirror and the `$7E` window are the SAME bytes, so they must share a counter —
    /// two separate slots would show one hot address as two cold ones.
    #[test]
    fn low_ram_mirror_shares_a_counter_with_the_linear_window() {
        assert_eq!(heatmap_index(0x00_0042), heatmap_index(0x7E_0042));
        assert_eq!(heatmap_index(0xB0_1FFF), heatmap_index(0x7E_1FFF));
        // $2000+ in a mirror bank is I/O, not WRAM.
        assert_eq!(heatmap_index(0x00_2100), None);
        // Cart space is not WRAM.
        assert_eq!(heatmap_index(0x00_8000), None);
        // The upper half of the linear window.
        assert_eq!(heatmap_index(0x7F_FFFF), Some(0x1_FFFF));

        let mut st = TraceState::default();
        st.set_counting(true);
        st.note_access(0x00_0042, true);
        st.note_access(0x7E_0042, false);
        let counts = st.counts(0x7E_0042, 1);
        assert_eq!(counts[0].reads, 1);
        assert_eq!(counts[0].writes, 1);
        assert_eq!(counts[0].total(), 2);
        assert_eq!(st.peak_count(), 2);
    }

    /// Counting is armed separately from tracing, and reads outside WRAM are simply not counted.
    #[test]
    fn counting_is_independent_and_wram_only() {
        let mut st = TraceState::default();
        st.set_counting(true);
        assert!(!st.is_tracing());
        st.note_access(0x00_8000, false); // cart ROM
        st.note_access(0x00_2100, true); // PPU register
        assert_eq!(st.peak_count(), 0);
        st.record_step(entry(0x00_8000));
        assert!(st.entries().is_empty(), "counting must not imply tracing");

        st.note_access(0x7E_1000, false);
        assert_eq!(st.counts(0x7E_1000, 1)[0].reads, 1);
        st.clear_counts();
        assert_eq!(st.peak_count(), 0);
        // The allocation survives a clear, so re-arming does not re-allocate.
        st.note_access(0x7E_1000, false);
        assert_eq!(st.peak_count(), 1);
    }

    /// A window straddling the end of WRAM reads zero past the edge rather than panicking or
    /// wrapping to the start of the map.
    #[test]
    fn counts_past_the_end_of_wram_read_zero() {
        let mut st = TraceState::default();
        st.set_counting(true);
        st.note_access(0x7F_FFFF, true);
        let counts = st.counts(0x7F_FFFE, 4);
        assert_eq!(counts.len(), 4);
        assert!(counts[0].is_zero());
        assert_eq!(counts[1].writes, 1);
        // $800000 is not WRAM.
        assert!(counts[2].is_zero() && counts[3].is_zero());
    }
}
