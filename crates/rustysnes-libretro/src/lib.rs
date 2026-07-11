//! RustySNES Libretro Core.
//!
//! Implements the C ABI boundary for the `rustysnes-core` engine, exposing the standard libretro
//! lifecycle hooks (`retro_init`, `retro_load_game`, `retro_run`, etc.) required by RetroArch and
//! other compatible frontends. A thin, safe facade over
//! [`rustysnes_core::facade::EmuCore`] — the same relocated facade (`v1.2.0`) the native/wasm
//! frontend's own `EmuCore` wraps, so this core never duplicates emulation logic.
//!
//! # Architecture
//!
//! - **Video**: `EmuCore::framebuffer()` (RGBA8) is byte-swapped (R<->B) into XRGB8888 and handed
//!   to `RunContext::draw_frame`. Geometry/timing are region-dependent (NTSC 256x224 @ 60.0988 Hz
//!   vs. PAL 256x239 @ 50.007 Hz, `docs/scheduler.md`); the true region is only known once the
//!   cart header has been parsed (`System::reset`, triggered by the first `run_frame`), so the
//!   core reports a conservative NTSC default in `on_get_av_info` and corrects it via
//!   `RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO` on the first `on_run` after a ROM loads.
//! - **Audio**: `EmuCore::audio()` already produces 32 kHz signed 16-bit stereo pairs — no format
//!   conversion needed, just interleaving into libretro's batch API.
//! - **Input**: the standard 12-button pad is wired for P1/P2 (`RETRO_DEVICE_JOYPAD`), plus
//!   Mouse / Super Scope / Super Multitap peripheral negotiation via `RETRO_DEVICE_SUBCLASS`
//!   (`RETRO_ENVIRONMENT_SET_CONTROLLER_INFO` + `Core::on_set_controller_port_device`), mirroring
//!   bsnes's own libretro core's device menu (`ref-proj/bsnes/bsnes/target-libretro/libretro.cpp`)
//!   — Mouse and Super Multitap are only offered on port 2 (index `1`), matching real SNES
//!   hardware (a Super Scope's beam-latch and the multitap's sub-pad addressing are both
//!   port-2-only, `docs/scheduler.md`/`rustysnes_core::controller`'s own docs). Super Multitap's
//!   four sub-pads poll libretro ports `[1, 4]` (bsnes' own precedent: sub-pad `N` reads from
//!   libretro port `1 + N`, i.e. RetroArch's Player 2-5), each still the standard 12-button pad.
//!   Super Scope reads `RETRO_DEVICE_LIGHTGUN`'s absolute screen coordinates + trigger/cursor/
//!   turbo/pause, converting libretro's `[-0x8000, 0x7fff]` screen-space range into the pixel
//!   coordinates `EmuCore::set_superscope` expects.
//! - **Firmware**: coprocessor firmware dumps (DSP-1..4, CX4) are auto-resolved from the
//!   frontend's system directory (`RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY`), mirroring
//!   `rustysnes-frontend`'s own `firmware_candidates`/`install_firmware` policy.
//! - **Save RAM / WRAM / VRAM**: exposed as raw pointers via `get_memory_data`/`get_memory_size`
//!   (`Bus::wram_mut`/`Ppu::vram_mut`/`Cart::sram_mut`, `v1.2.0`) for RetroArch's own SRAM
//!   autosave and for RetroAchievements/cheat tooling that reads memory directly.
//! - **Cheats**: Game Genie / Pro Action Replay codes via `on_cheat_set`/`on_cheat_reset`, decoded
//!   through `rustysnes_core::cheat::decode` and applied via `Bus::set_cheats` — the same decoder
//!   and CPU-read-intercept mechanism `rustysnes-frontend`'s Cheats window uses.
//! - **Save states**: `EmuCore::save_state`/`load_state` (a versioned `System` snapshot,
//!   `docs/adr/0006`) via `get_serialize_size`/`on_serialize`/`on_unserialize`.

// FFI boundary wrapper, same posture as rustysnes-frontend/rustysnes-cheevos: every unsafe use
// here is a `rust_libretro`-mandated raw pointer/environment-callback interaction, not emulation
// logic (which stays entirely safe, inside `rustysnes-core`).
#![allow(unsafe_code)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::doc_markdown)]

use std::collections::BTreeMap;
use std::ffi::CString;

use rust_libretro::{
    contexts::*,
    core::{Core, CoreOptions},
    retro_core,
    sys::*,
    types::*,
};
use rustysnes_core::cart::Region;
use rustysnes_core::cheat::CheatPatch;
use rustysnes_core::facade::{EmuCore, MAX_H, MAX_W, SNES_H_NTSC, SNES_H_PAL, SNES_W};

/// The S-DSP's fixed output rate (`docs/apu.md`) — matches `rustysnes-frontend`'s own
/// `audio_core::SDSP_RATE` convention (a round 32 kHz, not the more precise 32040 Hz some
/// documentation cites; this project's own established constant).
const SAMPLE_RATE: f64 = 32_000.0;
/// NTSC frame rate (60.0988 Hz) — matches `rustysnes-frontend::FRAME_RATE_NTSC`.
const FPS_NTSC: f64 = 60.098_8;
/// PAL frame rate (50.007 Hz) — matches `rustysnes-frontend::FRAME_RATE_PAL`.
const FPS_PAL: f64 = 50.006_98;

/// `libretro.h`'s `RETRO_DEVICE_TYPE_SHIFT` — bindgen doesn't expose the function-like
/// `RETRO_DEVICE_SUBCLASS` macro, so [`retro_device_subclass`] replicates it by hand.
const RETRO_DEVICE_TYPE_SHIFT: u32 = 8;

/// Mirrors `libretro.h`'s `#define RETRO_DEVICE_SUBCLASS(base, id) (((id + 1) <<
/// RETRO_DEVICE_TYPE_SHIFT) | base)` exactly — a function-like macro bindgen can't expose as a
/// Rust item.
const fn retro_device_subclass(base: u32, id: u32) -> u32 {
    ((id + 1) << RETRO_DEVICE_TYPE_SHIFT) | base
}

/// Super Multitap's device code (`RETRO_DEVICE_JOYPAD` subclass 0) — matches bsnes's own
/// `RETRO_DEVICE_JOYPAD_MULTITAP` (`ref-proj/bsnes/bsnes/target-libretro/libretro.cpp`).
const RETRO_DEVICE_JOYPAD_MULTITAP: u32 = retro_device_subclass(RETRO_DEVICE_JOYPAD, 0);
/// Super Scope's device code (`RETRO_DEVICE_LIGHTGUN` subclass 0) — matches bsnes's own
/// `RETRO_DEVICE_LIGHTGUN_SUPER_SCOPE`.
const RETRO_DEVICE_LIGHTGUN_SUPER_SCOPE: u32 = retro_device_subclass(RETRO_DEVICE_LIGHTGUN, 0);

/// Map a raw libretro device code (as passed to [`Core::on_set_controller_port_device`] and
/// stored in [`RustySnesLibretro::device_per_port`]) to the core's own peripheral enum. Anything
/// unrecognized (including plain `RETRO_DEVICE_JOYPAD`/`RETRO_DEVICE_NONE`) falls back to
/// `Gamepad` — the safe default every port already starts at.
const fn map_libretro_device(device: u32) -> rustysnes_core::controller::PortDevice {
    use rustysnes_core::controller::PortDevice;
    if device == RETRO_DEVICE_MOUSE {
        PortDevice::Mouse
    } else if device == RETRO_DEVICE_JOYPAD_MULTITAP {
        PortDevice::Multitap
    } else if device == RETRO_DEVICE_LIGHTGUN_SUPER_SCOPE {
        PortDevice::SuperScope
    } else {
        PortDevice::Gamepad
    }
}

/// Convert a libretro lightgun/pointer absolute screen coordinate (`[-0x8000, 0x7fff]`, zero =
/// center) into a pixel offset in `[0, dimension)`, the space `EmuCore::set_superscope` expects.
/// Widened to `i64` for the intermediate product purely for clarity, not because `i32` would
/// overflow here (`dimension` never exceeds a few hundred).
fn lightgun_screen_to_pixel(raw: i16, dimension: u32) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    let pixel = (i64::from(raw) + 0x8000) * i64::from(dimension) / 0x1_0000;
    pixel as i32
}

/// Poll one controller port's input for the peripheral `device` (a raw libretro device code,
/// mapped via [`map_libretro_device`]) and feed it to `core` — split out of `on_run` purely to
/// keep that function under the workspace's `too_many_lines` clippy budget, not because this
/// logic is reused elsewhere.
fn poll_port_input(ctx: &RunContext<'_>, core: &mut EmuCore, port: usize, device: u32) {
    match map_libretro_device(device) {
        rustysnes_core::controller::PortDevice::Mouse => {
            let x =
                ctx.get_input_state(port as u32, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_X);
            let y =
                ctx.get_input_state(port as u32, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_Y);
            let left = ctx.get_input_state(
                port as u32,
                RETRO_DEVICE_MOUSE,
                0,
                RETRO_DEVICE_ID_MOUSE_LEFT,
            ) != 0;
            let right = ctx.get_input_state(
                port as u32,
                RETRO_DEVICE_MOUSE,
                0,
                RETRO_DEVICE_ID_MOUSE_RIGHT,
            ) != 0;
            core.set_mouse(port, i32::from(x), i32::from(y), left, right);
        }
        rustysnes_core::controller::PortDevice::SuperScope => {
            let offscreen = ctx.get_input_state(
                port as u32,
                RETRO_DEVICE_LIGHTGUN,
                0,
                RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN,
            ) != 0;
            let (fb_w, fb_h) = core.fb_dims();
            let (x, y) = if offscreen {
                // Matches `SuperScopeState::set_input`'s own `-16` fringe convention (one step
                // further out to stay unambiguously off-screen after its `-16..=dimension+16`
                // clamp).
                (-20, -20)
            } else {
                let raw_x = ctx.get_input_state(
                    port as u32,
                    RETRO_DEVICE_LIGHTGUN,
                    0,
                    RETRO_DEVICE_ID_LIGHTGUN_SCREEN_X,
                );
                let raw_y = ctx.get_input_state(
                    port as u32,
                    RETRO_DEVICE_LIGHTGUN,
                    0,
                    RETRO_DEVICE_ID_LIGHTGUN_SCREEN_Y,
                );
                (
                    lightgun_screen_to_pixel(raw_x, fb_w),
                    lightgun_screen_to_pixel(raw_y, fb_h),
                )
            };
            let mut buttons = 0u8;
            if ctx.get_input_state(
                port as u32,
                RETRO_DEVICE_LIGHTGUN,
                0,
                RETRO_DEVICE_ID_LIGHTGUN_TRIGGER,
            ) != 0
            {
                buttons |= rustysnes_core::controller::scope::TRIGGER;
            }
            if ctx.get_input_state(
                port as u32,
                RETRO_DEVICE_LIGHTGUN,
                0,
                RETRO_DEVICE_ID_LIGHTGUN_AUX_A,
            ) != 0
            {
                buttons |= rustysnes_core::controller::scope::CURSOR;
            }
            if ctx.get_input_state(
                port as u32,
                RETRO_DEVICE_LIGHTGUN,
                0,
                RETRO_DEVICE_ID_LIGHTGUN_AUX_B,
            ) != 0
            {
                buttons |= rustysnes_core::controller::scope::TURBO;
            }
            if ctx.get_input_state(
                port as u32,
                RETRO_DEVICE_LIGHTGUN,
                0,
                RETRO_DEVICE_ID_LIGHTGUN_START,
            ) != 0
            {
                buttons |= rustysnes_core::controller::scope::PAUSE;
            }
            core.set_superscope(port, x, y, buttons);
        }
        rustysnes_core::controller::PortDevice::Multitap => {
            // bsnes's own precedent (`ref-proj/bsnes/bsnes/target-libretro/program.cpp`): sub-pad
            // `N` polls libretro port `port + N` (RetroArch's Player 2-5 when the multitap sits on
            // SNES port 2), each still the standard 12-button pad.
            for sub in 0..4usize {
                let jp = ctx.get_joypad_state((port + sub) as u32, 0);
                core.set_multitap_pad(port, sub, joypad_state_to_snes_bits(jp));
            }
        }
        rustysnes_core::controller::PortDevice::Gamepad => {
            let jp = ctx.get_joypad_state(port as u32, 0);
            core.set_pad(port, joypad_state_to_snes_bits(jp));
        }
    }
}

/// The central libretro core structure for RustySNES.
pub struct RustySnesLibretro {
    /// The relocated pure facade. `None` until `on_load_game` (power-on with no ROM has no
    /// meaningful libretro state to expose — `retro_init` fires before any ROM is known).
    core: Option<EmuCore>,
    /// Intermediate buffer for interleaved i16 stereo audio samples — pre-allocated so the hot
    /// `on_run` loop avoids heap allocations in the steady state.
    audio_buffer: Vec<i16>,
    /// Intermediate buffer for the XRGB8888 video frame (byte-swapped from `EmuCore`'s RGBA8) —
    /// pre-allocated to the hi-res worst case (`MAX_W` x `MAX_H` x 4 bytes).
    video_buffer: Vec<u8>,
    /// Pre-computed save-state size, evaluated once at `on_load_game` (a `System` snapshot's size
    /// is constant for a given cart/board, `docs/adr/0006`).
    serialize_size: usize,
    /// Pre-allocated buffer for snapshot serialization.
    serialize_buffer: Vec<u8>,
    /// Set on `on_load_game`, cleared once the first `on_run` successfully reports the ROM's
    /// auto-detected region via `RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO` (only callable from
    /// `RunContext`, hence deferred past `on_load_game` itself).
    pending_av_info: bool,
    /// Currently-armed cheats, keyed by libretro's `index` (`on_cheat_set`'s "replace the entry at
    /// this index" contract) — `None` when the frontend cleared/disabled that index. Rebuilt into
    /// a flat `Vec<CheatPatch>` and pushed to `Bus::set_cheats` on every change, mirroring
    /// `rustysnes-frontend::cheats::sync`'s "always replace the whole active set" convention.
    cheats: BTreeMap<std::os::raw::c_uint, CheatPatch>,
    /// The raw libretro device code last selected for each of the two SNES controller ports
    /// (`Core::on_set_controller_port_device`), defaulting to `RETRO_DEVICE_JOYPAD`. Stored
    /// separately from the core's own `PortDevice` (rather than just calling
    /// `EmuCore::set_port_device` and forgetting it) because a frontend may select a device
    /// BEFORE `on_load_game` creates `self.core` — re-applied from here once the core exists
    /// (`on_load_game`), and consulted every `on_run` to decide which libretro input API to poll.
    device_per_port: [u32; 2],
}

impl Default for RustySnesLibretro {
    fn default() -> Self {
        Self {
            core: None,
            audio_buffer: Vec::with_capacity(4096),
            video_buffer: Vec::with_capacity((MAX_W * MAX_H * 4) as usize),
            serialize_size: 0,
            serialize_buffer: Vec::new(),
            pending_av_info: false,
            cheats: BTreeMap::new(),
            device_per_port: [RETRO_DEVICE_JOYPAD, RETRO_DEVICE_JOYPAD],
        }
    }
}

impl CoreOptions for RustySnesLibretro {}

/// A hand-rolled mirror of libretro's `retro_game_info_ext` C struct, bypassing
/// `rust-libretro-sys` 0.3.2's opaque bindgen output for it (see `on_load_game`'s doc). Field
/// order/types match `libretro.h` exactly (verified against the vendored header in
/// `rust-libretro-sys`'s own crate source) — every field here is either a plain pointer or a
/// `bool`, so this has no hidden padding/alignment surprises on any platform libretro targets.
#[repr(C)]
struct RetroGameInfoExt {
    full_path: *const std::os::raw::c_char,
    archive_path: *const std::os::raw::c_char,
    archive_file: *const std::os::raw::c_char,
    dir: *const std::os::raw::c_char,
    name: *const std::os::raw::c_char,
    ext: *const std::os::raw::c_char,
    meta_data: *const std::os::raw::c_char,
    data: *const std::os::raw::c_void,
    size: usize,
    file_in_archive: bool,
    persistent_data: bool,
}

/// `(width, height, fps)` for the given region — the geometry/timing `on_get_av_info` and the
/// post-load `SET_SYSTEM_AV_INFO` correction both report.
const fn region_av(region: Region) -> (u32, u32, f64) {
    match region {
        Region::Ntsc => (SNES_W, SNES_H_NTSC, FPS_NTSC),
        Region::Pal => (SNES_W, SNES_H_PAL, FPS_PAL),
    }
}

/// Map a libretro `JoypadState` bitmask into RustySNES's canonical SNES auto-joypad `u16`
/// (`B Y Select Start Up Down Left Right A X L R`, MSB-first at bit 15 — see
/// `rustysnes-frontend::input`'s doc for the full bit-order rationale). Libretro's own
/// `RETRO_DEVICE_ID_JOYPAD_*` numbering conveniently covers the same 12 buttons, just in a
/// different bit order, so this is a fixed remap table, not a lossy conversion.
fn joypad_state_to_snes_bits(state: JoypadState) -> u16 {
    let mut bits = 0u16;
    let map: [(JoypadState, u16); 12] = [
        (JoypadState::B, 1 << 15),
        (JoypadState::Y, 1 << 14),
        (JoypadState::SELECT, 1 << 13),
        (JoypadState::START, 1 << 12),
        (JoypadState::UP, 1 << 11),
        (JoypadState::DOWN, 1 << 10),
        (JoypadState::LEFT, 1 << 9),
        (JoypadState::RIGHT, 1 << 8),
        (JoypadState::A, 1 << 7),
        (JoypadState::X, 1 << 6),
        (JoypadState::L, 1 << 5),
        (JoypadState::R, 1 << 4),
    ];
    for (flag, bit) in map {
        if state.contains(flag) {
            bits |= bit;
        }
    }
    bits
}

impl Core for RustySnesLibretro {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("RustySNES").unwrap(),
            library_version: CString::new(env!("CARGO_PKG_VERSION")).unwrap(),
            valid_extensions: CString::new("sfc|smc|swc|fig").unwrap(),
            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
        // Conservative NTSC default before any ROM is loaded; corrected in `on_run` once the
        // cart header's region byte has actually been parsed (see `region_av`/`pending_av_info`).
        retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: SNES_W,
                base_height: SNES_H_NTSC,
                max_width: MAX_W,
                max_height: MAX_H,
                aspect_ratio: 4.0 / 3.0,
            },
            timing: retro_system_timing {
                fps: FPS_NTSC,
                sample_rate: SAMPLE_RATE,
            },
        }
    }

    fn on_set_environment(&mut self, _initial: bool, ctx: &mut SetEnvironmentContext) {
        // SAFETY: the libretro API guarantees a valid environment callback pointer here;
        // `set_pixel_format`/`set_input_descriptors` are safe FFI wrappers over it.
        unsafe {
            let generic_ctx: GenericContext = (&*ctx).into();
            let cb = *generic_ctx.environment_callback();

            if !rust_libretro::environment::set_pixel_format(cb, PixelFormat::XRGB8888) {
                eprintln!(
                    "[RustySNES] Error: frontend rejected XRGB8888 pixel format; colors will be broken."
                );
            }

            let descriptors = rust_libretro::input_descriptors!(
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_B, "B" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_Y, "Y" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_SELECT, "Select" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_START, "Start" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP, "D-Pad Up" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN, "D-Pad Down" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT, "D-Pad Left" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT, "D-Pad Right" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A, "A" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_X, "X" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_L, "L" },
                { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_R, "R" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_B, "B" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_Y, "Y" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_SELECT, "Select" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_START, "Start" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP, "D-Pad Up" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN, "D-Pad Down" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT, "D-Pad Left" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT, "D-Pad Right" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A, "A" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_X, "X" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_L, "L" },
                { 1, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_R, "R" }
            );
            rust_libretro::environment::set_input_descriptors(cb, &descriptors);

            // Peripheral negotiation (`RETRO_ENVIRONMENT_SET_CONTROLLER_INFO`) — mirrors bsnes's
            // own libretro core's device menu exactly (`ref-proj/bsnes/bsnes/target-libretro/
            // libretro.cpp`): Mouse/Super Multitap/Super Scope are only offered on port 2 (index
            // `1`), matching real SNES hardware wiring. `rust_libretro` has no `controller_info!`
            // helper macro (unlike `input_descriptors!`), so this builds the raw
            // `retro_controller_info` array by hand.
            let port0_types = [
                retro_controller_description {
                    desc: rust_libretro::c_char_ptr!("SNES Joypad"),
                    id: RETRO_DEVICE_JOYPAD,
                },
                retro_controller_description {
                    desc: rust_libretro::c_char_ptr!("SNES Mouse"),
                    id: RETRO_DEVICE_MOUSE,
                },
            ];
            let port1_types = [
                retro_controller_description {
                    desc: rust_libretro::c_char_ptr!("SNES Joypad"),
                    id: RETRO_DEVICE_JOYPAD,
                },
                retro_controller_description {
                    desc: rust_libretro::c_char_ptr!("SNES Mouse"),
                    id: RETRO_DEVICE_MOUSE,
                },
                retro_controller_description {
                    desc: rust_libretro::c_char_ptr!("Super Multitap"),
                    id: RETRO_DEVICE_JOYPAD_MULTITAP,
                },
                retro_controller_description {
                    desc: rust_libretro::c_char_ptr!("Super Scope"),
                    id: RETRO_DEVICE_LIGHTGUN_SUPER_SCOPE,
                },
            ];
            let controller_info = [
                retro_controller_info {
                    types: port0_types.as_ptr(),
                    num_types: port0_types.len() as std::os::raw::c_uint,
                },
                retro_controller_info {
                    types: port1_types.as_ptr(),
                    num_types: port1_types.len() as std::os::raw::c_uint,
                },
            ];
            rust_libretro::environment::set_controller_info(cb, &controller_info);
        }
    }

    fn on_set_controller_port_device(&mut self, port: u32, device: u32, _ctx: &mut GenericContext) {
        let Ok(idx) = usize::try_from(port) else {
            return;
        };
        if idx >= self.device_per_port.len() {
            return; // RustySNES only has two physical controller ports.
        }
        self.device_per_port[idx] = device;
        if let Some(core) = self.core.as_mut() {
            core.set_port_device(idx, map_libretro_device(device));
        }
    }

    fn on_load_game(
        &mut self,
        _game: Option<retro_game_info>,
        ctx: &mut LoadGameContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // `rust-libretro-sys` 0.3.2's bindgen output makes `retro_game_info` an opaque 1-byte
        // placeholder (a forward-declaration artifact — verified against the generated
        // `bindings_libretro.rs`), so `_game` above is unusable. `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT`
        // is the proven workaround (same one `rustynes-libretro`, the sibling NES core, uses):
        // a hand-rolled `#[repr(C)]` struct matching libretro's real `retro_game_info_ext` layout,
        // fetched directly through the raw environment callback.
        let ext_info = unsafe {
            let generic_ctx: GenericContext = (&*ctx).into();
            // A `None` callback here would mean the frontend never called `retro_set_environment`
            // before `retro_load_game` — a libretro spec violation, but external/untrusted input
            // to this crate nonetheless; fail gracefully via the caller's `Result` rather than
            // panicking (`rust-libretro`'s own examples `.unwrap()` here, but this project's own
            // convention is to never panic on data an external frontend controls).
            let Some(cb) = *generic_ctx.environment_callback() else {
                return Err("frontend provided no environment callback (spec violation)".into());
            };
            let mut ptr: *const RetroGameInfoExt = std::ptr::null();
            // SAFETY: `cb` is a valid function pointer supplied by the libretro frontend via the
            // environment context. If the callback returns true, the spec guarantees `ptr` is set
            // to a valid, aligned, frontend-owned `RetroGameInfoExt` whose lifetime is at least as
            // long as this `on_load_game` invocation. `as_ref()` returns `None` if `ptr` remains
            // null (a spec-violating frontend that returns `true` without setting the pointer).
            if cb(
                RETRO_ENVIRONMENT_GET_GAME_INFO_EXT,
                std::ptr::addr_of_mut!(ptr).cast::<std::os::raw::c_void>(),
            ) {
                ptr.as_ref()
            } else {
                None
            }
        }
        .ok_or("frontend does not support get_game_info_ext")?;

        if ext_info.data.is_null() {
            return Err(
                "ext_info data pointer is NULL (the frontend did not load the ROM into memory)"
                    .into(),
            );
        }
        // SAFETY: `data` is non-null (checked above). The libretro spec guarantees the pointer
        // references a valid, contiguous byte slice of exactly `size` bytes, owned by the
        // frontend for the duration of this call.
        let rom_bytes =
            unsafe { std::slice::from_raw_parts(ext_info.data.cast::<u8>(), ext_info.size) };

        let mut core = EmuCore::new(0, Region::Ntsc);
        // `load_rom` already copies the ROM into its own retained storage (for Power-Cycle) —
        // passing the borrowed slice directly avoids a redundant extra full-ROM copy here.
        core.load_rom(rom_bytes)
            .map_err(|e| format!("failed to load ROM: {e}"))?;

        // Auto-resolve coprocessor firmware (DSP-1..4/CX4) from the frontend's system directory —
        // mirrors `rustysnes-frontend`'s own `firmware_candidates`/`install_firmware` policy.
        if core.needs_firmware() {
            // SAFETY: same environment-callback contract as `on_set_environment`.
            let system_dir = unsafe {
                let generic_ctx: GenericContext = (&*ctx).into();
                let cb = *generic_ctx.environment_callback();
                rust_libretro::environment::get_system_directory(cb)
            };
            if let Some(dir) = system_dir {
                for name in core.firmware_candidates() {
                    if std::fs::read(dir.join(name))
                        .is_ok_and(|bytes| core.install_firmware(&bytes))
                    {
                        break;
                    }
                }
            }
            if core.needs_firmware() {
                eprintln!(
                    "[RustySNES] Warning: this cart needs coprocessor firmware ({:?}) not found \
                     in the frontend's system directory; the coprocessor will be non-functional.",
                    core.firmware_candidates()
                );
            }
        }

        // `save_state()` already returns a freshly allocated `Vec` — just measure it directly
        // instead of copying it again into a second buffer only to read its length.
        self.serialize_size = core.save_state().len();

        // Re-apply any peripheral already selected via `on_set_controller_port_device` — a
        // frontend commonly sets the controller port device BEFORE `retro_load_game` (from saved
        // per-game settings), which would otherwise be silently lost since `self.core` didn't
        // exist yet to receive it.
        for (idx, &device) in self.device_per_port.iter().enumerate() {
            core.set_port_device(idx, map_libretro_device(device));
        }

        self.core = Some(core);
        self.pending_av_info = true;
        self.cheats.clear();
        Ok(())
    }

    fn on_run(&mut self, ctx: &mut RunContext, _delta_us: Option<i64>) {
        let Some(core) = self.core.as_mut() else {
            return;
        };

        ctx.poll_input();
        for port in 0..2usize {
            poll_port_input(ctx, core, port, self.device_per_port[port]);
        }

        core.run_frame();

        // `run_frame`'s first call (`System::run_frame` internally resets on `!booted`) is what
        // actually parses the cart header and picks the real region — only correct after that
        // point, hence deferred here rather than attempted in `on_load_game`.
        if self.pending_av_info {
            let region = core.system_mut().bus.ppu.region();
            let core_region = match region {
                rustysnes_core::ppu::Region::Ntsc => Region::Ntsc,
                rustysnes_core::ppu::Region::Pal => Region::Pal,
            };
            let (w, h, fps) = region_av(core_region);
            let av_info = retro_system_av_info {
                geometry: retro_game_geometry {
                    base_width: w,
                    base_height: h,
                    max_width: MAX_W,
                    max_height: MAX_H,
                    aspect_ratio: 4.0 / 3.0,
                },
                timing: retro_system_timing {
                    fps,
                    sample_rate: SAMPLE_RATE,
                },
            };
            // SAFETY: `RunContext`'s own environment-callback contract; `av_info` is a plain,
            // fully-initialized POD struct passed by value.
            unsafe {
                let generic_ctx: GenericContext = (&*ctx).into();
                let cb = *generic_ctx.environment_callback();
                rust_libretro::environment::set_system_av_info(cb, av_info);
            }
            self.pending_av_info = false;
        }

        let (w, h) = core.fb_dims();
        self.video_buffer.clear();
        self.video_buffer.extend_from_slice(core.framebuffer());
        for chunk in self.video_buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2); // RGBA8 -> XRGB8888 (in-memory B,G,R,X): swap R and B.
        }
        ctx.draw_frame(&self.video_buffer, w, h, (w * 4) as usize);

        self.audio_buffer.clear();
        for &(l, r) in core.audio() {
            self.audio_buffer.push(l);
            self.audio_buffer.push(r);
        }
        AudioContext::from(&mut *ctx).batch_audio_samples(&self.audio_buffer);
    }

    fn on_cheat_reset(&mut self, _ctx: &mut CheatResetContext) {
        self.cheats.clear();
        if let Some(core) = self.core.as_mut() {
            core.system_mut().bus.set_cheats(&[]);
        }
    }

    fn on_cheat_set(
        &mut self,
        index: std::os::raw::c_uint,
        enabled: bool,
        code: &std::ffi::CStr,
        _ctx: &mut CheatSetContext,
    ) {
        if enabled {
            if let Some(patch) = code
                .to_str()
                .ok()
                .and_then(|s| rustysnes_core::cheat::decode(s).ok())
            {
                self.cheats.insert(index, patch);
            } else {
                eprintln!("[RustySNES] Warning: could not decode cheat code at index {index}");
                self.cheats.remove(&index);
            }
        } else {
            self.cheats.remove(&index);
        }
        if let Some(core) = self.core.as_mut() {
            let patches: Vec<CheatPatch> = self.cheats.values().copied().collect();
            core.system_mut().bus.set_cheats(&patches);
        }
    }

    fn get_memory_data(
        &mut self,
        id: std::os::raw::c_uint,
        _ctx: &mut GetMemoryDataContext,
    ) -> *mut std::os::raw::c_void {
        self.core
            .as_mut()
            .map_or(std::ptr::null_mut(), |core| match id {
                RETRO_MEMORY_SAVE_RAM => {
                    core.system_mut()
                        .bus
                        .cart
                        .as_mut()
                        .map_or(std::ptr::null_mut(), |c| {
                            let sram = c.sram_mut();
                            if sram.is_empty() {
                                std::ptr::null_mut()
                            } else {
                                sram.as_mut_ptr().cast::<std::os::raw::c_void>()
                            }
                        })
                }
                RETRO_MEMORY_SYSTEM_RAM => core
                    .system_mut()
                    .bus
                    .wram_mut()
                    .as_mut_ptr()
                    .cast::<std::os::raw::c_void>(),
                RETRO_MEMORY_VIDEO_RAM => core
                    .system_mut()
                    .bus
                    .ppu
                    .vram_mut()
                    .as_mut_ptr()
                    .cast::<std::os::raw::c_void>(),
                _ => std::ptr::null_mut(),
            })
    }

    fn get_memory_size(
        &mut self,
        id: std::os::raw::c_uint,
        _ctx: &mut GetMemorySizeContext,
    ) -> usize {
        self.core.as_ref().map_or(0, |core| match id {
            RETRO_MEMORY_SAVE_RAM => core.save_sram().len(),
            RETRO_MEMORY_SYSTEM_RAM => 0x2_0000, // WRAM_SIZE (128 KiB), fixed regardless of cart.
            RETRO_MEMORY_VIDEO_RAM => 0x8000 * 2, // 32Ki words * 2 bytes.
            _ => 0,
        })
    }

    fn get_serialize_size(&mut self, _ctx: &mut GetSerializeSizeContext) -> usize {
        self.serialize_size
    }

    fn on_serialize(&mut self, slice: &mut [u8], _ctx: &mut SerializeContext) -> bool {
        let Some(core) = self.core.as_ref() else {
            return false;
        };
        // `save_state()` already allocates a fresh `Vec` internally, so there is no capacity to
        // reuse by copying into `serialize_buffer` via `extend_from_slice` — assign it directly
        // instead of paying for a second full-snapshot copy (RetroArch's own rewind feature can
        // call this once per frame, making the extra copy a genuine hot-path cost).
        self.serialize_buffer = core.save_state();
        if slice.len() >= self.serialize_buffer.len() {
            slice[..self.serialize_buffer.len()].copy_from_slice(&self.serialize_buffer);
            true
        } else {
            false
        }
    }

    fn on_unserialize(&mut self, slice: &mut [u8], _ctx: &mut UnserializeContext) -> bool {
        self.core
            .as_mut()
            .is_some_and(|core| core.load_state(slice).is_ok())
    }
}

retro_core!(RustySnesLibretro {
    core: None,
    audio_buffer: Vec::with_capacity(4096),
    video_buffer: Vec::with_capacity((MAX_W * MAX_H * 4) as usize),
    serialize_size: 0,
    serialize_buffer: Vec::new(),
    pending_av_info: false,
    cheats: BTreeMap::new(),
    device_per_port: [RETRO_DEVICE_JOYPAD, RETRO_DEVICE_JOYPAD],
});
