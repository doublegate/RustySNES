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
//! priority tables where several modes deliberately share an identical layout; and the small
//! `Copy` structs (`Pixel`, `WindowLayer`) are passed by ref by helpers for call-site clarity.
#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    clippy::trivially_copy_pass_by_ref
)]

use crate::{Object, Ppu, SCREEN_WIDTH, VideoBus};

/// A composited layer pixel: a 8-bit CGRAM palette index + a priority + the source-layer id.
#[derive(Clone, Copy, Default)]
struct Pixel {
    palette: u8,
    priority: u8,
    /// Layer source: 0..=3 bg1..4, 4 obj, 5 backdrop. Drives color-math enable + direct color.
    layer: u8,
    /// Mode-0 direct-color paletteGroup carry (only meaningful for direct-color BG1).
    palette_group: u8,
    /// True if this pixel actually came from a non-transparent source.
    opaque: bool,
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

    /// Composite the current scanline (`self.v`, 1..=visible) into the framebuffer.
    pub(crate) fn render_scanline(&mut self, bus: &mut impl VideoBus) {
        let row = (self.v - 1) as usize;
        if row >= crate::SCREEN_HEIGHT {
            return;
        }
        let base = row * SCREEN_WIDTH;

        // Force-blank: the line is black.
        if self.io.display_disable {
            for x in 0..SCREEN_WIDTH {
                self.framebuffer[base + x] = 0;
            }
            return;
        }

        let pr = self.mode_priorities();

        // Build the main (above) and sub (below) layer pixels for each column.
        let mut above = [Pixel::default(); SCREEN_WIDTH];
        let mut below = [Pixel::default(); SCREEN_WIDTH];
        // Backdrop (CGRAM 0) at priority 0.
        for x in 0..SCREEN_WIDTH {
            above[x] = Pixel {
                palette: 0,
                priority: 0,
                layer: 5,
                palette_group: 0,
                opaque: false,
            };
            below[x] = above[x];
        }

        // --- Backgrounds ---
        if self.io.bg_mode == 7 {
            self.render_mode7(bus, &pr, &mut above, &mut below);
        } else {
            for bg in 0..4 {
                if pr.active[bg] {
                    self.render_bg(bg, &pr, &mut above, &mut below);
                }
            }
        }

        // --- Sprites ---
        self.render_objects(&pr, &mut above, &mut below);

        // --- Windows (clear a layer's priority where windowed-out) ---
        // Implemented inline during compositing below via per-pixel window tests.

        // --- Color math + DAC ---
        self.compose_dac(row, &above, &below);
    }

    /// Render one non-Mode-7 background into the above/below pixel buffers.
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

        // Mosaic vertical handling (simplified: applies to BG y).
        let mut line_y = u32::from(self.v - 1);
        if self.io.mosaic_enable[bg] && self.io.mosaic_size > 1 {
            let m = u32::from(self.io.mosaic_size);
            line_y = (line_y / m) * m;
        }

        for x in 0..SCREEN_WIDTH as u32 {
            let px = if self.io.mosaic_enable[bg] && self.io.mosaic_size > 1 {
                let m = u32::from(self.io.mosaic_size);
                (x / m) * m
            } else {
                x
            };
            let world_x = px.wrapping_add(hofs);
            let world_y = line_y.wrapping_add(vofs);

            let (palette_idx, group, priority_hi) = self.fetch_bg_pixel(
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

            // BG palette base for Mode 0 (each BG gets a 32-color region).
            let pal_base = if self.io.bg_mode == 0 {
                (bg as u8) << 5
            } else {
                0
            };
            let final_pal = pal_base.wrapping_add(palette_idx);
            let prio = pr.bg[bg][usize::from(priority_hi)];

            let xi = x as usize;
            let pixel = Pixel {
                palette: final_pal,
                priority: prio,
                layer: bg as u8,
                palette_group: group,
                opaque: true,
            };
            if main && !self.windowed_out(bg, xi, true) && prio > above[xi].priority {
                above[xi] = pixel;
            }
            if sub && !self.windowed_out(bg, xi, false) && prio > below[xi].priority {
                below[xi] = pixel;
            }
        }
    }

    /// Fetch one BG pixel: returns (palette index within the BG palette, palette group, hi-prio).
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
    ) -> (u8, u8, u8) {
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
        (color, palette_group, priority_hi)
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

        let mut y = u32::from(self.v - 1);
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

            // BG1 (or BG2 with EXTBG high-bit priority).
            if extbg {
                let prio_hi = (palette >> 7) & 1;
                palette &= 0x7f;
                if palette != 0 {
                    let prio = pr.bg[1][usize::from(prio_hi)];
                    let pixel = Pixel {
                        palette,
                        priority: prio,
                        layer: 1,
                        palette_group: 0,
                        opaque: true,
                    };
                    if main2 && !self.windowed_out(1, xi, true) && prio > above[xi].priority {
                        above[xi] = pixel;
                    }
                    if sub2 && !self.windowed_out(1, xi, false) && prio > below[xi].priority {
                        below[xi] = pixel;
                    }
                }
            } else if palette != 0 {
                let prio = pr.bg[0][0];
                let pixel = Pixel {
                    palette,
                    priority: prio,
                    layer: 0,
                    palette_group: 0,
                    opaque: true,
                };
                if main1 && !self.windowed_out(0, xi, true) && prio > above[xi].priority {
                    above[xi] = pixel;
                }
                if sub1 && !self.windowed_out(0, xi, false) && prio > below[xi].priority {
                    below[xi] = pixel;
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

    /// Render sprites for the current scanline: range/time evaluation + pixel fetch.
    fn render_objects(&mut self, pr: &ModePriorities, above: &mut [Pixel], below: &mut [Pixel]) {
        let main = self.io.main_enable[4];
        let sub = self.io.sub_enable[4];

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
        let mut tile_count = 0usize;

        for i in 0..128 {
            let idx = (first + i) & 0x7f;
            let obj = self.object(idx);
            let (w, h) = self.object_size(obj.size);
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
            tile_count += (w / 8) as usize;
        }

        if range_count > 32 {
            self.io.range_over = true;
        }
        if tile_count > 34 {
            self.io.time_over = true;
        }

        // Paint sprites: reverse-order so lower index ends up on top (last writer wins among
        // equal priority because we paint high index first). We honor the 34-tile limit by
        // dropping the lowest-index sprites first (reverse fetch).
        let count = range_count.min(32);
        // Determine which sprites survive the 34-tile budget (drop lowest index first — the HW
        // fetches in reverse, so the lowest-index tiles are the first to be starved).
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

        // Paint from highest index to lowest (so lowest index wins ties).
        for k in (0..count).rev() {
            if !budget_ok[k] {
                continue;
            }
            let idx = in_range[k] as usize;
            let obj = self.object(idx);
            let (w, h) = self.object_size(obj.size);

            let mut row = (scan_y.wrapping_sub(u32::from(obj.y))) & 0xff;
            if obj.vflip {
                row = h - 1 - row;
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
                    let pixel = Pixel {
                        palette: pal,
                        priority: prio,
                        layer: 4,
                        palette_group: 0,
                        opaque: true,
                    };
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

    /// Color-math + DAC: pick the main/sub colors, apply add/sub/half + windows, write the row.
    fn compose_dac(&mut self, row: usize, above: &[Pixel], below: &[Pixel]) {
        let base = row * SCREEN_WIDTH;
        let brightness = u32::from(self.io.display_brightness);

        for x in 0..SCREEN_WIDTH {
            let ap = above[x];
            let bp = below[x];

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

            let mut out = if main_force_black { 0 } else { main_color };

            if math_layer && math_allowed {
                let addend = if self.io.add_subscreen {
                    self.layer_color(&bp)
                } else {
                    self.io.fixed_color
                };
                let halve = self.io.color_halve && (!self.io.add_subscreen || bp.opaque);
                out = if self.io.color_subtract {
                    color_sub(out, addend, halve)
                } else {
                    color_add(out, addend, halve)
                };
            }

            self.framebuffer[base + x] = apply_brightness(out, brightness);
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

        // Tile 0 char data at word 0x1000: 2bpp. Make the top-left pixel color 1.
        // Row 0 plane0 = 0x80 (bit7 set => leftmost pixel), plane1 = 0.
        vram_set(&mut p, 0x1000, 0x0080);

        // Enable display + BG1 main.
        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x01); // TM: BG1
        run_frame(&mut p);

        let fb = p.framebuffer();
        // Top-left pixel should be red (color 1).
        assert_eq!(fb[0], 0x001f);
        // Next pixel (color 0) is backdrop = 0.
        assert_eq!(fb[1], 0x0000);
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
        // Mode 7 char data: tile 1, pixel (0,0). char addr = tile<<6 | (y<<3|x).
        // For tile 1, pixel 0: addr = 1<<6 = 0x40. Palette in high byte.
        vram_set(&mut p, 0x0040, 0x0100); // high byte = palette index 1
        cgram_set(&mut p, 1, 0x7c00); // blue

        p.write_reg(0x2100, 0x0f);
        p.write_reg(0x212c, 0x01); // TM: BG1
        run_frame(&mut p);

        let fb = p.framebuffer();
        // Pixel (0,0) maps to map tile (0,0) tile #1 pixel (0,0) = blue.
        assert_eq!(fb[0], 0x7c00);
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
}
