//! Fuzz the cheat-code decoders — Game Genie and Pro Action Replay.
//!
//! Codes arrive as text a user pastes from a website, so the input here is a `&str` rather than
//! bytes. `decode` validates shape before content and rejects non-ASCII up front, specifically to
//! avoid a truncating-cast aliasing bug its own comment records; this target is what keeps that
//! rejection honest as the alphabet handling changes.
//!
//! The existing tests (`genie_alphabet_matches_bsnes_and_mesen2_source`, `decodes_real_game_genie_codes`)
//! establish correctness on well-formed codes. This is the adversarial half.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_core::cheat;

fuzz_target!(|data: &[u8]| {
    // Non-UTF-8 is not a finding — the API takes `&str`, so the host has already validated that
    // much. Feed the lossy conversion so byte-level mutation still reaches the decoder with the
    // odd multi-byte character in play.
    let text = String::from_utf8_lossy(data);

    let _ = cheat::decode(&text);
    let _ = cheat::decode_game_genie(&text);
    let _ = cheat::decode_pro_action_replay(&text);
});
