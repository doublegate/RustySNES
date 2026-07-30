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
// Each parameter is an independent piece of panel state; bundling them behind one struct would
// only move the same fields behind another name.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub(super) fn render(
    ui: &mut egui::Ui,
    debug: &DebugSnapshot,
    breakpoints: &mut Vec<crate::emu::Breakpoint>,
    bp_addr_input: &mut String,
    bp_cond_input: &mut String,
    bp_addr_error: &mut Option<String>,
    asm_input: &mut String,
    asm_status: &mut Option<String>,
    symbols: Option<&crate::symbols::SymbolMap>,
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
        // The condition (`v1.25.0`, T-FP-C2). A plain address breakpoint answers "did execution
        // reach here", which is the wrong question for a routine called thousands of times a frame.
        ui.label("if:");
        ui.add(
            egui::TextEdit::singleline(bp_cond_input)
                .desired_width(160.0)
                .hint_text("a > $80  (optional)"),
        );
        if ui.button("Add").clicked() {
            add_breakpoint(breakpoints, bp_addr_input, bp_cond_input, bp_addr_error);
        }
    });
    if let Some(err) = bp_addr_error {
        ui.colored_label(egui::Color32::RED, err);
    }
    let mut remove = None;
    egui::Grid::new("breakpoint_list")
        .num_columns(2)
        .show(ui, |ui| {
            for (i, bp) in breakpoints.iter().enumerate() {
                ui.push_id(i, |ui| {
                    let text = bp.condition.as_ref().map_or_else(
                        || format!("${:06X}", bp.address),
                        |_| format!("${:06X}  if <condition>", bp.address),
                    );
                    ui.monospace(text);
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                    }
                });
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        let _ = breakpoints.remove(i);
    }

    ui.separator();
    ui.label("Disassembly:");
    egui::ScrollArea::vertical()
        .max_height(220.0)
        .show(ui, |ui| {
            let pbr_pc = (u32::from(r.pbr) << 16) | u32::from(r.pc);
            for (addr, text) in &debug.disassembly {
                let marker = if *addr == pbr_pc { ">" } else { " " };
                let bp_marker = if breakpoints.iter().any(|b| b.address == *addr) {
                    "*"
                } else {
                    " "
                };
                // A symbol turns `$009A3C` into `update_sprites+7`, which is most of what makes a
                // disassembly readable for a ROM someone has mapped (`v1.25.0`, T-FP-C2).
                let label = symbols
                    .filter(|s| !s.is_empty())
                    .map_or_else(|| format!("{addr:06X}"), |s| s.label(*addr, SYMBOL_SPAN));
                ui.monospace(format!("{marker}{bp_marker}{label:>16}  {text}"));
            }
        });

    ui.separator();
    assembler(ui, debug, asm_input, asm_status, actions);
}

/// How far past a symbol an address is still named as `symbol+off` (`v1.25.0`, T-FP-C2).
///
/// One 64 KiB bank would let a single symbol claim everything after it; 4 KiB is longer than any
/// realistic routine and short enough that a gap in the map reads as a gap.
const SYMBOL_SPAN: u32 = 0x1000;

/// Parse and arm a breakpoint from the address + condition entries.
fn add_breakpoint(
    breakpoints: &mut Vec<crate::emu::Breakpoint>,
    addr_input: &mut String,
    cond_input: &mut String,
    error: &mut Option<String>,
) {
    let trimmed = addr_input.trim().trim_start_matches('$');
    let address = match u32::from_str_radix(trimmed, 16) {
        Ok(a) if a <= 0x00FF_FFFF => a,
        Ok(_) => {
            *error = Some("address must fit the 24-bit CPU bus ($000000-$FFFFFF)".into());
            return;
        }
        Err(e) => {
            *error = Some(e.to_string());
            return;
        }
    };
    // A condition that does not parse must NOT arm silently — an unparsed condition is a
    // breakpoint that means something other than what it reads as.
    let condition = if cond_input.trim().is_empty() {
        None
    } else {
        match crate::expr::Expr::parse(cond_input) {
            Ok(e) => Some(e),
            Err(e) => {
                *error = Some(format!("condition: {e}"));
                return;
            }
        }
    };
    if !breakpoints
        .iter()
        .any(|b| b.address == address && b.condition == condition)
    {
        breakpoints.push(crate::emu::Breakpoint { address, condition });
    }
    addr_input.clear();
    cond_input.clear();
    *error = None;
}

/// The inline one-line assembler (`v1.25.0`, T-FP-C2).
///
/// Assembles against the CPU's **current** `PBR:PC` and `M`/`X` widths, because both change the
/// encoding — a branch operand is PC-relative and an immediate's width follows the flags. Patches
/// are emitted as `MenuAction::PokeBytes` and applied under the emu lock like every other edit;
/// this panel never reaches it.
fn assembler(
    ui: &mut egui::Ui,
    debug: &DebugSnapshot,
    asm_input: &mut String,
    asm_status: &mut Option<String>,
    actions: &mut Vec<MenuAction>,
) {
    let r = &debug.cpu;
    let pbr_pc = (u32::from(r.pbr) << 16) | u32::from(r.pc);
    ui.label(format!("Assemble at ${pbr_pc:06X}:"));
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(asm_input)
                .desired_width(200.0)
                .hint_text("lda #$12"),
        );
        if ui.button("Preview").clicked() {
            *asm_status = Some(preview(asm_input, debug));
        }
        if ui.button("Patch").clicked() {
            match assemble_here(asm_input, debug) {
                Ok(bytes) => {
                    *asm_status = Some(format!("Patched {} byte(s)", bytes.len()));
                    actions.push(MenuAction::PokeBytes(pbr_pc, bytes));
                }
                Err(e) => *asm_status = Some(e.to_string()),
            }
        }
    });
    if let Some(status) = asm_status.as_ref() {
        ui.label(egui::RichText::new(status).small().weak());
    }
    ui.label(
        egui::RichText::new(
            "Patches reach WRAM only, so a ROM instruction cannot be edited in place \u{2014} the same \
             restriction the Memory editor states.",
        )
        .small()
        .weak(),
    );
}

/// Assemble `line` at the CPU's current position and widths.
fn assemble_here(line: &str, debug: &DebugSnapshot) -> Result<Vec<u8>, crate::asm65816::AsmError> {
    use rustysnes_core::cpu::Status;
    let r = &debug.cpu;
    crate::asm65816::assemble(
        line,
        r.pbr,
        r.pc,
        r.p.contains(Status::M),
        r.p.contains(Status::X),
    )
}

/// A human-readable preview of what a line assembles to.
fn preview(line: &str, debug: &DebugSnapshot) -> String {
    match assemble_here(line, debug) {
        Ok(bytes) => {
            let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
            format!("{} ({} bytes)", hex.join(" "), bytes.len())
        }
        Err(e) => e.to_string(),
    }
}
