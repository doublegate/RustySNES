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

/// Instructions to execute after a successful load.
///
/// A ROM that loads is a machine that runs, and an image whose header parsed but whose size fields
/// are nonsense faults on the first fetch through them rather than at load — so *some* execution is
/// worth doing. This was `run_frame()` (~357,366 master clocks), and the budget replaced it on a
/// controlled measurement: 300 runs over one fixed seed, everything else identical.
///
/// | after load | exec/s | wall | edges |
/// |---|---:|---:|---:|
/// | `run_frame()` | 37 | 8 s | 1883 |
/// | 2048 instructions | 100 | 3 s | 1760 |
///
/// So **2.7x throughput for ~120 edges**, not the three-orders-of-magnitude difference a naive
/// comparison against `rom_header` suggests — that reading is confounded by corpus size, since
/// libFuzzer's cumulative exec/s is dominated by replaying a few hundred multi-megabyte seeds.
///
/// The trade is taken because the edges given up are the vblank/NMI/HDMA frame paths, which
/// `cpu_step` and `apu_port_io` reach directly and far more cheaply, whereas this target's own
/// subject — the zip container, header detection, and the board's window arithmetic — is fully
/// covered inside the budget.
///
/// (`EmuCore` exposes no cycle-budget entry point, so this counts instructions via
/// `System::step_instruction`, not cycles.)
const STEP_BUDGET: usize = 2048;

fuzz_target!(|data: &[u8]| {
    let mut core = EmuCore::new(0, Region::Ntsc);
    if core.load_rom(data).is_ok() {
        let sys = core.system_mut();
        for _ in 0..STEP_BUDGET {
            sys.step_instruction();
        }
    }
});
