//! iOS presentation host (`v1.16.0 "Beacon"`, Mobile Phase 3) — `wgpu`-on-`CAMetalLayer`
//! rendering only. Owns no emulation state: the `SwiftUI` shell drives `rustysnes-mobile`'s
//! `MobileCore` (ROM load, `run_frame`, input, save state) through its own `UniFFI`-generated
//! Swift bindings, then hands this crate exactly `(RGBA8 framebuffer bytes, width, height)` once
//! per frame via [`rustysnes_ios_present_frame`] — the same separation of concerns
//! `rustysnes-core`/`rustysnes-frontend` already have on desktop, and the same shape
//! `rustysnes-android` (`v1.15.0`) already proved on Android, just across a plain C-ABI FFI
//! boundary instead of JNI.
//!
//! Reuses `rustysnes-gfx-shaders::BLIT_WGSL` verbatim for the actual blit pass — the same shader
//! `rustysnes-frontend::gfx` and `rustysnes-android` use for their unfiltered path, so "what the
//! framebuffer looks like on screen" stays identical across desktop/Android/iOS for the
//! unfiltered case (the `Crt`/`Hqx`/`Xbrz` post-filters are a documented follow-up, not wired
//! here yet, matching `rustysnes-android`'s own `v1.15.0` scope).
//!
//! # Verification status (honest, per `docs/mobile-readiness.md`)
//!
//! This crate's Rust source is real and `cargo build --release --target aarch64-apple-ios`
//! genuinely succeeds in a Linux sandbox with no Xcode/macOS SDK installed (confirmed by
//! actually trying it) — a `staticlib` only needs the downloaded `rust-std` component for the
//! target, deferring the link against `Metal.framework`/`UIKit.framework`/`Foundation.framework`
//! to Xcode's own final link step. What is genuinely **not** verified here: the `SwiftUI` shell
//! (`ios/`) has never been compiled by a Swift compiler, the `.xcframework` packaging has never
//! been produced, and no on-device or simulator run has happened — none of that is possible
//! without a real Mac/Xcode toolchain, which this development environment does not have. See
//! `docs/mobile-readiness.md` for the full disposition.
//!
//! # Surface lifecycle
//!
//! [`rustysnes_ios_surface_created`]/[`rustysnes_ios_surface_changed`]/
//! [`rustysnes_ios_surface_destroyed`] mirror `rustysnes-android`'s
//! `nativeSurfaceCreated`/`Changed`/`Destroyed` triple exactly, called from Swift in that order
//! (typically from a `UIViewRepresentable`'s `makeUIView`/`layoutSubviews`/`dismantleUIView`).
//! The renderer is `None` outside a created/valid surface, so a frame presented while
//! backgrounded is silently dropped rather than crashing — matching `rustysnes-android`'s own
//! contract.
//!
//! # Safety contract with the Swift caller
//!
//! [`rustysnes_ios_surface_created`]'s `ui_view` pointer is used to build a `wgpu::Surface`
//! without Rust ever taking an owning/retained reference to the `UIView` itself (unlike
//! `rustysnes-android`, which had to explicitly clone-and-hold an NDK `NativeWindow` handle after
//! finding a real premature-release bug in review — see that crate's `CHANGELOG.md` entry). The
//! reason this crate doesn't need the equivalent fix: Android's bug was a *Rust-owned* value
//! (`NativeWindow`) being dropped while still needed; here there is no such Rust-owned value —
//! the raw pointer is used directly, exactly matching `wgpu`'s own documented
//! `SurfaceTargetUnsafe::RawHandle` contract ("the target must remain valid until after the
//! returned `Surface` is dropped"). The Swift caller must keep the `UIView` alive (i.e. still
//! in the view hierarchy, not deallocated) for the entire span between
//! [`rustysnes_ios_surface_created`] and the matching [`rustysnes_ios_surface_destroyed`] call —
//! which is exactly what `UIViewRepresentable`'s own `makeUIView`/`dismantleUIView` lifecycle
//! already guarantees when this crate's calls are wired to those two methods respectively.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Mutex;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, UiKitDisplayHandle, UiKitWindowHandle, WindowHandle,
};

use rustysnes_gfx_shaders::BLIT_WGSL;

/// A thin [`HasWindowHandle`]/[`HasDisplayHandle`] wrapper around the raw `UIView*` Swift hands
/// over — `wgpu`'s `Instance::create_surface_unsafe` needs a type implementing both traits, and
/// a bare pointer doesn't. See the module doc's "Safety contract with the Swift caller" section
/// for why this does NOT take ownership of the view (unlike `rustysnes-android`'s
/// `AndroidWindowHandle`, which had to after a real bug was found there).
struct IosViewHandle {
    ui_view: NonNull<c_void>,
}

impl HasWindowHandle for IosViewHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: `ui_view` is a valid, live `UIView*` for the duration of this call, upheld by
        // the Swift caller per the module doc's safety contract.
        let handle = UiKitWindowHandle::new(self.ui_view);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::UiKit(handle)) })
    }
}

impl HasDisplayHandle for IosViewHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(
            unsafe {
                DisplayHandle::borrow_raw(RawDisplayHandle::UiKit(UiKitDisplayHandle::new()))
            },
        )
    }
}

/// The live wgpu device + surface + blit pipeline, present only between
/// [`rustysnes_ios_surface_created`] and [`rustysnes_ios_surface_destroyed`].
struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    texture: wgpu::Texture,
    texture_w: u32,
    texture_h: u32,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
}

impl Renderer {
    // A linear wgpu device/surface/pipeline init sequence, same shape (and same justification)
    // as `rustysnes-frontend::gfx::Gfx::new_async`'s and `rustysnes-android::Renderer::new`'s
    // identical allow.
    #[allow(clippy::too_many_lines)]
    fn new(view: &IosViewHandle, width: u32, height: u32) -> Option<Self> {
        // `InstanceDescriptor` has no `Default` impl in this wgpu version -- matches
        // `rustysnes-android::Renderer::try_backend`'s identical `new_without_display_handle()` +
        // field-assignment pattern. Metal is the only backend on iOS (no Vulkan-vs-GLES choice
        // like Android), so no fallback chain is needed here.
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::METAL;
        // Disabled for the same reason as `rustysnes-android`'s identical field (found there by
        // actually crashing an emulator's software Vulkan renderer on debug-build `DEBUG`+
        // `VALIDATION` flags): real Metal hardware doesn't need debug-build validation layers
        // enabled by default to function, and this keeps both mobile presentation crates'
        // instance-creation posture consistent. Unlike the Android bug, this has not been
        // observed to crash anything on iOS (no device to observe it on) -- kept purely for
        // cross-platform consistency with the crate this one was modeled on.
        desc.flags = wgpu::InstanceFlags::empty();
        let instance = wgpu::Instance::new(desc);

        // SAFETY: `view` outlives the surface it creates for as long as the Swift caller upholds
        // the module doc's safety contract (the `UIView` stays alive between
        // `rustysnes_ios_surface_created` and `rustysnes_ios_surface_destroyed`).
        //
        // `from_display_and_window`, not `from_window` -- matches `rustysnes-android`'s identical
        // fix (found there in review): `from_window` unconditionally sets
        // `raw_display_handle: None`, and `wgpu-core`'s `create_surface` hard-errors whenever
        // both the per-surface AND the `InstanceDescriptor::display` handles are `None`. UiKit's
        // display handle is a marker-only value (`UiKitDisplayHandle` carries no data), but it
        // still must be the one actually forwarded.
        let surface = unsafe {
            instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_display_and_window(view, view).ok()?,
                )
                .inspect_err(|e| log::error!("rustysnes-ios: create_surface_unsafe failed: {e}"))
                .ok()?
        };
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .inspect_err(|e| log::error!("rustysnes-ios: request_adapter failed: {e}"))
        .ok()?;
        log::info!("rustysnes-ios: adapter: {:?}", adapter.get_info());

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rustysnes-ios-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .inspect_err(|e| log::error!("rustysnes-ios: request_device failed: {e}"))
        .ok()?;

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
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let framebuffer_format = if format.is_srgb() {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        // Sized to the hi-res worst case up front, matching `rustysnes-android::Renderer`'s own
        // streaming-texture convention -- 512x448 is small (~900KB), cheap to always allocate.
        let (texture_w, texture_h) = (512, 448);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustysnes-ios-framebuffer"),
            size: wgpu::Extent3d {
                width: texture_w,
                height: texture_h,
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
            label: Some("rustysnes-ios-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rustysnes-ios-blit-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rustysnes-ios-bgl"),
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rustysnes-ios-blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustysnes-ios-blit-pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustysnes-ios-blit-pipeline"),
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
        let _ = view; // rebuilt per-frame in `present` after `upload`; the initial view is unused.

        Some(Self {
            device,
            queue,
            surface,
            config,
            texture,
            texture_w,
            texture_h,
            sampler,
            bind_group_layout,
            pipeline,
            uniform_buf,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Upload `rgba` (must be `w*h*4` bytes) and present it, letterboxed to the current surface
    /// aspect ratio. Silently returns on any transient wgpu error (a dropped frame is the correct
    /// behavior mid-resize, not a crash) -- matches `rustysnes-android::Renderer::present`
    /// exactly, including its bounds-check-before-multiply ordering (found as a real overflow bug
    /// there in review, applied here from the start rather than re-discovering it).
    #[allow(clippy::cast_precision_loss)]
    fn present(&self, rgba: &[u8], w: u32, h: u32) {
        if w == 0 || h == 0 || w > self.texture_w || h > self.texture_h {
            return;
        }
        if rgba.len() < (w * h * 4) as usize {
            return;
        }
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

        let target_aspect = 4.0_f32 / 3.0;
        let win_aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        let (pos_x, pos_y) = if win_aspect > target_aspect {
            (target_aspect / win_aspect, 1.0)
        } else {
            (1.0, win_aspect / target_aspect)
        };
        let uv_x = w as f32 / self.texture_w as f32;
        let uv_y = h as f32 / self.texture_h as f32;
        self.queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::cast_slice(&[uv_x, uv_y, pos_x, pos_y]),
        );

        let view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rustysnes-ios-bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buf.as_entire_binding(),
                },
            ],
        });

        // wgpu 29 returns the `CurrentSurfaceTexture` enum (not a `Result`): present on
        // `Success`/`Suboptimal`, reconfigure and drop this frame on `Lost`/`Outdated`, skip
        // otherwise -- matches `rustysnes-android::Renderer::present`'s identical handling.
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rustysnes-ios-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
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
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

// The Swift shell's view lifecycle (`UIViewRepresentable.makeUIView`/`dismantleUIView`) runs on
// the main thread, while the frame-present loop is expected to run on a background thread
// (matching `rustysnes-android`'s `Dispatchers.Default` frame loop) -- this `Mutex` genuinely
// serializes that real cross-thread access, matching the corrected understanding from
// `rustysnes-android`'s own `RENDERER` static (a wrong "single-thread-only" claim was found and
// fixed there in review; documented correctly here from the start).
static RENDERER: Mutex<Option<Renderer>> = Mutex::new(None);

/// Builds a fresh wgpu device/surface from the `UIView*` Swift hands over. Call from
/// `UIViewRepresentable.makeUIView` (or `layoutSubviews`, on first layout).
///
/// No logger is installed by this crate (see `Cargo.toml`'s comment on the dropped `oslog`
/// dependency) — the `log::*!` calls throughout this module are inert until the Swift shell (or
/// a future `v1.16.0`+ follow-up, once real Xcode-based verification is possible) installs one.
///
/// # Safety
/// `ui_view` must be a valid, non-null pointer to a live `UIView` that the Swift caller keeps
/// alive at least until the matching [`rustysnes_ios_surface_destroyed`] call — see the module
/// doc's "Safety contract with the Swift caller" section.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustysnes_ios_surface_created(
    ui_view: *mut c_void,
    width: u32,
    height: u32,
) {
    let Some(ui_view) = NonNull::new(ui_view) else {
        log::error!("rustysnes-ios: rustysnes_ios_surface_created called with a null ui_view");
        return;
    };
    let handle = IosViewHandle { ui_view };
    let renderer = Renderer::new(&handle, width.max(1), height.max(1));
    if renderer.is_none() {
        log::error!("rustysnes-ios: Renderer::new failed");
    }
    *RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = renderer;
}

/// Call on every layout/size change (e.g. rotation, split-view resize).
///
/// # Safety
/// Safe to call from any thread; internally synchronized. Kept `unsafe extern "C"` for a
/// consistent FFI surface with the other three entry points here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustysnes_ios_surface_changed(width: u32, height: u32) {
    if let Some(r) = RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_mut()
    {
        r.resize(width.max(1), height.max(1));
    }
}

/// Drops the wgpu device/surface. Must be called before the underlying `UIView` is deallocated —
/// call from `UIViewRepresentable.dismantleUIView`.
///
/// # Safety
/// See [`rustysnes_ios_surface_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustysnes_ios_surface_destroyed() {
    *RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// Upload + present one framebuffer. A no-op if no surface is currently live (backgrounded, or
/// called before the first [`rustysnes_ios_surface_created`]).
///
/// # Safety
/// `rgba` must be a valid pointer to at least `len` readable bytes for the duration of this call
/// (e.g. from Swift's `Data.withUnsafeBytes`). Unlike `rustysnes-android`'s JNI equivalent, no
/// copy into a Rust-owned buffer is needed on the way in — `write_texture` reads directly from
/// this borrowed slice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rustysnes_ios_present_frame(
    rgba: *const u8,
    len: usize,
    width: u32,
    height: u32,
) {
    if rgba.is_null() {
        log::error!("rustysnes-ios: rustysnes_ios_present_frame called with a null rgba pointer");
        return;
    }
    // SAFETY: `rgba` is valid for `len` bytes for the duration of this call, per this function's
    // own safety contract (upheld by the Swift caller).
    let bytes = unsafe { std::slice::from_raw_parts(rgba, len) };
    if let Some(r) = RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_mut()
    {
        r.present(bytes, width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blit_wgsl_validates() {
        let module = naga::front::wgsl::parse_str(BLIT_WGSL).expect("WGSL parses");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("WGSL validates");
    }
}
