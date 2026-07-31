//! Shared fixtures for the fuzz targets.
//!
//! Several boundaries can only be reached from a machine that already has a cart loaded — a
//! save-state never embeds ROM bytes (`docs/adr/0006`), so `load_state` on a bare `System` would
//! reject every input for the wrong reason and the target would fuzz nothing. These helpers build
//! the minimum machine each such target needs.
//!
//! Deliberately duplicated rather than imported: the equivalents in the engine
//! (`scheduler::tests::synth_rom`, `facade::tests::minimal_lorom`) are `#[cfg(test)]` and so are
//! invisible outside their own crate. Making them `pub` to serve fuzzing would put test scaffolding
//! in the shipped API.

#![allow(dead_code)] // Each target uses a subset; a target-local `unused` is not a finding.

/// A minimal-but-valid all-zero-body LoROM image: 32 KiB with just enough internal header at
/// `$7FC0` for `Header::detect`'s permissive scoring to accept it.
///
/// Kept as small as the detector allows (one 32 KiB bank) because every target that restores a
/// state builds one of these per input, and `Header::detect` rejects anything shorter.
#[must_use]
pub fn minimal_lorom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    let h = 0x7FC0;
    rom[h..h + 21].copy_from_slice(b"RUSTYSNES FUZZ       ");
    rom[h + 0x15] = 0x20; // MAP_MODE: slow LoROM
    rom[h + 0x16] = 0x00; // CHIPSET: ROM only
    rom[h + 0x17] = 0x08; // ROM_SIZE
    rom[h + 0x18] = 0x00; // RAM_SIZE: none
    rom[h + 0x19] = 0x00; // REGION: Japan/NTSC
    let checksum: u16 = 0x1234;
    rom[h + 0x1C..h + 0x1E].copy_from_slice(&(!checksum).to_le_bytes());
    rom[h + 0x1E..h + 0x20].copy_from_slice(&checksum.to_le_bytes());
    rom[h + 0x3C..h + 0x3E].copy_from_slice(&0x8000u16.to_le_bytes()); // reset vector
    rom
}

/// An [`EmuCore`](rustysnes_core::facade::EmuCore) with [`minimal_lorom`] already loaded.
///
/// # Panics
/// Panics if the fixture ROM stops being accepted. That is a real regression in header detection,
/// not a fuzz finding — failing loudly here beats every input silently short-circuiting.
#[must_use]
pub fn minimal_core() -> rustysnes_core::facade::EmuCore {
    let mut core =
        rustysnes_core::facade::EmuCore::new(0, rustysnes_cart::Region::Ntsc);
    core.load_rom(&minimal_lorom())
        .expect("the fuzz fixture ROM must load; if this fails, header detection regressed");
    core
}
