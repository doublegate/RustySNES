//! The Memory Compare debugger panel (`v1.8.0 "Tracepoint"`): diff the Memory panel's current
//! [`crate::debug_snapshot::DebugSnapshot::memory_window`] against a captured baseline, to spot
//! what changed across a step/run without eyeballing two hex dumps by hand.

use crate::debug_snapshot::DebugSnapshot;
use crate::debugger::hex_row_bytes;
use crate::ui_shell::ShellState;

impl ShellState {
    /// Render the baseline capture/clear controls and, once a baseline exists, a row-by-row diff
    /// against the live memory window. Only meaningful when both snapshots start at the same
    /// address (no scroll control exists yet — same honestly-tracked gap the Memory panel itself
    /// carries) — flags a mismatch instead of showing a misleading diff.
    ///
    /// # Panics
    /// Never in practice: `MEMORY_WINDOW_LEN` (512) and every `row * 16` byte offset within it fit
    /// comfortably in a `u32`, so the narrowing cast below can't actually truncate.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn render_memory_compare_panel(
        &mut self,
        ui: &mut egui::Ui,
        debug: Option<&DebugSnapshot>,
    ) {
        let Some(debug) = debug else {
            ui.label("(no debugger snapshot yet)");
            return;
        };
        ui.horizontal(|ui| {
            if ui.button("Capture baseline").clicked() {
                self.memcmp_baseline = Some((debug.memory_window, debug.memory_window_start));
            }
            if self.memcmp_baseline.is_some() && ui.button("Clear baseline").clicked() {
                self.memcmp_baseline = None;
            }
        });
        let Some((baseline, baseline_start)) = &self.memcmp_baseline else {
            ui.label(
                "No baseline captured yet. Click \"Capture baseline\" to snapshot the current \
                 Memory panel window, then come back after stepping or running to see what \
                 changed.",
            );
            return;
        };
        if *baseline_start != debug.memory_window_start {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Baseline was captured at a different memory window start address -- \
                 re-capture for a meaningful diff.",
            );
            return;
        }
        let mut changed_rows = 0usize;
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .id_salt("memcmp_scroll")
            .show(ui, |ui| {
                for (row, (base_chunk, cur_chunk)) in baseline
                    .chunks(16)
                    .zip(debug.memory_window.chunks(16))
                    .enumerate()
                {
                    if base_chunk == cur_chunk {
                        continue;
                    }
                    changed_rows += 1;
                    let addr = baseline_start.wrapping_add((row * 16) as u32) & 0x00FF_FFFF;
                    ui.monospace(format!(
                        "{addr:06X}: {} -> {}",
                        hex_row_bytes(base_chunk),
                        hex_row_bytes(cur_chunk)
                    ));
                }
            });
        if changed_rows == 0 {
            ui.label("No differences from baseline.");
        }
    }
}
