//! The APU debugger panel: SPC700 (S-SMP) PC + halt state, and the 8 S-DSP voices' key
//! registers. Extracted from `ui_shell.rs` verbatim (`v1.7.0 "Telemetry"`).

use crate::debug_snapshot::DebugSnapshot;

/// Render the SMP's own PC/halt state and the 8 voices' key registers.
pub(super) fn render(ui: &mut egui::Ui, debug: &DebugSnapshot) {
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
