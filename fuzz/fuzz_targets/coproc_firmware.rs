//! Fuzz `Cart::install_coprocessor_firmware` — user-supplied chip-ROM dumps.
//!
//! Coprocessor firmware (`dsp1.rom`, the CX4 data ROM, the ST01x program ROMs) is a separate file
//! the user sources themselves and drops beside the ROM, so it is untrusted input that never went
//! through header detection. It is also the only boundary here that returns a bare `bool` rather
//! than a `Result`: `false` means "this board carries no chip-ROM coprocessor, or the dump is the
//! wrong size", the honesty posture of `docs/adr/0003` — absent the dump the coprocessor is
//! non-functional, never silently degraded.
//!
//! A `bool` return is exactly why this needs fuzzing: there is no error variant to carry a reason,
//! so any input the size check lets through is split straight into the chip's program and data
//! ROMs by fixed offsets.
//!
//! The fixture cart is a plain LoROM with no coprocessor, so most inputs return `false` early. The
//! value is in the size-check boundary itself; a corpus seeded with real dump sizes (2 KiB, 8 KiB,
//! 32 KiB, 128 KiB) is what pushes past it.

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use rustysnes_cart::Cart;

fuzz_target!(|data: &[u8]| {
    let Ok(mut cart) = Cart::load(&common::minimal_lorom()) else {
        return;
    };

    if cart.install_coprocessor_firmware(data) {
        // Accepted: tick the coprocessor so the firmware is actually decoded and executed rather
        // than merely stored. A dump that passes the size check but holds nonsense opcodes faults
        // here, not at install.
        for _ in 0..64 {
            cart.coprocessor_tick();
        }
    }
});
