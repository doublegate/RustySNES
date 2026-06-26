//! Headless framebuffer capture for debugging — loads a staged game through the same `EmuCore` the
//! GUI uses, runs it to a gameplay/title frame, and writes the RGBA8 present buffer to a PPM in
//! the scratch dir so it can be eyeballed. Skips if the ROM is absent. Not a correctness gate.
#![cfg(test)]

use std::path::PathBuf;

use rustysnes_frontend::config::Region;
use rustysnes_frontend::emu::EmuCore;

fn rom(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/roms/external/commercial")
        .join(rel)
}

fn dump_ppm(core: &EmuCore, path: &str) {
    let (w, h) = core.fb_dims();
    let fb = core.framebuffer();
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in fb.chunks_exact(4) {
        out.extend_from_slice(&px[..3]); // R,G,B (drop A)
    }
    std::fs::write(path, out).expect("write ppm");
    eprintln!("wrote {path}  ({w}x{h})");
}

#[test]
fn capture_smw_frames() {
    let path = rom("LoRom/None/Super Mario World.sfc");
    if !path.is_file() {
        eprintln!("SKIP capture: {} absent", path.display());
        return;
    }
    let bytes = std::fs::read(&path).expect("read rom");
    let mut core = EmuCore::new(0, Region::Ntsc);
    core.load_rom(&bytes).expect("load smw");

    let out = "/tmp/claude-1000/-home-parobek-Code-OSS-Public-Projects-RustySNES/377cb5d1-974f-43c1-aadf-97277afd4cec/scratchpad";
    std::fs::create_dir_all(out).ok();
    for f in 1..=700u32 {
        core.run_frame();
        if matches!(f, 120 | 360 | 700) {
            dump_ppm(&core, &format!("{out}/smw_{f:04}.ppm"));
        }
    }
}
