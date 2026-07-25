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
//! The **full** suite (`cputest-full.sfc`, 1610 tests) also reports "Success" as of the WDC 65816
//! `(dp,X)` emulation-mode `DL!=0` high-byte page-wrap fix (`docs/cpu.md`, `crates/rustysnes-cpu`).
//! Before that fix it stopped at test 39 with "Failed" — NOT at `adc ($10,s),y` (that op is
//! oracle-correct) but at the first `(dp,X)` emulation test exercising the silicon page-wrap bug,
//! which the `SingleStepTests` reference reads linearly and the hardware-accurate cores
//! (bsnes/ares/MesenCE) wrap. Both the basic and full cputest suites are now committed gates.
//!
//! The SPC-700 suite (`spctest.sfc`, 558 tests, every opcode except SLEEP/STOP) also reports
//! "Success" and is gated here too; it uses the same on-screen result protocol (an `'S'` tile at
//! VRAM `$32`).
#![cfg(feature = "test-roms")]

use std::path::PathBuf;

use rustysnes_core::{System, cart::Cart};

/// WRAM address of the gilyon `test_num` counter (ZEROPAGE + `$10`).
const TEST_NUM_ADDR: u32 = 0x00_0010;
/// Tilemap position the result text ("Success"/"FAIL") is written to.
const RESULT_TILE_VADDR: u16 = 0x32;
/// The basic suite runs tests `0..=1106` (1107 total).
const BASIC_LAST_TEST: u16 = 1106;
/// The full suite runs tests `0..=1609` (1610 total).
const FULL_LAST_TEST: u16 = 1609;

fn rom_path(subdir: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../tests/roms/gilyon/{subdir}/{name}.sfc"))
}

/// Boot a gilyon ROM and run frames until the CPU settles into its result self-loop (PC stable
/// across 60 consecutive frames) or `frame_cap`. Returns `(test_num, result_tile, settled)`.
/// `test_num` is only meaningful for the cputest ROMs (WRAM `$10`); spctest renders its count.
fn run_to_result(subdir: &str, name: &str, frame_cap: u32) -> (u16, u16, bool) {
    let rom = std::fs::read(rom_path(subdir, name)).expect("read gilyon rom");
    let cart = Cart::from_rom(&rom).expect("detect gilyon header");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();

    let mut last_pc = 0u16;
    let mut stable = 0u32;
    let mut settled = false;
    for _ in 0..frame_cap {
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
    if !rom_path("cputest", "cputest-basic").is_file() {
        eprintln!("SKIP gilyon_cputest_basic: ROM absent");
        return;
    }
    let (test_num, tile, settled) = run_to_result("cputest", "cputest-basic", 400);
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

/// The **full** gilyon 65C816 suite (1610 tests): every opcode × every addressing mode plus the
/// emulation-mode wrapping edge cases the basic suite skips. Reports "Success" as of the `(dp,X)`
/// emulation `DL!=0` high-byte page-wrap fix (`docs/cpu.md`); before it, this stopped at test 39.
#[test]
fn gilyon_cputest_full_reports_success() {
    if !rom_path("cputest", "cputest-full").is_file() {
        eprintln!("SKIP gilyon_cputest_full: ROM absent");
        return;
    }
    // The full suite runs ~1.5x the basic suite's tests, so it needs a larger frame budget.
    let (test_num, tile, settled) = run_to_result("cputest", "cputest-full", 3000);
    eprintln!("gilyon cputest-full: settled={settled} test_num={test_num} result_tile={tile:#06X}");
    assert!(settled, "ROM did not settle into its result loop");
    assert_eq!(
        test_num, FULL_LAST_TEST,
        "not all 1610 tests ran (test_num should be 1609)"
    );
    assert_eq!(
        tile & 0xFF,
        0x53,
        "result text is not 'Success' (tile != 'S') — a CPU regression the full suite caught"
    );
}

/// The gilyon SPC-700 suite (`spctest.sfc`, 558 tests: every opcode except SLEEP/STOP, each
/// addressing mode, with/without the P flag). Boots on the full `System`, runs the SPC-700 through
/// the SMP/IPL path, and asserts it reaches "Success" (an `'S'` tile at VRAM `$32`, the same result
/// protocol the cputest ROMs use).
#[test]
fn gilyon_spctest_reports_success() {
    if !rom_path("spctest", "spctest").is_file() {
        eprintln!("SKIP gilyon_spctest: ROM absent");
        return;
    }
    let (_test_num, tile, settled) = run_to_result("spctest", "spctest", 800);
    eprintln!("gilyon spctest: settled={settled} result_tile={tile:#06X}");
    assert!(settled, "spctest did not settle into its result loop");
    assert_eq!(
        tile & 0xFF,
        0x53,
        "result text is not 'Success' (tile != 'S') — an SPC-700 regression"
    );
}
