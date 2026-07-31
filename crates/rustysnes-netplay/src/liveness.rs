//! Peer liveness, round-trip time, and connection timeouts (`v1.27.0`).
//!
//! Before this, `rustysnes-netplay` had **no liveness handling at all**: no clock anywhere in the
//! crate, no handshake timeout (an absent `Sync` stalled `advance()` forever), and
//! [`crate::message::NetMessage::Quality`] — which carries the peer's ping and
//! frame advantage — was received and explicitly discarded. A peer that unplugged its cable simply
//! stopped producing frames, with nothing to say why.
//!
//! # Why this is a `Transport` decorator
//!
//! Determinism (`docs/adr/0004`) is the hard contract: [`RollbackSession`] must be a pure function
//! of its input stream so rollback re-simulation reproduces a zero-latency reference run
//! bit-identically. A clock inside the session would break that outright.
//!
//! So the clock lives *outside* it. [`LivenessTransport`] wraps any [`Transport`] and timestamps
//! what passes through, which means the session sees a plain `Transport`, stays clock-free, and
//! `tests/determinism.rs` keeps proving what it proved before. The frontend constructs the
//! decorator and reads the liveness view off it.
//!
//! # Why the clock is injected
//!
//! [`Clock`] is a trait, not a call to `Instant::now()`. Timeout behaviour tested against the wall
//! clock means `thread::sleep` in tests: slow, and flaky under CI load precisely because the
//! thresholds being tested are short. With [`ManualClock`] the whole state machine is driven
//! instantly and deterministically — a 5-second peer timeout is tested in microseconds, and the
//! test cannot fail because a runner was busy.
//!
//! [`RollbackSession`]: crate::session::RollbackSession

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::message::NetMessage;
use crate::transport::Transport;

/// A source of monotonic time.
///
/// Only *differences* between readings are ever used, so an implementation need not agree with any
/// particular epoch.
pub trait Clock {
    /// The current instant.
    fn now(&self) -> Instant;
}

/// The real clock — [`Instant::now`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A clock that only moves when told to.
///
/// This is what lets the liveness state machine be tested exhaustively and instantly instead of
/// with `thread::sleep` against tight thresholds. It is `pub` rather than `#[cfg(test)]` on
/// purpose: a frontend integration test outside this crate needs it too.
///
/// The offset is an [`AtomicU64`] of nanoseconds rather than a `Cell`, which keeps `ManualClock`
/// `Sync` — a `Cell` would make `Arc<ManualClock>` fail to compile, and a frontend test driving a
/// session from another thread is exactly the case this type exists for.
#[derive(Debug)]
pub struct ManualClock {
    base: Instant,
    offset_nanos: AtomicU64,
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ManualClock {
    /// A clock parked at an arbitrary origin.
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: Instant::now(),
            offset_nanos: AtomicU64::new(0),
        }
    }

    /// Move time forward by `d`.
    pub fn advance(&self, d: Duration) {
        let add = u64::try_from(d.as_nanos()).unwrap_or(u64::MAX);
        self.offset_nanos.fetch_add(add, Ordering::Relaxed);
    }

    /// Move time forward by `ms` milliseconds.
    pub fn advance_ms(&self, ms: u64) {
        self.advance(Duration::from_millis(ms));
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        self.base + Duration::from_nanos(self.offset_nanos.load(Ordering::Relaxed))
    }
}

/// How alive the remote peer looks right now.
///
/// Graded rather than boolean, and the grading is the point. Mesen's netplay uses a roughly 150 ms
/// trigger, which flags ordinary Wi-Fi and LTE jitter as a disconnect — a connection that reports
/// "lost" every time a packet is late trains the user to ignore it. The thresholds here are chosen
/// so a **single lost ping can never** move the grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerLink {
    /// Traffic is arriving within the expected interval.
    Live,
    /// Nothing has arrived for longer than [`LivenessConfig::interrupt_after`] — probably a hiccup,
    /// worth surfacing, not worth tearing anything down for.
    Interrupted,
    /// Nothing for longer than [`LivenessConfig::disconnect_after`] — the peer is gone.
    TimedOut,
}

/// Why a connection ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DisconnectReason {
    /// The peer's `Sync` never arrived within [`LivenessConfig::handshake_timeout`].
    #[error("handshake timed out — no peer responded")]
    HandshakeTimeout,
    /// The peer stopped sending for [`LivenessConfig::disconnect_after`].
    #[error("peer timed out — no traffic received")]
    PeerTimeout,
}

/// Default weight of a new RTT sample in the smoothing average.
///
/// Also the fallback when a supplied weight is NaN, which `f64::clamp` would otherwise propagate.
pub const DEFAULT_PING_SMOOTHING: f64 = 0.2;

/// Timing policy for [`LivenessTransport`].
#[derive(Debug, Clone, Copy)]
pub struct LivenessConfig {
    /// How often to emit a [`NetMessage::Quality`] ping.
    pub ping_interval: Duration,
    /// Silence after which the peer grades to [`PeerLink::Interrupted`].
    pub interrupt_after: Duration,
    /// Silence after which the peer grades to [`PeerLink::TimedOut`].
    pub disconnect_after: Duration,
    /// How long to wait for the peer's first message before giving up on the handshake.
    pub handshake_timeout: Duration,
    /// Weight of a new RTT sample in the exponentially-weighted moving average, `0.0..=1.0`.
    ///
    /// Low, because the number is shown to a human: an unsmoothed ping readout flickers with every
    /// packet and is unreadable, while a smoothed one still tracks a genuine route change within a
    /// few samples.
    pub ping_smoothing: f64,
}

impl Default for LivenessConfig {
    /// Deliberately forgiving, and each number is a decision rather than a round figure:
    ///
    /// * **1 s ping** — frequent enough that two intervals is still a short wall-clock window.
    /// * **2 s interrupt** — two full ping intervals plus slack, so one lost ping cannot trip it.
    /// * **5 s disconnect** — in the range GGPO and Parsec use for their grace windows; long
    ///   enough to survive a Wi-Fi roam, short enough that a truly dead peer is not waited on.
    /// * **10 s handshake** — a peer that has not answered in ten seconds is not coming.
    fn default() -> Self {
        Self {
            ping_interval: Duration::from_secs(1),
            interrupt_after: Duration::from_secs(2),
            disconnect_after: Duration::from_secs(5),
            handshake_timeout: Duration::from_secs(10),
            ping_smoothing: DEFAULT_PING_SMOOTHING,
        }
    }
}

/// A [`Transport`] decorator that tracks peer liveness and round-trip time.
///
/// Wrap the real transport in this, hand it to the session, and read the liveness view here. The
/// session is unaware it exists and stays clock-free — see the module doc.
#[derive(Debug)]
pub struct LivenessTransport<T: Transport, C: Clock = SystemClock> {
    inner: T,
    clock: C,
    config: LivenessConfig,
    /// When the decorator was created — the handshake deadline is measured from here.
    started: Instant,
    /// When anything last arrived from the peer.
    last_recv: Option<Instant>,
    /// When the outstanding ping was emitted, and hence what an echo is measured against.
    /// `None` means no ping is in flight — an arriving echo is then ignored rather than measured.
    ping_sent_at: Option<Instant>,
    /// When a ping was last *issued*, answered or not — paces the cadence independently of whether
    /// one is still outstanding, so an abandoned ping does not trigger an immediate resend.
    last_ping_issued: Option<Instant>,
    /// How many abandoned pings may still have a reply in flight.
    ///
    /// `Quality` carries no sequence number, so an echo cannot be matched to a specific ping — only
    /// to "the outstanding one". Once a ping is abandoned its reply may still arrive and would
    /// otherwise be credited to whatever ping is outstanding *then*, reading as a near-zero round
    /// trip. Each abandoned ping is counted here and swallows exactly one incoming echo before
    /// measurement resumes.
    orphaned_echoes: u32,
    /// Smoothed round-trip estimate.
    smoothed_ping_ms: Option<f64>,
    /// The peer's most recently reported frame advantage.
    remote_frame_advantage: i32,
    /// Set once a disconnect verdict has been reached; sticky, because a connection does not come
    /// back on its own and a caller polling after teardown must keep getting the same answer.
    disconnected: Option<DisconnectReason>,
}

impl<T: Transport> LivenessTransport<T, SystemClock> {
    /// Wrap `inner` with the default policy on the real clock.
    #[must_use]
    pub fn new(inner: T) -> Self {
        Self::with_clock_and_config(inner, SystemClock, LivenessConfig::default())
    }
}

impl<T: Transport, C: Clock> LivenessTransport<T, C> {
    /// Wrap `inner` with an explicit clock and policy.
    #[must_use]
    pub fn with_clock_and_config(inner: T, clock: C, config: LivenessConfig) -> Self {
        let started = clock.now();
        Self {
            inner,
            clock,
            config,
            started,
            last_recv: None,
            ping_sent_at: None,
            last_ping_issued: None,
            orphaned_echoes: 0,
            smoothed_ping_ms: None,
            remote_frame_advantage: 0,
            disconnected: None,
        }
    }

    /// The wrapped transport.
    pub const fn inner(&self) -> &T {
        &self.inner
    }

    /// Emit a [`NetMessage::Quality`] ping if one is due, and re-evaluate the liveness verdict.
    ///
    /// Call once per frame from the same loop that drives the session. Returns the current
    /// [`PeerLink`]. This is the only method that *acts* on time; [`Self::peer_link`] and friends
    /// merely report.
    pub fn tick(&mut self, local_frame_advantage: i32) -> PeerLink {
        let now = self.clock.now();

        // Handshake deadline: nothing has EVER arrived and the window has closed.
        if self.last_recv.is_none()
            && self.disconnected.is_none()
            && now.saturating_duration_since(self.started) >= self.config.handshake_timeout
        {
            self.disconnected = Some(DisconnectReason::HandshakeTimeout);
        }

        // Only ONE ping may be outstanding at a time.
        //
        // `Quality` carries no sequence number, so an echo can only be matched to a ping by "the
        // one that is outstanding". Sending a second while the first is unanswered breaks that: the
        // late reply to the FIRST ping gets measured against the SECOND ping's send time and
        // records a near-zero RTT that never happened. Raised in review, and real — it is exactly
        // the case where the readout matters most, a slow link.
        //
        // So a ping is issued only when none is in flight. If one has been outstanding longer than
        // `interrupt_after` it is abandoned rather than replaced: the marker is cleared, so a reply
        // arriving later finds nothing to measure against and is ignored instead of producing a
        // fabricated sample. Ping cadence therefore drops on a lossy link, which costs nothing —
        // liveness is refreshed by ANY traffic, not by pings.
        if self.disconnected.is_none() {
            let outstanding_for = self
                .ping_sent_at
                .map(|sent| now.saturating_duration_since(sent));
            if outstanding_for.is_some_and(|d| d >= self.config.interrupt_after) {
                self.ping_sent_at = None;
                // Its reply may still be in flight; count it so it is swallowed rather than
                // credited to the next ping.
                self.orphaned_echoes = self.orphaned_echoes.saturating_add(1);
            }
            let due = self.ping_sent_at.is_none()
                && self.last_ping_issued.is_none_or(|at| {
                    now.saturating_duration_since(at) >= self.config.ping_interval
                });
            if due {
                // The ping doubles as the echo request: the peer's own `Quality` reply is what
                // closes the round trip. Nothing extra goes on the wire for RTT.
                self.inner.send(&NetMessage::Quality {
                    ping_ms: self.ping_ms().unwrap_or(0),
                    frame_advantage: local_frame_advantage,
                });
                self.ping_sent_at = Some(now);
                self.last_ping_issued = Some(now);
            }
        }

        let link = self.peer_link();
        if link == PeerLink::TimedOut && self.disconnected.is_none() {
            self.disconnected = Some(DisconnectReason::PeerTimeout);
        }
        link
    }

    /// The current liveness grade.
    #[must_use]
    pub fn peer_link(&self) -> PeerLink {
        if self.disconnected.is_some() {
            return PeerLink::TimedOut;
        }
        let Some(last) = self.last_recv else {
            // Nothing has arrived yet. That is the handshake's problem, not the liveness grade's —
            // reporting `TimedOut` here would make a connection look dead during normal startup.
            return PeerLink::Live;
        };
        let silent = self.clock.now().saturating_duration_since(last);
        if silent >= self.config.disconnect_after {
            PeerLink::TimedOut
        } else if silent >= self.config.interrupt_after {
            PeerLink::Interrupted
        } else {
            PeerLink::Live
        }
    }

    /// Why the connection ended, if it has.
    #[must_use]
    pub const fn disconnect_reason(&self) -> Option<DisconnectReason> {
        self.disconnected
    }

    /// Smoothed round-trip time in milliseconds, once a ping has been echoed.
    #[must_use]
    pub fn ping_ms(&self) -> Option<u32> {
        self.smoothed_ping_ms.map(|ms| {
            // Rounded through `u64` rather than cast straight from `f64`: a direct `as u32` is
            // saturating but the pedantic cast lints (rightly) object to it, and NaN would silently
            // become 0. Clamping first makes the conversion total and the intent explicit.
            // Clamped to `u32`'s range and NaN-checked FIRST, which is what makes the cast
            // below total — the pedantic cast lints cannot see that and would otherwise object to
            // a conversion that provably cannot truncate or lose a sign.
            let clamped = ms.clamp(0.0, f64::from(u32::MAX));
            if clamped.is_nan() {
                0
            } else {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped to 0..=u32::MAX and NaN-checked immediately above"
                )]
                {
                    clamped.round() as u32
                }
            }
        })
    }

    /// The peer's most recently reported frame advantage (how far ahead it believes it is).
    #[must_use]
    pub const fn remote_frame_advantage(&self) -> i32 {
        self.remote_frame_advantage
    }

    /// The policy in effect.
    #[must_use]
    pub const fn config(&self) -> &LivenessConfig {
        &self.config
    }

    /// Fold a fresh round-trip sample into the smoothed estimate.
    fn record_rtt(&mut self, rtt: Duration) {
        let sample = rtt.as_secs_f64() * 1000.0;
        // `f64::clamp` returns NaN for a NaN input rather than clamping it, so a `ping_smoothing`
        // of NaN would poison every future sample. Raised in review. Fall back to the default
        // weight rather than silently disabling smoothing.
        let w = if self.config.ping_smoothing.is_nan() {
            DEFAULT_PING_SMOOTHING
        } else {
            self.config.ping_smoothing.clamp(0.0, 1.0)
        };
        self.smoothed_ping_ms = Some(
            self.smoothed_ping_ms
                .map_or(sample, |prev| prev.mul_add(1.0 - w, sample * w)),
        );
    }
}

impl<T: Transport, C: Clock> Transport for LivenessTransport<T, C> {
    fn send(&mut self, msg: &NetMessage) {
        self.inner.send(msg);
    }

    fn poll(&mut self) -> Vec<NetMessage> {
        let msgs = self.inner.poll();
        if msgs.is_empty() {
            return msgs;
        }
        let now = self.clock.now();
        // ANY traffic proves the peer is alive — the liveness clock is refreshed by receipt, not
        // by message kind. Gating it on `Quality` alone would let a peer that is streaming input
        // perfectly grade as Interrupted simply because its pings were the packets that dropped.
        self.last_recv = Some(now);

        for m in &msgs {
            if let NetMessage::Quality {
                frame_advantage, ..
            } = m
            {
                self.remote_frame_advantage = *frame_advantage;
                // Swallow one echo per abandoned ping before measuring anything: that reply
                // belongs to a ping we gave up on, and crediting it to the current one would read
                // as a near-zero round trip.
                if self.orphaned_echoes > 0 {
                    self.orphaned_echoes -= 1;
                    continue;
                }
                // The peer's `Quality` closes the round trip our own ping opened. Take the sample
                // once and clear the marker: a second echo for the same ping (a duplicated
                // datagram) would otherwise measure from a stale send time and inflate the
                // estimate.
                if let Some(sent) = self.ping_sent_at.take() {
                    self.record_rtt(now.saturating_duration_since(sent));
                }
            }
        }
        msgs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MemoryTransport;

    /// A transport that records what was sent and replays a scripted inbox.
    #[derive(Default)]
    struct FakeTransport {
        sent: Vec<NetMessage>,
        inbox: Vec<NetMessage>,
    }

    impl Transport for FakeTransport {
        fn send(&mut self, msg: &NetMessage) {
            self.sent.push(msg.clone());
        }
        fn poll(&mut self) -> Vec<NetMessage> {
            std::mem::take(&mut self.inbox)
        }
    }

    fn tight() -> LivenessConfig {
        LivenessConfig {
            ping_interval: Duration::from_millis(100),
            interrupt_after: Duration::from_millis(200),
            disconnect_after: Duration::from_millis(500),
            handshake_timeout: Duration::from_secs(1),
            ping_smoothing: 0.2,
        }
    }

    fn harness() -> (LivenessTransport<FakeTransport, ManualClock>, ()) {
        (
            LivenessTransport::with_clock_and_config(
                FakeTransport::default(),
                ManualClock::new(),
                tight(),
            ),
            (),
        )
    }

    /// Deliver one message to the decorator's inbox and poll it through.
    fn deliver(t: &mut LivenessTransport<FakeTransport, ManualClock>, msg: NetMessage) {
        t.inner.inbox.push(msg);
        let _ = t.poll();
    }

    fn input(frame: u32) -> NetMessage {
        NetMessage::Input {
            player: 1,
            frame,
            input: 0,
        }
    }

    #[test]
    fn a_fresh_connection_is_live_not_timed_out() {
        // Nothing has arrived yet, but that is the handshake's business. Grading a starting
        // connection `TimedOut` would make normal startup look like a failure.
        let (t, ()) = harness();
        assert_eq!(t.peer_link(), PeerLink::Live);
        assert_eq!(t.disconnect_reason(), None);
        assert_eq!(t.ping_ms(), None);
    }

    #[test]
    fn silence_grades_live_then_interrupted_then_timed_out() {
        let (mut t, ()) = harness();
        deliver(&mut t, input(0));
        assert_eq!(t.peer_link(), PeerLink::Live);

        t.clock.advance_ms(150);
        assert_eq!(
            t.peer_link(),
            PeerLink::Live,
            "still inside the interrupt window"
        );

        t.clock.advance_ms(100); // 250ms total, past interrupt_after=200
        assert_eq!(t.peer_link(), PeerLink::Interrupted);

        t.clock.advance_ms(300); // 550ms total, past disconnect_after=500
        assert_eq!(t.peer_link(), PeerLink::TimedOut);
        assert_eq!(t.tick(0), PeerLink::TimedOut);
        assert_eq!(t.disconnect_reason(), Some(DisconnectReason::PeerTimeout));
    }

    #[test]
    fn a_single_lost_ping_cannot_grade_the_peer() {
        // The whole reason `interrupt_after` is two ping intervals plus slack. Mesen's ~150ms
        // trigger flags ordinary Wi-Fi jitter; this must not.
        let (mut t, ()) = harness();
        deliver(&mut t, input(0));
        // One ping interval passes with nothing arriving — exactly one lost packet's worth.
        t.clock.advance(t.config.ping_interval);
        assert_eq!(
            t.peer_link(),
            PeerLink::Live,
            "one missed interval must not move the grade"
        );
    }

    #[test]
    fn any_traffic_refreshes_liveness_not_just_pings() {
        // Gating on `Quality` alone would let a peer streaming input perfectly grade as
        // Interrupted just because its pings were the packets that dropped.
        let (mut t, ()) = harness();
        deliver(&mut t, input(0));
        for i in 1..10 {
            t.clock.advance_ms(150);
            deliver(&mut t, input(i));
            assert_eq!(t.peer_link(), PeerLink::Live);
        }
    }

    #[test]
    fn the_handshake_times_out_only_when_nothing_ever_arrives() {
        let (mut t, ()) = harness();
        t.clock.advance_ms(1001);
        assert_eq!(t.tick(0), PeerLink::TimedOut);
        assert_eq!(
            t.disconnect_reason(),
            Some(DisconnectReason::HandshakeTimeout),
            "never-connected must be distinguishable from went-away"
        );
    }

    #[test]
    fn a_peer_that_answered_once_times_out_as_peer_not_handshake() {
        // The negative control for the test above: the two reasons must not be confusable, or a
        // user cannot tell "wrong address" from "your friend's Wi-Fi died".
        let (mut t, ()) = harness();
        deliver(&mut t, input(0));
        t.clock.advance_ms(2000);
        let _ = t.tick(0);
        assert_eq!(t.disconnect_reason(), Some(DisconnectReason::PeerTimeout));
    }

    #[test]
    fn rtt_is_measured_from_our_ping_to_the_peers_echo() {
        let (mut t, ()) = harness();
        // `tick` emits the ping (none sent yet, so one is due immediately).
        t.tick(0);
        assert_eq!(t.inner.sent.len(), 1);
        assert!(matches!(t.inner.sent[0], NetMessage::Quality { .. }));

        t.clock.advance_ms(40);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 3,
            },
        );
        assert_eq!(t.ping_ms(), Some(40), "first sample is taken unsmoothed");
        assert_eq!(t.remote_frame_advantage(), 3);
    }

    #[test]
    fn rtt_is_smoothed_so_the_readout_does_not_flicker() {
        let (mut t, ()) = harness();
        t.tick(0);
        t.clock.advance_ms(40);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );
        assert_eq!(t.ping_ms(), Some(40));

        // A single 240ms spike must move the estimate a little, not to 240.
        t.clock.advance(t.config.ping_interval);
        t.tick(0);
        t.clock.advance_ms(240);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );
        let smoothed = t.ping_ms().expect("a sample exists");
        assert!(
            (60..=100).contains(&smoothed),
            "one spike must not become the readout; got {smoothed}"
        );
    }

    #[test]
    fn a_duplicated_echo_does_not_inflate_the_estimate() {
        // A duplicated datagram would otherwise be measured against a stale send time, reporting
        // an RTT that never happened.
        let (mut t, ()) = harness();
        t.tick(0);
        t.clock.advance_ms(40);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );
        let first = t.ping_ms().expect("a sample");

        t.clock.advance_ms(400);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );
        assert_eq!(
            t.ping_ms(),
            Some(first),
            "the second echo for the same ping must be ignored"
        );
    }

    #[test]
    fn a_late_echo_cannot_record_a_fabricated_near_zero_rtt() {
        // Raised in review, and real. `Quality` carries no sequence number, so an echo is matched
        // to "the outstanding ping". Sending a second ping while the first is unanswered would
        // measure the FIRST ping's late reply against the SECOND ping's send time — a near-zero
        // RTT that never happened, on exactly the slow link where the readout matters most.
        let (mut t, ()) = harness();
        t.tick(0);
        assert_eq!(t.inner.sent.len(), 1, "one ping issued");

        // A full ping interval passes with no echo. A second ping must NOT go out while the first
        // is still outstanding and still inside the abandon window.
        t.clock.advance(t.config.ping_interval);
        t.tick(0);
        assert_eq!(
            t.inner.sent.len(),
            1,
            "no second ping may be in flight alongside the first"
        );

        // The first ping's reply finally lands, 150ms after it was sent.
        t.clock.advance_ms(50);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );
        assert_eq!(
            t.ping_ms(),
            Some(150),
            "the sample must be measured against the ping it actually answers"
        );
    }

    #[test]
    fn an_abandoned_ping_is_not_measured_when_its_reply_finally_arrives() {
        // The other half: once a ping has been outstanding past `interrupt_after` it is abandoned
        // rather than replaced, so a much-later reply finds no marker and produces no sample at
        // all — better than a fabricated one.
        let (mut t, ()) = harness();
        t.tick(0);
        t.clock
            .advance(t.config.interrupt_after + Duration::from_millis(10));
        t.tick(0); // abandons the stale ping and issues a fresh one

        // The ORIGINAL ping's reply arrives immediately after the fresh ping went out. It must not
        // be credited to the fresh ping as a ~0ms round trip.
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );
        let measured = t.ping_ms();
        assert!(
            measured.is_none_or(|ms| ms > 0),
            "a reply credited to the wrong ping would read as ~0ms; got {measured:?}"
        );
    }

    #[test]
    fn a_nan_smoothing_weight_does_not_poison_the_estimate() {
        // `f64::clamp` returns NaN for a NaN input rather than clamping it, so a NaN weight would
        // propagate into every future sample and the readout would never recover.
        let mut cfg = tight();
        cfg.ping_smoothing = f64::NAN;
        let mut t = LivenessTransport::with_clock_and_config(
            FakeTransport::default(),
            ManualClock::new(),
            cfg,
        );
        t.tick(0);
        t.clock.advance_ms(40);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );
        t.clock.advance(t.config.ping_interval);
        t.tick(0);
        t.clock.advance_ms(80);
        deliver(
            &mut t,
            NetMessage::Quality {
                ping_ms: 0,
                frame_advantage: 0,
            },
        );

        let ms = t.ping_ms().expect("a sample exists");
        assert!(
            (40..=80).contains(&ms),
            "a NaN weight must fall back to the default, not poison the estimate; got {ms}"
        );
    }

    #[test]
    fn the_manual_clock_is_sync_so_it_can_be_shared_across_threads() {
        // Raised in review: a `Cell` offset would make `Arc<ManualClock>` fail to compile, and a
        // frontend test driving a session from another thread is exactly what this type is for.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ManualClock>();

        let clock = std::sync::Arc::new(ManualClock::new());
        let c2 = std::sync::Arc::clone(&clock);
        let before = clock.now();
        std::thread::spawn(move || c2.advance_ms(500))
            .join()
            .expect("thread");
        assert!(clock.now().saturating_duration_since(before) >= Duration::from_millis(500));
    }

    #[test]
    fn the_decorator_passes_messages_through_unchanged() {
        // It must be transparent: the session sees exactly the stream the inner transport carries,
        // or wrapping it would change what the session simulates.
        let (a, mut b) = MemoryTransport::ideal_pair();
        let mut wrapped = LivenessTransport::with_clock_and_config(a, ManualClock::new(), tight());
        b.send(&input(7));
        b.send(&input(8));
        let got = wrapped.poll();
        assert_eq!(got, vec![input(7), input(8)]);
    }

    #[test]
    fn disconnection_is_sticky() {
        // A caller polling after teardown must keep getting the same answer; a connection does not
        // silently come back.
        let (mut t, ()) = harness();
        deliver(&mut t, input(0));
        t.clock.advance_ms(600);
        let _ = t.tick(0);
        assert_eq!(t.disconnect_reason(), Some(DisconnectReason::PeerTimeout));

        // Even if the peer starts talking again.
        deliver(&mut t, input(1));
        assert_eq!(t.peer_link(), PeerLink::TimedOut);
        assert_eq!(t.disconnect_reason(), Some(DisconnectReason::PeerTimeout));
    }
}
