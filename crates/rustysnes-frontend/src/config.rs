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

/// Which peripheral is connected to controller port 2 (`v0.9.0`, Phase 7 niche peripherals).
///
/// Port 1 is always a standard [`PeripheralKind::Gamepad`] — matching real hardware convention
/// (mice/light guns/multitaps are documented as port-2-only devices in practice; ares' own Super
/// Scope note: "no commercial game ever utilizes a Super Scope in port 1") and this project's
/// existing P1-is-the-primary-live-input-source posture (`app.rs`'s `apply_frame_input`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeripheralKind {
    /// The standard SNES pad.
    #[default]
    Gamepad,
    /// SNES Mouse.
    Mouse,
    /// Super Scope light gun.
    SuperScope,
    /// Super Multitap (4 sub-pads).
    Multitap,
}

/// egui visual theme for the desktop UX shell (menu bar, status bar, windows) — `v1.0.0` desktop
/// UX shell maturity; `v1.13.0 "Vantage"` adds two accessibility-oriented variants.
///
/// [`AppTheme::HighContrast`] and [`AppTheme::Colorblind`] are appended after the original three
/// (not inserted between them) purely for readability — an existing `config.toml` storing
/// `"light"`/`"dark"`/`"system"` was already safe to grow additively regardless of variant order,
/// since `#[serde(rename_all = "lowercase")]` tags each variant by its STRING name, not its
/// discriminant position; this matches every other `PostFilter`/theme-shaped enum growth in this
/// project.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    /// Light visuals.
    Light,
    /// Dark visuals (default).
    #[default]
    Dark,
    /// Follow the OS theme when the windowing system reports one (falls back to
    /// [`AppTheme::Dark`] when unknown — `egui::Context::system_theme`).
    System,
    /// High-contrast dark theme for low-vision accessibility: near-black backgrounds, near-white
    /// text, and a bright cyan selection accent, with every foreground/background pair pushed
    /// past the WCAG 2.1 AA (4.5:1) contrast ratio — most clear AAA (7:1) — for normal-size text.
    #[serde(rename = "high-contrast")]
    HighContrast,
    /// Colorblind-safe dark theme whose interactive accents (selection, hover, hyperlinks) are
    /// drawn from the Okabe-Ito palette, chosen to stay mutually distinguishable under the most
    /// common (red-green) forms of color-vision deficiency.
    Colorblind,
}

impl AppTheme {
    /// Human-readable label for the Settings radio row.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
            Self::HighContrast => "High Contrast",
            Self::Colorblind => "Colorblind-Safe",
        }
    }

    /// All themes in display order — the single source of truth the Settings radio row iterates,
    /// so it can never drift out of sync with the enum.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Light,
            Self::Dark,
            Self::System,
            Self::HighContrast,
            Self::Colorblind,
        ]
    }
}

impl PeripheralKind {
    /// The matching [`rustysnes_core::controller::PortDevice`] this config value selects.
    #[must_use]
    pub const fn to_core(self) -> rustysnes_core::controller::PortDevice {
        match self {
            Self::Gamepad => rustysnes_core::controller::PortDevice::Gamepad,
            Self::Mouse => rustysnes_core::controller::PortDevice::Mouse,
            Self::SuperScope => rustysnes_core::controller::PortDevice::SuperScope,
            Self::Multitap => rustysnes_core::controller::PortDevice::Multitap,
        }
    }
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

/// A presentation post-filter (`v1.2.0`). Applied after the plain nearest-sample framebuffer
/// blit, before the always-on egui shell pass — see `crate::gfx`'s module doc for the pipeline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostFilter {
    /// No post-filter — the plain nearest-sample blit, pixel-identical to a filter-less build
    /// (default; a `config.toml` predating this field deserializes to this value via
    /// `#[serde(default)]`, so existing setups behave exactly as before. Note this is a BEHAVIOR
    /// guarantee, not a textual one — `Config::save` re-serializes the whole struct, so an old
    /// config gains this field's TOML text the next time settings are saved, same as any other
    /// added field).
    #[default]
    None,
    /// Scanlines + an RGB aperture-grille mask, approximating a CRT's phosphor structure.
    Crt,
    /// A single-pass, edge-directed diagonal blend that softens staircase edges on flat-color
    /// pixel art — an HQ2x-style *approximation* (not a literal `HQ2x` lookup-table port).
    Hqx,
    /// A single-pass, context-aware corner-rounding blend (`v1.12.0 "Refraction"`) — an
    /// xBRZ-style *approximation* (not a literal multi-pass xBRZ port); see
    /// [`rustysnes_gfx_shaders::XBRZ_WGSL`]'s own doc for how it differs from [`Self::Hqx`].
    Xbrz,
}

impl PostFilter {
    /// Human-readable label for the Settings radio row.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Crt => "CRT",
            Self::Hqx => "HQx",
            Self::Xbrz => "xBRZ",
        }
    }

    /// All filters in display order — the single source of truth the Settings radio row
    /// iterates, so it can never drift out of sync with the enum.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::None, Self::Crt, Self::Hqx, Self::Xbrz]
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
    /// The active presentation post-filter (`v1.2.0`, default `None` — byte-identical to every
    /// prior release when unchanged).
    pub filter: PostFilter,
    /// [`PostFilter::Crt`] scanline intensity, `0.0..=1.0` (0 = no scanlines).
    pub crt_scanline: f32,
    /// [`PostFilter::Crt`] RGB aperture-mask intensity, `0.0..=1.0` (0 = no mask).
    pub crt_mask: f32,
    /// [`PostFilter::Hqx`] edge-directed blend strength, `0.0..=1.0` (0 = plain bilinear).
    pub hqx_strength: f32,
    /// [`PostFilter::Xbrz`] context-gated corner-blend strength, `0.0..=1.0` (0 = plain
    /// bilinear) — `v1.12.0 "Refraction"`.
    pub xbrz_strength: f32,
    /// The active HD texture pack's name for the current ROM (`v1.3.0`), or `None` (the default
    /// — byte-identical config round-trip for every prior release). Present regardless of
    /// whether this build has the `hd-pack` Cargo feature on, matching every other config field's
    /// posture (`port2_peripheral`, `rewind`, …) — an inert value in a build that can't act on
    /// it, not a compile-time-gated field.
    pub hd_pack_name: Option<String>,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            present_mode: "fifo".into(),
            pacing: PacingMode::default(),
            integer_scale: false,
            filter: PostFilter::default(),
            crt_scanline: 0.3,
            crt_mask: 0.15,
            hqx_strength: 0.6,
            xbrz_strength: 0.6,
            hd_pack_name: None,
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
    /// Per-voice (S-DSP channel 0-7) mute toggles (`v1.0.1`). A frontend/debug convenience, not
    /// real hardware state — see `rustysnes_apu::dsp::Dsp::set_voice_mutes`'s doc (`docs/apu.md`
    /// §Per-voice mute has the full mechanism). All `false` (unmuted) by default, byte-identical
    /// to every prior release.
    pub voice_mutes: [bool; 8],
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            volume: 0.8,
            enabled: true,
            voice_mutes: [false; 8],
        }
    }
}

/// Rewind settings (`crate::rewind::RewindBuffer`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RewindConfig {
    /// Maximum snapshots retained. `0` disables rewind entirely (additive-default-off).
    pub capacity: usize,
    /// Record a snapshot every this many real frames (minimum 1 — clamped by
    /// `RewindBuffer::new`).
    pub interval_frames: u32,
}

impl Default for RewindConfig {
    fn default() -> Self {
        // 300 snapshots @ every 6th frame (~10 Hz recording) covers ~30s of NTSC rewind at a
        // memory cost bounded by `capacity`, not by frame count — see `crate::rewind` module docs
        // for why full snapshots (not delta-compressed keyframes) were chosen. Off by default
        // (`capacity: 0`) until Settings UI + a hotkey to actually trigger it lands.
        Self {
            capacity: 0,
            interval_frames: 6,
        }
    }
}

/// Run-ahead settings (`crate::rewind::step_with_run_ahead`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RunAheadConfig {
    /// Frames to peek ahead each displayed frame. `0` disables run-ahead entirely
    /// (additive-default-off) — `step_with_run_ahead` degrades to a plain `run_frame`.
    pub frames: u32,
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
    /// Which peripheral occupies controller port 2 (`v0.9.0`). Host-input capture (a real mouse
    /// pointer driving Super Scope aim / SNES Mouse deltas, extra gamepads for Multitap sub-pads)
    /// is a follow-up frontend task — selecting a non-`Gamepad` device here wires the core's
    /// protocol correctly (`rustysnes_core::controller`) but this frontend does not yet feed it
    /// live host input (`docs/frontend.md` §Peripherals).
    pub port2_peripheral: PeripheralKind,
    /// Rewind (`v0.3.0 "Continuum"`).
    pub rewind: RewindConfig,
    /// Run-ahead (`v0.3.0 "Continuum"`).
    pub run_ahead: RunAheadConfig,
    /// The desktop UX shell's egui visual theme (`v1.0.0`).
    pub theme: AppTheme,
    /// Whether the first-run welcome modal has already been dismissed (`v1.0.0`). `false` (the
    /// default) shows it once on the very next launch; dismissing it flips this and saves.
    pub first_run_seen: bool,
}

impl Config {
    /// The on-disk config path (`<platform-config-dir>/RustySNES/config.toml`), or `None` if no
    /// config dir is resolvable — always `None` on `wasm32` (no filesystem; `load`/`save` below
    /// degrade to "always the default" / "always a no-op" as a result, not specially cased).
    // The wasm32 body is trivially `const`-eligible; the native body (a `directories` crate call)
    // is not, so the same function can't uniformly satisfy the lint across targets.
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn path() -> Option<std::path::PathBuf> {
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            directories::ProjectDirs::from("io.github", "doublegate", "RustySNES")
                .map(|d| d.config_dir().join("config.toml"))
        }
    }

    /// Load the config from disk, falling back to defaults on any error (a missing or corrupt
    /// file should never block launch) — always the default on `wasm32` (`path()` returns `None`
    /// there).
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

    /// Persist the config to disk (best-effort; creates the parent dir) — always a no-op on
    /// `wasm32` (`path()` returns `None` there).
    ///
    /// # Errors
    /// Returns an [`std::io::Error`] if the directory cannot be created or the file written.
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

    #[test]
    fn theme_default_is_dark_and_round_trips() {
        assert_eq!(Config::default().theme, AppTheme::Dark);
        for theme in AppTheme::all() {
            let cfg = Config {
                theme,
                ..Config::default()
            };
            let s = toml::to_string_pretty(&cfg).expect("serialize");
            let back: Config = toml::from_str(&s).expect("deserialize");
            assert_eq!(back.theme, theme);
        }
    }

    #[test]
    fn voice_mutes_default_to_unmuted_and_round_trip() {
        assert_eq!(Config::default().audio.voice_mutes, [false; 8]);
        let mut audio = AudioConfig::default();
        audio.voice_mutes[2] = true;
        audio.voice_mutes[7] = true;
        let cfg = Config {
            audio,
            ..Config::default()
        };
        let s = toml::to_string_pretty(&cfg).expect("serialize");
        let back: Config = toml::from_str(&s).expect("deserialize");
        assert_eq!(
            back.audio.voice_mutes,
            [false, false, true, false, false, false, false, true]
        );
    }
}
