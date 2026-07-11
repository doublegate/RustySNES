//! HD texture pack tile-identity hashing (`v1.3.0`, `hd-pack` feature).
//!
//! The core's only job for HD texture packs (per `docs/adr/0010` and Mesen2's own NES HD Pack
//! system, the direct architectural precedent) is to compute a **tile-identity hash** the
//! frontend can use as a cache key — it never loads a pack, matches a hash against one, or
//! composites a replacement texture itself (that stays entirely frontend-side, keeping the core
//! pack-agnostic per `docs/adr/0004`'s determinism boundary).
//!
//! Every replaceable unit is a single 8×8 character (BG 16×16 tiles decompose into 4 independent
//! 8×8 fetches, OBJ always iterates 8-pixel columns, Mode 7 is a map of 8×8 characters — there is
//! no 16×16 special case).
//!
//! # Hashing spec
//!
//! [`hash_tile`] is XXH3-64 (fast, non-cryptographic — this is a cache key, not a security
//! boundary; the same family Dolphin's own texture-replacement system converges on independently)
//! over a fixed byte sequence:
//!
//! 1. A 1-byte [`TileClass`] discriminant.
//! 2. A 1-byte bits-per-pixel value.
//! 3. The tile's raw **pre-flip** VRAM words, little-endian byte order.
//! 4. The resolved effective palette (`2^bpp` CGRAM colors) actually used by this instance,
//!    little-endian byte order.
//!
//! Flip is deliberately excluded from the hash — both orientations of the same tile share one
//! pack entry, and the frontend mirrors the source rect at composite time instead. This is
//! **palette-inclusive** (the same bitmap combined with two different palettes hashes to two
//! different entries) — matching Mesen2's own `HdTileKey` precedent exactly, trading storage for
//! zero recoloring math in the render hot path.

use alloc::vec::Vec;

/// Which PPU render path produced a tile.
///
/// Included in the hash (not just its own bookkeeping field) because the SAME
/// `(tile_addr, bpp, palette)` triple can coincidentally arise from two different hardware paths
/// (e.g. a BG tile and an OBJ tile sharing VRAM real estate) that a texture pack should still be
/// able to target independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TileClass {
    /// A background-layer character (BG1-4, any mode).
    Bg = 0,
    /// A sprite (OBJ) character.
    Obj = 1,
    /// A Mode 7 affine-map character.
    Mode7 = 2,
}

/// One composited pixel's tile-identity tag — the PPU-side write-only recording hook's per-pixel
/// output element.
///
/// `Ppu::tile_tags()` returns a side-buffer of these, sized/indexed exactly like
/// `Ppu::framebuffer()`, populated only when `Ppu::set_hd_pack_tagging(true)` is active (off by
/// default; see that method's doc for the byte-identical-when-off guarantee). `hash == 0` means
/// "no tagged tile wrote this pixel" (the backdrop, or tagging was off) — `hash_tile`'s output
/// space is 64-bit and effectively never legitimately collides with `0`, so this is a safe sentinel
/// rather than a separate `Option` discriminant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TileTag {
    /// [`hash_tile`]'s output for this pixel's source tile, or `0` (see the struct doc).
    pub hash: u64,
    /// Horizontal flip of the source tile instance at this pixel. Excluded from the hash itself
    /// (both orientations share one entry — see this module's doc) but needed by the frontend
    /// compositor to mirror the replacement source rect correctly.
    pub hflip: bool,
    /// Vertical flip of the source tile instance at this pixel.
    pub vflip: bool,
}

/// Compute the palette-inclusive tile-identity hash for one 8×8 character.
///
/// `tile_words` is the tile's raw pre-flip VRAM data: `bpp * 4` words for a BG/OBJ character
/// (the standard SNES bitplane-pair layout, matching `rustysnes_ppu`'s own internal tile-word
/// stride), or 64 words for a Mode 7 character (a fixed 8bpp block). `palette` is the resolved
/// `2^bpp` CGRAM colors this specific instance uses (already resolved to the correct palette
/// group/base — see `docs/ppu.md` for how a BG's palette group folds into the CGRAM index).
///
/// Callers pass whatever slice length is correct for their class/bpp; this function does not
/// validate the length itself (the caller — `Ppu`'s own render paths — always knows the exact
/// expected size for the class/bpp it's hashing, so a length mismatch would be a caller bug, not
/// malformed external data).
#[must_use]
pub fn hash_tile(class: TileClass, bpp: u8, tile_words: &[u16], palette: &[u16]) -> u64 {
    let mut buf = Vec::with_capacity(2 + tile_words.len() * 2 + palette.len() * 2);
    buf.push(class as u8);
    buf.push(bpp);
    for &w in tile_words {
        buf.extend_from_slice(&w.to_le_bytes());
    }
    for &c in palette {
        buf.extend_from_slice(&c.to_le_bytes());
    }
    xxhash_rust::xxh3::xxh3_64(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn same_input_hashes_identically() {
        let words = [0x1234u16, 0x5678, 0xABCD, 0xEF01];
        let palette = [0x0000u16, 0x7FFFu16];
        let a = hash_tile(TileClass::Bg, 2, &words, &palette);
        let b = hash_tile(TileClass::Bg, 2, &words, &palette);
        assert_eq!(a, b);
    }

    #[test]
    fn different_class_hashes_differently() {
        let words = [0x1234u16, 0x5678];
        let palette = [0x0000u16];
        let bg = hash_tile(TileClass::Bg, 2, &words, &palette);
        let obj = hash_tile(TileClass::Obj, 2, &words, &palette);
        let m7 = hash_tile(TileClass::Mode7, 2, &words, &palette);
        assert_ne!(bg, obj);
        assert_ne!(bg, m7);
        assert_ne!(obj, m7);
    }

    #[test]
    fn different_bpp_hashes_differently() {
        let words = [0x1234u16, 0x5678];
        let palette = [0x0000u16];
        let a = hash_tile(TileClass::Bg, 2, &words, &palette);
        let b = hash_tile(TileClass::Bg, 4, &words, &palette);
        assert_ne!(a, b);
    }

    #[test]
    fn different_tile_bytes_hash_differently() {
        let palette = [0x0000u16];
        let a = hash_tile(TileClass::Bg, 2, &[0x1234, 0x5678], &palette);
        let b = hash_tile(TileClass::Bg, 2, &[0x1234, 0x5679], &palette);
        assert_ne!(a, b);
    }

    #[test]
    fn different_palette_hashes_differently() {
        // Palette-inclusive: the identical bitmap under two different palettes must hash
        // differently (this is the whole point of the "palette-inclusive" design choice).
        let words = [0x1234u16, 0x5678];
        let a = hash_tile(TileClass::Bg, 2, &words, &[0x0000, 0x7FFF]);
        let b = hash_tile(TileClass::Bg, 2, &words, &[0x001F, 0x7FFF]);
        assert_ne!(a, b);
    }

    #[test]
    fn empty_slices_do_not_panic() {
        // A degenerate but well-defined input (e.g. bpp=0, no tile data) must not panic --
        // this is internal PPU bookkeeping, not untrusted external data, but the function
        // should still be total over its documented domain.
        let _ = hash_tile(TileClass::Bg, 0, &[], &[]);
    }

    #[test]
    fn known_vector_is_stable() {
        // A fixed, hand-picked input/output pair pinned as a regression guard: if this ever
        // changes, every previously-hashed HD texture pack tile entry silently stops matching,
        // which must be a deliberate, reviewed decision (a `hd-pack` `format_version` bump), not
        // an accidental side effect of an unrelated dependency update.
        let words: Vec<u16> = vec![
            0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
        ];
        let palette = [0x0000u16, 0x7FFF];
        let h = hash_tile(TileClass::Bg, 2, &words, &palette);
        assert_eq!(h, 0x3a74_93ef_1d8a_f377_u64);
    }
}
