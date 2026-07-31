//! Fuzz the ROM soft-patching formats — IPS, UPS, and BPS.
//!
//! All three are offset-and-length instruction streams over a base image, which is the shape most
//! prone to "trust a length from the file" bugs. UPS and BPS additionally use variable-length
//! integers, where `1u64 << 70` is a panic in debug and a wrong answer in release — `read_varint`
//! checks the shift *before* shifting, and this target is what keeps that true.
//!
//! `MAX_TARGET` (32 MiB) bounds a hostile patch that declares a multi-gigabyte output. As with
//! `rom_load`, libFuzzer's RSS limit is the real regression detector for that cap.
//!
//! The input is split into a base ROM and a patch so both sides vary: a patch is only interesting
//! against a base whose length interacts with its offsets, and holding the base constant would
//! leave every out-of-range offset failing the same way.
//!
//! Already pinned by unit tests: truncated IPS, an absurd IPS offset, and a UPS/BPS source-size
//! mismatch. This target covers the interior of each format.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_frontend::patch;

fuzz_target!(|data: &[u8]| {
    // Two bytes of length prefix, so the split point is itself fuzzed rather than fixed.
    if data.len() < 2 {
        return;
    }
    let split = usize::from(u16::from_le_bytes([data[0], data[1]])).min(data.len() - 2);
    let (rom, patch_bytes) = data[2..].split_at(split);

    // `detect` is infallible (`Option`) and runs first in the real flow.
    let _ = patch::PatchFormat::detect(patch_bytes);

    // `apply` dispatches on the detected format; call the three directly as well so a format whose
    // magic never appears in the corpus still gets exercised.
    let _ = patch::apply(rom, patch_bytes);
    let _ = patch::apply_ips(rom, patch_bytes);
    let _ = patch::apply_ups(rom, patch_bytes);
    let _ = patch::apply_bps(rom, patch_bytes);
});
