use std::fs;
fn main() {
    let rom = fs::read("tests/roms/external/commercial/LoRom/GSU-1/Star Fox 2.sfc").unwrap_or_default();
    if rom.is_empty() { return; }
    let offset = 0x7FC0;
    let title_bytes = &rom[offset..offset+21];
    println!("Star Fox 2 title: {:?}", String::from_utf8_lossy(title_bytes));
    println!("Star Fox 2 sram_size byte: {:?}", rom[offset+0x18]);
}
