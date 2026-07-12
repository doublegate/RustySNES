//! The Memory/Watch debugger panel: read/write watchpoints (`v0.8.0` T-81-001b, extracted from
//! `ui_shell.rs` verbatim in `v1.7.0 "Telemetry"`) plus a general 24-bit-bus memory hex viewer
//! (new in `v1.7.0`).

use crate::debug_snapshot::{DebugSnapshot, WatchpointEntry, WatchpointKind};
use crate::debugger::hex_row_bytes;
use crate::ui_shell::ShellState;

impl ShellState {
    /// The Memory/Watch panel: an "add a watchpoint" address entry (hex, e.g. `7E0848`) + kind
    /// picker, the armed list with remove buttons, the hit log recorded since the debugger last
    /// polled (`debug.watchpoint_hits` — empty when no snapshot is available, same "no data yet"
    /// framing the other panels use), and a read-only hex dump of `debug.memory_window`
    /// (`v1.7.0`). Mutates `watchpoints` directly, same pattern `render_cheats` already uses for
    /// its list.
    ///
    /// # Panics
    /// Never in practice: `MEMORY_WINDOW_LEN` (512) and every `row * 16` byte offset within it
    /// fit comfortably in a `u32`, so the narrowing cast below can't actually truncate.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn render_watch_panel(
        &mut self,
        ui: &mut egui::Ui,
        debug: Option<&DebugSnapshot>,
        watchpoints: &mut Vec<WatchpointEntry>,
    ) {
        ui.horizontal(|ui| {
            ui.label("Address ($):");
            ui.add(egui::TextEdit::singleline(&mut self.watch_addr_input).desired_width(80.0));
            ui.radio_value(&mut self.watch_kind_input, WatchpointKind::Read, "R");
            ui.radio_value(&mut self.watch_kind_input, WatchpointKind::Write, "W");
            ui.radio_value(&mut self.watch_kind_input, WatchpointKind::ReadWrite, "RW");
            if ui.button("Add").clicked() {
                let trimmed = self.watch_addr_input.trim().trim_start_matches('$');
                match u32::from_str_radix(trimmed, 16) {
                    Ok(address) if address <= 0x00FF_FFFF => {
                        watchpoints.push(WatchpointEntry {
                            address,
                            kind: self.watch_kind_input,
                        });
                        self.watch_addr_input.clear();
                        self.watch_addr_error = None;
                    }
                    Ok(_) => {
                        self.watch_addr_error =
                            Some("address must fit the 24-bit CPU bus ($000000-$FFFFFF)".into());
                    }
                    Err(e) => self.watch_addr_error = Some(e.to_string()),
                }
            }
        });
        if let Some(err) = &self.watch_addr_error {
            ui.colored_label(egui::Color32::RED, err);
        }
        ui.separator();
        let mut remove = None;
        egui::Grid::new("watchpoint_list")
            .num_columns(3)
            .show(ui, |ui| {
                for (i, w) in watchpoints.iter().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.label(format!("${:06X}", w.address));
                        ui.label(match w.kind {
                            WatchpointKind::Read => "R",
                            WatchpointKind::Write => "W",
                            WatchpointKind::ReadWrite => "RW",
                        });
                        if ui.button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.end_row();
                }
            });
        if let Some(i) = remove {
            watchpoints.remove(i);
        }
        ui.separator();
        ui.label("Hits since last poll:");
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .id_salt("watch_hits_scroll")
            .show(ui, |ui| match debug {
                Some(debug) if !debug.watchpoint_hits.is_empty() => {
                    for h in &debug.watchpoint_hits {
                        ui.label(format!(
                            "pc={:06X} {} ${:06X} = {:02X}",
                            h.pbr_pc,
                            if h.is_write { "W" } else { "R" },
                            h.address,
                            h.value
                        ));
                    }
                }
                _ => {
                    ui.label("(none)");
                }
            });
        ui.separator();
        ui.label("Memory (24-bit bus, read-only):");
        match debug {
            Some(debug) => {
                // Fixed at $7E0000 (WRAM bank 0) by default — no UI scroll control yet, same
                // honestly-tracked gap the PPU panel's VRAM window already carries ("no UI
                // control calls it yet"). `EmuCore::set_debug_memory_scroll` exists for a future
                // scroll control to call.
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .id_salt("memory_scroll")
                    .show(ui, |ui| {
                        for (row, chunk) in debug.memory_window.chunks(16).enumerate() {
                            let addr = debug.memory_window_start.wrapping_add((row * 16) as u32)
                                & 0x00FF_FFFF;
                            ui.monospace(format!("{addr:06X}: {}", hex_row_bytes(chunk)));
                        }
                    });
            }
            None => {
                ui.label("(no debugger snapshot yet)");
            }
        }
    }
}
