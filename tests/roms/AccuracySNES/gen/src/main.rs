//! `accuracysnes-gen` — builds the AccuracySNES test cartridge.
//!
//! Every test is authored once in [`tests`]; this binary turns those definitions into
//! 65816 assembly, assembles and links them with `ca65`/`ld65`, patches the SNES header
//! checksum (which `ld65` cannot compute), and writes the catalog the host harness scores
//! against. Because the assembly and the catalog come from the same source, they cannot drift.
//!
//! ```text
//! cargo run -p accuracysnes-gen
//! ```
//!
//! Requires `ca65` and `ld65` (cc65 2.19+) on `PATH`.

mod dossier;
mod dsl;
mod emit;
mod font;
mod scenes;
mod spc;
mod tests;

use std::path::{Path, PathBuf};
use std::process::Command;

/// File offset of the header checksum words for a LoROM image.
///
/// LoROM maps bank `$00:$8000-$FFFF` to file offset `$0000-$7FFF`, so `$00:FFDC` is `$7FDC`.
const CHECKSUM_OFFSET: usize = 0x7FDC;

/// File offset of the header's country/region byte, `$00:FFD9`.
const COUNTRY_OFFSET: usize = 0x7FD9;

/// Country codes. `Header::region_from_code` maps `$02..=$0C` to PAL, so `$02` (Europe) is the
/// canonical PAL value and `$01` (USA) the canonical NTSC one.
const COUNTRY_NTSC: u8 = 0x01;
const COUNTRY_PAL: u8 = 0x02;

/// Expected image size: 128 KiB (four 32 KiB LoROM banks).
const ROM_SIZE: usize = 256 * 1024;

/// File offset of the HiROM header checksum words. HiROM maps `$00:FFDC` to file offset `$FFDC`.
const HIROM_CHECKSUM_OFFSET: usize = 0xFFDC;

/// The parallel HiROM image size: 64 KiB (the smallest HiROM layout — a `$C0` linear low half plus
/// the `$00:8000` runtime window).
const HIROM_ROM_SIZE: usize = 64 * 1024;

/// File offset of the ExHiROM header checksum words. The ExHiROM header sits at file `$40FFC0`.
const EXHIROM_CHECKSUM_OFFSET: usize = 0x40_FFDC;

/// The ExHiROM image size: `$410000` (4 MiB + 64 KiB). It must exceed 4 MiB so the two halves
/// (ROM `$0xxxxx` / `$4xxxxx`) are distinct physical bytes rather than mirror-collapsed — the whole
/// property `G1.16` checks. The gap is `$FF` fill, which git compresses.
const EXHIROM_ROM_SIZE: usize = 0x0041_0000;

fn main() {
    let root = cart_root();
    let asm_dir = root.join("asm");
    let build_dir = root.join("build");
    std::fs::create_dir_all(&build_dir).expect("create build dir");

    let battery = tests::all();
    println!("accuracysnes-gen: {} tests", battery.len());

    // --- generated sources ---
    write(&asm_dir.join("tests_group_a.s"), &emit::asm(&battery));
    write(&asm_dir.join("font.s"), &font::asm());
    write(&asm_dir.join("scenes.s"), &scenes::asm());

    // --- generated data the host side consumes ---
    // Coverage and MAP validation span BOTH images' batteries: a dossier row covered only in the
    // HiROM image (e.g. G1.15) is still covered. Each image has its own WRAM measurement channel,
    // so the slot-collision check stays per-image.
    let hirom_battery = tests::hirom();
    let exhirom_battery = tests::exhirom();
    let mut coverage_battery = battery.clone();
    coverage_battery.extend(hirom_battery.iter().cloned());
    coverage_battery.extend(exhirom_battery.iter().cloned());
    dossier::validate(&coverage_battery);
    dossier::check_slots(&battery);
    dossier::check_slots(&hirom_battery);
    dossier::check_slots(&exhirom_battery);
    write(&root.join("SOURCE_CATALOG.tsv"), &emit::catalog(&battery));
    // Next to the ROM, not in the source tree: it describes THIS build's scene numbering, and a
    // host that reads a manifest from a different build would key its goldens off the wrong names.
    write(&root.join("build/scenes.tsv"), &scenes::manifest());

    // The coverage report is regenerated with the ROM so it cannot drift from the battery.
    let dossier_path = root
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.join("docs/accuracysnes-research-dossier.md"))
        .expect("locate the dossier from the cart directory");
    let dossier_src = std::fs::read_to_string(&dossier_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", dossier_path.display()));
    let enumerated = dossier::parse_enumeration(&dossier_src);
    dossier::validate_scenes(&enumerated);
    let coverage_path = dossier_path.with_file_name("accuracysnes-coverage.md");
    write(
        &coverage_path,
        &dossier::coverage_report(&coverage_battery, &enumerated),
    );
    write(&root.join("ERROR_CODES.md"), &emit::readme_codes(&battery));

    // --- assemble ---
    let units = ["runtime", "header", "tests_group_a", "font", "scenes"];
    let mut objects = Vec::new();
    for unit in units {
        let src = asm_dir.join(format!("{unit}.s"));
        let obj = build_dir.join(format!("{unit}.o"));
        run(
            Command::new("ca65")
                .arg("--cpu")
                .arg("65816")
                .arg("-I")
                .arg(&asm_dir)
                .arg("-o")
                .arg(&obj)
                .arg(&src),
            &format!("ca65 {unit}"),
        );
        objects.push(obj);
    }

    // --- link ---
    let sfc = build_dir.join("accuracysnes.sfc");
    let mut link = Command::new("ld65");
    link.arg("-C")
        .arg(root.join("lorom.cfg"))
        .arg("-o")
        .arg(&sfc)
        .arg("-m")
        .arg(build_dir.join("accuracysnes.map"));
    for obj in &objects {
        link.arg(obj);
    }
    run(&mut link, "ld65");
    report_bank_headroom(&build_dir.join("accuracysnes.map"));

    // --- patch the header checksum ---
    let mut image = std::fs::read(&sfc).expect("read linked image");
    assert_eq!(
        image.len(),
        ROM_SIZE,
        "linked image is {} bytes, expected {ROM_SIZE}",
        image.len()
    );
    patch_checksum(&mut image);
    std::fs::write(&sfc, &image).expect("write patched image");

    let sum = u16::from_le_bytes([image[CHECKSUM_OFFSET + 2], image[CHECKSUM_OFFSET + 3]]);
    let comp = u16::from_le_bytes([image[CHECKSUM_OFFSET], image[CHECKSUM_OFFSET + 1]]);
    println!(
        "accuracysnes-gen: wrote {} ({} bytes, checksum ${sum:04X}, complement ${comp:04X})",
        sfc.display(),
        image.len()
    );
    assert_eq!(sum ^ comp, 0xFFFF, "checksum/complement invariant broken");

    write_pal_image(&image, &build_dir.join("accuracysnes-pal.sfc"));

    build_hirom_image(&root, &asm_dir, &build_dir, &hirom_battery);
    build_exhirom_image(&root, &asm_dir, &build_dir, &exhirom_battery);
}

/// Build the parallel HiROM image: the shared `runtime.s` (assembled with `-D HIROM_BUILD` so its
/// LoROM-only per-bank signature blocks are dropped) plus the small HiROM battery, linked with
/// `hirom.cfg` and `header-hirom.s`. It self-scores the emulator's HiROM decode and SRAM window.
///
/// Each unit gets its own object file (`*-hirom.o`) so the LoROM objects are untouched, and the
/// checksum is patched at the HiROM header offset (`$FFDC`) rather than LoROM's `$7FDC`.
fn build_hirom_image(root: &Path, asm_dir: &Path, build_dir: &Path, battery: &[dsl::Test]) {
    println!("accuracysnes-gen: HiROM image — {} test(s)", battery.len());
    write(&asm_dir.join("tests_hirom.s"), &emit::asm(battery));

    let units = [
        ("runtime", "runtime-hirom"),
        ("header-hirom", "header-hirom"),
        ("tests_hirom", "tests_hirom"),
        ("font", "font-hirom"),
    ];
    let mut objects = Vec::new();
    for (src_stem, obj_stem) in units {
        let src = asm_dir.join(format!("{src_stem}.s"));
        let obj = build_dir.join(format!("{obj_stem}.o"));
        run(
            Command::new("ca65")
                .arg("--cpu")
                .arg("65816")
                .arg("-D")
                .arg("HIROM_BUILD")
                .arg("-I")
                .arg(asm_dir)
                .arg("-o")
                .arg(&obj)
                .arg(&src),
            &format!("ca65 {src_stem} (HiROM)"),
        );
        objects.push(obj);
    }

    let sfc = build_dir.join("accuracysnes-hirom.sfc");
    let mut link = Command::new("ld65");
    link.arg("-C")
        .arg(root.join("hirom.cfg"))
        .arg("-o")
        .arg(&sfc)
        .arg("-m")
        .arg(build_dir.join("accuracysnes-hirom.map"));
    for obj in &objects {
        link.arg(obj);
    }
    run(&mut link, "ld65 (HiROM)");

    let mut image = std::fs::read(&sfc).expect("read linked HiROM image");
    assert_eq!(
        image.len(),
        HIROM_ROM_SIZE,
        "linked HiROM image is {} bytes, expected {HIROM_ROM_SIZE}",
        image.len()
    );
    patch_checksum_at(&mut image, HIROM_CHECKSUM_OFFSET);
    std::fs::write(&sfc, &image).expect("write patched HiROM image");
    let sum = u16::from_le_bytes([
        image[HIROM_CHECKSUM_OFFSET + 2],
        image[HIROM_CHECKSUM_OFFSET + 3],
    ]);
    let comp = u16::from_le_bytes([
        image[HIROM_CHECKSUM_OFFSET],
        image[HIROM_CHECKSUM_OFFSET + 1],
    ]);
    assert_eq!(
        sum ^ comp,
        0xFFFF,
        "HiROM checksum/complement invariant broken"
    );
    println!(
        "accuracysnes-gen: wrote {} ({} bytes, HiROM, checksum ${sum:04X})",
        sfc.display(),
        image.len()
    );
}

/// Build the parallel ExHiROM image: the shared `runtime.s` (again under `-D HIROM_BUILD`, which
/// drops the LoROM-only per-bank signature blocks and the scene loop for any second image) plus the
/// ExHiROM battery (`G1.16`), linked with `exhirom.cfg` and `header-exhirom.s`.
///
/// The image is a genuine two-half >4 MiB layout: the runtime and tests live in the *extra* half
/// (bank `$00` has A23=0, so `phk/plb` keeps `DBR` in `$00-$3F` where `$21xx/$42xx` decode as MMIO),
/// while `EXSIG_LO` (`$A1`) sits at ROM `$000000` in the first half and `EXSIG_HI` (`$E2`) at ROM
/// `$400000` in the extra half. `G1.16` reads both through the ExHiROM banks to self-score the
/// A23->A22 inversion. The checksum is patched at the ExHiROM header offset (`$40FFDC`).
fn build_exhirom_image(root: &Path, asm_dir: &Path, build_dir: &Path, battery: &[dsl::Test]) {
    println!("accuracysnes-gen: ExHiROM image — {} test(s)", battery.len());
    write(&asm_dir.join("tests_exhirom.s"), &emit::asm(battery));

    let units = [
        ("runtime", "runtime-exhirom"),
        ("header-exhirom", "header-exhirom"),
        ("tests_exhirom", "tests_exhirom"),
        ("font", "font-exhirom"),
    ];
    let mut objects = Vec::new();
    for (src_stem, obj_stem) in units {
        let src = asm_dir.join(format!("{src_stem}.s"));
        let obj = build_dir.join(format!("{obj_stem}.o"));
        run(
            Command::new("ca65")
                .arg("--cpu")
                .arg("65816")
                .arg("-D")
                .arg("HIROM_BUILD")
                .arg("-I")
                .arg(asm_dir)
                .arg("-o")
                .arg(&obj)
                .arg(&src),
            &format!("ca65 {src_stem} (ExHiROM)"),
        );
        objects.push(obj);
    }

    let sfc = build_dir.join("accuracysnes-exhirom.sfc");
    let mut link = Command::new("ld65");
    link.arg("-C")
        .arg(root.join("exhirom.cfg"))
        .arg("-o")
        .arg(&sfc)
        .arg("-m")
        .arg(build_dir.join("accuracysnes-exhirom.map"));
    for obj in &objects {
        link.arg(obj);
    }
    run(&mut link, "ld65 (ExHiROM)");

    let mut image = std::fs::read(&sfc).expect("read linked ExHiROM image");
    assert_eq!(
        image.len(),
        EXHIROM_ROM_SIZE,
        "linked ExHiROM image is {} bytes, expected {EXHIROM_ROM_SIZE}",
        image.len()
    );
    patch_checksum_at(&mut image, EXHIROM_CHECKSUM_OFFSET);
    std::fs::write(&sfc, &image).expect("write patched ExHiROM image");
    let sum = u16::from_le_bytes([
        image[EXHIROM_CHECKSUM_OFFSET + 2],
        image[EXHIROM_CHECKSUM_OFFSET + 3],
    ]);
    let comp = u16::from_le_bytes([
        image[EXHIROM_CHECKSUM_OFFSET],
        image[EXHIROM_CHECKSUM_OFFSET + 1],
    ]);
    assert_eq!(
        sum ^ comp,
        0xFFFF,
        "ExHiROM checksum/complement invariant broken"
    );
    println!(
        "accuracysnes-gen: wrote {} ({} bytes, ExHiROM, checksum ${sum:04X})",
        sfc.display(),
        image.len()
    );
}

/// Write the PAL sibling image: the same battery, one header byte apart.
///
/// "This assertion needs a PAL console" is only half true. A console's region fixes the *timing*,
/// but which timing an emulator boots is decided by the cart header's country code — so a PAL
/// image exercises the PAL line count and frame rate on every emulator, unmodified, with no
/// harness-side region switch that a reference emulator would have no equivalent of.
///
/// It is deliberately produced by patching one byte of the linked NTSC image rather than by a
/// second assembly pass. That makes the two images provably identical apart from the region: any
/// behavioural difference between them is the region and cannot be anything else. On real hardware
/// the console still wins — a PAL-header cart in an NTSC console runs at NTSC timing — which is why
/// the cart reads `$213F` and reports the region it *actually* ran in rather than the one its
/// header asked for.
fn write_pal_image(ntsc: &[u8], path: &Path) {
    let mut image = ntsc.to_vec();
    assert_eq!(
        image[COUNTRY_OFFSET], COUNTRY_NTSC,
        "the NTSC image's country byte is not where the header says it is"
    );
    image[COUNTRY_OFFSET] = COUNTRY_PAL;
    patch_checksum(&mut image);
    std::fs::write(path, &image).expect("write PAL image");

    let differing = ntsc
        .iter()
        .zip(&image)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    // Only the country byte and the checksum field may move. (Not *exactly* five bytes: a
    // checksum byte can coincidentally keep its value.)
    let allowed = [
        COUNTRY_OFFSET,
        CHECKSUM_OFFSET,
        CHECKSUM_OFFSET + 1,
        CHECKSUM_OFFSET + 2,
        CHECKSUM_OFFSET + 3,
    ];
    assert!(
        differing.contains(&COUNTRY_OFFSET) && differing.iter().all(|i| allowed.contains(i)),
        "PAL image differs from NTSC outside the country/checksum bytes: {differing:x?}"
    );
    println!(
        "accuracysnes-gen: wrote {} (region byte + checksum only)",
        path.display()
    );
}

/// Compute and write the SNES header checksum and its complement at `offset` (the file offset of
/// `$FFDC`: `$7FDC` for LoROM, `$FFDC` for HiROM).
///
/// The image is a power of two, so this is a plain 16-bit sum of every byte with the checksum
/// field itself neutralised first (complement `$0000`, checksum `$FFFF`) — the convention every
/// SNES header follows.
fn patch_checksum_at(image: &mut [u8], offset: usize) {
    image[offset] = 0x00;
    image[offset + 1] = 0x00;
    image[offset + 2] = 0xFF;
    image[offset + 3] = 0xFF;
    let sum = image
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    let comp = !sum;
    image[offset] = (comp & 0xFF) as u8;
    image[offset + 1] = (comp >> 8) as u8;
    image[offset + 2] = (sum & 0xFF) as u8;
    image[offset + 3] = (sum >> 8) as u8;
}

/// LoROM convenience wrapper for [`patch_checksum_at`].
fn patch_checksum(image: &mut [u8]) {
    patch_checksum_at(image, CHECKSUM_OFFSET);
}

/// The `tests/roms/AccuracySNES` directory, derived from this crate's manifest path.
pub(crate) fn cart_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen/ has a parent")
        .to_path_buf()
}

/// Report how much room is left in each bank, and fail the build before one of them runs out.
///
/// A segment overflow is an `ld65` error with no warning beforehand, and it lands in the middle of
/// writing a test rather than at a moment anyone chose. It has now happened four times: three times
/// in bank `$00` and once when `B1.05` pushed `CATALOG` 73 bytes past the end. Each cost a
/// debugging cycle to recognise, because the error names a *segment* and the fix is always to move
/// a different one.
///
/// So the layout is measured every build. The map file gives each segment's start and end; the bank
/// is the top byte of the address, and what is left is the distance from the last segment in that
/// bank to the end of it. Bank `$00` stops at `$FFB0`, where the header begins.
///
/// **The threshold is deliberately generous.** Failing at 512 bytes free means the build breaks
/// while there is still room to land the change in hand and move something afterwards, rather than
/// at the moment there is none. That is the whole point: the same reasoning as
/// `dossier::check_slots`, which prints the free list on a collision instead of merely refusing.
fn report_bank_headroom(map: &Path) {
    /// Bank `$00` ends where the header starts, not at `$FFFF`.
    const BANK0_END: u32 = 0x00_FFB0;
    /// Free bytes below which the build fails rather than warns.
    const MIN_FREE: u32 = 512;

    let Ok(text) = std::fs::read_to_string(map) else {
        return; // no map, nothing to check — the link would have failed first
    };
    let mut last_end: std::collections::BTreeMap<u32, (u32, String)> =
        std::collections::BTreeMap::new();
    let mut in_list = false;
    for line in text.lines() {
        if line.starts_with("Segment list:") {
            in_list = true;
            continue;
        }
        if in_list && line.trim().is_empty() {
            break;
        }
        let mut f = line.split_whitespace();
        let (Some(name), Some(start), Some(end)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let (Ok(start), Ok(end)) = (u32::from_str_radix(start, 16), u32::from_str_radix(end, 16))
        else {
            continue;
        };
        if start < 0x8000 {
            continue; // RAM areas: nothing of theirs reaches the file
        }
        let bank = start >> 16;
        if bank == 0 && start >= BANK0_END {
            // HEADER and VECTORS are pinned at the top of bank $00 by the hardware, not stacked
            // after the growing segments. Counting them as "last" makes the bank look full when
            // what matters is the gap below them, which is where the catalog grows into.
            continue;
        }
        let slot = last_end.entry(bank).or_insert((0, String::new()));
        if end >= slot.0 {
            *slot = (end, name.to_owned());
        }
    }

    let mut tight = Vec::new();
    println!("accuracysnes-gen: bank headroom");
    for (bank, (end, name)) in &last_end {
        // One past the last usable byte. `|` rather than `+` here was wrong in a way that only
        // showed on odd banks, since `bank << 16` already has the bit that `0x1_0000` would set.
        let bank_end = if *bank == 0 {
            BANK0_END
        } else {
            (bank << 16) + 0x0001_0000
        };
        let free = bank_end.saturating_sub(end + 1);
        println!("  bank ${bank:02X}  {free:>6} bytes free  (last: {name})");
        if free < MIN_FREE {
            tight.push(format!(
                "bank ${bank:02X} has {free} bytes free after {name}"
            ));
        }
    }
    assert!(
        tight.is_empty(),
        "a bank is nearly full and the next test to land will overflow it:\n  {}\n\
         Move a segment to an empty bank in lorom.cfg, or relocate a group's bodies via \
         emit.rs's OUT_OF_BANK — bank $00 can only be relieved by the latter.",
        tight.join("\n  ")
    );
}

fn write(path: &Path, contents: &str) {
    // Normalise the trailing newline. The emitters end each section with a blank line, which
    // leaves the file ending in two — a markdownlint MD012 failure on a file nobody edits by hand,
    // so it can only be fixed here.
    let contents = format!("{}\n", contents.trim_end_matches('\n'));
    let contents = contents.as_str();
    std::fs::write(path, contents).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn run(cmd: &mut Command, what: &str) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("{what}: failed to spawn ({e}) — is cc65 on PATH?"));
    assert!(status.success(), "{what}: exited with {status}");
}
