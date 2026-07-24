//! Per-scanline compositor: BG modes 0–7 (incl. Mode 7 affine), the 128-sprite OAM pipeline,
//! windows, and color math. Re-implemented clean-room from `docs/ppu.md` + documented SNES
//! hardware behavior, structurally informed by ares (ISC). Nothing was ported verbatim.
//!
//! The model is per-scanline (composite a whole visible line at line end). This is bit-
//! identical in the *final framebuffer* to a per-dot renderer — the determinism contract only
//! requires the finished frame be reproducible — and dramatically simpler. Mid-scanline raster
//! tricks that need dot-resolution (HDMA palette splits) land later via the scheduler driving
//! register writes at the exact dot; the per-line compositor already sees those writes because
//! it samples register state at the *end* of the line.
//!
//! reason for the module-level allows: the compositor is intrinsically a long, branch-dense
//! state machine. `too_many_lines` fires on the per-scanline / per-sprite/mode-7 loops, which
//! are clearer kept whole than split mid-pixel; `many_single_char_names` fires on the Mode-7
//! matrix (`a`/`b`/`c`/`d` are the hardware register names); `match_same_arms` fires on the BG
//! priority tables where several modes deliberately share an identical layout; the small
//! `Copy` structs (`Pixel`, `WindowLayer`) are passed by ref by helpers for call-site clarity;
//! and `needless_update` fires on every `..Pixel::default()` when the `hd-pack` feature is off
//! (its only non-default field, `tag`, vanishes, making the spread a literal no-op) -- kept
//! anyway so every `Pixel` literal compiles unchanged across both feature states, rather than
//! forking each construction site into two copies.
#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    clippy::trivially_copy_pass_by_ref,
    clippy::needless_update
)]

use crate::{Object, Ppu, SCREEN_WIDTH, VideoBus};

/// The DAC state one composited column hands to the NEXT column's hi-res below-pass.
///
/// ares recomputes the hi-res subscreen's blend-mode/halve gates from each column's OWN
/// below-opacity, and the *next* column's below-pass is what actually consumes them (`dac.cpp`
/// lines 124-129), so the composite is threaded left-to-right through this carry rather than read
/// from the current column. Seeded per line as ares' pre-line reset.
#[derive(Clone, Copy, Default)]
pub struct DacCarry {
    /// This column's main pixel was not forced black (gates the next column's hi-res below color).
    above_enable: bool,
    /// This column's color-math was enabled (gates the next column's hi-res below-pass math).
    below_enable: bool,
    /// This column's **unclipped** 15-bit main-screen CGRAM color (the next column's below addend
    /// when its blend mode selects the subscreen).
    above_raw: u16,
    /// Whether this column's subscreen pixel was opaque (drives the next column's blend/halve gates).
    below_opaque: bool,
}

/// The per-line constants a column composite reads but never varies within a line: the framebuffer
/// row base, the master brightness, and whether the frame is hi-res. Grouped so [`Ppu::compose_pixel`]
/// takes them as one argument instead of three.
#[derive(Clone, Copy)]
struct LineCtx {
    base: usize,
    brightness: u32,
    hires: bool,
}

/// A composited layer pixel: a 8-bit CGRAM palette index + a priority + the source-layer id.
#[derive(Clone, Copy, Default)]
pub struct Pixel {
    palette: u8,
    priority: u8,
    /// Layer source: 0..=3 bg1..4, 4 obj, 5 backdrop. Drives color-math enable + direct color.
    layer: u8,
    /// Mode-0 direct-color paletteGroup carry (only meaningful for direct-color BG1).
    palette_group: u8,
    /// True if this pixel actually came from a non-transparent source.
    opaque: bool,
    /// This pixel's HD-pack tile-identity tag (`v1.3.0`, `hd-pack` feature) -- stays
    /// [`crate::hdtag::TileTag::default`] (hash `0`) unless `Ppu::hd_pack_tagging` was on when
    /// this pixel was fetched. See `Ppu::tile_tags`'s field doc for the full mechanism.
    #[cfg(feature = "hd-pack")]
    tag: crate::hdtag::TileTag,
}

/// Priority tables per BG mode. Indexed `[mode][layer]` where layer 0..=3 are bg1..4 and 4 is
/// obj. Each BG has a low/high priority (tile bit 13 selects); sprites have 4 (one per OAM
/// priority). We model BG priorities as `[low, high]` and OBJ as four entries. Values are the
/// composited priority used by the painter's-algorithm DAC (higher wins).
struct ModePriorities {
    bg: [[u8; 2]; 4],
    obj: [u8; 4],
    active: [bool; 4], // which BGs participate in this mode
}

impl Ppu {
    /// Resolve the priority table for the current BG mode (per `docs/ppu.md`'s mode table,
    /// matching the documented SNES layering order).
    const fn mode_priorities(&self) -> ModePriorities {
        match self.io.bg_mode {
            0 => ModePriorities {
                bg: [[8, 11], [7, 10], [2, 5], [1, 4]],
                obj: [3, 6, 9, 12],
                active: [true, true, true, true],
            },
            1 => {
                if self.io.bg3_priority {
                    ModePriorities {
                        bg: [[5, 8], [4, 7], [1, 10], [0, 0]],
                        obj: [2, 3, 6, 9],
                        active: [true, true, true, false],
                    }
                } else {
                    ModePriorities {
                        bg: [[6, 9], [5, 8], [1, 3], [0, 0]],
                        obj: [2, 4, 7, 10],
                        active: [true, true, true, false],
                    }
                }
            }
            2 => ModePriorities {
                bg: [[3, 7], [1, 5], [0, 0], [0, 0]],
                obj: [2, 4, 6, 8],
                active: [true, true, false, false],
            },
            3 => ModePriorities {
                bg: [[3, 7], [1, 5], [0, 0], [0, 0]],
                obj: [2, 4, 6, 8],
                active: [true, true, false, false],
            },
            4 => ModePriorities {
                bg: [[3, 7], [1, 5], [0, 0], [0, 0]],
                obj: [2, 4, 6, 8],
                active: [true, true, false, false],
            },
            5 => ModePriorities {
                bg: [[3, 7], [1, 5], [0, 0], [0, 0]],
                obj: [2, 4, 6, 8],
                active: [true, true, false, false],
            },
            6 => ModePriorities {
                bg: [[2, 5], [0, 0], [0, 0], [0, 0]],
                obj: [1, 3, 4, 6],
                active: [true, false, false, false],
            },
            _ => {
                // Mode 7
                if self.io.extbg {
                    ModePriorities {
                        bg: [[3, 3], [1, 5], [0, 0], [0, 0]],
                        obj: [2, 4, 6, 7],
                        active: [true, true, false, false],
                    }
                } else {
                    ModePriorities {
                        bg: [[2, 2], [0, 0], [0, 0], [0, 0]],
                        obj: [1, 3, 4, 5],
                        active: [true, false, false, false],
                    }
                }
            }
        }
    }

    /// Bits-per-pixel of a BG in the current mode (0 means inactive).
    const fn bg_bpp(&self, bg: usize) -> u8 {
        match (self.io.bg_mode, bg) {
            (0, _) => 2,
            (1, 0 | 1) => 4,
            (1, 2) => 2,
            (2, 0 | 1) => 4,
            (3, 0) => 8,
            (3, 1) => 4,
            (4, 0) => 8,
            (4, 1) => 2,
            (5, 0) => 4,
            (5, 1) => 2,
            (6, 0) => 4,
            (7, _) => 8,
            _ => 0,
        }
    }

    /// Compute the [`crate::hdtag::TileTag`] for a tile whose raw pre-flip VRAM words start at
    /// `tile_base` (word address; `bpp` gives the word count: `bpp * 4` for BG/OBJ, or a fixed 64
    /// for Mode 7's 8bpp block) and whose resolved `2^bpp`-color palette starts at CGRAM index
    /// `pal_base`. `hflip`/`vflip` are stored alongside the hash (excluded from it by design --
    /// see `hdtag`'s module doc) so the frontend compositor can mirror the replacement source
    /// rect. Only called from the three render paths when `self.hd_pack_tagging` is on -- every
    /// address/bpp/palette-base input here is already resolved by the caller, so this adds one
    /// small VRAM+CGRAM copy (bounded: at most 64 words / 256 colors) and one hash, never more.
    #[cfg(feature = "hd-pack")]
    fn tile_tag(
        &self,
        class: crate::hdtag::TileClass,
        bpp: u8,
        tile_base: u16,
        pal_base: u8,
        hflip: bool,
        vflip: bool,
    ) -> crate::hdtag::TileTag {
        let word_count = if matches!(class, crate::hdtag::TileClass::Mode7) {
            64
        } else {
            usize::from(bpp) * 4
        };
        let mut words = [0u16; 64];
        for (i, w) in words.iter_mut().take(word_count).enumerate() {
            *w = self.vram[(tile_base.wrapping_add(i as u16) & 0x7fff) as usize];
        }
        let color_count = 1usize << bpp;
        let mut palette = [0u16; 256];
        for (i, c) in palette.iter_mut().take(color_count).enumerate() {
            *c = self.cgram[(usize::from(pal_base) + i) & 0xff];
        }
        let hash =
            crate::hdtag::hash_tile(class, bpp, &words[..word_count], &palette[..color_count]);
        crate::hdtag::TileTag { hash, hflip, vflip }
    }

    /// Per-dot compositor (`docs/adr/0014` T-CA-10 Phase 4): fetch this visible line's `above`/`below`
    /// pixels once (backdrop + backgrounds + sprites, without the per-column composite), seed the DAC
    /// carry, and reset the draw cursor. Called lazily by [`Self::pd_render_to_dot`] at each line's
    /// first active dot (or after a save-state load). Fetch happens once per line, before this line's
    /// HDMA, so on a static line it reads the register state a whole-line composite at `RENDER_DOT`
    /// would.
    fn pd_fetch_line(&mut self, bus: &mut impl VideoBus) {
        let row = (self.v - 1) as usize;
        if row == 0 {
            self.frame_hires = self.is_hires();
        }
        // ALWAYS build the line, even under force-blank: display-disable is a compose-time decision
        // (`pd_render_to_dot` outputs black per column while it is set), so a line that is blanked at
        // its start but UN-blanked mid-line must still have real pixels ready to draw. (The batch
        // early-returns on force-blank because it composites the whole line at once; the per-dot path
        // cannot, since blank can toggle within the line — MesenCE fetches regardless of force-blank.)
        let mut above = [Pixel::default(); SCREEN_WIDTH];
        let mut below = [Pixel::default(); SCREEN_WIDTH];
        let pr = self.mode_priorities();
        for x in 0..SCREEN_WIDTH {
            above[x] = Pixel {
                palette: 0,
                priority: 0,
                layer: 5,
                palette_group: 0,
                opaque: false,
                ..Pixel::default()
            };
            below[x] = above[x];
        }
        if self.io.bg_mode == 7 {
            self.render_mode7(bus, &pr, &mut above, &mut below);
        } else {
            for bg in 0..4 {
                if pr.active[bg] {
                    self.render_bg(bg, &pr, &mut above, &mut below);
                }
            }
        }
        self.render_objects(&pr, &mut above, &mut below);
        self.pd_above.copy_from_slice(&above);
        self.pd_below.copy_from_slice(&below);
        self.pd_carry = DacCarry {
            above_enable: false,
            below_enable: false,
            above_raw: self.cgram[0],
            below_opaque: false,
        };
        self.pd_draw_x = 0;
        // Seed the OAM sprite-evaluation index for this line (MesenCE `_oamEvaluationIndex` at
        // `_spriteEvalStart == 0`): the priority-rotation base, or 0. The in-render `$2104` redirect
        // reads `seed + (min(h,255)+1)/2` from here.
        //
        // Capture it ONLY at the true line start (`h == 0`). A fetch at `h > 0` is a post-`load_state`
        // re-fetch (the only thing that invalidates `pd_fetched_line` mid-line): there, `OAMADDR` has
        // already diverged from its line-start value via mid-line redirected writes — which is exactly
        // why `pd_oam_eval_seed` is serialized (`FORMAT_VERSION 7`) — so re-deriving it from the current
        // `OAMADDR` would clobber the deserialized value and break mid-scanline save-state determinism.
        // Leaving it untouched preserves the restored seed (MesenCE serializes `_oamEvaluationIndex`
        // and likewise never re-derives it on load).
        if self.h == 0 {
            self.pd_oam_eval_seed = if self.io.oam_priority_rotation {
                ((self.io.oam_address >> 2) & 0x7f) as u8
            } else {
                0
            };
        }
        self.pd_fetched_line = self.v;
    }

    /// Per-dot compositor driver, called every dot from [`crate::Ppu::tick_dot`]. Composites the
    /// visible line's columns incrementally up to the column the DAC has reached at the current dot,
    /// using **live** register state per column (so a mid-line color-math/brightness/force-blank write
    /// only affects columns drawn after it), and tracks [`crate::Ppu::internal_cgram_address`] = the
    /// last drawn palette (the exact CGRAM-redirect target). All columns finish by `RENDER_DOT` so the
    /// composite still reads pre-line-HDMA state (a static line composites identically to a whole-line
    /// pass at `RENDER_DOT`).
    pub(crate) fn pd_render_to_dot(&mut self, bus: &mut impl VideoBus) {
        if self.v < 1 || self.v > self.visible_height() {
            return;
        }
        if self.pd_fetched_line != self.v {
            self.pd_fetch_line(bus);
        }
        let target = if self.h < crate::ACTIVE_DOT_START {
            0
        } else if self.h >= crate::RENDER_DOT {
            SCREEN_WIDTH
        } else {
            usize::from(self.h - crate::ACTIVE_DOT_START + 1).min(SCREEN_WIDTH)
        };
        let base = (self.v - 1) as usize * self.visible_width();
        let hires = self.frame_hires;
        while usize::from(self.pd_draw_x) < target {
            let x = usize::from(self.pd_draw_x);
            if self.io.display_disable {
                if hires {
                    self.framebuffer[base + 2 * x] = 0;
                    self.framebuffer[base + 2 * x + 1] = 0;
                } else {
                    self.framebuffer[base + x] = 0;
                }
            } else {
                let ctx = LineCtx {
                    base,
                    brightness: u32::from(self.io.display_brightness),
                    hires,
                };
                let ap = self.pd_above[x];
                let bp = self.pd_below[x];
                self.pd_carry = self.compose_pixel(x, ap, bp, ctx, self.pd_carry);
                self.internal_cgram_address = ap.palette;
            }
            self.pd_draw_x += 1;
        }
    }

    /// Render one non-Mode-7 background into the above/below pixel buffers.
    // `bg3_hofs`/`bg3_vofs` (offset-per-tile) intentionally mirror the `hofs`/`vofs` naming.
    #[allow(clippy::similar_names)]
    fn render_bg(&self, bg: usize, pr: &ModePriorities, above: &mut [Pixel], below: &mut [Pixel]) {
        let bpp = self.bg_bpp(bg);
        if bpp == 0 {
            return;
        }
        let main = self.io.main_enable[bg];
        let sub = self.io.sub_enable[bg];
        if !main && !sub {
            return;
        }

        let tile_w = if self.io.tile_size[bg] { 16u32 } else { 8 };
        let tile_h = tile_w;

        let hofs = u32::from(self.io.bg_hofs[bg]);
        let vofs = u32::from(self.io.bg_vofs[bg]);
        let screen_size = self.io.bg_screen_size[bg];
        let screen_addr = self.io.bg_screen_addr[bg];
        let char_addr = self.io.bg_tiledata_addr[bg];

        // Mosaic vertical handling. Quantised in SCREEN space, then converted back to the BG's
        // line: mosaic blocks are anchored to the top of the picture, not to the BG's own
        // coordinate space.
        let mut line_y = u32::from(self.v);
        if self.io.mosaic_enable[bg] && self.io.mosaic_size > 1 {
            let m = u32::from(self.io.mosaic_size);
            // Cannot underflow: the caller renders only for `self.v >= 1` (see `tick_ppu_dot`),
            // and `line_y` is `self.v` until this point. Saturating here instead would turn a
            // broken invariant into a silently wrong picture, which is the harder bug to find.
            debug_assert!(line_y >= 1, "render_bg called for scanline 0");
            let screen_y = line_y - 1;
            line_y = (screen_y / m) * m + 1;
        }

        // Offset-per-tile (OPT) applies to BG1/BG2 in modes 2, 4, 6: BG3's tilemap supplies a
        // per-tile-column horizontal and/or vertical offset that overrides the BG's own scroll for
        // that column (ares `PPU::Background::render`). Star Fox's intro planet lives in the lower
        // half of BG2's 64x64 tilemap and is scrolled into view column-by-column via OPT V-offsets;
        // ignoring OPT is what left the planet off-screen (only the star quadrant showed).
        let opt_mode = matches!(self.io.bg_mode, 2 | 4 | 6) && bg < 2;
        let opt_valid = 0x2000u16 << bg; // BG1 => 0x2000, BG2 => 0x4000
        let hofs_fine = hofs & 7;

        // Per-dot compositor, Phase 2 (`docs/adr/0014`): the per-column FETCH fills a per-line pixel
        // buffer (an opaque `Pixel` per column, or a transparent default), and a separate DRAIN pass
        // below composites it into `above`/`below`. Splitting fetch from composite is the structural
        // step toward driving the drain one dot at a time; under static state (the whole line fetched
        // before any composite, as here) it is BYTE-IDENTICAL to the fused per-column write, because
        // each column touches only its own `above[x]`/`below[x]` and reads none of its own line's
        // prior columns. Fixed-size stack buffer — no allocation on the hot path.
        let mut bg_line = [Pixel::default(); SCREEN_WIDTH];

        for x in 0..SCREEN_WIDTH as u32 {
            let px = if self.io.mosaic_enable[bg] && self.io.mosaic_size > 1 {
                let m = u32::from(self.io.mosaic_size);
                (x / m) * m
            } else {
                x
            };
            let mut world_x = px.wrapping_add(hofs);
            let mut world_y = line_y.wrapping_add(vofs);
            if opt_mode {
                let offset_x = (px + hofs_fine) & !7;
                if offset_x >= tile_w {
                    // first tile column(s) are exempt
                    let bg3_hofs = u32::from(self.io.bg_hofs[2]);
                    let bg3_vofs = u32::from(self.io.bg_vofs[2]);
                    let base_x = (offset_x - tile_w).wrapping_add(bg3_hofs & !7);
                    let hlookup = self.bg3_opt_tile(base_x, bg3_vofs);
                    let fine = (px + hofs_fine) & 7;
                    if self.io.bg_mode == 4 {
                        if hlookup & opt_valid != 0 {
                            if hlookup & 0x8000 == 0 {
                                world_x = offset_x + (u32::from(hlookup) & !7) + fine;
                            } else {
                                world_y = line_y.wrapping_add(u32::from(hlookup));
                            }
                        }
                    } else {
                        let vlookup = self.bg3_opt_tile(base_x, bg3_vofs.wrapping_add(8));
                        if hlookup & opt_valid != 0 {
                            world_x = offset_x + (u32::from(hlookup) & !7) + fine;
                        }
                        if vlookup & opt_valid != 0 {
                            world_y = line_y.wrapping_add(u32::from(vlookup));
                        }
                    }
                }
            }

            let (palette_idx, group, priority_hi, tile_base, hflip, vflip) = self.fetch_bg_pixel(
                world_x,
                world_y,
                tile_w,
                tile_h,
                screen_size,
                screen_addr,
                char_addr,
                bpp,
            );
            if palette_idx == 0 {
                continue;
            }
            // Only the `hd-pack` tile-tagging hook below consumes these; keep them from
            // triggering an unused-variable warning when that feature is compiled out.
            #[cfg(not(feature = "hd-pack"))]
            let _ = (tile_base, hflip, vflip);

            // BG palette index: Mode 0 gives each BG its own 32-color region; every other mode
            // shares the 256-entry CGRAM. The tilemap's 3-bit palette group selects a sub-palette
            // of `2^bpp` colors, contributing `group << bpp` (masked to a byte; 8bpp ignores the
            // group). Dropping this group offset is what collapsed every BG tile onto palette
            // group 0 and washed the SMW logo/border colors. Matches ares `background.cpp`:
            //   paletteIndex = paletteBase + (paletteNumber << paletteShift) & 0xff
            let pal_base: u16 = if self.io.bg_mode == 0 {
                (bg as u16) << 5
            } else {
                0
            };
            let group_off = (u16::from(group) << bpp) & 0xff;
            let final_pal = (pal_base + group_off + u16::from(palette_idx)) as u8;
            let prio = pr.bg[bg][usize::from(priority_hi)];

            let xi = x as usize;
            #[allow(unused_mut)]
            let mut pixel = Pixel {
                palette: final_pal,
                priority: prio,
                layer: bg as u8,
                palette_group: group,
                opaque: true,
                ..Pixel::default()
            };
            #[cfg(feature = "hd-pack")]
            if self.hd_pack_tagging {
                let group_base = ((pal_base + group_off) & 0xff) as u8;
                pixel.tag = self.tile_tag(
                    crate::hdtag::TileClass::Bg,
                    bpp,
                    tile_base,
                    group_base,
                    hflip,
                    vflip,
                );
            }
            // FETCH: stash the resolved pixel; the DRAIN pass below does the window+priority write
            // (the priority travels in `pixel.priority`).
            bg_line[xi] = pixel;
        }

        // DRAIN: composite the fetched line into `above`/`below`. This is the pass a future per-dot
        // compositor will step one dot at a time; here it runs after the full-line fetch, so the
        // result is byte-identical to the fused loop (see the buffer's doc comment above).
        for xi in 0..SCREEN_WIDTH {
            let pixel = bg_line[xi];
            if !pixel.opaque {
                continue;
            }
            let prio = pixel.priority;
            if main && !self.windowed_out(bg, xi, true) && prio > above[xi].priority {
                above[xi] = pixel;
            }
            if sub && !self.windowed_out(bg, xi, false) && prio > below[xi].priority {
                below[xi] = pixel;
            }
        }
    }

    /// Read a raw BG3 tilemap entry at world `(hoffset, voffset)` — the offset-per-tile source for
    /// modes 2/4/6 (ares `PPU::Background::getTile` applied to BG3). The entry is reinterpreted as
    /// a scroll offset, not a character, by the OPT logic in the BG render loop.
    fn bg3_opt_tile(&self, hoffset: u32, voffset: u32) -> u16 {
        let ss = self.io.bg_screen_size[2];
        let shift = if self.io.tile_size[2] { 4 } else { 3 };
        let tile_x = hoffset >> shift;
        let tile_y = voffset >> shift;
        let screen_x = if ss & 1 != 0 { 32u32 << 5 } else { 0 };
        let screen_y = if ss & 2 != 0 {
            32u32 << (5 + (ss & 1))
        } else {
            0
        };
        let mut offset = ((tile_y & 0x1f) << 5) | (tile_x & 0x1f);
        if tile_x & 0x20 != 0 {
            offset += screen_x;
        }
        if tile_y & 0x20 != 0 {
            offset += screen_y;
        }
        let addr = (u32::from(self.io.bg_screen_addr[2]).wrapping_add(offset)) & 0x7fff;
        self.vram[addr as usize]
    }

    /// Fetch one BG pixel: returns (palette index within the BG palette, palette group, hi-prio,
    /// the resolved 8×8 sub-tile's raw pre-flip VRAM word address, hflip, vflip). The last three
    /// are only consumed by the `hd-pack` feature's tile-tagging hook, but are cheap enough
    /// (already-computed locals) to always return rather than threading a second feature-gated
    /// fetch path through this hot function.
    #[allow(clippy::too_many_arguments)]
    fn fetch_bg_pixel(
        &self,
        world_x: u32,
        world_y: u32,
        tile_w: u32,
        tile_h: u32,
        screen_size: u8,
        screen_addr: u16,
        char_addr: u16,
        bpp: u8,
    ) -> (u8, u8, u8, u16, bool, bool) {
        // Map size in pixels: base 256 (32 tiles * 8px), doubled per screen-size bit, and again
        // for 16x16 tiles (each quadrant stays 32 tiles wide => 512px when tiles are 16px).
        let big_h = u32::from(screen_size & 1 != 0);
        let big_v = u32::from(screen_size & 2 != 0);
        let hsize = (256u32 << big_h) << i32::from(tile_w == 16);
        let vsize = (256u32 << big_v) << i32::from(tile_h == 16);
        let wx = world_x & (hsize - 1);
        let wy = world_y & (vsize - 1);

        let htile = wx / tile_w;
        let vtile = wy / tile_h;

        // Which 32x32 quadrant (for 64-tile-wide/tall maps).
        let hscreen = if screen_size & 1 != 0 { 0x400u16 } else { 0 };
        let vscreen = if screen_size & 2 != 0 {
            if screen_size & 1 != 0 {
                0x800u16
            } else {
                0x400
            }
        } else {
            0
        };
        let mut offset = ((htile & 0x1f) | ((vtile & 0x1f) << 5)) as u16;
        if htile & 0x20 != 0 {
            offset = offset.wrapping_add(hscreen);
        }
        if vtile & 0x20 != 0 {
            offset = offset.wrapping_add(vscreen);
        }
        let map_addr = (screen_addr.wrapping_add(offset)) & 0x7fff;
        let entry = self.vram[map_addr as usize];

        let mut character = entry & 0x03ff;
        let palette_group = ((entry >> 10) & 0x07) as u8;
        let priority_hi = ((entry >> 13) & 0x01) as u8;
        let hflip = entry & 0x4000 != 0;
        let vflip = entry & 0x8000 != 0;

        // Pixel within tile, honoring 16x16 tiles (which span 4 8x8 chars).
        let mut fine_x = (wx % tile_w) as u16;
        let mut fine_y = (wy % tile_h) as u16;
        if hflip {
            fine_x = tile_w as u16 - 1 - fine_x;
        }
        if vflip {
            fine_y = tile_h as u16 - 1 - fine_y;
        }
        if tile_w == 16 {
            if fine_x >= 8 {
                character = character.wrapping_add(1);
                fine_x -= 8;
            }
            if fine_y >= 8 {
                character = character.wrapping_add(16);
                fine_y -= 8;
            }
        }

        let words_per_tile = u16::from(bpp) * 8 / 16; // 2bpp=1word/row*... actually compute below
        let _ = words_per_tile;
        // Each 8x8 tile occupies (bpp/2) bitplane-pairs; row stride is one word per plane-pair.
        let tile_words = u16::from(bpp) * 4; // 2bpp=8,4bpp=16,8bpp=32 words per tile
        let tile_base = (char_addr.wrapping_add(character.wrapping_mul(tile_words))) & 0x7fff;

        let color = self.read_planar(tile_base, fine_x, fine_y, bpp);
        (color, palette_group, priority_hi, tile_base, hflip, vflip)
    }

    /// Decode the `bpp`-bit color at (`fine_x`, `fine_y`) from a tile at `tile_base` (word addr).
    fn read_planar(&self, tile_base: u16, fine_x: u16, fine_y: u16, bpp: u8) -> u8 {
        let bit = 7 - (fine_x & 7);
        let mut color = 0u8;
        // Each plane-pair is 8 words apart; row index = fine_y.
        let pairs = bpp / 2;
        for p in 0..pairs {
            let word_addr = (tile_base
                .wrapping_add(u16::from(p) * 8)
                .wrapping_add(fine_y & 7))
                & 0x7fff;
            let word = self.vram[word_addr as usize];
            let lo = ((word >> bit) & 1) as u8;
            let hi = ((word >> (8 + bit)) & 1) as u8;
            color |= lo << (p * 2);
            color |= hi << (p * 2 + 1);
        }
        color
    }

    /// Render Mode 7 (BG1 affine; BG2 = the high-bit priority layer when EXTBG).
    fn render_mode7(
        &self,
        _bus: &mut impl VideoBus,
        pr: &ModePriorities,
        above: &mut [Pixel],
        below: &mut [Pixel],
    ) {
        let a = self.io.m7a as i16 as i32;
        let b = self.io.m7b as i16 as i32;
        let c = self.io.m7c as i16 as i32;
        let d = self.io.m7d as i16 as i32;

        // 13-bit signed center + scroll.
        let sext13 = |n: u16| -> i32 {
            let n = (n & 0x1fff) as i32;
            if n & 0x1000 != 0 { n | !0x1fff } else { n }
        };
        let hcenter = sext13(self.io.m7x);
        let vcenter = sext13(self.io.m7y);
        let hoffset = sext13(self.io.m7_hofs);
        let voffset = sext13(self.io.m7_vofs);

        let clip = |n: i32| -> i32 {
            if n & 0x2000 != 0 {
                n | !0x3ff
            } else {
                n & 0x3ff
            }
        };

        // Mosaic, quantised in SCREEN space exactly as `render_bg` does it: the block grid is
        // anchored to the top-left of the picture, not to whatever the transform maps there.
        // Mode 7 had no mosaic handling at all until the framebuffer oracle rendered the same
        // picture with and without it.
        let mosaic = self.io.mosaic_enable[0] && self.io.mosaic_size > 1;
        let msize = u32::from(self.io.mosaic_size);

        let mut y = u32::from(self.v);
        if mosaic {
            y = ((y - 1) / msize) * msize + 1;
        }
        if self.io.m7_vflip {
            y = 255 - (y & 0xff);
        }

        let origin_x = ((a * clip(hoffset - hcenter)) & !63)
            + ((b * clip(voffset - vcenter)) & !63)
            + ((b * y as i32) & !63)
            + (hcenter << 8);
        let origin_y = ((c * clip(hoffset - hcenter)) & !63)
            + ((d * clip(voffset - vcenter)) & !63)
            + ((d * y as i32) & !63)
            + (vcenter << 8);

        let main1 = self.io.main_enable[0];
        let sub1 = self.io.sub_enable[0];
        let extbg = self.io.extbg;
        let main2 = extbg && self.io.main_enable[1];
        let sub2 = extbg && self.io.sub_enable[1];

        for screen_x in 0..SCREEN_WIDTH as u32 {
            let mut x = screen_x;
            if mosaic {
                x = (x / msize) * msize;
            }
            if self.io.m7_hflip {
                x = 255 - (x & 0xff);
            }

            let pixel_x = (origin_x + a * x as i32) >> 8;
            let pixel_y = (origin_y + c * x as i32) >> 8;

            let out_of_bounds = (pixel_x | pixel_y) & !1023 != 0;

            let palette_addr = (((pixel_y as u32) & 7) << 3) | ((pixel_x as u32) & 7);
            let tile_x = ((pixel_x >> 3) as u32) & 0x7f;
            let tile_y = ((pixel_y >> 3) as u32) & 0x7f;
            let tile_addr = (tile_y << 7) | tile_x;

            let tile = if self.io.m7_repeat == 3 && out_of_bounds {
                0u16
            } else {
                self.vram[(tile_addr & 0x7fff) as usize] & 0xff
            };
            let mut palette = if self.io.m7_repeat == 2 && out_of_bounds {
                0u8
            } else {
                let addr = ((tile << 6) | (palette_addr as u16)) & 0x7fff;
                (self.vram[addr as usize] >> 8) as u8
            };

            let xi = screen_x as usize;

            // Only the `hd-pack` tile-tagging hook below consumes this; keep it from triggering
            // an unused-variable warning when that feature is compiled out.
            #[cfg(not(feature = "hd-pack"))]
            let _ = tile;

            // BG1 always renders, with the FULL 8-bit palette. EXTBG adds a second layer from
            // the same pixels — it does not replace the first. Treating it as an either/or made
            // BG1 vanish the moment EXTBG was enabled, which the framebuffer oracle caught by
            // rendering a picture both references disagreed with.
            let palette_bg1 = palette;
            if palette_bg1 != 0 {
                let prio = pr.bg[0][0];
                #[allow(unused_mut)]
                let mut pixel = Pixel {
                    palette: palette_bg1,
                    priority: prio,
                    layer: 0,
                    palette_group: 0,
                    opaque: true,
                    ..Pixel::default()
                };
                #[cfg(feature = "hd-pack")]
                if self.hd_pack_tagging {
                    let tile_base = (tile << 6) & 0x7fff;
                    pixel.tag = self.tile_tag(
                        crate::hdtag::TileClass::Mode7,
                        8,
                        tile_base,
                        0,
                        false,
                        false,
                    );
                }
                if main1 && !self.windowed_out(0, xi, true) && prio > above[xi].priority {
                    above[xi] = pixel;
                }
                if sub1 && !self.windowed_out(0, xi, false) && prio > below[xi].priority {
                    below[xi] = pixel;
                }
            }

            // BG2, present only under EXTBG: the same pixel, with bit 7 promoted from palette
            // data to a priority selector and the remaining seven bits as the colour.
            if extbg {
                let prio_hi = (palette >> 7) & 1;
                palette &= 0x7f;
                if palette != 0 {
                    let prio = pr.bg[1][usize::from(prio_hi)];
                    #[allow(unused_mut)]
                    let mut pixel = Pixel {
                        palette,
                        priority: prio,
                        layer: 1,
                        palette_group: 0,
                        opaque: true,
                        ..Pixel::default()
                    };
                    #[cfg(feature = "hd-pack")]
                    if self.hd_pack_tagging {
                        let tile_base = (tile << 6) & 0x7fff;
                        pixel.tag = self.tile_tag(
                            crate::hdtag::TileClass::Mode7,
                            8,
                            tile_base,
                            0,
                            false,
                            false,
                        );
                    }
                    if main2 && !self.windowed_out(1, xi, true) && prio > above[xi].priority {
                        above[xi] = pixel;
                    }
                    if sub2 && !self.windowed_out(1, xi, false) && prio > below[xi].priority {
                        below[xi] = pixel;
                    }
                }
            }
        }
    }

    /// Decode a sprite from OAM by index 0..=127.
    fn object(&self, index: usize) -> Object {
        let lo = index * 4;
        let x_low = self.oam[lo];
        let y = self.oam[lo + 1];
        let character = self.oam[lo + 2];
        let attr = self.oam[lo + 3];
        let hi = self.oam[0x200 + index / 4];
        let shift = (index % 4) * 2;
        let x_high = (hi >> shift) & 1;
        let size = (hi >> (shift + 1)) & 1;
        Object {
            x: u16::from(x_low) | (u16::from(x_high) << 8),
            y,
            character,
            nameselect: attr & 0x01 != 0,
            palette: (attr >> 1) & 0x07,
            priority: (attr >> 4) & 0x03,
            hflip: attr & 0x40 != 0,
            vflip: attr & 0x80 != 0,
            size: size != 0,
        }
    }

    /// (width, height) of a sprite given OBSEL base size + its size toggle.
    fn object_size(&self, large: bool) -> (u32, u32) {
        const SMALL: [(u32, u32); 8] = [
            (8, 8),
            (8, 8),
            (8, 8),
            (16, 16),
            (16, 16),
            (32, 32),
            (16, 32),
            (16, 32),
        ];
        const LARGE: [(u32, u32); 8] = [
            (16, 16),
            (32, 32),
            (64, 64),
            (32, 32),
            (64, 64),
            (64, 64),
            (32, 64),
            (32, 32),
        ];
        let table = if large { LARGE } else { SMALL };
        table[usize::from(self.io.obj_base_size)]
    }

    /// Per-dot sprite over-flag (STAT77 range/time) timing — the 4b increment of the per-dot
    /// compositor (`docs/adr/0014`; dossier C7.05/C7.06). Hardware evaluates a line's sprites one
    /// line AHEAD of drawing them (MesenCE `EvaluateNextLineSprites`), setting `range_over` when a
    /// 33rd in-range sprite is found and `time_over` once more than 34 sprite-tiles are due. So during
    /// display line `self.v` this evaluates `scan_y = self.v` — the sprites that paint on `self.v + 1`
    /// — and exposes each flag at the exact dot a cart reading `$213E` observes it: `range_over` at
    /// `V = OBJ.YLOC, H = OAM.INDEX*2` (the eval cycle of the 33rd sprite), `time_over` by the fetch
    /// phase so it reads set by `V = OBJ.YLOC + 1, H = 0`. The paint pass (`eval_objects_range`,
    /// `scan_y = self.v - 1`) no longer sets them.
    ///
    /// The set-dots are computed once at line start (from OAMADDR's priority-rotation base, exactly as
    /// the paint pass snapshots it) and NOT re-derived if the CPU writes OAM/OAMADDR mid-line before
    /// the flag trips — the same whole-line snapshot approximation `eval_objects_range` already makes
    /// for painting; a true mid-eval OAM re-read is a separate, unvalidated refinement.
    ///
    /// Transient (recomputed per line); `pd_over_computed_line` is invalidated on `load_state` so a
    /// mid-line restore re-derives the pending set-dot. `io.range_over`/`io.time_over` themselves are
    /// serialized, so a flag already set before a save survives regardless.
    pub(crate) fn pd_eval_over_flags(&mut self) {
        // Only lines whose sprites paint on a visible line (`scan_y = self.v` draws on `self.v + 1`).
        if self.v >= self.visible_height() {
            return;
        }
        if self.pd_over_computed_line != self.v {
            self.pd_over_computed_line = self.v;
            let (range_dot, time_dot) = self.compute_over_flag_dots(u32::from(self.v));
            self.pd_over_range_dot = range_dot;
            self.pd_over_time_dot = time_dot;
        }
        if self.pd_over_range_dot == Some(self.h) {
            self.io.range_over = true;
        }
        if self.pd_over_time_dot == Some(self.h) {
            self.io.time_over = true;
        }
    }

    /// Scan OAM for `scan_y` exactly as [`Self::eval_objects_range`] does (same in-range test, same
    /// 32-sprite break, same reverse-fetch tile budget) and return the dots at which `range_over` and
    /// `time_over` should trip. `range_over`: the 33rd in-range sprite's evaluation dot (`2 * i + 1`,
    /// the odd in-range-check cycle of the `i`-th evaluated sprite). `time_over`: `HBLANK_START_DOT`,
    /// since the tile fetch runs at dots 272+ and C7.06 only pins observability by the next line start.
    fn compute_over_flag_dots(&self, scan_y: u32) -> (Option<u16>, Option<u16>) {
        let first = if self.io.oam_priority_rotation {
            (self.io.oam_address >> 2) as usize & 0x7f
        } else {
            0
        };
        let mut range_count = 0usize;
        let mut tile_count = 0usize;
        let mut range_dot = None;
        let mut time_dot = None;
        for i in 0..128usize {
            let idx = (first + i) & 0x7f;
            let obj = self.object(idx);
            let (w, h) = self.object_size(obj.size);
            let h = h >> u32::from(self.io.obj_interlace);
            let top = u32::from(obj.y);
            let dy = (scan_y.wrapping_sub(top)) & 0xff;
            if dy >= h {
                continue;
            }
            if obj.x > 256 && obj.x + (w as u16) - 1 < 512 {
                continue;
            }
            range_count += 1;
            if range_count > 32 {
                // 33rd in-range sprite: range-over trips and evaluation stops (mirrors the paint
                // pass's `break`). `2 * i + 1` is the odd (in-range-check) cycle of sprite `i`.
                // `i < 128` so `2 * i + 1 <= 255`; `try_from` (not `as`) keeps the pedantic
                // truncation lint happy, and the fallback is unreachable.
                range_dot = Some(u16::try_from(2 * i + 1).unwrap_or(255));
                break;
            }
            tile_count += (w / 8) as usize;
            if tile_count > 34 && time_dot.is_none() {
                time_dot = Some(crate::HBLANK_START_DOT);
            }
        }
        (range_dot, time_dot)
    }

    /// Render sprites for the current scanline: range evaluation + pixel fetch. (The STAT77 over-flags
    /// are timed separately by [`Self::pd_eval_over_flags`], one line ahead.)
    fn render_objects(&self, pr: &ModePriorities, above: &mut [Pixel], below: &mut [Pixel]) {
        let (in_range, count, budget_ok) = self.eval_objects_range();
        self.paint_objects(pr, above, below, &in_range, count, &budget_ok);
    }

    /// Sprite range + tile-budget evaluation for the current scanline (the `render_objects`
    /// first phase). Collects up to 32 in-range sprites into `in_range`, sets the `$213E`
    /// range/time over-flags, and computes which survive the 34-tile fetch budget. Returns
    /// `(in_range, count, budget_ok)` for [`Ppu::paint_objects`] to draw. Split out from the
    /// paint so phase 4b can drive it one dot at a time (the per-dot compositor, `docs/adr/0014`);
    /// today it still runs whole-line, byte-identically.
    fn eval_objects_range(&self) -> ([u8; 32], usize, [bool; 32]) {
        let scan_y = u32::from(self.v - 1);

        // Range evaluation: collect up to 32 sprites that intersect this scanline. Lower index
        // = on top, so we iterate in index order and the painter respects priority + order.
        let first = if self.io.oam_priority_rotation {
            (self.io.oam_address >> 2) as usize & 0x7f
        } else {
            0
        };

        let mut in_range: [u8; 32] = [0; 32];
        let mut range_count = 0usize;

        for i in 0..128 {
            let idx = (first + i) & 0x7f;
            let obj = self.object(idx);
            let (w, h) = self.object_size(obj.size);
            // OBJ interlace ($2133 bit 1) halves the height a sprite occupies on screen — each
            // displayed line samples every other sprite row (ares `Object::onScanline`,
            // `height >> io.interlace`). A 16x32 sprite is in range for 16 lines, not 32.
            let h = h >> u32::from(self.io.obj_interlace);
            // Vertical intersection (Y wraps in 256).
            let top = u32::from(obj.y);
            let dy = (scan_y.wrapping_sub(top)) & 0xff;
            if dy >= h {
                continue;
            }
            // Horizontal on-screen check (sprite fully in 256..512 is off-screen).
            if obj.x > 256 && obj.x + (w as u16) - 1 < 512 {
                continue;
            }
            if range_count < 32 {
                in_range[range_count] = idx as u8;
            }
            range_count += 1;
            if range_count > 32 {
                break;
            }
        }

        // NOTE: `range_over`/`time_over` (STAT77 bits 6/7) are NOT set here. This pass evaluates the
        // sprites of the line being *drawn* (`scan_y = self.v - 1`) for painting; the over-flags belong
        // to the *evaluation* of the NEXT line's sprites, which hardware performs one line ahead at a
        // specific dot. That timing is driven separately by `pd_eval_over_flags` (MesenCE
        // `EvaluateNextLineSprites`; dossier C7.05/C7.06) so a cart reading `$213E` sees the flag set at
        // `V = OBJ.YLOC, H = OAM.INDEX*2`, not at the draw line's start.

        // Sprites paint in reverse index order so lower index ends up on top (last writer wins
        // among equal priority). We honor the 34-tile limit by dropping the lowest-index sprites
        // first (the HW fetches in reverse, so the lowest-index tiles are the first to be starved).
        let count = range_count.min(32);
        let mut budget_ok = [true; 32];
        let mut acc = 0usize;
        for k in (0..count).rev() {
            let obj = self.object(in_range[k] as usize);
            let (w, _) = self.object_size(obj.size);
            let cost = (w / 8) as usize;
            if acc + cost > 34 {
                budget_ok[k] = false;
            } else {
                acc += cost;
            }
        }

        (in_range, count, budget_ok)
    }

    /// Paint the evaluated, budget-surviving sprites into the `above`/`below` line buffers (the
    /// `render_objects` second phase). Consumes the `(in_range, count, budget_ok)` produced by
    /// [`Ppu::eval_objects_range`]. Kept a distinct phase so the per-dot compositor can fetch and
    /// paint sprite columns independently of range evaluation (`docs/adr/0014`, phase 4b).
    fn paint_objects(
        &self,
        pr: &ModePriorities,
        above: &mut [Pixel],
        below: &mut [Pixel],
        in_range: &[u8; 32],
        count: usize,
        budget_ok: &[bool; 32],
    ) {
        let main = self.io.main_enable[4];
        let sub = self.io.sub_enable[4];
        let scan_y = u32::from(self.v - 1);

        // Paint from highest index to lowest (so lowest index wins ties).
        for k in (0..count).rev() {
            if !budget_ok[k] {
                continue;
            }
            let idx = in_range[k] as usize;
            let obj = self.object(idx);
            // The height is not needed: the vertical-flip rule below is expressed in terms of the
            // width, and the row range is already bounded by sprite evaluation.
            let (w, _h) = self.object_size(obj.size);

            let mut row = (scan_y.wrapping_sub(u32::from(obj.y))) & 0xff;
            // OBJ interlace: the displayed line maps to twice the sprite row, so only every other
            // row is fetched (ares `Object::fetch`, `y <<= 1` before the flip). The field parity is
            // added after the flip so it selects even/odd rows per frame (`y += field`, `-` when
            // v-flipped). This squishes a 16x32 into the 16 lines the range test now allows.
            if self.io.obj_interlace {
                row <<= 1;
            }
            if obj.vflip {
                // Vertical flip is computed against the sprite's WIDTH, not its height, and that
                // is not a typo. For a square sprite the two are the same and this is the ordinary
                // whole-sprite flip. For the undocumented rectangular sizes (OBJSEL pairs 6 and 7,
                // whose members are 16x32 / 32x64 / 32x32) it means each square half flips inside
                // itself and the halves do NOT swap positions — the hardware quirk AccuracySNES
                // `C7.13` pins, and which the `c7-vflip-tall-halves` scene caught this core
                // getting wrong once the scene was corrected to use a genuinely tall sprite.
                row = if row < w {
                    w - 1 - row
                } else {
                    w * 3 - 1 - row
                };
            }
            if self.io.obj_interlace {
                // Field parity selects the even or odd sprite rows (ares `Object::fetch`,
                // `y = !vflip ? y + field : y - field`), applied after the flip.
                row = if obj.vflip {
                    row.wrapping_sub(u32::from(self.field))
                } else {
                    row + u32::from(self.field)
                } & 0xff;
            }

            let pal_base = 128 + (u16::from(obj.palette) << 4);
            let prio = pr.obj[usize::from(obj.priority)];

            let tile_row = (row / 8) & 0x0f;
            let fine_y = row & 7;
            let tiles_w = w / 8;

            let mut base = self.io.obj_tiledata_addr;
            if obj.nameselect {
                base = base.wrapping_add((1 + self.io.obj_nameselect) << 12);
            }
            let chr_x = u16::from(obj.character) & 0x0f;
            let chr_y = ((u16::from(obj.character) >> 4) + (tile_row as u16)) & 0x0f;

            for tx in 0..tiles_w {
                let sx = (u32::from(obj.x) + tx * 8) & 0x1ff;
                let mx = if obj.hflip { tiles_w - 1 - tx } else { tx } as u16;
                let char_idx = (chr_y << 4) | ((chr_x + mx) & 0x0f);
                // 4bpp sprite tile = 16 words; addressing matches obj layout.
                let tile_addr = (base.wrapping_add(char_idx << 4)) & 0xfff0;
                let plane01 = (tile_addr | (fine_y as u16)) & 0x7fff;

                // Computed once per 8-pixel column (not per pixel) -- `tile_addr` is already this
                // specific 8x8 sub-tile's raw pre-flip VRAM base, so every pixel in this column
                // shares one tag.
                #[cfg(feature = "hd-pack")]
                let obj_tag = self.hd_pack_tagging.then(|| {
                    self.tile_tag(
                        crate::hdtag::TileClass::Obj,
                        4,
                        tile_addr,
                        (pal_base & 0xff) as u8,
                        obj.hflip,
                        obj.vflip,
                    )
                });

                for col in 0..8u32 {
                    let screen_x = sx + col;
                    if screen_x >= SCREEN_WIDTH as u32 {
                        continue;
                    }
                    let bit = if obj.hflip { col } else { 7 - col } as u16;
                    let w0 = self.vram[plane01 as usize];
                    let w1 = self.vram[((plane01 + 8) & 0x7fff) as usize];
                    let c0 = ((w0 >> bit) & 1) as u8;
                    let c1 = ((w0 >> (8 + bit)) & 1) as u8;
                    let c2 = ((w1 >> bit) & 1) as u8;
                    let c3 = ((w1 >> (8 + bit)) & 1) as u8;
                    let color = c0 | (c1 << 1) | (c2 << 2) | (c3 << 3);
                    if color == 0 {
                        continue;
                    }
                    let pal = (pal_base + u16::from(color)) as u8;
                    let xi = screen_x as usize;
                    #[allow(unused_mut)]
                    let mut pixel = Pixel {
                        palette: pal,
                        priority: prio,
                        layer: 4,
                        palette_group: 0,
                        opaque: true,
                        ..Pixel::default()
                    };
                    #[cfg(feature = "hd-pack")]
                    if let Some(tag) = obj_tag {
                        pixel.tag = tag;
                    }
                    // We paint high-index sprites first, so a `>=` test lets a lower-index
                    // sprite at the same priority win the tie (it is painted later).
                    if main && !self.windowed_out(4, xi, true) && prio >= above[xi].priority {
                        above[xi] = pixel;
                    }
                    if sub && !self.windowed_out(4, xi, false) && prio >= below[xi].priority {
                        below[xi] = pixel;
                    }
                }
            }
        }
    }

    /// Whether the given layer is masked out by its window at column `x` on the main (`above`)
    /// or sub (`!above`) screen. Layer ids: 0..=3 bg, 4 obj.
    const fn windowed_out(&self, layer: usize, x: usize, above: bool) -> bool {
        let enable = if above {
            self.io.win_main_enable[layer]
        } else {
            self.io.win_sub_enable[layer]
        };
        if !enable {
            return false;
        }
        // Window layer index in WindowIo: bg1..4=0..3, obj=4.
        let wl = &self.io.win.layer[layer];
        let xb = x as u8;
        let one = xb >= self.io.win.one_left && xb <= self.io.win.one_right;
        let two = xb >= self.io.win.two_left && xb <= self.io.win.two_right;
        window_test(wl, one, two)
    }

    /// Test-only whole-line composite: the row loop that drives the shipped per-column
    /// [`Self::compose_pixel`], seeded at ares' pre-line DAC reset. The per-dot compositor drives
    /// `compose_pixel` one dot at a time instead ([`Self::pd_render_to_dot`]); this helper exists so
    /// the hi-res DAC column-threading tests below can feed a hand-built pixel row straight into that
    /// same per-column path without standing up full BG/tilemap register state.
    ///
    /// In hi-res (`self.frame_hires`) each input column `x` emits *two* output columns, mirroring
    /// ares' `PPU::DAC::run()`/`above()`/`below()` (`ref-proj/ares/ares/sfc/ppu/dac.cpp`): the
    /// "odd" column is always today's normal main-screen color-math result (`aboveColor` below,
    /// unchanged from the non-hires path — this is why the non-hires path stays byte-identical).
    /// The "even" column (`belowColor`) is the *subscreen's own* color, color-math'd with the
    /// operand roles swapped — but gated by the color-math state from the *previous* column's
    /// `aboveColor` pass, not this column's own (a genuine one-pixel-clock-delayed hardware
    /// pipeline stage, not a translation artifact — see `docs/ppu.md` §Hi-res (Modes 5/6)
    /// color-math precision for the full derivation). The `DacCarry` value threaded below carries
    /// that delayed state; it starts at the documented power-on/scanline-start boundary (ares
    /// `DAC::scanline()`): no color math enabled, raw color = backdrop — which is exactly why the
    /// first hires column of every scanline is transparent on real hardware.
    #[cfg(test)]
    fn compose_dac(&mut self, row: usize, above: &[Pixel], below: &[Pixel]) {
        let ctx = LineCtx {
            base: row * self.visible_width(),
            brightness: u32::from(self.io.display_brightness),
            hires: self.frame_hires,
        };

        // Threaded left-to-right: each column composites from its own layers plus the PREVIOUS
        // column's DAC carry (the hi-res below-pass). Seeded as ares' pre-line reset. This is the
        // per-pixel decomposition the per-dot compositor drives (`docs/adr/0014`, Phase 1).
        let mut carry = DacCarry {
            above_enable: false,
            below_enable: false,
            above_raw: self.cgram[0],
            below_opaque: false,
        };
        for x in 0..SCREEN_WIDTH {
            carry = self.compose_pixel(x, above[x], below[x], ctx, carry);
        }
    }

    /// Composite one output column into the framebuffer and return the DAC carry-state the NEXT
    /// column's hi-res below-pass consumes. Bit-identical to the former inline `compose_dac` loop
    /// body — the per-pixel entry point the per-dot compositor drives (`docs/adr/0014`).
    #[inline]
    fn compose_pixel(
        &mut self,
        x: usize,
        ap: Pixel,
        bp: Pixel,
        ctx: LineCtx,
        prev: DacCarry,
    ) -> DacCarry {
        let LineCtx {
            base,
            brightness,
            hires,
        } = ctx;
        // Main color.
        let main_color = self.layer_color(&ap);

        // Determine whether color math applies to this main pixel's layer.
        let math_layer = match ap.layer {
            0..=3 => self.io.color_math_enable[ap.layer as usize],
            4 => self.io.color_math_enable[4] && ap.palette >= 192,
            _ => self.io.color_math_enable[5], // backdrop
        };

        // Color window: above mask gates whether main is forced black; below mask gates math.
        let col_win = self.color_window(x);
        let math_allowed = self.math_region_allowed(col_win, false);
        let main_force_black = !self.math_region_allowed(col_win, true);
        let above_enable = !main_force_black;
        let below_enable = math_layer && math_allowed;

        let mut out = if main_force_black { 0 } else { main_color };

        if below_enable {
            // SNES color-math addend selection (ares `DAC::above`): the subscreen is the
            // addend ONLY when "add subscreen" is enabled AND the subscreen pixel is opaque
            // (a real layer wrote it). When the subscreen pixel is the backdrop (transparent),
            // the hardware falls back to the COLDATA fixed color even with add-subscreen on —
            // this is what paints SMW's blue sky (fixed_color) over the black main backdrop.
            let use_subscreen = self.io.add_subscreen && bp.opaque;
            let addend = if use_subscreen {
                self.layer_color(&bp)
            } else {
                self.io.fixed_color
            };
            // Halving applies only when the main pixel is not forced black and (for the
            // subscreen addend) the subscreen is opaque — matching ares' `colorHalve` gate.
            let halve =
                self.io.color_halve && above_enable && (!self.io.add_subscreen || bp.opaque);
            out = if self.io.color_subtract {
                color_sub(out, addend, halve)
            } else {
                color_add(out, addend, halve)
            };
        }

        if hires {
            // `layer_color` already falls back to `cgram[0]` for a non-opaque pixel (the
            // same fallback ares' `below()` priority-resolution applies when nothing wrote
            // this column on the subscreen), so no separate opacity check is needed here.
            let below_screen_color = self.layer_color(&bp);
            let mut below_out = if prev.above_enable {
                below_screen_color
            } else {
                0
            };
            if prev.below_enable {
                // The one-column-delayed mirror of `above()`'s addend/halve selection: the
                // "blend mode" and halve gates ares recomputes each column from that column's
                // OWN below-opacity, then that recomputed value is what the NEXT column's
                // below-pass actually consumes (`math.blendMode`/`math.colorHalve`, dac.cpp
                // lines 124-129) — hence `prev.below_opaque`, not this column's `bp.opaque`.
                let prev_blend_mode = self.io.add_subscreen && prev.below_opaque;
                let addend = if prev_blend_mode {
                    prev.above_raw
                } else {
                    self.io.fixed_color
                };
                let halve = self.io.color_halve
                    && prev.above_enable
                    && (!self.io.add_subscreen || prev.below_opaque);
                below_out = if self.io.color_subtract {
                    color_sub(below_out, addend, halve)
                } else {
                    color_add(below_out, addend, halve)
                };
            }
            self.framebuffer[base + 2 * x] = apply_brightness(below_out, brightness);
            self.framebuffer[base + 2 * x + 1] = apply_brightness(out, brightness);
            #[cfg(feature = "hd-pack")]
            if self.hd_pack_tagging {
                self.tile_tags[base + 2 * x] = bp.tag;
                self.tile_tags[base + 2 * x + 1] = ap.tag;
            }
        } else {
            self.framebuffer[base + x] = apply_brightness(out, brightness);
            #[cfg(feature = "hd-pack")]
            if self.hd_pack_tagging {
                self.tile_tags[base + x] = ap.tag;
            }
        }

        DacCarry {
            above_enable,
            below_enable,
            above_raw: main_color,
            below_opaque: bp.opaque,
        }
    }

    /// The 15-bit color for a composited layer pixel (direct-color for BG1 in modes 3/4/7).
    fn layer_color(&self, p: &Pixel) -> u16 {
        if !p.opaque {
            return self.cgram[0];
        }
        if p.layer == 0 && self.io.direct_color && matches!(self.io.bg_mode, 3 | 4 | 7) {
            direct_color(p.palette, p.palette_group)
        } else {
            self.cgram[usize::from(p.palette)]
        }
    }

    /// Evaluate the color-math window at column x (true = inside the col window region).
    const fn color_window(&self, x: usize) -> bool {
        let wl = &self.io.win.layer[5];
        let xb = x as u8;
        let one = xb >= self.io.win.one_left && xb <= self.io.win.one_right;
        let two = xb >= self.io.win.two_left && xb <= self.io.win.two_right;
        window_test(wl, one, two)
    }

    /// Resolve the 2-bit CGWSEL mask (0=always,1=inside-win,2=outside-win,3=never) against the
    /// color-window value. `above`=true uses the force-main-black mask, else the math mask.
    const fn math_region_allowed(&self, in_window: bool, above: bool) -> bool {
        let mask = if above {
            self.io.color_window_above
        } else {
            self.io.color_window_below
        };
        match mask {
            0 => true,
            1 => in_window,
            2 => !in_window,
            _ => false,
        }
    }
}

/// SNES window combine: OR/AND/XOR/XNOR with per-window enable + invert.
const fn window_test(wl: &crate::WindowLayer, one_raw: bool, two_raw: bool) -> bool {
    let one = one_raw ^ wl.one_invert;
    let two = two_raw ^ wl.two_invert;
    if !wl.one_enable {
        return wl.two_enable && two;
    }
    if !wl.two_enable {
        return one;
    }
    match wl.mask {
        0 => one | two,
        1 => one & two,
        2 => one ^ two,
        _ => !(one ^ two),
    }
}

/// 15-bit per-channel saturating add (with optional halve), SNES color-math semantics.
fn color_add(x: u16, y: u16, halve: bool) -> u16 {
    let chan = |s: u8, a: u16, b: u16| -> u16 {
        let mut v = ((a >> s) & 0x1f) + ((b >> s) & 0x1f);
        if halve {
            v >>= 1;
        }
        if v > 0x1f {
            v = 0x1f;
        }
        v << s
    };
    chan(0, x, y) | chan(5, x, y) | chan(10, x, y)
}

/// 15-bit per-channel saturating subtract (with optional halve).
fn color_sub(x: u16, y: u16, halve: bool) -> u16 {
    let chan = |s: u8, a: u16, b: u16| -> u16 {
        let av = ((a >> s) & 0x1f) as i16;
        let bv = ((b >> s) & 0x1f) as i16;
        let mut v = av - bv;
        if v < 0 {
            v = 0;
        }
        if halve {
            v >>= 1;
        }
        (v as u16) << s
    };
    chan(0, x, y) | chan(5, x, y) | chan(10, x, y)
}

/// Mode 3/4/7 direct-color expansion: palette index bits become the color directly.
fn direct_color(palette: u8, group: u8) -> u16 {
    let p = u16::from(palette);
    let g = u16::from(group);
    (p << 7 & 0x6000)
        | (g << 10 & 0x1000)
        | (p << 4 & 0x0380)
        | (g << 5 & 0x0040)
        | (p << 2 & 0x001c)
        | (g << 1 & 0x0002)
}

/// Apply the INIDISP master brightness (0..=15) to a 15-bit color (15/16 scaling per step).
fn apply_brightness(color: u16, brightness: u32) -> u16 {
    if brightness == 15 {
        return color;
    }
    if brightness == 0 {
        return 0;
    }
    let chan = |s: u32| -> u16 {
        let v = u32::from((color >> s) & 0x1f);
        let scaled = (v * (brightness + 1)) / 16;
        (scaled.min(0x1f) as u16) << s
    };
    chan(0) | chan(5) | chan(10)
}

#[cfg(test)]
mod tests {
    use crate::bus::NullVideoBus;
    use crate::{DOTS_PER_LINE, Ppu, SCREEN_WIDTH};

    /// Helper: write a VRAM word at `addr` via the register path (increment-on-high mode).
    fn vram_set(p: &mut Ppu, addr: u16, word: u16) {
        p.write_reg(0x2115, 0x80); // VMAIN: step 1, increment on high
        p.write_reg(0x2116, (addr & 0xff) as u8);
        p.write_reg(0x2117, (addr >> 8) as u8);
        p.write_reg(0x2118, (word & 0xff) as u8);
        p.write_reg(0x2119, (word >> 8) as u8);
    }

    /// Helper: set a CGRAM color via the register path.
    fn cgram_set(p: &mut Ppu, index: u8, color: u16) {
        p.write_reg(0x2121, index);
        p.write_reg(0x2122, (color & 0xff) as u8);
        p.write_reg(0x2122, (color >> 8) as u8);
    }

    /// Run a full NTSC frame.
    fn run_frame(p: &mut Ppu) {
        let mut bus = NullVideoBus;
        let total = u32::from(DOTS_PER_LINE) * 262;
        for _ in 0..total {
            p.tick_dot(&mut bus);
        }
    }

    #[test]
    fn vram_write_read_roundtrip_with_prefetch() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80); // force-blank so VRAM is accessible
        vram_set(&mut p, 0x0010, 0xbeef);
        assert_eq!(p.vram_word(0x0010), 0xbeef);

        // Read path: set address, first read returns prefetch (the word at 0x0010), then advances.
        p.write_reg(0x2115, 0x00); // increment on low read ($2139)
        p.write_reg(0x2116, 0x10);
        p.write_reg(0x2117, 0x00);
        // $2116/7 prefetch the word at 0x0010.
        let lo = p.read_reg(0x2139);
        let hi = p.read_reg(0x213a);
        assert_eq!(u16::from(lo) | (u16::from(hi) << 8), 0xbeef);
    }

    #[test]
    fn vram_increment_modes() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80);
        // step 32 words, increment on high.
        p.write_reg(0x2115, 0x81);
        p.write_reg(0x2116, 0x00);
        p.write_reg(0x2117, 0x00);
        p.write_reg(0x2118, 0x11);
        p.write_reg(0x2119, 0x22); // commits at addr 0, then +32
        p.write_reg(0x2118, 0x33);
        p.write_reg(0x2119, 0x44); // commits at addr 32
        assert_eq!(p.vram_word(0), 0x2211);
        assert_eq!(p.vram_word(32), 0x4433);
    }

    #[test]
    fn cgram_write_twice_and_read_twice() {
        let mut p = Ppu::new();
        cgram_set(&mut p, 5, 0x7abc & 0x7fff);
        assert_eq!(p.cgram_word(5), 0x7abc & 0x7fff);
        // Read back via $213B (read twice).
        p.write_reg(0x2121, 5);
        let lo = p.read_reg(0x213b);
        let hi = p.read_reg(0x213b);
        let got = u16::from(lo) | (u16::from(hi & 0x7f) << 8);
        assert_eq!(got, 0x7abc & 0x7fff);
    }

    #[test]
    fn oam_write_read_roundtrip() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80);
        // Set OAM address to 0 and write sprite 0's 4 bytes (write-twice even latch).
        p.write_reg(0x2102, 0x00);
        p.write_reg(0x2103, 0x00);
        p.write_reg(0x2104, 0x50); // x-low
        p.write_reg(0x2104, 0x60); // y
        p.write_reg(0x2104, 0x01); // tile-low
        p.write_reg(0x2104, 0x30); // attr (priority=3)
        assert_eq!(p.oam_byte(0), 0x50);
        assert_eq!(p.oam_byte(1), 0x60);
        assert_eq!(p.oam_byte(2), 0x01);
        assert_eq!(p.oam_byte(3), 0x30);
    }

    /// OBJ interlace (`SETINI` $2133 bit 1) halves the on-screen height of a sprite: a 16x32 sprite
    /// occupies 16 scanlines, sampling every other row (ares `Object::onScanline`/`fetch`). Ported
    /// from ares; with it in place RustySNES matched Mesen2 exactly on a rendered 16x32 scene, and
    /// the existing `c7-*` sprite scenes (interlace off) are unregressed.
    #[test]
    fn obj_interlace_halves_sprite_height() {
        let extent = |interlace: bool| -> usize {
            let mut p = Ppu::new();
            p.write_reg(0x2100, 0x80); // force-blank for setup
            // Make every sprite tile fully opaque: all-ones bitplanes across the tile region.
            for addr in 0..0x400u16 {
                vram_set(&mut p, addr, 0xffff);
            }
            // All-ones bitplanes give colour 15; sprite palette 0 -> CGRAM 128 + 15 = 143.
            cgram_set(&mut p, 143, 0x7fff); // backdrop (CGRAM 0) stays black
            // OAM sprite 0 at (100, 100), tile 0, palette 0, priority 3. The high table stays zero,
            // so the size bit is clear (the pair's small 16x32 member) and X bit 8 is clear. Placed
            // well away from the 127 unused sprites, which default to (0,0) and also carry the now-
            // opaque tile 0 — so the measured column (102) and rows (100+) see only sprite 0.
            p.write_reg(0x2102, 0x00);
            p.write_reg(0x2103, 0x00);
            p.write_reg(0x2104, 100);
            p.write_reg(0x2104, 100);
            p.write_reg(0x2104, 0x00);
            p.write_reg(0x2104, 0x30);
            p.write_reg(0x2101, 0xc0); // OBJSEL pair 6: 16x32 / 32x64, name base 0
            if interlace {
                p.write_reg(0x2133, 0x02); // SETINI bit 1: OBJ interlace
            }
            p.write_reg(0x212c, 0x10); // OBJ on the main screen
            p.write_reg(0x2100, 0x0f); // display on, full brightness
            run_frame(&mut p);
            let fb = p.framebuffer();
            // Count scanlines whose pixel inside the sprite's column is opaque (non-backdrop).
            (0..crate::SCREEN_HEIGHT)
                .filter(|&y| fb[y * SCREEN_WIDTH + 102] != 0)
                .count()
        };
        assert_eq!(
            extent(false),
            32,
            "a 16x32 sprite spans 32 lines without OBJ interlace"
        );
        assert_eq!(
            extent(true),
            16,
            "OBJ interlace should squish the 16x32 sprite to 16 lines"
        );
    }

    #[test]
    fn force_blank_renders_black() {
        let mut p = Ppu::new();
        // Backdrop is bright white but force-blank should win.
        cgram_set(&mut p, 0, 0x7fff);
        p.write_reg(0x2100, 0x8f); // force-blank + full brightness
        run_frame(&mut p);
        let fb = p.framebuffer();
        assert!(fb[..SCREEN_WIDTH].iter().all(|&c| c == 0));
    }

    #[test]
    fn backdrop_renders_when_enabled() {
        let mut p = Ppu::new();
        cgram_set(&mut p, 0, 0x1234 & 0x7fff);
        p.write_reg(0x2100, 0x0f); // display on, full brightness
        run_frame(&mut p);
        let fb = p.framebuffer();
        assert_eq!(fb[0], 0x1234 & 0x7fff);
    }

    #[test]
    fn mode0_bg_renders_one_tile() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80); // force-blank for setup

        // Mode 0, BG1 enabled on main screen.
        p.write_reg(0x2105, 0x00);
        p.write_reg(0x2107, 0x00); // BG1 tilemap base = word 0
        p.write_reg(0x210b, 0x01); // BG1 char base = word 0x1000

        // Palette: BG1 region in mode 0 is colors 0..31. Color 1 = red.
        cgram_set(&mut p, 1, 0x001f); // red (low 5 bits)

        // Tilemap entry at (0,0): character 0, palette group 0, priority 0.
        vram_set(&mut p, 0x0000, 0x0000);

        // Tile 0 char data at word 0x1000: 2bpp, one word per row, plane0 in the low byte.
        // The marker goes on tile row **1**, not row 0, because the first displayed scanline shows
        // BG row `BGnVOFS + 1` — the background is fetched a line ahead of the line it appears on.
        // Row 0 is left blank so the assertions below can tell the two apart.
        vram_set(&mut p, 0x1000, 0x0000); // tile row 0: nothing
        vram_set(&mut p, 0x1001, 0x0080); // tile row 1: bit 7 => leftmost pixel is color 1

        // Enable display + BG1 main.
        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x01); // TM: BG1
        run_frame(&mut p);

        let fb = p.framebuffer();
        // Top-left pixel should be red (color 1) — from BG row 1, per the fetch-ahead above.
        assert_eq!(
            fb[0], 0x001f,
            "the first displayed line must show BG row BGnVOFS+1, not row 0"
        );
        // Next pixel (color 0) is backdrop = 0.
        assert_eq!(fb[1], 0x0000);
        // And the blank BG row 0 must not appear anywhere: it is fetched for scanline 0, which is
        // not displayed. Line 2 shows BG row 2, still blank, so only line 1 carries the marker.
        assert_eq!(fb[crate::SCREEN_WIDTH], 0x0000);
    }

    /// Builds the exact `mode0_bg_renders_one_tile` scene on a fresh [`Ppu`], applying `setup`
    /// to it beforehand (used to flip `hd_pack_tagging` on/off before the frame renders).
    #[cfg(feature = "hd-pack")]
    fn render_mode0_one_tile_scene(setup: impl FnOnce(&mut Ppu)) -> Ppu {
        let mut p = Ppu::new();
        setup(&mut p);
        p.write_reg(0x2100, 0x80);
        p.write_reg(0x2105, 0x00);
        p.write_reg(0x2107, 0x00);
        p.write_reg(0x210b, 0x01);
        cgram_set(&mut p, 1, 0x001f);
        vram_set(&mut p, 0x0000, 0x0000);
        vram_set(&mut p, 0x1000, 0x0080);
        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x01);
        run_frame(&mut p);
        p
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn hd_pack_tagging_toggle_does_not_alter_framebuffer_output() {
        let off = render_mode0_one_tile_scene(|_| {});
        let on = render_mode0_one_tile_scene(|p| p.set_hd_pack_tagging(true));
        assert_eq!(
            off.framebuffer(),
            on.framebuffer(),
            "toggling hd_pack_tagging must never change the composited framebuffer"
        );
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn hd_pack_tagging_off_leaves_tile_tags_untouched() {
        let p = render_mode0_one_tile_scene(|_| {});
        assert!(
            p.tile_tags()
                .iter()
                .all(|t| *t == crate::hdtag::TileTag::default()),
            "tile_tags must stay all-default when hd_pack_tagging was never enabled"
        );
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn turning_tagging_off_clears_stale_tags_from_a_prior_frame() {
        let mut p = Ppu::new();
        p.set_hd_pack_tagging(true);
        p.write_reg(0x2100, 0x80);
        p.write_reg(0x2105, 0x00);
        p.write_reg(0x2107, 0x00);
        p.write_reg(0x210b, 0x01);
        cgram_set(&mut p, 1, 0x001f);
        vram_set(&mut p, 0x0000, 0x0000);
        vram_set(&mut p, 0x1000, 0x0080);
        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x01);
        run_frame(&mut p);
        assert_ne!(
            p.tile_tags()[0].hash,
            0,
            "sanity: tagging-on frame actually recorded a tag"
        );

        p.set_hd_pack_tagging(false);
        assert!(
            p.tile_tags()
                .iter()
                .all(|t| *t == crate::hdtag::TileTag::default()),
            "turning tagging off must clear stale tags from the last tagged frame, not just stop \
             updating them"
        );
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn hd_pack_tagging_records_the_documented_hash_for_a_known_bg_tile() {
        let p = render_mode0_one_tile_scene(|p| p.set_hd_pack_tagging(true));

        let tag0 = p.tile_tags()[0];
        assert_ne!(
            tag0.hash, 0,
            "tile 0's opaque pixel must record a nonzero tile hash"
        );
        assert!(!tag0.hflip);
        assert!(!tag0.vflip);

        // Independently recompute the same hash from the raw tile 0 words (2bpp => 8 words) and
        // BG1's mode-0 palette region (colors 0..=3, since bg index 0 and palette group 0) to
        // prove the recorded value is the documented recipe, not just "some function".
        let words: alloc::vec::Vec<u16> = (0..8).map(|i| p.vram_word(0x1000 + i)).collect();
        let palette: alloc::vec::Vec<u16> = (0..4).map(|i| p.cgram_word(i)).collect();
        let expected = crate::hdtag::hash_tile(crate::hdtag::TileClass::Bg, 2, &words, &palette);
        assert_eq!(tag0.hash, expected);

        // The backdrop pixel (column 1, per `mode0_bg_renders_one_tile`) was never tagged.
        assert_eq!(p.tile_tags()[1], crate::hdtag::TileTag::default());
    }

    #[test]
    fn one_sprite_renders_at_oam_coordinate() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80);

        // OBSEL: base size 0 (8x8 small), tile base word 0.
        p.write_reg(0x2101, 0x00);

        // Sprite 0 at x=10, y=0, tile 0, palette 0, priority 2.
        p.write_reg(0x2102, 0x00);
        p.write_reg(0x2103, 0x00);
        p.write_reg(0x2104, 10); // x
        p.write_reg(0x2104, 0); // y
        p.write_reg(0x2104, 0); // tile
        p.write_reg(0x2104, 0x20); // attr: priority 2

        // Sprite palette starts at CGRAM 128. palette group 0 => colors 128..135.
        cgram_set(&mut p, 129, 0x03e0); // green => color 1

        // Sprite tile 0 (4bpp). Top-left pixel = color 1: plane0 row0 bit7 = 1.
        vram_set(&mut p, 0x0000, 0x0080);

        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x10); // TM: OBJ
        run_frame(&mut p);

        let fb = p.framebuffer();
        // Row 0, x=10 should be green.
        assert_eq!(fb[10], 0x03e0);
    }

    #[test]
    fn range_over_flag_sets_with_many_sprites() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80);
        p.write_reg(0x2101, 0x00); // small 8x8

        // 40 sprites all on scanline 0 (y=0), spread across x.
        p.write_reg(0x2102, 0x00);
        p.write_reg(0x2103, 0x00);
        for i in 0..40u16 {
            p.write_reg(0x2104, ((i * 6) & 0xff) as u8); // x
            p.write_reg(0x2104, 0); // y=0
            p.write_reg(0x2104, 0); // tile
            p.write_reg(0x2104, 0x20); // attr
        }
        // Make tile 0 nonempty so they actually fetch.
        vram_set(&mut p, 0x0000, 0x0080);
        cgram_set(&mut p, 129, 0x03e0);

        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x10);
        // Render the visible portion only, then sample during VBlank (before the end-of-frame
        // reset that clears the over-flags at the start of the next frame).
        let mut bus = NullVideoBus;
        for _ in 0..(u32::from(DOTS_PER_LINE) * 230) {
            p.tick_dot(&mut bus);
        }

        // STAT77 bit 6 (range over) should be set.
        let stat = p.read_reg(0x213e);
        assert!(stat & 0x40 != 0, "range-over flag not set: {stat:#04x}");
    }

    #[test]
    fn incremental_range_over_sets_on_next_line_at_the_33rd_sprite() {
        // 40 8x8 sprites at Y=100 (indices 0-39, seed 0), the other 88 parked off-screen. The 33rd
        // in-range sprite is OAM index 32, evaluated during display line 100 (the line whose sprites
        // paint on 101), at its odd in-range-check cycle `2*32+1 = 65`. `range_over` must trip exactly
        // there (dossier C7.05: V = OBJ.YLOC, H = OAM.INDEX*2), NOT at the draw line's start.
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80);
        p.write_reg(0x2101, 0x00);
        p.write_reg(0x2102, 0x00);
        p.write_reg(0x2103, 0x00);
        for i in 0..128u16 {
            if i < 40 {
                p.write_reg(0x2104, ((i * 6) & 0xff) as u8);
                p.write_reg(0x2104, 100);
            } else {
                p.write_reg(0x2104, 0x00);
                p.write_reg(0x2104, 0xf0);
            }
            p.write_reg(0x2104, 0);
            p.write_reg(0x2104, 0x20);
        }
        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x10);

        let mut bus = NullVideoBus;
        let mut first: Option<(u16, u16)> = None;
        for _ in 0..(u32::from(DOTS_PER_LINE) * 130) {
            let before = p.io.range_over;
            p.tick_dot(&mut bus);
            if !before && p.io.range_over {
                first = Some((p.v, p.h));
                break;
            }
        }
        assert_eq!(
            first,
            Some((100, 66)),
            "range_over must trip during line 100 at the 33rd sprite's eval dot, not the draw line"
        );
    }

    #[test]
    fn mode7_identity_maps_1to1() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80);
        p.write_reg(0x2105, 0x07); // Mode 7

        // Identity matrix: A=1.0 (0x0100), B=0, C=0, D=1.0 (0x0100), center (0,0), scroll 0.
        p.write_reg(0x211b, 0x00); // M7A low
        p.write_reg(0x211b, 0x01); // M7A high => 0x0100
        p.write_reg(0x211c, 0x00);
        p.write_reg(0x211c, 0x00); // M7B = 0
        p.write_reg(0x211d, 0x00);
        p.write_reg(0x211d, 0x00); // M7C = 0
        p.write_reg(0x211e, 0x00);
        p.write_reg(0x211e, 0x01); // M7D = 0x0100
        p.write_reg(0x211f, 0x00);
        p.write_reg(0x211f, 0x00); // M7X = 0
        p.write_reg(0x2120, 0x00);
        p.write_reg(0x2120, 0x00); // M7Y = 0
        // BG1 scroll (mode7) = 0.
        p.write_reg(0x210d, 0x00);
        p.write_reg(0x210d, 0x00);
        p.write_reg(0x210e, 0x00);
        p.write_reg(0x210e, 0x00);

        // Tile (0,0) in the 128x128 map = tile index N. Put tile #1 at map (0,0).
        // Map entry word at addr 0: low byte = tile number.
        vram_set(&mut p, 0x0000, 0x0001);
        // Mode 7 char data: char addr = tile<<6 | (y<<3 | x), palette in the high byte.
        //
        // The marker goes at map row **1**, not row 0, for the same reason as
        // `mode0_bg_renders_one_tile`: the fetch runs a line ahead of the line it appears on, so
        // the first displayed scanline shows map row 1. Row 0 is left blank to tell the two apart.
        vram_set(&mut p, 0x0048, 0x0100); // tile 1, pixel (0,1)
        cgram_set(&mut p, 1, 0x7c00); // blue
        cgram_set(&mut p, 2, 0x001f); // red — the row-0 marker, which must NOT appear on line 1
        vram_set(&mut p, 0x0040, 0x0200); // tile 1, pixel (0,0)

        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x01); // TM: BG1
        run_frame(&mut p);

        let fb = p.framebuffer();
        assert_eq!(
            fb[0], 0x7c00,
            "the first displayed line must show Mode 7 map row 1, not row 0"
        );
        // And map row 0 must appear nowhere: it is fetched for scanline 0, which is not displayed.
        assert!(
            !fb.contains(&0x001f),
            "Mode 7 map row 0 was displayed; it belongs to the undisplayed scanline 0"
        );
    }

    #[test]
    fn hv_counter_latch_via_slhv() {
        let mut p = Ppu::new();
        let mut bus = NullVideoBus;
        // Advance a bit.
        for _ in 0..500 {
            p.tick_dot(&mut bus);
        }
        let h = p.dot();
        let v = p.scanline();
        let _ = p.read_reg(0x2137); // SLHV latches
        // OPHCT read twice.
        let hl = p.read_reg(0x213c);
        let hh = p.read_reg(0x213c);
        let latched_h = u16::from(hl) | (u16::from(hh & 1) << 8);
        let vl = p.read_reg(0x213d);
        let vh = p.read_reg(0x213d);
        let latched_v = u16::from(vl) | (u16::from(vh & 1) << 8);
        assert_eq!(latched_h, h);
        assert_eq!(latched_v, v);
        // STAT78 read clears latch.
        let stat = p.read_reg(0x213f);
        assert!(stat & 0x40 != 0); // counter was latched
        let stat2 = p.read_reg(0x213f);
        assert!(stat2 & 0x40 == 0); // now cleared
    }

    #[test]
    fn mpy_readback_mode7_multiply() {
        let mut p = Ppu::new();
        // M7A = 2 (0x0002), M7B high = 3 => product = 6.
        p.write_reg(0x211b, 0x02);
        p.write_reg(0x211b, 0x00); // M7A = 0x0002
        p.write_reg(0x211c, 0x00);
        p.write_reg(0x211c, 0x03); // M7B = 0x0300, high byte = 3
        let l = p.read_reg(0x2134);
        let m = p.read_reg(0x2135);
        let h = p.read_reg(0x2136);
        let product = u32::from(l) | (u32::from(m) << 8) | (u32::from(h) << 16);
        assert_eq!(product, 6);
    }

    #[test]
    fn color_math_add_fixed() {
        // add 0x0010 + 0x0010 => 0x0020 in the red channel? red is low 5 bits.
        let r = super::color_add(0x0010, 0x0008, false);
        assert_eq!(r & 0x1f, 0x18);
    }

    #[test]
    fn deterministic_frames_identical() {
        let mut a = Ppu::new();
        let mut b = Ppu::new();
        for p in [&mut a, &mut b] {
            cgram_set(p, 0, 0x1234 & 0x7fff);
            p.write_reg(0x2100, 0x0f);
            run_frame(p);
        }
        assert_eq!(a.framebuffer(), b.framebuffer());
    }

    // --- Hi-res (Modes 5/6) DAC: the one-pixel-clock-delayed below-color pass ---
    // (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision). `compose_dac` and `Pixel` are
    // called/constructed directly here rather than through full BG/tilemap register setup: the
    // mechanism under test is entirely in the DAC's column-to-column state threading, which a
    // hand-built `Pixel` row isolates far more precisely than an incidental tile pattern would.
    use super::Pixel;

    /// An opaque BG1 pixel with the given palette index (color-math layer 0).
    // Not `const fn`: `..Pixel::default()` needs `Default::default()`, which isn't `const` --
    // fine, both helpers are test-only and never used in a const context.
    fn bg1_pixel(palette: u8) -> Pixel {
        Pixel {
            palette,
            priority: 1,
            layer: 0,
            palette_group: 0,
            opaque: true,
            ..Pixel::default()
        }
    }

    /// The default (backdrop) pixel: transparent, CGRAM 0.
    fn backdrop_pixel() -> Pixel {
        Pixel {
            palette: 0,
            priority: 0,
            layer: 5,
            palette_group: 0,
            opaque: false,
            ..Pixel::default()
        }
    }

    #[test]
    fn hires_first_column_of_scanline_is_always_transparent() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display on, full brightness
        cgram_set(&mut p, 1, 0x7fff); // BG1 palette 1 = bright white
        p.write_reg(0x2131, 0x01); // CGADSUB: BG1 color-math enabled
        p.frame_hires = true;

        // Every column strongly opaque + math-enabled on both screens — if the x=0 boundary
        // condition were wrong, this is exactly the input that would make it obviously non-zero.
        let above = [bg1_pixel(1); SCREEN_WIDTH];
        let below = [bg1_pixel(1); SCREEN_WIDTH];
        p.compose_dac(0, &above, &below);

        let fb = p.framebuffer();
        assert_eq!(
            fb[0], 0,
            "the first hires pixel of every scanline is documented as transparent on real \
             hardware (ares DAC::scanline()'s power-on/scanline-start boundary) — this must hold \
             regardless of how strongly the column-0 pixel data would otherwise composite"
        );
        assert_ne!(
            fb[1], 0,
            "the odd/above column at the same PPU pixel clock is the normal, unaffected \
             main-screen composite — it must NOT inherit the below-column's transparency"
        );
    }

    #[test]
    fn hires_below_color_depends_on_previous_column_not_its_own() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f);
        cgram_set(&mut p, 1, 0x7fff); // BG1 color = white
        cgram_set(&mut p, 0, 0x0400); // backdrop (CGRAM 0) = a mid blue, so column 1's
        // below-screen color (backdrop, since column 1 is never BG1-opaque here) is nonzero —
        // otherwise an add-of-zero would mask the enable-gate difference this test isolates.
        p.write_reg(0x2131, 0x01); // BG1 color-math enabled
        p.io.fixed_color = 0x0010; // COLDATA: nonzero, so the "was color-math applied" gate
        // (gated on column 0's, not column 1's, state) actually changes column 1's output.
        p.frame_hires = true;

        // Column 1 is held IDENTICAL across both runs (backdrop only); only column 0 changes
        // (opaque math-enabled BG1 vs. plain backdrop). If column 1's belowColor is computed
        // from column 0's state (the documented one-pixel-clock delay), it must differ between
        // the two runs despite column 1's own pixel data never changing.
        let below_col0_backdrop = [backdrop_pixel(); SCREEN_WIDTH];
        let above_col0_backdrop = below_col0_backdrop;

        let mut above_col0_bg1 = above_col0_backdrop;
        above_col0_bg1[0] = bg1_pixel(1);
        let below_col0_bg1 = above_col0_bg1;

        p.compose_dac(0, &above_col0_backdrop, &below_col0_backdrop);
        let fb_backdrop = p.framebuffer().to_vec();

        p.compose_dac(0, &above_col0_bg1, &below_col0_bg1);
        let fb_bg1 = p.framebuffer().to_vec();

        // Column 1's own input pixels are backdrop in BOTH runs — only column 0 differs.
        assert_ne!(
            fb_backdrop[2], fb_bg1[2],
            "column 1's belowColor (the even/hires output column) must depend on column 0's \
             above-pass state, not column 1's own (unchanged-between-runs) pixel data"
        );
        // Column 1's aboveColor (the odd column, today's ordinary composited path) must be
        // identical in both runs — it never reads any other column's state.
        assert_eq!(
            fb_backdrop[3], fb_bg1[3],
            "column 1's aboveColor must be unaffected by column 0's state — only the hires \
             below-pass has the one-column delay"
        );
    }

    // --- Per-dot compositor: the exact in-render CGRAM write redirect (T-CA-10, dossier C3.04).
    //
    // The redirect target is `internal_cgram_address` — the palette of the last column the per-dot
    // compositor drew (MesenCE `_state.InternalCgramAddress`), maintained live by `pd_render_to_dot`.
    // These unit tests set it directly to exercise the NEW logic: the active-display gate, and that a
    // mid-display write commits to that drawn-palette index rather than the CPU-programmed index.

    #[test]
    fn cgram_write_during_active_display_redirects_to_drawn_palette() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display ENABLED, brightness 15
        // Inside the active-display window (line 50, dot 100), the DAC last drew palette 7.
        p.v = 50;
        p.h = 100;
        p.internal_cgram_address = 7;
        // A NON-redirected write would land at the programmed index 5.
        p.write_reg(0x2121, 0x05);
        p.write_reg(0x2122, 0x34);
        p.write_reg(0x2122, 0x12); // commits word $1234
        assert_eq!(
            p.cgram[7], 0x1234,
            "the in-render write must hit the color being drawn (internal_cgram_address = 7)"
        );
        assert_eq!(
            p.cgram[5], 0x0000,
            "the in-render write must NOT land at the CPU-programmed index"
        );
        assert_eq!(
            p.io.cgram_address, 6,
            "the programmed address still advances (ares io.cgramAddress++ is unconditional)"
        );
    }

    #[test]
    fn cgram_write_outside_active_display_is_not_redirected() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display enabled
        p.v = 50;
        p.h = 300; // dot 300 >= 274 → HBlank, outside the redirect window
        p.internal_cgram_address = 7;
        p.write_reg(0x2121, 0x05);
        p.write_reg(0x2122, 0x34);
        p.write_reg(0x2122, 0x12);
        assert_eq!(
            p.cgram[5], 0x1234,
            "a write in HBlank must commit to the CPU-programmed index"
        );
        assert_eq!(
            p.cgram[7], 0x0000,
            "the drawn-palette index must be untouched"
        );
    }

    #[test]
    fn cgram_write_under_force_blank_is_not_redirected() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80); // FORCE BLANK on — CGRAM is freely accessible, no redirect
        p.v = 50;
        p.h = 100; // in the dot window, but force-blank gates the redirect off
        p.internal_cgram_address = 7;
        p.write_reg(0x2121, 0x05);
        p.write_reg(0x2122, 0x34);
        p.write_reg(0x2122, 0x12);
        assert_eq!(
            p.cgram[5], 0x1234,
            "under force-blank the write must commit to the programmed index"
        );
        assert_eq!(p.cgram[7], 0x0000);
    }

    // --- OAM in-render write redirect (C7.16, MesenCE GetOamAddress / the Uniracers quirk). During
    // sprite evaluation a $2104 write is aimed at the evaluator's OAM index, not the CPU's OAMADDR.
    // `render_addr = eval_index << 2` is always even and in the low table, so the low-table write
    // only latches; the value lands in the high table at the remapped address `(render&0x1f0)>>4`.

    #[test]
    fn oam_write_during_evaluation_redirects_to_high_table() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display ENABLED
        p.write_reg(0x2102, 0x00); // OAMADDR = 0
        // Line 50, dot 100, priority-rotation off ⇒ eval_seed 0. eval_index = 0 + (100+1)/2 = 50,
        // render_addr = 200 (0xC8); high-table remap = (0xC8 & 0x1F0) >> 4 = 12 ⇒ oam[0x200 + 12].
        p.v = 50;
        p.h = 100;
        p.pd_oam_eval_seed = 0;
        p.write_reg(0x2104, 0xab);
        assert_eq!(
            p.oam[0x200 + 12],
            0xab,
            "the in-render write must corrupt the high table at the remapped evaluation address"
        );
        assert_eq!(
            p.oam[200], 0x00,
            "the low-table entry at the (even) render address must only latch, never commit"
        );
        assert_eq!(
            p.io.oam_byte_latch, 0xab,
            "the even-byte buffer latches the value"
        );
        assert_eq!(
            p.io.oam_address, 1,
            "OAMADDR advances even when the write was redirected"
        );
    }

    #[test]
    fn oam_write_in_fetch_phase_is_not_redirected() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display enabled
        p.write_reg(0x2102, 0x00); // OAMADDR = 0
        p.v = 50;
        p.h = 300; // dot > 255 → fetch phase, not modelled ⇒ no redirect
        p.pd_oam_eval_seed = 0;
        p.write_reg(0x2104, 0x11); // even → latch
        p.write_reg(0x2104, 0x22); // odd → commit word to oam[0]/oam[1]
        assert_eq!(
            (p.oam[0], p.oam[1]),
            (0x11, 0x22),
            "a fetch-phase write uses the CPU OAMADDR (low table), not the redirect"
        );
        assert_eq!(p.oam[0x200 + 12], 0x00, "the high table must be untouched");
    }

    #[test]
    fn oam_write_under_force_blank_is_not_redirected() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x80); // FORCE BLANK — OAM freely accessible, no redirect
        p.write_reg(0x2102, 0x00); // OAMADDR = 0
        p.v = 50;
        p.h = 100; // in the eval dot range, but force-blank gates the redirect off
        p.pd_oam_eval_seed = 0;
        p.write_reg(0x2104, 0x11);
        p.write_reg(0x2104, 0x22);
        assert_eq!(
            (p.oam[0], p.oam[1]),
            (0x11, 0x22),
            "under force-blank the write commits to the CPU OAMADDR"
        );
        assert_eq!(
            p.oam[0x200 + 12],
            0x00,
            "the high table must be untouched under force-blank"
        );
    }

    #[test]
    fn oam_eval_seed_uses_priority_rotation_base_at_line_start() {
        use crate::bus::NullVideoBus;
        let mut p = Ppu::new();
        let mut bus = NullVideoBus;
        p.write_reg(0x2100, 0x0f); // display enabled
        p.write_reg(0x2103, 0x80); // OAM priority rotation ON
        p.write_reg(0x2102, 0x20); // OAMADDL → OAMADDR = 0x40
        p.v = 50; // a visible line
        p.pd_fetch_line(&mut bus);
        assert_eq!(
            p.pd_oam_eval_seed,
            ((0x40u16 >> 2) & 0x7f) as u8, // 0x10
            "with priority rotation the evaluation index seeds from (OAMADDR >> 2) at line start"
        );
        // At dot 100 the redirect then reads seed + (100>>1) = 0x10 + 50 = 66 → render_addr 0x108.
        p.h = 100;
        p.write_reg(0x2104, 0xcd);
        assert_eq!(
            p.oam[0x200 + 16],
            0xcd,
            "the priority-rotation seed shifts the redirect's high-table target ((0x108&0x1F0)>>4=16)"
        );

        // With rotation OFF the seed is 0 regardless of OAMADDR. The seed is (re-)captured only at the
        // line start (`h == 0`) — a mid-line `pd_fetch_line` is a post-load re-fetch and preserves the
        // restored seed — so rewind to the line start before re-fetching.
        p.write_reg(0x2103, 0x00);
        p.h = 0;
        p.pd_fetch_line(&mut bus);
        assert_eq!(
            p.pd_oam_eval_seed, 0,
            "without priority rotation the evaluation index seeds from 0"
        );
    }

    #[test]
    fn oam_read_during_evaluation_redirects_to_render_address() {
        // C1.08: a $2138 (OAMDATAREAD) during a rendering scanline reads the evaluator's OAM entry,
        // not the CPU's OAMADDR (MesenCE $2138 = GetOamAddress()).
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display enabled
        p.write_reg(0x2102, 0x00); // OAMADDR = 0
        p.oam[0] = 0x11; // what a NON-redirected read at OAMADDR=0 would return
        p.oam[200] = 0x77; // eval_index 50 << 2 = 200 — the render address at v=50, h=100
        p.pd_oam_eval_seed = 0;
        p.v = 50;
        p.h = 100;
        assert_eq!(
            p.read_reg(0x2138),
            0x77,
            "the in-render read must return the evaluator's OAM entry, not OAMADDR's"
        );
        assert_eq!(
            p.io.oam_address, 1,
            "OAMADDR still advances on the redirected read"
        );
    }

    #[test]
    fn oam_read_outside_render_uses_cpu_address() {
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display enabled
        p.write_reg(0x2102, 0x00); // OAMADDR = 0
        p.oam[0] = 0x11;
        p.oam[200] = 0x77;
        p.pd_oam_eval_seed = 0;
        p.v = 0; // vblank line → not rendering → no redirect
        p.h = 100;
        assert_eq!(
            p.read_reg(0x2138),
            0x11,
            "outside a rendering scanline the read uses the CPU OAMADDR"
        );
    }

    #[test]
    fn load_state_mid_line_re_fetch_preserves_oam_eval_seed() {
        // A mid-scanline save deserializes `pd_oam_eval_seed` because it has diverged from `OAMADDR`
        // via in-render redirected writes and cannot be re-derived. `load_state` invalidates the line
        // (`pd_fetched_line = u16::MAX`), so the next `pd_render_to_dot` re-fetches it — and that
        // re-fetch (at `h > 0`) must NOT clobber the restored seed by re-deriving it from the diverged
        // `OAMADDR`, or mid-scanline save-state determinism breaks (Antigravity review, #227).
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display on
        p.io.oam_priority_rotation = true; // a re-derive from OAMADDR would be non-zero here
        p.io.oam_address = 0xA8; // `(0xA8 >> 2) & 0x7f` = 0x2A — what a re-derive would produce
        // The state a mid-line `load_state` leaves behind.
        p.v = 50;
        p.h = 100;
        p.pd_fetched_line = u16::MAX;
        p.pd_oam_eval_seed = 0x55; // the restored line-start seed, distinct from the 0x2A re-derive
        let mut bus = NullVideoBus;
        p.pd_render_to_dot(&mut bus); // triggers `pd_fetch_line` (pd_fetched_line != v)
        assert_eq!(
            p.pd_oam_eval_seed, 0x55,
            "a post-load mid-line re-fetch (h > 0) must preserve the deserialized OAM eval seed"
        );

        // Sanity: a genuine line-start fetch (h == 0) DOES capture the seed from OAMADDR.
        p.pd_fetched_line = u16::MAX;
        p.h = 0;
        p.pd_render_to_dot(&mut bus);
        assert_eq!(
            p.pd_oam_eval_seed, 0x2A,
            "a line-start fetch (h == 0) captures the seed from OAMADDR"
        );
    }
}
