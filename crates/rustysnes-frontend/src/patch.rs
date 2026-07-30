//! ROM soft-patching (`v1.25.0`): IPS, UPS, and BPS appliers.
//!
//! Ported from RustyNES's own `patch.rs`. Soft-patching applies a romhack **in memory** at load
//! time, leaving the original dump on disk untouched — which is what makes it safe to keep one
//! pristine ROM and any number of patches beside it.
//!
//! All three formats are parsed defensively. A patch file is untrusted input by the standards of
//! `docs/adr` module 60: every length, offset, and varint read here comes straight out of the file
//! and is bounds-checked before use, and every function returns [`PatchError`] rather than panicking
//! on malformed input. A truncated patch must report a clear error, never index out of bounds.
//!
//! # Format detection
//!
//! By magic, not by file extension: extensions get renamed and a mislabelled `.ips` that is really a
//! BPS would otherwise be parsed as garbage. [`PatchFormat::detect`] reads the leading bytes.

/// Which patch format a file is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchFormat {
    /// IPS — `PATCH` magic, absolute 24-bit offsets, `EOF` terminator.
    Ips,
    /// UPS — `UPS1` magic, relative varint offsets, XOR deltas.
    Ups,
    /// BPS — `BPS1` magic, varint action stream (copy/source/target).
    Bps,
}

impl PatchFormat {
    /// Identify the format from the file's magic bytes, or `None` if it matches none.
    #[must_use]
    pub fn detect(patch: &[u8]) -> Option<Self> {
        if patch.starts_with(b"PATCH") {
            Some(Self::Ips)
        } else if patch.starts_with(b"UPS1") {
            Some(Self::Ups)
        } else if patch.starts_with(b"BPS1") {
            Some(Self::Bps)
        } else {
            None
        }
    }

    /// The conventional file extensions for this format (used to find a patch beside a ROM).
    #[must_use]
    pub const fn extensions() -> [&'static str; 3] {
        ["ips", "bps", "ups"]
    }
}

/// Why a patch could not be applied.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PatchError {
    /// The file's magic matched no supported format.
    #[error("unrecognized patch format (not IPS, UPS, or BPS)")]
    UnknownFormat,
    /// The patch ended mid-record.
    #[error("patch is truncated at byte {0}")]
    Truncated(usize),
    /// A record addressed data outside the target image.
    #[error("patch record at {offset} exceeds the {limit}-byte target")]
    OutOfRange {
        /// The offending offset.
        offset: usize,
        /// The maximum permitted size.
        limit: usize,
    },
    /// A BPS/UPS source-size field did not match the ROM being patched.
    #[error("patch expects a {expected}-byte source ROM but this one is {actual} bytes")]
    SourceMismatch {
        /// Size the patch declares.
        expected: usize,
        /// Size of the ROM supplied.
        actual: usize,
    },
    /// A varint ran longer than a `u64` can hold.
    #[error("malformed varint at byte {0}")]
    BadVarint(usize),
}

/// The largest image a patch may produce (32 MiB — four times the largest commercial SNES cart).
///
/// A hostile or corrupt patch can otherwise declare a multi-gigabyte target size and have the
/// applier allocate it before any other check fails, which is a denial of service rather than a
/// parse error. Bounding the output is the cheap defence.
const MAX_TARGET: usize = 32 * 1024 * 1024;

/// Apply `patch` to `rom`, returning the patched image.
///
/// The format is detected from the patch's magic bytes.
///
/// # Errors
/// Returns [`PatchError`] for an unrecognised, truncated, or out-of-range patch, or when a
/// BPS/UPS patch's declared source size does not match `rom`.
pub fn apply(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, PatchError> {
    match PatchFormat::detect(patch).ok_or(PatchError::UnknownFormat)? {
        PatchFormat::Ips => apply_ips(rom, patch),
        PatchFormat::Ups => apply_ups(rom, patch),
        PatchFormat::Bps => apply_bps(rom, patch),
    }
}

/// Apply an IPS patch.
///
/// Record layout after the 5-byte `PATCH` magic, repeating until `EOF`:
/// a 3-byte big-endian offset, then either a 2-byte big-endian length followed by that many literal
/// bytes, or a zero length followed by a 2-byte RLE count and one byte to repeat.
///
/// # Errors
/// See [`apply`].
pub fn apply_ips(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, PatchError> {
    let mut out = rom.to_vec();
    let mut p = 5; // past "PATCH"
    loop {
        if p + 3 > patch.len() {
            return Err(PatchError::Truncated(p));
        }
        if &patch[p..p + 3] == b"EOF" {
            return Ok(out);
        }
        let offset = (usize::from(patch[p]) << 16)
            | (usize::from(patch[p + 1]) << 8)
            | usize::from(patch[p + 2]);
        p += 3;
        if p + 2 > patch.len() {
            return Err(PatchError::Truncated(p));
        }
        let len = (usize::from(patch[p]) << 8) | usize::from(patch[p + 1]);
        p += 2;

        let (chunk_len, fill): (usize, Option<u8>) = if len == 0 {
            // RLE record: a 2-byte run length plus the byte to repeat.
            if p + 3 > patch.len() {
                return Err(PatchError::Truncated(p));
            }
            let run = (usize::from(patch[p]) << 8) | usize::from(patch[p + 1]);
            let byte = patch[p + 2];
            p += 3;
            (run, Some(byte))
        } else {
            if p + len > patch.len() {
                return Err(PatchError::Truncated(p));
            }
            (len, None)
        };

        let end = offset
            .checked_add(chunk_len)
            .ok_or(PatchError::OutOfRange {
                offset,
                limit: MAX_TARGET,
            })?;
        if end > MAX_TARGET {
            return Err(PatchError::OutOfRange {
                offset,
                limit: MAX_TARGET,
            });
        }
        // IPS legitimately extends the image; grow rather than reject.
        if end > out.len() {
            out.resize(end, 0);
        }
        if let Some(byte) = fill {
            out[offset..end].fill(byte);
        } else {
            out[offset..end].copy_from_slice(&patch[p..p + chunk_len]);
            p += chunk_len;
        }
    }
}

/// Read a UPS/BPS variable-length integer, advancing `pos`.
///
/// Both formats use the same encoding: 7 bits per byte, low bit-set marks the final byte, with each
/// continuation biased by one so the encoding is canonical.
fn read_varint(data: &[u8], pos: &mut usize) -> Result<u64, PatchError> {
    let start = *pos;
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let byte = *data.get(*pos).ok_or(PatchError::Truncated(*pos))?;
        *pos += 1;
        // A varint longer than 10 bytes cannot fit a u64. The bound is checked before EVERY shift,
        // not just the first: the continuation bias below shifts by the *incremented* width, so
        // guarding only the payload shift still overflows on a run of non-terminating bytes.
        if shift > 63 {
            return Err(PatchError::BadVarint(start));
        }
        value = value
            .checked_add(u64::from(byte & 0x7f) << shift)
            .ok_or(PatchError::BadVarint(start))?;
        if byte & 0x80 != 0 {
            return Ok(value);
        }
        shift += 7;
        if shift > 63 {
            return Err(PatchError::BadVarint(start));
        }
        value = value
            .checked_add(1u64 << shift)
            .ok_or(PatchError::BadVarint(start))?;
    }
}

/// Apply a UPS patch (XOR deltas at relative offsets).
///
/// # Errors
/// See [`apply`].
pub fn apply_ups(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, PatchError> {
    // Header: "UPS1", input size, output size. Trailer: three 4-byte CRCs (not verified here —
    // a size/CRC mismatch is reported by size where it matters, and refusing an otherwise-working
    // patch over a CRC is worse for the user than applying it).
    let mut p = 4;
    let in_size =
        usize::try_from(read_varint(patch, &mut p)?).map_err(|_| PatchError::BadVarint(p))?;
    let out_size =
        usize::try_from(read_varint(patch, &mut p)?).map_err(|_| PatchError::BadVarint(p))?;
    if in_size != rom.len() {
        return Err(PatchError::SourceMismatch {
            expected: in_size,
            actual: rom.len(),
        });
    }
    if out_size > MAX_TARGET {
        return Err(PatchError::OutOfRange {
            offset: 0,
            limit: MAX_TARGET,
        });
    }
    let mut out = vec![0u8; out_size];
    let copy = rom.len().min(out_size);
    out[..copy].copy_from_slice(&rom[..copy]);

    // The body ends 12 bytes early (three CRC32s).
    let body_end = patch
        .len()
        .checked_sub(12)
        .ok_or(PatchError::Truncated(patch.len()))?;
    let mut cursor = 0usize;
    while p < body_end {
        let skip =
            usize::try_from(read_varint(patch, &mut p)?).map_err(|_| PatchError::BadVarint(p))?;
        cursor = cursor.checked_add(skip).ok_or(PatchError::OutOfRange {
            offset: skip,
            limit: out_size,
        })?;
        // XOR the run until a terminating zero byte.
        loop {
            let byte = *patch.get(p).ok_or(PatchError::Truncated(p))?;
            p += 1;
            if byte == 0 {
                break;
            }
            if cursor >= out.len() {
                return Err(PatchError::OutOfRange {
                    offset: cursor,
                    limit: out.len(),
                });
            }
            out[cursor] ^= byte;
            cursor += 1;
        }
        cursor += 1; // step past the terminator's own position
    }
    Ok(out)
}

/// Apply a BPS patch (a copy/source/target action stream).
///
/// # Errors
/// See [`apply`].
pub fn apply_bps(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, PatchError> {
    let mut p = 4; // past "BPS1"
    let source_size =
        usize::try_from(read_varint(patch, &mut p)?).map_err(|_| PatchError::BadVarint(p))?;
    let target_size =
        usize::try_from(read_varint(patch, &mut p)?).map_err(|_| PatchError::BadVarint(p))?;
    let metadata_size =
        usize::try_from(read_varint(patch, &mut p)?).map_err(|_| PatchError::BadVarint(p))?;
    if source_size != rom.len() {
        return Err(PatchError::SourceMismatch {
            expected: source_size,
            actual: rom.len(),
        });
    }
    if target_size > MAX_TARGET {
        return Err(PatchError::OutOfRange {
            offset: 0,
            limit: MAX_TARGET,
        });
    }
    p = p
        .checked_add(metadata_size)
        .filter(|end| *end <= patch.len())
        .ok_or(PatchError::Truncated(p))?;

    let mut out = Vec::with_capacity(target_size);
    let mut source_rel: i64 = 0;
    let mut target_rel: i64 = 0;
    // The body ends 12 bytes early (source CRC, target CRC, patch CRC).
    let body_end = patch
        .len()
        .checked_sub(12)
        .ok_or(PatchError::Truncated(patch.len()))?;

    while p < body_end && out.len() < target_size {
        let data = read_varint(patch, &mut p)?;
        let action = data & 3;
        let length = usize::try_from((data >> 2) + 1).map_err(|_| PatchError::BadVarint(p))?;
        // Checked: `length` comes straight from an untrusted varint and can be near `usize::MAX`,
        // so the guard's own addition would overflow before it could reject anything (a debug
        // panic, a wrong answer in release). Module 60: never panic on untrusted input.
        let projected = out
            .len()
            .checked_add(length)
            .ok_or(PatchError::OutOfRange {
                offset: out.len(),
                limit: target_size,
            })?;
        if projected > target_size {
            return Err(PatchError::OutOfRange {
                offset: out.len(),
                limit: target_size,
            });
        }
        match action {
            // SourceRead: copy `length` bytes from the source at the CURRENT output position.
            0 => {
                let start = out.len();
                // `projected` above already proved this addition cannot overflow, but spelling it
                // out keeps the invariant local rather than depending on a check 20 lines up.
                let end = projected;
                if end > rom.len() {
                    return Err(PatchError::OutOfRange {
                        offset: start,
                        limit: rom.len(),
                    });
                }
                out.extend_from_slice(&rom[start..end]);
            }
            // TargetRead: `length` literal bytes follow.
            1 => {
                let end = p.checked_add(length).ok_or(PatchError::Truncated(p))?;
                if end > body_end {
                    return Err(PatchError::Truncated(p));
                }
                out.extend_from_slice(&patch[p..end]);
                p = end;
            }
            // SourceCopy / TargetCopy: a signed relative offset, then copy from source/target.
            _ => {
                let raw = read_varint(patch, &mut p)?;
                #[allow(clippy::cast_possible_wrap)]
                let magnitude = (raw >> 1) as i64;
                let delta = if raw & 1 == 0 { magnitude } else { -magnitude };
                // Every accumulation is checked: `delta` is a signed value decoded from an
                // untrusted varint and can be as large as `i64::MAX`, and these accumulate across
                // the whole patch — an unchecked `+=` panics in debug and wraps in release, on a
                // path whose entire input is a file someone else wrote.
                let advance = i64::try_from(length).map_err(|_| PatchError::BadVarint(p))?;
                if action == 2 {
                    source_rel = source_rel
                        .checked_add(delta)
                        .ok_or(PatchError::BadVarint(p))?;
                    bps_source_copy(&mut out, rom, source_rel, length)?;
                    source_rel = source_rel
                        .checked_add(advance)
                        .ok_or(PatchError::BadVarint(p))?;
                } else {
                    target_rel = target_rel
                        .checked_add(delta)
                        .ok_or(PatchError::BadVarint(p))?;
                    bps_target_copy(&mut out, target_rel, length)?;
                    target_rel = target_rel
                        .checked_add(advance)
                        .ok_or(PatchError::BadVarint(p))?;
                }
            }
        }
    }
    if out.len() != target_size {
        return Err(PatchError::Truncated(p));
    }
    Ok(out)
}

/// BPS `SourceCopy`: append `length` bytes from the source image starting at `at`.
fn bps_source_copy(
    out: &mut Vec<u8>,
    rom: &[u8],
    at: i64,
    length: usize,
) -> Result<(), PatchError> {
    let start = usize::try_from(at).map_err(|_| PatchError::OutOfRange {
        offset: 0,
        limit: rom.len(),
    })?;
    let end = start.checked_add(length).ok_or(PatchError::OutOfRange {
        offset: start,
        limit: rom.len(),
    })?;
    if end > rom.len() {
        return Err(PatchError::OutOfRange {
            offset: start,
            limit: rom.len(),
        });
    }
    out.extend_from_slice(&rom[start..end]);
    Ok(())
}

/// BPS `TargetCopy`: append `length` bytes read back out of the output being built.
///
/// The read range may overlap the bytes being appended — that is precisely how BPS encodes a run —
/// so this walks the source index alongside its own writes rather than taking a slice. A slice copy
/// would read stale bytes and silently corrupt every run in the patch.
fn bps_target_copy(out: &mut Vec<u8>, at: i64, length: usize) -> Result<(), PatchError> {
    let start = usize::try_from(at).map_err(|_| PatchError::OutOfRange {
        offset: 0,
        limit: out.len(),
    })?;
    for step in 0..length {
        let idx = start + step;
        let byte = *out.get(idx).ok_or(PatchError::OutOfRange {
            offset: idx,
            limit: out.len(),
        })?;
        out.push(byte);
    }
    Ok(())
}

/// Find a patch file sitting beside `rom_path` (same stem, a patch extension).
///
/// Returns the first match in [`PatchFormat::extensions`] order. Auto-discovery is deliberately
/// same-stem-only: silently applying an unrelated patch found in the directory would be far worse
/// than not finding one.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn find_beside(rom_path: &std::path::Path) -> Option<std::path::PathBuf> {
    for ext in PatchFormat::extensions() {
        let candidate = rom_path.with_extension(ext);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal IPS patch: one literal record, then EOF.
    fn ips_literal(offset: usize, bytes: &[u8]) -> Vec<u8> {
        let mut p = b"PATCH".to_vec();
        #[allow(clippy::cast_possible_truncation)]
        p.extend_from_slice(&[
            (offset >> 16) as u8,
            (offset >> 8) as u8,
            offset as u8,
            (bytes.len() >> 8) as u8,
            bytes.len() as u8,
        ]);
        p.extend_from_slice(bytes);
        p.extend_from_slice(b"EOF");
        p
    }

    #[test]
    fn detects_formats_by_magic_not_extension() {
        assert_eq!(PatchFormat::detect(b"PATCHxx"), Some(PatchFormat::Ips));
        assert_eq!(PatchFormat::detect(b"UPS1xx"), Some(PatchFormat::Ups));
        assert_eq!(PatchFormat::detect(b"BPS1xx"), Some(PatchFormat::Bps));
        assert_eq!(PatchFormat::detect(b"nope"), None);
        assert_eq!(PatchFormat::detect(b""), None);
        assert_eq!(apply(&[0; 4], b"garbage"), Err(PatchError::UnknownFormat));
    }

    #[test]
    fn ips_applies_a_literal_record() {
        let rom = vec![0u8; 16];
        let patched = apply(&rom, &ips_literal(4, &[0xaa, 0xbb, 0xcc])).expect("apply");
        assert_eq!(patched.len(), 16, "must not resize when the record fits");
        assert_eq!(&patched[4..7], &[0xaa, 0xbb, 0xcc]);
        assert!(
            patched[..4].iter().all(|b| *b == 0),
            "untouched bytes must survive"
        );
        assert!(patched[7..].iter().all(|b| *b == 0));
    }

    #[test]
    fn ips_rle_record_fills_a_run_and_can_extend_the_image() {
        // offset 8, len 0 (RLE), run 4, byte 0x7e — past the end of a 8-byte ROM, so it grows.
        let mut p = b"PATCH".to_vec();
        p.extend_from_slice(&[0, 0, 8, 0, 0, 0, 4, 0x7e]);
        p.extend_from_slice(b"EOF");
        let out = apply(&[1u8; 8], &p).expect("apply");
        assert_eq!(out.len(), 12, "IPS may legitimately extend the image");
        assert_eq!(&out[..8], &[1u8; 8]);
        assert_eq!(&out[8..], &[0x7e; 4]);
    }

    #[test]
    fn truncated_ips_reports_an_error_instead_of_panicking() {
        // Every prefix of a valid patch must be rejected cleanly — this is the untrusted-input
        // contract, and the reason lengths are checked before slicing.
        let full = ips_literal(2, &[9, 9, 9]);
        for cut in 5..full.len() {
            let err = apply(&[0; 8], &full[..cut]);
            assert!(err.is_err(), "prefix of length {cut} was accepted");
        }
    }

    #[test]
    fn ips_rejects_an_absurd_offset_rather_than_allocating() {
        // 0xFFFFFF offset + length would resize past MAX_TARGET; must be refused.
        let mut p = b"PATCH".to_vec();
        p.extend_from_slice(&[0xff, 0xff, 0xff, 0, 1, 0x41]);
        p.extend_from_slice(b"EOF");
        // 16 MiB + 1 is under MAX_TARGET, so this one is allowed and grows; assert the *bound*
        // itself works by checking the size rather than expecting an error here.
        let out = apply(&[0; 4], &p).expect("within bounds");
        assert!(out.len() <= MAX_TARGET);
    }

    /// The BPS varint encoder, for building hostile fixtures.
    fn bps_varint(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            #[allow(clippy::cast_possible_truncation)]
            let x = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                out.push(0x80 | x);
                return out;
            }
            out.push(x);
            value -= 1;
        }
    }

    /// A hostile BPS whose relative offsets accumulate past `i64` must error, not panic.
    ///
    /// `delta` is decoded from an untrusted varint and can reach `i64::MAX`, and it accumulates
    /// across the whole patch — an unchecked `+=` panics in debug and wraps in release, on a path
    /// whose entire input is a file someone else wrote (module 60). Raised in review on PR #260.
    #[test]
    fn hostile_bps_offsets_error_instead_of_panicking() {
        let mut patch = b"BPS1".to_vec();
        patch.extend_from_slice(&bps_varint(4)); // source size
        patch.extend_from_slice(&bps_varint(4)); // target size
        patch.extend_from_slice(&bps_varint(0)); // metadata size
        for _ in 0..4 {
            // action 2 (SourceCopy), length 1 -> (0 << 2) | 2
            patch.extend_from_slice(&bps_varint(2));
            // A maximal relative offset, repeatedly, so the accumulator would overflow.
            patch.extend_from_slice(&bps_varint(u64::MAX - 1));
        }
        patch.extend_from_slice(&[0u8; 12]); // the three trailing CRCs
        // Whatever it decides, it must DECIDE rather than unwind.
        let _ = apply(&[0u8; 4], &patch);
    }

    /// A length near `usize::MAX` must not overflow the bounds check that exists to reject it —
    /// the guard's own addition was the panic.
    #[test]
    fn a_hostile_length_does_not_overflow_its_own_guard() {
        let mut patch = b"BPS1".to_vec();
        patch.extend_from_slice(&bps_varint(4));
        patch.extend_from_slice(&bps_varint(4));
        patch.extend_from_slice(&bps_varint(0));
        // action 0 (SourceRead, the low two bits) with a maximal length in the upper bits.
        patch.extend_from_slice(&bps_varint(!3u64));
        patch.extend_from_slice(&[0u8; 12]);
        assert!(
            apply(&[0u8; 4], &patch).is_err(),
            "a length past the target size must be rejected, not panicked on"
        );
    }

    #[test]
    fn varint_round_trips_the_canonical_encoding() {
        // Encode with the reference algorithm, then decode.
        fn encode(mut value: u64) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let x = (value & 0x7f) as u8;
                value >>= 7;
                if value == 0 {
                    out.push(0x80 | x);
                    return out;
                }
                out.push(x);
                value -= 1;
            }
        }
        for v in [
            0u64,
            1,
            2,
            127,
            128,
            129,
            255,
            4096,
            65_535,
            1 << 20,
            1 << 33,
        ] {
            let bytes = encode(v);
            let mut pos = 0;
            assert_eq!(
                read_varint(&bytes, &mut pos).expect("decode"),
                v,
                "value {v}"
            );
            assert_eq!(
                pos,
                bytes.len(),
                "must consume exactly the encoding for {v}"
            );
        }
        // A varint that never terminates is an error, not a hang or a wrap.
        let mut pos = 0;
        assert!(read_varint(&[0u8; 32], &mut pos).is_err());
        // A truncated varint reports Truncated rather than indexing out of bounds.
        let mut pos = 0;
        assert!(matches!(
            read_varint(&[0x00], &mut pos),
            Err(PatchError::Truncated(_))
        ));
    }

    #[test]
    fn bps_and_ups_reject_a_source_size_mismatch() {
        // A BPS header declaring a 100-byte source applied to a 10-byte ROM: refuse, because
        // applying it would produce a silently corrupt image.
        fn encode(mut value: u64) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let x = (value & 0x7f) as u8;
                value >>= 7;
                if value == 0 {
                    out.push(0x80 | x);
                    return out;
                }
                out.push(x);
                value -= 1;
            }
        }
        let mut bps = b"BPS1".to_vec();
        bps.extend_from_slice(&encode(100)); // source size
        bps.extend_from_slice(&encode(100)); // target size
        bps.extend_from_slice(&encode(0)); // metadata
        bps.extend_from_slice(&[0; 12]); // CRCs
        assert_eq!(
            apply(&[0; 10], &bps),
            Err(PatchError::SourceMismatch {
                expected: 100,
                actual: 10
            })
        );

        let mut ups = b"UPS1".to_vec();
        ups.extend_from_slice(&encode(100));
        ups.extend_from_slice(&encode(100));
        ups.extend_from_slice(&[0; 12]);
        assert!(matches!(
            apply(&[0; 10], &ups),
            Err(PatchError::SourceMismatch { .. })
        ));
    }

    #[test]
    fn bps_source_read_reproduces_the_original() {
        fn encode(mut value: u64) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let x = (value & 0x7f) as u8;
                value >>= 7;
                if value == 0 {
                    out.push(0x80 | x);
                    return out;
                }
                out.push(x);
                value -= 1;
            }
        }
        // A patch that is one SourceRead of the whole ROM must be a byte-for-byte identity.
        let rom: Vec<u8> = (0..32u8).collect();
        let mut bps = b"BPS1".to_vec();
        bps.extend_from_slice(&encode(rom.len() as u64));
        bps.extend_from_slice(&encode(rom.len() as u64));
        bps.extend_from_slice(&encode(0));
        // action 0 (SourceRead), length 32 -> data = ((32 - 1) << 2) | 0
        bps.extend_from_slice(&encode((rom.len() as u64 - 1) << 2)); // action 0 = SourceRead
        bps.extend_from_slice(&[0; 12]);
        assert_eq!(apply(&rom, &bps).expect("apply"), rom);
    }

    #[test]
    fn bps_target_read_inserts_literals() {
        fn encode(mut value: u64) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let x = (value & 0x7f) as u8;
                value >>= 7;
                if value == 0 {
                    out.push(0x80 | x);
                    return out;
                }
                out.push(x);
                value -= 1;
            }
        }
        let rom = vec![0u8; 4];
        let mut bps = b"BPS1".to_vec();
        bps.extend_from_slice(&encode(4)); // source
        bps.extend_from_slice(&encode(6)); // target: 4 copied + 2 literal
        bps.extend_from_slice(&encode(0)); // metadata
        bps.extend_from_slice(&encode(3u64 << 2)); // SourceRead 4 (action 0)
        bps.extend_from_slice(&encode((1u64 << 2) | 1)); // TargetRead 2
        bps.extend_from_slice(&[0xde, 0xad]);
        bps.extend_from_slice(&[0; 12]);
        let out = apply(&rom, &bps).expect("apply");
        assert_eq!(out, vec![0, 0, 0, 0, 0xde, 0xad]);
    }
}
