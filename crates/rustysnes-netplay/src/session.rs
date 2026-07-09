//! `RollbackSession` — GGPO-style rollback netplay (`v0.9.0 "Community"`, T-82-002).
//!
//! Ported from RustyNES's `rustynes-netplay::session::RollbackSession` (the core rollback
//! loop's shape is carried over faithfully; the N-player mesh/Roster/spectator/NAT-traversal
//! breadth RustyNES also has is deliberately NOT ported — out of this ticket's stated
//! acceptance criteria, and the SNES core itself only has two physical controller ports
//! (`Bus::joypad: [u16; 2]`, no multitap emulation), so this is scoped to exactly 2 players,
//! not RustyNES's up-to-4).
//!
//! The model: every real frame, predict the remote player's input (repeat its last known value
//! if nothing new arrived), run the frame, and remember a checkpoint (a full [`System::save_state`]
//! snapshot) at the point just before running an unconfirmed frame. When a remote input arrives
//! that contradicts an already-run prediction, restore the checkpoint and re-simulate forward
//! with the now-corrected input history — this is what makes the two peers' final state
//! bit-identical to a hypothetical zero-latency run, proven by `tests/determinism.rs`.

use std::collections::VecDeque;

use rustysnes_core::System;

use crate::message::{NetMessage, PROTOCOL_VERSION, SYNC_MAGIC};
use crate::transport::Transport;

/// The SNES core has exactly two physical controller ports (`Bus::joypad: [u16; 2]`) — no
/// multitap emulation exists, so rollback netplay is scoped to this many players.
pub const MAX_PLAYERS: usize = 2;

/// A session's tuning knobs.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Which controller slot (`0` or `1`) this peer's own input drives.
    pub local_player: u8,
    /// How many frames of input-buffering delay to add before an input takes effect — trades
    /// perceived input latency for fewer rollbacks (GGPO's own "input delay" knob). `0` disables
    /// it (input applies the instant it's read).
    pub input_delay: u32,
    /// The maximum number of unconfirmed frames the local simulation may run ahead of the last
    /// confirmed frame before stalling — bounds how much resimulation a late misprediction can
    /// ever cost, and bounds the checkpoint replay-forward distance.
    pub max_rollback_frames: u32,
    /// Send a [`NetMessage::Checksum`] every this many frames for desync detection. `0` disables
    /// it entirely.
    pub checksum_interval: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            local_player: 0,
            input_delay: 0,
            max_rollback_frames: 8,
            checksum_interval: 30,
        }
    }
}

/// Errors a [`RollbackSession`] can raise.
#[derive(Debug, thiserror::Error)]
pub enum NetplayError {
    /// The remote peer's [`NetMessage::Sync`] didn't carry the expected magic value — not
    /// speaking this protocol at all.
    #[error("sync handshake failed: expected magic {expected:#x}, got {got:#x}")]
    BadMagic {
        /// The magic this build expects ([`SYNC_MAGIC`]).
        expected: u32,
        /// The magic the remote peer actually sent.
        got: u32,
    },
    /// The remote peer speaks a different protocol version.
    #[error("protocol version mismatch: local {local}, remote {remote}")]
    VersionMismatch {
        /// This build's [`PROTOCOL_VERSION`].
        local: u16,
        /// The remote peer's protocol version.
        remote: u16,
    },
    /// The remote peer's loaded ROM hash doesn't match this peer's — not the same game.
    #[error("ROM hash mismatch — peers are not running the identical ROM")]
    RomMismatch,
    /// A desync: the two peers' hashed state diverged at a confirmed frame — a real
    /// determinism-contract violation somewhere in the emulated core, not a network artifact.
    #[error(
        "desync detected at frame {frame}: local hash {local_hash:#x}, remote hash {remote_hash:#x}"
    )]
    Desync {
        /// The frame the checksums were taken at.
        frame: u32,
        /// This peer's own computed hash for `frame`.
        local_hash: u64,
        /// The hash the remote peer reported for `frame`.
        remote_hash: u64,
    },
    /// A save-state failed to restore during a rollback — should be unreachable (the checkpoint
    /// is always a blob this same session produced), surfaced rather than panicking regardless.
    #[error("save-state error during rollback: {0}")]
    SaveState(#[from] rustysnes_savestate::SaveStateError),
}

/// What one [`RollbackSession::advance`] call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceOutcome {
    /// A new frame was produced.
    Advanced {
        /// Whether producing this frame required rolling back and re-simulating first.
        rolled_back: bool,
        /// How many frames were re-simulated during this call's rollback (`0` if `rolled_back`
        /// is `false`).
        resimulated_frames: u32,
        /// The frame number just produced.
        frame: u32,
    },
    /// No new frame was produced this call — the session is too far ahead of the last confirmed
    /// frame (`SessionConfig::max_rollback_frames`) and is waiting for the remote peer to catch
    /// up before running further.
    Stalled,
}

#[derive(Debug, Clone, Copy, Default)]
struct PlayerInput {
    input: u16,
    confirmed: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct FrameInputs {
    players: [PlayerInput; MAX_PLAYERS],
    simulated: bool,
}

/// A GGPO-style rollback netplay session driving a [`System`] against a remote peer over `T`.
pub struct RollbackSession<T: Transport> {
    config: SessionConfig,
    transport: T,
    rom_hash: [u8; 32],
    handshaken: bool,
    current_frame: u32,
    last_confirmed_frame: Option<u32>,
    history: Vec<FrameInputs>,
    /// The last confirmed frame's full snapshot, taken just before a still-unconfirmed frame is
    /// simulated — the restore point a rollback replays forward from.
    checkpoint: Option<(u32, Vec<u8>)>,
    /// Checksums we've computed locally but not yet compared (waiting on the remote's report for
    /// that same frame, or vice versa) — `(frame, local_hash, local_fb_hash)`.
    pending_local_checksums: VecDeque<(u32, u64, u64)>,
    pending_remote_checksums: VecDeque<(u32, u64, u64)>,
    /// The highest frame the remote peer has cumulatively acknowledged (via
    /// [`NetMessage::InputAck`]) — everything after this, up to our own last confirmed local
    /// frame, gets resent each [`Self::advance`] call so a dropped `Input` packet is never
    /// permanently lost.
    remote_ack_frame: Option<u32>,
}

impl<T: Transport> RollbackSession<T> {
    /// Start a new session. `rom_hash` is this peer's loaded ROM's SHA-256 — sent to the remote
    /// peer during [`Self::send_handshake`] and compared against theirs before any input is
    /// trusted.
    #[must_use]
    pub const fn new(config: SessionConfig, transport: T, rom_hash: [u8; 32]) -> Self {
        Self {
            config,
            transport,
            rom_hash,
            handshaken: false,
            current_frame: 0,
            last_confirmed_frame: None,
            history: Vec::new(),
            checkpoint: None,
            pending_local_checksums: VecDeque::new(),
            pending_remote_checksums: VecDeque::new(),
            remote_ack_frame: None,
        }
    }

    /// Send this peer's [`NetMessage::Sync`]. Call once before the first [`Self::advance`]; the
    /// remote peer's `Sync` is verified internally the first time it arrives.
    pub fn send_handshake(&mut self) {
        self.transport.send(&NetMessage::Sync {
            magic: SYNC_MAGIC,
            version: PROTOCOL_VERSION,
            rom_hash: self.rom_hash,
        });
    }

    /// Whether the remote peer's `Sync` has arrived and matched.
    #[must_use]
    pub const fn is_handshaken(&self) -> bool {
        self.handshaken
    }

    fn ensure_frame(&mut self, frame: u32) {
        let need = frame as usize + 1;
        if self.history.len() < need {
            self.history.resize(need, FrameInputs::default());
        }
    }

    /// Record the local player's input for the upcoming frame (queued for the next
    /// [`Self::advance`] call to consume).
    pub fn add_local_input(&mut self, input: u16) {
        let frame = self.current_frame;
        self.ensure_frame(frame);
        let lp = self.config.local_player as usize;
        self.history[frame as usize].players[lp] = PlayerInput {
            input,
            confirmed: true,
        };
    }

    fn ingest(&mut self) -> Result<Option<u32>, NetplayError> {
        let mut earliest_mispredict = None;
        for msg in self.transport.poll() {
            match msg {
                NetMessage::Sync {
                    magic,
                    version,
                    rom_hash,
                } => {
                    if magic != SYNC_MAGIC {
                        return Err(NetplayError::BadMagic {
                            expected: SYNC_MAGIC,
                            got: magic,
                        });
                    }
                    if version != PROTOCOL_VERSION {
                        return Err(NetplayError::VersionMismatch {
                            local: PROTOCOL_VERSION,
                            remote: version,
                        });
                    }
                    if rom_hash != self.rom_hash {
                        return Err(NetplayError::RomMismatch);
                    }
                    self.handshaken = true;
                }
                NetMessage::Input {
                    player,
                    frame,
                    input,
                } => {
                    let remote_player = usize::from(player);
                    if remote_player >= MAX_PLAYERS {
                        continue;
                    }
                    self.ensure_frame(frame);
                    let entry = &self.history[frame as usize];
                    let slot = entry.players[remote_player];
                    let was_predicted_differently = slot.confirmed && slot.input != input;
                    let already_simulated_this_frame =
                        entry.simulated && frame < self.current_frame;
                    self.history[frame as usize].players[remote_player] = PlayerInput {
                        input,
                        confirmed: true,
                    };
                    if was_predicted_differently && already_simulated_this_frame {
                        earliest_mispredict = Some(match earliest_mispredict {
                            Some(e) if e <= frame => e,
                            _ => frame,
                        });
                    }
                }
                NetMessage::InputAck { frame } => {
                    self.remote_ack_frame =
                        Some(self.remote_ack_frame.map_or(frame, |f| f.max(frame)));
                }
                NetMessage::Quality { .. } => {
                    // Non-critical connection telemetry — this scoped port doesn't act on it,
                    // matching the ticket's stated acceptance criteria (rollback correctness +
                    // both transports working), not a production-tuned reliability layer.
                }
                NetMessage::Checksum {
                    frame,
                    hash,
                    fb_hash,
                } => {
                    self.pending_remote_checksums
                        .push_back((frame, hash, fb_hash));
                }
            }
        }
        Ok(earliest_mispredict)
    }

    fn recompute_confirmed(&mut self) {
        let mut frame = self.last_confirmed_frame.map_or(0, |f| f + 1);
        while (frame as usize) < self.history.len()
            && self.history[frame as usize]
                .players
                .iter()
                .all(|p| p.confirmed)
        {
            self.last_confirmed_frame = Some(frame);
            frame += 1;
        }
    }

    /// Restore the checkpoint and re-simulate forward through every already-recorded frame up to
    /// (but not including) `self.current_frame`, using the now-corrected input history. Returns
    /// how many frames were re-simulated.
    fn resync(&mut self, sys: &mut System) -> Result<u32, NetplayError> {
        let Some((checkpoint_frame, blob)) = self.checkpoint.clone() else {
            return Ok(0);
        };
        sys.load_state(&blob)?;
        let mut resimulated = 0u32;
        for frame in checkpoint_frame..self.current_frame {
            self.apply_and_run(sys, frame);
            resimulated += 1;
            self.settle_if_confirmed(sys, frame);
        }
        Ok(resimulated)
    }

    fn apply_and_run(&mut self, sys: &mut System, frame: u32) {
        self.ensure_frame(frame);
        for (player, slot) in self.history[frame as usize].players.iter().enumerate() {
            sys.bus.set_joypad(player, slot.input);
        }
        sys.run_frame();
        self.history[frame as usize].simulated = true;
    }

    /// Fill in a prediction for any not-yet-confirmed player slot at `frame` — repeat that
    /// player's last known input (classic GGPO prediction: "probably still holding the same
    /// buttons").
    fn predict_remotes(&mut self, frame: u32) {
        self.ensure_frame(frame);
        for player in 0..MAX_PLAYERS {
            if self.history[frame as usize].players[player].confirmed {
                continue;
            }
            let last_known = (0..frame)
                .rev()
                .map(|f| self.history[f as usize].players[player])
                .find(|p| p.confirmed || p.input != 0)
                .map_or(0, |p| p.input);
            self.history[frame as usize].players[player].input = last_known;
        }
    }

    /// Resend every one of our own player's confirmed inputs the remote peer hasn't acked yet
    /// (`NetMessage::Input`'s own reliability layer — this transport-agnostic session, not any
    /// particular [`Transport`] impl, is what makes the protocol reliable over a lossy link like
    /// UDP or [`crate::transport::MemoryTransport`]'s synthetic packet loss).
    fn resend_unacked_local_inputs(&mut self) {
        let lp = self.config.local_player as usize;
        let start = self.remote_ack_frame.map_or(0, |f| f + 1);
        // `history.len()` is bounded by `ensure_frame`, which never grows it past a real frame
        // count driven by `u32` frame numbers, so this never actually truncates.
        #[allow(clippy::cast_possible_truncation)]
        let history_len = self.history.len() as u32;
        let end = self.current_frame.min(history_len);
        for frame in start..end {
            let slot = self.history[frame as usize].players[lp];
            if slot.confirmed {
                self.transport.send(&NetMessage::Input {
                    player: self.config.local_player,
                    frame,
                    input: slot.input,
                });
            }
        }
    }

    const fn should_stall(&self) -> bool {
        let Some(confirmed) = self.last_confirmed_frame else {
            return false;
        };
        self.current_frame > confirmed + self.config.max_rollback_frames
    }

    /// Called immediately after `frame` has been simulated, exactly when `frame` is known to be
    /// fully confirmed (both players' real input, not a prediction) — the ONLY moment state is
    /// guaranteed never to change again for that frame. Advances the checkpoint to `frame + 1`
    /// (bounding future resimulation distance instead of always replaying from the session's
    /// very first frame) and, at `checksum_interval` boundaries, emits a checksum computed from
    /// this same settled state.
    ///
    /// This settled-only timing is load-bearing: computing/sending a checksum from "live"
    /// state — which may still hold a prediction the peer hasn't corrected yet — races the
    /// eventual correction and produces a false desync between two peers that are, in fact,
    /// converging correctly.
    fn settle_if_confirmed(&mut self, sys: &System, frame: u32) {
        if self.last_confirmed_frame != Some(frame) {
            return;
        }
        self.checkpoint = Some((frame + 1, sys.save_state()));
        if self.config.checksum_interval == 0
            || !frame.is_multiple_of(self.config.checksum_interval)
        {
            return;
        }
        let fb_hash = hash_u16_slice(sys.bus.framebuffer());
        let hash = hash_bytes(&sys.save_state());
        self.pending_local_checksums
            .push_back((frame, hash, fb_hash));
        self.transport.send(&NetMessage::Checksum {
            frame,
            hash,
            fb_hash,
        });
    }

    /// Compare every locally-computed checksum against a matching remote report (matched by
    /// frame number) once both sides exist. Returns [`NetplayError::Desync`] on the first
    /// mismatch found.
    fn compare_pending_checksums(&mut self) -> Result<(), NetplayError> {
        let mut still_pending = VecDeque::with_capacity(self.pending_local_checksums.len());
        let mut desync = None;
        for (frame, hash, fb_hash) in self.pending_local_checksums.drain(..) {
            let Some(pos) = self
                .pending_remote_checksums
                .iter()
                .position(|&(rf, ..)| rf == frame)
            else {
                still_pending.push_back((frame, hash, fb_hash));
                continue;
            };
            let (_, remote_hash, _) = self.pending_remote_checksums.remove(pos).unwrap();
            if desync.is_none() && remote_hash != hash {
                desync = Some((frame, hash, remote_hash));
            }
        }
        self.pending_local_checksums = still_pending;
        match desync {
            Some((frame, local_hash, remote_hash)) => Err(NetplayError::Desync {
                frame,
                local_hash,
                remote_hash,
            }),
            None => Ok(()),
        }
    }

    /// Ingest everything received, roll back and re-simulate if a misprediction was just
    /// corrected, then predict and run exactly one new frame (unless stalled, waiting for the
    /// remote peer to confirm more input first).
    ///
    /// # Errors
    /// Returns [`NetplayError`] on a failed handshake, a ROM mismatch, a confirmed-state desync,
    /// or a save-state error during rollback.
    pub fn advance(&mut self, sys: &mut System) -> Result<AdvanceOutcome, NetplayError> {
        let earliest_mispredict = self.ingest()?;

        let confirmed_before = self.last_confirmed_frame;
        self.recompute_confirmed();
        let confirmation_advanced = self.last_confirmed_frame != confirmed_before;

        if let Some(frame) = self.last_confirmed_frame {
            self.transport.send(&NetMessage::InputAck { frame });
        }

        let mispredicted = earliest_mispredict.is_some_and(|m| m < self.current_frame);
        let mut rolled_back = false;
        let mut resimulated_frames = 0;
        if (mispredicted || confirmation_advanced) && self.checkpoint.is_some() {
            resimulated_frames = self.resync(sys)?;
            rolled_back = mispredicted;
        }

        self.compare_pending_checksums()?;
        // Resend BEFORE the stall check: a stall means we're waiting on the remote's
        // confirmation, and a dropped `Input` packet is exactly why that confirmation might
        // never have arrived — resending here is the recovery path, not an afterthought.
        self.resend_unacked_local_inputs();

        if self.should_stall() {
            return Ok(AdvanceOutcome::Stalled);
        }

        let frame = self.current_frame;
        self.ensure_frame(frame);
        self.predict_remotes(frame);

        if self.checkpoint.is_none() {
            self.checkpoint = Some((frame, sys.save_state()));
        }
        self.apply_and_run(sys, frame);
        self.settle_if_confirmed(sys, frame);

        let lp = self.config.local_player as usize;
        if self.history[frame as usize].players[lp].confirmed {
            self.transport.send(&NetMessage::Input {
                player: self.config.local_player,
                frame,
                input: self.history[frame as usize].players[lp].input,
            });
        }

        self.current_frame += 1;

        Ok(AdvanceOutcome::Advanced {
            rolled_back,
            resimulated_frames,
            frame,
        })
    }

    /// The next frame this session will produce.
    #[must_use]
    pub const fn current_frame(&self) -> u32 {
        self.current_frame
    }

    /// The highest frame confirmed (every player's input known, not predicted) so far.
    #[must_use]
    pub const fn last_confirmed_frame(&self) -> Option<u32> {
        self.last_confirmed_frame
    }
}

/// FNV-1a over a `u16` slice (the framebuffer's own native BGR555 element type) — matches this
/// project's existing determinism-proof hash style (`movie_determinism.rs`'s `hash_fb`).
fn hash_u16_slice(data: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &v in data {
        h ^= u64::from(v);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// FNV-1a over raw bytes — used to hash the full `save_state()` blob for [`NetMessage::Checksum`]
/// (a stronger desync signal than the framebuffer alone: it also catches an audio/timing-only
/// divergence that hasn't yet visibly affected the picture).
fn hash_bytes(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
