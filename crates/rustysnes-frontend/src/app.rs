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
#[cfg(not(target_arch = "wasm32"))]
use crate::gfx::{SNES_H_NTSC, SNES_W};
use crate::input::{Button, Buttons};
use crate::pacing::Pacer;
use crate::ui_shell::{MenuAction, ShellInfo, ShellState};

#[cfg(feature = "emu-thread")]
use crate::emu_thread::{EmuControl, EmuThread, SharedInput};
#[cfg(feature = "emu-thread")]
use crate::present_buffer::PresentBuffer;

// Lua scripting + TAS movies (`v0.8.0`, T-81-002) — native-only (`mlua`'s vendored Lua VM needs a
// C compiler + `std`).
#[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
use rustysnes_core::cart::Region as CartRegion;
#[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
use rustysnes_core::movie::{Movie, MoviePlayer, MovieRecorder};
#[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
use rustysnes_script::ScriptEngine;

// Game Genie / Pro Action Replay cheats (`v0.8.0`, T-81-003) — no platform constraint, unlike
// `scripting`'s `mlua`.
#[cfg(feature = "cheats")]
use crate::cheats::CheatEntry;

// Read/write watchpoints (`v0.8.0`, T-81-001b). `WatchpointEntry` is always compiled (see
// `debug_snapshot.rs`'s doc), unlike `CheatEntry` above.
use crate::debug_snapshot::WatchpointEntry;

// Rollback netplay (`v0.8.0`, T-82-002) — native-only (`netplay.rs`'s own module doc has why).
#[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
use crate::netplay::NetplayState;

// RetroAchievements (`v0.8.0`, T-82-003) — native-only (`cheevos.rs`'s own module doc has why).
#[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
use crate::cheevos::CheevosState;

/// The fixed upscale factor `crate::hd_compositor::composite` applies when an HD texture pack is
/// active (`v1.3.0`). `2` keeps the composited output's worst case (a hi-res frame, 512×448 native
/// → 1024×896) comfortably under this device's actual granted `max_texture_dimension_2d`
/// regardless of region (`Gfx::ensure_texture_capacity`'s own backstop, which now tracks the real
/// device limit rather than a hardcoded constant — see that method's doc). Not yet
/// user-configurable — a fixed v1 scope choice (`docs/adr/0010`), not a technical ceiling:
/// `Gfx::ensure_texture_capacity` grows to fit whatever scale is requested.
#[cfg(all(not(feature = "emu-thread"), feature = "hd-pack"))]
const HD_PACK_SCALE: u32 = 2;

/// The window's initial/default scale — `INITIAL_SCALE`x the SNES native resolution (`v1.3.0`,
/// RustyNES parity: that sibling project also defaults to 3x/300%). Native only; `wasm32`'s
/// canvas size is controlled by the page, not this constant.
#[cfg(not(target_arch = "wasm32"))]
const INITIAL_SCALE: u32 = 3;

/// A floor on the requested window width (`v1.3.0`, View → Window Size), padding past the raw
/// `SNES_W * scale` so the egui menu bar (File / Emulation / Tools / View / Debug / Help) never
/// gets clipped at `1x`. Native only.
#[cfg(not(target_arch = "wasm32"))]
const MIN_CHROME_WIDTH: f64 = 560.0;

/// Extra window height (`v1.3.0`, View → Window Size) added past the raw `SNES_H_NTSC * scale`
/// for the egui menu bar + status bar, which are drawn as a fixed-size overlay on top of the
/// (letterboxed) game image rather than reserving their own space in the framebuffer. Native
/// only.
#[cfg(not(target_arch = "wasm32"))]
const CHROME_HEIGHT: f64 = 56.0;

/// The typed winit user-event, used by both native and `wasm32`.
///
/// On `wasm32` the wgpu init is async and the ROM arrives via the browser file picker, so
/// neither can be produced synchronously inside `ApplicationHandler::resumed`. Instead they're
/// delivered back into the event loop as user events via an `EventLoopProxy` (RustyNES's own
/// `AppEvent`, ported). `EmuFrame` (`v1.1.0`) is native-only (`emu-thread` feature): the emu
/// thread pings the winit thread after every produced frame so it can do per-frame housekeeping
/// (`RetroAchievements` polling once that lands, `docs/frontend.md` §emu-thread) and request a
/// redraw — `about_to_wait` already requests one every idle tick, so today this is only the
/// redraw request, but the hook exists for the housekeeping this port's "Known remaining gaps"
/// still needs.
pub enum AppEvent {
    /// The async `Gfx::new_async` future resolved (`wasm32`).
    GfxReady(Box<Gfx>),
    /// The browser file picker delivered ROM bytes (`wasm32`).
    RomLoaded(Vec<u8>),
    /// The emu thread published a new frame (`v1.1.0`, native `emu-thread` only).
    EmuFrame,
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
    /// The running emulation thread (joined on drop; `.control()` re-synced from the shell/config
    /// state each present, `v1.1.0`).
    #[cfg(feature = "emu-thread")]
    emu_thread: EmuThread,
    /// The lock-free framebuffer handoff the emu thread publishes into each produced frame, so
    /// the present path never blocks on the emu mutex to copy it (`v1.1.0`).
    #[cfg(feature = "emu-thread")]
    present: Arc<crate::present_buffer::PresentBuffer>,
    /// The present-staging buffer `present.take_into` copies the latest published frame into —
    /// reused across presents; unchanged when nothing new was published (so a slow emu thread
    /// simply re-presents the last frame rather than flashing black).
    #[cfg(feature = "emu-thread")]
    present_staging: Vec<u8>,
    /// Scratch buffer the present path copies `present_staging` into once the `emu` mutex has
    /// been released (`v1.1.0` fix, reviewed): reused every frame via `clear()` +
    /// `extend_from_slice()`, which only reallocates if the framebuffer's resolution grows
    /// (e.g. an SNES hi-res mode switch), unlike `Vec::clone`'s always-fresh allocation.
    #[cfg(feature = "emu-thread")]
    fb_scratch: Vec<u8>,
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
    /// The current emulation-speed multiplier (`v1.0.0`; `1.0` = normal). Transient session
    /// state, like `pad1` — never persisted to `config.toml`, always starts at `1.0` (the
    /// determinism-safe default). Scales both `pacer`'s target rate (`MenuAction::SetSpeed`) and
    /// the audio resampler's DRC ratio (so alt-speed audio pitch-shifts instead of over/underrunning
    /// the ring).
    speed: f32,
    /// The wall-clock time spent producing this present's emulated frame(s) (`v1.0.0`, the
    /// Performance panel), or `None` when nothing was measured this present — always `None` on
    /// the `emu-thread` build (frame production happens on a different thread, outside this
    /// timing scope; a known `emu-thread` gap, same posture as speed presets above).
    last_frame_time_ms: Option<f32>,
    /// The present-mode string currently applied to the surface; compared against the live config
    /// each present so a Settings → Video toggle reconfigures the wgpu surface.
    applied_present_mode: String,
    /// The egui [`crate::config::AppTheme`] currently applied to `egui_ctx`; compared against the
    /// live config each frame (`v1.0.0`) so a Settings → System theme change re-themes the shell,
    /// same change-guard pattern as `applied_present_mode` above.
    applied_theme: crate::config::AppTheme,
    /// Whether borderless fullscreen is currently applied to `window`; compared against
    /// `shell.fullscreen` each frame (`v1.0.0`), same change-guard pattern as the two above.
    applied_fullscreen: bool,
    /// The rewind ring buffer (`config.rewind`-driven; a zero-capacity buffer is a permanent
    /// no-op, so this is always constructed, never `Option`-wrapped).
    rewind: crate::rewind::RewindBuffer,
    /// A single quick-save-state slot (`MenuAction::SaveState`/`LoadState`).
    quick_save: Option<Vec<u8>>,
    /// The loaded Lua script, if any (`v0.8.0`, T-81-002). `None` until `MenuAction::LoadScript`.
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    script: Option<ScriptEngine>,
    /// TAS movie record/playback state (`v0.8.0`, T-81-002).
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    movie: MovieState,
    /// The in-session cheat-code list (`v0.8.0`, T-81-003). Empty until `Cheats…` adds entries.
    #[cfg(feature = "cheats")]
    cheats: Vec<CheatEntry>,
    /// The debugger's armed read/write watchpoint list (`v0.8.0`, T-81-001b). Empty until the
    /// debugger overlay's Watch panel adds entries. Not feature-gated (unlike `cheats` above) —
    /// [`WatchpointEntry`] is one of `debug_snapshot.rs`'s always-compiled types (see that
    /// module's doc), so this field stays a plain, unconditional `Vec` too; only the actual
    /// `Bus::set_watchpoints` sync call below is `debug-hooks`-gated.
    watchpoints: Vec<WatchpointEntry>,
    /// The debugger's armed PC-breakpoint list (`v0.9.0`, T-81-001 PR B). Empty until the
    /// debugger overlay's 65C816 panel adds entries — same always-compiled, unconditional-`Vec`
    /// posture as `watchpoints` above.
    breakpoints: Vec<u32>,
    /// Native rollback netplay connection state (`v0.8.0`, T-82-002). `Idle` until
    /// `MenuAction::ConnectNetplay`.
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    netplay: NetplayState,
    /// Native `RetroAchievements` session state (`v0.8.0`, T-82-003). No `rc_client` exists until
    /// the first `MenuAction::LoginCheevos`.
    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
    cheevos: CheevosState,
}

/// TAS movie record/playback state (`v0.8.0`, T-81-002) — mutually exclusive with itself (you
/// can't record and play back at once), independent of whether a Lua script is also loaded.
#[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
#[derive(Default)]
enum MovieState {
    /// Neither recording nor playing.
    #[default]
    Idle,
    /// Recording; the recorder accumulates one [`rustysnes_core::movie::FrameInput`] per real
    /// frame via [`MovieRecorder::capture`].
    Recording(MovieRecorder),
    /// Replaying a loaded movie; each real frame consumes one recorded input via
    /// [`MoviePlayer::next_frame`].
    Playing(MoviePlayer),
}

/// The app: holds the config + the deferred ROM path until `resumed()` builds `Active`.
pub struct App {
    config: Config,
    /// A ROM path passed on the native CLI, opened once the window exists. Native only —
    /// `wasm32` has no CLI; its ROM arrives via the browser file picker instead.
    #[cfg(not(target_arch = "wasm32"))]
    pending_rom: Option<PathBuf>,
    active: Option<Active>,
    /// The proxy used to deliver [`AppEvent`]s back into this event loop. Always `Some` on
    /// `wasm32` (native builds `Gfx` synchronously and only needs one for `emu-thread`'s
    /// [`AppEvent::EmuFrame`] ping, `v1.1.0`) — created in [`Self::run`] before `run_app`, since
    /// `EventLoop::create_proxy` needs the built (not yet active) event loop.
    #[cfg(any(target_arch = "wasm32", feature = "emu-thread"))]
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
            #[cfg(feature = "emu-thread")]
            proxy: None,
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
        #[cfg(feature = "emu-thread")]
        {
            self.proxy = Some(event_loop.create_proxy());
        }
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
        // `wasm32` only: `resumed()` can legitimately fire again (e.g. a tab losing/regaining
        // visibility) before the async `Gfx::new_async` spawned by an EARLIER `resumed()` call
        // has resolved (`self.active` is still `None` at that point, so the check above alone
        // doesn't catch this) — without this guard, a second call would spawn a second
        // concurrent `Gfx::new_async` future and a second window, racing the first.
        #[cfg(target_arch = "wasm32")]
        if self.pending_window.is_some() {
            return; // a Gfx::new_async from an earlier resumed() call is still in flight
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

    /// `wasm32` — the async `Gfx` + browser ROM bytes arrive here (native never sends either).
    /// `EmuFrame` (`v1.1.0`) arrives on native only (`emu-thread`): request a redraw so the
    /// present path picks up the frame the emu thread just published — unconditional across
    /// targets since the variant always exists (only its senders differ per platform/feature).
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            #[cfg(target_arch = "wasm32")]
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
            #[cfg(target_arch = "wasm32")]
            AppEvent::RomLoaded(bytes) => {
                if self.active.is_some() {
                    self.load_rom_bytes_wasm(&bytes);
                } else {
                    // `Active` doesn't exist yet (the async `Gfx::new_async` hasn't resolved) —
                    // stash it; `GfxReady` above consumes it as soon as `Active` is built.
                    self.pending_rom_bytes = Some(bytes);
                }
            }
            // Native never constructs either variant (they're wasm32-only in practice); this
            // arm only exists so the match stays exhaustive on that target.
            #[cfg(not(target_arch = "wasm32"))]
            AppEvent::GfxReady(_) | AppEvent::RomLoaded(_) => {}
            AppEvent::EmuFrame => {
                if let Some(active) = self.active.as_ref() {
                    active.window.request_redraw();
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
                if let Some(button) = active.shell.awaiting_bind {
                    // Only act on the key-DOWN edge; the matching release (of whatever key was
                    // just captured) falls through to `latch_key` below on the next call, same as
                    // any other key release.
                    if key_event.state.is_pressed() {
                        active.shell.awaiting_bind = None;
                        if let winit::keyboard::PhysicalKey::Code(code) = key_event.physical_key {
                            // Esc cancels the rebind rather than binding itself to `button`.
                            if code != winit::keyboard::KeyCode::Escape {
                                Self::rebind_key(&mut self.config, button, code);
                            }
                        }
                    }
                } else {
                    // `v1.0.1` global hotkeys — fixed, not rebindable, checked before gameplay
                    // latching. Only on the key-DOWN edge, and never on OS auto-repeat (holding a
                    // hotkey must not spam Reset/Quit/etc. every ~30ms) — see
                    // `Self::hotkey_menu_action`. Suppressed while an egui widget (e.g. a Settings
                    // text field) has keyboard focus, so e.g. typing a space doesn't also toggle
                    // pause.
                    let handled_as_hotkey = if key_event.state.is_pressed()
                        && !key_event.repeat
                        && !active.egui_ctx.egui_wants_keyboard_input()
                        && let winit::keyboard::PhysicalKey::Code(code) = key_event.physical_key
                    {
                        Self::dispatch_hotkey(active, &mut self.config, event_loop, code)
                    } else {
                        false
                    };
                    if !handled_as_hotkey {
                        Self::latch_key(active, &self.config, &key_event);
                    }
                }
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
        // Native defaults to `INITIAL_SCALE`x (RustyNES parity, `v1.3.0`); the wasm32 canvas is
        // sized by the page's own CSS (`web/index.html`), so the `LogicalSize` passed there is a
        // fallback only, kept at a fixed 2x for parity with that page's `512x448` canvas rule.
        #[cfg(not(target_arch = "wasm32"))]
        let (init_w, init_h) = Self::chrome_padded_size(INITIAL_SCALE);
        #[cfg(target_arch = "wasm32")]
        let (init_w, init_h) = (512.0, 448.0);
        let attrs = Window::default_attributes()
            .with_title("RustySNES")
            .with_inner_size(winit::dpi::LogicalSize::new(init_w, init_h));
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

    /// Compute a chrome-padded `(width, height)` logical size for `scale`x the SNES native
    /// resolution (`v1.3.0`, View → Window Size / `create_window`'s default; RustyNES parity):
    /// floors width at `MIN_CHROME_WIDTH` and always adds `CHROME_HEIGHT`, so the egui menu bar
    /// has room even at `1x`.
    #[cfg(not(target_arch = "wasm32"))]
    fn chrome_padded_size(scale: u32) -> (f64, f64) {
        let w = (f64::from(SNES_W) * f64::from(scale)).max(MIN_CHROME_WIDTH);
        let h = f64::from(SNES_H_NTSC).mul_add(f64::from(scale), CHROME_HEIGHT);
        (w, h)
    }

    /// Apply a View → Window Size selection (`v1.3.0`, RustyNES parity). Exits fullscreen first
    /// (synchronously, updating `applied_fullscreen` in lockstep, rather than leaving the
    /// resize to race the next frame's fullscreen change-guard in `render`) so the resize below
    /// actually takes effect against a normal window rather than a fullscreen one. Clamps `scale`
    /// to `1..=4` and requests the chrome-padded inner size: `request_inner_size` may grant the
    /// resize synchronously (`Some`, in which case no separate `Resized` event will follow, so
    /// `Gfx::resize` is called directly here) or asynchronously (`None`, in which case the
    /// existing `WindowEvent::Resized` handler in `window_event` picks it up when it fires).
    #[cfg(not(target_arch = "wasm32"))]
    fn set_window_scale(active: &mut Active, scale: u32) {
        let scale = scale.clamp(1, 4);
        if active.shell.fullscreen {
            active.shell.fullscreen = false;
            active.window.set_fullscreen(None);
            active.applied_fullscreen = false;
        }
        let (w, h) = Self::chrome_padded_size(scale);
        if let Some(granted) = active
            .window
            .request_inner_size(winit::dpi::LogicalSize::new(w, h))
        {
            active.gfx.resize(granted.width, granted.height);
        }
        active.shell.status = format!("Window size: {scale}x ({}%)", scale * 100);
    }

    /// Shared post-`Gfx`-init setup, called by `resumed` on native and by
    /// `user_event(AppEvent::GfxReady)` on `wasm32`: builds the egui integration, powers on the
    /// emulator, opens native audio (a no-op on `wasm32`, which uses [`crate::wasm_audio`]
    /// instead, driven per-frame from `render`), and constructs `Active`.
    // One straight-line construction sequence (egui + emulator + audio + `emu-thread`'s
    // control/present/producer + the `Active` literal); the length is inherent to how much
    // state one session needs to stand up once, not a sign this needs splitting.
    #[allow(clippy::too_many_lines)]
    fn on_gfx_ready(&mut self, gfx: Gfx, window: Arc<Window>) {
        let egui_ctx = egui::Context::default();
        // Apply the configured theme immediately (`v1.0.0`) rather than leaving egui's own
        // built-in dark default in place until the first `render()` change-check happens to
        // notice a mismatch — `Active::applied_theme` below is initialized to this SAME value,
        // so `render()`'s guard correctly stays a no-op until the user changes it in Settings.
        crate::ui_shell::apply_theme(&egui_ctx, self.config.theme);
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
        // Re-select the configured HD texture pack (`v1.3.0`) for a ROM passed on the CLI, same
        // as `MenuAction::OpenRom`'s File-menu path does.
        #[cfg(all(not(target_arch = "wasm32"), feature = "hd-pack"))]
        if emu.rom_loaded()
            && let Some(name) = self.config.video.hd_pack_name.as_deref()
        {
            let _ = emu.set_hd_pack(Some(name));
        }
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
        // `EmuCore: Send` (via `rustysnes-cart::Board: Send`, `v1.0.0`) — the emu-thread build
        // is not the reason this is `Arc<Mutex<_>>` rather than a bare owned value; it's the
        // shape both the (default-off) dedicated emulation thread AND the present path need to
        // share it either way.
        let core = Arc::new(Mutex::new(emu));

        #[cfg(feature = "emu-thread")]
        let rom_loaded_at_spawn = core
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .rom_loaded();
        #[cfg(feature = "emu-thread")]
        let input = Arc::new(SharedInput::default());
        #[cfg(feature = "emu-thread")]
        let control = EmuControl::new(self.config.region.frame_rate());
        #[cfg(feature = "emu-thread")]
        control.set_has_rom(rom_loaded_at_spawn);
        #[cfg(feature = "emu-thread")]
        let present = PresentBuffer::new();
        #[cfg(feature = "emu-thread")]
        let audio_producer = audio
            .as_ref()
            .map(|a| a.make_producer(self.config.audio.volume));
        #[cfg(feature = "emu-thread")]
        let proxy = self
            .proxy
            .clone()
            .expect("proxy created in App::run before on_gfx_ready");
        #[cfg(feature = "emu-thread")]
        let emu_thread = EmuThread::spawn(
            Arc::clone(&core),
            Arc::clone(&input),
            audio_producer,
            proxy,
            Arc::clone(&control),
            Arc::clone(&present),
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
            emu_thread,
            #[cfg(feature = "emu-thread")]
            present,
            #[cfg(feature = "emu-thread")]
            present_staging: Vec::new(),
            #[cfg(feature = "emu-thread")]
            fb_scratch: Vec::new(),
            pad1: Buttons::default(),
            #[cfg(not(target_arch = "wasm32"))]
            audio,
            #[cfg(not(target_arch = "wasm32"))]
            resampler,
            shell: ShellState {
                status: initial_status,
                #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
                netplay_local_addr: "0.0.0.0:7777".into(),
                #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
                netplay_peer_addr: "127.0.0.1:7777".into(),
                welcome_open: !self.config.first_run_seen,
                ..ShellState::default()
            },
            pacer: Pacer::new(self.config.region.frame_rate()),
            speed: 1.0,
            last_frame_time_ms: None,
            applied_present_mode: self.config.video.present_mode.clone(),
            applied_theme: self.config.theme,
            applied_fullscreen: false,
            rewind: crate::rewind::RewindBuffer::new(
                self.config.rewind.capacity,
                self.config.rewind.interval_frames,
            ),
            quick_save: None,
            #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
            script: None,
            #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
            movie: MovieState::default(),
            #[cfg(feature = "cheats")]
            cheats: Vec::new(),
            watchpoints: Vec::new(),
            breakpoints: Vec::new(),
            #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
            netplay: NetplayState::default(),
            #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
            cheevos: CheevosState::default(),
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

    /// Set `emu`'s controller input for the emulated frame about to run: live keyboard/gamepad
    /// input (`pad1`) normally, or a TAS movie's recorded/replayed input when one is active
    /// (`v0.8.0`, T-81-002) — recording observes `pad1` without changing it; playback overrides
    /// it entirely with the movie's own P1/P2. A finished playback returns to
    /// [`MovieState::Idle`] and falls back to live input for this same call, not the next one.
    ///
    /// Takes `movie`/`status` as separate `&mut` parameters (not `active: &mut Active`) so this
    /// can be called while a `MutexGuard` borrowed from `active.core` (`emu`) is still live —
    /// passing the whole `Active` struct into a function call (unlike a direct field projection
    /// like `active.rewind.record(..)`) defeats the borrow checker's disjoint-field analysis.
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    fn apply_frame_input(
        movie: &mut MovieState,
        pad1: Buttons,
        status: &mut String,
        emu: &mut EmuCore,
    ) {
        match movie {
            MovieState::Playing(player) => {
                if let Some(f) = player.next_frame() {
                    emu.set_pad(0, Buttons(f.p1));
                    emu.set_pad(1, Buttons(f.p2));
                    return;
                }
                *movie = MovieState::Idle;
                *status = "Movie playback finished".into();
            }
            MovieState::Recording(rec) => rec.capture(pad1.0, 0),
            MovieState::Idle => {}
        }
        // Outside `Playing`, P2 is not live-driven by this frontend (single-player input) — reset
        // it explicitly rather than leaving it at whatever a just-finished movie last set it to.
        emu.set_pad(0, pad1);
        emu.set_pad(1, Buttons(0));
    }

    /// Run the loaded Lua script's per-frame callback, if any (`v0.8.0`, T-81-002). Writes are
    /// gated whenever a movie is recording or playing — a script must never perturb a
    /// deterministic run it doesn't own. A script that errors (a bug, or the runaway-loop
    /// instruction-budget guard tripping) is unloaded rather than left to error every frame. See
    /// [`Self::apply_frame_input`]'s doc for why this takes separate fields, not `&mut Active`.
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    fn pump_script(script_slot: &mut Option<ScriptEngine>, movie: &MovieState, emu: &mut EmuCore) {
        let Some(script) = script_slot.as_mut() else {
            return;
        };
        script.set_writes_locked(!matches!(movie, MovieState::Idle));
        if let Err(e) = script.on_frame(&mut emu.system_mut().bus) {
            eprintln!("rustysnes: script error, unloading: {e}");
            *script_slot = None;
        }
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

    /// Apply a Settings → Input tab rebind capture to P1's table (`config.p1`, the only table
    /// `latch_key` actually consults — P2 keyboard binding isn't wired into gameplay input yet).
    fn rebind_key(config: &mut Config, button: Button, code: winit::keyboard::KeyCode) {
        config.p1.rebind(format!("{code:?}"), button);
    }

    /// Pure global-hotkey key→action mapping (`v1.0.1`) for the hotkeys that already have a
    /// corresponding [`MenuAction`] — split out from [`Self::dispatch_hotkey`] specifically so it
    /// can be unit-tested without a live `Active`/`ActiveEventLoop`. Fixed, not (yet) rebindable.
    /// None of these physical keys collide with the default P1 gameplay binds (arrows/X/Z/S/A/Q/
    /// W/RShift/Enter), but a user COULD rebind a gameplay button onto one of them via the Input
    /// tab; a hotkey always wins that conflict, a deliberate, predictable choice rather than
    /// trying to arbitrate it.
    ///
    /// `rustysnes help hotkeys` documents this exact table — keep the two in sync.
    const fn hotkey_menu_action(code: winit::keyboard::KeyCode) -> Option<MenuAction> {
        use winit::keyboard::KeyCode;
        Some(match code {
            KeyCode::Escape => MenuAction::Quit,
            KeyCode::F1 => MenuAction::SaveState,
            KeyCode::F4 => MenuAction::LoadState,
            KeyCode::F2 => MenuAction::Reset,
            KeyCode::F3 => MenuAction::PowerCycle,
            KeyCode::F5 => MenuAction::Rewind,
            KeyCode::F12 => MenuAction::OpenRom,
            KeyCode::Space => MenuAction::TogglePause,
            // Mirrors the Debug menu's own gating exactly (`debugger_open` must never become
            // `true` without `debug-hooks` — see `ui_shell.rs`'s module doc) rather than
            // introducing a second, hotkey-only way to reach a UI surface the default build
            // never vets.
            #[cfg(feature = "debug-hooks")]
            KeyCode::Backquote => MenuAction::ToggleDebugger,
            _ => return None,
        })
    }

    /// Dispatch a global hotkey (`v1.0.1`). Returns `true` if `code` was recognized (the caller
    /// must NOT also latch it as gameplay input), `false` otherwise. F9/F11 have no existing
    /// `MenuAction` (the mouse-driven UI flips the `ShellState` field directly — a checkbox / a
    /// menu button setting `= true`), so the hotkey does the same rather than inventing an action
    /// variant with no other caller; everything else goes through [`Self::hotkey_menu_action`].
    fn dispatch_hotkey(
        active: &mut Active,
        config: &mut Config,
        event_loop: &ActiveEventLoop,
        code: winit::keyboard::KeyCode,
    ) -> bool {
        match code {
            winit::keyboard::KeyCode::F9 => {
                active.shell.save_states_open = !active.shell.save_states_open;
                true
            }
            winit::keyboard::KeyCode::F11 => {
                active.shell.fullscreen = !active.shell.fullscreen;
                true
            }
            _ => {
                let Some(action) = Self::hotkey_menu_action(code) else {
                    return false;
                };
                Self::dispatch_actions(active, config, event_loop, vec![action]);
                true
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

        // --- (0b) Apply a pending Settings → System theme change (`v1.0.0`). ---
        if config.theme != active.applied_theme {
            crate::ui_shell::apply_theme(&active.egui_ctx, config.theme);
            active.applied_theme = config.theme;
        }

        // --- (0c) Apply a pending View → Fullscreen toggle (`v1.0.0`). ---
        if active.shell.fullscreen != active.applied_fullscreen {
            let mode = active
                .shell
                .fullscreen
                .then_some(winit::window::Fullscreen::Borderless(None));
            active.window.set_fullscreen(mode);
            active.applied_fullscreen = active.shell.fullscreen;
        }

        let paused = active.shell.paused;
        // --- (1) Copy framebuffer + audio + read-only info under a BRIEF lock, then drop it. ---
        let (fb, fb_dims, info, audio_samples, debug, save_slots) = {
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
                active.last_frame_time_ms = None;
                Vec::new()
            } else {
                let frames = active.pacer.tick();
                let mut samples = Vec::new();
                // `v1.0.0` Performance panel: time the frame-production loop below. Only
                // overwritten when at least one frame actually ran this present, so an idle
                // present (0 frames earned this tick) doesn't flicker the reading to ~0.
                let produce_t0 = web_time::Instant::now();
                for _ in 0..frames {
                    // Netplay (`v0.8.0`, T-82-002) is its OWN drive loop, deliberately never the
                    // single-player path below it: a `RollbackSession` owns pad application,
                    // frame production, AND presentation (`NetplayState::drive`) — running it
                    // alongside movie/cheat/rewind/run-ahead machinery designed for a single,
                    // locally-authoritative `System` would race or double-drive the same state
                    // a remote peer is also authoritative over.
                    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
                    if active.netplay.is_connected() {
                        let local_input = active.pad1.sanitize_dpad().0;
                        if let Err(e) = active.netplay.drive(local_input, &mut emu) {
                            eprintln!("rustysnes: netplay error, disconnecting: {e}");
                            active.netplay = NetplayState::Idle;
                            active.shell.status = format!("Netplay error: {e}");
                        }
                        samples.extend_from_slice(emu.audio());
                        continue;
                    }
                    // Sets `emu`'s pad(s) for THIS emulated frame: live input (the pre-existing
                    // behavior, unchanged when `scripting` is off or no movie is active), or a
                    // movie's recorded/replayed input (`v0.8.0`, T-81-002) when one is active —
                    // moved inside the loop (was once before it) so a catch-up burst of several
                    // emulated frames in one real tick each gets its OWN movie frame, not the
                    // same one repeated. Byte-identical to the prior single-call-before-the-loop
                    // behavior when idle/off, since `active.pad1` doesn't change mid-tick either
                    // way.
                    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                    Self::apply_frame_input(
                        &mut active.movie,
                        active.pad1,
                        &mut active.shell.status,
                        &mut emu,
                    );
                    #[cfg(not(all(feature = "scripting", not(target_arch = "wasm32"))))]
                    emu.set_pad(0, active.pad1);
                    // Cheat patches (`v0.8.0`, T-81-003) are host-applied external input, same
                    // framing as a movie's recorded input — installed as a `Bus` read intercept
                    // (real Game Genie/Pro Action Replay codes overwhelmingly target cartridge
                    // ROM, not WRAM, so a poke-based model would silently do nothing for them).
                    #[cfg(feature = "cheats")]
                    crate::cheats::sync(&active.cheats, &mut emu.system_mut().bus);
                    // Read/write watchpoints (`v0.8.0`, T-81-001b) — same "just re-sync
                    // unconditionally, once per real frame" pattern as cheats above.
                    #[cfg(feature = "debug-hooks")]
                    crate::watchpoints::sync(&active.watchpoints, &mut emu.system_mut().bus);
                    // Controller port 2 peripheral selection (`v0.9.0`, Phase 7 niche
                    // peripherals) — same "just re-sync unconditionally, once per real frame"
                    // pattern as cheats/watchpoints above; cheap (one enum-tag write) when
                    // unchanged. Host-input capture for the non-Gamepad devices (a real mouse
                    // pointer driving Super Scope aim / Mouse deltas, extra gamepads for
                    // Multitap sub-pads) is a follow-up frontend task — this wires the CORE's
                    // protocol correctly but doesn't yet feed it live host input
                    // (`docs/frontend.md` §Peripherals).
                    emu.set_port_device(1, config.port2_peripheral.to_core());
                    // PC breakpoints (`v0.9.0`, T-81-001 PR B) — same re-sync pattern as above;
                    // a no-op branch in `EmuCore::run_frame` when the list is empty.
                    emu.set_breakpoints(&active.breakpoints);
                    // Per-voice audio mutes (`v1.0.1`) — same re-sync pattern as above; all-false
                    // (unmuted) is the default, byte-identical to every prior release.
                    emu.set_voice_mutes(config.audio.voice_mutes);
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
                    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                    Self::pump_script(&mut active.script, &active.movie, &mut emu);
                    // RetroAchievements (`v0.8.0`, T-82-003): one rc_client frame per emulated
                    // frame, reading WRAM through the same `Bus::peek_wram` the debugger/scripting
                    // already use. Scope cut, honestly noted: not wired into the netplay `drive`
                    // path above (a `RollbackSession`-driven `System` and achievement tracking
                    // interacting — e.g. resimulation re-triggering rc_client frames — is a
                    // separate, deferred concern).
                    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
                    active.cheevos.do_frame(emu.system_mut());
                }
                if frames > 0 {
                    active.last_frame_time_ms = Some(produce_t0.elapsed().as_secs_f32() * 1000.0);
                }
                samples
            };
            // Threaded build (`v1.1.0`): the emu thread owns frame production AND audio (via its
            // own `AudioProducer`, pushed directly into the ring — see `emu_thread.rs`'s
            // `drive_one`), so this present never produces samples of its own; `audio_samples`
            // stays empty here and the resampler-push step below is naturally a no-op. Still
            // advance the FPS meter's wall clock so the status bar reads the present rate, and
            // re-sync the thread's lifecycle control from the current shell/config state — the
            // same "just re-sync unconditionally once per frame" pattern cheats/watchpoints/etc.
            // already use on the synchronous path above.
            #[cfg(feature = "emu-thread")]
            let audio_samples: Vec<(i16, i16)> = {
                if paused {
                    active.pacer.idle();
                } else {
                    active.pacer.note_present();
                }
                let control = active.emu_thread.control();
                control.set_has_rom(emu.rom_loaded());
                control.set_user_paused(paused);
                control.set_speed(active.speed);
                Vec::new()
            };
            // Run-ahead presents the deepest peeked frame, not `emu`'s own (1-real-frame-behind)
            // framebuffer; when it didn't run (paused, disabled, or the threaded build) fall back
            // to the emulator's own current frame, matching the pre-run-ahead behavior exactly.
            #[cfg(not(feature = "emu-thread"))]
            let (fb, dims) =
                run_ahead_frame.unwrap_or_else(|| (emu.framebuffer().to_vec(), emu.fb_dims()));
            // Composite the HD texture pack (`v1.3.0`), if one is active, while `emu` is still
            // locked -- pure CPU work, no wgpu touched here, so this doesn't hold the lock any
            // longer than the plain framebuffer copy above already did. Not wired for the
            // `emu-thread` build: that build's framebuffer arrives via the lock-free
            // `PresentBuffer` handoff below, outside this locked block, with no equivalent
            // `TileTag` handoff yet -- a documented scope cut (`docs/frontend.md`), not silently
            // dropped.
            #[cfg(all(not(feature = "emu-thread"), feature = "hd-pack"))]
            let (fb, dims) = if let Some((tags, tiles)) = emu.hd_pack_composite_inputs() {
                let (out_w, out_h, out) = crate::hd_compositor::composite(
                    &fb,
                    dims.0,
                    dims.1,
                    &tags,
                    tiles,
                    HD_PACK_SCALE,
                );
                (out, (out_w, out_h))
            } else {
                (fb, dims)
            };
            // `v1.1.0`: `dims` is cheap to read under the still-held `emu` lock; the actual
            // framebuffer BYTES come from the lock-free `PresentBuffer` handoff instead of
            // `emu.framebuffer()` — copied only AFTER `emu` is dropped below (see the `drop(emu)`
            // site), so the present path never blocks the emu thread's next `run_frame()` on this
            // — potentially hi-res, up to ~896 KiB — copy. A prior revision ran `take_into` + the
            // buffer copy while `emu` was still locked, serializing the two threads for nothing.
            #[cfg(feature = "emu-thread")]
            let dims = emu.fb_dims();
            // `v1.0.0` Performance panel: the audio ring's occupancy as a percentage of its
            // capacity (a rough "audio health" gauge — persistently near 0% or 100% means the
            // producer/consumer are drifting apart). `None` on `wasm32` (no `active.audio` there
            // — see `crate::wasm_audio` instead) or when no audio device opened.
            #[cfg(not(target_arch = "wasm32"))]
            let audio_health_pct = active.audio.as_ref().map(|a| {
                let cap = a.ring.capacity();
                #[allow(clippy::cast_precision_loss)]
                if cap == 0 {
                    0.0
                } else {
                    (a.ring.occupancy() as f32 / cap as f32) * 100.0
                }
            });
            #[cfg(target_arch = "wasm32")]
            let audio_health_pct: Option<f32> = None;
            // `available_hd_packs` walks a directory on disk (`discover_packs`) -- only pay that
            // cost while the Settings window is actually open, mirroring the debug-snapshot/
            // save-slots guards just below. `active_hd_pack` is a cheap field read, always taken.
            #[cfg(feature = "hd-pack")]
            let available_hd_packs = if active.shell.settings_open {
                emu.available_hd_packs()
            } else {
                Vec::new()
            };
            #[cfg(feature = "hd-pack")]
            let active_hd_pack = emu.hd_pack_name().map(str::to_string);
            let info = ShellInfo {
                cart_name: emu.cart_name().map(str::to_string),
                region: emu.region(),
                fps: active.pacer.fps,
                rom_loaded: emu.rom_loaded(),
                speed: active.speed,
                frame_time_ms: active.last_frame_time_ms,
                audio_health_pct,
                #[cfg(feature = "hd-pack")]
                available_hd_packs,
                #[cfg(feature = "hd-pack")]
                active_hd_pack,
            };
            // Only build the debugger snapshot when the window is actually open — a real,
            // avoidable per-frame cost otherwise (`docs/frontend.md` §Debugger overlay).
            let debug = active.shell.debugger_open.then(|| emu.debug_snapshot());
            // Same guard for the Save States slot grid (`v1.0.0`, `save_states.rs`): only read
            // the per-ROM slot directory from disk while the manager window is actually open.
            let save_slots = active
                .shell
                .save_states_open
                .then(|| {
                    emu.rom_loaded().then(|| {
                        let hash = rustysnes_core::movie::hash_rom(emu.rom());
                        (0..crate::save_states::NUM_SLOTS)
                            .map(|slot| crate::save_states::slot_meta(&hash, slot))
                            .collect::<Vec<_>>()
                    })
                })
                .flatten();
            drop(emu); // release the brief lock BEFORE the wgpu upload, egui pass, AND (for the
            // emu-thread build) the PresentBuffer copy below.
            // `take_into` returning `false` (nothing new published yet, e.g. paused or the very
            // first present before any frame exists) simply keeps whatever `present_staging`
            // already held — a black frame until the first publish. `fb_scratch` reuses its
            // prior allocation via `clear()` + `extend_from_slice()` (steady-state zero
            // allocations after the framebuffer resolution first stabilizes) rather than
            // `Vec::clone`'s always-fresh heap allocation.
            #[cfg(feature = "emu-thread")]
            let fb = {
                active.present.take_into(&mut active.present_staging);
                let mut scratch = std::mem::take(&mut active.fb_scratch);
                scratch.clear();
                scratch.extend_from_slice(&active.present_staging);
                scratch
            };
            (fb, dims, info, audio_samples, debug, save_slots)
        };

        // Drain RetroAchievements HTTP completions/events (outside the emu lock — `CheevosState`
        // isn't emu state). Surfaces any newly-unlocked achievement in the status bar.
        #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
        for toast in active.cheevos.poll() {
            active.shell.status = toast;
        }

        // --- Push the frame's audio through the resampler into the ring (outside the lock). ---
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(audio) = active
            .audio
            .as_ref()
            .filter(|_| config.audio.enabled && !audio_samples.is_empty())
        {
            active.resampler.set_volume(config.audio.volume);
            let cap = audio.ring.capacity();
            // Fold the speed multiplier into the DRC ratio (`v1.0.0` speed presets): at 2x speed
            // there are ~2x as many source samples per real second, so the resampler must consume
            // them ~2x as fast to avoid ring overrun — the side effect is exactly the expected
            // pitch-shifted "fast forward" sound, matching RustyNES's own speed-preset design.
            let ratio = drc_ratio(audio.ring.occupancy(), cap / 2, cap) * f64::from(active.speed);
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
        // Hand `fb`'s allocation back to `fb_scratch` so next present's copy reuses its capacity
        // instead of allocating fresh (see the `fb_scratch` field doc + the copy site above).
        #[cfg(feature = "emu-thread")]
        {
            active.fb_scratch = fb;
        }

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

        // --- (3) Present the framebuffer (clears then draws the fullscreen triangle, through
        // the active post-filter if any -- `v1.2.0`, `PostFilter::None` is byte-identical to the
        // pre-filter-pipeline direct blit). ---
        active.gfx.present(
            &mut encoder,
            &view,
            config.video.filter,
            config.video.crt_scanline,
            config.video.crt_mask,
            config.video.hqx_strength,
        );

        // --- (4) Run the always-on egui shell pass. The shell NEVER touches the emu lock. ---
        #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
        let cheevos_status = crate::ui_shell::CheevosStatus {
            logged_in: active.cheevos.is_logged_in(),
            pending: active.cheevos.login_pending(),
            display_name: active.cheevos.display_name(),
            error: active.cheevos.login_error(),
        };
        let raw_input = active.egui_state.take_egui_input(&active.window);
        let mut actions = Vec::new();
        let full_output = active.egui_ctx.run_ui(raw_input, |ui| {
            actions = active.shell.render(
                ui,
                &info,
                config,
                debug.as_ref(),
                &mut active.watchpoints,
                &mut active.breakpoints,
                save_slots.as_deref(),
                #[cfg(feature = "cheats")]
                &mut active.cheats,
                #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
                active.netplay.is_connected(),
                #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
                &cheevos_status,
            );
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
                            // Re-select the configured HD texture pack (`v1.3.0`) for the new
                            // ROM, if any -- `load_rom` above already cleared the previous ROM's
                            // pack; a load failure here (the new ROM has no matching pack, or
                            // none was ever configured) just leaves tagging off, same as never
                            // having selected one.
                            #[cfg(feature = "hd-pack")]
                            if let Some(name) = config.video.hd_pack_name.as_deref() {
                                let _ = emu.set_hd_pack(Some(name));
                            }
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
                #[cfg(feature = "hd-pack")]
                MenuAction::SetHdPack(name) => {
                    let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                    match emu.set_hd_pack(name.as_deref()) {
                        Ok(()) => {
                            active.shell.status = name.map_or_else(
                                || "HD texture pack cleared".to_string(),
                                |n| format!("HD texture pack: {n}"),
                            );
                        }
                        Err(e) => {
                            active.shell.status = format!("HD texture pack load failed: {e}");
                        }
                    }
                    drop(emu);
                    let _ = config.save();
                }
                MenuAction::SetRegion(region) => {
                    config.region = region;
                    let _ = config.save();
                    active.shell.status = format!("Region: {region:?} (restart to apply)");
                }
                MenuAction::SetSpeed(speed) => {
                    active.speed = speed;
                    // Takes effect immediately on the synchronous drive path (the `Pacer`'s
                    // period feeds `render`'s fixed-timestep loop directly). The `emu-thread`
                    // build (`v1.1.0`) picks it up on the next present too, via `render`'s own
                    // per-present `EmuControl::set_speed` re-sync (`emu_thread.rs`'s thread-owned
                    // `Pacer` consults it every loop iteration) — no longer a no-op there.
                    active
                        .pacer
                        .set_rate(config.region.frame_rate() * f64::from(speed));
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let pct = (speed * 100.0).round() as u32;
                    active.shell.status = format!("Speed: {pct}%");
                }
                MenuAction::SetWindowScale(scale) => {
                    // Native only (View → Window Size); the wasm32 canvas is sized by the page's
                    // own CSS, so this menu entry doesn't exist there (`ui_shell.rs`) but the
                    // variant itself stays unconditional to keep this match exhaustive on both
                    // targets.
                    #[cfg(not(target_arch = "wasm32"))]
                    Self::set_window_scale(active, scale);
                    #[cfg(target_arch = "wasm32")]
                    let _ = scale;
                }
                MenuAction::DismissWelcome => {
                    config.first_run_seen = true;
                    let _ = config.save();
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
                MenuAction::DebuggerContinue => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .resume();
                }
                MenuAction::DebuggerPause => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .pause();
                }
                MenuAction::DebuggerStepInto => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .step_into();
                }
                MenuAction::DebuggerStepOver => {
                    active
                        .core
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .step_over();
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
                MenuAction::SaveStateSlot(slot) => {
                    let saved = {
                        let emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                        emu.rom_loaded().then(|| {
                            let hash = rustysnes_core::movie::hash_rom(emu.rom());
                            let (fb_w, fb_h) = emu.fb_dims();
                            let thumb = crate::save_states::nearest_resize(
                                emu.framebuffer(),
                                fb_w,
                                fb_h,
                                crate::save_states::THUMB_W,
                                crate::save_states::THUMB_H,
                            );
                            (hash, thumb, emu.save_state())
                        })
                    };
                    active.shell.status = match saved {
                        #[allow(clippy::cast_possible_truncation)]
                        Some((hash, thumb, state)) => match crate::save_states::save_to_slot(
                            &hash,
                            slot,
                            crate::save_states::THUMB_W as u16,
                            crate::save_states::THUMB_H as u16,
                            &thumb,
                            &state,
                        ) {
                            Ok(()) => format!("Saved to slot {slot}"),
                            Err(e) => format!("Save to slot {slot} failed: {e}"),
                        },
                        None => "Save States: no ROM loaded".into(),
                    };
                }
                MenuAction::LoadStateSlot(slot) => {
                    let hash = {
                        let emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                        emu.rom_loaded()
                            .then(|| rustysnes_core::movie::hash_rom(emu.rom()))
                    };
                    active.shell.status = match hash {
                        Some(hash) => match crate::save_states::load_from_slot(&hash, slot) {
                            Ok(state) => {
                                let mut emu =
                                    active.core.lock().unwrap_or_else(PoisonError::into_inner);
                                match emu.load_state(&state) {
                                    Ok(()) => format!("Loaded slot {slot}"),
                                    Err(e) => format!("Load slot {slot} failed: {e}"),
                                }
                            }
                            Err(e) => format!("Load slot {slot} failed: {e}"),
                        },
                        None => "Save States: no ROM loaded".into(),
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
                #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                MenuAction::LoadScript => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Lua script", &["lua"])
                        .pick_file()
                    {
                        active.shell.status = match std::fs::read_to_string(&path) {
                            Ok(src) => match ScriptEngine::new().and_then(|mut engine| {
                                engine.load(&src)?;
                                Ok(engine)
                            }) {
                                Ok(engine) => {
                                    active.script = Some(engine);
                                    "Script loaded".into()
                                }
                                Err(e) => format!("Script load failed: {e}"),
                            },
                            Err(e) => format!("Script read failed: {e}"),
                        };
                    }
                }
                #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                MenuAction::StartMovieRecording => {
                    let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                    active.shell.status = if emu.rom_loaded() {
                        let region = emu
                            .system_mut()
                            .bus
                            .cart
                            .as_ref()
                            .map_or(CartRegion::Ntsc, |c| c.header.region);
                        let rom = emu.rom().to_vec();
                        let recorder =
                            MovieRecorder::from_current_state(region, &rom, emu.system_mut());
                        drop(emu);
                        active.movie = MovieState::Recording(recorder);
                        "Movie recording started".into()
                    } else {
                        "Start recording: no ROM loaded".into()
                    };
                }
                #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                MenuAction::StopMovieRecording => {
                    let recorded = match core::mem::take(&mut active.movie) {
                        MovieState::Recording(rec) => Some(rec.finish()),
                        other => {
                            active.movie = other;
                            None
                        }
                    };
                    active.shell.status = match recorded {
                        Some(movie) => {
                            let frame_count = movie.frames.len();
                            let dest = rfd::FileDialog::new()
                                .add_filter("RustySNES movie", &["rsnesmov"])
                                .set_file_name("movie.rsnesmov")
                                .save_file();
                            dest.map_or_else(
                                || "Movie recording stopped (not saved)".into(),
                                |path| match std::fs::write(&path, movie.serialize()) {
                                    Ok(()) => format!("Movie saved ({frame_count} frames)"),
                                    Err(e) => format!("Movie save failed: {e}"),
                                },
                            )
                        }
                        None => "Stop recording: not currently recording".into(),
                    };
                }
                #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                MenuAction::LoadAndPlayMovie => {
                    active.shell.status = 'load_movie: {
                        let Some(path) = rfd::FileDialog::new()
                            .add_filter("RustySNES movie", &["rsnesmov"])
                            .pick_file()
                        else {
                            break 'load_movie "Load movie: cancelled".into();
                        };
                        let bytes = match std::fs::read(&path) {
                            Ok(b) => b,
                            Err(e) => break 'load_movie format!("Movie read failed: {e}"),
                        };
                        let movie: Movie = match Movie::deserialize(&bytes) {
                            Ok(m) => m,
                            Err(e) => break 'load_movie format!("Movie load failed: {e}"),
                        };
                        let mut emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                        if let Err(e) = movie.verify_rom(emu.rom()) {
                            break 'load_movie format!("Movie load failed: {e}");
                        }
                        if let Err(e) = movie.seek_to_start(emu.system_mut()) {
                            break 'load_movie format!("Movie load failed: {e}");
                        }
                        drop(emu);
                        active.movie = MovieState::Playing(MoviePlayer::new(movie));
                        active.rewind.clear();
                        active.quick_save = None;
                        "Movie playback started".into()
                    };
                }
                #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                MenuAction::StopMoviePlayback => {
                    active.shell.status = if matches!(active.movie, MovieState::Playing(_)) {
                        active.movie = MovieState::Idle;
                        "Movie playback stopped".into()
                    } else {
                        "Stop playback: not currently playing".into()
                    };
                }
                #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
                MenuAction::ConnectNetplay => {
                    active.shell.netplay_error = None;
                    active.shell.status = 'connect: {
                        let local_addr: std::net::SocketAddr =
                            match active.shell.netplay_local_addr.trim().parse() {
                                Ok(a) => a,
                                Err(e) => {
                                    let msg = format!("Bad local address: {e}");
                                    active.shell.netplay_error = Some(msg.clone());
                                    break 'connect msg;
                                }
                            };
                        let peer_addr: std::net::SocketAddr =
                            match active.shell.netplay_peer_addr.trim().parse() {
                                Ok(a) => a,
                                Err(e) => {
                                    let msg = format!("Bad peer address: {e}");
                                    active.shell.netplay_error = Some(msg.clone());
                                    break 'connect msg;
                                }
                            };
                        let emu = active.core.lock().unwrap_or_else(PoisonError::into_inner);
                        if !emu.rom_loaded() {
                            let msg = "Connect: no ROM loaded".to_string();
                            active.shell.netplay_error = Some(msg.clone());
                            break 'connect msg;
                        }
                        let rom = emu.rom().to_vec();
                        drop(emu);
                        match crate::netplay::start(
                            local_addr,
                            peer_addr,
                            active.shell.netplay_local_player,
                            &rom,
                        ) {
                            Ok(session) => {
                                active.netplay = NetplayState::Connected(session);
                                active.rewind.clear();
                                active.quick_save = None;
                                "Netplay connected".into()
                            }
                            Err(e) => {
                                let msg = format!("Netplay connect failed: {e}");
                                active.shell.netplay_error = Some(msg.clone());
                                msg
                            }
                        }
                    };
                }
                #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
                MenuAction::DisconnectNetplay => {
                    active.netplay = NetplayState::Idle;
                    active.shell.status = "Netplay disconnected".into();
                }
                #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
                MenuAction::LoginCheevos => {
                    active.cheevos.login(
                        &active.shell.cheevos_username.clone(),
                        &active.shell.cheevos_password.clone(),
                    );
                    // Don't linger a plaintext password in memory longer than the call needs it.
                    active.shell.cheevos_password.clear();
                }
                #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
                MenuAction::LogoutCheevos => {
                    active.cheevos.logout();
                    active.shell.status = "RetroAchievements: logged out".into();
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

#[cfg(all(test, feature = "scripting", not(target_arch = "wasm32")))]
mod frame_input_tests {
    use rustysnes_core::movie::{FrameInput, Movie, MoviePlayer, StartPoint};

    use super::{App, Buttons, CartRegion, EmuCore, MovieState};
    use crate::config::Region;

    fn one_frame_movie(p2: u16) -> Movie {
        Movie {
            seed: 0,
            region: CartRegion::Ntsc,
            rom_sha256: [0; 32],
            start: StartPoint::PowerOn,
            frames: vec![FrameInput { p1: 0, p2 }],
        }
    }

    #[test]
    fn p2_resets_to_zero_once_movie_playback_finishes() {
        let mut movie_state = MovieState::Playing(MoviePlayer::new(one_frame_movie(0x8080)));
        let mut status = String::new();
        let mut emu = EmuCore::new(0, Region::Ntsc);

        // First call consumes the movie's only frame — P2 should reflect it.
        App::apply_frame_input(&mut movie_state, Buttons(0), &mut status, &mut emu);
        emu.run_frame();
        assert_eq!(emu.system_mut().bus.joypad(1), 0x8080);

        // Second call: playback is finished — must fall back to live input, not leave P2 stuck
        // at the movie's last value.
        App::apply_frame_input(&mut movie_state, Buttons(0), &mut status, &mut emu);
        emu.run_frame();
        assert_eq!(emu.system_mut().bus.joypad(1), 0);
        assert!(matches!(movie_state, MovieState::Idle));
    }
}

#[cfg(test)]
mod hotkey_tests {
    use super::*;

    #[test]
    fn known_hotkeys_map_to_expected_actions() {
        use winit::keyboard::KeyCode;
        assert_eq!(
            App::hotkey_menu_action(KeyCode::Escape),
            Some(MenuAction::Quit)
        );
        assert_eq!(
            App::hotkey_menu_action(KeyCode::F1),
            Some(MenuAction::SaveState)
        );
        assert_eq!(
            App::hotkey_menu_action(KeyCode::F2),
            Some(MenuAction::Reset)
        );
        assert_eq!(
            App::hotkey_menu_action(KeyCode::F3),
            Some(MenuAction::PowerCycle)
        );
        assert_eq!(
            App::hotkey_menu_action(KeyCode::F4),
            Some(MenuAction::LoadState)
        );
        assert_eq!(
            App::hotkey_menu_action(KeyCode::F5),
            Some(MenuAction::Rewind)
        );
        assert_eq!(
            App::hotkey_menu_action(KeyCode::F12),
            Some(MenuAction::OpenRom)
        );
        assert_eq!(
            App::hotkey_menu_action(KeyCode::Space),
            Some(MenuAction::TogglePause)
        );
    }

    #[test]
    fn unmapped_keys_are_not_hotkeys() {
        use winit::keyboard::KeyCode;
        // A representative sample of default P1 gameplay keys + a few arbitrary others — none of
        // these should ever be claimed by the hotkey table.
        for code in [
            KeyCode::ArrowUp,
            KeyCode::KeyX,
            KeyCode::KeyZ,
            KeyCode::KeyS,
            KeyCode::KeyA,
            KeyCode::KeyQ,
            KeyCode::KeyW,
            KeyCode::ShiftRight,
            KeyCode::Enter,
            KeyCode::KeyM,
            KeyCode::Digit1,
        ] {
            assert_eq!(
                App::hotkey_menu_action(code),
                None,
                "{code:?} must not be a hotkey"
            );
        }
    }

    #[cfg(feature = "debug-hooks")]
    #[test]
    fn backquote_toggles_debugger_only_when_debug_hooks_is_enabled() {
        assert_eq!(
            App::hotkey_menu_action(winit::keyboard::KeyCode::Backquote),
            Some(MenuAction::ToggleDebugger)
        );
    }

    #[cfg(not(feature = "debug-hooks"))]
    #[test]
    fn backquote_is_not_a_hotkey_without_debug_hooks() {
        assert_eq!(
            App::hotkey_menu_action(winit::keyboard::KeyCode::Backquote),
            None
        );
    }
}
