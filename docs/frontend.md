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
- On native, the emulator runs on a **dedicated thread** (communicating via an
  `Arc<Mutex<EmuCore>>` handle + a lock-free `SharedInput`); the winit thread only does UI +
  present.

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
  (`emu.rs`) wrap it for the frontend, additionally re-rendering the framebuffer and clearing the
  audio FIFO on load (a state load jumps time discontinuously). Emulation → Save State / Load
  State drives a single quick-save slot held in `Active::quick_save`.
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
