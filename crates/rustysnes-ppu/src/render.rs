//! Per-dot compositor (`docs/adr/0014`, T-CA-10): BG modes 0–7 (incl. Mode 7 affine), the
//! 128-sprite OAM pipeline, windows, and color math. Re-implemented clean-room from `docs/ppu.md` +
//! documented SNES hardware behavior, structurally informed by ares (ISC) and cross-validated
//! against a headless MesenCE. Nothing was ported verbatim.
//!
//! The **compose/draw** stage is per-dot: `pd_render_to_dot` drains one column at a time against
//! live registers, so a mid-line CGRAM/OAM access during active display, an `INIDISP`
//! brightness/blank change, and the sprite over-flag timing all take effect at dot resolution
//! (the sole renderer since the batch `compose_dac` whole-line path was removed; `compose_dac`
//! survives only as a `#[cfg(test)]` hi-res DAC driver). The **BG fetch** is also per-dot (T-CA-10
//! Phase 4c): `pd_fetch_bg_to` advances a fetch cursor `BG_FETCH_AHEAD` (22) columns ahead of the
//! draw, storing each column's raw per-layer pixels in `pd_bg` from live **BG-data** registers — the
//! tiled modes via `fetch_bg_column`, Mode 7's affine layer via `fetch_mode7_column` (dispatched on
//! live `bg_mode` per column, so a mid-line `BGMODE` flip switches paths correctly) — so a mid-line
//! scroll/tilemap/OPT/mosaic write, or a Mode-7 matrix/centre/scroll write, reaches only
//! not-yet-fetched columns. The **composite** — window, `TM`/`TS` enable, and cross-layer priority
//! over `pd_bg` + the once-per-line resolved `pd_sprite` — runs at the DRAW cursor
//! (`pd_compose_column`), so a mid-line window/`TM`/`TS` write reaches only columns past the draw
//! cursor (`BG_FETCH_AHEAD` behind the fetch), matching MesenCE's fetch-vs-composite split. Only the
//! tile pixel's own priority is baked at fetch time (a fetch-side property).
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

use crate::{Object, Ppu, SCREEN_WIDTH};

/// How many columns the BG fetch cursor runs ahead of the draw cursor (`docs/adr/0014` Phase 4c).
/// MesenCE fetches BG tile data one tile-column-plus ahead of the draw (`_fetchBgEnd = min(hPos,263)`
/// vs `_drawEndX = min(hPos-22,255)`), so a mid-line BG-data write takes effect ~22 columns to the
/// right of where a composite-register write would. On a static line the offset is irrelevant (every
/// column is fetched with identical registers); it only shapes where a raster write's boundary lands.
const BG_FETCH_AHEAD: usize = 22;

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

    /// Per-dot compositor line-start setup (`docs/adr/0014` T-CA-10 Phase 4/4c). Resets the DAC carry
    /// and the draw + fetch cursors, seeds the backdrop, and resolves this line's SPRITES into
    /// `pd_sprite` (latched — sprites do not change from a mid-line BG-data write). It does NOT fetch
    /// the BGs: the fetch cursor builds those incrementally, ~`BG_FETCH_AHEAD` columns ahead of the
    /// draw, reading live registers ([`Self::pd_fetch_bg_to`]) — for both tiled modes and Mode 7's
    /// affine layer. Called lazily by [`Self::pd_render_to_dot`] at each line's first active dot (or
    /// after a save-state load).
    fn pd_fetch_line(&mut self) {
        let row = (self.v - 1) as usize;
        if row == 0 {
            self.frame_hires = self.is_hires();
        }
        let pr = self.mode_priorities();
        // Resolve this line's sprites once (they do not change from a mid-line BG write). The fetch
        // cursor fills `pd_bg` per layer per column ahead of the draw; the draw composite applies the
        // backdrop base and the window/`TM`/`TS`/priority resolution live. Nothing to pre-fill here:
        // every column the draw reads was written by the fetch cursor first (it runs ahead). ALWAYS
        // resolve regardless of force-blank — display-disable is a compose-time decision, so a line
        // blanked at its start but un-blanked mid-line must have real pixels ready.
        let mut sprite = [Pixel::default(); SCREEN_WIDTH];
        self.resolve_sprite_line(&pr, &mut sprite);
        self.pd_sprite.copy_from_slice(&sprite);
        self.pd_fetch_x = 0;
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

    /// Advance the BG fetch cursor up to `fetch_target`, storing each newly-reached column's raw
    /// per-layer BG pixels into `pd_bg` from LIVE BG-data registers — the tiled BGs
    /// ([`Self::fetch_bg_column`]) or the Mode-7 affine layer(s) ([`Self::fetch_mode7_column`]). Only
    /// the tile/affine lookup and its baked-in priority happen here (both fetch-side); the window,
    /// `TM`/`TS` enable, and cross-layer priority resolution are deferred to the DRAW cursor
    /// ([`Self::pd_compose_column`]). `bg_mode` is read live per column, so a mid-line `BGMODE` flip
    /// across the Mode-7 boundary switches the fetch path for the remaining columns (both directions).
    /// On a static line every column reads identical registers, so the composited result is
    /// byte-identical to the old whole-line fetch; a mid-line BG-data write only reaches columns
    /// beyond the cursor (`docs/adr/0014` Phase 4c). Idempotent per line: each column is built once.
    fn pd_fetch_bg_to(&mut self, fetch_target: usize) {
        let pr = self.mode_priorities();
        while usize::from(self.pd_fetch_x) < fetch_target {
            let x = usize::from(self.pd_fetch_x);
            if self.io.bg_mode == 7 {
                // Mode 7: BG1 = affine layer 0, BG2 = the EXTBG high-priority layer 1 (transparent
                // when EXTBG is off, `fetch_mode7_column` gates it). Layers 2/3 do not exist — clear
                // them so a prior tiled line's pixels are not composited.
                let (bg1, bg2) = self.fetch_mode7_column(x, &pr);
                self.pd_bg[x][0] = bg1.unwrap_or_default();
                self.pd_bg[x][1] = bg2.unwrap_or_default();
                self.pd_bg[x][2] = Pixel::default();
                self.pd_bg[x][3] = Pixel::default();
            } else {
                // Tiled: store each active layer's fetched pixel (transparent for a colour-0 dot);
                // an inactive layer in this mode is cleared to transparent so a prior line's or a
                // prior mode's pixel is not composited.
                for bg in 0..4 {
                    self.pd_bg[x][bg] = if pr.active[bg] {
                        self.fetch_bg_column(bg, x, &pr)
                    } else {
                        Pixel::default()
                    };
                }
            }
            self.pd_fetch_x += 1;
        }
    }

    /// Resolve one column's `(main, sub)` pixels from the fetched per-layer BG pixels (`pd_bg`) and
    /// the latched sprite (`pd_sprite`), applying the DRAW-cursor-timed composite registers LIVE: the
    /// `TM`/`TS` layer enables, the windows, and cross-layer priority. Called at the draw cursor, so a
    /// mid-line window/`TM`/`TS` write takes effect only on later columns (MesenCE's split between
    /// BG-data at the fetch cursor and composite registers at the draw cursor). Layer active-ness and
    /// colour-0 are encoded as transparency in `pd_bg`, so no mode table is needed here.
    fn pd_compose_column(&self, x: usize) -> (Pixel, Pixel) {
        // The backdrop base: layer 5, priority 0, transparent (so a bare backdrop column stays
        // colour-0 for `compose_pixel` to render from CGRAM[0]).
        let backdrop = Pixel {
            palette: 0,
            priority: 0,
            layer: 5,
            palette_group: 0,
            opaque: false,
            ..Pixel::default()
        };
        let mut a = backdrop;
        let mut b = backdrop;
        for bg in 0..4 {
            let px = self.pd_bg[x][bg];
            if !px.opaque {
                continue;
            }
            let prio = px.priority;
            if self.io.main_enable[bg] && !self.windowed_out(bg, x, true) && prio > a.priority {
                a = px;
            }
            if self.io.sub_enable[bg] && !self.windowed_out(bg, x, false) && prio > b.priority {
                b = px;
            }
        }
        // Sprites compose last (highest layer), with `>=` so an obj ties over a BG of equal priority.
        let sp = self.pd_sprite[x];
        if sp.opaque {
            let prio = sp.priority;
            if self.io.main_enable[4] && !self.windowed_out(4, x, true) && prio >= a.priority {
                a = sp;
            }
            if self.io.sub_enable[4] && !self.windowed_out(4, x, false) && prio >= b.priority {
                b = sp;
            }
        }
        (a, b)
    }

    /// Per-dot compositor driver, called every dot from [`crate::Ppu::tick_dot`]. First advances the
    /// BG fetch cursor to `BG_FETCH_AHEAD` columns ahead of the draw (storing raw per-layer pixels in
    /// `pd_bg` from live BG-data registers), then composites the visible line's columns to the
    /// framebuffer up to the column the DAC has reached — resolving window/`TM`/`TS`/priority
    /// ([`Self::pd_compose_column`]) and color-math/brightness/force-blank/CGRAM-redirect with **live**
    /// registers per column, so a mid-line composite-register write only affects columns drawn after
    /// it — tracking [`crate::Ppu::internal_cgram_address`] = the last drawn palette. All columns
    /// finish by `RENDER_DOT` (pre-line-HDMA), so a static line composites identically to a whole-line
    /// pass.
    pub(crate) fn pd_render_to_dot(&mut self) {
        if self.v < 1 || self.v > self.visible_height() {
            return;
        }
        if self.pd_fetched_line != self.v {
            self.pd_fetch_line();
        }
        let target = if self.h < crate::ACTIVE_DOT_START {
            0
        } else if self.h >= crate::RENDER_DOT {
            SCREEN_WIDTH
        } else {
            usize::from(self.h - crate::ACTIVE_DOT_START + 1).min(SCREEN_WIDTH)
        };
        // Keep the BG fetch cursor ahead of the draw cursor (a no-op once the line is fully fetched,
        // e.g. Mode 7 where `pd_fetch_x` is seeded at `SCREEN_WIDTH`).
        self.pd_fetch_bg_to((target + BG_FETCH_AHEAD).min(SCREEN_WIDTH));
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
                // Resolve the column now, at the DRAW cursor: window/`TM`/`TS`/priority live, over
                // the per-layer pixels the fetch cursor built ~`BG_FETCH_AHEAD` columns ago.
                let (ap, bp) = self.pd_compose_column(x);
                self.pd_carry = self.compose_pixel(x, ap, bp, ctx, self.pd_carry);
                self.internal_cgram_address = ap.palette;
            }
            self.pd_draw_x += 1;
        }
    }

    /// Fetch one non-Mode-7 BG's pixel at screen column `x`, reading every BG-data register LIVE
    /// (`docs/adr/0014` Phase 4c): scroll (`BGnHOFS`/`VOFS`), tilemap (`BGnSC`), char base
    /// (`BGnNBA`), tile size, mosaic, and the BG3 offset-per-tile lookup. Returns a transparent
    /// [`Pixel`] (`opaque == false`) for a colour-0 dot or a disabled BG. Extracted verbatim from
    /// `render_bg`'s per-column loop so an incremental fetch cursor can call it one column at a time
    /// mid-line; while it is still invoked whole-line at line start (`render_bg`), a static line
    /// reads identical registers every column, so the result is byte-identical to the old fused loop.
    // `bg3_hofs`/`bg3_vofs` (offset-per-tile) intentionally mirror the `hofs`/`vofs` naming.
    #[allow(clippy::similar_names)]
    fn fetch_bg_column(&self, bg: usize, x: usize, pr: &ModePriorities) -> Pixel {
        let bpp = self.bg_bpp(bg);
        if bpp == 0 {
            return Pixel::default();
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
            debug_assert!(line_y >= 1, "fetch_bg_column called for scanline 0");
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

        let x = x as u32;
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
            return Pixel::default();
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
        pixel
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

    /// Fetch the Mode-7 affine pixel(s) at screen column `x`, reading every M7 register LIVE
    /// (`docs/adr/0014` Phase 4c): the `M7A`-`M7D` matrix, the `M7X`/`M7Y` center, the `M7HOFS`/
    /// `M7VOFS` scroll, the flips, mosaic, and `M7SEL` repeat mode. Returns `(BG1, BG2)` where BG1 is
    /// the affine layer (full 8-bit palette) and BG2 is the EXTBG high-priority layer (bit 7 promoted
    /// to a priority selector, low 7 bits the colour) — `None` for a colour-0 (transparent) pixel or a
    /// disabled layer. The compositor ([`Self::pd_fetch_bg_to`]) applies window + enable + priority,
    /// so a mid-line M7 write reaches only columns the fetch cursor has not yet built. Extracted from
    /// the old whole-line `render_mode7`; the per-line affine origin is recomputed per column so the
    /// read is live (the extra arithmetic is a Mode-7-only cost). Byte-identical on a static line.
    fn fetch_mode7_column(&self, x: usize, pr: &ModePriorities) -> (Option<Pixel>, Option<Pixel>) {
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

        // Mosaic, quantised in SCREEN space exactly as `fetch_bg_column` does it: the block grid is
        // anchored to the top-left of the picture, not to whatever the transform maps there.
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

        let mut sx = x as u32;
        if mosaic {
            sx = (sx / msize) * msize;
        }
        if self.io.m7_hflip {
            sx = 255 - (sx & 0xff);
        }

        let pixel_x = (origin_x + a * sx as i32) >> 8;
        let pixel_y = (origin_y + c * sx as i32) >> 8;

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
        let palette = if self.io.m7_repeat == 2 && out_of_bounds {
            0u8
        } else {
            let addr = ((tile << 6) | (palette_addr as u16)) & 0x7fff;
            (self.vram[addr as usize] >> 8) as u8
        };

        // BG1 always renders, with the FULL 8-bit palette. EXTBG adds a second layer from the same
        // pixels — it does not replace the first (treating it as either/or made BG1 vanish under
        // EXTBG, which the framebuffer oracle caught). The `hd-pack` tag reuses `tile`.
        let bg1 = (palette != 0).then(|| {
            #[allow(unused_mut)]
            let mut pixel = Pixel {
                palette,
                priority: pr.bg[0][0],
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
            pixel
        });

        // BG2, present only under EXTBG: bit 7 promoted from palette data to a priority selector,
        // the remaining seven bits the colour.
        let bg2 = if self.io.extbg {
            let prio_hi = (palette >> 7) & 1;
            let p2 = palette & 0x7f;
            (p2 != 0).then(|| {
                #[allow(unused_mut)]
                let mut pixel = Pixel {
                    palette: p2,
                    priority: pr.bg[1][usize::from(prio_hi)],
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
                pixel
            })
        } else {
            None
        };

        #[cfg(not(feature = "hd-pack"))]
        let _ = tile;

        (bg1, bg2)
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
        // Capture the priority-rotation seed at the TRUE line start (`h == 0`), before any mid-line
        // `$2104` write can advance `oam_address`. Serialized, so a post-load recompute uses it rather
        // than the diverged live address. A mid-line re-entry (`h > 0`, only after `load_state`) skips
        // the capture and reuses the restored seed.
        if self.h == 0 {
            self.pd_over_eval_seed = if self.io.oam_priority_rotation {
                ((self.io.oam_address >> 2) & 0x7f) as u8
            } else {
                0
            };
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
        // Key off the serialized line-start seed, NOT the live `oam_address` — the latter diverges
        // from the line-start value after redirected active-display `$2104` writes on a
        // priority-rotated line, and this function is re-run on `load_state`, so using the live
        // address would shift `$213E` timing across a mid-line save/load.
        let first = usize::from(self.pd_over_eval_seed);
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

    /// Resolve this line's sprites into a per-column buffer (`docs/adr/0014` Phase 4c). Inter-sprite
    /// priority is decided here — [`Self::paint_objects`] paints high-index → low with `>=` into the
    /// transparent buffer, so the lowest-index highest-priority sprite wins — but the window and
    /// main/sub gating are deferred to the composite. This is equivalent to the old fused
    /// `render_objects` drain because the inter-sprite winner does not depend on the BG underneath:
    /// the composite (here or in [`Self::pd_fetch_bg_to`]) applies the same window + enable + the
    /// `sprite.priority >= layer.priority` test against whatever the BGs left in the column.
    fn resolve_sprite_line(&self, pr: &ModePriorities, sprite: &mut [Pixel]) {
        let (in_range, count, budget_ok) = self.eval_objects_range();
        self.paint_objects(pr, sprite, &in_range, count, &budget_ok);
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

    /// Paint the evaluated, budget-surviving sprites into a single per-column sprite buffer (the
    /// `resolve_sprite_line` second phase). Consumes the `(in_range, count, budget_ok)` produced by
    /// [`Ppu::eval_objects_range`]. Only inter-sprite priority is resolved here — highest index →
    /// lowest with `>=`, so the lowest-index highest-priority sprite ends up in `sprite[x]`; window
    /// and main/sub gating are applied later by whichever composite consumes the buffer. Kept a
    /// distinct phase so the per-dot compositor can fetch/paint sprite columns independently of range
    /// evaluation (`docs/adr/0014`, phase 4b).
    fn paint_objects(
        &self,
        pr: &ModePriorities,
        sprite: &mut [Pixel],
        in_range: &[u8; 32],
        count: usize,
        budget_ok: &[bool; 32],
    ) {
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
                    // sprite at the same priority win the tie (it is painted later). The buffer
                    // starts transparent (priority 0) and obj priorities are always >= 1, so any
                    // opaque sprite wins the first write. Window/enable are applied by the composite.
                    if prio >= sprite[xi].priority {
                        sprite[xi] = pixel;
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
    use super::BG_FETCH_AHEAD;
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
        let mut p = Ppu::new();
        p.write_reg(0x2100, 0x0f); // display enabled
        p.write_reg(0x2103, 0x80); // OAM priority rotation ON
        p.write_reg(0x2102, 0x20); // OAMADDL → OAMADDR = 0x40
        p.v = 50; // a visible line
        p.pd_fetch_line();
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
        p.pd_fetch_line();
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
        p.pd_render_to_dot(); // triggers `pd_fetch_line` (pd_fetched_line != v)
        assert_eq!(
            p.pd_oam_eval_seed, 0x55,
            "a post-load mid-line re-fetch (h > 0) must preserve the deserialized OAM eval seed"
        );

        // Sanity: a genuine line-start fetch (h == 0) DOES capture the seed from OAMADDR.
        p.pd_fetched_line = u16::MAX;
        p.h = 0;
        p.pd_render_to_dot();
        assert_eq!(
            p.pd_oam_eval_seed, 0x2A,
            "a line-start fetch (h == 0) captures the seed from OAMADDR"
        );
    }

    /// Phase 4c: a mid-line BG horizontal-scroll write reaches only the columns the fetch cursor has
    /// NOT yet fetched (MesenCE `_fetchBgStart..End`), running `BG_FETCH_AHEAD` columns ahead of the
    /// draw. Under the old whole-line-at-line-start fetch the write would have affected nothing (the
    /// line was already fetched); this pins the incremental behaviour AND the fetch-ahead offset.
    #[test]
    fn mid_line_bg_scroll_shifts_only_columns_past_the_fetch_cursor() {
        // A 4bpp BG1 (Mode 1) whose tile 0 is a horizontal colour ramp — pixel i has colour i+1 —
        // so the colour at screen column x is CGRAM[((x + hofs) & 7) + 1]. A sub-tile scroll change
        // (0 -> 3) therefore shifts EVERY column's colour, so each column's value reveals which
        // scroll it was fetched with.
        fn setup() -> Ppu {
            let mut p = Ppu::new();
            p.write_reg(0x2100, 0x80); // force-blank for VRAM/CGRAM setup
            for y in 0..8u16 {
                let (mut w0, mut w1) = (0u16, 0u16); // planes 0/1 and 2/3 for this row
                for i in 0..8u16 {
                    let c = i + 1; // colour 1..=8 across the 8 pixels
                    let b = 7 - i; // read_planar uses bit = 7 - fine_x
                    if c & 1 != 0 {
                        w0 |= 1 << b;
                    }
                    if c & 2 != 0 {
                        w0 |= 1 << (8 + b);
                    }
                    if c & 4 != 0 {
                        w1 |= 1 << b;
                    }
                    if c & 8 != 0 {
                        w1 |= 1 << (8 + b);
                    }
                }
                vram_set(&mut p, y, w0); // tile 0 (char base 0): plane 0/1 row y
                vram_set(&mut p, 8 + y, w1); // plane 2/3 row y (a plane-pair is 8 words on)
            }
            for c in 1..=8u16 {
                cgram_set(&mut p, c as u8, (c * 0x0841) & 0x7fff); // 8 distinct non-zero colours
            }
            // Tilemap base 0x0400: default-zero entries = character 0, palette group 0, no flip.
            p.io.bg_mode = 1;
            p.io.bg_tiledata_addr[0] = 0;
            p.io.bg_screen_addr[0] = 0x0400;
            p.io.bg_screen_size[0] = 0;
            p.io.tile_size[0] = false;
            p.io.bg_vofs[0] = 0;
            p.io.mosaic_enable[0] = false;
            p.io.main_enable[0] = true;
            p.io.display_disable = false;
            p.io.display_brightness = 15;
            p
        }

        // Render visible line 1 by stepping the per-dot compositor, optionally writing a new hofs at
        // dot `d` (mid-line). Returns the 256 framebuffer colours of row 0.
        let render = |start: u16, inject: Option<(u16, u16)>| -> [u16; SCREEN_WIDTH] {
            let mut p = setup();
            p.io.bg_hofs[0] = start;
            p.v = 1;
            p.pd_fetched_line = u16::MAX;
            for h in 0..=crate::RENDER_DOT {
                if let Some((d, new_hofs)) = inject
                    && h == d
                {
                    p.io.bg_hofs[0] = new_hofs;
                }
                p.h = h;
                p.pd_render_to_dot();
            }
            let fb = p.framebuffer();
            core::array::from_fn(|x| fb[x])
        };

        let s0 = render(0, None); // whole line at hofs 0
        let s3 = render(3, None); // whole line at hofs 3

        // The ramp + sub-tile shift makes the two static lines differ at EVERY column, so each
        // `split` column is unambiguously one scroll or the other.
        assert!(
            (0..SCREEN_WIDTH).all(|x| s0[x] != s3[x]),
            "setup: a hofs 0->3 shift must change every column's colour"
        );

        // Inject the scroll change at dot `d`, then find the split boundary: every column must be
        // exactly the old- or new-scroll colour (no garbage), monotone (old left / new right), and
        // the write must affect SOME columns (not the old whole-line no-op).
        let boundary_at = |d: u16| -> usize {
            let split = render(0, Some((d, 3)));
            let mut boundary = None;
            for x in 0..SCREEN_WIDTH {
                let is_old = split[x] == s0[x];
                let is_new = split[x] == s3[x];
                assert!(
                    is_old || is_new,
                    "d={d}: column {x} is neither the old nor the new scroll colour (garbage)"
                );
                match boundary {
                    None if is_new => boundary = Some(x),
                    None => {} // still in the old-scroll run
                    Some(_) => assert!(
                        is_new,
                        "d={d}: column {x} reverted to the old scroll after the boundary (not monotone)"
                    ),
                }
            }
            let boundary = boundary
                .unwrap_or_else(|| panic!("d={d}: the mid-line write must affect SOME columns"));
            assert!(
                boundary > 0,
                "d={d}: column 0 (fetched first, before the write) must keep the old scroll"
            );
            boundary
        };

        // A write at dot `d` reaches the column the fetch cursor is about to build, which runs
        // BG_FETCH_AHEAD ahead of the draw, so the boundary lands at ~column `d` (the draw is then at
        // ~d - BG_FETCH_AHEAD). Two dots pin the *relation* boundary == d, not one coincidental
        // point: a constant cursor offset that happened to give boundary == 128 would miss at 60. A
        // draw-cursor fetch (BG_FETCH_AHEAD effectively 0) would land ~22 columns low and fail both.
        for d in [60u16, 128] {
            let boundary = boundary_at(d);
            assert!(
                boundary.abs_diff(usize::from(d)) <= 2,
                "fetch-ahead boundary {boundary} should be ~dot {d} (write reaches the fetch cursor, \
                 {BG_FETCH_AHEAD} columns ahead of the draw)"
            );
        }
    }

    /// Phase 4c, Mode 7: a mid-line `M7HOFS` write reaches only columns the fetch cursor has not yet
    /// built — the affine layer runs through the same fetch cursor as the tiled BGs. With an identity
    /// matrix and zero centre/vscroll, screen column x samples Mode-7 pixel `(x + m7_hofs)`, so the
    /// colour is a horizontal ramp that shifts with the scroll (the same probe as the tiled test).
    #[test]
    fn mid_line_mode7_scroll_shifts_only_columns_past_the_fetch_cursor() {
        fn setup() -> Ppu {
            let mut p = Ppu::new();
            p.write_reg(0x2100, 0x80); // force-blank for VRAM/CGRAM setup
            // Mode-7 VRAM is interleaved: low byte = tilemap (tile index), high byte = char data.
            // Keep every tile 0 (low byte 0, the default) and fill tile 0's 8x8 char block (word
            // addrs 0..64) so pixel `(px&7, py&7)` has colour `(px&7)+1` — a horizontal ramp.
            for pa in 0..64u16 {
                let colour = (pa & 7) + 1; // low 3 bits of palette_addr == pixel_x & 7
                vram_set(&mut p, pa, colour << 8); // high byte = char colour, low byte = tile 0
            }
            for c in 1..=8u16 {
                cgram_set(&mut p, c as u8, (c * 0x0841) & 0x7fff); // 8 distinct non-zero colours
            }
            p.io.bg_mode = 7;
            p.io.main_enable[0] = true;
            // Identity matrix (8.8 fixed point) with zero centre and vscroll:
            //   pixel_x = x + m7_hofs, pixel_y = v.
            p.io.m7a = 0x0100;
            p.io.m7b = 0;
            p.io.m7c = 0;
            p.io.m7d = 0x0100;
            p.io.m7x = 0;
            p.io.m7y = 0;
            p.io.m7_hofs = 0;
            p.io.m7_vofs = 0;
            p.io.m7_hflip = false;
            p.io.m7_vflip = false;
            p.io.m7_repeat = 0;
            p.io.extbg = false;
            p.io.mosaic_enable[0] = false;
            p.io.display_disable = false;
            p.io.display_brightness = 15;
            p
        }

        let render = |start: u16, inject: Option<(u16, u16)>| -> [u16; SCREEN_WIDTH] {
            let mut p = setup();
            p.io.m7_hofs = start;
            p.v = 1;
            p.pd_fetched_line = u16::MAX;
            for h in 0..=crate::RENDER_DOT {
                if let Some((d, new_hofs)) = inject
                    && h == d
                {
                    p.io.m7_hofs = new_hofs;
                }
                p.h = h;
                p.pd_render_to_dot();
            }
            let fb = p.framebuffer();
            core::array::from_fn(|x| fb[x])
        };

        let s0 = render(0, None);
        let s3 = render(3, None);
        assert!(
            (0..SCREEN_WIDTH).all(|x| s0[x] != s3[x]),
            "setup: a Mode-7 hofs 0->3 shift must change every column's colour"
        );

        let boundary_at = |d: u16| -> usize {
            let split = render(0, Some((d, 3)));
            let mut boundary = None;
            for x in 0..SCREEN_WIDTH {
                let is_old = split[x] == s0[x];
                let is_new = split[x] == s3[x];
                assert!(
                    is_old || is_new,
                    "d={d}: Mode-7 column {x} is neither scroll's colour (garbage)"
                );
                match boundary {
                    None if is_new => boundary = Some(x),
                    None => {}
                    Some(_) => assert!(is_new, "d={d}: Mode-7 column {x} is not a monotone split"),
                }
            }
            boundary
                .unwrap_or_else(|| panic!("d={d}: the mid-line M7 write must affect SOME columns"))
        };

        for d in [60u16, 128] {
            let boundary = boundary_at(d);
            assert!(
                boundary.abs_diff(usize::from(d)) <= 2,
                "Mode-7 fetch-ahead boundary {boundary} should be ~dot {d} ({BG_FETCH_AHEAD} ahead)"
            );
        }
    }

    /// Phase 4c refinement: a mid-line `TM` (main-screen layer-enable) write takes effect at the
    /// **draw** cursor, NOT the fetch cursor — the split lands ~`ACTIVE_DOT_START` columns to the LEFT
    /// of where a BG-DATA write at the same dot would (`BG_FETCH_AHEAD` columns behind the fetch), so
    /// this both proves the composite moved to the draw cursor and distinguishes it from the fetch
    /// timing. Two solid BGs: BG1 (higher priority, colour A) over BG2 (colour B); disabling BG1 on
    /// the main screen reveals BG2, so the column colour flips A→B exactly where the enable is read.
    #[test]
    fn mid_line_tm_write_takes_effect_at_the_draw_cursor() {
        fn setup() -> Ppu {
            let mut p = Ppu::new();
            p.write_reg(0x2100, 0x80); // force-blank for VRAM/CGRAM setup
            // Char 0 (4bpp) = solid colour 1: plane 0 all ones, planes 1-3 zero.
            for y in 0..8u16 {
                vram_set(&mut p, y, 0x00ff); // word0: low byte = plane 0 (all set), high byte = 0
                vram_set(&mut p, 8 + y, 0x0000); // word1: planes 2/3 = 0
            }
            // BG1 tilemap at 0x0800 stays default 0 (tile 0, palette group 0). BG2 tilemap at 0x0C00:
            // tile 0, palette group 1 (entry bits 10-12 = 001 = 0x0400), for the visible first rows.
            for e in 0..0x40u16 {
                vram_set(&mut p, 0x0c00 + e, 0x0400);
            }
            cgram_set(&mut p, 1, 0x001f); // BG1 colour A (group 0, colour 1 -> CGRAM 1)
            cgram_set(&mut p, 17, 0x7c00); // BG2 colour B (group 1, colour 1 -> CGRAM 17)
            p.io.bg_mode = 1; // BG1 (prio 6) over BG2 (prio 5)
            p.io.bg_tiledata_addr[0] = 0;
            p.io.bg_tiledata_addr[1] = 0;
            p.io.bg_screen_addr[0] = 0x0800;
            p.io.bg_screen_addr[1] = 0x0c00;
            p.io.bg_screen_size[0] = 0;
            p.io.bg_screen_size[1] = 0;
            p.io.tile_size[0] = false;
            p.io.tile_size[1] = false;
            p.io.main_enable[1] = true; // BG2 always on the main screen
            p.io.mosaic_enable[0] = false;
            p.io.mosaic_enable[1] = false;
            p.io.display_disable = false;
            p.io.display_brightness = 15;
            p
        }

        // Render line 1, optionally disabling BG1's main-screen enable at dot `d`.
        let render = |bg1_start: bool, disable_at: Option<u16>| -> [u16; SCREEN_WIDTH] {
            let mut p = setup();
            p.io.main_enable[0] = bg1_start;
            p.v = 1;
            p.pd_fetched_line = u16::MAX;
            for h in 0..=crate::RENDER_DOT {
                if let Some(d) = disable_at
                    && h == d
                {
                    p.io.main_enable[0] = false;
                }
                p.h = h;
                p.pd_render_to_dot();
            }
            let fb = p.framebuffer();
            core::array::from_fn(|x| fb[x])
        };

        let with_bg1 = render(true, None); // BG1 wins everywhere -> colour A
        let without_bg1 = render(false, None); // BG1 off -> BG2 shows -> colour B
        assert!(
            (0..SCREEN_WIDTH).all(|x| with_bg1[x] != without_bg1[x]),
            "setup: disabling BG1 on the main screen must change every column's colour"
        );

        for d in [60u16, 128] {
            let split = render(true, Some(d));
            let mut boundary = None;
            for x in 0..SCREEN_WIDTH {
                let is_a = split[x] == with_bg1[x];
                let is_b = split[x] == without_bg1[x];
                assert!(
                    is_a || is_b,
                    "d={d}: column {x} is neither colour (garbage)"
                );
                match boundary {
                    None if is_b => boundary = Some(x),
                    None => {}
                    Some(_) => assert!(is_b, "d={d}: column {x} is not a monotone split"),
                }
            }
            let boundary = boundary
                .unwrap_or_else(|| panic!("d={d}: the mid-line TM write must affect columns"));
            // The draw cursor at dot d is column `d - ACTIVE_DOT_START`; that is where the enable is
            // read, and it is BG_FETCH_AHEAD columns to the LEFT of a BG-data write's boundary (`d`).
            let expected = usize::from(d - crate::ACTIVE_DOT_START);
            assert!(
                boundary.abs_diff(expected) <= 2,
                "TM write at dot {d} should take effect at the DRAW cursor (~column {expected}), not \
                 the fetch cursor (~column {d}); got boundary {boundary}"
            );
        }
    }
}
