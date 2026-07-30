//! Per-game configuration overrides (`v1.25.0`).
//!
//! Ported from RustyNES's own `per_game.rs`. Some settings are genuinely per-title rather than
//! global: a PAL-only game wants PAL timing, one game's music wants a different EQ, a hack wants a
//! different aspect. Storing those in the global config forces the user to keep re-editing it.
//!
//! # Design: an explicit small set of Options, not a whole nested Config
//!
//! An overlay of "the entire `Config`, optionally" cannot distinguish "this game wants the default"
//! from "this game was saved before that field existed" — every absent field would silently become
//! whatever the *current* default is, and a global change would stop reaching games that had ever
//! been saved. So the overlay names exactly the knobs that make sense per-title, each an `Option`
//! that means "override" when present and "inherit the global value" when absent.
//!
//! Files live in `<config-dir>/RustySNES/per-game/<key>.toml`, keyed by the ROM's file stem
//! sanitised to a safe filename.

use serde::{Deserialize, Serialize};

use crate::config::{AspectMode, Config, Overscan, PostFilter, Region};

/// Per-title overrides. Every field is `None` = inherit the global setting.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PerGameOverrides {
    /// Console region (a PAL-only title wants PAL timing regardless of the global default).
    pub region: Option<Region>,
    /// Presentation aspect ratio.
    pub aspect: Option<AspectMode>,
    /// Integer-scale the framebuffer.
    pub integer_scale: Option<bool>,
    /// Active post-filter.
    pub filter: Option<PostFilter>,
    /// Per-side presentation crop.
    pub overscan: Option<Overscan>,
    /// Crop the 239-line overscan extension.
    pub hide_overscan: Option<bool>,
    /// Target audio latency in milliseconds.
    pub latency_ms: Option<u32>,
    /// Master volume.
    pub volume: Option<f32>,
    /// Run-ahead depth in frames.
    pub run_ahead_frames: Option<u32>,
}

impl PerGameOverrides {
    /// Whether every field is absent (nothing to save, nothing to apply).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.region.is_none()
            && self.aspect.is_none()
            && self.integer_scale.is_none()
            && self.filter.is_none()
            && self.overscan.is_none()
            && self.hide_overscan.is_none()
            && self.latency_ms.is_none()
            && self.volume.is_none()
            && self.run_ahead_frames.is_none()
    }

    /// Snapshot the overridable knobs from `cfg` (the "save current settings for this game" action).
    ///
    /// Captures every field rather than only those differing from the default: the user's intent is
    /// "this game should look and sound like it does right now", and a diff-against-default overlay
    /// would silently change when a global default later changed.
    #[must_use]
    pub const fn capture(cfg: &Config) -> Self {
        Self {
            region: Some(cfg.region),
            aspect: Some(cfg.video.aspect),
            integer_scale: Some(cfg.video.integer_scale),
            filter: Some(cfg.video.filter),
            overscan: Some(cfg.video.overscan),
            hide_overscan: Some(cfg.video.hide_overscan),
            latency_ms: Some(cfg.audio.latency_ms),
            volume: Some(cfg.audio.volume),
            run_ahead_frames: Some(cfg.run_ahead.frames),
        }
    }

    /// Apply the present overrides onto `cfg` in place.
    ///
    /// Absent fields leave `cfg` untouched, which is what makes this an overlay rather than a
    /// replacement.
    pub const fn apply(&self, cfg: &mut Config) {
        if let Some(v) = self.region {
            cfg.region = v;
        }
        if let Some(v) = self.aspect {
            cfg.video.aspect = v;
        }
        if let Some(v) = self.integer_scale {
            cfg.video.integer_scale = v;
        }
        if let Some(v) = self.filter {
            cfg.video.filter = v;
        }
        if let Some(v) = self.overscan {
            cfg.video.overscan = v;
        }
        if let Some(v) = self.hide_overscan {
            cfg.video.hide_overscan = v;
        }
        if let Some(v) = self.latency_ms {
            cfg.audio.latency_ms = v;
        }
        if let Some(v) = self.volume {
            cfg.audio.volume = v;
        }
        if let Some(v) = self.run_ahead_frames {
            cfg.run_ahead.frames = v;
        }
    }
}

/// Reduce a ROM identifier to a safe filename key.
///
/// The same rule `crate::screenshot` uses, and for the same reason: a ROM stem can contain path
/// separators and platform-illegal characters, and writing one straight into a path is how the
/// override silently fails to save.
#[must_use]
pub fn key_for(rom_stem: &str) -> String {
    let cleaned: String = rom_stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The directory per-game override files live in, or `None` when no config dir is resolvable
/// (always `None` on `wasm32`).
#[must_use]
pub fn dir() -> Option<std::path::PathBuf> {
    Config::path().and_then(|p| p.parent().map(|d| d.join("per-game")))
}

/// The override file path for `rom_stem`.
#[must_use]
pub fn path_for(rom_stem: &str) -> Option<std::path::PathBuf> {
    dir().map(|d| d.join(format!("{}.toml", key_for(rom_stem))))
}

/// Load `rom_stem`'s overrides, or `None` if there are none (or they cannot be parsed).
///
/// A corrupt override file degrades to "no overrides" rather than blocking the ROM from loading —
/// the same fail-soft posture `Config::load` takes, for the same reason.
#[must_use]
pub fn load(rom_stem: &str) -> Option<PerGameOverrides> {
    let path = path_for(rom_stem)?;
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

/// Persist `overrides` for `rom_stem`.
///
/// # Errors
/// Returns an [`std::io::Error`] if the directory cannot be created or the file written.
pub fn save(rom_stem: &str, overrides: &PerGameOverrides) -> std::io::Result<()> {
    let Some(path) = path_for(rom_stem) else {
        return Ok(()); // no config dir (wasm) — a no-op, matching Config::save
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(overrides)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, text)
}

/// Delete `rom_stem`'s overrides, if any.
///
/// # Errors
/// Returns an [`std::io::Error`] if the file exists but cannot be removed. A missing file is
/// success: "forget the overrides for this game" is satisfied by there being none.
pub fn forget(rom_stem: &str) -> std::io::Result<()> {
    let Some(path) = path_for(rom_stem) else {
        return Ok(());
    };
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_fields_leave_the_global_config_untouched() {
        let mut cfg = Config::default();
        cfg.audio.volume = 0.5;
        cfg.video.aspect = AspectMode::Square;
        // An empty overlay must change nothing at all — that is what makes it an overlay.
        let empty = PerGameOverrides::default();
        assert!(empty.is_empty());
        empty.apply(&mut cfg);
        assert!((cfg.audio.volume - 0.5).abs() < 1e-6);
        assert_eq!(cfg.video.aspect, AspectMode::Square);
    }

    #[test]
    fn present_fields_override_and_others_still_inherit() {
        let mut cfg = Config::default();
        cfg.audio.volume = 0.5;
        cfg.video.integer_scale = false;
        let overlay = PerGameOverrides {
            volume: Some(0.9),
            ..PerGameOverrides::default()
        };
        assert!(!overlay.is_empty());
        overlay.apply(&mut cfg);
        assert!((cfg.audio.volume - 0.9).abs() < 1e-6, "override must win");
        assert!(!cfg.video.integer_scale, "unspecified fields must inherit");
    }

    #[test]
    fn capture_then_apply_round_trips_every_knob() {
        let base = Config::default();
        let source = Config {
            region: Region::Pal,
            video: crate::config::VideoConfig {
                aspect: AspectMode::Par,
                integer_scale: true,
                filter: PostFilter::Crt,
                hide_overscan: true,
                overscan: Overscan {
                    top: 4,
                    bottom: 4,
                    left: 2,
                    right: 2,
                },
                ..base.video
            },
            audio: crate::config::AudioConfig {
                latency_ms: 24,
                volume: 0.33,
                ..base.audio
            },
            run_ahead: crate::config::RunAheadConfig {
                frames: 2,
                ..base.run_ahead
            },
            ..base
        };

        let captured = PerGameOverrides::capture(&source);
        let mut target = Config::default();
        captured.apply(&mut target);

        assert_eq!(target.region, Region::Pal);
        assert_eq!(target.video.aspect, AspectMode::Par);
        assert!(target.video.integer_scale);
        assert_eq!(target.video.filter, PostFilter::Crt);
        assert!(target.video.hide_overscan);
        assert_eq!(target.video.overscan, source.video.overscan);
        assert_eq!(target.audio.latency_ms, 24);
        assert!((target.audio.volume - 0.33).abs() < 1e-6);
        assert_eq!(target.run_ahead.frames, 2);
    }

    #[test]
    fn overlay_round_trips_through_toml_including_absent_fields() {
        let overlay = PerGameOverrides {
            region: Some(Region::Pal),
            volume: Some(0.25),
            ..PerGameOverrides::default()
        };
        let text = toml::to_string_pretty(&overlay).expect("serialize");
        let back: PerGameOverrides = toml::from_str(&text).expect("deserialize");
        assert_eq!(back, overlay);
        // The distinction that matters: an absent field stays absent, rather than materialising as
        // the current default and freezing it into the file.
        assert!(back.aspect.is_none());
        assert!(back.latency_ms.is_none());
        // An empty document is a valid, fully-inheriting overlay.
        let none: PerGameOverrides = toml::from_str("").expect("deserialize empty");
        assert!(none.is_empty());
    }

    #[test]
    fn keys_are_filename_safe() {
        assert_eq!(key_for("Super Mario World"), "Super_Mario_World");
        assert_eq!(key_for("../../etc/passwd"), "etc_passwd");
        assert!(!key_for("../../etc/passwd").contains('/'));
        assert!(!key_for("a/../b").contains(".."));
        // An all-punctuation stem still yields a usable key rather than an empty filename.
        assert_eq!(key_for("***"), "unknown");
        assert_eq!(key_for(""), "unknown");
    }
}
