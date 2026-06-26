//! wgpu surface + texture-blit pipeline for the SNES framebuffer + the BGR555 palette decode.
//!
//! The PPU emits an RGBA8 framebuffer. Each frame the frontend uploads it to a wgpu texture; a
//! fullscreen-triangle render pass samples that texture with nearest filtering and
//! aspect-ratio-correct letterbox. With no post-process filter active the direct nearest-blit
//! is taken and the output is pixel-identical to a filter-less build.
//!
//! SNES specifics vs. the NES template:
//! - Framebuffer dims: 256x224 (NTSC active) / 256x239 (PAL active) / 512x448 (hi-res / pseudo).
//! - Color decode: the SNES CGRAM stores 15-bit **BGR555** (`0bbbbbgggggrrrrr`); [`bgr555_to_rgba8`]
//!   expands the 5-bit channels to 8-bit (the standard `c << 3 | c >> 2` left-justify).
//!
//! See `docs/frontend.md` (filled by the docs agent) for the render-path architecture.
//!
//! v0.1.0: the deep post-process chain (CRT / NTSC / upscalers) is a TODO — only the direct
//! nearest-blit ships in the skeleton.

#![allow(clippy::cast_possible_truncation)]

use std::sync::Arc;

use wgpu::util::DeviceExt as _;
use winit::window::Window;

/// SNES native width (constant across the NTSC/PAL active-region heights and the lo-res mode).
pub const SNES_W: u32 = 256;
/// SNES NTSC active-region height (224 visible scanlines).
pub const SNES_H_NTSC: u32 = 224;
/// SNES PAL active-region height (239 visible scanlines).
pub const SNES_H_PAL: u32 = 239;
/// SNES hi-res / pseudo-hi-res width (mode 5/6 + interlace double the base dims).
pub const SNES_W_HIRES: u32 = 512;
/// SNES hi-res / interlace height (448 = 224 active * 2 fields).
pub const SNES_H_HIRES: u32 = 448;

/// The maximum framebuffer the texture is sized for (hi-res worst case), so a mode change
/// never needs a texture realloc. Sub-modes upload into the top-left sub-rect.
pub const MAX_W: u32 = SNES_W_HIRES;
/// The maximum framebuffer height the texture is sized for (see [`MAX_W`]).
pub const MAX_H: u32 = SNES_H_HIRES;

/// The SNES display aspect ratio (4:3) the blit letterboxes the framebuffer into.
const TARGET_ASPECT: f32 = 4.0 / 3.0;

/// Expand a 15-bit SNES **BGR555** color word (`0bbbbbgggggrrrrr`) to a packed little-endian
/// RGBA8 (`0xAABBGGRR`) value suitable for an RGBA8 framebuffer / wgpu texture upload.
///
/// The 5-bit channels are left-justified to 8 bits (`c << 3 | c >> 2`), matching how Mesen2 /
/// bsnes expand CGRAM. Alpha is forced opaque.
#[must_use]
pub const fn bgr555_to_rgba8(bgr555: u16) -> u32 {
    let r5 = (bgr555 & 0x1F) as u32;
    let g5 = ((bgr555 >> 5) & 0x1F) as u32;
    let b5 = ((bgr555 >> 10) & 0x1F) as u32;
    let r8 = (r5 << 3) | (r5 >> 2);
    let g8 = (g5 << 3) | (g5 >> 2);
    let b8 = (b5 << 3) | (b5 >> 2);
    // Pack as 0xAABBGGRR (little-endian RGBA8 byte order: R,G,B,A).
    0xFF00_0000 | (b8 << 16) | (g8 << 8) | r8
}

/// Resolve the configured present-mode string against the surface's supported modes.
///
/// Recognized (case-insensitive): `"fifo"` (vsync; safe default), `"mailbox"` (triple-buffered,
/// no tearing, no vsync gate), `"immediate"` (uncapped, may tear). An unsupported request falls
/// back to `Fifo`, which every wgpu backend guarantees. The native wall-clock pacer is the
/// authoritative timing source.
fn select_present_mode(pref: &str, supported: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    let requested = match pref.to_ascii_lowercase().as_str() {
        "mailbox" => wgpu::PresentMode::Mailbox,
        "immediate" => wgpu::PresentMode::Immediate,
        _ => wgpu::PresentMode::Fifo,
    };
    if supported.contains(&requested) {
        requested
    } else {
        wgpu::PresentMode::Fifo
    }
}

/// The wgpu device + surface + the framebuffer-blit pipeline.
///
/// Owns the streaming texture the PPU framebuffer uploads into each frame and the
/// fullscreen-triangle pass that samples it. The egui pass (the always-on shell) is layered on
/// top by the caller after this blit.
pub struct Gfx {
    /// The wgpu device (kept for resource creation + the per-frame upload).
    pub device: wgpu::Device,
    /// The command queue (texture uploads + submit).
    pub queue: wgpu::Queue,
    /// The window surface presented each frame.
    pub surface: wgpu::Surface<'static>,
    /// The negotiated surface configuration (format + size + present mode).
    pub config: wgpu::SurfaceConfiguration,
    /// The streaming framebuffer texture (sized to the hi-res worst case).
    texture: wgpu::Texture,
    /// The bind group binding `texture` + the nearest sampler + the blit uniform.
    bind_group: wgpu::BindGroup,
    /// The fullscreen-triangle blit pipeline.
    pipeline: wgpu::RenderPipeline,
    /// The blit uniform (`vec4<f32>` = `uv_scale.xy`, `pos_scale.xy`): the UV sub-rect of the live
    /// framebuffer within the oversized texture + the aspect-correct letterbox scale. Rewritten
    /// each `blit`.
    uniform_buf: wgpu::Buffer,
    /// The active framebuffer dimensions (the sub-rect of the texture that's live this mode).
    fb_w: u32,
    /// See [`Gfx::fb_w`].
    fb_h: u32,
}

impl Gfx {
    /// Initialize wgpu against `window`. Blocks on adapter/device acquisition via `pollster`
    /// on native (the wasm path uses the async constructor — TODO when `wasm.rs` is filled).
    ///
    /// # Errors
    /// Returns a [`GfxError`] if no compatible adapter is found or device request fails.
    pub fn new(window: Arc<Window>, present_pref: &str) -> Result<Self, GfxError> {
        pollster::block_on(Self::new_async(window, present_pref))
    }

    /// The async core of [`Gfx::new`] (shared with the future wasm path).
    ///
    /// # Errors
    /// See [`Gfx::new`].
    // Linear wgpu device/surface/pipeline setup: one straight-line init sequence that reads more
    // clearly as a unit than split across helpers.
    #[allow(clippy::too_many_lines)]
    pub async fn new_async(window: Arc<Window>, present_pref: &str) -> Result<Self, GfxError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|e| GfxError::Surface(e.to_string()))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .map_err(|e| GfxError::Adapter(e.to_string()))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("rustysnes-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .map_err(|e| GfxError::Device(e.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: select_present_mode(present_pref, &caps.present_modes),
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // The streaming framebuffer texture, sized to the hi-res worst case; sub-modes upload
        // into the top-left sub-rect, so a mode change never reallocates.
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustysnes-framebuffer"),
            size: wgpu::Extent3d {
                width: MAX_W,
                height: MAX_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rustysnes-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            // Clamp so the sub-rect sample never bleeds into the unused texture region.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        // The blit uniform: uv_scale (live sub-rect within the oversized texture) + pos_scale
        // (aspect-correct letterbox). Initialized to the full-texture identity; rewritten per blit.
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rustysnes-blit-uniform"),
            contents: bytemuck::cast_slice(&[1.0f32, 1.0, 1.0, 1.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustysnes-blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rustysnes-blit-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustysnes-blit-bg"),
            layout: &bgl,
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
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustysnes-blit-pl"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustysnes-blit-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            device,
            queue,
            surface,
            config,
            texture,
            bind_group,
            pipeline,
            uniform_buf,
            fb_w: SNES_W,
            fb_h: SNES_H_NTSC,
        })
    }

    /// Re-negotiate the surface on a window resize.
    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
    }

    /// Upload an RGBA8 framebuffer (`w*h*4` bytes) into the streaming texture's top-left
    /// sub-rect and record the active mode dims. A length mismatch is skipped (mirrors the
    /// RustyNES ROM-close fix: never feed wgpu an empty/short source).
    pub fn upload(&mut self, rgba: &[u8], w: u32, h: u32) {
        if w == 0 || h == 0 || w > MAX_W || h > MAX_H {
            return;
        }
        if rgba.len() < (w as usize) * (h as usize) * 4 {
            return;
        }
        self.fb_w = w;
        self.fb_h = h;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
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
    }

    /// The active framebuffer dimensions (`(w, h)`), for the caller's aspect math.
    #[must_use]
    pub const fn fb_dims(&self) -> (u32, u32) {
        (self.fb_w, self.fb_h)
    }

    /// Acquire the next surface texture for the frame, or `None` if the surface is lost
    /// (the caller reconfigures and retries next frame).
    ///
    /// wgpu 29 returns the [`wgpu::CurrentSurfaceTexture`] enum (not a `Result`): use the
    /// texture on `Success`/`Suboptimal`, reconfigure on `Lost`/`Outdated`, skip otherwise.
    pub fn acquire(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => Some(t),
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                None
            }
            _ => None,
        }
    }

    /// Record the framebuffer blit into `encoder`, clearing then drawing the fullscreen
    /// triangle that samples the streaming texture. The egui shell pass is layered after this.
    ///
    /// Before drawing it rewrites the blit uniform with (a) the UV sub-rect of the live
    /// framebuffer inside the oversized texture and (b) the aspect-correct letterbox scale, so
    /// only the `fb_w × fb_h` content shows and it keeps the 4:3 SNES display aspect regardless of
    /// the window shape.
    // The blit math casts small screen/texture dimensions (all well under 2^23) to f32 for the
    // UV + letterbox ratios; the precision loss is irrelevant for these layout fractions.
    #[allow(clippy::cast_precision_loss)]
    pub fn blit(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        // (a) UV scale: the live sub-rect within the MAX_W × MAX_H texture.
        let uv_x = self.fb_w as f32 / MAX_W as f32;
        let uv_y = self.fb_h as f32 / MAX_H as f32;
        // (b) Letterbox: fit the 4:3 SNES display aspect inside the surface.
        let win_w = self.config.width.max(1) as f32;
        let win_h = self.config.height.max(1) as f32;
        let win_aspect = win_w / win_h;
        let (pos_x, pos_y) = if win_aspect > TARGET_ASPECT {
            (TARGET_ASPECT / win_aspect, 1.0) // window too wide -> pillarbox
        } else {
            (1.0, win_aspect / TARGET_ASPECT) // window too tall -> letterbox
        };
        self.queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::cast_slice(&[uv_x, uv_y, pos_x, pos_y]),
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rustysnes-blit-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The fullscreen-triangle blit shader (nearest sample of the framebuffer texture).
const BLIT_WGSL: &str = r"
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

/// wgpu initialization failures.
#[derive(Debug, thiserror::Error)]
pub enum GfxError {
    /// Surface creation failed.
    #[error("wgpu surface creation failed: {0}")]
    Surface(String),
    /// No compatible adapter was found.
    #[error("no compatible wgpu adapter: {0}")]
    Adapter(String),
    /// Device request failed.
    #[error("wgpu device request failed: {0}")]
    Device(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgr555_black_and_white() {
        assert_eq!(bgr555_to_rgba8(0x0000), 0xFF00_0000); // opaque black
        // All channels max (0x7FFF) -> opaque white.
        assert_eq!(bgr555_to_rgba8(0x7FFF), 0xFFFF_FFFF);
    }

    #[test]
    fn bgr555_pure_red() {
        // Red = low 5 bits set -> R8 = 0xFF, others 0.
        assert_eq!(bgr555_to_rgba8(0x001F), 0xFF00_00FF);
    }

    #[test]
    fn present_mode_falls_back_to_fifo() {
        let supported = [wgpu::PresentMode::Fifo];
        assert_eq!(
            select_present_mode("mailbox", &supported),
            wgpu::PresentMode::Fifo
        );
        assert_eq!(
            select_present_mode("fifo", &supported),
            wgpu::PresentMode::Fifo
        );
    }

    #[test]
    fn blit_wgsl_validates() {
        // Validate the embedded WGSL with the same naga wgpu uses at runtime.
        let module = naga::front::wgsl::parse_str(BLIT_WGSL).expect("WGSL parses");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("WGSL validates");
    }
}
