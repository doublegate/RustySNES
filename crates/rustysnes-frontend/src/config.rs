//! Frontend configuration (TOML), loaded from the platform config dir and surfaced in the
//! tabbed Settings window.
//!
//! Carries the display-sync pacing preference, the region (NTSC/PAL → frame-rate target), the
//! audio settings, and the per-player [`crate::input::KeyBindings`]. This is the
//! RustyNES config schema, SNES-adapted (the region drives the SNES frame rate + the active
//! scanline count).

use serde::{Deserialize, Serialize};

use crate::input::KeyBindings;

/// The display-sync pacing strategy (the RustyNES pacing matrix, ported).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PacingMode {
    /// Pick the best mode from the display + present-mode caps (default).
    #[default]
    Auto,
    /// Lock to the display's refresh (Fifo vsync); audio resampled to fit.
    Display,
    /// Variable-refresh-rate aware (present when the frame is ready).
    Vrr,
    /// Free-run on the wall clock at the region frame rate; present-mode mailbox/immediate.
    Wallclock,
}

/// The console region (timing + active-scanline count).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Region {
    /// 60.0988 Hz, 224 active scanlines.
    #[default]
    Ntsc,
    /// 50.007 Hz, 239 active scanlines.
    Pal,
}

impl Region {
    /// The wall-clock frame-rate target for this region (the pacer's authoritative cadence).
    #[must_use]
    pub const fn frame_rate(self) -> f64 {
        match self {
            Self::Ntsc => crate::FRAME_RATE_NTSC,
            Self::Pal => crate::FRAME_RATE_PAL,
        }
    }

    /// The active-region framebuffer height for this region (256 wide always).
    #[must_use]
    pub const fn active_height(self) -> u32 {
        match self {
            Self::Ntsc => crate::gfx::SNES_H_NTSC,
            Self::Pal => crate::gfx::SNES_H_PAL,
        }
    }
}

/// Video / windowing settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoConfig {
    /// The wgpu present mode preference (`"fifo"` / `"mailbox"` / `"immediate"`).
    pub present_mode: String,
    /// The display-sync pacing strategy.
    pub pacing: PacingMode,
    /// Integer-scale the framebuffer (true) or fit-to-window with aspect correction (false).
    pub integer_scale: bool,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            present_mode: "fifo".into(),
            pacing: PacingMode::default(),
            integer_scale: false,
        }
    }
}

/// Audio settings (the lock-free ring + dynamic-rate-control servo live in `audio.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Master output sample rate (the cpal stream target; the resampler fits the S-DSP's
    /// 32 kHz native rate to it).
    pub sample_rate: u32,
    /// Master volume in `0.0..=1.0`.
    pub volume: f32,
    /// Whether audio output is enabled at all.
    pub enabled: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            volume: 0.8,
            enabled: true,
        }
    }
}

/// The full frontend config (serialized to `config.toml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The console region (timing + active scanlines).
    pub region: Region,
    /// Video / windowing.
    pub video: VideoConfig,
    /// Audio.
    pub audio: AudioConfig,
    /// Player 1 keyboard binds.
    pub p1: KeyBindings,
    /// Player 2 keyboard binds (the second-pad default is a TODO; empty = unbound).
    pub p2: KeyBindings,
}

impl Config {
    /// The on-disk config path (`<platform-config-dir>/RustySNES/config.toml`), or `None` if no
    /// config dir is resolvable. Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("io.github", "doublegate", "RustySNES")
            .map(|d| d.config_dir().join("config.toml"))
    }

    /// Load the config from disk, falling back to defaults on any error (a missing or corrupt
    /// file should never block launch). Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        std::fs::read_to_string(&path).map_or_else(
            |_| Self::default(),
            |s| toml::from_str(&s).unwrap_or_default(),
        )
    }

    /// Persist the config to disk (best-effort; creates the parent dir). Native-only.
    ///
    /// # Errors
    /// Returns an [`std::io::Error`] if the directory cannot be created or the file written.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips_through_toml() {
        let cfg = Config::default();
        let s = toml::to_string_pretty(&cfg).expect("serialize");
        let back: Config = toml::from_str(&s).expect("deserialize");
        assert_eq!(back.region, cfg.region);
        assert_eq!(back.audio.sample_rate, cfg.audio.sample_rate);
        assert_eq!(back.p1.binds.len(), 12);
    }

    #[test]
    fn region_frame_rates_and_heights() {
        assert!((Region::Ntsc.frame_rate() - 60.0988).abs() < 1e-3);
        assert_eq!(Region::Ntsc.active_height(), 224);
        assert_eq!(Region::Pal.active_height(), 239);
    }
}
