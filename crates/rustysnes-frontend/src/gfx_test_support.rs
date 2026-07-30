//! Offscreen wgpu rendering for shader tests (`v1.25.0`, T-FP-D).
//!
//! # Why this exists
//!
//! Before this, `gfx.rs`'s tests only asked naga to *validate* the WGSL — no device was ever
//! created and no pixel was ever produced. That catches a syntax error and nothing else: a shader
//! that compiles and renders the wrong thing passes. Every visual claim in this crate was therefore
//! unverified, which is exactly the position the rest of the project refuses to be in (`docs/adr/
//! 0013`, the AccuracySNES scene goldens, exists for the same reason on the emulation side).
//!
//! This renders to an **offscreen texture** — no window, no surface, no swapchain — reads the
//! pixels back, and hashes them. That is what makes "this shader still produces the same image" a
//! testable statement.
//!
//! # Why offscreen specifically
//!
//! A window-backed path hangs under Xvfb in this project's sandbox (recorded during earlier
//! wgpu work), and CI runners have no GPU at all. Offscreen sidesteps the first entirely, and
//! [`TestGpu::new`] returns `None` when no adapter exists so the second **self-skips** rather than
//! failing — the same posture the gitignored-ROM oracles already take. A skipped test says so; it
//! does not quietly pass.

#![cfg(test)]

/// A headless wgpu device for rendering a pass into a buffer.
pub struct TestGpu {
    /// The device.
    pub device: wgpu::Device,
    /// The queue.
    pub queue: wgpu::Queue,
    /// The adapter's reported name, for a skip message that says *which* adapter ran.
    pub adapter_name: String,
}

/// A rendered image read back from the GPU.
pub struct Readback {
    /// Tightly packed RGBA8 rows (the 256-byte row padding wgpu requires is already removed).
    pub rgba: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Readback {
    /// A stable hash of the image, for a golden comparison.
    ///
    /// FNV-1a rather than a cryptographic hash: this identifies an image, it does not authenticate
    /// one, and a golden that changes when the *renderer* changes is the entire point.
    #[must_use]
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in &self.rgba {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    /// The RGBA8 pixel at `(x, y)`, or `None` when out of bounds.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        self.rgba.get(idx..idx + 4)?.try_into().ok()
    }
}

impl TestGpu {
    /// Request a headless adapter + device, or `None` when none exists.
    ///
    /// No surface is requested, which is what makes this work where the windowed path does not.
    #[must_use]
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        let adapter_name = adapter.get_info().name;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rustysnes-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .ok()?;
        Some(Self {
            device,
            queue,
            adapter_name,
        })
    }

    /// Create an offscreen colour target of `w` x `h`.
    #[must_use]
    pub fn target(&self, w: u32, h: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rustysnes-test-target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Non-sRGB on purpose: the test compares the bytes the shader wrote, and an sRGB view
            // would fold an encode into the comparison that has nothing to do with the shader.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// Read a rendered texture back to tightly-packed RGBA8.
    ///
    /// wgpu requires each copied row to start on a 256-byte boundary, so the copy is padded and the
    /// padding stripped here — forgetting that is the classic way a readback comes out sheared.
    #[must_use]
    pub fn read_back(&self, texture: &wgpu::Texture, w: u32, h: u32) -> Readback {
        const ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded = w * 4;
        let padded = unpadded.div_ceil(ALIGN) * ALIGN;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rustysnes-test-readback"),
            size: u64::from(padded) * u64::from(h),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rustysnes-test-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        // Blocking is correct here and only here: a test wants the pixels now, unlike the live
        // present path, where blocking on the GPU is the thing `gpu_timer` goes out of its way to
        // avoid.
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let _ = rx.recv();

        let view = buffer.slice(..).get_mapped_range();
        let mut rgba = Vec::with_capacity((unpadded * h) as usize);
        for row in 0..h {
            let start = (row * padded) as usize;
            let end = start + unpadded as usize;
            rgba.extend_from_slice(&view[start..end]);
        }
        drop(view);
        buffer.unmap();
        Readback {
            rgba,
            width: w,
            height: h,
        }
    }
}

/// Skip a test with a printed reason when no GPU is available.
///
/// A macro rather than a helper returning `Option` so the skip message names the test, and so the
/// early return is visible at the call site — a silently-skipped GPU test is indistinguishable from
/// a passing one, which is the failure mode this whole module exists to avoid.
#[macro_export]
macro_rules! gpu_test {
    ($gpu:ident) => {
        let Some($gpu) = $crate::gfx_test_support::TestGpu::new() else {
            eprintln!(
                "SKIP {}: no wgpu adapter available (expected in CI)",
                module_path!()
            );
            return;
        };
    };
}

#[cfg(test)]
mod tests {

    /// The spike: can this environment create a headless device and read pixels back at all?
    /// Everything else in T-FP-D depends on the answer.
    #[test]
    fn offscreen_render_and_readback_works_or_skips() {
        crate::gpu_test!(gpu);
        eprintln!("offscreen harness running on: {}", gpu.adapter_name);

        let (w, h) = (64u32, 32u32);
        let texture = gpu.target(w, h);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        // A clear is the simplest possible pass, and its expected output is exactly known — which
        // is what makes it a usable check that the readback path itself is correct.
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        gpu.queue.submit(Some(encoder.finish()));

        let image = gpu.read_back(&texture, w, h);
        assert_eq!(image.width, w);
        assert_eq!(image.height, h);
        assert_eq!(
            image.rgba.len(),
            (w * h * 4) as usize,
            "row padding must be stripped, or every readback is sheared"
        );
        // Every pixel is the clear colour, including the last — which is where a row-padding
        // mistake shows up first.
        assert_eq!(image.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(image.pixel(w - 1, h - 1), Some([255, 0, 0, 255]));
        assert_eq!(image.pixel(w, 0), None, "out of bounds reads as None");

        // The hash is stable and depends on the content.
        let a = image.hash();
        assert_eq!(a, image.hash());
        let other = gpu.target(w, h);
        let other_view = other.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc2 = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let _pass = enc2.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear2"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &other_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        gpu.queue.submit(Some(enc2.finish()));
        assert_ne!(
            a,
            gpu.read_back(&other, w, h).hash(),
            "a different image must hash differently, or goldens prove nothing"
        );
    }
}
