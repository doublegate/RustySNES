//! The OAM / sprite viewer panel (`v1.25.0`, T-FP-C1).
//!
//! `PpuSnapshot::oam` has carried the full 544-byte OAM since the debugger overlay existed, and
//! nothing read it — the PPU panel showed registers only. OAM is the one PPU structure whose raw
//! bytes are genuinely unreadable by eye: each sprite's X sign bit and size bit live in a *separate*
//! 32-byte high table, two bits per sprite, so "why is this sprite off-screen" is a question the hex
//! cannot answer but the decode can.
//!
//! Pure decode over the snapshot; touches nothing.

use crate::debug_snapshot::DebugSnapshot;

/// Sprites in OAM.
pub const SPRITE_COUNT: usize = 128;
/// Where the high table begins (`SPRITE_COUNT * 4`).
pub const HIGH_TABLE: usize = SPRITE_COUNT * 4;
/// The tallest display the PPU produces: 239 lines, the `$2133` overscan mode. The off-screen
/// heuristic keys on this rather than the 224 of a normal NTSC frame so it cannot mark a sprite
/// that overscan genuinely puts on screen.
pub const MAX_VISIBLE_LINES: u8 = 239;

/// One decoded OAM entry.
///
/// The four `bool`s are four independent hardware bits from two different OAM tables, not a state
/// machine: the lint's suggested enum refactor would obscure exactly the bit-for-bit correspondence
/// this type exists to make legible.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sprite {
    /// Index in OAM (0-127); also the priority order within a priority level.
    pub index: u8,
    /// Screen X, **sign-extended** from the high table's per-sprite bit — the whole reason this
    /// panel exists, since the low table only holds the bottom 8 bits.
    pub x: i16,
    /// Screen Y (0-255; a sprite at Y >= 224 is below a normal NTSC display, and at
    /// Y >= 239 below even an overscan one).
    pub y: u8,
    /// Tile number within the sprite name table (0-255; `name_select` picks which table).
    pub tile: u8,
    /// Name-table select bit (chooses between the two OBJ character bases).
    pub name_select: bool,
    /// Palette index 0-7 (CGRAM entries 128 + 16*palette).
    pub palette: u8,
    /// Priority 0-3.
    pub priority: u8,
    /// Horizontal flip.
    pub flip_x: bool,
    /// Vertical flip.
    pub flip_y: bool,
    /// The high table's size bit: `false` = the small of the configured pair, `true` = the large.
    pub large: bool,
}

impl Sprite {
    /// Whether the sprite is positioned entirely outside the display.
    ///
    /// A heuristic for the panel's "off-screen" marker, deliberately biased toward *not* marking:
    /// it uses the maximum sprite extent (64 px) rather than the configured size, and the taller
    /// [`MAX_VISIBLE_LINES`] rather than 224 — a game running 239-line overscan puts real, visible
    /// sprites on lines 224-238, and marking those off-screen would hide the very sprite being
    /// hunted for. The panel does not know the current `$2133` overscan bit, so it assumes the
    /// larger screen; the cost of that assumption is at most an unmarked sprite in a thin band,
    /// which is the harmless direction.
    #[must_use]
    pub const fn likely_offscreen(self) -> bool {
        self.x <= -64 || self.x >= 256 || self.y >= MAX_VISIBLE_LINES
    }
}

/// Decode all 128 sprites from a raw 544-byte OAM image.
///
/// The high table packs two bits per sprite (`X` bit 8, then the size bit) into one byte per four
/// sprites, which is why the shift is `(index % 4) * 2` rather than anything simpler.
#[must_use]
pub fn decode(oam: &[u8; 544]) -> Vec<Sprite> {
    (0..SPRITE_COUNT)
        .map(|i| {
            let base = i * 4;
            let attr = oam[base + 3];
            let high_byte = oam[HIGH_TABLE + i / 4];
            let shift = (i % 4) * 2;
            let x_high = (high_byte >> shift) & 0x01;
            let large = (high_byte >> (shift + 1)) & 0x01 != 0;
            // X is 9-bit signed: bit 8 set means a negative screen position (the sprite is partly
            // off the left edge), which is why this sign-extends instead of just OR-ing bit 8 in.
            let x = if x_high == 0 {
                i16::from(oam[base])
            } else {
                i16::from(oam[base]) - 256
            };
            Sprite {
                index: u8::try_from(i).unwrap_or(u8::MAX),
                x,
                y: oam[base + 1],
                tile: oam[base + 2],
                name_select: attr & 0x01 != 0,
                palette: (attr >> 1) & 0x07,
                priority: (attr >> 4) & 0x03,
                flip_x: attr & 0x40 != 0,
                flip_y: attr & 0x80 != 0,
                large,
            }
        })
        .collect()
}

/// Render the panel: a summary line plus the decoded table.
pub fn render(ui: &mut egui::Ui, debug: &DebugSnapshot) {
    let sprites = decode(&debug.ppu.oam);
    let onscreen = sprites.iter().filter(|s| !s.likely_offscreen()).count();
    ui.label(format!(
        "{onscreen} of {SPRITE_COUNT} sprites plausibly on-screen"
    ));
    ui.separator();
    egui::ScrollArea::vertical()
        .max_height(360.0)
        .show(ui, |ui| {
            egui::Grid::new("oam_grid")
                .num_columns(8)
                .striped(true)
                .show(ui, |ui| {
                    for h in ["#", "X", "Y", "Tile", "Pal", "Pri", "Flip", "Size"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for s in &sprites {
                        // Off-screen rows are dimmed rather than hidden: "sprite 47 exists but is
                        // parked at Y=240" is exactly the answer this panel is opened for.
                        let dim = s.likely_offscreen();
                        let cell = |t: String| {
                            let r = egui::RichText::new(t).monospace();
                            if dim { r.weak() } else { r }
                        };
                        ui.label(cell(format!("{:3}", s.index)));
                        ui.label(cell(format!("{:4}", s.x)));
                        ui.label(cell(format!("{:3}", s.y)));
                        ui.label(cell(format!(
                            "{:02X}{}",
                            s.tile,
                            if s.name_select { "'" } else { " " }
                        )));
                        ui.label(cell(format!("{}", s.palette)));
                        ui.label(cell(format!("{}", s.priority)));
                        ui.label(cell(
                            match (s.flip_x, s.flip_y) {
                                (false, false) => "  ",
                                (true, false) => "H ",
                                (false, true) => " V",
                                (true, true) => "HV",
                            }
                            .to_string(),
                        ));
                        ui.label(cell(if s.large { "large" } else { "small" }.to_string()));
                        ui.end_row();
                    }
                });
        });
}

#[cfg(test)]
mod tests {
    use super::{HIGH_TABLE, SPRITE_COUNT, decode};

    fn blank() -> [u8; 544] {
        [0u8; 544]
    }

    /// The X sign bit lives in the high table, two bits per sprite — the decode this panel exists
    /// for. Getting the shift wrong silently reads a *neighbouring* sprite's bits.
    #[test]
    fn x_sign_bit_comes_from_the_high_table_at_the_right_shift() {
        let mut oam = blank();
        // Sprite 0: X low = 0x10, high bit set -> -240.
        oam[0] = 0x10;
        // Sprite 3 shares high byte 0 at shift 6.
        oam[12] = 0x20;
        // Sprite N owns bits (2N, 2N+1) = (x-high, size). Bit 0 is sprite 0's x-high; bit 7 is
        // sprite 3's SIZE (its x-high is bit 6, left clear here).
        oam[HIGH_TABLE] = 0b1000_0001;
        let s = decode(&oam);
        assert_eq!(s[0].x, 0x10 - 256);
        assert!(!s[0].large);
        assert_eq!(s[1].x, 0, "neighbour must not pick up sprite 0's bit");
        assert_eq!(s[3].x, 0x20, "sprite 3's x-high is clear");
        assert!(s[3].large, "sprite 3's size bit is set");
    }

    /// A second high byte covers sprites 4-7, so the byte index is `i / 4`.
    #[test]
    fn high_table_byte_index_advances_every_four_sprites() {
        let mut oam = blank();
        oam[4 * 4] = 0x05;
        oam[HIGH_TABLE + 1] = 0b0000_0001; // sprite 4, shift 0
        let s = decode(&oam);
        assert_eq!(s[4].x, 0x05 - 256);
        assert_eq!(s[0].x, 0);
    }

    /// The attribute byte's bit layout: name-select, palette, priority, and the two flips.
    #[test]
    fn attribute_byte_decodes_every_field() {
        let mut oam = blank();
        oam[3] = 0b1100_1111; // flipY, flipX, pri=0, pal=7, nameSelect=1
        let s = decode(&oam);
        assert!(s[0].flip_x && s[0].flip_y);
        assert!(s[0].name_select);
        assert_eq!(s[0].palette, 7);
        assert_eq!(s[0].priority, 0);

        oam[3] = 0b0011_0000; // pri=3, pal=0, nameSelect=0, no flips
        let s = decode(&oam);
        assert_eq!(s[0].priority, 3);
        assert!(!s[0].flip_x && !s[0].flip_y && !s[0].name_select);
    }

    /// Off-screen uses the maximum sprite extent, so it never hides a sprite that might be partly
    /// visible — being wrong in the other direction would hide the sprite being hunted for.
    #[test]
    fn offscreen_is_conservative() {
        let mut oam = blank();
        oam[1] = 200; // Y on-screen
        let s = decode(&oam);
        assert!(!s[0].likely_offscreen());

        // 230 is below a 224-line display but ON a 239-line overscan one, and the panel cannot
        // see the `$2133` bit — so it must NOT mark it. This assertion used to read the other
        // way, which is how the heuristic came to hide real sprites in the overscan band.
        oam[1] = 230;
        let s = decode(&oam);
        assert!(!s[0].likely_offscreen());

        // Past even the overscan display, it is off-screen on any configuration.
        oam[1] = 245;
        let s = decode(&oam);
        assert!(s[0].likely_offscreen());

        // X = -32 could still put a 64-px sprite half on-screen.
        let mut oam = blank();
        oam[0] = 0xE0; // 256 - 32
        oam[super::HIGH_TABLE] = 0b0000_0001;
        let s = decode(&oam);
        assert_eq!(s[0].x, -32);
        assert!(!s[0].likely_offscreen());
    }

    #[test]
    fn decodes_every_sprite() {
        assert_eq!(decode(&blank()).len(), SPRITE_COUNT);
    }
}
