//! Fuzz `SymbolMap::load` — the debugger's symbol/source-map reader.
//!
//! The odd one out in this set: `load` returns no `Result` at all. It is deliberately tolerant —
//! unrecognized lines are skipped and counted in `LoadStats.skipped` — because a `.sym` file from
//! WLA-DX, bass, or no$sns is a best-effort convenience, not a contract.
//!
//! That makes panic-freedom the *entire* specification. There is no error path to fall back on: any
//! input that does not produce a `LoadStats` is a crash in a debugger the user opened on a file
//! they downloaded. Nothing else about the output is asserted, because "skipped" is a legitimate
//! answer for every line.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_frontend::symbols::SymbolMap;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);

    let mut map = SymbolMap::new();
    let _stats = map.load(&text);

    // Loading twice must be as safe as loading once: the panel reloads a map in place when the
    // file changes on disk, so the second load runs against non-empty state.
    let _stats = map.load(&text);
});
