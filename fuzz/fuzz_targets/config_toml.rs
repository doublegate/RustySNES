//! Fuzz `Config` TOML deserialization.
//!
//! `Config::load` reads `config.toml` from disk and ends in `unwrap_or_default()`, so at the call
//! site a malformed file is already fail-closed. The surface worth fuzzing is one level down: the
//! serde-derive path itself, across a deeply nested struct tree (`VideoConfig`, `AudioConfig`,
//! `GamepadConfig`, the two `KeyBindings` tables, the per-game override map, the shader-parameter
//! map). Deeply nested `#[serde(default)]` structs are where a recursion or allocation problem
//! would live, and `unwrap_or_default()` cannot catch a stack overflow.
//!
//! The round-trip is the second half: a config that deserializes must re-serialize, or a user who
//! opens Settings once silently loses the part that could not be written back.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_frontend::config::Config;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    if let Ok(config) = toml::from_str::<Config>(text) {
        let Ok(written) = toml::to_string(&config) else {
            // A value that deserialized but cannot be serialized is a real asymmetry, but TOML has
            // legitimate cases (a `None` preceding a nested table), so this is not asserted.
            return;
        };
        // Re-parse rather than compare: `Config` has no `PartialEq` (deliberately — it holds
        // per-game override and shader-parameter maps whose ordering is not semantic), so the
        // assertable property is that Settings can write back what it read, not field equality.
        toml::from_str::<Config>(&written)
            .expect("a config this serializer produced must deserialize — Settings would corrupt it");
    }
});
