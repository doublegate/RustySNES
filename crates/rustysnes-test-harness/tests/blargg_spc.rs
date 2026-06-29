#![allow(missing_docs)]
//! blargg `spc_*` cycle-accurate SPC700 / S-DSP suite — the audio oracle (Phase 3 / T-31-003).
//!
//! These are bootable SNES ROMs: the main CPU uploads an SPC700 audio program through the four
//! `$2140-$2143` ports (the IPL boot handshake), the SPC700 + S-DSP run the test, and blargg's
//! **text shell** streams the result back to the SNES, which renders it to a BG tilemap. There is
//! **no machine-readable PASS/FAIL status byte** on the SNES side: the reference emulators
//! (confirmed against Mesen2's `RecordedRomTest`, which decides pass/fail purely by comparing
//! rendered frames to a recorded baseline) validate this suite by **framebuffer comparison**. The
//! readable signal blargg exposes is the rendered text only — a BG tilemap at VRAM word `$0400`
//! where `tile_index & 0xFF == ASCII` (so "Running tests:" / "Passed" / "Failed" can be OCR'd).
//!
//! This gate therefore does two honest things, mirroring `undisbeliever_golden.rs`:
//!
//! 1. **Determinism (the hard contract, `docs/adr/0004`):** boot each ROM on a real
//!    `rustysnes_core::System` with the integrated CPU + master-clock scheduler + the async-resync
//!    APU, run a fixed number of frames, and assert the framebuffer + ARAM + ports hash is
//!    **bit-identical across two runs**. This is the committable, ROM-independent guarantee that
//!    the integer-accumulator SPC resync is deterministic.
//! 2. **Progress baseline:** hash the rendered state against the committed baseline TSV
//!    (`tests/golden/blargg-spc.tsv`) so a regression in the boot/upload/run path is caught.
//!
//! ## Current accuracy status (honest — all four literal PASSes)
//!
//! The SMP advances **cycle-exact** in sub-instruction lockstep with the main CPU (each
//! `Apu::advance_smp_cycle` releases exactly one SMP base clock from a recorded micro-op timeline),
//! the **S-DSP is itself cycle-stepped** (32-step ares micro-sequence, one `Dsp::tick` per 2 SMP
//! base clocks), the SPC700 timer is clocked in the correct phase (`RecordingSmpBus::write` advances
//! the SMP timebase + clocks the three timers **before** the write side effect lands, matching ares
//! `step()` / Mesen2 `Spc::Write` — `IncCycleCount` first; the Phase-3 timer-phase fix,
//! `docs/apu.md` §timer phase), and the S-DSP **GAIN mode-7 (bent increase) threshold** compares its
//! internal envelope latch against `0x600` **unsigned** (matching blargg `SPC_DSP`
//! `(unsigned) hidden_env` / ares `(u32) _envelope`; `docs/apu.md` §DSP GAIN mode-7 threshold).
//!
//! With those corrected, the decoded verdicts are:
//!
//! - `spc_smp` → full per-opcode CPU-Instructions + CPU-Timing + Timers grid → **"PASSED TESTS"**.
//! - `spc_timer` → **"PASSED TESTS"**.
//! - `spc_mem_access_times` → **"PASSED TESTS"**.
//! - `spc_dsp6` → the full DSP suite (Echo · Envelope · KON · Misc · Order · Random · Timing) →
//!   **"PASSED TESTS"** (rendered at `$0800` row 30 near frame 8.8k). Previously stalled at
//!   `Envelope/gain $E0 threshold` → "Failed 02"; the GAIN mode-7 unsigned-threshold fix resolves it.
//!
//! This test therefore **asserts the literal blargg PASS for all four ROMs**. The determinism +
//! baseline-hash assertions are retained (not weakened to determinism-only); the GAIN mode-7 fix
//! does not move any ROM's 120-frame boot hash (the quirk only fires deep in the Envelope suite),
//! so the baseline TSV is unchanged.
#![cfg(feature = "test-roms")]

use std::collections::HashMap;
use std::path::PathBuf;

use rustysnes_core::{System, cart::Cart};

/// Frames to run before hashing — enough for the boot + IPL upload + the program to reach its
/// (current) stable state.
const FRAMES: u32 = 120;

/// Frames to run when decoding the on-screen verdict — long enough for blargg to stream the
/// per-opcode / result grid (the timing tests bail earlier; the opcode/DSP grids take longer). The
/// full `spc_dsp6` DSP suite (Echo · Envelope · KON · Misc · Order · Random · Timing) is the slowest:
/// it reaches its literal "PASSED TESTS" near frame 8.8k, so this is sized with margin above that.
const VERDICT_FRAMES: u32 = 12000;

/// The four blargg SPC/DSP ROMs (gitignored external tier; the test self-skips when absent).
const ROMS: [&str; 4] = ["spc_smp", "spc_timer", "spc_mem_access_times", "spc_dsp6"];

/// **All four** ROMs now reach blargg's **literal PASS**. The three timer-mechanism ROMs passed
/// after the timer-phase fix (`docs/apu.md` §timer phase); `spc_dsp6` joins them after the S-DSP
/// **GAIN mode-7 threshold** fix — the bent/two-slope `GAIN` increase compares its internal envelope
/// latch against `0x600` **unsigned** (`crates/rustysnes-apu/src/dsp.rs`, matching blargg `SPC_DSP`
/// `(unsigned) hidden_env` and ares `(u32) _envelope`), so a latch left negative by a prior GAIN
/// decrease still trips the reduced slope. A signed compare over-incremented and diverged the
/// `Envelope/gain $E0 threshold` sub-test. `spc_dsp6`'s "PASSED TESTS" is the last to render (frame
/// ~8.8k, `$0800` row 30), which is why `VERDICT_FRAMES` and `screen_text` were widened for it.
const EXPECT_PASS: [&str; 4] = ["spc_smp", "spc_timer", "spc_mem_access_times", "spc_dsp6"];

fn rom_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../tests/roms/external/blargg-spc/{name}.sfc"))
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/blargg-spc.tsv")
}

/// FNV-1a over the framebuffer + ARAM + the four CPU-readable ports — the full observable
/// audio+video state, so the determinism check covers the APU domain, not just the picture.
fn hash_state(sys: &System) -> u64 {
    const fn fold(h: u64, b: u64) -> u64 {
        (h ^ b).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in sys.bus.framebuffer() {
        h = fold(h, u64::from(p));
    }
    for &b in &sys.bus.apu.aram()[..] {
        h = fold(h, u64::from(b));
    }
    for n in 0..4u8 {
        h = fold(h, u64::from(sys.bus.apu.cpu_read_port(n)));
    }
    h
}

fn boot_and_hash(path: &std::path::Path) -> Option<u64> {
    let rom = std::fs::read(path).ok()?;
    let cart = Cart::from_rom(&rom).ok()?;
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..FRAMES {
        sys.run_frame();
    }
    Some(hash_state(&sys))
}

/// Decode blargg's rendered text (BG tilemap, `tile & 0xFF == ASCII`). blargg renders the running
/// header to VRAM word `$0400` and the scrolling per-opcode / result **grid** to `$0800`; scan the
/// full 32×32 name-table of both so a literal "Passed"/"Failed NN" verdict is captured wherever the
/// scroll has parked it. (The `spc_dsp6` "PASSED TESTS" line lands at `$0800` row 30 once its long
/// Echo/Envelope/KON/Misc/Order/Random/Timing list has scrolled — past the first 28 rows, so the
/// scan must cover all 32 nametable rows, not just the visible window.)
fn screen_text(sys: &System) -> String {
    let mut out = String::new();
    for base in [0x0400u16, 0x0800u16] {
        for i in 0..(32 * 32u16) {
            let t = (sys.bus.ppu.vram_word(base + i) & 0xFF) as u8;
            if (0x20..0x7F).contains(&t) {
                out.push(t as char);
            }
            if i % 32 == 31 {
                out.push('\n');
            }
        }
    }
    out
}

/// Run a ROM to its result and accumulate every distinct text line blargg scrolls through the
/// `$0800` grid (the grid scrolls, so the final frame alone misses earlier "Failed NN" lines).
fn collect_verdict(path: &std::path::Path) -> Option<String> {
    let rom = std::fs::read(path).ok()?;
    let cart = Cart::from_rom(&rom).ok()?;
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    let mut seen: Vec<String> = Vec::new();
    let mut prev = String::new();
    for _ in 0..VERDICT_FRAMES {
        sys.run_frame();
        let s = screen_text(&sys);
        if s != prev {
            for line in s.lines() {
                let t = line.trim();
                if t.len() > 1 && t.chars().any(|c| c.is_ascii_alphanumeric()) {
                    let owned = t.to_string();
                    if !seen.contains(&owned) {
                        seen.push(owned);
                    }
                }
            }
            prev = s;
        }
    }
    Some(seen.join("\n"))
}

fn load_golden() -> HashMap<String, u64> {
    let text = std::fs::read_to_string(golden_path()).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let (name, hex) = line.split_once('\t')?;
            let v = u64::from_str_radix(hex.trim().trim_start_matches("0x"), 16).ok()?;
            Some((name.to_string(), v))
        })
        .collect()
}

#[test]
fn blargg_spc_boots_deterministically() {
    // Self-skip when the external-tier ROMs are absent (they are gitignored, unstated license).
    if !ROMS.iter().any(|n| rom_path(n).is_file()) {
        eprintln!("SKIP blargg_spc: ROMs absent (tests/roms/external/blargg-spc/)");
        return;
    }

    let golden = load_golden();
    let mut mismatches = Vec::new();
    let mut det_checked = 0u32;

    for name in ROMS {
        let path = rom_path(name);
        if !path.is_file() {
            eprintln!("  {name}: ROM absent, skipped");
            continue;
        }

        let Some(got) = boot_and_hash(&path) else {
            mismatches.push(format!("{name}: failed to boot/hash"));
            continue;
        };
        // Determinism: a second identical run must reproduce the identical hash.
        let again = boot_and_hash(&path).unwrap_or(0);
        assert_eq!(
            got, again,
            "{name}: APU+framebuffer state is NON-deterministic across runs (resync must be \
             integer-exact, docs/adr/0004)"
        );
        det_checked += 1;

        // Decode the real on-screen verdict blargg streams (header `$0400` + grid `$0800`,
        // accumulated across the scroll), then ASSERT it: the three timer-mechanism ROMs must reach
        // blargg's literal "Passed" (`EXPECT_PASS`); `spc_dsp6` is an honest residual that must at
        // least still stream a result (a regression to a boot stall is caught). No verdict is faked.
        let decoded = collect_verdict(&path).unwrap_or_default();
        let lc = decoded.to_lowercase();
        let passed = lc.contains("passed");
        let verdict = if passed {
            "RENDERED-PASS"
        } else if lc.contains("failed") {
            "RENDERED-FAIL"
        } else {
            "no-result-streamed (boots+uploads+runs; bails in blargg's timing self-check)"
        };
        eprintln!("  {name}: hash={got:#018x} -> {verdict}");
        for line in decoded.lines() {
            eprintln!("      | {line}");
        }

        if EXPECT_PASS.contains(&name) {
            // The committable accuracy claim: a literal blargg PASS, not a determinism-only proxy.
            assert!(
                passed,
                "{name}: expected literal blargg PASS but decoded {verdict}. An SPC700/S-DSP \
                 accuracy regression has broken — see docs/apu.md (§timer phase / §DSP GAIN mode-7 \
                 threshold)."
            );
        }

        match golden.get(name) {
            Some(&exp) if exp == got => {}
            Some(&exp) => {
                mismatches.push(format!("{name}: hash {got:#018x} != baseline {exp:#018x}"));
            }
            None => mismatches.push(format!("{name}: NO baseline entry (got {got:#018x})")),
        }
    }

    eprintln!("blargg_spc: {det_checked} ROM(s) boot bit-deterministically");
    assert!(
        mismatches.is_empty(),
        "blargg-spc baseline mismatch (re-bless tests/golden/blargg-spc.tsv only for an \
         intentional boot/timing change):\n{}",
        mismatches.join("\n")
    );
}
