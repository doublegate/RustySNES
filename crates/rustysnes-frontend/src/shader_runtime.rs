//! Executes a [`crate::shader_pass::ShaderChain`] on the GPU (`v1.25.0`, T-FP-D).
//!
//! Owns the ping-pong intermediate targets, one pipeline and bind group per pass, and the uniform
//! buffer each pass's `#pragma parameter` values are packed into. Rebuilt only when the chain or a
//! size actually changes — a per-frame rebuild would allocate a texture and compile a pipeline
//! every present, which is the kind of cost that makes a shader stack unusable rather than slow.
//!
//! # The uniform layout, once
//!
//! Every pass shares one layout so a pass can be swapped without touching the Rust side:
//!
//! | binding | contents |
//! |---|---|
//! | 0 | input texture (the previous pass's output, or the framebuffer for pass 0) |
//! | 1 | sampler (`filter_linear` and `wrap_mode` from the pass) |
//! | 2 | `Uniforms` — source/output sizes, the frame counter, and [`MAX_PARAMS`] parameter floats |
//!
//! `Uniforms` is `repr(C)` and explicitly padded to a 16-byte multiple, because WGSL's uniform
//! address space requires 16-byte alignment for a struct and a mismatch here does not error — it
//! reads the wrong fields.

use crate::shader_pass::{MAX_PARAMS, PassDesc, ShaderChain};

/// The per-pass uniform block.
///
/// Field order is the WGSL declaration order and must stay in step with it. Sizes are `vec4`-packed
/// (`source.xy`, `output.zw`) so the whole header is one 16-byte row.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Uniforms {
    /// `x`/`y` = input size in pixels, `z`/`w` = output size in pixels.
    pub sizes: [f32; 4],
    /// `x` = this pass's frame counter, `y`/`z`/`w` = reserved (zero).
    ///
    /// Reserved rather than removed so adding a global (a time value, a pass index) later does not
    /// change the struct's size and silently shift every parameter by one row.
    pub frame: [f32; 4],
    /// The pass's parameters, one per slot, zero past the declared count.
    pub params: [f32; MAX_PARAMS],
}

// SAFETY-adjacent invariant, checked at compile time rather than trusted: WGSL requires a uniform
// struct to be 16-byte aligned, and a mismatch is not an error — it reads the wrong fields.
const _: () = assert!(size_of::<Uniforms>().is_multiple_of(16));

impl Uniforms {
    /// Build the uniform block for one pass.
    #[must_use]
    pub fn for_pass(pass: &PassDesc, input: (u32, u32), output: (u32, u32), frame: u32) -> Self {
        #[allow(clippy::cast_precision_loss)]
        Self {
            sizes: [
                input.0 as f32,
                input.1 as f32,
                output.0 as f32,
                output.1 as f32,
            ],
            frame: [pass.frame_counter(frame) as f32, 0.0, 0.0, 0.0],
            params: pass.pack_params(),
        }
    }

    /// The block as bytes, for `Queue::write_buffer`.
    #[must_use]
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(size_of::<Self>());
        for v in self
            .sizes
            .iter()
            .chain(self.frame.iter())
            .chain(self.params.iter())
        {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }
}

/// The WGSL prelude every stack shader is compiled with.
///
/// Prepended rather than duplicated in each shader so the binding layout is declared exactly once —
/// a shader whose bindings drift from the Rust side fails to create a pipeline with a message that
/// names a group and a binding rather than the mistake.
pub const PRELUDE: &str = r"
struct StackUniforms {
    sizes: vec4<f32>,
    frame: vec4<f32>,
    params: array<vec4<f32>, 4>,
};

@group(0) @binding(0) var stack_tex: texture_2d<f32>;
@group(0) @binding(1) var stack_samp: sampler;
@group(0) @binding(2) var<uniform> stack: StackUniforms;

fn source_size() -> vec2<f32> { return stack.sizes.xy; }
fn output_size() -> vec2<f32> { return stack.sizes.zw; }
fn frame_count() -> f32 { return stack.frame.x; }

// Parameters are packed four to a vec4 to satisfy the uniform array-stride rule (a
// `array<f32, N>` in the uniform address space has a 16-byte stride per element in WGSL, which
// would waste 4x the space and silently misalign against the Rust side's tight packing).
fn param(i: u32) -> f32 {
    let row = stack.params[i / 4u];
    switch (i % 4u) {
        case 0u: { return row.x; }
        case 1u: { return row.y; }
        case 2u: { return row.z; }
        default: { return row.w; }
    }
}

struct StackVsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// A fullscreen triangle, not a quad: three vertices, no index buffer, and no seam along the
// diagonal where a two-triangle quad would double-shade the shared edge.
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> StackVsOut {
    var out: StackVsOut;
    let x = f32((idx << 1u) & 2u);
    let y = f32(idx & 2u);
    out.uv = vec2<f32>(x, y);
    out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return out;
}
";

/// Compose a pass's WGSL with the prelude.
///
/// A pass supplies only its `fs_main`; the prelude supplies the bindings, the helpers, and the
/// vertex stage. That is what lets a `#pragma parameter` shader be swapped in without any Rust
/// change — which is the whole point of the stack.
#[must_use]
pub fn compose(source: &str) -> String {
    format!("{PRELUDE}\n{source}")
}

/// Whether a chain's every pass compiles, reported per pass.
///
/// Used by the Settings UI to show which passes are live before anything renders, so a broken
/// custom shader is named rather than showing as a black frame.
#[must_use]
pub fn validate(chain: &ShaderChain) -> Vec<Result<(), String>> {
    chain
        .passes
        .iter()
        .map(|pass| {
            let wgsl = compose(&pass.source);
            match naga::front::wgsl::parse_str(&wgsl) {
                Ok(module) => naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::empty(),
                )
                .validate(&module)
                .map(|_| ())
                .map_err(|e| format!("{}: {e}", pass.name)),
                Err(e) => Err(format!("{}: {e}", pass.name)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PRELUDE, Uniforms, compose, validate};
    use crate::shader_pass::{MAX_PARAMS, Param, PassDesc, ShaderChain};

    /// A minimal pass body that exercises every prelude helper, so a helper that fails to compile
    /// is caught here rather than in whichever shader happens to use it first.
    const PASSTHROUGH: &str = r"
@fragment
fn fs_main(in: StackVsOut) -> @location(0) vec4<f32> {
    let s = source_size();
    let o = output_size();
    let f = frame_count();
    let p = param(0u) + param(5u);
    let c = textureSample(stack_tex, stack_samp, in.uv);
    return vec4<f32>(c.rgb * (1.0 - p * 0.0) + vec3<f32>(s.x, o.x, f) * 0.0, c.a);
}
";

    #[test]
    fn the_prelude_and_a_pass_compile_together() {
        let chain = ShaderChain::single(PassDesc::new("passthrough", PASSTHROUGH));
        let results = validate(&chain);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok(), "{:?}", results[0]);
    }

    /// A broken pass is reported by name, not swallowed — the Settings UI shows this string.
    #[test]
    fn a_broken_pass_is_named() {
        let chain = ShaderChain::single(PassDesc::new("broken", "this is not wgsl at all"));
        let results = validate(&chain);
        let err = results[0].as_ref().expect_err("should not compile");
        assert!(err.starts_with("broken:"), "{err}");
    }

    /// Every pass is reported independently, so one bad pass does not hide the good ones.
    #[test]
    fn each_pass_is_validated_independently() {
        let chain = ShaderChain {
            name: "mixed".into(),
            passes: vec![
                PassDesc::new("good", PASSTHROUGH),
                PassDesc::new("bad", "nonsense"),
                PassDesc::new("also_good", PASSTHROUGH),
            ],
            unsupported: Vec::new(),
        };
        let results = validate(&chain);
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok(), "a later pass is still validated");
    }

    /// The uniform block must be 16-byte-aligned and laid out in the declared order — WGSL does not
    /// error on a mismatch, it reads the wrong fields.
    #[test]
    fn uniform_layout_matches_the_wgsl_declaration() {
        assert_eq!(size_of::<Uniforms>() % 16, 0);
        // sizes(4) + frame(4) + params(MAX_PARAMS) floats.
        assert_eq!(size_of::<Uniforms>(), (8 + MAX_PARAMS) * 4);

        let pass = PassDesc::new("p", PASSTHROUGH).with_params(vec![
            Param::unit("a", "A", 0.25),
            Param::unit("b", "B", 0.75),
        ]);
        let u = Uniforms::for_pass(&pass, (256, 224), (512, 448), 7);
        assert!((u.sizes[0] - 256.0).abs() < 1e-6);
        assert!((u.sizes[3] - 448.0).abs() < 1e-6);
        assert!((u.frame[0] - 7.0).abs() < 1e-6);
        assert!((u.params[0] - 0.25).abs() < 1e-6);
        assert!((u.params[1] - 0.75).abs() < 1e-6);

        let bytes = u.as_bytes();
        assert_eq!(bytes.len(), size_of::<Uniforms>());
        // The first float is little-endian 256.0.
        assert_eq!(&bytes[0..4], &256.0f32.to_le_bytes());
    }

    /// The prelude declares the parameter array four-to-a-`vec4`, which must match `MAX_PARAMS`.
    #[test]
    fn the_prelude_param_array_matches_max_params() {
        let rows = MAX_PARAMS / 4;
        assert!(
            PRELUDE.contains(&format!("array<vec4<f32>, {rows}>")),
            "prelude array size must be MAX_PARAMS/4 = {rows}"
        );
        assert_eq!(MAX_PARAMS % 4, 0, "MAX_PARAMS must be a multiple of 4");
    }

    #[test]
    fn compose_prepends_the_prelude() {
        let out = compose("// body");
        assert!(out.starts_with(PRELUDE));
        assert!(out.ends_with("// body"));
    }
}
