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
    /// Setup body. The runtime enters with forced blank on and the registers freshly
    /// re-initialised, and **the scene is responsible for releasing forced blank itself** — every
    /// scene ends by writing `INIDISP` ($2100), because brightness is part of what a scene may
    /// want to vary. Omit that write and the scene renders black.
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
        dossier: "C8.11",
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
    Scene {
        id: "c5-mode0-four-bg-priority",
        dossier: "C5.01",
        what: "Mode 0 with all four backgrounds enabled, each scrolled by a different amount so \
               the lower layers show through the transparent pixels of the ones above. Evidence \
               for the mode-0 priority order and for four independent 2bpp layers existing at all.",
        setup: &[
            "sep #$20",
            "stz $2105         ; BGMODE 0: four 2bpp layers",
            "stz $210B",
            "stz $210C         ; all four BGs take character data from word $0000",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "sta $2108",
            "sta $2109",
            "sta $210A         ; one tilemap, four layers — the difference is scroll and palette",
            "; Scroll each layer by a different amount. Without this the higher-priority layer",
            "; covers the others exactly and the priority order is unobservable.",
            "lda #$04",
            "sta $210F",
            "stz $210F         ; BG2 H scroll = 4",
            "lda #$08",
            "sta $2111",
            "stz $2111         ; BG3 H scroll = 8",
            "lda #$0C",
            "sta $2113",
            "stz $2113         ; BG4 H scroll = 12",
            "lda #$0F",
            "sta $212C         ; BG1-4 all on the main screen",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c5-mode0-palette-segregation",
        dossier: "C5.09",
        what: "Mode 0 again, but read for colour rather than order: each background takes its \
               palette from its own 32-entry CGRAM region (BG1 0-31, BG2 32-63, BG3 64-95, \
               BG4 96-127). The canvas fills all 128 entries with distinct colours, so a core \
               that ignores the per-BG offset renders the four layers in the same colours.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "stz $210B",
            "stz $210C",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "sta $2108",
            "sta $2109",
            "sta $210A",
            "; Vertical rather than horizontal offsets this time, so the layers interleave by row",
            "; and each region's colours land in a different part of the picture.",
            "lda #$02",
            "sta $2110",
            "stz $2110         ; BG2 V scroll = 2",
            "lda #$04",
            "sta $2112",
            "stz $2112         ; BG3 V scroll = 4",
            "lda #$06",
            "sta $2114",
            "stz $2114         ; BG4 V scroll = 6",
            "lda #$0F",
            "sta $212C",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c5-mode3-8bpp",
        dossier: "C5.04",
        what: "Mode 3: BG1 8bpp, BG2 4bpp, both reading the same VRAM. The extra bitplanes are \
               zero (the canvas only writes a 2bpp font), so this pins how a core assembles a \
               deeper pixel from planes that are not all present — deterministic, because the \
               VRAM it reads was cleared before the font was loaded.",
        setup: &[
            "sep #$20",
            "lda #$03",
            "sta $2105         ; BGMODE 3",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "sta $2108",
            "jsr scene_low_tiles ; 8bpp tiles are 32 words; the canvas map indexes past the font",
            "lda #$03",
            "sta $212C         ; BG1 + BG2",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c5-tilemap-flip-bits",
        dossier: "C5.10",
        what: "The tilemap entry's H and V flip bits (bits 14 and 15 of `vhopppcc cccccccc`). The \
               canvas writes neither, so this scene rewrites the tilemap with both set on \
               alternating cells — a core that ignores a flip bit, or swaps the two, renders \
               recognisably different glyphs rather than subtly wrong ones.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "; Rewrite the tilemap: cell index bit 0 picks H flip, bit 5 (row parity) picks V flip.",
            "lda #$80",
            "sta $2115         ; VMAIN: increment after the high byte, so a 16-bit store = one entry",
            "rep #$30",
            "ldx #MAP_BASE",
            "stx $2116",
            "ldx #$0000",
            "@flipcell:",
            "txa",
            "and #$003F",
            "clc",
            "adc #$0041        ; a letter glyph, so a flip is obvious",
            "sta f:V_TMP",
            "txa",
            "and #$0001",
            "beq :+",
            "lda #$4000        ; H flip",
            "bra :++",
            ":",
            "lda #$0000",
            ":",
            "sta f:V_TMP2",
            "txa",
            "and #$0020        ; every other row of cells",
            "beq :+",
            "lda #$8000        ; V flip",
            "bra :++",
            ":",
            "lda #$0000",
            ":",
            "ora f:V_TMP2",
            "ora f:V_TMP",
            "sta $2118",
            "inx",
            "cpx #(SCREEN_COLS * 32)",
            "bne @flipcell",
            "sep #$20",
            "lda #$01",
            "sta $212C",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c5-16x16-tiles",
        dossier: "C5.11",
        what: "BG1 switched to 16x16 tiles (BGMODE bit 4). A 16x16 cell is assembled from the \
               named tile plus +1, +16 and +17, so a core that uses the wrong neighbour renders \
               the right glyphs in the wrong quadrants — visible, and specific about which \
               quadrant is wrong.",
        setup: &[
            "sep #$20",
            "lda #$10",
            "sta $2105         ; BGMODE 0 with BG1 in 16x16 tiles",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "lda #$01",
            "sta $212C",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-subtract-mode",
        dossier: "C8.10",
        what: "Colour math in SUBTRACT mode (`$2131` bit 7) against the fixed colour, the \
               counterpart to `c8-fixed-colour-add`. Together they pin the sign: a core that \
               ignores bit 7 renders the two scenes identically.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "lda #$02",
            "sta $2130         ; CGWSEL: the subscreen is the fixed colour",
            "lda #$A1",
            "sta $2131         ; CGADSUB: subtract, applied to BG1 and the backdrop",
            "lda #$9F",
            "sta $2132         ; COLDATA: blue = 31",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-clamp-no-wrap",
        dossier: "C8.02",
        what: "Additive colour math driven hard enough to saturate every channel. Results clamp \
               at 31 and do not wrap, so the bright areas go white and stay white — a core that \
               wraps produces dark speckle exactly where the picture should be brightest, which \
               is unmistakable rather than subtle.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "lda #$02",
            "sta $2130",
            "lda #$21",
            "sta $2131         ; add",
            "lda #$FF",
            "sta $2132         ; COLDATA: all three channel selects, value 31 — saturate everything",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-half-ignored-on-fixed-backdrop",
        dossier: "C8.03",
        what: "Identical to `c8-fixed-colour-add` except that CGADSUB's half/div2 bit is set. The \
               documented behaviour is that half is IGNORED when the subscreen is the fixed \
               backdrop, so this scene must hash the same as that one. Two scenes that must \
               agree is a stronger statement than one scene that must match a number.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "lda #$02",
            "sta $2130         ; CGWSEL: the subscreen is the fixed colour",
            "lda #$61",
            "sta $2131         ; CGADSUB: add + HALF, applied to BG1",
            "lda #$9F",
            "sta $2132         ; COLDATA: blue = 31, as in the add scene",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-window-bounds-inclusive",
        dossier: "C8.04",
        what: "Window 1 set to 64..191 and used to clip BG1 on the main screen. Both bounds are \
               inclusive, so the masked band is 128 pixels wide — a core with an exclusive edge \
               is off by one column at a hard black/colour boundary, which a hash catches even \
               though an eye would not.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "lda #$02",
            "sta $2123         ; W12SEL: BG1 uses window 1, not inverted",
            "lda #64",
            "sta $2126         ; WH0: window 1 left edge",
            "lda #191",
            "sta $2127         ; WH1: window 1 right edge",
            "lda #$01",
            "sta $212E         ; TMW: clip BG1 on the main screen",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-window-left-gt-right-empty",
        dossier: "C8.05",
        what: "The same window with its bounds crossed (left 200, right 50). That is an EMPTY \
               window, not a wrapped one: BG1 stays fully visible. A core that treats the pair \
               as a wraparound range clips both ends of the screen instead.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "lda #$02",
            "sta $2123",
            "lda #200",
            "sta $2126",
            "lda #50",
            "sta $2127         ; left > right",
            "lda #$01",
            "sta $212E",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-window-inverted-empty-is-full",
        dossier: "C8.06",
        what: "The crossed bounds again, with window 1 inverted. An inverted empty window is a \
               FULL one, so BG1 disappears entirely and the backdrop is all that remains. Paired \
               with the previous scene this pins the inversion as acting on the region rather \
               than on the comparison.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "lda #$03",
            "sta $2123         ; W12SEL: BG1 uses window 1, INVERTED",
            "lda #200",
            "sta $2126",
            "lda #50",
            "sta $2127",
            "lda #$01",
            "sta $212E",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-both-windows-disabled-empty",
        dossier: "C8.07",
        what: "TMW clips BG1, but neither window is enabled for it in W12SEL. The combined mask \
               is EMPTY, not full — BG1 stays fully visible. This is the errata case: the \
               intuitive reading is that clipping with no window means clipping everything, and \
               a core that implements the intuition blanks the layer.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "stz $2123         ; W12SEL: neither window enabled for BG1",
            "lda #64",
            "sta $2126",
            "lda #191",
            "sta $2127         ; bounds set, but no window selects them",
            "lda #$01",
            "sta $212E         ; TMW: clip BG1 anyway",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c8-window-logic-xor",
        dossier: "C8.08",
        what: "Both windows enabled for BG1 and combined with XOR: window 1 covers 32..159, \
               window 2 covers 96..223, so the mask is the two non-overlapping wings and the \
               64-pixel overlap is left alone. Each of OR, AND, XOR and XNOR produces a visibly \
               different band pattern, so picking the wrong one cannot pass.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$01",
            "sta $212C",
            "lda #$0A",
            "sta $2123         ; W12SEL: BG1 uses window 1 AND window 2, neither inverted",
            "lda #32",
            "sta $2126",
            "lda #159",
            "sta $2127         ; window 1",
            "lda #96",
            "sta $2128",
            "lda #223",
            "sta $2129         ; window 2, overlapping",
            "lda #$02",
            "sta $212A         ; WBGLOG: BG1 combines its two windows with XOR",
            "lda #$01",
            "sta $212E",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c10-mosaic-screen-anchored",
        dossier: "C10.02",
        what: "Mosaic 4 with BG1 scrolled by 2 pixels in each axis — deliberately not a multiple \
               of the mosaic size. The block grid stays anchored to the top-left of the SCREEN, \
               so the content moves through a stationary grid. A core that anchors to the scroll \
               origin instead produces the same picture shifted, which the `c10-mosaic-4x` scene \
               alone cannot distinguish.",
        setup: &[
            "sep #$20",
            "stz $2105",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "stz $210B",
            "lda #$02",
            "sta $210D",
            "stz $210D         ; BG1 H scroll = 2",
            "lda #$02",
            "sta $210E",
            "stz $210E         ; BG1 V scroll = 2",
            "lda #$01",
            "sta $212C",
            "lda #$31",
            "sta $2106         ; MOSAIC: size 4, enabled on BG1",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c12-direct-colour-mode3",
        dossier: "C12.01,C12.03",
        what: "Direct colour mode (CGWSEL bit 0) on Mode 3's 8bpp BG1, where the pixel value \
               supplies the colour directly instead of indexing CGRAM. Only modes 3, 4 and 7 \
               offer it, so this also pins that CGRAM is bypassed rather than merely reordered: \
               the canvas's 128 palette entries stop mattering entirely.",
        setup: &[
            "sep #$20",
            "lda #$03",
            "sta $2105         ; BGMODE 3 — 8bpp BG1, a precondition for direct colour",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "jsr scene_low_tiles ; without this every 8bpp pixel reads zero and the screen is empty",
            "lda #$01",
            "sta $212C",
            "lda #$01",
            "sta $2130         ; CGWSEL bit 0: direct colour",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c5-mode2-plain",
        dossier: "C5.03",
        what: "Mode 2 with BG1 and BG2 displayed and offset-per-tile left inert (BG3's table is \
               all zeroes, so no entry carries an enable bit). The control for the `c6-*` scenes: \
               if this renders and they do not, the fault is in their setup rather than in the \
               mode. It is also `C5.03`'s own evidence, which is why it is a scene and not a \
               scratch file.",
        setup: &[
            "sep #$20",
            "lda #$02",
            "sta $2105         ; BGMODE 2",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "sta $2108",
            "lda #(OPT_MAP_BASE >> 8)",
            "sta $2109",
            "jsr scene_low_tiles ; 4bpp tiles are 16 words; the canvas map indexes past the font",
            "rep #$30",
            "lda #$0000",
            "sta f:V_OPT_H_EVEN",
            "sta f:V_OPT_H_ODD",
            "sta f:V_OPT_V_EVEN",
            "sta f:V_OPT_V_ODD ; an all-zero table: no entry has an enable bit set",
            "jsr scene_opt_map",
            "sep #$20",
            "lda #$03",
            "sta $212C         ; BG1 + BG2",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c6-opt-v-alternating-columns",
        dossier: "C6.05,C6.06",
        what: "Mode 2 offset-per-tile: BG3's row-1 entries give even columns a vertical offset of \
               64 and odd columns none. Two assertions at once — each entry moves a WHOLE tile \
               column (C6.06), and the leftmost tile is never affected, so the first entry \
               controls the SECOND visible column (C6.05, an errata). Alternating rather than \
               uniform offsets is what makes the second one legible: the shifted columns come out \
               odd-numbered, and a core without the exemption shifts the even ones instead.",
        setup: &[
            "sep #$20",
            "lda #$02",
            "sta $2105         ; BGMODE 2 — BG1/BG2 4bpp, BG3 is the offset table",
            "stz $210B         ; BG1/BG2 character data at word $0000",
            "lda #(MAP_BASE >> 8)",
            "sta $2107         ; BG1 tilemap",
            "lda #(OPT_MAP_BASE >> 8)",
            "sta $2109         ; BG3 tilemap = the offset table",
            "jsr scene_low_tiles ; 4bpp tiles are 16 words; the canvas map indexes past the font",
            "rep #$30",
            "lda #$0000",
            "sta f:V_OPT_H_EVEN",
            "sta f:V_OPT_H_ODD ; no horizontal offsets in this scene",
            "lda #($2000 | 100) ; bit 13 = applies to BG1; 100 rows down — deliberately NOT a",
            "                   ; multiple of 8, so the glyph row shifts too and the offset is",
            "                   ; visible whatever the tilemap happens to contain",
            "sta f:V_OPT_V_EVEN",
            "lda #$0000",
            "sta f:V_OPT_V_ODD",
            "jsr scene_opt_map",
            "sep #$20",
            "lda #$01",
            "sta $212C         ; BG1 only — BG3 is a table here, not a layer",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c6-opt-v-replaces-vofs",
        dossier: "C6.04",
        what: "The same vertical offset, but with BG1VOFS already set to 32. An offset-per-tile V \
               entry REPLACES the background's own scroll rather than adding to it, so the \
               offset columns land at row 64 and not at 96. The unaffected columns still show the \
               scroll, which is what makes the two behaviours distinguishable in one picture.",
        setup: &[
            "sep #$20",
            "lda #$02",
            "sta $2105",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "lda #(OPT_MAP_BASE >> 8)",
            "sta $2109",
            "jsr scene_low_tiles",
            "sep #$20",
            "lda #32",
            "sta $210E",
            "stz $210E         ; BG1VOFS = 32",
            "rep #$30",
            "lda #$0000",
            "sta f:V_OPT_H_EVEN",
            "sta f:V_OPT_H_ODD",
            "lda #($2000 | 100) ; not a multiple of 8 — see c6-opt-v-alternating-columns",
            "sta f:V_OPT_V_EVEN",
            "lda #$0000",
            "sta f:V_OPT_V_ODD",
            "jsr scene_opt_map",
            "sep #$20",
            "lda #$01",
            "sta $212C",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c6-opt-h-keeps-fine-scroll",
        dossier: "C6.03",
        what: "A horizontal offset-per-tile entry of 64 with BG1HOFS = 5. Unlike the vertical \
               case, an H entry replaces only the COARSE part of the scroll — the background's \
               own low three HOFS bits survive, so the offset columns sit at 64+5 rather than at \
               64. Five pixels is small, and a hash notices it where an eye would not.",
        setup: &[
            "sep #$20",
            "lda #$02",
            "sta $2105",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "lda #(OPT_MAP_BASE >> 8)",
            "sta $2109",
            "jsr scene_low_tiles",
            "sep #$20",
            "lda #5",
            "sta $210D",
            "stz $210D         ; BG1HOFS = 5 — the fine bits that must survive",
            "rep #$30",
            "lda #($2000 | 64)",
            "sta f:V_OPT_H_EVEN",
            "lda #$0000",
            "sta f:V_OPT_H_ODD",
            "sta f:V_OPT_V_EVEN",
            "sta f:V_OPT_V_ODD",
            "jsr scene_opt_map",
            "sep #$20",
            "lda #$01",
            "sta $212C",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c6-opt-enable-bit-bg1",
        dossier: "C6.01",
        what: "Both BG1 and BG2 are displayed and the offset entries carry bit 13 only. Only BG1 \
               moves. Paired with `c6-opt-enable-bit-bg2` this pins which bit belongs to which \
               layer — neither scene alone can, because a core that swaps the two bits produces a \
               picture that is equally plausible until the pair is compared.",
        setup: &[
            "sep #$20",
            "lda #$02",
            "sta $2105",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "sta $2108         ; BG2 shows the same map, so a shift is visible on either layer",
            "lda #(OPT_MAP_BASE >> 8)",
            "sta $2109",
            "jsr scene_low_tiles",
            "sep #$20",
            "lda #8",
            "sta $2110",
            "stz $2110         ; BG2 scrolled down 8 so the two layers are separable",
            "rep #$30",
            "lda #$0000",
            "sta f:V_OPT_H_EVEN",
            "sta f:V_OPT_H_ODD",
            "lda #($2000 | 100) ; bit 13: BG1 only",
            "sta f:V_OPT_V_EVEN",
            "lda #$0000",
            "sta f:V_OPT_V_ODD",
            "jsr scene_opt_map",
            "sep #$20",
            "lda #$03",
            "sta $212C         ; BG1 + BG2",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c6-opt-enable-bit-bg2",
        dossier: "C6.01",
        what: "Identical to `c6-opt-enable-bit-bg1` except that the entries carry bit 14 instead \
               of bit 13, so BG2 moves and BG1 does not. The two scenes must NOT hash the same; \
               a core that treats the two bits alike renders them identically.",
        setup: &[
            "sep #$20",
            "lda #$02",
            "sta $2105",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "sta $2108",
            "lda #(OPT_MAP_BASE >> 8)",
            "sta $2109",
            "jsr scene_low_tiles",
            "sep #$20",
            "lda #8",
            "sta $2110",
            "stz $2110",
            "rep #$30",
            "lda #$0000",
            "sta f:V_OPT_H_EVEN",
            "sta f:V_OPT_H_ODD",
            "lda #($4000 | 100) ; bit 14: BG2 only",
            "sta f:V_OPT_V_EVEN",
            "lda #$0000",
            "sta f:V_OPT_V_ODD",
            "jsr scene_opt_map",
            "sep #$20",
            "lda #$03",
            "sta $212C",
            "lda #$0F",
            "sta $2100",
        ],
    },
    Scene {
        id: "c6-mode4-h-vs-v-select",
        dossier: "C6.02",
        what: "Mode 4 packs both offsets into a single row and picks between them with bit 15: \
               clear selects horizontal, set selects vertical. Even columns get a horizontal \
               offset and odd columns a vertical one of the same magnitude, so a core that reads \
               the selector backwards produces a picture with the two displacements exchanged \
               rather than one that merely looks wrong.",
        setup: &[
            "sep #$20",
            "lda #$04",
            "sta $2105         ; BGMODE 4 — BG1 8bpp, BG2 2bpp, BG3 is the offset table",
            "stz $210B",
            "lda #(MAP_BASE >> 8)",
            "sta $2107",
            "lda #(OPT_MAP_BASE >> 8)",
            "sta $2109",
            "jsr scene_low_tiles ; 8bpp tiles are 32 words — mandatory here",
            "rep #$30",
            "lda #($2000 | 64) ; bit 15 clear: horizontal, applied to BG1",
            "sta f:V_OPT_H_EVEN",
            "lda #($A000 | 100) ; bit 15 set: vertical (not a multiple of 8)",
            "sta f:V_OPT_H_ODD",
            "lda #$0000",
            "sta f:V_OPT_V_EVEN",
            "sta f:V_OPT_V_ODD ; mode 4 reads row 0 only",
            "jsr scene_opt_map",
            "sep #$20",
            "lda #$01",
            "sta $212C",
            "lda #$0F",
            "sta $2100",
        ],
    },
];

/// Emit `scene_low_tiles`, the shared tilemap rewrite a deep-colour scene needs.
///
/// Split out of [`asm`] because it is a self-contained routine, not part of assembling the scene
/// list — and because keeping it inline pushed `asm` past the line limit for no benefit.
fn low_tiles_helper() -> String {
    let mut s = String::new();
    // A shared helper rather than fifteen copies. The canvas fills the tilemap with glyph indices
    // spread over the whole font, which is right for 2bpp and useless for a deeper mode: an 8bpp
    // tile is 32 words, so tile $21 starts past the end of a 512-word font and every pixel reads
    // as zero — transparent. A mode-3 or direct-colour scene built on the canvas map renders an
    // empty screen and proves nothing. Two of them did, on the first run.
    let _ = writeln!(
        s,
        "\n; Rewrite the tilemap with tile indices that both EXIST and are NON-BLANK in a deep mode."
    );
    let _ = writeln!(s, ";");
    let _ = writeln!(
        s,
        "; Two constraints, and missing the second is the subtler mistake. A tile must lie inside"
    );
    let _ = writeln!(
        s,
        "; the font: 8bpp is 32 words/tile against a 1024-word font, so $00-$1F exist. It must also"
    );
    let _ = writeln!(
        s,
        "; cover PRINTABLE glyphs, and how many glyphs a tile spans depends on the depth: a 4bpp"
    );
    let _ = writeln!(
        s,
        "; tile covers glyphs 2T and 2T+1, an 8bpp tile covers 4T..4T+3. So $10-$1F is glyphs 32-63"
    );
    let _ = writeln!(
        s,
        "; at 4bpp and 64-127 at 8bpp — printable at both. Anything below $10 lands on ASCII 0-31,"
    );
    let _ = writeln!(s, "; the control characters, which are blank in this font.");
    let _ = writeln!(s, ";");
    let _ = writeln!(
        s,
        "; This cost two rounds. $00-$0F rendered an EMPTY screen in mode 2 while mode 4 looked"
    );
    let _ = writeln!(
        s,
        "; fine, which reads as a broken mode; $08-$0F fixed 8bpp and left 4bpp still blank, which"
    );
    let _ = writeln!(
        s,
        "; reads as a broken depth. Neither was true. An empty scene hashes stably and the"
    );
    let _ = writeln!(
        s,
        "; reference emulators agree with it, so only looking at the picture finds this."
    );
    let _ = writeln!(s, ";");
    let _ = writeln!(
        s,
        "; WIDTH-NEUTRAL: P is saved and restored, so the A/X/Y widths on exit are"
    );
    let _ = writeln!(
        s,
        "; exactly what they were on entry. Deliberate rather than merely tidy: the"
    );
    let _ = writeln!(
        s,
        "; caller is generated assembly whose .a8/.a16 directives come from its OWN"
    );
    let _ = writeln!(
        s,
        "; sep/rep lines, and a JSR is not one of those — so a helper that changed"
    );
    let _ = writeln!(
        s,
        "; the width would leave the assembler believing one thing while the CPU did"
    );
    let _ = writeln!(
        s,
        "; another, and the next immediate operand would be assembled at the wrong"
    );
    let _ = writeln!(
        s,
        "; size. That failure already cost this project a debugging session; see the"
    );
    let _ = writeln!(s, "; .a8/.a16 emission in `asm` below.");
    let _ = writeln!(s, ".proc scene_low_tiles");
    let _ = writeln!(s, "    php");
    let _ = writeln!(s, "    .a16");
    let _ = writeln!(s, "    .i16");
    let _ = writeln!(s, "    sep #$20");
    let _ = writeln!(s, "    .a8");
    let _ = writeln!(s, "    lda #$80");
    let _ = writeln!(s, "    sta VMAIN");
    let _ = writeln!(s, "    rep #$30");
    let _ = writeln!(s, "    .a16");
    let _ = writeln!(s, "    .i16");
    let _ = writeln!(s, "    ldx #MAP_BASE");
    let _ = writeln!(s, "    stx VMADDL");
    let _ = writeln!(s, "    ldx #$0000");
    let _ = writeln!(s, "@cell:");
    let _ = writeln!(s, "    txa");
    let _ = writeln!(s, "    lsr a");
    let _ = writeln!(s, "    lsr a");
    let _ = writeln!(s, "    lsr a");
    let _ = writeln!(s, "    lsr a");
    let _ = writeln!(s, "    lsr a             ; row");
    let _ = writeln!(s, "    sta f:V_TMP2");
    let _ = writeln!(s, "    txa");
    let _ = writeln!(s, "    clc");
    let _ = writeln!(
        s,
        "    adc f:V_TMP2      ; tile varies with row AND column: a column-constant map cannot"
    );
    let _ = writeln!(
        s,
        "                      ; show a vertical shift, which is how the first OPT scenes came"
    );
    let _ = writeln!(
        s,
        "                      ; out identical to their own control."
    );
    let _ = writeln!(s, "    and #$000F");
    let _ = writeln!(
        s,
        "    ora #$0010        ; tile $10-$1F: inside the font AND printable at 4bpp and 8bpp"
    );
    let _ = writeln!(s, "    sta f:V_TMP");
    let _ = writeln!(s, "    lda f:V_TMP2");
    let _ = writeln!(s, "    and #$0007");
    let _ = writeln!(s, "    .repeat 10");
    let _ = writeln!(s, "    asl a");
    let _ = writeln!(s, "    .endrepeat");
    let _ = writeln!(s, "    ora f:V_TMP       ; palette in bits 10-12");
    let _ = writeln!(s, "    sta VMDATAL");
    let _ = writeln!(s, "    inx");
    let _ = writeln!(s, "    cpx #(SCREEN_COLS * 32)");
    let _ = writeln!(s, "    bne @cell");
    let _ = writeln!(
        s,
        "    plp               ; restore the caller's register widths"
    );
    let _ = writeln!(s, "    rts");
    let _ = writeln!(s, ".endproc");

    s
}

/// Emit `scene_opt_map`, which fills BG3's tilemap with offset-per-tile entries.
///
/// In modes 2 and 6 BG3 stops being a layer and becomes a table: tile row 0 supplies each column's
/// horizontal offset and row 1 its vertical offset, with bit 13 enabling the entry for BG1 and
/// bit 14 for BG2. Mode 4 packs both into row 0 and picks between them with bit 15.
///
/// Entries are written per column *parity* — even columns take one value, odd columns another —
/// because that is what makes the errata visible. `C6.05` says the leftmost tile is never affected
/// and the first entry therefore controls the *second* visible column; with alternating values the
/// shifted columns come out odd-numbered rather than even, which is a difference in the picture
/// rather than a difference of one pixel at an edge.
///
/// Width-neutral (`php`/`plp`), for the reason spelled out on `scene_low_tiles`.
fn opt_map_helper() -> String {
    let mut s = String::new();
    let mut w = |line: &str| {
        let _ = writeln!(s, "{line}");
    };
    w("");
    w("; Fill BG3's tilemap with offset-per-tile entries (see scenes.rs::opt_map_helper).");
    w("; Reads V_OPT_H_EVEN / V_OPT_H_ODD (row 0) and V_OPT_V_EVEN / V_OPT_V_ODD (row 1).");
    w(".proc scene_opt_map");
    w("    php");
    w("    .a16");
    w("    .i16");
    w("    sep #$20");
    w("    .a8");
    w("    lda #$80");
    w("    sta VMAIN");
    w("    rep #$30");
    w("    .a16");
    w("    .i16");
    w("    ldx #OPT_MAP_BASE");
    w("    stx VMADDL");
    w("    ldx #$0000                ; cell index across the whole 32x32 map");
    w("@cell:");
    w("    txa");
    w("    and #$001F                ; column");
    w("    lsr a                     ; carry = column parity");
    w("    txa");
    w("    cmp #SCREEN_COLS          ; row 0?");
    w("    bcs :+");
    w("    bcc @row0");
    w("  :");
    w("    cmp #(SCREEN_COLS * 2)    ; row 1?");
    w("    bcc @row1");
    w("    lda #$0000                ; every other row is empty");
    w("    bra @put");
    w("@row0:");
    w("    txa");
    w("    and #$0001");
    w("    bne :+");
    w("    lda f:V_OPT_H_EVEN");
    w("    bra @put");
    w("  :");
    w("    lda f:V_OPT_H_ODD");
    w("    bra @put");
    w("@row1:");
    w("    txa");
    w("    and #$0001");
    w("    bne :+");
    w("    lda f:V_OPT_V_EVEN");
    w("    bra @put");
    w("  :");
    w("    lda f:V_OPT_V_ODD");
    w("@put:");
    w("    sta VMDATAL");
    w("    inx");
    w("    cpx #(SCREEN_COLS * 32)");
    w("    bne @cell");
    w("    plp                       ; restore the caller's register widths");
    w("    rts");
    w(".endproc");
    s
}

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

    s.push_str(&low_tiles_helper());
    s.push_str(&opt_map_helper());

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
