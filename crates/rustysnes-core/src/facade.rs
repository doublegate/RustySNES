//! The pure emulation-core facade: load a ROM, step a frame, expose the framebuffer + audio.
//!
//! `std`-only (`#[cfg(feature = "std")]`) since it needs heap collections beyond `alloc` alone
//! (`zip` archive extraction) and is meant for a real host process — every consumer that embeds
//! this crate as a library (the native/wasm frontend, `rustysnes-libretro`, any future host) drives
//! the emulator through [`EmuCore`] rather than reaching into [`crate::scheduler::System`]
//! directly. Debugger-only concerns (breakpoints, single-step, register/VRAM/CGRAM/OAM snapshots)
//! deliberately stay OUTSIDE this facade — those are frontend-owned (built on
//! [`EmuCore::system_mut`]'s raw access), not something every consumer needs.
//!
//! `v1.2.0`: relocated here from `rustysnes-frontend::emu` (that crate's dependency weight —
//! winit/wgpu/cpal/egui — is wrong for a libretro core or any other headless embedding). The
//! frontend keeps a thin wrapper (`rustysnes-frontend::emu::EmuCore`) around this type that adds
//! the debugger fields; zero behavior change for existing frontend call sites.

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use rustysnes_cart::{Cart, Coprocessor, Region};

use crate::scheduler::System;

/// SNES native width (constant across the NTSC/PAL active-region heights and the lo-res mode).
pub const SNES_W: u32 = 256;
/// SNES NTSC active-region height (224 visible scanlines).
pub const SNES_H_NTSC: u32 = 224;
/// SNES PAL active-region height (239 visible scanlines).
pub const SNES_H_PAL: u32 = 239;
/// SNES hi-res / pseudo-hi-res width (mode 5/6 + interlace double the base dims).
pub const SNES_W_HIRES: u32 = 512;
/// SNES hi-res / interlace height (448 = 224 active * 2 fields).
pub const SNES_H_HIRES: u32 = 448;

/// The maximum framebuffer size a consumer's texture/surface needs to be sized for (hi-res worst
/// case), so a video-mode change never needs a realloc. Sub-modes occupy the top-left sub-rect.
pub const MAX_W: u32 = SNES_W_HIRES;
/// The maximum framebuffer height (see [`MAX_W`]).
pub const MAX_H: u32 = SNES_H_HIRES;

/// The active-region framebuffer height for a region (256 wide always).
#[must_use]
pub const fn active_height(region: Region) -> u32 {
    match region {
        Region::Ntsc => SNES_H_NTSC,
        Region::Pal => SNES_H_PAL,
    }
}

/// Expand a 15-bit SNES **BGR555** color word (`0bbbbbgggggrrrrr`) to a packed little-endian
/// RGBA8 (`0xAABBGGRR`) value suitable for an RGBA8 framebuffer / texture upload.
///
/// The 5-bit channels are left-justified to 8 bits (`c << 3 | c >> 2`), matching how Mesen2 /
/// bsnes expand CGRAM. Alpha is forced opaque.
#[must_use]
pub const fn bgr555_to_rgba8(bgr555: u16) -> u32 {
    let r5 = (bgr555 & 0x1F) as u32;
    let g5 = ((bgr555 >> 5) & 0x1F) as u32;
    let b5 = ((bgr555 >> 10) & 0x1F) as u32;
    let r8 = (r5 << 3) | (r5 >> 2);
    let g8 = (g5 << 3) | (g5 >> 2);
    let b8 = (b5 << 3) | (b5 >> 2);
    // Pack as 0xAABBGGRR (little-endian RGBA8 byte order: R,G,B,A).
    0xFF00_0000 | (b8 << 16) | (g8 << 8) | r8
}

/// Coprocessor firmware dumps a consumer should try, in order, for a cart that carries a
/// chip-ROM-dump coprocessor. The matching dump (when supplied) is the only one
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

/// The pure emulation-core facade: load a ROM, step a frame, expose the framebuffer + audio.
///
/// For the present path. Lives behind an `Arc<Mutex<…>>` when a consumer drives it from a
/// dedicated thread; stepped synchronously otherwise.
pub struct EmuCore {
    /// The master-clock scheduler + Bus (owns every chip).
    system: System,
    /// The current console region (drives the active framebuffer height + pacing).
    region: Region,
    /// The RGBA8 framebuffer (sized to the hi-res worst case; the active sub-rect is `fb_dims`
    /// large). Copied out under one brief lock by the present path.
    framebuffer: Vec<u8>,
    /// The active framebuffer dims `(w, h)` for the current video mode.
    fb_dims: (u32, u32),
    /// The 32 kHz stereo samples the S-DSP emitted during the most recent [`Self::run_frame`].
    audio: Vec<(i16, i16)>,
    /// The latest latched controller state for P1 / P2 (late-latched by the host).
    pads: [u16; 2],
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
            fb_dims: (SNES_W, active_height(region)),
            audio: Vec::new(),
            pads: [0, 0],
            rom_loaded: false,
            rom: Vec::new(),
            firmware: Vec::new(),
        }
    }

    /// Load a raw ROM image, transparently unwrapping a zip archive first if `rom` is one (the
    /// common case of a `.sfc`/`.smc` distributed zipped). Header detection + board selection
    /// happen in `rustysnes-cart`. On success the cart is installed
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
    /// the cart needs no external firmware. A host uses this to locate + install the dump.
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
    /// non-functional, so a host should prompt for it.
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
        self.fb_dims = (SNES_W, active_height(self.region));
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub const fn rom_loaded(&self) -> bool {
        self.rom_loaded
    }

    /// The loaded cartridge's board name (for a status display), if any.
    #[must_use]
    pub fn cart_name(&self) -> Option<&'static str> {
        self.system.bus.cart.as_ref().map(|c| c.board.name())
    }

    /// The raw ROM byte image currently loaded (empty if none) — for TAS movie recording's
    /// ROM-identity hash ([`crate::movie::hash_rom`]/[`crate::movie::Movie::verify_rom`]), which
    /// needs the exact bytes rather than `Cart`'s parsed/header-stripped internal representation.
    #[must_use]
    pub fn rom(&self) -> &[u8] {
        &self.rom
    }

    /// Direct mutable access to the deterministic core, for TAS movie record/playback
    /// ([`crate::movie`]), Lua scripting, or a debugger overlay — all need genuine read/write
    /// reach into the running [`System`]/`Bus`.
    pub const fn system_mut(&mut self) -> &mut System {
        &mut self.system
    }

    /// Latch the controller state for a player (`0` = P1, `1` = P2), applied to the Bus at the
    /// top of the next [`Self::run_frame`]/[`Self::apply_pads`]. Taken as a raw button-state
    /// word, same as [`crate::bus::Bus::set_joypad`] — a host is responsible for any input
    /// hygiene it wants (e.g. the frontend's `Buttons::sanitize_dpad` clearing illegal opposite
    /// D-pad directions before calling this), matching the honest "this facade stores what it's
    /// told" posture the rest of its setters already have.
    pub fn set_pad(&mut self, player: usize, buttons: u16) {
        if let Some(slot) = self.pads.get_mut(player) {
            *slot = buttons;
        }
    }

    /// Push the latched pad state to the Bus without running a frame — split out of
    /// [`Self::run_frame`] so a host that needs to interleave conditional stepping (a debugger's
    /// pause/breakpoint gate) can latch input once and then choose how to advance the `System`.
    pub fn apply_pads(&mut self) {
        self.system.bus.set_joypad(0, self.pads[0]);
        self.system.bus.set_joypad(1, self.pads[1]);
    }

    /// Select which peripheral is attached to controller port `port` — see
    /// [`crate::bus::Bus::set_port_device`].
    pub fn set_port_device(&mut self, port: usize, device: crate::controller::PortDevice) {
        self.system.bus.set_port_device(port, device);
    }

    /// Feed one frame's worth of SNES Mouse input for port `port` — see
    /// [`crate::bus::Bus::set_mouse`].
    pub fn set_mouse(&mut self, port: usize, dx: i32, dy: i32, left: bool, right: bool) {
        self.system.bus.set_mouse(port, dx, dy, left, right);
    }

    /// Set the 8 per-voice audio mute toggles — see [`crate::bus::Bus::set_voice_mutes`]'s doc.
    pub const fn set_voice_mutes(&mut self, mutes: [bool; 8]) {
        self.system.bus.set_voice_mutes(mutes);
    }

    /// Feed one frame's worth of Super Scope input for port `port` — see
    /// [`crate::bus::Bus::set_superscope`].
    pub fn set_superscope(&mut self, port: usize, x: i32, y: i32, buttons: u8) {
        self.system.bus.set_superscope(port, x, y, buttons);
    }

    /// Feed one frame's worth of input for Super Multitap sub-pad `sub_index` of port `port` —
    /// see [`crate::bus::Bus::set_multitap_pad`].
    pub fn set_multitap_pad(&mut self, port: usize, sub_index: usize, buttons: u16) {
        self.system.bus.set_multitap_pad(port, sub_index, buttons);
    }

    /// Advance one full video frame unconditionally: latch pads, run the scheduler to the next
    /// frame boundary, then decode the PPU framebuffer + drain the S-DSP audio. A host that needs
    /// conditional stepping (pause/breakpoints) should call [`Self::apply_pads`] +
    /// [`Self::system_mut`] + [`Self::present_current_frame`] directly instead.
    pub fn run_frame(&mut self) {
        self.apply_pads();
        self.system.run_frame();
        self.present_current_frame();
    }

    /// Decode the PPU framebuffer + drain the S-DSP audio from the `System`'s CURRENT state,
    /// without advancing it — the second half of [`Self::run_frame`], split out for a host that
    /// drives `System::run_frame` (or `step_instruction`) directly (netplay rollback, a debugger's
    /// single-step) and needs to pick up the result afterward.
    pub fn present_current_frame(&mut self) {
        self.audio.clear();
        if self.rom_loaded {
            self.system.bus.apu.drain_audio(&mut self.audio);
            self.render_framebuffer();
        }
    }

    /// Decode the PPU's (256|512)×(224|239) BGR555 framebuffer into the RGBA8 present buffer.
    /// Width tracks `Ppu::visible_width` — 512-wide for a hi-res (Modes 5/6) frame, 256-wide
    /// otherwise (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision).
    fn render_framebuffer(&mut self) {
        let h = u32::from(self.system.bus.ppu.visible_height()).min(SNES_H_PAL);
        // `visible_width()` is always SCREEN_WIDTH (256) or MAX_SCREEN_WIDTH (512) — never near
        // u32::MAX, so this narrowing cast can't actually truncate.
        #[allow(clippy::cast_possible_truncation)]
        let w = self.system.bus.ppu.visible_width() as u32;
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

    /// Snapshot the full deterministic core state ([`System::save_state`], `docs/adr/0006`) for
    /// rewind/run-ahead/quick-save. Host-only state (the decoded RGBA8 framebuffer, the retained
    /// ROM/firmware bytes for Power-Cycle, latched pads) is NOT part of this — it's outside the
    /// deterministic core and is re-derived after [`Self::load_state`].
    #[must_use]
    pub fn save_state(&self) -> Vec<u8> {
        self.system.save_state()
    }

    /// Restore a snapshot taken by [`Self::save_state`] from a `System` with the SAME cart
    /// already loaded (a save-state never embeds ROM bytes, `docs/adr/0006`) — the caller must
    /// have already `load_rom`'d the matching ROM. Re-renders the framebuffer immediately so a
    /// host reflects the restored frame without waiting for the next [`Self::run_frame`], and
    /// clears the audio FIFO (a state load jumps time discontinuously; there is no continuous
    /// audio stream to drain across that jump).
    ///
    /// # Errors
    /// Propagates [`rustysnes_savestate::SaveStateError`] if `bytes` is truncated/corrupt, from
    /// an incompatible format version, or doesn't match this `System`'s currently-loaded cart
    /// (SRAM size, coprocessor presence) — the state is left unchanged on error.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), rustysnes_savestate::SaveStateError> {
        self.system.load_state(bytes)?;
        self.audio.clear();
        if self.rom_loaded {
            self.render_framebuffer();
        }
        Ok(())
    }
}

/// ROM-load / emulation errors surfaced to a host.
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
fn extract_rom_bytes(bytes: &[u8]) -> Result<Cow<'_, [u8]>, EmuError> {
    let is_zip = bytes.len() >= 4 && (bytes[..4] == *b"PK\x03\x04" || bytes[..4] == *b"PK\x05\x06");
    if !is_zip {
        return Ok(Cow::Borrowed(bytes));
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
    Ok(Cow::Owned(out))
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

    #[test]
    fn bgr555_decode_matches_known_values() {
        assert_eq!(bgr555_to_rgba8(0x0000), 0xFF00_0000); // opaque black
        assert_eq!(bgr555_to_rgba8(0x7FFF), 0xFFFF_FFFF); // opaque white
        assert_eq!(bgr555_to_rgba8(0x001F), 0xFF00_00FF); // pure red
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
        let zipped = zip_containing("test.sfc", &minimal_lorom());
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&zipped).expect("zip-wrapped ROM should load");
        assert!(core.rom_loaded());
    }

    /// A minimal-but-valid, all-zero-body LoROM image with just enough of a header at $7FC0 for
    /// `Cart::from_rom` to accept it (mirrors `rustysnes-cart::header`'s permissive scoring —
    /// only the size/map-mode bytes need to line up).
    fn minimal_lorom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x8000];
        rom[0x7FC0..0x7FC0 + 21].copy_from_slice(b"TEST ROM             ");
        rom[0x7FD5] = 0x20; // LoROM
        rom[0x7FD6] = 0x00; // no coprocessor
        rom[0x7FD7] = 0x08; // ROM size (2^8 KiB = 256 KiB, permissive)
        rom
    }

    #[test]
    fn set_pad_and_apply_pads_reach_the_bus() {
        let mut core = minimal_lorom_core();
        core.set_pad(0, 0x8000); // Button::B in the standard SNES bit layout
        core.apply_pads();
        assert_eq!(core.system_mut().bus.joypad(0), 0x8000);
    }

    fn minimal_lorom_core() -> EmuCore {
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&minimal_lorom())
            .expect("minimal LoROM should load");
        core
    }
}
