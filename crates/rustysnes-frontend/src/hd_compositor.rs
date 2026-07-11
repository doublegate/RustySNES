//! Pure CPU-side HD texture pack compositor (`v1.3.0`, `hd-pack` feature).
//!
//! Takes the native framebuffer (already BGR555 → RGBA8 decoded, `crate::gfx::bgr555_to_rgba8`)
//! plus the PPU's per-pixel [`rustysnes_core::ppu::hdtag::TileTag`] side-buffer and a loaded
//! [`crate::hd_pack::HdPack`], and produces an upscaled RGBA8 buffer: every 8×8 output cell whose
//! sampled tag hash matches a pack entry is replaced by that entry's decoded image (mirrored per
//! the tag's `hflip`/`vflip` — both tile orientations share one pack entry, see `hdtag`'s module
//! doc); every other cell is nearest-neighbor-upscaled from its native low-res color instead —
//! the standard per-tile graceful fallback that lets "some tiles replaced, others native" work in
//! one frame. Wiring this into the live wgpu present path (`gfx.rs`) is a follow-up; these
//! functions are deliberately pure (no wgpu/`EmuCore` dependency) so they're testable standalone.

use std::collections::HashMap;

use rustysnes_core::ppu::hdtag::TileTag;

use crate::hd_pack::DecodedTile;

/// Composite one frame, substituting any tag-matched 8×8 cell with its pack replacement.
///
/// `fb_rgba` (`fb_w * fb_h * 4` RGBA8 bytes) and `tags` (`fb_w * fb_h` entries, indexed
/// identically to `fb_rgba` — i.e. exactly [`rustysnes_core::ppu::Ppu::framebuffer`]/
/// [`rustysnes_core::ppu::Ppu::tile_tags`]'s own pairing) combine with `pack`'s decoded tiles
/// into an `(fb_w * scale, fb_h * scale)` output. `scale` only controls the *fallback*
/// nearest-neighbor path's upscale factor — a hash-matched cell is drawn at its own replacement
/// image's native resolution instead, so packs mixing tile resolutions are fine. Returns
/// `(out_w, out_h, out_rgba)`.
///
/// Each 8×8 output cell is sampled once, at its top-left source pixel — real content never tags
/// a single tile's interior with two different hashes, so this is exact, not an approximation.
// `tiles` always comes from `HdPack::tiles` (a plain `HashMap` with the default hasher) -- no
// second caller needs a generic `S: BuildHasher`.
#[allow(clippy::implicit_hasher)]
#[must_use]
pub fn composite(
    fb_rgba: &[u8],
    fb_w: u32,
    fb_h: u32,
    tags: &[TileTag],
    tiles: &HashMap<u64, DecodedTile>,
    scale: u32,
) -> (u32, u32, Vec<u8>) {
    let scale = scale.max(1);
    let out_w = fb_w * scale;
    let out_h = fb_h * scale;
    let mut out = vec![0u8; (out_w as usize) * (out_h as usize) * 4];

    let cells_x = fb_w.div_ceil(8);
    let cells_y = fb_h.div_ceil(8);

    for cell_y in 0..cells_y {
        let cy = cell_y * 8;
        let cell_h = (fb_h - cy).min(8);
        for cell_x in 0..cells_x {
            let cx = cell_x * 8;
            let cell_w = (fb_w - cx).min(8);

            let sample_idx = (cy as usize) * (fb_w as usize) + (cx as usize);
            let tag = tags.get(sample_idx).copied().unwrap_or_default();
            let replacement = if tag.hash == 0 {
                None
            } else {
                tiles.get(&tag.hash)
            };

            if let Some(tile) = replacement {
                blit_replacement(
                    &mut out,
                    out_w,
                    cx * scale,
                    cy * scale,
                    cell_w * scale,
                    cell_h * scale,
                    tile,
                    tag.hflip,
                    tag.vflip,
                );
            } else {
                blit_native_upscale(
                    &mut out, out_w, fb_rgba, fb_w, cx, cy, cell_w, cell_h, scale,
                );
            }
        }
    }
    (out_w, out_h, out)
}

/// Nearest-neighbor upscale one `(cell_w, cell_h)` native-resolution cell at `(cx, cy)` into the
/// `out` buffer at `(cx * scale, cy * scale)`.
#[allow(clippy::too_many_arguments)]
fn blit_native_upscale(
    out: &mut [u8],
    out_w: u32,
    fb: &[u8],
    fb_w: u32,
    cx: u32,
    cy: u32,
    cell_w: u32,
    cell_h: u32,
    scale: u32,
) {
    for y in 0..cell_h {
        for x in 0..cell_w {
            let src = (((cy + y) * fb_w + (cx + x)) as usize) * 4;
            let Some(px) = fb.get(src..src + 4) else {
                continue;
            };
            for sy in 0..scale {
                for sx in 0..scale {
                    let ox = (cx + x) * scale + sx;
                    let oy = (cy + y) * scale + sy;
                    write_pixel(out, out_w, ox, oy, px);
                }
            }
        }
    }
}

/// Resample `tile`'s own decoded image onto the `(dst_w, dst_h)` output region at `(ox0, oy0)`,
/// mirroring the source rect per `hflip`/`vflip`.
#[allow(clippy::too_many_arguments)]
fn blit_replacement(
    out: &mut [u8],
    out_w: u32,
    ox0: u32,
    oy0: u32,
    dst_w: u32,
    dst_h: u32,
    tile: &DecodedTile,
    hflip: bool,
    vflip: bool,
) {
    if tile.width == 0 || tile.height == 0 || dst_w == 0 || dst_h == 0 {
        return;
    }
    for y in 0..dst_h {
        let unflipped_y = y * tile.height / dst_h;
        let sy = if vflip {
            tile.height - 1 - unflipped_y
        } else {
            unflipped_y
        };
        for x in 0..dst_w {
            let unflipped_x = x * tile.width / dst_w;
            let sx = if hflip {
                tile.width - 1 - unflipped_x
            } else {
                unflipped_x
            };
            let src = ((sy * tile.width + sx) as usize) * 4;
            let Some(px) = tile.rgba.get(src..src + 4) else {
                continue;
            };
            write_pixel(out, out_w, ox0 + x, oy0 + y, px);
        }
    }
}

/// Write one RGBA8 pixel at `(x, y)` into `out` (row stride `out_w`), silently clipping any
/// out-of-bounds write (a rounding edge case, never a memory-safety concern).
fn write_pixel(out: &mut [u8], out_w: u32, x: u32, y: u32, px: &[u8]) {
    let dst = ((y * out_w + x) as usize) * 4;
    if let Some(slot) = out.get_mut(dst..dst + 4) {
        slot.copy_from_slice(px);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hd_pack::DecodedTile;

    /// A `w x h` solid-color RGBA8 framebuffer.
    fn solid_fb(w: u32, h: u32, color: [u8; 4]) -> Vec<u8> {
        (0..w * h).flat_map(|_| color).collect()
    }

    #[test]
    fn untagged_frame_is_nearest_upscaled_native_color() {
        let (w, h) = (8, 8);
        let fb = solid_fb(w, h, [10, 20, 30, 255]);
        let tags = vec![TileTag::default(); (w * h) as usize];
        let tiles = HashMap::new();

        let (out_w, out_h, out) = composite(&fb, w, h, &tags, &tiles, 2);
        assert_eq!((out_w, out_h), (16, 16));
        for px in out.chunks_exact(4) {
            assert_eq!(px, [10, 20, 30, 255]);
        }
    }

    #[test]
    fn tagged_cell_is_replaced_by_its_pack_tile() {
        let (w, h) = (8, 8);
        let fb = solid_fb(w, h, [0, 0, 0, 255]); // native color must NOT show through
        let mut tags = vec![TileTag::default(); (w * h) as usize];
        tags[0] = TileTag {
            hash: 0xABCD,
            hflip: false,
            vflip: false,
        };

        let mut tiles = HashMap::new();
        tiles.insert(
            0xABCD,
            DecodedTile {
                width: 2,
                height: 2,
                rgba: vec![
                    255, 0, 0, 255, // top-left: red
                    0, 255, 0, 255, // top-right: green
                    0, 0, 255, 255, // bottom-left: blue
                    255, 255, 0, 255, // bottom-right: yellow
                ],
            },
        );

        let (out_w, _out_h, out) = composite(&fb, w, h, &tags, &tiles, 1);
        // The tagged 8x8 cell (columns 0..8, rows 0..8) is nearest-upscaled from the 2x2
        // replacement -- the top-left quadrant (4x4) must be red, not the native black.
        let px_at = |x: u32, y: u32| {
            let i = ((y * out_w + x) as usize) * 4;
            &out[i..i + 4]
        };
        assert_eq!(px_at(0, 0), [255, 0, 0, 255]);
        assert_eq!(px_at(4, 0), [0, 255, 0, 255]);
        assert_eq!(px_at(0, 4), [0, 0, 255, 255]);
        assert_eq!(px_at(4, 4), [255, 255, 0, 255]);
    }

    #[test]
    fn hflip_mirrors_the_replacement_source_rect() {
        let (w, h) = (8, 8);
        let fb = solid_fb(w, h, [0, 0, 0, 255]);
        let mut tags = vec![TileTag::default(); (w * h) as usize];
        tags[0] = TileTag {
            hash: 1,
            hflip: true,
            vflip: false,
        };
        let mut tiles = HashMap::new();
        tiles.insert(
            1,
            DecodedTile {
                width: 2,
                height: 1,
                rgba: vec![
                    255, 0, 0, 255, // left: red
                    0, 255, 0, 255, // right: green
                ],
            },
        );

        let (out_w, _out_h, out) = composite(&fb, w, h, &tags, &tiles, 1);
        let px_at = |x: u32, y: u32| {
            let i = ((y * out_w + x) as usize) * 4;
            &out[i..i + 4]
        };
        // Mirrored: the LEFT half of the output cell now shows the source's right (green) pixel.
        assert_eq!(px_at(0, 0), [0, 255, 0, 255]);
        assert_eq!(px_at(7, 0), [255, 0, 0, 255]);
    }

    #[test]
    fn partial_edge_cell_does_not_panic_and_covers_only_its_own_pixels() {
        // 10x10: the second row/column of cells is only 2px wide/tall (10 = 8 + 2).
        let (w, h) = (10, 10);
        let fb = solid_fb(w, h, [7, 7, 7, 255]);
        let tags = vec![TileTag::default(); (w * h) as usize];
        let tiles = HashMap::new();

        let (out_w, out_h, out) = composite(&fb, w, h, &tags, &tiles, 3);
        assert_eq!((out_w, out_h), (30, 30));
        assert_eq!(out.len(), (30 * 30 * 4) as usize);
        for px in out.chunks_exact(4) {
            assert_eq!(px, [7, 7, 7, 255]);
        }
    }
}
