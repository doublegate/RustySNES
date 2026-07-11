# Frontend — RustySNES

**References:** `docs/architecture.md` §6; `ref-docs/research-report.md` "External
dependencies"; `docs/adr/0004` (the determinism boundary).

## Purpose

`rustysnes-frontend` is the desktop + wasm shell: **winit + wgpu + cpal + egui**, pure Rust
and permissive (mirrors RustyNES). It is an **always-on egui shell, not a bare window** —
egui runs every frame.

**Status (Phase 5): playable native.** A real commercial ROM boots in a window with picture
(PPU BGR555 → RGBA8, aspect-correct 4:3 sub-rect letterbox blit), sound (S-DSP 32 kHz FIFO →
producer-side DRC-paced linear resampler → lock-free ring → cpal stereo), and control (keyboard +
gilrs gamepad → `Bus::set_joypad`). ROM load auto-resolves coprocessor firmware + `.srm` SRAM;
Reset / Power-Cycle / Pause are wired. The dependency stack tracks the latest mutually-compatible
tier: egui/egui-wgpu/egui-winit **0.35**, wgpu **29**, winit **0.30** (winit 0.31 is beta-only and
egui-winit 0.35 pins to 0.30 — winit is the crate gating us off 0.31). Native + `wasm32` both
build; the `playable_smoke` test is the headless AV proof.

## The shell model (the load-bearing rule)

- egui draws a **persistent menu bar** (File / Emulation / Tools / View / Debug / Help) +
  **status bar** + **tabbed Settings**, with toggleable CPU/PPU/APU/memory **debugger panels**
  layered on top.
- **Never hold the emu lock inside the egui closure.** Menu interactions return a `MenuAction`
  that the app dispatches *after* the egui pass; the hidden render branch copies the
  framebuffer under a brief lock, drops it, then renders / presents.
- By default, the emulator runs synchronously inline in the winit render pass (a fixed-timestep
  loop in `App::render`), still behind an `Arc<Mutex<EmuCore>>` handle. The default-OFF
  `emu-thread` feature moves single-player frame production onto a **dedicated thread**
  (`emu_thread.rs`) instead, communicating via that same `Arc<Mutex<EmuCore>>` + a lock-free
  `SharedInput` + an `EmuControl` lifecycle block + a `PresentBuffer` lock-free framebuffer
  handoff. `v1.1.0` closed the two biggest gaps: the thread now has real audio output
  (`crate::audio::AudioProducer`, pushed once per produced frame) and a proper pause/ROM-loaded
  lifecycle (`EmuControl`, driving a thread-owned `Pacer` that tracks live speed-preset changes)
  instead of an independent, uncontrollable pacing loop. Still not full parity: none of
  cheats/watchpoints/breakpoints/port2-peripheral/run-ahead/rewind/TAS-movies/Lua-scripting/
  netplay-aware-pause/RetroAchievements are ported into its loop yet — each needs a genuinely new
  shared-mutable-state design (those lists/buffers are currently plain `Active` fields the UI
  edits directly), not a mechanical port; see `crates/rustysnes-frontend/Cargo.toml`'s
  `emu-thread` feature comment and `emu_thread.rs`'s own module doc for the exact remaining list.
  Verified with a real headless launch (`xvfb-run`, a staged commercial ROM, no panics over
  several seconds of runtime) in addition to the unit suite. Stays opt-in rather than default
  until that remaining parity work lands.

**`EmuCore` split (`v1.2.0`).** The pure facade half of `EmuCore` — `new`/`load_rom`/firmware
resolution/SRAM/reset/power-cycle/the `set_*` peripheral feeds/`run_frame`/`present_current_frame`/
`framebuffer`/`audio`/`save_state`/`load_state` — relocated to
`rustysnes_core::facade::EmuCore` (a libretro core or any other headless embedder needs exactly
this surface without pulling in winit/wgpu/cpal/egui). `rustysnes-frontend::emu::EmuCore` is now a
thin wrapper (`inner: rustysnes_core::facade::EmuCore`) that adds only the debugger-only fields
(VRAM viewer scroll, `breakpoints`, `paused`) and the methods built on top of them
(`step_into`/`step_over`/`debug_snapshot`/breakpoint-aware `run_frame`). Every pure-facade method
is a one-line delegation — zero behavior change, verified by the unchanged frontend test suite
plus the `no_std` CI job (the acid test that the new `#[cfg(feature = "std")]` gate on the facade
module actually vanishes it from the `thumbv7em` build).

## Theme (`v1.0.0`)

`config.theme` (`crate::config::AppTheme`: `Light` / `Dark` (default) / `System`) selects the
egui `Visuals` for the whole shell (menu bar, status bar, all windows), set via Settings → System.
`ui_shell::apply_theme` performs the actual `ctx.set_visuals` call; `System` reads
`egui::Context::system_theme()` and falls back to `Dark` when the windowing system reports none.
`Active::applied_theme` tracks what's currently live so `App::render` only re-themes on an actual
change (the same guard `applied_present_mode` already uses for the Settings → Video present-mode
toggle), applied once explicitly at `egui::Context` construction time so the configured theme is
live from the very first frame, not just after the user opens Settings.

## Presentation post-filters (`v1.2.0`)

`config.video.filter` (`crate::config::PostFilter`: `None` (default) / `Crt` / `Hqx`) selects a
post-process pass applied after the plain nearest-sample framebuffer blit, set via Settings →
Video (a radio row + per-filter strength sliders) or the View → Post-filter submenu.

- **`PostFilter::None`** is the pre-`v1.2.0` direct blit, kept byte-for-byte unchanged: `Gfx::blit`
  itself was never modified by this addition, and `Gfx::present`'s `None` arm calls it directly
  rather than routing through any new code path — "no post-process filter active" is pixel-
  identical to a filter-less build by construction, not just by testing.
- **`PostFilter::Crt`** — scanlines (a parabolic per-source-row brightness profile) + an RGB
  aperture-grille mask (a fixed-pitch phosphor-triad tint keyed off the output pixel column), each
  with its own `0.0..=1.0` strength slider (`config.video.crt_scanline`/`crt_mask`).
  `crt_scanline`/`crt_mask` both default to a subtle preset (`0.3`/`0.15`) rather than `0.0`, since
  a `Crt`-selecting user almost certainly wants a visible effect immediately — this is the one
  config default in this feature that is NOT "byte-identical to off" (selecting the filter at all
  is the deliberate opt-in; `PostFilter::None` remains the neutral default).
- **`PostFilter::Hqx`** — a single-pass, edge-directed diagonal blend (a diagonal-similarity
  heuristic in the 2xSaI/Eagle family: if the TL-BR texel diagonal is more self-similar than the
  TR-BL diagonal, or vice versa, the bilinear blend weight is biased toward the matching diagonal),
  softening staircase edges on flat-color pixel art. This is an HQ2x-**style** approximation, not a
  literal HQ2x pattern-lookup-table port — the right fit for a fixed-resolution architecture that
  never actually renders to a literal 2×-sized intermediate buffer. One strength slider
  (`config.video.hqx_strength`, default `0.6`).
- **Both filters share** the exact same clip-space letterbox convention `Gfx::blit`'s own vertex
  shader uses (position-scale, not UV-space cropping) — `Gfx::letterbox_scale` was extracted out of
  `blit`'s inline math specifically so `blit` and the two filter passes stay pixel-aligned, a pure,
  behavior-preserving refactor (verified by `letterbox_scale_matches_known_cases`, a hand-computed
  regression test for windows wider-than / narrower-than / exactly the 4:3 SNES aspect).
- **Architecture**: `Gfx` builds both filter pipelines unconditionally at init (`Gfx::new_async`,
  cheap — two small pipelines) so switching the Settings radio needs no pipeline
  creation/reallocation; `Gfx::present` selects between `blit`/`crt`/`hqx` per frame based on the
  live config. Both filter shaders (`CRT_WGSL`/`HQX_WGSL` in `gfx.rs`) are inline `const &str` WGSL,
  matching the existing `BLIT_WGSL` convention — deliberately NOT split into a separate
  `rustysnes-gfx-shaders` crate (that split only earns its keep with a second consumer, e.g. a
  mobile target, which this project has no near-term plan for).
- **Verified**: `naga` WGSL-parse+validate tests for both new shaders (same machinery `wgpu`
  itself uses at runtime); a real headless `xvfb-run` launch of the native binary against a staged
  ROM with each of `None`/`Crt`/`Hqx` set in `config.toml` in turn — all three ran clean (zero
  stderr output, no panics) for the full run window against a real (llvmpipe/software) wgpu
  adapter, confirming both new pipelines actually build and render, not just that their shaders
  parse statically. No golden-screenshot regression harness exists in this project today (the
  existing `commercial_screenshots.rs` captures the raw core framebuffer directly, entirely
  upstream of this wgpu render path) — the `None`-path-unchanged guarantee here is a structural
  one (the exact same `blit` function, not a re-derived equivalent), not a pixel-diff proof.
- **Not built** (documented scope cuts, not silent gaps): RustyNES's NTSC composite-signal
  simulation and `.slangp` RetroArch shader-preset loading — both explicitly out of this ticket's
  "CRT/HQx" scope. Overscan cropping remains a separate, pre-existing `TODO(impl-phase)` in the
  View menu.

## HD texture packs (`v1.3.0`, `hd-pack` feature)

**Status: fully implemented and wired into the live present path.** See `docs/ppu.md`'s own "HD
texture pack `TileTag` recording hook" section for the core-side half of this feature (the
write-only per-pixel tile-identity side-buffer).

- **Feature propagation**: `rustysnes-frontend/hd-pack` → `rustysnes-core/hd-pack` →
  `rustysnes-ppu/hd-pack`. The frontend never depends on `rustysnes-ppu` directly (the
  one-directional crate-graph rule) — it reaches `Ppu::set_hd_pack_tagging`/`Ppu::tile_tags` via
  `rustysnes_core::ppu` (an existing unconditional re-export) through
  `EmuCore::system_mut().bus.ppu` (both `System::bus` and `Bus::ppu` are already `pub`).
- **`crate::hd_pack`**: the manifest schema (`HdPackManifest`/`TileEntry`, TOML, keyed per tile by
  the hex tile-identity hash), the loader (`HdPack::load` — parses `pack.toml`, decodes every
  referenced PNG to RGBA8 via the pure-Rust `png` crate, normalizing any source color
  type/bit-depth), and per-ROM discovery (`discover_packs`/`load_pack`, mirroring
  `save_states.rs`'s `<data_dir>/hd-packs/<rom_sha256_hex>/<pack-name>/` directory convention —
  same SHA-256 identity `rustysnes_core::movie::hash_rom` already provides). A malformed pack
  (unsupported `format_version`, an invalid hex hash, an undecodable image) fails `HdPack::load`
  entirely rather than partially applying — a pack is accepted whole or not at all.
- **`crate::hd_compositor`**: a pure function, `composite(fb_rgba, fb_w, fb_h, tags, tiles,
  scale)`, taking the already-BGR555→RGBA8-decoded native framebuffer plus the PPU's per-pixel
  `TileTag` side-buffer and a loaded pack's decoded tiles. Each 8×8 output cell is sampled once
  (its top-left source pixel); a hash match blits that tile's own replacement image (mirrored per
  the tag's `hflip`/`vflip` — both orientations share one pack entry), a miss/backdrop
  nearest-neighbor-upscales the native color instead — the standard per-tile graceful fallback
  that lets "some tiles replaced, others native" work within one frame. Deliberately has no
  wgpu/`EmuCore` dependency, so it is fully testable standalone (`cargo test -p rustysnes-frontend
  --features hd-pack hd_compositor`) without a live GPU adapter.
- **`crate::emu::EmuCore` pack management** (`v1.3.0`): `available_hd_packs()` (discovery for the
  current ROM, only computed while Settings is open — a real filesystem `read_dir` call),
  `hd_pack_name()`, and `set_hd_pack(Option<&str>)` (loads/clears a pack and toggles
  `Ppu::set_hd_pack_tagging` to match — either fully active or fully off, never half-applied on a
  load failure). `load_rom`/`close_rom` clear any active pack (it's keyed to the ROM it was
  discovered under); `power_cycle` re-enables tagging on the freshly (re)constructed `Ppu` if a
  pack was active, since that reconstruction resets the tagging flag to its `false` default.
- **Settings → Video** gains a pack `ComboBox` (dynamic, unlike the fixed-choice present-mode/
  theme radios — the pack list depends on what's actually installed for this ROM) populated from
  `available_hd_packs()`, dispatching `MenuAction::SetHdPack` on selection. `VideoConfig` gains
  `hd_pack_name: Option<String>` (default `None`, additive); the configured pack is re-selected
  automatically after loading a ROM (both the CLI-argument path and File → Open ROM).
- **Final integration** (`v1.3.0`): `app.rs`'s present path now calls `hd_compositor::composite`
  (still under the brief `emu` lock — pure CPU work, no wgpu touched there) whenever a pack is
  active, replacing the plain framebuffer with the composited RGBA8 buffer before
  `Gfx::upload`, at a fixed `HD_PACK_SCALE = 2` upscale (`docs/adr/0010`'s documented v1 scope
  choice — not yet user-configurable). `Gfx`'s streaming texture, previously a fixed `MAX_W ×
  MAX_H` allocation, now grows via `Gfx::ensure_texture_capacity` to fit whatever the composited
  output needs (a hi-res frame at 2x tops out at 1024×896, comfortably under this device's actual
  granted `max_texture_dimension_2d` — see "Device texture limits" below); `Gfx::blit`/
  `Gfx::present`'s UV math divides by the texture's *current* actual size, not the `MAX_W`/`MAX_H`
  constants, so this stays correct after a grow. When no pack is active the texture never grows
  past its original `MAX_W × MAX_H` allocation and this is pixel-identical to before — verified
  both by the existing test suite and a real headless (`xvfb-run`) launch with no pack configured.
  Verified separately via headless launches with a real generated pack (both at the default 2x
  scale, and with scale temporarily forced to 3x specifically to exercise the texture-growth
  path) — all ran clean with no panics or wgpu validation errors.

### Device texture limits (post-`v1.3.0` fix)

`Gfx::new_async` used to request `wgpu::Limits::downlevel_webgl2_defaults()` unconditionally on
every target, which hard-caps `max_texture_dimension_2d` at 2048 even on native desktop GPUs that
support far more. Fullscreening on a monitor wider or taller than 2048px (e.g. an ultrawide at
3440×1368) made `Surface::configure` receive an out-of-range request and panic/abort the process
— `wgpu::Surface::configure` has no recoverable error path for this. Fixed by splitting the
requested limits by target: `wasm32` keeps `downlevel_webgl2_defaults()` (WebGL2's real ceiling),
native uses `downlevel_defaults()`, and both now call `.using_resolution(adapter.limits())`, which
raises the floor preset up to whatever the real adapter reports where that's higher. `Gfx` stores
the actual granted limit as `max_texture_dim` (`device.limits().max_texture_dimension_2d`) and
uses it everywhere the old hardcoded `MAX_TEXTURE_DIM` constant used to be checked
(`ensure_texture_capacity`, `upload`, and a new defensive clamp in `resize` and the initial
`SurfaceConfiguration`) — so the real backstop is now "whatever this device actually supports,"
not a fixed 2048 that was only ever correct for the WebGL2 downlevel case.
- **Not yet done**: a user-configurable upscale factor (fixed at 2x for now) and `emu-thread`-
  build compositing (that build's framebuffer arrives via a lock-free `PresentBuffer` handoff
  outside the locked block this wiring reads `Ppu::tile_tags` from, with no equivalent `TileTag`
  channel built yet) — both honestly tracked scope cuts, see `docs/adr/0010`.

## Global hotkeys (`v1.0.1`)

Every system/emulation action used to be menu-bar-only (`rustysnes help hotkeys` said so
explicitly). `app::window_event`'s `KeyboardInput` arm now checks a fixed, non-rebindable hotkey
table (`App::hotkey_menu_action` + `App::dispatch_hotkey`) **before** falling through to
`Self::latch_key` (P1 gameplay input), on the key-down edge only and never on OS auto-repeat:

| Key | Action |
|---|---|
| `Escape` | Quit |
| `F1` | Save State (quick slot) |
| `F2` | Reset |
| `F3` | Power Cycle |
| `F4` | Load State (quick slot) |
| `F5` | Rewind |
| `F9` | Toggle the Save States... window |
| `F11` | Toggle Fullscreen |
| `F12` | Open ROM |
| `Space` | Pause/Resume |
| `` ` `` (Backquote) | Toggle Debugger overlay (feature-gated: `debug-hooks`, mirrors the Debug menu's own gating exactly — no second way to open a surface the default build never vets) |

Hotkeys are suppressed while an egui widget has keyboard focus (`egui::Context::egui_wants_keyboard_input`)
— e.g. typing in a Settings text field — so `Space`/`` ` `` don't double as both a typed character
and a hotkey. `F9`/`F11` have no existing `MenuAction` variant (the mouse-driven UI flips the
`ShellState` field directly), so the hotkey path does the same rather than inventing an action
variant with no other caller; everything else dispatches through the existing `MenuAction`/
`App::dispatch_actions` pipeline, called directly from `window_event` rather than only from the
render/egui pass. `hotkey_menu_action` is a pure, unit-tested mapping (`app::hotkey_tests`),
independent of any live winit/wgpu context. The key-map deliberately avoids every default P1
binding (Arrows/X/Z/S/A/Q/W/RShift/Enter).

## The determinism boundary

Rate control (the dynamic-rate-control resampler) and run-ahead (snapshot/restore
orchestration) live **here, in the frontend, never in the core synthesis** — that is what
keeps the core's bit-identical contract intact (`docs/adr/0004`, `docs/architecture.md` §5).
Netplay rollback is likewise frontend-orchestrated against the deterministic core.

## Audio + pacing

- A **lock-free audio ring** fed by the core's 32 kHz stereo output, drained by cpal, with
  dynamic rate control to absorb pacing jitter.
- A display-sync pacing matrix targeting 60.0988 Hz (NTSC) / 50.0070 Hz (PAL).
- The optional non-deterministic "hardware-accurate audio" SPC-drift toggle (`docs/apu.md`
  §determinism-caveat) is a frontend setting, off by default, outside the deterministic path.
- **Per-voice mute** (`v1.0.1`) — Settings → Audio has 8 checkboxes (`config.audio.voice_mutes`),
  re-synced once per real frame (`Bus::set_voice_mutes`, the same "just re-sync unconditionally"
  pattern cheats/watchpoints/breakpoints already use). A frontend/debug convenience with no real
  hardware register behind it — see `docs/apu.md` §Per-voice mute for the exact mix-time-only gate
  and why it's excluded from save-states. All unmuted by default, byte-identical to every prior
  release.

### Fixed-timestep wall-clock pacing (synchronous drive)

winit's `RedrawRequested` fires once per **display** vsync, so stepping exactly one emulated
frame per redraw runs the emulator at the *monitor's* refresh — e.g. 2.4× too fast on a 144 Hz
panel. The synchronous (default, non-`emu-thread`) path therefore drives emulation from a
**wall-clock fixed-timestep accumulator** (`app::Pacer`): each present accumulates the real
elapsed time and runs `run_frame` only once `1 / region.frame_rate()` seconds have accrued,
presenting the latest framebuffer in between. Catch-up after a stall is capped
(`MAX_CATCHUP_FRAMES`, with the leftover backlog dropped) to avoid a spiral of death, and the
delta is clamped. The **present mode then governs only vsync/tearing, never emulation speed.**
The pacer's math is unit-tested (`pacing_tracks_region_rate_not_present_rate`) to hold ~60 fps
across 30/60/75/144/240 Hz present rates.

### FPS meter

`Pacer` doubles as the FPS meter: it counts emulated frames produced per wall-second over a
0.5 s window and exposes the smoothed value as `ShellInfo::fps`, which the status bar renders.
(In the `emu-thread` build the meter counts presents instead, since frames are produced off the
winit thread.)

### Speed presets (`v1.0.0`)

Emulation → Speed offers `[25%, 50%, 75%, 100%, 150%, 200%, 300%]` (`ui_shell::SPEED_PRESETS`,
matching RustyNES's own 7-tier array). Selecting one sets `Active::speed` (transient session
state — never persisted to `config.toml`; the app always launches at `1.0`x, the
determinism-safe default) and calls `Pacer::set_rate` with `region.frame_rate()` scaled by the
chosen multiplier, which live-reconfigures the fixed-timestep accumulator's target period without
resetting it (no burst/no stall on the change, same posture as `Gfx::set_present_mode`). The
audio resampler's DRC ratio is multiplied by `speed` too, so alt-speed audio pitch-shifts
(more/fewer source samples per real second) instead of over/underrunning the ring — the emulated
core itself never sees a speed concept; only the frontend's pacing + resampling scale. The
`emu-thread` build (`v1.1.0`) now honors speed presets too: `render`'s per-present sync pushes
`Active::speed` into `EmuControl`, and the thread's own `Pacer` instance (which drives its cadence)
picks it up on the next loop iteration — no longer the no-op it was before that port.

### Performance panel (`v1.0.0`)

View → Performance panel opens a small read-only diagnostic window: FPS, the current speed
preset, frame time (`Active::last_frame_time_ms` — wall-clock time spent in the frame-production
loop this present; `None`/"n/a" while paused or on the `emu-thread` build, where production
happens on a different thread outside this timing scope), and audio ring health (occupancy as a
percentage of capacity; `None`/"n/a" on `wasm32` or with no audio device). Unlike Settings/Save
States, this window has no controls — it only reads `ShellInfo`'s fields the app already builds
each present. It also plots a rolling ~2s frame-time sparkline (`ShellState::frame_time_history`,
capped at 120 samples) via a handful of `Painter::line` segments — no `egui_plot` dependency for
something this small.

### Fullscreen (`v1.0.0`)

View → Fullscreen toggles borderless fullscreen (`winit::window::Fullscreen::Borderless(None)`),
applied via the same "compare live state to `Active::applied_*` each frame, apply on mismatch"
pattern as the present-mode/theme toggles above.

### Window size presets (post-`v1.3.0`, RustyNES parity)

Native only (`#[cfg(not(target_arch = "wasm32"))]`; the wasm32 canvas is sized by the page's own
CSS, not this feature) — View → Window Size offers 1x/2x/3x/4x (100%-400%) of the SNES native
resolution, dispatching `MenuAction::SetWindowScale(u32)`. `App::create_window` uses `3x`
(`INITIAL_SCALE`) as the launch default, matching RustyNES's own default. `App::set_window_scale`
exits fullscreen first (so the resize takes effect against a normal window), clamps the requested
scale to `1..=4`, and computes a chrome-padded `LogicalSize` (`MIN_CHROME_WIDTH`/`CHROME_HEIGHT`,
padding for the egui menu bar so the emulated image area lands near the requested multiple even at
`1x`) before calling `window.request_inner_size`. That call may grant the resize synchronously
(`Some`, no separate `Resized` event follows, so `Gfx::resize` is called directly) or
asynchronously (`None`, handled by the existing `WindowEvent::Resized` handler). Transient,
session-only — no `config.toml` field, same posture as `MenuAction::SetSpeed`.

### First-run welcome modal (`v1.0.0`)

A brief orientation window shown once, the very first time the app launches with
`config.first_run_seen == false`. Its "Get Started" button is the only way to dismiss it
(`MenuAction::DismissWelcome`, which sets `first_run_seen = true` and saves the config so it
never reappears) — there's no title-bar close button.

### Present-mode application

The Settings → Video present-mode radio writes `config.video.present_mode`; the present path
detects a change against the last-applied mode and calls `Gfx::set_present_mode`, which
re-validates the request against the surface's supported modes (falling back to `Fifo`) and
**reconfigures the live wgpu surface**. Previously the surface was only ever configured once at
startup, so the toggle had no effect.

## Input

- USB gamepads auto-bind to P1; keyboard fallback for P1/P2.
- Late-latched input (sampled as close to the frame as possible) for responsiveness without
  breaking determinism.

### Key rebinding (`v1.0.0`)

Settings → Input renders a 12-row grid (one per `input::Button::ALL`, `ui_shell.rs`) showing each
SNES button's currently-bound key (`config.p1`) next to a "Rebind" button. Clicking it arms
`ShellState::awaiting_bind`; the very next physical key press is intercepted by
`App::window_event`'s `KeyboardInput` arm (`app.rs`) instead of being latched as gameplay input,
and applied via `KeyBindings::rebind` (`input.rs`), which clears any prior bind on the same key or
the same button first so the table never gets a duplicate. Esc cancels the capture instead of
binding itself. Only P1 is exposed: `config.p2` exists and round-trips through `config.toml`, but
no keyboard-driven gameplay path consults it yet (P2 today is only ever driven by TAS movie
playback or netplay) — a rebind UI for a table nothing reads would be misleading, so it's left for
whenever P2 local keyboard play is wired.

### Peripherals (Mouse / Super Scope / Super Multitap) — `v0.9.0`

The core (`rustysnes_core::controller`) implements the real 2-bit-per-clock (`data1`/`data2`)
serial-shift-register protocol for all three, ported from ares' `sfc/controller/
{mouse,super-scope,super-multitap}` — not stubs. `Bus::set_port_device` selects which peripheral
occupies a port (default: `Gamepad`, byte-identical to every prior release); `Bus::set_mouse`/
`set_superscope`/`set_multitap_pad` feed host input once per frame, the same "always replace,
re-synced once per frame" convention `set_joypad` already uses. Save-stated as real hardware
state (`FORMAT_VERSION` 2→3, `docs/adr/0006`), not host debug tooling.

**What this frontend wires today:** a Settings → Input control (`ui_shell.rs`) selects controller
port 2's peripheral via `config.port2_peripheral`, re-synced to the Bus every frame
(`app.rs`, alongside the cheats/watchpoints sync). **What it does NOT yet wire: live host-input
capture.** No code path currently feeds `set_mouse`/`set_superscope`/`set_multitap_pad` from a
real OS mouse pointer, a Super Scope crosshair overlay, or extra `gilrs` gamepads for Multitap
sub-pads 2-4 — selecting a non-`Gamepad` device correctly changes what the emulated hardware
reports (verifiable via `rustysnes-script`/the test harness calling the `EmuCore`/`Bus` methods
directly), but the default GUI session won't yet feel it move. This is a real, open follow-up
frontend task, not a silently-incomplete claim: closing it needs (a) a `WindowEvent::CursorMoved`/
`MouseInput` capture path (or reading `egui::Context`'s own pointer state, already available every
frame since `egui_state.on_window_event` runs unconditionally) mapped from window pixels through
the present path's letterbox/integer-scale transform (`gfx.rs`) to SNES `0..256`/`0..240` pixel
space, and (b) binding `gilrs` device indices 1-3 to Multitap sub-pads 1-3 (index 0 already has a
natural home in the existing P1 gamepad auto-bind).

## Save-states, rewind, run-ahead

- **Save-states** (`v0.2.0 "Persistence"`, `docs/adr/0006`) serialize the deterministic core
  state (including the SPC relative-time accumulator and the seeded power-on phase) into one
  versioned envelope via `System::save_state`/`load_state`. `EmuCore::save_state`/`load_state`
  wrap it, additionally re-rendering the framebuffer and clearing the
  audio FIFO on load (a state load jumps time discontinuously) — since `v1.2.0` this wrapping
  lives in `rustysnes_core::facade::EmuCore` (see the note below), with `rustysnes-frontend::emu`
  delegating straight through. Emulation → Save State / Load
  State drives a single quick-save slot held in `Active::quick_save` (RAM-only; lost on exit).
- **Save States manager** (`v1.0.0`, `save_states.rs`) is a separate, disk-backed,
  thumbnail-previewed 10-slot picker (Emulation → Save States…), additive on top of the RAM-only
  quick-save slot above, not a replacement for it. Slots live at
  `<platform-data-dir>/saves/<rom_sha256_hex>/slotN.rsst`, keyed by the same
  `rustysnes_core::movie::hash_rom` SHA-256 identity movies already use. Each slot file wraps an
  UNMODIFIED `EmuCore::save_state()` blob in a small frontend-only header carrying a
  nearest-neighbor-downsampled `128x112` RGBA8 thumbnail of the framebuffer at save time — this is
  a frontend-only addition, not a `rustysnes-savestate` `FORMAT_VERSION` bump (currently `3`,
  `docs/adr/0006`), unlike RustyNES's own approach of embedding the thumbnail inside the core
  blob itself. The manager window rebuilds its slot grid (thumbnail + "saved Ns ago") from disk
  once per frame while open, the same "only pay the cost while the overlay needing it is visible"
  convention the debugger snapshot already uses.
- **Rewind** (`v0.3.0 "Continuum"`, `crate::rewind::RewindBuffer`) is a bounded ring buffer of
  FULL save-state snapshots, recorded every `config.rewind.interval_frames` real frames (default
  6, i.e. ~10 Hz) up to `config.rewind.capacity` entries, oldest evicted first. This is simpler
  than the originally-sketched "keyframes + deltas" design — delta-compression is a possible
  future memory optimization, not a correctness requirement. **`capacity: 0` is the shipped
  default**, making recording a permanent no-op — off by default until a Settings-UI toggle + a
  dedicated hotkey land; the Emulation → Rewind menu item and the mechanism itself are both live
  today, driven purely by config. A user (or future UI) enabling it might reasonably pick
  something like `capacity: 300` at the default 6-frame interval (≈30s of NTSC rewind) — that's
  an example config, not what ships. Recorded snapshots are discarded (`RewindBuffer::clear`) on
  ROM load/close (a new cart invalidates any prior snapshot), NOT on Reset/Power-Cycle (rewinding
  past an accidental reset is a legitimate use).
- **Run-ahead** (`v0.3.0 "Continuum"`, `crate::rewind::step_with_run_ahead`) peeks
  `config.run_ahead.frames` frames ahead using the currently-latched input each displayed frame,
  presents that peek's video, then rolls back and re-runs exactly ONE real frame — so the
  persisted state (and its audio, the continuous stream — peek audio is never played) only ever
  advances by one frame per call, regardless of the peek depth. `frames: 0` (the shipped default)
  degrades to a plain `run_frame` — off by default.
- Both are pure re-simulation of the SAME deterministic core (`docs/adr/0004`): no injected
  timing/RNG, just running the existing `run_frame`/`save_state`/`load_state` extra times. Proven
  by `rewind.rs`'s tests, which hand-assemble a tiny 65C816 program (NMI-driven WRAM counter →
  CGRAM backdrop write) to get a real, observable per-frame state signal rather than asserting
  against a synthetic fingerprint.

## wasm

Two independently-functional wasm32 frontends, feature-gated so exactly one is compiled
(`lib.rs`); the determinism path is identical to native in both — the wasm build never injects
timing/RNG, matching the `docs/adr/0004` boundary.

**`wasm-winit` (default, `v0.8.0`, T-81-006)** routes the browser through the SAME `App`/
`ApplicationHandler<AppEvent>` the native binary uses (`app.rs`) — the full winit + wgpu + egui
shell, debugger overlay included, ported from RustyNES's own `wasm_winit.rs` (confirmed by
reading its source directly). Native and `wasm32` share one `ApplicationHandler` impl with
internal `#[cfg(target_arch = "wasm32")]` branches, not two parallel copies:

- **Window/`Gfx` init.** `wgpu`'s adapter/device request is a real async operation in the
  browser (`pollster::block_on` cannot block on `wasm32` — there is no second thread to block on
  while the single JS thread keeps the event loop alive), so `resumed()` `spawn_local`s
  `Gfx::new_async` and delivers the result back into the event loop as `AppEvent::GfxReady` via
  an `EventLoopProxy` (native drives the same async core synchronously via `pollster::block_on`
  inside `Gfx::new` and skips the proxy round-trip entirely). The window attaches to the
  existing `<canvas id="snes-canvas">` from `index.html` (`WindowAttributesExtWebSys::with_canvas`)
  — the same element `wasm-canvas` uses — rather than letting winit create a detached one, so the
  page's own CSS sizing/layout applies.
- **Backend selection.** `Gfx` probes `navigator.gpu`'s mere *presence* (not a real adapter
  attempt) to choose `wgpu::Backends::BROWSER_WEBGPU` or `::GL` and commits to exactly one before
  ever touching the canvas — a `<canvas>` can only bind one context type for its whole lifetime,
  and `Instance::create_surface` on a WebGPU-backed instance calls `canvas.getContext("webgpu")`
  immediately regardless of whether `request_adapter` later succeeds, permanently poisoning the
  canvas for a subsequent GL attempt. A browser that advertises `navigator.gpu` but then fails a
  real adapter request (disabled flag, blocklisted, no working ICD) surfaces a hard error rather
  than silently falling back to GL — a real, documented limitation, not pretended away.
- **Color space.** WebGPU/native round-trip an sRGB surface + sRGB framebuffer texture to
  identity (sampler sRGB→linear decode, surface linear→sRGB encode cancel out). The WebGL2
  (`Backend::Gl`) fallback does NOT: wgpu-hal's GL surface can't present to a real sRGB default
  framebuffer, so it adds an extra explicit encode at present time that, combined with GL's own
  automatic sRGB framebuffer encoding, breaks the round-trip and washes out the palette. Fix: on
  the GL backend only, keep everything in the UNORM domain (non-sRGB surface + non-sRGB
  framebuffer texture) — zero color conversion anywhere, matching `wasm-canvas`'s byte-exact
  output.
- **Audio.** `wasm32` drives `crate::wasm_audio` per-frame from `App::render` instead of the
  native `cpal`/`AudioOutput` path — the same `AudioWorkletNode`/`ScriptProcessorNode` graph
  `wasm-canvas` (T-81-005) uses, reusing the native DRC/resampler core (`audio_core.rs`)
  verbatim.
- **ROM loading.** No native file dialog on the web — `MenuAction::OpenRom` points the user at
  the page's own `<input id="rom-input">` (the same element `wasm-canvas` uses) instead of
  calling `rfd`. Selecting a file reads its bytes via `FileReader` and delivers them as
  `AppEvent::RomLoaded` through the `EventLoopProxy`, which `App` turns into a running `EmuCore`
  exactly like a native `MenuAction::OpenRom` would.
- **Config persistence.** `Config::path()` returns `None` on `wasm32` (no filesystem) — `load`/
  `save` degrade to "always the default" / "always a no-op" rather than being separately gated.
  The `v1.0.0` Save States manager (`save_states.rs`) hits the same wall: `base_dir()` also
  returns `None` on `wasm32`, so the menu entry is present but every save/load reports a
  "no writable data directory" status — a real, disclosed browser-vs-native gap (`index.html`'s
  own hint paragraph says so), not a silent no-op.

**Verified with a real headless-browser load** (Playwright/Chromium): the WebGL2 fallback path
renders correctly end-to-end — confirmed via a full-page screenshot showing the egui menu bar,
the status bar (region/FPS/ROM-loaded state), and the actual emulated framebuffer for a real
committed test ROM, not just "no console errors." **Honest gap:** this sandbox's headless
Chromium exposes `navigator.gpu` but returns "no compatible wgpu adapter" for a real WebGPU
request (several software-Vulkan launch-flag combinations were tried without success) — the
WebGPU-specific code path is exercised by the same shared `Gfx::new_async` core the verified GL
path uses, and its backend-selection/color-space reasoning is grounded in real prior hardware
testing (see the code comments), but a live screenshot of the WebGPU path specifically is not
achievable in this environment and is not claimed here.

**`wasm-canvas` (`v0.8.0`, T-81-005)** is a lighter, independently-functional fallback: a direct
`CanvasRenderingContext2d.putImageData` blit, no `wgpu`/`egui`, `requestAnimationFrame`-driven,
sharing the same `pacing::Pacer`/`wasm_audio`/`audio_core` modules `wasm-winit` uses. Selectable
via `--features wasm-canvas --no-default-features`; still fully functional and covered by CI —
"exactly one wasm frontend is compiled" per both modules' own docs, and the manifest keeps both
working rather than deleting the MVP once the full shell landed.

### The hosted demo page (`v1.0.0`)

`crates/rustysnes-frontend/web/index.html` (deployed by `.github/workflows/pages.yml`) got a
polish pass: a visible `<h1>RustySNES` title, a keyboard-controls + feature-parity hint paragraph
(matching the real `input::KeyBindings` defaults, and disclosing the Save States browser gap
above rather than staying silent about it), an inline-SVG favicon (no logo asset exists yet,
unlike RustyNES's `assets/RustyNES_Icon/` set, so this avoids either shipping no favicon at all
or a new binary asset to keep in sync), and a `theme-color`/description meta pair. Deliberately
NOT ported: RustyNES's touch-controls overlay, PWA manifest/service worker, browser-Lua panel,
and `?settings=` share-link — none of those features exist in RustySNES today (no touch input
handling, no wasm Lua backend, no config-to-URL serialization), so faking their UI would be the
same "claims support that doesn't exist" anti-pattern this project avoids everywhere else.

## The `full` build (`v1.0.0`)

`cargo full-build` / `cargo full-run <rom>` (aliases in `.cargo/config.toml`) build/run the most
fully-featured NATIVE binary in one command, activating `rustysnes-frontend`'s `full` feature —
ported from RustyNES's own identical convention. `full` aggregates every native opt-in feature
(`debug-hooks`, `scripting`, `cheats`, `netplay`, `retroachievements`, `hd-pack`) on top of
`default` (cargo merges the two automatically, so `full` doesn't re-list `wasm-winit`/`help-tui`).
Purely additive: the plain `cargo build`/`cargo run` default is unchanged.

`emu-thread` is deliberately excluded from `full` — it isn't feature-complete yet (see its own
Cargo.toml comment), and combining it with `scripting` specifically fails to compile under
`-D warnings` today (the synchronous-path-only input/movie/script helpers become genuinely
unreachable dead code once `emu-thread`'s separate loop takes over frame production). Including
it in `full` would make the "maximal build" simply not build.

`full-run`'s alias ends in `--`, so every trailing argument (the ROM path) forwards to the
emulator binary rather than being consumed by Cargo itself; `full-build` takes no binary args, so
it has no trailing `--`. CI tests `--features full` directly (`.github/workflows/ci.yml`'s `lint`
and `full-test` jobs) rather than re-listing the flag combo, so the tested combo and `full`'s own
definition can never silently drift apart.

## Reuse posture

Reuse the egui shell, the audio ring, the pacing matrix, and the debugger-panel scaffolding
from the RustyNES frontend; SNES-specific work is the second CPU/APU panel, the Mode-7 / HDMA
debug views, and the coprocessor status panel.

## Debugger overlay (`v0.8.0 "Instrumentation"`, T-81-001)

`ui_shell.rs`'s debugger window's 5 panels (65C816 / PPU1+2 / SPC700+S-DSP / Cart / Watch) render a
`DebugSnapshot` the app copies out under the same brief lock `ShellInfo` already uses — CPU
registers/flags, key PPU registers + the dot/scanline timeline + a scrollable VRAM window + full
CGRAM, SPC700 PC/halt state + all 8 S-DSP voices' key registers, and the active board name.
Gated behind the `debug-hooks` feature (default off) at the menu-entry level: without it,
`debugger_open` can never become `true`, so the app never builds a snapshot and the default
build's emulation output is unaffected.

**Disassembly + PC breakpoints + step controls (`v0.9.0`, T-81-001 PR B):** the 65C816 panel's
`docs/frontend.md`-tracked follow-up, now landed. Entirely frontend-side (`emu.rs`) — no new
`rustysnes-core` API beyond one addition, [`Bus::peek`](#bus-peek), needed because the debugger's
own disassembly reads must never perturb the open-bus latch or trip watchpoints the way the live
`CpuBus::read24` a real CPU access uses would. `EmuCore::disassembly_window` walks
`rustysnes_cpu::disasm::disassemble_one` forward from PC (a linear byte-walk, not flow-tracing,
tracking `REP`/`SEP` along the way so the `M`/`X` widths used for later instructions' operand
lengths stay correct across a width change — the one thing that matters for decoding a
straight-line stream correctly). PC breakpoints (`EmuCore::set_breakpoints`, re-synced every
frame like cheats/watchpoints) are checked once per instruction boundary via
`System::step_instruction()` — a real behavior change to `EmuCore::run_frame` only when at least
one breakpoint is armed (an empty list takes the exact prior `System::run_frame()` fast path, so
the default build's determinism/output is untouched). Step Into (`EmuCore::step_into`) and Step
Over (`EmuCore::step_over` — runs a `JSR`/`JSL` to completion via the disassembler's own mnemonic
check, bounded by `MAX_STEP_OVER_INSTRUCTIONS` so an infinite/self-modifying subroutine can't hang
the debugger) both only act while `EmuCore::is_paused()`.

### `Bus::peek`

A new, genuinely side-effect-free read added to `rustysnes-core` specifically for this: unlike
`CpuBus::read24`, it never touches the open-bus latch, never checks watchpoints, and never
triggers an I/O register's own read side effect (VRAM auto-increment, NMI-flag-clear, the H/V
latch, …). Real 65C816 code only ever executes from WRAM or cart ROM/RAM space, so it only
special-cases those two regions (mirroring `Bus::peek_wram`'s existing "not for register space"
posture); any other address returns `0` rather than reaching into a register's live side effects,
which is fine since real code never lives there anyway.

**Watch panel (`v0.8.0 "Community"`, T-81-001b):** 65C816 read/write watchpoints. Needed a new
`debug-hooks` feature on `rustysnes-core` itself (previously the flag only existed as this
frontend's own UI gate) plus a `Bus`-level hook: `rustysnes_core::watchpoint::WatchpointState`,
checked in `CpuBus::read24`/`write24` (an `is_empty()` fast path keeps the hot path free when
nothing is armed), recording up to 256 hits per poll (a ring, oldest dropped first). The frontend
mirrors the existing `cheats` feature's architecture exactly: `watchpoints.rs`'s `sync` installs
the armed `WatchpointEntry` list into the `Bus` once per real frame (`app.rs`'s drive loop, same
cadence cheats already use), and `EmuCore::debug_snapshot` drains recorded hits into
`DebugSnapshot::watchpoint_hits` each poll. The Watch panel itself is a hex address entry + R/W/RW
kind picker + Add button, the armed list with per-row Remove buttons, and a scrollable hit log
(`pc`/`R`or`W`/address/value per hit). `WatchpointEntry`/`WatchHit`/`WatchpointKind`
(`debug_snapshot.rs`) are deliberately NOT `rustysnes_core::watchpoint`'s own types reused
directly — `DebugSnapshot` itself stays unconditionally compiled (see that struct's own doc), so
its fields can't depend on a type that only exists when core's `debug-hooks` is on.

## Scripting + TAS movies (`v0.8.0 "Instrumentation"`, T-81-002)

A Tools menu (native only, `#[cfg(all(feature = "scripting", not(target_arch = "wasm32")))]`)
exposes Load Script, Start/Stop Movie Recording, and Load & Play / Stop Movie Playback.
`ScriptEngine` (`rustysnes-script`) wraps a sandboxed `mlua` 5.4 VM: `emu.read`/`emu.write`
(WRAM only, bound via `Lua::scope` for the duration of one `on_frame` call so the `&mut Bus`
borrow never escapes the persistent Lua state) and `emu.onFrame(fn)`. TAS movies
(`rustysnes_core::movie`, no_std, no Lua coupling) record a deterministic `p1`/`p2` input stream
per frame plus a determinism seed + ROM SHA-256 + start point (power-on or an embedded
save-state); `MoviePlayer::next_frame()` returns pure data rather than writing `Bus::set_joypad`
directly, since `EmuCore::run_frame()` already re-applies its own retained pad state every call —
the frontend applies a movie's frame through `EmuCore::set_pad` instead, in `Active::render`'s
per-frame drive loop (`apply_frame_input`). While a movie is recording or playing,
`ScriptEngine::set_writes_locked` makes `emu.write` a silent no-op, so a loaded script can never
perturb a run it doesn't own. `rustysnes-script` is an optional native-only dependency
(`dep:rustysnes-script`, gated out of the wasm32 dependency graph entirely); with `scripting`
off, none of this compiles in and the default build is unaffected.

## Rollback netplay (`v0.8.0 "Community"`, T-82-002)

A Tools → Netplay… window (native/UDP only, `#[cfg(all(feature = "netplay", not(target_arch =
"wasm32")))]`) takes a local `host:port`, a peer `host:port`, and a P1/P2 slot, and dispatches
`MenuAction::ConnectNetplay` (the actual socket bind/connect happens in `App::dispatch_actions`,
never inside the egui pass). `rustysnes-netplay::RollbackSession` — ported from RustyNES's own
`RollbackSession` shape, scoped to 2 players since the SNES core has no multitap emulation —
drives `rustysnes_core::System` directly, not `EmuCore`: `Active::render`'s per-frame loop checks
`NetplayState::is_connected()` first and, when true, calls `NetplayState::drive` (which calls
`RollbackSession::advance` on the `System`, then `EmuCore::present_current_frame` to decode the
framebuffer/drain audio from whatever the session settled on) via an early `continue` that skips
the entire single-player `apply_frame_input`/cheats/rewind/script/`run_frame` path for that
iteration — netplay's own drive loop, verified independent of `emu-thread` (`docs/adr/0004`'s
determinism contract requires exactly one thing ever drive a given `System`). A dropped
`NetMessage::Input` packet is resent every `advance()` call until the remote peer's cumulative
ack catches up. **Known limitation, shared with rollback netplay generally**: a rollback event
may audibly glitch (audio already sent to the output device during a since-corrected
misprediction can't be "unplayed") even though video always reflects the corrected state
cleanly. `rustysnes-netplay` is an optional native-only dependency (`dep:rustysnes-netplay`,
gated out of the wasm32 dependency graph); with `netplay` off, none of this compiles in and the
default build is unaffected. The crate's `WebRtcTransport` (wasm32) is itself complete and
clippy-verified against the real `web_sys` API, but frontend SDP-negotiation UI to actually use
it in-browser is a separate, not-yet-landed scope.

## RetroAchievements (`v0.8.0 "Community"`, T-82-003)

A Tools → RetroAchievements… window (native-only, `#[cfg(all(feature = "retroachievements",
not(target_arch = "wasm32")))]`) takes a username/password and dispatches
`MenuAction::LoginCheevos`; `App::dispatch_actions` clears the password field from `ShellState`
immediately after handing it to `CheevosState::login` (don't linger a plaintext credential in
memory longer than the call needs it). `CheevosState`
(`crates/rustysnes-frontend/src/cheevos.rs`) owns a `rustysnes_cheevos::RaClient`, created lazily
on first login attempt — nothing allocates or spawns the crate's HTTP worker thread until a user
actually opens the window and logs in. Login is asynchronous: the `rc_client` completion fires
from inside `RaClient::poll_http_completions` on whatever thread calls it (here, the render
thread), and since the completion closure must be `'static` it can't hold `&mut CheevosState`
directly — it writes into a shared `Rc<RefCell<Option<Result<...>>>>` slot instead, which
`CheevosState::poll` (called once per real frame, same cadence as `NetplayState::drive`) drains
on the main thread to update `user`/`login_error`.

`CheevosState::do_frame` runs once per emulated frame (inside `Active::render`'s per-frame
catch-up loop, right after `EmuCore::run_frame`), reading WRAM through `Bus::peek_wram` — the
same non-intrusive accessor the debugger overlay and Lua scripting integrations already use, no
new mutation path. `RaClient::take_events`' `AchievementTriggered` events surface as status-bar
toast messages via `CheevosState::poll`'s return value. **Honest scope notes**: not wired into
the netplay `drive` path (a `RollbackSession`-driven `System` and achievement tracking
interacting — e.g. resimulation re-triggering `rc_client` frames — is a separate, deferred
concern, noted at the `do_frame` call site); no leaderboard/rich-presence UI panel yet (`RaClient`
already exposes `leaderboard_list`/`rich_presence`, just not consumed by any window). SRAM-backed
achievement sets aren't supported — `rustysnes_cheevos::ra_addr_to_snes` only maps the SNES's 128
KiB WRAM (`docs/adr/0003`-style honest scope cut, documented in the crate itself). With
`retroachievements` off, `rustysnes-cheevos` never enters the frontend's dependency graph
(`dep:rustysnes-cheevos`) and the default build is unaffected.

## Open questions

- ~~Whether the second-CPU (SA-1 / Super FX) state warrants its own debugger panel from day one
  or a Phase 8 add~~ — **resolved, `v0.8.0`:** yes, from day one. The Cart panel shows SA-1's
  second-CPU registers (`System::sa1_regs`) or the Super FX/GSU register file
  (`Board::debug_gsu_state`) when the loaded cart uses either.
