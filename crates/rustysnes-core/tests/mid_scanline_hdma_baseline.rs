//! Regression baseline for the confirmed off-by-one-line HDMA/compositor timing bug
//! (`docs/ppu.md` §"Mid-scanline/HDMA-driven register timing — researched, confirmed, deferred
//! (v0.5.0)"). This test does **not** assert correct hardware behavior — it locks in RustySNES's
//! current (confirmed-buggy) output for a minimal, self-authored reproduction, so any *unrelated*
//! regression to this exact scenario is still caught, and so the eventual fix has a concrete,
//! numeric target to flip.
//!
//! The reproduction drives `$2100` (`INIDISP`, master brightness) with HDMA mode 0 (1 byte, 1
//! register), alternating between full brightness (`$0F`, backdrop renders white) and force-off
//! brightness (`$00`, backdrop renders black) at a single transition partway through the frame —
//! chosen instead of a scroll register because it needs no BG/tilemap setup at all: with every
//! background layer left disabled, every pixel falls through to the backdrop color
//! (`Ppu::layer_color`'s `!p.opaque` path returns `cgram[0]`), so this isolates the exact
//! compositor-vs-HDMA dot-timing bug from any background-rendering code at all.

use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use rustysnes_core::ppu::SCREEN_WIDTH;

/// Number of visible scanlines the first ("A", full brightness) HDMA table phase covers.
const PHASE_A_LINES: usize = 100;
/// Non-overscan visible height (`Ppu::visible_height()` with `overscan == false`).
const VISIBLE_LINES: usize = 224;

/// Build a minimal LoROM ROM whose reset-vector program:
/// 1. Sets CGRAM entry 0 (the backdrop color) to white (`$7FFF`) once.
/// 2. Sets `$2100` (`INIDISP`) to full brightness (`$0F`) once, force-blank off.
/// 3. Programs HDMA channel 0 for mode 0 (1 byte -> `$2100`), source = the table below.
/// 4. Enables HDMAEN channel 0, then spins forever.
///
/// No background layer is ever enabled (`$212C`/`$212D` stay at their power-on `0`), so the
/// entire visible frame renders as a flat backdrop-color field whose per-line brightness is
/// driven purely by the HDMA table — the simplest possible probe for the compositor's
/// end-of-line register-read timing relative to that same line's own HDMA run.
///
/// The HDMA table uses `count = 1`, non-repeat entries for every single line (never the `bit7`
/// "continuous" repeat mode) so this reproduction doesn't depend on this crate's own repeat-mode
/// data-pointer semantics being exercised correctly — only the well-exercised one-line-at-a-time
/// path already proven by the committed `undisbeliever` HDMA goldens.
fn mid_scanline_hdma_probe_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x1_0000];

    #[rustfmt::skip]
    let program: [u8; 52] = [
        0x78,                   // SEI
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x21, 0x21,       // STA $2121      (CGADD = 0)
        0xA9, 0xFF,             // LDA #$FF
        0x8D, 0x22, 0x21,       // STA $2122      (CGDATA low: backdrop = white low byte)
        0xA9, 0x7F,             // LDA #$7F
        0x8D, 0x22, 0x21,       // STA $2122      (CGDATA high: backdrop = white high byte)
        0xA9, 0x0F,             // LDA #$0F
        0x8D, 0x00, 0x21,       // STA $2100      (INIDISP: force-blank off, brightness 15)
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x00, 0x43,       // STA $4300      (DMAP0: A->B, direct, mode 0 -- 1 byte/reg)
        0x8D, 0x01, 0x43,       // STA $4301      (BBAD0 = $00 -> target $2100)
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x02, 0x43,       // STA $4302      (A1T0L: table addr low)
        0xA9, 0x90,             // LDA #$90
        0x8D, 0x03, 0x43,       // STA $4303      (A1T0H: table addr high -> $9000)
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x04, 0x43,       // STA $4304      (A1B0: table bank 0)
        0xA9, 0x01,             // LDA #$01
        0x8D, 0x0C, 0x42,       // STA $420C      (HDMAEN: enable channel 0)
        0x4C, 0x31, 0x80,       // loop: JMP $8031 (self -- spin forever; offset 49 = $8031)
    ];
    rom[..program.len()].copy_from_slice(&program);

    // HDMA table at ROM offset 0x1000 == CPU address $9000 (A1T0H:A1T0L above).
    // Every entry is [count=$01 (non-repeat, 1 line), data byte]. VISIBLE_LINES entries total,
    // then a $00 terminator. Phase A (full brightness) for the first PHASE_A_LINES scanlines,
    // phase B (force-off) for the remainder.
    let table_offset = 0x1000;
    let mut w = table_offset;
    for line in 0..VISIBLE_LINES {
        let data = if line < PHASE_A_LINES { 0x0F } else { 0x00 };
        rom[w] = 0x01;
        rom[w + 1] = data;
        w += 2;
    }
    rom[w] = 0x00; // terminator

    let h = 0x7FC0;
    rom[h..h + 21].copy_from_slice(b"MIDLINE HDMA PROBE   ");
    rom[h + 0x15] = 0x20; // LoROM, slow
    rom[h + 0x16] = 0x00; // no coprocessor, no RAM, no battery
    rom[h + 0x18] = 0x00; // RAM size 0
    rom[h + 0x19] = 0x01; // North America / NTSC
    let checksum: u16 = 0x1234;
    let complement = !checksum;
    rom[h + 0x1C..h + 0x1E].copy_from_slice(&complement.to_le_bytes());
    rom[h + 0x1E..h + 0x20].copy_from_slice(&checksum.to_le_bytes());
    rom[h + 0x3C..h + 0x3E].copy_from_slice(&0x8000u16.to_le_bytes()); // reset vector
    rom
}

fn booted_system() -> System {
    let mut sys = System::new(0);
    sys.bus.cart = Some(Cart::from_rom(&mid_scanline_hdma_probe_rom()).expect("probe ROM header"));
    sys.reset();
    sys
}

/// Scans the composited framebuffer's backdrop column (x=0, uniform across every row since no
/// BG/OBJ layer is enabled) and returns `(last_white_row, first_black_row)`.
fn find_transition(framebuffer: &[u16]) -> (usize, usize) {
    let mut last_white = None;
    let mut first_black = None;
    for row in 0..VISIBLE_LINES {
        let px = framebuffer[row * SCREEN_WIDTH];
        if px == 0x7FFF {
            last_white = Some(row);
        } else if px == 0x0000 && first_black.is_none() {
            first_black = Some(row);
        }
    }
    (
        last_white.expect("at least one white row expected"),
        first_black.expect("at least one black row expected"),
    )
}

/// Locks in the CURRENT (confirmed-buggy) transition position: RustySNES's end-of-line
/// compositor reads `$2100` at dot 340, AFTER that same line's own HDMA run at dot 276 has
/// already written the *next* phase's value — so the transition lands one scanline EARLIER than
/// real hardware. `docs/ppu.md`'s analysis derives the two candidate positions precisely:
///
/// - **Real hardware** (not what this asserts): last-white row 100 (`V=101`), first-black row
///   101 (`V=102`) — line `V`'s own HDMA write only takes effect starting `V+1`.
/// - **RustySNES today** (what this asserts): last-white row 99 (`V=100`), first-black row 100
///   (`V=101`) — the write meant for `V+1` visibly lands on `V` itself.
///
/// When the real fix lands (`docs/ppu.md`'s "What a future investigation/fix needs"), this
/// specific assertion is EXPECTED to change to `(100, 101)` — that flip, confirmed deliberately
/// and reviewed (a Golden-Vector update, not an accidental diff), is the fix's acceptance test.
#[test]
fn mid_scanline_hdma_transition_is_one_line_early_current_known_bug() {
    let mut sys = booted_system();
    sys.run_frame();

    let (last_white, first_black) = find_transition(sys.bus.framebuffer());

    assert_eq!(
        (last_white, first_black),
        (99, 100),
        "current (buggy) transition position changed -- if this is an intentional fix for the \
         off-by-one-line bug (docs/ppu.md §Mid-scanline/HDMA-driven register timing) landing at \
         exactly (100, 101), update this assertion deliberately and cross-check against the full \
         `--features test-roms` golden suite before committing; any other value is a real, \
         unrelated regression"
    );

    // Sanity: exactly one transition (no HDMA-table or register-write bugs of this test's own
    // making producing a noisier, ambiguous pattern) -- every row up to and including
    // `last_white` is white, and every row after it is black.
    let framebuffer = sys.bus.framebuffer();
    for row in 0..VISIBLE_LINES {
        let px = framebuffer[row * SCREEN_WIDTH];
        let expected = if row <= last_white { 0x7FFF } else { 0x0000 };
        assert_eq!(
            px, expected,
            "row {row} was {px:#06x}, expected {expected:#06x} -- the probe ROM's HDMA table or \
             CGRAM setup is not behaving as this test assumes"
        );
    }
}

/// Determinism sanity: re-running the exact same probe from a fresh `System` produces the exact
/// same composited framebuffer (this is a pure function of the deterministic core,
/// `docs/adr/0004` — no wall-clock, no OS RNG anywhere on this path). Compares the full
/// framebuffer rather than just the derived transition point for a stronger guarantee.
#[test]
fn mid_scanline_hdma_probe_is_deterministic_across_fresh_runs() {
    let mut a = booted_system();
    a.run_frame();
    let mut b = booted_system();
    b.run_frame();
    assert_eq!(a.bus.framebuffer(), b.bus.framebuffer());
}
