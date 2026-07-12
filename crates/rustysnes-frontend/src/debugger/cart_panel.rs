//! The Cart debugger panel: the active board + (when present) a Core/Curated coprocessor's own
//! register state — SA-1's second-CPU regs or the Super FX/GSU register file. Extracted from
//! `ui_shell.rs` verbatim (`v1.7.0 "Telemetry"`).

use crate::debug_snapshot::DebugSnapshot;
use crate::debugger::hex_row;

/// Render the active board name and any Core/Curated coprocessor's live register state.
pub(super) fn render(ui: &mut egui::Ui, debug: &DebugSnapshot) {
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
