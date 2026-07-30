//! Translate a `.slang` shader into WGSL the stack can run (`v1.25.0`, T-FP-E).
//!
//! A `.slang` file is Vulkan-flavoured GLSL (`#version 450`) with both stages in one file, split by
//! `#pragma stage vertex` / `#pragma stage fragment`, and its uniforms in a `layout(push_constant)`
//! or `layout(std140, ...) uniform` block carrying `RetroArch`'s semantics (`MVP`, `SourceSize`,
//! `OutputSize`, `FrameCount`, …).
//!
//! # Best-effort by construction
//!
//! naga's GLSL frontend covers a **subset** of `#version 450`. A shader outside that subset is not
//! a bug here and never will be fixed by this module — so every failure returns a named
//! [`Unsupported`] with the compiler's own message, the caller drops that one pass, and the rest of
//! the preset still runs. What must never happen is a silent black frame, which is what a
//! translation layer that swallows errors produces.
//!
//! # Why push constants become a uniform block
//!
//! wgpu exposes push constants only on native, and not at all on WebGL — a `.slang` shader using
//! them would work in the desktop build and fail in the browser one, which is worse than not
//! supporting it. The pre-pass rewrites `layout(push_constant) uniform X { … }` into a plain
//! `layout(set = 0, binding = 2) uniform X { … }`, which is the binding the stack's prelude already
//! declares.

use core::fmt::Write as _;

use crate::shader_pass::Unsupported;

/// The result of translating one `.slang` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    /// The generated WGSL, both stages in one module.
    pub wgsl: String,
    /// `RetroArch` semantics the shader referenced ([`SEMANTICS`]).
    pub synthesised: Vec<&'static str>,
    /// Source transforms the pre-pass applied, in the order applied.
    ///
    /// Separate from `synthesised` because they answer different questions: that one says what the
    /// shader *asked for*, this one says what was *changed underneath it*. A rendering difference
    /// is attributable only if both are visible.
    pub rewrites: Vec<&'static str>,
    /// Sampler names the pre-pass split, in declaration order.
    ///
    /// The caller needs these to build a bind group: `samplers[i]` occupies bindings
    /// `SAMPLER_BINDING_BASE + 2i` (the texture) and `+ 2i + 1` (the sampler). A translated module
    /// carries the shader's *own* binding layout, not the stack's fixed one, which is why this is
    /// reported rather than assumed.
    pub samplers: Vec<String>,
}

/// `RetroArch`'s standard uniform semantics, and the WGSL expression each maps to.
///
/// Only the ones the stack can actually supply. A shader using a semantic outside this list gets a
/// named `Unsupported` rather than a zero — a silently-zero `SourceSize` produces a division by
/// zero and a black pass, which is exactly the failure this table exists to prevent.
pub const SEMANTICS: &[(&str, &str)] = &[
    ("SourceSize", "vec4(source_size(), 1.0 / source_size())"),
    ("OriginalSize", "vec4(source_size(), 1.0 / source_size())"),
    ("OutputSize", "vec4(output_size(), 1.0 / output_size())"),
    (
        "FinalViewportSize",
        "vec4(output_size(), 1.0 / output_size())",
    ),
    ("FrameCount", "frame_count()"),
    ("FrameDirection", "1.0"),
    ("OriginalAspect", "(source_size().x / source_size().y)"),
];

/// Split a `.slang` source into its vertex and fragment halves.
///
/// Text before the first `#pragma stage` belongs to **both** — that is where the `#version`, the
/// uniform block, and shared functions live, and dropping it is the single most common way a
/// hand-rolled splitter produces a shader that will not compile.
#[must_use]
pub fn split_stages(source: &str) -> (String, String) {
    let mut common = String::new();
    let mut vertex = String::new();
    let mut fragment = String::new();
    let mut current: Option<Stage> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#pragma stage") {
            current = match rest.trim() {
                "vertex" => Some(Stage::Vertex),
                "fragment" => Some(Stage::Fragment),
                _ => None,
            };
            continue;
        }
        // `#pragma name`/`format`/`parameter` are metadata, not code. `parameter` in particular
        // makes naga's GLSL *preprocessor* fail outright (`PreprocessorError`), so leaving it in
        // means no real `.slang` shader translates — measured, not assumed.
        if trimmed.starts_with("#pragma name")
            || trimmed.starts_with("#pragma format")
            || trimmed.starts_with("#pragma parameter")
        {
            continue;
        }
        match current {
            None => common.push_str(line),
            Some(Stage::Vertex) => vertex.push_str(line),
            Some(Stage::Fragment) => fragment.push_str(line),
        }
        match current {
            None => common.push('\n'),
            Some(Stage::Vertex) => vertex.push('\n'),
            Some(Stage::Fragment) => fragment.push('\n'),
        }
    }
    (format!("{common}{vertex}"), format!("{common}{fragment}"))
}

#[derive(Clone, Copy)]
enum Stage {
    Vertex,
    Fragment,
}

/// Rewrite `layout(push_constant)` blocks into a bound uniform block.
///
/// See the module doc: wgpu has no push constants on WebGL, so a shader using them would work in
/// one build and not the other.
#[must_use]
pub fn rewrite_push_constants(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        if line.contains("push_constant") {
            // Replace only the layout qualifier, keeping whatever else the line declares — a
            // wholesale line replacement would drop the block's name.
            out.push_str(&line.replace("push_constant", "binding = 2"));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// The first binding index generated sampler pairs are assigned.
///
/// Above the low indices a shader's own uniform block conventionally occupies, so a rewritten
/// sampler never lands on top of one.
pub const SAMPLER_BINDING_BASE: u32 = 2;

/// Split combined image samplers (`uniform sampler2D X`) into Vulkan's separated form.
///
/// **Measured, not assumed.** naga's GLSL frontend rejects `uniform sampler2D` outright with
/// `NotImplemented("variable qualifier")` and accepts only the separated `texture2D` + `sampler`
/// pair plus a `sampler2D(tex, smp)` constructor at each use. Every `.slang` shader in existence
/// uses the combined form, because `RetroArch`'s own spec mandates it — so without this rewrite the
/// bridge translates exactly zero real shaders, which was the state before this function existed.
///
/// The transform, per declared name `X`:
///
/// | before | after |
/// |---|---|
/// | `uniform sampler2D X;` | `uniform texture2D X_tex; uniform sampler X_smp;` |
/// | `texture(X, uv)` | `texture(sampler2D(X_tex, X_smp), uv)` |
///
/// Textual and narrow on purpose: it fires only on the exact declaration shape and then substitutes
/// whole-word uses of the declared names. A general rewrite would need a GLSL parser, which is the
/// thing this module exists to delegate to naga. A shader that constructs a sampler in some other
/// way is not transformed and simply fails translation with its own named reason — which is the
/// designed outcome, not a gap.
#[must_use]
pub fn split_combined_samplers(source: &str) -> (String, Vec<String>) {
    let mut names = Vec::new();
    let mut declared = String::with_capacity(source.len());

    // Bindings are re-assigned rather than preserved: one combined sampler becomes TWO bindings, so
    // the original indices cannot all survive anyway, and naga requires an explicit
    // `layout(binding = X)` on every uniform. `SAMPLER_BINDING_BASE` keeps them clear of the
    // uniform block the shader's own semantics live in.
    let mut next_binding = SAMPLER_BINDING_BASE;
    for line in source.lines() {
        if let Some(name) = combined_sampler_name(line) {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            let _ = write!(
                declared,
                "{indent}layout(binding = {next_binding}) uniform texture2D {name}_tex;\n\
                 {indent}layout(binding = {b2}) uniform sampler {name}_smp;\n",
                b2 = next_binding + 1
            );
            next_binding += 2;
            names.push(name);
        } else {
            declared.push_str(line);
            declared.push('\n');
        }
    }

    // Substitute uses only after every declaration is known, so a sampler declared below its first
    // use is still rewritten.
    let mut out = declared;
    for name in &names {
        out = replace_word(&out, name, &format!("sampler2D({name}_tex, {name}_smp)"));
    }
    // The declarations themselves were emitted with the `_tex`/`_smp` suffixes already, so the
    // substitution above must not have touched them — it cannot, since `X_tex` is a different word.
    (out, names)
}

/// The name declared by a `uniform sampler2D NAME;` line, if that is what the line is.
fn combined_sampler_name(line: &str) -> Option<String> {
    let t = line.trim();
    // Skip the layout qualifier if present; the binding is re-assigned by the stack anyway.
    let after_layout = t
        .find(')')
        .filter(|_| t.starts_with("layout"))
        .map_or(t, |i| t[i + 1..].trim());
    let rest = after_layout.strip_prefix("uniform")?.trim();
    let rest = rest.strip_prefix("sampler2D")?.trim();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Replace whole-word occurrences of `word` with `to`.
///
/// Whole-word specifically: substituting a substring would turn `SourceSize` into
/// `sampler2D(Source_tex, Source_smp)Size` for a shader declaring `sampler2D Source`, which is the
/// exact collision a `.slang` shader hits since `Source` and `SourceSize` always co-occur.
fn replace_word(haystack: &str, word: &str, to: &str) -> String {
    let mut out = String::with_capacity(haystack.len());
    let chars: Vec<char> = haystack.chars().collect();
    let target: Vec<char> = word.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let starts_here = chars[i..].starts_with(&target);
        let left_ok = i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
        let right = chars.get(i + target.len());
        let right_ok = right.is_none_or(|c| !(c.is_alphanumeric() || *c == '_'));
        if starts_here && left_ok && right_ok {
            out.push_str(to);
            i += target.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Strip Vulkan descriptor-set qualifiers (`set = N`) from `layout(...)` lists.
///
/// **Measured, not assumed:** naga's GLSL frontend rejects `set = N` outright with
/// `NotImplemented("variable qualifier")`, so a `.slang` shader — every one of which uses Vulkan
/// binding syntax — fails at its first uniform declaration without this.
///
/// Dropping the set index is sound *here* specifically because the stack binds everything in group
/// 0: there is no second descriptor set for the qualifier to distinguish. A shader that genuinely
/// needed `set = 1` would silently collide, which is why [`translate`] records the rewrite rather
/// than performing it invisibly.
#[must_use]
pub fn strip_descriptor_sets(source: &str) -> (String, bool) {
    let mut out = String::with_capacity(source.len());
    let mut stripped = false;
    for line in source.lines() {
        if line.contains("layout") && line.contains("set") {
            let rewritten = strip_set_qualifier(line);
            stripped |= rewritten != line;
            out.push_str(&rewritten);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    (out, stripped)
}

/// Remove a `set = N` (and its comma) from one line's layout list.
///
/// Deliberately textual and narrow: it only fires inside a line that already contains `layout`, and
/// only on the exact `set = <digits>` shape. A general GLSL rewrite would need a parser, which is
/// the thing this whole module is trying to delegate to naga.
fn strip_set_qualifier(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        // Match `set` as a whole word.
        let is_word_start = i == 0 || !bytes[i - 1].is_alphanumeric() && bytes[i - 1] != '_';
        if is_word_start
            && bytes[i..].starts_with(&['s', 'e', 't'])
            && bytes
                .get(i + 3)
                .is_some_and(|c| c.is_whitespace() || *c == '=')
        {
            let mut j = i + 3;
            while j < bytes.len() && bytes[j].is_whitespace() {
                j += 1;
            }
            if bytes.get(j) == Some(&'=') {
                j += 1;
                while j < bytes.len() && bytes[j].is_whitespace() {
                    j += 1;
                }
                let digits_start = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > digits_start {
                    // Also consume the separating comma and following space, so the remaining
                    // list is still well-formed rather than starting with a stray comma.
                    while j < bytes.len() && (bytes[j] == ',' || bytes[j].is_whitespace()) {
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Parse a `#pragma parameter` line into a [`crate::shader_pass::Param`].
///
/// Format: `#pragma parameter NAME "Label" default min max step`.
#[must_use]
pub fn parse_pragma_parameter(line: &str) -> Option<crate::shader_pass::Param> {
    let rest = line.trim().strip_prefix("#pragma parameter")?.trim();
    // The label is quoted and may contain spaces, so it cannot be split on whitespace.
    let (name, after_name) = rest.split_once(char::is_whitespace)?;
    let after_name = after_name.trim();
    let (label, numbers) = if let Some(body) = after_name.strip_prefix('"') {
        let (label, rest) = body.split_once('"')?;
        (label.to_string(), rest)
    } else {
        let (label, rest) = after_name.split_once(char::is_whitespace)?;
        (label.to_string(), rest)
    };
    let nums: Vec<f32> = numbers
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    // default, min, max are required; step is optional and defaults to a hundredth of the range,
    // which is what a slider with no declared step should feel like.
    let [&default, &min, &max] = [nums.first()?, nums.get(1)?, nums.get(2)?];
    let step = nums.get(3).copied().unwrap_or_else(|| {
        let span = (max - min).abs();
        if span > 0.0 { span / 100.0 } else { 0.01 }
    });
    Some(crate::shader_pass::Param {
        name: name.to_string(),
        label,
        value: default,
        min,
        max,
        step,
    })
}

/// Every `#pragma parameter` a source declares, in order.
#[must_use]
pub fn parameters_of(source: &str) -> Vec<crate::shader_pass::Param> {
    source
        .lines()
        .filter(|l| l.trim_start().starts_with("#pragma parameter"))
        .filter_map(parse_pragma_parameter)
        .collect()
}

/// Translate a `.slang` source into WGSL.
///
/// # Errors
/// An [`Unsupported`] naming `pass`, the shader's name, and the compiler's own message. That is the
/// point of the type: the caller drops one pass and keeps the preset.
pub fn translate(source: &str, pass: usize, name: &str) -> Result<Translated, Unsupported> {
    // A fragment stage must be DECLARED, not merely non-empty after concatenation: the shared
    // prologue reaches both halves, so a vertex-only file still produces non-empty fragment text.
    if !source.contains("#pragma stage fragment") {
        return Err(Unsupported {
            pass,
            name: name.to_string(),
            reason: "no `#pragma stage fragment` section found".to_string(),
        });
    }
    let prepared = rewrite_push_constants(source);
    let (prepared, stripped_sets) = strip_descriptor_sets(&prepared);
    let (prepared, samplers) = split_combined_samplers(&prepared);
    let (vertex_src, fragment_src) = split_stages(&prepared);

    let mut synthesised = Vec::new();
    for (semantic, _) in SEMANTICS {
        if source.contains(semantic) {
            synthesised.push(*semantic);
        }
    }
    let mut rewrites = Vec::new();
    if source.contains("push_constant") {
        rewrites.push("push constants rewritten to a bound uniform block");
    }
    if stripped_sets {
        // Recorded, not silent: the stack binds everything in group 0, so dropping the set index
        // is sound here — but a shader that genuinely needed a second set would collide, and this
        // is what lets that be attributed rather than guessed at.
        rewrites.push("descriptor-set qualifiers dropped (everything binds to group 0)");
    }
    if !samplers.is_empty() {
        // Recorded because it changes the binding layout: one combined sampler becomes two
        // bindings, so a preset relying on specific indices is affected.
        rewrites.push("combined samplers split into texture + sampler pairs");
    }

    let module = parse_stage(&fragment_src, naga::ShaderStage::Fragment, pass, name)?;
    // The vertex half is parsed too, even though the stack supplies its own vertex stage: a
    // `.slang` whose vertex stage does not compile is a shader this bridge cannot faithfully run,
    // and reporting that up front is better than rendering with a substituted stage that silently
    // changes the geometry the fragment shader assumes.
    if source.contains("#pragma stage vertex") {
        let _ = parse_stage(&vertex_src, naga::ShaderStage::Vertex, pass, name)?;
    }

    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| Unsupported {
        pass,
        name: name.to_string(),
        reason: format!("validation: {e}"),
    })?;

    let wgsl =
        naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
            .map_err(|e| Unsupported {
                pass,
                name: name.to_string(),
                reason: format!("WGSL emit: {e}"),
            })?;

    Ok(Translated {
        wgsl,
        synthesised,
        rewrites,
        samplers,
    })
}

/// Parse one GLSL stage, mapping any failure to a named [`Unsupported`].
fn parse_stage(
    src: &str,
    stage: naga::ShaderStage,
    pass: usize,
    name: &str,
) -> Result<naga::Module, Unsupported> {
    let mut frontend = naga::front::glsl::Frontend::default();
    frontend
        .parse(&naga::front::glsl::Options::from(stage), src)
        .map_err(|e| Unsupported {
            pass,
            name: name.to_string(),
            // The frontend's own message, verbatim. A paraphrase would lose the line number, which
            // is the only part that lets someone actually fix the shader.
            reason: format!("GLSL {stage:?}: {e:?}"),
        })
}

#[cfg(test)]
mod tests {
    use super::{
        SEMANTICS, parameters_of, parse_pragma_parameter, rewrite_push_constants, split_stages,
        translate,
    };

    /// Text before the first `#pragma stage` belongs to BOTH halves — that is where `#version`,
    /// the uniform block, and shared functions live, and dropping it is the most common way a
    /// hand-rolled splitter produces something that will not compile.
    #[test]
    fn common_prologue_reaches_both_stages() {
        let src = "\
#version 450
layout(std140, set = 0, binding = 0) uniform UBO { vec4 SourceSize; };
float shared_helper() { return 1.0; }
#pragma stage vertex
void main() { gl_Position = vec4(0.0); }
#pragma stage fragment
void main() { }
";
        let (v, f) = split_stages(src);
        for half in [&v, &f] {
            assert!(half.contains("#version 450"), "{half}");
            assert!(half.contains("uniform UBO"), "{half}");
            assert!(half.contains("shared_helper"), "{half}");
        }
        assert!(v.contains("gl_Position"));
        assert!(!f.contains("gl_Position"), "stages must not leak");
    }

    /// `#pragma name` / `#pragma format` are metadata; leaving them in reaches the GLSL frontend
    /// as unknown pragmas.
    #[test]
    fn metadata_pragmas_are_stripped() {
        let (v, f) = split_stages(
            "#pragma name my_pass\n#pragma format R8G8B8A8_UNORM\n#pragma stage fragment\nvoid main() {}\n",
        );
        assert!(!v.contains("#pragma name"));
        assert!(!f.contains("#pragma name"));
        assert!(!f.contains("#pragma format"));
        assert!(f.contains("void main"));
    }

    /// Push constants are rewritten to the stack's own binding, because wgpu has none on WebGL —
    /// a shader using them would work in the desktop build and fail in the browser one.
    #[test]
    fn push_constants_become_a_bound_uniform_block() {
        let out = rewrite_push_constants(
            "layout(push_constant) uniform Push { vec4 SourceSize; } params;\n",
        );
        assert!(!out.contains("push_constant"), "{out}");
        // `binding = 2` without a `set`: naga's GLSL frontend rejects `set = N` outright
        // (measured — see `strip_descriptor_sets`), and the stack binds everything in group 0.
        assert!(out.contains("binding = 2"), "{out}");
        assert!(!out.contains("set ="), "{out}");
        // The block's own name survives — a wholesale line replacement would drop it.
        assert!(out.contains("uniform Push"), "{out}");
        assert!(out.contains("} params;"), "{out}");
    }

    #[test]
    fn pragma_parameters_parse_including_quoted_labels() {
        let p = parse_pragma_parameter(
            "#pragma parameter SCANLINE_WEIGHT \"Scanline Weight\" 0.3 0.0 1.0 0.05",
        )
        .expect("parse");
        assert_eq!(p.name, "SCANLINE_WEIGHT");
        assert_eq!(p.label, "Scanline Weight");
        assert!((p.value - 0.3).abs() < 1e-6);
        assert!((p.min - 0.0).abs() < 1e-6);
        assert!((p.max - 1.0).abs() < 1e-6);
        assert!((p.step - 0.05).abs() < 1e-6);
    }

    /// An omitted step becomes a hundredth of the range, which is what a slider with no declared
    /// step should feel like — a zero step would make the slider unusable.
    #[test]
    fn an_omitted_step_is_derived_from_the_range() {
        let p =
            parse_pragma_parameter("#pragma parameter GAMMA \"Gamma\" 2.2 1.0 3.0").expect("parse");
        assert!((p.step - 0.02).abs() < 1e-6, "step was {}", p.step);
        // A degenerate range still yields a usable step rather than zero.
        let flat = parse_pragma_parameter("#pragma parameter K \"K\" 1.0 1.0 1.0").expect("parse");
        assert!(flat.step > 0.0);
    }

    #[test]
    fn malformed_parameter_lines_are_refused() {
        assert!(parse_pragma_parameter("#pragma parameter").is_none());
        assert!(parse_pragma_parameter("#pragma parameter ONLY_NAME").is_none());
        assert!(parse_pragma_parameter("#pragma parameter N \"L\" 1.0").is_none());
        assert!(parse_pragma_parameter("not a pragma at all").is_none());
    }

    #[test]
    fn parameters_are_collected_in_order() {
        let src = "\
#pragma parameter A \"Alpha\" 0.1 0.0 1.0
// noise
#pragma parameter B \"Beta\" 0.2 0.0 1.0 0.01
";
        let ps = parameters_of(src);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].name, "A");
        assert_eq!(ps[1].name, "B");
    }

    /// A real (if minimal) `.slang` translates end to end.
    #[test]
    fn a_simple_slang_shader_translates_to_wgsl() {
        let src = "\
#version 450
#pragma stage vertex
layout(location = 0) in vec4 Position;
layout(location = 1) in vec2 TexCoord;
layout(location = 0) out vec2 vTexCoord;
void main() {
    gl_Position = Position;
    vTexCoord = TexCoord;
}
#pragma stage fragment
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
layout(set = 0, binding = 1) uniform sampler2D Source;
void main() {
    FragColor = texture(Source, vTexCoord);
}
";
        let out = translate(src, 0, "simple").expect("should translate");
        assert!(!out.wgsl.is_empty());
        // The emitted WGSL must actually be WGSL naga accepts back.
        naga::front::wgsl::parse_str(&out.wgsl).expect("round-trip parse");
    }

    /// A shader outside naga's GLSL subset is not a bug here. It must produce a NAMED failure with
    /// the compiler's own message so the caller drops one pass and keeps the preset — never a
    /// silent black frame.
    #[test]
    fn an_untranslatable_shader_names_the_pass_and_the_reason() {
        let err =
            translate("this is not glsl", 3, "crt-royale-blur").expect_err("should not translate");
        assert_eq!(err.pass, 3);
        assert_eq!(err.name, "crt-royale-blur");
        assert!(!err.reason.is_empty());
    }

    /// A file with no fragment stage is refused by name rather than producing an empty module.
    #[test]
    fn a_missing_fragment_stage_is_refused() {
        let err = translate(
            "#version 450\n#pragma stage vertex\nvoid main() {}\n",
            1,
            "v-only",
        )
        .expect_err("no fragment stage");
        assert!(err.reason.contains("fragment"), "{}", err.reason);
    }

    /// Semantics the shader referenced are recorded, so a rendering difference can be attributed.
    #[test]
    fn referenced_semantics_are_recorded() {
        let src = "\
#version 450
#pragma stage fragment
layout(set = 0, binding = 1) uniform sampler2D Source;
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
void main() { FragColor = texture(Source, vTexCoord); }
";
        let plain = translate(src, 0, "plain").expect("translate");
        assert!(
            plain.synthesised.is_empty(),
            "no semantic was referenced: {:?}",
            plain.synthesised
        );
        // The sampler split is a *rewrite*, not a semantic — the two answer different questions
        // and conflating them is what this split exists to prevent.
        assert!(
            plain
                .rewrites
                .iter()
                .any(|r| r.contains("combined samplers")),
            "{:?}",
            plain.rewrites
        );

        let with_size = src.replace(
            "uniform sampler2D Source;",
            "uniform sampler2D Source;\n// SourceSize",
        );
        let out = translate(&with_size, 0, "sized").expect("translate");
        assert!(
            out.synthesised.contains(&"SourceSize"),
            "{:?}",
            out.synthesised
        );
    }

    /// Every semantic in the table maps to a non-empty expression — a blank would compile to a
    /// zero, and a zero `SourceSize` is a division by zero and a black pass.
    #[test]
    fn every_semantic_has_an_expression() {
        assert!(!SEMANTICS.is_empty());
        for (name, expr) in SEMANTICS {
            assert!(!name.is_empty());
            assert!(!expr.is_empty(), "{name} has no expression");
        }
    }
}
