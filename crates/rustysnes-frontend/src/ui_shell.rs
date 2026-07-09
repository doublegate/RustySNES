//! The always-on egui shell: the menu bar (File / Emulation / Tools / View / Debug / Help), the
//! status bar, the tabbed Settings window, and the debugger overlay.
//!
//! THE NON-NEGOTIABLE RULE (RustyNES `docs/frontend.md`): egui runs **every frame**, and the
//! shell NEVER holds the emu lock inside the egui closure. Menu interactions return a
//! [`MenuAction`]; the app dispatches it *after* the egui pass. The debugger's 4 panels
//! (65C816 / PPU1+PPU2 / SPC700+S-DSP / cart-coprocessor) render the [`DebugSnapshot`] the app
//! copies out under the same brief lock [`ShellInfo`] already uses — never touched from inside
//! this module. The Debug menu entry that opens the overlay is gated behind the `debug-hooks`
//! feature (default off): without it, `debugger_open` can never become `true`, so the app never
//! builds a snapshot and the debugger is unreachable in a shipped default build.

// `ShellState`'s bools are each an independent, feature-gated window-visibility/UI-transient
// flag (debugger/settings/cheats open, paused) — a state machine would only obscure that they're
// orthogonal, not a single mode to switch between.
#![allow(clippy::struct_excessive_bools)]

#[cfg(feature = "cheats")]
use crate::cheats::CheatEntry;
use crate::config::{Config, Region};
use crate::debug_snapshot::DebugSnapshot;

/// An action requested from the egui pass, dispatched by `App::dispatch_menu_action` AFTER the
/// pass returns (so it never runs while the emu lock is held inside the egui closure).
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Switch the console region (NTSC/PAL).
    SetRegion(Region),
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
}

/// Which debugger panel is selected in the overlay (SNES chip set).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DebugPanel {
    /// 65C816 main CPU registers + disassembly.
    #[default]
    Cpu,
    /// PPU1 (5C77) + PPU2 (5C78) video registers + the BG/OBJ/Mode-7 viewers.
    Ppu,
    /// SPC700 + S-DSP audio (the second clock domain) + the 8 BRR voices.
    Apu,
    /// The cart memory map + any on-cart coprocessor (DSP-1..4 / Super FX / SA-1 / S-DD1 / …).
    Cart,
}

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
    #[allow(clippy::too_many_lines)]
    pub fn render(
        &mut self,
        root_ui: &mut egui::Ui,
        info: &ShellInfo,
        cfg: &mut Config,
        debug: Option<&DebugSnapshot>,
        #[cfg(feature = "cheats")] cheats: &mut Vec<CheatEntry>,
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
                    // TODO(impl-phase): NSF/SPC player, ROM-DB editor, TAStudio.
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut cfg.video.integer_scale, "Integer scale");
                    // TODO(impl-phase): fullscreen toggle, shader/filter picklist, overscan.
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
                    ui.label("RustySNES v0.1.0 (scaffold)");
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

        // The Settings + debugger windows float above the panels (rendered on the same ctx).
        if self.settings_open {
            self.render_settings(&ctx, cfg);
        }
        if self.debugger_open {
            self.render_debugger(&ctx, debug);
        }
        #[cfg(feature = "cheats")]
        if self.cheats_open {
            self.render_cheats(&ctx, cheats);
        }

        actions
    }

    /// The tabbed Settings window (Video / Audio / Input / System). v0.1 wires the live config
    /// fields; deep per-knob panels (NTSC, shader stack, per-game overrides) are TODO.
    fn render_settings(&mut self, ctx: &egui::Context, cfg: &mut Config) {
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
                    }
                    1 => {
                        ui.checkbox(&mut cfg.audio.enabled, "Audio enabled");
                        ui.add(egui::Slider::new(&mut cfg.audio.volume, 0.0..=1.0).text("Volume"));
                    }
                    2 => {
                        // TODO(impl-phase): the SNES key-rebind grid (12 buttons * 2 players).
                        ui.label("Input rebinding — TODO (defaults in input.rs).");
                    }
                    _ => {
                        ui.label("Region:");
                        ui.radio_value(&mut cfg.region, Region::Ntsc, "NTSC");
                        ui.radio_value(&mut cfg.region, Region::Pal, "PAL");
                    }
                }
            });
        self.settings_open = open;
    }

    /// The debugger overlay: a panel selector + the SNES chip-panel live state viewers. `debug`
    /// is `None` only when the debugger opens before the app's next lock-scope has built a
    /// snapshot yet — every panel handles that by showing "no data yet" rather than assuming
    /// the app has already supplied one.
    fn render_debugger(&mut self, ctx: &egui::Context, debug: Option<&DebugSnapshot>) {
        let mut open = self.debugger_open;
        egui::Window::new("Debugger")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.panel, DebugPanel::Cpu, "65C816");
                    ui.selectable_value(&mut self.panel, DebugPanel::Ppu, "PPU1+2");
                    ui.selectable_value(&mut self.panel, DebugPanel::Apu, "SPC700+DSP");
                    ui.selectable_value(&mut self.panel, DebugPanel::Cart, "Cart");
                });
                ui.separator();
                let Some(debug) = debug else {
                    // `debug` tracks `debugger_open`, not ROM state (a snapshot builds fine for a
                    // blank core) — don't claim a ROM-load reason that may not be why it's `None`.
                    ui.label("(no debugger snapshot yet)");
                    return;
                };
                match self.panel {
                    DebugPanel::Cpu => render_cpu_panel(ui, debug),
                    DebugPanel::Ppu => render_ppu_panel(ui, debug),
                    DebugPanel::Apu => render_apu_panel(ui, debug),
                    DebugPanel::Cart => render_cart_panel(ui, debug),
                }
            });
        self.debugger_open = open;
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
                        ui.checkbox(&mut entry.enabled, "");
                        ui.label(&entry.code);
                        ui.label(format!(
                            "${:06X}={:02X}",
                            entry.patch.address, entry.patch.value
                        ));
                        if ui.button("Remove").clicked() {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
                if let Some(i) = remove {
                    cheats.remove(i);
                }
            });
        self.cheats_open = open;
    }
}

/// 65C816 registers + processor-status flags. Disassembly + breakpoints/stepping are a
/// follow-up (T-81-006) — this panel is the live-state half of the ticket.
fn render_cpu_panel(ui: &mut egui::Ui, debug: &DebugSnapshot) {
    let r = &debug.cpu;
    egui::Grid::new("cpu_regs").num_columns(2).show(ui, |ui| {
        ui.label("A");
        ui.label(format!("{:04X}", r.a));
        ui.end_row();
        ui.label("X");
        ui.label(format!("{:04X}", r.x));
        ui.end_row();
        ui.label("Y");
        ui.label(format!("{:04X}", r.y));
        ui.end_row();
        ui.label("S");
        ui.label(format!("{:04X}", r.s));
        ui.end_row();
        ui.label("D");
        ui.label(format!("{:04X}", r.d));
        ui.end_row();
        ui.label("DBR");
        ui.label(format!("{:02X}", r.dbr));
        ui.end_row();
        ui.label("PBR");
        ui.label(format!("{:02X}", r.pbr));
        ui.end_row();
        ui.label("PC");
        ui.label(format!("{:04X}", r.pc));
        ui.end_row();
        ui.label("P");
        ui.label(format!("{:?}", r.p));
        ui.end_row();
        ui.label("E (emulation)");
        ui.label(if r.emulation { "1" } else { "0" });
        ui.end_row();
    });
}

/// Format a row of 16-bit words as space-separated 4-hex-digit groups, for the VRAM/CGRAM hex
/// dumps. A plain loop, not `.map(...).collect::<String>()`, since collecting a `String` from a
/// `format!`-per-item iterator reallocates on every item (`clippy::format_collect`).
fn hex_row(words: &[u16]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(words.len() * 5);
    for w in words {
        let _ = write!(out, "{w:04X} ");
    }
    out
}

/// Key PPU registers + the dot/scanline timeline + CGRAM/a scrollable VRAM window.
///
/// # Panics
/// Never in practice: `VRAM_WINDOW_LEN` (1024) and every `row * 8` byte offset within it fit
/// comfortably in a `u16`, so the narrowing casts below can't actually truncate.
#[allow(clippy::cast_possible_truncation)]
fn render_ppu_panel(ui: &mut egui::Ui, debug: &DebugSnapshot) {
    let p = &debug.ppu;
    egui::Grid::new("ppu_regs").num_columns(2).show(ui, |ui| {
        ui.label("BGMODE");
        ui.label(p.bg_mode.to_string());
        ui.end_row();
        ui.label("Brightness");
        ui.label(p.display_brightness.to_string());
        ui.end_row();
        ui.label("Hi-res");
        ui.label(if p.is_hires { "yes (512-wide)" } else { "no" });
        ui.end_row();
        ui.label("Scanline / dot");
        ui.label(format!("{} / {}", p.scanline, p.dot));
        ui.end_row();
        ui.label("VBlank / HBlank");
        ui.label(format!(
            "{} / {}",
            if p.in_vblank { "yes" } else { "no" },
            if p.in_hblank { "yes" } else { "no" }
        ));
        ui.end_row();
    });
    ui.separator();
    ui.label(format!(
        "VRAM window (words {:04X}-{:04X}):",
        p.vram_window_start,
        p.vram_window_start
            .wrapping_add(crate::debug_snapshot::VRAM_WINDOW_LEN as u16 - 1)
    ));
    egui::ScrollArea::vertical()
        .max_height(160.0)
        .id_salt("vram_scroll")
        .show(ui, |ui| {
            for (row, chunk) in p.vram_window.chunks(8).enumerate() {
                let addr = p.vram_window_start.wrapping_add((row * 8) as u16);
                ui.monospace(format!("{addr:04X}: {}", hex_row(chunk)));
            }
        });
    ui.separator();
    ui.label("CGRAM (256 colors):");
    egui::ScrollArea::vertical()
        .max_height(100.0)
        .id_salt("cgram_scroll")
        .show(ui, |ui| {
            for (row, chunk) in p.cgram.chunks(8).enumerate() {
                ui.monospace(format!("{:02X}: {}", row * 8, hex_row(chunk)));
            }
        });
}

/// SPC700 + S-DSP: the SMP's own PC + halt state, and the 8 voices' key registers.
fn render_apu_panel(ui: &mut egui::Ui, debug: &DebugSnapshot) {
    let a = &debug.apu;
    ui.label(format!(
        "SMP PC: {:04X}  (stopped: {})",
        a.smp_pc,
        if a.smp_stopped { "yes" } else { "no" }
    ));
    ui.separator();
    egui::Grid::new("dsp_voices").num_columns(8).show(ui, |ui| {
        for h in [
            "V", "VOL L/R", "PITCH", "SRCN", "ADSR", "GAIN", "ENVX", "OUTX",
        ] {
            ui.strong(h);
        }
        ui.end_row();
        for (i, v) in a.voices.iter().enumerate() {
            ui.label(i.to_string());
            ui.label(format!("{}/{}", v.vol.0, v.vol.1));
            ui.label(format!("{:04X}", v.pitch));
            ui.label(format!("{:02X}", v.srcn));
            ui.label(format!("{:02X}/{:02X}", v.adsr.0, v.adsr.1));
            ui.label(format!("{:02X}", v.gain));
            ui.label(format!("{:02X}", v.envx));
            ui.label(format!("{:02X}", v.outx));
            ui.end_row();
        }
    });
}

/// The active board + (when present) a Core/Curated coprocessor's own register state — SA-1's
/// second-CPU regs or the Super FX/GSU register file, resolving `docs/frontend.md`'s open
/// question in the breadth-inclusive direction this whole ladder takes.
fn render_cart_panel(ui: &mut egui::Ui, debug: &DebugSnapshot) {
    let c = &debug.cart;
    ui.label(format!("Board: {}", c.board_name.unwrap_or("(no cart)")));
    if let Some(r) = c.sa1 {
        ui.separator();
        ui.label("SA-1 second CPU:");
        egui::Grid::new("sa1_regs").num_columns(2).show(ui, |ui| {
            ui.label("A");
            ui.label(format!("{:04X}", r.a));
            ui.end_row();
            ui.label("X");
            ui.label(format!("{:04X}", r.x));
            ui.end_row();
            ui.label("Y");
            ui.label(format!("{:04X}", r.y));
            ui.end_row();
            ui.label("PC");
            ui.label(format!("{:02X}:{:04X}", r.pbr, r.pc));
            ui.end_row();
            ui.label("P");
            ui.label(format!("{:?}", r.p));
            ui.end_row();
        });
    }
    if let Some(g) = c.gsu {
        ui.separator();
        ui.label("Super FX / GSU:");
        egui::Grid::new("gsu_regs").num_columns(2).show(ui, |ui| {
            for (i, chunk) in g.r.chunks(4).enumerate() {
                ui.label(format!("R{}-R{}", i * 4, i * 4 + 3));
                ui.monospace(hex_row(chunk));
                ui.end_row();
            }
            ui.label("SFR");
            ui.label(format!("{:04X}", g.sfr));
            ui.end_row();
            ui.label("PBR");
            ui.label(format!("{:02X}", g.pbr));
            ui.end_row();
        });
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
