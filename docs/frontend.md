# Frontend — RustySNES

**References:** `docs/architecture.md` §6; `ref-docs/research-report.md` "External
dependencies"; `docs/adr/0004` (the determinism boundary).

## Purpose

`rustysnes-frontend` is the desktop + wasm shell: **winit + wgpu + cpal + egui**, pure Rust
and permissive (mirrors RustyNES). It is an **always-on egui shell, not a bare window** —
egui runs every frame.

**Status (Phase 5): playable native.** A real commercial ROM boots in a window with picture
(PPU BGR555 → RGBA8, aspect-correct 4:3 sub-rect letterbox blit), sound (S-DSP 32 kHz FIFO →
producer-side DRC-paced linear resampler → lock-free ring → cpal stereo), and control (keyboard →
`Bus::set_joypad`). **Correction (`v1.20.0`):** this line previously also claimed "gilrs gamepad" —
found false while scoping the Peripherals fix below: `gilrs::Gilrs` is never actually instantiated
anywhere in this crate, so controller port 1 is keyboard-only today; see "Peripherals" further
down for the honest disposition and why it's a genuinely separate, larger fix than it looks.
ROM load auto-resolves coprocessor firmware + `.srm` SRAM;
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
  instead of an independent, uncontrollable pacing loop. Post-`v1.3.0`, three more items landed:
  cheats/watchpoints/breakpoints/port2-peripheral/voice-mutes now re-sync from the threaded
  build too (a genuinely mechanical port, since they only need to land in the shared
  `Arc<Mutex<EmuCore>>` before the thread's next `run_frame()`, not run on the thread itself, so
  `render`'s `emu-thread` block re-syncs them once per present under the same brief lock it
  already holds for the control-block sync); **run-ahead** (`crate::rewind::step_with_run_ahead`
  called unconditionally from `drive_one`, its own `frames == 0` case matching plain
  `run_frame()` — the peeked `(bytes, dims)` pair now travels through `PresentBuffer` together,
  since a peeked frame's dims can differ from `EmuCore::fb_dims()`'s current-state reading
  across a hi-res-mode-toggle-mid-peek edge case); and **netplay-aware pause**
  (`EmuControl::netplay_paused`, ported from RustyNES's own `EmuControl` near-verbatim — the emu
  thread idles while a session is connected, and `NetplayState::drive` — previously unreachable
  at all under `emu-thread`, since it lived inside the synchronous-only production loop — is now
  driven once per present from `render`'s `emu-thread` block instead).
  **Intentionally NOT ported, matching RustyNES's own mature `emu_thread.rs` precedent** (RustyNES
  itself keeps all three of these on its own winit thread too, confirmed by reading it directly —
  not a gap this project is behind on): TAS movie apply/record, Lua script pump, and
  `RetroAchievements` per-frame drive. `mlua`'s Lua state isn't `Send`; movie record/playback and
  `RetroAchievements`' `rc_client` cooldown tracking both need per-produced-frame cadence with no
  thread-safe handle today (`Active::movie`/`script`/`cheevos` are plain winit-thread-owned
  fields); rewind *recording* is the same story (`Active::rewind` isn't `EmuCore`-owned the way
  RustyNES's own rewind buffer is — RustyNES doesn't port rewind recording to its thread either).
  See `crates/rustysnes-frontend/Cargo.toml`'s `emu-thread` feature comment and `emu_thread.rs`'s
  own module doc for the full detail. Post-`v1.3.0`'s netplay/run-ahead port is verified via the
  unit suite (`emu-thread,netplay` is now a dedicated CI combo, both clippy- and test-gated,
  closing a real prior gap where `emu-thread` was never actually gated in CI at all) and the full
  clippy/fmt/doc matrix; a real headless launch specifically exercising a live netplay session
  under `emu-thread` has NOT been re-verified this pass (this sandbox's headless GUI automation
  is unreliable regardless of feature combo — see the project's own recorded finding) and is
  flagged here as an honest verification gap, not silently claimed. Stays opt-in rather than
  default (its one remaining incompatibility, `emu-thread`+`scripting`, is permanent — see
  `full`'s own Cargo.toml comment).

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

## Theme (`v1.0.0`; two accessibility variants in `v1.13.0 "Vantage"`)

`config.theme` (`crate::config::AppTheme`: `Light` / `Dark` (default) / `System` / `HighContrast`
/ `Colorblind`) selects the egui `Visuals` for the whole shell (menu bar, status bar, all
windows), set via Settings → System. `ui_shell::apply_theme` performs the actual
`ctx.set_visuals` call; `System` reads `egui::Context::system_theme()` and falls back to `Dark`
when the windowing system reports none. `Active::applied_theme` tracks what's currently live so
`App::render` only re-themes on an actual change (the same guard `applied_present_mode` already
uses for the Settings → Video present-mode toggle), applied once explicitly at `egui::Context`
construction time so the configured theme is live from the very first frame, not just after the
user opens Settings.

`v1.13.0` adds two accessibility-oriented variants, both additive (appended after the original
three so an existing `config.toml` keeps deserializing unchanged, matching every prior additive
enum growth in this project):

- **`AppTheme::HighContrast`** (`ui_shell::high_contrast_visuals`) — starts from the stock dark
  theme and pushes every foreground/background pair to the extremes (near-black backgrounds,
  near-white text, a bright cyan selection accent, thicker opaque widget strokes), clearing WCAG
  2.1 AA (4.5:1) — most clear AAA (7:1) — for normal-size text.
- **`AppTheme::Colorblind`** (`ui_shell::colorblind_visuals`) — the stock dark theme with its
  interactive accents (selection, hover, hyperlinks) swapped to the Okabe-Ito palette, chosen to
  stay mutually distinguishable under the most common (red-green) forms of color-vision
  deficiency.

Both are regression-tested (`ui_shell::tests::high_contrast_visuals_diverges_from_stock_dark`,
`...colorblind_visuals_diverges_from_stock_dark_and_uses_okabe_ito`,
`...apply_theme_handles_every_variant`) rather than only visually spot-checked — a builder that
forgot to override any `Visuals` field would otherwise silently ship a theme indistinguishable
from `Dark`.

**Keyboard-only navigation — honestly scoped as a checklist, not a code change this release.**
`v1.13.0`'s originally-planned "keyboard-only-navigation audit across every UI surface added
since `v1.7.0`" was investigated and found to be a poor fit for a single crisp code change: this
project has no custom focus/tab-order management anywhere in `ui_shell.rs` — every panel and
window relies entirely on egui's own default `Tab`/`Shift+Tab` traversal, which is neither
broken nor specifically hardened here. What actually exists today: the global hotkey table
(`app.rs`'s `KeyboardInput` handler) is correctly suppressed while an egui widget holds keyboard
focus (`egui::Context::egui_wants_keyboard_input`, see "Global hotkeys" below), so typing in a
text field never double-fires a hotkey — but no one has walked every window/panel added across
`v1.7.0`-`v1.12.0` (the debugger's CPU/PPU/APU/coprocessor/trace/memory-compare/docs panels, the
Settings tabs, the HD-pack pack selector, the RetroAchievements/netplay/cheats windows) confirming
egui's default Tab order visits every interactive control in a sane sequence, or that no widget
is keyboard-unreachable. **Deferred, not silently dropped**: this is real accessibility work worth
doing, but it is a manual-walkthrough audit task, not a bug with a discrete fix — tracking it here
as an explicit open item rather than converting it into a hollow "audit passed" claim with no
teeth.

## Presentation post-filters (`v1.2.0`; a third filter + a shader-source crate extraction in `v1.12.0 "Refraction"`)

`config.video.filter` (`crate::config::PostFilter`: `None` (default) / `Crt` / `Hqx` / `Xbrz`)
selects a post-process pass applied after the plain nearest-sample framebuffer blit, set via
Settings → Video (a radio row + per-filter strength sliders) or the View → Post-filter submenu.

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
- **`PostFilter::Xbrz`** (`v1.12.0`) — the same 2×2-corner blend `Hqx` does, but the edge-detection
  decision is additionally gated by a wider 4×4-neighborhood read: one extra texel further out
  along each candidate diagonal must also agree with the near corner's trend before the full
  diagonal pull is committed, otherwise the pull strength is scaled down toward plain bilinear.
  This is an xBRZ-**style** approximation of that algorithm's "look past the immediate corner
  before rounding it" philosophy, distilled into one extra context sample per diagonal — not a
  literal port of xBRZ's real multi-pass, 2×/3×/4×/5× rule-table algorithm. One strength slider
  (`config.video.xbrz_strength`, default `0.6`).
- **All three filters share** the exact same clip-space letterbox convention `Gfx::blit`'s own
  vertex shader uses (position-scale, not UV-space cropping) — `Gfx::letterbox_scale` was
  extracted out of `blit`'s inline math specifically so `blit` and the filter passes stay
  pixel-aligned, a pure, behavior-preserving refactor (verified by
  `letterbox_scale_matches_known_cases`, a hand-computed regression test for windows
  wider-than / narrower-than / exactly the 4:3 SNES aspect).
- **Architecture**: `Gfx` builds all three filter pipelines unconditionally at init
  (`Gfx::new_async`, cheap — three small pipelines) so switching the Settings radio needs no
  pipeline creation/reallocation; `Gfx::present` selects between `blit`/`crt`/`hqx`/`xbrz` per
  frame based on the live config. `v1.12.0` moved the shader sources
  (`BLIT_WGSL`/`CRT_WGSL`/`HQX_WGSL`, byte-identical; plus the new `XBRZ_WGSL`) out of `gfx.rs` and
  into a new `rustysnes-gfx-shaders` crate — reversing this doc's own prior "no near-term second
  consumer" call, now that Mobile Phase 1 (`v1.14.0 "Foundry"`) is a concrete, planned second
  consumer that needs the shader strings without pulling in this crate's winit/egui/cpal
  dependency graph.
- **Verified**: `naga` WGSL-parse+validate tests for all four shaders (`blit_wgsl_validates`/
  `crt_wgsl_validates`/`hqx_wgsl_validates`/`xbrz_wgsl_validates` in `gfx.rs`, same machinery
  `wgpu` itself uses at runtime). `XBRZ_WGSL` was additionally exercised through a real (no-window)
  wgpu render pipeline — adapter/device request, shader-module + pipeline creation, an actual draw
  against a 4×4 source texture with a clean diagonal edge, and a GPU readback — on a real
  discrete-GPU Vulkan adapter; the output showed a genuine blended gradient band along the
  diagonal (not just uniform `0`/`255`), confirming the corner-blend logic actually executes and
  behaves sensibly, not merely that the shader parses statically. This is a standalone, no-window
  diagnostic (the project's own `wgpu-headless-repro-gotcha` note: the real winit-based
  `rustysnes` binary hangs, not crashes, when launched headlessly under Xvfb in this environment,
  so a real windowed `xvfb-run` smoke test — as `v1.2.0`'s original `Crt`/`Hqx` verification used —
  was not repeated here) and was not added to the repo as a permanent test. No golden-screenshot
  regression harness exists in this project today (the existing `commercial_screenshots.rs`
  captures the raw core framebuffer directly, entirely upstream of this wgpu render path) — the
  `None`-path-unchanged guarantee here is a structural one (the exact same `blit` function, not a
  re-derived equivalent), not a pixel-diff proof. A real windowed run (all four filters, on a real
  display) is still worth the maintainer confirming on their own machine before release.
- **Not built** (documented scope cuts, not silent gaps — unrevisited from `v1.2.0`'s original
  call, not a `v1.12.0` finding): RustyNES's NTSC composite-signal simulation and RetroArch
  `.slangp`/`.cgp` shader-preset import both remain explicitly out of scope.

### Hide Overscan (`v1.20.0`)

View → Hide Overscan crops the trailing "overscan" scanlines a real 4:3 CRT wouldn't reliably
show. This is distinct from every other post-filter above — it's a scanline COUNT crop, not a
pixel-shader effect, and it's tied to a real SNES hardware register: `SETINI` (`rustysnes_ppu`)
lets a game extend the standard 224-line display to 239 lines; `app.rs`'s `crop_overscan` crops
exactly that extra 15-line extension back off, once per frame, after every other buffer transform
(HD-pack compositing, run-ahead, the `emu-thread` build's `PresentBuffer` handoff) has already
settled on the bytes actually being presented. Crops a FRACTION (`15/239`) of the current height
rather than a fixed `224` pixel count, so it stays exact under an HD-pack integer upscale too
(`239 * scale * 15 / 239` reduces to exactly `15 * scale`, no rounding, for any integer `scale`).
Presentation-only — the deterministic core's own framebuffer is untouched, matching every other
filter's determinism-boundary posture (`docs/adr/0004`). Additive, `config.video.hide_overscan`
defaults to `false` — byte-identical presentation to every prior release when unchanged. 3 real
unit tests (`app.rs`'s `overscan_tests` module) cover native resolution, an HD-pack-scaled
resolution, and that the kept (leading) bytes are untouched, not re-derived.

### Per-side overscan crop (`v1.25.0`)

`config.video.overscan` crops an independent number of SNES pixels from each of the four sides, the
fine-grained companion to the all-or-nothing Hide Overscan above (both apply; this one second).
`app.rs`'s `apply_overscan` scales the request by the frame's integer upscale factor, so a configured
"8 pixels" means 8 *SNES* pixels whether or not an HD pack has upscaled the buffer — a fixed byte
count would crop a quarter of the picture at 4x. `Overscan::clamped` guarantees at least a 16x16
image survives however absurd the config file is, because a zero-sized texture upload is a wgpu
validation error rather than a merely-ugly result. Row-only crops take a `truncate`/`copy_within` fast
path; a column crop necessarily moves every row and compacts in place, so neither path allocates a
second buffer. Presentation-only, all zero by default.

### Aspect ratio and integer scaling (`v1.25.0`)

The letterbox target was a hardcoded `TARGET_ASPECT` constant; `config.video.aspect` now selects
between **4:3** (the television's shape, the default and unchanged), **8:7** (the pixel aspect the dot
clock implies — close to 4:3 at 224 lines and visibly different at PAL's 239), and **1:1** (square
pixels, no correction). `AspectMode::ratio` keys off the *measured* framebuffer size, never the region
bit, so a hi-res or overscan frame corrects by what is actually on screen.

`config.video.integer_scale` **now has an effect**: the flag and two Settings checkboxes had existed
since early on, but `letterbox_scale` never read the value. `gfx::letterbox_scale_for` quantises the
fit to a whole multiple of the framebuffer's scanline count — which is what keeps a scanline a uniform
height instead of alternating between one and two output pixels, the shimmer non-integer scaling
causes on pixel art. Only the vertical axis is quantised (the horizontal is already being stretched by
aspect correction, so there is no whole-pixel grid to land on there), and if not even 1x fits the
continuous fit is kept, since an image cropped by the window edge is worse than a slightly soft one.
Both knobs live on `Gfx` as state rather than as `letterbox_scale` parameters because
`crate::peripherals` calls that same method through a shared `&Gfx` to map host pointer coordinates
into SNES pixel space: if the two disagreed about the display shape, Super Scope aim would land
somewhere the picture is not.

The previous test-only hand-copied mirror of the letterbox formula was deleted in favour of the tests
calling the real function — a formula duplicated for testability is a formula that can silently
diverge from the one that ships, which is the very thing the test exists to prevent.

## The multi-pass shader stack (`v1.25.0`, T-FP-D)

`Gfx::present` took a `PostFilter` enum plus four hardcoded `f32` arguments, applied exactly **one**
filter, and gained an argument every time a filter gained a knob. That shape cannot express what
presentation shaders are: an ordered chain, each pass rendering into a target the next one samples,
each with its own scale, filtering, and parameter set.

`crate::shader_pass` describes such a chain; `crate::shader_runtime` and `Gfx::present_chain` run it.
The per-pass fields are named after `RetroArch`'s `.slangp` keys (`scale_type`, `filter_linear`,
`wrap_mode`, `float_framebuffer`, `mipmap_input`, `alias`, `frame_count_mod`) so T-FP-E's preset
parser maps onto this with no translation layer.

**The `v1.2.0` `PostFilter` path is untouched.** `video.stack` defaults to `Off`, which builds an
*empty* chain, and `present_chain` falls straight through to the plain blit — so an existing
`config.toml` renders byte-identically and the two systems never both apply.

### Parameters are declared by the shader

A `#pragma parameter` declares a named, ranged, defaulted knob. Modelling those as Rust struct
fields would mean the Rust side has to know every shader's knobs at compile time, which is exactly
what makes a shader stack unable to load a shader it was not built with. Here they are a
name-indexed list packed into one uniform, the Settings sliders are **generated from the pass**, and
edits are stored as `"<chain>.<param>"` overrides — so adding a knob touches no UI or config code,
and a saved value for a knob a shader no longer declares is ignored rather than landing in the wrong
slot.

### The uniform layout, declared once

The prelude (`shader_runtime::PRELUDE`) is prepended to every pass, so the binding layout exists in
exactly one place. It supplies the bindings, `source_size()`/`output_size()`/`frame_count()`/
`param(i)`, and the fullscreen-triangle vertex stage; a pass supplies only its `fs_main`.

### The input is a sub-rect, and every pass must know it

Pass 0's input is the emulator's **backing** framebuffer texture, which is allocated once at the
maximum size and holds the live 256x224 (or hi-res) image in its **top-left corner**. Sampling
`0..1` across it therefore stretches a mostly-unwritten allocation over the screen. The uniform
block carries the live fraction (`source_rect()`), the shared vertex stage applies it to `in.uv`,
and every later pass gets `(1, 1)` because an intermediate is sized exactly to its own image.

That one fact generates four prelude helpers, and a pass that ignores them is subtly wrong rather
than obviously broken:

| helper | for |
|---|---|
| `image_uv(uv)` / `tex_uv(uv)` | effects centred on the **picture** — barrel distortion, a vignette |
| `texel()` | a neighbouring **source** pixel. NOT `1 / source_size()`, which is an image-space step |
| `out_texel()` | a neighbouring **output** pixel, for effects sized to the destination |
| `clamp_uv(uv)` | any offset tap. The sampler's own `ClampToEdge` clamps to the edge of the whole *texture*, so an unclamped tap past the picture reads never-written black and draws a dark rim down the right and bottom edges |

Two layout facts that do not error when wrong, they just read the wrong data:

- `Uniforms` must be 16-byte aligned (WGSL's uniform address space). A `const` assertion enforces it.
- Parameters are packed **four to a `vec4`**, because a `array<f32, N>` in the uniform address space
  has a 16-byte *stride per element* in WGSL — a tight Rust-side array against that declaration
  misaligns silently.

### Rebuild discipline

Pipelines and intermediate targets are rebuilt only when the chain's identity or a target size
changes, tracked by a cheap signature string. Compiling a pipeline and allocating a texture every
present is the difference between a shader stack being usable and being a slideshow. A slider move
writes a uniform; it does not rebuild.

**Everything a built pass captures must appear in that signature**, because anything omitted is a
stale binding waiting to happen — and the failure lands at *draw* time as a wgpu validation error,
not at build time. It therefore covers: the target format; the **backing texture's dimensions**,
which are its identity here since `ensure_texture_capacity` recreates it exactly when one of them
grows (an HD pack, a hi-res mode) while pass 0's bind group still holds a view of the old one; a
CRC of each pass's source rather than its length, since a length collision would keep silently
running the previous pipeline; and `filter_linear`/`wrap_mode`, which `BuiltPass::build` bakes into
a sampler it captures.

A pass's pipeline format matches **its own target**, not the surface: only the last pass renders to
the swapchain. Using the surface format throughout is a validation error at *draw* time, so it would
surface as a mid-frame crash rather than a failed build.

### Failure is named, never silent

Each pass's WGSL is validated with naga *before* `create_shader_module`, because wgpu reports a
shader error by panicking the device — which for a user-supplied shader would mean a typo takes down
the emulator. A pass that fails to build stops the chain, `present_chain` falls back to the plain
blit, and Settings names the failing pass with the compiler's message. `ShaderChain::unsupported`
carries passes a *preset* could not produce at all, which is what makes T-FP-E's GLSL bridge able to
be best-effort rather than all-or-nothing.

### The richer CRT and NTSC passes

`CRT_STACK_WGSL` takes six knobs: scanlines, aperture mask, curvature, beam shape, glow, vignette.
Curvature samples outside the image at the corners and returns **black** there rather than clamping —
a clamped edge texel smeared around a curved border is the classic "stretched corner" artefact.

`NTSC_STACK_WGSL` is an **RGB-domain** approximation in the [LMP88959](https://github.com/LMP88959/NTSC-CRT)
style: horizontal chroma bandwidth reduction, luma/chroma cross-talk, colour fringing, and a
frame-advancing dot crawl. It works in RGB deliberately — the full encode/decode technique needs a
palette-index framebuffer, which is a *NES* property; the SNES PPU emits 15-bit BGR555 direct
colour, so there is no index to export and no core change to make. What is reproducible in RGB is
the visible half, and claiming more would be the dishonest version.

`NtscCrt` runs NTSC **then** CRT: composite artefacts happen in the signal, scanlines and the mask
at the phosphor. Reversing them would smear the mask itself.

### Offscreen golden tests (`gfx_test_support`, `shader_golden`)

Before this, `gfx.rs`'s tests only asked naga to *validate* the WGSL — no device was created and no
pixel produced, so a shader that compiled and rendered the wrong thing passed. `gfx_test_support`
renders to an **offscreen** texture (no window, no surface — the windowed path hangs under Xvfb here,
and CI has no GPU at all), reads it back, and hashes it. It returns `None` with a printed reason when
no adapter exists, so CI self-skips visibly rather than quietly passing.

`shader_golden` asserts **properties**, not committed hashes:

- every knob at zero is a **bit-exact** pass-through (the same contract `crate::eq` holds for audio,
  and what makes a pass safe to leave in the chain);
- a knob turned up changes the image *in the direction it claims* — chroma bleed measurably reduces
  horizontal colour variation, scanlines darken alternate rows below their source value, curvature
  blacks out corners while leaving the centre untouched, the mask keeps exactly one channel per
  column;
- dot crawl differs between frames and **cycles** with its period;
- the same render hashes identically, so the stability a golden depends on is itself verified.

That is a stronger statement than a committed hash, which only says "the same as last time" —
including the last time it was wrong.

## `.slangp` presets and the GLSL bridge (`v1.25.0`, T-FP-E)

`crate::slang_preset` parses a `RetroArch` preset into the per-pass description T-FP-D's stack
already uses — which is why that ticket named its fields after these keys. `crate::glsl_bridge`
translates each referenced `.slang` shader into WGSL via naga's GLSL frontend.

**Best-effort by construction.** naga's GLSL frontend covers a *subset* of `#version 450`, so a
preset that only partly translates is the normal case, not an error case. Every failure becomes a
named `ShaderChain::unsupported` entry carrying the compiler's own message; the remaining passes
still run, and the status line says how many were dropped. A preset with **no** translatable pass is
not adopted at all — replacing a working picture with a pass-through would look like the preset "did
nothing".

### What the GLSL frontend actually accepts — measured, not assumed

Three rewrites exist because naga rejected the real thing during development. Each was found by
running a shader through it, not by reading documentation:

| naga rejects | rewrite | why it is sound here |
|---|---|---|
| `set = N` in a `layout(...)` list (`NotImplemented("variable qualifier")`) | dropped | the stack binds everything in group 0; there is no second descriptor set to distinguish |
| `uniform sampler2D X` (same error) | split into `texture2D X_tex` + `sampler X_smp`, with each use rewritten to `sampler2D(X_tex, X_smp)` | Vulkan's separated form, which naga does accept — verified round-tripping to WGSL |
| `#pragma parameter` (`PreprocessorError`) | stripped before parsing; the declarations are read separately | it is metadata, not code |

The middle one is load-bearing: **every** `.slang` shader uses the combined form, because
`RetroArch`'s spec mandates it. Without that rewrite the bridge translates exactly zero real
shaders — which was its state before the rewrite existed.

Push constants are also rewritten to a bound uniform block, because wgpu exposes push constants only
on native and not at all on WebGL: a shader using them would work in the desktop build and fail in
the browser one, which is worse than not supporting it.

Whole-word substitution matters in the sampler rewrite: a substring replace turns `SourceSize` into
`sampler2D(Source_tex, Source_smp)Size` for any shader declaring `sampler2D Source` — and `Source`
and `SourceSize` always co-occur in a `.slang` shader.

### Two lists, not one

`Translated` reports `synthesised` (which `RetroArch` semantics the shader *asked for*) separately
from `rewrites` (what was *changed underneath it*). They answer different questions, and a rendering
difference is only attributable if both are visible. Conflating them was an actual mistake caught by
a test during development.

### Recognised-but-unhonoured keys are recorded

`#reference` (preset inheritance) and `srgb_framebufferN` are parsed and then listed in
`Preset::ignored` with a reason, rather than silently dropped — so a preset that renders differently
from its author's intent can say which key was responsible. Likewise a `textures` entry with no path
and a `parameters` entry with no value.

Paths resolve against the **preset's own directory**, never the working directory: the latter is the
classic way a preset loads for its author and for nobody else.

### Parameters

A pass's knobs come from the shader's own `#pragma parameter` declarations; the preset's
`parameters` list then overrides them **by name**. A preset commonly sets only some of a shader's
knobs, and a positional application would put a value on the wrong one.

An omitted step becomes a hundredth of the declared range, because a zero step makes a slider
unusable.

### What is not done

A translated module carries the **shader's own** binding layout, not the stack's fixed one — the
sampler rewrite reports which names occupy which bindings (`Translated::samplers`) precisely so a
caller can build a matching bind group. Reflecting that into the live `StackState` is the remaining
step for running an arbitrary preset on the GPU; the parse, the translation, and the failure
reporting are complete and tested. Preset **LUT textures** (`textures = …`) are parsed but not yet
uploaded, for the same reason.

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

- **Not yet done**: a user-configurable upscale factor (fixed at 2x for now) — an honestly
  tracked scope cut, see `docs/adr/0010`. `emu-thread`-build compositing landed in
  `v1.10.0 "Atelier"`: `emu_thread::drive_one` now composites an active pack into its own
  `PresentBuffer` publish (both the run-ahead and plain-frame branches), reusing
  `EmuCore::hd_pack_composite_inputs` the same way `app.rs`'s synchronous path already did —
  previously a threaded build with a pack selected silently rendered the native framebuffer.

## Audio mixer, rewind compression, and the run-ahead fast path (`v1.25.0`, T-FP-F)

### Per-voice gain (`Dsp::set_voice_gains`)

The `v1.0.1` debugger already had per-voice **mute**. What it could not do is the thing a mixer is
for — turning one voice down to hear what another is doing — which needs a continuous gain.

The gain applies at the **existing** per-voice-mute site (`apu/src/dsp.rs`'s `voice_output`), which
already drops a voice from both the main mix and the echo send. Everything upstream — envelope, BRR
decode, pitch, `OUTX`/`ENDX`/`ENVX` — is untouched, so a ROM cannot observe a gain change and
adjusting one mid-note resumes exactly where the voice already was.

Three properties, each deliberate:

- **`1.0` is a bit-exact bypass**, checked with an *exact* comparison. A tolerance — which the
  `float_cmp` lint suggests — would make gains near but not at unity silently skip the multiply,
  which is the opposite of the guarantee. Same reasoning as `crate::eq`'s exact flat bypass.
- **Never in a save state.** Like mutes, cheats, and watchpoints, this is host UI state; a save-state
  round-trip must not reset or preserve it as if it were emulated hardware.
- **Bounded at 4x** (`MAX_VOICE_GAIN`). The mix saturates rather than overflowing, so an enormous
  gain does not corrupt anything — it just clips everything else out, which reads as "the emulator
  broke" rather than "that slider is too high". A non-finite or negative value falls back to unity
  rather than silencing or inverting a voice permanently.

`Dsp::voice_taps` reports what each voice contributed to the **most recent sample**. That is the
honest primitive: a meter needs a decay constant, and a decay constant in the DSP would be a UI
concern living in the emulation core. The mixer panel does the smoothing, where the frame rate is
known — rise is instant, fall is exponential, because a symmetric response averages a drum hit down
to nothing between frames.

**Solo wins over mute**, computed in one place (`MixerState::mutes`) so the two buttons cannot
disagree about which is authoritative.

### Rewind compression (`crate::delta`)

Successive save-states are **overwhelmingly identical**: a few hundred bytes of WRAM, some PPU
registers, and the CPU's registers change out of hundreds of kilobytes. Storing them whole spent
almost all of a rewind buffer's memory on identical data.

XOR against the previous state turns that into a buffer that is almost entirely zero, which
run-length encodes to a few dozen bytes. That is a better fit than a general compressor **and needs
no new dependency** — a rewind buffer pulling in a compression crate to store data that is already
99% zeros is paying for the wrong tool.

- **Keyframes every 16 snapshots.** Without them, reaching the oldest state means replaying every
  delta from the start — and the ring evicts from the *front*, so the base a delta chain depends on
  is exactly what disappears first. A keyframe bounds both the replay cost and that dependency.
- **The round-trip is bit-identical.** Anything less is not a rewind: a nearly-right state resumes
  emulation from a machine that never existed, and the divergence surfaces later as an unexplainable
  bug rather than as a failed decompress.
- **Corrupt input is refused, never partially decoded.** A length is checked before allocating, so a
  corrupt run cannot ask for a gigabyte; a varint shift is checked *before* shifting, because
  `1u64 << 70` is a panic in debug and a wrong answer in release.
- A **length change forces a keyframe**: two states of different sizes have no meaningful XOR, and
  delta-ing the common prefix would decode to a truncated state.

### Run-ahead's per-frame allocation (`save_state_into`)

Run-ahead snapshots the machine **every frame**. `EmuCore::save_state` allocated a fresh
hundreds-of-kilobyte `Vec` each time, which this document long recorded as *the* blocker on making
run-ahead default-on.

`System::save_state_into` (and its `SaveWriter::with_buffer`) reuse a caller-owned buffer, clearing
it but **keeping its capacity**. Output is byte-for-byte identical; the only difference is that the
allocation happens once instead of sixty times a second.

#### …was not actually the blocker (`v1.26.0`, measured)

With the allocation gone, the default-on question was **re-measured rather than assumed resolved**:

| | cost |
|---|---:|
| `save_state` (`save_state_cost` bench) | ~119 µs |
| `load_state` | ~285 µs |
| **save/load round trip** | **~0.40 ms** |
| one emulated frame (`headless_frame_steady_state`) | **6.39 ms** |
| NTSC frame budget | 16.64 ms |

The round trip is **2.4% of the frame budget**. It was never the dominant cost, and removing the
allocation — while a real improvement — did not move the decision.

What run-ahead actually costs is the **extra frame of emulation**, which is inherent to the
technique and cannot be optimised away. `frames = 1` needs 13.18 ms of a 16.64 ms budget (**79%**),
leaving ~3.5 ms for present, UI, and audio on a fast development machine; `frames = 2` needs
19.57 ms (**118%**) and cannot hold 60 fps at all.

**So run-ahead stays opt-in.** Defaulting it on would spend most of every frame's budget — and miss
deadlines outright on ordinary hardware — to buy latency the user never asked for. The
`run_ahead.throttle_ms` budget throttle exists to make the feature safe *when a user opts in*, not
to make it safe as a default.

One thing did change as a result: `RunAheadConfig`'s `Default` is now hand-written so
`throttle_ms` defaults to **14 ms (armed)** rather than the derived `0.0` (**disabled**). The
derived default left the safety net off for precisely the user who had just enabled run-ahead and
had no `throttle_ms` line in their `config.toml`. An existing config that spells the field out
keeps whatever it says.

The buffer is owned by the caller for a reason: a local `Vec` inside the run-ahead function would
allocate just the same. The synchronous path keeps one on `Active`; the `emu-thread` build keeps its
own in `emu_thread::run_loop`, because frame production there lives on a different thread — and a
buffer on `Active` for that build would be written by nobody and read by nobody, which is exactly
the inert-feature trap this project has hit before (see the `emu-thread` note in the parity
CHANGELOG entries).

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

## A/V capture, virtual pad, databases, and input macros (`v1.25.0`, T-FP-G1)

### A/V capture (`crate::av_record`, native)

Writes **raw** streams — `.y4m` video and `.wav` audio — and prints the `ffmpeg` command that muxes
them when the recording stops.

Muxing MP4/MKV directly would mean either an `FFmpeg` dependency (a C toolchain on every platform, a
licensing surface, and a build that breaks when the system library moves) or hand-rolling a
container, whose bugs are invisible until someone tries to play the file six months later. Raw
streams are what an emulator can produce *exactly*, and every tool reads them.

Details that matter:

- The Y4M header carries the frame rate as an **exact rational** (`F60099:1000`), not a rounded
  `60:1`. NTSC is 60.0988 Hz, and rounding makes a 20-minute recording drift about two seconds
  against its own audio.
- The WAV header's two size fields are **patched on finish**. A streamed WAV cannot know its length
  up front, and a player reading the placeholder reports a zero-length file — indistinguishable from
  a failed capture.
- A **resolution change mid-recording is refused**, by name. Y4M declares one size for the whole
  stream, so writing a differently-sized frame produces a file whose header lies about its contents.
- **Drift is reported, not corrected** (`sync_drift_samples`). Silently resampling to hide drift
  would make a genuinely broken capture look fine; the number is what says whether it is usable.
- 4:4:4 rather than 4:2:0 chroma: this is a lossless intermediate, and the re-encode is where size
  is meant to be traded for quality.

### Virtual pad (`crate::virtual_pad`)

An on-screen controller, serving both as a touch input surface (the `wasm32` build on a phone has no
keyboard) and as a visual input display for movies and netplay.

**One source of geometry.** `Layout::hit` answers "which button is at this point" from the same
rectangles the renderer draws. A renderer with its own hardcoded boxes plus a separate hit-test
table drift apart the moment either is edited, and the symptom — a button that highlights but does
not press — is maddening. A test asserts no two rectangles overlap, and it *caught a real layout
bug*: two face buttons shared an edge that float rounding made ambiguous.

Buttons carry the existing `input::Button` enum, not a raw mask, so the bit layout stays defined once.
`buttons_for` resolves **all simultaneous touches at once** — holding a direction while tapping a
face button is the normal case, and one-at-a-time resolution would drop one every frame — then
sanitises, because a two-thumb touch genuinely can land Left and Right together.

### Game and Game Genie databases (`crate::game_db`)

CRC32-keyed, because that is what every existing SNES list (No-Intro, the Game Genie code sets) is
keyed by; anything else would mean they cannot be imported. `crc32fast` was already a dependency.

A cart's internal 21-byte title is frequently blank, mojibake, or a development codename — a hash is
not, which is why a canonical title is worth looking up at all.

Parsing is tab- or pipe-separated text (what these lists exist as in the wild), tolerant of junk
lines but **reporting how many it skipped** — the same posture `crate::symbols` takes. A CRC with no
title, or a cheat code with no description, is skipped rather than stored half-formed: a blank name
in the UI is worse than falling back to the cart header.

### Input macros (`crate::input_macros`)

A short recorded pad sequence replayed on a hotkey. Deliberately **not** a TAS movie, and a separate
type for a reason: a movie owns the whole session from a known start point with a seed and a ROM
hash, which is exactly what a macro cannot have. One type with a flag would either weaken the movie's
contract or make the macro claim a guarantee it does not meet.

- Playback is **OR-ed onto live input**, not substituted. Replacing would drop a held direction the
  instant a macro starts — the opposite of what a motion-plus-button macro is for.
- Recording **trims dead frames at both ends**. A hand-started recording always has them, and
  replaying them makes a macro feel laggy for a reason nobody would guess at. An internal gap is
  preserved, since that is part of the timing.
- Recording **stops itself** at 512 frames. A recording left running is the actual failure mode,
  since stopping is a hotkey that is easy to forget.
- Playing an empty slot is a no-op rather than a playback that emits nothing, which would be
  indistinguishable from "playing silence".

## TAStudio (`v1.25.0`, T-FP-G2)

A TAS is written by editing a **timeline**, not by playing it. `crate::tastudio` is that timeline
(`PianoRoll`), the cache that makes seeking into it usable (`greenzone::Greenzone`), and the grid
(`tastudio::panel`).

### Invalidation is the whole correctness story

Emulation is deterministic: a machine state is a function of the initial state and every input up to
it. Change frame 500's input and every cached state from **500 onward** describes a machine that no
longer exists.

So every `PianoRoll` edit **returns** the frame from which cached state is invalid, and the panel
forwards it as `MenuAction::TasInvalidate`. Returning it rather than leaving it implicit is
deliberate: a caller that ignored the invalidation would have to actively discard a value.

It is at-or-after, not after. The state *at* frame N is computed from the input *at* frame N, so
invalidating from N+1 would leave exactly the one stale state about to be resumed from.

### The greenzone reuses the rewind compression

A greenzone of raw save-states is gigabytes. But TAS states have precisely the property
`crate::delta` exploits — successive states differ in a few hundred bytes — so the same XOR-delta +
RLE chain applies unchanged. **This is why T-FP-F was sequenced before T-FP-G2** rather than the
reverse.

States are kept every 30 frames, not every frame: replaying up to 29 frames from a cached state is
microseconds, while storing every frame is the whole problem.

A seek with no cached state at or before the target **says so** rather than appearing to work — the
alternative is replaying from power-on, which looks like a hang. The **first** state is therefore
seeded whatever frame it lands on (`offer_seed`), checkpoint or not: emulation starts at frame 0 and
the first checkpoint is a whole interval later, so without it every seek below the interval had
nothing at or before it and failed outright.

A seek consumes every cached state **after** the one it returns, and keeps that one. The chain is
only readable from its end, so reading the target means popping it — but it is re-stored
immediately, because the caller has just restored *to* it and it is the state most likely to be
wanted again (seek to 60, edit, run, seek to 60). The states beyond it are genuinely gone, which is
correct: the reason to seek backwards in a TAS is to change something, so they are about to be
recomputed. One consequence worth knowing: `seek` is not a destructive iterator, so draining the
greenzone is `invalidate_from`'s job, not a seek loop's.

### A bug this found in the compression

Building the greenzone exposed a real defect in `crate::delta::Chain`, which T-FP-F had already
landed: the chain evicts from the **front**, which is exactly where a delta chain's keyframe base
lives. Evicting a keyframe left every following delta undecodable, so the chain reported N entries
and could restore only the newest few — a capacity of 4 against a keyframe interval of 8 restored
exactly **one** of its four claimed states.

`Chain::evict_front` now reconstructs the new front **while its base still exists** and re-stores it
whole. That costs one reconstruct per eviction and is what makes "the ring holds N states" mean N
*recoverable* states. A test now asserts that property directly across several
capacity/keyframe-interval combinations, including ones where the interval exceeds the capacity.

This mattered for the rewind buffer too, not just the greenzone: it is the same chain.

### Recording and cost

The timeline records played input and offers a state at each checkpoint **only while the TAStudio
window is open**. A greenzone nobody is editing against is pure cost, and without the guard the app
would take a save-state every 30 frames for the whole session.

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

### Resampling, latency, and buffer health (`v1.25.0`)

Three related changes, all in `crate::audio_core` (still console-agnostic and still shared with the
wasm `AudioWorklet` path):

- **Kernel.** `ResampleKernel::Hermite` — a 4-tap Catmull-Rom cubic, matching RustyNES — replaces
  2-point linear interpolation as the default. It is continuous in the first derivative across sample
  boundaries, which removes the aliasing a linear blend leaves on the S-DSP's 32 kHz output. The
  resampler holds a four-sample window (`hist`) so the cubic can see one sample either side of the
  interval it is interpolating, which costs exactly one source sample of delay (~31 µs).
  `ResampleKernel::Linear` remains selectable to reproduce earlier output exactly.
- **Latency as a setpoint.** `config.audio.latency_ms` (default 60, clamped `10..=250`) is now the
  target the DRC servo holds, via `drc_ratio_latency` (error normalised by the target, correction
  clamped to ±1%). The ring is sized at 4x the target rather than a fixed 8192 samples, whose achieved
  latency silently varied with the device's negotiated rate. Latency is therefore what the user asked
  for, and the buffer is merely headroom around it.
- **Health + the refill gate.** `AudioRing` counts `underruns` (a pop with nothing queued) and
  `overrun_dropped` (a push onto a full ring), surfaced in Settings → Audio. More importantly, an
  underrun is not a one-off click: once the consumer catches up to the producer it tends to *stay*
  caught up, emitting a silence sample every callback and turning one dropout into continuous crackle.
  So the ring has a **refill gate** — armed at half the latency target — which feeds silence until the
  buffer has rebuilt, and which an underrun **re-arms**. One short gap replaces the crackle. A paused
  emulator mutes the ring instead, deliberately *not* counting as starvation, or every callback while
  paused would bury the real signal. The gate defaults to disabled (threshold `0`), which is exactly
  the ungated behaviour of every prior release; the app opts in when it opens the device.

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

### Frame pacing (`v1.25.0`, T-FP-B)

`config.video.pacing` (`auto` / `display` / `vrr` / `wallclock`) was, until `v1.25.0`, declared,
serialized, and **read by nothing** — every variant behaved identically. `pacing::resolve` now turns
the request into a `PacingPlan` by checking two things the request cannot know on its own: the
monitor's reported refresh (winit's `Monitor::refresh_rate_millihertz`, `None` when the windowing
system does not expose one) and which present modes the surface actually offers
(`Gfx::present_mode_caps`).

| requested | resolves to |
|---|---|
| `display` | display-sync (`fifo`, vsync blocks) **only if** the refresh is within `DISPLAY_SYNC_TOLERANCE_HZ` (1 Hz) of the region rate — otherwise wall-clock, reported as declined |
| `vrr` | mailbox present **only if** the surface offers it — otherwise wall-clock, reported as declined |
| `wallclock` | self-paced at the region rate; `immediate` present when available, else `fifo` |
| `auto` | display-sync when the panel genuinely matches, then mailbox, then wall-clock |

The tolerance is against the **region** rate, not a hardcoded 60: a PAL core on a 50 Hz panel is a
match. A 60 Hz panel against NTSC's 60.0988 Hz is a 0.0988 Hz mismatch, which is exactly the case
display-sync exists for; a 75 Hz panel would need the emulator to run 25% fast, so `auto` must not
pick it.

`PacingPlan::reason` is shown verbatim in Settings → Video and in the Performance panel. A downgrade
the user cannot see is indistinguishable from a bug, so a declined request always says why.

`video.present_mode` gained an `"auto"` value — the new default — which defers to the plan. An
explicit `"fifo"`/`"mailbox"`/`"immediate"` still wins, so a `config.toml` written by an earlier
release (which always carries an explicit value) behaves exactly as it did.

**Emulation speed is unaffected by any of this.** A pacing mode governs presentation — tearing,
latency, and whether the frontend blocks on vsync or on its own clock. The `Pacer` accumulator above
remains the only thing that decides how many emulated frames run.

#### Sleep-then-spin

Under a self-paced plan nothing blocks, so `about_to_wait` waits out the remainder of the frame
period itself: `pacing::sleep_spin_split` returns a coarse sleep to `deadline - SPIN_MARGIN` (2 ms)
plus a spin for the rest. Sleeping the whole way routinely overshoots — an OS sleep is only accurate
to the scheduler's granularity — and drops the frame. Under display-sync this is skipped entirely;
the swapchain's vsync block *is* the wait, and spinning on top of it would only add latency.

#### Occlusion watchdog

`WindowEvent::Occluded` feeds `Pacer::set_occluded`. A compositor may stop delivering vsync to a
fully hidden window, which under display-sync stalls the present loop and silently stops the
emulator — so an occluded window is treated as self-paced regardless of the plan. Becoming visible
again drops the accumulated hidden interval rather than replaying it as a catch-up burst, the same
guarantee `Pacer::idle` gives around a pause.

### Performance panel (`v1.0.0`; rebuilt `v1.25.0`, T-FP-B)

View → Performance panel opens a diagnostic window rendered by `crate::perf_panel` over a
`perf::PerfReport` snapshot. For each of **emulation time**, **present time**, **GPU time**, and
**audio ring occupancy** it shows the latest reading, a `p50 / p95 / p99 / max` line, and a
sparkline of the ~5 s window (`perf::WINDOW`, 300 samples). The `v1.0.0` panel showed a single
current value per metric, which cannot answer the only question that matters when frames hitch: a
16.6 ms mean hides a 40 ms p99 completely.

Alongside those: the resolved pacing plan, and the **emulated-vs-presented** counters — tracked
separately on purpose, because they diverge exactly when something is wrong (a catch-up burst
emulates several frames for one present) and a single combined number hides that.

Two controls: **Reset stats** clears every ring, and **Start/Stop CSV log** toggles the session log
below. `PerfStats::report` allocates (it sorts each window), so the snapshot is taken only while the
window is open — which is the boundary that keeps `Metric::push` allocation-free on the
unconditional present path.

An idle present records **no** emulation sample. A `0.0` there would drag every percentile toward
zero and make a paused emulator look impossibly fast, which is precisely the reading that would hide
a real problem.

### CSV session log (`v1.25.0`, T-FP-B, native only)

The Performance panel's **Start CSV log** button writes one row per present to
`<data-dir>/rustysnes/perf/rustysnes-perf-<n>.csv`: `frame`, `elapsed_s`, `produce_ms`,
`present_ms`, `gpu_ms`, `audio_pct`, `produced`, `fps`. The rolling window answers "how is it running
now"; this answers "what happened during that 20-minute session, and does this build regress against
the last one?" — a hitch four minutes ago is gone from the ring but is still a row here.

CSV rather than a binary trace format deliberately: it opens in a spreadsheet, `csvlens`, pandas, or
gnuplot without this project shipping a reader. A missing measurement is an **empty field**, never
`0` — every CSV consumer treats an empty cell as missing, while a `0` silently drags an average down.

**Off by default.** A row per present is a syscall per present, exactly the kind of thing that
perturbs the measurement it is taking. Rows are buffered and flushed once a second so the logger's
own I/O is not visible in the numbers it records, and the buffer is flushed on drop so a session that
ends badly — the one worth having a log of — keeps its tail.

### GPU pass timing (`v1.25.0`, T-FP-B, `gpu-timing` feature)

`produce_ms` and `present_ms` are both **CPU** measurements. On a GPU-bound machine both can look
healthy while the display still stutters, because the cost is entirely in passes the CPU only
submits. `crate::gpu_timer` closes that blind spot with two `wgpu` timestamp queries bracketing the
frame's command encoder, resolved into a buffer and read back **one or more frames later** — reading
in the frame that wrote it would block the render thread on the GPU and make the measurement the
largest thing being measured. A frame is skipped rather than timed while a previous readback is
still in flight.

Two honesty properties, both deliberate:

- The device request **intersects** with `adapter.features()` instead of demanding
  `TIMESTAMP_QUERY`/`TIMESTAMP_QUERY_INSIDE_ENCODERS`. Requesting an unsupported feature makes
  `request_device` fail, which would turn a diagnostic build into one that cannot launch at all on
  such hardware.
- When the capability is absent the GPU row is **absent**, and the panel names the cause (feature not
  compiled in, vs. adapter lacks `TIMESTAMP_QUERY`) — never a flat zero line that reads as "the GPU
  costs nothing".

Not in `full`, and off by default: a query set plus a per-frame resolve and readback is measurement
apparatus, not something a normal play session should pay for.

### Fullscreen (`v1.0.0`)

View → Fullscreen toggles borderless fullscreen (`winit::window::Fullscreen::Borderless(None)`),
applied via the same "compare live state to `Active::applied_*` each frame, apply on mismatch"
pattern as the present-mode/theme toggles above.

### Window size presets (post-`v1.3.0`, RustyNES parity)

The View → Window Size menu itself is native only (`#[cfg(not(target_arch = "wasm32"))]`) —
offers 1x/2x/3x/4x (100%-400%) of the SNES native resolution, dispatching
`MenuAction::SetWindowScale(u32)`. `App::create_window` uses `3x` (`INITIAL_SCALE`) as the launch
default on **both** native and `wasm32` (`v1.7.0`) — winit's web backend resizes the attached
`<canvas>` to match the requested inner size at creation, overriding `web/index.html`'s own CSS
rule (found live: that CSS was a dead letter, not a real fallback — RustyNES's own
`create_window` requests `NES_W * INITIAL_SCALE` unconditionally too, which is why its wasm demo
already rendered at 3x while RustySNES's rendered at a smaller, page-declared 2x before this fix).
Only the *runtime* resize (the View → Window Size menu, `App::set_window_scale`) stays native-only
— `request_inner_size`'s async-grant semantics on web are a separate scope not covered here.
`App::set_window_scale` exits fullscreen first (so the resize takes effect against a normal
window), clamps the requested
scale to `1..=4`, and computes a chrome-padded `LogicalSize` via `App::chrome_padded_size` before
calling `window.request_inner_size`. That call may grant the resize synchronously (`Some`, no
separate `Resized` event follows, so `Gfx::resize` is called directly) or asynchronously (`None`,
handled by the existing `WindowEvent::Resized` handler). Transient, session-only — no
`config.toml` field, same posture as `MenuAction::SetSpeed`.

`chrome_padded_size` derives width from the scaled height via `Gfx`'s own `TARGET_ASPECT` (4:3),
not `SNES_W * scale` directly (floored at `MIN_CHROME_WIDTH`; height is `region.active_height() *
scale + CHROME_HEIGHT`, padding for the egui menu bar so the emulated image area lands near the
requested multiple even at `1x`). The SNES's native pixel ratio (256:224 ≈ 1.14:1) is narrower
than the 4:3 aspect `Gfx::blit` letterboxes every frame into, so a width derived directly from
`SNES_W` would make the window narrower than the content it's meant to hold — `Gfx`'s own
letterbox math would then scale the image back down to fit, silently defeating the requested
integer scale (caught in review before merge: a requested `3x` would have rendered at only
`~2.57x` vertically). Height uses `config.region.active_height()` (224 NTSC / 239 PAL, the same
per-region height `Config::Region` already exposes) rather than hardcoding NTSC's 224 — a PAL
session's "3x" preset would otherwise under-represent PAL's own native resolution (also caught in
review).

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

- USB gamepads auto-bind to P1 (and a second pad to P2) — `v1.25.0`, see below; keyboard drives P1.
- Late-latched input (sampled as close to the frame as possible) for responsiveness without
  breaking determinism.

### Physical gamepads (`v1.25.0`, `crate::gamepad`, native only)

`gilrs` had been a declared dependency and `input::gamepad_button` — the Xbox-diamond-to-SNES-diamond
rotation — had been unit-tested since early on, but **nothing instantiated the backend**, so port 1
was keyboard-only no matter how many pads were attached. `GamepadRuntime` closes that:

- **Assignment is by connection order** — first pad connected drives P1, second drives P2. That is
  what makes an unplug/replug land back on the same player instead of shuffling; disconnecting P1
  promotes P2, as pulling a controller does on hardware.
- **State is polled, not accumulated.** The event queue is drained purely so `gilrs` updates its own
  internal state, which is then read outright each frame. A frontend that only summed press/release
  deltas would desynchronise permanently the first time an event was missed (a button released while
  unfocused, a pad re-enumerated mid-session); polling self-corrects on the next frame.
- The left analog stick translates to the D-pad past `config.gamepad.deadzone` (default `0.35`).
  Sticks rest slightly off-centre, so without a deadzone a worn pad holds a direction permanently —
  which reads as "the D-pad is stuck", not "the stick drifts". Opposing directions are cancelled by
  `Buttons::sanitize_dpad`, the same treatment keyboard input gets.
- Keyboard and pad are **OR-ed**, so either works at any moment with no mode switch
  (`App::effective_pads`). **P2 is now live-driven**; before this it was explicitly zeroed every
  frame, and only TAS playback or netplay ever set it.

### Autofire / turbo (`v1.25.0`)

`config.turbo` selects any of A/B/X/Y and a cycle length in frames (`period_frames`, clamped to
`2..=60` — a period of 1 would hold the button permanently and thereby silently *disable* the feature
it looks like it configures). `input::apply_turbo` only ever **clears** bits in the mask, never sets
them: an implementation that set bits would make an autofire button fire while untouched. It runs
where host input is sampled, so the core still receives an ordinary button stream and the determinism
contract is untouched — the same boundary the DRC servo and the post-filters respect.

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

**What this frontend wires today (`v0.9.0` → `v1.20.0`):** a Settings → Input control
(`ui_shell.rs`) selects controller port 2's peripheral via `config.port2_peripheral`, re-synced to
the Bus every frame (`app.rs`, alongside the cheats/watchpoints sync). **Mouse and Super Scope now
get live host-input capture too** (`v1.20.0`, `crate::peripherals`): `egui::Context`'s own pointer
state (available every frame since `egui_state.on_window_event` runs unconditionally — no new
`WindowEvent` plumbing needed) feeds `EmuCore::set_mouse`/`set_superscope` once per frame,
alongside the existing port-device re-sync. Super Scope's absolute aim position is mapped from
window pixels through the present path's own letterbox transform (`Gfx::letterbox_scale`, exposed
`pub(crate)` for exactly this reuse — never re-derived) into SNES `0..256`/`0..240` pixel space;
trigger/cursor/turbo map to left/right/middle mouse buttons (no fourth button exists for `Pause`,
left unset). Portable to wasm on purpose — both the pointer API and the `EmuCore` calls are
already platform-agnostic, so the hosted demo gets this too, not just the native build. The pure
coordinate-mapping math is unit-tested directly (`peripherals.rs`'s own `#[cfg(test)]` module),
not just "compiles and is presumed correct."

**What's still NOT wired: Super Multitap sub-pads 1-3.** Real host gamepad polling would be the
input source, but a genuinely separate, larger discovery blocks it: `gilrs::Gilrs` is never
actually instantiated anywhere in this crate today — confirmed while scoping the Mouse/Super Scope
fix above, `input::gamepad_button` (the gilrs-button-name → SNES-button mapping function) has zero
callers. **Controller port 1's own gamepad support is unwired too** — despite this doc's earlier
"Status" line claiming "keyboard + gilrs gamepad", the default GUI session is keyboard-only right
now. This is a real, separate finding, not a silently-incomplete claim: closing Multitap host
input needs a genuinely new prerequisite (a live `Gilrs` instance + per-frame event polling loop),
not a small addition on top of the Mouse/Super Scope wiring above — see
`to-dos/ROADMAP.md`/the UI/UX-parity plan's Phase B/C backlog for where this is tracked.

## Per-game overrides (`v1.25.0`, `crate::per_game`)

A ROM's own settings live in `<config-dir>/RustySNES/per-game/<key>.toml`, keyed by the ROM's file
stem sanitised to a safe filename (the same rule `crate::screenshot` uses, for the same reason).
File -> "Save Settings for This Game" captures the overridable knobs; they are applied right after a
successful load, before the next present reads the config, so the ROM's first frame already honours
them. "Clear Settings for This Game" deletes the file, and a missing file counts as success.

The overlay is a **flat set of `Option` fields**, not an optional nested `Config`. That is the
load-bearing decision: an "optional whole Config" cannot distinguish "this game wants the default"
from "this game was saved before that field existed", so every absent field would silently resolve to
whatever the *current* default is — freezing it — and a later global change would stop reaching every
game that had ever been saved. `capture` records every knob (the user's intent is "keep it looking
like this"), `apply` writes only the present ones, and a corrupt file degrades to "no overrides"
rather than blocking the load.

## Graphic equaliser (`v1.25.0`, `crate::eq`)

Five RBJ peaking biquads at 60 / 240 / 1k / 3.5k / 10k Hz, +/-12 dB each, shared Q of 0.9 so adjacent
bands overlap gently rather than leaving a notch between sliders. The top band sits at 10 kHz because
the S-DSP's 32 kHz output has a 16 kHz Nyquist limit — a band above that would filter content that
cannot exist. It runs on the resampled `f32` stream inside `Resampler` (where samples become floats;
filtering the 32 kHz `i16` input instead would put every band centre in the wrong place), which keeps
it on the frontend side of the determinism boundary.

Three properties are asserted by tests rather than assumed:

- **Flat or disabled is an exact, bit-identical pass-through.** Mathematically the filters are an
  identity at 0 dB, but running them would still perturb the output by float rounding. Detecting flat
  and returning the input untouched is what makes the stage safe to leave permanently in the path.
- **The channels are filtered independently.** Sharing biquad state across L/R collapses stereo into a
  smear while still sounding like EQ, so a test drives the left channel hard and requires the right to
  stay silent.
- **A boost raises measured energy and a cut lowers it**, which is what actually catches a sign error
  in the coefficients — something no "it runs without panicking" test would notice.

Output is clamped to full scale after the cascade: a boosted chain can exceed 1.0, and letting it wrap
is the difference between "loud" and "harsh digital distortion".

## ROM soft-patching (`v1.25.0`, `crate::patch`)

A same-stem `.ips` / `.bps` / `.ups` sitting beside the ROM is applied **in memory** at load time, so
the dump on disk stays pristine and any number of hacks can live next to one clean ROM. All three
formats are implemented (IPS literal + RLE records, UPS XOR deltas, BPS's
`SourceRead`/`TargetRead`/`SourceCopy`/`TargetCopy` action stream).

- **Detected by magic, not by extension.** Extensions get renamed, and a mislabelled `.ips` that is
  really a BPS would otherwise be parsed as garbage.
- **A patch file is untrusted input** (module 60): every length, offset, and varint is bounds-checked
  before use and every path returns `PatchError` rather than panicking. A truncated patch reports the
  byte it died at; the test suite feeds *every prefix* of a valid patch and requires a clean error
  from each. Output size is capped at 32 MiB, because a corrupt header can otherwise declare a
  multi-gigabyte target and have the applier allocate it before any other check fails.
- **`TargetCopy` must copy byte-by-byte**, observing its own output: the read range legitimately
  overlaps the bytes being appended (that is how BPS encodes a run), so a slice copy would read stale
  bytes and silently corrupt every run.
- **A patch that fails to apply is reported and the UNPATCHED ROM is loaded.** Booting the original
  beats refusing to boot, and silently swallowing the failure would leave the user wondering why the
  hack did nothing — the status line names the reason either way.
- BPS/UPS declare their source size; a mismatch against the supplied ROM is refused rather than
  applied, since applying it produces a silently corrupt image.

## ROM loading, Recent ROMs, and screenshots (`v1.25.0`)

Three entry points now open a ROM — the File → Open picker, **drag-and-drop**
(`WindowEvent::DroppedFile`), and the **Recent ROMs** menu — and all three funnel through the single
`MenuAction::OpenRom` handler rather than reimplementing parts of it. The mechanism is
`Active::queued_rom`: the out-of-band paths set it and queue the action via
`ShellState::pending_actions` (drained into the present's action list after the egui pass, which
*assigns* `actions` and would otherwise discard them), and the handler prefers a queued path over
opening the picker. That keeps firmware install, HD-pack re-select, RetroAchievements re-identify,
ROM-info re-hash, and rewind/quick-save invalidation on every path.

`config.recent` keeps 10 entries, newest first, de-duplicated by path (re-opening promotes rather than
duplicates), and only records on the `"Loaded "` success prefix so a rejected file never enters a list
of things you can open. A recent entry whose file has moved reports that and prunes itself.

**Screenshots** (`crate::screenshot`) write a numbered PNG into the platform picture directory (or
`config.screenshot_dir`) or copy to the system clipboard via `arboard`. Capture happens at the one
point in `render` where the finalized RGBA buffer is in hand, so it costs a single encode of a buffer
that already exists rather than retaining a copy of every frame against the chance one is wanted. The
captured pixels are the emulator's output at **native resolution** — after the overscan crop and any
HD-pack composite, before the letterbox/post-filter GPU pass — so two screenshots of the same frame are
byte-identical regardless of window size, which is what makes them usable as reference images. ROM
titles are sanitised to `[A-Za-z0-9_-]` before entering a filename, because internal SNES titles
legitimately contain punctuation and path separators.

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
  - **Frame-budget throttle (`v1.25.0`, `config.run_ahead.throttle_ms`).** Run-ahead multiplies
    emulation cost by `frames + 1`, so on a machine already missing its frame deadline it converts a
    latency win into visible stutter. When the previous frame's measured production time
    (`Active::last_frame_time_ms`) exceeded `throttle_ms`, the peek is skipped for that frame and the
    plain (correct, cheaper) path runs instead, so an overrun cannot compound. `0` disables the
    throttle. RustyNES throttles for the same reason; without it the feature cannot safely be left on.
    Note RustyNES additionally uses a **dedicated lightweight snapshot** for the peek rather than the
    full `save_state`/`load_state` round trip `step_with_run_ahead` performs — that fast path is not
    yet ported, and is why `frames` remains `0` by default here.
- Both are pure re-simulation of the SAME deterministic core (`docs/adr/0004`): no injected
  timing/RNG, just running the existing `run_frame`/`save_state`/`load_state` extra times. Proven
  by `rewind.rs`'s tests, which hand-assemble a tiny 65C816 program (NMI-driven WRAM counter →
  CGRAM backdrop write) to get a real, observable per-frame state signal rather than asserting
  against a synthetic fingerprint.
- **`FORMAT_VERSION` versioning is intentionally fail-loud, not migrating — `v1.13.0` correction,
  not new work.** `to-dos/VERSION-PLAN.md`'s original `v1.13.0` plan text asked for "a save-state
  versioned-migration regression fixture... the one real save-state gap found." Investigating it
  found the premise itself was stale: `System::load_state`
  (`crates/rustysnes-core/src/scheduler.rs`'s `FORMAT_VERSION` doc) only ever rejects a blob
  *newer* than it supports — it was never designed to gracefully load an *older*-format blob, by
  deliberate choice recorded in that doc comment since the `2` and `3` bumps. A regression fixture
  proving exactly this behavior ALREADY exists
  (`crates/rustysnes-test-harness/tests/save_state_backward_compat.rs`'s
  `old_format_version_blob_fails_loudly_not_silently`, against a genuine captured
  `FORMAT_VERSION = 1` blob) and has existed since `v0.7.0`. So there was no gap to close: the
  "one real save-state gap" was already both intentionally designed-around and regression-tested
  before
  `v1.13.0` started. Building an actual graceful-migration path (translating an old envelope's
  section layout forward) was considered and explicitly rejected — it would add real complexity to
  a determinism-critical serialization boundary for a feature nobody has asked for, in exchange for
  a save-format promise ("your old save always loads") this project has never made. The 10-slot/
  thumbnail Save States manager above is already at full parity with RustyNES's own UI; this was
  the only outstanding save-state item, and it's now closed as verified-non-issue rather than
  carried forward as a phantom TODO.

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

`crates/rustysnes-frontend/web/index.html` (deployed by `.github/workflows/web.yml` since
`v1.6.0 "Lighthouse"`, `pages.yml` before that) got a
polish pass: a visible `<h1>RustySNES` title, a keyboard-controls + feature-parity hint paragraph
(matching the real `input::KeyBindings` defaults, and disclosing the Save States browser gap
above rather than staying silent about it), an inline-SVG favicon (no logo asset exists yet,
unlike RustyNES's `assets/RustyNES_Icon/` set, so this avoids either shipping no favicon at all
or a new binary asset to keep in sync), and a `theme-color`/description meta pair. Deliberately
NOT ported: RustyNES's touch-controls overlay, PWA manifest/service worker, browser-Lua panel,
and `?settings=` share-link — none of those features exist in RustySNES today (no touch input
handling, no wasm Lua backend, no config-to-URL serialization), so faking their UI would be the
same "claims support that doesn't exist" anti-pattern this project avoids everywhere else.

**`v1.20.0`:** `.github/workflows/web.yml`'s `trunk build` gained `--features cheats,debug-hooks`
— both are pure computation with zero wasm-incompatible dependencies (confirmed via a real
`cargo check --target wasm32-unknown-unknown`), and had simply never been added to the deployed
demo's build, not excluded for any architectural reason. Tools → Cheats and Debug →
Debugger overlay now show their real controls in the hosted demo instead of a
`(rebuild with --features ...)` placeholder label. `scripting`/`netplay`/`retroachievements`
remain genuinely unavailable on wasm today (`mlua`/native sockets/`rcheevos` FFI are not
wasm-portable) — their placeholders are honest, not a gap in this fix.

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
`rustysnes-core` API beyond one addition, `Bus::peek`, needed because the debugger's
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

**ROM Info panel (`v1.20.0`):** a read-only CRC32/SHA-256/header decode of the loaded cart —
`crates/rustysnes-frontend/src/debugger/rom_info_panel.rs`. `RomInfo::capture` is called once per
ROM load/close (native `MenuAction::OpenRom`/`CloseRom` and the `wasm32` file-picker path), not
recomputed every frame like `DebugSnapshot` — a loaded ROM's identity and header never change while
it stays loaded. CRC32 comes from `crc32fast` (already resolved in the workspace's dependency tree
as a transitive dep, so a direct pin was free); SHA-256 reuses `rustysnes_core::movie::hash_rom`
rather than recomputing it. The header decode also picked up a genuinely new field along the way:
`rustysnes_cart::header::Header` gained a `title: String` (the raw 21-byte internal title,
non-printable bytes replaced with spaces, trailing padding trimmed) — `Header::parse` already had
the raw bytes in hand for its own coprocessor-disambiguation title match, this just surfaces them.

### Trace, events, and the access heat map (`v1.25.0`, T-FP-C1)

`rustysnes_core::trace` is the third rung of the same opt-in observability `watchpoint` established,
under the same `debug-hooks` gate and the same never-in-a-save-state contract. A watchpoint answers
"who touched *this* address?"; these answer the three questions it structurally cannot:

| Facility | Answers | Hooked at |
|---|---|---|
| `record_step` | what ran, with the full register file **pre**-execution | `System::run_frame`/`step_instruction`, beside the existing `set_debug_pc` |
| `record_event` | how control got here — call / return / interrupt, with a depth | after each step, classified from the opcode |
| `note_access` | what is hot — per-address WRAM read/write counts | `Bus::note_bus_access`, the same hook watchpoints use |

Two design points that are not obvious:

- **Recording is separately armed, not implied by `debug-hooks`.** A watchpoint list is naturally
  empty until the user arms one, so `is_empty()` gates it for free. A trace records *everything* by
  nature and has no such empty state, so each facility carries an explicit flag starting `false`. The
  heat map's 128 K-entry allocation is made on the first enable and never at all on a build that
  leaves it off.
- **Events are classified from the actual post-step `PBR:PC`, not from decoding the operand.** A
  `JSR (a,X)` has no static destination and a conditional path would need the CPU re-implemented to
  know whether it was taken; reading where the CPU actually went cannot be wrong about either.
- **An interrupt is detected before the opcode is consulted, and has to be.** On a step that vectors
  an NMI/IRQ the CPU fetches no opcode at all, so the carried `pending_trace_opcode` still holds the
  *previous* instruction's byte. Classifying from it does not merely miss the interrupt: an NMI
  arriving right after a `JSR` is recorded as a second `Call` whose `to` points at the NMI vector's
  target. `Cpu::interrupts_taken` — a counter written only on the interrupt path, never per
  instruction — is the unambiguous signal, snapshotted before the step and compared after. This is
  still one classifier rather than separate call and interrupt hooks; it just asks the right
  question first.

The heat map covers WRAM only, and folds the `$0000-$1FFF` low-RAM mirror onto the same slots as the
`$7E` linear window: they are the same bytes, and two counters would show one hot address as two
cold ones. `heatmap_index` masks its argument to 24 bits, which is both the CPU bus's own wrap at
`$FF:FFFF` and what stops a window walking off the top of memory from reporting low-RAM heat that
belongs to a different address (`$FF:FFFF + 1` truncates to bank `$00`). The **freeze list** keys on
the same canonical form for the same reason: `$00:0042` and `$7E:0042` are one byte, and two entries
holding different values would fight every frame with the later one silently winning.

### Memory editor panel (`v1.25.0`, T-FP-C1)

The `v1.7.0` panel showed a fixed 512-byte read-only dump with no way to move it. The Memory panel
adds go-to/paging, byte editing, freezes, and the heat column.

**Edits reach WRAM only** (`Bus::poke_wram`). Non-WRAM rows are greyed and the poke button refuses
them by name — a debugger that appeared to edit a ROM byte which then read back unchanged would look
like an emulation bug, so "not writable" is stated rather than silently performed.

**A freeze is re-applied every present, not once.** Its whole purpose is to hold a value the *game*
is rewriting; a one-shot poke would be overwritten by the next frame and look like the freeze failed.
Freezes are written after pokes, so a freeze always wins over a same-address poke rather than the
result depending on click order.

The heat column is normalised **logarithmically against the map's peak**: access counts span orders
of magnitude between a once-per-frame variable and an inner-loop pointer, and a linear scale renders
every ordinary address indistinguishable from untouched.

The panel never moves the window itself — it *requests* a new start address, which `app.rs` applies
outside the egui pass, because `set_debug_memory_scroll` is on `EmuCore` and the shell's
non-negotiable rule is that egui never reaches the emu lock.

### OAM panel (`v1.25.0`, T-FP-C1)

`PpuSnapshot::oam` had carried all 544 bytes since the overlay existed and nothing read them. OAM is
the one PPU structure whose raw bytes are genuinely unreadable by eye: each sprite's X sign bit and
size bit live in a *separate* 32-byte high table, two bits per sprite at `(index % 4) * 2`. So "why
is this sprite off-screen" is a question the hex cannot answer and the decode can — X is 9-bit
**signed**, and a sprite at X = -32 is partly visible while one at X = 224 with bit 8 set is not.

Off-screen rows are dimmed, never hidden ("sprite 47 exists but is parked at Y=240" is exactly what
the panel is opened for), and the off-screen test uses the *maximum* 64 px sprite extent so it never
claims a sprite is invisible when the configured size might still put part of it on screen.

### Map panel (`v1.25.0`, T-FP-C1)

Answers "what is at `$C0:8000`?". ROM Info decodes the header and the Cart panel names the board;
neither says where ROM, SRAM, WRAM, and I/O land on the CPU bus — and that differs by mapping, which
is why the table is **derived from the detected `MapMode`** rather than hardcoded. A LoROM and a
HiROM cart genuinely show different maps; a static table would look authoritative and describe
whichever cart the author happened to have open.

Lookup returns the *first* matching range, so the map lists the specific regions (low-RAM mirror,
I/O) before the broad ROM windows that would otherwise swallow them. An address no range covers
reports as uncovered rather than being forced into the nearest one — "open bus" is a real answer, and
a wrong guess sends someone hunting a bug that is not there. Coprocessor windows are board-specific
and are explicitly out of scope here, with a note in the panel saying so.

### Conditional breakpoints and the expression evaluator (`v1.25.0`, T-FP-C2)

A plain address breakpoint answers "did execution reach here", which is the wrong question for the
bugs that are actually hard: a routine called two thousand times a frame where one call misbehaves.
`crate::expr` adds a condition — `a > $80 && [$7E0300] == 3` — turning that into a breakpoint that
fires once.

The evaluator is integer-only, with no assignment, no calls, and read-only memory access (`[addr]`
for a byte, `{addr}` for a little-endian word). It cannot have side effects on the machine it is
inspecting, which is a requirement rather than a simplification: a condition is evaluated on every
hit of its address.

Three properties worth knowing:

- **Evaluation is total.** Division and modulo by zero yield `0`, and out-of-range shifts saturate
  rather than wrapping (`1 << 64` is `0`, not `1 << 0`). A breakpoint that stopped working
  mid-session because a divisor transiently hit zero would be worse than one that briefly reads a
  wrong value, and a shift that silently wrapped to a no-op would be a wrong answer with no signal.
- **`&&`/`||` short-circuit**, so `[ptr] != 0 && [[ptr]] == 5` genuinely guards its own deref.
- **A condition that does not parse never arms.** The panel shows the parse error beside the entry.
  A breakpoint that means something other than what it reads as is worse than one that refuses.

The cost model: the address is compared first and the condition is evaluated **only** on a match, so
a conditional breakpoint costs the same per instruction as an unconditional one. On a hit it
captures the register file plus a 128 KiB WRAM copy — paid once per hit, not once per instruction,
and **once per hit rather than once per condition**, since several breakpoints can share an address
and all of them read the same non-advancing machine. The buffer lives on `EmuCore` and is refilled
in place, so a condition on a frequently-hit address does not churn an allocation each time; an
unconditional breakpoint at the same address short-circuits before any snapshot is taken.

The snapshot is whole-WRAM rather than a read-through cache of "just the addresses the condition
names", and that is forced rather than lazy: `expr::Context::peek` takes `&self` (deliberately — a
condition must not be able to mutate the machine it inspects) while `Bus::peek` needs `&mut`, and
`Expr::Byte`/`Word` take an arbitrary sub-expression, so `[a + 4]`'s address is not known until the
condition is already being evaluated.

`x` deliberately stays the index register; the width flags are spelled `fm`/`fx`, because a
condition asks about X far more often than about the `X` status bit.

### Symbol maps (`v1.25.0`, T-FP-C2)

`JSR $9A3C` says nothing; `JSR update_sprites` says what the program is doing. `crate::symbols`
loads WLA-DX-style `.sym` files (`[labels]` sections of `BB:AAAA name`) and flat/assignment forms,
and names addresses in the disassembly, trace, call stack, and hot-address views.

`nearest` resolves an address to `symbol+offset`, bounded at 4 KiB — exact-match-only would name the
entry point and leave every instruction after it anonymous, while an unbounded search lets one
symbol claim the whole ROM.

Parsing is **tolerant**: an unrecognised line is skipped rather than failing the load, because
refusing 4,000 good symbols over one stray directive trades the feature for pedantry. It is not
*silent*, though — the load reports how many lines it skipped, so a file that produced nothing says
so. Non-`[labels]` sections are skipped entirely: a WLA `.sym` also carries `[breakpoints]` and
`[definitions]` whose lines look enough like symbols to poison the map.

### Trace panel (`v1.25.0`, T-FP-C2)

The reader for T-FP-C1's recording, with four views:

- **Instructions** — the trace ring disassembled and symbol-labelled, with the register file as it
  was before each instruction ran.
- **Call stack** — **reconstructed from the event log**, not walked. The 65C816 stack holds return
  addresses but nothing distinguishes one from any other pushed word, so walking `S` upward
  confidently invents frames that never existed. Replaying the recorded enters and leaves produces
  the stack that is actually there; the cost is that it reaches back only to when recording started,
  which the panel says rather than papers over. An unmatched leave is dropped, never turned into a
  fabricated caller.
- **Events** — the raw call/return/interrupt log, indented by depth.
- **Hot addresses** — the access counter's top 32, ties broken by address so the list does not
  reshuffle every frame while counts are still climbing.

The whole read-out (disassembling up to 4,096 rows, scanning 128 K counters) is gated on this panel
being the open one — the same guard `available_hd_packs` and the save-slot grid already use.

### Inline assembler (`v1.25.0`, T-FP-C2)

`crate::asm65816` assembles one instruction at the CPU's current `PBR:PC` and `M`/`X` widths. Both
matter: a branch operand is PC-relative, so the same source assembles to a different byte at every
address, and `LDA #$12` is two bytes with `M=1` and three with `M=0`.

**It assembles by searching the disassembler.** For each candidate opcode it synthesizes bytes, runs
the *real* `rustysnes_cpu::disasm` over them, and keeps the encoding whose disassembly matches the
requested text. The obvious alternative — a second, hand-maintained opcode table — has the worst
possible failure mode: it stays plausible while being wrong for one opcode, and the assembler is
exactly the tool you would reach for to investigate the bug that causes. This way the encoder is
correct by construction against the decoder, the round-trip is the natural test, and adding an
opcode to the decoder makes it assemblable for free.

Scope: one instruction, no labels, no directives, no expressions. A debugger patch is "make this
branch unconditional", not a build system. Patches go through the Memory editor's WRAM-only poke
path, so one place decides what is writable, and the panel says so.

An unreachable branch reports as **out of range with the distance**, not as an unknown encoding —
that is the one failure the user fixes by moving the target rather than rewriting the line.

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

### Connection quality and the desync banner (`v1.27.0`)

The session's transport is wrapped in `rustysnes_netplay::LivenessTransport` (`netplay.rs`'s
`SessionTransport` alias), ticked once per frame from `NetplayState::drive` with this peer's own
frame advantage. `NetplayState::status()` samples the whole view into a plain `NetplayStatus`
snapshot rather than handing the UI a borrow of the session — the session is driven inside
`Active::render`'s frame loop and read again in the egui pass, and a borrow would make those two
uses fight over `&mut App`.

The Netplay window renders it: peer grade, ping, frame advantage, current frame, plus a handshake
notice and the graded desync banner. Two deliberate choices in the colours. **`Interrupted` is amber,
not red** — it is two full ping intervals of silence, which ordinary Wi-Fi produces, and painting it
red would train the user to ignore the colour that matters. **Ping shows `—`, not `0 ms`, until a
round trip completes**, because a zero reads as a perfect connection at exactly the moment nothing is
known. The banner distinguishes `Suspect` from `Desynced` because that is the entire point of the
graded verdict (`docs/netplay.md`): a transient must not look like a lost game, and a real divergence
must not look survivable.

A liveness verdict ends the session through `NetplayError::Disconnected`, which `Active::render`
already treats like any other netplay error — disconnect and fall back to single-player.

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
toast messages via `CheevosState::poll`'s return value.

**`v1.11.0 "Podium"`: `CheevosState::load_game`/`unload_game`.** Before this release, no code
path ever called `RaClient::begin_load_game` — every other piece (login, the per-frame `do_frame`
hook, the unlock-toast plumbing) was wired up, but with no game ever identified/loaded into
`rc_client`, there was no achievement set loaded to evaluate WRAM against, so achievements could
never actually trigger. `load_game`/`unload_game` are now called from `app.rs`'s
`MenuAction::OpenRom`/`CloseRom` handlers (`load_game` is a no-op unless a user is logged in); a
`poll()`-drained toast ("game identified, achievement set loaded" / "game identification
failed: …") makes the fix observably verifiable in the running app, not just type-checked. A ROM
loaded via the CLI at startup, followed by a *later* login through the Tools window, is not
retroactively announced (see `cheevos.rs`'s module doc for why and what a real fix needs) — the
common launch-then-log-in-then-open-a-ROM path is unaffected.

**Honest scope notes**: not wired into the netplay `drive` path (a `RollbackSession`-driven
`System` and achievement tracking interacting — e.g. resimulation re-triggering `rc_client`
frames — is a separate, deferred concern, noted at the `do_frame` call site); no hardcore-mode
gating of rewind/save-load/cheats/TAS, and no leaderboard/rich-presence UI panel yet (`RaClient`
already exposes `set_hardcore_enabled`/`leaderboard_list`/`rich_presence`, just not consumed by
any window or gate — both real, substantial follow-ups that were meaningless before this
release's game-load fix landed, `to-dos/VERSION-PLAN.md`'s `v1.11.0` section). SRAM-backed
achievement sets aren't supported — `rustysnes_cheevos::ra_addr_to_snes` only maps the SNES's 128
KiB WRAM (`docs/adr/0003`-style honest scope cut, documented in the crate itself). With
`retroachievements` off, `rustysnes-cheevos` never enters the frontend's dependency graph
(`dep:rustysnes-cheevos`) and the default build is unaffected.

## Open questions

- ~~Whether the second-CPU (SA-1 / Super FX) state warrants its own debugger panel from day one
  or a Phase 8 add~~ — **resolved, `v0.8.0`:** yes, from day one. The Cart panel shows SA-1's
  second-CPU registers (`System::sa1_regs`) or the Super FX/GSU register file
  (`Board::debug_gsu_state`) when the loaded cart uses either.
