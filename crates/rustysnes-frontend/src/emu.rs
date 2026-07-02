//! The emulation core wrapper the frontend drives: load a ROM, step a frame, expose the
//! framebuffer + audio for the present path.
//!
//! [`EmuCore`] owns the `rustysnes-core` [`System`] (the master-clock lockstep scheduler) and the
//! frontend-side derived state the present path consumes under a brief lock: an RGBA8 framebuffer
//! (decoded from the PPU's 15-bit BGR555 output) and the per-frame audio FIFO drained from the
//! S-DSP. The determinism contract lives in the core; the frontend NEVER injects timing/RNG into
//! synthesis — rate control + run-ahead are pure frontend concerns (in `app.rs` / `audio.rs`).

use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use rustysnes_core::cart::header::Coprocessor;

use crate::config::Region;
use crate::gfx::{MAX_H, MAX_W, SNES_W, bgr555_to_rgba8};
use crate::input::Buttons;

/// Coprocessor firmware dumps the frontend will try, in order, for a cart that carries a
/// chip-ROM-dump coprocessor. The matching dump (when the user has supplied it) is the only one
/// [`Cart::install_coprocessor_firmware`] accepts; the rest are rejected and left unchanged.
const fn firmware_candidates(co: Coprocessor) -> &'static [&'static str] {
    match co {
        // The µPD77C25 DSP-1..4 family — the right dump depends on the game; try the common ones.
        Coprocessor::Dsp => &["dsp1.rom", "dsp1b.rom", "dsp2.rom", "dsp3.rom", "dsp4.rom"],
        Coprocessor::Cx4 => &["cx4.rom"],
        // Logic-only / on-die coprocessors (Super FX, SA-1, S-DD1, SPC7110, OBC1) need no external
        // firmware dump in this core.
        _ => &[],
    }
}

/// The frontend's view of the emulator. Lives behind an `Arc<Mutex<…>>` on native (shared with
/// the dedicated emulation thread) and is stepped synchronously on wasm.
pub struct EmuCore {
    /// The master-clock scheduler + Bus (owns every chip).
    system: System,
    /// The current console region (drives the active framebuffer height + pacing).
    region: Region,
    /// The frontend-side RGBA8 framebuffer (sized to the hi-res worst case; the active sub-rect is
    /// `fb_dims` large). Copied out under one brief lock by the present path.
    framebuffer: Vec<u8>,
    /// The active framebuffer dims `(w, h)` for the current video mode.
    fb_dims: (u32, u32),
    /// The 32 kHz stereo samples the S-DSP emitted during the most recent [`Self::run_frame`].
    audio: Vec<(i16, i16)>,
    /// The latest latched controller state for P1 / P2 (late-latched by the window handler).
    pads: [Buttons; 2],
    /// Whether a ROM is currently loaded (the present path shows a blank frame otherwise).
    rom_loaded: bool,
    /// The raw ROM image, retained so Power-Cycle can rebuild a clean machine deterministically.
    rom: Vec<u8>,
    /// The coprocessor firmware dump installed for this cart (if any), retained for Power-Cycle.
    firmware: Vec<u8>,
}

impl EmuCore {
    /// Power on with a determinism seed and a region. No ROM is loaded yet.
    #[must_use]
    pub fn new(seed: u64, region: Region) -> Self {
        Self {
            system: System::new(seed),
            region,
            framebuffer: vec![0u8; (MAX_W * MAX_H * 4) as usize],
            fb_dims: (SNES_W, region.active_height()),
            audio: Vec::new(),
            pads: [Buttons::default(); 2],
            rom_loaded: false,
            rom: Vec::new(),
            firmware: Vec::new(),
        }
    }

    /// Load a raw ROM image, transparently unwrapping a zip archive first if `rom` is one (see
    /// `extract_rom_bytes` — the common case of a `.sfc`/`.smc` distributed zipped). Header
    /// detection + board selection happen in `rustysnes-cart`. On success the cart is installed
    /// in a fresh Bus and the system is left ready to boot from the cart's reset vector on the
    /// first [`Self::run_frame`].
    ///
    /// # Errors
    /// Returns an [`EmuError`] if the image is empty, is a zip archive that can't be opened or
    /// contains no recognizable ROM entry, or no valid SNES header is found.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), EmuError> {
        if rom.is_empty() {
            return Err(EmuError::Empty);
        }
        let rom = extract_rom_bytes(rom)?;
        if rom.is_empty() {
            return Err(EmuError::Empty);
        }
        let cart = Cart::from_rom(&rom).map_err(|e| EmuError::Header(format!("{e:?}")))?;
        let mut system = System::new(0);
        system.bus.cart = Some(cart);
        self.system = system;
        self.rom = rom.into_owned();
        self.firmware.clear();
        self.rom_loaded = true;
        self.audio.clear();
        Ok(())
    }

    /// The coprocessor firmware dumps this cart will accept (filenames), in try order. Empty when
    /// the cart needs no external firmware. The UI uses this to locate + install the dump.
    #[must_use]
    pub fn firmware_candidates(&self) -> &'static [&'static str] {
        self.system
            .bus
            .cart
            .as_ref()
            .map_or(&[], |c| firmware_candidates(c.header.coprocessor))
    }

    /// Whether the loaded cart carries a coprocessor that needs a (not-yet-installed) firmware dump
    /// to function. The honesty posture of `docs/adr/0003`: without the dump the coprocessor is
    /// non-functional, so the UI should prompt for it.
    #[must_use]
    pub fn needs_firmware(&self) -> bool {
        !self.firmware_candidates().is_empty() && self.firmware.is_empty()
    }

    /// Supply a coprocessor firmware dump. Returns `true` if the cart's board accepted it (right
    /// coprocessor + size); on success the bytes are retained for Power-Cycle.
    pub fn install_firmware(&mut self, bytes: &[u8]) -> bool {
        let accepted = self
            .system
            .bus
            .cart
            .as_mut()
            .is_some_and(|c| c.install_coprocessor_firmware(bytes));
        if accepted {
            self.firmware = bytes.to_vec();
        }
        accepted
    }

    /// Restore battery SRAM from a `.srm` image (truncated/zero-padded to the board's SRAM size).
    pub fn load_sram(&mut self, data: &[u8]) {
        if let Some(c) = self.system.bus.cart.as_mut() {
            c.load_sram(data);
        }
    }

    /// The current battery SRAM contents (empty when the cart has no SRAM), for a `.srm` save.
    #[must_use]
    pub fn save_sram(&self) -> &[u8] {
        self.system.bus.cart.as_ref().map_or(&[], Cart::save_sram)
    }

    /// Soft reset: re-run the cart's reset vector without clearing RAM (the SNES front-panel
    /// Reset button). A no-op when no ROM is loaded.
    pub fn reset(&mut self) {
        if self.rom_loaded {
            self.system.reset();
            self.audio.clear();
        }
    }

    /// Power-cycle (hard reset): rebuild a clean machine from the retained ROM + firmware. Battery
    /// SRAM is NOT preserved here (the caller reloads `.srm` if desired).
    pub fn power_cycle(&mut self) {
        if !self.rom_loaded {
            return;
        }
        if let Ok(cart) = Cart::from_rom(&self.rom) {
            let mut system = System::new(0);
            system.bus.cart = Some(cart);
            if let Some(c) = system
                .bus
                .cart
                .as_mut()
                .filter(|_| !self.firmware.is_empty())
            {
                let _ = c.install_coprocessor_firmware(&self.firmware);
            }
            self.system = system;
            self.audio.clear();
        }
    }

    /// Close the loaded ROM and present a clean blank frame.
    pub fn close_rom(&mut self) {
        self.system = System::new(0);
        self.rom.clear();
        self.firmware.clear();
        self.rom_loaded = false;
        self.audio.clear();
        self.framebuffer.iter_mut().for_each(|b| *b = 0);
        self.fb_dims = (SNES_W, self.region.active_height());
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
    /// handler each frame; applied to the Bus at the top of [`Self::run_frame`].
    pub fn set_pad(&mut self, player: usize, buttons: Buttons) {
        if let Some(slot) = self.pads.get_mut(player) {
            *slot = buttons.sanitize_dpad();
        }
    }

    /// Advance one full video frame: feed the latched pads to the Bus, run the scheduler to the
    /// next frame boundary, then decode the PPU framebuffer + drain the S-DSP audio.
    pub fn run_frame(&mut self) {
        self.system.bus.set_joypad(0, self.pads[0].0);
        self.system.bus.set_joypad(1, self.pads[1].0);
        self.system.run_frame();
        self.audio.clear();
        if self.rom_loaded {
            self.system.bus.apu.drain_audio(&mut self.audio);
            self.render_framebuffer();
        }
    }

    /// Decode the PPU's 256×(224|239) BGR555 framebuffer into the RGBA8 present buffer.
    fn render_framebuffer(&mut self) {
        let h = u32::from(self.system.bus.ppu.visible_height()).min(crate::gfx::SNES_H_PAL);
        let w = SNES_W;
        self.fb_dims = (w, h);
        let count = (w * h) as usize;
        let src = self.system.bus.framebuffer();
        for (i, &px) in src.iter().take(count).enumerate() {
            let bytes = bgr555_to_rgba8(px).to_le_bytes();
            let o = i * 4;
            self.framebuffer[o..o + 4].copy_from_slice(&bytes);
        }
    }

    /// The current RGBA8 framebuffer slice (the active mode's `w*h*4` bytes), for the present path.
    #[must_use]
    pub fn framebuffer(&self) -> &[u8] {
        let (w, h) = self.fb_dims;
        let len = (w * h * 4) as usize;
        &self.framebuffer[..len.min(self.framebuffer.len())]
    }

    /// The audio samples (32 kHz stereo) produced during the most recent [`Self::run_frame`].
    #[must_use]
    pub fn audio(&self) -> &[(i16, i16)] {
        &self.audio
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
    /// The image looked like a zip archive but couldn't be opened, or contained no recognizable
    /// SNES ROM entry.
    #[error("zip archive: {0}")]
    Archive(String),
}

/// SNES ROM file extensions recognized inside a zip archive, checked case-insensitively
/// (`.sfc`/`.smc` are by far the most common; `.fig`/`.swc` are older copier-header dumps this
/// project's header detection already strips — see `docs/cartridge-format.md`).
const ROM_EXTENSIONS: [&str; 4] = ["sfc", "smc", "fig", "swc"];

/// Hard cap on a zip entry's decompressed size, enforced while reading (not just checked against
/// the (attacker-controlled, spoofable) declared size up front). The largest official SNES ROM is
/// 6 MiB and the largest known fan hack is ~12 MiB; 32 MiB leaves generous headroom while still
/// bounding a "zip bomb" (a small archive that claims/produces a huge decompressed stream) to a
/// sane memory ceiling instead of unbounded growth.
const MAX_DECOMPRESSED_ROM_SIZE: u64 = 32 * 1024 * 1024;

/// If `bytes` is a zip archive (sniffed by the local-file-header magic `PK\x03\x04`, or the
/// empty-archive end-of-central-directory magic `PK\x05\x06`), extract the first non-directory
/// entry whose extension is in [`ROM_EXTENSIONS`] and return its decompressed bytes. Otherwise
/// returns `bytes` unchanged — a plain `.sfc`/`.smc` file passes straight through. Pure in-memory
/// (a `Cursor` over the slice already in hand), so this is identical on native and wasm32.
fn extract_rom_bytes(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>, EmuError> {
    let is_zip = bytes.len() >= 4 && (bytes[..4] == *b"PK\x03\x04" || bytes[..4] == *b"PK\x05\x06");
    if !is_zip {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| EmuError::Archive(format!("{e}")))?;
    let rom_index = (0..archive.len())
        .find(|&i| {
            archive.name_for_index(i).is_some_and(|name| {
                // Directory entries conventionally end with `/` (zip spec) — a directory named
                // e.g. `Game.sfc/` must not match, or extraction below fails on an empty read.
                !name.ends_with('/')
                    && std::path::Path::new(name)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| {
                            ROM_EXTENSIONS.iter().any(|r| ext.eq_ignore_ascii_case(r))
                        })
            })
        })
        .ok_or_else(|| {
            EmuError::Archive(format!(
                "no .sfc/.smc/.fig/.swc entry found (tried {} entries)",
                archive.len()
            ))
        })?;
    let mut entry = archive
        .by_index(rom_index)
        .map_err(|e| EmuError::Archive(format!("{e}")))?;
    // `read_to_end` grows the buffer as needed; no need to pre-size from `entry.size()` (a
    // `u64` that would need a lossy cast on 32-bit targets for a capacity hint only) — and that
    // declared size is attacker-controlled anyway, which is exactly what `take` below guards
    // against: capping the ACTUAL bytes read, not trusting the header's claim.
    let mut limited = std::io::Read::take(&mut entry, MAX_DECOMPRESSED_ROM_SIZE + 1);
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut limited, &mut out)
        .map_err(|e| EmuError::Archive(format!("{e}")))?;
    if out.len() as u64 > MAX_DECOMPRESSED_ROM_SIZE {
        return Err(EmuError::Archive(format!(
            "entry exceeds the {MAX_DECOMPRESSED_ROM_SIZE}-byte decompressed-size limit \
             (zip bomb protection)"
        )));
    }
    Ok(std::borrow::Cow::Owned(out))
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
        assert!(core.audio().is_empty());
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

    fn zip_containing(name: &str, bytes: &[u8]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buf);
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
        writer.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn non_zip_bytes_pass_through_unchanged() {
        let rom = b"not a zip, just a plain ROM image";
        let out = extract_rom_bytes(rom).unwrap();
        assert_eq!(&*out, rom);
    }

    #[test]
    fn zip_wrapped_rom_is_transparently_extracted() {
        let rom = vec![0xAB_u8; 512];
        let zipped = zip_containing("Game.sfc", &rom);
        let out = extract_rom_bytes(&zipped).unwrap();
        assert_eq!(&*out, rom.as_slice());
    }

    #[test]
    fn zip_with_no_rom_entry_errors() {
        let zipped = zip_containing("readme.txt", b"not a ROM");
        assert!(matches!(
            extract_rom_bytes(&zipped),
            Err(EmuError::Archive(_))
        ));
    }

    #[test]
    fn zip_directory_entry_named_like_a_rom_is_not_matched() {
        // A directory entry conventionally ends with `/` in the zip central directory; a folder
        // literally named "Game.sfc" must not be picked over (or instead of) a real ROM entry.
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buf);
        writer
            .add_directory("Game.sfc/", zip::write::SimpleFileOptions::default())
            .unwrap();
        let rom = vec![0xCD_u8; 128];
        writer
            .start_file("Real Game.sfc", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, &rom).unwrap();
        writer.finish().unwrap();
        let zipped = buf.into_inner();
        let out = extract_rom_bytes(&zipped).unwrap();
        assert_eq!(&*out, rom.as_slice());
    }

    #[test]
    fn oversized_zip_entry_is_rejected_not_read_unbounded() {
        let huge = vec![0u8; usize::try_from(MAX_DECOMPRESSED_ROM_SIZE + 1).unwrap()];
        let zipped = zip_containing("Big.sfc", &huge);
        assert!(matches!(
            extract_rom_bytes(&zipped),
            Err(EmuError::Archive(_))
        ));
    }

    #[test]
    fn zip_wrapped_rom_loads_end_to_end() {
        // A minimal-but-valid LoROM header at $7FC0 so `Cart::from_rom` accepts it — mirrors
        // the header layout `rustysnes-cart::header` scores (title + map-mode + checksum bytes
        // are permissive; only the size/offset need to line up for LoROM detection to win).
        let mut rom = vec![0u8; 0x8000];
        rom[0x7FC0..0x7FC0 + 21].copy_from_slice(b"TEST ROM             ");
        rom[0x7FD5] = 0x20; // LoROM
        rom[0x7FD6] = 0x00; // no coprocessor
        rom[0x7FD7] = 0x08; // ROM size (2^8 KiB = 256 KiB, permissive)
        let zipped = zip_containing("test.sfc", &rom);
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&zipped).expect("zip-wrapped ROM should load");
        assert!(core.rom_loaded());
    }
}
