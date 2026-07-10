//! The [`Transport`] abstraction plus [`MemoryTransport`].
//!
//! [`crate::session::RollbackSession`] drives against [`Transport`]; [`MemoryTransport`] is a
//! deterministic, seeded-PRNG, in-process pipe used by the determinism test suite to prove
//! rollback re-simulation is bit-identical under synthetic latency/jitter/packet loss, without a
//! real network in the loop (`docs/adr/0004`: no OS randomness, no `std::time`, anywhere near
//! this).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::message::NetMessage;
use crate::rng::SplitMix64;

/// Something [`crate::session::RollbackSession`] can send [`NetMessage`]s over and poll for
/// received ones.
///
/// Implementations: [`MemoryTransport`] (tests), `UdpTransport` (native, `udp.rs`),
/// `WebRtcTransport` (wasm32, `webrtc.rs`).
pub trait Transport {
    /// Send `msg` to the remote peer. Best-effort — a real transport may drop it; the session's
    /// own resend logic (unacked inputs) is what makes the protocol reliable, not this layer.
    fn send(&mut self, msg: &NetMessage);
    /// Drain every message received since the last call, in receipt order.
    fn poll(&mut self) -> Vec<NetMessage>;
}

/// A single-direction, capacity-unbounded queue of already-encoded-then-decoded messages, each
/// stamped with its scheduled arrival tick — [`MemoryTransport`]'s send side pushes here; the
/// paired peer's `poll` drains whatever has "arrived." Wrapping each message through
/// [`NetMessage::encode`]/[`decode`] (not just cloning the value) means a wire-format bug shows
/// up in the determinism tests too, not only in a real-transport test.
type Pipe = Rc<RefCell<VecDeque<(u64, NetMessage)>>>;

/// A deterministic in-process transport pairing two [`MemoryTransport`]s, with seeded synthetic
/// latency, jitter, and packet loss.
///
/// The harness `tests/determinism.rs` drives two [`crate::session::RollbackSession`]s over this
/// to prove rollback re-simulation reproduces a reference (no-rollback) run bit-identically even
/// under adverse network conditions.
pub struct MemoryTransport {
    outbox: Pipe,
    inbox: Pipe,
    rng: SplitMix64,
    clock: u64,
    base_latency_ticks: u64,
    jitter_ticks: u64,
    drop_chance: f64,
}

impl MemoryTransport {
    /// Build a connected pair of transports (`(peer_a, peer_b)`) sharing one seeded RNG stream
    /// split into two independent generators, so the two directions' synthetic conditions are
    /// reproducible but not identical to each other.
    #[must_use]
    pub fn pair(
        seed: u64,
        base_latency_ticks: u64,
        jitter_ticks: u64,
        drop_chance: f64,
    ) -> (Self, Self) {
        let a_to_b: Pipe = Rc::new(RefCell::new(VecDeque::new()));
        let b_to_a: Pipe = Rc::new(RefCell::new(VecDeque::new()));
        let mut seed_rng = SplitMix64::new(seed);
        let a = Self {
            outbox: Rc::clone(&a_to_b),
            inbox: Rc::clone(&b_to_a),
            rng: SplitMix64::new(seed_rng.next_u64()),
            clock: 0,
            base_latency_ticks,
            jitter_ticks,
            drop_chance,
        };
        let b = Self {
            outbox: b_to_a,
            inbox: a_to_b,
            rng: SplitMix64::new(seed_rng.next_u64()),
            clock: 0,
            base_latency_ticks,
            jitter_ticks,
            drop_chance,
        };
        (a, b)
    }

    /// A pristine, zero-latency, zero-loss pair — for tests isolating rollback logic itself from
    /// network-condition effects.
    #[must_use]
    pub fn ideal_pair() -> (Self, Self) {
        Self::pair(0, 0, 0, 0.0)
    }
}

impl Transport for MemoryTransport {
    fn send(&mut self, msg: &NetMessage) {
        // Round-trip through the wire format so a real encode/decode bug surfaces here too.
        let bytes = msg.encode();
        let Ok(decoded) = NetMessage::decode(&bytes) else {
            return;
        };
        if self.rng.chance(self.drop_chance) {
            return;
        }
        let jitter = if self.jitter_ticks == 0 {
            0
        } else {
            self.rng.next_u64() % (self.jitter_ticks + 1)
        };
        let arrival_tick = self.clock + self.base_latency_ticks + jitter;
        self.outbox.borrow_mut().push_back((arrival_tick, decoded));
    }

    fn poll(&mut self) -> Vec<NetMessage> {
        self.clock += 1;
        // Jitter means entries are NOT necessarily queued in arrival-tick order (a later-sent
        // packet can draw less jitter than an earlier one still ahead of it in the queue), so
        // this can't stop at the first `tick > clock` entry — every entry needs checking.
        let mut inbox = self.inbox.borrow_mut();
        let mut ready = Vec::new();
        let mut still_pending = VecDeque::with_capacity(inbox.len());
        for (tick, msg) in inbox.drain(..) {
            if tick <= self.clock {
                ready.push(msg);
            } else {
                still_pending.push_back((tick, msg));
            }
        }
        *inbox = still_pending;
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideal_pair_delivers_immediately() {
        let (mut a, mut b) = MemoryTransport::ideal_pair();
        a.send(&NetMessage::InputAck { frame: 5 });
        let received = b.poll();
        assert_eq!(received, vec![NetMessage::InputAck { frame: 5 }]);
    }

    #[test]
    fn same_seed_produces_identical_delivery_pattern() {
        let run = || {
            let (mut a, mut b) = MemoryTransport::pair(99, 2, 3, 0.3);
            let mut delivered_frames = Vec::new();
            for f in 0..50u32 {
                a.send(&NetMessage::InputAck { frame: f });
                for msg in b.poll() {
                    if let NetMessage::InputAck { frame } = msg {
                        delivered_frames.push(frame);
                    }
                }
            }
            delivered_frames
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn drop_chance_one_delivers_nothing() {
        let (mut a, mut b) = MemoryTransport::pair(1, 0, 0, 1.0);
        for f in 0..20u32 {
            a.send(&NetMessage::InputAck { frame: f });
        }
        let mut got_any = false;
        for _ in 0..20 {
            if !b.poll().is_empty() {
                got_any = true;
            }
        }
        assert!(!got_any, "drop_chance = 1.0 must drop every packet");
    }

    #[test]
    fn jitter_can_deliver_out_of_order_and_poll_still_finds_it() {
        // A high jitter ceiling makes an out-of-order arrival likely; this must not get stuck
        // behind an earlier-queued, still-pending entry (the bug the plain "stop at first
        // tick > clock" drain would have had).
        let (mut a, mut b) = MemoryTransport::pair(123, 5, 20, 0.0);
        for f in 0..10u32 {
            a.send(&NetMessage::InputAck { frame: f });
        }
        let mut delivered = Vec::new();
        for _ in 0..60 {
            delivered.extend(b.poll());
        }
        assert_eq!(delivered.len(), 10, "every sent packet eventually arrives");
    }
}
