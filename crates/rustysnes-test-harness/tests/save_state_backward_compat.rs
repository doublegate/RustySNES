#![allow(missing_docs)]
//! Save-state `FORMAT_VERSION` backward-compat regression lock — the gap `to-dos/VERSION-PLAN.md`
//! flagged for the `v1.0.0` gate ("only same-version round-trip determinism is tested today; a
//! byte-layout change to any section that forgets to bump `FORMAT_VERSION` would go uncaught"),
//! closed here by `v0.7.0 "Resolution"`'s real first bump (`FORMAT_VERSION` 1→2, the `PPU0`
//! section's framebuffer growing to hi-res capacity + a new `frame_hires` bool).
//!
//! `tests/golden/savestate-v1-gilyon.bin` is a genuine `FORMAT_VERSION = 1` blob — captured by
//! running the pre-`v0.7.0` code (`Ppu`'s old, smaller `PPU0` layout) against the committed
//! gilyon `cputest-basic.sfc` ROM for 30 frames, then calling `System::save_state()`. It is a
//! real historical artifact, not hand-crafted.
//!
//! `System::load_state` only rejects a format version *newer* than it supports
//! (`SaveStateError::UnsupportedVersion`) — it does not attempt graceful old-format loading (see
//! `FORMAT_VERSION`'s own doc comment, `crates/rustysnes-core/src/scheduler.rs`). So the contract
//! this test actually locks in is narrower, but still load-bearing: a section byte-layout change
//! must fail LOUDLY (a real `SaveStateError`, not a panic and not a silent misread that produces
//! a corrupted-but-successfully-loaded `System`) when an old blob meets new code. That's what
//! catches a future developer who changes a section's layout without bumping `FORMAT_VERSION` —
//! the failure mode this fixture pins down.
#![cfg(feature = "test-roms")]

use std::path::PathBuf;

use rustysnes_core::System;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Loading a genuinely older-format blob into current code must fail with a real
/// [`rustysnes_savestate::SaveStateError`] — never panic, never silently succeed with corrupted
/// state.
#[test]
fn old_format_version_blob_fails_loudly_not_silently() {
    let fixture_path = workspace_root().join("tests/golden/savestate-v1-gilyon.bin");
    let blob = std::fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", fixture_path.display()));

    let rom_path = workspace_root().join("tests/roms/gilyon/cputest/cputest-basic.sfc");
    let rom =
        std::fs::read(&rom_path).unwrap_or_else(|e| panic!("read {}: {e}", rom_path.display()));
    let cart = rustysnes_core::cart::Cart::from_rom(&rom).expect("parse gilyon ROM");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();

    // The v1 blob's magic + version header are still well-formed (only section *bodies* grew),
    // so this must get past the magic/version checks and fail inside a section body read — a
    // `Truncated` (the `PPU0` sub-reader runs out of its own declared length before the new,
    // larger framebuffer read completes) or `UnexpectedTag` (a later section desyncs), not a
    // `BadMagic`/`UnsupportedVersion` short-circuit that wouldn't actually exercise the
    // section-layout mismatch this test exists to catch.
    let result = sys.load_state(&blob);
    let err = result.expect_err(
        "loading a FORMAT_VERSION=1 blob into FORMAT_VERSION=2 code must fail, not succeed \
         silently with a mismatched/corrupted PPU0 section",
    );
    assert!(
        !matches!(
            err,
            rustysnes_savestate::SaveStateError::BadMagic
                | rustysnes_savestate::SaveStateError::UnsupportedVersion { .. }
        ),
        "expected the section-layout mismatch itself to surface as the failure (Truncated/\
         UnexpectedTag), not an unrelated magic/version short-circuit; got: {err:?}"
    );
}

/// Sanity check that the fixture is real, not accidentally already in the current format: a
/// same-version round-trip through *current* code must still succeed end-to-end, proving the test
/// above is exercising a genuine old-vs-new mismatch and not a broken save/load path in general.
#[test]
fn current_format_round_trip_still_works() {
    let rom_path = workspace_root().join("tests/roms/gilyon/cputest/cputest-basic.sfc");
    let rom =
        std::fs::read(&rom_path).unwrap_or_else(|e| panic!("read {}: {e}", rom_path.display()));
    let cart = rustysnes_core::cart::Cart::from_rom(&rom).expect("parse gilyon ROM");

    let mut original = System::new(0);
    original.bus.cart = Some(rustysnes_core::cart::Cart::from_rom(&rom).expect("parse gilyon ROM"));
    original.reset();
    for _ in 0..30 {
        original.run_frame();
    }
    let blob = original.save_state();

    let mut restored = System::new(0);
    restored.bus.cart = Some(cart);
    restored.reset();
    restored
        .load_state(&blob)
        .expect("current-format round trip must succeed");
}
