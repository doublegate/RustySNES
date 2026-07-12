//! The 65C816 debugger panel: registers, PC breakpoints, step controls, and a disassembly window
//! around the current PC (`v0.9.0`, T-81-001 PR B — the disassembly/breakpoints/stepping half of
//! the ticket; PR A landed the live-state register view alone). Extracted from `ui_shell.rs`
//! verbatim (`v1.7.0 "Telemetry"`).

use crate::debug_snapshot::DebugSnapshot;
use crate::ui_shell::MenuAction;

/// Render the 65C816 panel's registers, breakpoints, step controls, and disassembly view.
// One straight-line immediate-mode egui pass (registers + step controls + breakpoint list +
// disassembly view); same "reads more clearly as a unit" reasoning as `ShellState::render`'s own
// `too_many_lines` allow.
#[allow(clippy::too_many_lines)]
pub(super) fn render(
    ui: &mut egui::Ui,
    debug: &DebugSnapshot,
    breakpoints: &mut Vec<u32>,
    bp_addr_input: &mut String,
    bp_addr_error: &mut Option<String>,
    actions: &mut Vec<MenuAction>,
) {
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

    ui.separator();
    ui.horizontal(|ui| {
        if debug.paused {
            if ui.button("Continue").clicked() {
                actions.push(MenuAction::DebuggerContinue);
            }
            if ui.button("Step Into").clicked() {
                actions.push(MenuAction::DebuggerStepInto);
            }
            if ui.button("Step Over").clicked() {
                actions.push(MenuAction::DebuggerStepOver);
            }
        } else if ui.button("Pause").clicked() {
            actions.push(MenuAction::DebuggerPause);
        }
    });

    ui.separator();
    ui.label("Breakpoints (PC):");
    ui.horizontal(|ui| {
        ui.label("Address ($bank:offset):");
        ui.add(egui::TextEdit::singleline(bp_addr_input).desired_width(80.0));
        if ui.button("Add").clicked() {
            let trimmed = bp_addr_input.trim().trim_start_matches('$');
            match u32::from_str_radix(trimmed, 16) {
                Ok(address) if address <= 0x00FF_FFFF => {
                    if !breakpoints.contains(&address) {
                        breakpoints.push(address);
                    }
                    bp_addr_input.clear();
                    *bp_addr_error = None;
                }
                Ok(_) => {
                    *bp_addr_error =
                        Some("address must fit the 24-bit CPU bus ($000000-$FFFFFF)".into());
                }
                Err(e) => *bp_addr_error = Some(e.to_string()),
            }
        }
    });
    if let Some(err) = bp_addr_error {
        ui.colored_label(egui::Color32::RED, err);
    }
    let mut remove = None;
    egui::Grid::new("breakpoint_list")
        .num_columns(2)
        .show(ui, |ui| {
            for (i, addr) in breakpoints.iter().enumerate() {
                ui.push_id(i, |ui| {
                    ui.label(format!("${addr:06X}"));
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                    }
                });
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        breakpoints.remove(i);
    }

    ui.separator();
    ui.label("Disassembly:");
    egui::ScrollArea::vertical()
        .max_height(220.0)
        .show(ui, |ui| {
            let pbr_pc = (u32::from(r.pbr) << 16) | u32::from(r.pc);
            for (addr, text) in &debug.disassembly {
                let marker = if *addr == pbr_pc { ">" } else { " " };
                let bp_marker = if breakpoints.contains(addr) { "*" } else { " " };
                ui.monospace(format!("{marker}{bp_marker}{addr:06X}  {text}"));
            }
        });
}
