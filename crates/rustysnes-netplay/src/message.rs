//! The netplay wire protocol — hand-rolled, little-endian, tag-byte-discriminated (no serde
//! dependency, matching RustyNES's `rustynes-netplay::message` this crate ports its shape from).
//!
//! Every [`NetMessage`] round-trips through [`NetMessage::encode`]/[`NetMessage::decode`]
//! byte-for-byte; `decode` rejects truncated/malformed input rather than panicking (untrusted
//! network input, `master-core` module 60's input-validation rule).

/// The protocol version this build speaks — bumped whenever the wire format changes so two
/// mismatched builds fail the [`NetMessage::Sync`] handshake cleanly instead of misinterpreting
/// bytes.
pub const PROTOCOL_VERSION: u16 = 1;

/// [`NetMessage::Sync`]'s magic value — identifies a peer as speaking this protocol at all,
/// before the ROM-hash/version fields are even trusted. `RSNP` (RustySNES Netplay) in ASCII.
pub const SYNC_MAGIC: u32 = 0x5253_4E50;

/// A message exchanged between two netplay peers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetMessage {
    /// The connection handshake: proves both peers speak this protocol version and are loaded
    /// on the identical ROM (`rom_hash`, SHA-256) before any input is trusted.
    Sync {
        /// Must equal [`SYNC_MAGIC`].
        magic: u32,
        /// Must equal [`PROTOCOL_VERSION`].
        version: u16,
        /// The sender's loaded ROM's SHA-256 hash.
        rom_hash: [u8; 32],
    },
    /// One player's input for one frame.
    Input {
        /// Which controller slot this input is for (`0` or `1` — the SNES core has exactly two
        /// physical controller ports; multitap is not emulated, so netplay is scoped to 2
        /// players, matching the core's own capability).
        player: u8,
        /// The frame this input applies to.
        frame: u32,
        /// The raw 16-bit button state (`Bus::set_joypad`'s own format).
        input: u16,
    },
    /// Cumulative input acknowledgement: "I have every input up to and including `frame`,
    /// contiguously" — NOT "the highest frame I've seen," so a dropped low frame keeps getting
    /// resent even after later frames arrive out of order.
    InputAck {
        /// The highest frame acknowledged as part of a contiguous run from frame 0.
        frame: u32,
    },
    /// A periodic desync-detection checksum for one frame's post-execution state.
    Checksum {
        /// The frame this checksum was taken after.
        frame: u32,
        /// A hash of the full `System::save_state()` blob — catches a pure-timing/audio
        /// divergence a framebuffer-only hash might miss (see `fb_hash` below).
        hash: u64,
        /// A hash of the framebuffer alone — isolates whether a mismatch is a rendered-output
        /// divergence specifically, distinct from an audio/timing-only divergence.
        fb_hash: u64,
    },
    /// A lightweight, non-critical connection-quality signal (never gates correctness).
    Quality {
        /// Measured round-trip time, milliseconds.
        ping_ms: u32,
        /// This peer's frame count minus the last frame it has confirmed from the other peer —
        /// how far ahead (positive) or behind (negative) this peer is running.
        frame_advantage: i32,
    },
}

/// Error decoding a [`NetMessage`] from untrusted bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The buffer ended before a complete message could be read.
    #[error("truncated netplay message")]
    Truncated,
    /// The leading tag byte didn't match any known [`NetMessage`] variant.
    #[error("unrecognized netplay message tag {0}")]
    UnknownTag(u8),
}

const TAG_SYNC: u8 = 0;
const TAG_INPUT: u8 = 1;
const TAG_INPUT_ACK: u8 = 2;
const TAG_CHECKSUM: u8 = 3;
const TAG_QUALITY: u8 = 4;

impl NetMessage {
    /// Serialize this message to its wire format.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            Self::Sync {
                magic,
                version,
                rom_hash,
            } => {
                buf.push(TAG_SYNC);
                buf.extend_from_slice(&magic.to_le_bytes());
                buf.extend_from_slice(&version.to_le_bytes());
                buf.extend_from_slice(rom_hash);
            }
            Self::Input {
                player,
                frame,
                input,
            } => {
                buf.push(TAG_INPUT);
                buf.push(*player);
                buf.extend_from_slice(&frame.to_le_bytes());
                buf.extend_from_slice(&input.to_le_bytes());
            }
            Self::InputAck { frame } => {
                buf.push(TAG_INPUT_ACK);
                buf.extend_from_slice(&frame.to_le_bytes());
            }
            Self::Checksum {
                frame,
                hash,
                fb_hash,
            } => {
                buf.push(TAG_CHECKSUM);
                buf.extend_from_slice(&frame.to_le_bytes());
                buf.extend_from_slice(&hash.to_le_bytes());
                buf.extend_from_slice(&fb_hash.to_le_bytes());
            }
            Self::Quality {
                ping_ms,
                frame_advantage,
            } => {
                buf.push(TAG_QUALITY);
                buf.extend_from_slice(&ping_ms.to_le_bytes());
                buf.extend_from_slice(&frame_advantage.to_le_bytes());
            }
        }
        buf
    }

    /// Deserialize a message from `bytes` (the exact output of a prior [`Self::encode`], or
    /// arbitrary untrusted network input — this never panics on malformed data).
    ///
    /// # Errors
    /// Returns [`DecodeError::Truncated`] if `bytes` ends before a complete message is read, or
    /// [`DecodeError::UnknownTag`] if the leading tag byte is unrecognized.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader { bytes, pos: 0 };
        let tag = r.u8()?;
        match tag {
            TAG_SYNC => Ok(Self::Sync {
                magic: r.u32()?,
                version: r.u16()?,
                rom_hash: r.bytes32()?,
            }),
            TAG_INPUT => Ok(Self::Input {
                player: r.u8()?,
                frame: r.u32()?,
                input: r.u16()?,
            }),
            TAG_INPUT_ACK => Ok(Self::InputAck { frame: r.u32()? }),
            TAG_CHECKSUM => Ok(Self::Checksum {
                frame: r.u32()?,
                hash: r.u64()?,
                fb_hash: r.u64()?,
            }),
            TAG_QUALITY => Ok(Self::Quality {
                ping_ms: r.u32()?,
                frame_advantage: r.i32()?,
            }),
            other => Err(DecodeError::UnknownTag(other)),
        }
    }
}

/// A minimal cursor-based little-endian reader over untrusted bytes — every accessor bounds-checks
/// before reading, returning [`DecodeError::Truncated`] rather than panicking or reading OOB.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], DecodeError> {
        let end = self.pos.checked_add(n).ok_or(DecodeError::Truncated)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(DecodeError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn bytes32(&mut self) -> Result<[u8; 32], DecodeError> {
        Ok(self.take(32)?.try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(msg: &NetMessage) {
        let bytes = msg.encode();
        assert_eq!(&NetMessage::decode(&bytes).unwrap(), msg);
    }

    #[test]
    fn every_variant_round_trips() {
        round_trip(&NetMessage::Sync {
            magic: SYNC_MAGIC,
            version: PROTOCOL_VERSION,
            rom_hash: [0x42; 32],
        });
        round_trip(&NetMessage::Input {
            player: 1,
            frame: 12345,
            input: 0x8421,
        });
        round_trip(&NetMessage::InputAck { frame: 999 });
        round_trip(&NetMessage::Checksum {
            frame: 42,
            hash: 0xDEAD_BEEF_CAFE_F00D,
            fb_hash: 0x1234_5678_9ABC_DEF0,
        });
        round_trip(&NetMessage::Quality {
            ping_ms: 30,
            frame_advantage: -3,
        });
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = NetMessage::Input {
            player: 0,
            frame: 1,
            input: 1,
        }
        .encode();
        for len in 0..full.len() {
            assert_eq!(
                NetMessage::decode(&full[..len]),
                Err(DecodeError::Truncated),
                "length {len} should be truncated"
            );
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        assert_eq!(
            NetMessage::decode(&[0xFF, 0, 0, 0]),
            Err(DecodeError::UnknownTag(0xFF))
        );
    }

    #[test]
    fn decode_rejects_empty_input() {
        assert_eq!(NetMessage::decode(&[]), Err(DecodeError::Truncated));
    }
}
