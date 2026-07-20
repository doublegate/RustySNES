#![allow(missing_docs)]
#![cfg(feature = "test-roms")]
//! AccuracySNES rendered scenes — the host-side framebuffer oracle (`docs/adr/0013`).
//!
//! # Why this is a separate tier
//!
//! Part of the PPU decides only *what appears on screen*: backgrounds and modes, colour math and
//! windows, mosaic, direct colour. No register reads back and no counter moves, and a cart cannot
//! see its own framebuffer — so those assertions cannot be self-scored.
//!
//! The cart therefore renders and **this** judges. Results are reported here and are deliberately
//! **not** folded into the on-cart pass rate in `accuracysnes.rs`. That separation is the point:
//! the battery's headline number means "the identical image means the same thing on any emulator
//! and on real hardware", and a rendered scene does not have that property — on a flash cart it
//! displays a picture, and only a host holding the golden can say whether it is the right one.
//!
//! # How capture works
//!
//! The cart drives itself rather than being driven, which is what preserves portability. After the
//! battery finishes, the runtime walks its scene list: set up PPU state, publish the scene ID to
//! `R_SCENE`, hold for `SCENE_FRAMES` frames, repeat. This steps frames, watches the marker, and
//! hashes on the last frame of each hold.
//!
//! # A golden is agreement, not truth
//!
//! `docs/scheduler.md` records what that distinction cost when `hdmaen_latch_test` had to be
//! re-blessed. Per ADR 0013 rule 4 a scene's golden is committed only once the references agree on
//! it; a scene without one is reported as **unblessed** and does not fail the gate, because an
//! unblessed scene is not yet evidence of anything.

use std::collections::BTreeMap;
use std::path::PathBuf;

use rustysnes_core::{System, cart::Cart};

/// Results-block offsets, shared with `asm/runtime.inc`.
const RESULTS: u32 = 0x7E_F000;
const R_DONE: u32 = RESULTS + 0x08;
const R_SCENE: u32 = RESULTS + 0x12;
const R_SCENE_DONE: u32 = RESULTS + 0x13;
const DONE_MARK: u8 = 0xA5;
const SCENE_DONE_MARK: u8 = 0x5A;

/// Frame budget. The battery runs first, then one hold per scene.
const MAX_FRAMES: u32 = 4_000;

/// Which frame of a scene's published window to hash, 1-based. Must match the reference hosts.
const CAPTURE_SIGHTING: u32 = 2;

fn rom_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/roms/AccuracySNES/build/accuracysnes.sfc")
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/AccuracySNES/build/scenes.tsv")
}

/// Scene index (1-based, as the cart publishes it) -> stable scene ID.
///
/// The cart can only publish a number, and a number is a poor golden key: inserting a scene would
/// silently re-point every golden after it at a different picture. The generator therefore writes
/// this manifest next to the ROM it built.
fn manifest() -> BTreeMap<u8, String> {
    let text = std::fs::read_to_string(manifest_path()).unwrap_or_default();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut f = l.split('\t');
            let idx: u8 = f.next()?.trim().parse().ok()?;
            Some((idx, f.next()?.trim().to_string()))
        })
        .collect()
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/accuracysnes-scenes.tsv")
}

/// The canonical region every scene is hashed over: the visible NTSC picture.
///
/// Fixed rather than derived, because the hash has to mean the same thing on emulators that do not
/// agree about geometry — RustySNES composites 256x239, snes9x's libretro core hands back 256x224,
/// and an overscan or hi-res frame is wider or taller still. Hashing "whatever the emulator gave
/// us" would make a golden a statement about the emulator's output conventions rather than about
/// the picture. So scenes stay inside 256x224 and non-hi-res; `SCENE_W`/`SCENE_H` are the contract.
const SCENE_W: usize = 256;
const SCENE_H: usize = 224;

/// The buffer row this host's picture starts on.
///
/// Each host reports its frame with its own leading rows: RustySNES composites from scanline 0
/// (the first visible NTSC line is 1), snes9x's libretro core already starts at the first visible
/// line, and Mesen2 hands back 256x239 whose picture begins 7 rows in. These are output
/// conventions, exactly like pixel format, and they are calibrated by comparing renders — with the
/// wrong value two emulators that agree completely still produce different hashes, which is what
/// made the first three-way comparison look like a triple disagreement.
const FIRST_ROW: usize = 0;

/// FNV-1a over the scene region, on **canonical** `0RRRRRGGGGGBBBBB` pixels.
///
/// The SNES framebuffer here is BGR555 (`0bbbbbgggggrrrrr`, see `frontend::gfx`) and a libretro
/// core hands back RGB565 or XRGB8888. Each side converts to the same canonical 15-bit form before
/// hashing, so a golden compares pictures rather than pixel-format conventions. The hash itself is
/// the one `undisbeliever_golden.rs` uses, so the two golden sets are comparable in kind.
fn hash_scene(fb: &[u16], width: usize) -> (u64, Vec<u16>) {
    assert_eq!(
        width, SCENE_W,
        "a rendered scene must not be hi-res: the golden is defined over {SCENE_W}x{SCENE_H}"
    );
    assert!(
        fb.len() >= (FIRST_ROW + SCENE_H) * width,
        "the framebuffer holds {} pixels, too few for rows {FIRST_ROW}..{} at width {width} — the \
         scene region is a contract, and silently hashing whatever fits would produce a golden \
         that describes a different picture",
        fb.len(),
        FIRST_ROW + SCENE_H
    );
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut px = Vec::with_capacity(SCENE_W * SCENE_H);
    for y in FIRST_ROW..FIRST_ROW + SCENE_H {
        for x in 0..SCENE_W {
            let p = fb[y * width + x];
            let canonical = ((p & 0x1F) << 10) | (p & 0x03E0) | ((p >> 10) & 0x1F);
            px.push(canonical);
            h ^= u64::from(canonical);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    (h, px)
}

/// Committed goldens, keyed by scene ID. Absent file or absent key = unblessed, not failing.
fn goldens() -> BTreeMap<String, u64> {
    let Ok(text) = std::fs::read_to_string(golden_path()) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let (k, v) = l.split_once('\t')?;
            let v = v.trim().strip_prefix("0x")?;
            Some((k.to_string(), u64::from_str_radix(v, 16).ok()?))
        })
        .collect()
}

/// Run the cart and hash the framebuffer at the end of every scene's hold.
///
/// Returns `(scene index -> hash)`, 1-based to match the cart's IDs.
fn capture_scenes() -> BTreeMap<u8, u64> {
    let rom = std::fs::read(rom_path()).expect("AccuracySNES ROM must be built");
    let cart = Cart::from_rom(&rom).expect("header must be detectable");
    let mut sys = System::new(0);
    sys.bus.cart = Some(cart);
    sys.reset();

    let mut seen: BTreeMap<u8, u64> = BTreeMap::new();
    let mut sightings: BTreeMap<u8, u32> = BTreeMap::new();
    let mut current = 0u8;
    let mut battery_done = false;

    for _ in 0..MAX_FRAMES {
        sys.run_frame();
        if !battery_done {
            battery_done = sys.bus.peek_wram(R_DONE) == DONE_MARK;
            continue;
        }
        let scene = sys.bus.peek_wram(R_SCENE);
        if scene != 0 {
            *sightings.entry(scene).or_insert(0) += 1;
        }
        // The SECOND sighting, by agreement with the reference hosts. A host samples WRAM at its
        // own frame boundary, which need not be the one the cart's vblank poll sees, so the first
        // and last frames of the published window are each at risk of being off by one: snes9x
        // caught scene 1 with a black band where forced blank was released, and this harness
        // caught the previous scene outright. Both ends are avoided by agreeing on an interior
        // frame rather than by trying to make the two clocks agree.
        if scene != 0 && sightings[&scene] == CAPTURE_SIGHTING && !seen.contains_key(&scene) {
            let width = sys.bus.ppu.visible_width();
            let (hash, px) = hash_scene(sys.bus.ppu.framebuffer(), width);
            seen.insert(scene, hash);
            // Same escape hatch the libretro host has (`--scene-dump=`): when two renders disagree,
            // the hashes say only *that* they differ. Set ACCURACYSNES_SCENE_DUMP to a path prefix
            // and the canonical pixels land next to the reference's for a real diff.
            if let Ok(prefix) = std::env::var("ACCURACYSNES_SCENE_DUMP") {
                let bytes: Vec<u8> = px.iter().flat_map(|p| p.to_le_bytes()).collect();
                let _ = std::fs::write(format!("{prefix}.scene{scene}.bin"), bytes);
            }
            current = scene;
        }
        if sys.bus.peek_wram(R_SCENE_DONE) == SCENE_DONE_MARK && current != 0 {
            break;
        }
    }
    seen
}

/// Scene pairs that must render **identically**, and the behaviour that makes them so.
///
/// A stronger statement than "each matches its committed number": an equivalence survives a change
/// to the canvas, and it catches a core that gets both scenes wrong in the same way — which two
/// independent hash comparisons cannot, because a consistent misreading matches neither number and
/// so produces two failures that look unrelated.
const EQUIVALENCES: &[(&str, &str, &str)] = &[
    (
        "c8-half-ignored-on-fixed-backdrop",
        "c8-fixed-colour-add",
        "C8.03: CGADSUB's half/div2 bit is ignored when the subscreen is the fixed backdrop, so \
         setting it must change nothing",
    ),
    (
        "c8-window-left-gt-right-empty",
        "c8-both-windows-disabled-empty",
        "C8.05 and C8.07: crossed window bounds and no enabled window are both EMPTY masks rather \
         than full ones, so BG1 is fully visible in both",
    ),
];

/// The declared scene equivalences hold.
#[test]
fn equivalent_scenes_render_identically() {
    let captured = capture_scenes();
    assert!(!captured.is_empty(), "no rendered scenes were captured");
    let names = manifest();
    let by_name: BTreeMap<&str, u64> = names
        .iter()
        .filter_map(|(idx, name)| Some((name.as_str(), *captured.get(idx)?)))
        .collect();

    for (a, b, why) in EQUIVALENCES {
        let (Some(&ha), Some(&hb)) = (by_name.get(a), by_name.get(b)) else {
            panic!("equivalence names a scene that did not render: {a} / {b}");
        };
        assert_eq!(
            ha, hb,
            "{a} and {b} must render identically, and did not ({ha:#018x} vs {hb:#018x}).\n  {why}"
        );
    }
    println!("\n  {} scene equivalence(s) hold.", EQUIVALENCES.len());
}

/// Every rendered scene matches its committed golden, and unblessed scenes are reported.
#[test]
fn rendered_scenes_match_goldens() {
    let captured = capture_scenes();
    assert!(
        !captured.is_empty(),
        "no rendered scenes were captured — the cart's scene loop did not run, or the frame \
         budget was exhausted before it did"
    );

    let goldens = goldens();
    let names = manifest();
    assert!(
        !names.is_empty(),
        "scenes.tsv is missing or empty — rebuild the cart with 'cargo run -p accuracysnes-gen' so \
         the goldens can be keyed by name instead of by a position that shifts"
    );
    let mut blessed_ok = 0usize;
    let mut unblessed = Vec::new();
    let mut mismatched = Vec::new();

    println!("\n  AccuracySNES rendered scenes:");
    for (&id, &hash) in &captured {
        let key = names.get(&id).cloned().unwrap_or_else(|| {
            panic!("the cart published scene {id}, which is not in scenes.tsv — stale build?")
        });
        match goldens.get(&key) {
            Some(&want) if want == hash => {
                blessed_ok += 1;
                println!("    {key}  0x{hash:016x}  match");
            }
            Some(&want) => {
                mismatched.push(format!("{key}: got 0x{hash:016x}, golden 0x{want:016x}"));
                println!("    {key}  0x{hash:016x}  MISMATCH (golden 0x{want:016x})");
            }
            None => {
                unblessed.push(format!("{key}\t0x{hash:016x}"));
                println!("    {key}  0x{hash:016x}  unblessed");
            }
        }
    }

    if !unblessed.is_empty() {
        println!(
            "\n  {} scene(s) have no committed golden. Per ADR 0013 a golden is blessed only from \
             a cross-validated render, so these are reported, not failed. To bless, verify the \
             scene renders identically on the reference emulators and add:\n",
            unblessed.len()
        );
        for line in &unblessed {
            println!("    {line}");
        }
    }

    assert!(
        mismatched.is_empty(),
        "rendered scene(s) diverged from their committed goldens:\n  {}\n\nA mismatch is a \
         behaviour change. If it is intended, re-bless deliberately and record why — see \
         docs/scheduler.md for what an undocumented re-bless costs.",
        mismatched.join("\n  ")
    );
    println!(
        "\n  {blessed_ok} blessed scene(s) match; {} unblessed.",
        unblessed.len()
    );
}
