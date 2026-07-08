//! The always-on egui shell: the menu bar (File / Emulation / Tools / View / Debug / Help), the
//! status bar, the tabbed Settings window, and the toggleable debugger-overlay scaffold.
//!
//! THE NON-NEGOTIABLE RULE (RustyNES `docs/frontend.md`): egui runs **every frame**, and the
//! shell NEVER holds the emu lock inside the egui closure. Menu interactions return a
//! [`MenuAction`]; the app dispatches it *after* the egui pass. The debugger panels are SNES
//! stubs (65C816 / PPU1+PPU2 / SPC700+S-DSP / cart-coprocessor) — TODO bodies, not real
//! register read-outs, until the chip models land.

use crate::config::{Config, Region};

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
                    // TODO(impl-phase): NSF/SPC player, cheat editor, ROM-DB editor, TAStudio.
                    ui.label("(tools — TODO)");
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut cfg.video.integer_scale, "Integer scale");
                    // TODO(impl-phase): fullscreen toggle, shader/filter picklist, overscan.
                });

                ui.menu_button("Debug", |ui| {
                    if ui
                        .checkbox(&mut self.debugger_open, "Debugger overlay")
                        .clicked()
                    {
                        ui.close();
                    }
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
            self.render_debugger(&ctx);
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

    /// The debugger overlay: a panel selector + the SNES chip-panel stubs.
    fn render_debugger(&mut self, ctx: &egui::Context) {
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
                match self.panel {
                    // TODO(impl-phase): each panel reads the live chip state (copied out under
                    // the brief emu lock, never read inside this egui closure) and renders the
                    // register grid + disassembly / viewers.
                    DebugPanel::Cpu => {
                        ui.label("65C816 — registers (A/X/Y 8/16-bit, D/DBR/PBR/S/P, E latch),");
                        ui.label("disassembly, breakpoints. TODO(impl-phase).");
                    }
                    DebugPanel::Ppu => {
                        ui.label("PPU1 (5C77) + PPU2 (5C78) — BG modes 0-7 + Mode 7 affine,");
                        ui.label("OAM/CGRAM/VRAM viewers, the dot/scanline timeline. TODO.");
                    }
                    DebugPanel::Apu => {
                        ui.label("SPC700 + S-DSP — the 2nd clock domain, 8 BRR voices, ARAM,");
                        ui.label("the $2140-$2143 port handshake. TODO(impl-phase).");
                    }
                    DebugPanel::Cart => {
                        ui.label("Cart — LoROM/HiROM/ExHiROM map + coprocessor (DSP-1..4 /");
                        ui.label("Super FX / SA-1 / S-DD1 / SPC7110 / CX4 / OBC1). TODO.");
                    }
                }
            });
        self.debugger_open = open;
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
