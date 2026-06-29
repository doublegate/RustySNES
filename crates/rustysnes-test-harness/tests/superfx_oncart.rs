#![allow(missing_docs)]
//! Super FX / GSU on-cart coprocessor — boot + liveness + determinism gate.
//!
//! Boots the locally-staged Krom GSU test ROMs (CC0/homebrew, gitignored under
//! `tests/roms/external/krom/CHIP/GSU/`) on the full `rustysnes_core::System` and asserts:
//!
//! 1. **Detection** — the `$FFD6` chipset byte resolves `Coprocessor::SuperFx` and the board is a
//!    `Core/Curated` Super FX board (the GSU program lives in the cart ROM — no chip dump, never
//!    silently degraded, `docs/adr/0003`).
//! 2. **The GSU is live** — the cart's coprocessor-activity counter is non-zero, which can only
//!    happen if the `$3000-$32FF` register window is mapped right *and* the GSU actually executed
//!    its program out of the cart ROM (the host-sync "run on Go" path). This is the GSU analogue
//!    of the DSP-1 RQM access count.
//! 3. **The GSU observably plots a bitmap** — the `FillPoly` suites fill a polygon into the Game
//!    Pak RAM. We read that RAM back (`Board::sram`) and assert a substantial non-zero bitmap, so
//!    the *whole* plot pipeline is proven end-to-end at the cart boundary — opcode fetch + cache,
//!    the ROM-buffer scan-table reads (`getbl`/`getbh`), RAM load/store (`ldw`/`stw`), and the
//!    PLOT pixel-cache → character-format flush — independent of any PPU display quirk. (A 4 bpp
//!    `FillPoly` polygon also reaches the framebuffer; PPU BG-mode coverage for 2/8 bpp is a PPU
//!    concern, not the GSU's.)
//! 4. **Determinism** — same seed + ROM ⇒ a bit-identical framebuffer across two runs, matched
//!    against a committed golden hash (`superfx-framebuffer.tsv`).
//!
//! The ROMs live under the gitignored `tests/roms/external/`; the test self-skips when the corpus
//! is absent so CI without it stays green. Re-bless with `BLESS_SUPERFX=1`.
#![cfg(feature = "test-roms")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rustysnes_core::cart::Coprocessor;
use rustysnes_core::{System, cart::Cart};

/// Frames to run before hashing — enough for the CPU to set up + trigger the GSU (which then runs
/// to completion synchronously on the host-sync path) and DMA the result toward VRAM.
const FRAMES: u32 = 30;

/// A `FillPoly` polygon plots well over this many bytes into Game Pak RAM; the threshold is far
/// above any incidental write so it can only pass if the whole plot pipeline ran.
const PLOT_BITMAP_MIN: usize = 128;

fn gsu_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/krom/CHIP/GSU")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/superfx-framebuffer.tsv")
}

fn hash_fb(fb: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in fb {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Count of distinct 15-bit colours in the framebuffer (informational "is it structured?" signal).
fn distinct_colors(fb: &[u16]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for &p in fb {
        seen.insert(p);
        if seen.len() > 64 {
            break;
        }
    }
    seen.len()
}

struct Boot {
    copro: Coprocessor,
    fb_hash: u64,
    accesses: u64,
    distinct: usize,
    /// Non-zero bytes the GSU plotted into the Game Pak RAM (the plot-pipeline liveness signal).
    ram_nonzero: usize,
}

fn boot(path: &Path) -> Option<Boot> {
    let rom = std::fs::read(path).ok()?;
    let cart = Cart::from_rom(&rom).ok()?;
    let copro = cart.header.coprocessor;
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();
    for _ in 0..FRAMES {
        sys.run_frame();
    }
    let cart = sys.bus.cart.as_ref()?;
    let accesses = cart.board.coprocessor_host_accesses();
    let ram_nonzero = cart.board.sram().iter().filter(|&&b| b != 0).count();
    let fb = sys.bus.framebuffer();
    Some(Boot {
        copro,
        fb_hash: hash_fb(fb),
        accesses,
        distinct: distinct_colors(fb),
        ram_nonzero,
    })
}

/// Recursively collect every `*.sfc` under the GSU corpus, returning `(stem, path)` sorted.
fn collect_roms(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "sfc") {
                let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
                out.push((stem, p));
            }
        }
    }
    out.sort();
    out
}

fn load_golden() -> HashMap<String, u64> {
    std::fs::read_to_string(golden_path())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('\t')?;
            Some((
                k.to_string(),
                u64::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok()?,
            ))
        })
        .collect()
}

#[test]
fn superfx_boots_live_and_deterministic() {
    let dir = gsu_dir();
    if !dir.is_dir() {
        eprintln!("SKIP superfx_oncart: GSU corpus absent ({})", dir.display());
        return;
    }
    let roms = collect_roms(&dir);
    if roms.is_empty() {
        eprintln!("SKIP superfx_oncart: no GSU .sfc ROMs found");
        return;
    }

    let golden = load_golden();
    let bless = std::env::var("BLESS_SUPERFX").is_ok();
    let mut blessed = Vec::new();
    let mut mismatches = Vec::new();
    let mut booted = 0u32;
    let mut live = 0u32; // GSU executed (accesses > 0)
    let mut plotted = 0u32; // FillPoly suites that plotted a substantial bitmap into Game Pak RAM

    for (stem, path) in &roms {
        let Some(b) = boot(path) else {
            mismatches.push(format!("{stem}: failed to boot"));
            continue;
        };
        booted += 1;

        assert_eq!(
            b.copro,
            Coprocessor::SuperFx,
            "{stem}: header must detect Super FX"
        );
        // The GSU must have executed its program out of cart ROM.
        assert!(
            b.accesses > 0,
            "{stem}: no GSU activity — the register window or the run path is broken"
        );
        if b.accesses > 0 {
            live += 1;
        }

        // The FillPoly suites must plot a substantial bitmap into Game Pak RAM (the end-to-end
        // plot pipeline). PlotPixel plots a single pixel and PlotLine a thin line by design, so
        // only FillPoly carries the strong "GSU drew a shape" threshold.
        if stem.contains("FillPoly") {
            assert!(
                b.ram_nonzero >= PLOT_BITMAP_MIN,
                "{stem}: GSU plotted only {} non-zero RAM bytes (< {PLOT_BITMAP_MIN}) — the plot \
                 pipeline did not fill the polygon",
                b.ram_nonzero
            );
            plotted += 1;
        }

        eprintln!(
            "{stem}: copro=SuperFx accesses={} ram_nonzero={} distinct_colors={}",
            b.accesses, b.ram_nonzero, b.distinct
        );

        if bless {
            blessed.push(format!("{stem}\t{:#018x}", b.fb_hash));
            continue;
        }
        match golden.get(stem) {
            Some(&exp) if exp == b.fb_hash => {}
            Some(&exp) => mismatches.push(format!(
                "{stem}: got {:#018x} expected {exp:#018x}",
                b.fb_hash
            )),
            None => mismatches.push(format!("{stem}: no golden entry (got {:#018x})", b.fb_hash)),
        }
    }

    if bless {
        blessed.sort();
        std::fs::write(golden_path(), format!("{}\n", blessed.join("\n"))).expect("write golden");
        eprintln!(
            "BLESSED superfx-framebuffer.tsv ({} entries)",
            blessed.len()
        );
        return;
    }

    // Determinism (the contract, `docs/adr/0004`): a second boot of a FillPoly ROM is bit-
    // identical (the GSU is purely a function of seed + ROM, host-sync removing any clock skew).
    if let Some((stem, path)) = roms.iter().find(|(s, _)| s.contains("FillPoly")) {
        let a = boot(path).expect("re-boot");
        let b = boot(path).expect("re-boot");
        assert_eq!(
            a.fb_hash, b.fb_hash,
            "{stem}: Super FX framebuffer is NON-deterministic across runs"
        );
    }

    eprintln!(
        "superfx: booted={booted} live(GSU executed)={live} plotted(FillPoly bitmaps)={plotted}"
    );

    // Liveness: every booted Super FX cart actually ran the GSU (asserted per-ROM above); and at
    // least one FillPoly suite plotted a real bitmap (proven per-ROM above). Guard the corpus is
    // non-trivial so an empty run can't silently pass.
    assert!(
        booted == 0 || (live == booted && plotted > 0),
        "Super FX corpus did not exercise the GSU end-to-end (live={live}/{booted}, plotted={plotted})"
    );

    assert!(
        mismatches.is_empty(),
        "Super FX boot/golden mismatches (re-bless with BLESS_SUPERFX=1 if intentional):\n{}",
        mismatches.join("\n")
    );
}
