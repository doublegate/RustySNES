//! `rustysnes-savestate` — the shared save-state wire-format primitives (savestate).
//!
//! A leaf crate in the one-directional chip-crate graph (see `docs/architecture.md`): every
//! chip crate (`rustysnes-cpu`/`-ppu`/`-apu`/`-cart`) and `rustysnes-core` depend on this for a
//! single, versioned binary format, per `docs/adr/0006-save-state-format.md`. No dependents
//! among the chip crates — this only ever gets DEPENDED ON, keeping the graph acyclic.
//!
//! Deliberately no `serde`/reflection: every `Board`/`Cpu`/`Ppu`/`Apu` implementation writes an
//! explicit `save_state`/`load_state` pair using the primitives here (ADR 0006's "no derive
//! magic" decision — keeps `#![no_std]` targets byte-identical to native and keeps each
//! component's on-disk format change local + auditable, matching this project's existing style
//! for the `Board` trait itself).
//!
//! # Format
//!
//! This crate defines the **section framing** a save-state's payload is built from — a full
//! save-state additionally has a top-level header (magic bytes + format version + crate-version
//! string, ADR 0006) that `rustysnes-core::System::save_state`/`load_state` writes/checks before
//! ever touching a section; that envelope is out of scope here (see [`SaveStateError::BadMagic`]/
//! [`SaveStateError::UnsupportedVersion`], which exist for that caller to use).
//!
//! A section is **tagged and length-prefixed** — a 4-byte ASCII tag, a `u32` little-endian byte
//! length, then that many bytes of section-defined content. Wrapping a component's state as one
//! such section (via [`SaveWriter::section`] / [`SaveReader::section`]) is what lets a newer
//! format skip a section it doesn't recognize (a bumped-version load) rather than corrupting the
//! rest of the load, and lets a struct that changed its own internal layout stay self-describing
//! without a central schema registry.

#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use thiserror::Error;

/// Errors from decoding a save-state.
///
/// `docs/adr/0006`'s "reject loudly on an unrecognized/corrupt format, never silently truncate
/// or zero-fill" honesty posture — the same posture `docs/adr/0003` already applies to
/// coprocessor accuracy.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum SaveStateError {
    /// The buffer ended before the expected number of bytes were available.
    #[error("truncated save-state data (expected {expected} more bytes, {available} available)")]
    Truncated {
        /// Bytes the read attempted to consume.
        expected: usize,
        /// Bytes actually remaining in the buffer.
        available: usize,
    },
    /// A section's declared tag didn't match what the caller expected at this point in the
    /// format — either genuine corruption, or a format version whose section order changed.
    #[error("unexpected section tag: expected {expected:?}, found {found:?}")]
    UnexpectedTag {
        /// The 4-byte ASCII tag the caller expected.
        expected: [u8; 4],
        /// The 4-byte ASCII tag actually present.
        found: [u8; 4],
    },
    /// The blob's leading magic bytes didn't match — not a RustySNES save-state at all.
    #[error("not a RustySNES save-state (bad magic)")]
    BadMagic,
    /// The blob's format-version major number is newer than this build understands.
    #[error("save-state format version {found} is newer than this build supports (max {max})")]
    UnsupportedVersion {
        /// The format major version found in the blob.
        found: u16,
        /// The highest format major version this build can load.
        max: u16,
    },
    /// A component rejected the decoded bytes as semantically invalid (e.g. an enum discriminant
    /// with no matching variant) even though the section framing itself was well-formed.
    #[error("invalid save-state content: {0}")]
    Invalid(String),
}

/// A growable little-endian binary writer over an in-memory buffer (`#![no_std]` + `alloc`, no
/// `std::io::Write` — every chip crate this feeds is `no_std`).
#[derive(Debug, Default)]
pub struct SaveWriter {
    buf: Vec<u8>,
}

impl SaveWriter {
    /// A fresh, empty writer.
    #[must_use]
    pub const fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Consume the writer, returning the assembled bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// The bytes written so far (for a nested writer being spliced into a parent — see
    /// [`Self::section`]).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Append raw bytes verbatim (for a component embedding another's already-serialized bytes,
    /// e.g. a coprocessor's own sub-engine).
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Write one byte.
    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    /// Write a `bool` as one byte (`0`/`1`).
    pub fn write_bool(&mut self, v: bool) {
        self.write_u8(u8::from(v));
    }

    /// Write a little-endian `u16`.
    pub fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write a little-endian `u32`.
    pub fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write a little-endian `u64`.
    pub fn write_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write a byte slice prefixed with its `u32` length (for a variable-length payload, e.g. an
    /// SRAM image — distinct from [`Self::section`], which additionally carries a 4-byte tag for
    /// forward-compatible skipping; a length-prefixed blob has no tag because its meaning is
    /// implied by its fixed position in the caller's own section).
    pub fn write_len_prefixed(&mut self, bytes: &[u8]) {
        #[allow(clippy::cast_possible_truncation)] // save-state payloads never approach 4 GiB
        self.write_u32(bytes.len() as u32);
        self.write_bytes(bytes);
    }

    /// Write a nested, self-describing section: a 4-byte ASCII `tag`, a `u32` byte length, then
    /// whatever `body` writes. `tag` should be a short, stable, human-legible identifier (e.g.
    /// `*b"CPU0"`, `*b"BRD1"`) — stable across releases even if the section's internal layout
    /// changes, since [`SaveReader::section`] uses it to skip a section it doesn't recognize.
    ///
    /// `body` writes directly into this writer's own buffer (no nested `Vec` allocation — a
    /// save-state gets created and restored repeatedly during rewind/run-ahead, so allocating one
    /// throwaway buffer per (possibly nested) section on every such call would add up); the length
    /// header is a zero placeholder written up front and patched in place once `body` returns and
    /// the section's true length is known.
    pub fn section(&mut self, tag: [u8; 4], body: impl FnOnce(&mut Self)) {
        self.write_bytes(&tag);
        let len_pos = self.buf.len();
        self.write_u32(0); // placeholder, patched below
        let start = self.buf.len();
        body(self);
        let len = self.buf.len() - start;
        #[allow(clippy::cast_possible_truncation)] // save-state sections never approach 4 GiB
        self.buf[len_pos..len_pos + 4].copy_from_slice(&(len as u32).to_le_bytes());
    }
}

/// A little-endian binary cursor reader over a borrowed byte slice.
///
/// Bounds-checked reads return [`SaveStateError::Truncated`] instead of panicking on
/// malformed/corrupt input (external, untrusted data — a save-state file the user can hand-edit
/// or a corrupted disk write) — "validate at boundaries" / "never `unwrap` on untrusted data"
/// (project-wide house rules, `master-core/modules/60-security.md`).
#[derive(Debug, Clone, Copy)]
pub struct SaveReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> SaveReader<'a> {
    /// A reader positioned at the start of `buf`.
    #[must_use]
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Bytes remaining unread.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], SaveStateError> {
        if self.remaining() < n {
            return Err(SaveStateError::Truncated {
                expected: n,
                available: self.remaining(),
            });
        }
        let out = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    /// Read raw bytes verbatim (the counterpart to [`SaveWriter::write_bytes`] for a component
    /// that embeds another's already-serialized bytes at a caller-known fixed length).
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if fewer than `n` bytes remain.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], SaveStateError> {
        self.take(n)
    }

    /// Read one byte.
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if the buffer is exhausted.
    pub fn read_u8(&mut self) -> Result<u8, SaveStateError> {
        Ok(self.take(1)?[0])
    }

    /// Read a `bool` (any nonzero byte is `true`, matching how `write_bool` only ever emits 0/1
    /// but a hand-corrupted file might not — never a hard error for either value).
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if the buffer is exhausted.
    pub fn read_bool(&mut self) -> Result<bool, SaveStateError> {
        Ok(self.read_u8()? != 0)
    }

    /// Read a little-endian `u16`.
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if fewer than 2 bytes remain.
    pub fn read_u16(&mut self) -> Result<u16, SaveStateError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    /// Read a little-endian `u32`.
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if fewer than 4 bytes remain.
    pub fn read_u32(&mut self) -> Result<u32, SaveStateError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Read a little-endian `u64`.
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if fewer than 8 bytes remain.
    pub fn read_u64(&mut self) -> Result<u64, SaveStateError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    /// Read a `u32`-length-prefixed byte slice (the counterpart to
    /// [`SaveWriter::write_len_prefixed`]).
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if the declared length exceeds what remains.
    pub fn read_len_prefixed(&mut self) -> Result<&'a [u8], SaveStateError> {
        let len = self.read_u32()? as usize;
        self.take(len)
    }

    /// Read a nested section written by [`SaveWriter::section`]: consumes its 4-byte tag + `u32`
    /// length, and returns `(tag, sub_reader)` scoped to exactly that section's bytes — reading
    /// past the sub-reader's end fails with [`SaveStateError::Truncated`] even if the OUTER
    /// buffer has more data, which is what makes an unrecognized/truncated-by-a-bug section fail
    /// locally instead of desynchronizing every section after it.
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] if the tag/length header or the declared body is truncated.
    pub fn section(&mut self) -> Result<([u8; 4], Self), SaveStateError> {
        let tag_bytes = self.take(4)?;
        let tag = [tag_bytes[0], tag_bytes[1], tag_bytes[2], tag_bytes[3]];
        let body = self.read_len_prefixed()?;
        Ok((tag, Self::new(body)))
    }

    /// [`Self::section`], additionally checking the tag matches `expected` — the common case
    /// where the caller knows exactly which section should come next (most of the format is a
    /// fixed sequence; only the top-level `Board`-family dispatch genuinely branches on tag).
    ///
    /// # Errors
    /// [`SaveStateError::Truncated`] per [`Self::section`], or [`SaveStateError::UnexpectedTag`]
    /// if the next section's tag doesn't match `expected`.
    pub fn expect_section(&mut self, expected: [u8; 4]) -> Result<Self, SaveStateError> {
        let (found, reader) = self.section()?;
        if found == expected {
            Ok(reader)
        } else {
            Err(SaveStateError::UnexpectedTag { expected, found })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_round_trip() {
        let mut w = SaveWriter::new();
        w.write_u8(0xAB);
        w.write_bool(true);
        w.write_u16(0x1234);
        w.write_u32(0xDEAD_BEEF);
        w.write_u64(0x1122_3344_5566_7788);
        w.write_len_prefixed(&[1, 2, 3]);
        let bytes = w.into_bytes();

        let mut r = SaveReader::new(&bytes);
        assert_eq!(r.read_u8().unwrap(), 0xAB);
        assert!(r.read_bool().unwrap());
        assert_eq!(r.read_u16().unwrap(), 0x1234);
        assert_eq!(r.read_u32().unwrap(), 0xDEAD_BEEF);
        assert_eq!(r.read_u64().unwrap(), 0x1122_3344_5566_7788);
        assert_eq!(r.read_len_prefixed().unwrap(), &[1, 2, 3]);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn truncated_read_errors_instead_of_panicking() {
        let mut r = SaveReader::new(&[0x01, 0x02]);
        assert_eq!(
            r.read_u32(),
            Err(SaveStateError::Truncated {
                expected: 4,
                available: 2
            })
        );
    }

    #[test]
    fn nested_sections_round_trip_and_stay_scoped() {
        let mut w = SaveWriter::new();
        w.section(*b"AAAA", |s| s.write_u32(1));
        w.section(*b"BBBB", |s| s.write_u32(2));
        let bytes = w.into_bytes();

        let mut r = SaveReader::new(&bytes);
        let mut a = r.expect_section(*b"AAAA").unwrap();
        assert_eq!(a.read_u32().unwrap(), 1);
        assert_eq!(a.remaining(), 0);
        // Reading past a sub-section's own bound fails locally...
        assert!(a.read_u8().is_err());
        // ...without desynchronizing the outer reader's position for the next section.
        let mut b = r.expect_section(*b"BBBB").unwrap();
        assert_eq!(b.read_u32().unwrap(), 2);
    }

    #[test]
    fn unexpected_tag_is_reported_not_silently_accepted() {
        let mut w = SaveWriter::new();
        w.section(*b"AAAA", |s| s.write_u8(1));
        let bytes = w.into_bytes();
        let mut r = SaveReader::new(&bytes);
        assert_eq!(
            r.expect_section(*b"ZZZZ").unwrap_err(),
            SaveStateError::UnexpectedTag {
                expected: *b"ZZZZ",
                found: *b"AAAA"
            }
        );
    }
}
