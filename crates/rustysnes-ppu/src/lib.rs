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
//! H runs 0..=339 (340 wraps to a new line), V runs 0..=261 (NTSC) / 0..=311 (PAL). Active
//! output is dots 22..=277 on lines 1..=224 (1..=239 overscan); `VBlank` asserts at V=225
//! (V=240 overscan). The renderer is per-scanline (it composites a whole visible line at the
//! line's end), which is bit-identical in the final framebuffer to a per-dot renderer and far
//! simpler — the determinism contract only requires the *final frame* be reproducible.
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

pub mod bus;
mod regs;
mod render;

pub use bus::VideoBus;

/// Master clocks per PPU dot (nominal). The scheduler owns the long-dot remainder; this crate
/// just counts dots. See `docs/scheduler.md` "Convention (binding)".
pub const MASTER_CLOCKS_PER_DOT: u32 = 4;

/// Dots per scanline (the RustySNES convention: 341 dots of nominally 4 master clocks).
pub const DOTS_PER_LINE: u16 = 341;

/// Visible width of one rendered scanline, in pixels.
pub const SCREEN_WIDTH: usize = 256;

/// Maximum visible height (overscan). Standard frames fill the first 224 rows.
pub const SCREEN_HEIGHT: usize = 239;

/// First active output dot on a visible line.
const ACTIVE_DOT_START: u16 = 22;

/// Total framebuffer length, in 15-bit BGR pixels.
pub const FRAMEBUFFER_LEN: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

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

    /// The composited 256×239 15-bit BGR framebuffer.
    framebuffer: Box<[u16; FRAMEBUFFER_LEN]>,
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
            framebuffer: Box::new([0u16; FRAMEBUFFER_LEN]),
        }
    }

    /// Set the video region (NTSC/PAL). Affects line count + the STAT78 region bit.
    pub const fn set_region(&mut self, region: Region) {
        self.region = region;
    }

    /// The total visible height for the current overscan setting (224 or 239).
    #[must_use]
    pub const fn visible_height(&self) -> u16 {
        if self.io.overscan { 239 } else { 224 }
    }

    /// First `VBlank` scanline for the current overscan setting (225 or 240).
    #[must_use]
    const fn vblank_line(&self) -> u16 {
        if self.io.overscan { 240 } else { 225 }
    }

    /// Advance the PPU by exactly one dot, the scheduler's video quantum. Composites a full
    /// visible scanline when that line completes; raises NMI at `VBlank` start and the HV-IRQ
    /// when the programmed H/V is hit; calls [`VideoBus::notify_scanline`] at each line start
    /// and [`VideoBus::notify_vblank`] when `VBlank` begins. Hot path: allocation-free.
    pub fn tick_dot(&mut self, bus: &mut impl VideoBus) {
        // HBlank region: dots 274..=340 (active output ends near dot 274).
        self.hblank = self.h >= 274 || self.h < ACTIVE_DOT_START;

        // HV-IRQ comparator (level): fire when the enabled H and/or V positions match.
        self.check_hv_irq();

        self.h += 1;
        if self.h >= DOTS_PER_LINE {
            // End of a scanline: composite the line we just finished (if visible), then advance V.
            self.h = 0;
            self.end_of_scanline(bus);
        }
    }

    /// At the end of a scanline: render it if visible, advance V, and fire frame/VBlank events.
    fn end_of_scanline(&mut self, bus: &mut impl VideoBus) {
        // Render the line that just ended (V is the line number 1..=visible).
        if self.v >= 1 && self.v <= self.visible_height() {
            self.render_scanline(bus);
        }

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
            bus.notify_vblank();
        } else if self.v == 0 {
            self.vblank = false;
        }
    }

    /// Level-evaluate the HV-IRQ comparator at the current (h, v).
    const fn check_hv_irq(&mut self) {
        let h_match = !self.irq_enable_h || self.h == self.irq_h;
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

    /// Whether a finished frame is available. Cleared by [`Ppu::take_frame`].
    #[must_use]
    pub const fn frame_ready(&self) -> bool {
        self.frame_ready
    }

    /// Completed-frame count since power-on (monotonic, wrapping).
    #[must_use]
    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// The composited framebuffer: `SCREEN_WIDTH * SCREEN_HEIGHT` 15-bit BGR pixels, row-major.
    /// Only the first [`Ppu::visible_height`] rows are written each frame.
    #[must_use]
    pub fn framebuffer(&self) -> &[u16] {
        &self.framebuffer[..]
    }

    pub fn get_vram(&self) -> &[u16] {
        &self.vram[..]
    }

    /// Borrow the framebuffer and clear the `frame_ready` flag (one-shot presentation).
    pub fn take_frame(&mut self) -> &[u16] {
        self.frame_ready = false;
        &self.framebuffer[..]
    }

    // --- Storage accessors (for tests / save-state plumbing) ---

    /// Read a VRAM word directly (test/diagnostic; not the register path).
    #[must_use]
    pub fn vram_word(&self, addr: u16) -> u16 {
        self.vram[(addr & 0x7fff) as usize]
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
        // The IRQ is evaluated at the start of tick before H increments, so it fires the dot
        // after the match; assert we observed it on the programmed scanline.
        assert_eq!(v, 50);
        assert!(h == 100 || h == 101);
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
}
