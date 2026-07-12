//! Android JNI presentation host (`v1.15.0 "Sideload"`, Mobile Phase 2) — wgpu-on-`Surface`
//! rendering only. Owns no emulation state: the Kotlin shell drives `rustysnes-mobile`'s
//! `MobileCore` (ROM load, `run_frame`, input, save state) through its own `UniFFI` bindings, then
//! hands this crate exactly `(RGBA8 framebuffer bytes, width, height)` once per frame via
//! [`Java_com_doublegate_rustysnes_NativeRenderer_nativePresentFrame`] — the same separation of
//! concerns `rustysnes-core`/`rustysnes-frontend` already have on desktop, just across a JNI
//! boundary instead of an in-process crate boundary.
//!
//! Reuses `rustysnes-gfx-shaders::BLIT_WGSL` verbatim for the actual blit pass — the same shader
//! `rustysnes-frontend::gfx` uses for its `PostFilter::None` path, so "what the framebuffer looks
//! like on screen" stays identical across desktop and Android for the unfiltered case (`v1.15.0`
//! ships blit-only; the `Crt`/`Hqx`/`Xbrz` post-filters are a documented follow-up, not wired
//! here yet).
//!
//! # Surface lifecycle
//!
//! Android's `SurfaceView` can destroy and recreate its `Surface` at any point (app backgrounded,
//! screen rotated, window resized) — [`nativeSurfaceCreated`]/[`nativeSurfaceChanged`]/
//! [`nativeSurfaceDestroyed`] mirror `SurfaceHolder.Callback`'s own three lifecycle methods
//! exactly, called from Kotlin in that order. The renderer is `None` outside a created/valid
//! surface, so a frame presented while backgrounded is silently dropped rather than crashing.

#![allow(unsafe_code)]

use std::sync::Mutex;

use jni::JNIEnv;
use jni::objects::{JClass, JObject};
use jni::sys::{jbyteArray, jint};
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
    HasWindowHandle, RawDisplayHandle, RawWindowHandle, WindowHandle,
};

use rustysnes_gfx_shaders::BLIT_WGSL;

/// A thin [`HasWindowHandle`]/[`HasDisplayHandle`] wrapper around the raw `ANativeWindow*`
/// [`ndk::native_window::NativeWindow::from_surface`] hands back — wgpu's `Instance::create_surface`
/// needs a type implementing both traits, and neither the raw pointer nor `NativeWindow` itself
/// implements them directly.
struct AndroidWindowHandle {
    native_window: ndk::native_window::NativeWindow,
}

impl HasWindowHandle for AndroidWindowHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: `native_window` is a valid, live `ANativeWindow*` for as long as this struct
        // exists (owned exclusively by `Renderer`, rebuilt on every `nativeSurfaceCreated`,
        // dropped on `nativeSurfaceDestroyed` before Android considers the old window invalid).
        let ptr = self.native_window.ptr();
        let handle = AndroidNdkWindowHandle::new(ptr.cast());
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AndroidNdk(handle)) })
    }
}

impl HasDisplayHandle for AndroidWindowHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Android(AndroidDisplayHandle::new()))
        })
    }
}

/// The live wgpu device + surface + blit pipeline, present only between
/// [`nativeSurfaceCreated`] and [`nativeSurfaceDestroyed`].
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
    // as `rustysnes-frontend::gfx::Gfx::new_async`'s identical allow.
    #[allow(clippy::too_many_lines)]
    fn new(window: &AndroidWindowHandle, width: u32, height: u32) -> Option<Self> {
        // Explicit Vulkan-first, GLES-fallback backend selection -- `wgpu::Instance::default()`'s
        // own auto-detection was found (by actually running this on a real Android emulator) to
        // pick the GLES/EGL backend even when a real Vulkan driver is present (`ro.hardware.
        // vulkan=ranchu` on this AVD), and GLES then failed at native-surface creation ("EGL says
        // it can present to the window but not natively"). Almost every Android device shipping
        // since Android 10 has a real Vulkan driver (Google's own CDD requirement), so trying it
        // first is the right default; GLES stays as the fallback for the older/budget devices
        // that genuinely lack one, matching `rustysnes-frontend::gfx`'s own explicit (not
        // ambiguous-auto-detected) wasm WebGPU-vs-GL backend choice.
        // `_instance` isn't needed past adapter/surface/device creation, matching
        // `rustysnes-frontend::gfx::Gfx::new_async`'s identical `_instance` convention.
        let (_instance, surface, adapter) = Self::try_backend(window, wgpu::Backends::VULKAN)
            .or_else(|| Self::try_backend(window, wgpu::Backends::GL))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rustysnes-android-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .inspect_err(|e| log::error!("rustysnes-android: request_device failed: {e}"))
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
        // Sized to the hi-res worst case up front, matching `rustysnes-frontend::gfx::Gfx`'s own
        // streaming-texture convention -- 512x448 is small (~900KB), cheap to always allocate.
        let (texture_w, texture_h) = (512, 448);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustysnes-android-framebuffer"),
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
            label: Some("rustysnes-android-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rustysnes-android-blit-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rustysnes-android-bgl"),
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
            label: Some("rustysnes-android-blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rustysnes-android-blit-pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rustysnes-android-blit-pipeline"),
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

    /// Try building an `(Instance, Surface, Adapter)` triple restricted to `backends` — `None`
    /// if this backend has no working adapter for `window`'s surface (surface creation,
    /// adapter request, or both can fail; either way this backend isn't usable and the caller
    /// should try the next one). See [`Self::new`]'s own doc for why Vulkan is tried first.
    fn try_backend(
        window: &AndroidWindowHandle,
        backends: wgpu::Backends,
    ) -> Option<(wgpu::Instance, wgpu::Surface<'static>, wgpu::Adapter)> {
        // `InstanceDescriptor` has no `Default` impl in this wgpu version -- matches
        // `rustysnes-frontend::gfx::Gfx::create_instance_surface_adapter_wasm`'s identical
        // `new_without_display_handle()` + field-assignment pattern.
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backends;
        // `InstanceFlags::default()` enables `DEBUG`+`VALIDATION` on debug builds (`cargo ndk
        // build`'s default profile) -- found (by actually running this on the real AVD) to crash
        // the emulator's SwiftShader software Vulkan renderer outright (`SPIR-V ERROR: Invalid
        // source language operand`, then the whole emulator process dying). Real devices ship a
        // hardware Vulkan driver, not a software one, so this is specifically an emulator/
        // SwiftShader fragility, not a device-representative bug -- disabling both debug info and
        // the validation layer sidesteps it without weakening anything users would see.
        desc.flags = wgpu::InstanceFlags::empty();
        let instance = wgpu::Instance::new(desc);
        // SAFETY: `window` outlives the surface it creates -- `create_surface_unsafe` only
        // borrows it for the duration of this call, and the returned surface itself does not
        // retain any reference back to `window` past that point (matching `SurfaceTargetUnsafe`'s
        // own documented contract: the target must be valid for as long as the surface it built
        // is used, which this function upholds by fully consuming `window`'s borrow here and
        // never touching it again).
        //
        // `from_display_and_window`, not `from_window` -- `from_window` unconditionally sets
        // `raw_display_handle: None` (it only requires `HasWindowHandle`), and wgpu-core's
        // `create_surface` hard-errors ("No `DisplayHandle` is available...") whenever both the
        // per-surface AND the `InstanceDescriptor::display` handles are `None` (confirmed by
        // reading `wgpu-core::instance::Instance::create_surface`). Android has no real display
        // object, but `AndroidWindowHandle`'s `HasDisplayHandle` impl already supplies the
        // marker-only `RawDisplayHandle::Android` value the Vulkan/GLES backends expect --
        // `from_display_and_window` is the variant that actually forwards it.
        let surface = unsafe {
            instance.create_surface_unsafe(
                wgpu::SurfaceTargetUnsafe::from_display_and_window(window, window).ok()?,
            )
        }
        .inspect_err(|e| {
            log::error!("rustysnes-android: {backends:?} create_surface_unsafe failed: {e}");
        })
        .ok()?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .inspect_err(|e| log::error!("rustysnes-android: {backends:?} request_adapter failed: {e}"))
        .ok()?;
        log::info!(
            "rustysnes-android: {backends:?} adapter: {:?}",
            adapter.get_info()
        );
        Some((instance, surface, adapter))
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
    /// behavior for a `SurfaceView` mid-resize, not a crash).
    // The letterbox math casts small screen/texture dimensions (all well under 2^23) to f32 for
    // the UV + aspect ratios, matching `rustysnes-frontend::gfx::Gfx::present`'s identical
    // precision-loss allowance for the same reason.
    #[allow(clippy::cast_precision_loss)]
    fn present(&self, rgba: &[u8], w: u32, h: u32) {
        if w == 0 || h == 0 || rgba.len() < (w * h * 4) as usize {
            return;
        }
        if w > self.texture_w || h > self.texture_h {
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
            label: Some("rustysnes-android-bg"),
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
        // otherwise -- matches `rustysnes-frontend::gfx::Gfx::acquire`'s identical handling.
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
                label: Some("rustysnes-android-pass"),
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

// SAFETY: a `Renderer` is only ever touched from Android's single `SurfaceView` render thread
// (Kotlin never calls two `native*` functions concurrently for the same view), so the `Mutex`
// here exists only to satisfy `static` initialization, not for real cross-thread contention.
static RENDERER: Mutex<Option<Renderer>> = Mutex::new(None);

/// `SurfaceHolder.Callback.surfaceCreated` — builds a fresh wgpu device/surface from the
/// `android.view.Surface` Kotlin hands over.
///
/// # Safety
/// Called only from the JVM via JNI with a valid `Surface` object, per the standard JNI FFI
/// contract for `extern "system"` entry points.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_doublegate_rustysnes_NativeRenderer_nativeSurfaceCreated(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
    width: jint,
    height: jint,
) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    // SAFETY: `surface` is a live `android.view.Surface` JNI object for the duration of this
    // call, upheld by the JVM caller (`SurfaceHolder.Callback.surfaceCreated`'s own contract).
    let Ok(native_window) = (unsafe {
        ndk::native_window::NativeWindow::from_surface(env.get_native_interface(), surface.as_raw())
    })
    .ok_or(()) else {
        log::error!("rustysnes-android: NativeWindow::from_surface returned null");
        return;
    };
    let handle = AndroidWindowHandle { native_window };
    let renderer = Renderer::new(
        &handle,
        width.max(1).cast_unsigned(),
        height.max(1).cast_unsigned(),
    );
    if renderer.is_none() {
        log::error!("rustysnes-android: Renderer::new failed");
    }
    *RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = renderer;
}

/// `SurfaceHolder.Callback.surfaceChanged`.
///
/// # Safety
/// See [`Java_com_doublegate_rustysnes_NativeRenderer_nativeSurfaceCreated`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_doublegate_rustysnes_NativeRenderer_nativeSurfaceChanged(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    if let Some(r) = RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_mut()
    {
        r.resize(width.max(1).cast_unsigned(), height.max(1).cast_unsigned());
    }
}

/// `SurfaceHolder.Callback.surfaceDestroyed` — drops the wgpu device/surface. Must be called
/// before Android considers the underlying `ANativeWindow` invalid.
///
/// # Safety
/// See [`Java_com_doublegate_rustysnes_NativeRenderer_nativeSurfaceCreated`].
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_doublegate_rustysnes_NativeRenderer_nativeSurfaceDestroyed(
    _env: JNIEnv,
    _class: JClass,
) {
    *RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// Upload + present one framebuffer. A no-op if no surface is currently live (backgrounded, or
/// called before the first `nativeSurfaceCreated`).
///
/// # Safety
/// Called only from the JVM via JNI with a valid, non-null `rgba` byte array, per the standard
/// JNI FFI contract for `extern "system"` entry points.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_doublegate_rustysnes_NativeRenderer_nativePresentFrame(
    env: JNIEnv,
    _class: JClass,
    rgba: jbyteArray,
    width: jint,
    height: jint,
) {
    // SAFETY: `rgba` is a live, valid `byte[]` JNI reference for the duration of this call.
    let rgba_arr = unsafe { jni::objects::JByteArray::from_raw(rgba) };
    let bytes: Vec<u8> = match env.convert_byte_array(&rgba_arr) {
        Ok(v) => v,
        Err(e) => {
            log::error!("rustysnes-android: convert_byte_array failed: {e}");
            return;
        }
    };
    if let Some(r) = RENDERER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_mut()
    {
        r.present(
            &bytes,
            width.max(0).cast_unsigned(),
            height.max(0).cast_unsigned(),
        );
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
