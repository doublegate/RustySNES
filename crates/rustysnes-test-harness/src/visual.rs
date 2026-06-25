//! Visual-golden / `.snap` comparator — the SNES port of RustyNES's framebuffer-hash +
//! screenshot diff.
//!
//! Hashes a rendered frame and diffs it against the committed `tests/golden/` + `screenshots/`
//! corpus (insta-style `.snap` snapshots). A frame-hash mismatch flags a regression and the
//! screenshot diff localizes it. SKELETON: the SNES frame is 256×224 (NTSC, hi-res / overscan
//! variants are TODO); the hash is a placeholder until a real framebuffer exists.

/// A stable hash of one rendered frame, used as the `.snap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHash(pub u64);

/// SNES base visible resolution (NTSC). Hi-res (512-wide) + overscan (239-tall) are TODO.
pub const FRAME_WIDTH: usize = 256;
/// SNES base visible scanlines (NTSC).
pub const FRAME_HEIGHT: usize = 224;

/// Hash a frame buffer (`FRAME_WIDTH * FRAME_HEIGHT` packed RGBA pixels). FNV-1a placeholder;
/// the real hash + format track the frontend's framebuffer layout.
#[must_use]
pub fn hash_frame(rgba: &[u8]) -> FrameHash {
    // TODO(T-04): match the frontend framebuffer pixel format once it exists.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in rgba {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    FrameHash(h)
}

/// Compare a freshly hashed frame against the expected golden hash.
#[must_use]
pub fn compare_snapshot(actual: FrameHash, expected: FrameHash) -> bool {
    actual == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let buf = vec![0x11u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        assert_eq!(hash_frame(&buf), hash_frame(&buf));
    }

    #[test]
    fn distinct_frames_differ() {
        let a = [0u8; 16];
        let b = [1u8; 16];
        assert_ne!(hash_frame(&a), hash_frame(&b));
        assert!(compare_snapshot(hash_frame(&a), hash_frame(&a)));
        assert!(!compare_snapshot(hash_frame(&a), hash_frame(&b)));
    }
}
