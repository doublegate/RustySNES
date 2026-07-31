//! Fuzz `Movie::deserialize` — the TAS movie format.
//!
//! Layout: `RSNESMOV` magic, version, region, determinism seed, ROM SHA-256, a `u32` frame count, a
//! start point that may embed a whole save-state, then a per-frame `(p1: u16, p2: u16)` stream.
//!
//! The `u32` frame count is the classic shape of an OOM denial of service — allocate what the file
//! claims before reading what the file contains. `deserialize` deliberately does *not*
//! `Vec::with_capacity(frame_count)` and grows organically instead, and
//! `deserialize_rejects_a_forged_huge_frame_count_without_oom` pins that. This target is the
//! continuous version of that one test: it looks for the same class anywhere else in the format,
//! including the embedded-save-state branch, which reaches the whole `SaveReader` surface from
//! inside a movie file.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_core::movie::Movie;

fuzz_target!(|data: &[u8]| {
    if let Ok(movie) = Movie::deserialize(data) {
        // Re-serialize: a movie that parsed must be representable. This also walks the whole frame
        // stream, so a count that survived parsing but disagrees with the actual frame data
        // surfaces here rather than during playback.
        let _ = movie.serialize();
    }
});
