//! Mid-line-raster cross-check — RustySNES side (T-CA-10 Phase 4c, `docs/adr/0014`).
//!
//! Renders the synthetic raster ROM (`scripts/raster_crossval/`) and reports the raster boundary —
//! the column at which the per-scanline mid-line register write takes effect. A companion MesenCE
//! render (`scripts/raster_crossval/mce_boundary.lua`) reports the same for the reference; the driver
//! `scripts/raster_crossval/raster_crossval.sh` compares them. The ROM is a build artifact
//! (gitignored), so this self-skips when it is absent, leaving CI unaffected.
//!
//! What it validates: the ROM's `DRAW` variant writes a composite register (`TM`) mid-line (boundary
//! at the DRAW cursor); the `FETCH` variant writes a BG-data register (`BGnNBA` char base) mid-line
//! (boundary at the FETCH cursor). The FETCH boundary sits ~`BG_FETCH_AHEAD` columns right of the
//! DRAW boundary, and that OFFSET — unlike the absolute boundary — is independent of the H-IRQ/ISR
//! latency, so comparing it against MesenCE isolates the compositor's fetch-vs-draw split. See the
//! README for the measured numbers.
#![cfg(feature = "test-roms")]
use rustysnes_core::{System, cart::Cart};
use std::path::PathBuf;

const SCENE_W: usize = 256;
const PROBE_ROW: usize = 112; // a row well clear of the top/bottom edge

#[test]
fn raster_boundary_is_reported() {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/raster_crossval/raster.sfc");
    if !p.is_file() {
        eprintln!(
            "SKIP raster_crossval: {p:?} absent (build with scripts/raster_crossval/build.sh)"
        );
        return;
    }
    let rom = std::fs::read(&p).unwrap();
    let mut sys = System::new(0);
    sys.bus.cart = Some(Cart::from_rom(&rom).unwrap());
    sys.reset();
    sys.bus.set_joypad(0, 0);
    sys.bus.set_joypad(1, 0);
    for _ in 0..16 {
        sys.run_frame();
    }
    let fb = sys.bus.ppu.framebuffer();
    let width = sys.bus.ppu.visible_width();
    assert_eq!(
        width, SCENE_W,
        "the raster ROM must render non-hi-res 256-wide"
    );

    // Colour A (BG1, the pre-boundary colour) canonical = red 0x7c00. Count the leading run on the
    // probe row: BG1 shows [0, boundary), the post-boundary colour after.
    let canon = |raw: u16| ((raw & 0x1F) << 10) | (raw & 0x03E0) | ((raw >> 10) & 0x1F);
    let mut boundary = SCENE_W;
    for x in 0..SCENE_W {
        if canon(fb[PROBE_ROW * width + x]) != 0x7c00 {
            boundary = x;
            break;
        }
    }
    // Report for the driver, and assert it is a genuine mid-line split (not all-A or all-B), which
    // is what the whole ROM exists to produce.
    eprintln!("RASTER_BOUNDARY row={PROBE_ROW} boundary={boundary}");
    assert!(
        boundary > 0 && boundary < SCENE_W,
        "the raster ROM must produce a mid-line A→B split, got boundary {boundary} \
         (0 = no BG1, {SCENE_W} = no write); is the ROM the intended build?"
    );
}
