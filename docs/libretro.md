# Libretro core — RustySNES

**References:** `docs/architecture.md` §3/§6 (the `rustysnes-core::facade` embedding surface);
`crates/rustysnes-libretro/src/lib.rs`'s own module doc.

## Purpose

`rustysnes-libretro` (`v1.2.0`) is a thin C-ABI wrapper exposing `rustysnes-core`'s
[`facade::EmuCore`](../crates/rustysnes-core/src/facade.rs) — the same relocated pure emulation
facade `rustysnes-frontend`'s own `EmuCore` wraps — as a [libretro](https://www.libretro.com/)
core, loadable by RetroArch or any other libretro-compatible frontend. It duplicates zero
emulation logic; every hook (`on_run`, `on_serialize`, `get_memory_data`, …) is a thin translation
between the libretro C ABI and `EmuCore`'s existing safe Rust API.

## Building

```bash
cargo build -p rustysnes-libretro --release
```

Produces `librustysnes_libretro.so` (Linux) / `.dylib` (macOS) / `.dll` (Windows) under
`target/release/`, plus a static `.a`/`.lib` (the crate's `crate-type` is
`["cdylib", "staticlib"]`). Needs a working `clang`/`libclang` at build time
(`rust-libretro-sys`'s `bindgen` build script) — present by default on GitHub-hosted CI runners
and most desktop Linux/macOS toolchains; on Windows, install LLVM and ensure `libclang.dll` is on
`PATH` if the build script can't find it.

## Manual RetroArch verification

No automated RetroArch integration test exists yet (this would need a real RetroArch binary +
headless display in CI, out of this ticket's scope) — verify manually after any change to this
crate:

1. Build the core: `cargo build -p rustysnes-libretro --release`.
2. Copy `target/release/librustysnes_libretro.so` (or platform equivalent) into RetroArch's
   `cores/` directory, or point RetroArch's "Load Core" dialog at it directly.
3. Load a ROM (`.sfc`/`.smc`/`.swc`/`.fig`) via RetroArch's content browser — confirm:
   - Picture renders at the correct resolution (256×224 NTSC / 256×239 PAL — RetroArch's on-screen
     display should reflect the corrected geometry within the first second, once
     `RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO` fires from the first `on_run`) and colors are correct
     (a botched R/B channel swap is the most likely regression if this is ever touched — check a
     scene with distinct red and blue elements).
   - Audio plays without popping/pitch artifacts (confirms the `EmuCore::audio()` → interleaved
     `i16` batch path).
   - P1/P2 standard controller input responds (D-pad + all 8 face/shoulder buttons).
   - Save states work: RetroArch's quick-save/quick-load (`F2`/`F4` by default) round-trips
     correctly (confirms `get_serialize_size`/`on_serialize`/`on_unserialize`).
   - SRAM persists across a RetroArch restart for a battery-backed cart (confirms
     `get_memory_data`/`get_memory_size` for `RETRO_MEMORY_SAVE_RAM` — RetroArch reads/writes this
     pointer directly, there is no separate `.srm` file management in this core unlike the native
     frontend).
   - A coprocessor cart (e.g. a DSP-1 title) either works correctly (if the firmware dump is
     present in RetroArch's configured system directory, named per
     `EmuCore::firmware_candidates()`) or logs a clear warning to RetroArch's core log instead of
     silently misbehaving.
4. Try a Game Genie or Pro Action Replay code via RetroArch's Cheats menu — confirm it applies
   (`on_cheat_set`) and that disabling it / resetting cheats (`on_cheat_reset`) reverts cleanly.
5. Peripheral negotiation (post-`v1.3.0`): in RetroArch's Quick Menu → Controls, change Port 2's
   device to each of "SNES Mouse", "Super Multitap", and "Super Scope" in turn (Port 1 only offers
   "SNES Joypad"/"SNES Mouse" — Mouse and Multitap/Super Scope selection mirrors bsnes's own
   libretro core's per-port menu, `ref-proj/bsnes/bsnes/target-libretro/libretro.cpp`) — confirm:
   - **SNES Mouse**: a title that supports it (e.g. Mario Paint) tracks pointer motion and
     left/right clicks.
   - **Super Multitap**: a 4-player title (e.g. Super Bomberman) responds to RetroArch Players 2-5
     independently (sub-pad `N` polls libretro port `1 + N`, matching bsnes's own convention).
   - **Super Scope**: a compatible title (e.g. Super Scope 6) tracks the pointer/lightgun device's
     aim and trigger; moving off-screen should read as "not aiming" rather than aiming at a screen
     edge (confirms the `RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN` handling).

## Known scope cuts (documented, not silent gaps)

- **No automated RetroArch smoke test in CI** — see "Manual RetroArch verification" above. CI
  instead: (a) the `lint` job's `cargo clippy -p rustysnes-libretro` (implicitly, via
  `--workspace`) plus an explicit `cargo build -p rustysnes-libretro` (proves the cdylib/staticlib
  actually links — the main FFI-crate-specific risk); (b) `full-test`'s workspace-wide
  test/clippy/doc gate at tag-push time.
- **Region/timing correction is one-shot, not per-`on_run`**: `RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO`
  is only callable from a `RunContext` (this binding's own restriction, not a libretro-spec one),
  so the core defers the NTSC-default → real-region correction to the first `on_run` after
  `on_load_game`, using a `pending_av_info` flag. A region-hotswap cart (there is no such real SNES
  cart) or a future re-region feature would need a different mechanism — out of scope today.

## SNES-specific deltas from a hypothetical NES-libretro template

(`rustynes-libretro`, the sibling NES core, was the direct porting reference — see that crate's
own `src/lib.rs` for the shared skeleton this one adapted.)

- **Region-aware geometry + timing**: NTSC 256×224 @ 60.0988 Hz vs. PAL 256×239 @ 50.007 Hz (vs.
  a single fixed NES geometry) — corrected post-load, see "Known scope cuts" above.
- **`sample_rate: 32000.0`**, not the NES core's 44100 — the S-DSP's real, fixed output rate
  (`docs/apu.md`), matching `rustysnes-frontend::audio_core::SDSP_RATE`'s own established
  constant.
- **`EmuCore::audio()` already emits signed 16-bit stereo pairs** — no `f32`→`i16` scaling dance
  needed (unlike an APU that emits floats), just interleave-and-batch.
- **Coprocessor firmware auto-resolution** (`RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY` →
  `EmuCore::firmware_candidates`/`install_firmware`) — no NES equivalent; the µPD77C25 DSP-1..4
  family and CX4 need an external firmware dump this core tries to locate automatically.
- **Cheat support** (`on_cheat_set`/`on_cheat_reset` → `rustysnes_core::cheat::decode` →
  `Bus::set_cheats`) — wired here; had no template to follow since the NES sibling core doesn't
  implement libretro cheat support.
- **New `Bus::wram_mut`/`Ppu::vram_mut`/`Cart::sram_mut` accessors** (`v1.2.0`) — added
  specifically for this crate's `get_memory_data`; previously only word-at-a-time debug accessors
  (`peek_wram`/`vram_word`) or a copy-based `load_sram` existed.

## `retro_game_info` is opaque in `rust-libretro-sys` 0.3.2 — use `GET_GAME_INFO_EXT` instead

`on_load_game`'s `game: Option<retro_game_info>` parameter is **unusable** with this pinned
version: bindgen generates `retro_game_info` as a 1-byte opaque placeholder (`pub _address: u8`)
rather than the real 4-field C struct (verified against the crate's own generated
`bindings_libretro.rs` and its self-inconsistent `bindgen_test_layout_retro_game_info` test, which
asserts a 32-byte size against a 1-byte type — the test would fail if it ever actually ran). The
proven workaround (same one `rustynes-libretro` already uses successfully): fetch the ROM via
`RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` through the raw environment callback, using a hand-rolled
`#[repr(C)]` `RetroGameInfoExt` struct that mirrors libretro.h's real layout directly, bypassing
the broken opaque binding entirely. If a future `rust-libretro`/`rust-libretro-sys` upgrade fixes
this upstream, `on_load_game` could be simplified back to the straightforward
`game.unwrap().data`/`.size` form — check the generated bindings first.
