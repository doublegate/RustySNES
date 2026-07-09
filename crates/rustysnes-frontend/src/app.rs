//! The winit [`ApplicationHandler`] that drives the always-on egui shell, the framebuffer
//! present path, the emulator, and audio.
//!
//! The structure is the RustyNES `app.rs`, distilled to the load-bearing flow, and — as of
//! `v0.8.0`'s `wasm-winit` unification (T-81-006) — this ONE `ApplicationHandler<AppEvent>` impl
//! serves BOTH native and `wasm32`, matching RustyNES's own "the impl serves both" design
//! (confirmed by reading its source directly, not inferred):
//!
//! 1. `resumed()` (winit 0.30 idiom) creates the window + [`Gfx`]. Native builds `Gfx`
//!    synchronously (`pollster::block_on` inside `Gfx::new`) and continues straight into
//!    `App::on_gfx_ready`; `wasm32`'s wgpu init is async, so it `spawn_local`s the future and
//!    delivers the result back into the event loop as [`AppEvent::GfxReady`] via an
//!    `EventLoopProxy` (native never sends a user event — the typed loop is otherwise identical).
//! 2. `window_event()` feeds input to egui, late-latches the SNES pad into the lock-free
//!    `SharedInput`, and on `RedrawRequested` runs one render:
//!    - copy the framebuffer out under a BRIEF emu lock, then DROP the lock;
//!    - blit it via wgpu;
//!    - run the egui shell pass (which NEVER touches the emu lock) and collect [`MenuAction`]s;
//!    - present;
//!    - dispatch the collected actions AFTER the egui pass.
//! 3. On `wasm32`, ROM bytes arrive from the browser's `<input type="file">` (wired in
//!    `wasm_winit.rs`) as [`AppEvent::RomLoaded`], not `rfd`'s native file dialog.
//!
//! The frontend owns pacing + run-ahead; the core never sees wall-clock time (determinism).

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(feature = "emu-thread")]
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, PoisonError};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(not(target_arch = "wasm32"))]
use crate::audio::{AudioOutput, AudioRing, Resampler, drc_ratio};
use crate::config::Config;
use crate::emu::EmuCore;
use crate::gfx::Gfx;
use crate::input::Buttons;
use crate::pacing::Pacer;
use crate::ui_shell::{MenuAction, ShellInfo, ShellState};

#[cfg(feature = "emu-thread")]
use crate::emu_thread::{EmuThread, SharedInput};

/// The typed winit user-event, used by both native and `wasm32` (native simply never sends one).
///
/// On `wasm32` the wgpu init is async and the ROM arrives via the browser file picker, so
/// neither can be produced synchronously inside `ApplicationHandler::resumed`. Instead they're
/// delivered back into the event loop as user events via an `EventLoopProxy` (RustyNES's own
/// `AppEvent`, ported).
pub enum AppEvent {
    /// The async `Gfx::new_async` future resolved (`wasm32`).
    GfxReady(Box<Gfx>),
    /// The browser file picker delivered ROM bytes (`wasm32`).
    RomLoaded(Vec<u8>),
}

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
    /// `None` if no audio device was available (the emulator still runs, silently). Native only —
    /// `wasm32` pushes audio through [`crate::wasm_audio`] instead (its own `AudioWorkletNode`/
    /// `ScriptProcessorNode` graph, with its own internal resampler).
    #[cfg(not(target_arch = "wasm32"))]
    audio: Option<AudioOutput>,
    /// The producer-side 32 kHz → device-rate resampler. Native only — see `audio` above.
    #[cfg(not(target_arch = "wasm32"))]
    resampler: Resampler,
    /// The egui shell's persistent UI state.
    shell: ShellState,
    /// Wall-clock fixed-timestep pacer + FPS meter (drives the synchronous emulation cadence
    /// independent of the display refresh).
    pacer: Pacer,
    /// The present-mode string currently applied to the surface; compared against the live config
    /// each present so a Settings → Video toggle reconfigures the wgpu surface.
    applied_present_mode: String,
    /// The rewind ring buffer (`config.rewind`-driven; a zero-capacity buffer is a permanent
    /// no-op, so this is always constructed, never `Option`-wrapped).
    rewind: crate::rewind::RewindBuffer,
    /// A single quick-save-state slot (`MenuAction::SaveState`/`LoadState`).
    quick_save: Option<Vec<u8>>,
}

/// The app: holds the config + the deferred ROM path until `resumed()` builds `Active`.
pub struct App {
    config: Config,
    /// A ROM path passed on the native CLI, opened once the window exists. Native only —
    /// `wasm32` has no CLI; its ROM arrives via the browser file picker instead.
    #[cfg(not(target_arch = "wasm32"))]
    pending_rom: Option<PathBuf>,
    active: Option<Active>,
    /// The proxy used to deliver [`AppEvent`]s back into this event loop. Only ever `Some` on
    /// `wasm32` (native builds `Gfx` synchronously and never needs one).
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<AppEvent>>,
    /// The window `resumed()` created, held here between kicking off the async `Gfx::new_async`
    /// future and that future's `AppEvent::GfxReady` delivering the result — `Gfx::new_async`
    /// takes the window by value, so it isn't otherwise reachable from `on_gfx_ready`.
    #[cfg(target_arch = "wasm32")]
    pending_window: Option<Arc<Window>>,
    /// ROM bytes that arrived (via the browser file picker) before `Active` existed yet — an
    /// unlikely but real race (the file `<input>` is wired at boot, before `Gfx::new_async`
    /// resolves). Consumed as soon as `on_gfx_ready` builds `Active`.
    #[cfg(target_arch = "wasm32")]
    pending_rom_bytes: Option<Vec<u8>>,
}

impl App {
    /// Create the app with a loaded config and an optional ROM to open once the window exists.
    /// Native only — `wasm32` uses `Self::new_empty` instead.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub const fn new(config: Config, rom: Option<PathBuf>) -> Self {
        Self {
            config,
            pending_rom: rom,
            active: None,
        }
    }

    /// Create the app for `wasm32`: no ROM yet (it arrives via the browser file picker as an
    /// [`AppEvent::RomLoaded`]); `proxy` is what `wasm_winit.rs` uses to deliver it.
    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub const fn new_empty(
        config: Config,
        proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    ) -> Self {
        Self {
            config,
            active: None,
            proxy: Some(proxy),
            pending_window: None,
            pending_rom_bytes: None,
        }
    }

    /// Run the native event loop to completion.
    ///
    /// # Errors
    /// Returns any winit [`winit::error::EventLoopError`] from creating or running the loop.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(mut self) -> Result<(), winit::error::EventLoopError> {
        let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
        event_loop.run_app(&mut self)
    }

    /// Drive the `wasm32` run loop: build the typed event loop, wire the `EventLoopProxy`
    /// into a fresh [`App`], and spawn it via `EventLoopExtWebSys::spawn_app` (non-blocking —
    /// `run_app` would block the browser's single JS thread forever). The returned proxy is
    /// what `wasm_winit.rs` uses to deliver browser ROM bytes as [`AppEvent::RomLoaded`].
    ///
    /// # Panics
    /// Panics if the event loop can't be constructed (the browser lacks the APIs winit needs) —
    /// surfaced via `console_error_panic_hook`.
    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn run_wasm(config: Config) -> winit::event_loop::EventLoopProxy<AppEvent> {
        use winit::platform::web::EventLoopExtWebSys;
        let event_loop = EventLoop::<AppEvent>::with_user_event()
            .build()
            .expect("build event loop");
        let proxy = event_loop.create_proxy();
        let app = Self::new_empty(config, proxy.clone());
        event_loop.spawn_app(app);
        proxy
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.active.is_some() {
            return; // already initialized (e.g. resumed after suspend)
        }
        let window = match Self::create_window(event_loop) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("rustysnes: failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
        // `Gfx::new_async` is the shared init core; native drives it to completion synchronously
        // via `pollster::block_on` (inside `Gfx::new`) and continues straight into
        // `on_gfx_ready`. `wasm32`'s wgpu init is genuinely async (`request_adapter`/
        // `request_device` are real awaits in the browser, and `pollster::block_on` cannot block
        // on wasm32 — there is no second thread to block on while the single JS thread keeps the
        // event loop alive), so it spawns the future and delivers the result back through the
        // `EventLoopProxy<AppEvent>` (handled in `user_event`).
        #[cfg(not(target_arch = "wasm32"))]
        match Gfx::new(Arc::clone(&window), &self.config.video.present_mode) {
            Ok(gfx) => self.on_gfx_ready(gfx, window),
            Err(e) => {
                eprintln!("rustysnes: wgpu init failed: {e}");
                event_loop.exit();
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(proxy) = self.proxy.clone() {
            let present_mode = self.config.video.present_mode.clone();
            let window_for_gfx = Arc::clone(&window);
            self.pending_window = Some(window);
            wasm_bindgen_futures::spawn_local(async move {
                match Gfx::new_async(window_for_gfx, &present_mode).await {
                    Ok(gfx) => {
                        let _ = proxy.send_event(AppEvent::GfxReady(Box::new(gfx)));
                    }
                    Err(e) => web_sys::console::error_1(
                        &format!("rustysnes: wgpu init failed: {e}").into(),
                    ),
                }
            });
        }
    }

    /// `wasm32` — the async `Gfx` + browser ROM bytes arrive here (native never sends one).
    #[cfg(target_arch = "wasm32")]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::GfxReady(gfx) => {
                let window = self
                    .pending_window
                    .take()
                    .expect("window created in resumed() before Gfx::new_async resolved");
                self.on_gfx_ready(*gfx, window);
                if let Some(bytes) = self.pending_rom_bytes.take() {
                    self.load_rom_bytes_wasm(&bytes);
                }
            }
            AppEvent::RomLoaded(bytes) => {
                if self.active.is_some() {
                    self.load_rom_bytes_wasm(&bytes);
                } else {
                    // `Active` doesn't exist yet (the async `Gfx::new_async` hasn't resolved) —
                    // stash it; `GfxReady` above consumes it as soon as `Active` is built.
                    self.pending_rom_bytes = Some(bytes);
                }
            }
        }
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
    /// Create the window: a normal OS window on native, or (on `wasm32`) an attachment to the
    /// EXISTING `<canvas id="snes-canvas">` from `index.html` — the same canvas the `wasm-canvas`
    /// MVP uses (only one wasm frontend is ever compiled at a time, so reusing the element id is
    /// safe) — rather than letting winit create a detached canvas, so the page's CSS sizing and
    /// layout apply. Per the winit 0.30 web platform docs this is
    /// `WindowAttributesExtWebSys::with_canvas`.
    fn create_window(event_loop: &ActiveEventLoop) -> Result<Arc<Window>, String> {
        let attrs = Window::default_attributes()
            .with_title("RustySNES")
            .with_inner_size(winit::dpi::LogicalSize::new(512.0, 448.0));
        #[cfg(target_arch = "wasm32")]
        let attrs = {
            use wasm_bindgen::JsCast as _;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("snes-canvas"))
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            let Some(canvas) = canvas else {
                return Err("missing <canvas id=\"snes-canvas\">".to_string());
            };
            attrs.with_canvas(Some(canvas))
        };
        event_loop
            .create_window(attrs)
            .map(Arc::new)
            .map_err(|e| e.to_string())
    }

    /// Shared post-`Gfx`-init setup, called by `resumed` on native and by
    /// `user_event(AppEvent::GfxReady)` on `wasm32`: builds the egui integration, powers on the
    /// emulator, opens native audio (a no-op on `wasm32`, which uses [`crate::wasm_audio`]
    /// instead, driven per-frame from `render`), and constructs `Active`.
    fn on_gfx_ready(&mut self, gfx: Gfx, window: Arc<Window>) {
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
        #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
        let mut emu = EmuCore::new(0, self.config.region);
        // Native: a ROM passed on the CLI loads now. `wasm32`: the ROM arrives later via the
        // browser file picker as `AppEvent::RomLoaded`, so this is always empty here.
        #[cfg(not(target_arch = "wasm32"))]
        let initial_status = self
            .pending_rom
            .take()
            .map_or_else(String::new, |path| load_rom_file(&mut emu, &path));
        #[cfg(target_arch = "wasm32")]
        let initial_status = String::new();

        // Open the audio device (best-effort: a missing device leaves the emulator silent, not
        // dead). The producer-side resampler converts the S-DSP 32 kHz stream to the device rate.
        // Native only — `wasm32` drives `crate::wasm_audio` per-frame from `render` instead.
        #[cfg(not(target_arch = "wasm32"))]
        let audio = AudioOutput::new(Arc::new(AudioRing::new(13))).ok();
        #[cfg(not(target_arch = "wasm32"))]
        let dst_rate = audio.as_ref().map_or(48_000, |a| a.sample_rate);
        #[cfg(not(target_arch = "wasm32"))]
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
            #[cfg(not(target_arch = "wasm32"))]
            audio,
            #[cfg(not(target_arch = "wasm32"))]
            resampler,
            shell: ShellState {
                status: initial_status,
                ..ShellState::default()
            },
            pacer: Pacer::new(self.config.region.frame_rate()),
            applied_present_mode: self.config.video.present_mode.clone(),
            rewind: crate::rewind::RewindBuffer::new(
                self.config.rewind.capacity,
                self.config.rewind.interval_frames,
            ),
            quick_save: None,
        });
    }

    /// Load ROM bytes delivered by the browser file picker (`AppEvent::RomLoaded`) into the
    /// already-built `Active`. A no-op if `Active` doesn't exist yet (the caller stashes the
    /// bytes in `pending_rom_bytes` instead in that case).
    #[cfg(target_arch = "wasm32")]
    fn load_rom_bytes_wasm(&mut self, bytes: &[u8]) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
        active.shell.status = match emu.load_rom(bytes) {
            Ok(()) => "ROM loaded".to_string(),
            Err(e) => format!("ROM load failed: {e}"),
        };
        drop(emu);
        // A new cart invalidates every prior snapshot, same as the native `MenuAction::OpenRom`.
        active.rewind.clear();
        active.quick_save = None;
    }

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
        // --- (0) Apply a pending Settings → Video present-mode change to the live surface. ---
        // The Settings window mutates `config.video.present_mode` during the prior egui pass; the
        // surface was only ever configured once at startup, so the toggle did nothing until now.
        if config.video.present_mode != active.applied_present_mode {
            let applied = active.gfx.set_present_mode(&config.video.present_mode);
            active
                .applied_present_mode
                .clone_from(&config.video.present_mode);
            eprintln!("rustysnes: present mode applied -> {applied:?}");
        }

        let paused = active.shell.paused;
        // --- (1) Copy framebuffer + audio + read-only info under a BRIEF lock, then drop it. ---
        let (fb, fb_dims, info, audio_samples, debug) = {
            // `mut` is only needed on the synchronous drive path (run_frame/set_pad); the threaded
            // build only reads through the guard here.
            #[cfg_attr(feature = "emu-thread", allow(unused_mut))]
            let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
            // When NOT threaded, run as many whole emulated frames as real elapsed time has earned
            // (fixed-timestep), so emulation tracks the region rate, not the display refresh.
            #[cfg(not(feature = "emu-thread"))]
            let mut run_ahead_frame = None;
            #[cfg(not(feature = "emu-thread"))]
            let audio_samples = if paused {
                active.pacer.idle();
                Vec::new()
            } else {
                let frames = active.pacer.tick();
                emu.set_pad(0, active.pad1);
                let mut samples = Vec::new();
                for _ in 0..frames {
                    // Run-ahead (config-driven, off by default): peeks `run_ahead.frames` frames
                    // ahead for the PRESENTED video, while `emu`'s own persisted state (and audio
                    // — the continuous stream) only ever advances by exactly one real frame, same
                    // as the plain path below. See `crate::rewind::step_with_run_ahead`.
                    if config.run_ahead.frames > 0 {
                        run_ahead_frame = Some(crate::rewind::step_with_run_ahead(
                            &mut emu,
                            config.run_ahead.frames,
                        ));
                    } else {
                        emu.run_frame();
                    }
                    samples.extend_from_slice(emu.audio());
                    active.rewind.record(&emu);
                }
                samples
            };
            // Threaded build: the emu thread owns frame production; audio-from-thread is a TODO.
            // Still advance the FPS meter's wall clock so the status bar reads the present rate.
            #[cfg(feature = "emu-thread")]
            let audio_samples: Vec<(i16, i16)> = {
                if paused {
                    active.pacer.idle();
                } else {
                    active.pacer.note_present();
                }
                Vec::new()
            };
            // Run-ahead presents the deepest peeked frame, not `emu`'s own (1-real-frame-behind)
            // framebuffer; when it didn't run (paused, disabled, or the threaded build) fall back
            // to the emulator's own current frame, matching the pre-run-ahead behavior exactly.
            #[cfg(not(feature = "emu-thread"))]
            let (fb, dims) =
                run_ahead_frame.unwrap_or_else(|| (emu.framebuffer().to_vec(), emu.fb_dims()));
            #[cfg(feature = "emu-thread")]
            let (fb, dims) = (emu.framebuffer().to_vec(), emu.fb_dims());
            let info = ShellInfo {
                cart_name: emu.cart_name().map(str::to_string),
                region: emu.region(),
                fps: active.pacer.fps,
                rom_loaded: emu.rom_loaded(),
            };
            // Only build the debugger snapshot when the window is actually open — a real,
            // avoidable per-frame cost otherwise (`docs/frontend.md` §Debugger overlay).
            let debug = active.shell.debugger_open.then(|| emu.debug_snapshot());
            drop(emu); // release the brief lock BEFORE the wgpu upload + egui pass
            (fb, dims, info, audio_samples, debug)
        };

        // --- Push the frame's audio through the resampler into the ring (outside the lock). ---
        #[cfg(not(target_arch = "wasm32"))]
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
        // `wasm32`: `crate::wasm_audio` owns its own resampler/DRC servo (built for the
        // `wasm-canvas` MVP, T-81-005) — no `active.resampler`/`AudioRing` involved here.
        #[cfg(target_arch = "wasm32")]
        if config.audio.enabled && !audio_samples.is_empty() {
            crate::wasm_audio::set_volume(config.audio.volume);
            crate::wasm_audio::push_samples(&audio_samples);
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
            actions = active.shell.render(ui, &info, config, debug.as_ref());
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
    // The `MenuAction` match is inherently one function; the `wasm32` `OpenRom` split pushed it
    // just over the line-count lint.
    #[allow(clippy::too_many_lines)]
    fn dispatch_actions(
        active: &mut Active,
        config: &mut Config,
        event_loop: &ActiveEventLoop,
        actions: Vec<MenuAction>,
    ) {
        for action in actions {
            match action {
                MenuAction::OpenRom => {
                    // `wasm32`: a browser can't show a native file dialog from here — the
                    // page's own `<input id="rom-input">` (outside the canvas, the same element
                    // the `wasm-canvas` MVP uses) is the ROM-load affordance; point the user at
                    // it rather than risk an unverified gesture-propagation `.click()` chain
                    // through winit's event pump. `AppEvent::RomLoaded` (wired in
                    // `wasm_winit.rs`) does the actual loading.
                    #[cfg(target_arch = "wasm32")]
                    {
                        active.shell.status =
                            "Use the file picker below the canvas to load a ROM".into();
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SNES ROM", &["sfc", "smc", "fig", "swc"])
                            .pick_file()
                        {
                            let mut emu =
                                active.core.lock().unwrap_or_else(PoisonError::into_inner);
                            active.shell.status = load_rom_file(&mut emu, &path);
                        }
                        // A new cart invalidates every prior snapshot (rewind ring +
                        // quick-save) — restoring one now would apply a foreign ROM's state to
                        // this System.
                        active.rewind.clear();
                        active.quick_save = None;
                    }
                }
                MenuAction::CloseRom => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .close_rom();
                    active.shell.status = "ROM closed".into();
                    active.rewind.clear();
                    active.quick_save = None;
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
                MenuAction::SaveState => {
                    let saved = {
                        let emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                        emu.rom_loaded().then(|| emu.save_state())
                    };
                    active.shell.status = if let Some(bytes) = saved {
                        active.quick_save = Some(bytes);
                        "Save state saved".into()
                    } else {
                        "Save state: no ROM loaded".into()
                    };
                }
                MenuAction::LoadState => {
                    let result = active.quick_save.as_ref().map(|bytes| {
                        let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                        emu.load_state(bytes)
                    });
                    active.shell.status = match result {
                        Some(Ok(())) => "Save state loaded".into(),
                        Some(Err(e)) => format!("Save state load failed: {e}"),
                        None => "Load state: no save state yet".into(),
                    };
                }
                MenuAction::Rewind => {
                    let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                    active.shell.status = if active.rewind.step_back(&mut emu) {
                        "Rewound".into()
                    } else if active.rewind.is_enabled() {
                        "Rewind: buffer empty".into()
                    } else {
                        "Rewind: disabled (config.rewind.capacity == 0)".into()
                    };
                }
            }
        }
        // Persist any Settings-window edits to config (best-effort).
        let _ = config.save();
    }
}

/// Read a ROM file into `emu`, then best-effort install any required coprocessor firmware and a
/// `.srm` battery save sitting next to the ROM. Returns a human-readable status line.
///
/// Native only — `wasm32` has no filesystem; ROM bytes arrive from the browser file picker as
/// `AppEvent::RomLoaded` (`App::load_rom_bytes_wasm`) instead.
#[cfg(not(target_arch = "wasm32"))]
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
/// first one the board accepts. Returns whether a dump was installed. Native only (see
/// `load_rom_file`).
#[cfg(not(target_arch = "wasm32"))]
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
