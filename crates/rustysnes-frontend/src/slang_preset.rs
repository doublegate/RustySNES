//! `.slangp` / `.cgp` shader-preset parsing (`v1.25.0`, T-FP-E).
//!
//! A `RetroArch` preset is a flat `key = value` file where the *index* is part of the key
//! (`scale_type0`, `filter_linear1`, …). This parses that into the per-pass description
//! [`crate::shader_pass::PassDesc`] already uses — which is why T-FP-D named its fields after these
//! keys in the first place.
//!
//! # What is parsed and what is ignored
//!
//! Every documented per-pass key that maps onto a field the stack can honour. Keys that do not —
//! `srgb_framebufferN` on a backend where the surface format already decides that, and the
//! `#reference` include directive — are **recorded as ignored rather than silently dropped**, so a
//! preset that renders differently than its author intended says which key was responsible.
//!
//! Parsing is deliberately separate from *building*: this module turns text into a description and
//! never touches a GPU or a shader compiler. [`crate::glsl_bridge`] is what may fail to translate a
//! pass, and it reports that as a named [`crate::shader_pass::Unsupported`] rather than an error
//! that discards the whole preset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::shader_pass::{ScaleType, WrapMode};

/// A parsed preset, before any shader is read or compiled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Preset {
    /// One entry per pass, in render order.
    pub passes: Vec<PresetPass>,
    /// `textures` LUT declarations, by name.
    pub textures: BTreeMap<String, PresetTexture>,
    /// Preset-level `#pragma parameter` overrides, by parameter name.
    pub parameters: BTreeMap<String, f32>,
    /// Keys the parser recognised but this stack cannot honour, with the reason.
    ///
    /// Kept rather than dropped: a preset that renders differently from its author's intent should
    /// be able to say which key was responsible.
    pub ignored: Vec<IgnoredKey>,
}

/// A key that was recognised but not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredKey {
    /// The key as written in the preset.
    pub key: String,
    /// Why it was not applied.
    pub reason: &'static str,
}

/// One pass's declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetPass {
    /// The shader path, relative to the preset.
    pub shader: PathBuf,
    /// `filter_linearN`.
    pub filter_linear: bool,
    /// `wrap_modeN`.
    pub wrap_mode: WrapMode,
    /// `scale_typeN` / `scale_type_xN` / `scale_type_yN`, resolved per axis.
    pub scale_type: (ScaleType, ScaleType),
    /// `scaleN` / `scale_xN` / `scale_yN`.
    pub scale: (f32, f32),
    /// `float_framebufferN`.
    pub float_framebuffer: bool,
    /// `mipmap_inputN`.
    pub mipmap_input: bool,
    /// `aliasN`.
    pub alias: Option<String>,
    /// `frame_count_modN`.
    pub frame_count_mod: u32,
}

impl Default for PresetPass {
    fn default() -> Self {
        Self {
            shader: PathBuf::new(),
            filter_linear: false,
            wrap_mode: WrapMode::ClampToEdge,
            // `RetroArch`'s own default is `source` at 1.0, which is the identity — a pass that says
            // nothing about scale must not change the size.
            scale_type: (ScaleType::Source, ScaleType::Source),
            scale: (1.0, 1.0),
            float_framebuffer: false,
            mipmap_input: false,
            alias: None,
            frame_count_mod: 0,
        }
    }
}

/// A LUT texture the preset declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetTexture {
    /// Path relative to the preset.
    pub path: PathBuf,
    /// `<name>_linear`.
    pub linear: bool,
    /// `<name>_mipmap`.
    pub mipmap: bool,
    /// `<name>_wrap_mode`.
    pub wrap_mode: WrapMode,
}

/// Why a preset could not be parsed at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetError {
    /// No `shaders` count, or a count of zero.
    NoPasses,
    /// `shaders` was present but not a number.
    BadShaderCount(String),
    /// A declared pass had no `shaderN` key.
    MissingShaderPath(usize),
    /// The count claimed more passes than the file declares.
    TooFewPasses {
        /// What `shaders` said.
        declared: usize,
        /// How many `shaderN` keys were actually found.
        found: usize,
    },
}

impl core::fmt::Display for PresetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoPasses => write!(f, "preset declares no passes (`shaders` missing or zero)"),
            Self::BadShaderCount(s) => write!(f, "`shaders` is not a number: {s}"),
            Self::MissingShaderPath(i) => write!(f, "pass {i} has no `shader{i}` path"),
            Self::TooFewPasses { declared, found } => {
                write!(
                    f,
                    "`shaders = {declared}` but only {found} shader paths found"
                )
            }
        }
    }
}

impl core::error::Error for PresetError {}

/// Parse a preset's text.
///
/// `base` is the preset's own directory; every path in the result is resolved against it, because a
/// preset's paths are relative to the preset and resolving them later against the *working
/// directory* is the classic way a preset loads for its author and for nobody else.
///
/// # Errors
/// [`PresetError`] when the file declares no usable passes.
pub fn parse(text: &str, base: &Path) -> Result<Preset, PresetError> {
    let kv = parse_kv(text);
    let count_text = kv.get("shaders").ok_or(PresetError::NoPasses)?;
    let count: usize = count_text
        .parse()
        .map_err(|_| PresetError::BadShaderCount(count_text.clone()))?;
    if count == 0 {
        return Err(PresetError::NoPasses);
    }

    let mut preset = Preset::default();
    for (found, i) in (0..count).enumerate() {
        let Some(shader) = kv.get(&format!("shader{i}")) else {
            // Stop at the first gap and report both numbers, so "the file is truncated" and "the
            // count is wrong" are distinguishable.
            return Err(if found == 0 {
                PresetError::MissingShaderPath(i)
            } else {
                PresetError::TooFewPasses {
                    declared: count,
                    found,
                }
            });
        };
        preset.passes.push(pass_from_kv(&kv, i, base, shader));
    }

    parse_textures(&kv, base, &mut preset);
    parse_parameters(&kv, &mut preset);
    note_ignored(&kv, count, &mut preset);
    Ok(preset)
}

/// Build one pass from the key-value map.
fn pass_from_kv(kv: &BTreeMap<String, String>, i: usize, base: &Path, shader: &str) -> PresetPass {
    let mut pass = PresetPass {
        shader: base.join(strip_quotes(shader)),
        ..PresetPass::default()
    };
    if let Some(v) = kv.get(&format!("filter_linear{i}")) {
        pass.filter_linear = parse_bool(v);
    }
    if let Some(v) = kv.get(&format!("wrap_mode{i}")) {
        pass.wrap_mode = parse_wrap(v);
    }
    if let Some(v) = kv.get(&format!("float_framebuffer{i}")) {
        pass.float_framebuffer = parse_bool(v);
    }
    if let Some(v) = kv.get(&format!("mipmap_input{i}")) {
        pass.mipmap_input = parse_bool(v);
    }
    if let Some(v) = kv.get(&format!("alias{i}"))
        && !v.trim().is_empty()
    {
        pass.alias = Some(strip_quotes(v).to_string());
    }
    if let Some(v) = kv.get(&format!("frame_count_mod{i}")) {
        pass.frame_count_mod = v.trim().parse().unwrap_or(0);
    }

    // Per-axis keys override the combined one, which is `RetroArch`'s own precedence: `scale_type`
    // sets both axes, then `scale_type_x`/`scale_type_y` refine either.
    let both = kv
        .get(&format!("scale_type{i}"))
        .map(|v| parse_scale_type(v));
    let x = kv
        .get(&format!("scale_type_x{i}"))
        .map(|v| parse_scale_type(v))
        .or(both);
    let y = kv
        .get(&format!("scale_type_y{i}"))
        .map(|v| parse_scale_type(v))
        .or(both);
    pass.scale_type = (
        x.unwrap_or(ScaleType::Source),
        y.unwrap_or(ScaleType::Source),
    );

    let both_scale = kv.get(&format!("scale{i}")).and_then(|v| parse_f32(v));
    let sx = kv
        .get(&format!("scale_x{i}"))
        .and_then(|v| parse_f32(v))
        .or(both_scale);
    let sy = kv
        .get(&format!("scale_y{i}"))
        .and_then(|v| parse_f32(v))
        .or(both_scale);
    pass.scale = (sx.unwrap_or(1.0), sy.unwrap_or(1.0));
    pass
}

/// Collect the `textures` LUT declarations.
fn parse_textures(kv: &BTreeMap<String, String>, base: &Path, preset: &mut Preset) {
    let Some(list) = kv.get("textures") else {
        return;
    };
    for name in list.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let Some(path) = kv.get(name) else {
            preset.ignored.push(IgnoredKey {
                key: name.to_string(),
                reason: "declared in `textures` but has no path key",
            });
            continue;
        };
        preset.textures.insert(
            name.to_string(),
            PresetTexture {
                path: base.join(strip_quotes(path)),
                linear: kv
                    .get(&format!("{name}_linear"))
                    .is_some_and(|v| parse_bool(v)),
                mipmap: kv
                    .get(&format!("{name}_mipmap"))
                    .is_some_and(|v| parse_bool(v)),
                wrap_mode: kv
                    .get(&format!("{name}_wrap_mode"))
                    .map_or(WrapMode::ClampToEdge, |v| parse_wrap(v)),
            },
        );
    }
}

/// Collect preset-level parameter overrides.
///
/// A preset may list them under `parameters` *or* simply set `name = value` for a parameter the
/// shader declares. Only the explicit list is taken here, because the second form is
/// indistinguishable from an unrecognised key without the shader in hand — and guessing would let
/// a typo silently become a parameter.
fn parse_parameters(kv: &BTreeMap<String, String>, preset: &mut Preset) {
    let Some(list) = kv.get("parameters") else {
        return;
    };
    for name in list.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(v) = kv.get(name).and_then(|v| parse_f32(v)) {
            preset.parameters.insert(name.to_string(), v);
        } else {
            preset.ignored.push(IgnoredKey {
                key: name.to_string(),
                reason: "listed in `parameters` but has no numeric value",
            });
        }
    }
}

/// Record recognised-but-unhonoured keys.
fn note_ignored(kv: &BTreeMap<String, String>, count: usize, preset: &mut Preset) {
    if kv.contains_key("#reference") {
        preset.ignored.push(IgnoredKey {
            key: "#reference".to_string(),
            reason: "preset inheritance is not supported; load the referenced preset directly",
        });
    }
    for i in 0..count {
        let key = format!("srgb_framebuffer{i}");
        if kv.get(&key).is_some_and(|v| parse_bool(v)) {
            preset.ignored.push(IgnoredKey {
                key,
                reason: "sRGB intermediate targets are not supported; the pass renders linear",
            });
        }
    }
}

/// Split a preset into its `key = value` pairs.
///
/// Comments start with `#`, except `#reference`, which is a directive rather than a comment and is
/// recorded so [`note_ignored`] can report it. Values keep their inner spaces and lose surrounding
/// quotes.
fn parse_kv(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#reference") {
            out.insert("#reference".to_string(), rest.trim().to_string());
            continue;
        }
        if line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.split('#').next().unwrap_or("").trim();
        out.insert(key.trim().to_string(), strip_quotes(value).to_string());
    }
    out
}

/// Drop matching surrounding quotes.
fn strip_quotes(s: &str) -> &str {
    let t = s.trim();
    t.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(t)
}

/// `"true"`, `"1"`, and `"yes"` are true; everything else is false.
fn parse_bool(v: &str) -> bool {
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

/// Parse a float, tolerating a trailing comment or stray text.
fn parse_f32(v: &str) -> Option<f32> {
    v.split_whitespace().next()?.parse().ok()
}

/// `"source"` / `"viewport"` / `"absolute"`, defaulting to source.
fn parse_scale_type(v: &str) -> ScaleType {
    match v.trim().to_ascii_lowercase().as_str() {
        "viewport" => ScaleType::Viewport,
        "absolute" => ScaleType::Absolute,
        _ => ScaleType::Source,
    }
}

/// `wrap_mode` spellings `RetroArch` accepts.
fn parse_wrap(v: &str) -> WrapMode {
    match v.trim().to_ascii_lowercase().as_str() {
        "repeat" => WrapMode::Repeat,
        "mirrored_repeat" | "mirror_repeat" => WrapMode::MirrorRepeat,
        _ => WrapMode::ClampToEdge,
    }
}

/// Load a preset from disk and translate each pass into a [`crate::shader_pass::ShaderChain`].
///
/// **Best-effort by construction.** A pass whose shader cannot be read or translated becomes a
/// named [`crate::shader_pass::Unsupported`] on the chain; the remaining passes still run. That is
/// the whole design: naga's GLSL frontend covers a subset of `#version 450`, so a preset that only
/// partly translates is the normal case, not an error case.
///
/// # Errors
/// Only when the preset file itself is unusable ([`PresetError`]) — a shader-level failure is
/// carried on the chain instead.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_chain(path: &Path) -> Result<crate::shader_pass::ShaderChain, PresetError> {
    let text = std::fs::read_to_string(path).map_err(|_| PresetError::NoPasses)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let preset = parse(&text, base)?;
    Ok(to_chain(
        &preset,
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("preset"),
        |p| std::fs::read_to_string(p).ok(),
    ))
}

/// Turn a parsed preset into a chain, reading each shader through `read`.
///
/// `read` is a parameter so this is testable without touching the filesystem — the alternative is
/// testing preset loading only through real files, which is exactly the slow, brittle loop that
/// lets a path-resolution bug survive.
#[must_use]
pub fn to_chain<F>(preset: &Preset, name: &str, mut read: F) -> crate::shader_pass::ShaderChain
where
    F: FnMut(&Path) -> Option<String>,
{
    use crate::shader_pass::{PassDesc, ShaderChain, Unsupported};

    let mut chain = ShaderChain {
        name: name.to_string(),
        passes: Vec::new(),
        unsupported: Vec::new(),
    };
    for (i, pass) in preset.passes.iter().enumerate() {
        let label = pass
            .shader
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("pass")
            .to_string();
        let Some(source) = read(&pass.shader) else {
            chain.unsupported.push(Unsupported {
                pass: i,
                name: label,
                reason: format!("could not read {}", pass.shader.display()),
            });
            continue;
        };
        let translated = match crate::glsl_bridge::translate(&source, i, &label) {
            Ok(t) => t,
            Err(e) => {
                chain.unsupported.push(e);
                continue;
            }
        };

        // Parameters come from the shader's own `#pragma parameter` lines, then the preset's
        // overrides are applied by NAME — a preset may set only some of a shader's knobs, and a
        // positional application would put a value on the wrong one.
        let mut params = crate::glsl_bridge::parameters_of(&source);
        for param in &mut params {
            if let Some(v) = preset.parameters.get(&param.name) {
                param.value = *v;
            }
        }

        let mut desc = PassDesc::new(&label, translated.wgsl)
            .with_linear(pass.filter_linear)
            .with_params(params);
        // The stack carries one scale type; a preset with different types per axis takes the X
        // one and the mismatch is recorded, rather than silently rendering at a size neither axis
        // asked for.
        desc.scale_type = pass.scale_type.0;
        desc.scale = pass.scale;
        desc.wrap_mode = pass.wrap_mode;
        desc.float_framebuffer = pass.float_framebuffer;
        desc.mipmap_input = pass.mipmap_input;
        desc.alias.clone_from(&pass.alias);
        desc.frame_count_mod = pass.frame_count_mod;
        if pass.scale_type.0 != pass.scale_type.1 {
            chain.unsupported.push(Unsupported {
                pass: i,
                name: desc.name.clone(),
                reason: format!(
                    "per-axis scale types differ ({:?} / {:?}); using the X axis for both",
                    pass.scale_type.0, pass.scale_type.1
                ),
            });
        }
        chain.passes.push(desc);
    }
    chain
}

#[cfg(test)]
mod tests {
    use super::{Preset, PresetError, parse};
    use crate::shader_pass::{ScaleType, WrapMode};
    use std::path::Path;

    fn base() -> &'static Path {
        Path::new("/presets")
    }

    #[test]
    fn parses_a_minimal_two_pass_preset() {
        let text = "\
shaders = 2
shader0 = first.slang
shader1 = second.slang
";
        let p = parse(text, base()).expect("parse");
        assert_eq!(p.passes.len(), 2);
        assert_eq!(p.passes[0].shader, Path::new("/presets/first.slang"));
        assert_eq!(p.passes[1].shader, Path::new("/presets/second.slang"));
        // Defaults: `RetroArch`'s own identity, so a pass that says nothing changes nothing.
        assert_eq!(p.passes[0].scale, (1.0, 1.0));
        assert_eq!(
            p.passes[0].scale_type,
            (ScaleType::Source, ScaleType::Source)
        );
        assert!(!p.passes[0].filter_linear);
        assert_eq!(p.passes[0].wrap_mode, WrapMode::ClampToEdge);
    }

    /// Paths resolve against the **preset's** directory. Resolving against the working directory
    /// is the classic way a preset loads for its author and for nobody else.
    #[test]
    fn shader_paths_resolve_against_the_preset_directory() {
        let text = "shaders = 1\nshader0 = shaders/crt/crt-lottes.slang\n";
        let p = parse(text, Path::new("/home/u/presets/crt")).expect("parse");
        assert_eq!(
            p.passes[0].shader,
            Path::new("/home/u/presets/crt/shaders/crt/crt-lottes.slang")
        );
    }

    /// Per-axis keys refine the combined one — `RetroArch`'s own precedence.
    #[test]
    fn per_axis_scale_keys_override_the_combined_one() {
        let text = "\
shaders = 1
shader0 = a.slang
scale_type0 = source
scale0 = 2.0
scale_type_y0 = viewport
scale_y0 = 1.0
";
        let p = parse(text, base()).expect("parse");
        assert_eq!(
            p.passes[0].scale_type,
            (ScaleType::Source, ScaleType::Viewport)
        );
        assert!((p.passes[0].scale.0 - 2.0).abs() < 1e-6);
        assert!((p.passes[0].scale.1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn per_pass_flags_parse() {
        let text = "\
shaders = 2
shader0 = a.slang
shader1 = b.slang
filter_linear0 = true
wrap_mode0 = mirrored_repeat
float_framebuffer0 = \"true\"
mipmap_input1 = 1
alias1 = \"BlurPass\"
frame_count_mod1 = 4
";
        let p = parse(text, base()).expect("parse");
        assert!(p.passes[0].filter_linear);
        assert_eq!(p.passes[0].wrap_mode, WrapMode::MirrorRepeat);
        assert!(p.passes[0].float_framebuffer);
        assert!(!p.passes[1].float_framebuffer, "flags are per pass");
        assert!(p.passes[1].mipmap_input);
        assert_eq!(p.passes[1].alias.as_deref(), Some("BlurPass"));
        assert_eq!(p.passes[1].frame_count_mod, 4);
    }

    #[test]
    fn textures_and_parameters_are_collected() {
        let text = "\
shaders = 1
shader0 = a.slang
textures = \"MASK;NOISE\"
MASK = luts/mask.png
MASK_linear = true
MASK_wrap_mode = repeat
NOISE = luts/noise.png
parameters = \"brightness;contrast\"
brightness = 0.75
contrast = 1.25
";
        let p = parse(text, base()).expect("parse");
        assert_eq!(p.textures.len(), 2);
        let mask = p.textures.get("MASK").expect("MASK");
        assert_eq!(mask.path, Path::new("/presets/luts/mask.png"));
        assert!(mask.linear);
        assert_eq!(mask.wrap_mode, WrapMode::Repeat);
        assert!(!p.textures.get("NOISE").expect("NOISE").linear);
        assert!((p.parameters["brightness"] - 0.75).abs() < 1e-6);
        assert!((p.parameters["contrast"] - 1.25).abs() < 1e-6);
    }

    /// A recognised key that cannot be honoured is **recorded**, so a preset that renders
    /// differently from its author's intent can say which key was responsible.
    #[test]
    fn unhonoured_keys_are_recorded_not_dropped() {
        let text = "\
#reference other.slangp
shaders = 1
shader0 = a.slang
srgb_framebuffer0 = true
textures = \"GHOST\"
parameters = \"missing\"
";
        let p = parse(text, base()).expect("parse");
        let keys: Vec<&str> = p.ignored.iter().map(|i| i.key.as_str()).collect();
        assert!(keys.contains(&"#reference"), "{keys:?}");
        assert!(keys.contains(&"srgb_framebuffer0"), "{keys:?}");
        assert!(keys.contains(&"GHOST"), "a texture with no path: {keys:?}");
        assert!(keys.contains(&"missing"), "a parameter with no value");
        for entry in &p.ignored {
            assert!(!entry.reason.is_empty(), "every reason must be stated");
        }
    }

    /// A comment is a comment, but `#reference` is a directive — telling them apart is why the
    /// lexer special-cases it rather than dropping every `#` line.
    #[test]
    fn comments_are_ignored_but_reference_is_not() {
        let text = "\
# this is a comment
shaders = 1  # trailing comment
shader0 = a.slang
// also a comment
";
        let p = parse(text, base()).expect("parse");
        assert_eq!(p.passes.len(), 1);
        assert!(p.ignored.is_empty());
    }

    /// A malformed preset is refused with a reason, never half-loaded into something that renders
    /// wrongly.
    #[test]
    fn malformed_presets_are_refused_with_a_reason() {
        assert_eq!(parse("", base()), Err(PresetError::NoPasses));
        assert_eq!(parse("shaders = 0\n", base()), Err(PresetError::NoPasses));
        assert!(matches!(
            parse("shaders = many\n", base()),
            Err(PresetError::BadShaderCount(_))
        ));
        assert_eq!(
            parse("shaders = 1\n", base()),
            Err(PresetError::MissingShaderPath(0))
        );
        // A count larger than the declared paths names both numbers, so "truncated file" and
        // "wrong count" are distinguishable.
        assert_eq!(
            parse("shaders = 3\nshader0 = a.slang\n", base()),
            Err(PresetError::TooFewPasses {
                declared: 3,
                found: 1
            })
        );
        for e in [
            PresetError::NoPasses,
            PresetError::BadShaderCount("x".into()),
            PresetError::MissingShaderPath(2),
            PresetError::TooFewPasses {
                declared: 3,
                found: 1,
            },
        ] {
            assert!(!e.to_string().is_empty());
        }
    }

    /// A pass that cannot be read or translated becomes a NAMED entry and the rest still runs —
    /// the whole point of the best-effort design.
    #[test]
    fn an_untranslatable_pass_is_named_and_the_rest_survive() {
        let text = "\
shaders = 3
shader0 = good.slang
shader1 = missing.slang
shader2 = broken.slang
";
        let preset = parse(text, base()).expect("parse");
        let good = "\
#version 450
#pragma stage fragment
layout(set = 0, binding = 1) uniform sampler2D Source;
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
void main() { FragColor = texture(Source, vTexCoord); }
";
        let chain = super::to_chain(&preset, "mixed", |p| {
            match p.file_name().and_then(|s| s.to_str()) {
                Some("good.slang") => Some(good.to_string()),
                Some("broken.slang") => Some("not glsl".to_string()),
                _ => None,
            }
        });
        assert_eq!(chain.passes.len(), 1, "the good pass still runs");
        assert_eq!(chain.passes[0].name, "good");
        assert_eq!(chain.unsupported.len(), 2);
        let names: Vec<&str> = chain.unsupported.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"missing"), "{names:?}");
        assert!(names.contains(&"broken"), "{names:?}");
        for u in &chain.unsupported {
            assert!(!u.reason.is_empty(), "every failure must state a reason");
        }
    }

    /// Preset overrides apply to the shader's own `#pragma parameter` declarations, by name.
    #[test]
    fn preset_parameters_override_the_shaders_declarations_by_name() {
        let text = "\
shaders = 1
shader0 = a.slang
parameters = \"GAMMA\"
GAMMA = 2.8
";
        let preset = parse(text, base()).expect("parse");
        let shader = "\
#version 450
#pragma parameter GAMMA \"Gamma\" 2.2 1.0 3.0 0.05
#pragma parameter UNTOUCHED \"Other\" 0.5 0.0 1.0
#pragma stage fragment
layout(binding = 1) uniform sampler2D Source;
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
void main() { FragColor = texture(Source, vTexCoord); }
";
        let chain = super::to_chain(&preset, "p", |_| Some(shader.to_string()));
        assert_eq!(chain.passes.len(), 1, "{:?}", chain.unsupported);
        let pass = &chain.passes[0];
        assert!((pass.param("GAMMA").expect("g").value - 2.8).abs() < 1e-6);
        // A knob the preset did not name keeps the shader's own default.
        assert!((pass.param("UNTOUCHED").expect("u").value - 0.5).abs() < 1e-6);
    }

    /// Per-pass keys reach the built pass, and a per-axis scale-type mismatch is reported rather
    /// than silently rendering at a size neither axis asked for.
    #[test]
    fn per_pass_keys_reach_the_built_pass() {
        let text = "\
shaders = 1
shader0 = a.slang
filter_linear0 = true
wrap_mode0 = repeat
float_framebuffer0 = true
alias0 = Blur
frame_count_mod0 = 8
scale_type_x0 = source
scale_type_y0 = viewport
";
        let preset = parse(text, base()).expect("parse");
        let shader = "\
#version 450
#pragma stage fragment
layout(binding = 1) uniform sampler2D Source;
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
void main() { FragColor = texture(Source, vTexCoord); }
";
        let chain = super::to_chain(&preset, "p", |_| Some(shader.to_string()));
        let pass = &chain.passes[0];
        assert!(pass.filter_linear);
        assert_eq!(pass.wrap_mode, WrapMode::Repeat);
        assert!(pass.float_framebuffer);
        assert_eq!(pass.alias.as_deref(), Some("Blur"));
        assert_eq!(pass.frame_count_mod, 8);
        assert!(
            chain
                .unsupported
                .iter()
                .any(|u| u.reason.contains("per-axis scale types differ")),
            "the mismatch must be reported: {:?}",
            chain.unsupported
        );
    }

    #[test]
    fn an_empty_preset_is_default() {
        assert_eq!(Preset::default().passes.len(), 0);
    }
}
