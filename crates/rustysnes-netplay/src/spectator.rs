//! Read-only spectating of a live match (`v1.27.0`).
//!
//! A spectator receives the players' confirmed input stream and replays it into its own [`System`].
//! It is **receive-only**: it never sends an ack, a checksum, or a quality reply, so a spectator
//! cannot perturb the match it is watching however many of them attach.
//!
//! # It never predicts and never rolls back
//!
//! That is the whole difference from [`RollbackSession`]. A player must predict to hide latency and
//! must therefore be able to roll back when a prediction was wrong; a spectator has no input of its
//! own to hide latency for, so it simply waits until a frame's inputs are all known and then runs
//! it. No prediction means no misprediction, which means no rollback machinery, which means a
//! spectator cannot desync — it either has a frame's inputs or it does not.
//!
//! # `delay_frames` is presentation-only
//!
//! The delayed-stream buffer holds a confirmed frame back until `frame + delay_frames` is *also*
//! confirmed. It exists for tournament broadcast (an anti-spoiler delay) and for smoothing jitter,
//! and it moves **only when a frame is revealed, never what it contains** — the emulation is
//! byte-identical either way, which `spectator_output_matches_a_reference_run` pins directly.
//!
//! [`RollbackSession`]: crate::session::RollbackSession

use rustysnes_core::System;

use crate::message::{NetMessage, PROTOCOL_VERSION, SYNC_MAGIC};
use crate::session::MAX_PLAYERS;
use crate::transport::Transport;

/// The furthest ahead of the confirmation horizon a remote-reported frame index is trusted to be.
///
/// A spectator never shows anything beyond the confirmed horizon, so any legitimate in-flight
/// `Input.frame` is only a small bounded distance ahead. Without this cap a peer-supplied `frame`
/// near `u32::MAX` would make the history buffer resize unboundedly — an OOM denial of service from
/// a single datagram. Dropping an out-of-window frame is safe because the players' own session
/// retransmits anything unacknowledged.
const MAX_SPECTATOR_FRAME_LOOKAHEAD: u32 = 1024;

/// [`MAX_PLAYERS`] as the `u8` a wire-level player index uses.
///
/// Written as a literal with a compile-time equality check rather than `MAX_PLAYERS as u8`: the
/// widening direction (`u8 as usize`) cannot truncate, so the pedantic cast lints stay on, and if
/// `MAX_PLAYERS` ever grows past 255 this fails the build instead of silently truncating the clamp
/// that exists to reject out-of-range player indices.
const MAX_PLAYERS_U8: u8 = 2;
const _: () = assert!(MAX_PLAYERS_U8 as usize == MAX_PLAYERS);

/// Spectator tuning.
#[derive(Debug, Clone, Copy)]
pub struct SpectatorConfig {
    /// How many players to expect. Clamped to `1..=MAX_PLAYERS`.
    pub num_players: u8,
    /// Extra frames to hold a confirmed frame before revealing it.
    ///
    /// `0` reveals as soon as a frame is confirmed. A positive value is the broadcast /
    /// anti-spoiler delay: frame `f` is shown only once `f + delay_frames` is also confirmed.
    /// Clamped to [`SpectatorConfig::MAX_DELAY_FRAMES`], because the buffer it implies is a real
    /// allocation and an unbounded value from a config file should not become one.
    pub delay_frames: u32,
}

impl SpectatorConfig {
    /// The largest accepted [`Self::delay_frames`] — ~8.5 s at 60 fps, far beyond any broadcast
    /// delay anyone sets, and a hard bound on the buffered history.
    pub const MAX_DELAY_FRAMES: u32 = 512;
}

impl Default for SpectatorConfig {
    fn default() -> Self {
        Self {
            num_players: 2,
            delay_frames: 0,
        }
    }
}

/// What one [`SpectatorSession::advance`] did.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SpectatorOutcome {
    /// Whether a frame was produced.
    ///
    /// `false` means the spectator is waiting for the next frame's inputs (or for the reveal
    /// horizon to catch up), and the caller should skip presenting — it would re-show the same
    /// picture.
    pub produced_frame: bool,
    /// The frame just produced; meaningful only when [`Self::produced_frame`].
    pub frame: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct FrameInputs {
    inputs: [u16; MAX_PLAYERS],
    /// Bit `p` set once player `p`'s real input for this frame has arrived.
    arrived: u8,
}

/// A read-only spectator replaying a match's confirmed input stream into a local [`System`].
///
/// Construct, then call [`Self::advance`] once per visual frame.
#[derive(Debug)]
pub struct SpectatorSession<T: Transport> {
    config: SpectatorConfig,
    transport: T,
    rom_hash: [u8; 32],
    synced: bool,
    history: Vec<FrameInputs>,
    current_frame: u32,
    last_confirmed_frame: Option<u32>,
}

impl<T: Transport> SpectatorSession<T> {
    /// Start spectating. `rom_hash` is this peer's loaded ROM's SHA-256, compared against the
    /// match's `Sync` before anything is trusted.
    #[must_use]
    pub fn new(config: SpectatorConfig, transport: T, rom_hash: [u8; 32]) -> Self {
        let num_players = config.num_players.clamp(1, MAX_PLAYERS_U8);
        let delay_frames = config.delay_frames.min(SpectatorConfig::MAX_DELAY_FRAMES);
        Self {
            config: SpectatorConfig {
                num_players,
                delay_frames,
            },
            transport,
            rom_hash,
            synced: false,
            history: Vec::new(),
            current_frame: 0,
            last_confirmed_frame: None,
        }
    }

    /// The next frame this spectator will produce.
    #[must_use]
    pub const fn current_frame(&self) -> u32 {
        self.current_frame
    }

    /// The highest frame whose every player input has arrived.
    #[must_use]
    pub const fn last_confirmed_frame(&self) -> Option<u32> {
        self.last_confirmed_frame
    }

    /// The player count in effect (clamped at construction).
    #[must_use]
    pub const fn num_players(&self) -> u8 {
        self.config.num_players
    }

    /// The reveal delay in effect (clamped at construction).
    #[must_use]
    pub const fn delay_frames(&self) -> u32 {
        self.config.delay_frames
    }

    /// Whether the match's handshake has been seen and accepted.
    #[must_use]
    pub const fn is_synced(&self) -> bool {
        self.synced
    }

    /// The wrapped transport.
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    /// How many confirmed frames are buffered but not yet shown — i.e. how far behind the live
    /// match this spectator is running.
    ///
    /// The `current_frame > c` case is the one worth spelling out: it means every confirmed frame
    /// has already been shown, and the answer is zero. Writing this as
    /// `c.saturating_sub(current) + 1` gets it wrong, because the saturation floor is `0` and the
    /// `+ 1` then lifts it back to `1` — a "frames behind" readout that can never say "caught up"
    /// is exactly the reading a viewer would use it for.
    #[must_use]
    pub fn pending_frames(&self) -> u32 {
        self.last_confirmed_frame.map_or(0, |c| {
            if c < self.current_frame {
                0
            } else {
                c - self.current_frame + 1
            }
        })
    }

    /// Ingest, then produce one frame if the next one is both confirmed and past the reveal
    /// horizon.
    pub fn advance(&mut self, sys: &mut System) -> SpectatorOutcome {
        self.ingest();
        self.recompute_confirmed();

        let frame = self.current_frame;
        let ready = self.reveal_horizon().is_some_and(|h| frame <= h);
        if !ready {
            return SpectatorOutcome::default();
        }

        self.apply_and_run(sys, frame);
        self.current_frame = self.current_frame.saturating_add(1);
        SpectatorOutcome {
            produced_frame: true,
            frame,
        }
    }

    /// The highest frame allowed to be revealed right now.
    fn reveal_horizon(&self) -> Option<u32> {
        self.last_confirmed_frame
            .map(|c| c.saturating_sub(self.config.delay_frames))
    }

    /// Poll and fold in every relevant message. **Sends nothing** — that is the invariant that
    /// makes a spectator free to attach.
    fn ingest(&mut self) {
        for msg in self.transport.poll() {
            match msg {
                NetMessage::Sync {
                    magic,
                    version,
                    rom_hash,
                } => {
                    if magic == SYNC_MAGIC
                        && version == PROTOCOL_VERSION
                        && rom_hash == self.rom_hash
                    {
                        self.synced = true;
                    }
                }
                NetMessage::Input {
                    player,
                    frame,
                    input,
                } => {
                    // Input before a accepted handshake is not watchable input. Without this the
                    // `synced` flag gated nothing: a peer that never sent `Sync`, or whose `Sync`
                    // named a different ROM, could still drive frames into the `System` — which is
                    // precisely the "half-watch a different game" case the handshake exists to
                    // refuse. `is_synced()` reporting `false` while the session ran anyway made the
                    // flag a readout rather than a boundary.
                    if !self.synced {
                        continue;
                    }
                    // An out-of-range player index can never become valid — `num_players` is fixed
                    // at construction — so dropping it is correct rather than best-effort, and it
                    // keeps a malformed or foreign packet from indexing out of bounds.
                    if player >= self.config.num_players {
                        continue;
                    }
                    // See `MAX_SPECTATOR_FRAME_LOOKAHEAD`: without this bound one datagram
                    // carrying a frame near `u32::MAX` resizes `history` unboundedly.
                    let horizon = self.last_confirmed_frame.map_or(0, |c| c.saturating_add(1));
                    if frame > horizon.saturating_add(MAX_SPECTATOR_FRAME_LOOKAHEAD) {
                        continue;
                    }
                    self.ensure_frame(frame);
                    let slot = &mut self.history[frame as usize];
                    slot.inputs[player as usize] = input;
                    slot.arrived |= 1 << player;
                }
                // Everything else belongs to the players' own protocol. A spectator ignores acks,
                // checksums, and quality reports rather than acting on them: it has no state to
                // compare and nothing to reply with.
                NetMessage::InputAck { .. }
                | NetMessage::Checksum { .. }
                | NetMessage::Quality { .. } => {}
            }
        }
    }

    /// Extend the confirmation horizon over every frame whose whole player set has arrived.
    fn recompute_confirmed(&mut self) {
        let all = if self.config.num_players >= 8 {
            u8::MAX
        } else {
            (1u8 << self.config.num_players) - 1
        };
        let mut next = self.last_confirmed_frame.map_or(0, |c| c.saturating_add(1));
        while (next as usize) < self.history.len() && self.history[next as usize].arrived == all {
            self.last_confirmed_frame = Some(next);
            next = next.saturating_add(1);
        }
    }

    fn ensure_frame(&mut self, frame: u32) {
        // `saturating_add` rather than `+ 1`: unreachable behind the lookahead bound above, but a
        // 32-bit `usize` would wrap this to zero and silently truncate the history instead.
        let needed = (frame as usize).saturating_add(1);
        if self.history.len() < needed {
            self.history.resize(needed, FrameInputs::default());
        }
    }

    /// Takes `&self`, not `&mut self` — dropping the `ensure_frame` call left nothing here that
    /// mutates the session, and the signature now says so.
    fn apply_and_run(&self, sys: &mut System, frame: u32) {
        // No `ensure_frame` here. `advance` only reaches this with `frame <= reveal_horizon <=
        // last_confirmed_frame`, and `recompute_confirmed` only confirms frames already inside
        // `history` — so growing the buffer here could only ever paper over a broken invariant by
        // running an all-zero input frame, which is a desync-shaped bug that is far harder to find
        // than the index panic it would replace.
        debug_assert!(
            (frame as usize) < self.history.len(),
            "advance must only run confirmed frames"
        );
        let slot = self.history[frame as usize];
        for (player, input) in slot.inputs.iter().enumerate() {
            sys.bus.set_joypad(player, *input);
        }
        sys.run_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MemoryTransport;

    fn minimal_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x8000];
        let h = 0x7FC0;
        rom[h..h + 21].copy_from_slice(b"SPECTATOR TEST ROM   ");
        rom[h + 0x15] = 0x20;
        rom[h + 0x17] = 0x08;
        rom[h + 0x1C..h + 0x1E].copy_from_slice(&(!0x1234u16).to_le_bytes());
        rom[h + 0x1E..h + 0x20].copy_from_slice(&0x1234u16.to_le_bytes());
        rom[h + 0x3C..h + 0x3E].copy_from_slice(&0x8000u16.to_le_bytes());
        rom
    }

    fn fresh_system(rom: &[u8]) -> System {
        let mut sys = System::new(0);
        sys.bus.cart = Some(rustysnes_core::cart::Cart::from_rom(rom).expect("test ROM"));
        sys.reset();
        sys
    }

    fn fnv(bytes: &[u16]) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for w in bytes {
            h ^= u64::from(*w);
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        h
    }

    /// Per-frame input for the scripted match both sides replay.
    fn input_for(frame: u32) -> (u16, u16) {
        (
            u16::try_from(frame % 0x1000).unwrap_or(0),
            u16::try_from((frame * 7) % 0x1000).unwrap_or(0),
        )
    }

    const FRAMES: u32 = 40;

    /// Feed a spectator the whole match's input stream, driving it once per frame.
    fn run_spectator(delay_frames: u32) -> (Vec<u64>, SpectatorSession<MemoryTransport>) {
        let rom = minimal_rom();
        let hash = [0x7Au8; 32];
        let (ta, mut tb) = MemoryTransport::ideal_pair();
        let mut sys = fresh_system(&rom);
        let mut spec = SpectatorSession::new(
            SpectatorConfig {
                num_players: 2,
                delay_frames,
            },
            ta,
            hash,
        );
        tb.send(&NetMessage::Sync {
            magic: SYNC_MAGIC,
            version: PROTOCOL_VERSION,
            rom_hash: hash,
        });

        let mut hashes = Vec::new();
        // Feed everything up front, then drain — a spectator has no pacing opinion.
        for frame in 0..FRAMES {
            let (p1, p2) = input_for(frame);
            tb.send(&NetMessage::Input {
                player: 0,
                frame,
                input: p1,
            });
            tb.send(&NetMessage::Input {
                player: 1,
                frame,
                input: p2,
            });
        }
        for _ in 0..FRAMES * 2 {
            if spec.advance(&mut sys).produced_frame {
                hashes.push(fnv(sys.bus.framebuffer()));
            }
        }
        (hashes, spec)
    }

    /// The same input stream applied directly, with no netplay in the way.
    fn reference_run(upto: u32) -> Vec<u64> {
        let rom = minimal_rom();
        let mut sys = fresh_system(&rom);
        let mut hashes = Vec::new();
        for frame in 0..upto {
            let (p1, p2) = input_for(frame);
            sys.bus.set_joypad(0, p1);
            sys.bus.set_joypad(1, p2);
            sys.run_frame();
            hashes.push(fnv(sys.bus.framebuffer()));
        }
        hashes
    }

    #[test]
    fn spectator_output_matches_a_reference_run() {
        // The load-bearing test. A spectator that replayed the inputs even slightly differently
        // would show a different game from the one being played — and because it never predicts,
        // there is no rollback to paper over it.
        let (hashes, _) = run_spectator(0);
        assert_eq!(hashes.len(), FRAMES as usize, "every frame was produced");
        assert_eq!(
            hashes,
            reference_run(FRAMES),
            "spectator emulation must be byte-identical to a direct run"
        );
    }

    #[test]
    fn the_reveal_delay_changes_when_not_what() {
        // `delay_frames` is presentation-only. It must hold frames back without altering a single
        // one of them, or a broadcast delay would silently be a different game.
        let (delayed, spec) = run_spectator(10);
        assert_eq!(spec.delay_frames(), 10);
        assert!(
            delayed.len() < FRAMES as usize,
            "a delay must hold the tail back; produced {}",
            delayed.len()
        );
        assert_eq!(
            delayed,
            reference_run(u32::try_from(delayed.len()).expect("frame count fits a u32")),
            "the frames it DID show must be byte-identical to the reference"
        );
    }

    /// Counts everything sent through it. Local to this test rather than a `sent_count()` on
    /// `MemoryTransport`, which would widen a shipped type's API to serve one assertion.
    #[derive(Default)]
    struct CountingTransport {
        sent: usize,
        inbox: Vec<NetMessage>,
    }

    impl Transport for CountingTransport {
        fn send(&mut self, _msg: &NetMessage) {
            self.sent += 1;
        }
        fn poll(&mut self) -> Vec<NetMessage> {
            std::mem::take(&mut self.inbox)
        }
    }

    #[test]
    fn a_spectator_never_sends_anything() {
        // The invariant that makes spectators free to attach: however many watch, the match sees no
        // extra traffic. A stray ack or checksum reply would put a spectator into the players'
        // protocol — and a relay fanning one match to many spectators would multiply it.
        let hash = [0x7Au8; 32];
        let mut inbox = vec![NetMessage::Sync {
            magic: SYNC_MAGIC,
            version: PROTOCOL_VERSION,
            rom_hash: hash,
        }];
        for frame in 0..FRAMES {
            let (p1, p2) = input_for(frame);
            inbox.push(NetMessage::Input {
                player: 0,
                frame,
                input: p1,
            });
            inbox.push(NetMessage::Input {
                player: 1,
                frame,
                input: p2,
            });
            // Messages a spectator must ignore rather than answer.
            inbox.push(NetMessage::InputAck { frame });
            inbox.push(NetMessage::Checksum {
                frame,
                hash: 1,
                fb_hash: 2,
            });
            inbox.push(NetMessage::Quality {
                ping_ms: 5,
                frame_advantage: 1,
                probe: frame + 1,
                echo: 0,
            });
        }
        let transport = CountingTransport { sent: 0, inbox };
        let mut spec = SpectatorSession::new(SpectatorConfig::default(), transport, hash);
        let mut sys = fresh_system(&minimal_rom());
        for _ in 0..FRAMES * 2 {
            let _ = spec.advance(&mut sys);
        }
        assert!(
            spec.is_synced(),
            "the scripted stream did include a valid Sync"
        );
        assert_eq!(
            spec.transport().sent,
            0,
            "a spectator must be receive-only, even when fed acks/checksums/pings"
        );
    }

    /// Both boundary tests below must hand-shake first. Without the `Sync` their input would be
    /// dropped by the *handshake* gate, and each would then pass no matter what the bound it names
    /// actually did.
    fn synced_spectator(hash: [u8; 32]) -> (SpectatorSession<MemoryTransport>, MemoryTransport) {
        let (ta, mut tb) = MemoryTransport::ideal_pair();
        let spec = SpectatorSession::new(SpectatorConfig::default(), ta, hash);
        tb.send(&NetMessage::Sync {
            magic: SYNC_MAGIC,
            version: PROTOCOL_VERSION,
            rom_hash: hash,
        });
        (spec, tb)
    }

    #[test]
    fn an_out_of_window_frame_is_dropped_rather_than_allocated() {
        // One datagram carrying a frame near `u32::MAX` would otherwise resize the history buffer
        // unboundedly — an OOM from a single packet.
        let hash = [0x7Au8; 32];
        let (mut spec, mut tb) = synced_spectator(hash);
        tb.send(&NetMessage::Input {
            player: 0,
            frame: u32::MAX,
            input: 0,
        });
        let mut sys = fresh_system(&minimal_rom());
        let out = spec.advance(&mut sys);
        assert!(spec.is_synced(), "the bound under test, not the handshake");
        assert!(!out.produced_frame);
        assert_eq!(spec.last_confirmed_frame(), None);
    }

    #[test]
    fn an_out_of_range_player_index_is_dropped() {
        // `num_players` is fixed at construction, so such an index can never become valid.
        let hash = [0x7Au8; 32];
        let (mut spec, mut tb) = synced_spectator(hash);
        tb.send(&NetMessage::Input {
            player: 9,
            frame: 0,
            input: 0xFFFF,
        });
        let mut sys = fresh_system(&minimal_rom());
        assert!(!spec.advance(&mut sys).produced_frame);
        assert!(spec.is_synced(), "the bound under test, not the handshake");
        assert_eq!(spec.last_confirmed_frame(), None);
    }

    #[test]
    fn the_delay_and_player_count_are_clamped_at_construction() {
        let (ta, _) = MemoryTransport::ideal_pair();
        let spec = SpectatorSession::new(
            SpectatorConfig {
                num_players: 200,
                delay_frames: u32::MAX,
            },
            ta,
            [0u8; 32],
        );
        assert_eq!(spec.num_players(), MAX_PLAYERS_U8);
        assert_eq!(spec.delay_frames(), SpectatorConfig::MAX_DELAY_FRAMES);
    }

    #[test]
    fn pending_frames_reaches_zero_once_everything_confirmed_has_been_shown() {
        // Raised in review, and real: the obvious `saturating_sub(current) + 1` has a floor of 1,
        // so a "frames behind" readout could never say "caught up" — which is the one reading it
        // exists to give.
        let (hashes, played) = run_spectator(0);
        assert_eq!(hashes.len(), FRAMES as usize, "the run drained completely");
        assert!(
            played.current_frame() > played.last_confirmed_frame().expect("something confirmed"),
            "the caught-up branch is genuinely the one under test, not an unreached one"
        );
        assert_eq!(
            played.pending_frames(),
            0,
            "every confirmed frame has been shown"
        );

        // The negative control: a spectator still holding buffered frames must count them, so the
        // fix cannot be "always return 0".
        let (delayed_hashes, delayed) = run_spectator(8);
        assert!(
            delayed_hashes.len() < FRAMES as usize,
            "the reveal delay genuinely held frames back"
        );
        assert!(
            delayed.pending_frames() > 0,
            "buffered-but-unshown frames must still count"
        );
    }

    #[test]
    fn a_foreign_rom_hash_is_not_watchable_at_all() {
        // Asserting only `!is_synced()` would be the vacuous version of this test: the flag was
        // once purely a readout, so a rejected handshake still let the following inputs drive the
        // `System`. What the claim actually needs is that *no frame is produced* — verified by
        // injection, by removing the `!self.synced` guard in `ingest` and confirming this fails.
        let (ta, mut tb) = MemoryTransport::ideal_pair();
        let mut spec = SpectatorSession::new(SpectatorConfig::default(), ta, [0x11u8; 32]);
        tb.send(&NetMessage::Sync {
            magic: SYNC_MAGIC,
            version: PROTOCOL_VERSION,
            rom_hash: [0x22u8; 32],
        });
        for frame in 0..8u32 {
            let (p1, p2) = input_for(frame);
            tb.send(&NetMessage::Input {
                player: 0,
                frame,
                input: p1,
            });
            tb.send(&NetMessage::Input {
                player: 1,
                frame,
                input: p2,
            });
        }
        let mut sys = fresh_system(&minimal_rom());
        for _ in 0..16 {
            assert!(
                !spec.advance(&mut sys).produced_frame,
                "a different game must not be watchable"
            );
        }
        assert!(!spec.is_synced());
        assert_eq!(spec.last_confirmed_frame(), None);
    }
}
