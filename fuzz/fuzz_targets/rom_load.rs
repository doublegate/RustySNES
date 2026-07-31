//! Fuzz `EmuCore::load_rom` — the host-facing ROM entry point, **including the zip path**.
//!
//! This is the highest-value target in the set. Unlike every other boundary here, `load_rom` hands
//! attacker-controlled bytes to a third-party crate (`zip`) before any of this project's own code
//! sees them: `extract_rom_bytes` sniffs `PK\x03\x04` / `PK\x05\x06`, opens the archive, walks the
//! central directory, and decompresses the first entry whose extension is in `ROM_EXTENSIONS`.
//!
//! Two properties are under test:
//!
//! 1. **No input panics.** `load_rom` returns `Result`, so a crash is a bug in the decompression
//!    plumbing or in header detection downstream of it.
//! 2. **The zip-bomb ceiling holds.** `MAX_DECOMPRESSED_ROM_SIZE` (32 MiB) is enforced *while*
//!    reading rather than against the declared size, because the declared size is attacker-
//!    controlled. libFuzzer's RSS limit is what actually catches a regression here — if the cap
//!    were removed or moved back to a declared-size check, a small crafted archive would trip
//!    `-rss_limit_mb` rather than returning an error.
//!
//! `libretro`'s `on_load_game` reaches this same function with bytes from a raw host pointer, so
//! this target covers that path's byte-level content too.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_cart::Region;
use rustysnes_core::facade::EmuCore;

fuzz_target!(|data: &[u8]| {
    let mut core = EmuCore::new(0, Region::Ntsc);
    if core.load_rom(data).is_ok() {
        // A ROM that loads is a machine that runs. One frame reaches reset-vector fetch, the
        // scheduler, and every chip — an image whose header parsed but whose size fields are
        // nonsense fails here rather than at load.
        core.run_frame();
    }
});
