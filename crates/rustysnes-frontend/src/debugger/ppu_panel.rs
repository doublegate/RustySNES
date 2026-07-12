//! The PPU debugger panel: key registers, the dot/scanline timeline, and CGRAM/a scrollable VRAM
//! window. Extracted from `ui_shell.rs` verbatim (`v1.7.0 "Telemetry"`).

use crate::debug_snapshot::DebugSnapshot;
use crate::debugger::hex_row;

/// Render the PPU panel's registers, timeline, VRAM window, and CGRAM dump.
///
/// # Panics
/// Never in practice: `VRAM_WINDOW_LEN` (1024) and every `row * 8` byte offset within it fit
/// comfortably in a `u16`, so the narrowing casts below can't actually truncate.
#[allow(clippy::cast_possible_truncation)]
pub(super) fn render(ui: &mut egui::Ui, debug: &DebugSnapshot) {
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
