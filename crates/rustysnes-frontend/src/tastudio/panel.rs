//! The `TAStudio` piano-roll panel (`v1.25.0`, T-FP-G2).
//!
//! A grid: one row per frame, one column per button, click to toggle. The panel is deliberately
//! thin — every edit goes through [`super::PianoRoll`], which returns the frame from which cached
//! state is invalid, and the panel forwards that as a [`crate::ui_shell::MenuAction`] rather than
//! touching the emulator itself.
//!
//! That routing is the shell's standing rule (egui never touches the emu lock) and it also makes
//! the invalidation impossible to forget: an edit *returns* the frame, so a caller that ignored it
//! would have to actively discard a value.

use super::{Cell, PianoRoll, Player};
use crate::input::Button;
use crate::ui_shell::{MenuAction, ShellState};

/// Rows rendered at once.
///
/// A TAS timeline is tens of thousands of frames; drawing all of them costs far more than it
/// informs, and egui's own scroll virtualisation does not extend to a hand-built grid.
const VISIBLE_ROWS: usize = 40;

/// The columns, in the order a TAS editor conventionally shows them.
const COLUMNS: [(Button, &str); 12] = [
    (Button::Up, "U"),
    (Button::Down, "D"),
    (Button::Left, "L"),
    (Button::Right, "R"),
    (Button::Select, "s"),
    (Button::Start, "S"),
    (Button::Y, "Y"),
    (Button::B, "B"),
    (Button::X, "X"),
    (Button::A, "A"),
    (Button::L, "l"),
    (Button::R, "r"),
];

/// The panel's own editing state.
#[derive(Debug, Clone, Default)]
pub struct TasState {
    /// The timeline being edited.
    pub roll: PianoRoll,
    /// The frame the view is scrolled to.
    pub view_frame: usize,
    /// The frame the emulator is currently at, drawn as the playhead.
    pub play_frame: usize,
    /// Which controller the grid edits.
    pub player: Player,
    /// The copy buffer.
    pub clipboard: Vec<(u16, u16)>,
    /// How many frames the insert/delete buttons act on.
    pub span: usize,
    /// Cached greenzone stats for display, refreshed by the app.
    pub greenzone_states: usize,
    /// Cached greenzone size in bytes.
    pub greenzone_bytes: usize,
}

impl TasState {
    /// A fresh editor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            span: 1,
            ..Self::default()
        }
    }
}

impl ShellState {
    /// The `TAStudio` panel.
    pub(crate) fn render_tastudio(&mut self, ctx: &egui::Context) -> Vec<MenuAction> {
        let mut actions = Vec::new();
        let mut open = self.tastudio_open;
        egui::Window::new("`TAStudio`")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                toolbar(ui, &mut self.tas, &mut actions);
                ui.separator();
                grid(ui, &mut self.tas, &mut actions);
                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "greenzone: {} states, {} KiB \u{2014} editing a frame discards every \
                         cached state from it onward, because emulation is deterministic and \
                         they describe a machine that no longer exists.",
                        self.tas.greenzone_states,
                        self.tas.greenzone_bytes / 1024
                    ))
                    .small()
                    .weak(),
                );
            });
        self.tastudio_open = open;
        actions
    }
}

/// The controls above the grid.
fn toolbar(ui: &mut egui::Ui, tas: &mut TasState, actions: &mut Vec<MenuAction>) {
    ui.horizontal(|ui| {
        ui.label("Player:");
        ui.selectable_value(&mut tas.player, Player::One, "1");
        ui.selectable_value(&mut tas.player, Player::Two, "2");
        ui.separator();
        ui.label(format!("frame {} / {}", tas.play_frame, tas.roll.len()));
        if ui.button("Seek here").clicked() {
            actions.push(MenuAction::TasSeek(tas.view_frame));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Span:");
        ui.add(egui::DragValue::new(&mut tas.span).range(1..=600));
        if ui.button("Insert").clicked() {
            let from = tas.roll.insert_frames(tas.view_frame, tas.span);
            actions.push(MenuAction::TasInvalidate(from));
        }
        if ui.button("Delete").clicked() {
            let from = tas.roll.delete_frames(tas.view_frame, tas.span);
            actions.push(MenuAction::TasInvalidate(from));
        }
        if ui.button("Copy").clicked() {
            tas.clipboard = tas.roll.copy(tas.view_frame, tas.span);
        }
        if ui.button("Paste").clicked() && !tas.clipboard.is_empty() {
            let clip = tas.clipboard.clone();
            let from = tas.roll.paste(tas.view_frame, &clip);
            actions.push(MenuAction::TasInvalidate(from));
        }
        ui.label(
            egui::RichText::new(format!("{} frames copied", tas.clipboard.len()))
                .small()
                .weak(),
        );
    });
}

/// The piano roll itself.
fn grid(ui: &mut egui::Ui, tas: &mut TasState, actions: &mut Vec<MenuAction>) {
    // The scrollable extent is the whole timeline plus a page of room past its end: a TAS is
    // written by *extending* it, and rows that do not exist yet are exactly where the next input
    // goes. `show_rows` then builds only the rows actually on screen, so the scrollbar and the
    // wheel move through the timeline itself. Rendering a fixed window around `view_frame` inside
    // a `ScrollArea` instead — which is what this did — gave the area only those rows to scroll
    // within, so the wheel ran out after one page and the timeline could not be browsed at all.
    let total_rows = tas.roll.len().saturating_add(VISIBLE_ROWS);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace);

    egui::ScrollArea::vertical().max_height(420.0).show_rows(
        ui,
        row_height,
        total_rows,
        |ui, rows| {
            egui::Grid::new("tas_grid")
                .num_columns(COLUMNS.len() + 1)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Frame");
                    for (_, label) in COLUMNS {
                        ui.strong(label);
                    }
                    ui.end_row();

                    for frame in rows {
                        // The playhead marker is what makes the grid legible while running; without
                        // it the rows are indistinguishable and the view is just a number soup.
                        let marker = if frame == tas.play_frame { ">" } else { " " };
                        let text = egui::RichText::new(format!("{marker}{frame:>7}")).monospace();
                        if ui
                            .add(egui::Label::new(text).sense(egui::Sense::click()))
                            .clicked()
                        {
                            tas.view_frame = frame;
                        }
                        for (button, label) in COLUMNS {
                            let cell = Cell {
                                frame,
                                player: tas.player,
                                button,
                            };
                            let pressed = tas.roll.get(cell);
                            let shown = if pressed { label } else { "." };
                            if ui
                                .add(
                                    egui::Label::new(
                                        egui::RichText::new(shown).monospace().strong(),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                let from = tas.roll.toggle(cell);
                                actions.push(MenuAction::TasInvalidate(from));
                            }
                        }
                        ui.end_row();
                    }
                });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::{COLUMNS, TasState};
    use crate::input::Button;

    /// Every pad input must have a column, or it is one the TAS author simply cannot set.
    #[test]
    fn every_button_has_a_column() {
        assert_eq!(COLUMNS.len(), 12);
        for button in Button::ALL {
            let count = COLUMNS.iter().filter(|(b, _)| *b == button).count();
            assert_eq!(count, 1, "{button:?} appears {count} times");
        }
    }

    /// Column labels must be distinguishable, or the grid is unreadable — note the deliberate
    /// case split between Select/Start and L/R shoulder vs. the Left/Right d-pad.
    #[test]
    fn column_labels_are_unique() {
        let mut labels: Vec<&str> = COLUMNS.iter().map(|(_, l)| *l).collect();
        labels.sort_unstable();
        let before = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), before, "duplicate column labels: {labels:?}");
    }

    #[test]
    fn a_new_editor_has_a_usable_span() {
        let tas = TasState::new();
        assert_eq!(tas.span, 1, "a zero span would make insert/delete no-ops");
        assert!(tas.roll.is_empty());
        assert!(tas.clipboard.is_empty());
    }
}
