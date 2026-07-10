//! `rustysnes-netplay` — GGPO-style rollback netplay (`v0.9.0 "Community"`, T-82-002).
//!
//! Ported from RustyNES's `rustynes-netplay::session::RollbackSession` (the rollback loop's
//! shape is carried over faithfully — see `session.rs`'s module doc for the exact scope this
//! port covers vs. RustyNES's broader N-player mesh/NAT-traversal/spectator feature set, which
//! is out of this ticket's stated acceptance criteria and not ported here).
//!
//! The frontend drives a session with its own loop, independent of the single-player
//! `emu-thread`/pacer path (`docs/frontend.md`) — this crate itself has no opinion on threading
//! or pacing; it is pure `System`-driving logic plus a pluggable [`Transport`].
//!
//! Determinism (`docs/adr/0004`) is the whole point: [`session::RollbackSession::advance`]'s
//! rollback/re-simulate path must reproduce a hypothetical zero-latency reference run
//! bit-identically. `tests/determinism.rs` proves this over synthetic latency/jitter/packet-loss
//! network conditions via [`transport::MemoryTransport`].

pub mod message;
pub mod rng;
pub mod session;
pub mod transport;

#[cfg(not(target_arch = "wasm32"))]
pub mod udp;
#[cfg(target_arch = "wasm32")]
pub mod webrtc;

pub use message::NetMessage;
pub use session::{AdvanceOutcome, MAX_PLAYERS, NetplayError, RollbackSession, SessionConfig};
pub use transport::Transport;
