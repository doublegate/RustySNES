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

impl PacingMode {
    /// The label shown in Settings and in a resolved [`crate::pacing::PacingPlan`].
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Display => "Display-sync",
            Self::Vrr => "VRR",
            Self::Wallclock => "Wall-clock",
        }
    }

    /// Every variant, in Settings display order.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Auto, Self::Display, Self::Vrr, Self::Wallclock]
    }
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

/// How the framebuffer's pixels are shaped when fitted to the window.
///
/// The SNES's 256x224 framebuffer is not square-pixel: it was designed for a 4:3 television, which
/// stretches each pixel horizontally. Which correction is "right" is a genuine preference, so all
/// three are offered rather than one being hardcoded (as it was before `v1.25.0`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectMode {
    /// Stretch to a 4:3 display, the shape a CRT television actually presented (default —
    /// identical to every release before this setting existed).
    #[default]
    #[serde(rename = "4:3")]
    FourThree,
    /// Correct by the 8:7 pixel aspect ratio the hardware's dot clock implies. Very close to 4:3
    /// at 224 lines, and noticeably different at PAL's 239 — the arithmetically-derived shape
    /// rather than the television's.
    #[serde(rename = "8:7")]
    Par,
    /// Square pixels: no correction at all, the framebuffer's own 256:H ratio. Geometrically
    /// "wrong" versus hardware, but what pixel-art purists often want.
    #[serde(rename = "1:1")]
    Square,
}

impl AspectMode {
    /// Human-readable label for the Settings radio row.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::FourThree => "4:3 (CRT)",
            Self::Par => "8:7 (pixel aspect)",
            Self::Square => "1:1 (square pixels)",
        }
    }

    /// All modes in display order — the single source of truth the Settings row iterates.
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::FourThree, Self::Par, Self::Square]
    }

    /// The target display aspect for a framebuffer of `fb_w` x `fb_h`.
    ///
    /// `fb_h` is the *measured* framebuffer height, not the region — a hi-res or overscan frame
    /// must correct by what is actually being displayed, and keying off the region bit instead is
    /// how a 239-line image ends up squashed into a 224-line shape.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn ratio(self, fb_w: u32, fb_h: u32) -> f32 {
        let w = fb_w.max(1) as f32;
        let h = fb_h.max(1) as f32;
        match self {
            // The same constant `app.rs` derives its window size from, so the window it opens and
            // the shape drawn inside it cannot disagree.
            Self::FourThree => crate::gfx::TARGET_ASPECT,
            Self::Par => (w * (8.0 / 7.0)) / h,
            Self::Square => w / h,
        }
    }
}

/// Per-side presentation crop, in framebuffer pixels.
///
/// A real television hid a few pixels behind the bezel, and some games leave garbage there (a
/// partially-scrolled column, an uninitialised row). Cropping is **presentation-only** — the
/// deterministic core still renders every pixel — which is the same boundary every post-filter in
/// this crate respects. All zero by default, so the default build presents the full framebuffer
/// exactly as before.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Overscan {
    /// Scanlines removed from the top.
    pub top: u32,
    /// Scanlines removed from the bottom.
    pub bottom: u32,
    /// Columns removed from the left.
    pub left: u32,
    /// Columns removed from the right.
    pub right: u32,
}

impl Overscan {
    /// Whether any side actually crops (the fast path skips the copy entirely when not).
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.top == 0 && self.bottom == 0 && self.left == 0 && self.right == 0
    }

    /// Clamp the crop so at least a 16x16 image survives, whatever the config file asked for.
    ///
    /// A hand-edited `config.toml` must not be able to crop the picture out of existence — that
    /// would produce a zero-sized texture upload, which is a wgpu validation error rather than a
    /// merely-ugly result.
    #[must_use]
    pub const fn clamped(self, fb_w: u32, fb_h: u32) -> Self {
        const MIN: u32 = 16;
        let max_x = fb_w.saturating_sub(MIN);
        let max_y = fb_h.saturating_sub(MIN);
        let (mut left, mut right) = (self.left, self.right);
        let (mut top, mut bottom) = (self.top, self.bottom);
        if left + right > max_x {
            right = max_x.saturating_sub(left);
            if left > max_x {
                left = max_x;
                right = 0;
            }
        }
        if top + bottom > max_y {
            bottom = max_y.saturating_sub(top);
            if top > max_y {
                top = max_y;
                bottom = 0;
            }
        }
        Self {
            top,
            bottom,
            left,
            right,
        }
    }
}

/// Which built-in multi-pass chain the shader stack runs (`v1.25.0`, T-FP-D).
///
/// `Off` is the default and is **byte-identical** to a build without the stack — the chain is empty
/// so `Gfx::present_chain` falls straight through to the plain blit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShaderStack {
    /// No stack; the `v1.2.0` `PostFilter` path is used unchanged.
    #[default]
    Off,
    /// The richer CRT pass (scanlines, mask, curvature, beam shape, glow, vignette).
    Crt,
    /// The NTSC composite-artefact pass (chroma bleed, artefacts, fringing, dot crawl).
    Ntsc,
    /// NTSC into CRT — the chain most people actually want, and the case a single-filter enum
    /// structurally cannot express.
    NtscCrt,
}

impl ShaderStack {
    /// The label shown in Settings.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Crt => "CRT",
            Self::Ntsc => "NTSC",
            Self::NtscCrt => "NTSC + CRT",
        }
    }

    /// Every variant, in Settings order.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::Off, Self::Crt, Self::Ntsc, Self::NtscCrt]
    }
}

/// Video / windowing settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoConfig {
    /// The wgpu present mode preference: `"auto"` (default since `v1.25.0` — let [`Self::pacing`]
    /// choose), or an explicit `"fifo"` / `"mailbox"` / `"immediate"` override.
    ///
    /// An explicit value always wins; only `"auto"` defers to the resolved
    /// [`crate::pacing::PacingPlan`]. A config written by an earlier release carries an explicit
    /// `"fifo"`, so upgrading changes nothing for an existing install.
    pub present_mode: String,
    /// The display-sync pacing strategy. Resolved against the monitor refresh and the surface's
    /// present-mode caps by [`crate::pacing::resolve`] at window creation.
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
    /// The multi-pass shader stack's selected chain (`v1.25.0`, T-FP-D).
    ///
    /// Independent of [`Self::filter`], which stays the `v1.2.0` single-pass path and is
    /// byte-identical when [`ShaderStack::Off`] (the default) is selected here. The two are
    /// separate because the stack replaces the *architecture*, not the existing filters, and an
    /// existing `config.toml` must keep rendering exactly as it did.
    pub stack: ShaderStack,
    /// A loaded `.slangp`/`.cgp` preset path (`v1.25.0`, T-FP-E), or `None`.
    ///
    /// Takes precedence over [`Self::stack`] when set: a user who loaded a preset means the preset.
    /// A preset that fails to load falls back to `stack`, with the reason shown in Settings.
    #[serde(default)]
    pub preset_path: Option<String>,
    /// Per-chain parameter overrides, keyed `"<chain>.<param>"` (`v1.25.0`, T-FP-D).
    ///
    /// A flat map rather than typed fields, for the same reason the parameters themselves are a
    /// name-indexed list: a shader's knobs are declared by the *shader*, so a typed config would
    /// mean the Rust side has to know every shader it might ever load.
    #[serde(default)]
    pub stack_params: std::collections::BTreeMap<String, f32>,
    /// The active HD texture pack's name for the current ROM (`v1.3.0`), or `None` (the default
    /// — byte-identical config round-trip for every prior release). Present regardless of
    /// whether this build has the `hd-pack` Cargo feature on, matching every other config field's
    /// posture (`port2_peripheral`, `rewind`, …) — an inert value in a build that can't act on
    /// it, not a compile-time-gated field.
    pub hd_pack_name: Option<String>,
    /// Crop the trailing "overscan" scanlines a real 4:3 CRT wouldn't reliably show (`v1.20.0`,
    /// View → Hide Overscan). SNES hardware's own `SETINI` register (`rustysnes_ppu`) already
    /// distinguishes the standard 224-line display from an extended 239-line one a game can
    /// opt into — this toggle crops exactly that extra 15-line extension back off on the
    /// PRESENTATION side only (`app.rs`'s `crop_overscan`), the same "display-only, never the
    /// deterministic core" boundary every other post-filter in this module already respects.
    /// Additive, `false` by default — byte-identical presentation to every prior release when
    /// unchanged.
    pub hide_overscan: bool,
    /// How framebuffer pixels are shaped when fitted to the window (`v1.25.0`). Default
    /// [`AspectMode::FourThree`] reproduces the previously-hardcoded behaviour exactly.
    pub aspect: AspectMode,
    /// Per-side presentation crop in framebuffer pixels (`v1.25.0`). All-zero by default. This is
    /// the fine-grained companion to [`Self::hide_overscan`], which only drops PAL's 239-line
    /// extension wholesale; both apply, this one second.
    pub overscan: Overscan,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            present_mode: "auto".into(),
            pacing: PacingMode::default(),
            integer_scale: false,
            filter: PostFilter::default(),
            crt_scanline: 0.3,
            crt_mask: 0.15,
            hqx_strength: 0.6,
            xbrz_strength: 0.6,
            stack: ShaderStack::default(),
            preset_path: None,
            stack_params: std::collections::BTreeMap::new(),
            hd_pack_name: None,
            hide_overscan: false,
            aspect: AspectMode::default(),
            overscan: Overscan::default(),
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
    /// Target output latency in milliseconds (`v1.25.0`; clamped to `10..=250` by
    /// [`Self::latency_ms_clamped`]).
    ///
    /// This is the setpoint the dynamic-rate-control servo holds
    /// ([`crate::audio_core::drc_ratio_latency`]) and the size the ring is derived from, replacing
    /// the previous fixed 8192-sample buffer whose achieved latency silently depended on the
    /// device's rate. Lower is more responsive and more dropout-prone; 60 ms matches RustyNES.
    pub latency_ms: u32,
    /// Which interpolation kernel the producer-side resampler uses (`v1.25.0`).
    pub resampler: crate::audio_core::ResampleKernel,
    /// Graphic equaliser (`v1.25.0`).
    pub eq: EqConfig,
    /// Preferred output device name, or `None` for the host default (`v1.25.0`). A name that no
    /// longer matches any present device falls back to the default rather than refusing to start —
    /// devices legitimately disappear between sessions.
    pub device: Option<String>,
}

impl AudioConfig {
    /// [`Self::latency_ms`] clamped to the supported `10..=250` ms range.
    ///
    /// Below ~10 ms no amount of servoing keeps a general-purpose OS audio callback fed; above
    /// 250 ms the added input lag is worse than the dropouts it prevents.
    // Written as comparisons rather than `u32::clamp`, which is not const-callable on the pinned
    // toolchain (`Ord` is not a const trait yet).
    #[must_use]
    pub const fn latency_ms_clamped(&self) -> u32 {
        if self.latency_ms < 10 {
            10
        } else if self.latency_ms > 250 {
            250
        } else {
            self.latency_ms
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            volume: 0.8,
            enabled: true,
            voice_mutes: [false; 8],
            latency_ms: 60,
            resampler: crate::audio_core::ResampleKernel::default(),
            eq: EqConfig::default(),
            device: None,
        }
    }
}

/// Graphic-equaliser settings (`v1.25.0`, `crate::eq`).
///
/// Off by default with every band flat, so the default build's audio path is bit-identical to before
/// this existed — `Equalizer` detects flat and bypasses exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EqConfig {
    /// Whether the EQ stage runs at all.
    pub enabled: bool,
    /// Per-band gain in dB, clamped to +/-12 by `crate::eq::Equalizer::set_gains`. Band centres are
    /// `crate::eq::CENTRES_HZ`.
    pub gains_db: [f32; crate::eq::BANDS],
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            gains_db: [0.0; crate::eq::BANDS],
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
    /// Skip run-ahead for the next displayed frame whenever the previous one took longer than
    /// this many milliseconds (`0` disables the throttle). `v1.25.0`.
    ///
    /// Run-ahead multiplies emulation cost by `frames + 1`, so on a machine that is already
    /// missing its frame deadline it converts a latency improvement into visible stutter. RustyNES
    /// throttles on a frame budget for exactly this reason; without it the feature cannot safely
    /// be on by default.
    pub throttle_ms: f32,
}

/// Autofire ("turbo") settings — hold a face button and the frontend pulses it (`v1.25.0`).
///
/// Purely a host-input convenience: the pulsing happens where host input is sampled, so the core
/// still sees an ordinary button stream and determinism is unaffected. All off by default.
// One independent flag per face button IS the data model here: they are four unrelated toggles the
// user sets individually, not a state machine that a struct/enum would express better.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TurboConfig {
    /// Autofire the A button.
    pub a: bool,
    /// Autofire the B button.
    pub b: bool,
    /// Autofire the X button.
    pub x: bool,
    /// Autofire the Y button.
    pub y: bool,
    /// Full on/off cycle length in frames (minimum 2 — one frame pressed, one released; clamped by
    /// [`Self::period_clamped`]).
    pub period_frames: u32,
}

impl TurboConfig {
    /// [`Self::period_frames`] clamped to a physically meaningful `2..=60`.
    ///
    /// A period of 1 would hold the button permanently (never releasing), i.e. silently *disable*
    /// the feature it looks like it is configuring — hence the floor of 2.
    // Comparisons rather than `u32::clamp` for the same const-callability reason as
    // `AudioConfig::latency_ms_clamped`.
    #[must_use]
    pub const fn period_clamped(&self) -> u32 {
        if self.period_frames < 2 {
            2
        } else if self.period_frames > 60 {
            60
        } else {
            self.period_frames
        }
    }

    /// Whether any button is set to autofire (the fast path skips the per-frame work when not).
    #[must_use]
    pub const fn any(&self) -> bool {
        self.a || self.b || self.x || self.y
    }

    /// The [`crate::input::Buttons`] bit mask of the autofire-enabled buttons.
    #[must_use]
    pub const fn mask(&self) -> u16 {
        use crate::input::Button;
        let mut m = 0u16;
        if self.a {
            m |= Button::A.mask();
        }
        if self.b {
            m |= Button::B.mask();
        }
        if self.x {
            m |= Button::X.mask();
        }
        if self.y {
            m |= Button::Y.mask();
        }
        m
    }
}

impl Default for TurboConfig {
    fn default() -> Self {
        Self {
            a: false,
            b: false,
            x: false,
            y: false,
            // ~8 presses/second at 60 Hz, the usual "turbo controller" rate.
            period_frames: 8,
        }
    }
}

/// Physical gamepad settings (`v1.25.0`, the `gilrs` runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GamepadConfig {
    /// Whether to open the gamepad backend at all. On by default — a machine with no pad simply
    /// enumerates none, and keyboard input is unaffected either way.
    pub enabled: bool,
    /// Analog-stick magnitude below which axis motion is ignored, `0.0..=0.9`.
    ///
    /// Sticks rest slightly off-centre, so without a deadzone a worn pad holds a direction
    /// permanently — which reads as "the d-pad is stuck", not "the stick drifts".
    pub deadzone: f32,
}

impl GamepadConfig {
    /// [`Self::deadzone`] clamped to a usable `0.0..=0.9`.
    // Comparisons rather than `f32::clamp` so this can be `const`, matching the other clamp
    // helpers in this module.
    #[must_use]
    pub const fn deadzone_clamped(&self) -> f32 {
        if self.deadzone < 0.0 {
            0.0
        } else if self.deadzone > 0.9 {
            0.9
        } else {
            self.deadzone
        }
    }
}

impl Default for GamepadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deadzone: 0.35,
        }
    }
}

/// The most-recently-used ROM list (`v1.25.0`).
///
/// Newest first, de-duplicated by path, capped at [`Self::CAP`]. Stored as strings rather than
/// `PathBuf` so the TOML round-trip is platform-agnostic text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecentRoms {
    /// The paths, newest first.
    pub paths: Vec<String>,
}

impl RecentRoms {
    /// Maximum retained entries.
    pub const CAP: usize = 10;

    /// Record `path` as the most recently used ROM, moving an existing entry to the front rather
    /// than duplicating it, and dropping the oldest beyond [`Self::CAP`].
    pub fn touch(&mut self, path: &std::path::Path) {
        let s = path.to_string_lossy().into_owned();
        self.paths.retain(|p| p != &s);
        self.paths.insert(0, s);
        self.paths.truncate(Self::CAP);
    }

    /// Forget every entry (the menu's "Clear" item).
    pub fn clear(&mut self) {
        self.paths.clear();
    }
}

/// The full frontend config (serialized to `config.toml`).
///
/// `Default` is implemented by hand rather than derived because `p2` must NOT be
/// `KeyBindings::default()` — that is the P1 layout, and now that P2 keyboard input is live
/// (`v1.25.0`) an identical table would make every P1 key drive both pads.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Autofire settings (`v1.25.0`).
    pub turbo: TurboConfig,
    /// Physical gamepad settings (`v1.25.0`).
    pub gamepad: GamepadConfig,
    /// Recently-opened ROMs, newest first (`v1.25.0`).
    pub recent: RecentRoms,
    /// Directory screenshots are written to, or `None` for the platform picture dir (`v1.25.0`).
    pub screenshot_dir: Option<String>,
    /// User-interface language (`v1.25.0`, T-FP-A). Defaults to English.
    pub locale: crate::i18n::Locale,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            region: Region::default(),
            video: VideoConfig::default(),
            audio: AudioConfig::default(),
            p1: KeyBindings::default(),
            // The one field that is deliberately not `Default::default()` — see the struct doc.
            p2: KeyBindings::default_p2(),
            port2_peripheral: PeripheralKind::default(),
            rewind: RewindConfig::default(),
            run_ahead: RunAheadConfig::default(),
            theme: AppTheme::default(),
            first_run_seen: false,
            turbo: TurboConfig::default(),
            gamepad: GamepadConfig::default(),
            recent: RecentRoms::default(),
            screenshot_dir: None,
            locale: crate::i18n::Locale::default(),
        }
    }
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
    fn new_v1_25_defaults_preserve_prior_behaviour() {
        let cfg = Config::default();
        // Presentation must be identical to every prior release out of the box.
        assert_eq!(cfg.video.aspect, AspectMode::FourThree);
        assert!(cfg.video.overscan.is_zero());
        assert!(!cfg.video.integer_scale);
        // New input/audio features are off / at RustyNES's defaults.
        assert!(!cfg.turbo.any());
        assert!(cfg.recent.paths.is_empty());
        assert_eq!(cfg.audio.latency_ms, 60);
        assert!(cfg.audio.device.is_none());
    }

    #[test]
    fn aspect_ratio_is_measured_from_the_framebuffer_not_the_region() {
        // 4:3 ignores the framebuffer entirely (it is the television's shape).
        assert!((AspectMode::FourThree.ratio(256, 224) - 4.0 / 3.0).abs() < 1e-6);
        assert!((AspectMode::FourThree.ratio(256, 239) - 4.0 / 3.0).abs() < 1e-6);
        // 8:7 and 1:1 both track the measured height, so PAL differs from NTSC.
        assert!(AspectMode::Par.ratio(256, 239) < AspectMode::Par.ratio(256, 224));
        assert!((AspectMode::Square.ratio(256, 224) - 256.0 / 224.0).abs() < 1e-6);
        // 8:7 sits close to, but not on, 4:3 at 224 lines.
        let par = AspectMode::Par.ratio(256, 224);
        assert!((par - 4.0 / 3.0).abs() < 0.03 && (par - 4.0 / 3.0).abs() > 1e-4);
        // Degenerate dimensions must not divide by zero.
        assert!(AspectMode::Square.ratio(0, 0).is_finite());
    }

    #[test]
    fn overscan_clamp_always_leaves_a_usable_image() {
        // A hand-edited config asking to crop everything still leaves >= 16x16.
        let absurd = Overscan {
            top: 500,
            bottom: 500,
            left: 500,
            right: 500,
        };
        let c = absurd.clamped(256, 224);
        assert!(
            c.left + c.right <= 256 - 16,
            "left+right={}",
            c.left + c.right
        );
        assert!(
            c.top + c.bottom <= 224 - 16,
            "top+bottom={}",
            c.top + c.bottom
        );
        // A sane crop is passed through untouched.
        let sane = Overscan {
            top: 8,
            bottom: 8,
            left: 8,
            right: 8,
        };
        assert_eq!(sane.clamped(256, 224), sane);
        assert!(!sane.is_zero() && Overscan::default().is_zero());
    }

    #[test]
    fn recent_roms_dedupe_promote_and_cap() {
        let mut r = RecentRoms::default();
        for i in 0..(RecentRoms::CAP + 5) {
            r.touch(std::path::Path::new(&format!("/roms/game{i}.sfc")));
        }
        assert_eq!(r.paths.len(), RecentRoms::CAP, "must cap");
        assert!(
            r.paths[0].ends_with(&format!("game{}.sfc", RecentRoms::CAP + 4)),
            "newest first"
        );
        // Re-touching an existing entry promotes it instead of duplicating.
        let again = r.paths[3].clone();
        r.touch(std::path::Path::new(&again));
        assert_eq!(r.paths[0], again);
        assert_eq!(r.paths.iter().filter(|p| **p == again).count(), 1);
        r.clear();
        assert!(r.paths.is_empty());
    }

    #[test]
    fn clamps_reject_values_that_would_disable_the_feature_they_configure() {
        // period 1 would hold the button forever; the floor of 2 is what makes it pulse.
        let t = TurboConfig {
            period_frames: 1,
            ..TurboConfig::default()
        };
        assert_eq!(t.period_clamped(), 2);
        let audio = AudioConfig {
            latency_ms: 0,
            ..AudioConfig::default()
        };
        assert_eq!(audio.latency_ms_clamped(), 10);
        let audio_hi = AudioConfig {
            latency_ms: 10_000,
            ..AudioConfig::default()
        };
        assert_eq!(audio_hi.latency_ms_clamped(), 250);
        let g = GamepadConfig {
            deadzone: 5.0,
            ..GamepadConfig::default()
        };
        assert!((g.deadzone_clamped() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn default_p2_is_not_the_p1_table() {
        // Regression guard on the hand-written `Default`: deriving it would silently give `p2` the
        // P1 layout, and now that P2 keyboard input is live that makes one key drive both pads.
        let cfg = Config::default();
        assert!(
            cfg.p2.conflicts_with(&cfg.p1).is_empty(),
            "Config::default() must not bind the same key to both players"
        );
        // And a config file that omits `p2` entirely still gets the distinct layout.
        let back: Config = toml::from_str("region = \"NTSC\"").expect("deserialize");
        assert!(back.p2.conflicts_with(&back.p1).is_empty());
    }

    #[test]
    fn new_fields_round_trip_through_toml() {
        let mut cfg = Config::default();
        cfg.video.aspect = AspectMode::Par;
        cfg.video.overscan = Overscan {
            top: 8,
            bottom: 8,
            left: 4,
            right: 4,
        };
        cfg.audio.latency_ms = 32;
        cfg.audio.resampler = crate::audio_core::ResampleKernel::Linear;
        cfg.audio.device = Some("Speakers".into());
        cfg.turbo.b = true;
        cfg.gamepad.deadzone = 0.5;
        cfg.recent.touch(std::path::Path::new("/roms/a.sfc"));
        cfg.run_ahead.throttle_ms = 20.0;
        let s = toml::to_string_pretty(&cfg).expect("serialize");
        let back: Config = toml::from_str(&s).expect("deserialize");
        assert_eq!(back.video.aspect, AspectMode::Par);
        assert_eq!(back.video.overscan, cfg.video.overscan);
        assert_eq!(back.audio.latency_ms, 32);
        assert_eq!(
            back.audio.resampler,
            crate::audio_core::ResampleKernel::Linear
        );
        assert_eq!(back.audio.device.as_deref(), Some("Speakers"));
        assert!(back.turbo.b);
        assert!((back.gamepad.deadzone - 0.5).abs() < 1e-6);
        assert_eq!(back.recent.paths.len(), 1);
        assert!((back.run_ahead.throttle_ms - 20.0).abs() < 1e-6);
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
