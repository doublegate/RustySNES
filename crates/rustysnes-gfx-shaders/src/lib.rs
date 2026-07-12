//! Shared WGSL presentation-shader sources for RustySNES's wgpu blit + post-filter pipeline
//! (`v1.12.0 "Refraction"`).
//!
//! Extracted verbatim out of `rustysnes-frontend::gfx` (`v1.2.0`'s `BLIT_WGSL`/`CRT_WGSL`/
//! `HQX_WGSL` inline constants moved here byte-for-byte; `XBRZ_WGSL` is new in this release) so a
//! non-wgpu-owning consumer — the planned `rustysnes-mobile` bridge (`v1.14.0 "Foundry"`) reusing
//! these exact shader strings against its own native Compose/SwiftUI-hosted wgpu surface — can
//! depend on the shader source without pulling in `rustysnes-frontend`'s winit/egui/cpal shell.
//!
//! `#![no_std]`: these are `&'static str` constants with zero runtime logic, so there is nothing
//! here that needs `alloc` or `std` — matching the chip crates' own `no_std` posture and keeping
//! this crate trivially reusable from `rustysnes-frontend/src/gfx.rs`'s existing `no_std` CI gate
//! neighbors as well as any future mobile target.
//!
//! Every shader here shares one calling convention, documented once instead of per-constant:
//! binding 0 = the framebuffer texture, binding 1 = a sampler (the plain blit's vertex-stage-only
//! uniform samples it; the post-filter passes bind it only to keep the bind-group layout shape
//! identical, then sample via `textureLoad` instead), binding 2 = a `vec4<f32>` (or two, for the
//! post-filters) uniform buffer carrying `uv_scale.xy`/`pos_scale.xy` (the live sub-rect + the
//! aspect-correct letterbox scale — see `rustysnes_frontend::gfx::Gfx::letterbox_scale`) plus,
//! for the post-filter shaders, a second `vec4<f32>` of filter-specific parameters documented on
//! each constant below.
#![no_std]
#![warn(missing_docs)]

/// The fullscreen-triangle blit shader (nearest sample of the framebuffer texture).
///
/// `PostFilter::None`'s pixel-identical-to-a-filter-less-build path. Uniform: `params.xy` = UV
/// scale (live sub-rect), `params.zw` = clip-space letterbox scale.
pub const BLIT_WGSL: &str = r"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
// params.xy = UV scale (live sub-rect); params.zw = clip-space letterbox scale.
@group(0) @binding(2) var<uniform> params: vec4<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    // Fullscreen triangle: (-1,-1), (3,-1), (-1,3) in clip space.
    var out: VsOut;
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    // Map the base fullscreen UV (0..1 over the visible screen), scaled to the live sub-rect.
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5) * params.xy;
    // Letterbox by shrinking the triangle; the cleared black border fills the rest.
    out.pos = vec4<f32>(x * params.z, y * params.w, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
";

/// The `PostFilter::Crt` post-pass: scanlines + an RGB aperture-grille mask, approximating a
/// CRT's phosphor structure.
///
/// Same vertex-shader letterbox convention as [`BLIT_WGSL`]: `scale.xy` = UV scale (live
/// sub-rect), `scale.zw` = letterbox position scale; `strength.x` = scanline intensity,
/// `strength.y` = aperture-mask intensity, `strength.z` = source scanline count (the active
/// framebuffer height), `strength.w` unused.
pub const CRT_WGSL: &str = r"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
struct Uniforms {
    scale: vec4<f32>,
    strength: vec4<f32>,
};
@group(0) @binding(2) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5) * u.scale.xy;
    out.pos = vec4<f32>(x * u.scale.z, y * u.scale.w, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var rgb = textureSample(tex, samp, in.uv).rgb;

    let scan_amt = u.strength.x;
    let mask_amt = u.strength.y;

    // Scanlines in SOURCE-row space: normalize `in.uv` (already scaled by `u.scale.xy`, the live
    // sub-rect fraction) back to 0..1 over the live framebuffer before computing the row index, so
    // the scanline pitch tracks the source resolution, not the oversized backing texture.
    let norm_uv = in.uv / max(u.scale.xy, vec2<f32>(1e-6, 1e-6));
    let rows = max(u.strength.z, 1.0);
    let src_y = norm_uv.y * rows;
    let d = fract(src_y) - 0.5;
    // Parabolic profile: 1.0 at the row centre, (1 - scan_amt) at the row boundary.
    let scan = (1.0 - scan_amt) + scan_amt * (1.0 - 4.0 * d * d);
    rgb = rgb * scan;

    // Aperture grille: tint output columns in an R/G/B triad, keyed off the OUTPUT pixel column
    // (not the source column) so the mask stays a fixed-pitch overlay regardless of scale.
    let col = i32(floor(in.pos.x)) % 3;
    var mask = vec3<f32>(1.0 - mask_amt, 1.0 - mask_amt, 1.0 - mask_amt);
    if (col == 0) {
        mask.r = 1.0;
    } else if (col == 1) {
        mask.g = 1.0;
    } else {
        mask.b = 1.0;
    }
    rgb = rgb * mask;

    // Brightness compensation: scanlines + mask remove energy; add a little back so a
    // mid-strength CRT does not look washed-out dark.
    let comp = 1.0 + 0.5 * (scan_amt + mask_amt);
    rgb = rgb * comp;

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0)), 1.0);
}
";

/// The `PostFilter::Hqx` post-pass: a single-pass, edge-directed diagonal blend.
///
/// An HQ2x-STYLE *approximation* (a diagonal-similarity heuristic in the 2xSaI/Eagle family),
/// not a literal `HQ2x` pattern-lookup-table port (the right fit for this project's
/// fixed-resolution architecture, which never actually renders at a literal 2x buffer size).
///
/// Same letterbox convention as [`CRT_WGSL`]: `scale.xy`/`scale.zw` identical. `fb.xy` = the
/// live framebuffer's `(width, height)` in texels (for the source-space texel walk below),
/// `fb.z` = edge-bias strength (`0` = plain bilinear, `1` = full diagonal pull), `fb.w` unused.
///
/// Reads the framebuffer with `textureLoad` (integer texel coordinates, no sampler filtering
/// involved) rather than `textureSample`, so it works identically whether or not the bound
/// texture is filterable.
pub const HQX_WGSL: &str = r"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler; // unused (textureLoad needs no sampler) -- kept only so
                                          // this pipeline shares the plain blit's bind-group-layout shape.
struct Uniforms {
    scale: vec4<f32>,
    fb: vec4<f32>,
};
@group(0) @binding(2) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>, // normalized 0..1 over the LIVE sub-rect (not yet scaled by scale.xy)
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    out.pos = vec4<f32>(x * u.scale.z, y * u.scale.w, 0.0, 1.0);
    return out;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn fetch(coord: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
    let c = clamp(coord, vec2<i32>(0, 0), dims - vec2<i32>(1, 1));
    return textureLoad(tex, c, 0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(i32(u.fb.x), i32(u.fb.y));
    let fdims = u.fb.xy;

    // Source-space fractional position within the live sub-rect (half-texel offset so `frac`
    // is 0 exactly at texel centres, matching bilinear convention).
    let src = in.uv * fdims - vec2<f32>(0.5, 0.5);
    let base = vec2<i32>(floor(src));
    let frac = fract(src);

    let tl = fetch(base, dims);
    let tr = fetch(base + vec2<i32>(1, 0), dims);
    let bl = fetch(base + vec2<i32>(0, 1), dims);
    let br = fetch(base + vec2<i32>(1, 1), dims);

    let l_tl = luma(tl.rgb);
    let l_tr = luma(tr.rgb);
    let l_bl = luma(bl.rgb);
    let l_br = luma(br.rgb);

    // Diagonal-similarity edge detection (2xSaI/Eagle-family heuristic): if the TL-BR diagonal
    // is more self-similar than the TR-BL diagonal (or vice versa), bias the bilinear blend
    // toward the matching diagonal instead of a plain axis-aligned average -- this softens
    // staircase edges while leaving flat-color regions (all four samples equal, both diagonal
    // differences zero) completely unaffected.
    let diag_main = abs(l_tl - l_br);
    let diag_anti = abs(l_tr - l_bl);
    var w_tl = (1.0 - frac.x) * (1.0 - frac.y);
    var w_tr = frac.x * (1.0 - frac.y);
    var w_bl = (1.0 - frac.x) * frac.y;
    var w_br = frac.x * frac.y;
    let edge_bias = clamp(u.fb.z, 0.0, 1.0);
    if (diag_main < diag_anti) {
        let pull = edge_bias * (0.5 - min(frac.x, frac.y));
        w_tl = w_tl + pull;
        w_br = w_br + pull;
        w_tr = w_tr - pull;
        w_bl = w_bl - pull;
    } else if (diag_anti < diag_main) {
        let pull = edge_bias * (0.5 - min(1.0 - frac.x, frac.y));
        w_tr = w_tr + pull;
        w_bl = w_bl + pull;
        w_tl = w_tl - pull;
        w_br = w_br - pull;
    }
    let wsum = max(w_tl + w_tr + w_bl + w_br, 1e-4);
    let rgb = (tl.rgb * w_tl + tr.rgb * w_tr + bl.rgb * w_bl + br.rgb * w_br) / wsum;
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0)), 1.0);
}
";

/// The `PostFilter::Xbrz` post-pass (`v1.12.0 "Refraction"`): a single-pass, context-aware
/// corner-rounding blend.
///
/// An xBRZ-STYLE *approximation* of the algorithm's corner rule, not a literal xBRZ port. Real
/// xBRZ is a multi-pass, integer-scale-factor (2x/3x/4x/5x) algorithm driven by a large
/// hand-tuned rule table examining up to a 5-texel diagonal run per
/// corner to decide sub-pixel corner geometry; this fixed-resolution fragment shader distills
/// that "look past the immediate corner before rounding it" idea into ONE extra context sample
/// per diagonal (an 4x4-neighborhood read, not just [`HQX_WGSL`]'s bare 2x2), gating the
/// diagonal-pull strength by how well the wider neighborhood actually supports treating the edge
/// as a genuine corner rather than isolated-pixel noise. This is the meaningful difference from
/// [`HQX_WGSL`] — both blend the same 2x2 corner, but this one only commits to the full pull when
/// the outward-neighbor context agrees the diagonal continues.
///
/// Same letterbox + uniform convention as [`HQX_WGSL`]: `scale.xy`/`scale.zw` letterbox, `fb.xy`
/// = live framebuffer `(width, height)` in texels, `fb.z` = blend strength (`0` = plain bilinear,
/// `1` = full context-gated diagonal pull), `fb.w` unused. `textureLoad`-based, same rationale as
/// [`HQX_WGSL`].
pub const XBRZ_WGSL: &str = r"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler; // unused (textureLoad needs no sampler) -- kept only so
                                          // this pipeline shares the plain blit's bind-group-layout shape.
struct Uniforms {
    scale: vec4<f32>,
    fb: vec4<f32>,
};
@group(0) @binding(2) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    out.pos = vec4<f32>(x * u.scale.z, y * u.scale.w, 0.0, 1.0);
    return out;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn fetch(coord: vec2<i32>, dims: vec2<i32>) -> vec4<f32> {
    let c = clamp(coord, vec2<i32>(0, 0), dims - vec2<i32>(1, 1));
    return textureLoad(tex, c, 0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(i32(u.fb.x), i32(u.fb.y));
    let fdims = u.fb.xy;

    let src = in.uv * fdims - vec2<f32>(0.5, 0.5);
    let base = vec2<i32>(floor(src));
    let frac = fract(src);

    // The immediate 2x2 corner (same positions [`HQX_WGSL`] blends).
    let tl = fetch(base, dims);
    let tr = fetch(base + vec2<i32>(1, 0), dims);
    let bl = fetch(base + vec2<i32>(0, 1), dims);
    let br = fetch(base + vec2<i32>(1, 1), dims);
    let l_tl = luma(tl.rgb);
    let l_tr = luma(tr.rgb);
    let l_bl = luma(bl.rgb);
    let l_br = luma(br.rgb);

    // The wider 4x4 context: one step further out along each diagonal, so a corner only gets a
    // full round when the trend actually continues past the immediate 2x2 -- xBRZ's own
    // multi-sample corner rule distilled into a single extra check per diagonal.
    let l_further_tl = luma(fetch(base + vec2<i32>(-1, -1), dims).rgb);
    let l_further_br = luma(fetch(base + vec2<i32>(2, 2), dims).rgb);
    let l_further_tr = luma(fetch(base + vec2<i32>(2, -1), dims).rgb);
    let l_further_bl = luma(fetch(base + vec2<i32>(-1, 2), dims).rgb);

    let diag_main = abs(l_tl - l_br);
    let diag_anti = abs(l_tr - l_bl);
    let context_main = clamp(abs(l_further_tl - l_tl) + abs(l_further_br - l_br), 0.0, 1.0);
    let context_anti = clamp(abs(l_further_tr - l_tr) + abs(l_further_bl - l_bl), 0.0, 1.0);

    var w_tl = (1.0 - frac.x) * (1.0 - frac.y);
    var w_tr = frac.x * (1.0 - frac.y);
    var w_bl = (1.0 - frac.x) * frac.y;
    var w_br = frac.x * frac.y;
    let edge_bias = clamp(u.fb.z, 0.0, 1.0);
    if (diag_main < diag_anti) {
        let confidence = 1.0 - context_main;
        let pull = edge_bias * confidence * (0.5 - min(frac.x, frac.y));
        w_tl = w_tl + pull;
        w_br = w_br + pull;
        w_tr = w_tr - pull;
        w_bl = w_bl - pull;
    } else if (diag_anti < diag_main) {
        let confidence = 1.0 - context_anti;
        let pull = edge_bias * confidence * (0.5 - min(1.0 - frac.x, frac.y));
        w_tr = w_tr + pull;
        w_bl = w_bl + pull;
        w_tl = w_tl - pull;
        w_br = w_br - pull;
    }
    let wsum = max(w_tl + w_tr + w_bl + w_br, 1e-4);
    let rgb = (tl.rgb * w_tl + tr.rgb * w_tr + bl.rgb * w_bl + br.rgb * w_br) / wsum;
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0)), 1.0);
}
";
