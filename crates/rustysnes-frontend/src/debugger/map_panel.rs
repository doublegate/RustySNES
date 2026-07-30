//! The cart memory-map panel (`v1.25.0`, T-FP-C1).
//!
//! Answers the question every other address-taking panel raises: **what is at `$C0:8000`?** The ROM
//! Info panel decodes the header, and the Cart panel names the board, but neither says where ROM,
//! SRAM, WRAM, and I/O actually land on the CPU bus — and that differs by mapping, which is exactly
//! why a static table would be worse than none.
//!
//! The ranges come from `emu::cart_bank_map`, derived from the mapping the header reported. This
//! module only renders them, plus an address lookup that answers the question directly.

use crate::debug_snapshot::{BankRange, DebugSnapshot};
use crate::ui_shell::ShellState;

impl BankRange {
    /// Whether `addr24` falls inside this range.
    #[must_use]
    pub const fn contains(&self, addr24: u32) -> bool {
        let bank = ((addr24 >> 16) & 0xFF) as u8;
        let offset = (addr24 & 0xFFFF) as u16;
        bank >= self.bank_lo
            && bank <= self.bank_hi
            && offset >= self.offset_lo
            && offset <= self.offset_hi
    }

    /// `"$00-3F:8000-FFFF"`.
    #[must_use]
    pub fn range_text(&self) -> String {
        format!(
            "${:02X}-{:02X}:{:04X}-{:04X}",
            self.bank_lo, self.bank_hi, self.offset_lo, self.offset_hi
        )
    }
}

/// What an address maps to, or `None` when no declared range covers it.
///
/// Returns the **first** matching range, which is why `emu::cart_bank_map` lists the specific
/// regions (low-RAM mirror, I/O) before the broad ROM windows that would otherwise swallow them.
#[must_use]
pub fn lookup(map: &[BankRange], addr24: u32) -> Option<&BankRange> {
    map.iter().find(|r| r.contains(addr24))
}

impl ShellState {
    /// The memory-map panel: an address lookup plus the decoded range table.
    pub(crate) fn render_map_panel(&mut self, ui: &mut egui::Ui, debug: Option<&DebugSnapshot>) {
        let Some(debug) = debug else {
            ui.label("(no debugger snapshot yet)");
            return;
        };
        if debug.map.is_empty() {
            // Distinguish "no ROM" from "a ROM whose map we could not decode" — they need
            // different reactions from whoever is reading this panel.
            ui.label("No cart loaded, so there is no cart memory map.");
            return;
        }

        ui.horizontal(|ui| {
            ui.label("What is at ($):");
            ui.add(egui::TextEdit::singleline(&mut self.map_lookup_input).desired_width(80.0));
        });
        if let Some(addr) = super::memory_panel::parse_address(&self.map_lookup_input) {
            match lookup(&debug.map, addr) {
                Some(range) => {
                    ui.label(format!(
                        "${addr:06X} -> {} ({})",
                        range.what,
                        range.range_text()
                    ));
                }
                None => {
                    ui.label(
                        egui::RichText::new(format!(
                            "${addr:06X} is in no declared range (open bus, or a board-specific \
                             window this map does not describe)"
                        ))
                        .weak(),
                    );
                }
            }
        }
        ui.separator();

        egui::Grid::new("map_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Range");
                ui.strong("Maps to");
                ui.end_row();
                for range in &debug.map {
                    ui.monospace(range.range_text());
                    ui.label(range.what);
                    ui.end_row();
                }
            });
        ui.separator();
        ui.label(
            egui::RichText::new(
                "Derived from the detected mapping. Coprocessor windows (DSP, Super FX, SA-1, \
                 S-DD1) are board-specific and are not listed here; see the Cart panel.",
            )
            .weak()
            .small(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::lookup;
    use crate::debug_snapshot::BankRange;

    fn range(
        bank_lo: u8,
        bank_hi: u8,
        offset_lo: u16,
        offset_hi: u16,
        what: &'static str,
    ) -> BankRange {
        BankRange {
            bank_lo,
            bank_hi,
            offset_lo,
            offset_hi,
            what,
        }
    }

    #[test]
    fn contains_checks_both_bank_and_offset() {
        let r = range(0x00, 0x3F, 0x8000, 0xFFFF, "ROM");
        assert!(r.contains(0x00_8000));
        assert!(r.contains(0x3F_FFFF));
        assert!(!r.contains(0x00_7FFF), "offset below the range");
        assert!(!r.contains(0x40_8000), "bank above the range");
    }

    /// Lookup returns the FIRST match, which is why the map must list specific regions before the
    /// broad ROM windows — otherwise `$00:1000` would report as ROM instead of the low-RAM mirror.
    #[test]
    fn lookup_prefers_the_earlier_more_specific_range() {
        let map = vec![
            range(0x00, 0x3F, 0x0000, 0x1FFF, "WRAM (low mirror)"),
            range(0x00, 0x3F, 0x2000, 0x5FFF, "I/O"),
            range(0x00, 0x7D, 0x8000, 0xFFFF, "ROM"),
        ];
        assert_eq!(
            lookup(&map, 0x00_1000).expect("low RAM").what,
            "WRAM (low mirror)"
        );
        assert_eq!(lookup(&map, 0x00_2100).expect("io").what, "I/O");
        assert_eq!(lookup(&map, 0x00_8000).expect("rom").what, "ROM");
    }

    /// An address no range covers reports as uncovered rather than being forced into the nearest
    /// one — "open bus" is a real answer and a wrong guess would send someone hunting a bug that
    /// is not there.
    #[test]
    fn uncovered_addresses_report_none() {
        let map = vec![range(0x00, 0x3F, 0x0000, 0x1FFF, "WRAM (low mirror)")];
        assert!(lookup(&map, 0x00_6000).is_none());
        assert!(lookup(&map, 0xFE_0000).is_none());
    }

    /// The map derivation genuinely differs by mapping, which is the whole point of deriving it.
    #[test]
    fn lorom_and_hirom_maps_differ() {
        use rustysnes_core::cart::MapMode;
        let lo = crate::emu::cart_bank_map(Some(MapMode::LoRom));
        let hi = crate::emu::cart_bank_map(Some(MapMode::HiRom));
        assert_ne!(lo, hi);
        // LoROM's SRAM lives in banks $70+ at $0000; HiROM's at $20-3F:$6000.
        assert_eq!(
            lookup(&lo, 0x70_0000).map(|r| r.what),
            Some("SRAM"),
            "LoROM SRAM window"
        );
        assert_eq!(
            lookup(&hi, 0x20_6000).map(|r| r.what),
            Some("SRAM"),
            "HiROM SRAM window"
        );
        // HiROM maps ROM linearly into $40+; LoROM does not.
        assert_eq!(
            lookup(&hi, 0x40_0000).map(|r| r.what),
            Some("ROM (64 KiB per bank, linear)")
        );
        assert!(lookup(&lo, 0x40_0000).is_none());
        // No cart, no map.
        assert!(crate::emu::cart_bank_map(None).is_empty());
    }
}
