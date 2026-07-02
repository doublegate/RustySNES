#![allow(missing_docs)]
//! Boot-screenshot generator for the whole locally-staged ROM corpus.
//!
//! Walks EVERY `.sfc`/`.smc` under `tests/roms/` (commercial dumps, krom, undisbeliever, blargg,
//! gilyon — all gitignored), boots each on the full `rustysnes_core::System` (installing the
//! matching coprocessor firmware when the cart needs it), and — only when `RUSTYSNES_DUMP_FRAMES=1`
//! — captures a spread of ~13 CANDIDATE frames across four input profiles (no input / periodic
//! Start / Start+A+B action mash / directional menu-navigation), split into a "title" pool (early
//! checkpoints) and a "gameplay" pool (later checkpoints, see `CANDIDATES`). Picking rules:
//!
//! * **Title** (`pick_title`) — the EARLIEST candidate that is both non-blank AND
//!   [`Candidate::is_stable`] (holding still over a short peek, not actively transitioning). Content
//!   alone can't tell a static title card apart from a scrolling intro backstory or an autoplaying
//!   attract-mode demo — both can be just as colorful — so stability is the signal that actually
//!   distinguishes "a screen waiting for input" from "a screen still playing itself out".
//! * **Gameplay** (`pick_latest`) — the LATEST candidate whose content clears a "not blank" bar,
//!   not the highest-content one: raw pixel count tends to prefer a colorful character/level-select
//!   grid over the (often visually simpler) actual play field, and menu/select screens usually
//!   *precede* real gameplay in the input timeline, so "latest that isn't blank" favors having
//!   gotten past them. Additionally excludes any candidate [`same_screen`] as the title pick (an
//!   attract-mode loop back to the title/menu), falling back to the raw pick only if every gameplay
//!   candidate matches.
//!
//! Written as `<rel>_title.ppm` / `<rel>_gameplay.ppm` (path mirrors the ROM's location under
//! `tests/roms/`) so `scripts/screenshots/generate.sh` can convert + montage them.
//!
//! This is a SCREENSHOT GENERATOR, not a correctness gate: with no env var set it just boots each
//! ROM (a smoke net) and self-skips entirely when the corpus is absent (CI stays green). Only the
//! produced PNGs (`screenshots/`) are ever committed — never a ROM or firmware byte (ADR 0003).
//!
//! Usage:
//! ```text
//! RUSTYSNES_DUMP_FRAMES=1 [RUSTYSNES_ROM_FILTER=substr] [RUSTYSNES_DUMP_DIR=/tmp/...] \
//!   [RUSTYSNES_DUMP_CANDIDATES=1] \
//!   cargo test -p rustysnes-test-harness --features test-roms --test commercial_screenshots -- --nocapture
//! ```
#![cfg(feature = "test-roms")]
// Screenshot generator: bounded framebuffer values narrowed to u8 (intentional), and the
// module doc lists bare env-var names / paths in a usage block.
#![allow(clippy::cast_possible_truncation, clippy::doc_markdown)]

use std::path::{Path, PathBuf};

use rustysnes_core::System;
use rustysnes_core::cart::{Cart, Coprocessor};

fn roms_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms")
}

fn firmware_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/firmware")
}

/// Firmware dumps to try, in order, for a cart's coprocessor (gitignored, user-supplied).
const fn firmware_candidates(co: Coprocessor) -> &'static [&'static str] {
    match co {
        Coprocessor::Dsp => &["dsp1b.rom", "dsp1.rom", "dsp2.rom", "dsp3.rom", "dsp4.rom"],
        Coprocessor::Cx4 => &["cx4.rom"],
        _ => &[],
    }
}

/// SNES 15-bit BGR555 -> 24-bit RGB (red in the low bits; the 5->8 expansion the frontend uses).
fn bgr555_to_rgb(px: u16) -> [u8; 3] {
    let r5 = u32::from(px & 0x1f);
    let g5 = u32::from((px >> 5) & 0x1f);
    let b5 = u32::from((px >> 10) & 0x1f);
    [
        ((r5 << 3) | (r5 >> 2)) as u8,
        ((g5 << 3) | (g5 >> 2)) as u8,
        ((b5 << 3) | (b5 >> 2)) as u8,
    ]
}

/// Recursively collect every `*.sfc`/`*.smc` under `root`, as paths relative to `root`, sorted.
fn all_roms(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("sfc") || x.eq_ignore_ascii_case("smc"))
            {
                out.push(p);
            }
        }
    }
    let mut abs = Vec::new();
    walk(root, &mut abs);
    let mut rel: Vec<PathBuf> = abs
        .iter()
        .filter_map(|p| p.strip_prefix(root).ok().map(Path::to_path_buf))
        .collect();
    rel.sort();
    rel
}

/// SNES pad bits (16-bit `BYsS UDLR AXLR` order): tap edges advance most title/menu flows.
mod pad {
    pub const B: u16 = 0x8000;
    pub const START: u16 = 0x1000;
    pub const UP: u16 = 0x0800;
    pub const DOWN: u16 = 0x0400;
    pub const RIGHT: u16 = 0x0100;
    pub const A: u16 = 0x0080;
}

/// Input profile used to drive a ROM toward a particular kind of screen. Each is a distinct
/// simulation (a game's state depends on its whole input history, so trying several "shapes" of
/// input — not just several checkpoints of one history — is what lets at least one candidate land
/// on a meaningful screen regardless of the game's menu/control flow).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Profile {
    /// No controller input at all: title cards, logos, and intro cinematics that autoplay.
    None,
    /// Periodic `Start` taps: the common "press start" flow.
    Start,
    /// Rotates `Start` / `A` / `Down+A` / `B`: clears menus (default-item confirm) and, once in a
    /// stage, provides continuous fire/action input for shmups and action games.
    Action,
    /// Rotates `Down`/`Right`/`Up`/`A`/`Start`: for games whose menu-to-gameplay path needs
    /// directional navigation (cursor-driven menus, overworld maps) rather than a single confirm.
    Explore,
}

impl Profile {
    const fn input(self, f: u32) -> u16 {
        match self {
            Self::None => 0,
            Self::Start => {
                if f % 30 < 4 {
                    pad::START
                } else {
                    0
                }
            }
            Self::Action => {
                if f % 16 >= 4 {
                    0
                } else {
                    match (f / 16) % 6 {
                        0 | 3 => pad::START,
                        1 | 2 => pad::A,
                        4 => pad::DOWN | pad::A,
                        _ => pad::B,
                    }
                }
            }
            Self::Explore => {
                if f % 20 >= 6 {
                    0
                } else {
                    match (f / 20) % 5 {
                        0 => pad::DOWN,
                        1 => pad::RIGHT,
                        2 => pad::A,
                        3 => pad::UP,
                        _ => pad::START,
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Title,
    Gameplay,
}

/// The ~13 (checkpoint frame, input profile, pool) candidates captured per ROM.
///
/// `Title`-bucket checkpoints mix `Profile::None` (a wide spread, since some games run an
/// unskippable intro/backstory before the title, or free-run an attract demo after it — no fixed
/// checkpoint reliably lands ON the held title card for every game) with `Profile::Start`'s two
/// earliest checkpoints (some games gate their first company-logo screens on *any* button, which
/// zero input never clears). Neither "earliest" nor "latest" content-clearing alone reliably picks
/// the title among these — content-count doesn't distinguish "static title card" from "a colorful
/// intro backdrop" or "an attract-mode demo screen" (all can clear a low content bar) — so `pick`
/// additionally scores each candidate's [`Candidate::stability`] (how much the screen changes over
/// a short peek) and takes the EARLIEST candidate that is both non-blank AND stable: an intro or
/// demo is actively changing frame-to-frame, while a title card sits still waiting for input.
///
/// `Gameplay`-bucket checkpoints use the mash profiles and run later, so they've had time to clear
/// menus for most genres; `pick` additionally excludes any gameplay candidate that matches the
/// title pick (an attract-mode loop back to the title/menu screen).
const CANDIDATES: &[(u32, Profile, Bucket)] = &[
    (180, Profile::None, Bucket::Title),
    (420, Profile::None, Bucket::Title),
    (420, Profile::Start, Bucket::Title),
    (700, Profile::None, Bucket::Title),
    (700, Profile::Start, Bucket::Title),
    (1000, Profile::None, Bucket::Title),
    (1400, Profile::None, Bucket::Title),
    (1000, Profile::Start, Bucket::Gameplay),
    (1400, Profile::Action, Bucket::Gameplay),
    (1800, Profile::Action, Bucket::Gameplay),
    (2200, Profile::Action, Bucket::Gameplay),
    (2600, Profile::Action, Bucket::Gameplay),
    (3000, Profile::Explore, Bucket::Gameplay),
];

/// Frames to advance past a checkpoint (still under the same profile) to measure how much the
/// screen changes — see [`Candidate::stability`].
const STABILITY_PEEK: u32 = 10;

/// Above this per-pixel-count delta over [`STABILITY_PEEK`] frames, a candidate is treated as
/// "actively transitioning" (an intro scrolling, an attract demo playing) rather than a screen
/// sitting still waiting for input.
const STABILITY_BAR: u32 = 600;

/// Count non-black pixels in a framebuffer — the "how much is on screen" proxy used to rank
/// candidates (dodges black transition frames and profiles that never left a black screen).
fn content(fb: &[u16]) -> u32 {
    fb.iter().filter(|&&p| p != 0).count() as u32
}

/// Minimum `content()` (out of 256*224 = 57344 pixels) for a candidate to count as "not blank" —
/// about 3% coverage, comfortably above a black/near-black transition frame but well below a
/// fully-rendered scene. Used to pick the LATEST qualifying candidate in a pool rather than the
/// highest-content one (see the module doc for why).
const MEANINGFUL_CONTENT: u32 = 1800;

/// A coarse 16x14 luminance thumbnail of a framebuffer, used only to tell whether two candidates
/// are "the same screen" (e.g. a gameplay-pool checkpoint that looped back to the title screen)
/// when their raw pixels differ slightly (an animated logo, a toggled menu option) but the overall
/// composition is identical. Deliberately coarse: exact pixel comparison misses this case (proven
/// by Star Fox 2 — its title screen's "MONO"/"STEREO" toggle text differs byte-for-byte between
/// two visits but the screens are otherwise the same).
const THUMB_GX: usize = 16;
const THUMB_GY: usize = 14;

fn thumb(fb: &[u16], w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(THUMB_GX * THUMB_GY);
    for grid_y in 0..THUMB_GY {
        let src_y = (grid_y * h) / THUMB_GY;
        for grid_x in 0..THUMB_GX {
            let src_x = (grid_x * w) / THUMB_GX;
            let px = fb.get(src_y * w + src_x).copied().unwrap_or(0);
            let red = u32::from(px & 0x1f);
            let grn = u32::from((px >> 5) & 0x1f);
            let blu = u32::from((px >> 10) & 0x1f);
            out.push(((red + grn + blu) / 3) as u8);
        }
    }
    out
}

/// Whether two framebuffers depict "the same screen" for de-duplication purposes: mean per-cell
/// luminance difference under a small bar (a handful of levels out of 31), which tolerates a
/// toggled menu option or an animating logo but not a genuinely different scene. Compared as
/// integers (`diff * cells < bar * cells`) to avoid a lossy int-to-float cast.
fn same_screen(a: &[u16], b: &[u16], w: usize, h: usize) -> bool {
    const BAR_PER_CELL: u32 = 2;
    let (ta, tb) = (thumb(a, w, h), thumb(b, w, h));
    let diff: u32 = ta
        .iter()
        .zip(&tb)
        .map(|(&x, &y)| u32::from(x.abs_diff(y)))
        .sum();
    diff < BAR_PER_CELL * ta.len() as u32
}

fn write_ppm(fb: &[u16], w: u32, h: u32, dump_dir: &str, rel: &Path, suffix: &str) -> bool {
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for &px in fb.iter().take((w * h) as usize) {
        ppm.extend_from_slice(&bgr555_to_rgb(px));
    }
    let stem = rel.with_extension("");
    let file_name = format!(
        "{}_{suffix}.ppm",
        stem.file_name().unwrap().to_string_lossy()
    );
    let out = Path::new(dump_dir)
        .join(rel.parent().unwrap_or(Path::new("")))
        .join(file_name);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&out, ppm).is_ok()
}

/// One captured candidate: the checkpoint frame it was taken at, which pool it belongs to, its
/// content score, how much the screen changed over the next [`STABILITY_PEEK`] frames (low =
/// sitting still waiting for input; high = an intro/demo actively transitioning), and the
/// framebuffer itself.
struct Candidate {
    frame: u32,
    profile: Profile,
    bucket: Bucket,
    content: u32,
    stability: u32,
    fb: Vec<u16>,
}

impl Candidate {
    /// Whether this candidate looks like a screen sitting still (see the field doc).
    const fn is_stable(&self) -> bool {
        self.stability <= STABILITY_BAR
    }
}

/// Boot `rel` fresh under `profile`, capturing its framebuffer (plus a [`STABILITY_PEEK`]-frame-
/// later sample, to score stability) at every candidate checkpoint that uses this profile
/// (`checkpoints`, sorted ascending).
fn run_profile(bytes: &[u8], profile: Profile, checkpoints: &[(u32, Bucket)]) -> Vec<Candidate> {
    let Ok(mut cart) = Cart::from_rom(bytes) else {
        return Vec::new();
    };
    let co = cart.header.coprocessor;
    // A board that knows exactly which chip dump it needs (e.g. a single-game NEC DSP variant)
    // MUST try that exact file first: several NEC DSP chips share an identical firmware byte size,
    // so falling through to the generic per-family candidate list could silently load the wrong
    // chip's firmware (see `Board::firmware_hint`'s doc for why this isn't just a fallback order).
    let hinted = cart
        .firmware_hint()
        .and_then(|name| std::fs::read(firmware_dir().join(name)).ok())
        .is_some_and(|f| cart.install_coprocessor_firmware(&f));
    if !hinted {
        for fw in firmware_candidates(co) {
            if let Ok(f) = std::fs::read(firmware_dir().join(fw))
                && cart.install_coprocessor_firmware(&f)
            {
                break;
            }
        }
    }
    let mut system = System::new(0);
    system.bus.cart = Some(cart);
    let w = 256usize;
    let h = usize::from(system.bus.ppu.visible_height()).min(239);
    let n = w * h;

    let mut out = Vec::with_capacity(checkpoints.len());
    let mut frame = 0u32;
    for &(target, bucket) in checkpoints {
        while frame < target {
            system.bus.set_joypad(0, profile.input(frame));
            system.run_frame();
            frame += 1;
        }
        let fb = system.bus.framebuffer()[..n].to_vec();
        let primary_content = content(&fb);
        for _ in 0..STABILITY_PEEK {
            system.bus.set_joypad(0, profile.input(frame));
            system.run_frame();
            frame += 1;
        }
        let peek_content = content(&system.bus.framebuffer()[..n]);
        out.push(Candidate {
            frame: target,
            profile,
            bucket,
            content: primary_content,
            stability: primary_content.abs_diff(peek_content),
            fb,
        });
    }
    out
}

/// From a pool, pick the LATEST candidate clearing [`MEANINGFUL_CONTENT`], falling back to the
/// highest-content candidate if none clear it (see the module doc for why "latest", not
/// "highest-content", is the right rule for the gameplay pool).
fn pick_latest<'a>(pool: &[&'a Candidate]) -> Option<&'a Candidate> {
    pool.iter()
        .copied()
        .filter(|c| c.content >= MEANINGFUL_CONTENT)
        .max_by_key(|c| c.frame)
        .or_else(|| pool.iter().copied().max_by_key(|c| c.content))
}

/// From the title pool, pick the EARLIEST candidate that both clears [`MEANINGFUL_CONTENT`] and
/// [`Candidate::is_stable`] (see the module doc for why neither signal alone suffices).
/// Progressively relaxes the rule if nothing qualifies: earliest non-blank ignoring stability,
/// then highest-content overall, so a title is always produced.
/// Tiebreak for two candidates at the SAME checkpoint frame: a mash profile (`Start`/`Action`/
/// `Explore`) has, by definition, had real button presses applied by that frame, while `None` has
/// had none — so at equal frames a mash profile's candidate is the more-progressed one (e.g. past
/// a "press any button" company splash that zero input never clears). Lower sorts first.
const fn profile_priority(p: Profile) -> u8 {
    match p {
        Profile::Start => 0,
        Profile::Action => 1,
        Profile::Explore => 2,
        Profile::None => 3,
    }
}

fn pick_title<'a>(pool: &[&'a Candidate]) -> Option<&'a Candidate> {
    pool.iter()
        .copied()
        .filter(|c| c.content >= MEANINGFUL_CONTENT && c.is_stable())
        .min_by_key(|c| (c.frame, profile_priority(c.profile)))
        .or_else(|| {
            pool.iter()
                .copied()
                .filter(|c| c.content >= MEANINGFUL_CONTENT)
                .min_by_key(|c| (c.frame, profile_priority(c.profile)))
        })
        .or_else(|| pool.iter().copied().max_by_key(|c| c.content))
}

/// Pick the title candidate (see `pick_title`) and the gameplay candidate (latest qualifying
/// mash-profile checkpoint, see `pick_latest`) from `candidates`. The gameplay pick excludes any
/// candidate [`same_screen`] as the title pick (an attract-mode loop back to the title/menu
/// screen), falling back to the raw pick only if every gameplay candidate matches.
fn pick(candidates: &[Candidate], w: usize, h: usize) -> (Option<&Candidate>, Option<&Candidate>) {
    let title_pool: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.bucket == Bucket::Title)
        .collect();
    let title = pick_title(&title_pool);

    let gameplay_pool: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.bucket == Bucket::Gameplay)
        .collect();
    let mut gameplay = pick_latest(&gameplay_pool);
    if let (Some(t), Some(g)) = (title, gameplay)
        && same_screen(&t.fb, &g.fb, w, h)
    {
        let rest: Vec<&Candidate> = gameplay_pool
            .iter()
            .copied()
            .filter(|c| !same_screen(&c.fb, &t.fb, w, h))
            .collect();
        gameplay = pick_latest(&rest).or(gameplay);
    }
    (title, gameplay)
}

#[test]
fn generate_commercial_screenshots() {
    let root = roms_root();
    let filter = std::env::var("RUSTYSNES_ROM_FILTER").unwrap_or_default();
    let roms: Vec<PathBuf> = all_roms(&root)
        .into_iter()
        .filter(|r| filter.is_empty() || r.to_string_lossy().contains(&filter))
        .collect();
    if roms.is_empty() {
        eprintln!(
            "SKIP commercial_screenshots: no ROMs under {} (gitignored corpus / filter '{filter}')",
            root.display()
        );
        return;
    }
    let dump = std::env::var("RUSTYSNES_DUMP_FRAMES").is_ok();
    let dump_candidates = std::env::var("RUSTYSNES_DUMP_CANDIDATES").is_ok();
    let dump_dir = std::env::var("RUSTYSNES_DUMP_DIR")
        .unwrap_or_else(|_| "/tmp/rustysnes-screenshots".to_string());

    // Group the candidate checkpoints by profile so each ROM runs exactly one simulation per
    // distinct profile (not once per candidate), each capturing all of that profile's checkpoints
    // along the way.
    let mut by_profile: Vec<(Profile, Vec<(u32, Bucket)>)> = Vec::new();
    for &(f, p, b) in CANDIDATES {
        match by_profile.iter_mut().find(|(pp, _)| *pp == p) {
            Some((_, v)) => v.push((f, b)),
            None => by_profile.push((p, vec![(f, b)])),
        }
    }
    for (_, v) in &mut by_profile {
        v.sort_unstable_by_key(|&(f, _)| f);
    }

    let (mut ok, mut shot, mut skipped) = (0u32, 0u32, 0u32);
    for rel in &roms {
        let abs = root.join(rel);
        let Ok(bytes) = std::fs::read(&abs) else {
            continue;
        };
        if Cart::from_rom(&bytes).is_err() {
            skipped += 1;
            continue;
        }

        let mut candidates: Vec<Candidate> = Vec::with_capacity(CANDIDATES.len());
        for (profile, checkpoints) in &by_profile {
            candidates.extend(run_profile(&bytes, *profile, checkpoints));
        }
        ok += 1;
        if !dump {
            continue;
        }

        if dump_candidates {
            for (i, c) in candidates.iter().enumerate() {
                let tag = if c.bucket == Bucket::Title { "t" } else { "g" };
                let stable = if c.is_stable() { "s" } else { "u" };
                let suffix = format!(
                    "candidate_{i:02}_{tag}{stable}_f{}_{}_d{}",
                    c.frame, c.content, c.stability
                );
                let (w, h) = (256u32, (c.fb.len() / 256) as u32);
                write_ppm(&c.fb, w, h, &dump_dir, rel, &suffix);
            }
        }

        let h_px = candidates.first().map_or(224, |c| c.fb.len() / 256);
        let (title, gameplay) = pick(&candidates, 256, h_px);
        let (w, h) = (256u32, h_px as u32);
        if let Some(c) = title
            && write_ppm(&c.fb, w, h, &dump_dir, rel, "title")
        {
            shot += 1;
        }
        if let Some(c) = gameplay
            && write_ppm(&c.fb, w, h, &dump_dir, rel, "gameplay")
        {
            shot += 1;
        }
    }
    eprintln!(
        "commercial_screenshots: {ok} booted, {shot} shots written to {dump_dir} ({skipped} unreadable headers){}",
        if dump {
            ""
        } else {
            "  [set RUSTYSNES_DUMP_FRAMES=1 to write PPMs]"
        }
    );
    assert!(ok > 0, "no ROM booted");
}
