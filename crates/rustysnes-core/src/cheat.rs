//! SNES cheat-code decoding: Game Genie and Pro Action Replay (`v0.8.0 "Instrumentation"`,
//! T-81-003).
//!
//! Ported from bsnes's `CheatEditor::decodeSNES` (`ref-proj/bsnes/bsnes/target-bsnes/tools/
//! cheat-editor.cpp`) and cross-checked bit-for-bit against Mesen2's independent
//! `CheatManager::ConvertFromSnesGameGenie`/`ConvertFromSnesProActionReplay`
//! (`ref-proj/Mesen2/Core/Shared/CheatManager.cpp`) — both codebases compute an identical 24-bit
//! address and value byte for any given code string. Test vectors below are real commercial
//! codes drawn from Mesen2's shipped cheat database (`ref-proj/Mesen2/UI/Dependencies/Internal/
//! CheatDb.Snes.json`), decoded independently by hand against the bit formula as a third check.
//!
//! Both formats decode to a plain 24-bit CPU-bus address (`$bank:offset`) plus an 8-bit
//! substitute value — no LoROM/HiROM bank translation happens here (that is the Bus's normal
//! memory-map job, same as [`crate::Bus::poke_wram`]/[`crate::Bus::peek_wram`]). Neither SNES
//! format supports a compare byte (unlike NES's 8-character Game Genie) — a decoded cheat is
//! always an unconditional address/value substitution.
//!
//! A cheat is host-applied external input, not emulated hardware behavior (`docs/adr/0004`) — it
//! is not part of any save state and is not evaluated unless the frontend's `cheats` feature is
//! on and a patch is actually applied, so the determinism contract is untouched when no cheat is
//! active.

/// The SNES Game Genie's 16-character alphabet; a character's position in this string is its
/// decoded nibble value (`D` = 0, `F` = 1, ... `E` = 15).
const GENIE_ALPHABET: &[u8; 16] = b"DF4709156BC8A23E";

/// Error decoding a cheat-code string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CheatError {
    /// Not a recognized Game Genie (`XXXX-XXXX`, 9 characters) or Pro Action Replay (8 hex
    /// characters) shape.
    #[error("not a recognized SNES Game Genie or Pro Action Replay code")]
    UnrecognizedFormat,
    /// Contained a character outside the expected alphabet for its detected format.
    #[error("invalid cheat-code character '{0}'")]
    InvalidCharacter(char),
}

/// A decoded cheat patch: substitute `value` at CPU-bus `address` (`$bank:offset`, 24-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheatPatch {
    /// The 24-bit CPU-bus address (`$bank:offset`) this patch targets.
    pub address: u32,
    /// The substitute byte value.
    pub value: u8,
}

/// Map a Game Genie character to its 4-bit nibble (case-insensitive).
///
/// Rejects non-ASCII input outright rather than truncating it to a `u8` — a truncating cast
/// (`c as u8`) could alias an unrelated non-ASCII codepoint onto a valid alphabet byte (e.g.
/// `'\u{0144}'` truncates to `0x44`, `'D'`) and falsely "succeed" decoding garbage input.
fn genie_nibble(c: char) -> Option<u8> {
    if !c.is_ascii() {
        return None;
    }
    let upper = c.to_ascii_uppercase() as u8;
    u8::try_from(GENIE_ALPHABET.iter().position(|&a| a == upper)?).ok()
}

/// Decode a Game Genie code: `XXXX-XXXX` (case-insensitive, a dash at index 4, 9 characters
/// total).
///
/// # Errors
/// Returns [`CheatError::UnrecognizedFormat`] if `code` isn't 9 characters with a dash at index
/// 4, or [`CheatError::InvalidCharacter`] if a non-dash character is outside the Game Genie
/// alphabet.
pub fn decode_game_genie(code: &str) -> Result<CheatPatch, CheatError> {
    // Determine the shape (length + dash position) BEFORE validating any character content —
    // `decode`'s fallback to Pro Action Replay depends on `UnrecognizedFormat` meaning "this
    // wasn't shaped like a Game Genie code at all," not "the first bad character happened to be
    // found before an eventual length mismatch would have been noticed." Two passes over the
    // iterator, but no heap allocation (an earlier `Vec<char>` collect was flagged for exactly
    // that unnecessary no_std allocation).
    if code.chars().count() != 9 || code.chars().nth(4) != Some('-') {
        return Err(CheatError::UnrecognizedFormat);
    }
    let mut raw: u32 = 0;
    for (i, c) in code.chars().enumerate() {
        if i == 4 {
            continue;
        }
        let nibble = genie_nibble(c).ok_or(CheatError::InvalidCharacter(c))?;
        raw = (raw << 4) | u32::from(nibble);
    }

    // bsnes `CheatEditor::decodeSNES`'s bit-scramble, verbatim (each destination address bit's
    // source mask/shift in `raw`, low 24 bits only — the top byte of `raw` is the value below).
    let bit = |mask: u32| u32::from(raw & mask != 0);
    let address = (bit(0x00_2000) << 23)
        | (bit(0x00_1000) << 22)
        | (bit(0x00_0800) << 21)
        | (bit(0x00_0400) << 20)
        | (bit(0x00_0020) << 19)
        | (bit(0x00_0010) << 18)
        | (bit(0x00_0008) << 17)
        | (bit(0x00_0004) << 16)
        | (bit(0x80_0000) << 15)
        | (bit(0x40_0000) << 14)
        | (bit(0x20_0000) << 13)
        | (bit(0x10_0000) << 12)
        | (bit(0x00_0002) << 11)
        | (bit(0x00_0001) << 10)
        | (bit(0x00_8000) << 9)
        | (bit(0x00_4000) << 8)
        | (bit(0x08_0000) << 7)
        | (bit(0x04_0000) << 6)
        | (bit(0x02_0000) << 5)
        | (bit(0x01_0000) << 4)
        | (bit(0x00_0200) << 3)
        | (bit(0x00_0100) << 2)
        | (bit(0x00_0080) << 1)
        | bit(0x00_0040);
    // `raw >> 24` is always in 0..=255 (`raw` is packed from exactly 8 nibbles).
    #[allow(clippy::cast_possible_truncation)]
    let value = (raw >> 24) as u8;

    Ok(CheatPatch { address, value })
}

/// Decode a Pro Action Replay code: 8 hex digits (case-insensitive), no scrambling —
/// `AAAAAADD` (6 hex-digit address, high; 2 hex-digit value, low).
///
/// # Errors
/// Returns [`CheatError::UnrecognizedFormat`] if `code` isn't 8 characters, or
/// [`CheatError::InvalidCharacter`] if a character isn't a hex digit.
pub fn decode_pro_action_replay(code: &str) -> Result<CheatPatch, CheatError> {
    // Shape (length) first, content second — see `decode_game_genie`'s doc comment for why.
    if code.chars().count() != 8 {
        return Err(CheatError::UnrecognizedFormat);
    }
    let mut raw: u32 = 0;
    for c in code.chars() {
        let nibble = c.to_digit(16).ok_or(CheatError::InvalidCharacter(c))?;
        raw = (raw << 4) | nibble;
    }
    // `raw & 0xFF` is always in 0..=255 by construction.
    #[allow(clippy::cast_possible_truncation)]
    let value = (raw & 0xFF) as u8;
    Ok(CheatPatch {
        address: raw >> 8,
        value,
    })
}

/// Decode `code` as a Game Genie code, falling back to Pro Action Replay only when `code`
/// doesn't match the Game Genie shape at all.
///
/// The two formats' valid shapes never overlap (9 characters with a dash vs. exactly 8 hex
/// digits), so this dispatch is unambiguous. Only [`CheatError::UnrecognizedFormat`] falls
/// through to the Pro Action Replay decoder — a Game Genie–shaped code with a genuinely invalid
/// character (e.g. `C282-070G`) returns that specific [`CheatError::InvalidCharacter`] instead
/// of a misleading "wrong format" from a decoder that was never going to match its shape either.
///
/// # Errors
/// Returns the [`CheatError`] from whichever format `code`'s length suggests; if neither format
/// recognizes the shape at all, returns [`CheatError::UnrecognizedFormat`].
pub fn decode(code: &str) -> Result<CheatPatch, CheatError> {
    match decode_game_genie(code) {
        Err(CheatError::UnrecognizedFormat) => decode_pro_action_replay(code),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genie_alphabet_matches_bsnes_and_mesen2_source() {
        // "DF4709156BC8A23E", position = value — confirmed identical in both reference decoders.
        let expected: [(char, u8); 16] = [
            ('D', 0),
            ('F', 1),
            ('4', 2),
            ('7', 3),
            ('0', 4),
            ('9', 5),
            ('1', 6),
            ('5', 7),
            ('6', 8),
            ('B', 9),
            ('C', 10),
            ('8', 11),
            ('A', 12),
            ('2', 13),
            ('3', 14),
            ('E', 15),
        ];
        for (c, v) in expected {
            assert_eq!(genie_nibble(c), Some(v));
            assert_eq!(genie_nibble(c.to_ascii_lowercase()), Some(v));
        }
    }

    // Real commercial Game Genie codes from Mesen2's shipped `CheatDb.Snes.json`, decoded by
    // hand against the bit-scramble formula as an independent third check (see the module doc).
    #[test]
    fn decodes_real_game_genie_codes() {
        let gc = decode_game_genie("C282-0706").expect("valid code");
        assert_eq!(gc.address, 0x02_B1DD);
        assert_eq!(gc.value, 0xAD);

        let gc = decode_game_genie("DBB7-0704").expect("valid code");
        assert_eq!(gc.address, 0x00_993D);
        assert_eq!(gc.value, 0x09);
    }

    #[test]
    fn game_genie_is_case_insensitive() {
        let upper = decode_game_genie("C282-0706").unwrap();
        let lower = decode_game_genie("c282-0706").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn game_genie_rejects_bad_shape() {
        assert_eq!(
            decode_game_genie("C2820706"),
            Err(CheatError::UnrecognizedFormat)
        );
        assert_eq!(
            decode_game_genie("C282_0706"),
            Err(CheatError::UnrecognizedFormat)
        );
        assert_eq!(
            decode_game_genie("C282-070"),
            Err(CheatError::UnrecognizedFormat)
        );
        assert_eq!(
            decode_game_genie("W282-0706"),
            Err(CheatError::InvalidCharacter('W'))
        );
    }

    // Real commercial Pro Action Replay / raw-hex codes from the same database — all land in
    // WRAM ($7E0000-$7FFFFF), matching the module doc's WRAM-cheat framing.
    #[test]
    fn decodes_real_pro_action_replay_codes() {
        let pc = decode_pro_action_replay("7E0A2A06").expect("valid code");
        assert_eq!(pc.address, 0x7E_0A2A);
        assert_eq!(pc.value, 0x06);

        let pc = decode_pro_action_replay("7E1E6B14").expect("valid code");
        assert_eq!(pc.address, 0x7E_1E6B);
        assert_eq!(pc.value, 0x14);
    }

    #[test]
    fn pro_action_replay_is_case_insensitive() {
        let upper = decode_pro_action_replay("7E0A2A06").unwrap();
        let lower = decode_pro_action_replay("7e0a2a06").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn pro_action_replay_rejects_bad_shape() {
        assert_eq!(
            decode_pro_action_replay("7E0A2A0"),
            Err(CheatError::UnrecognizedFormat)
        );
        assert_eq!(
            decode_pro_action_replay("7E0A2A0G"),
            Err(CheatError::InvalidCharacter('G'))
        );
    }

    #[test]
    fn unified_decode_dispatches_to_the_matching_format() {
        assert_eq!(
            decode("C282-0706").unwrap(),
            decode_game_genie("C282-0706").unwrap()
        );
        assert_eq!(
            decode("7E0A2A06").unwrap(),
            decode_pro_action_replay("7E0A2A06").unwrap()
        );
        assert_eq!(decode("not a code"), Err(CheatError::UnrecognizedFormat));
    }

    #[test]
    fn unified_decode_does_not_mask_a_genuine_game_genie_character_error() {
        // "C282-070G" is Game-Genie-shaped (9 chars, dash at index 4) but 'G' isn't in the
        // alphabet — `decode` must surface that specific error, not silently fall through to
        // Pro Action Replay (which would also fail, but with a less useful "wrong format").
        assert_eq!(decode("C282-070G"), Err(CheatError::InvalidCharacter('G')));
    }

    #[test]
    fn genie_nibble_rejects_non_ascii_rather_than_truncating() {
        // '\u{0144}' truncates to 0x44 ('D') under a lossy `as u8` cast — must be rejected, not
        // silently aliased onto a valid alphabet character.
        assert_eq!(genie_nibble('\u{0144}'), None);
    }
}
