<!-- markdownlint-disable MD033 MD041 -->
<div align="center">

# RustySNES

**A cycle-accurate Super Nintendo / Super Famicom emulator in Rust.**

</div>

RustySNES is a cycle-accurate Super Nintendo Entertainment System / Super Famicom emulator in Rust, architected at the Mesen2 / ares / higan
accuracy bar (a master-clock lockstep scheduler, a Bus that owns everything mutable, a
one-directional `no_std + alloc` chip-crate graph, a hard determinism contract, test-ROM-is-spec).

## Crates
- `rustysnes-cpu` — WDC 65C816
- `rustysnes-ppu` — PPU1 (5C77) + PPU2 (5C78)
- `rustysnes-apu` — SPC700 + S-DSP
- `rustysnes-cart` — LoROM/HiROM/ExHiROM + coprocessors
- `rustysnes-core` — the Bus + scheduler tie crate
- `rustysnes-frontend` — the `winit + wgpu + cpal + egui` shell (binary `rustysnes`)
- `rustysnes-test-harness` — the AccuracyCoin-equivalent oracle

## Build / test
```bash
cargo check --workspace
cargo test --workspace
cargo test --workspace --features test-roms
cargo run --release -p rustysnes-frontend -- path/to/rom
```

## License
RustySNES is dual-licensed under **MIT OR Apache-2.0**. See `LICENSE-MIT` and `LICENSE-APACHE`.
