//! `rustysnes-ppu` — PPU1 (5C77) + PPU2 (5C78) (video).
//!
//! Dual-chip PPU: BG modes 0-7 (incl. Mode 7 affine), OAM sprites, the dot-clock timeline.
//! The PPU owns its own VRAM (64 KiB), CGRAM (palette), and OAM. Anything that has to reach
//! the cartridge — Mode 7 / extended-bank reads on coprocessor boards, board IRQ/scanline
//! notifies — goes through the narrow [`VideoBus`] trait, whose only concrete impl in
//! production is the cart-mediated router in `rustysnes-core`. This is the RustyNES `PpuBus`
//! shape, ported: the video chip depends ONLY on `rustysnes-cart` (its memory bus).
//!
//! Part of the one-directional chip-crate graph (see `docs/architecture.md`): this crate
//! does NOT depend on the cpu/apu chip crates. `#![no_std]` + alloc so it cross-compiles to
//! a bare-metal target; only the frontend carries `std` + `unsafe`.
//!
//! # Timing convention
//!
//! Per `docs/scheduler.md` (binding): RustySNES counts **341 dots of nominally 4 master
//! clocks** per line; the scheduler advances the PPU one dot at a time via [`Ppu::tick_dot`].
//! H runs 0..=340 (341 wraps to a new line), V runs 0..=261 (NTSC) / 0..=311 (PAL). Active
//! output is dots 22..=277 on lines 1..=224 (1..=239 overscan); `VBlank` asserts at V=225
//! (V=240 overscan). The renderer is per-scanline (it composites a whole visible line at
//! [`RENDER_DOT`], one dot before that line's own per-line HDMA run can observe/mutate the
//! registers the composite reads), which is far simpler than a per-dot renderer and
//! bit-identical to one for every currently-modeled case, including a per-line HDMA-driven
//! register write (e.g. a raster scroll split) — which only becomes visible starting the
//! following line, matching real hardware (`docs/ppu.md` §Mid-scanline/HDMA-driven register
//! timing, landed `v0.8.0`).
//!
//! # Rendering note (clean-room)
//!
//! The register semantics and rendering math here are re-implemented from `docs/ppu.md` plus
//! the documented SNES hardware behavior (SNESdev/Fullsnes), structurally informed by the ares
//! (ISC) PPU. No source was copied/ported verbatim.

#![no_std]
#![forbid(unsafe_code)]
// reason: the pixel-compositing math is full of deliberate width-narrowing and sign-changing
// casts (folding 16-bit fixed-point Mode 7 coordinates to indices, taking the low byte of a
// palette word, mapping a signed scroll delta into a VRAM offset). They are intrinsic to a
// bit-accurate PPU model and ubiquitous; flagging each one would bury the genuine signal.
// This mirrors how `rustysnes-cpu/src/exec.rs` blankets the same lints for the ALU.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless
)]
// reason: the register/state structs (`Io`, `Ppu`, `WindowIo`/`WindowLayer`) mirror the SNES
// hardware bit-fields, where a flock of independent enable/flip/invert bits is the genuine
// shape — bundling them into bitflags would obscure the 1:1 register mapping. And `Box::new`ing
// the 64 KiB VRAM / 122 KiB framebuffer arrays does build them on the stack transiently, but
// that is the one-shot power-on path, not a hot loop; the heap home is the whole point.
#![allow(clippy::struct_excessive_bools, clippy::large_stack_arrays)]
extern crate alloc;

use alloc::boxed::Box;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

pub mod bus;
// HD texture pack tile-identity hashing + the `Ppu`-side `TileTag` recording hook (`v1.3.0`) --
// off by default (`Ppu::set_hd_pack_tagging`), and compiled out entirely (not just runtime-inert)
// when the `hd-pack` feature is off.
#[cfg(feature = "hd-pack")]
pub mod hdtag;
mod regs;
mod render;

pub use bus::VideoBus;

/// Master clocks per PPU dot (nominal). The scheduler owns the long-dot remainder; this crate
/// just counts dots. See `docs/scheduler.md` "Convention (binding)".
pub const MASTER_CLOCKS_PER_DOT: u32 = 4;

/// Dots per scanline: **340**, numbered `0..=339`.
///
/// Hardware has no dot 340 — fullsnes' H-counter-latch histogram reports it latching *never*, where
/// every real dot latches four times and dots 323 and 327 latch six. The line is still 1364 master
/// clocks; the scheduler owns the distribution (`rustysnes-core`'s `LONG_DOTS`) and this crate just
/// counts dots. `T-06-A`.
pub const DOTS_PER_LINE: u16 = 340;

/// The dot at which a visible scanline's composited output becomes final for that line.
///
/// The SNES hardware fact behind this: a line's own per-line HDMA transfer (`rustysnes-core`'s
/// `HDMA_RUN_DOT`) fires at hcounter 1104 = dot 276, strictly *after* real hardware's per-pixel
/// active-region output for that same line has already completed (ares' `cycleRenderPixel()`
/// only runs for hcounter `[56, 1078]` — dot ~269.5 — `ref-proj/ares/ares/sfc/ppu/main.cpp`).
/// [`Ppu::tick_dot`] composites the finishing line here, one dot before this line's own HDMA run
/// can observe/mutate the registers that composite reads, so an HDMA-driven per-line register
/// write during line `V` is only ever visible starting line `V+1` — matching real hardware. This
/// is the single source of truth `rustysnes-core::bus`'s `HDMA_RUN_DOT` is defined equal to
/// (PPU-owned since it is fundamentally a video-timing fact, not a DMA-specific one); a `#[test]`
/// in `rustysnes-core` (which depends on this crate, not the reverse) asserts the two never
/// drift apart. See `docs/ppu.md` §Mid-scanline/HDMA-driven register timing for the full
/// mechanism and regression history.
pub const RENDER_DOT: u16 = 276;

/// Visible width of one rendered scanline in normal (non-hires) resolution, in pixels.
///
/// This is also the per-pixel-clock compositing width: one PPU pixel clock produces one
/// `above`/`below` layer-pixel pair regardless of resolution — only the DAC output stage doubles
/// for hi-res (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision).
pub const SCREEN_WIDTH: usize = 256;

/// Output width of one rendered scanline in hi-res (Modes 5/6, or pseudo-hires `SETINI` bit 3),
/// in pixels — the DAC emits two output columns per pixel clock in this mode.
pub const MAX_SCREEN_WIDTH: usize = 512;

/// Maximum visible height (overscan). Standard frames fill the first 224 rows.
pub const SCREEN_HEIGHT: usize = 239;

/// First active output dot on a visible line.
const ACTIVE_DOT_START: u16 = 22;

/// Dots by which the HV-IRQ horizontal comparator lags the programmed `HTIME`, modelling the
/// SNES hardware communication delay between the counter unit and the CPU's interrupt logic
/// (ares `hcounter(10) == (HTIME+1)<<2` ⇒ fire at dot `HTIME + 3.5`; see `check_hv_irq`).
const HIRQ_TRIGGER_DELAY: u16 = 4;

/// The dot at which a V-only IRQ's comparator is sampled.
///
/// The SNESdev timing notes put a V-IRQ at `V = VTIME, H ~ 2.5`; 2 is the nearest whole dot the
/// counter takes. Only relevant when `$4200` selects V without H — with H enabled the H target
/// already pins the trigger to one dot.
const VIRQ_TRIGGER_DOT: u16 = 2;

/// Framebuffer length at normal (non-hires) resolution, in 15-bit BGR pixels.
///
/// This is the length [`Ppu::framebuffer`] returns for every frame that never enters hi-res —
/// i.e. every currently shipping ROM/golden-vector, which stays byte-identical to before this
/// const's hi-res sibling was added.
pub const FRAMEBUFFER_LEN: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

/// The framebuffer's fixed backing-array allocation size — always hi-res-capacity, so a
/// mode change mid-run never needs a reallocation. [`Ppu::framebuffer`] returns a
/// resolution-sized *slice* of this backing storage, not the whole array.
const MAX_FRAMEBUFFER_LEN: usize = MAX_SCREEN_WIDTH * SCREEN_HEIGHT;

/// Video region — fixes the line count and the NTSC/PAL status bit. Data, not behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Region {
    /// 262 lines / frame, 60 Hz. The default for the North-American / Japanese SNES.
    #[default]
    Ntsc,
    /// 312 lines / frame, 50 Hz.
    Pal,
}

impl Region {
    /// Total scanlines per (non-interlaced) frame for this region.
    #[must_use]
    pub const fn lines_per_frame(self) -> u16 {
        match self {
            Self::Ntsc => 262,
            Self::Pal => 312,
        }
    }
}

/// One sprite as decoded from the OAM low + high tables. `width`/`height` depend on `OBSEL`.
#[derive(Debug, Clone, Copy, Default)]
struct Object {
    x: u16,           // 9-bit X (sign via bit 8)
    y: u8,            // Y top
    character: u8,    // tile number low 8 bits
    nameselect: bool, // name-table select bit
    palette: u8,      // 3-bit palette group
    priority: u8,     // 2-bit priority
    hflip: bool,
    vflip: bool,
    size: bool, // large/small toggle (high table)
}

/// The shared B-bus / PPU register I/O state. Most fields map 1:1 onto a documented register
/// bit-field in `docs/ppu.md`; grouped here so the renderer reads a single struct.
#[derive(Debug, Clone)]
struct Io {
    // INIDISP $2100
    display_brightness: u8, // 0..=15
    display_disable: bool,

    // BGMODE $2105
    bg_mode: u8, // 0..=7
    bg3_priority: bool,
    tile_size: [bool; 4], // per-BG 16x16 select

    // MOSAIC $2106
    mosaic_size: u8, // 1..=16
    mosaic_enable: [bool; 4],

    // Per-BG: BGnSC ($2107..a) + BGnNBA ($210b..c)
    bg_screen_addr: [u16; 4],   // word address of the tilemap base
    bg_screen_size: [u8; 4],    // 0..=3 (H/V 32/64-tile expansion)
    bg_tiledata_addr: [u16; 4], // word address of the char base
    bg_hofs: [u16; 4],
    bg_vofs: [u16; 4],

    // VMAIN $2115 + VMADD $2116/7 + the $2139/A read prefetch latch
    vram_increment_size: u16,  // 1 / 32 / 128 words
    vram_mapping: u8,          // 0..=3 address remap
    vram_increment_high: bool, // false = increment on $2118/$2139 (low), true = high
    vram_address: u16,         // word address
    vram_read_latch: u16,      // prefetch latch for $2139/A

    // M7SEL $211A + M7A..D + M7X/Y
    m7_hflip: bool,
    m7_vflip: bool,
    m7_repeat: u8, // 0..=3
    m7a: u16,
    m7b: u16,
    m7c: u16,
    m7d: u16,
    m7x: u16,
    m7y: u16,
    m7_hofs: u16,
    m7_vofs: u16,

    // CGADD $2121 / CGDATA $2122 / $213B read
    cgram_address: u8,
    cgram_latch_high: bool, // false: next access is the low byte
    cgram_byte_latch: u8,   // low byte held between the two CGDATA writes

    // OAMADD $2102/3 + the running address
    oam_base_address: u16, // word-pair base (<<1 of the register value)
    oam_priority_rotation: bool,
    oam_address: u16,   // running 10-bit address
    oam_byte_latch: u8, // even-byte hold for OAMDATA writes

    // Windows $2123..$2129 + $212A/B masks, per layer
    win: WindowIo,

    // TM/TS $212C/D — main / sub layer enable (bg1..4, obj)
    main_enable: [bool; 5],
    sub_enable: [bool; 5],
    // TMW/TSW $212E/F — per-layer window masking enable
    win_main_enable: [bool; 5],
    win_sub_enable: [bool; 5],

    // CGWSEL $2130 / CGADSUB $2131 / COLDATA $2132
    direct_color: bool,
    add_subscreen: bool,          // false: fixed color, true: subscreen as addend
    color_window_above: u8,       // 0..=3 (force-main mask)
    color_window_below: u8,       // 0..=3 (sub/clip mask)
    color_math_enable: [bool; 6], // bg1..4, obj, backdrop
    color_halve: bool,
    color_subtract: bool, // false: add, true: subtract
    fixed_color: u16,     // 15-bit BGR

    // SETINI $2133
    interlace: bool,
    obj_interlace: bool,
    overscan: bool,
    pseudo_hires: bool,
    extbg: bool,

    // OBSEL $2101
    obj_tiledata_addr: u16,
    obj_nameselect: u16,
    obj_base_size: u8, // 0..=7

    // H/V counter latch ($2137 / $213C/D / $213F)
    latch_h: u16,
    latch_v: u16,
    counter_latched: bool,
    ophct_high_toggle: bool,
    opvct_high_toggle: bool,

    // Sprite over-flags (STAT77 bits 6/7) — set during OAM evaluation.
    range_over: bool,
    time_over: bool,

    // PPU open-bus / MDR latches for read-back of write-only / unused registers.
    ppu1_mdr: u8,
    ppu2_mdr: u8,
}

/// Per-layer window configuration (`$2123`–`$212B`). Six "layers" share the same shape: the
/// four BGs, the sprites, and the color-math region.
#[derive(Debug, Clone, Default)]
struct WindowIo {
    one_left: u8,
    one_right: u8,
    two_left: u8,
    two_right: u8,
    /// Per layer 0..=5 (bg1..4, obj, col): (`w1_enable`, `w1_invert`, `w2_enable`, `w2_invert`, mask).
    layer: [WindowLayer; 6],
}

#[derive(Debug, Clone, Copy, Default)]
struct WindowLayer {
    one_enable: bool,
    one_invert: bool,
    two_enable: bool,
    two_invert: bool,
    mask: u8, // 0=OR 1=AND 2=XOR 3=XNOR
}

impl Default for Io {
    fn default() -> Self {
        Self {
            display_brightness: 0,
            display_disable: true, // power-on is force-blank
            bg_mode: 0,
            bg3_priority: false,
            tile_size: [false; 4],
            mosaic_size: 1,
            mosaic_enable: [false; 4],
            bg_screen_addr: [0; 4],
            bg_screen_size: [0; 4],
            bg_tiledata_addr: [0; 4],
            bg_hofs: [0; 4],
            bg_vofs: [0; 4],
            vram_increment_size: 1,
            vram_mapping: 0,
            vram_increment_high: false,
            vram_address: 0,
            vram_read_latch: 0,
            m7_hflip: false,
            m7_vflip: false,
            m7_repeat: 0,
            m7a: 0,
            m7b: 0,
            m7c: 0,
            m7d: 0,
            m7x: 0,
            m7y: 0,
            m7_hofs: 0,
            m7_vofs: 0,
            cgram_address: 0,
            cgram_latch_high: false,
            cgram_byte_latch: 0,
            oam_base_address: 0,
            oam_priority_rotation: false,
            oam_address: 0,
            oam_byte_latch: 0,
            win: WindowIo::default(),
            main_enable: [false; 5],
            sub_enable: [false; 5],
            win_main_enable: [false; 5],
            win_sub_enable: [false; 5],
            direct_color: false,
            add_subscreen: false,
            color_window_above: 0,
            color_window_below: 0,
            color_math_enable: [false; 6],
            color_halve: false,
            color_subtract: false,
            fixed_color: 0,
            interlace: false,
            obj_interlace: false,
            overscan: false,
            pseudo_hires: false,
            extbg: false,
            obj_tiledata_addr: 0,
            obj_nameselect: 0,
            obj_base_size: 0,
            latch_h: 0,
            latch_v: 0,
            counter_latched: false,
            ophct_high_toggle: false,
            opvct_high_toggle: false,
            range_over: false,
            time_over: false,
            ppu1_mdr: 0,
            ppu2_mdr: 0,
        }
    }
}

impl WindowLayer {
    fn save_state(self, s: &mut SaveWriter) {
        s.write_bool(self.one_enable);
        s.write_bool(self.one_invert);
        s.write_bool(self.two_enable);
        s.write_bool(self.two_invert);
        s.write_u8(self.mask);
    }

    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.one_enable = s.read_bool()?;
        self.one_invert = s.read_bool()?;
        self.two_enable = s.read_bool()?;
        self.two_invert = s.read_bool()?;
        self.mask = s.read_u8()? & 0x03;
        Ok(())
    }
}

impl WindowIo {
    fn save_state(&self, s: &mut SaveWriter) {
        s.write_u8(self.one_left);
        s.write_u8(self.one_right);
        s.write_u8(self.two_left);
        s.write_u8(self.two_right);
        for l in &self.layer {
            l.save_state(s);
        }
    }

    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.one_left = s.read_u8()?;
        self.one_right = s.read_u8()?;
        self.two_left = s.read_u8()?;
        self.two_right = s.read_u8()?;
        for l in &mut self.layer {
            l.load_state(s)?;
        }
        Ok(())
    }
}

impl Io {
    /// Write every register field, in declaration order, into the caller's section.
    fn save_state(&self, s: &mut SaveWriter) {
        s.write_u8(self.display_brightness);
        s.write_bool(self.display_disable);
        s.write_u8(self.bg_mode);
        s.write_bool(self.bg3_priority);
        for &v in &self.tile_size {
            s.write_bool(v);
        }
        s.write_u8(self.mosaic_size);
        for &v in &self.mosaic_enable {
            s.write_bool(v);
        }
        for &v in &self.bg_screen_addr {
            s.write_u16(v);
        }
        for &v in &self.bg_screen_size {
            s.write_u8(v);
        }
        for &v in &self.bg_tiledata_addr {
            s.write_u16(v);
        }
        for &v in &self.bg_hofs {
            s.write_u16(v);
        }
        for &v in &self.bg_vofs {
            s.write_u16(v);
        }
        s.write_u16(self.vram_increment_size);
        s.write_u8(self.vram_mapping);
        s.write_bool(self.vram_increment_high);
        s.write_u16(self.vram_address);
        s.write_u16(self.vram_read_latch);
        s.write_bool(self.m7_hflip);
        s.write_bool(self.m7_vflip);
        s.write_u8(self.m7_repeat);
        s.write_u16(self.m7a);
        s.write_u16(self.m7b);
        s.write_u16(self.m7c);
        s.write_u16(self.m7d);
        s.write_u16(self.m7x);
        s.write_u16(self.m7y);
        s.write_u16(self.m7_hofs);
        s.write_u16(self.m7_vofs);
        s.write_u8(self.cgram_address);
        s.write_bool(self.cgram_latch_high);
        s.write_u8(self.cgram_byte_latch);
        s.write_u16(self.oam_base_address);
        s.write_bool(self.oam_priority_rotation);
        s.write_u16(self.oam_address);
        s.write_u8(self.oam_byte_latch);
        self.win.save_state(s);
        for &v in &self.main_enable {
            s.write_bool(v);
        }
        for &v in &self.sub_enable {
            s.write_bool(v);
        }
        for &v in &self.win_main_enable {
            s.write_bool(v);
        }
        for &v in &self.win_sub_enable {
            s.write_bool(v);
        }
        s.write_bool(self.direct_color);
        s.write_bool(self.add_subscreen);
        s.write_u8(self.color_window_above);
        s.write_u8(self.color_window_below);
        for &v in &self.color_math_enable {
            s.write_bool(v);
        }
        s.write_bool(self.color_halve);
        s.write_bool(self.color_subtract);
        s.write_u16(self.fixed_color);
        s.write_bool(self.interlace);
        s.write_bool(self.obj_interlace);
        s.write_bool(self.overscan);
        s.write_bool(self.pseudo_hires);
        s.write_bool(self.extbg);
        s.write_u16(self.obj_tiledata_addr);
        s.write_u16(self.obj_nameselect);
        s.write_u8(self.obj_base_size);
        s.write_u16(self.latch_h);
        s.write_u16(self.latch_v);
        s.write_bool(self.counter_latched);
        s.write_bool(self.ophct_high_toggle);
        s.write_bool(self.opvct_high_toggle);
        s.write_bool(self.range_over);
        s.write_bool(self.time_over);
        s.write_u8(self.ppu1_mdr);
        s.write_u8(self.ppu2_mdr);
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input. No field here is used as an unchecked
    /// array index: `cgram_address` (`u8`) indexes the 256-entry `cgram` exactly; `oam_address`
    /// is masked `& 0x03ff` at every read/write site in `regs.rs` before use (never trusted
    /// verbatim there either); `vram_address`/`vram_read_latch`-derived offsets are masked
    /// `& 0x7fff` at every VRAM access site — so none of those needed additional masking for
    /// memory safety. Every register a normal write constrains to a narrower width than its
    /// storage type IS masked here to that width, though (`display_brightness`/`mosaic_size`
    /// 4-bit, `bg_mode`/`obj_base_size` 3-bit, `bg_screen_size`/`vram_mapping`/`m7_repeat`/
    /// `color_window_above`/`color_window_below`/`WindowLayer::mask` 2-bit), matching the same
    /// "apply the engine's own normal-operation invariant on load" reasoning already applied
    /// elsewhere in this project.
    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.display_brightness = s.read_u8()? & 0x0F;
        self.display_disable = s.read_bool()?;
        self.bg_mode = s.read_u8()? & 0x07;
        self.bg3_priority = s.read_bool()?;
        for v in &mut self.tile_size {
            *v = s.read_bool()?;
        }
        self.mosaic_size = s.read_u8()? & 0x0F;
        for v in &mut self.mosaic_enable {
            *v = s.read_bool()?;
        }
        for v in &mut self.bg_screen_addr {
            *v = s.read_u16()?;
        }
        for v in &mut self.bg_screen_size {
            *v = s.read_u8()? & 0x03;
        }
        for v in &mut self.bg_tiledata_addr {
            *v = s.read_u16()?;
        }
        for v in &mut self.bg_hofs {
            *v = s.read_u16()?;
        }
        for v in &mut self.bg_vofs {
            *v = s.read_u16()?;
        }
        self.vram_increment_size = s.read_u16()?;
        self.vram_mapping = s.read_u8()? & 0x03;
        self.vram_increment_high = s.read_bool()?;
        self.vram_address = s.read_u16()?;
        self.vram_read_latch = s.read_u16()?;
        self.m7_hflip = s.read_bool()?;
        self.m7_vflip = s.read_bool()?;
        self.m7_repeat = s.read_u8()? & 0x03;
        self.m7a = s.read_u16()?;
        self.m7b = s.read_u16()?;
        self.m7c = s.read_u16()?;
        self.m7d = s.read_u16()?;
        self.m7x = s.read_u16()?;
        self.m7y = s.read_u16()?;
        self.m7_hofs = s.read_u16()?;
        self.m7_vofs = s.read_u16()?;
        self.cgram_address = s.read_u8()?;
        self.cgram_latch_high = s.read_bool()?;
        self.cgram_byte_latch = s.read_u8()?;
        self.oam_base_address = s.read_u16()?;
        self.oam_priority_rotation = s.read_bool()?;
        self.oam_address = s.read_u16()?;
        self.oam_byte_latch = s.read_u8()?;
        self.win.load_state(s)?;
        for v in &mut self.main_enable {
            *v = s.read_bool()?;
        }
        for v in &mut self.sub_enable {
            *v = s.read_bool()?;
        }
        for v in &mut self.win_main_enable {
            *v = s.read_bool()?;
        }
        for v in &mut self.win_sub_enable {
            *v = s.read_bool()?;
        }
        self.direct_color = s.read_bool()?;
        self.add_subscreen = s.read_bool()?;
        self.color_window_above = s.read_u8()? & 0x03;
        self.color_window_below = s.read_u8()? & 0x03;
        for v in &mut self.color_math_enable {
            *v = s.read_bool()?;
        }
        self.color_halve = s.read_bool()?;
        self.color_subtract = s.read_bool()?;
        self.fixed_color = s.read_u16()?;
        self.interlace = s.read_bool()?;
        self.obj_interlace = s.read_bool()?;
        self.overscan = s.read_bool()?;
        self.pseudo_hires = s.read_bool()?;
        self.extbg = s.read_bool()?;
        self.obj_tiledata_addr = s.read_u16()?;
        self.obj_nameselect = s.read_u16()?;
        self.obj_base_size = s.read_u8()? & 0x07;
        self.latch_h = s.read_u16()?;
        self.latch_v = s.read_u16()?;
        self.counter_latched = s.read_bool()?;
        self.ophct_high_toggle = s.read_bool()?;
        self.opvct_high_toggle = s.read_bool()?;
        self.range_over = s.read_bool()?;
        self.time_over = s.read_bool()?;
        self.ppu1_mdr = s.read_u8()?;
        self.ppu2_mdr = s.read_u8()?;
        Ok(())
    }
}

/// PPU1 (5C77) + PPU2 (5C78) state.
///
/// Owns VRAM (32 K words), CGRAM (256 × 15-bit BGR), OAM (544 bytes), the full register file,
/// the dot/scanline timeline, and a 256×239 15-bit framebuffer. Advanced one dot at a time by
/// the master-clock scheduler via [`Ppu::tick_dot`]; the scheduler polls [`Ppu::nmi_pending`],
/// [`Ppu::irq_pending`], and [`Ppu::frame_ready`] to drive interrupts and presentation.
#[derive(Clone)]
pub struct Ppu {
    /// 64 KiB of video RAM as 32 K 16-bit words (word-addressed).
    vram: Box<[u16; 0x8000]>,
    /// 256 palette entries, 15-bit BGR.
    cgram: [u16; 256],
    /// Object attribute memory: 512-byte low table + 32-byte high table.
    oam: [u8; 544],

    io: Io,
    region: Region,

    // --- Shared write latches (cross-register, so they live on the Ppu, not Io) ---
    /// Mode-7 / scroll write-twice byte latch (`docs/ppu.md`: the M7A–M7Y + BG-offset latch).
    mode7_byte_latch: u16,
    /// BG-offset "previous PPU1 byte" half of the shared scroll write latch.
    bgofs_prev1: u16,
    /// BG-offset "previous PPU2 byte" half of the shared scroll write latch.
    bgofs_prev2: u16,

    // --- Timeline ---
    /// Horizontal dot counter, 0..=340.
    h: u16,
    /// Vertical scanline counter, 0..=(lines_per_frame-1).
    v: u16,
    /// Interlace field (toggles each frame when interlace is on).
    field: bool,
    /// Currently inside vertical blank.
    vblank: bool,
    /// Currently inside horizontal blank.
    hblank: bool,

    // --- Interrupt / frame polls ---
    /// NMI request, latched at `VBlank` start; cleared by [`Ppu::ack_nmi`].
    nmi_pending: bool,
    /// IRQ request from the HV comparator; cleared by [`Ppu::ack_irq`].
    irq_pending: bool,
    /// Whether the HV-IRQ comparator is armed (the scheduler/CPU programs H/V + enable).
    irq_enable_h: bool,
    irq_enable_v: bool,
    irq_h: u16,
    irq_v: u16,
    /// Set when a full frame has been composited; cleared by [`Ppu::take_frame`].
    frame_ready: bool,
    /// Monotonic completed-frame counter (determinism diagnostics / save-states).
    frame_count: u64,
    /// Whether the *current* frame's output is hi-res (512-wide) — latched from [`Ppu::is_hires`]
    /// at the first visible scanline of each frame (row 0) and held for the rest of that frame,
    /// so the framebuffer's row stride stays consistent across every line of one frame even if
    /// `BGMODE`/`SETINI` change mid-frame (`docs/ppu.md` §Hi-res (Modes 5/6) color-math
    /// precision — a documented, deliberate per-frame-not-per-scanline simplification).
    frame_hires: bool,

    /// The composited 15-bit BGR framebuffer: [`FRAMEBUFFER_LEN`] (256×239) words for a normal
    /// frame, [`MAX_FRAMEBUFFER_LEN`] (512×239) for a hi-res one. Backing storage is always
    /// allocated at hi-res capacity; [`Ppu::framebuffer`] returns the resolution-sized slice.
    framebuffer: Box<[u16; MAX_FRAMEBUFFER_LEN]>,

    /// Whether [`render::render_scanline`](crate) records a [`hdtag::TileTag`] per composited
    /// pixel into `tile_tags` this frame (`v1.3.0`, `hd-pack` feature). Off by default — a
    /// host/frontend convenience switch, never part of `save_state`/`load_state` (the same carve-
    /// out already established for cheats/watchpoints/voice-mutes/port2-peripheral).
    #[cfg(feature = "hd-pack")]
    hd_pack_tagging: bool,
    /// Write-only tile-identity side-buffer paralleling [`Ppu::framebuffer`] pixel-for-pixel
    /// (same indexing, same hi-res-capacity backing length — a boxed slice rather than a boxed
    /// fixed-size array: building a ~2 MiB `[TileTag; MAX_FRAMEBUFFER_LEN]` array value before
    /// moving it into a `Box` materializes it on the stack first in an unoptimized build, which
    /// overflows the default thread stack; `alloc::vec!` fills the heap allocation in place,
    /// one element at a time, with no such stack temporary). Only written when `hd_pack_tagging`
    /// is set; when it is `false` (the default), every entry stays [`hdtag::TileTag::default`]
    /// and `render_scanline`'s hot paths never touch it or the (feature-gated-out-entirely-when-
    /// off) hashing helper — see `hd_pack_tagging_toggle_does_not_alter_framebuffer_output` for
    /// the regression proof that toggling this never changes `framebuffer()`'s bytes.
    #[cfg(feature = "hd-pack")]
    tile_tags: Box<[hdtag::TileTag]>,

    // --- Per-dot compositor state (`per-dot-compositor` feature, `docs/adr/0014`, T-CA-10 Phase 4).
    //
    // The visible line is composited one dot at a time (blueprint: MesenCE `SnesPpu::RenderScanline`).
    // These hold the current line's fetched pixels and the incremental draw cursor. They are transient
    // (re-derived at each line start from serialized PPU state), so they are deliberately NOT saved —
    // a save at a frame boundary (the normal case) has no mid-line cursor to preserve; a mid-line save
    // under this experimental flag re-fetches on load, documented in `docs/ppu.md`.
    /// This line's fetched above/below pixels (built once per line, drained per dot).
    #[cfg(feature = "per-dot-compositor")]
    pd_above: alloc::boxed::Box<[render::Pixel; SCREEN_WIDTH]>,
    #[cfg(feature = "per-dot-compositor")]
    pd_below: alloc::boxed::Box<[render::Pixel; SCREEN_WIDTH]>,
    /// The DAC carry threaded through the incremental composite (ares' one-column hi-res delay).
    #[cfg(feature = "per-dot-compositor")]
    pd_carry: render::DacCarry,
    /// Next output column to composite (the draw cursor; MesenCE `_drawStartX`). Reset to 0 per line.
    #[cfg(feature = "per-dot-compositor")]
    pd_draw_x: u16,
    /// Which visible line (`self.v`) `pd_above`/`pd_below` were fetched for; `u16::MAX` = not fetched
    /// (forces a re-fetch — also the state after a save-state load).
    #[cfg(feature = "per-dot-compositor")]
    pd_fetched_line: u16,
    /// The CGRAM index of the last-drawn column (MesenCE `_state.InternalCgramAddress`, `dac.cpp`
    /// `paletteColor`). A CGRAM write during active display is redirected here — the exact,
    /// draw-cursor-driven form of the in-render redirect (supersedes the on-demand `h-22` approximation).
    #[cfg(feature = "per-dot-compositor")]
    internal_cgram_address: u8,
}

impl core::fmt::Debug for Ppu {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ppu")
            .field("h", &self.h)
            .field("v", &self.v)
            .field("vblank", &self.vblank)
            .field("region", &self.region)
            .field("frame_count", &self.frame_count)
            .finish_non_exhaustive()
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    /// Construct at power-on (NTSC). Storage starts zeroed; this is deterministic — the
    /// determinism contract forbids the OS RNG (`docs/adr/0004`). Phase alignment, if any, is
    /// driven by the scheduler's seeded PRNG, not here.
    #[must_use]
    pub fn new() -> Self {
        Self::with_region(Region::Ntsc)
    }

    /// Construct at power-on for a specific [`Region`].
    #[must_use]
    pub fn with_region(region: Region) -> Self {
        Self {
            vram: Box::new([0u16; 0x8000]),
            cgram: [0u16; 256],
            oam: [0u8; 544],
            io: Io::default(),
            region,
            mode7_byte_latch: 0,
            bgofs_prev1: 0,
            bgofs_prev2: 0,
            h: 0,
            v: 0,
            field: false,
            vblank: false,
            hblank: false,
            nmi_pending: false,
            irq_pending: false,
            irq_enable_h: false,
            irq_enable_v: false,
            irq_h: 0,
            irq_v: 0,
            frame_ready: false,
            frame_count: 0,
            frame_hires: false,
            framebuffer: Box::new([0u16; MAX_FRAMEBUFFER_LEN]),
            #[cfg(feature = "hd-pack")]
            hd_pack_tagging: false,
            #[cfg(feature = "hd-pack")]
            tile_tags: alloc::vec![hdtag::TileTag::default(); MAX_FRAMEBUFFER_LEN]
                .into_boxed_slice(),
            #[cfg(feature = "per-dot-compositor")]
            pd_above: Box::new([render::Pixel::default(); SCREEN_WIDTH]),
            #[cfg(feature = "per-dot-compositor")]
            pd_below: Box::new([render::Pixel::default(); SCREEN_WIDTH]),
            #[cfg(feature = "per-dot-compositor")]
            pd_carry: render::DacCarry::default(),
            #[cfg(feature = "per-dot-compositor")]
            pd_draw_x: 0,
            #[cfg(feature = "per-dot-compositor")]
            pd_fetched_line: u16::MAX,
            #[cfg(feature = "per-dot-compositor")]
            internal_cgram_address: 0,
        }
    }

    /// Set the video region (NTSC/PAL). Affects line count + the STAT78 region bit.
    pub const fn set_region(&mut self, region: Region) {
        self.region = region;
    }

    /// The active video region (NTSC/PAL).
    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }

    /// The total visible height for the current overscan setting (224 or 239).
    #[must_use]
    pub const fn visible_height(&self) -> u16 {
        if self.io.overscan { 239 } else { 224 }
    }

    /// Whether the PPU is *currently configured* for hi-res output: `BGMODE` 5/6, or pseudo-hires
    /// (`SETINI` $2133 bit 3) in any mode — matching ares' `hires = pseudoHires || bgMode==5 ||
    /// bgMode==6` (`ref-proj/ares/ares/sfc/ppu/dac.cpp`). This is the live per-scanline register
    /// state; [`Ppu::frame_hires`] is the value latched from this at the start of the frame that's
    /// actually being composited into the framebuffer right now.
    #[must_use]
    pub const fn is_hires(&self) -> bool {
        self.io.pseudo_hires || self.io.bg_mode == 5 || self.io.bg_mode == 6
    }

    /// Whether the framebuffer *currently being composited* is hi-res (512-wide) — the value
    /// [`Ppu::framebuffer`]'s length/stride is based on for this frame. See the `frame_hires`
    /// field doc for why this is latched once per frame rather than read live per scanline.
    #[must_use]
    pub const fn frame_hires(&self) -> bool {
        self.frame_hires
    }

    /// The output framebuffer's width for the frame currently being composited: [`SCREEN_WIDTH`]
    /// normally, [`MAX_SCREEN_WIDTH`] when [`Ppu::frame_hires`] is set.
    #[must_use]
    pub const fn visible_width(&self) -> usize {
        if self.frame_hires {
            MAX_SCREEN_WIDTH
        } else {
            SCREEN_WIDTH
        }
    }

    /// First `VBlank` scanline for the current overscan setting (225 or 240).
    #[must_use]
    const fn vblank_line(&self) -> u16 {
        if self.io.overscan { 240 } else { 225 }
    }

    /// Advance the PPU by exactly one dot, the scheduler's video quantum. Composites a full
    /// visible scanline at [`RENDER_DOT`] (one dot before that line's own per-line HDMA run can
    /// observe/mutate the registers the composite reads — see `docs/ppu.md` §Mid-scanline/
    /// HDMA-driven register timing); raises NMI at `VBlank` start and the HV-IRQ when the
    /// programmed H/V is hit; calls [`VideoBus::notify_scanline`] at each line start and
    /// [`VideoBus::notify_vblank`] when `VBlank` begins. Hot path: allocation-free.
    pub fn tick_dot(&mut self, bus: &mut impl VideoBus) {
        // HBlank region: dots 274..=340 (active output ends near dot 274).
        self.hblank = self.h >= 274 || self.h < ACTIVE_DOT_START;

        // HV-IRQ comparator (level): fire when the enabled H and/or V positions match.
        self.check_hv_irq();

        // Composite the line that's finishing (V is the line number 1..=visible) using register
        // state as it stood just BEFORE this line's own HDMA run, not after (`docs/ppu.md`'s
        // confirmed off-by-one-line fix). `advance_master` services this line's HDMA at this
        // same dot, strictly AFTER this render call within the same master-clock tick (the HDMA
        // check runs after `tick_ppu_dot` returns) -- see `docs/scheduler.md` §DMA/HDMA bus-steal.
        #[cfg(not(feature = "per-dot-compositor"))]
        if self.h == RENDER_DOT && self.v >= 1 && self.v <= self.visible_height() {
            self.render_scanline(bus);
        }
        // Per-dot compositor: composite incrementally up to the current dot's column every tick,
        // with live register state (`docs/adr/0014`, T-CA-10 Phase 4). Finishes each line by
        // `RENDER_DOT`, before that line's HDMA — byte-identical to the batch on a static line.
        #[cfg(feature = "per-dot-compositor")]
        self.pd_render_to_dot(bus);

        self.h += 1;
        if self.h >= DOTS_PER_LINE {
            // End of a scanline: advance V and fire frame/VBlank events. Rendering already
            // happened above at RENDER_DOT, not here.
            self.h = 0;
            self.end_of_scanline(bus);
        }
    }

    /// At the end of a scanline: advance V, and fire frame/VBlank events. Rendering happens
    /// earlier, at [`RENDER_DOT`] within [`Ppu::tick_dot`] -- see that method's doc.
    fn end_of_scanline(&mut self, bus: &mut impl VideoBus) {
        self.v += 1;
        let lines = self.region.lines_per_frame();
        if self.v >= lines {
            self.v = 0;
            self.field = !self.field;
            // A new frame begins; the composited buffer for the prior frame is ready.
            self.frame_ready = true;
            self.frame_count = self.frame_count.wrapping_add(1);
            // Sprite over-flags reset at end of VBlank (start of new frame).
            self.io.range_over = false;
            self.io.time_over = false;
        }

        // Notify the board of the new scanline.
        bus.notify_scanline();

        // VBlank starts at V=225 (or 240 in overscan).
        let vbl = self.vblank_line();
        if self.v == vbl {
            self.vblank = true;
            self.nmi_pending = true;
            // The OAM address reloads from its base once per frame, as vblank begins, and only
            // while forced blank is off. Sprite evaluation leaves the running counter wherever it
            // finished, so without this reload an address a game set up would not survive a frame.
            // Conditional on force-blank because a forced-blank frame runs no evaluation and so
            // performs no reload — see AccuracySNES C1.06 and `docs/ppu.md`.
            if !self.io.display_disable {
                self.io.oam_address = self.io.oam_base_address;
            }
            bus.notify_vblank();
        } else if self.v == 0 {
            self.vblank = false;
        }
    }

    /// Level-evaluate the HV-IRQ comparator at the current (h, v).
    ///
    /// The horizontal match is asserted [`HIRQ_TRIGGER_DELAY`] dots *after* the programmed
    /// `HTIME`, modelling the SNES's hardware communication delay between the H/V counter unit
    /// and the CPU's interrupt logic. ares encodes this as `hcounter(10) == io.htime` with
    /// `io.htime` stored as `(HTIME + 1) << 2` clocks (`sfc/cpu/irq.cpp`, `sfc/cpu/io.cpp`), i.e.
    /// the IRQ fires at hcounter `HTIME*4 + 14` = dot `HTIME + 3.5`. Without this delay an
    /// IRQ-gated register write (e.g. the `hdmaen_latch_test` `STA $420C`) lands ~3–4 dots early,
    /// which — combined with the dot-1104 HDMA latch — collapses the test's banded HDMAEN-vs-latch
    /// crossing into a uniform per-line alternation.
    const fn check_hv_irq(&mut self) {
        // Adding the delay can push the target past the end of the line. On hardware the H counter
        // never reaches those values (ares' stored `(HTIME+1)<<2 + 10` clocks then exceeds the max
        // hcounter), so the IRQ simply never fires for such HTIME — suppress rather than wrap into
        // the next line, which would be a spurious match hardware/ares never produce.
        let h_target = self.irq_h + HIRQ_TRIGGER_DELAY;
        let h_match = if self.irq_enable_h {
            h_target < DOTS_PER_LINE && self.h == h_target
        } else {
            // V-only IRQ: the comparator is sampled once near the start of the line, not held
            // across it. Modelling `h_match` as unconditionally true here made `V == VTIME` a
            // level that re-raised the IRQ on all 341 dots of the target line, so acknowledging
            // via `$4211` was undone a few dots later and a V-only handler saw a storm instead of
            // one interrupt. ares gets this from `irqValid.raise(...)` being an edge detector
            // (`sfc/cpu/irq.cpp:26-30`); firing on a single dot is the same thing statelessly.
            // The dot is the dossier's documented H ~ 2.5 rounded to the nearest whole dot.
            // Found by AccuracySNES B4.12.
            self.h == VIRQ_TRIGGER_DOT
        };
        let v_match = !self.irq_enable_v || self.v == self.irq_v;
        if (self.irq_enable_h || self.irq_enable_v) && h_match && v_match {
            self.irq_pending = true;
        }
    }

    // --- Interrupt / frame polls (the scheduler reads these) ---

    /// Whether an NMI is pending (asserted at `VBlank` start). Poll; clear with [`Ppu::ack_nmi`].
    #[must_use]
    pub const fn nmi_pending(&self) -> bool {
        self.nmi_pending
    }

    /// Acknowledge / clear the pending NMI (the CPU does this when it takes the vector).
    pub const fn ack_nmi(&mut self) {
        self.nmi_pending = false;
    }

    /// Whether an HV-timer IRQ is pending. Poll; clear with [`Ppu::ack_irq`].
    #[must_use]
    pub const fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    /// Acknowledge / clear the pending IRQ.
    pub const fn ack_irq(&mut self) {
        self.irq_pending = false;
    }

    /// Program the HV-IRQ comparator (the CPU writes `$4200`/`$4207`–`$420A`; the scheduler
    /// forwards them here so the PPU owns the comparison against its own H/V phase).
    pub const fn set_hv_irq(&mut self, enable_h: bool, enable_v: bool, h: u16, v: u16) {
        self.irq_enable_h = enable_h;
        self.irq_enable_v = enable_v;
        self.irq_h = h;
        self.irq_v = v;
    }

    /// PPU1's open-bus latch (`ppu1_mdr`), as a write-only or unreadable PPU1 register returns it.
    ///
    /// Exposed for the Bus, which owns the `$4201` bit 7 wiring to the counter-latch pin: when that
    /// gate is closed a `$2137` read must return open bus **without** latching, so the Bus needs
    /// the value that [`Ppu::read_reg`] would have returned without its side effect.
    #[must_use]
    pub const fn ppu1_open_bus(&self) -> u8 {
        self.io.ppu1_mdr
    }

    /// Whether the PPU is currently in vertical blank.
    #[must_use]
    pub const fn in_vblank(&self) -> bool {
        self.vblank
    }

    /// Whether the PPU is currently in horizontal blank.
    #[must_use]
    pub const fn in_hblank(&self) -> bool {
        self.hblank
    }

    /// The current horizontal dot counter (0..=340).
    #[must_use]
    pub const fn dot(&self) -> u16 {
        self.h
    }

    /// The current vertical scanline counter.
    #[must_use]
    pub const fn scanline(&self) -> u16 {
        self.v
    }

    /// Latch the current H/V dot counters into `OPHCT`/`OPVCT` ($213C/$213D) right now — the same
    /// effect a CPU read of `SLHV` ($2137) has (`regs.rs`'s `0x2137` arm calls this too, so there
    /// is one implementation of the latch itself). Real hardware also drives this from the WRIO
    /// ($4201) I/O port's bit7 falling edge (the controller-port-2 IOBIT pin, wired straight to
    /// this latch — a Super Scope's light sensor toggles it to record the CRT beam position when
    /// it "sees" it, `rustysnes_core::controller`); exposed as `pub` so the Bus (which owns WRIO,
    /// not the PPU) can trigger the identical latch from that path without duplicating it.
    pub const fn latch_hv_counters(&mut self) {
        self.io.latch_h = self.h;
        self.io.latch_v = self.v;
        self.io.counter_latched = true;
    }

    /// Whether a finished frame is available. Cleared by [`Ppu::take_frame`].
    #[must_use]
    pub const fn frame_ready(&self) -> bool {
        self.frame_ready
    }

    /// The current `BGMODE` ($2105) value (0..=7) — which of the 8 tile/priority layouts is
    /// active. For the debugger overlay's PPU panel (`docs/frontend.md` §Debugger overlay).
    #[must_use]
    pub const fn bg_mode(&self) -> u8 {
        self.io.bg_mode
    }

    /// The current `INIDISP` ($2100) master brightness (0..=15). For the debugger overlay.
    #[must_use]
    pub const fn display_brightness(&self) -> u8 {
        self.io.display_brightness
    }

    /// Completed-frame count since power-on (monotonic, wrapping).
    #[must_use]
    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// The composited framebuffer: `visible_width() * SCREEN_HEIGHT` 15-bit BGR pixels, row-major
    /// at the current frame's stride ([`SCREEN_WIDTH`] normally, [`MAX_SCREEN_WIDTH`] for a
    /// hi-res frame — see [`Ppu::frame_hires`]). Only the first [`Ppu::visible_height`] rows are
    /// written each frame. A non-hires frame's length here is exactly [`FRAMEBUFFER_LEN`],
    /// unchanged from before hi-res output existed.
    #[must_use]
    pub fn framebuffer(&self) -> &[u16] {
        &self.framebuffer[..self.visible_width() * SCREEN_HEIGHT]
    }

    /// Borrow the framebuffer and clear the `frame_ready` flag (one-shot presentation).
    pub fn take_frame(&mut self) -> &[u16] {
        self.frame_ready = false;
        &self.framebuffer[..self.visible_width() * SCREEN_HEIGHT]
    }

    /// Enable/disable per-pixel [`hdtag::TileTag`] recording (`v1.3.0`, `hd-pack` feature).
    ///
    /// Off by default. Toggling this never changes [`Ppu::framebuffer`]'s bytes for the same
    /// input — only whether [`Ppu::tile_tags`] gets populated alongside it (see
    /// `hd_pack_tagging_toggle_does_not_alter_framebuffer_output` in this module's tests for the
    /// regression proof). Turning tagging OFF also clears every entry back to
    /// [`hdtag::TileTag::default`] — without this, a caller could otherwise observe stale tags
    /// from the last frame tagging was on, contradicting [`Ppu::tile_tags`]'s own "every entry is
    /// default unless tagging was on while that pixel was rendered" guarantee. A host/frontend
    /// convenience switch, never part of `save_state`/`load_state`.
    #[cfg(feature = "hd-pack")]
    pub fn set_hd_pack_tagging(&mut self, enabled: bool) {
        self.hd_pack_tagging = enabled;
        if !enabled {
            self.tile_tags.fill(hdtag::TileTag::default());
        }
    }

    /// Whether [`hdtag::TileTag`] recording is currently enabled.
    #[cfg(feature = "hd-pack")]
    #[must_use]
    pub const fn hd_pack_tagging(&self) -> bool {
        self.hd_pack_tagging
    }

    /// The tile-identity side-buffer for the frame currently composited — same length/indexing
    /// as [`Ppu::framebuffer`]. Every entry is [`hdtag::TileTag::default`] (hash `0`) unless
    /// [`Ppu::set_hd_pack_tagging`] was on while that pixel was rendered.
    #[cfg(feature = "hd-pack")]
    #[must_use]
    pub fn tile_tags(&self) -> &[hdtag::TileTag] {
        &self.tile_tags[..self.visible_width() * SCREEN_HEIGHT]
    }

    // --- Storage accessors (for tests / save-state plumbing) ---

    /// Read a VRAM word directly (test/diagnostic; not the register path).
    #[must_use]
    pub fn vram_word(&self, addr: u16) -> u16 {
        self.vram[(addr & 0x7fff) as usize]
    }

    /// The full 64 KiB VRAM as a flat word slice (32Ki x `u16`, native word addressing) — for a
    /// host embedder that needs a raw memory-map pointer (e.g. a libretro core's
    /// `RETRO_MEMORY_VIDEO_RAM`).
    #[must_use]
    pub fn vram(&self) -> &[u16] {
        &*self.vram
    }

    /// The mutable counterpart to [`Self::vram`] — same host-embedder use case.
    pub fn vram_mut(&mut self) -> &mut [u16] {
        &mut *self.vram
    }

    /// Read a CGRAM entry directly (test/diagnostic).
    #[must_use]
    pub const fn cgram_word(&self, index: u8) -> u16 {
        self.cgram[index as usize]
    }

    /// Read an OAM byte directly (test/diagnostic).
    #[must_use]
    pub const fn oam_byte(&self, index: u16) -> u8 {
        self.oam[(index as usize) % 544]
    }

    /// Write VRAM/CGRAM/OAM, the full register file, the write latches, the dot/scanline
    /// timeline, the interrupt/frame poll state, and the composited framebuffer into a `"PPU0"`
    /// section. There is no firmware/ROM byte here to exclude — the PPU carries no chip-ROM
    /// dump (`docs/adr/0003`); `region` is written too since it's set by cart detection, not
    /// re-derivable from anything else stored here.
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"PPU0", |s| {
            for &word in self.vram.iter() {
                s.write_u16(word);
            }
            for &word in &self.cgram {
                s.write_u16(word);
            }
            s.write_bytes(&self.oam);
            self.io.save_state(s);
            s.write_u8(match self.region {
                Region::Ntsc => 0,
                Region::Pal => 1,
            });
            s.write_u16(self.mode7_byte_latch);
            s.write_u16(self.bgofs_prev1);
            s.write_u16(self.bgofs_prev2);
            s.write_u16(self.h);
            s.write_u16(self.v);
            s.write_bool(self.field);
            s.write_bool(self.vblank);
            s.write_bool(self.hblank);
            s.write_bool(self.nmi_pending);
            s.write_bool(self.irq_pending);
            s.write_bool(self.irq_enable_h);
            s.write_bool(self.irq_enable_v);
            s.write_u16(self.irq_h);
            s.write_u16(self.irq_v);
            s.write_bool(self.frame_ready);
            s.write_u64(self.frame_count);
            s.write_bool(self.frame_hires);
            for &word in self.framebuffer.iter() {
                s.write_u16(word);
            }
        });
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input, a section with unconsumed trailing bytes,
    /// or [`SaveStateError::Invalid`] if the encoded `region` discriminant doesn't match one of
    /// [`Region`]'s two variants (a semantic enum constraint, not a hardware register width).
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"PPU0")?;
        for word in self.vram.iter_mut() {
            *word = s.read_u16()?;
        }
        for word in &mut self.cgram {
            *word = s.read_u16()? & 0x7FFF;
        }
        self.oam.copy_from_slice(s.read_bytes(544)?);
        self.io.load_state(&mut s)?;
        let region = s.read_u8()?;
        self.region = match region {
            0 => Region::Ntsc,
            1 => Region::Pal,
            _ => {
                return Err(SaveStateError::Invalid(alloc::format!(
                    "Ppu region discriminant {region} is not a valid Region variant (0-1)"
                )));
            }
        };
        self.mode7_byte_latch = s.read_u16()?;
        self.bgofs_prev1 = s.read_u16()?;
        self.bgofs_prev2 = s.read_u16()?;
        self.h = s.read_u16()?;
        self.v = s.read_u16()?;
        self.field = s.read_bool()?;
        self.vblank = s.read_bool()?;
        self.hblank = s.read_bool()?;
        self.nmi_pending = s.read_bool()?;
        self.irq_pending = s.read_bool()?;
        self.irq_enable_h = s.read_bool()?;
        self.irq_enable_v = s.read_bool()?;
        self.irq_h = s.read_u16()?;
        self.irq_v = s.read_u16()?;
        self.frame_ready = s.read_bool()?;
        self.frame_count = s.read_u64()?;
        self.frame_hires = s.read_bool()?;
        for word in self.framebuffer.iter_mut() {
            *word = s.read_u16()? & 0x7FFF;
        }
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "PPU0 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::NullVideoBus;

    #[test]
    fn constructs() {
        let p = Ppu::new();
        assert_eq!(p.framebuffer().len(), FRAMEBUFFER_LEN);
        assert_eq!(p.region, Region::Ntsc);
    }

    #[test]
    fn full_state_round_trips_through_save_state() {
        let mut p = Ppu::with_region(Region::Pal);
        p.io.bg_mode = 7;
        p.io.vram_address = 0x1234;
        p.vram[0x100] = 0xBEEF;
        p.cgram[10] = 0x7FFF;
        p.oam[5] = 0x42;
        p.h = 200;
        p.v = 150;
        p.frame_count = 99;

        let mut w = SaveWriter::new();
        p.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = Ppu::new();
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.region, Region::Pal);
        assert_eq!(fresh.io.bg_mode, 7);
        assert_eq!(fresh.io.vram_address, 0x1234);
        assert_eq!(fresh.vram[0x100], 0xBEEF);
        assert_eq!(fresh.cgram[10], 0x7FFF);
        assert_eq!(fresh.oam[5], 0x42);
        assert_eq!(fresh.h, 200);
        assert_eq!(fresh.v, 150);
        assert_eq!(fresh.frame_count, 99);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn out_of_range_region_discriminant_is_rejected_not_panicked_on() {
        let p = Ppu::new();
        let mut w = SaveWriter::new();
        p.save_state(&mut w);
        let mut bytes = w.into_bytes();

        // The region byte follows VRAM + CGRAM + OAM + the whole Io section. Skip past those by
        // replaying load_state's own field order (rather than hardcoding a byte offset), so this
        // stays correct if a field is added/removed above it.
        let mut r = SaveReader::new(&bytes);
        let mut s = r.expect_section(*b"PPU0").unwrap();
        for _ in 0..0x8000 {
            s.read_u16().unwrap(); // vram
        }
        for _ in 0..256 {
            s.read_u16().unwrap(); // cgram
        }
        s.read_bytes(544).unwrap(); // oam
        let mut io_scratch = Io::default();
        io_scratch.load_state(&mut s).unwrap(); // consumes exactly one Io's worth of bytes

        let offset = bytes.len() - s.remaining();
        bytes[offset] = 99;

        let mut fresh = Ppu::new();
        let mut r2 = SaveReader::new(&bytes);
        assert!(matches!(
            fresh.load_state(&mut r2),
            Err(SaveStateError::Invalid(_))
        ));
    }

    #[test]
    fn counter_wraps_at_341_and_262() {
        let mut p = Ppu::new();
        let mut bus = NullVideoBus;
        // One full NTSC frame = 341 * 262 dots.
        let total = u32::from(DOTS_PER_LINE) * u32::from(Region::Ntsc.lines_per_frame());
        for _ in 0..total {
            p.tick_dot(&mut bus);
        }
        assert_eq!(p.h, 0);
        assert_eq!(p.v, 0);
        assert_eq!(p.frame_count, 1);
    }

    #[test]
    fn vblank_asserts_at_225() {
        let mut p = Ppu::new();
        let mut bus = NullVideoBus;
        // Tick until V reaches 225.
        while p.v != 225 {
            p.tick_dot(&mut bus);
        }
        assert!(p.vblank);
        assert!(p.nmi_pending);
    }

    #[test]
    fn vblank_at_240_with_overscan() {
        let mut p = Ppu::new();
        let mut bus = NullVideoBus;
        // Enable overscan via SETINI.
        p.write_reg(0x2133, 0x04);
        while p.v != 240 {
            p.tick_dot(&mut bus);
        }
        assert!(p.vblank);
    }

    #[test]
    fn hv_irq_fires_at_programmed_position() {
        let mut p = Ppu::new();
        let mut bus = NullVideoBus;
        p.set_hv_irq(true, true, 100, 50);
        let mut fired_at = None;
        for _ in 0..(u32::from(DOTS_PER_LINE) * 60) {
            p.tick_dot(&mut bus);
            if p.irq_pending() && fired_at.is_none() {
                fired_at = Some((p.dot(), p.scanline()));
                p.ack_irq();
            }
        }
        assert!(fired_at.is_some());
        let (h, v) = fired_at.unwrap();
        // The H comparator lags HTIME by `HIRQ_TRIGGER_DELAY` dots (hardware counter→IRQ
        // communication delay; see `check_hv_irq`), and it is evaluated at the start of tick
        // before H increments, so the IRQ is observed at dot `HTIME + HIRQ_TRIGGER_DELAY` (or the
        // dot after) on the programmed scanline.
        assert_eq!(v, 50);
        let target = 100 + HIRQ_TRIGGER_DELAY;
        assert!(h == target || h == target + 1);
    }

    #[test]
    fn frame_ready_toggles_once_per_frame() {
        let mut p = Ppu::new();
        let mut bus = NullVideoBus;
        let total = u32::from(DOTS_PER_LINE) * u32::from(Region::Ntsc.lines_per_frame());
        for _ in 0..total {
            p.tick_dot(&mut bus);
        }
        assert!(p.frame_ready());
        let _ = p.take_frame();
        assert!(!p.frame_ready());
    }

    #[test]
    fn pal_has_312_lines() {
        let p = Ppu::with_region(Region::Pal);
        assert_eq!(p.region.lines_per_frame(), 312);
    }

    #[test]
    fn clone_is_independent() {
        let mut p = Ppu::new();
        // VMAIN increment-on-high so L then H both land at the same word address.
        p.write_reg(0x2115, 0x80);
        p.write_reg(0x2116, 0x00);
        p.write_reg(0x2117, 0x00);
        p.write_reg(0x2118, 0x34);
        p.write_reg(0x2119, 0x12);
        let q = p.clone();
        assert_eq!(q.vram_word(0), 0x1234);
    }

    #[test]
    fn vram_and_vram_mut_expose_the_same_flat_64kib() {
        let mut p = Ppu::new();
        assert_eq!(p.vram().len(), 0x8000);
        p.write_reg(0x2115, 0x80);
        p.write_reg(0x2116, 0x00);
        p.write_reg(0x2117, 0x00);
        p.write_reg(0x2118, 0x34);
        p.write_reg(0x2119, 0x12);
        assert_eq!(p.vram()[0], 0x1234);
        p.vram_mut()[1] = 0xABCD;
        assert_eq!(p.vram_word(1), 0xABCD);
    }
}
