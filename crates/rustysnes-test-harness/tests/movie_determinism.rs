#![allow(missing_docs)]
//! The TAS movie determinism test — T-81-002's "replaying a recorded movie against the same ROM
//! + save-state-at-frame-0 seeding produces a bit-identical framebuffer/audio trace" acceptance criterion.
//!
//! Mirrors `save_state_determinism.rs`'s exact pattern: record a run with varying synthetic
//! input (not a static all-zero pad — a movie that never actually exercises input divergence
//! would pass trivially even with a broken recorder), serialize the movie to bytes and back
//! (round-tripping through the real on-disk format, not just the in-memory struct), replay it
//! against a FRESH `System` from its power-on start point, and assert the framebuffer + drained
//! audio are byte-identical to the original run, frame for frame.
#![cfg(feature = "test-roms")]

use std::path::PathBuf;

use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use rustysnes_core::movie::{Movie, MoviePlayer, MovieRecorder};

/// Frames to record/replay — enough to exercise several distinct input patterns, short enough to
/// keep the test fast.
const FRAME_COUNT: u32 = 40;
/// The determinism seed both the recording and the replay use (`Movie::seek_to_start` verifies
/// this matches before booting a `PowerOn` movie — see `movie.rs`'s own doc).
const SEED: u64 = 12345;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn hash_audio(samples: &[(i16, i16)]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &(l, r) in samples {
        h ^= u64::from(l.cast_unsigned());
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= u64::from(r.cast_unsigned());
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A varying (not constant) synthetic input pattern per frame — proves the replay actually
/// reconstructs per-frame INPUT-dependent state, not just a static boot screen. `p2` stays 0
/// (this ROM is single-player); `p1` cycles through a handful of distinct bit patterns.
const fn input_for_frame(frame: u32) -> (u16, u16) {
    const PATTERNS: [u16; 4] = [0x8000, 0x0000, 0x0808, 0xFFF0];
    (PATTERNS[frame as usize % PATTERNS.len()], 0)
}

/// Boot the committed no-coprocessor gilyon ROM fresh with `SEED`, installed but not yet reset.
fn fresh_system() -> System {
    let rom_path = workspace_root().join("tests/roms/gilyon/cputest/cputest-basic.sfc");
    let rom =
        std::fs::read(&rom_path).unwrap_or_else(|e| panic!("read {}: {e}", rom_path.display()));
    let cart = Cart::from_rom(&rom).unwrap_or_else(|e| panic!("parse cart: {e:?}"));
    let mut sys = System::new(SEED);
    sys.bus.cart = Some(cart);
    sys
}

#[test]
fn movie_replay_is_byte_identical_to_the_original_recording() {
    // --- Record the original run. ---
    let mut original = fresh_system();
    let rom_path = workspace_root().join("tests/roms/gilyon/cputest/cputest-basic.sfc");
    let rom = std::fs::read(&rom_path).expect("read rom for hashing");
    let region = original
        .bus
        .cart
        .as_ref()
        .map_or(rustysnes_core::cart::Region::Ntsc, |c| c.header.region);
    let mut recorder = MovieRecorder::power_on(SEED, region, &rom);

    let mut fb_orig = Vec::new();
    let mut audio_orig = Vec::new();
    for frame in 0..FRAME_COUNT {
        let (p1, p2) = input_for_frame(frame);
        original.bus.set_joypad(0, p1);
        original.bus.set_joypad(1, p2);
        recorder.capture(p1, p2);
        original.run_frame();
        fb_orig.push(hash_fb(original.bus.framebuffer()));
        original.bus.apu.drain_audio(&mut audio_orig);
    }
    assert_eq!(recorder.frame_count(), FRAME_COUNT as usize);
    let movie = recorder.finish();

    // --- Round-trip the movie through its real on-disk byte format. ---
    let bytes = movie.serialize();
    let movie = Movie::deserialize(&bytes).expect("movie round-trips through its byte format");
    movie.verify_rom(&rom).expect("recorded ROM hash matches");

    // --- Replay against a FRESH System, from the movie's power-on start point. ---
    let mut replay = fresh_system();
    movie
        .seek_to_start(&mut replay)
        .expect("seek_to_start succeeds (same seed, cart installed, never stepped)");
    let mut player = MoviePlayer::new(movie);

    let mut fb_replay = Vec::new();
    let mut audio_replay = Vec::new();
    while let Some(f) = player.next_frame() {
        replay.bus.set_joypad(0, f.p1);
        replay.bus.set_joypad(1, f.p2);
        replay.run_frame();
        fb_replay.push(hash_fb(replay.bus.framebuffer()));
        replay.bus.apu.drain_audio(&mut audio_replay);
    }
    assert!(player.is_finished());

    // --- Compare, frame index included in the panic message for an easy first-divergence read. ---
    assert_eq!(
        fb_orig.len(),
        fb_replay.len(),
        "replay produced a different number of frames than the recording"
    );
    for (i, (orig, replay)) in fb_orig.iter().zip(fb_replay.iter()).enumerate() {
        assert_eq!(
            orig, replay,
            "framebuffer diverged at replayed frame {i} (orig={orig:#018x} replay={replay:#018x})"
        );
    }
    assert_eq!(
        hash_audio(&audio_orig),
        hash_audio(&audio_replay),
        "audio diverged between the original recording and the replay ({} vs {} samples)",
        audio_orig.len(),
        audio_replay.len()
    );
}

#[test]
fn seed_mismatch_is_rejected_before_any_replay_happens() {
    let mut original = fresh_system();
    let rom_path = workspace_root().join("tests/roms/gilyon/cputest/cputest-basic.sfc");
    let rom = std::fs::read(&rom_path).expect("read rom");
    let region = original
        .bus
        .cart
        .as_ref()
        .map_or(rustysnes_core::cart::Region::Ntsc, |c| c.header.region);
    let mut recorder = MovieRecorder::power_on(SEED, region, &rom);
    recorder.capture(0x8000, 0);
    original.run_frame();
    let movie = recorder.finish();

    // A System built with a DIFFERENT seed than the movie was recorded with.
    let mut wrong_seed = System::new(SEED + 1);
    wrong_seed.bus.cart = Cart::from_rom(&rom).ok();
    let err = movie
        .seek_to_start(&mut wrong_seed)
        .expect_err("a seed mismatch must be rejected, not silently replayed wrong");
    assert!(matches!(
        err,
        rustysnes_core::movie::MovieError::SeedMismatch { .. }
    ));
}
