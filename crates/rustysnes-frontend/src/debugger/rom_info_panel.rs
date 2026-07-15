//! The ROM Info debugger panel: a read-only CRC32/SHA-256/header decode of the loaded cart.
//!
//! `v1.20.0`: closes a named gap against RustyNES's own `rom_info_panel.rs` — that panel and
//! this one share the same *purpose* (identify the loaded image + show its header fields at a
//! glance) but not code: RustySNES's `rustysnes_cart::Header` is a different shape from
//! RustyNES's iNES header, so this is a fresh implementation, not a port.
//!
//! [`RomInfo`] is captured once per ROM load (`app.rs`'s `MenuAction::OpenRom`/`CloseRom` and the
//! `wasm32` file-picker path), not recomputed every frame like [`crate::debug_snapshot::DebugSnapshot`]
//! — the ROM's identity and header never change while it stays loaded, so hashing it again on
//! every present would be pure waste.

// `rustysnes-frontend` doesn't depend on `rustysnes-cart` directly — it goes through
// `rustysnes-core`'s `pub use rustysnes_cart as cart;` re-export, matching how the rest of this
// crate reaches cart types (see `emu.rs`'s `system_mut().bus.cart` access pattern).
use rustysnes_core::cart::{Coprocessor, MapMode, Region};

/// A loaded ROM's identity (CRC32/SHA-256) + decoded internal header, for the ROM Info panel.
#[derive(Debug, Clone)]
pub struct RomInfo {
    /// CRC32 of the raw ROM image (copier-prefix included, matching what most ROM databases key
    /// on).
    pub crc32: u32,
    /// SHA-256 of the raw ROM image — the same hash [`rustysnes_core::movie::hash_rom`] uses to
    /// key save-states/movies to a specific ROM, reused here rather than recomputed.
    pub sha256: [u8; 32],
    /// The active board's name (e.g. `"LoROM"`, `"HiROM+SA-1"`).
    pub board_name: &'static str,
    /// The decoded internal title — see `rustysnes_cart::header::Header::title`.
    pub title: String,
    /// The base cartridge map mode.
    pub map_mode: MapMode,
    /// Whether the cart runs the FastROM access window.
    pub fast_rom: bool,
    /// The console region (from the destination-code header byte).
    pub region: Region,
    /// The on-cart coprocessor, if any.
    pub coprocessor: Coprocessor,
    /// ROM size in bytes (post copier-prefix-strip).
    pub rom_size: usize,
    /// SRAM size in bytes (0 if none).
    pub sram_size: usize,
    /// Whether the cart is battery-backed.
    pub has_battery: bool,
    /// The byte offset the header was found at (relative to the prefix-stripped image).
    pub header_offset: usize,
    /// The number of leading copier-prefix bytes stripped (0 or 512).
    pub copier_prefix: usize,
}

impl RomInfo {
    /// Capture a [`RomInfo`] snapshot from the currently-loaded ROM, or `None` if none is loaded.
    #[must_use]
    pub fn capture(emu: &mut crate::emu::EmuCore) -> Option<Self> {
        let crc32 = crc32fast::hash(emu.rom());
        let sha256 = rustysnes_core::movie::hash_rom(emu.rom());
        let board_name = emu.cart_name()?;
        let header = emu.system_mut().bus.cart.as_ref()?.header.clone();
        Some(Self {
            crc32,
            sha256,
            board_name,
            title: header.title,
            map_mode: header.map_mode,
            fast_rom: header.fast_rom,
            region: header.region,
            coprocessor: header.coprocessor,
            rom_size: header.rom_size,
            sram_size: header.sram_size,
            has_battery: header.has_battery,
            header_offset: header.offset,
            copier_prefix: header.copier_prefix,
        })
    }
}

/// Render the ROM Info panel — a plain field grid, "(no ROM loaded)" when `info` is `None`.
pub(super) fn render(ui: &mut egui::Ui, info: Option<&RomInfo>) {
    let Some(info) = info else {
        ui.label("(no ROM loaded)");
        return;
    };
    egui::Grid::new("rom_info").num_columns(2).show(ui, |ui| {
        ui.label("Title");
        ui.label(if info.title.is_empty() {
            "(blank)"
        } else {
            &info.title
        });
        ui.end_row();

        ui.label("Board");
        ui.label(info.board_name);
        ui.end_row();

        ui.label("Map mode");
        ui.label(format!("{:?}", info.map_mode));
        ui.end_row();

        ui.label("Speed");
        ui.label(if info.fast_rom { "FastROM" } else { "SlowROM" });
        ui.end_row();

        ui.label("Region");
        ui.label(format!("{:?}", info.region));
        ui.end_row();

        ui.label("Coprocessor");
        ui.label(format!("{:?}", info.coprocessor));
        ui.end_row();

        ui.label("ROM size");
        ui.label(format!(
            "{} bytes ({} KiB)",
            info.rom_size,
            info.rom_size / 1024
        ));
        ui.end_row();

        ui.label("SRAM size");
        ui.label(format!("{} bytes", info.sram_size));
        ui.end_row();

        ui.label("Battery-backed");
        ui.label(if info.has_battery { "yes" } else { "no" });
        ui.end_row();

        ui.label("Header offset");
        ui.label(format!("${:06X}", info.header_offset));
        ui.end_row();

        ui.label("Copier prefix");
        ui.label(format!("{} bytes", info.copier_prefix));
        ui.end_row();

        ui.label("CRC32");
        ui.monospace(format!("{:08X}", info.crc32));
        ui.end_row();

        ui.label("SHA-256");
        ui.monospace(hex_sha256(&info.sha256));
        ui.end_row();
    });
}

/// Format a SHA-256 digest as a contiguous lowercase hex string (unlike [`super::hex_row_bytes`],
/// which space-separates for a hex-dump view — a digest reads better unbroken).
fn hex_sha256(bytes: &[u8; 32]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::hex_sha256;

    #[test]
    fn hex_sha256_formats_lowercase_contiguous() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xAB;
        bytes[1] = 0x01;
        bytes[31] = 0xFF;
        let s = hex_sha256(&bytes);
        assert_eq!(s.len(), 64);
        assert!(s.starts_with("ab01"));
        assert!(s.ends_with("ff"));
        assert_eq!(s, s.to_lowercase());
    }
}
