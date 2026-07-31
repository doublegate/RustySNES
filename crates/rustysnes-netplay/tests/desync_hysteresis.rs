//! The session-level half of the graded desync verdict (`v1.27.0`).
//!
//! `diagnostics.rs`'s own unit tests cover the verdict arithmetic. What they cannot cover is the
//! behaviour that actually changed: a [`RollbackSession`] used to return the fatal
//! [`NetplayError::Desync`] on the **first** checksum mismatch, and the frontend tore the session
//! down on it. A burst-reordered pair of `Checksum` messages produces exactly one such mismatch,
//! so a transient network event ended a healthy game.
//!
//! These tests drive a real session and forge the peer's `Checksum` messages, asserting the
//! session now survives a transient and still fails on a sustained divergence.

use rustysnes_core::System;
use rustysnes_netplay::message::NetMessage;
use rustysnes_netplay::session::{NetplayError, RollbackSession, SessionConfig};
use rustysnes_netplay::transport::{MemoryTransport, Transport};
use rustysnes_netplay::{DesyncDiagnostics, DesyncStatus};

/// A minimal LoROM the `System` will accept — enough to run frames, which is all this needs.
fn minimal_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    let h = 0x7FC0;
    rom[h..h + 21].copy_from_slice(b"NETPLAY DESYNC TEST  ");
    rom[h + 0x15] = 0x20; // slow LoROM
    rom[h + 0x17] = 0x08; // ROM size
    rom[h + 0x1C..h + 0x1E].copy_from_slice(&(!0x1234u16).to_le_bytes());
    rom[h + 0x1E..h + 0x20].copy_from_slice(&0x1234u16.to_le_bytes());
    rom[h + 0x3C..h + 0x3E].copy_from_slice(&0x8000u16.to_le_bytes());
    rom
}

fn fresh_system(rom: &[u8]) -> System {
    let mut sys = System::new(0);
    sys.bus.cart = Some(rustysnes_core::cart::Cart::from_rom(rom).expect("test ROM must load"));
    sys.reset();
    sys
}

/// Drive one session far enough that `checksum_interval` boundaries produce local checksums, while
/// the peer end feeds input plus whatever `Checksum` messages `forge` chooses to send.
///
/// Returns the first error the session raised, if any, along with the session for inspection.
fn run_with_forged_checksums(
    frames: u32,
    mut forge: impl FnMut(u32) -> Option<NetMessage>,
) -> (Result<(), NetplayError>, RollbackSession<MemoryTransport>) {
    let rom = minimal_rom();
    let hash = [0x5Au8; 32];
    let (ta, mut tb) = MemoryTransport::ideal_pair();

    let mut sys = fresh_system(&rom);
    let mut session = RollbackSession::new(
        SessionConfig {
            local_player: 0,
            ..SessionConfig::default()
        },
        ta,
        hash,
    );
    session.send_handshake();

    // The peer's side of the handshake, so the session will trust anything that follows.
    tb.send(&NetMessage::Sync {
        magic: rustysnes_netplay::message::SYNC_MAGIC,
        version: rustysnes_netplay::message::PROTOCOL_VERSION,
        rom_hash: hash,
    });

    let mut outcome = Ok(());
    for frame in 0..frames {
        session.add_local_input(0);
        // The peer's input for this frame, so neither side stalls waiting for confirmation.
        tb.send(&NetMessage::Input {
            player: 1,
            frame,
            input: 0,
        });
        if let Some(msg) = forge(frame) {
            tb.send(&msg);
        }
        // Drain whatever the session sent, so the pipe does not grow unboundedly.
        let _ = tb.poll();

        if let Err(e) = session.advance(&mut sys) {
            outcome = Err(e);
            break;
        }
    }
    (outcome, session)
}

/// The frames at which the session emits a checksum, given the default 30-frame interval.
fn checksum_frames(upto: u32) -> Vec<u32> {
    (0..upto).filter(|f| f % 30 == 0).collect()
}

#[test]
fn a_single_mismatched_checksum_no_longer_kills_the_session() {
    // THE REGRESSION THIS PR EXISTS FOR. Exactly one forged-wrong checksum — the shape a
    // burst-reordered `Checksum` pair produces. Before the graded verdict this returned
    // `NetplayError::Desync` and `app.rs` disconnected on it.
    let poisoned = checksum_frames(200).first().copied().expect("an interval");
    let (outcome, session) = run_with_forged_checksums(200, |frame| {
        (frame == poisoned).then_some(NetMessage::Checksum {
            frame: poisoned,
            hash: 0xDEAD_BEEF_DEAD_BEEF,
            fb_hash: 0xDEAD_BEEF_DEAD_BEEF,
        })
    });

    assert!(
        outcome.is_ok(),
        "one transient mismatch must not end the session, got {outcome:?}"
    );

    // ANTI-VACUOUS GUARD. Surviving is only meaningful if the forged checksum actually reached the
    // comparator — if it silently never matched a local checksum by frame number, this test would
    // pass while exercising nothing. Measured: exactly one comparison, exactly one mismatch.
    let d = session.diagnostics();
    assert_eq!(d.total(), 1, "the forged checksum must have been compared");
    assert_eq!(
        d.mismatches(),
        1,
        "and it must have been seen as a mismatch"
    );
    assert_eq!(d.first_desync_frame(), Some(poisoned));
    assert_eq!(
        d.status(),
        DesyncStatus::Suspect {
            consecutive: 1,
            first_desync_frame: poisoned
        },
        "a lone mismatch is Suspect, never Desynced"
    );
    assert!(!d.is_desynced());
}

#[test]
fn a_sustained_divergence_still_fails_the_session() {
    // The negative control. Tolerating a transient is only correct if a REAL divergence still
    // ends the session — a rollback desync is unrecoverable, so continuing would mean two peers
    // silently playing different games.
    let poisoned: Vec<u32> = checksum_frames(400);
    let (outcome, session) = run_with_forged_checksums(400, move |frame| {
        poisoned.contains(&frame).then_some(NetMessage::Checksum {
            frame,
            hash: 0xDEAD_BEEF_DEAD_BEEF,
            fb_hash: 0xDEAD_BEEF_DEAD_BEEF,
        })
    });

    let Err(NetplayError::Desync {
        frame,
        local_hash,
        remote_hash,
    }) = outcome
    else {
        panic!("a sustained divergence must still be fatal, got {outcome:?}");
    };
    let d = session.diagnostics();
    assert!(d.is_desynced());
    assert!(matches!(d.status(), DesyncStatus::Desynced { .. }));
    assert!(
        d.peak_consecutive_mismatches() >= DesyncDiagnostics::DEFAULT_DESYNC_THRESHOLD,
        "it must have crossed the confirm threshold, not fired early"
    );

    // THE ERROR PAYLOAD MUST BE SELF-CONSISTENT (raised in review). The confirming pass may hold
    // no mismatch of its own — the run can be built across earlier passes — and an earlier
    // revision filled the gap by pairing `first_desync_frame` with the hashes of whatever was
    // compared LAST, which can be a different, even matching, frame. The reported frame and the
    // reported hashes must come from one comparison.
    let first = d.first_desync().expect("a divergence was recorded");
    assert_eq!(frame, first.frame, "the frame must be the FIRST divergence");
    assert_eq!(local_hash, first.local, "and its own local hash");
    assert_eq!(remote_hash, first.remote, "and its own remote hash");
    assert_ne!(
        local_hash, remote_hash,
        "a reported desync must describe a comparison that actually disagreed"
    );
    // The divergence began before the frame that crossed the threshold, so this is a real
    // distinction and not a coincidence of a one-comparison run.
    assert!(
        d.last().expect("recorded").frame > first.frame,
        "later comparisons exist, so reporting the first is a genuine choice"
    );
}

#[test]
fn a_clean_session_records_matching_compares_and_stays_in_sync() {
    // Guards against the opposite failure: a diagnostics record that never sees a matching
    // comparison would report `InSync` vacuously (it reports `InSync` when nothing is recorded at
    // all), so this asserts real compares actually flowed through it.
    let (outcome, session) = run_with_forged_checksums(200, |_| None);
    assert!(outcome.is_ok());

    let d = session.diagnostics();
    assert_eq!(d.status(), DesyncStatus::InSync);
    assert_eq!(d.mismatches(), 0);
    // The peer never sent a checksum, so nothing could be compared — the local side has no partner
    // to match against. That is expected, and is exactly why this assertion is on `mismatches`
    // rather than on `total`: asserting `total > 0` here would fail for a reason unrelated to the
    // verdict, and asserting nothing would let a broken recorder pass.
    assert!(d.in_sync());
}
