//! The Memory editor panel (`v1.25.0`, T-FP-C1).
//!
//! The `v1.7.0` Memory/Watch panel showed a fixed 512-byte read-only hex dump starting wherever
//! `debug_memory_scroll` happened to be, with no way to move it and no way to change a byte. This
//! panel adds the three things that make a hex dump a debugging tool rather than a decoration:
//!
//! - **Go to an address**, and page through memory from there.
//! - **Edit a byte**, written through `Bus::poke_wram`.
//! - **Freeze an address**, re-applied once per frame so a value the game is actively rewriting
//!   stays put — which is how you test a hypothesis about what a variable controls.
//! - **A heat column** from the core's access counter, when counting is armed.
//!
//! Edits reach **WRAM only**. `poke_wram` is deliberately not a general bus write: a debugger that
//! appeared to edit a ROM byte which then read back unchanged would look like an emulation bug, so
//! rows outside WRAM are shown greyed and their edit boxes are disabled. See
//! [`crate::emu::EmuCore::poke`].
//!
//! Follows the same non-negotiable rule as every other panel: it never touches the emu lock. Edits
//! and freezes are *requests* pushed into the caller's lists, applied by `app.rs` under the brief
//! lock it already holds.

use crate::debug_snapshot::{AccessHeat, DebugSnapshot, MEMORY_WINDOW_LEN};
use crate::ui_shell::{MenuAction, ShellState};

/// Bytes per displayed row.
const ROW_BYTES: usize = 16;

/// One address the debugger holds at a fixed value (`v1.25.0`, T-FP-C1).
///
/// Applied every present rather than once, because the point is to hold a value the *game* is
/// writing — a one-shot poke would be overwritten by the next frame's game logic and look like the
/// freeze silently failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreezeEntry {
    /// The 24-bit CPU-bus address held.
    pub address: u32,
    /// The value written to it each frame.
    pub value: u8,
}

/// A one-shot byte write requested by the editor, applied by `app.rs` under the emu lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PokeRequest {
    /// The 24-bit CPU-bus address to write.
    pub address: u32,
    /// The byte to write.
    pub value: u8,
}

/// Whether an address is editable — i.e. whether `Bus::poke_wram` will actually change it.
///
/// Mirrors `poke_wram`'s own accepted ranges exactly. Kept here (rather than inferred from a failed
/// write) so the UI can disable an edit box *before* the user types into it: a write that silently
/// does nothing is worse than one that is visibly unavailable.
#[must_use]
pub const fn is_editable(addr24: u32) -> bool {
    let bank = (addr24 >> 16) & 0xFF;
    let offset = addr24 & 0xFFFF;
    match bank {
        0x7E..=0x7F => true,
        0x00..=0x3F | 0x80..=0xBF => offset < 0x2000,
        _ => false,
    }
}

/// Reduce a WRAM address to its canonical linear `$7E`/`$7F` form.
///
/// The `$0000-$1FFF` low-RAM window in banks `$00-3F`/`$80-BF` is not a separate memory — it is the
/// same 8 KiB as `$7E:0000-1FFF`. A freeze list keyed on the raw address would therefore accept
/// `$00:0042` and `$7E:0042` as two independent entries holding two different values for one byte,
/// and the per-frame poke loop would apply both in list order, so whichever came last would win and
/// the other would silently do nothing. Everything else is returned unchanged.
#[must_use]
pub const fn canonical_wram(addr24: u32) -> u32 {
    let bank = (addr24 >> 16) & 0xFF;
    let offset = addr24 & 0xFFFF;
    match bank {
        0x00..=0x3F | 0x80..=0xBF if offset < 0x2000 => 0x7E_0000 | offset,
        _ => addr24,
    }
}

/// Parse a hex address from user input, tolerating a `$` or `0x` prefix and surrounding space.
///
/// Returns `None` for anything that is not a valid 24-bit address, so a typo leaves the view where
/// it was instead of jumping somewhere arbitrary.
#[must_use]
pub fn parse_address(input: &str) -> Option<u32> {
    let t = input.trim().trim_start_matches('$');
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    if t.is_empty() {
        return None;
    }
    u32::from_str_radix(t, 16)
        .ok()
        .filter(|a| *a <= 0x00FF_FFFF)
}

/// Parse a hex byte from user input (`"7F"`, `"$7f"`, `"0x7F"`).
#[must_use]
pub fn parse_byte(input: &str) -> Option<u8> {
    let t = input.trim().trim_start_matches('$');
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    u8::from_str_radix(t, 16).ok()
}

/// A heat value's 0.0..=1.0 intensity against the map's peak.
///
/// Normalised against the **peak** rather than an absolute scale, because access counts span orders
/// of magnitude between a once-per-frame variable and an inner-loop pointer; a fixed scale would
/// render every address either black or white.
#[must_use]
pub fn heat_intensity(count: u32, peak: u32) -> f32 {
    if peak == 0 || count == 0 {
        return 0.0;
    }
    // Logarithmic, for the same order-of-magnitude reason: linear normalisation against a hot inner
    // loop leaves every ordinary variable indistinguishable from untouched.
    let c = f64::from(count).ln_1p();
    let p = f64::from(peak).ln_1p();
    if p <= 0.0 {
        return 0.0;
    }
    #[allow(clippy::cast_possible_truncation)]
    let v = (c / p) as f32;
    v.clamp(0.0, 1.0)
}

impl ShellState {
    /// The Memory editor panel.
    ///
    /// `freezes` and `pokes` are mutated in place, matching how `render_watch_panel` already
    /// handles its watchpoint list — the app drains `pokes` after the egui pass.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn render_memory_panel(
        &mut self,
        ui: &mut egui::Ui,
        debug: Option<&DebugSnapshot>,
        freezes: &mut Vec<FreezeEntry>,
        pokes: &mut Vec<PokeRequest>,
        actions: &mut Vec<MenuAction>,
    ) {
        let Some(debug) = debug else {
            ui.label("(no debugger snapshot yet)");
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Go to ($):");
            ui.add(egui::TextEdit::singleline(&mut self.mem_goto_input).desired_width(80.0));
            if ui.button("Go").clicked() {
                if let Some(addr) = parse_address(&self.mem_goto_input) {
                    self.mem_goto_target = Some(addr);
                    self.mem_error = None;
                } else {
                    self.mem_error = Some("Not a 24-bit hex address".into());
                }
            }
            // Paging moves by the whole window, so successive pages do not overlap or skip.
            if ui.button("<< Page").clicked() {
                let start = debug
                    .memory_window_start
                    .wrapping_sub(MEMORY_WINDOW_LEN as u32);
                self.mem_goto_target = Some(start & 0x00FF_FFFF);
            }
            if ui.button("Page >>").clicked() {
                let start = debug
                    .memory_window_start
                    .wrapping_add(MEMORY_WINDOW_LEN as u32);
                self.mem_goto_target = Some(start & 0x00FF_FFFF);
            }
        });
        if let Some(err) = self.mem_error.as_ref() {
            ui.colored_label(ui.visuals().error_fg_color, err);
        }

        ui.horizontal(|ui| {
            ui.label(format!("Window: ${:06X}", debug.memory_window_start));
            if debug.counting_armed {
                ui.label(format!("· heat peak {}", debug.heat_peak));
            }
        });

        // Core instrumentation arming (`v1.25.0`, T-FP-C1). Both are off by default and cost one
        // `bool` test per hook while disarmed — see `rustysnes_core::trace`.
        ui.horizontal(|ui| {
            let mut counting = debug.counting_armed;
            if ui
                .checkbox(&mut counting, "Count accesses")
                .on_hover_text(
                    "Record per-address read/write counts for WRAM, shown as the heat column.",
                )
                .changed()
            {
                actions.push(MenuAction::SetAccessCounting(counting));
            }
            if debug.counting_armed && ui.button("Reset counts").clicked() {
                actions.push(MenuAction::ClearAccessCounts);
            }

            let mut tracing = debug.trace_armed;
            if ui
                .checkbox(&mut tracing, "Trace instructions")
                .on_hover_text(
                    "Record executed instructions and call/return/interrupt events into a bounded \
                     ring. The viewer lands with the trace panel.",
                )
                .changed()
            {
                actions.push(MenuAction::SetTracing(tracing));
            }
            if debug.trace_armed || debug.trace_len.0 > 0 {
                let (steps, events) = debug.trace_len;
                ui.label(
                    egui::RichText::new(format!("{steps} instrs · {events} events"))
                        .small()
                        .weak(),
                );
            }
        });
        ui.separator();

        self.memory_rows(ui, debug, pokes);

        ui.separator();
        self.freeze_controls(ui, freezes);
    }

    /// The hex rows themselves: address, heat, 16 bytes, ASCII.
    #[allow(clippy::cast_possible_truncation)]
    fn memory_rows(
        &mut self,
        ui: &mut egui::Ui,
        debug: &DebugSnapshot,
        pokes: &mut Vec<PokeRequest>,
    ) {
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                for (row, chunk) in debug.memory_window.chunks(ROW_BYTES).enumerate() {
                    let base = debug
                        .memory_window_start
                        .wrapping_add((row * ROW_BYTES) as u32)
                        & 0x00FF_FFFF;
                    ui.horizontal(|ui| {
                        let editable = is_editable(base);
                        let addr_text = egui::RichText::new(format!("${base:06X}")).monospace();
                        // A non-WRAM row is greyed, so it is obvious *before* clicking that its
                        // bytes cannot be edited.
                        ui.label(if editable {
                            addr_text
                        } else {
                            addr_text.weak()
                        });
                        if debug.counting_armed {
                            heat_swatch(ui, &debug.heat, row * ROW_BYTES, debug.heat_peak);
                        }
                        ui.monospace(super::hex_row_bytes(chunk));
                        ui.monospace(ascii_row(chunk));
                    });
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Write ($addr = $val):");
            ui.add(egui::TextEdit::singleline(&mut self.mem_poke_addr).desired_width(70.0));
            ui.add(egui::TextEdit::singleline(&mut self.mem_poke_value).desired_width(40.0));
            if ui.button("Poke").clicked() {
                match (
                    parse_address(&self.mem_poke_addr),
                    parse_byte(&self.mem_poke_value),
                ) {
                    (Some(address), Some(value)) if is_editable(address) => {
                        pokes.push(PokeRequest { address, value });
                        self.mem_error = None;
                    }
                    (Some(address), Some(_)) => {
                        // Named explicitly rather than silently ignored: a poke that does nothing
                        // reads as an emulator bug.
                        self.mem_error =
                            Some(format!("${address:06X} is not WRAM; only WRAM is writable"));
                    }
                    _ => self.mem_error = Some("Need a hex address and a hex byte".into()),
                }
            }
        });
    }

    /// The freeze list: add from the poke fields, and remove.
    fn freeze_controls(&mut self, ui: &mut egui::Ui, freezes: &mut Vec<FreezeEntry>) {
        ui.horizontal(|ui| {
            ui.label(format!("Frozen ({}):", freezes.len()));
            if ui.button("Freeze the address above").clicked() {
                match (
                    parse_address(&self.mem_poke_addr),
                    parse_byte(&self.mem_poke_value),
                ) {
                    (Some(address), Some(value)) if is_editable(address) => {
                        // Keyed on the canonical address, so freezing a byte through the low-RAM
                        // mirror and through linear `$7E` is one entry, not two that fight.
                        let address = canonical_wram(address);
                        // Re-freezing an address replaces its held value rather than stacking a
                        // second entry that would fight the first every frame.
                        if let Some(existing) = freezes.iter_mut().find(|f| f.address == address) {
                            existing.value = value;
                        } else {
                            freezes.push(FreezeEntry { address, value });
                        }
                        self.mem_error = None;
                    }
                    (Some(address), Some(_)) => {
                        self.mem_error = Some(format!(
                            "${address:06X} is not WRAM; only WRAM is freezable"
                        ));
                    }
                    _ => self.mem_error = Some("Need a hex address and a hex byte".into()),
                }
            }
            if ui.button("Clear all").clicked() {
                freezes.clear();
            }
        });
        let mut remove = None;
        for (i, f) in freezes.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(format!("${:06X} = ${:02X}", f.address, f.value));
                if ui.button("x").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            freezes.remove(i);
        }
    }
}

/// A small coloured block summarising one row's access heat.
fn heat_swatch(ui: &mut egui::Ui, heat: &[AccessHeat], offset: usize, peak: u32) {
    let total: u32 = heat
        .iter()
        .skip(offset)
        .take(ROW_BYTES)
        .map(|h| h.total())
        .fold(0u32, u32::saturating_add);
    let t = heat_intensity(total, peak);
    let (rect, _r) = ui.allocate_exact_size(egui::vec2(10.0, 12.0), egui::Sense::hover());
    // Cold rows stay at the panel background rather than being drawn dark-blue: "not accessed"
    // should read as absence, not as a low value on a scale.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = (t * 255.0) as u8;
    let color = egui::Color32::from_rgb(level, level / 3, 40);
    ui.painter().rect_filled(rect, 1.0, color);
}

/// Render a row of bytes as printable ASCII, with `.` for anything unprintable.
fn ascii_row(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if (0x20..0x7F).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ascii_row, canonical_wram, heat_intensity, is_editable, parse_address, parse_byte,
    };

    /// A freeze list keyed on the raw address would treat `$00:0042` and `$7E:0042` as two
    /// independent entries — but they are one byte, so the per-frame poke loop would apply both in
    /// list order and whichever came last would silently win. Canonicalising collapses them.
    #[test]
    fn the_low_ram_mirror_canonicalises_onto_linear_wram() {
        assert_eq!(canonical_wram(0x00_0042), 0x7E_0042);
        assert_eq!(canonical_wram(0x3F_1FFF), 0x7E_1FFF);
        assert_eq!(canonical_wram(0x80_0000), 0x7E_0000);
        assert_eq!(canonical_wram(0xBF_0100), 0x7E_0100);
        // Already linear, or past the mirror window, or not WRAM at all: unchanged.
        assert_eq!(canonical_wram(0x7E_0042), 0x7E_0042);
        assert_eq!(canonical_wram(0x7F_FFFF), 0x7F_FFFF);
        assert_eq!(canonical_wram(0x00_2000), 0x00_2000);
        assert_eq!(canonical_wram(0x40_0000), 0x40_0000);
        // Every mirror alias of one byte must land on the SAME key.
        let aliases = [0x00_0042, 0x01_0042, 0x3F_0042, 0x80_0042, 0xBF_0042];
        for a in aliases {
            assert_eq!(canonical_wram(a), 0x7E_0042, "${a:06X}");
        }
    }

    /// Only what `Bus::poke_wram` actually writes counts as editable — the UI must be able to
    /// disable an edit box rather than accept a write that silently does nothing.
    #[test]
    fn editable_matches_poke_wram_ranges() {
        assert!(is_editable(0x7E_0000));
        assert!(is_editable(0x7F_FFFF));
        assert!(is_editable(0x00_1FFF), "low-RAM mirror");
        assert!(is_editable(0xB0_0000), "mirror in the $80-BF half");
        assert!(!is_editable(0x00_2100), "PPU register");
        assert!(!is_editable(0x00_8000), "cart ROM");
        assert!(!is_editable(0x40_0000), "HiROM space");
    }

    /// Address entry tolerates the prefixes a user actually types, and rejects the rest rather
    /// than jumping somewhere arbitrary.
    #[test]
    fn address_parsing_is_forgiving_but_bounded() {
        assert_eq!(parse_address("7E0000"), Some(0x7E_0000));
        assert_eq!(parse_address(" $7e0000 "), Some(0x7E_0000));
        assert_eq!(parse_address("0x7E0000"), Some(0x7E_0000));
        assert_eq!(parse_address("0"), Some(0));
        assert_eq!(parse_address(""), None);
        assert_eq!(parse_address("$"), None);
        assert_eq!(parse_address("zzz"), None);
        assert_eq!(parse_address("1000000"), None, "past the 24-bit bus");
    }

    #[test]
    fn byte_parsing_accepts_the_same_prefixes() {
        assert_eq!(parse_byte("7F"), Some(0x7F));
        assert_eq!(parse_byte("$0a"), Some(0x0A));
        assert_eq!(parse_byte("0xFF"), Some(0xFF));
        assert_eq!(parse_byte("100"), None, "does not fit a byte");
        assert_eq!(parse_byte(""), None);
    }

    /// The heat scale is logarithmic against the peak: an ordinary once-per-frame variable must
    /// still be visible next to an inner-loop pointer accessed thousands of times.
    #[test]
    fn heat_intensity_is_log_scaled_and_bounded() {
        assert!((heat_intensity(0, 1000)).abs() < 1e-6, "untouched is zero");
        assert!((heat_intensity(5, 0)).abs() < 1e-6, "no peak is zero");
        assert!(
            (heat_intensity(1000, 1000) - 1.0).abs() < 1e-6,
            "peak is one"
        );
        let modest = heat_intensity(10, 10_000);
        assert!(
            modest > 0.2,
            "a linear scale would render 10-of-10000 as invisible ({modest})"
        );
        assert!(modest < 1.0);
        // Monotonic.
        assert!(heat_intensity(100, 10_000) > modest);
        // Never out of range even if a count somehow exceeds the recorded peak.
        assert!((heat_intensity(20_000, 10_000) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ascii_row_replaces_unprintables() {
        assert_eq!(ascii_row(b"Hi!"), "Hi!");
        assert_eq!(ascii_row(&[0x00, 0x41, 0x7F, 0xFF]), ".A..");
    }
}
