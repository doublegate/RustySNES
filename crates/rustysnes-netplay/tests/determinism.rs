//! `RollbackSession`'s determinism proof — T-82-002's core acceptance criterion: rollback
//! re-simulation must be bit-identical, mirroring `movie_determinism.rs`'s pattern (a real
//! committed ROM, VARYING synthetic input, a fresh reference run with no rollback machinery in
//! the loop at all, framebuffer-hash comparison per frame).
//!
//! Both peer sessions run over [`MemoryTransport`] — a deterministic, seeded-PRNG in-process
//! pipe, never a real socket, so this test is itself fully reproducible (`docs/adr/0004`).

use std::path::PathBuf;

use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use rustysnes_netplay::session::NetplayError;
use rustysnes_netplay::transport::MemoryTransport;
use rustysnes_netplay::{RollbackSession, SessionConfig};

const SEED: u64 = 777;
const FRAME_COUNT: u32 = 60;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rom_bytes() -> Vec<u8> {
    let path = workspace_root().join("tests/roms/gilyon/cputest/cputest-basic.sfc");
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn fresh_system(rom: &[u8]) -> System {
    let cart = Cart::from_rom(rom).unwrap_or_else(|e| panic!("parse cart: {e:?}"));
    let mut sys = System::new(SEED);
    sys.bus.cart = Some(cart);
    sys
}

/// Not a cryptographic hash — a test-scoped stand-in for a ROM-identity value, just needs to be
/// deterministic and equal for both peers loading the same bytes (the actual
/// [`NetMessage::Sync`](rustysnes_netplay::NetMessage::Sync) handshake only compares this value
/// for equality; it never needs to resist forgery in this test).
fn fnv_rom_hash(rom: &[u8]) -> [u8; 32] {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in rom {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&h.to_le_bytes());
    out
}

fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A varying (not constant) synthetic input pattern per frame, per player — a session that never
/// actually exercises input divergence between the two players would pass trivially even with a
/// broken rollback implementation (mirrors `movie_determinism.rs`'s own rationale).
const fn input_for_frame(frame: u32) -> (u16, u16) {
    const P1: [u16; 4] = [0x8000, 0x0000, 0x0808, 0xFFF0];
    const P2: [u16; 4] = [0x0000, 0x4000, 0x0101, 0x00F0];
    (
        P1[frame as usize % P1.len()],
        P2[(frame as usize + 2) % P2.len()],
    )
}

/// Drive both sessions, interleaved, until each has produced `target_frame`. Interleaving (not
/// draining one session fully before touching the other) is load-bearing: each session's own
/// input for a frame is only sent to its peer from inside that same `advance()` call, so a
/// session run to completion in isolation would eventually stall waiting on input its peer
/// hasn't been given the chance to send yet.
fn drive_both_to_frame(
    session_a: &mut RollbackSession<MemoryTransport>,
    sys_a: &mut System,
    session_b: &mut RollbackSession<MemoryTransport>,
    sys_b: &mut System,
    target_frame: u32,
) {
    let mut guard = 0u32;
    while session_a.current_frame() <= target_frame || session_b.current_frame() <= target_frame {
        if session_a.current_frame() <= target_frame {
            session_a.advance(sys_a).expect("session A advance");
        }
        if session_b.current_frame() <= target_frame {
            session_b.advance(sys_b).expect("session B advance");
        }
        guard += 1;
        assert!(
            guard < 1_000_000,
            "sessions failed to reach frame {target_frame} (stalled or deadlocked)"
        );
    }
}

/// Run a full paired-session replay against `transport_pair` and return each side's per-frame
/// framebuffer hash sequence.
fn run_paired_sessions(
    rom: &[u8],
    (transport_a, transport_b): (MemoryTransport, MemoryTransport),
) -> (Vec<u64>, Vec<u64>) {
    let hash = fnv_rom_hash(rom);
    let mut sys_a = fresh_system(rom);
    let mut sys_b = fresh_system(rom);
    let mut session_a = RollbackSession::new(
        SessionConfig {
            local_player: 0,
            ..SessionConfig::default()
        },
        transport_a,
        hash,
    );
    let mut session_b = RollbackSession::new(
        SessionConfig {
            local_player: 1,
            ..SessionConfig::default()
        },
        transport_b,
        hash,
    );
    session_a.send_handshake();
    session_b.send_handshake();

    let mut fb_a = Vec::new();
    let mut fb_b = Vec::new();
    for frame in 0..FRAME_COUNT {
        let (p1, p2) = input_for_frame(frame);
        session_a.add_local_input(p1);
        session_b.add_local_input(p2);
        drive_both_to_frame(
            &mut session_a,
            &mut sys_a,
            &mut session_b,
            &mut sys_b,
            frame,
        );
        fb_a.push(hash_fb(sys_a.bus.framebuffer()));
        fb_b.push(hash_fb(sys_b.bus.framebuffer()));
    }
    (fb_a, fb_b)
}

fn reference_run(rom: &[u8]) -> Vec<u64> {
    let mut sys = fresh_system(rom);
    let mut fb = Vec::new();
    for frame in 0..FRAME_COUNT {
        let (p1, p2) = input_for_frame(frame);
        sys.bus.set_joypad(0, p1);
        sys.bus.set_joypad(1, p2);
        sys.run_frame();
        fb.push(hash_fb(sys.bus.framebuffer()));
    }
    fb
}

#[test]
fn rollback_matches_reference_under_ideal_conditions() {
    let rom = rom_bytes();
    let reference = reference_run(&rom);
    let (fb_a, fb_b) = run_paired_sessions(&rom, MemoryTransport::ideal_pair());

    for (i, (a, r)) in fb_a.iter().zip(reference.iter()).enumerate() {
        assert_eq!(
            a, r,
            "session A framebuffer diverged from reference at frame {i}"
        );
    }
    for (i, (b, r)) in fb_b.iter().zip(reference.iter()).enumerate() {
        assert_eq!(
            b, r,
            "session B framebuffer diverged from reference at frame {i}"
        );
    }
}

#[test]
fn rollback_matches_reference_under_latency_jitter_and_packet_loss() {
    let rom = rom_bytes();
    let reference = reference_run(&rom);
    // Real adverse conditions: base latency, jitter on top, and a real (non-zero) drop chance —
    // this is what actually exercises misprediction + rollback + resend, not just the
    // once-per-frame "own input not seen yet" case the ideal-pair test already covers.
    let (fb_a, fb_b) = run_paired_sessions(&rom, MemoryTransport::pair(2026, 3, 4, 0.1));

    for (i, (a, r)) in fb_a.iter().zip(reference.iter()).enumerate() {
        assert_eq!(
            a, r,
            "session A framebuffer diverged from reference at frame {i} under adverse conditions"
        );
    }
    for (i, (b, r)) in fb_b.iter().zip(reference.iter()).enumerate() {
        assert_eq!(
            b, r,
            "session B framebuffer diverged from reference at frame {i} under adverse conditions"
        );
    }
}

#[test]
fn rom_hash_mismatch_is_rejected_before_any_frame_runs() {
    let rom = rom_bytes();
    let (transport_a, transport_b) = MemoryTransport::ideal_pair();
    let mut sys_a = fresh_system(&rom);
    let mut session_a = RollbackSession::new(SessionConfig::default(), transport_a, [0xAA; 32]);
    let mut session_b = RollbackSession::new(
        SessionConfig {
            local_player: 1,
            ..SessionConfig::default()
        },
        transport_b,
        [0xBB; 32], // deliberately different from session_a's
    );
    session_a.send_handshake();
    session_b.send_handshake();

    // session_a's ingest sees session_b's Sync (rom_hash [0xBB; 32]) and must reject it against
    // its own [0xAA; 32].
    let mut saw_mismatch = false;
    for _ in 0..10 {
        match session_a.advance(&mut sys_a) {
            Err(NetplayError::RomMismatch) => {
                saw_mismatch = true;
                break;
            }
            Err(other) => panic!("expected RomMismatch, got {other}"),
            Ok(_) => {}
        }
    }
    assert!(
        saw_mismatch,
        "a genuine ROM-hash mismatch must be rejected, not silently ignored"
    );
}
