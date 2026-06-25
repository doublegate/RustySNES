//! The emulation core wrapper the frontend drives: load a ROM, step a frame, expose the
//! framebuffer for the present path.
//!
//! [`EmuCore`] owns the `rustysnes-core` [`System`] (the master-clock lockstep scheduler) and a
//! frontend-side RGBA8 framebuffer that the present path copies under a brief lock. The
//! determinism contract lives in the core; the frontend NEVER injects timing/RNG into
//! synthesis — rate control + run-ahead are pure frontend concerns (in `app.rs` / the audio
//! resampler).
//!
//! v0.1.0: the chips are skeletons, so [`EmuCore::run_frame`] advances the scheduler but the
//! PPU produces no pixels yet — [`EmuCore::framebuffer`] returns a deterministically cleared
//! frame. The wiring (load → step → present) is real; only the pixel content is pending the PPU.

use rustysnes_core::System;
use rustysnes_core::cart::Cart;

use crate::config::Region;
use crate::gfx::{MAX_H, MAX_W};
use crate::input::Buttons;

/// The frontend's view of the emulator. Lives behind an `Arc<Mutex<…>>` on native (shared with
/// the dedicated emulation thread) and is stepped synchronously on wasm.
pub struct EmuCore {
    /// The master-clock scheduler + Bus (owns every chip).
    system: System,
    /// The current console region (drives the active framebuffer height).
    region: Region,
    /// The frontend-side RGBA8 framebuffer (sized to the hi-res worst case; the active sub-rect
    /// is `region.active_height()` rows tall in lo-res). Lives here so the present path copies
    /// it out under one brief lock without touching the core's internals.
    framebuffer: Vec<u8>,
    /// The active framebuffer dims `(w, h)` for the current video mode.
    fb_dims: (u32, u32),
    /// The latest latched controller state for P1 / P2 (late-latched by the window handler).
    pads: [Buttons; 2],
    /// Whether a ROM is currently loaded (the present path shows a blank frame otherwise).
    rom_loaded: bool,
}

impl EmuCore {
    /// Power on with a determinism seed and a region. No ROM is loaded yet.
    #[must_use]
    pub fn new(seed: u64, region: Region) -> Self {
        let fb_dims = (crate::gfx::SNES_W, region.active_height());
        Self {
            system: System::new(seed),
            region,
            framebuffer: vec![0u8; (MAX_W * MAX_H * 4) as usize],
            fb_dims,
            pads: [Buttons::default(); 2],
            rom_loaded: false,
        }
    }

    /// Load a raw ROM image (header detection + board selection happen in `rustysnes-cart`).
    /// On success the cart is installed in the Bus and the system is reset to a clean power-on.
    ///
    /// # Errors
    /// Returns an [`EmuError`] if the image is empty or no valid SNES header is found.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), EmuError> {
        if rom.is_empty() {
            return Err(EmuError::Empty);
        }
        let cart = Cart::from_rom(rom).map_err(|e| EmuError::Header(format!("{e:?}")))?;
        // Fresh power-on with the cart installed (a real reset-vector fetch lands with the CPU
        // model; the skeleton just attaches the cart so the Bus routes accesses to it).
        let mut system = System::new(0);
        system.bus.cart = Some(cart);
        self.system = system;
        self.rom_loaded = true;
        Ok(())
    }

    /// Close the loaded ROM and present a clean blank frame (the RustyNES ROM-close behavior).
    pub fn close_rom(&mut self) {
        self.system = System::new(0);
        self.rom_loaded = false;
        self.framebuffer.iter_mut().for_each(|b| *b = 0);
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub const fn rom_loaded(&self) -> bool {
        self.rom_loaded
    }

    /// The loaded cartridge's board name (for the status bar / title), if any.
    #[must_use]
    pub fn cart_name(&self) -> Option<&'static str> {
        self.system.bus.cart.as_ref().map(|c| c.board.name())
    }

    /// Latch the controller state for a player (`0` = P1, `1` = P2). Late-latched by the window
    /// handler each frame; the core reads it at the auto-joypad point (TODO when the CPU model
    /// drives `$4016/$4218`).
    pub fn set_pad(&mut self, player: usize, buttons: Buttons) {
        if let Some(slot) = self.pads.get_mut(player) {
            *slot = buttons.sanitize_dpad();
        }
    }

    /// Advance one video frame (run the scheduler to the next vblank boundary).
    ///
    /// v0.1.0: the chips are skeletons so this is a bounded tick budget — `run_frame` loops the
    /// real boundary detection once the PPU signals end-of-frame. The framebuffer is left
    /// cleared (no PPU output yet).
    pub fn run_frame(&mut self) {
        // TODO(impl-phase): the core's `System::run_frame` will loop `tick_one_master` until the
        // PPU signals vblank; for the skeleton we call it directly so the wiring is exercised.
        self.system.run_frame();
        // TODO(impl-phase): copy the PPU's BGR555 frame through `bgr555_to_rgba8` into
        // `framebuffer`, and update `fb_dims` for the live hi-res/interlace mode.
    }

    /// The current RGBA8 framebuffer slice (the active mode's `w*h*4` bytes), for the present
    /// path. The caller copies this under the brief emu lock, then drops the lock before
    /// rendering.
    #[must_use]
    pub fn framebuffer(&self) -> &[u8] {
        let (w, h) = self.fb_dims;
        let len = (w * h * 4) as usize;
        &self.framebuffer[..len.min(self.framebuffer.len())]
    }

    /// The active framebuffer dimensions `(w, h)`.
    #[must_use]
    pub const fn fb_dims(&self) -> (u32, u32) {
        self.fb_dims
    }

    /// The active region.
    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }
}

/// ROM-load / emulation errors surfaced to the UI.
#[derive(Debug, thiserror::Error)]
pub enum EmuError {
    /// The ROM image was empty.
    #[error("empty ROM image")]
    Empty,
    /// No valid SNES header was found (LoROM/HiROM/ExHiROM detection failed).
    #[error("invalid SNES ROM header: {0}")]
    Header(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_core_presents_cleared_frame_of_region_size() {
        let core = EmuCore::new(0, Region::Ntsc);
        let (w, h) = core.fb_dims();
        assert_eq!((w, h), (256, 224));
        assert_eq!(core.framebuffer().len(), (256 * 224 * 4) as usize);
        assert!(core.framebuffer().iter().all(|&b| b == 0));
        assert!(!core.rom_loaded());
    }

    #[test]
    fn empty_rom_rejected() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        assert!(matches!(core.load_rom(&[]), Err(EmuError::Empty)));
    }

    #[test]
    fn run_frame_does_not_panic_without_rom() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.run_frame();
    }
}
