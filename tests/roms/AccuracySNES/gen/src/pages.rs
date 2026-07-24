//! Menu pages — the AccuracyCoin-style grouping of the battery into named pages of at most ten tests.
//!
//! AccuracyCoin hand-authors a `Suite_*` table per page (a group name plus its tests); this is the
//! generator equivalent. The battery is emitted in ticket/authoring order, so a group's dossier
//! sub-groups (`A1`, `A2`, …) are interleaved. For the menu we regroup by **sub-group** (the part of a
//! dossier id before the dot) into semantic pages, each carrying an EXPLICIT list of battery test
//! indices (the sub-group's tests are not contiguous in battery order). A sub-group larger than ten
//! tests splits across several pages, suffixed ` 1`, ` 2`, …
//!
//! The per-test result byte and the skyline both index by battery position, so a page entry is just
//! that index; the display order (this module's order) is decoupled from the scored battery order.

use crate::dsl::Test;

/// Most tests a single page shows — the tallest skyline column and the menu's row budget.
pub const MAX_PER_PAGE: usize = 10;

/// One menu page: a title and the battery indices of the tests it lists, in display order.
pub struct Page {
    /// Centered page-group title, e.g. `65816: XCE & FLAGS`. ASCII, kept short for a 32-column row.
    pub name: String,
    /// Battery indices of this page's tests (`0..MAX_PER_PAGE` of them).
    pub tests: Vec<usize>,
}

/// The sub-group of a dossier id: the part before the `.` (`"A3.02"` -> `"A3"`).
fn subgroup(id: &str) -> &str {
    id.split('.').next().unwrap_or(id)
}

/// Human page title for a sub-group. Hand-authored, AccuracyCoin-style; unknown sub-groups fall back
/// to the bare code so a newly-added group still pages (just with a terse title until named here).
fn subgroup_title(sub: &str) -> &'static str {
    match sub {
        "A1" => "65816: XCE & FLAGS",
        "A2" => "65816: ARITHMETIC",
        "A3" => "65816: LOAD/STORE",
        "A4" => "65816: INDIRECT & WRAP",
        "A5" => "65816: OPCODE CYCLES",
        "A6" => "65816: INTERRUPTS",
        "A7" => "65816: STACK & TRANSFER",
        "A8" => "65816: BLOCK MOVE",
        "A9" => "65816: MISC",
        "B1" => "BUS: FASTROM SPEED",
        "B2" => "BUS: SCANLINE GEOMETRY",
        "B3" => "BUS: DRAM REFRESH",
        "B4" => "BUS: IRQ TIMING",
        "B5" => "BUS: MUL/DIV UNIT",
        "C1" => "PPU: VRAM ACCESS",
        "C2" => "PPU: VMAIN REMAP",
        "C3" => "PPU: CGRAM & COUNTERS",
        "C7" => "PPU: SPRITES",
        "C9" => "PPU: OVERSCAN & HI-RES",
        "C11" => "PPU: MODE 7",
        "C13" => "PPU: INIDISP ARTIFACTS",
        "C14" => "PPU: CHIP VERSION",
        "D1" => "DMA: GENERAL",
        "D2" => "DMA: HDMA",
        "E1" => "APU: SPC700 CORE",
        "E2" => "APU: SPC700 FLAGS",
        "E3" => "APU: SPC700 TIMING",
        "E4" => "APU: IPL & UPLOAD",
        "E5" => "APU: TIMERS",
        "E6" => "APU: DSP VOICES",
        "E7" => "APU: DSP ENVELOPE",
        "E8" => "APU: DSP MIXING",
        "E9" => "APU: DSP REGISTERS",
        "E10" => "APU: DSP ECHO",
        "F1" => "INPUT: CONTROLLERS",
        "G1" => "POWER-ON STATE",
        other => leak_static(other),
    }
}

/// Fall-back title for an un-named sub-group: leak a `'static` copy of the code so the signature can
/// stay `&'static str`. Only reached for a brand-new sub-group before it is named above, so the leak
/// is one tiny string per generator run, never in the ROM.
fn leak_static(code: &str) -> &'static str {
    Box::leak(code.to_owned().into_boxed_str())
}

/// Regroup the battery into menu pages. Sub-groups appear in first-seen order; each sub-group's tests
/// keep their battery order and split into pages of at most [`MAX_PER_PAGE`].
#[must_use]
pub fn pages(battery: &[Test]) -> Vec<Page> {
    // First-seen sub-group order, and each sub-group's battery indices.
    let mut order: Vec<&str> = Vec::new();
    let mut groups: std::collections::BTreeMap<&str, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, t) in battery.iter().enumerate() {
        let sub = subgroup(t.id);
        if !groups.contains_key(sub) {
            order.push(sub);
        }
        groups.entry(sub).or_default().push(i);
    }

    let mut out = Vec::new();
    for sub in order {
        let idxs = &groups[sub];
        let title = subgroup_title(sub);
        let chunks: Vec<&[usize]> = idxs.chunks(MAX_PER_PAGE).collect();
        let multi = chunks.len() > 1;
        for (n, chunk) in chunks.iter().enumerate() {
            let name = if multi {
                format!("{title} {}", n + 1)
            } else {
                title.to_owned()
            };
            assert!(
                name.len() <= 28,
                "page title '{name}' exceeds the 28-column centered row"
            );
            out.push(Page {
                name,
                tests: chunk.to_vec(),
            });
        }
    }
    out
}
