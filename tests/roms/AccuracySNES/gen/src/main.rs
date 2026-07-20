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
mod tests;

use std::path::{Path, PathBuf};
use std::process::Command;

/// File offset of the header checksum words for a LoROM image.
///
/// LoROM maps bank `$00:$8000-$FFFF` to file offset `$0000-$7FFF`, so `$00:FFDC` is `$7FDC`.
const CHECKSUM_OFFSET: usize = 0x7FDC;

/// Expected image size: 128 KiB (four 32 KiB LoROM banks).
const ROM_SIZE: usize = 128 * 1024;

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
    dossier::validate(&battery);
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
    let coverage_path = dossier_path.with_file_name("accuracysnes-coverage.md");
    write(
        &coverage_path,
        &dossier::coverage_report(&battery, &enumerated),
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
}

/// Compute and write the SNES header checksum and its complement.
///
/// The image is a power of two, so this is a plain 16-bit sum of every byte with the checksum
/// field itself neutralised first (complement `$0000`, checksum `$FFFF`) — the convention every
/// SNES header follows.
fn patch_checksum(image: &mut [u8]) {
    image[CHECKSUM_OFFSET] = 0x00;
    image[CHECKSUM_OFFSET + 1] = 0x00;
    image[CHECKSUM_OFFSET + 2] = 0xFF;
    image[CHECKSUM_OFFSET + 3] = 0xFF;
    let sum = image
        .iter()
        .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
    let comp = !sum;
    image[CHECKSUM_OFFSET] = (comp & 0xFF) as u8;
    image[CHECKSUM_OFFSET + 1] = (comp >> 8) as u8;
    image[CHECKSUM_OFFSET + 2] = (sum & 0xFF) as u8;
    image[CHECKSUM_OFFSET + 3] = (sum >> 8) as u8;
}

/// The `tests/roms/AccuracySNES` directory, derived from this crate's manifest path.
fn cart_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen/ has a parent")
        .to_path_buf()
}

fn write(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn run(cmd: &mut Command, what: &str) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("{what}: failed to spawn ({e}) — is cc65 on PATH?"));
    assert!(status.success(), "{what}: exited with {status}");
}
