#![allow(missing_docs)]
#![cfg(feature = "test-roms")]
//! Integration test for running Star Fox screens to produce screenshots.

use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use std::fs;

fn bgr555_to_rgb(px: u16) -> [u8; 3] {
    let r5 = (px & 0x1f) as u8;
    let g5 = ((px >> 5) & 0x1f) as u8;
    let b5 = ((px >> 10) & 0x1f) as u8;
    let r = (r5 << 3) | (r5 >> 2);
    let g = (g5 << 3) | (g5 >> 2);
    let b = (b5 << 3) | (b5 >> 2);
    [r, g, b]
}

#[test]
fn test_star_fox_screens() {
    let rom_path = "../../tests/roms/external/commercial/LoRom/GSU-1/Star Fox.sfc";
    let rom = fs::read(rom_path).unwrap();
    let cart = Cart::from_rom(&rom).unwrap();
    let mut system = System::new(0);
    system.bus.cart = Some(cart);

    let frames_to_capture = [840, 1080, 1200, 1320, 1560];
    let max_frame = *frames_to_capture.iter().max().unwrap();

    for f in 1..=max_frame {
        let pad: u16 = if f >= 180 && f % 90 < 6 { 0x1000 } else { 0 };
        system.bus.set_joypad(0, pad);
        system.run_frame();

        if frames_to_capture.contains(&f) {
            let mut ppm = format!("P3\n256 224\n255\n");
            for y in 0..224 {
                for x in 0..256 {
                    let pixel = system.bus.framebuffer()[y * 256 + x];
                    let rgb = bgr555_to_rgb(pixel);
                    ppm.push_str(&format!("{} {} {} ", rgb[0], rgb[1], rgb[2]));
                }
                ppm.push('\n');
            }
            let filename = format!(
                "/home/parobek/.gemini/antigravity-cli/brain/a26a6c4d-7308-48db-a8e6-25e3cd18e612/starfox_frame_{}_fix.ppm",
                f
            );
            fs::write(&filename, ppm).unwrap();
            println!("Saved {}", filename);
        }
    }
}
