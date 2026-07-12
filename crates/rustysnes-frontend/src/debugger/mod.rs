//! The debugger overlay: a panel selector + the SNES chip-panel live-state viewers.
//!
//! `v1.7.0 "Telemetry"`: extracted out of `ui_shell.rs` (previously 4 panels inline in that
//! file) into this dedicated module — a pure structural move, zero behavior change — as the
//! scaffold every later debugger-depth rung (`v1.8.0` onward) plugs new panels into. Follows
//! `ui_shell.rs`'s own non-negotiable rule: these functions NEVER touch the emu lock; they only
//! render the [`crate::debug_snapshot::DebugSnapshot`] the app copied out under the same brief
//! lock `ShellInfo` already uses.

mod apu_panel;
mod cart_panel;
mod cpu_panel;
mod ppu_panel;
mod watch_panel;

use crate::debug_snapshot::{DebugSnapshot, WatchpointEntry};
use crate::ui_shell::{MenuAction, ShellState};

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
    /// Read/write watchpoints (`v0.8.0` T-81-001b) + a general memory hex viewer (`v1.7.0`).
    Watch,
}

/// Format a row of 16-bit words as space-separated 4-hex-digit groups — shared by the PPU (VRAM/
/// CGRAM) and Cart (GSU registers) panels. A plain loop, not `.map(...).collect::<String>()`,
/// since collecting a `String` from a `format!`-per-item iterator reallocates on every item
/// (`clippy::format_collect`).
fn hex_row(words: &[u16]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(words.len() * 5);
    for w in words {
        let _ = write!(out, "{w:04X} ");
    }
    out
}

/// Format a row of bytes as space-separated 2-hex-digit groups — the Memory panel's hex dump
/// (`v1.7.0`). See [`hex_row`]'s own doc for why this is a plain loop.
fn hex_row_bytes(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 3);
    for b in bytes {
        let _ = write!(out, "{b:02X} ");
    }
    out
}

impl ShellState {
    /// The debugger overlay: a panel selector + the SNES chip-panel live state viewers. `debug`
    /// is `None` only when the debugger opens before the app's next lock-scope has built a
    /// snapshot yet — every panel handles that by showing "no data yet" rather than assuming
    /// the app has already supplied one.
    pub(crate) fn render_debugger(
        &mut self,
        ctx: &egui::Context,
        debug: Option<&DebugSnapshot>,
        watchpoints: &mut Vec<WatchpointEntry>,
        breakpoints: &mut Vec<u32>,
        actions: &mut Vec<MenuAction>,
    ) {
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
                    ui.selectable_value(&mut self.panel, DebugPanel::Watch, "Memory/Watch");
                });
                ui.separator();
                if self.panel == DebugPanel::Watch {
                    self.render_watch_panel(ui, debug, watchpoints);
                    return;
                }
                let Some(debug) = debug else {
                    // `debug` tracks `debugger_open`, not ROM state (a snapshot builds fine for a
                    // blank core) — don't claim a ROM-load reason that may not be why it's `None`.
                    ui.label("(no debugger snapshot yet)");
                    return;
                };
                match self.panel {
                    DebugPanel::Cpu => cpu_panel::render(
                        ui,
                        debug,
                        breakpoints,
                        &mut self.bp_addr_input,
                        &mut self.bp_addr_error,
                        actions,
                    ),
                    DebugPanel::Ppu => ppu_panel::render(ui, debug),
                    DebugPanel::Apu => apu_panel::render(ui, debug),
                    DebugPanel::Cart => cart_panel::render(ui, debug),
                    DebugPanel::Watch => unreachable!("handled above"),
                }
            });
        self.debugger_open = open;
    }
}
