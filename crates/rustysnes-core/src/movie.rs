//! TAS movie record/playback — a deterministic input log plus a `System::save_state`-compatible
//! start point.
//!
//! `docs/adr/0004`'s determinism contract: same seed + ROM + input ⇒ bit-identical
//! framebuffer/audio. Ported from RustyNES's proven `rustynes-core::movie` shape (confirmed by
//! reading its source directly), SNES-adapted: a single [`FrameInput`] per frame (the SNES
//! auto-joypad latches one `u16` per controller, unlike the NES's separate button set), and the
//! start point's seed is recorded explicitly since `System::new` takes a caller-chosen seed
//! rather than always defaulting to one value.
//!
//! [`MovieRecorder`]/[`MoviePlayer`] are pure data + a capture/apply loop — no Lua/frontend
//! coupling, and no `System`/`Bus` reach-around either (see [`MoviePlayer::next_frame`]'s doc for
//! why). The frontend's per-frame drive calls [`MovieRecorder::capture`] (recording) or
//! [`MoviePlayer::next_frame`] (playback) and feeds the result through whatever input
//! abstraction it already uses, immediately before [`crate::System::run_frame`] — the same place
//! it already applies live controller input today.
//!
//! # Format
//!
//! ```text
//! HEADER:
//!     magic            : "RSNESMOV"  (8 bytes)
//!     format version   : u16 LE       (1 = MOVIE_FORMAT_VERSION)
//!     region           : u8           (0 = NTSC, 1 = PAL)
//!     seed             : u64 LE       (the System::new seed this recording used)
//!     rom sha-256      : [u8; 32]     (full hash — authoritative ROM identity, checked on replay)
//!     frame count      : u32 LE
//! START POINT:
//!     kind             : u8           (0 = power-on, 1 = embedded save-state)
//!     [save-state]     : u32 LE length-prefixed bytes (only when kind == 1)
//! INPUT STREAM:
//!     frame_count * 4 bytes; each frame = p1 (u16 LE), p2 (u16 LE)
//! ```

use alloc::vec::Vec;

use rustysnes_cart::Region;
use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};
use sha2::{Digest as _, Sha256};

use crate::scheduler::System;

/// The movie envelope's leading magic bytes.
const MAGIC: &[u8; 8] = b"RSNESMOV";
/// The movie format's version. Bumped any time the header/frame layout changes in a way an
/// older reader can't skip past.
const MOVIE_FORMAT_VERSION: u16 = 1;

/// One frame's worth of recorded controller input — the SNES auto-joypad's latched `u16` per
/// port (the same value [`crate::bus::Bus::set_joypad`] takes).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameInput {
    /// Player 1's latched controller state.
    pub p1: u16,
    /// Player 2's latched controller state.
    pub p2: u16,
}

/// Where a movie's replay begins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartPoint {
    /// Boot fresh from the cart's reset vector (`System::reset`). The caller is expected to have
    /// already constructed `System::new(movie.seed)` with the movie's ROM installed and never
    /// stepped it — [`Movie::seek_to_start`] calls `reset()` to boot it from there.
    PowerOn,
    /// Restore this embedded save-state blob (a branch point mid-recording), via
    /// `System::load_state`.
    SaveState(Vec<u8>),
}

/// A recorded TAS movie: a deterministic input log plus everything needed to reproduce the exact
/// power-on/branch-point state it was recorded against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    /// The `System::new` seed this recording used (power-on phase alignment,
    /// `docs/adr/0004`) — irrelevant for a [`StartPoint::SaveState`] start (the blob already
    /// carries its own seed), but always recorded for a uniform format.
    pub seed: u64,
    /// The cart's region at recording time.
    pub region: Region,
    /// SHA-256 of the exact ROM byte image recorded against — the authoritative "is this the
    /// right ROM" check on replay, independent of the cart's internal parsed representation.
    pub rom_sha256: [u8; 32],
    /// Where replay begins.
    pub start: StartPoint,
    /// The recorded per-frame input log, oldest first.
    pub frames: Vec<FrameInput>,
}

/// Errors from decoding or replaying a movie.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum MovieError {
    /// The blob's leading magic bytes didn't match — not a RustySNES movie at all.
    #[error("not a RustySNES movie (bad magic)")]
    BadMagic,
    /// The format version is newer than this build understands.
    #[error("unsupported movie format version {found} (this build supports up to {max})")]
    UnsupportedVersion {
        /// The version found in the blob.
        found: u16,
        /// The newest version this build supports.
        max: u16,
    },
    /// The start-point kind byte was neither 0 (power-on) nor 1 (save-state).
    #[error("unrecognized movie start-point kind {0}")]
    BadStartPointKind(u8),
    /// `Movie::verify_rom` was called with bytes that don't hash to this movie's recorded
    /// `rom_sha256` — replaying against the wrong ROM would not reproduce the recording.
    #[error("ROM does not match this movie's recorded ROM (wrong ROM loaded)")]
    RomMismatch,
    /// [`Movie::seek_to_start`] was called for a [`StartPoint::PowerOn`] movie against a
    /// `System` whose seed doesn't match the movie's recorded seed — replay would diverge from
    /// the very first frame (different power-on phase alignment), not just eventually.
    #[error("System seed {found} does not match this movie's recorded seed {expected}")]
    SeedMismatch {
        /// The seed the movie was recorded with.
        expected: u64,
        /// The seed the `System` passed to `seek_to_start` was actually constructed with.
        found: u64,
    },
    /// The embedded save-state failed to decode/restore.
    #[error("embedded save-state: {0}")]
    SaveState(#[from] SaveStateError),
    /// The buffer ended before the expected number of bytes were available.
    #[error("truncated movie data")]
    Truncated,
}

/// SHA-256 of `rom`, in the form [`Movie::rom_sha256`] / [`Movie::verify_rom`] compare against.
#[must_use]
pub fn hash_rom(rom: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(rom);
    hasher.finalize().into()
}

impl Movie {
    /// Verify `rom` is the exact byte image this movie was recorded against.
    ///
    /// # Errors
    /// [`MovieError::RomMismatch`] if the hash doesn't match.
    pub fn verify_rom(&self, rom: &[u8]) -> Result<(), MovieError> {
        if hash_rom(rom) == self.rom_sha256 {
            Ok(())
        } else {
            Err(MovieError::RomMismatch)
        }
    }

    /// Put `sys` into this movie's recorded starting position, ready for
    /// [`MoviePlayer::next_frame`] + [`System::run_frame`] to replay the input log.
    ///
    /// For [`StartPoint::PowerOn`], `sys` MUST already be a freshly-constructed
    /// `System::new(self.seed)` with the movie's ROM installed and never yet stepped — this
    /// verifies the seed matches (a mismatch cannot possibly replay identically) and calls
    /// `System::reset()` to boot it. For [`StartPoint::SaveState`], this restores the embedded
    /// blob via `System::load_state` (which carries its own seed).
    ///
    /// Callers should call [`Self::verify_rom`] separately before this — `sys`/`System` retain no
    /// raw ROM bytes to hash against, so the ROM-identity check happens at the byte-image level
    /// the caller already has (e.g. the frontend's retained ROM buffer).
    ///
    /// # Errors
    /// [`MovieError::SeedMismatch`] if `sys`'s seed doesn't match a `PowerOn` movie's recorded
    /// seed; [`MovieError::SaveState`] if an embedded save-state fails to decode/restore.
    pub fn seek_to_start(&self, sys: &mut System) -> Result<(), MovieError> {
        match &self.start {
            StartPoint::PowerOn => {
                if sys.seed() != self.seed {
                    return Err(MovieError::SeedMismatch {
                        expected: self.seed,
                        found: sys.seed(),
                    });
                }
                sys.reset();
                Ok(())
            }
            StartPoint::SaveState(blob) => {
                sys.load_state(blob)?;
                Ok(())
            }
        }
    }

    /// Serialize this movie to its on-disk byte format (see the module doc for the layout).
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut w = SaveWriter::new();
        w.write_bytes(MAGIC);
        w.write_u16(MOVIE_FORMAT_VERSION);
        w.write_u8(match self.region {
            Region::Ntsc => 0,
            Region::Pal => 1,
        });
        w.write_u64(self.seed);
        w.write_bytes(&self.rom_sha256);
        w.write_u32(u32::try_from(self.frames.len()).unwrap_or(u32::MAX));
        match &self.start {
            StartPoint::PowerOn => w.write_u8(0),
            StartPoint::SaveState(blob) => {
                w.write_u8(1);
                w.write_len_prefixed(blob);
            }
        }
        for f in &self.frames {
            w.write_u16(f.p1);
            w.write_u16(f.p2);
        }
        w.into_bytes()
    }

    /// The inverse of [`Self::serialize`].
    ///
    /// # Errors
    /// [`MovieError::BadMagic`] if `bytes` doesn't lead with the expected magic;
    /// [`MovieError::UnsupportedVersion`] if the format version is newer than this build
    /// understands; [`MovieError::BadStartPointKind`] on a corrupt start-point tag;
    /// [`MovieError::Truncated`] on truncated/corrupt input.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, MovieError> {
        let mut r = SaveReader::new(bytes);
        if r.read_bytes(8).map_err(|_| MovieError::Truncated)? != MAGIC {
            return Err(MovieError::BadMagic);
        }
        let version = r.read_u16().map_err(|_| MovieError::Truncated)?;
        if version > MOVIE_FORMAT_VERSION {
            return Err(MovieError::UnsupportedVersion {
                found: version,
                max: MOVIE_FORMAT_VERSION,
            });
        }
        let region = match r.read_u8().map_err(|_| MovieError::Truncated)? {
            1 => Region::Pal,
            _ => Region::Ntsc,
        };
        let seed = r.read_u64().map_err(|_| MovieError::Truncated)?;
        let rom_sha256: [u8; 32] = r
            .read_bytes(32)
            .map_err(|_| MovieError::Truncated)?
            .try_into()
            .unwrap_or([0; 32]);
        let frame_count = r.read_u32().map_err(|_| MovieError::Truncated)? as usize;
        let start = match r.read_u8().map_err(|_| MovieError::Truncated)? {
            0 => StartPoint::PowerOn,
            1 => {
                let blob = r.read_len_prefixed().map_err(|_| MovieError::Truncated)?;
                StartPoint::SaveState(blob.to_vec())
            }
            other => return Err(MovieError::BadStartPointKind(other)),
        };
        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let p1 = r.read_u16().map_err(|_| MovieError::Truncated)?;
            let p2 = r.read_u16().map_err(|_| MovieError::Truncated)?;
            frames.push(FrameInput { p1, p2 });
        }
        Ok(Self {
            seed,
            region,
            rom_sha256,
            start,
            frames,
        })
    }
}

/// Records a movie frame-by-frame as the emulator runs.
#[derive(Debug)]
pub struct MovieRecorder {
    movie: Movie,
}

impl MovieRecorder {
    /// Start recording from a power-on. `rom` is the exact byte image that was (or is about to
    /// be) loaded — hashed for the recorded movie's ROM-identity check on replay.
    #[must_use]
    pub fn power_on(seed: u64, region: Region, rom: &[u8]) -> Self {
        Self {
            movie: Movie {
                seed,
                region,
                rom_sha256: hash_rom(rom),
                start: StartPoint::PowerOn,
                frames: Vec::new(),
            },
        }
    }

    /// Start recording from `sys`'s current live state (a branch point mid-session) — embeds a
    /// full save-state as the start point. `rom` is the exact byte image `sys`'s cart was loaded
    /// from.
    #[must_use]
    pub fn from_current_state(region: Region, rom: &[u8], sys: &System) -> Self {
        Self {
            movie: Movie {
                seed: sys.seed(),
                region,
                rom_sha256: hash_rom(rom),
                start: StartPoint::SaveState(sys.save_state()),
                frames: Vec::new(),
            },
        }
    }

    /// Capture this frame's about-to-be-consumed input. Call BEFORE [`System::run_frame`] —
    /// this records exactly what that call will consume, matching [`MoviePlayer::next_frame`]'s
    /// own "apply, then run" order on replay.
    pub fn capture(&mut self, p1: u16, p2: u16) {
        self.movie.frames.push(FrameInput { p1, p2 });
    }

    /// The number of frames captured so far.
    #[must_use]
    pub const fn frame_count(&self) -> usize {
        self.movie.frames.len()
    }

    /// Consume the recorder, returning the finished [`Movie`].
    #[must_use]
    pub fn finish(self) -> Movie {
        self.movie
    }
}

/// Replays a recorded [`Movie`]'s input log against a `System` already positioned at its start
/// point (via [`Movie::seek_to_start`]).
///
/// Owns the [`Movie`] (rather than borrowing it) specifically so a long-lived host — the
/// frontend's per-frame drive, holding a player across many real frames — can store one without
/// a self-referential lifetime; [`Self::movie`] hands it back if the caller needs it afterward
/// (e.g. to re-verify the ROM hash, or to inspect how many frames were recorded).
#[derive(Debug)]
pub struct MoviePlayer {
    movie: Movie,
    index: usize,
}

impl MoviePlayer {
    /// A player starting at the first recorded frame.
    #[must_use]
    pub const fn new(movie: Movie) -> Self {
        Self { movie, index: 0 }
    }

    /// The movie being played back.
    #[must_use]
    pub const fn movie(&self) -> &Movie {
        &self.movie
    }

    /// Advance the playback cursor and return the next recorded frame's input, or `None` if the
    /// movie is exhausted (the caller should stop).
    ///
    /// Deliberately does NOT touch a `System`/`Bus` itself (unlike an earlier design) — a host
    /// that drives input through its own abstraction (e.g. `rustysnes-frontend`'s `EmuCore::
    /// set_pad`, which `EmuCore::run_frame` re-applies from its OWN retained pad state every
    /// call) needs to feed the returned [`FrameInput`] through THAT abstraction, not have this
    /// reach around it and write `Bus::set_joypad` directly — the two would race for who "wins"
    /// depending on call order. A caller working with a bare `System` directly (as the
    /// determinism-replay test does) can just call `sys.bus.set_joypad(0, f.p1)` /
    /// `set_joypad(1, f.p2)` itself with the returned value.
    pub fn next_frame(&mut self) -> Option<FrameInput> {
        let f = *self.movie.frames.get(self.index)?;
        self.index += 1;
        Some(f)
    }

    /// Frames not yet applied.
    #[must_use]
    pub const fn frames_remaining(&self) -> usize {
        self.movie.frames.len() - self.index
    }

    /// Whether every recorded frame has been applied.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.index >= self.movie.frames.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_movie() -> Movie {
        Movie {
            seed: 42,
            region: Region::Ntsc,
            rom_sha256: hash_rom(b"fake rom bytes"),
            start: StartPoint::PowerOn,
            frames: alloc::vec![
                FrameInput { p1: 0x8000, p2: 0 },
                FrameInput {
                    p1: 0x0000,
                    p2: 0x4000
                },
                FrameInput {
                    p1: 0xFFFF,
                    p2: 0xFFFF
                },
            ],
        }
    }

    #[test]
    fn format_round_trip_power_on() {
        let movie = tiny_movie();
        let bytes = movie.serialize();
        let decoded = Movie::deserialize(&bytes).expect("round-trips");
        assert_eq!(decoded, movie);
    }

    #[test]
    fn format_round_trip_with_save_state_start() {
        let mut movie = tiny_movie();
        movie.start = StartPoint::SaveState(alloc::vec![1, 2, 3, 4, 5]);
        let bytes = movie.serialize();
        let decoded = Movie::deserialize(&bytes).expect("round-trips");
        assert_eq!(decoded, movie);
    }

    #[test]
    fn deserialize_rejects_bad_magic_cleanly() {
        let bytes = alloc::vec![0u8; 64];
        assert_eq!(Movie::deserialize(&bytes), Err(MovieError::BadMagic));
    }

    #[test]
    fn deserialize_rejects_truncated_data_cleanly() {
        let movie = tiny_movie();
        let bytes = movie.serialize();
        // Chop off the last frame's worth of bytes.
        let truncated = &bytes[..bytes.len() - 2];
        assert_eq!(Movie::deserialize(truncated), Err(MovieError::Truncated));
    }

    #[test]
    fn verify_rom_accepts_matching_and_rejects_different_bytes() {
        let movie = tiny_movie();
        assert!(movie.verify_rom(b"fake rom bytes").is_ok());
        assert_eq!(
            movie.verify_rom(b"a different rom"),
            Err(MovieError::RomMismatch)
        );
    }

    #[test]
    fn recorder_and_player_round_trip_the_same_inputs() {
        let mut rec = MovieRecorder::power_on(7, Region::Ntsc, b"rom");
        let inputs = [(0x8000u16, 0u16), (0, 0x4000), (0xFFFF, 0xFFFF)];
        for &(p1, p2) in &inputs {
            rec.capture(p1, p2);
        }
        assert_eq!(rec.frame_count(), 3);
        let movie = rec.finish();

        let mut player = MoviePlayer::new(movie);
        let mut sys = System::new(7);
        let mut seen = alloc::vec::Vec::new();
        while let Some(f) = player.next_frame() {
            sys.bus.set_joypad(0, f.p1);
            sys.bus.set_joypad(1, f.p2);
            seen.push((sys.bus.joypad(0), sys.bus.joypad(1)));
        }
        assert!(player.is_finished());
        assert_eq!(player.frames_remaining(), 0);
        assert_eq!(seen.as_slice(), inputs.as_slice());
    }

    #[test]
    fn seek_to_start_power_on_rejects_seed_mismatch() {
        let movie = tiny_movie(); // seed 42
        let mut sys = System::new(43); // wrong seed
        assert_eq!(
            movie.seek_to_start(&mut sys),
            Err(MovieError::SeedMismatch {
                expected: 42,
                found: 43,
            })
        );
    }

    #[test]
    fn seek_to_start_power_on_boots_with_matching_seed() {
        let movie = tiny_movie(); // seed 42
        let mut sys = System::new(42);
        assert!(movie.seek_to_start(&mut sys).is_ok());
    }
}
