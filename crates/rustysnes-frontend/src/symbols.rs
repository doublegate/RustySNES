//! Symbol maps: names for addresses (`v1.25.0`, T-FP-C2).
//!
//! A disassembly of `JSR $9A3C` says nothing; `JSR update_sprites` says what the program is doing.
//! For homebrew and for any ROM someone has mapped by hand, that difference is most of what makes a
//! debugger usable, and the map already exists as a file the assembler emitted.
//!
//! # Formats
//!
//! Two, both line-based and both widely produced:
//!
//! - **`.sym` (WLA-DX / bass / no$sns)** — `[labels]` sections with `BB:AAAA name` lines. This is
//!   what `wla-65816` and most SNES homebrew toolchains emit.
//! - **`.cpu`-style flat** — `AAAAAA name` or `name = $AAAAAA`, the shape hand-written maps take.
//!
//! Parsing is **tolerant by design**: an unrecognised line is skipped, not an error. A symbol file
//! is an aid, and refusing to load 4,000 good symbols because line 12 has a stray directive would
//! trade the whole feature for pedantry. What is *not* tolerated is silence — [`SymbolMap::load`]
//! reports how many lines it skipped, so a file that produced nothing says so.

use std::collections::BTreeMap;

/// Address-to-name mappings for the debugger.
#[derive(Debug, Clone, Default)]
pub struct SymbolMap {
    /// Sorted by address, so [`Self::nearest`] can binary-search.
    by_addr: BTreeMap<u32, String>,
}

/// What a load produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadStats {
    /// Symbols added.
    pub added: usize,
    /// Lines that matched no known form and were skipped.
    pub skipped: usize,
}

impl SymbolMap {
    /// An empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many symbols are loaded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_addr.len()
    }

    /// Whether no symbols are loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_addr.is_empty()
    }

    /// Forget every symbol.
    pub fn clear(&mut self) {
        self.by_addr.clear();
    }

    /// The exact name for an address, if one exists.
    #[must_use]
    pub fn name(&self, addr: u32) -> Option<&str> {
        self.by_addr.get(&addr).map(String::as_str)
    }

    /// The nearest symbol at or below `addr`, with the offset from it.
    ///
    /// This is what turns an arbitrary PC into `update_sprites+7`, which is the form that is
    /// actually useful mid-routine — an exact-match-only lookup names only the entry point and
    /// leaves every other instruction anonymous.
    ///
    /// Bounded by `max_offset` so a lone symbol at `$008000` does not claim every address in the
    /// ROM as belonging to it; past that distance the answer is honestly "no symbol".
    #[must_use]
    pub fn nearest(&self, addr: u32, max_offset: u32) -> Option<(&str, u32)> {
        let (found, name) = self.by_addr.range(..=addr).next_back()?;
        let offset = addr - found;
        (offset <= max_offset).then_some((name.as_str(), offset))
    }

    /// Format an address as `name+off` (or `name`), falling back to plain hex.
    #[must_use]
    pub fn label(&self, addr: u32, max_offset: u32) -> String {
        match self.nearest(addr, max_offset) {
            Some((name, 0)) => name.to_string(),
            Some((name, off)) => format!("{name}+{off}"),
            None => format!("${addr:06X}"),
        }
    }

    /// Add one symbol, replacing any previous name for that address.
    pub fn insert(&mut self, addr: u32, name: impl Into<String>) {
        self.by_addr.insert(addr & 0x00FF_FFFF, name.into());
    }

    /// Every symbol, address-ordered.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.by_addr.iter().map(|(a, n)| (*a, n.as_str()))
    }

    /// Parse symbol text, adding to whatever is already loaded.
    ///
    /// Additive rather than replacing, so several maps (a toolchain's plus a hand-written one) can
    /// be layered — which is how they are actually used.
    pub fn load(&mut self, text: &str) -> LoadStats {
        let mut stats = LoadStats {
            added: 0,
            skipped: 0,
        };
        let mut in_labels = true;
        for raw in text.lines() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            // A `[section]` header switches context. Everything except `[labels]` is skipped, since
            // a WLA `.sym` also carries `[definitions]`, `[breakpoints]`, and `[source files]` whose
            // lines look enough like symbols to poison the map.
            if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                in_labels = section.eq_ignore_ascii_case("labels");
                continue;
            }
            if !in_labels {
                continue;
            }
            if let Some((addr, name)) = parse_line(line) {
                self.insert(addr, name);
                stats.added += 1;
            } else {
                stats.skipped += 1;
            }
        }
        stats
    }
}

/// Drop a `;` or `//` trailing comment.
fn strip_comment(line: &str) -> &str {
    let end = line
        .find(';')
        .into_iter()
        .chain(line.find("//"))
        .min()
        .unwrap_or(line.len());
    &line[..end]
}

/// Parse one symbol line in any supported form.
fn parse_line(line: &str) -> Option<(u32, String)> {
    // `name = $AAAAAA` / `name = AAAAAA`
    if let Some((name, value)) = line.split_once('=') {
        let addr = parse_addr(value.trim())?;
        let name = name.trim();
        return (!name.is_empty()).then(|| (addr, name.to_string()));
    }
    // `BB:AAAA name` or `AAAAAA name`
    let mut parts = line.split_whitespace();
    let addr_text = parts.next()?;
    let name: Vec<&str> = parts.collect();
    if name.is_empty() {
        return None;
    }
    let addr = parse_addr(addr_text)?;
    Some((addr, name.join(" ")))
}

/// Parse `BB:AAAA`, `$AAAAAA`, or a bare hex address.
fn parse_addr(text: &str) -> Option<u32> {
    let t = text.trim().trim_start_matches('$');
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    if let Some((bank, offset)) = t.split_once(':') {
        let bank = u32::from_str_radix(bank.trim(), 16).ok()?;
        let offset = u32::from_str_radix(offset.trim(), 16).ok()?;
        if bank > 0xFF || offset > 0xFFFF {
            return None;
        }
        return Some((bank << 16) | offset);
    }
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(t, 16)
        .ok()
        .filter(|a| *a <= 0x00FF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::SymbolMap;

    #[test]
    fn parses_the_wla_labels_section() {
        let mut m = SymbolMap::new();
        let stats = m.load(
            "[labels]\n\
             00:8000 reset\n\
             00:9A3C update_sprites\n\
             7E:0300 player_hp\n",
        );
        assert_eq!(stats.added, 3);
        assert_eq!(stats.skipped, 0);
        assert_eq!(m.name(0x00_8000), Some("reset"));
        assert_eq!(m.name(0x00_9A3C), Some("update_sprites"));
        assert_eq!(m.name(0x7E_0300), Some("player_hp"));
    }

    /// A `.sym` also carries sections whose lines look like symbols; loading them would poison the
    /// map with breakpoint addresses and source paths.
    #[test]
    fn non_label_sections_are_skipped_entirely() {
        let mut m = SymbolMap::new();
        m.load(
            "[labels]\n\
             00:8000 reset\n\
             [breakpoints]\n\
             00:9000 trap\n\
             [definitions]\n\
             00:A000 SOMETHING\n",
        );
        assert_eq!(m.len(), 1);
        assert_eq!(m.name(0x00_8000), Some("reset"));
        assert!(m.name(0x00_9000).is_none());
    }

    #[test]
    fn parses_flat_and_assignment_forms() {
        let mut m = SymbolMap::new();
        let stats = m.load(
            "008000 reset\n\
             player_hp = $7E0300\n\
             frame_count = 7E0310\n\
             $00A000 draw\n",
        );
        assert_eq!(stats.added, 4);
        assert_eq!(m.name(0x00_8000), Some("reset"));
        assert_eq!(m.name(0x7E_0300), Some("player_hp"));
        assert_eq!(m.name(0x7E_0310), Some("frame_count"));
        assert_eq!(m.name(0x00_A000), Some("draw"));
    }

    /// Tolerant of junk, but never silent about it — a file that produced nothing must say so.
    #[test]
    fn unparsable_lines_are_counted_not_fatal() {
        let mut m = SymbolMap::new();
        let stats = m.load(
            "[labels]\n\
             00:8000 reset\n\
             this line is nonsense\n\
             .some directive\n\
             \n\
             ; just a comment\n\
             00:9000 loop ; trailing comment\n",
        );
        assert_eq!(stats.added, 2);
        assert_eq!(stats.skipped, 2, "the nonsense and the directive");
        assert_eq!(m.name(0x00_9000), Some("loop"), "comment stripped");

        let mut empty = SymbolMap::new();
        let stats = empty.load("nothing here\nnor here\n");
        assert_eq!(stats.added, 0);
        assert_eq!(stats.skipped, 2, "a file that produced nothing says so");
    }

    /// `nearest` is what names an address mid-routine; exact-match-only would leave every
    /// instruction after the entry point anonymous.
    #[test]
    fn nearest_names_addresses_inside_a_routine() {
        let mut m = SymbolMap::new();
        m.insert(0x00_8000, "reset");
        m.insert(0x00_9000, "main_loop");
        assert_eq!(m.nearest(0x00_9000, 0x100), Some(("main_loop", 0)));
        assert_eq!(m.nearest(0x00_9007, 0x100), Some(("main_loop", 7)));
        assert_eq!(m.label(0x00_9007, 0x100), "main_loop+7");
        assert_eq!(m.label(0x00_9000, 0x100), "main_loop");
    }

    /// Bounded, so a lone symbol does not claim the whole ROM.
    #[test]
    fn nearest_is_bounded_by_max_offset() {
        let mut m = SymbolMap::new();
        m.insert(0x00_8000, "reset");
        assert!(m.nearest(0x00_8100, 0x10).is_none());
        assert_eq!(m.label(0x00_8100, 0x10), "$008100");
        // Below the lowest symbol there is nothing to attach to.
        assert!(m.nearest(0x00_7000, 0x1000).is_none());
    }

    /// Loading is additive so several maps can be layered, and a later name wins for the same
    /// address.
    #[test]
    fn loads_are_additive_and_later_names_win() {
        let mut m = SymbolMap::new();
        m.load("00:8000 reset\n");
        m.load("00:9000 main\n");
        assert_eq!(m.len(), 2);
        m.load("00:8000 real_reset\n");
        assert_eq!(m.len(), 2);
        assert_eq!(m.name(0x00_8000), Some("real_reset"));
        m.clear();
        assert!(m.is_empty());
    }

    /// Out-of-range addresses are rejected rather than truncated into a wrong one.
    #[test]
    fn out_of_range_addresses_are_rejected() {
        let mut m = SymbolMap::new();
        let stats = m.load("FF:10000 bad\nGG:0000 bad2\n1000000 bad3\n");
        assert_eq!(stats.added, 0);
        assert_eq!(stats.skipped, 3);
    }
}
