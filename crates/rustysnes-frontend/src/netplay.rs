//! Native rollback netplay integration (`v0.8.0 "Community"`, T-82-002).
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
//! deferred rather than half-wired — see `v0.8.0`'s CHANGELOG entry.

use std::net::SocketAddr;

use rustysnes_netplay::udp::UdpTransport;
use rustysnes_netplay::{
    AdvanceOutcome, DesyncStatus, LivenessTransport, NetplayError, PeerLink, RollbackSession,
    SessionConfig,
};

use crate::emu::EmuCore;

/// The session's transport, wrapped so peer liveness and RTT are tracked (`v1.27.0`).
///
/// The decorator sits *outside* the session on purpose. `RollbackSession` must stay a pure
/// function of its input stream for the determinism contract (`docs/adr/0004`), so it reads no
/// clock; all the time-based behaviour lives here, in the layer the session only sees as a
/// `Transport`. See `docs/netplay.md`.
pub type SessionTransport = LivenessTransport<UdpTransport>;

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
    Connected(Box<RollbackSession<SessionTransport>>),
}

/// Everything the UI needs to describe a live session, sampled once per frame.
///
/// A plain snapshot rather than borrowed accessors, because the session is driven inside
/// `Active::render`'s frame loop and read again in the egui pass — handing the UI a borrow of the
/// session would make those two uses fight over `&mut App`.
#[derive(Clone, Copy, Debug)]
pub struct NetplayStatus {
    /// How alive the peer looks.
    ///
    /// Never `TimedOut` in practice: [`NetplayState::drive`] raises the verdict as an error the
    /// same frame it appears, and the caller tears the session down — so the terminal state is
    /// reported through `netplay_error` on the shell rather than here. `Live` and `Interrupted`
    /// are the two a live session actually shows.
    pub link: PeerLink,
    /// Smoothed round-trip time, once a sample exists.
    pub ping_ms: Option<u32>,
    /// The peer's reported frame advantage: positive means it is running ahead of us.
    pub remote_frame_advantage: i32,
    /// The graded desync verdict.
    pub desync: DesyncStatus,
    /// Whether the peer's handshake has arrived and matched.
    pub handshaken: bool,
    /// The frame the session is about to run.
    pub frame: u32,
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
) -> std::io::Result<Box<RollbackSession<SessionTransport>>> {
    let transport = LivenessTransport::new(UdpTransport::connect(local_addr, peer_addr)?);
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
        // Tick liveness first, so the probe for this frame goes out alongside the session's own
        // traffic rather than a frame behind it. The frame advantage reported to the peer is how
        // far ahead of the confirmed horizon we are running — the same number the peer's readout
        // shows, which is what makes the two ends agree about who is ahead.
        let advantage = i64::from(session.current_frame())
            - session.last_confirmed_frame().map_or(-1, i64::from);
        // Clamp rather than `unwrap_or(i32::MAX)`: the difference of two `u32` frame counters can
        // exceed `i32` in *either* direction, and a single fallback constant would turn a large
        // negative advantage into "maximally ahead" — the opposite reading.
        let advantage = advantage.clamp(i64::from(i32::MIN), i64::from(i32::MAX));
        #[allow(clippy::cast_possible_truncation)] // clamped to i32's range on the line above
        session.transport_mut().tick(advantage as i32);

        // A dead peer must end the session rather than stall it. The session has no clock and so
        // cannot tell "waiting for input" from "waiting forever" — before the liveness decorator
        // existed, a peer that never handshook left `advance` spinning with nothing to report.
        if let Some(reason) = session.transport().disconnect_reason() {
            return Err(NetplayError::Disconnected(reason));
        }

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

    /// Sample everything the UI shows about the session, or `None` when idle.
    #[must_use]
    pub fn status(&self) -> Option<NetplayStatus> {
        let Self::Connected(session) = self else {
            return None;
        };
        let link = session.transport();
        Some(NetplayStatus {
            link: link.peer_link(),
            ping_ms: link.ping_ms(),
            remote_frame_advantage: link.remote_frame_advantage(),
            desync: session.diagnostics().status(),
            handshaken: session.is_handshaken(),
            frame: session.current_frame(),
        })
    }
}
