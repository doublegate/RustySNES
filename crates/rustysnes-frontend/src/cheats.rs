//! In-session cheat-code list (Game Genie / Pro Action Replay), `v0.8.0` T-81-003.
//!
//! Cheats are host-applied external input (`docs/adr/0004`), not emulated hardware — the
//! currently-enabled patches are installed into `Bus` via [`Bus::set_cheats`], which checks them
//! on every CPU-visible read (the same point real Game Genie/Pro Action Replay hardware
//! intercepts at). With the `cheats` feature off, or no entries enabled, nothing here executes
//! and the determinism contract is untouched.
//!
//! This is deliberately a **read intercept**, not a `Bus::poke_wram`-style direct write: real
//! Game Genie/Pro Action Replay codes overwhelmingly target cartridge ROM (the SNES equivalent
//! of a pass-through cart, same as NES's own Game Genie), not WRAM — a write-based model would
//! silently do nothing for the vast majority of real codes. Decoding to a `(bank:offset address,
//! value)` patch never touches LoROM/HiROM bank mapping itself — that stays the Bus's own job,
//! same as everything else that reaches it through [`Bus::set_cheats`].
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

/// Install every enabled entry's patch into `bus` (replacing any previously installed set), so
/// [`Bus::read24`](rustysnes_core::cpu::Bus::read24)'s cheat intercept reflects the current
/// list.
///
/// Called once per real frame from the app's drive loop — re-syncing unconditionally (rather
/// than tracking a separate "did the list change" flag) is simpler, and the cost of clearing and
/// re-filling a handful of small `Copy` patches is negligible next to a whole emulated frame.
pub fn sync(entries: &[CheatEntry], bus: &mut Bus) {
    let patches: Vec<CheatPatch> = entries
        .iter()
        .filter(|e| e.enabled)
        .map(|e| e.patch)
        .collect();
    bus.set_cheats(&patches);
}

#[cfg(test)]
mod tests {
    use rustysnes_core::System;
    use rustysnes_core::cpu::Bus as CpuBus;

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
    fn sync_installs_only_enabled_entries() {
        let mut sys = System::new(0);
        let entries = vec![
            CheatEntry {
                code: "A".into(),
                patch: CheatPatch {
                    address: 0x02_B1DD,
                    value: 0x42,
                },
                enabled: true,
            },
            CheatEntry {
                code: "B".into(),
                patch: CheatPatch {
                    address: 0x00_993D,
                    value: 0x99,
                },
                enabled: false,
            },
        ];
        sync(&entries, &mut sys.bus);
        // The enabled entry's ROM-address patch is what the CPU-visible read sees — proving this
        // is a read intercept, not a WRAM poke (a WRAM-only accessor cannot reach these
        // addresses at all).
        assert_eq!(sys.bus.read24(0x02_B1DD), 0x42);
        // The disabled entry's address isn't intercepted, so it reads whatever an unmapped ROM
        // access returns (open bus, not the disabled patch's value).
        assert_ne!(sys.bus.read24(0x00_993D), 0x99);
    }

    #[test]
    fn sync_replaces_the_previously_installed_set() {
        let mut sys = System::new(0);
        sync(
            &[CheatEntry {
                code: "A".into(),
                patch: CheatPatch {
                    address: 0x02_B1DD,
                    value: 0x42,
                },
                enabled: true,
            }],
            &mut sys.bus,
        );
        assert_eq!(sys.bus.read24(0x02_B1DD), 0x42);

        // An empty sync must clear the previous set, not merely leave it stale. A cartless read
        // falls back to the open-bus latch (whatever was last driven), so first drive it to a
        // known, unrelated value — if the cheat were still installed it would force 0x42
        // regardless of what open-bus currently holds; if it's truly cleared, the read reflects
        // that unrelated driven value instead.
        sync(&[], &mut sys.bus);
        sys.bus.write24(0x00_0000, 0x77);
        assert_eq!(sys.bus.read24(0x02_B1DD), 0x77);
    }
}
