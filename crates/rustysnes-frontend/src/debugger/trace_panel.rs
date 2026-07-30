//! The trace, call-stack, event, and access-counter views (`v1.25.0`, T-FP-C2).
//!
//! T-FP-C1 landed the core-side recording (`rustysnes_core::trace`) and a fill indicator. This is
//! the reader: the recorded instructions disassembled and symbol-labelled, the call stack
//! reconstructed from the event log, the raw event list, and the hottest addresses from the access
//! counter.
//!
//! # Why the call stack is reconstructed rather than read
//!
//! The 65C816 stack holds return addresses, but nothing distinguishes one from an ordinary pushed
//! word — walking `S` upward and calling every plausible-looking value a frame is guesswork that
//! confidently invents frames that never existed. The event log records each enter and leave *as it
//! happened*, with the depth after it, so replaying it produces the stack that is actually there.
//! The cost is that the view only reaches back to when tracing was armed, which is stated in the
//! panel rather than papered over.

use crate::debug_snapshot::{DebugSnapshot, TraceEventRow, TraceRow};
use crate::ui_shell::{MenuAction, ShellState};

/// Rows shown at once in the trace list. The ring holds 4,096; rendering all of them every frame
/// costs more than it informs, and the tail is what a trace is read for.
const VISIBLE_ROWS: usize = 200;

/// Hottest addresses listed by the counter view.
const TOP_ADDRESSES: usize = 32;

/// Which sub-view of the trace panel is showing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TraceView {
    /// Disassembled executed instructions, newest last.
    #[default]
    Instructions,
    /// The call stack reconstructed from the event log.
    CallStack,
    /// The raw call/return/interrupt log.
    Events,
    /// The hottest addresses from the access counter.
    Hot,
}

impl ShellState {
    /// The trace panel.
    pub(crate) fn render_trace_panel(
        &mut self,
        ui: &mut egui::Ui,
        debug: Option<&DebugSnapshot>,
        symbols: Option<&crate::symbols::SymbolMap>,
        actions: &mut Vec<MenuAction>,
    ) {
        let Some(debug) = debug else {
            ui.label("(no debugger snapshot yet)");
            return;
        };

        ui.horizontal(|ui| {
            let mut tracing = debug.trace_armed;
            if ui.checkbox(&mut tracing, "Record").changed() {
                actions.push(MenuAction::SetTracing(tracing));
            }
            if ui.button("Clear").clicked() {
                actions.push(MenuAction::ClearTrace);
            }
            let (steps, events) = debug.trace_len;
            ui.label(
                egui::RichText::new(format!("{steps} instrs · {events} events"))
                    .small()
                    .weak(),
            );
        });
        ui.horizontal(|ui| {
            if ui.button("Load symbols…").clicked() {
                actions.push(MenuAction::LoadSymbols);
            }
            match symbols {
                Some(s) if !s.is_empty() => {
                    ui.label(format!("{} symbols", s.len()));
                    if ui.button("Clear symbols").clicked() {
                        actions.push(MenuAction::ClearSymbols);
                    }
                }
                _ => {
                    ui.label(egui::RichText::new("no symbols loaded").weak().small());
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.trace_view,
                TraceView::Instructions,
                "Instructions",
            );
            ui.selectable_value(&mut self.trace_view, TraceView::CallStack, "Call stack");
            ui.selectable_value(&mut self.trace_view, TraceView::Events, "Events");
            ui.selectable_value(&mut self.trace_view, TraceView::Hot, "Hot addresses");
        });
        ui.separator();

        match self.trace_view {
            TraceView::Instructions => instructions(ui, debug, symbols),
            TraceView::CallStack => call_stack(ui, debug, symbols),
            TraceView::Events => events(ui, debug, symbols),
            TraceView::Hot => hot(ui, debug, symbols),
        }
    }
}

/// Name an address, preferring a symbol.
fn label(symbols: Option<&crate::symbols::SymbolMap>, addr: u32) -> String {
    symbols
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("${addr:06X}"), |s| s.label(addr, 0x1000))
}

/// The disassembled trace, newest last.
fn instructions(
    ui: &mut egui::Ui,
    debug: &DebugSnapshot,
    symbols: Option<&crate::symbols::SymbolMap>,
) {
    if debug.trace.is_empty() {
        ui.label(if debug.trace_armed {
            "Recording; nothing captured yet."
        } else {
            "Not recording. Tick Record to start capturing executed instructions."
        });
        return;
    }
    let start = debug.trace.len().saturating_sub(VISIBLE_ROWS);
    if start > 0 {
        ui.label(
            egui::RichText::new(format!(
                "showing the last {VISIBLE_ROWS} of {} recorded",
                debug.trace.len()
            ))
            .small()
            .weak(),
        );
    }
    egui::ScrollArea::vertical()
        .max_height(340.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for row in &debug.trace[start..] {
                ui.monospace(format_trace_row(row, symbols));
            }
        });
}

/// One trace row: address, disassembly, and the registers as they were before it ran.
#[must_use]
pub fn format_trace_row(row: &TraceRow, symbols: Option<&crate::symbols::SymbolMap>) -> String {
    format!(
        "{:<20} {:<18} A={:04X} X={:04X} Y={:04X} S={:04X} P={:02X}{}",
        label(symbols, row.pbr_pc),
        row.text,
        row.a,
        row.x,
        row.y,
        row.sp,
        row.p,
        if row.emulation { " E" } else { "" }
    )
}

/// The call stack, replayed from the event log.
fn call_stack(
    ui: &mut egui::Ui,
    debug: &DebugSnapshot,
    symbols: Option<&crate::symbols::SymbolMap>,
) {
    let frames = reconstruct_stack(&debug.trace_events);
    ui.label(
        egui::RichText::new(
            "Replayed from the event log, so it reaches back only to when recording started \
             — the 65C816 stack itself cannot be walked reliably (a return address is \
             indistinguishable from any other pushed word).",
        )
        .small()
        .weak(),
    );
    ui.separator();
    if frames.is_empty() {
        ui.label("No open frames recorded.");
        return;
    }
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            // Innermost first, which is the order a stack trace is read in.
            for (depth, frame) in frames.iter().rev().enumerate() {
                ui.monospace(format!(
                    "{:>2}  {:<22} called from {}",
                    depth,
                    label(symbols, frame.to),
                    label(symbols, frame.from)
                ));
            }
        });
}

/// A frame that was entered and not yet left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// Where the call came from.
    pub from: u32,
    /// Where control went.
    pub to: u32,
}

/// Replay an event log into the currently-open frames, outermost first.
///
/// A leave with no matching enter is **ignored** rather than treated as an error: tracing armed
/// mid-execution genuinely sees returns from calls it never saw made, and dropping those is the only
/// honest reading — inventing a frame to pop would fabricate a caller.
#[must_use]
pub fn reconstruct_stack(events: &[TraceEventRow]) -> Vec<Frame> {
    let mut stack: Vec<Frame> = Vec::new();
    for e in events {
        if e.is_enter {
            stack.push(Frame {
                from: e.from,
                to: e.to,
            });
        } else if !stack.is_empty() {
            stack.pop();
        }
    }
    stack
}

/// The raw event log.
fn events(ui: &mut egui::Ui, debug: &DebugSnapshot, symbols: Option<&crate::symbols::SymbolMap>) {
    if debug.trace_events.is_empty() {
        ui.label("No control-flow events recorded.");
        return;
    }
    egui::ScrollArea::vertical()
        .max_height(340.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for e in &debug.trace_events {
                // Indented by depth, so nesting is visible without reading the numbers.
                let indent = "  ".repeat(usize::from(e.depth.min(16)));
                ui.monospace(format!(
                    "{indent}{:<8} {} -> {}",
                    e.kind,
                    label(symbols, e.from),
                    label(symbols, e.to)
                ));
            }
        });
}

/// The hottest addresses from the access counter.
fn hot(ui: &mut egui::Ui, debug: &DebugSnapshot, symbols: Option<&crate::symbols::SymbolMap>) {
    if !debug.counting_armed {
        ui.label("Access counting is off. Tick 'Count accesses' in the Memory panel.");
        return;
    }
    let rows = top_addresses(&debug.hot, TOP_ADDRESSES);
    if rows.is_empty() {
        ui.label("Counting, but nothing has been accessed yet.");
        return;
    }
    egui::Grid::new("hot_grid")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            for h in ["Address", "Reads", "Writes", "Total"] {
                ui.strong(h);
            }
            ui.end_row();
            for row in rows {
                ui.monospace(label(symbols, row.address));
                ui.monospace(format!("{}", row.reads));
                ui.monospace(format!("{}", row.writes));
                ui.monospace(format!("{}", row.reads.saturating_add(row.writes)));
                ui.end_row();
            }
        });
}

/// One entry in the hot-address list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotAddress {
    /// The 24-bit address.
    pub address: u32,
    /// Read count.
    pub reads: u32,
    /// Write count.
    pub writes: u32,
}

/// The `n` most-accessed addresses, hottest first.
///
/// Ties break by address so the list is stable frame to frame — a view that reshuffles equal-count
/// rows every frame is unreadable while the counts are still climbing.
#[must_use]
pub fn top_addresses(hot: &[HotAddress], n: usize) -> Vec<HotAddress> {
    let mut v: Vec<HotAddress> = hot
        .iter()
        .copied()
        .filter(|h| h.reads > 0 || h.writes > 0)
        .collect();
    v.sort_unstable_by(|a, b| {
        let (ta, tb) = (
            a.reads.saturating_add(a.writes),
            b.reads.saturating_add(b.writes),
        );
        tb.cmp(&ta).then(a.address.cmp(&b.address))
    });
    v.truncate(n);
    v
}

#[cfg(test)]
mod tests {
    use super::{Frame, HotAddress, reconstruct_stack, top_addresses};
    use crate::debug_snapshot::TraceEventRow;

    fn ev(is_enter: bool, from: u32, to: u32, depth: u16) -> TraceEventRow {
        TraceEventRow {
            kind: if is_enter { "Call" } else { "Return" },
            from,
            to,
            depth,
            is_enter,
        }
    }

    /// The stack is what the events say it is, innermost last.
    #[test]
    fn stack_replays_enters_and_leaves() {
        let events = vec![
            ev(true, 0x00_8000, 0x00_9000, 1),
            ev(true, 0x00_9010, 0x00_A000, 2),
            ev(false, 0x00_A020, 0x00_9010, 1),
            ev(true, 0x00_9030, 0x00_B000, 2),
        ];
        let stack = reconstruct_stack(&events);
        assert_eq!(
            stack,
            vec![
                Frame {
                    from: 0x00_8000,
                    to: 0x00_9000
                },
                Frame {
                    from: 0x00_9030,
                    to: 0x00_B000
                },
            ]
        );
    }

    /// Tracing armed mid-execution sees returns from calls it never saw. Those must be dropped, not
    /// turned into a fabricated caller.
    #[test]
    fn unmatched_leaves_are_dropped_not_invented() {
        let events = vec![
            ev(false, 0x00_A000, 0x00_9000, 0),
            ev(false, 0x00_9000, 0x00_8000, 0),
            ev(true, 0x00_8000, 0x00_C000, 1),
        ];
        let stack = reconstruct_stack(&events);
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0].to, 0x00_C000);
        assert!(reconstruct_stack(&[]).is_empty());
    }

    /// Hottest first, with ties broken by address so the list does not reshuffle every frame while
    /// counts are climbing.
    #[test]
    fn hot_list_is_sorted_and_stable() {
        let hot = vec![
            HotAddress {
                address: 0x7E_0002,
                reads: 5,
                writes: 5,
            },
            HotAddress {
                address: 0x7E_0000,
                reads: 100,
                writes: 0,
            },
            HotAddress {
                address: 0x7E_0001,
                reads: 5,
                writes: 5,
            },
            HotAddress {
                address: 0x7E_0003,
                reads: 0,
                writes: 0,
            },
        ];
        let top = top_addresses(&hot, 10);
        assert_eq!(top.len(), 3, "untouched addresses are excluded");
        assert_eq!(top[0].address, 0x7E_0000);
        // Equal totals: lower address first, every time.
        assert_eq!(top[1].address, 0x7E_0001);
        assert_eq!(top[2].address, 0x7E_0002);
        // Stable across repeated calls.
        assert_eq!(top_addresses(&hot, 10), top);
    }

    #[test]
    fn hot_list_respects_the_limit() {
        let hot: Vec<HotAddress> = (0..50)
            .map(|i| HotAddress {
                address: 0x7E_0000 + i,
                reads: 50 - i,
                writes: 0,
            })
            .collect();
        let top = top_addresses(&hot, 5);
        assert_eq!(top.len(), 5);
        assert_eq!(top[0].address, 0x7E_0000, "highest count first");
    }
}
