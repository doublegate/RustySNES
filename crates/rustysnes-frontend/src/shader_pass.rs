//! A multi-pass presentation shader stack (`v1.25.0`, T-FP-D).
//!
//! # What this replaces
//!
//! `Gfx::present` took a `PostFilter` enum plus four hardcoded `f32` arguments threaded through the
//! call, applied exactly **one** filter, and gained a new argument every time a filter gained a
//! knob. That shape cannot express what presentation shaders actually are: an ordered chain, each
//! pass rendering into a target the next one samples, each with its own scale, filtering, and its
//! own parameter set.
//!
//! This module is the description of such a chain plus the ping-pong target management to run it.
//! The per-pass fields are deliberately named after `RetroArch`'s `.slangp` keys (`scale_type`,
//! `filter_linear`, `wrap_mode`, `float_framebuffer`, `mipmap_input`, `alias`, `frame_count_mod`),
//! so T-FP-E's preset parser maps onto this without a translation layer in between.
//!
//! # Why parameters are a uniform block rather than fields
//!
//! A `#pragma parameter` in a shader declares a named, ranged, defaulted knob. Modelling those as
//! struct fields means the Rust side must know every shader's knobs at compile time, which is
//! precisely what makes a shader stack unable to load a shader it was not built with. Here the
//! parameters are a name-indexed list packed into one uniform buffer, so a pass's UI is generated
//! from the pass and adding a knob touches no Rust code.

use std::borrow::Cow;

/// Parameters a single pass can carry, matching the uniform block's `vec4` array size.
///
/// 16 is well past what any real preset uses and keeps the uniform under the 64 KiB minimum
/// guaranteed uniform-buffer binding size with room to spare.
pub const MAX_PARAMS: usize = 16;

/// How a pass's output is sized relative to something else (`.slangp`'s `scale_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScaleType {
    /// A multiple of this pass's *input* size — the default, and what a doubling filter wants.
    #[default]
    Source,
    /// A multiple of the final output viewport, for a pass that must land at display resolution.
    Viewport,
    /// A fixed pixel size regardless of anything else.
    Absolute,
}

/// How a pass samples outside `[0,1]` (`.slangp`'s `wrap_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapMode {
    /// Clamp to the edge texel — the correct default for an image, since repeating an emulator
    /// framebuffer wraps the right edge onto the left and produces a visible seam.
    #[default]
    ClampToEdge,
    /// Repeat.
    Repeat,
    /// Mirror on repeat.
    MirrorRepeat,
}

impl WrapMode {
    /// The wgpu address mode.
    #[must_use]
    pub const fn to_wgpu(self) -> wgpu::AddressMode {
        match self {
            Self::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            Self::Repeat => wgpu::AddressMode::Repeat,
            Self::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

/// One `#pragma parameter`-style knob.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// The shader-side identifier.
    pub name: String,
    /// The label shown in the UI.
    pub label: String,
    /// The current value.
    pub value: f32,
    /// Minimum.
    pub min: f32,
    /// Maximum.
    pub max: f32,
    /// Slider step.
    pub step: f32,
}

impl Param {
    /// A parameter with the usual `0.0..=1.0` range.
    #[must_use]
    pub fn unit(name: &str, label: &str, default: f32) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
            value: default,
            min: 0.0,
            max: 1.0,
            step: 0.01,
        }
    }

    /// A parameter over an arbitrary range.
    #[must_use]
    pub fn ranged(name: &str, label: &str, default: f32, min: f32, max: f32, step: f32) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
            value: default,
            min,
            max,
            step,
        }
    }

    /// The value clamped into its declared range.
    ///
    /// Applied when packing rather than trusted, because a value can reach here from a
    /// hand-edited config or a preset file, and a shader fed an out-of-range parameter produces
    /// garbage rather than an error.
    #[must_use]
    pub fn clamped(&self) -> f32 {
        if self.min <= self.max {
            self.value.clamp(self.min, self.max)
        } else {
            // A preset with min > max is malformed; take the value as-is rather than clamping to
            // an empty range, which would silently pin every such parameter to one number.
            self.value
        }
    }
}

/// One pass in the chain.
#[derive(Debug, Clone)]
pub struct PassDesc {
    /// A human name, used in the UI and for `alias` lookups.
    pub name: String,
    /// The WGSL source.
    pub source: Cow<'static, str>,
    /// How the output is sized.
    pub scale_type: ScaleType,
    /// The scale factor (or the absolute size when `scale_type` is [`ScaleType::Absolute`]).
    pub scale: (f32, f32),
    /// Whether the pass's *input* is sampled linearly.
    pub filter_linear: bool,
    /// Address mode outside `[0,1]`.
    pub wrap_mode: WrapMode,
    /// Whether the output target is a float format, for a pass that needs range beyond `[0,1]`.
    pub float_framebuffer: bool,
    /// Whether the input should be mipmapped.
    pub mipmap_input: bool,
    /// An optional alias a later pass can reference this one by.
    pub alias: Option<String>,
    /// Modulus applied to the frame counter before it reaches the shader, so an animated effect
    /// with an N-frame cycle gets a counter that actually cycles instead of one that grows forever
    /// and loses float precision.
    pub frame_count_mod: u32,
    /// This pass's parameters.
    pub params: Vec<Param>,
}

impl PassDesc {
    /// A 1:1 pass with no parameters.
    #[must_use]
    pub fn new(name: &str, source: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.to_string(),
            source: source.into(),
            scale_type: ScaleType::Source,
            scale: (1.0, 1.0),
            filter_linear: false,
            wrap_mode: WrapMode::ClampToEdge,
            float_framebuffer: false,
            mipmap_input: false,
            alias: None,
            frame_count_mod: 0,
            params: Vec::new(),
        }
    }

    /// Set the scale.
    #[must_use]
    pub const fn with_scale(mut self, scale_type: ScaleType, x: f32, y: f32) -> Self {
        self.scale_type = scale_type;
        self.scale = (x, y);
        self
    }

    /// Set linear input filtering.
    #[must_use]
    pub const fn with_linear(mut self, linear: bool) -> Self {
        self.filter_linear = linear;
        self
    }

    /// Attach parameters.
    #[must_use]
    pub fn with_params(mut self, params: Vec<Param>) -> Self {
        self.params = params;
        self
    }

    /// This pass's output size given its input size and the final viewport.
    ///
    /// Sizes are clamped to at least 1x1: a `scale` of `0.0` in a hand-edited preset would
    /// otherwise ask wgpu for a zero-sized texture, which is a validation panic rather than a
    /// recoverable error.
    #[must_use]
    pub fn output_size(&self, input: (u32, u32), viewport: (u32, u32)) -> (u32, u32) {
        let base = match self.scale_type {
            ScaleType::Source => input,
            ScaleType::Viewport => viewport,
            ScaleType::Absolute => {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                return (
                    (self.scale.0.max(1.0) as u32).max(1),
                    (self.scale.1.max(1.0) as u32).max(1),
                );
            }
        };
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let out = (
            ((base.0 as f32 * self.scale.0).round() as u32).max(1),
            ((base.1 as f32 * self.scale.1).round() as u32).max(1),
        );
        out
    }

    /// The frame counter this pass sees.
    ///
    /// A `frame_count_mod` of `0` means "no modulus" (`RetroArch`'s own convention), not "modulo
    /// zero" — which would be a division by zero.
    #[must_use]
    pub const fn frame_counter(&self, global: u32) -> u32 {
        if self.frame_count_mod == 0 {
            global
        } else {
            global % self.frame_count_mod
        }
    }

    /// Pack this pass's parameters into the uniform layout the shaders read.
    ///
    /// One `f32` per slot, zero-filled past the declared parameters, so a shader reading a slot the
    /// preset did not define gets `0.0` rather than whatever was last in the buffer.
    #[must_use]
    pub fn pack_params(&self) -> [f32; MAX_PARAMS] {
        let mut out = [0.0; MAX_PARAMS];
        for (slot, param) in out.iter_mut().zip(self.params.iter()) {
            *slot = param.clamped();
        }
        out
    }

    /// Look up a parameter by shader-side name.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    /// Mutable lookup, for the UI sliders.
    pub fn param_mut(&mut self, name: &str) -> Option<&mut Param> {
        self.params.iter_mut().find(|p| p.name == name)
    }
}

/// An ordered chain of passes.
#[derive(Debug, Clone, Default)]
pub struct ShaderChain {
    /// The passes, in render order.
    pub passes: Vec<PassDesc>,
    /// A name for the whole chain, shown in the UI.
    pub name: String,
    /// Passes this chain could not build, with the reason.
    ///
    /// Kept **on the chain** rather than discarded, because T-FP-E's GLSL bridge is best-effort by
    /// construction: a preset with one unsupported pass must still run its other passes and say
    /// which one it dropped, never fail silently into a black frame.
    pub unsupported: Vec<Unsupported>,
}

/// A pass that could not be built, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    /// The pass's index in the original preset.
    pub pass: usize,
    /// The pass's name, if the preset gave one.
    pub name: String,
    /// A human-readable reason, shown verbatim in the UI.
    pub reason: String,
}

impl ShaderChain {
    /// An empty chain (the pass-through case).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// A single-pass chain.
    #[must_use]
    pub fn single(pass: PassDesc) -> Self {
        Self {
            name: pass.name.clone(),
            passes: vec![pass],
            unsupported: Vec::new(),
        }
    }

    /// Whether the chain would render nothing (so the caller should blit directly).
    #[must_use]
    pub const fn is_passthrough(&self) -> bool {
        self.passes.is_empty()
    }

    /// The output size of every pass, given the framebuffer size and the final viewport.
    ///
    /// Each pass's input is the previous pass's output, which is the whole point of computing these
    /// as a chain rather than per pass.
    #[must_use]
    pub fn output_sizes(&self, source: (u32, u32), viewport: (u32, u32)) -> Vec<(u32, u32)> {
        let mut sizes = Vec::with_capacity(self.passes.len());
        let mut input = source;
        for pass in &self.passes {
            let out = pass.output_size(input, viewport);
            sizes.push(out);
            input = out;
        }
        sizes
    }

    /// Whether any intermediate target needs a float format.
    ///
    /// Only intermediates matter: the last pass renders to the surface, whose format the swapchain
    /// owns.
    #[must_use]
    pub fn needs_float_targets(&self) -> bool {
        let last = self.passes.len().saturating_sub(1);
        self.passes.iter().take(last).any(|p| p.float_framebuffer)
    }

    /// Find a pass by its alias (or, failing that, its name).
    #[must_use]
    pub fn find(&self, key: &str) -> Option<&PassDesc> {
        self.passes
            .iter()
            .find(|p| p.alias.as_deref() == Some(key))
            .or_else(|| self.passes.iter().find(|p| p.name == key))
    }
}

/// Build a built-in chain from the config selection, applying any saved parameter overrides.
///
/// Overrides are keyed `"<chain>.<param>"` and applied by **name**, so a saved value for a knob a
/// shader no longer declares is ignored rather than landing in the wrong slot — which is exactly
/// what a positional format would do after a shader is edited.
#[must_use]
pub fn build_chain(
    selection: crate::config::ShaderStack,
    overrides: &std::collections::BTreeMap<String, f32>,
) -> ShaderChain {
    use crate::config::ShaderStack;
    let mut chain = match selection {
        ShaderStack::Off => return ShaderChain::empty(),
        ShaderStack::Crt => ShaderChain {
            name: "CRT".into(),
            passes: vec![crt_pass()],
            unsupported: Vec::new(),
        },
        ShaderStack::Ntsc => ShaderChain {
            name: "NTSC".into(),
            passes: vec![ntsc_pass()],
            unsupported: Vec::new(),
        },
        // NTSC first, then CRT: composite artefacts happen in the signal, scanlines and the
        // aperture mask happen at the phosphor. Reversing them would smear the mask itself.
        ShaderStack::NtscCrt => ShaderChain {
            name: "NTSC + CRT".into(),
            passes: vec![ntsc_pass(), crt_pass()],
            unsupported: Vec::new(),
        },
    };
    let prefix = selection.display_name().to_string();
    for pass in &mut chain.passes {
        for param in &mut pass.params {
            if let Some(v) = overrides.get(&format!("{prefix}.{}", param.name)) {
                param.value = *v;
            }
        }
    }
    chain
}

/// The richer CRT pass with its declared knobs.
#[must_use]
pub fn crt_pass() -> PassDesc {
    PassDesc::new("crt", rustysnes_gfx_shaders::CRT_STACK_WGSL)
        .with_scale(ScaleType::Viewport, 1.0, 1.0)
        .with_params(vec![
            Param::unit("scanline", "Scanlines", 0.3),
            Param::unit("mask", "Aperture mask", 0.15),
            Param::unit("curvature", "Curvature", 0.0),
            Param::unit("beam", "Beam shape", 0.4),
            Param::unit("glow", "Glow", 0.0),
            Param::unit("vignette", "Vignette", 0.0),
        ])
}

/// The NTSC composite pass with its declared knobs.
#[must_use]
pub fn ntsc_pass() -> PassDesc {
    PassDesc::new("ntsc", rustysnes_gfx_shaders::NTSC_STACK_WGSL)
        .with_scale(ScaleType::Source, 1.0, 1.0)
        .with_params(vec![
            Param::unit("bleed", "Chroma bleed", 0.5),
            Param::unit("artifacts", "Artifacts", 0.3),
            Param::unit("fringing", "Fringing", 0.2),
            Param::unit("crawl", "Dot crawl", 0.3),
        ])
}

#[cfg(test)]
mod tests {
    use super::{MAX_PARAMS, Param, PassDesc, ScaleType, ShaderChain, Unsupported, WrapMode};

    fn pass(name: &str) -> PassDesc {
        PassDesc::new(name, "// wgsl")
    }

    /// Each pass's input is the previous pass's output — the reason sizes are computed as a chain.
    #[test]
    fn chained_sizes_compound() {
        let chain = ShaderChain {
            name: "test".into(),
            passes: vec![
                pass("a").with_scale(ScaleType::Source, 2.0, 2.0),
                pass("b").with_scale(ScaleType::Source, 2.0, 2.0),
                pass("c").with_scale(ScaleType::Viewport, 1.0, 1.0),
            ],
            unsupported: Vec::new(),
        };
        let sizes = chain.output_sizes((256, 224), (1920, 1080));
        assert_eq!(sizes, vec![(512, 448), (1024, 896), (1920, 1080)]);
    }

    /// `Viewport` ignores the input entirely; `Absolute` ignores both.
    #[test]
    fn scale_types_key_off_the_right_thing() {
        let p = pass("v").with_scale(ScaleType::Viewport, 0.5, 0.5);
        assert_eq!(p.output_size((256, 224), (1920, 1080)), (960, 540));
        let a = pass("a").with_scale(ScaleType::Absolute, 640.0, 480.0);
        assert_eq!(a.output_size((256, 224), (1920, 1080)), (640, 480));
        assert_eq!(a.output_size((1, 1), (1, 1)), (640, 480));
    }

    /// A zero or negative scale from a hand-edited preset must not reach wgpu, which panics on a
    /// zero-sized texture rather than returning an error.
    #[test]
    fn degenerate_scales_clamp_to_one_pixel() {
        for scale in [0.0_f32, -1.0, 0.001] {
            let p = pass("z").with_scale(ScaleType::Source, scale, scale);
            let (w, h) = p.output_size((256, 224), (1920, 1080));
            assert!(w >= 1 && h >= 1, "scale {scale} produced {w}x{h}");
        }
        let a = pass("z").with_scale(ScaleType::Absolute, 0.0, 0.0);
        assert_eq!(a.output_size((256, 224), (1920, 1080)), (1, 1));
    }

    /// `frame_count_mod = 0` means "no modulus", not "modulo zero".
    #[test]
    fn frame_counter_modulus_zero_is_no_modulus() {
        let mut p = pass("f");
        assert_eq!(p.frame_counter(12_345), 12_345);
        p.frame_count_mod = 4;
        assert_eq!(p.frame_counter(12_345), 1);
        assert_eq!(p.frame_counter(0), 0);
    }

    /// Parameters pack in declaration order, clamped, with unused slots zeroed — a shader reading
    /// an undeclared slot must not see whatever was last in the buffer.
    #[test]
    fn params_pack_clamped_and_zero_filled() {
        let p = pass("p").with_params(vec![
            Param::unit("a", "A", 0.5),
            Param::ranged("b", "B", 99.0, 0.0, 1.0, 0.1),
            Param::ranged("c", "C", -5.0, 0.0, 10.0, 1.0),
        ]);
        let packed = p.pack_params();
        assert!((packed[0] - 0.5).abs() < 1e-6);
        assert!((packed[1] - 1.0).abs() < 1e-6, "clamped to max");
        assert!((packed[2] - 0.0).abs() < 1e-6, "clamped to min");
        for slot in &packed[3..] {
            assert!(slot.abs() < 1e-6, "unused slots must be zero");
        }
        assert_eq!(packed.len(), MAX_PARAMS);
    }

    /// A malformed range (min > max) takes the value rather than pinning to an empty range.
    #[test]
    fn inverted_ranges_do_not_pin_the_value() {
        let p = Param::ranged("x", "X", 0.7, 1.0, 0.0, 0.1);
        assert!((p.clamped() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn params_are_addressable_by_name() {
        let mut p = pass("p").with_params(vec![Param::unit("scanline", "Scanlines", 0.3)]);
        assert!((p.param("scanline").expect("present").value - 0.3).abs() < 1e-6);
        assert!(p.param("nope").is_none());
        p.param_mut("scanline").expect("present").value = 0.9;
        assert!((p.param("scanline").expect("present").value - 0.9).abs() < 1e-6);
    }

    /// Only *intermediate* targets can be float; the final pass renders to the surface.
    #[test]
    fn float_targets_are_only_needed_for_intermediates() {
        let mut a = pass("a");
        a.float_framebuffer = true;
        let mut b = pass("b");
        b.float_framebuffer = true;

        let last_only = ShaderChain {
            name: "x".into(),
            passes: vec![pass("a"), b],
            unsupported: Vec::new(),
        };
        assert!(
            !last_only.needs_float_targets(),
            "the last pass renders to the surface"
        );

        let with_intermediate = ShaderChain {
            name: "x".into(),
            passes: vec![a, pass("b")],
            unsupported: Vec::new(),
        };
        assert!(with_intermediate.needs_float_targets());
    }

    /// A pass is found by alias first, then by name — that is the `.slangp` lookup order.
    #[test]
    fn lookup_prefers_alias_over_name() {
        let mut aliased = pass("second");
        aliased.alias = Some("first".into());
        let chain = ShaderChain {
            name: "x".into(),
            passes: vec![pass("first"), aliased],
            unsupported: Vec::new(),
        };
        assert_eq!(chain.find("first").expect("found").name, "second");
        assert_eq!(chain.find("second").expect("found").name, "second");
        assert!(chain.find("third").is_none());
    }

    /// A chain carries what it could not build, so a partially-supported preset says which pass it
    /// dropped instead of rendering a black frame.
    #[test]
    fn unsupported_passes_are_carried_not_discarded() {
        let chain = ShaderChain {
            name: "partial".into(),
            passes: vec![pass("a")],
            unsupported: vec![Unsupported {
                pass: 1,
                name: "crt-royale-blur".into(),
                reason: "GLSL frontend: unsupported `texture2DLod`".into(),
            }],
        };
        assert!(!chain.is_passthrough());
        assert_eq!(chain.unsupported.len(), 1);
        assert!(chain.unsupported[0].reason.contains("texture2DLod"));
    }

    /// `Off` is an empty chain, which is what makes the stack byte-identical to a build without
    /// it — the caller falls straight through to the plain blit.
    #[test]
    fn off_builds_an_empty_chain() {
        use crate::config::ShaderStack;
        let chain = super::build_chain(ShaderStack::Off, &std::collections::BTreeMap::default());
        assert!(chain.is_passthrough());
    }

    /// NTSC runs before CRT: composite artefacts happen in the signal, the mask at the phosphor.
    /// Reversing them would smear the mask itself.
    #[test]
    fn the_combined_chain_orders_ntsc_before_crt() {
        use crate::config::ShaderStack;
        let chain =
            super::build_chain(ShaderStack::NtscCrt, &std::collections::BTreeMap::default());
        assert_eq!(chain.passes.len(), 2);
        assert_eq!(chain.passes[0].name, "ntsc");
        assert_eq!(chain.passes[1].name, "crt");
    }

    /// Overrides apply by NAME, so a saved value for a knob a shader no longer declares is
    /// ignored rather than landing in the wrong slot — which is what a positional format would do.
    #[test]
    fn parameter_overrides_apply_by_name() {
        use crate::config::ShaderStack;
        let mut overrides = std::collections::BTreeMap::new();
        overrides.insert("CRT.scanline".to_string(), 0.9);
        overrides.insert("CRT.no_such_knob".to_string(), 0.5);
        overrides.insert("NTSC.bleed".to_string(), 0.1);
        let chain = super::build_chain(ShaderStack::Crt, &overrides);
        let crt = &chain.passes[0];
        assert!((crt.param("scanline").expect("k").value - 0.9).abs() < 1e-6);
        // A default the override did not name is untouched, and an unknown key changed nothing.
        assert!((crt.param("mask").expect("k").value - 0.15).abs() < 1e-6);
        assert!(crt.param("no_such_knob").is_none());
        // A key for a different chain does not leak across.
        assert!(crt.param("bleed").is_none());
    }

    #[test]
    fn an_empty_chain_is_passthrough() {
        assert!(ShaderChain::empty().is_passthrough());
        assert!(!ShaderChain::single(pass("a")).is_passthrough());
    }

    /// Clamp-to-edge is the default: repeating an emulator framebuffer wraps the right edge onto
    /// the left and produces a visible seam.
    #[test]
    fn default_wrap_is_clamp_to_edge() {
        assert_eq!(WrapMode::default(), WrapMode::ClampToEdge);
        assert_eq!(
            WrapMode::default().to_wgpu(),
            wgpu::AddressMode::ClampToEdge
        );
        assert_eq!(WrapMode::Repeat.to_wgpu(), wgpu::AddressMode::Repeat);
    }
}
