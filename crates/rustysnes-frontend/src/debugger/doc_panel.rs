//! The Doc debugger panel (`v1.8.0 "Tracepoint"`): an in-app SNES-terminology glossary, for
//! looking up a register/chip abbreviation without leaving the debugger mid-session.
//!
//! Deliberately scoped to `docs/glossary.md` alone (embedded via `include_str!`, ~3KB, negligible
//! wasm size impact), not the full subsystem-spec tree (`cpu.md`/`ppu.md`/`apu.md`/`scheduler.md`/
//! `cart.md`/`frontend.md` are 10KB-50KB each) — that full tree is already published as the
//! `MkDocs` handbook (`v1.6.0 "Lighthouse"`, `https://doublegate.github.io/RustySNES/docs/`),
//! linked below rather than duplicated into the binary.

/// The embedded glossary text, read once at compile time.
const GLOSSARY_MD: &str = include_str!("../../../../docs/glossary.md");

/// Render the embedded glossary as plain scrollable monospace text (no markdown rendering — a
/// full `CommonMark` renderer is real weight for a debugger panel whose job is quick lookup, not
/// pretty typesetting) plus a link to the full documentation handbook.
pub(super) fn render(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Quick reference below. Full subsystem specs:");
        ui.hyperlink_to(
            "doublegate.github.io/RustySNES/docs/",
            "https://doublegate.github.io/RustySNES/docs/",
        );
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .max_height(400.0)
        .id_salt("doc_panel_scroll")
        .show(ui, |ui| {
            ui.monospace(GLOSSARY_MD);
        });
}
