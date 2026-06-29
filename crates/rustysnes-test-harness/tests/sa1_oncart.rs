#![allow(missing_docs)]
//! SA-1 on-cart coprocessor — boot + liveness + determinism gate.
//!
//! Boots the locally-staged commercial SA-1 cartridges (gitignored under
//! `tests/roms/external/commercial/LoRom/SA-1/`) on the full `rustysnes_core::System` and asserts:
//!
//! 1. **Detection** — the `$FFD6` chipset byte resolves `Coprocessor::Sa1` and the board is a
//!    `Core/Curated` SA-1 board (the SA-1 program lives in the cart ROM — no chip dump, never
//!    silently degraded, `docs/adr/0003`).
//! 2. **S-CPU ↔ SA-1 traffic (per ROM)** — the cart's coprocessor-host-access counter (S-CPU
//!    accesses to the `$2200-$23FF` register window) is non-zero: every SA-1 cart's boot code talks
//!    to the SA-1 register file, proving the `$2200-$23FF` S-CPU map is wired.
//! 3. **The SA-1 CPU executed (aggregate)** — `System::sa1_cycles()` is non-zero for at least
//!    [`LIVE_FLOOR`] of the corpus, which can only happen if the main CPU programmed the SA-1 reset
//!    vector + Super-MMC banks and cleared RESB, the scheduler instantiated the second 65C816, and
//!    that CPU fetched + ran out of the cart ROM (e.g. Super Mario RPG, both Kirby titles, PGA Tour
//!    96, Power Rangers Zeo run millions of SA-1 cycles in the boot window). It is an **aggregate**
//!    floor, not per-ROM, by design: several SA-1 titles defer waking the SA-1 (RDYB held) past a
//!    fixed boot-smoke budget — sometimes behind a main-CPU intro — so requiring 100 % liveness in
//!    N frames would be dishonest about real boot timing.
//! 4. **Determinism** — same seed + ROM ⇒ a bit-identical framebuffer across two runs (checked on a
//!    ROM that actually ran the SA-1, so the SA-1 catch-up determinism is exercised), matched
//!    against a committed golden hash (`sa1-framebuffer.tsv`).
//!
//! The ROMs live under the gitignored `tests/roms/external/`; the test self-skips when the corpus
//! is absent so CI without it stays green. Re-bless with `BLESS_SA1=1`.
//!
//! Honesty note: the golden hash pins whatever the determinism contract produces — it is a
//! determinism + liveness gate, not a per-pixel "correct title screen" assertion. The structured-
//! framebuffer signal (`distinct_colors`) is reported, not asserted, because boot timing to a fully
//! rendered screen within a fixed frame budget varies per title.
#![cfg(feature = "test-roms")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rustysnes_core::cart::Coprocessor;
use rustysnes_core::{System, cart::Cart};

/// Frames to run before hashing — enough for the boot ROM to bring up the SA-1 and start driving it
/// (Super Mario RPG renders a structured title by ~frame 60).
const FRAMES: u32 = 60;

/// Minimum number of corpus ROMs whose SA-1 second CPU must have executed (the aggregate liveness
/// floor; see the module docs for why it is aggregate, not per-ROM). Comfortably below the observed
/// live count on the staged corpus, so it is robust to small boot-timing shifts.
const LIVE_FLOOR: u32 = 8;

fn sa1_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/roms/external/commercial/LoRom/SA-1")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/sa1-framebuffer.tsv")
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
    sa1_cycles: u64,
    host_accesses: u64,
    distinct: usize,
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
    let sa1_cycles = sys.sa1_cycles().unwrap_or(0);
    let host_accesses = sys
        .bus
        .cart
        .as_ref()
        .map_or(0, |c| c.board.coprocessor_host_accesses());
    let fb = sys.bus.framebuffer();
    Some(Boot {
        copro,
        fb_hash: hash_fb(fb),
        sa1_cycles,
        host_accesses,
        distinct: distinct_colors(fb),
    })
}

/// Collect every `*.sfc` directly under the SA-1 corpus, returning `(stem, path)` sorted.
fn collect_roms(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "sfc") {
            let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
            out.push((stem, p));
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
fn sa1_boots_live_and_deterministic() {
    let dir = sa1_dir();
    if !dir.is_dir() {
        eprintln!("SKIP sa1_oncart: SA-1 corpus absent ({})", dir.display());
        return;
    }
    let roms = collect_roms(&dir);
    if roms.is_empty() {
        eprintln!("SKIP sa1_oncart: no SA-1 .sfc ROMs found");
        return;
    }

    let golden = load_golden();
    let bless = std::env::var("BLESS_SA1").is_ok();
    let mut blessed = Vec::new();
    let mut mismatches = Vec::new();
    let mut booted = 0u32;
    let mut live = 0u32; // SA-1 CPU executed (sa1_cycles > 0)
    let mut talking = 0u32; // S-CPU <-> SA-1 register traffic
    let mut first_live: Option<&PathBuf> = None;

    for (stem, path) in &roms {
        let Some(b) = boot(path) else {
            mismatches.push(format!("{stem}: failed to boot"));
            continue;
        };
        booted += 1;

        assert_eq!(b.copro, Coprocessor::Sa1, "{stem}: header must detect SA-1");
        // Per-ROM: every SA-1 cart's boot code touches the SA-1 register window.
        assert!(
            b.host_accesses > 0,
            "{stem}: no S-CPU access to the SA-1 register window — the $2200-$23FF map is broken"
        );
        talking += 1;
        if b.sa1_cycles > 0 {
            live += 1;
            if first_live.is_none() {
                first_live = Some(path);
            }
        }

        eprintln!(
            "{stem}: copro=SA-1 sa1_cycles={} host_accesses={} distinct_colors={}",
            b.sa1_cycles, b.host_accesses, b.distinct
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
        eprintln!("BLESSED sa1-framebuffer.tsv ({} entries)", blessed.len());
        return;
    }

    // Determinism (the contract, `docs/adr/0004`): a second boot is bit-identical. Check it on a
    // ROM that actually ran the SA-1 so the SA-1 catch-up determinism is exercised (it is bounded
    // entirely by the deterministic master clock).
    if let Some(path) = first_live {
        let a = boot(path).expect("re-boot");
        let c = boot(path).expect("re-boot");
        assert_eq!(
            a.fb_hash,
            c.fb_hash,
            "SA-1 framebuffer is NON-deterministic across runs ({})",
            path.display()
        );
    }

    eprintln!("sa1: booted={booted} live(SA-1 executed)={live} talking(S-CPU<->SA-1)={talking}");

    // Aggregate liveness: a meaningful number of real SA-1 carts drove the second CPU end-to-end
    // (reset handshake + Super-MMC ROM fetch + execution). Per the module docs this is a floor, not
    // 100 %, because some titles defer waking the SA-1 past the boot-smoke budget.
    assert!(
        booted == 0 || live >= LIVE_FLOOR,
        "SA-1 corpus liveness below floor: only {live}/{booted} carts ran the SA-1 CPU \
         (expected >= {LIVE_FLOOR})"
    );

    assert!(
        mismatches.is_empty(),
        "SA-1 boot/golden mismatches (re-bless with BLESS_SA1=1 if intentional):\n{}",
        mismatches.join("\n")
    );
}
