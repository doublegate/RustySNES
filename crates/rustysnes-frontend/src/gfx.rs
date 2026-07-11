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
//! See `docs/frontend.md` §Presentation post-filters for the render-path architecture.
//!
//! `v1.2.0`: two optional post-filters (CRT scanlines+mask, an HQ2x-style edge-directed blend —
//! see [`crate::config::PostFilter`]) layer on top of the plain blit above. RustyNES's NTSC
//! composite-signal simulation and `.slangp` shader-preset loading are explicitly out of scope.

#![allow(clippy::cast_possible_truncation)]

use std::sync::Arc;

use wgpu::util::DeviceExt as _;
use winit::window::Window;

// `v1.2.0`: the SNES video geometry constants + the BGR555->RGBA8 decode moved to
// `rustysnes_core::facade` (relocated alongside `EmuCore` — a libretro core or any other headless
// embedder needs them too, not just this wgpu-based frontend). Re-exported here so every existing
// `crate::gfx::SNES_W`/`MAX_W`/`bgr555_to_rgba8` call site in this crate keeps working unchanged.
pub use rustysnes_core::facade::{
    MAX_H, MAX_W, SNES_H_HIRES, SNES_H_NTSC, SNES_H_PAL, SNES_W, SNES_W_HIRES, bgr555_to_rgba8,
};

/// The SNES display aspect ratio (4:3) the blit letterboxes the framebuffer into.
const TARGET_ASPECT: f32 = 4.0 / 3.0;

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
    /// The surface's supported present modes (queried once at init), so a later Settings change
    /// can re-validate a requested mode against the hardware without re-acquiring the adapter.
    present_modes: Vec<wgpu::PresentMode>,
    /// The [`crate::config::PostFilter::Crt`] pipeline (`v1.2.0`) — always built (cheap, one
    /// small pipeline) regardless of whether the filter is active, so switching it on in
    /// Settings needs no reallocation.
    crt: FilterPipeline,
    /// The [`crate::config::PostFilter::Hqx`] pipeline (`v1.2.0`) — see [`Self::crt`].
    hqx: FilterPipeline,
}

/// A post-filter's pipeline + its own bind group + its own uniform buffer (`v1.2.0`). [`Gfx::crt`]
/// and [`Gfx::hqx`] each own one, built once at [`Gfx::new_async`] time and selected by
/// [`Gfx::present`]; [`Gfx::blit`]'s own `pipeline`/`bind_group`/`uniform_buf` fields are a separate,
/// untouched pair (the [`crate::config::PostFilter::None`] path).
struct FilterPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    /// 32 bytes: bytes `0..16` = `uv_scale.xy, pos_scale.xy` (the same letterbox uniform
    /// [`Gfx::blit`] uses), bytes `16..32` = the filter's own `params` (meaning is
    /// filter-specific — see [`CRT_WGSL`]/[`HQX_WGSL`]'s own doc for the layout each expects).
    uniform_buf: wgpu::Buffer,
}

impl Gfx {
    /// Initialize wgpu against `window`. Blocks on adapter/device acquisition via `pollster`.
    /// Native only — `wasm32`'s wgpu init is genuinely async (`pollster::block_on` cannot block
    /// on wasm32); the `wasm-winit` frontend (`app.rs`) calls [`Self::new_async`] directly instead.
    ///
    /// # Errors
    /// Returns a [`GfxError`] if no compatible adapter is found or device request fails.
    #[cfg(not(target_arch = "wasm32"))]
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
        // `instance` itself isn't needed past adapter/surface/device creation — `Gfx` doesn't
        // retain it as a field, matching the pre-`wasm32` code's shape.
        #[cfg(target_arch = "wasm32")]
        let (_instance, surface, adapter) =
            Self::create_instance_surface_adapter_wasm(window).await?;
        #[cfg(not(target_arch = "wasm32"))]
        let (_instance, surface, adapter) = {
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
            (instance, surface, adapter)
        };
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
        // Color-space handling differs by backend. On native (Vulkan/Metal/DX12) we render
        // through an sRGB surface and store the framebuffer in an sRGB texture: the sampler's
        // sRGB->linear decode and the surface's linear->sRGB encode round-trip to identity, so
        // the SNES palette bytes reach the screen untouched.
        //
        // On the WebGL2 backend (wgpu `Backend::Gl`, the GitHub Pages fallback when WebGPU is
        // absent) that round-trip is NOT identity: wgpu-hal's GL surface cannot present to a real
        // sRGB default framebuffer, so when the surface format `is_srgb()` it renders into an
        // intermediate sRGB texture and runs an EXTRA explicit linear-to-sRGB encode at present
        // time. Combined with GL's automatic sRGB framebuffer encoding on that intermediate
        // write, the encode count no longer matches the decode count and the palette comes out
        // wrong (washed out / too dark) — a real bug RustyNES's own wasm-winit path hit and fixed
        // (`ref-proj` equivalent research, confirmed by reading its `gfx.rs` directly), not a
        // hypothetical. Fix: on the GL backend, keep EVERYTHING in the UNORM domain (non-sRGB
        // surface + non-sRGB framebuffer texture); with the plain pass-through blit shader here
        // (no manual color math) that performs zero color conversion anywhere, matching the
        // wasm-canvas path's byte-exact output.
        let is_gl_backend = adapter.get_info().backend == wgpu::Backend::Gl;
        let format = if is_gl_backend {
            caps.formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0])
        } else {
            caps.formats
                .iter()
                .copied()
                .find(wgpu::TextureFormat::is_srgb)
                .unwrap_or(caps.formats[0])
        };
        // The framebuffer texture's sRGB-ness MUST match the surface's so the sample-decode /
        // write-encode pair round-trips to identity (sRGB pair on native, UNORM pair on WebGL2).
        let framebuffer_format = if format.is_srgb() {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
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
            format: framebuffer_format,
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
        let crt = Self::build_filter_pipeline(
            &device,
            format,
            &view,
            &sampler,
            CRT_WGSL,
            "rustysnes-crt",
        );
        let hqx = Self::build_filter_pipeline(
            &device,
            format,
            &view,
            &sampler,
            HQX_WGSL,
            "rustysnes-hqx",
        );
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
            present_modes: caps.present_modes,
            crt,
            hqx,
        })
    }

    /// Build one [`FilterPipeline`] (`v1.2.0`) — same bind-group-layout shape for both
    /// [`Self::crt`]/[`Self::hqx`] (texture + sampler + a 32-byte uniform, twice the plain blit's
    /// 16-byte one to also carry the filter's own `params`), differing only in `shader_src`.
    fn build_filter_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        shader_src: &str,
        label: &str,
    ) -> FilterPipeline {
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[0.0f32; 8]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
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
        FilterPipeline {
            pipeline,
            bind_group,
            uniform_buf,
        }
    }

    /// `wasm32`-only: pick WebGPU or WebGL2 (`Backend::Gl`) BEFORE ever touching the canvas, and
    /// commit to exactly one.
    ///
    /// Two real, hardware-confirmed constraints rule out a "try WebGPU, fall back to GL"
    /// sequential attempt on the SAME canvas (both found by direct testing in a real headless
    /// browser, not assumed):
    /// 1. A single `Instance` requesting `Backends::BROWSER_WEBGPU | Backends::GL` together does
    ///    NOT behave as an in-instance fallback the way multiple NATIVE backends do (Vulkan |
    ///    Metal | DX12 genuinely do fall back within one `wgpu-core` instance) — wgpu's `wasm32`
    ///    target picks exactly ONE top-level implementation per `Instance` based on which bit is
    ///    set (the pure-JS `webgpu` feature's direct `navigator.gpu` passthrough, entirely
    ///    separate Rust code from the `webgl` feature's `wgpu-core`/`wgpu-hal` `Gles` backend).
    /// 2. A `<canvas>` element can only bind ONE context type for its entire lifetime —
    ///    `Instance::create_surface` on a `BROWSER_WEBGPU`-backed instance calls
    ///    `canvas.getContext("webgpu")` immediately (regardless of whether `request_adapter`
    ///    later succeeds), which permanently poisons the canvas: a SUBSEQUENT `Backends::GL`
    ///    attempt on the same element fails with `"canvas.getContext() returned null; webgl2 not
    ///    available or canvas already in use"`, even on a build that renders correctly when GL is
    ///    the ONLY backend ever requested. So the two-attempt loop this function replaced could
    ///    never actually reach its own fallback.
    ///
    /// The fix: probe `navigator.gpu`'s mere PRESENCE (not a real context/adapter attempt) to
    /// decide the backend, then touch the canvas exactly once with that single choice. This means
    /// a browser that advertises `navigator.gpu` but then fails a real WebGPU adapter request for
    /// some other reason (disabled by flag, blocklisted) surfaces a hard error rather than falling
    /// back to GL — a real, documented limitation, not silently pretended away.
    ///
    /// # Errors
    /// Returns a [`GfxError`] if the chosen backend produces no working adapter.
    #[cfg(target_arch = "wasm32")]
    async fn create_instance_surface_adapter_wasm(
        window: Arc<Window>,
    ) -> Result<(wgpu::Instance, wgpu::Surface<'static>, wgpu::Adapter), GfxError> {
        let backends = if wasm_navigator_has_gpu() {
            wgpu::Backends::BROWSER_WEBGPU
        } else {
            wgpu::Backends::GL
        };
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backends;
        let instance = wgpu::Instance::new(desc);
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
        Ok((instance, surface, adapter))
    }

    /// Re-apply a present-mode preference to the live surface (the Settings → Video toggle).
    ///
    /// Re-validates `pref` against the surface's supported modes, rewrites the surface
    /// configuration, and reconfigures so the new vsync/tearing behavior takes effect on the next
    /// present. Returns the mode actually applied (which falls back to `Fifo` if the request is
    /// unsupported). A no-op if the resolved mode already matches the live one.
    pub fn set_present_mode(&mut self, pref: &str) -> wgpu::PresentMode {
        let mode = select_present_mode(pref, &self.present_modes);
        if mode == self.config.present_mode {
            return mode;
        }
        self.config.present_mode = mode;
        self.surface.configure(&self.device, &self.config);
        mode
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
    ///
    /// This is the [`crate::config::PostFilter::None`] path — kept completely unchanged by the
    /// `v1.2.0` filter pipeline addition (see [`Self::present`]) so "no post-process filter active" stays
    /// pixel-identical to a filter-less build, not just visually similar.
    // The blit math casts small screen/texture dimensions (all well under 2^23) to f32 for the
    // UV + letterbox ratios; the precision loss is irrelevant for these layout fractions.
    #[allow(clippy::cast_precision_loss)]
    pub fn blit(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        // (a) UV scale: the live sub-rect within the MAX_W × MAX_H texture.
        let uv_x = self.fb_w as f32 / MAX_W as f32;
        let uv_y = self.fb_h as f32 / MAX_H as f32;
        // (b) Letterbox: fit the 4:3 SNES display aspect inside the surface.
        let (pos_x, pos_y) = self.letterbox_scale();
        self.queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::cast_slice(&[uv_x, uv_y, pos_x, pos_y]),
        );

        let mut pass = Self::begin_pass(encoder, target);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Present the framebuffer through `filter`, falling back to the unchanged [`Self::blit`]
    /// (`PostFilter::None`) path. `crt_scanline`/`crt_mask`/`hqx_strength` are only read by their
    /// matching filter (`v1.2.0`).
    // The blit math casts small screen/texture dimensions (all well under 2^23) to f32 for the
    // UV + letterbox ratios; the precision loss is irrelevant for these layout fractions.
    #[allow(clippy::cast_precision_loss)]
    pub fn present(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        filter: crate::config::PostFilter,
        crt_scanline: f32,
        crt_mask: f32,
        hqx_strength: f32,
    ) {
        use crate::config::PostFilter;
        let (fp, params) = match filter {
            PostFilter::None => {
                self.blit(encoder, target);
                return;
            }
            PostFilter::Crt => (&self.crt, [crt_scanline, crt_mask, self.fb_h as f32, 0.0]),
            PostFilter::Hqx => (
                &self.hqx,
                [self.fb_w as f32, self.fb_h as f32, hqx_strength, 0.0],
            ),
        };
        let uv_x = self.fb_w as f32 / MAX_W as f32;
        let uv_y = self.fb_h as f32 / MAX_H as f32;
        let (pos_x, pos_y) = self.letterbox_scale();
        self.queue.write_buffer(
            &fp.uniform_buf,
            0,
            bytemuck::cast_slice(&[uv_x, uv_y, pos_x, pos_y]),
        );
        self.queue
            .write_buffer(&fp.uniform_buf, 16, bytemuck::cast_slice(&params));

        let mut pass = Self::begin_pass(encoder, target);
        pass.set_pipeline(&fp.pipeline);
        pass.set_bind_group(0, &fp.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// The aspect-correct letterbox scale `(pos_x, pos_y)` applied to the fullscreen triangle's
    /// clip-space position so the 4:3 SNES display fits inside the current surface regardless of
    /// window shape — shared by [`Self::blit`] and [`Self::present`]'s filter passes (`v1.2.0`;
    /// extracted from `blit`'s own inline math, a pure behavior-preserving refactor).
    #[allow(clippy::cast_precision_loss)]
    fn letterbox_scale(&self) -> (f32, f32) {
        let win_w = self.config.width.max(1) as f32;
        let win_h = self.config.height.max(1) as f32;
        let win_aspect = win_w / win_h;
        if win_aspect > TARGET_ASPECT {
            (TARGET_ASPECT / win_aspect, 1.0) // window too wide -> pillarbox
        } else {
            (1.0, win_aspect / TARGET_ASPECT) // window too tall -> letterbox
        }
    }

    /// Shared render-pass setup (clear to black, single color attachment) — the only difference
    /// between [`Self::blit`] and [`Self::present`]'s filter passes is which pipeline/bind group
    /// gets bound afterward.
    fn begin_pass<'e>(
        encoder: &'e mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'e> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        })
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

/// The [`crate::config::PostFilter::Crt`] post-pass: scanlines + an RGB aperture-grille mask,
/// approximating a CRT's phosphor structure. Same vertex-shader letterbox convention as
/// [`BLIT_WGSL`] (clip-space position scale, not UV-space cropping) — `scale.xy` = UV scale
/// (live sub-rect), `scale.zw` = letterbox position scale; `strength.x` = scanline intensity,
/// `strength.y` = aperture-mask intensity, `strength.z` = source scanline count (the active
/// framebuffer height), `strength.w` unused.
const CRT_WGSL: &str = r"
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

/// The [`crate::config::PostFilter::Hqx`] post-pass: a single-pass, edge-directed diagonal blend
/// — an HQ2x-STYLE *approximation* (a diagonal-similarity heuristic in the 2xSaI/Eagle family),
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
const HQX_WGSL: &str = r"
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

/// Whether `navigator.gpu` exists (a browser advertises SOME WebGPU support) — a cheap presence
/// check via `js_sys::Reflect`, not a real context/adapter attempt, so it never touches a canvas.
/// See [`Gfx::create_instance_surface_adapter_wasm`] for why this decides the backend up front
/// instead of trying WebGPU and falling back to GL on failure.
#[cfg(target_arch = "wasm32")]
fn wasm_navigator_has_gpu() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    js_sys::Reflect::get(&window.navigator(), &wasm_bindgen::JsValue::from_str("gpu"))
        .is_ok_and(|v| !v.is_undefined() && !v.is_null())
}

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

    /// Parse + validate an embedded WGSL shader with the same naga machinery wgpu uses at
    /// runtime — shared by the `crt`/`hqx` validity tests below (`v1.2.0`).
    fn validate_wgsl(src: &str) {
        let module = naga::front::wgsl::parse_str(src).expect("WGSL parses");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("WGSL validates");
    }

    #[test]
    fn crt_wgsl_validates() {
        validate_wgsl(CRT_WGSL);
    }

    #[test]
    fn hqx_wgsl_validates() {
        validate_wgsl(HQX_WGSL);
    }

    /// The letterbox scale extracted into [`Gfx::letterbox_scale`] must reproduce the exact
    /// values `blit`'s own inline math computed before the `v1.2.0` refactor — a pure
    /// behavior-preserving extraction, verified against hand-computed cases for a window
    /// wider than / narrower than / exactly the 4:3 SNES aspect.
    // The `1.0` literals below are the exact value the pillarbox/letterbox branch returns
    // unchanged (not the result of a division that could round) — an exact comparison is
    // correct, not merely close-enough.
    #[allow(clippy::float_cmp)]
    #[test]
    fn letterbox_scale_matches_known_cases() {
        // Window exactly 4:3 -> no letterboxing either axis.
        assert_eq!(letterbox_scale_pure(800.0, 600.0), (1.0, 1.0));
        // Window wider than 4:3 (16:9) -> pillarbox: pos_x shrinks, pos_y stays full.
        let (px, py) = letterbox_scale_pure(1920.0, 1080.0);
        assert!((px - (4.0 / 3.0) / (1920.0 / 1080.0)).abs() < 1e-6);
        assert_eq!(py, 1.0);
        // Window narrower than 4:3 (3:4, portrait) -> letterbox: pos_y shrinks, pos_x stays full.
        let (px2, py2) = letterbox_scale_pure(600.0, 800.0);
        assert_eq!(px2, 1.0);
        assert!((py2 - (600.0 / 800.0) / (4.0 / 3.0)).abs() < 1e-6);
    }

    /// A free-function mirror of [`Gfx::letterbox_scale`]'s math (which needs a live `Gfx` with a
    /// real wgpu device to construct) — kept byte-for-byte identical to that method's body so this
    /// test exercises the exact same formula without needing a GPU in CI.
    fn letterbox_scale_pure(win_w: f32, win_h: f32) -> (f32, f32) {
        let win_aspect = win_w / win_h;
        if win_aspect > TARGET_ASPECT {
            (TARGET_ASPECT / win_aspect, 1.0)
        } else {
            (1.0, win_aspect / TARGET_ASPECT)
        }
    }
}
