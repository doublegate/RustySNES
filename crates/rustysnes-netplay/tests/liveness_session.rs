//! A `RollbackSession` driven through a `LivenessTransport` (`v1.27.0`).
//!
//! The decorator and the session are tested separately elsewhere; what is only testable together
//! is that the seam actually works — that the session is usable through the decorator at all, and
//! that a dead peer becomes a *reported* disconnect rather than the silent stall the session
//! produces on its own. The session has no clock by design (`docs/adr/0004`), so it cannot tell
//! "waiting for the peer's next input" from "waiting forever"; before the decorator existed, a
//! peer that never sent its handshake left `advance` spinning with nothing to say.

use std::path::PathBuf;
use std::sync::Arc;

use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use rustysnes_netplay::liveness::{
    DisconnectReason, LivenessConfig, LivenessTransport, ManualClock,
};
use rustysnes_netplay::message::{NetMessage, PROTOCOL_VERSION, SYNC_MAGIC};
use rustysnes_netplay::session::{RollbackSession, SessionConfig};
use rustysnes_netplay::transport::{MemoryTransport, Transport};

const SEED: u64 = 777;

/// The same committed permissive ROM `determinism.rs` uses. The emulated content is irrelevant
/// here — only the session plumbing is under test — but a real cart keeps `advance` on its normal
/// path rather than a degenerate one.
fn rom_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/roms/gilyon/cputest/cputest-basic.sfc");
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn fresh_system(rom: &[u8]) -> System {
    let cart = Cart::from_rom(rom).unwrap_or_else(|e| panic!("parse cart: {e:?}"));
    let mut sys = System::new(SEED);
    sys.bus.cart = Some(cart);
    sys
}

/// A deterministic stand-in for a ROM-identity value — the handshake only compares it for
/// equality, exactly as `determinism.rs` notes.
fn rom_hash(rom: &[u8]) -> [u8; 32] {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in rom {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&h.to_le_bytes());
    out
}

fn tight() -> LivenessConfig {
    LivenessConfig {
        ping_interval: std::time::Duration::from_millis(100),
        interrupt_after: std::time::Duration::from_millis(200),
        disconnect_after: std::time::Duration::from_millis(500),
        handshake_timeout: std::time::Duration::from_secs(1),
        ..LivenessConfig::default()
    }
}

/// Mirrors `NetplayState::drive`: tick liveness, raise a disconnect verdict if there is one, then
/// advance. The frontend's own copy is what a user actually runs; this pins the contract it relies
/// on.
fn drive(
    session: &mut RollbackSession<LivenessTransport<MemoryTransport, Arc<ManualClock>>>,
    sys: &mut System,
    input: u16,
) -> Result<bool, DisconnectReason> {
    session.transport_mut().tick(0);
    if let Some(reason) = session.transport().disconnect_reason() {
        return Err(reason);
    }
    session.add_local_input(input);
    let outcome = session
        .advance(sys)
        .expect("no protocol error in this test");
    Ok(matches!(
        outcome,
        rustysnes_netplay::AdvanceOutcome::Advanced { .. }
    ))
}

fn session_with(
    transport: MemoryTransport,
    clock: &Arc<ManualClock>,
    rom: &[u8],
) -> RollbackSession<LivenessTransport<MemoryTransport, Arc<ManualClock>>> {
    let decorated = LivenessTransport::with_clock_and_config(transport, Arc::clone(clock), tight());
    let mut session = RollbackSession::new(SessionConfig::default(), decorated, rom_hash(rom));
    session.send_handshake();
    session
}

#[test]
fn a_peer_that_never_handshakes_disconnects_instead_of_stalling_forever() {
    let rom = rom_bytes();
    let mut sys = fresh_system(&rom);
    let clock = Arc::new(ManualClock::new());
    // `ideal_pair` gives a peer socket that is simply never read or written: nothing ever arrives.
    let (ours, _silent_peer) = MemoryTransport::ideal_pair();
    let mut session = session_with(ours, &clock, &rom);

    // Before the window closes the session is merely stalled, which is correct — startup looks
    // exactly like this.
    for _ in 0..4 {
        assert_eq!(
            drive(&mut session, &mut sys, 0),
            Ok(false),
            "no frame can be produced, but nothing is wrong yet"
        );
    }

    clock.advance(tight().handshake_timeout);
    assert_eq!(
        drive(&mut session, &mut sys, 0),
        Err(DisconnectReason::HandshakeTimeout),
        "the stall must become a reported disconnect once the window closes"
    );
}

#[test]
fn a_peer_that_answered_once_and_then_vanished_reports_peer_timeout() {
    // The negative control: the two reasons must stay distinguishable through the session seam
    // too, or a user cannot tell "wrong address" from "your friend's connection died".
    let rom = rom_bytes();
    let mut sys = fresh_system(&rom);
    let clock = Arc::new(ManualClock::new());
    let (ours, mut peer) = MemoryTransport::ideal_pair();
    let mut session = session_with(ours, &clock, &rom);

    peer.send(&NetMessage::Sync {
        magic: SYNC_MAGIC,
        version: PROTOCOL_VERSION,
        rom_hash: rom_hash(&rom),
    });
    // Whether a frame comes out here is the rollback predictor's business, not this test's — the
    // point is only that the peer was heard from.
    assert!(drive(&mut session, &mut sys, 0).is_ok());
    assert!(session.is_handshaken(), "the handshake genuinely landed");

    clock.advance(tight().disconnect_after);
    assert_eq!(
        drive(&mut session, &mut sys, 0),
        Err(DisconnectReason::PeerTimeout),
        "a peer that spoke and then vanished is a different failure from one that never spoke"
    );
}

#[test]
fn two_sessions_wrapped_in_liveness_still_play_a_normal_game() {
    // The seam has to be transparent. If wrapping the transport perturbed the session at all, the
    // determinism contract would be the first casualty — so this drives a real two-peer game
    // through two decorators and asserts frames are produced on both ends.
    let rom = rom_bytes();
    let clock = Arc::new(ManualClock::new());
    let (ta, tb) = MemoryTransport::ideal_pair();
    let hash = rom_hash(&rom);

    let mut a = RollbackSession::new(
        SessionConfig {
            local_player: 0,
            ..SessionConfig::default()
        },
        LivenessTransport::with_clock_and_config(ta, Arc::clone(&clock), tight()),
        hash,
    );
    let mut b = RollbackSession::new(
        SessionConfig {
            local_player: 1,
            ..SessionConfig::default()
        },
        LivenessTransport::with_clock_and_config(tb, Arc::clone(&clock), tight()),
        hash,
    );
    a.send_handshake();
    b.send_handshake();

    let mut sys_a = fresh_system(&rom);
    let mut sys_b = fresh_system(&rom);
    let (mut frames_a, mut frames_b) = (0u32, 0u32);
    for i in 0..40u32 {
        clock.advance_ms(16);
        #[allow(clippy::cast_possible_truncation)]
        let input = i as u16;
        if drive(&mut a, &mut sys_a, input).expect("a stays connected") {
            frames_a += 1;
        }
        if drive(&mut b, &mut sys_b, input.wrapping_add(1)).expect("b stays connected") {
            frames_b += 1;
        }
    }

    assert!(
        frames_a > 0 && frames_b > 0,
        "both peers must actually produce frames through the decorator; got {frames_a}/{frames_b}"
    );
    assert!(a.is_handshaken() && b.is_handshaken());
    // And the decorator measured a real round trip while they played, which is only possible
    // because each end answers the other's probe.
    assert!(
        a.transport().ping_ms().is_some(),
        "a live game must produce an RTT sample"
    );
}
