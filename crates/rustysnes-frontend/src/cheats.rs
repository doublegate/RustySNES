//! In-session cheat-code list (Game Genie / Pro Action Replay), `v0.8.0` T-81-003.
//!
//! Cheats are host-applied external input (`docs/adr/0004`), not emulated hardware — a decoded
//! patch is poked into WRAM every frame via [`Bus::poke_wram`] (the same bank/mirror-aware WRAM
//! accessor `rustysnes-script`'s `emu.write` and TAS movie playback both build on), so with the
//! `cheats` feature off, or no entries enabled, the determinism contract is untouched: nothing
//! here executes.
//!
//! Decoding to a `(bank:offset address, value)` patch never touches LoROM/HiROM bank mapping —
//! that is the Bus's own job. A cheat targeting a non-WRAM address is a silent no-op
//! ([`Bus::poke_wram`]'s own documented behavior for an out-of-range address), matching how real
//! Game Genie/Pro Action Replay codes are overwhelmingly WRAM patches in practice.
//!
//! In-memory only for this pass — no per-ROM disk persistence yet, matching this frontend's own
//! quick-save slot's current in-memory-only maturity level. A `RustyNES`-style per-ROM (keyed by
//! ROM SHA-256) TOML file is a natural follow-up once save-states themselves persist to disk.

use rustysnes_core::Bus;
use rustysnes_core::cheat::{self, CheatPatch};

/// One user-entered cheat: the code as typed, its decoded patch, and whether it's active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheatEntry {
    /// The canonical (upper-case) code string, as entered.
    pub code: String,
    /// The decoded `(address, value)` patch.
    pub patch: CheatPatch,
    /// Whether this cheat is currently applied.
    pub enabled: bool,
}

impl CheatEntry {
    /// Parse `code` as a Game Genie or Pro Action Replay code (tried in that order — the two
    /// formats' valid shapes never overlap) and wrap it as a new, enabled entry.
    ///
    /// # Errors
    /// Returns [`cheat::CheatError`] if `code` matches neither format.
    pub fn parse(code: &str) -> Result<Self, cheat::CheatError> {
        let trimmed = code.trim();
        let patch = cheat::decode(trimmed)?;
        Ok(Self {
            code: trimmed.to_ascii_uppercase(),
            patch,
            enabled: true,
        })
    }
}

/// Apply every enabled entry's patch to `bus`'s WRAM.
///
/// Called once per real emulated frame, before it runs, so code that reads the target address
/// during that frame sees the forced value rather than glimpsing one un-patched frame first.
pub fn apply_all(entries: &[CheatEntry], bus: &mut Bus) {
    for entry in entries.iter().filter(|e| e.enabled) {
        bus.poke_wram(entry.patch.address, entry.patch.value);
    }
}

#[cfg(test)]
mod tests {
    use rustysnes_core::System;

    use super::*;

    #[test]
    fn parse_accepts_game_genie_and_pro_action_replay() {
        let gg = CheatEntry::parse("c282-0706").expect("valid GG code");
        assert_eq!(gg.code, "C282-0706");
        assert_eq!(gg.patch.address, 0x02_B1DD);
        assert_eq!(gg.patch.value, 0xAD);
        assert!(gg.enabled);

        let par = CheatEntry::parse(" 7e0a2a06 ").expect("valid PAR code (trims whitespace)");
        assert_eq!(par.code, "7E0A2A06");
        assert_eq!(par.patch.address, 0x7E_0A2A);
        assert_eq!(par.patch.value, 0x06);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(CheatEntry::parse("not a code").is_err());
    }

    #[test]
    fn apply_all_pokes_only_enabled_entries_into_wram() {
        let mut sys = System::new(0);
        let entries = vec![
            CheatEntry {
                code: "A".into(),
                patch: CheatPatch {
                    address: 0x7E_0010,
                    value: 0x42,
                },
                enabled: true,
            },
            CheatEntry {
                code: "B".into(),
                patch: CheatPatch {
                    address: 0x7E_0020,
                    value: 0x99,
                },
                enabled: false,
            },
        ];
        apply_all(&entries, &mut sys.bus);
        assert_eq!(sys.bus.peek_wram(0x7E_0010), 0x42);
        assert_eq!(sys.bus.peek_wram(0x7E_0020), 0x00);
    }

    #[test]
    fn apply_all_reapplies_every_call_overriding_game_writes() {
        // The whole point of a per-frame cheat: even if the game's own code writes a different
        // value in between, the next `apply_all` call forces it back.
        let mut sys = System::new(0);
        let entries = vec![CheatEntry {
            code: "A".into(),
            patch: CheatPatch {
                address: 0x7E_0010,
                value: 0x42,
            },
            enabled: true,
        }];
        apply_all(&entries, &mut sys.bus);
        sys.bus.poke_wram(0x7E_0010, 0x00);
        apply_all(&entries, &mut sys.bus);
        assert_eq!(sys.bus.peek_wram(0x7E_0010), 0x42);
    }
}
