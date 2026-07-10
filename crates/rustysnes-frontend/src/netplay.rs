//! Native rollback netplay integration (`v0.9.0 "Community"`, T-82-002).
//!
//! Wraps a [`rustysnes_netplay::RollbackSession`] over a real [`UdpTransport`], driving the
//! `System` directly (`RollbackSession::advance` operates on `rustysnes_core::System`, not this
//! frontend's `EmuCore`, since the netplay crate depends only on the core crate — see
//! `rustysnes-netplay`'s own crate doc). The app's render loop calls [`NetplayState::drive`]
//! instead of `EmuCore::run_frame` whenever a session is active — netplay's own loop, never the
//! single-player `apply_frame_input`/pacer/`emu-thread` path (`docs/frontend.md`
//! §determinism-boundary), so the two production models can never both drive the same `System`.
//!
//! **Native (UDP) only for this pass.** `rustysnes_netplay::webrtc::WebRtcTransport` is itself
//! complete and wasm32-clippy-verified against the real `web_sys` API, but the browser-side SDP
//! offer/answer/ICE negotiation UI is genuinely separate scope (async signaling glue), honestly
//! deferred rather than half-wired — see `v0.9.0`'s CHANGELOG entry.

use std::net::SocketAddr;

use rustysnes_netplay::udp::UdpTransport;
use rustysnes_netplay::{AdvanceOutcome, NetplayError, RollbackSession, SessionConfig};

use crate::emu::EmuCore;

/// A native netplay session's connection state.
///
/// `Connected` is boxed: `RollbackSession` carries its own frame-input history plus (on a
/// misprediction) a full save-state checkpoint blob, making it far larger than `Idle` — boxing
/// keeps `NetplayState` itself small regardless of which variant is live.
#[derive(Default)]
pub enum NetplayState {
    /// No session active — the frontend drives `EmuCore` through its normal single-player path.
    #[default]
    Idle,
    /// A session is connected and driving the `System` directly.
    Connected(Box<RollbackSession<UdpTransport>>),
}

/// Start a new netplay session: bind `local_addr`, connect to `peer_addr`, and send the
/// handshake.
///
/// `local_player` selects which controller slot (`0` or `1`) this peer's own input drives;
/// `rom` is the currently-loaded ROM's raw bytes (hashed and compared against the remote peer's
/// during the handshake, so two peers on different ROMs are rejected rather than silently
/// diverging).
///
/// # Errors
/// Returns the underlying `std::io::Error` if the UDP socket can't be bound/connected.
pub fn start(
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    local_player: u8,
    rom: &[u8],
) -> std::io::Result<Box<RollbackSession<UdpTransport>>> {
    let transport = UdpTransport::connect(local_addr, peer_addr)?;
    let rom_hash = rustysnes_core::movie::hash_rom(rom);
    let mut session = RollbackSession::new(
        SessionConfig {
            local_player,
            ..SessionConfig::default()
        },
        transport,
        rom_hash,
    );
    session.send_handshake();
    Ok(Box::new(session))
}

impl NetplayState {
    /// Drive one real frame: apply `local_input` (this peer's own controller state, already
    /// sanitized) into the session, advance it, and — if a new frame was actually produced (not
    /// [`AdvanceOutcome::Stalled`], waiting on the remote peer) — present it through `emu`
    /// exactly as [`EmuCore::run_frame`] would (see [`EmuCore::present_current_frame`]'s doc for
    /// why this is a separate call from driving the `System` itself).
    ///
    /// # Errors
    /// Returns [`NetplayError`] on a failed handshake, a ROM mismatch, a confirmed-state desync,
    /// or a save-state error — the caller should end the session and fall back to single-player
    /// on any of these, they are not recoverable mid-session.
    pub fn drive(&mut self, local_input: u16, emu: &mut EmuCore) -> Result<(), NetplayError> {
        let Self::Connected(session) = self else {
            return Ok(());
        };
        session.add_local_input(local_input);
        match session.advance(emu.system_mut())? {
            AdvanceOutcome::Advanced { .. } => emu.present_current_frame(),
            AdvanceOutcome::Stalled => {}
        }
        Ok(())
    }

    /// Whether a session is currently connected.
    #[must_use]
    pub const fn is_connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }
}
