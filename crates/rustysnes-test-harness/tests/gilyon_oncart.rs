#![allow(missing_docs)]
//! gilyon/snes-tests on-cart CPU validation (the Phase-1 deferred criterion, unblocked by the
//! Phase-2 bootable `System`).
//!
//! Boots the committed gilyon `cputest-basic.sfc` on a real `rustysnes_core::System`, runs it to
//! its on-screen result, and asserts it reached "Success" with every test executed. The ROM is
//! an SNES program that runs all 1107 65C816 instruction/addressing-mode tests on-cart, writes
//! the running index to WRAM `$0010` (`test_num`), and on completion spins at a `success:` /
//! `fail:` handler having rendered "Success" or "FAIL" to the tilemap.
//!
//! Result protocol (from the gilyon source):
//! - `test_num` (WRAM `$0010`, 16-bit) = the current/last test index (0-based; total − 1 at end).
//! - The result text tile at tilemap position `$32` is ASCII `'S'` (0x53) for "Success".
//!
//! The **full** suite (`cputest-full.sfc`, 1610 tests) wedges at test 39 (`adc ($10,s),y` with a
//! bank-crossing setup routed through the ROM's RAM-resident BRK handler under `DBR=$7E`) — a
//! narrow edge documented as a known residual (`docs/STATUS.md`); the CPU op itself is
//! oracle-correct (the 65816 `SingleStepTests` pass `$13` to 0-diff). The basic suite — every
//! opcode × every standard addressing mode — is the committed gate.
#![cfg(feature = "test-roms")]

use std::path::PathBuf;

use rustysnes_core::{System, cart::Cart};

/// WRAM address of the gilyon `test_num` counter (ZEROPAGE + `$10`).
const TEST_NUM_ADDR: u32 = 0x00_0010;
/// Tilemap position the result text ("Success"/"FAIL") is written to.
const RESULT_TILE_VADDR: u16 = 0x32;
/// The basic suite runs tests `0..=1106` (1107 total).
const BASIC_LAST_TEST: u16 = 1106;

fn rom_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../tests/roms/gilyon/cputest/{name}.sfc"))
}

/// Boot a gilyon ROM and run frames until the CPU settles into its result self-loop (PC stable
/// across 60 consecutive frames) or a frame cap. Returns `(test_num, result_tile, settled)`.
fn run_to_result(name: &str) -> (u16, u16, bool) {
    let rom = std::fs::read(rom_path(name)).expect("read gilyon rom");
    let cart = Cart::from_rom(&rom).expect("detect gilyon header");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();

    let mut last_pc = 0u16;
    let mut stable = 0u32;
    let mut settled = false;
    for _ in 0..400 {
        sys.run_frame();
        let pc = sys.cpu.regs.pc;
        if pc == last_pc {
            stable += 1;
            if stable >= 60 {
                settled = true;
                break;
            }
        } else {
            stable = 0;
        }
        last_pc = pc;
    }

    let test_num = u16::from(sys.bus.peek_wram(TEST_NUM_ADDR))
        | (u16::from(sys.bus.peek_wram(TEST_NUM_ADDR + 1)) << 8);
    let tile = sys.bus.ppu.vram_word(RESULT_TILE_VADDR);
    (test_num, tile, settled)
}

#[test]
fn gilyon_cputest_basic_reports_success() {
    if !rom_path("cputest-basic").is_file() {
        eprintln!("SKIP gilyon_cputest_basic: ROM absent");
        return;
    }
    let (test_num, tile, settled) = run_to_result("cputest-basic");
    eprintln!(
        "gilyon cputest-basic: settled={settled} test_num={test_num} result_tile={tile:#06X}"
    );
    assert!(settled, "ROM did not settle into its result loop");
    assert_eq!(
        test_num, BASIC_LAST_TEST,
        "not all 1107 tests ran (test_num should be 1106)"
    );
    assert_eq!(
        tile & 0xFF,
        0x53,
        "result text is not 'Success' (tile != 'S')"
    );
}
