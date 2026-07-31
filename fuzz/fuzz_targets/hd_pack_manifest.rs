//! Fuzz the HD-pack `pack.toml` manifest.
//!
//! An HD texture pack is a directory a user downloads and points the emulator at, so its manifest
//! is untrusted input in the ordinary way.
//!
//! **Scope limit, stated rather than hidden.** `HdPack::load` is filesystem-bound (it reads
//! `pack.toml` and then each referenced PNG from disk), and the pure helpers underneath it —
//! `resolve_tile_image_path`'s path-traversal guard and `rgba8_from_frame`'s colour-type
//! normalizer — are private. This target therefore reaches the manifest deserialization and
//! `hash_value` only. Making those two helpers public purely so a fuzz target could call them
//! would be test scaffolding in the shipped API; the four unit tests that already cover them
//! (including `load_rejects_a_tile_image_path_that_escapes_the_pack_directory`) stay the guard
//! there, and PNG decoding itself belongs to the third-party `png` crate, which has its own.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_frontend::hd_pack::HdPackManifest;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    if let Ok(manifest) = toml::from_str::<HdPackManifest>(text) {
        // `hash_value` parses each tile's hash out of a manifest-supplied hex string. It returns
        // `Option`, so a crash here is a slice or radix bug rather than a bad-hash case.
        for tile in &manifest.tiles {
            let _ = tile.hash_value();
        }

        let Ok(written) = toml::to_string(&manifest) else {
            return;
        };
        toml::from_str::<HdPackManifest>(&written)
            .expect("a manifest this serializer produced must deserialize");
    }
});
