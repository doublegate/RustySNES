//! Fuzz `Header::detect` and `Cart::load` — the cartridge-header boundary.
//!
//! `detect` scores candidate offsets (LoROM `$7FC0` / HiROM `$FFC0` / ExHiROM `$40FFC0`) after
//! optionally stripping a 512-byte copier prefix, and `Cart::load` then hands the stripped image to
//! `board::select`, which dispatches on the chipset byte to one of ~20 board implementations
//! including the coprocessor carts. That dispatch is the interesting part: an arbitrary chipset
//! byte picks a board, and the board then computes windows and masks from ROM-supplied size fields.
//!
//! Contract under test: **no input is a panic**. Both entry points return `Result`, so a crash here
//! is a slice-index or arithmetic bug, not a missing error case.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // A 32 MiB ceiling matching `facade::MAX_DECOMPRESSED_ROM_SIZE`. libFuzzer rarely generates
    // anything this large, but capping keeps a pathological corpus entry from dominating the
    // campaign's time budget.
    if data.len() > 32 * 1024 * 1024 {
        return;
    }

    if let Ok(cart) = rustysnes_cart::Cart::load(data) {
        // Exercise the board's own decode, not just detection: `select` chose a board from the
        // chipset byte, and the window/mask arithmetic it derives from ROM-supplied size fields is
        // where an out-of-range value would land. Sample the three bank regions that differ most
        // between LoROM, HiROM, and the coprocessor carts.
        let mut cart = cart;
        for addr in [0x00_8000u32, 0x40_0000, 0x7E_0000, 0xC0_0000, 0xFF_FFFF] {
            let _ = cart.read24(addr, 0);
        }
        // SRAM writes go through the same window arithmetic from the other direction.
        for addr in [0x70_0000u32, 0x30_6000, 0xF0_0000] {
            cart.write24(addr, 0xA5);
        }
        // The coprocessor boards do their arithmetic on tick, not on access.
        cart.coprocessor_tick();
    }
});
