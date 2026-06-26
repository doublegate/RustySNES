//! The winit [`ApplicationHandler`] that drives the always-on egui shell, the framebuffer
//! present path, the emulator, and audio.
//!
//! Native only (the wasm path lives in `wasm.rs`). The structure is the RustyNES `app.rs`,
//! distilled to the load-bearing flow:
//!
//! 1. `resumed()` (winit 0.30 idiom) creates the window + [`Gfx`] + the egui integration and,
//!    when `emu-thread` is on, spawns the dedicated `EmuThread`.
//! 2. `window_event()` feeds input to egui, late-latches the SNES pad into the lock-free
//!    `SharedInput`, and on `RedrawRequested` runs one render:
//!    - copy the framebuffer out under a BRIEF emu lock, then DROP the lock;
//!    - blit it via wgpu;
//!    - run the egui shell pass (which NEVER touches the emu lock) and collect [`MenuAction`]s;
//!    - present;
//!    - dispatch the collected actions AFTER the egui pass.
//!
//! The frontend owns pacing + run-ahead; the core never sees wall-clock time (determinism).

use std::path::{Path, PathBuf};
#[cfg(feature = "emu-thread")]
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, PoisonError};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::audio::{AudioOutput, AudioRing, Resampler, drc_ratio};
use crate::config::Config;
use crate::emu::EmuCore;
use crate::gfx::Gfx;
use crate::input::Buttons;
use crate::ui_shell::{MenuAction, ShellInfo, ShellState};

#[cfg(feature = "emu-thread")]
use crate::emu_thread::{EmuThread, SharedInput};

/// The live application state, constructed in `resumed()` (the winit 0.30 idiom — a window
/// cannot be created before the event loop resumes).
struct Active {
    window: Arc<Window>,
    gfx: Gfx,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    /// The emulator, shared with the emulation thread + the present path.
    core: Arc<Mutex<EmuCore>>,
    /// The lock-free input latch the window handler writes and the emu thread reads.
    #[cfg(feature = "emu-thread")]
    input: Arc<SharedInput>,
    /// The running emulation thread (joined on drop).
    #[cfg(feature = "emu-thread")]
    _emu_thread: EmuThread,
    /// The current frame's accumulated P1 button state (when not threaded, stepped inline).
    pad1: Buttons,
    /// The cpal output stream + its lock-free ring (the producer pushes resampled audio here).
    /// `None` if no audio device was available (the emulator still runs, silently).
    audio: Option<AudioOutput>,
    /// The producer-side 32 kHz → device-rate resampler.
    resampler: Resampler,
    /// The egui shell's persistent UI state.
    shell: ShellState,
}

/// The app: holds the config + the deferred ROM path until `resumed()` builds `Active`.
pub struct App {
    config: Config,
    pending_rom: Option<PathBuf>,
    active: Option<Active>,
}

impl App {
    /// Create the app with a loaded config and an optional ROM to open once the window exists.
    #[must_use]
    pub const fn new(config: Config, rom: Option<PathBuf>) -> Self {
        Self {
            config,
            pending_rom: rom,
            active: None,
        }
    }

    /// Run the native event loop to completion.
    ///
    /// # Errors
    /// Returns any winit [`winit::error::EventLoopError`] from creating or running the loop.
    pub fn run(mut self) -> Result<(), winit::error::EventLoopError> {
        let event_loop = EventLoop::new()?;
        event_loop.run_app(&mut self)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.active.is_some() {
            return; // already initialized (e.g. resumed after suspend)
        }
        let attrs = Window::default_attributes()
            .with_title("RustySNES")
            .with_inner_size(winit::dpi::LogicalSize::new(512.0, 448.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("rustysnes: failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
        let gfx = match Gfx::new(Arc::clone(&window), &self.config.video.present_mode) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("rustysnes: wgpu init failed: {e}");
                event_loop.exit();
                return;
            }
        };

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            None,
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &gfx.device,
            gfx.config.format,
            egui_wgpu::RendererOptions::default(),
        );

        // Power on the emulator at the configured region.
        let mut emu = EmuCore::new(0, self.config.region);
        let initial_status = self
            .pending_rom
            .take()
            .map_or_else(String::new, |path| load_rom_file(&mut emu, &path));

        // Open the audio device (best-effort: a missing device leaves the emulator silent, not
        // dead). The producer-side resampler converts the S-DSP 32 kHz stream to the device rate.
        let audio = AudioOutput::new(Arc::new(AudioRing::new(13))).ok();
        let dst_rate = audio.as_ref().map_or(48_000, |a| a.sample_rate);
        let resampler = Resampler::new(dst_rate, self.config.audio.volume);
        // `Arc<Mutex<EmuCore>>` is the right shape for the (default-off) dedicated emulation
        // thread + the present path. It is not yet `Send + Sync` only because
        // `rustysnes-cart`'s `Board` trait is not `Send` (the RustyNES `Mapper: Send` rule the
        // cart phase will land); once it is, the `emu-thread` default returns and this allow
        // goes away. TODO(impl-phase).
        #[allow(clippy::arc_with_non_send_sync)]
        let core = Arc::new(Mutex::new(emu));

        #[cfg(feature = "emu-thread")]
        let input = Arc::new(SharedInput::default());
        #[cfg(feature = "emu-thread")]
        let emu_thread = EmuThread::spawn(
            Arc::clone(&core),
            Arc::clone(&input),
            self.config.region.frame_rate(),
        );

        self.active = Some(Active {
            window,
            gfx,
            egui_ctx,
            egui_state,
            egui_renderer,
            core,
            #[cfg(feature = "emu-thread")]
            input,
            #[cfg(feature = "emu-thread")]
            _emu_thread: emu_thread,
            pad1: Buttons::default(),
            audio,
            resampler,
            shell: ShellState {
                status: initial_status,
                ..ShellState::default()
            },
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        // Feed egui first; if it consumes the event we still latch keys for the emulator below.
        let _ = active.egui_state.on_window_event(&active.window, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                active.gfx.resize(size.width, size.height);
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                Self::latch_key(active, &self.config, &key_event);
            }
            WindowEvent::RedrawRequested => {
                let actions = Self::render(active, &mut self.config);
                Self::dispatch_actions(active, &mut self.config, event_loop, actions);
                active.window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(active) = self.active.as_ref() {
            active.window.request_redraw();
        }
    }
}

impl App {
    /// Late-latch a key event into the P1 pad + the lock-free input latch.
    fn latch_key(active: &mut Active, config: &Config, key: &winit::event::KeyEvent) {
        let pressed = key.state.is_pressed();
        // winit logical key is mapped via the physical KeyCode debug name (the binding scheme
        // in `input::KeyBindings`).
        if let winit::keyboard::PhysicalKey::Code(code) = key.physical_key {
            let name = format!("{code:?}");
            if let Some(button) = config.p1.button_for(&name) {
                active.pad1.set(button, pressed);
                #[cfg(feature = "emu-thread")]
                active
                    .input
                    .p1
                    .store(u32::from(active.pad1.sanitize_dpad().0), Ordering::Release);
            }
        }
    }

    /// One render: copy the framebuffer under a brief lock, blit, run the egui shell, present.
    /// Returns the menu actions to dispatch AFTER this pass (never dispatched mid-egui).
    // One straight-line present pass (lock-copy → audio push → blit → egui → submit); the length
    // is inherent to the wgpu/egui frame sequence and reads more clearly as a unit.
    #[allow(clippy::too_many_lines)]
    fn render(active: &mut Active, config: &mut Config) -> Vec<MenuAction> {
        let paused = active.shell.paused;
        // --- (1) Copy framebuffer + audio + read-only info under a BRIEF lock, then drop it. ---
        let (fb, fb_dims, info, audio_samples) = {
            let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
            // When NOT threaded, step exactly one frame here (synchronous drive), unless paused.
            #[cfg(not(feature = "emu-thread"))]
            let audio_samples = if paused {
                Vec::new()
            } else {
                emu.set_pad(0, active.pad1);
                emu.run_frame();
                emu.audio().to_vec()
            };
            // Threaded build: the emu thread owns frame production; audio-from-thread is a TODO.
            #[cfg(feature = "emu-thread")]
            let audio_samples: Vec<(i16, i16)> = Vec::new();
            let dims = emu.fb_dims();
            let fb = emu.framebuffer().to_vec();
            let info = ShellInfo {
                cart_name: emu.cart_name().map(str::to_string),
                region: emu.region(),
                fps: 0.0, // TODO(impl-phase): wire the pacer's smoothed FPS estimate.
                rom_loaded: emu.rom_loaded(),
            };
            drop(emu); // release the brief lock BEFORE the wgpu upload + egui pass
            (fb, dims, info, audio_samples)
        };

        // --- Push the frame's audio through the resampler into the ring (outside the lock). ---
        if let Some(audio) = active
            .audio
            .as_ref()
            .filter(|_| config.audio.enabled && !audio_samples.is_empty())
        {
            active.resampler.set_volume(config.audio.volume);
            let cap = audio.ring.capacity();
            let ratio = drc_ratio(audio.ring.occupancy(), cap / 2, cap);
            active.resampler.process(&audio_samples, ratio, &audio.ring);
        }

        active.gfx.upload(&fb, fb_dims.0, fb_dims.1);

        // --- (2) Acquire the surface. ---
        let Some(frame) = active.gfx.acquire() else {
            return Vec::new();
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            active
                .gfx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("rustysnes-frame"),
                });

        // --- (3) Blit the framebuffer (clears then draws the fullscreen triangle). ---
        active.gfx.blit(&mut encoder, &view);

        // --- (4) Run the always-on egui shell pass. The shell NEVER touches the emu lock. ---
        let raw_input = active.egui_state.take_egui_input(&active.window);
        let mut actions = Vec::new();
        let full_output = active.egui_ctx.run_ui(raw_input, |ui| {
            actions = active.shell.render(ui, &info, config);
        });
        active
            .egui_state
            .handle_platform_output(&active.window, full_output.platform_output);
        let tris = active
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [active.gfx.config.width, active.gfx.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };
        for (id, delta) in &full_output.textures_delta.set {
            active
                .egui_renderer
                .update_texture(&active.gfx.device, &active.gfx.queue, *id, delta);
        }
        active.egui_renderer.update_buffers(
            &active.gfx.device,
            &active.gfx.queue,
            &mut encoder,
            &tris,
            &screen,
        );
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("rustysnes-egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // keep the framebuffer blit underneath
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            active.egui_renderer.render(&mut pass, &tris, &screen);
        }
        for id in &full_output.textures_delta.free {
            active.egui_renderer.free_texture(id);
        }

        // --- (5) Submit + present. ---
        active.gfx.queue.submit(Some(encoder.finish()));
        frame.present();
        actions
    }

    /// Dispatch the menu actions collected during the egui pass (AFTER the pass, so the emu lock
    /// is never taken inside the egui closure).
    fn dispatch_actions(
        active: &mut Active,
        config: &mut Config,
        event_loop: &ActiveEventLoop,
        actions: Vec<MenuAction>,
    ) {
        for action in actions {
            match action {
                MenuAction::OpenRom => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SNES ROM", &["sfc", "smc", "fig", "swc"])
                        .pick_file()
                    {
                        let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                        active.shell.status = load_rom_file(&mut emu, &path);
                    }
                }
                MenuAction::CloseRom => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .close_rom();
                    active.shell.status = "ROM closed".into();
                }
                MenuAction::SetRegion(region) => {
                    config.region = region;
                    let _ = config.save();
                    active.shell.status = format!("Region: {region:?} (restart to apply)");
                }
                MenuAction::TogglePause => {
                    active.shell.paused = !active.shell.paused;
                    active.shell.status = if active.shell.paused {
                        "Paused".into()
                    } else {
                        "Running".into()
                    };
                }
                MenuAction::ToggleDebugger => {
                    active.shell.debugger_open = !active.shell.debugger_open;
                }
                MenuAction::OpenSettings => active.shell.settings_open = true,
                MenuAction::Quit => event_loop.exit(),
                MenuAction::Reset => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .reset();
                    active.shell.status = "Reset".into();
                }
                MenuAction::PowerCycle => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .power_cycle();
                    active.shell.status = "Power cycled".into();
                }
                // Save-states need a core-wide deterministic snapshot (Clone/serialize across the
                // Board trait + APU/Bus/System); that is the next frontend sprint — see
                // docs/frontend.md "Save-states".
                MenuAction::SaveState | MenuAction::LoadState => {
                    active.shell.status =
                        "Save/Load state: not yet implemented (core snapshot pending)".into();
                }
            }
        }
        // Persist any Settings-window edits to config (best-effort).
        let _ = config.save();
    }
}

/// Read a ROM file into `emu`, then best-effort install any required coprocessor firmware and a
/// `.srm` battery save sitting next to the ROM. Returns a human-readable status line.
fn load_rom_file(emu: &mut EmuCore, path: &Path) -> String {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return format!("read failed: {e}"),
    };
    if let Err(e) = emu.load_rom(&bytes) {
        return format!("load failed: {e}");
    }
    let mut status = format!("Loaded {}", path.display());

    // Coprocessor firmware (DSP-1.. / CX4): try the known dumps next to the ROM + the dev dir.
    if emu.needs_firmware() && !try_install_firmware(emu, path) {
        status = format!(
            "{status} — coprocessor firmware required ({}); place it beside the ROM or in a \
             firmware/ folder",
            emu.firmware_candidates().join(", ")
        );
    }

    // Battery SRAM sidecar (`<rom>.srm`).
    let srm = path.with_extension("srm");
    if let Ok(sram) = std::fs::read(&srm) {
        emu.load_sram(&sram);
    }
    status
}

/// Search the standard locations for any of the cart's candidate firmware dumps and install the
/// first one the board accepts. Returns whether a dump was installed.
fn try_install_firmware(emu: &mut EmuCore, rom_path: &Path) -> bool {
    let rom_dir = rom_path.parent().map(Path::to_path_buf);
    for name in emu.firmware_candidates() {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(dir) = rom_dir.as_ref() {
            candidates.push(dir.join(name));
            candidates.push(dir.join("firmware").join(name));
        }
        // Dev convenience: the gitignored staging dir, relative to the workspace cwd.
        candidates.push(Path::new("tests/roms/external/firmware").join(name));
        for cand in candidates {
            if std::fs::read(&cand).is_ok_and(|bytes| emu.install_firmware(&bytes)) {
                return true;
            }
        }
    }
    false
}
