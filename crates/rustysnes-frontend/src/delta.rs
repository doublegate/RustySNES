//! XOR-delta + RLE compression for the rewind ring (`v1.25.0`, T-FP-F).
//!
//! # Why this shape
//!
//! Successive save-states are **overwhelmingly identical**: between two frames a few hundred bytes
//! of WRAM, some PPU registers, and the CPU's registers change, out of hundreds of kilobytes. XOR
//! against the previous state turns that into a buffer that is almost entirely zero, which run-length
//! encodes to a few dozen bytes. That is a much better fit than a general-purpose compressor here
//! and needs **no new dependency** — a rewind buffer that pulls in a compression crate to store data
//! that is already 99% zeros is paying for the wrong tool.
//!
//! # Keyframes
//!
//! Every `keyframe_interval`-th snapshot is stored whole. Without them, rewinding to the oldest
//! state means replaying every delta from the start of the ring — and the ring evicts from the
//! front, so the base a delta chain depends on is exactly what disappears first. A keyframe bounds
//! both the replay cost and that dependency.
//!
//! # The contract
//!
//! A round-trip must restore **bit-identically**. Anything less is not a rewind: a state that is
//! nearly right resumes emulation from a machine that never existed, and the divergence appears
//! later as an unexplainable bug rather than as a failed decompress.

/// The RLE encodes runs of zero bytes and literal spans. A run length is a `u32` varint, so a
/// fully-unchanged 512 KiB state costs five bytes.
///
/// Two opcodes only — a general scheme with more would be a compressor, and the input here is
/// specifically "mostly zeros with short literal islands".
const OP_ZEROS: u8 = 0x00;
/// A literal span follows: a varint length, then that many bytes.
const OP_LITERAL: u8 = 0x01;

/// One stored rewind entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// A whole state, stored uncompressed.
    Key(Vec<u8>),
    /// An XOR delta against the previous frame's *reconstructed* state.
    Delta(Vec<u8>),
}

impl Frame {
    /// Bytes this entry occupies.
    #[must_use]
    pub const fn len(&self) -> usize {
        match self {
            Self::Key(b) | Self::Delta(b) => b.len(),
        }
    }

    /// Whether this entry holds nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether this is a keyframe (a chain can start here).
    #[must_use]
    pub const fn is_key(&self) -> bool {
        matches!(self, Self::Key(_))
    }
}

/// Encode `current` against `previous`, or as a keyframe when there is none.
///
/// The delta is against the previous **reconstructed** state, not the previous compressed bytes —
/// so a caller must keep the running reconstruction, which [`Chain`] does.
///
/// A length change forces a keyframe: two states of different sizes have no meaningful XOR, and
/// silently XOR-ing the common prefix would produce a delta that decodes to a truncated state.
#[must_use]
pub fn encode(current: &[u8], previous: Option<&[u8]>) -> Frame {
    match previous {
        Some(prev) if prev.len() == current.len() => {
            let mut xored = Vec::with_capacity(current.len());
            for (c, p) in current.iter().zip(prev.iter()) {
                xored.push(c ^ p);
            }
            Frame::Delta(rle_encode(&xored))
        }
        _ => Frame::Key(current.to_vec()),
    }
}

/// Decode one entry against the previous reconstructed state.
///
/// # Errors
/// Returns `None` when the entry is malformed or a delta arrives with no base — both of which mean
/// the chain cannot be reconstructed, and returning a partial state would be worse than failing.
#[must_use]
pub fn decode(frame: &Frame, previous: Option<&[u8]>) -> Option<Vec<u8>> {
    match frame {
        Frame::Key(bytes) => Some(bytes.clone()),
        Frame::Delta(bytes) => {
            let prev = previous?;
            let xored = rle_decode(bytes, prev.len())?;
            Some(xored.iter().zip(prev.iter()).map(|(x, p)| x ^ p).collect())
        }
    }
}

/// Run-length encode a mostly-zero buffer.
fn rle_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let start = i;
        if data[i] == 0 {
            while i < data.len() && data[i] == 0 {
                i += 1;
            }
            out.push(OP_ZEROS);
            write_varint(&mut out, (i - start) as u64);
        } else {
            // A literal span ends at the first zero *run* worth encoding, not at the first zero
            // byte: a single zero between two changed bytes costs 1 byte as a literal and at least
            // 3 as a run, so ending the span there would make the output larger than the input on
            // scattered changes.
            while i < data.len() && !(data[i] == 0 && zeros_from(data, i) >= MIN_RUN) {
                i += 1;
            }
            out.push(OP_LITERAL);
            write_varint(&mut out, (i - start) as u64);
            out.extend_from_slice(&data[start..i]);
        }
    }
    out
}

/// Shortest zero run worth emitting as a run rather than as literal bytes.
///
/// An `OP_ZEROS` plus a one-byte varint is 2 bytes, so a run of 3 is the first that saves anything.
const MIN_RUN: usize = 3;

/// How many consecutive zeros start at `i`.
fn zeros_from(data: &[u8], i: usize) -> usize {
    data[i..].iter().take_while(|b| **b == 0).count()
}

/// Decode an RLE stream back to exactly `expected` bytes.
///
/// Returns `None` on any inconsistency — a truncated stream, a length that overruns, or a final
/// length that does not match. A rewind that silently produced a short state would resume
/// emulation from a machine that never existed.
fn rle_decode(data: &[u8], expected: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(expected);
    let mut i = 0;
    while i < data.len() {
        let op = data[i];
        i += 1;
        let (len, consumed) = read_varint(&data[i..])?;
        i += consumed;
        let len = usize::try_from(len).ok()?;
        // Checked before allocating or copying: a corrupt length must not be able to ask for a
        // gigabyte, which is the difference between a rejected state and an OOM.
        if out.len().checked_add(len)? > expected {
            return None;
        }
        match op {
            OP_ZEROS => out.resize(out.len() + len, 0),
            OP_LITERAL => {
                let end = i.checked_add(len)?;
                out.extend_from_slice(data.get(i..end)?);
                i = end;
            }
            _ => return None,
        }
    }
    (out.len() == expected).then_some(out)
}

/// LEB128-style varint.
fn write_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = u8::try_from(v & 0x7F).unwrap_or(0);
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Read a varint, returning the value and how many bytes it used.
fn read_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    for (i, byte) in data.iter().enumerate() {
        // Checked before the shift, not after: `1u64 << 70` is a panic in debug and a wrong answer
        // in release, and this data can be corrupt.
        if shift >= 64 {
            return None;
        }
        // The last group a `u64` can hold starts at bit 63, so it has room for exactly ONE bit.
        // A wider group there is not representable: the shift itself is well-defined (Rust only
        // traps when the shift *amount* reaches the width, which the guard above prevents), so
        // without this the extra bits are discarded and a corrupt varint decodes to a plausible
        // wrong length instead of being refused.
        if shift == 63 && byte & 0x7F > 1 {
            return None;
        }
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
    }
    None
}

/// A compressed chain of states with periodic keyframes.
#[derive(Debug, Clone)]
pub struct Chain {
    frames: std::collections::VecDeque<Frame>,
    /// The reconstruction of the newest frame, kept so the next `push` can delta against it
    /// without walking the chain.
    newest: Option<Vec<u8>>,
    /// Store a whole state every this many pushes.
    keyframe_interval: usize,
    /// Pushes since the last keyframe.
    since_key: usize,
    /// Maximum entries retained.
    capacity: usize,
}

impl Chain {
    /// A chain holding at most `capacity` states, with a keyframe every `keyframe_interval`.
    ///
    /// `keyframe_interval` is clamped to at least 1, so a zero cannot produce a chain that never
    /// starts.
    #[must_use]
    pub fn new(capacity: usize, keyframe_interval: usize) -> Self {
        Self {
            frames: std::collections::VecDeque::new(),
            newest: None,
            keyframe_interval: keyframe_interval.max(1),
            since_key: 0,
            capacity,
        }
    }

    /// Entries stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether nothing is stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Total bytes the chain occupies, for the UI to report what compression bought.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.frames.iter().map(Frame::len).sum()
    }

    /// Discard everything.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.newest = None;
        self.since_key = 0;
    }

    /// Append a state.
    ///
    /// Eviction from the front can remove the keyframe a later delta depends on, so the frame after
    /// an eviction that orphaned the chain is promoted to a keyframe. Without that, the ring
    /// silently becomes undecodable from its own oldest entry.
    pub fn push(&mut self, state: &[u8]) {
        if self.capacity == 0 {
            return;
        }
        // `keyframe_interval = N` means one keyframe every N frames, so the (N-1)th delta since
        // the last key is the last one — `>= interval` would give N-1 deltas plus the key, i.e.
        // every N+1 frames, and an interval of 1 would never be all-keyframes.
        let force_key =
            self.frames.is_empty() || self.since_key >= self.keyframe_interval.saturating_sub(1);
        let frame = if force_key {
            self.since_key = 0;
            Frame::Key(state.to_vec())
        } else {
            self.since_key += 1;
            encode(state, self.newest.as_deref())
        };
        self.frames.push_back(frame);
        self.newest = Some(state.to_vec());
        while self.frames.len() > self.capacity {
            self.evict_front();
        }
    }

    /// Drop the oldest entry, promoting the new front to a keyframe if it needs one.
    ///
    /// The ring evicts from the front, which is exactly where a delta chain's base lives. Evicting
    /// a keyframe therefore orphans every delta that referenced it: `reconstruct` walks backward
    /// looking for a key and gives up when it runs off the front, so a plain `pop_front` silently
    /// turns the whole surviving prefix into states the buffer can no longer produce. Once the ring
    /// has wrapped that is *most* of the rewind history, and it presents as rewind simply refusing
    /// to step — not as an error anyone would trace back to here.
    ///
    /// So the new front is materialised **before** the old one is removed, while its base still
    /// exists, and stored back whole. That costs one reconstruction per eviction — and only when
    /// the frame being promoted is a delta — and it is what makes "the ring holds N states" mean N
    /// *recoverable* states. A greenzone test measured the alternative directly: a capacity of 4
    /// against a keyframe interval of 8 could restore exactly one of its four claimed states.
    fn evict_front(&mut self) {
        // Reconstruct index 1 BEFORE removing index 0, while its base is still present.
        let promoted = (self.frames.len() > 1 && !self.frames[1].is_key())
            .then(|| self.reconstruct(1))
            .flatten();
        let _ = self.frames.pop_front();
        if let Some(state) = promoted
            && let Some(front) = self.frames.front_mut()
        {
            *front = Frame::Key(state);
        }
    }

    /// Remove and reconstruct the newest state.
    ///
    /// Returns `None` when the chain is empty or cannot be decoded (an eviction removed the base a
    /// delta needs). Refusing is the point: a partially-reconstructed state resumes emulation from
    /// a machine that never existed.
    #[must_use]
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        let state = self.reconstruct(self.frames.len().checked_sub(1)?)?;
        let _ = self.frames.pop_back();
        // The new newest must be re-derived, or the next `push` would delta against a state that
        // is no longer at the end of the chain.
        self.newest = self
            .frames
            .len()
            .checked_sub(1)
            .and_then(|i| self.reconstruct(i));
        self.since_key = 0; // force a keyframe next, since the chain's tail just changed
        Some(state)
    }

    /// Reconstruct the state at `index` by replaying from the nearest preceding keyframe.
    fn reconstruct(&self, index: usize) -> Option<Vec<u8>> {
        let start = (0..=index).rev().find(|i| self.frames[*i].is_key())?;
        let mut state = decode(&self.frames[start], None)?;
        for frame in self.frames.iter().take(index + 1).skip(start + 1) {
            state = decode(frame, Some(&state))?;
        }
        Some(state)
    }
}

#[cfg(test)]
mod tests {
    use super::{Chain, Frame, decode, encode};

    /// A deterministic pseudo-state. 251 is prime, so the pattern does not align with any
    /// power-of-two block size and cannot compress better than real data would by accident.
    fn state(seed: u8, len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| seed.wrapping_add(u8::try_from(i % 251).unwrap_or(0)))
            .collect()
    }

    /// Every surviving frame stays reconstructable after the ring has wrapped many times.
    ///
    /// Evicting a keyframe orphans the deltas that referenced it — `reconstruct` walks backward for
    /// a key and gives up at the front — so a plain `pop_front` silently turns most of the rewind
    /// history into states the buffer can no longer produce. It presents as rewind refusing to
    /// step, which is a symptom several layers away from the cause.
    #[test]
    fn eviction_leaves_every_surviving_frame_reconstructable() {
        // Capacity deliberately NOT a multiple of the keyframe interval, so evictions land on
        // keyframes and deltas alike.
        let mut chain = Chain::new(7, 4);
        for seed in 0..60u8 {
            chain.push(&state(seed, 64));
        }
        // Drain the whole ring newest-first. Every frame must decode, and to the state that was
        // actually pushed — `pop` returning `None` partway through is the bug this pins.
        let held = chain.len();
        assert_eq!(held, 7);
        for back in 0..held {
            let want = state(59 - u8::try_from(back).expect("small"), 64);
            assert_eq!(
                chain.pop().as_deref(),
                Some(want.as_slice()),
                "frame {back} back from newest"
            );
        }
        assert!(chain.is_empty());
    }

    /// A capacity below the keyframe interval is the degenerate case of the above: nearly every
    /// eviction removes a key, so without promotion the buffer would be permanently undecodable.
    #[test]
    fn a_capacity_smaller_than_the_keyframe_interval_still_decodes() {
        let mut chain = Chain::new(3, 16);
        for seed in 0..40u8 {
            chain.push(&state(seed, 32));
        }
        assert_eq!(chain.len(), 3);
        for back in 0..3u8 {
            let want = state(39 - back, 32);
            assert_eq!(chain.pop().as_deref(), Some(want.as_slice()), "{back} back");
        }
    }

    /// A varint whose final group does not fit in a `u64` is refused rather than silently
    /// truncated. The shift itself is well-defined — Rust only traps when the shift AMOUNT reaches
    /// the width, which the `shift >= 64` guard already prevents — so without this check the extra
    /// bits are discarded and a corrupt stream decodes to a plausible wrong length.
    #[test]
    fn an_unrepresentable_varint_is_refused_not_truncated() {
        // Nine continuation groups of 7 bits reach shift 63; a final group of 1 fits.
        let mut ok = vec![0x80u8; 9];
        ok.push(0x01);
        assert!(super::read_varint(&ok).is_some());
        // The same stream with 2 in the last group needs bit 64 and cannot be represented.
        let mut bad = vec![0x80u8; 9];
        bad.push(0x02);
        assert_eq!(super::read_varint(&bad), None);
        // And a stream that never terminates is still refused rather than looping.
        assert_eq!(super::read_varint(&[0x80u8; 32]), None);
    }

    /// The contract: a round-trip is BIT-IDENTICAL. Anything less resumes emulation from a machine
    /// that never existed.
    #[test]
    fn round_trip_is_bit_identical() {
        let a = state(0, 4096);
        let mut b = a.clone();
        b[100] = 0xFF;
        b[2000] = 0x01;
        b[4095] = 0x7E;

        let key = encode(&a, None);
        assert!(key.is_key());
        assert_eq!(decode(&key, None).expect("key"), a);

        let delta = encode(&b, Some(&a));
        assert!(!delta.is_key());
        assert_eq!(decode(&delta, Some(&a)).expect("delta"), b);
    }

    /// The whole reason for XOR+RLE: successive states are overwhelmingly identical, so a delta
    /// must be a tiny fraction of a whole state.
    #[test]
    fn a_near_identical_state_compresses_hard() {
        let a = state(0, 262_144); // 256 KiB, a realistic save-state size
        let mut b = a.clone();
        for i in 0..200 {
            b[1000 + i] ^= 0xA5;
        }
        let delta = encode(&b, Some(&a));
        assert!(
            delta.len() < a.len() / 100,
            "delta was {} bytes for a {} byte state",
            delta.len(),
            a.len()
        );
        assert_eq!(decode(&delta, Some(&a)).expect("decode"), b);
    }

    /// An identical state costs almost nothing.
    #[test]
    fn an_unchanged_state_is_a_handful_of_bytes() {
        let a = state(7, 262_144);
        let delta = encode(&a, Some(&a));
        assert!(
            delta.len() <= 8,
            "unchanged delta was {} bytes",
            delta.len()
        );
        assert_eq!(decode(&delta, Some(&a)).expect("decode"), a);
    }

    /// A length change forces a keyframe: two states of different sizes have no meaningful XOR,
    /// and delta-ing the common prefix would decode to a truncated state.
    #[test]
    fn a_length_change_forces_a_keyframe() {
        let a = state(0, 100);
        let b = state(0, 200);
        assert!(encode(&b, Some(&a)).is_key());
        assert!(encode(&a, None).is_key());
    }

    /// Even a worst case — every byte changed — round-trips, and does not blow up disastrously.
    #[test]
    fn a_completely_changed_state_still_round_trips() {
        let a = vec![0x00u8; 8192];
        let b = vec![0xFFu8; 8192];
        let delta = encode(&b, Some(&a));
        assert_eq!(decode(&delta, Some(&a)).expect("decode"), b);
        // A literal span plus its header, not one opcode per byte.
        assert!(
            delta.len() < a.len() + 16,
            "worst case grew to {}",
            delta.len()
        );
    }

    /// Corrupt input is refused, never partially decoded.
    #[test]
    fn corrupt_deltas_are_refused() {
        let a = state(0, 64);
        let good = encode(&state(1, 64), Some(&a));
        let Frame::Delta(bytes) = good else {
            panic!("expected a delta")
        };
        // Truncated.
        assert!(decode(&Frame::Delta(bytes[..bytes.len() / 2].to_vec()), Some(&a)).is_none());
        // An unknown opcode.
        assert!(decode(&Frame::Delta(vec![0x7F, 0x01, 0x00]), Some(&a)).is_none());
        // A run longer than the state.
        assert!(decode(&Frame::Delta(vec![0x00, 0xFF, 0xFF, 0x03]), Some(&a)).is_none());
        // A delta with no base.
        assert!(decode(&Frame::Delta(vec![0x00, 0x40]), None).is_none());
    }

    /// A chain reconstructs every state it holds, exactly.
    #[test]
    fn a_chain_pops_states_in_reverse_order_exactly() {
        let mut chain = Chain::new(16, 4);
        let states: Vec<Vec<u8>> = (0..10u8).map(|i| state(i, 1024)).collect();
        for s in &states {
            chain.push(s);
        }
        assert_eq!(chain.len(), 10);
        for expected in states.iter().rev() {
            assert_eq!(&chain.pop().expect("pop"), expected);
        }
        assert!(chain.is_empty());
        assert!(chain.pop().is_none());
    }

    /// A keyframe every N bounds both the replay cost and the dependency on an evicted base.
    #[test]
    fn keyframes_appear_at_the_configured_interval() {
        let mut chain = Chain::new(32, 3);
        for i in 0..12u8 {
            chain.push(&state(i, 256));
        }
        let keys = chain.frames.iter().filter(|f| f.is_key()).count();
        assert!(keys >= 3, "expected periodic keyframes, found {keys}");
        // And every state still reconstructs.
        for i in (0..12u8).rev() {
            assert_eq!(chain.pop().expect("pop"), state(i, 256));
        }
    }

    /// Eviction must not leave the ring decodable-looking but wrong. Every surviving state either
    /// reconstructs exactly or is refused.
    #[test]
    fn eviction_never_yields_a_wrong_state() {
        let mut chain = Chain::new(5, 2);
        let states: Vec<Vec<u8>> = (0..20u8).map(|i| state(i, 512)).collect();
        for s in &states {
            chain.push(s);
        }
        assert_eq!(chain.len(), 5, "capacity is enforced");
        let mut popped = Vec::new();
        while let Some(s) = chain.pop() {
            popped.push(s);
        }
        // Whatever came back must be the genuine tail of what went in, in reverse.
        assert!(!popped.is_empty());
        for (i, got) in popped.iter().enumerate() {
            let expected = &states[states.len() - 1 - i];
            assert_eq!(got, expected, "state {i} from the end differs");
        }
    }

    /// Compression must actually buy something over storing states whole.
    #[test]
    fn a_chain_of_similar_states_is_far_smaller_than_the_raw_states() {
        let mut chain = Chain::new(64, 16);
        let base = state(0, 65_536);
        let mut raw = 0usize;
        for i in 0..64u8 {
            let mut s = base.clone();
            s[usize::from(i) * 7] ^= 0xFF;
            raw += s.len();
            chain.push(&s);
        }
        assert!(
            chain.bytes() < raw / 4,
            "compressed to {} of {raw} raw bytes",
            chain.bytes()
        );
    }

    /// After heavy eviction, EVERY entry the chain still claims to hold must be recoverable.
    ///
    /// This is the property `evict_front` exists for, and it was not true before: evicting a
    /// keyframe left the following deltas with no base, so the chain reported N entries and could
    /// restore only the newest few. A greenzone test caught it — a capacity of 4 against a
    /// keyframe interval of 8 restored exactly one of its four claimed states.
    #[test]
    fn every_claimed_entry_survives_eviction() {
        // A keyframe interval LARGER than the capacity is the worst case: without promotion the
        // keyframe is always the first thing evicted.
        for (capacity, keyframes) in [(4usize, 8usize), (3, 16), (8, 2), (5, 5)] {
            let mut chain = Chain::new(capacity, keyframes);
            let total = 60u8;
            for i in 0..total {
                chain.push(&state(i, 1024));
            }
            let claimed = chain.len();
            assert_eq!(claimed, capacity, "capacity {capacity}/{keyframes}");
            let mut recovered = 0usize;
            let mut expected = usize::from(total) - 1;
            while let Some(got) = chain.pop() {
                assert_eq!(
                    got,
                    state(u8::try_from(expected).expect("fits"), 1024),
                    "capacity {capacity}/{keyframes}: entry for frame {expected} is wrong"
                );
                recovered += 1;
                expected = expected.wrapping_sub(1);
            }
            assert_eq!(
                recovered, claimed,
                "capacity {capacity}/{keyframes}: claimed {claimed} but recovered {recovered}"
            );
        }
    }

    /// A zero-capacity chain is a permanent no-op — the additive-default-off posture.
    #[test]
    fn zero_capacity_never_records() {
        let mut chain = Chain::new(0, 4);
        chain.push(&state(0, 128));
        assert!(chain.is_empty());
        assert!(chain.pop().is_none());
        assert_eq!(chain.bytes(), 0);
    }

    /// A zero interval must not produce a chain that never keyframes.
    #[test]
    fn a_zero_keyframe_interval_is_clamped() {
        let mut chain = Chain::new(4, 0);
        for i in 0..4u8 {
            chain.push(&state(i, 64));
        }
        assert!(chain.frames.iter().all(Frame::is_key));
        for i in (0..4u8).rev() {
            assert_eq!(chain.pop().expect("pop"), state(i, 64));
        }
    }

    #[test]
    fn clear_resets_everything() {
        let mut chain = Chain::new(8, 4);
        chain.push(&state(0, 128));
        chain.push(&state(1, 128));
        chain.clear();
        assert!(chain.is_empty() && chain.bytes() == 0);
        // And it still works afterward.
        chain.push(&state(2, 128));
        assert_eq!(chain.pop().expect("pop"), state(2, 128));
    }
}
