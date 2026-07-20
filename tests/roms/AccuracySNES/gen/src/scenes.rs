//! Rendered scenes — the host-side framebuffer oracle (ticket **T-04-H**, `docs/adr/0013`).
//!
//! # Why these are not tests
//!
//! Most of Group C's remainder decides only *what appears on screen*: backgrounds and modes,
//! colour math and windows, offset-per-tile, mosaic, direct colour. No register reads back, no
//! counter moves, no flag changes — and a cart cannot see its own framebuffer, because the PPU
//! offers no path from rendered pixels to the CPU.
//!
//! So a scene **asserts nothing**. It sets up PPU state, and the host hashes the resulting
//! framebuffer and compares it against a committed golden. Results are reported in their own tier
//! and are **never** folded into the on-cart pass rate, because a scene does not have the property
//! that makes the rest of the battery worth having: that the identical image means the same thing
//! on any emulator and on real hardware.
//!
//! # The rule that keeps a golden honest
//!
//! A golden is a snapshot of *agreement*, not of truth — `docs/scheduler.md` records what that cost
//! when `hdmaen_latch_test` had to be re-blessed. Per ADR 0013 rule 4, a scene's golden is
//! committed only once the reference emulators have been shown to agree on it; where they disagree
//! the scene is recorded as a variant set with each rendering attributed, never as one arbitrary
//! winner. A scene with no cross-validated golden simply is not in the gated set yet.
//!
//! # Where the bugs will be
//!
//! Setup, not rendering. This project has already produced four setup errors that each looked like
//! an emulator bug — a wrong `OBJSEL` field, a seed colliding with an open-bus value, an
//! uncontrolled field, a flag clobbered by the measurement harness. A scene has strictly more setup
//! surface and no on-cart assertion to catch a mistake early: the symptom is a wrong picture. Each
//! scene therefore states exactly what it is arranging and which assertion it is evidence for.

use core::fmt::Write as _;

/// One rendered scene: a name, the assertion it covers, and the setup it performs.
pub struct Scene {
    /// Stable identifier, used as the golden's key. Never renumber — the golden is keyed on it.
    pub id: &'static str,
    /// Dossier assertion(s) this scene is evidence for.
    pub dossier: &'static str,
    /// What the scene arranges, and what a reader should expect to see.
    pub what: &'static str,
    /// Setup body, run with the screen blanked; the runtime releases forced blank afterwards.
    pub setup: &'static [&'static str],
}

/// The scene set. Deliberately small to begin with — ADR 0013 gates only cross-validated scenes,
/// so the set grows as goldens are blessed rather than landing wholesale.
pub const SCENES: &[Scene] = &[
    Scene {
        id: "c5-mode1-bg-priority",
        dossier: "C5.02",
        what: "Mode 1 with BG1 and BG2 enabled at different priorities, each showing the font \
               tiles already in VRAM through a distinct palette. Evidence for the mode-1 layer \
               and priority ordering.",
        setup: &[
            "sep #$20",
            "lda #$01",
            "sta $2105         ; BGMODE 1",
            "lda #$00",
            "sta $210B         ; BG1/BG2 character data at word $0000",
            "lda #(MAP_BASE >> 8)",
            "sta $2107         ; BG1 tilemap base",
            "lda #(MAP_BASE >> 8)",
            "sta $2108         ; BG2 tilemap base, same map so both layers show content",
            "lda #$03",
            "sta $212C         ; BG1 + BG2 on the main screen",
            "lda #$0F",
            "sta $2100         ; brightness 15, forced blank off",
        ],
    },
    Scene {
        id: "c8-fixed-colour-add",
        dossier: "C8.10",
        what: "Colour math in additive mode against the fixed colour, with the subscreen left as \
               the fixed backdrop. Evidence for CGADSUB/COLDATA and the half/div2 behaviour.",
        setup: &[
            "sep #$20",
            "stz $2105         ; BGMODE 0",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "lda #$01",
            "sta $212C         ; BG1 on the main screen",
            "lda #$02",
            "sta $2130         ; CGWSEL: subscreen is the fixed colour",
            "lda #$21",
            "sta $2131         ; CGADSUB: add, applied to BG1",
            "lda #$9F",
            "sta $2132         ; COLDATA: blue = 31",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c10-mosaic-4x",
        dossier: "C10.01",
        what: "Mosaic size 4 applied to BG1. Evidence that mosaic is applied after scrolling and \
               anchored to the screen origin rather than the scroll origin.",
        setup: &[
            "sep #$20",
            "stz $2105         ; BGMODE 0",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "lda #$01",
            "sta $212C",
            "lda #$31",
            "sta $2106         ; MOSAIC: size 4, enabled on BG1",
            "lda #$0F",
            "sta $2100",
        ],
    },
];

/// Emit the scene setup routines and the dispatch table the runtime walks.
#[must_use]
pub fn asm() -> String {
    let mut s = String::new();
    let _ = writeln!(s, "; GENERATED by accuracysnes-gen — do not edit by hand.");
    let _ = writeln!(
        s,
        "; Rendered scenes for the host framebuffer oracle (docs/adr/0013)."
    );
    let _ = writeln!(s, ".p816");
    let _ = writeln!(s, "SCENES_IMPL = 1");
    let _ = writeln!(s, ".include \"runtime.inc\"");
    let _ = writeln!(s, "\n.segment \"TESTS\"");

    for sc in SCENES {
        let _ = writeln!(s, "\n; {} — {}", sc.id, sc.dossier);
        let _ = writeln!(s, "; {}", sc.what);
        let _ = writeln!(s, ".proc {}", label(sc.id));
        let _ = writeln!(s, "    .a16");
        let _ = writeln!(s, "    .i16");
        for line in sc.setup {
            let _ = writeln!(s, "    {line}");
            // ca65 tracks the operand width of immediates from `.a8`/`.a16` directives, not from
            // the `sep`/`rep` that actually changes it at runtime. Miss one and `lda #$01` after a
            // `sep #$20` assembles as a two-byte immediate, desynchronising everything after it —
            // which is exactly how the first version of these scenes crashed into a BRK loop.
            // Emitting the directive from the instruction removes the chance to forget.
            for d in width_directives(line) {
                let _ = writeln!(s, "    {d}");
            }
        }
        let _ = writeln!(s, "    rep #$30");
        let _ = writeln!(s, "    .a16");
        let _ = writeln!(s, "    .i16");
        let _ = writeln!(s, "    rts");
        let _ = writeln!(s, ".endproc");
    }

    let _ = writeln!(s, "\n.segment \"CATALOG\"");
    let _ = writeln!(s, ".export _scene_count");
    let _ = writeln!(s, ".export _scene_entries");
    let _ = writeln!(s, "_scene_count:");
    let _ = writeln!(s, "    .word {}", SCENES.len());
    let _ = writeln!(s, "_scene_entries:");
    for sc in SCENES {
        let _ = writeln!(s, "    .addr {}", label(sc.id));
    }
    s
}

/// The scene manifest the host harnesses read: `index<TAB>id<TAB>dossier`, 1-based to match the
/// IDs the cart publishes.
///
/// The cart can only publish a number, and a number is a poor golden key — inserting a scene would
/// silently re-point every golden after it. So the ROM carries the numbers and this carries the
/// stable names, written next to the ROM by the same build that produced it.
#[must_use]
pub fn manifest() -> String {
    let mut s = String::from("# GENERATED by accuracysnes-gen — index\tid\tdossier\n");
    for (i, sc) in SCENES.iter().enumerate() {
        let _ = writeln!(s, "{}\t{}\t{}", i + 1, sc.id, sc.dossier);
    }
    s
}

/// The `.a8`/`.a16`/`.i8`/`.i16` directives a `sep`/`rep` immediate implies, if any.
///
/// `sep` sets the width bits (narrow to 8-bit), `rep` clears them (widen to 16-bit); bit 5 is `M`
/// (the accumulator) and bit 4 is `X` (the index registers). Anything else — including a `sep`/`rep`
/// that only touches the carry or decimal bits — implies no directive.
fn width_directives(line: &str) -> Vec<&'static str> {
    let line = line.split(';').next().unwrap_or(line).trim();
    let mut it = line.split_whitespace();
    let (Some(op), Some(arg)) = (it.next(), it.next()) else {
        return Vec::new();
    };
    let narrow = match op.to_ascii_lowercase().as_str() {
        "sep" => true,
        "rep" => false,
        _ => return Vec::new(),
    };
    let Some(hex) = arg.trim().strip_prefix("#$") else {
        return Vec::new();
    };
    let Ok(bits) = u8::from_str_radix(hex, 16) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if bits & 0x20 != 0 {
        out.push(if narrow { ".a8" } else { ".a16" });
    }
    if bits & 0x10 != 0 {
        out.push(if narrow { ".i8" } else { ".i16" });
    }
    out
}

/// The assembly label for a scene.
fn label(id: &str) -> String {
    format!("scene_{}", id.replace('-', "_"))
}
