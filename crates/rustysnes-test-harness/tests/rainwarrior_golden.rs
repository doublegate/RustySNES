#![allow(missing_docs)]
//! rainwarrior (Brad Smith / bbbradsmith) SNES hi-res demo ROMs — deterministic framebuffer gate.
//!
//! `twoship` (Mode 5 512-px hi-res) and `elasticity` (per-scanline high-colour) exercise the PPU's
//! hi-res / colour-depth paths — the area the AccuracySNES scenes cover least. They render a fixed
//! picture rather than self-scoring, so the committable check is a **deterministic framebuffer hash**:
//! boot each on a real `rustysnes_core::System`, run a fixed number of frames, FNV-1a-hash the PPU
//! framebuffer, and assert it matches the committed baseline in
//! `tests/golden/rainwarrior-framebuffer.tsv`.
//!
//! The ROMs are in the **gitignored** `tests/roms/external/rainwarrior/` tier (no explicit upstream
//! license — usable locally, not redistributable; only the derived hashes are committed, per
//! `docs/adr/0003`), so this test **self-skips** when they are absent (CI, fresh clone). A local
//! developer fetches them from <https://github.com/bbbradsmith/SNES_stuff>.
//!
//! The goldens were cross-validated against MesenCE (`scripts/perdot_capture.lua`) via canonical
//! distinct-color sets — `twoship` matches exactly (26/26 colors), validating the hi-res Mode 5
//! render; `elasticity`'s only delta is 2 MesenCE-brightness-formula colors (`docs/adr/0013`,
//! `scripts/perdot_crossval.sh`). Like `undisbeliever_golden`, this is a regression/consistency
//! guard against the committed hash, with the reference agreement recorded at bless time.
#![cfg(feature = "test-roms")]

use std::collections::HashMap;
use std::path::PathBuf;

use rustysnes_core::{System, cart::Cart};

/// Frames to run before hashing — matches the MesenCE cross-validation (`MCE_FRAMES=60`).
const FRAMES: u32 = 60;

/// The exact ROM set this gate covers. Pinned here (not just inferred from the golden TSV) so the
/// gate cannot be silently narrowed by editing the TSV — the golden must name exactly these, and
/// each must be present and match.
const REQUIRED_ROMS: [&str; 2] = ["twoship", "elasticity"];

fn roms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/rainwarrior")
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/rainwarrior-framebuffer.tsv")
}

/// FNV-1a over the 15-bit-per-pixel framebuffer (same visual-golden hash as `undisbeliever_golden`).
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
    let text = std::fs::read_to_string(golden_path()).expect("read rainwarrior golden TSV");
    let mut map = HashMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, hash) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("golden line {}: not `name<TAB>hash`: {line:?}", n + 1));
        let name = name.trim().to_string();
        let hash = u64::from_str_radix(hash.trim(), 16)
            .unwrap_or_else(|_| panic!("golden line {}: hash not hex: {hash:?}", n + 1));
        assert!(
            map.insert(name.clone(), hash).is_none(),
            "golden line {}: duplicate baseline for {name:?}",
            n + 1
        );
    }
    map
}

#[test]
fn rainwarrior_framebuffers_match_golden() {
    let dir = roms_dir();
    if !dir.is_dir() {
        eprintln!("SKIP rainwarrior_golden: ROM dir absent (gitignored external tier)");
        return;
    }
    let golden = load_golden();
    // The golden must name EXACTLY the required set — no silent narrowing (a trimmed TSV) and no
    // stray extras.
    let golden_names: std::collections::HashSet<&str> = golden.keys().map(String::as_str).collect();
    let required: std::collections::HashSet<&str> = REQUIRED_ROMS.into_iter().collect();
    assert_eq!(
        golden_names, required,
        "golden TSV must pin exactly {REQUIRED_ROMS:?}, found {golden_names:?}"
    );

    let mut roms: Vec<_> = std::fs::read_dir(&dir)
        .expect("read rainwarrior dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sfc"))
        .collect();
    roms.sort();

    let mut mismatches = Vec::new();
    let mut matched: std::collections::HashSet<String> = std::collections::HashSet::new();
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
            Some(&exp) if exp == got => {
                matched.insert(name);
            }
            Some(&exp) => mismatches.push(format!("{name}: got {got:#018x} expected {exp:#018x}")),
            None => mismatches.push(format!(
                "{name}: present in corpus but unpinned in the golden"
            )),
        }
    }

    // The corpus tier is present (we did not self-skip above), so require EVERY pinned ROM to be
    // present and matched — reject partial coverage where one ROM is missing while another passes.
    for name in golden.keys() {
        if !matched.contains(name) {
            mismatches.push(format!(
                "{name}: pinned in the golden but not present/matched in the corpus"
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "rainwarrior framebuffer golden mismatches:\n  {}",
        mismatches.join("\n  ")
    );
    eprintln!(
        "rainwarrior_golden: all {} pinned framebuffer(s) matched",
        golden.len()
    );
}
