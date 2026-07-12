//! `UniFFI` mobile bridge (`v1.14.0 "Foundry"`, Mobile Phase 1) — generates Kotlin (Android,
//! `v1.15.0 "Sideload"`) and Swift (iOS, `v1.16.0 "Beacon"`) bindings over
//! [`rustysnes_core::facade::EmuCore`], the same `std`-only facade `rustysnes-frontend` and
//! `rustysnes-libretro` already drive the emulator through — this crate adds no new emulation
//! logic of its own, only an FFI-safe wrapper around the existing facade API.
//!
//! # Why this shape
//!
//! `rustysnes_cart::Board: Send` since `v1.0.0`, and every chip crate (`rustysnes-{cpu,ppu,apu,
//! cart}`) has been `#![no_std]` + `alloc` since before that — both prerequisites this crate
//! depends on were already proven, not something landing here for the first time.
//! [`EmuCore`] itself is `std`-only (needs `zip` archive
//! extraction for `.zip`-wrapped ROMs), which is fine: Android's and iOS's Rust targets are both
//! `std`-supporting (Tier 2/3), not bare-metal, unlike the `thumbv7em-none-eabihf` `no_std` CI
//! gate the chip crates are proven against.
//!
//! # MVP surface (`v1.14.0`)
//!
//! ROM load/close, `run_frame`, the peripheral setters (wrapping `rustysnes_core::controller`),
//! framebuffer + audio drain, save/load state, reset/power-cycle. Deliberately NOT in scope this
//! release (see `to-dos/VERSION-PLAN.md`'s `v1.14.0` entry for the full honest-deferral list):
//! HD-pack consumption, cheats, rewind/run-ahead, netplay, `RetroAchievements`, Lua/TAS scripting —
//! every one of those is a real, separate frontend concern layered on top of `EmuCore` in the
//! desktop build too, not something this bridge needs to re-invent to reach a playable MVP.
//!
//! # Threading
//!
//! [`MobileCore`] wraps its `EmuCore` in a [`std::sync::Mutex`] so the generated bindings are
//! `Send + Sync` (a `UniFFI` `Object` requirement) even though nothing here actually drives the
//! emulator from more than one thread at a time in practice — the mobile shell (Kotlin/Swift)
//! calls every method from its own single render/audio-callback thread, exactly like the desktop
//! frontend's synchronous (non-`emu-thread`) render path does.

uniffi::setup_scaffolding!();

use std::sync::{Arc, Mutex};

use rustysnes_cart::Region as CartRegion;
use rustysnes_core::controller::PortDevice;
use rustysnes_core::facade::EmuCore;

/// The console region.
///
/// Mirrors [`rustysnes_cart::Region`], re-exposed as its own `UniFFI` enum so the generated
/// Kotlin/Swift bindings don't need to know about the cart crate's own type, matching the
/// frontend's own `crate::config::Region` re-derivation pattern.
#[derive(uniffi::Enum, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MobileRegion {
    /// 60.0988 Hz, 224 active scanlines.
    #[default]
    Ntsc,
    /// 50.007 Hz, 239 active scanlines.
    Pal,
}

impl From<MobileRegion> for CartRegion {
    fn from(r: MobileRegion) -> Self {
        match r {
            MobileRegion::Ntsc => Self::Ntsc,
            MobileRegion::Pal => Self::Pal,
        }
    }
}

/// Which peripheral occupies a controller port — mirrors
/// [`rustysnes_core::controller::PortDevice`].
#[derive(uniffi::Enum, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MobilePortDevice {
    /// The standard SNES pad (default, both ports).
    #[default]
    Gamepad,
    /// SNES Mouse.
    Mouse,
    /// Super Scope light gun.
    SuperScope,
    /// Super Multitap (4 sub-pads).
    Multitap,
}

impl From<MobilePortDevice> for PortDevice {
    fn from(d: MobilePortDevice) -> Self {
        match d {
            MobilePortDevice::Gamepad => Self::Gamepad,
            MobilePortDevice::Mouse => Self::Mouse,
            MobilePortDevice::SuperScope => Self::SuperScope,
            MobilePortDevice::Multitap => Self::Multitap,
        }
    }
}

/// The active framebuffer's dimensions.
///
/// [`EmuCore::fb_dims`] as an FFI-safe record instead of a bare tuple (`UniFFI` has no tuple
/// type; a named record is also clearer at the Kotlin/Swift call site than a positional pair).
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSize {
    /// Active framebuffer width in pixels.
    pub width: u32,
    /// Active framebuffer height in pixels.
    pub height: u32,
}

/// Errors this bridge can surface across the FFI boundary.
///
/// Both variants wrap the underlying facade error's `Display` text rather than re-deriving
/// structured detail — a mobile UI shows this as a toast/dialog string, it doesn't need to
/// pattern-match a specific failure reason the way `rustysnes-frontend`'s own richer error
/// handling might.
#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum MobileError {
    /// [`EmuCore::load_rom`] failed (empty image, bad header, or a corrupt/unrecognized zip).
    #[error("failed to load ROM: {0}")]
    RomLoad(String),
    /// [`EmuCore::load_state`] failed (bad magic, unsupported `FORMAT_VERSION`, or truncated
    /// data).
    #[error("failed to load save state: {0}")]
    StateLoad(String),
}

/// The emulator handle exposed to Kotlin/Swift — one instance per emulation session, wrapping
/// exactly one [`EmuCore`]. See the module doc for why this is `Mutex`-wrapped.
#[derive(uniffi::Object)]
pub struct MobileCore(Mutex<EmuCore>);

#[uniffi::export]
impl MobileCore {
    /// Construct a fresh, ROM-less core for `region`. Matches [`EmuCore::new`]'s own
    /// power-on-seed convention (a fixed constant, not host randomness — determinism, `docs/adr/
    /// 0004`, applies identically on mobile).
    #[uniffi::constructor]
    #[must_use]
    pub fn new(region: MobileRegion) -> Arc<Self> {
        Arc::new(Self(Mutex::new(EmuCore::new(
            0x5A5A_5A5A_5A5A_5A5A,
            region.into(),
        ))))
    }

    /// Load a ROM image (raw `.sfc`/`.smc`/`.fig`/`.swc` bytes, or a `.zip` wrapping one — see
    /// [`EmuCore::load_rom`]'s own doc for the transparent-unzip behavior).
    ///
    /// # Errors
    /// [`MobileError::RomLoad`] on an empty image, bad header, or corrupt/unrecognized zip.
    // `#[uniffi::export]` needs an owned buffer here -- the generated Kotlin/Swift bindings
    // marshal a `ByteArray`/`Data` across the FFI boundary into an owned `Vec<u8>`, not a borrow
    // with a lifetime the generated glue code has no way to express.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_rom(&self, rom: Vec<u8>) -> Result<(), MobileError> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .load_rom(&rom)
            .map_err(|e| MobileError::RomLoad(e.to_string()))
    }

    /// Close the currently loaded ROM (present a blank/inert core). A no-op if none is loaded.
    pub fn close_rom(&self) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .close_rom();
    }

    /// Whether a ROM is currently loaded.
    #[must_use]
    pub fn rom_loaded(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .rom_loaded()
    }

    /// Soft-reset the console (the cart's reset vector, RAM contents preserved).
    pub fn reset(&self) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .reset();
    }

    /// Hard-reset (power-cycle) the console (RAM re-seeded from the deterministic power-on
    /// pattern, matching real hardware's own inconsistent-but-reproducible SRAM state).
    pub fn power_cycle(&self) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .power_cycle();
    }

    /// Run exactly one emulated frame.
    pub fn run_frame(&self) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .run_frame();
    }

    /// The current RGBA8 framebuffer, copied out (sized to [`Self::frame_size`]'s active
    /// sub-rect within the backing hi-res-worst-case allocation — see [`EmuCore::framebuffer`]'s
    /// own doc for the sub-rect convention the mobile shell's texture upload must respect).
    #[must_use]
    pub fn framebuffer(&self) -> Vec<u8> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .framebuffer()
            .to_vec()
    }

    /// The active framebuffer's `(width, height)`.
    #[must_use]
    pub fn frame_size(&self) -> FrameSize {
        let (width, height) = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .fb_dims();
        FrameSize { width, height }
    }

    /// Drain buffered audio as interleaved `[L, R, L, R, ...]` 16-bit PCM samples, ready for a
    /// mobile audio callback (`AAudio`/`AVAudioEngine`) to consume directly.
    #[must_use]
    pub fn drain_audio(&self) -> Vec<i16> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .audio()
            .iter()
            .flat_map(|&pair| <[i16; 2]>::from(pair))
            .collect()
    }

    /// Set player `player`'s (`0` or `1`) standard-gamepad button state (see
    /// [`EmuCore::set_pad`]'s own doc for the SNES button-bit layout).
    pub fn set_pad(&self, player: u8, buttons: u16) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_pad(player as usize, buttons);
    }

    /// Select which peripheral occupies controller port `port` (`0` or `1`).
    pub fn set_port_device(&self, port: u8, device: MobilePortDevice) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_port_device(port as usize, device.into());
    }

    /// Update the Mouse peripheral's relative-motion + button state on port `port`.
    pub fn set_mouse(&self, port: u8, dx: i32, dy: i32, left: bool, right: bool) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_mouse(port as usize, dx, dy, left, right);
    }

    /// Update the Super Scope's screen-space aim position + button state on port `port`.
    pub fn set_superscope(&self, port: u8, x: i32, y: i32, buttons: u8) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_superscope(port as usize, x, y, buttons);
    }

    /// Update one Super Multitap sub-pad's (`sub_index` `0..=3`) button state on port `port`.
    pub fn set_multitap_pad(&self, port: u8, sub_index: u8, buttons: u16) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_multitap_pad(port as usize, sub_index as usize, buttons);
    }

    /// Serialize the current deterministic core state (see `docs/adr/0006` for the on-disk
    /// format) to a byte blob the mobile shell can persist however it likes (a file, `SharedPreferences`/
    /// `UserDefaults`-adjacent storage, etc. — this crate has no filesystem opinion).
    #[must_use]
    pub fn save_state(&self) -> Vec<u8> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .save_state()
    }

    /// Restore a state blob previously produced by [`Self::save_state`] against the SAME ROM.
    ///
    /// # Errors
    /// [`MobileError::StateLoad`] on bad magic, an unsupported `FORMAT_VERSION`, or truncated
    /// data — see `crates/rustysnes-core/src/scheduler.rs`'s `FORMAT_VERSION` doc for why this is
    /// intentionally fail-loud rather than a graceful old-format migration.
    // See `load_rom`'s identical comment -- `#[uniffi::export]` needs an owned buffer.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_state(&self, blob: Vec<u8>) -> Result<(), MobileError> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .load_state(&blob)
            .map_err(|e| MobileError::StateLoad(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_lorom() -> Vec<u8> {
        // A minimal-but-valid LoROM header: name padded with spaces, mode byte 0x20 (LoROM,
        // FastROM off), a plausible checksum/complement pair at the fixed LoROM header offset
        // (0x7FC0) -- enough for `EmuCore::load_rom`'s header detection to accept it, matching
        // the same fixture shape `rustysnes-script`'s own bus-widening test uses.
        let mut rom = vec![0u8; 0x1_0000];
        let header = 0x7FC0;
        rom[header..header + 21].copy_from_slice(b"RUSTYSNES MOBILE TEST");
        rom[header + 0x15] = 0x20; // LoROM, FastROM off
        rom[header + 0x16] = 0x00; // no coprocessor
        rom[header + 0x17] = 0x08; // 2 Mbit ROM size
        rom[header + 0x18] = 0x00; // no SRAM
        rom[header + 0x1A] = 0x01; // licensee
        rom[header + 0x1C] = 0x00;
        rom[header + 0x1D] = 0x00;
        rom[header + 0x1E] = 0xFF;
        rom[header + 0x1F] = 0xFF;
        rom
    }

    #[test]
    fn new_core_has_no_rom_loaded() {
        let core = MobileCore::new(MobileRegion::Ntsc);
        assert!(!core.rom_loaded());
    }

    #[test]
    fn load_rom_then_run_frame_produces_a_correctly_sized_framebuffer() {
        let core = MobileCore::new(MobileRegion::Ntsc);
        core.load_rom(minimal_lorom()).expect("load");
        assert!(core.rom_loaded());
        core.run_frame();
        let size = core.frame_size();
        assert_eq!(
            size,
            FrameSize {
                width: 256,
                height: 224
            }
        );
        assert_eq!(
            core.framebuffer().len(),
            (size.width * size.height * 4) as usize
        );
    }

    #[test]
    fn load_rom_rejects_an_empty_image() {
        let core = MobileCore::new(MobileRegion::Ntsc);
        let err = core.load_rom(Vec::new()).unwrap_err();
        assert!(matches!(err, MobileError::RomLoad(_)));
    }

    #[test]
    fn save_state_round_trips_through_load_state() {
        let core = MobileCore::new(MobileRegion::Ntsc);
        core.load_rom(minimal_lorom()).expect("load");
        core.run_frame();
        let blob = core.save_state();
        assert!(!blob.is_empty());
        core.load_state(blob).expect("round trip");
    }

    #[test]
    fn load_state_rejects_garbage() {
        let core = MobileCore::new(MobileRegion::Ntsc);
        core.load_rom(minimal_lorom()).expect("load");
        let err = core.load_state(vec![0u8; 4]).unwrap_err();
        assert!(matches!(err, MobileError::StateLoad(_)));
    }

    #[test]
    fn set_pad_and_port_device_do_not_panic_without_a_rom() {
        let core = MobileCore::new(MobileRegion::Ntsc);
        core.set_pad(0, 0xFFFF);
        core.set_port_device(1, MobilePortDevice::Mouse);
        core.set_mouse(1, 5, -5, true, false);
        core.set_superscope(1, 100, 50, 0);
        core.set_multitap_pad(1, 2, 0x00FF);
    }

    #[test]
    fn drain_audio_returns_interleaved_stereo_samples() {
        let core = MobileCore::new(MobileRegion::Ntsc);
        core.load_rom(minimal_lorom()).expect("load");
        core.run_frame();
        let audio = core.drain_audio();
        assert_eq!(audio.len() % 2, 0, "interleaved L/R must be an even count");
    }
}
