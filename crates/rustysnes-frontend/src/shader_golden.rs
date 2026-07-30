//! Offscreen golden tests for the shader stack (`v1.25.0`, T-FP-D).
//!
//! These render each stack pass through [`crate::gfx_test_support`] and assert on the **pixels**,
//! not on whether the WGSL parsed. That distinction is the whole reason the harness exists: a
//! shader that compiles and renders the wrong thing passes a validation test.
//!
//! # What is asserted, and what is not
//!
//! Per `docs/adr/0013`'s posture on the emulation side, a golden may only be blessed from a render
//! that was inspected first. Rather than commit opaque hashes nobody has looked at, these tests
//! assert **properties that are true by construction and would break if the shader were wrong**:
//!
//! - Every knob at zero is a bit-exact pass-through (which is what makes a pass safe to leave in
//!   the chain — the same contract `crate::eq` holds for audio).
//! - A knob turned up changes the image, and changes it in the direction it claims.
//! - Rendering the same input twice produces the identical hash, so the *stability* a golden
//!   depends on is itself verified.
//!
//! That is a stronger statement than a committed hash, because a hash only says "the same as last
//! time" — including the last time it was wrong.
//!
//! Self-skips with a printed reason where no GPU exists.

#![cfg(test)]

use crate::gfx_test_support::{Readback, TestGpu};
use crate::shader_pass::{Param, PassDesc};
use crate::shader_runtime::{Uniforms, compose};

/// Render one pass over a generated source image and read the result back.
fn render_pass(gpu: &TestGpu, pass: &PassDesc, src: &Readback, frame: u32) -> Readback {
    render_pass_padded(gpu, pass, src, frame, 1)
}

/// As [`render_pass`], but the input image is placed in the **top-left corner** of a texture
/// `pad` times larger in each axis, exactly as the live frontend's oversized backing framebuffer
/// holds it. `pad = 1` is the ordinary case.
///
/// This exists to test the sub-rect handling: with `pad > 1` the pass must produce the same image
/// it does at `pad = 1`, because the extra texture is not part of the picture.
// One straight-line wgpu recipe (upload, sampler, uniforms, module, layout, bind group, pipeline,
// pass, readback); splitting it would scatter one sequence across functions taking the same args.
#[allow(clippy::too_many_lines)]
fn render_pass_padded(
    gpu: &TestGpu,
    pass: &PassDesc,
    src: &Readback,
    frame: u32,
    pad: u32,
) -> Readback {
    let (w, h) = (src.width, src.height);
    let (tex_w, tex_h) = (w * pad, h * pad);

    // Upload the source image into the top-left of the (possibly larger) input texture.
    let input = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("golden-input"),
        size: wgpu::Extent3d {
            width: tex_w,
            height: tex_h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &input,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &src.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );

    let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("golden-sampler"),
        address_mode_u: pass.wrap_mode.to_wgpu(),
        address_mode_v: pass.wrap_mode.to_wgpu(),
        address_mode_w: pass.wrap_mode.to_wgpu(),
        mag_filter: if pass.filter_linear {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        },
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    // The live fraction of the bound texture. `pad = 1` makes this `(1, 1)`, which is what keeps
    // a golden hash portable — it must not depend on how large a backing texture the frontend
    // happened to allocate.
    #[allow(clippy::cast_precision_loss)]
    let source_rect = (w as f32 / tex_w as f32, h as f32 / tex_h as f32);
    let uniforms = Uniforms::for_pass(pass, (w, h), (w, h), frame, source_rect);
    let ubuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("golden-uniforms"),
        size: size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue.write_buffer(&ubuf, 0, &uniforms.as_bytes());

    let shader = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("golden-shader"),
            source: wgpu::ShaderSource::Wgsl(compose(&pass.source).into()),
        });

    let layout = gpu
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("golden-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
    let view = input.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("golden-bg"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: ubuf.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("golden-pl"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
    let pipeline = gpu
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("golden-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

    let target = gpu.target(w, h);
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("golden-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        render.set_pipeline(&pipeline);
        render.set_bind_group(0, &bind_group, &[]);
        render.draw(0..3, 0..1);
    }
    gpu.queue.submit(Some(encoder.finish()));
    gpu.read_back(&target, w, h)
}

/// A deterministic test image: coloured vertical bars over a horizontal gradient.
///
/// Bars give the NTSC pass real vertical edges to smear (a flat image would make chroma bleed
/// invisible and the test vacuous), and the gradient gives the CRT pass rows that differ so
/// scanline darkening is measurable.
fn test_image(w: u32, h: u32) -> Readback {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let bar = (x / 8) % 3;
            #[allow(clippy::cast_possible_truncation)]
            let level = ((y * 255) / h.max(1)) as u8;
            let px = match bar {
                0 => [255, level, 0, 255],
                1 => [0, 255, level, 255],
                _ => [level, 0, 255, 255],
            };
            rgba.extend_from_slice(&px);
        }
    }
    Readback {
        rgba,
        width: w,
        height: h,
    }
}

/// A pass that copies its input, for verifying the harness itself round-trips exactly.
const IDENTITY: &str = r"
@fragment
fn fs_main(in: StackVsOut) -> @location(0) vec4<f32> {
    return textureSample(stack_tex, stack_samp, in.uv);
}
";

#[test]
fn the_harness_round_trips_an_image_exactly() {
    crate::gpu_test!(gpu);
    // If a nearest-sampled identity pass does not reproduce its input byte for byte, every other
    // assertion in this file is measuring the harness rather than the shader.
    let src = test_image(64, 32);
    let out = render_pass(&gpu, &PassDesc::new("identity", IDENTITY), &src, 0);
    assert_eq!(out.rgba, src.rgba, "identity pass must be byte-exact");
    assert_eq!(out.hash(), src.hash());
}

/// A pass sees the IMAGE, not the backing texture it happens to sit in.
///
/// The live frontend allocates one framebuffer texture at the maximum size and writes the current
/// 256x224 (or hi-res) image into its top-left corner, so pass 0's input is a sub-rect. Before the
/// `source_rect` uniform existed, the shared vertex stage emitted plain `0..1` UVs, so pass 0
/// stretched the whole allocation — mostly never-written texels — across the screen. Rendering the
/// same image at `pad = 1` and `pad = 2` must therefore agree byte for byte.
#[test]
fn a_padded_backing_texture_renders_the_same_image() {
    crate::gpu_test!(gpu);
    let src = test_image(64, 32);
    // Nearest sampling and an identity pass, so any difference is the UV mapping and nothing else.
    let pass = PassDesc::new("identity", IDENTITY);
    let exact = render_pass_padded(&gpu, &pass, &src, 0, 1);
    let padded = render_pass_padded(&gpu, &pass, &src, 0, 2);
    assert_eq!(
        padded.rgba, exact.rgba,
        "a 2x-oversized backing texture must not change the picture"
    );
    // And the image is genuinely the source, not merely self-consistent between the two runs.
    assert_eq!(padded.rgba, src.rgba);
}

/// The effects that reason in image space stay put when the backing texture grows.
///
/// The CRT pass curves, vignettes, and glows around the *picture's* centre. Those are computed
/// from `image_uv`/`out_texel` rather than from the raw UV precisely so an oversized backing
/// texture cannot move them — this asserts that, for a pass with every such knob engaged.
#[test]
fn image_space_effects_are_independent_of_the_backing_texture_size() {
    crate::gpu_test!(gpu);
    let src = test_image(64, 32);
    let pass = crt_pass(0.5, 0.5, 0.4, 0.5, 0.3, 0.4);
    let exact = render_pass_padded(&gpu, &pass, &src, 0, 1);
    let padded = render_pass_padded(&gpu, &pass, &src, 0, 2);
    assert_eq!(
        padded.hash(),
        exact.hash(),
        "curvature/vignette/glow must key on the image, not the allocation"
    );
}

#[test]
fn rendering_is_deterministic() {
    crate::gpu_test!(gpu);
    // A golden is worthless if the same input hashes differently between runs.
    let src = test_image(64, 32);
    let pass = crt_pass(0.5, 0.5, 0.3, 0.5, 0.2, 0.3);
    let a = render_pass(&gpu, &pass, &src, 0);
    let b = render_pass(&gpu, &pass, &src, 0);
    assert_eq!(a.hash(), b.hash(), "the same render must hash identically");
}

/// The CRT stack pass with explicit knob values.
fn crt_pass(
    scanline: f32,
    mask: f32,
    curvature: f32,
    beam: f32,
    glow: f32,
    vignette: f32,
) -> PassDesc {
    PassDesc::new("crt", rustysnes_gfx_shaders::CRT_STACK_WGSL).with_params(vec![
        Param::unit("scanline", "Scanlines", scanline),
        Param::unit("mask", "Aperture mask", mask),
        Param::unit("curvature", "Curvature", curvature),
        Param::unit("beam", "Beam shape", beam),
        Param::unit("glow", "Glow", glow),
        Param::unit("vignette", "Vignette", vignette),
    ])
}

/// The NTSC stack pass with explicit knob values.
fn ntsc_pass(bleed: f32, artifacts: f32, fringing: f32, crawl: f32) -> PassDesc {
    PassDesc::new("ntsc", rustysnes_gfx_shaders::NTSC_STACK_WGSL).with_params(vec![
        Param::unit("bleed", "Chroma bleed", bleed),
        Param::unit("artifacts", "Artifacts", artifacts),
        Param::unit("fringing", "Fringing", fringing),
        Param::unit("crawl", "Dot crawl", crawl),
    ])
}

#[test]
fn ntsc_is_an_exact_bypass_with_every_knob_at_zero() {
    crate::gpu_test!(gpu);
    // The same contract `crate::eq` holds for audio: a pass with everything off must be bit-exact,
    // which is what makes it safe to leave in the chain permanently.
    let src = test_image(64, 32);
    let out = render_pass(&gpu, &ntsc_pass(0.0, 0.0, 0.0, 0.0), &src, 0);
    assert_eq!(
        out.rgba, src.rgba,
        "every NTSC knob at zero must be a bit-exact pass-through"
    );
}

#[test]
fn ntsc_chroma_bleed_smears_horizontally_only() {
    crate::gpu_test!(gpu);
    let src = test_image(64, 32);
    let bled = render_pass(&gpu, &ntsc_pass(1.0, 0.0, 0.0, 0.0), &src, 0);
    assert_ne!(bled.rgba, src.rgba, "bleed must change the image");

    // The defining property of composite video: chroma bandwidth is reduced horizontally while
    // luma stays sharp. Measured as horizontal variance dropping while the row-to-row structure
    // (which the bars do not vary along) is untouched.
    let row_variation = |img: &Readback, y: u32| -> u32 {
        (1..img.width)
            .filter_map(|x| {
                let a = img.pixel(x - 1, y)?;
                let b = img.pixel(x, y)?;
                Some(u32::from(a[0].abs_diff(b[0])) + u32::from(a[2].abs_diff(b[2])))
            })
            .sum()
    };
    let before = row_variation(&src, 16);
    let after = row_variation(&bled, 16);
    assert!(
        after < before,
        "chroma bleed should reduce horizontal colour variation ({before} -> {after})"
    );
}

#[test]
fn ntsc_dot_crawl_advances_with_the_frame_counter() {
    crate::gpu_test!(gpu);
    // The phase walks with the frame, so a static input must still differ frame to frame — that is
    // the artefact's entire visible signature.
    let src = test_image(64, 32);
    let pass = ntsc_pass(0.0, 0.0, 0.0, 1.0);
    let f0 = render_pass(&gpu, &pass, &src, 0);
    let f1 = render_pass(&gpu, &pass, &src, 1);
    assert_ne!(
        f0.hash(),
        f1.hash(),
        "dot crawl must differ between frames, or it is not crawling"
    );
    // And it repeats: the sine phase has period 4 in the frame counter.
    let f4 = render_pass(&gpu, &pass, &src, 4);
    assert_eq!(f0.hash(), f4.hash(), "the crawl phase should cycle");
}

#[test]
fn crt_scanlines_darken_alternate_rows() {
    crate::gpu_test!(gpu);
    let src = test_image(64, 32);
    let plain = render_pass(&gpu, &crt_pass(0.0, 0.0, 0.0, 0.0, 0.0, 0.0), &src, 0);
    assert_eq!(
        plain.rgba, src.rgba,
        "every CRT knob at zero must be a bit-exact pass-through"
    );

    let lined = render_pass(&gpu, &crt_pass(1.0, 0.0, 0.0, 0.0, 0.0, 0.0), &src, 0);
    let luma = |img: &Readback, y: u32| -> u32 {
        (0..img.width)
            .filter_map(|x| img.pixel(x, y))
            .map(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]))
            .sum()
    };
    // Alternate rows must differ from each other after scanlines, and the darker one must be
    // darker than the same row was before the pass.
    let (a, b) = (luma(&lined, 8), luma(&lined, 9));
    assert_ne!(a, b, "scanlines must make alternate rows differ");
    let dark_row = if a < b { 8 } else { 9 };
    assert!(
        luma(&lined, dark_row) < luma(&src, dark_row),
        "the darkened row must be darker than the source"
    );
}

#[test]
fn crt_curvature_blacks_out_the_corners_rather_than_smearing_them() {
    crate::gpu_test!(gpu);
    // A clamped edge texel smeared around a curved border is the classic broken-looking artefact;
    // this asserts the shader returns black outside instead.
    let src = test_image(64, 32);
    let curved = render_pass(&gpu, &crt_pass(0.0, 0.0, 1.0, 0.0, 0.0, 0.0), &src, 0);
    let corner = curved.pixel(0, 0).expect("corner");
    assert_eq!(
        [corner[0], corner[1], corner[2]],
        [0, 0, 0],
        "a curved corner samples outside the image and must be black"
    );
    // The centre is untouched by curvature (r2 = 0 there).
    let centre = curved.pixel(32, 16).expect("centre");
    let src_centre = src.pixel(32, 16).expect("centre");
    assert_eq!(centre, src_centre, "curvature must not move the centre");
}

#[test]
fn crt_mask_tints_columns_in_an_rgb_cycle() {
    crate::gpu_test!(gpu);
    let src = test_image(64, 32);
    let masked = render_pass(&gpu, &crt_pass(0.0, 1.0, 0.0, 0.0, 0.0, 0.0), &src, 0);
    // With mask at full strength each column keeps exactly one primary from its source pixel.
    for x in 0..3u32 {
        let out = masked.pixel(x, 16).expect("px");
        let src_px = src.pixel(x, 16).expect("px");
        let kept = (x % 3) as usize;
        assert_eq!(
            out[kept], src_px[kept],
            "column {x} must keep channel {kept} intact"
        );
        for (other, channel) in out.iter().enumerate().take(3) {
            if other != kept {
                assert_eq!(*channel, 0, "column {x} must zero channel {other}");
            }
        }
    }
}
