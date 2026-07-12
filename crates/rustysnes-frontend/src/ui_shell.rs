//! The always-on egui shell: the menu bar (File / Emulation / Tools / View / Debug / Help), the
//! status bar, the tabbed Settings window, and the debugger overlay.
//!
//! THE NON-NEGOTIABLE RULE (RustyNES `docs/frontend.md`): egui runs **every frame**, and the
//! shell NEVER holds the emu lock inside the egui closure. Menu interactions return a
//! [`MenuAction`]; the app dispatches it *after* the egui pass. The debugger overlay's panels
//! (`crate::debugger`, `v1.7.0 "Telemetry"` onward) render the [`DebugSnapshot`] the app copies
//! out under the same brief lock [`ShellInfo`] already uses — never touched from inside this
//! module. The Debug menu entry that opens the overlay is gated behind the `debug-hooks` feature
//! (default off): without it, `debugger_open` can never become `true`, so the app never builds a
//! snapshot and the debugger is unreachable in a shipped default build.

// `ShellState`'s bools are each an independent, feature-gated window-visibility/UI-transient
// flag (debugger/settings/cheats open, paused) — a state machine would only obscure that they're
// orthogonal, not a single mode to switch between.
#![allow(clippy::struct_excessive_bools)]

#[cfg(feature = "cheats")]
use crate::cheats::CheatEntry;
use crate::config::{Config, Region};
use crate::debug_snapshot::{DebugSnapshot, MEMORY_WINDOW_LEN, WatchpointEntry, WatchpointKind};
use crate::input::Button;
use crate::save_states::SlotMeta;

/// An action requested from the egui pass, dispatched by `App::dispatch_menu_action` AFTER the
/// pass returns (so it never runs while the emu lock is held inside the egui closure).
// `Eq` dropped (kept only `PartialEq`) when `SetSpeed(f32)` was added (`v1.0.0`) — `f32` has no
// `Eq` impl (NaN), and nothing in this crate relies on `MenuAction: Eq` (no `HashSet`/`HashMap`
// keying), only `==` comparisons, which `PartialEq` alone already supports.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    /// Open the file picker to load a ROM.
    OpenRom,
    /// Close the currently loaded ROM (present a blank frame).
    CloseRom,
    /// Reset (soft) the console.
    Reset,
    /// Power-cycle (hard reset) the console.
    PowerCycle,
    /// Toggle pause.
    TogglePause,
    /// Save a save-state to the active (single) quick-save slot.
    SaveState,
    /// Load a save-state from the active (single) quick-save slot.
    LoadState,
    /// Step back by one recorded rewind snapshot (`config.rewind`; a no-op when disabled or the
    /// buffer is empty).
    Rewind,
    /// Save a thumbnail-previewed save state to slot `u8` (`v1.0.0`, `save_states.rs`; disk-backed
    /// and per-ROM, distinct from the RAM-only single [`Self::SaveState`] quick-save slot above).
    SaveStateSlot(u8),
    /// Load a thumbnail-previewed save state from slot `u8`.
    LoadStateSlot(u8),
    /// Switch the console region (NTSC/PAL).
    SetRegion(Region),
    /// Set the emulation-speed multiplier (`v1.0.0`; one of [`SPEED_PRESETS`], transient —
    /// session-only, never persisted to `config.toml`, always launches at `1.0`x — the
    /// determinism-safe default).
    SetSpeed(f32),
    /// Resize the window to `scale`x the SNES native resolution (`v1.3.0`, View → Window Size;
    /// RustyNES parity) — `1..=4`. Native only; a no-op on `wasm32` (the canvas size is
    /// controlled by the page). Transient, same posture as [`Self::SetSpeed`] — no config field.
    SetWindowScale(u32),
    /// Toggle the debugger overlay visibility.
    ToggleDebugger,
    /// Open the Settings window.
    OpenSettings,
    /// Quit the application.
    Quit,
    /// Load a Lua script from disk and start running it (`scripting` feature, T-81-002; native
    /// only — `mlua`'s vendored Lua VM needs a C compiler + `std`).
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    LoadScript,
    /// Start TAS movie recording from the current live state.
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    StartMovieRecording,
    /// Stop TAS movie recording and save the result to disk.
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    StopMovieRecording,
    /// Load a `.rsnesmov` file and start playing it back.
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    LoadAndPlayMovie,
    /// Stop TAS movie playback.
    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
    StopMoviePlayback,
    /// Bind/connect a native UDP netplay session (`v0.8.0` T-82-002) using the Netplay window's
    /// current local-address/peer-address/player-slot fields.
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    ConnectNetplay,
    /// End the active netplay session and fall back to single-player.
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    DisconnectNetplay,
    /// Begin an asynchronous `RetroAchievements` login (`v0.8.0` T-82-003) using the
    /// `RetroAchievements` window's current username/password fields.
    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
    LoginCheevos,
    /// Log out of the current `RetroAchievements` session.
    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
    LogoutCheevos,
    /// Resume from a debugger pause/breakpoint (`v0.9.0`, T-81-001 PR B).
    DebuggerContinue,
    /// Pause execution for the debugger without waiting for a breakpoint.
    DebuggerPause,
    /// Single-step exactly one CPU instruction.
    DebuggerStepInto,
    /// Step over the current instruction (runs a `JSR`/`JSL` to completion instead of into it).
    DebuggerStepOver,
    /// Dismiss the first-run welcome modal (`v1.0.0`) — persists `config.first_run_seen` so it
    /// never reappears.
    DismissWelcome,
    /// Select (`Some(name)`) or clear (`None`) the active HD texture pack for the current ROM
    /// (`v1.3.0`, `hd-pack` feature) — see `crate::emu::EmuCore::set_hd_pack`.
    #[cfg(feature = "hd-pack")]
    SetHdPack(Option<String>),
}

// `DebugPanel` lives in `crate::debugger` (`v1.7.0 "Telemetry"`) — re-exported via `use` below so
// `ShellState::panel`'s field type stays a short, unqualified name.
pub use crate::debugger::DebugPanel;

/// The egui shell's own persistent UI state (what's open, which tab/panel). Separate from the
/// emulator so the shell renders even with no ROM and the emu lock is never taken to draw it.
#[derive(Debug, Default)]
pub struct ShellState {
    /// Whether the debugger overlay is visible.
    pub debugger_open: bool,
    /// Whether the Settings window is open.
    pub settings_open: bool,
    /// The selected debugger panel.
    pub panel: DebugPanel,
    /// The selected Settings tab index.
    pub settings_tab: usize,
    /// A transient status-bar message (e.g. "Loaded `<game>`", "Save state 1").
    pub status: String,
    /// Whether emulation is paused (mirrored from the app for the menu checkmark).
    pub paused: bool,
    /// Whether the Cheats window is visible (`v0.8.0` T-81-003).
    #[cfg(feature = "cheats")]
    pub cheats_open: bool,
    /// The Cheats window's "add a code" text-entry buffer.
    #[cfg(feature = "cheats")]
    pub cheat_code_input: String,
    /// The Cheats window's last parse-error message, if the most recent "Add" attempt failed.
    #[cfg(feature = "cheats")]
    pub cheat_code_error: Option<String>,
    /// Whether the Netplay window is visible (`v0.8.0` T-82-002).
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    pub netplay_open: bool,
    /// The Netplay window's "local address" text-entry buffer (`host:port` to bind).
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    pub netplay_local_addr: String,
    /// The Netplay window's "peer address" text-entry buffer (`host:port` to connect to).
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    pub netplay_peer_addr: String,
    /// Which controller slot (`0` or `1`) this peer's own input will drive.
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    pub netplay_local_player: u8,
    /// The Netplay window's last connection-attempt error message, if any.
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    pub netplay_error: Option<String>,
    /// Whether the `RetroAchievements` window is visible (`v0.8.0` T-82-003).
    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
    pub cheevos_open: bool,
    /// The `RetroAchievements` window's username text-entry buffer.
    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
    pub cheevos_username: String,
    /// The `RetroAchievements` window's password text-entry buffer.
    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
    pub cheevos_password: String,
    /// The Watch panel's "add a watchpoint" address text-entry buffer (hex, `$bank:offset` or
    /// bare offset — `v0.8.0` T-81-001b).
    pub watch_addr_input: String,
    /// The Watch panel's "add a watchpoint" kind picker.
    pub watch_kind_input: WatchpointKind,
    /// The Watch panel's last address-parse error, if the most recent "Add" attempt failed.
    pub watch_addr_error: Option<String>,
    /// The 65C816 panel's "add a breakpoint" address text-entry buffer (`v0.9.0`, T-81-001 PR B).
    pub bp_addr_input: String,
    /// The 65C816 panel's last breakpoint address-parse error, if the most recent "Add" attempt
    /// failed.
    pub bp_addr_error: Option<String>,
    /// The Input tab's rebind-in-progress marker: the P1 button awaiting its next physical key,
    /// or `None` when idle. Set by the tab's own "Rebind" button; consumed by
    /// `App::window_event`'s key handler, which intercepts the very next key press instead of
    /// latching it as gameplay input.
    pub awaiting_bind: Option<Button>,
    /// Whether the Save States manager window is visible (`v1.0.0`, `save_states.rs`).
    pub save_states_open: bool,
    /// Whether the Performance panel is visible (`v1.0.0`).
    pub performance_open: bool,
    /// Whether borderless fullscreen is requested (`v1.0.0`). Compared against
    /// `Active::applied_fullscreen` each frame, same change-guard pattern as
    /// `applied_present_mode`/`applied_theme` — toggling this checkbox alone doesn't touch the
    /// OS window; `App::render` does that once it notices the mismatch.
    pub fullscreen: bool,
    /// Whether the first-run welcome modal is showing (`v1.0.0`). Starts `true` only when
    /// `Config::first_run_seen` was `false` at launch; dismissing it flips `first_run_seen` to
    /// `true` and persists the config so it never reappears.
    pub welcome_open: bool,
    /// Rolling frame-time history for the Performance panel's sparkline (`v1.0.0`), capped at
    /// `FRAME_TIME_HISTORY_LEN` samples (~2s at 60 fps). Pushed once per present whenever the
    /// panel is open, regardless of whether a real measurement exists that present (a paused
    /// present pushes `0.0`, same "no gap in the timeline" convention a real frame-time HUD uses).
    pub frame_time_history: std::collections::VecDeque<f32>,
    /// The Memory Compare panel's captured baseline (`debug.memory_window` bytes + the address it
    /// started at), or `None` before the first "Capture baseline" click (`v1.8.0 "Tracepoint"`).
    pub memcmp_baseline: Option<([u8; MEMORY_WINDOW_LEN], u32)>,
}

/// `ShellState::frame_time_history`'s cap.
const FRAME_TIME_HISTORY_LEN: usize = 120;

/// Read-only `RetroAchievements` session state the shell needs to render the login window,
/// copied out (never behind the emu lock — `CheevosState` isn't emu state) by the app each frame.
#[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
pub struct CheevosStatus<'a> {
    /// Whether a user is currently logged in.
    pub logged_in: bool,
    /// Whether a login attempt is currently in flight.
    pub pending: bool,
    /// The logged-in user's display name, if any.
    pub display_name: Option<&'a str>,
    /// The most recent login failure message, if any.
    pub error: Option<&'a str>,
}

/// Read-only facts the shell needs to render the status bar + window title without taking the
/// emu lock (the app copies these out under the brief lock, then renders).
#[derive(Debug, Clone, Default)]
pub struct ShellInfo {
    /// The loaded cart's board name, if any.
    pub cart_name: Option<String>,
    /// The current region.
    pub region: Region,
    /// The measured frames-per-second (the pacer's smoothed estimate).
    pub fps: f32,
    /// Whether a ROM is loaded.
    pub rom_loaded: bool,
    /// The current emulation-speed multiplier (`v1.0.0`; `1.0` = normal speed).
    pub speed: f32,
    /// Wall-clock time spent producing this present's emulated frame(s), in milliseconds
    /// (`v1.0.0`, the Performance panel). `None` when nothing was measured this present (paused,
    /// or the `emu-thread` build — see `Active::last_frame_time_ms`'s doc).
    pub frame_time_ms: Option<f32>,
    /// The audio ring's occupancy as a percentage of its capacity (`v1.0.0`, the Performance
    /// panel). `None` on `wasm32` or when no audio device opened.
    pub audio_health_pct: Option<f32>,
    /// HD texture pack names available for the current ROM (`v1.3.0`, `hd-pack` feature) — the
    /// Settings pack-selector's candidate list. Empty when no ROM is loaded.
    #[cfg(feature = "hd-pack")]
    pub available_hd_packs: Vec<String>,
    /// The currently active HD texture pack's name, if any.
    #[cfg(feature = "hd-pack")]
    pub active_hd_pack: Option<String>,
}

/// The emulation-speed presets surfaced in the Emulation → Speed submenu (`v1.0.0`).
///
/// `1.0` (100%) is the determinism-safe default the app always launches at — speed is transient
/// session state (`Active::speed`), never persisted to `config.toml`.
pub const SPEED_PRESETS: [f32; 7] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0];

/// Apply the configured [`crate::config::AppTheme`] to the egui context (`v1.0.0`).
///
/// Called by `App::render` only when the theme actually changed (tracked via
/// `Active::applied_theme`, mirroring the present-mode change-guard already used for
/// `config.video.present_mode`), not every frame.
pub fn apply_theme(ctx: &egui::Context, theme: crate::config::AppTheme) {
    use crate::config::AppTheme;
    match theme {
        AppTheme::Light => ctx.set_visuals(egui::Visuals::light()),
        AppTheme::Dark => ctx.set_visuals(egui::Visuals::dark()),
        AppTheme::System => match ctx.system_theme() {
            Some(egui::Theme::Light) => ctx.set_visuals(egui::Visuals::light()),
            _ => ctx.set_visuals(egui::Visuals::dark()),
        },
    }
}

impl ShellState {
    /// Render the always-on shell (menu bar + status bar + the optional Settings/debugger
    /// windows) and collect any requested [`MenuAction`]s. Returns the actions for the app to
    /// dispatch AFTER this pass — this function NEVER touches the emulator.
    ///
    /// Uses the egui 0.35 panel API: the caller passes the root `Ui` from `Context::run_ui`,
    /// into which the top/bottom panels are nested with `Panel::show`.
    // One straight-line immediate-mode egui pass (menu bar + status bar + windows); the line
    // count is inherent to the panel layout and reads more clearly as a unit than split apart.
    // The argument count only crosses the lint's threshold when every optional window feature
    // (cheats/netplay/retroachievements) is compiled in at once — each parameter is independently
    // feature-gated read-only shell input, not a sign this call needs restructuring.
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        root_ui: &mut egui::Ui,
        info: &ShellInfo,
        cfg: &mut Config,
        debug: Option<&DebugSnapshot>,
        watchpoints: &mut Vec<WatchpointEntry>,
        breakpoints: &mut Vec<u32>,
        save_slots: Option<&[SlotMeta]>,
        #[cfg(feature = "cheats")] cheats: &mut Vec<CheatEntry>,
        #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))] netplay_connected: bool,
        #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
        cheevos: &CheevosStatus<'_>,
    ) -> Vec<MenuAction> {
        let mut actions = Vec::new();
        let ctx = root_ui.ctx().clone();

        egui::Panel::top("menu_bar").show(root_ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open ROM…").clicked() {
                        actions.push(MenuAction::OpenRom);
                        ui.close();
                    }
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new("Close ROM"))
                        .clicked()
                    {
                        actions.push(MenuAction::CloseRom);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Settings…").clicked() {
                        self.settings_open = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        actions.push(MenuAction::Quit);
                        ui.close();
                    }
                });

                ui.menu_button("Emulation", |ui| {
                    let pause_label = if self.paused { "Resume" } else { "Pause" };
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new(pause_label))
                        .clicked()
                    {
                        actions.push(MenuAction::TogglePause);
                        ui.close();
                    }
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new("Reset"))
                        .clicked()
                    {
                        actions.push(MenuAction::Reset);
                        ui.close();
                    }
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new("Power Cycle"))
                        .clicked()
                    {
                        actions.push(MenuAction::PowerCycle);
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new("Save State"))
                        .clicked()
                    {
                        actions.push(MenuAction::SaveState);
                        ui.close();
                    }
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new("Load State"))
                        .clicked()
                    {
                        actions.push(MenuAction::LoadState);
                        ui.close();
                    }
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new("Rewind"))
                        .clicked()
                    {
                        actions.push(MenuAction::Rewind);
                        ui.close();
                    }
                    if ui
                        .add_enabled(info.rom_loaded, egui::Button::new("Save States…"))
                        .clicked()
                    {
                        self.save_states_open = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.menu_button("Region", |ui| {
                        if ui.radio(info.region == Region::Ntsc, "NTSC").clicked() {
                            actions.push(MenuAction::SetRegion(Region::Ntsc));
                            ui.close();
                        }
                        if ui.radio(info.region == Region::Pal, "PAL").clicked() {
                            actions.push(MenuAction::SetRegion(Region::Pal));
                            ui.close();
                        }
                    });
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let speed_pct = (info.speed * 100.0).round() as u32;
                    ui.menu_button(format!("Speed: {speed_pct}%"), |ui| {
                        for preset in SPEED_PRESETS {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let pct = (preset * 100.0).round() as u32;
                            // Exact float equality is intentional: `info.speed` is only ever set
                            // from a literal `SPEED_PRESETS` entry (`MenuAction::SetSpeed`'s only
                            // caller passes one straight through, no arithmetic drift), so this
                            // just asks "is this the currently-selected preset", not an
                            // approximate/computed comparison.
                            #[allow(clippy::float_cmp)]
                            let selected = info.speed == preset;
                            if ui.radio(selected, format!("{pct}%")).clicked() {
                                actions.push(MenuAction::SetSpeed(preset));
                                ui.close();
                            }
                        }
                    });
                });

                ui.menu_button("Tools", |ui| {
                    #[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]
                    {
                        if ui.button("Load Script…").clicked() {
                            actions.push(MenuAction::LoadScript);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Start Movie Recording").clicked() {
                            actions.push(MenuAction::StartMovieRecording);
                            ui.close();
                        }
                        if ui.button("Stop Movie Recording (save)").clicked() {
                            actions.push(MenuAction::StopMovieRecording);
                            ui.close();
                        }
                        if ui.button("Load && Play Movie…").clicked() {
                            actions.push(MenuAction::LoadAndPlayMovie);
                            ui.close();
                        }
                        if ui.button("Stop Movie Playback").clicked() {
                            actions.push(MenuAction::StopMoviePlayback);
                            ui.close();
                        }
                    }
                    #[cfg(not(all(feature = "scripting", not(target_arch = "wasm32"))))]
                    ui.label("(rebuild natively with --features scripting)");
                    #[cfg(feature = "cheats")]
                    {
                        ui.separator();
                        if ui.button("Cheats…").clicked() {
                            self.cheats_open = true;
                            ui.close();
                        }
                    }
                    #[cfg(not(feature = "cheats"))]
                    ui.label("(rebuild with --features cheats for Game Genie/PAR codes)");
                    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
                    {
                        ui.separator();
                        if ui.button("Netplay…").clicked() {
                            self.netplay_open = true;
                            ui.close();
                        }
                    }
                    #[cfg(not(all(feature = "netplay", not(target_arch = "wasm32"))))]
                    ui.label("(rebuild natively with --features netplay)");
                    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
                    {
                        ui.separator();
                        if ui.button("RetroAchievements…").clicked() {
                            self.cheevos_open = true;
                            ui.close();
                        }
                    }
                    #[cfg(not(all(feature = "retroachievements", not(target_arch = "wasm32"))))]
                    ui.label("(rebuild natively with --features retroachievements)");
                    // TODO(impl-phase): NSF/SPC player, ROM-DB editor, TAStudio.
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut cfg.video.integer_scale, "Integer scale");
                    ui.checkbox(&mut self.performance_open, "Performance panel");
                    ui.checkbox(&mut self.fullscreen, "Fullscreen");
                    ui.menu_button("Post-filter", |ui| {
                        for filter in crate::config::PostFilter::all() {
                            ui.radio_value(&mut cfg.video.filter, filter, filter.display_name());
                        }
                    });
                    // Window Size (`v1.3.0`, RustyNES parity) -- native only, a real OS window
                    // resize; meaningless on `wasm32` (the canvas size is controlled by the page,
                    // not the app).
                    #[cfg(not(target_arch = "wasm32"))]
                    ui.menu_button("Window Size", |ui| {
                        for (label, scale) in [
                            ("1x (100%)", 1u32),
                            ("2x (200%)", 2),
                            ("3x (300%)", 3),
                            ("4x (400%)", 4),
                        ] {
                            if ui.button(label).clicked() {
                                actions.push(MenuAction::SetWindowScale(scale));
                                ui.close();
                            }
                        }
                    });
                    // TODO(impl-phase): overscan.
                });

                ui.menu_button("Debug", |ui| {
                    #[cfg(feature = "debug-hooks")]
                    if ui
                        .checkbox(&mut self.debugger_open, "Debugger overlay")
                        .clicked()
                    {
                        ui.close();
                    }
                    #[cfg(not(feature = "debug-hooks"))]
                    ui.label("(rebuild with --features debug-hooks)");
                });

                ui.menu_button("Help", |ui| {
                    // TODO(impl-phase): in-app Documentation pane (the RustyNES Help → Docs).
                    ui.label(concat!("RustySNES v", env!("CARGO_PKG_VERSION")));
                });
            });
        });

        egui::Panel::bottom("status_bar").show(root_ui, |ui| {
            ui.horizontal(|ui| {
                let title = info.cart_name.as_deref().unwrap_or(if info.rom_loaded {
                    "<unknown cart>"
                } else {
                    "no ROM"
                });
                ui.label(title);
                ui.separator();
                ui.label(format!("{:?}", info.region));
                ui.separator();
                ui.label(format!("{:.1} fps", info.fps));
                if !self.status.is_empty() {
                    ui.separator();
                    ui.label(&self.status);
                }
            });
        });

        if self.welcome_open {
            self.render_welcome(&ctx, &mut actions);
        }
        // The Settings + debugger windows float above the panels (rendered on the same ctx).
        if self.settings_open {
            self.render_settings(&ctx, cfg, info, &mut actions);
        }
        if self.debugger_open {
            self.render_debugger(&ctx, debug, watchpoints, breakpoints, &mut actions);
        }
        if self.save_states_open {
            self.render_save_states(&ctx, save_slots, &mut actions);
        }
        if self.performance_open {
            self.render_performance(&ctx, info);
        }
        #[cfg(feature = "cheats")]
        if self.cheats_open {
            self.render_cheats(&ctx, cheats);
        }
        #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
        if self.netplay_open {
            self.render_netplay(&ctx, netplay_connected, &mut actions);
        }
        #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
        if self.cheevos_open {
            self.render_cheevos(&ctx, cheevos, &mut actions);
        }

        actions
    }

    /// The tabbed Settings window (Video / Audio / Input / System). v0.1 wires the live config
    /// fields; `v1.2.0` added the CRT/HQx post-filter picker + per-filter strength sliders. Deep
    /// per-knob panels (NTSC composite simulation, per-game overrides) are still TODO.
    // Straight-line per-tab settings UI (one `match` arm per tab) reads more clearly as a single
    // unit than split across helpers, matching this file's existing precedent for similarly-large
    // straight-line UI functions.
    #[allow(clippy::too_many_lines)]
    fn render_settings(
        &mut self,
        ctx: &egui::Context,
        cfg: &mut Config,
        info: &ShellInfo,
        actions: &mut Vec<MenuAction>,
    ) {
        // Only the `hd-pack` pack-selector block below reads these.
        #[cfg(not(feature = "hd-pack"))]
        let _ = (info, actions);
        let mut open = self.settings_open;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (i, name) in ["Video", "Audio", "Input", "System"].iter().enumerate() {
                        ui.selectable_value(&mut self.settings_tab, i, *name);
                    }
                });
                ui.separator();
                match self.settings_tab {
                    0 => {
                        ui.label("Present mode:");
                        for m in ["fifo", "mailbox", "immediate"] {
                            if ui.radio(cfg.video.present_mode == m, m).clicked() {
                                cfg.video.present_mode = m.to_string();
                            }
                        }
                        ui.checkbox(&mut cfg.video.integer_scale, "Integer scale");
                        ui.separator();
                        ui.label("Post-filter (`v1.2.0`):");
                        ui.horizontal(|ui| {
                            for filter in crate::config::PostFilter::all() {
                                ui.radio_value(
                                    &mut cfg.video.filter,
                                    filter,
                                    filter.display_name(),
                                );
                            }
                        });
                        match cfg.video.filter {
                            crate::config::PostFilter::None => {}
                            crate::config::PostFilter::Crt => {
                                ui.add(
                                    egui::Slider::new(&mut cfg.video.crt_scanline, 0.0..=1.0)
                                        .text("Scanlines"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut cfg.video.crt_mask, 0.0..=1.0)
                                        .text("Aperture mask"),
                                );
                            }
                            crate::config::PostFilter::Hqx => {
                                ui.add(
                                    egui::Slider::new(&mut cfg.video.hqx_strength, 0.0..=1.0)
                                        .text("Edge blend strength"),
                                );
                            }
                            crate::config::PostFilter::Xbrz => {
                                ui.add(
                                    egui::Slider::new(&mut cfg.video.xbrz_strength, 0.0..=1.0)
                                        .text("Corner blend strength"),
                                );
                            }
                        }
                        #[cfg(feature = "hd-pack")]
                        {
                            ui.separator();
                            ui.label("HD texture pack (`v1.3.0`):");
                            if info.available_hd_packs.is_empty() {
                                ui.label(if info.rom_loaded {
                                    "(none found for this ROM)"
                                } else {
                                    "(load a ROM first)"
                                });
                            } else {
                                // A clone, not `.as_deref()`, is required here: the closure below
                                // mutates `cfg.video.hd_pack_name` itself, and `current` (used
                                // both before and after that mutation within the same closure
                                // invocation) can't remain a live borrow of it across that write
                                // -- confirmed by trying the borrowed form, which fails to
                                // compile (E0502). The clone is one small `String` per
                                // Settings-window-open frame (already gated the same way
                                // `available_hd_packs`'s own I/O is), not a hot-path cost.
                                let current = cfg.video.hd_pack_name.clone();
                                egui::ComboBox::from_id_salt("hd_pack_selector")
                                    .selected_text(current.as_deref().unwrap_or("(none)"))
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(current.is_none(), "(none)")
                                            .clicked()
                                        {
                                            cfg.video.hd_pack_name = None;
                                            actions.push(MenuAction::SetHdPack(None));
                                        }
                                        for name in &info.available_hd_packs {
                                            if ui
                                                .selectable_label(
                                                    current.as_deref() == Some(name.as_str()),
                                                    name,
                                                )
                                                .clicked()
                                            {
                                                cfg.video.hd_pack_name = Some(name.clone());
                                                actions.push(MenuAction::SetHdPack(Some(
                                                    name.clone(),
                                                )));
                                            }
                                        }
                                    });
                            }
                            if let Some(active) = &info.active_hd_pack {
                                ui.label(format!("Active: {active}"));
                            } else if cfg.video.hd_pack_name.is_some() {
                                ui.label("Selected pack failed to load — see logs.");
                            }
                            ui.label(
                                "Compositing onto the live framebuffer is not yet wired in \
                                 (docs/frontend.md §HD texture packs) — selecting a pack here \
                                 enables PPU-side tagging only.",
                            );
                        }
                    }
                    1 => {
                        ui.checkbox(&mut cfg.audio.enabled, "Audio enabled");
                        ui.add(egui::Slider::new(&mut cfg.audio.volume, 0.0..=1.0).text("Volume"));
                        ui.separator();
                        ui.label("Per-voice mute (S-DSP channels 0-7):");
                        ui.horizontal(|ui| {
                            for (i, muted) in cfg.audio.voice_mutes.iter_mut().enumerate() {
                                ui.checkbox(muted, format!("V{i}"));
                            }
                        });
                    }
                    2 => {
                        ui.label("P1 key bindings:");
                        egui::Grid::new("p1_rebind_grid")
                            .num_columns(3)
                            .striped(true)
                            .show(ui, |ui| {
                                for button in Button::ALL {
                                    ui.label(format!("{button:?}"));
                                    let bound = cfg
                                        .p1
                                        .binds
                                        .iter()
                                        .find(|(_, b)| *b == button)
                                        .map_or("(unbound)", |(name, _)| name.as_str());
                                    ui.label(bound);
                                    let listening = self.awaiting_bind == Some(button);
                                    let label = if listening {
                                        "Press a key…"
                                    } else {
                                        "Rebind"
                                    };
                                    if ui.button(label).clicked() && !listening {
                                        self.awaiting_bind = Some(button);
                                    }
                                    ui.end_row();
                                }
                            });
                        if self.awaiting_bind.is_some() {
                            ui.label("Waiting for a key press — Esc cancels.");
                        }
                        ui.separator();
                        ui.label("Controller port 2 peripheral:");
                        ui.horizontal(|ui| {
                            for (kind, name) in [
                                (crate::config::PeripheralKind::Gamepad, "Gamepad"),
                                (crate::config::PeripheralKind::Mouse, "Mouse"),
                                (crate::config::PeripheralKind::SuperScope, "Super Scope"),
                                (crate::config::PeripheralKind::Multitap, "Multitap"),
                            ] {
                                ui.radio_value(&mut cfg.port2_peripheral, kind, name);
                            }
                        });
                        if cfg.port2_peripheral != crate::config::PeripheralKind::Gamepad {
                            ui.label(
                                "Wires the emulated hardware correctly; live host-input capture \
                                 (mouse pointer, extra gamepads) is not yet built — see \
                                 docs/frontend.md §Peripherals.",
                            );
                        }
                    }
                    _ => {
                        ui.label("Region:");
                        ui.radio_value(&mut cfg.region, Region::Ntsc, "NTSC");
                        ui.radio_value(&mut cfg.region, Region::Pal, "PAL");
                        ui.separator();
                        ui.label("Theme:");
                        ui.horizontal(|ui| {
                            for theme in crate::config::AppTheme::all() {
                                ui.radio_value(&mut cfg.theme, theme, theme.display_name());
                            }
                        });
                    }
                }
            });
        self.settings_open = open;
    }

    /// The Save States manager: a [`crate::save_states::NUM_SLOTS`]-slot thumbnail grid
    /// (`v1.0.0`, disk-backed per-ROM, `save_states.rs`). `slots` is `None` until the app has
    /// built it (only while this window is open and a ROM is loaded — the app rebuilds it once
    /// per frame from disk while open, same "only build when the overlay needing it is visible"
    /// pattern the debugger snapshot already uses above), rendered as a "load a ROM" message in
    /// that case. A texture is (re-)allocated per occupied thumbnail EVERY frame this window is
    /// open rather than cached across frames — simple and correct; the thumbnails are small
    /// (`THUMB_W x THUMB_H`, at most [`crate::save_states::NUM_SLOTS`] of them) and this window
    /// is only open when a user actively summons it, not during normal gameplay.
    fn render_save_states(
        &mut self,
        ctx: &egui::Context,
        slots: Option<&[SlotMeta]>,
        actions: &mut Vec<MenuAction>,
    ) {
        let mut open = self.save_states_open;
        egui::Window::new("Save States")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                let Some(slots) = slots else {
                    ui.label("Save States: load a ROM first.");
                    return;
                };
                egui::Grid::new("save_states_grid")
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        for (i, meta) in slots.iter().enumerate() {
                            #[allow(clippy::cast_possible_truncation)]
                            let slot = i as u8;
                            ui.label(format!("Slot {slot}"));
                            if let Some((w, h, rgba)) = &meta.thumbnail {
                                let image = egui::ColorImage::from_rgba_unmultiplied(
                                    [usize::from(*w), usize::from(*h)],
                                    rgba,
                                );
                                let texture = ctx.load_texture(
                                    format!("save_slot_{slot}"),
                                    image,
                                    egui::TextureOptions::NEAREST,
                                );
                                ui.image((texture.id(), texture.size_vec2()));
                            } else {
                                ui.label("(empty)");
                            }
                            if let Some(age) = meta.modified.and_then(|m| m.elapsed().ok()) {
                                ui.label(format!("{}s ago", age.as_secs()));
                            } else {
                                ui.label(String::new());
                            }
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    actions.push(MenuAction::SaveStateSlot(slot));
                                }
                                if ui
                                    .add_enabled(meta.occupied(), egui::Button::new("Load"))
                                    .clicked()
                                {
                                    actions.push(MenuAction::LoadStateSlot(slot));
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
        self.save_states_open = open;
    }

    /// The Performance panel (`v1.0.0`): a read-only diagnostic view of `info`'s live
    /// frame-timing/audio-health fields, plus a rolling frame-time sparkline
    /// (`frame_time_history`). No controls — this is purely informational, unlike
    /// Settings/Save States.
    fn render_performance(&mut self, ctx: &egui::Context, info: &ShellInfo) {
        self.frame_time_history
            .push_back(info.frame_time_ms.unwrap_or(0.0));
        if self.frame_time_history.len() > FRAME_TIME_HISTORY_LEN {
            self.frame_time_history.pop_front();
        }

        let mut open = self.performance_open;
        egui::Window::new("Performance")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("performance_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("FPS:");
                        ui.label(format!("{:.1}", info.fps));
                        ui.end_row();

                        ui.label("Speed:");
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let pct = (info.speed * 100.0).round() as u32;
                        ui.label(format!("{pct}%"));
                        ui.end_row();

                        ui.label("Frame time:");
                        ui.label(
                            info.frame_time_ms
                                .map_or_else(|| "n/a".to_string(), |ms| format!("{ms:.2} ms")),
                        );
                        ui.end_row();

                        ui.label("Audio ring:");
                        ui.label(
                            info.audio_health_pct
                                .map_or_else(|| "n/a".to_string(), |pct| format!("{pct:.0}%")),
                        );
                        ui.end_row();
                    });
                ui.separator();
                ui.label("Frame time history (last ~2s):");
                Self::draw_frame_time_sparkline(ui, &self.frame_time_history);
            });
        self.performance_open = open;
    }

    /// Draw `history` as a simple line sparkline in a fixed-size box, scaled to its own max
    /// (never below `1.0` ms, so a silent/all-zero history — paused, or `emu-thread` — draws a
    /// flat line at the bottom rather than dividing by ~0). No `egui_plot` dependency: a
    /// sparkline is exactly `Painter::line` over a handful of points, not worth a new crate.
    fn draw_frame_time_sparkline(ui: &mut egui::Ui, history: &std::collections::VecDeque<f32>) {
        let desired_size = egui::vec2(ui.available_width().min(240.0), 40.0);
        let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
        if history.len() < 2 {
            return;
        }
        #[allow(clippy::cast_precision_loss)]
        let max = history.iter().copied().fold(1.0_f32, f32::max);
        #[allow(clippy::cast_precision_loss)]
        let last_idx = (history.len() - 1) as f32;
        let points: Vec<egui::Pos2> = history
            .iter()
            .enumerate()
            .map(|(i, &ms)| {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f32 / last_idx;
                let x = rect.left() + t * rect.width();
                let y = (ms / max)
                    .clamp(0.0, 1.0)
                    .mul_add(-rect.height(), rect.bottom());
                egui::pos2(x, y)
            })
            .collect();
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
        ));
    }

    /// The first-run welcome modal (`v1.0.0`) — a brief orientation shown once on the very first
    /// launch (`Active::welcome_open` starts `true` only when `config.first_run_seen` was
    /// `false`). No title-bar close button (`.collapsible(false)`, no `.open()`): the only way
    /// out is the explicit "Get Started" button, which pushes [`MenuAction::DismissWelcome`] for
    /// the app to persist — this window never re-derives its own visibility from `self` alone.
    fn render_welcome(&mut self, ctx: &egui::Context, actions: &mut Vec<MenuAction>) {
        egui::Window::new("Welcome to RustySNES")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("A cycle-accurate Super Nintendo / Super Famicom emulator.");
                ui.separator();
                ui.label("File → Open ROM… to start playing.");
                ui.label("Emulation → Save States… for the thumbnail-previewed save-slot grid.");
                ui.label("Settings… for input rebinding, theme, and region.");
                ui.separator();
                if ui.button("Get Started").clicked() {
                    self.welcome_open = false;
                    actions.push(MenuAction::DismissWelcome);
                }
            });
    }

    /// The Cheats window (`v0.8.0` T-81-003): an "add a code" text entry (Game Genie `XXXX-XXXX`
    /// or Pro Action Replay's 8 hex digits) plus the current entry list, each with an
    /// enable/disable checkbox, its decoded `address=value`, and a remove button. Mutates
    /// `cheats` directly (the same pattern `render_settings` already uses for `cfg`) — this list
    /// lives in `Active`, not behind the emu lock, so mutating it here doesn't violate the
    /// shell's never-touch-the-emu-lock rule.
    #[cfg(feature = "cheats")]
    fn render_cheats(&mut self, ctx: &egui::Context, cheats: &mut Vec<CheatEntry>) {
        let mut open = self.cheats_open;
        egui::Window::new("Cheats")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.cheat_code_input);
                    if ui.button("Add").clicked() {
                        match CheatEntry::parse(&self.cheat_code_input) {
                            Ok(entry) => {
                                cheats.push(entry);
                                self.cheat_code_input.clear();
                                self.cheat_code_error = None;
                            }
                            Err(e) => self.cheat_code_error = Some(e.to_string()),
                        }
                    }
                });
                if let Some(err) = &self.cheat_code_error {
                    ui.colored_label(egui::Color32::RED, err);
                }
                ui.separator();
                let mut remove = None;
                egui::Grid::new("cheat_list").num_columns(4).show(ui, |ui| {
                    for (i, entry) in cheats.iter_mut().enumerate() {
                        // Every row's checkbox/button shares the same label ("" / "Remove") —
                        // without a per-row id scope, egui collides their widget IDs, causing
                        // clicks/toggles on one row to affect another.
                        ui.push_id(i, |ui| {
                            ui.checkbox(&mut entry.enabled, "");
                            ui.label(&entry.code);
                            ui.label(format!(
                                "${:06X}={:02X}",
                                entry.patch.address, entry.patch.value
                            ));
                            if ui.button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                        ui.end_row();
                    }
                });
                if let Some(i) = remove {
                    cheats.remove(i);
                }
            });
        self.cheats_open = open;
    }

    /// The Netplay window (`v0.8.0` T-82-002): local/peer `host:port` text entry, a P1/P2 slot
    /// picker, and a Connect/Disconnect button. Doesn't perform the actual socket I/O itself
    /// (that needs the currently-loaded ROM's bytes under the emu lock, and this function NEVER
    /// touches the emu lock) — it only edits the text-entry fields directly and pushes
    /// [`MenuAction::ConnectNetplay`]/[`MenuAction::DisconnectNetplay`] for `App::dispatch_actions`
    /// to actually act on afterward, same as every other I/O-performing menu action.
    #[cfg(all(feature = "netplay", not(target_arch = "wasm32")))]
    fn render_netplay(
        &mut self,
        ctx: &egui::Context,
        connected: bool,
        actions: &mut Vec<MenuAction>,
    ) {
        let mut open = self.netplay_open;
        egui::Window::new("Netplay")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!connected, |ui| {
                    egui::Grid::new("netplay_fields")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Local address");
                            ui.text_edit_singleline(&mut self.netplay_local_addr);
                            ui.end_row();
                            ui.label("Peer address");
                            ui.text_edit_singleline(&mut self.netplay_peer_addr);
                            ui.end_row();
                            ui.label("Player slot");
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut self.netplay_local_player, 0, "P1");
                                ui.radio_value(&mut self.netplay_local_player, 1, "P2");
                            });
                            ui.end_row();
                        });
                });
                ui.separator();
                if connected {
                    ui.label("Connected.");
                    if ui.button("Disconnect").clicked() {
                        actions.push(MenuAction::DisconnectNetplay);
                    }
                } else {
                    if ui.button("Connect").clicked() {
                        actions.push(MenuAction::ConnectNetplay);
                    }
                    if let Some(err) = &self.netplay_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                }
            });
        self.netplay_open = open;
    }

    /// The `RetroAchievements` window (`v0.8.0` T-82-003): username/password entry + a Log
    /// in/Log out button. Doesn't touch `RaClient` itself (that needs the frontend-owned
    /// `CheevosState`, and this function NEVER touches the emu lock or any other app state) — it
    /// only edits the text-entry fields directly and pushes
    /// [`MenuAction::LoginCheevos`]/[`MenuAction::LogoutCheevos`] for `App::dispatch_actions` to
    /// act on afterward, same as every other I/O-performing menu action.
    #[cfg(all(feature = "retroachievements", not(target_arch = "wasm32")))]
    fn render_cheevos(
        &mut self,
        ctx: &egui::Context,
        status: &CheevosStatus<'_>,
        actions: &mut Vec<MenuAction>,
    ) {
        let mut open = self.cheevos_open;
        egui::Window::new("RetroAchievements")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                if status.logged_in {
                    ui.label(format!(
                        "Logged in as {}",
                        status.display_name.unwrap_or("?")
                    ));
                    if ui.button("Log out").clicked() {
                        actions.push(MenuAction::LogoutCheevos);
                    }
                } else {
                    ui.add_enabled_ui(!status.pending, |ui| {
                        egui::Grid::new("cheevos_fields")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Username");
                                ui.text_edit_singleline(&mut self.cheevos_username);
                                ui.end_row();
                                ui.label("Password");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.cheevos_password)
                                        .password(true),
                                );
                                ui.end_row();
                            });
                    });
                    ui.separator();
                    if status.pending {
                        ui.label("Logging in…");
                    } else if ui.button("Log in").clicked() {
                        actions.push(MenuAction::LoginCheevos);
                    }
                    if let Some(err) = status.error {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                }
            });
        self.cheevos_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_action_equality() {
        assert_eq!(MenuAction::OpenRom, MenuAction::OpenRom);
        assert_ne!(MenuAction::OpenRom, MenuAction::Quit);
        assert_eq!(
            MenuAction::SetRegion(Region::Pal),
            MenuAction::SetRegion(Region::Pal)
        );
    }

    #[test]
    fn shell_state_defaults_closed() {
        let s = ShellState::default();
        assert!(!s.debugger_open);
        assert!(!s.settings_open);
        assert_eq!(s.panel, DebugPanel::Cpu);
    }
}
