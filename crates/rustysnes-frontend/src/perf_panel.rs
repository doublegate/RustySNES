//! The Performance panel's body (`v1.25.0`, T-FP-B).
//!
//! Split out of `ui_shell.rs` for the same reason the Settings window was: one panel that renders
//! four metric groups, a pacing readout, and a log control does not belong inline in a file that
//! also owns the menu bar. `ui_shell::ShellState::render_performance` calls in here.
//!
//! Everything drawn is a snapshot ([`crate::perf::PerfReport`]) taken by the caller, so this module
//! never touches the emulator or the metric rings — it cannot perturb what it displays.

use crate::perf::{MetricReport, PerfReport};

/// A sparkline's height in points. Wide enough that a hitch is visible, short enough that four
/// stacked metrics still fit a window that doesn't need scrolling on a laptop.
const SPARK_H: f32 = 34.0;

/// What the panel's controls asked for this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PanelActions {
    /// Clear every metric and counter.
    pub reset: bool,
    /// Flip the CSV session log on/off.
    pub toggle_log: bool,
}

/// Draw the panel body. Returns which controls were activated.
pub fn render(
    ui: &mut egui::Ui,
    report: Option<&PerfReport>,
    pacing: Option<&str>,
    fps: f32,
    speed: f32,
    log_status: Option<&str>,
) -> PanelActions {
    let mut actions = PanelActions::default();

    egui::Grid::new("perf_headline")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("FPS:");
            ui.label(format!("{fps:.1}"));
            ui.end_row();
            ui.label("Speed:");
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let pct = (speed * 100.0).round() as u32;
            ui.label(format!("{pct}%"));
            ui.end_row();
            if let Some(plan) = pacing {
                ui.label("Pacing:");
                ui.label(plan);
                ui.end_row();
            }
        });

    let Some(report) = report else {
        ui.separator();
        ui.label("No measurements yet.");
        return actions;
    };

    // Produced-vs-presented. Tracked as two counters precisely because they diverge when something
    // is wrong; a single combined "frames" number would hide exactly the case worth seeing.
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(format!(
            "Emulated {} · presented {}",
            report.frames_produced, report.frames_presented
        ));
        if report.catching_up {
            ui.label(
                egui::RichText::new("catching up")
                    .color(ui.visuals().warn_fg_color)
                    .small(),
            )
            .on_hover_text(
                "More emulated frames were produced than presented — the pacer is running a \
                 catch-up burst after a stall.",
            );
        }
    });

    for metric in report.metrics() {
        ui.separator();
        metric_block(ui, metric);
    }

    // The GPU row is absent, not zeroed, when no timestamp was ever resolved — say why rather than
    // leaving a hole the user has to guess at.
    if report.gpu.is_none() {
        ui.separator();
        ui.label(egui::RichText::new(gpu_unavailable_reason()).weak());
    }

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Reset stats").clicked() {
            actions.reset = true;
        }
        let log_label = if log_status.is_some() {
            "Stop CSV log"
        } else {
            "Start CSV log"
        };
        if ui.button(log_label).clicked() {
            actions.toggle_log = true;
        }
    });
    if let Some(status) = log_status {
        ui.label(egui::RichText::new(status).weak());
    }

    actions
}

/// Why no GPU timings are being reported, stated concretely.
///
/// Three distinct causes with three distinct fixes, so a single "unavailable" would be useless.
const fn gpu_unavailable_reason() -> &'static str {
    #[cfg(not(feature = "gpu-timing"))]
    {
        "GPU timing: not compiled in (build with --features gpu-timing)"
    }
    #[cfg(feature = "gpu-timing")]
    {
        "GPU timing: this adapter does not support TIMESTAMP_QUERY"
    }
}

/// One metric: name, latest reading, percentile line, and a sparkline of the window.
fn metric_block(ui: &mut egui::Ui, m: &MetricReport) {
    ui.horizontal(|ui| {
        ui.strong(m.label);
        ui.label(m.last_text());
    });
    ui.label(egui::RichText::new(&m.summary).small().weak());
    sparkline(ui, &m.series, m.max);
}

/// Draw `series` as a line scaled to `max`, oldest sample at the left.
///
/// The y-scale is the window's own max (floored at a small positive value so an all-zero series
/// draws a flat line at the bottom instead of dividing by zero) — an absolute scale would make a
/// well-behaved 16 ms trace and a catastrophic 200 ms one look identical at opposite extremes.
/// No `egui_plot` dependency: a sparkline is a `Shape::line` over a handful of points.
fn sparkline(ui: &mut egui::Ui, series: &[f32], max: Option<f32>) {
    let size = egui::vec2(ui.available_width().min(260.0), SPARK_H);
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    if series.len() < 2 {
        return;
    }
    let scale = max.unwrap_or(1.0).max(1e-3);
    #[allow(clippy::cast_precision_loss)]
    let last_idx = (series.len() - 1) as f32;
    let points: Vec<egui::Pos2> = series
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / last_idx;
            let x = rect.left() + t * rect.width();
            // A NaN sample must not become a NaN coordinate (egui would draw nothing at all and
            // the whole trace would vanish); clamp folds it to the floor.
            let norm = if v.is_finite() {
                (v / scale).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let y = norm.mul_add(-rect.height(), rect.bottom());
            egui::pos2(x, y)
        })
        .collect();
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
    ));
}

#[cfg(test)]
mod tests {
    use super::gpu_unavailable_reason;
    use crate::perf::PerfStats;

    /// The report must present exactly the metrics that have data — GPU included only when a
    /// timestamp actually resolved, so the panel never draws a flat zero line that reads as
    /// "the GPU costs nothing".
    #[test]
    fn gpu_metric_appears_only_once_resolved() {
        let mut stats = PerfStats::new();
        stats.record_present(Some(12.0), 2.0, Some(50.0), 1);
        let report = stats.report();
        assert!(report.gpu.is_none());
        assert_eq!(report.metrics().len(), 3, "produce + present + audio");

        stats.record_gpu(1.25);
        let report = stats.report();
        assert!(report.gpu.is_some());
        assert_eq!(report.metrics().len(), 4);
        assert_eq!(report.gpu.expect("gpu").last_text(), "1.25 ms");
    }

    /// An absent GPU row must name a cause with a fix, not just say "unavailable".
    #[test]
    fn gpu_unavailable_reason_is_actionable() {
        let reason = gpu_unavailable_reason();
        assert!(reason.contains("GPU timing"));
        assert!(
            reason.contains("gpu-timing") || reason.contains("TIMESTAMP_QUERY"),
            "must name the feature or the missing capability: {reason}"
        );
    }

    /// A metric with no samples reads as absent, not as a measured zero.
    #[test]
    fn empty_metrics_report_a_dash() {
        let report = PerfStats::new().report();
        assert_eq!(report.produce.last_text(), "—");
        assert_eq!(report.produce.summary, "—");
        assert!(report.produce.series.is_empty());
        assert!(!report.catching_up);
    }
}
