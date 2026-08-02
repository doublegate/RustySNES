#![allow(missing_docs)]
//! undisbeliever/snes-test-roms PPU/DMA/HDMA suite — deterministic golden framebuffer gate.
//!
//! These are *visual* hardware-behavior ROMs (HDMA glitches, INIDISP timing, S-CPU A-bus DMA
//! quirks): they render a pattern that demonstrates the behavior rather than writing a pass/fail
//! code. The committable gate is therefore a **deterministic framebuffer hash** — boot each ROM
//! on a real `rustysnes_core::System`, run a fixed number of frames, FNV-1a-hash the PPU
//! framebuffer, and assert it matches the committed baseline in
//! `tests/golden/undisbeliever-framebuffer.tsv`.
//!
//! This simultaneously satisfies two Phase-2 exit criteria: the undisbeliever suite **boots and
//! renders** through the integrated CPU + scheduler + bus + DMA/HDMA + PPU path, and the frame is
//! **bit-deterministic** (same seed + ROM ⇒ identical framebuffer — the determinism contract,
//! `docs/adr/0004`). Re-bless the TSV when an intentional rendering change lands.
#![cfg(feature = "test-roms")]

use std::collections::HashMap;
use std::path::PathBuf;

use rustysnes_core::{System, cart::Cart};

/// Frames to run before hashing (enough for the ROMs to reach their stable rendered pattern).
const FRAMES: u32 = 60;

/// ROMs whose per-dot output differs from the MesenCE-agreeing golden, with the difference
/// **adjudicated** rather than assumed to be ours. Each entry pins the exact per-dot hash, so a
/// *change* in this output still trips the gate.
///
/// Currently one: `inidisp_forgot_to_force_blank` does a PPU access during active display without
/// force-blank; per-dot returns `7fff` where MesenCE returns `7fc6`.
///
/// # This is a reference disagreement, not a RustySNES gap
///
/// It was recorded as "a Phase 4d (PPU access-during-render) gap — when 4d lands, remove the entry
/// and re-bless". Reading the third reference (2026-08-02) overturned that:
///
/// - **ares** (`sfc/ppu/dac.cpp:158`) sets `latch.cgramAddress = palette` **unconditionally**, in
///   `paletteColor`, called during per-dot priority resolution — including `paletteColor(0)` for the
///   transparent case at `dac.cpp:71`. That is RustySNES's model exactly: one assignment per dot,
///   from the composited pixel, backdrop giving zero.
/// - **MesenCE** writes it per *layer fetch* and then runs `RenderBgColor()` — which zeroes backdrop
///   columns — **after** every layer render in a span. What survives a span is therefore decided by
///   pass ordering across the whole span, which is an artefact of a span-based renderer rather than
///   a per-dot physical process.
///
/// So the `7fc6` encodes MesenCE's renderer architecture. Two corroborations: `C3.12` — the scored
/// AccuracySNES row that asserts the redirect target directly — **passes on both Mesen2 and
/// RustySNES**, so they agree on the assertion and differ only on this homebrew hash; and `C3.12`'s
/// provenance is a per-dot statement ("uses the colour the PPU *is drawing*"), which the per-dot
/// model implements directly.
///
/// **Do not "fix" this by matching MesenCE.** The golden TSV entry stays the batch value; it only
/// needs to differ from the pinned value to route through this branch.
///
/// The pinned hash was refreshed once for the DRAM-refresh stall (`docs/dram-refresh.md`): the
/// difference is unchanged, but the per-line refresh shifts the whole frame's timing, so the output
/// lands at a different instant and hashes differently.
///
/// Unconditional: the per-dot PPU is the only compositor (the batch path was removed), so this crate
/// always exercises it and the list is not gated on any feature.
const PERDOT_KNOWN_GAPS: &[(&str, u64)] =
    &[("inidisp_forgot_to_force_blank", 0xaeb6_78a4_165b_28c5)];

fn roms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/undisbeliever")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/undisbeliever-framebuffer.tsv")
}

/// FNV-1a over the 15-bit-per-pixel framebuffer (the visual-golden hash).
fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
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
    Some(hash_fb(sys.bus.framebuffer()))
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
fn undisbeliever_framebuffers_match_golden() {
    let dir = roms_dir();
    if !dir.is_dir() {
        eprintln!("SKIP undisbeliever_golden: ROM dir absent");
        return;
    }
    let golden = load_golden();
    assert!(!golden.is_empty(), "golden baseline TSV is empty/missing");

    let mut roms: Vec<_> = std::fs::read_dir(&dir)
        .expect("read undisbeliever dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sfc"))
        .collect();
    roms.sort();

    let mut mismatches = Vec::new();
    let mut checked = 0u32;
    let mut gaps = 0u32;
    for p in &roms {
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let Some(got) = boot_and_hash(p) else {
            mismatches.push(format!("{name}: failed to boot/hash"));
            continue;
        };
        // Determinism: a second run must produce the identical hash.
        let again = boot_and_hash(p).unwrap_or(0);
        assert_eq!(
            got, again,
            "{name}: framebuffer is NON-deterministic across runs"
        );

        match golden.get(&name) {
            Some(&exp) if exp == got => checked += 1,
            Some(&exp) => {
                // A documented per-dot gap: golden holds the correct (batch) hash, per-dot differs.
                // Accept it only if per-dot produces the exact pinned wrong hash — any other value is
                // a real, unexpected change and must fail.
                if let Some(&(_, gap)) = PERDOT_KNOWN_GAPS.iter().find(|(n, _)| *n == name) {
                    if got == gap {
                        eprintln!(
                            "known per-dot gap (Phase 4d pending): {name}: got {got:#018x} vs golden {exp:#018x}"
                        );
                        gaps += 1;
                    } else {
                        mismatches.push(format!(
                            "{name}: per-dot gap hash changed: got {got:#018x}, pinned {gap:#018x}, golden {exp:#018x}"
                        ));
                    }
                } else {
                    mismatches.push(format!("{name}: got {got:#018x} expected {exp:#018x}"));
                }
            }
            None => mismatches.push(format!("{name}: no golden entry (got {got:#018x})")),
        }
    }

    eprintln!(
        "undisbeliever golden: {checked}/{} matched, {gaps} documented per-dot gap(s)",
        roms.len()
    );
    assert!(
        mismatches.is_empty(),
        "framebuffer golden mismatches (re-bless tests/golden/undisbeliever-framebuffer.tsv if intentional):\n{}",
        mismatches.join("\n")
    );
}
