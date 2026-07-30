//! CRC32-keyed game and cheat-code databases (`v1.25.0`, T-FP-G1).
//!
//! Two lookups the frontend can do once a ROM is loaded:
//!
//! - **Game DB** — a canonical title, region, and per-game notes, so the window title and the ROM
//!   Info panel can say "Super Mario World (USA)" instead of whatever 21 bytes the cart's internal
//!   header happens to hold. An internal header is frequently blank, mojibake, or a development
//!   codename; a hash is not.
//! - **Genie DB** — named cheat codes per game, so the cheat panel can offer "Infinite lives"
//!   rather than requiring the user to have the raw code to hand.
//!
//! # Keyed by CRC32, and why that is enough
//!
//! CRC32 is not a cryptographic hash and this is not a security boundary — it identifies a dump
//! against a list the user supplied. It is also what every existing SNES database is keyed by
//! (No-Intro, the Game Genie code lists), so anything else would mean the databases could not be
//! imported. `crc32fast` is already a workspace dependency for the ROM Info panel, so this adds
//! nothing.
//!
//! # Format
//!
//! A tab- or pipe-separated text file, because that is what these lists exist as in the wild and a
//! bespoke binary format would mean writing a converter before the feature is usable at all.
//! Parsing is tolerant of junk lines but **reports how many it skipped**, the same posture
//! [`crate::symbols`] takes — a file that produced nothing must say so.

use std::collections::BTreeMap;

/// One game's entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameEntry {
    /// The canonical title.
    pub title: String,
    /// Region string as the database spells it ("USA", "Europe", "Japan", …).
    pub region: String,
    /// Free-form notes (special chips, known issues).
    pub notes: String,
}

/// One named cheat code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheatCode {
    /// What the code does.
    pub description: String,
    /// The raw code, in whatever form the database used (Game Genie or PAR).
    pub code: String,
}

/// What a load produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LoadStats {
    /// Entries added.
    pub added: usize,
    /// Lines that matched no known form.
    pub skipped: usize,
}

/// A CRC32-keyed game database.
#[derive(Debug, Clone, Default)]
pub struct GameDb {
    entries: BTreeMap<u32, GameEntry>,
}

impl GameDb {
    /// An empty database.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Entries loaded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up a ROM by its CRC32.
    #[must_use]
    pub fn get(&self, crc: u32) -> Option<&GameEntry> {
        self.entries.get(&crc)
    }

    /// Parse database text, adding to what is already loaded.
    ///
    /// Format per line: `CRC32<sep>Title<sep>Region<sep>Notes`, where `<sep>` is a tab or `|`.
    /// Region and notes are optional. `#` starts a comment.
    pub fn load(&mut self, text: &str) -> LoadStats {
        let mut stats = LoadStats::default();
        for raw in text.lines() {
            let line = strip_comment(raw);
            if line.trim().is_empty() {
                continue;
            }
            let mut fields = split_fields(line);
            let Some(crc) = fields.next().and_then(parse_crc) else {
                stats.skipped += 1;
                continue;
            };
            let Some(title) = fields.next().map(str::trim).filter(|t| !t.is_empty()) else {
                // A CRC with no title is not a usable entry — it would show as a blank name, which
                // is worse than falling back to the cart header.
                stats.skipped += 1;
                continue;
            };
            self.entries.insert(
                crc,
                GameEntry {
                    title: title.to_string(),
                    region: fields.next().unwrap_or("").trim().to_string(),
                    notes: fields.next().unwrap_or("").trim().to_string(),
                },
            );
            stats.added += 1;
        }
        stats
    }
}

/// A CRC32-keyed cheat-code database.
#[derive(Debug, Clone, Default)]
pub struct GenieDb {
    codes: BTreeMap<u32, Vec<CheatCode>>,
}

impl GenieDb {
    /// An empty database.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Games with at least one code.
    #[must_use]
    pub fn len(&self) -> usize {
        self.codes.len()
    }

    /// Whether nothing is loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.codes.is_empty()
    }

    /// Every code known for a ROM.
    #[must_use]
    pub fn get(&self, crc: u32) -> &[CheatCode] {
        self.codes.get(&crc).map_or(&[], Vec::as_slice)
    }

    /// Parse code-list text.
    ///
    /// Format per line: `CRC32<sep>Description<sep>Code`. Several lines may share a CRC; they
    /// accumulate rather than replacing, since a game has many codes.
    pub fn load(&mut self, text: &str) -> LoadStats {
        let mut stats = LoadStats::default();
        for raw in text.lines() {
            let line = strip_comment(raw);
            if line.trim().is_empty() {
                continue;
            }
            let mut fields = split_fields(line);
            let Some(crc) = fields.next().and_then(parse_crc) else {
                stats.skipped += 1;
                continue;
            };
            let description = fields.next().map_or("", str::trim);
            let code = fields.next().map_or("", str::trim);
            // Both are required: a code with no description is unusable in a menu, and a
            // description with no code does nothing at all.
            if description.is_empty() || code.is_empty() {
                stats.skipped += 1;
                continue;
            }
            self.codes.entry(crc).or_default().push(CheatCode {
                description: description.to_string(),
                code: code.to_string(),
            });
            stats.added += 1;
        }
        stats
    }
}

/// The CRC32 of a ROM image.
///
/// Used as the database key. Not a security property — see the module doc.
#[must_use]
pub fn rom_crc32(rom: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(rom);
    hasher.finalize()
}

/// Split a line on tabs or pipes.
fn split_fields(line: &str) -> impl Iterator<Item = &str> {
    line.split(['\t', '|'])
}

/// Drop a whole-line `#` comment.
///
/// **Only** a `#` that begins the line (after optional leading whitespace). A `#` anywhere else is
/// data: these files are tab/pipe-separated, and both columns legitimately contain one — game
/// titles (`Bases Loaded #2`) and cheat descriptions (`Player #1 infinite HP`). Splitting on the
/// first `#` silently truncated those rows, which is worse than failing to parse them; trailing
/// comments are simply not a feature of this format, which is the cheaper thing to give up.
fn strip_comment(line: &str) -> &str {
    if line.trim_start().starts_with('#') {
        return "";
    }
    line
}

/// Parse a CRC32 in hex, with or without a `0x`/`$` prefix.
fn parse_crc(field: &str) -> Option<u32> {
    let t = field.trim().trim_start_matches('$');
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(t, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::{GameDb, GenieDb, rom_crc32};

    #[test]
    fn a_game_db_parses_tab_and_pipe_forms() {
        let mut db = GameDb::new();
        let stats = db.load(
            "# comment line\n\
             B19ED489\tSuper Mario World\tUSA\tno special chip\n\
             0x1B4C1C1E|Star Fox|USA|Super FX\n\
             $7C0F0D2B\tPilotwings\tUSA\n\
             \n",
        );
        assert_eq!(stats.added, 3);
        assert_eq!(stats.skipped, 0);
        let smw = db.get(0xB19E_D489).expect("smw");
        assert_eq!(smw.title, "Super Mario World");
        assert_eq!(smw.region, "USA");
        assert_eq!(smw.notes, "no special chip");
        assert_eq!(db.get(0x1B4C_1C1E).expect("sf").notes, "Super FX");
        // Optional trailing fields simply come back empty.
        let pw = db.get(0x7C0F_0D2B).expect("pw");
        assert_eq!(pw.region, "USA");
        assert!(pw.notes.is_empty());
        assert!(db.get(0xDEAD_BEEF).is_none());
    }

    /// Tolerant of junk, but never silent about it — a file that produced nothing must say so.
    #[test]
    fn unparsable_lines_are_counted() {
        let mut db = GameDb::new();
        let stats = db.load(
            "not a crc\tSomething\n\
             ZZZZZZZZ\tBad hex\n\
             B19ED489\n\
             B19ED489\t\n\
             AABBCCDD\tFine\n",
        );
        assert_eq!(stats.added, 1, "only the last line is usable");
        assert_eq!(stats.skipped, 4);
        assert_eq!(db.len(), 1);
        // A CRC with no title is skipped rather than stored blank — a blank name in the UI is
        // worse than falling back to the cart header.
        assert!(db.get(0xB19E_D489).is_none());
    }

    #[test]
    fn a_genie_db_accumulates_codes_per_game() {
        let mut db = GenieDb::new();
        let stats = db.load(
            "B19ED489\tInfinite lives\tDDB1-14A7\n\
             B19ED489\tInvincibility\tC2B1-64A7\n\
             AABBCCDD\tStart with 9 coins\t1234-5678\n",
        );
        assert_eq!(stats.added, 3);
        assert_eq!(db.len(), 2, "two games");
        let smw = db.get(0xB19E_D489);
        assert_eq!(smw.len(), 2, "codes accumulate rather than replacing");
        assert_eq!(smw[0].description, "Infinite lives");
        assert_eq!(smw[1].code, "C2B1-64A7");
        assert!(db.get(0x0000_0000).is_empty(), "unknown game, no codes");
    }

    /// A code with no description is unusable in a menu; a description with no code does nothing.
    /// Both are refused rather than stored half-formed.
    #[test]
    fn incomplete_codes_are_refused() {
        let mut db = GenieDb::new();
        let stats = db.load(
            "B19ED489\tDescription only\n\
             B19ED489\t\tDDB1-14A7\n\
             B19ED489\tGood\tDDB1-14A7\n",
        );
        assert_eq!(stats.added, 1);
        assert_eq!(stats.skipped, 2);
        assert_eq!(db.get(0xB19E_D489).len(), 1);
    }

    /// Loading is additive, so several lists can be layered.
    #[test]
    fn loads_are_additive() {
        let mut db = GameDb::new();
        db.load("AABBCCDD\tFirst\n");
        db.load("11223344\tSecond\n");
        assert_eq!(db.len(), 2);
        // A repeated CRC replaces, so a later list can correct an earlier one.
        db.load("AABBCCDD\tCorrected\n");
        assert_eq!(db.len(), 2);
        assert_eq!(db.get(0xAABB_CCDD).expect("e").title, "Corrected");
        assert!(!db.is_empty());
    }

    /// The key must be the standard CRC32 every existing SNES database uses, or none of them can
    /// be imported.
    #[test]
    fn rom_crc32_matches_the_standard_algorithm() {
        // The canonical CRC-32/ISO-HDLC check value for "123456789".
        assert_eq!(rom_crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(rom_crc32(b""), 0);
        assert_ne!(rom_crc32(b"a"), rom_crc32(b"b"));
    }

    /// A `#` inside a field is data, not a comment. Titles and cheat descriptions contain them —
    /// `Bases Loaded #2`, `Player #1 infinite HP` — and a bare split truncated the row silently,
    /// which is worse than failing to parse it.
    #[test]
    fn a_hash_inside_a_field_is_not_a_comment() {
        assert_eq!(
            super::strip_comment("A\tBases Loaded #2"),
            "A\tBases Loaded #2"
        );
        assert_eq!(
            super::strip_comment("code|Player #1 HP"),
            "code|Player #1 HP"
        );
        // Even directly after whitespace: a field can and does start with one.
        assert_eq!(super::strip_comment("A\t#1 Ranked"), "A\t#1 Ranked");
        // Only a line that OPENS with `#` is a comment; leading whitespace is allowed.
        assert_eq!(super::strip_comment("# whole line"), "");
        assert_eq!(super::strip_comment("   \t# indented"), "");
        assert_eq!(super::strip_comment("no comment here"), "no comment here");
    }
}
