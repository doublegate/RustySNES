//! Multi-slot, thumbnail-previewed save states (`v1.0.0`), disk-backed alongside the pre-existing
//! RAM-only `quick_save` single slot (`app.rs`, untouched by this module).
//!
//! Slots are keyed per-ROM by `rustysnes_core::movie::hash_rom`'s SHA-256 (the same identity
//! movies already key on), mirroring RustyNES's own `<data_dir>/saves/<rom_sha256_hex>/slotN`
//! layout. Unlike RustyNES's approach (a core-level `THM ` thumbnail section embedded inside the
//! `.rns` blob itself), the thumbnail here lives in a small frontend-only wrapper AROUND the
//! unmodified `EmuCore::save_state()` bytes, so `rustysnes-savestate`'s `FORMAT_VERSION`
//! (currently `3`, `docs/adr/0006`) is untouched by this feature — a save-state slot's core
//! payload is byte-identical to what `EmuCore::save_state()`/`load_state()` already produce and
//! consume.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

/// Number of save-state slots per ROM (`0..NUM_SLOTS`).
pub const NUM_SLOTS: u8 = 10;

/// Thumbnail width (RGBA8, nearest-neighbor downsample of the live framebuffer at save time).
pub const THUMB_W: u32 = 128;
/// Thumbnail height (RGBA8, nearest-neighbor downsample of the live framebuffer at save time).
pub const THUMB_H: u32 = 112;

const MAGIC: &[u8; 4] = b"RSST";
const FORMAT_VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 2 + 2;

/// One save-state slot's metadata, read WITHOUT restoring it into the running core.
#[derive(Debug, Clone, Default)]
pub struct SlotMeta {
    /// The slot file's last-modified time, or `None` if empty/unreadable.
    pub modified: Option<SystemTime>,
    /// The stored thumbnail (`width, height, RGBA8 bytes`), or `None` if the slot is empty or the
    /// file fails to parse.
    pub thumbnail: Option<(u16, u16, Vec<u8>)>,
}

impl SlotMeta {
    /// Whether this slot holds a save (as opposed to `Default::default()`'s "never saved").
    #[must_use]
    pub const fn occupied(&self) -> bool {
        self.modified.is_some()
    }
}

/// The platform save-state directory (`<data_dir>/saves`), or `None` where no data dir is
/// resolvable (always `None` on `wasm32` — no filesystem).
#[must_use]
pub fn base_dir() -> Option<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        directories::ProjectDirs::from("io.github", "doublegate", "RustySNES")
            .map(|d| d.data_dir().join("saves"))
    }
}

/// Hex-encode a 32-byte hash (lowercase, no separators) — the shared ROM-hash-to-directory-name
/// convention every per-ROM-keyed disk feature uses (save-state slots here, HD texture packs in
/// `crate::hd_pack`).
pub(crate) fn hex(hash: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    hash.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// The path for `(rom_sha256, slot)`, or `None` if no data dir is resolvable or `slot` is out of
/// range.
#[must_use]
pub fn slot_path(rom_sha256: &[u8; 32], slot: u8) -> Option<PathBuf> {
    if slot >= NUM_SLOTS {
        return None;
    }
    base_dir().map(|dir| dir.join(hex(rom_sha256)).join(format!("slot{slot}.rsst")))
}

/// Persist `state` (an `EmuCore::save_state()` blob) to `slot`, wrapped with the
/// `thumb_w x thumb_h` RGBA8 `thumb_rgba` thumbnail.
///
/// # Errors
/// Returns an I/O error on write failure, or if no data directory is resolvable (always the case
/// on `wasm32`).
pub fn save_to_slot(
    rom_sha256: &[u8; 32],
    slot: u8,
    thumb_w: u16,
    thumb_h: u16,
    thumb_rgba: &[u8],
    state: &[u8],
) -> io::Result<()> {
    let path = slot_path(rom_sha256, slot)
        .ok_or_else(|| io::Error::other("no writable data directory"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut buf = Vec::with_capacity(HEADER_LEN + thumb_rgba.len() + state.len());
    buf.extend_from_slice(MAGIC);
    buf.push(FORMAT_VERSION);
    buf.extend_from_slice(&thumb_w.to_le_bytes());
    buf.extend_from_slice(&thumb_h.to_le_bytes());
    buf.extend_from_slice(thumb_rgba);
    buf.extend_from_slice(state);
    fs::write(&path, buf)
}

/// Read `slot`'s state bytes back out (for `EmuCore::load_state`), discarding the thumbnail.
///
/// # Errors
/// Returns an I/O error if the slot is empty/unreadable/unresolvable, or malformed (a foreign or
/// truncated file).
pub fn load_from_slot(rom_sha256: &[u8; 32], slot: u8) -> io::Result<Vec<u8>> {
    let path = slot_path(rom_sha256, slot)
        .ok_or_else(|| io::Error::other("no readable data directory"))?;
    let bytes = fs::read(&path)?;
    parse(&bytes)
        .map(|(_, _, _, state)| state.to_vec())
        .ok_or_else(|| io::Error::other("malformed save-state slot file"))
}

/// Read `slot`'s metadata (thumbnail + mtime) without restoring it.
///
/// Returns [`SlotMeta::default`] (empty/unoccupied) for a missing, unresolvable, or unparseable
/// slot, rather than erroring — an unreadable slot renders as "empty" in the manager grid, not a
/// crash.
#[must_use]
pub fn slot_meta(rom_sha256: &[u8; 32], slot: u8) -> SlotMeta {
    let Some(path) = slot_path(rom_sha256, slot) else {
        return SlotMeta::default();
    };
    let Ok(bytes) = fs::read(&path) else {
        return SlotMeta::default();
    };
    let modified = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    let thumbnail = parse(&bytes).map(|(w, h, thumb, _)| (w, h, thumb.to_vec()));
    SlotMeta {
        modified,
        thumbnail,
    }
}

/// Parse the wrapper header, returning `(thumb_w, thumb_h, thumb_bytes, state_bytes)` as slices
/// into `data`, or `None` if the magic/version/length don't check out.
fn parse(data: &[u8]) -> Option<(u16, u16, &[u8], &[u8])> {
    if data.len() < HEADER_LEN || data[0..4] != *MAGIC || data[4] != FORMAT_VERSION {
        return None;
    }
    let w = u16::from_le_bytes([data[5], data[6]]);
    let h = u16::from_le_bytes([data[7], data[8]]);
    let thumb_len = usize::from(w) * usize::from(h) * 4;
    let thumb_end = HEADER_LEN.checked_add(thumb_len)?;
    if data.len() < thumb_end {
        return None;
    }
    Some((w, h, &data[HEADER_LEN..thumb_end], &data[thumb_end..]))
}

/// Nearest-neighbor RGBA8 resize from `(src_w, src_h)` to `(dst_w, dst_h)`.
///
/// Used to shrink the live framebuffer down to a small save-slot thumbnail; no image-crate
/// dependency for such a simple, non-interpolated resize (the dependency-light-core convention).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn nearest_resize(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return out;
    }
    for y in 0..dst_h {
        let sy = (y * src_h) / dst_h;
        for x in 0..dst_w {
            let sx = (x * src_w) / dst_w;
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((y * dst_w + x) * 4) as usize;
            if si + 4 <= src.len() && di + 4 <= out.len() {
                out[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn nearest_resize_preserves_solid_color() {
        let src_w = 4;
        let src_h = 4;
        let mut src = vec![0u8; (src_w * src_h * 4) as usize];
        for px in src.chunks_exact_mut(4) {
            px.copy_from_slice(&[10, 20, 30, 255]);
        }
        let out = nearest_resize(&src, src_w, src_h, 2, 2);
        assert_eq!(out.len(), 2 * 2 * 4);
        for px in out.chunks_exact(4) {
            assert_eq!(px, [10, 20, 30, 255]);
        }
    }

    #[test]
    fn nearest_resize_empty_src_is_blank_not_a_panic() {
        let out = nearest_resize(&[], 0, 0, 4, 4);
        assert_eq!(out, vec![0u8; 4 * 4 * 4]);
    }

    #[test]
    fn slot_meta_missing_slot_is_unoccupied() {
        let meta = SlotMeta::default();
        assert!(!meta.occupied());
        assert!(meta.thumbnail.is_none());
    }

    #[test]
    fn parse_round_trips_thumb_and_state() {
        let thumb = vec![1u8, 2, 3, 4, 5, 6, 7, 8]; // a 1x2 RGBA8 thumbnail
        let state = b"pretend-save-state-bytes".to_vec();
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.push(FORMAT_VERSION);
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&thumb);
        buf.extend_from_slice(&state);

        let (w, hgt, got_thumb, got_state) = parse(&buf).expect("well-formed buffer parses");
        assert_eq!((w, hgt), (1, 2));
        assert_eq!(got_thumb, thumb.as_slice());
        assert_eq!(got_state, state.as_slice());
    }

    #[test]
    fn parse_rejects_bad_magic_and_truncated_data() {
        assert!(parse(b"not-a-slot-file-at-all").is_none());
        let mut short = Vec::new();
        short.extend_from_slice(MAGIC);
        short.push(FORMAT_VERSION);
        assert!(parse(&short).is_none(), "truncated header must not parse");
    }

    #[test]
    fn slot_path_rejects_out_of_range_slot() {
        assert!(slot_path(&h(0), NUM_SLOTS).is_none());
        assert!(slot_path(&h(0), NUM_SLOTS - 1).is_some() || base_dir().is_none());
    }
}
