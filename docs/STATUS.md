# RustySNES — STATUS (single source of truth)

This file is authoritative for per-suite pass counts, the board / coprocessor matrix, and
version policy. Everything else defers to it.

**Current release:** `v1.20.0 "Aperture"` (`v0.1.0 "Foundation"`,
`v0.2.0 "Persistence"`, `v0.3.0 "Continuum"`, `v0.4.0 "Completion"`, `v0.5.0 "Fidelity"`,
`v0.6.0 "Shippable"`, `v0.7.0 "Resolution"`, `v0.8.0 "Community"`, `v0.9.0 "Threshold"`,
`v1.0.0 "Zenith"`, `v1.0.1 "Aftertouch"`, `v1.1.0 "Latchkey"`, `v1.2.0 "Phosphor"`,
`v1.3.0 "Palimpsest"`, and `v1.4.0 "Convergence"` precede it; see `to-dos/VERSION-PLAN.md` for the
full ladder). **`v1.5.0`-`v1.19.0`** are the RustyNES-parity ladder — a CI
safety net (`v1.5.0 "Bedrock"`), a MkDocs documentation site + PWA + accuracy ledger
(`v1.6.0 "Lighthouse"`), the debugger extracted into its own module plus a hex Memory panel
(`v1.7.0 "Telemetry"`, `v1.7.1` patch), a Memory Compare panel + in-app Docs panel
(`v1.8.0 "Tracepoint"`), widening `rustysnes-script`'s `emu.read` to the full 24-bit bus
(`v1.9.0 "Marionette"`), wiring HD-pack compositing into the `emu-thread` build for the first
time (`v1.10.0 "Atelier"`), fixing RetroAchievements to actually load a game for the first
time (`v1.11.0 "Podium"`), adding a third presentation post-filter (`PostFilter::Xbrz`) plus
extracting the WGSL shader sources into a new `rustysnes-gfx-shaders` crate for reuse by the
mobile bridge (`v1.12.0 "Refraction"`), adding two accessibility theme variants
(`AppTheme::HighContrast`/`Colorblind`) while honestly re-scoping the other two originally-planned
items (`v1.13.0 "Vantage"`), reversing `v1.0.0`'s "no mobile appetite" default with a new
`rustysnes-mobile` `UniFFI` bridge crate over `EmuCore`, real-verified via a `cargo ndk` ARM64
cross-compile and inspected Kotlin/Swift binding output, plus the `no_std` CI gate expanded to a
per-crate matrix (`v1.14.0 "Foundry"`), a real Android alpha — a new `rustysnes-android`
presentation-only `wgpu`-on-`Surface` crate plus a minimal Kotlin Compose shell, verified for real
on a live Android emulator (a committed test ROM boots and its framebuffer visibly advances
frame-to-frame, background/foreground lifecycle exercised, zero `logcat` errors) — with the
`Mouse`/`Super Scope`/`Multitap` touch UX, save-state UI, and post-filter wiring honestly deferred
to `v1.15.1+` (`v1.15.0 "Sideload"`), and an iOS alpha — a new `rustysnes-ios` crate mirroring
`rustysnes-android`'s architecture, verified for real via genuine `aarch64-apple-ios`/
`aarch64-apple-ios-sim` cross-compiles in a Linux sandbox with no Xcode installed, plus a real,
passing, unsigned `xcodebuild` simulator build on a `macos-latest` CI runner (the project's only
real Xcode/Swift toolchain) after fixing four real CI-found build bugs and three real
PR-review-found runtime/lifecycle bugs (`v1.16.0 "Beacon"`), and a hardening pass adding
Save State/Load State to both mobile shells that, in the process of re-verifying it for real on
the Android AVD, found and fixed a real, pre-existing, already-shipped native crash present
since `v1.15.0` (per-frame allocation churn in the audio path disrupting `AudioTrack`'s native
buffer timing after ~10+ seconds of continuous run — never caught before because no prior
verification pass ran that long), with RetroAchievements wiring, an `mlua` migration, and
netplay honestly re-scoped to a later rung (`v1.17.0 "Parity"`), and dormant monetization
scaffolding — a new, standalone `rustysnes-monetization` `UniFFI` crate (never a dependency of
the deterministic core, every pricing/pacing figure an explicit placeholder pending the
standing "Mobile Phase 6" store-launch gate) wired into both mobile shells as an inert,
log-only startup call, real-verified on the Android AVD via `logcat` and compile-verified for
iOS via a real macOS CI build after fixing a genuine `xcodebuild` xcframework-modulemap
collision found on that same CI run (`v1.18.0 "Dormant"`), and finally an optional PGO/BOLT
pipeline for the shipping `rustysnes` binary — `scripts/pgo/run.sh` (instrument → train against
the committed permissive ROM corpus → optimized rebuild) plus `.github/workflows/pgo.yml`
(`workflow_dispatch`/release-tag-only, promotion gated on both a `>3%` Criterion speedup and a
byte-identical `--features test-roms` re-run under the PGO profile, never on speed alone),
real-verified end-to-end in this development environment including a genuine BOLT-stage bug
found and fixed in PR review (`v1.19.0 "Afterburner"`) — all frontend/tooling/CI work
with **zero
change** to the accuracy dashboard, per-suite pass counts, or coprocessor tier matrix below,
which stayed byte-identical throughout; see `CHANGELOG.md` for full per-release detail.
**`v1.20.0 "Aperture"`** opens a new UI/UX-parity ladder (Phase A) auditing the desktop frontend
and wasm demo against RustyNES's own frontend maturity: two wasm demo menu items (`Cheats`/
`Debugger overlay`) fixed from placeholder to real, live Mouse/Super Scope host-input capture
wired for the first time (`crate::peripherals`), a View → Hide Overscan toggle, and a new
Debug → ROM Info panel — again zero change to the accuracy dashboard; see `CHANGELOG.md` for
full detail. `v1.0.0`
closes the production-cut
gate: `Board: Send` (unblocking `emu-thread` to compile/test/lint clean for the first time, though
it stays off-by-default pending full feature parity — see `docs/frontend.md`), the five
desktop-UX-shell-maturity items (thumbnail Save States manager, key-rebind grid, themes, speed
presets, a Performance panel with a frame-time sparkline; plus fullscreen and a first-run welcome
modal), a CI frame-time performance-regression gate, an enhanced native CLI +
`cargo full-build`/`full-run`, a full README rewrite, and a GitHub Pages demo-page polish pass.
The save-state `FORMAT_VERSION` backward-compat fixture + regression test (once thought still
open) turned out to already be landed, from `v0.7.0`.
`v1.0.1` closes the two items explicitly deferred out of the `v1.0.0` bar:
per-voice audio mute (Settings → Audio, 8 checkboxes gating `Dsp::voice_output`; no real S-DSP
hardware register behind it — see `docs/apu.md` §Per-voice mute) and global keyboard hotkeys
(a fixed, non-rebindable table — `Escape`/`F1`-`F5`/`F9`/`F11`/`F12`/`Space`/`` ` ``, suppressed
while an egui widget has keyboard focus — see `docs/frontend.md` §Global hotkeys). Both are
additive/off-by-default in effect (unmuted / hotkeys simply weren't there before); the full
workspace suite (incl. `--features test-roms`, all 27 accuracy/oracle suites), `no_std`, and the
full clippy matrix are all green with zero regressions. Shipped as `v1.0.1` per explicit
project-owner instruction, deviating from this project's own "additive changes ship as MINOR"
convention for this one release — see `CHANGELOG.md`.
`v1.1.0` was a research + accuracy pass following up on the Reach-phase backlog.
Landed: a real, independent bug fix (`SuperFxBoard::map`'s Game-Pak-RAM-ownership open-bus gap,
zero regressions across the full `--features test-roms` battery) and `emu-thread`'s biggest
gaps (real audio output via a thread-owned `AudioProducer`, plus a proper pause/ROM-loaded/speed
lifecycle via `EmuControl` and a `PresentBuffer` lock-free framebuffer handoff — still not full
parity with the synchronous drive, see `docs/frontend.md`). Investigated-not-landed: the
open-bus-via-DMA-latch bug (substantially narrowed, still open — `docs/scheduler.md` §Open bus
via DMA/HDMA), DRAM refresh (empirically measured to already be correct without an additive
stall — implementing the originally-planned fix would have been a regression, `docs/scheduler.md`
§DRAM refresh), and a fractional-timebase refactor go/no-go assessment (conclusion: not
warranted yet, `docs/audit/fractional-timebase-go-no-go-2026-07-11.md`).
`v1.2.0` is the Libretro-core + CRT/HQ2x-shader-pipeline release. Landed: the pure
`EmuCore` embedding facade relocated from `rustysnes-frontend` into a new `std`-only
`rustysnes_core::facade` module (zero behavior change, plus a determinism-seed-discarding bug
found in review — see `docs/architecture.md` §3/§6), `rustysnes-libretro`, a real libretro
core wrapping that facade (region-aware NTSC/PAL geometry+timing, cheat support, coprocessor
firmware auto-resolution, raw WRAM/VRAM/SRAM memory-map pointers — see `docs/libretro.md`), and
the CRT/HQx presentation post-filter pipeline (Settings → Video / View → Post-filter — scanlines +
aperture mask, an HQ2x-style edge-directed blend approximation, `PostFilter::None` kept
byte-for-byte identical to the pre-filter direct blit — see `docs/frontend.md` §Presentation
post-filters). Full workspace suite, the `--features test-roms` accuracy/oracle battery, the full
clippy matrix, `no_std`, the doc-warnings gate, and both wasm32 frontends are all green with zero
regressions.
`v1.3.0` is the HD texture pack release (`hd-pack` feature, off by default). Landed: a
palette-inclusive XXH3-64 tile-identity hash computed in `rustysnes-ppu` (allocation-free on the
rendering hot path) and a write-only per-pixel `Ppu::tile_tags()` side-buffer, proven
byte-identical to every prior release when the feature is off or tagging is left at its `false`
default; a frontend `pack.toml` loader + pure-Rust PNG decoder (path-traversal-safe,
duplicate-hash-rejecting — `crate::hd_pack`); a pure CPU compositor fully unit-testable without a
GPU adapter (`crate::hd_compositor`); a Settings → Video pack selector with `config.toml`
persistence and automatic re-selection on ROM load; and the compositor wired into the live wgpu
present path (`Gfx`'s streaming texture now grows on demand to fit a composited frame, capped at
this device's actual texture-dimension limit, with the no-pack-active path staying
pixel-identical to before). Fixed at a 2× upscale for now (not yet user-configurable); not wired
for the `emu-thread` build. See `docs/adr/0010`, `docs/ppu.md` §HD texture pack `TileTag`
recording hook, and `docs/frontend.md` §HD texture packs. Full workspace suite, the
`--features test-roms` accuracy/oracle battery, the full clippy matrix, `no_std`, the
doc-warnings gate, both wasm32 frontends, and `rustysnes-libretro` are all green with zero
regressions.
`v1.4.0` is the emu-thread-parity + accuracy-bugfix release, closing out the post-`v1.3.0`
patch cluster. The fullscreen crash on monitors wider/taller than 2048px is fixed (`Gfx` now
floors its requested wgpu limits against the real adapter), RustyNES-parity Window Size presets
(1x-4x, default 3x) landed, `rustysnes-libretro` gained Mouse/Super Scope/Multitap peripheral
negotiation, and the **open-bus-via-DMA-latch bug is FIXED** (cross-checking directly against
ares' and bsnes' `CPU::Channel::readA`/`readB`/`writeA`/`writeB` — DMA/HDMA reads update
`open_bus`, writes never do; `superfx_boots_live_and_deterministic`'s 24 golden hashes re-blessed
with that citation trail as justification — see `docs/scheduler.md` §Open bus via DMA/HDMA).
`emu-thread`'s cheats/watchpoints/breakpoints/port2-peripheral/voice-mute re-sync is now
mechanically ported: `EmuCore` is the same `Arc<Mutex<...>>` both the winit thread and the emu
thread share, so re-syncing from `render`'s existing brief lock — once per present, before the
emu thread's next `run_frame()` — is sufficient; none of it needs to run ON the emu thread
itself. Run-ahead and netplay-aware pause are now ported too: `drive_one` takes the same
`run_ahead > 0` branch the synchronous path already does, calling
`crate::rewind::step_with_run_ahead` only when run-ahead is actually configured and otherwise
publishing straight from the borrowed framebuffer slice with zero extra allocation (a real
per-frame cost regression caught in review before merge — the helper's own `frames == 0` fast
path still does an avoidable `to_vec()` copy). `NetplayState::drive` — previously entirely
unreachable under `emu-thread` (dead code inside a `#[cfg(not(feature = "emu-thread"))]` block,
i.e. netplay was silently non-functional in threaded builds) — now runs once per present from the
winit thread while `EmuControl::netplay_paused` idles the emu thread under the same `EmuCore`
mutex (TOCTOU-safe: the flag is only trusted once re-checked under the lock in `drive_one`).
`PresentBuffer` was extended to carry the framebuffer's `(width, height)` alongside its bytes
(previously only safe because every published frame was exactly `emu.framebuffer()`'s current
dims; run-ahead's peeked frame can differ across a hi-res-mode-toggle-mid-peek edge case); a
second review finding caught that the consumer's own `dims` fallback (used when nothing new has
been published yet) still read the live, possibly-moved-on `emu.fb_dims()` instead of the dims
that actually match whatever bytes are sitting in `present_staging` — fixed via a tracked
`Active::present_dims` field, updated only when a new frame is actually taken. Movies/scripting/
RetroAchievements/rewind-recording remain unported — but reclassified as an intentional, permanent
architecture boundary (confirmed by directly reading RustyNES's own mature 914-line
`emu_thread.rs`, which doesn't port any of these to its thread either), not a parity gap — see
`emu_thread.rs`'s own module doc. Full workspace suite, the `--features test-roms` accuracy/oracle
battery, the full clippy matrix (including two new CI gates: `emu-thread` was never actually
clippy-gated before this release, and its own unit tests were never actually executed in CI),
`no_std`, the doc-warnings gate, both wasm32 frontends, and `rustysnes-libretro` are all green
with zero regressions.
`v0.5.0` closed out the accuracy-pass-rate dashboard (see "Accuracy dashboard" below) and the
full named hardware-gotcha regression list — every item fixed, correctly reclassified as an
intentional non-goal, or honestly researched-and-deferred with a full mechanism write-up. `v0.6.0`
closed out release engineering and doc parity — `security.yml`, checksummed release assets,
automated release-cutting (`release-auto.yml`), the `lint` job's `cargo doc` gate,
`docs/DOCUMENTATION_INDEX.md`, `docs/benchmarks.md`, `docs/audit/`, and 9 total ADRs
(`to-dos/VERSION-PLAN.md`'s v0.6.0 section has the per-item detail). `v0.7.0` implements true
512-px hi-res (Modes 5/6) output — `docs/ppu.md` §Hi-res (Modes 5/6) color-math precision has the
full mechanism (a genuine one-pixel-clock-delayed DAC pipeline, verified against ares' primary
source) and honest verification status (unit-test-proven, non-regression-proven; real-title
validation against Marvelous/SA-1 attempted and not achieved — the title never entered hi-res in
a 1200-frame headless run, and no working GUI environment was available to drive an `ares`
reference-screenshot comparison, both honestly tracked as open, not claimed done). The save-state
`FORMAT_VERSION` bumped `1`→`2` for this — its first real bump — closing the `v1.0.0` gate's
backward-compat-fixture gap early (`docs/adr/0006-save-state-format.md`'s bump log).
`v0.8.0` lands Phase 8's core (the debugger overlay + 65C816 read/write watchpoints +
`rustysnes_cpu::disasm`, Lua scripting/TAS, cheat codes, the real wasm frontend, GGPO-style
rollback netplay, RetroAchievements — `to-dos/VERSION-PLAN.md`'s ladder has the per-item detail)
alongside the mid-scanline/HDMA-driven register timing fix and a systemic cart-layer open-bus
fix. `v0.9.0` closes out everything still open after that: T-81-001 PR B (the 65C816 disassembly
view/PC breakpoints/step controls, plus a new non-intrusive `Bus::peek` read), Phase 7's niche
peripherals (Mouse/Super Scope/Super Multitap — `rustysnes_core::controller`, a real
serial-shift-register protocol ported from ares, not a stub), and a continued SPC7110
investigation that reclassified its boot gap as a ROM-identity issue rather than an open bug
(`docs/audit/spc7110-boot-crash-2026-07-08.md`). The save-state `FORMAT_VERSION` bumped `2`→`3`
for the peripherals' runtime state (`docs/adr/0006`'s bump log).
**Phases 1 (CPU + golden oracle)
and 2 (scheduler + video) are functionally complete** — the 65C816 passes the
SingleStepTests/65816 oracle to 0-diff (state + cycles), and the machine **boots and runs real
ROMs**: the master-clock lockstep scheduler + bus memory map + DMA/HDMA + the dual-chip PPU
produce a deterministic framebuffer. gilyon's on-cart CPU suite reports "Success" (all 1107
tests), and the undisbeliever PPU/DMA/HDMA suite renders bit-deterministic golden framebuffers.
Audio (Phase 3) is complete. Coprocessors (Phase 4/7): Core/Curated (DSP-1, Super FX, SA-1) plus
the BestEffort DSP-2/DSP-4/ST010, S-DD1, CX4, and OBC1 are implemented and validated against
real commercial ROMs (see the coprocessor matrix below); standalone S-RTC is now implemented
(`v0.4.0`, unit-tested only — no commercial dump in the local corpus); SPC7110 is implemented,
with several confirmed addressing/timing bugs found and fixed through `v0.8.0` — and the boot
crash this local corpus's one available dump hit turned out to be a fan-translation ROM hack, not
the original cartridge (three independent confirmations: a SHA256 mismatch against `ref-proj/
ares`'s database, a checksum inconsistency, and a public forum thread describing the patch's
non-standard memory map — `docs/audit/spc7110-boot-crash-2026-07-08.md`), so this is now a
ROM-sourcing gap rather than an open emulation bug; ST018 is now
implemented (`v0.4.0`, unit-tested only — no commercial dump in the local corpus).
Save-states (`v0.2.0`), rewind, run-ahead, PAL region auto-detection, and ExLoROM (all `v0.3.0`)
are implemented and shipped — see the frontend and memory-map-model tables below.

## Accuracy dashboard

RustySNES doesn't have one monolithic all-in-one oracle ROM the way RustyNES's AccuracyCoin does.
An early skeleton for exactly that approach exists (`rustysnes-test-harness::accuracy_battery`,
ticket T-04) but was never implemented and has since been superseded — no publicly available
SNES ROM plays the AccuracyCoin role, and the composed multi-suite approach below is what
actually shipped; that skeleton is tracked as dead code to remove in a follow-up, not a competing
source of truth. The accuracy story here is instead a **composed multi-layer battery** across
independently-sourced suites (`docs/testing-strategy.md`). Rather than force these heterogeneous
suites into one artificial summed fraction (a 5.12M-case CPU oracle would swamp a 4-ROM audio
suite in any raw sum, which would be misleading, not informative — this project's honesty-gate
posture, `docs/adr/0003`, applies to how numbers are presented too), each layer's own status is
tracked here, always current, reaffirmed every release:

| Layer | Status | Detail |
|---|---|---|
| CPU (65C816) per-opcode oracle | ✅ **0-diff vs. reference** | 5,119,999 / 5,120,000 (SingleStepTests/65816; the one residual is a documented inter-reference divergence, `docs/adr/0002`, not a bug — not literally 0 of 5,120,000, but 0 against the chosen reference behavior every other test vector agrees on) |
| SPC700 per-opcode oracle | ✅ **0-diff, 100.00%** | 256,000 / 256,000 (SingleStepTests/spc700) |
| On-cart CPU (gilyon `cputest-basic`) | ✅ **green** | 1107 / 1107 "Success" |
| PPU/DMA/HDMA golden framebuffer (undisbeliever) | ✅ **green, deterministic** | 29 / 29 ROMs bit-identical across runs |
| Audio boot+run (blargg `spc_*`) | ✅ **literal PASS, all 4** | `spc_smp`, `spc_timer`, `spc_mem_access_times`, `spc_dsp6` — asserted, not a determinism proxy |
| Core/Curated coprocessors (oracle-gated) | ✅ **3 / 3, honesty gate green** | DSP-1 (4 commercial ROMs), Super FX/GSU (58 Krom ROMs + per-opcode suite), SA-1 (18 commercial carts) — `ORACLE_COPROCESSORS` |
| BestEffort coprocessors, real-title validated | ✅ **6 / 9** | DSP-2, DSP-4, ST010, S-DD1, CX4, OBC1 — each boots a real commercial title to real gameplay content |
| BestEffort coprocessors, unit-test only | ⚠️ **3 / 9** | SPC7110 (the one available local dump turned out to be a fan-translation ROM hack that needs a patch-only memory region no real cartridge has — `docs/audit/spc7110-boot-crash-2026-07-08.md`; a genuine original-cartridge dump, sha256 `69d06a3f3a4f3ba769541fe94e92b42142e423e9f0924eab97865b2d826ec82d`, is the ROM-sourcing gap now tracked in `docs/rom-test-corpus.md`), ST018, S-RTC (neither has a commercial dump in the local corpus) |
| Determinism contract | ✅ **proven** | bit-identical framebuffer/audio across runs; save-state round-trip proven across all three board tiers (no-coprocessor, Curated, BestEffort) |

**Named residuals, tracked not hidden:** the 65816 `e1.e` divergence (`docs/adr/0002`); DSP-3 and
ST011 have no board wired (no verified board/window entry to pin against, `necdsp_variant.rs`);
SPC7110's local test ROM being a fan-translation, not an original-cartridge dump, pending a
correctly-sourced ROM (`docs/cart.md` §SPC7110); PAL and ExLoROM both lack
golden-ROM-boot proof (no ROM in the local corpus for either); hi-res (Modes 5/6) output is
implemented and unit-verified (`v0.7.0`) but has no real-title validation — neither named
motivating commercial title (Bishoujo Janshi Suchie-Pai: no local dump; Marvelous/SA-1: dumped
but never observed entering hi-res in a 1200-frame headless run) has confirmed the mechanism
against actual hi-res game content, `docs/ppu.md` §Hi-res (Modes 5/6) color-math precision.
`v0.5.0 "Fidelity"` is where the next layer — a named hardware-gotcha regression suite (DRAM
refresh, HDMA mid-scanline placement, the DMA/HDMA-collision crash quirk, open-bus-via-HDMA-latch,
true mid-dot writes, hi-res color-math precision, the `$4203` double-write edge case) — gets
added to this table; hi-res color-math precision itself closed in `v0.7.0 "Resolution"`.

## Subsystem progress

| Crate | Chip | State |
|---|---|---|
| `rustysnes-cpu` | WDC 65C816 (5A22) | **Phase 1 complete — 65816 oracle 0-diff (state+cycles), all 256 opcodes × modes, native+emulation, REP/SEP/XCE** |
| `rustysnes-ppu` | PPU1 (5C77) + PPU2 (5C78) | **Phase 2 — BG 0-7 + Mode 7 + 128-sprite OAM + color math + windows + dot/HV timeline; per-scanline compositor renders each line at `RENDER_DOT` (dot 276, one dot before that line's own HDMA run can mutate the registers the composite reads) — a per-line HDMA-driven register write only becomes visible starting the following line, matching real hardware, landed `v0.8.0` (`docs/ppu.md` §Mid-scanline/HDMA-driven register timing; SA-1's `SD F-1 Grand Prix` + 24 Super FX/GSU goldens updated, each independently row-level-verified, not blindly re-blessed). True 512-px hi-res (Modes 5/6, pseudo-hires) output landed `v0.7.0` — a genuine one-pixel-clock-delayed dual-column DAC pass mirroring ares' `PPU::DAC`, unit-verified + non-regression-verified, real-title validation still open (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision). Color fixes (ares pixel-diff vs SMW): color-math subscreen-backdrop addend = the COLDATA fixed color (blue-sky/black-bg fix), and the BG tilemap palette-group offset folded into the CGRAM index (washed multi-palette art fix); undisbeliever golden stays 29/29. `v1.3.0`: `hd-pack` feature — `hdtag::hash_tile` (palette-inclusive XXH3-64, allocation-free) + a write-only per-pixel `Ppu::tile_tags()` side-buffer for HD texture pack tile identity, off by default and compiled out entirely when the feature is off; proven byte-identical to every prior release when tagging is left off (`docs/ppu.md` §HD texture pack `TileTag` recording hook)** |
| `rustysnes-apu` | SPC700 (S-SMP) + S-DSP + ARAM | **Phase 3 — SPC700 oracle 0-diff; S-DSP behavioral; integrated into the machine: the 4 `$2140-$2143` ports route through the real `Apu`, the integer-accumulator async resync clocks the SMP in **cycle-exact sub-instruction lockstep** (`68_352/715_909`, ADR 0004), SMP base-clock + timer + DSP rates ares-correct; blargg `spc_*` boot+upload+run bit-deterministically; the **timer-phase fix** (timebase/timers clocked before the write side effect, ares/Mesen2-correct) + the **DSP GAIN mode-7 threshold fix** (unsigned `hidden_env >= 0x600`, blargg/ares-correct) drive **all four `spc_*` (`spc_smp`/`spc_timer`/`spc_mem_access_times`/`spc_dsp6`) to literal `PASSED TESTS`** (asserted)** |
| `rustysnes-cart` | LoROM/HiROM/ExHiROM + coprocessors | **Phase 2 base map modes + Phase 4 coprocessors: chipset-byte detection, the shared µPD77C25/µPD96050 LLE engine + DSP-1 board (real DSP-1 games with user-supplied firmware), and the Super FX/GSU — full Argonaut RISC core (`coproc::gsu`) + `SuperFxBoard` (`coproc::superfx`), host-synced on the Go flag, boots the Krom GSU suite (`superfx_oncart`). SA-1 next** |
| `rustysnes-core` | Bus + master-clock scheduler + DMA/HDMA | **Phase 2 — master-clock lockstep (6/8/12 access map), full memory decode, CPU regs + mul/div, GP-DMA + HDMA, NMI/HV-IRQ. `v0.9.0`: `rustysnes_core::controller` — Mouse/Super Scope/Super Multitap, the real 2-bit-per-clock (`data1`/`data2`) serial-shift-register protocol ported from ares' `sfc/controller/{mouse,super-scope,super-multitap}`, including WRIO (`$4201`/`$4213`) IOBIT plumbing and the Super Scope's PPU H/V-counter beam-latch (`Ppu::latch_hv_counters`); `Bus::set_port_device`/`set_mouse`/`set_superscope`/`set_multitap_pad`, save-stated (`FORMAT_VERSION` 2→3), 14 unit tests. `v1.2.0`: the pure `EmuCore` embedding facade relocated here from `rustysnes-frontend` (`facade` module, `std`-only, conditional `no_std` at the crate root) — load/step/framebuffer/audio/save-state, for any headless embedder; plus new `Bus::wram`/`wram_mut`/`Ppu::vram`/`vram_mut` raw memory-map accessors** |
| `rustysnes-frontend` | egui shell + audio ring + pacing | **Phase 5 — PLAYABLE: native winit 0.30 + wgpu 29 + egui 0.35 + cpal shell boots real commercial ROMs with picture, sound, and control. Video: PPU BGR555→RGBA8 decode, aspect-correct (4:3) sub-rect letterbox blit. Audio: S-DSP 32 kHz FIFO → producer-side linear resampler (DRC-paced) → lock-free ring → cpal stereo stream. Input: keyboard + gilrs gamepad → `bus.set_joypad`. ROM load (+ coprocessor-firmware + `.srm` SRAM auto-load), Reset / Power-Cycle / Pause wired. wasm32 target compiles: `wasm-winit` (default) is the SAME `App`/egui shell native uses, verified end-to-end with a real headless-browser load (`docs/frontend.md` §wasm); `wasm-canvas` is a lighter, independently-functional canvas-2D fallback. Proven by the `playable_smoke` headless gate (Super Mario World: 256×224 structured frame + 63,975 non-silent samples over 120 frames) + an xvfb launch run. Save-states landed (`docs/adr/0006`: a versioned `System::save_state()`/`load_state()` envelope across every `Board` + `Cpu`/`Ppu`/`Apu`/`Bus`, proven by a round-trip determinism test), wired to a quick-save menu slot; rewind (a bounded ring buffer of full snapshots, `crate::rewind::RewindBuffer`) and run-ahead (N-frame peek-and-discard, `crate::rewind::step_with_run_ahead`) landed in `v0.3.0 "Continuum"` — both config-driven and off by default (capacity/frames `0`), proven by tests that hand-assemble a tiny 65C816 program for a real per-frame state signal. Pacing/present fixes: wall-clock fixed-timestep drive (emulation tracks the region rate, not the display refresh — fixes the ~2–3× over-speed on high-refresh panels), a real smoothed FPS meter (was hardcoded `0.0`), and a live present-mode reconfigure on the Settings → Video toggle. `v0.8.0`: Settings → Input gained a controller-port-2 peripheral selector (`config.port2_peripheral` → `Bus::set_port_device`, re-synced every frame alongside cheats/watchpoints) — correctly changes emulated behavior, but live host-input capture (a real mouse pointer, extra gamepads) is not yet wired (`docs/frontend.md` §Peripherals has the precise remaining scope). `v0.8.0` T-81-003: a Tools → Cheats… window (Game Genie / Pro Action Replay, decode in `rustysnes_core::cheat`, applied via a `Bus::read24` CPU-read intercept — not a WRAM poke, since real codes overwhelmingly target cartridge ROM) behind the `cheats` flag, native + `wasm32`. `v1.0.0`: `Board: Send` unblocks `emu-thread` to compile/test/lint clean (still off-by-default, not yet feature-parity — see `docs/frontend.md`); a disk-backed 10-slot thumbnail Save States manager (`save_states.rs`, additive alongside the quick-save slot); a working Settings → Input key-rebind grid; light/dark/system themes; fullscreen; 25%–300% speed presets (scaling both `Pacer` and the audio DRC ratio); a Performance panel with FPS/frame-time/audio-health + a rolling sparkline; a first-run welcome modal; and a `full` feature + `cargo full-build`/`full-run` aliases. `v1.0.1`: 8 per-voice audio mute checkboxes (Settings → Audio, `config.audio.voice_mutes` → `Dsp::voice_output`, no real hardware register behind it) and a fixed global keyboard hotkey table (`Escape`/`F1`-`F5`/`F9`/`F11`/`F12`/`Space`/`` ` ``, checked in `window_event` before gameplay latching, suppressed while an egui widget has keyboard focus) — everything used to be menu-bar-only. `v1.1.0`: `emu-thread` gained real audio output (a thread-owned `AudioProducer`) and a proper pause/ROM-loaded/speed lifecycle (`EmuControl`, driving a thread-owned `Pacer`) plus a `PresentBuffer` lock-free framebuffer handoff, closing its two biggest documented gaps — still not full parity with the synchronous drive (cheats/watchpoints/breakpoints/port2-peripheral/voice-mutes/run-ahead/rewind/movies/scripting/netplay-pause/RetroAchievements remain unported, `emu_thread.rs`'s own module doc has the exact list)** |
| `rustysnes-netplay` | rollback netplay | **T-82-002 — GGPO-style 2-player rollback netcode, ported from RustyNES's `RollbackSession` shape (scoped to 2 remote players — a session-topology choice, not a core limitation: the core gained real Super Multitap emulation in `v0.9.0`, but rollback-netcode's 2-peer resimulation model is a separate concern from local extra-player input, matching how a netplay session and `emu-thread`'s single-player pacer already stay mutually exclusive by session type). Bit-identical resimulation proven under both ideal and adverse (latency/jitter/10% loss) conditions (`tests/determinism.rs`); a real UDP transport (OS-level loopback tested) + a wasm32-clippy-verified WebRTC transport. Frontend wiring (Tools → Netplay…, its own drive loop independent of `emu-thread`) behind the `netplay` flag is native/UDP only — the browser SDP-negotiation UI is an honestly deferred, separate scope** |
| `rustysnes-cheevos` | RetroAchievements (opt-in FFI) | **T-82-003 — native FFI bridge around the vendored `rcheevos` `rc_client` C API (MIT, vendored verbatim from RustyNES's own copy, ABI-pinned via a `size_of`-vs-C-`sizeof` guard test). Achievement logic evaluates SNES WRAM only (`ra_addr_to_snes`, verified against the real `RetroAchievements/RASnes9x` integration source — cartridge SRAM is an honest scope cut). The `RustySNES/<ver> rcheevos/<ver>` User-Agent is regression-tested. Frontend wiring (Tools → RetroAchievements… login window + a per-frame `do_frame` hook + unlock toasts) behind the `retroachievements` flag is native-only; leaderboards/rich-presence have no UI panel yet** |
| `rustysnes-script` | Lua scripting / TAS API | **T-81-002 — sandboxed `mlua` 5.4 scripting (WRAM read/write + per-frame callback, runaway-loop instruction budget, `io`/`os`/`require`/`debug` denied) + a `rustysnes_core::movie` TAS format (deterministic input log, power-on or embedded-save-state start, replay-verified bit-identical vs a real committed ROM); behind the `scripting` flag, wired into a Tools menu, native-only** |
| `rustysnes-test-harness` | golden-log + JSON-oracle + screenshot baseline | **implemented and in active use** — the accuracy oracle (65816/SPC700 SingleStepTests runners, gilyon/undisbeliever/blargg golden-log gates, the `*_oncart` per-coprocessor commercial-ROM validation harnesses, and `commercial_screenshots.rs` the boot-screenshot generator behind `test-roms`/`commercial-roms`) |

## Accuracy — per-suite pass counts

| Suite | Layer | License posture | Pass | Total |
|---|---|---|---|---|
| SingleStepTests/65816 (JSON) | CPU per-opcode oracle | **self-gen committed + upstream cross-check (external)** — ADR 0005 | **5,119,999** | 5,120,000 |
| SingleStepTests/spc700 (JSON) | SPC per-opcode oracle | MIT (committable) | **256,000** | 256,000 (0-diff) |
| gilyon/snes-tests (cputest-basic .sfc) | CPU on-cart (boots on `System`) | MIT (committed) | **1107** | 1107 (= "Success") |
| undisbeliever/snes-test-roms (.sfc) | PPU/DMA/HDMA hardware (golden framebuffer) | Zlib (committed) | **29** | 29 (deterministic) |
| blargg `spc_*` (spc_dsp6 / mem_access / smp / timer) | cycle-accurate SPC/DSP (cycle-stepped S-DSP + timer-phase + GAIN mode-7 fixes) | unstated (external) | **4 boot+run det.; 4 literal PASS** | 4 (all → literal `PASSED TESTS` asserted: `spc_smp`/`spc_timer`/`spc_mem_access_times` via timer-phase fix, `spc_dsp6` via DSP GAIN mode-7 unsigned-threshold fix) |
| DSP-1 commercial dumps (`dsp1_oncart`) | DSP-1 coprocessor (boots on `System` w/ user firmware) | ROMs+firmware gitignored (golden committed) | **4 boot+det.** | 4 (detection + RQM-access + golden + firmware-diff) |
| Krom GSU suite (`superfx_oncart`) | Super FX/GSU coprocessor (boots on `System`) | CC0/homebrew (gitignored; golden committed) | **58 boot+live+det.** | 58 (SuperFx detect + GSU-executed + FillPoly-into-RAM plot pipeline + golden) |
| Commercial SA-1 carts (`sa1_oncart`) | SA-1 second-65C816 coprocessor (boots on `System`) | ROMs gitignored (golden committed) | **18 boot+det.; 10 SA-1-live** | 18 (Sa1 detect + S-CPU↔SA-1 traffic, all 18; aggregate SA-1-executed floor ≥8 — observed 10, incl. Super Mario RPG / both Kirbys / PGA Tour 96 / Power Rangers Zeo at millions of SA-1 cycles; deterministic golden) |
| 240p Test Suite (SNES) | video / overscan | GPLv2 (run-only) | 0 | TBD |
| Visual golden corpus (`tests/golden/`) | framebuffer / audio hashes | own (committed) | **29** | 29 |

- **CPU 65816 oracle (0-diff):** **100.00%** — 5,119,999 / 5,120,000 full passes
  (state + RAM + cycle count) across all 512 opcode files × 10,000 tests each, native +
  emulation. The one
  residual is a single `e1.e` (`SBC (dp,X)`, emulation) test exercising the bsnes `readDirectX`
  `DL!=0` high-byte wrap that the rest of the SingleStepTests set does **not** model — a
  documented inter-reference divergence (`docs/adr/0002` posture), not point-fixed. Measured via
  `tests/cpu_oracle.rs` against the gitignored external set (ADR 0005: cross-check only, never a
  CI dependency). No textual Nintendulator-style 65816 log exists — this JSON oracle replaces it
  (`docs/testing-strategy.md`).
- **gilyon on-cart CPU (Phase 1's deferred criterion, unblocked by the Phase-2 boot):**
  `cputest-basic.sfc` boots on the integrated `System` and renders **"Success"** with all 1107
  65C816 instruction/addressing-mode tests run (`tests/gilyon_oncart.rs`). `cputest-full.sfc`
  wedges at test 39 (`adc ($10,s),y` routed through the ROM's RAM-resident BRK handler under
  `DBR=$7E`) — a narrow ROM-dispatch edge documented as a residual; the op itself is oracle-clean.
- **undisbeliever PPU/DMA/HDMA + golden framebuffer:** all **29** ROMs boot through the
  CPU+scheduler+bus+DMA/HDMA+PPU path and produce **bit-deterministic** framebuffer hashes
  matching the committed baseline `tests/golden/undisbeliever-framebuffer.tsv`
  (`tests/undisbeliever_golden.rs`). This is the Phase-2 deterministic-golden-framebuffer gate.
- **SPC700 per-opcode oracle (0-diff):** **100.00%** state + cycle count over all 256 opcodes
  (`tests/spc700_oracle.rs`): 12,800 committed-sample tests in-tree, 256,000 in the full external
  tier. Unaffected by the Phase-3 SMP base-clock correction (the oracle replays against its own
  flat bus, measuring access count, not the integrated wait-state model).
- **blargg `spc_*` (Phase-3 audio integration — cycle-exact SMP↔CPU + cycle-stepped S-DSP):** the
  SMP advances in **cycle-exact sub-instruction lockstep** with the main CPU (`Apu::advance_smp_cycle`
  releases one SMP base clock at a time from a recorded micro-op timeline; each SMP→CPU port write
  becomes visible at the precise base cycle it lands on), **and the S-DSP is now itself
  cycle-stepped** — it runs its 32-step ares micro-sequence one `Dsp::tick` per 2 SMP base clocks
  (32 ticks = one 32 kHz sample) instead of a whole sample per 64 clocks, so a mid-instruction
  DSP-register read sees cycle-correct OUTX/ENVX/ENDX/envelope state (`docs/apu.md`
  §cycle-accurate DSP, §cycle-exact). All four ROMs (`spc_smp`, `spc_timer`, `spc_mem_access_times`,
  `spc_dsp6`) boot, complete the IPL upload handshake, run the SPC700 program, and stream their
  detailed result text. The gate (`tests/blargg_spc.rs`) **decodes and asserts the real on-screen
  verdict**, retaining the deterministic + baseline-hash assertion (`spc_timer` / `spc_mem_access_times`
  re-blessed in `tests/golden/blargg-spc.tsv` for the timer-phase timing change; `spc_smp` /
  `spc_dsp6` hashes unchanged); it is **not** weakened to determinism-only. The **timer-phase fix**
  (`RecordingSmpBus::write` now advances the SMP timebase + clocks the timers **before** the write
  side effect, matching ares `step()` / Mesen2 `Spc::Write` `IncCycleCount`-first; `docs/apu.md`
  §timer phase) drove `spc_smp`, `spc_timer`, `spc_mem_access_times` to blargg's literal
  `PASSED TESTS`; the **DSP GAIN mode-7 threshold fix** (the bent/two-slope GAIN increase compares
  its internal envelope latch against `0x600` **unsigned** — blargg `SPC_DSP` `(unsigned) hidden_env`
  / ares `(u32) _envelope` — so a latch left negative by a prior GAIN decrease still trips the
  reduced slope; `docs/apu.md` §DSP GAIN mode-7 threshold) drove `spc_dsp6` to `PASSED TESTS` too.
  **All four ROMs now reach blargg's literal `PASSED TESTS`** (the gate asserts each, not a
  determinism proxy). The per-opcode SPC700 (oracle 0-diff) + per-tick S-DSP math is correct.
- **Master-clock totals:** a booted NTSC frame advances ≈357,374 master clocks (spec ≈357,368),
  confirming the 6/8/12 access map + dot timeline.
- **Accuracy battery (the composed multi-layer oracle):** see the "Accuracy dashboard" section at
  the top of this document for the always-current, per-layer status table — the CPU, SPC700,
  on-cart CPU, PPU/DMA golden, and audio layers are all green; the coprocessor layer is 6/9
  BestEffort boards real-title validated plus all 3 Core/Curated boards honesty-gate green.
- **Determinism contract:** the framebuffer is verified bit-identical across runs (same seed +
  ROM ⇒ identical hash) for all 29 undisbeliever ROMs; the full save-state round-trip (save,
  restore onto a fresh `System`, continue both, compare framebuffer + audio) is proven for a
  no-coprocessor ROM, a `Curated` Super FX ROM, and a `BestEffort` coprocessor ROM
  (`docs/adr/0006`, `save_state_determinism.rs`).

## Coprocessor / board tier matrix (honesty gate: `docs/adr/0003`)

`boards_tiered = true` — the honesty gate is real: **no BestEffort board backs the oracle.**

| Chip | Core | Tier | Shared LLE core | State |
|---|---|---|---|---|
| DSP-1/1A/1B | µPD77C25 | **Core/Curated** | µPD77C25 (shared, 6 chips) | **implemented** — full µPD7725 LLE engine (`coproc::upd77c25`) + `Dsp1Board` (Lo/HiROM DR/SR windows). Boots Super Mario Kart / Pilotwings / Super Bases Loaded 2 / Aim for the Ace on the full System with user-supplied `dsp1*.rom`; deterministic golden + firmware-differential + RQM-handshake access gate (`dsp1_oncart`, 4 ROMs). Honesty gate green (`ORACLE_COPROCESSORS` ∋ DSP). Firmware gitignored, never committed |
| Super FX / GSU-1/2 | Argonaut RISC | **Core/Curated** | — (cart ROM) | **implemented** — full GSU core (`coproc::gsu`: complete Argonaut RISC instruction set + ALT-mode machine, the multiplier, ROM/RAM buffers, opcode cache, the branch-delay pipeline, and the PLOT/RPIX pixel-plot pipeline) + `SuperFxBoard` (`coproc::superfx`: LoROM Super FX map, GSU register window, CPU↔GSU ROM/RAM arbitration). No chip dump — the GSU program is in cart ROM; host-synced on the Go flag (`run_until_stopped`, the DSP-1 `run_until_rqm` analogue), no core tick. Validated by `superfx_oncart` (58 Krom GSU ROMs: SuperFx detection + GSU-executed liveness + a FillPoly-into-RAM plot-pipeline assertion + deterministic golden) + the per-opcode `GSUTest` suite + engine unit tests. Honesty gate green (`ORACLE_COPROCESSORS` ∋ SuperFx) |
| SA-1 | 65C816 @ 10.74 MHz | **Core/Curated** | (reuses CPU core) | **implemented** — the full SA-1 system (`coproc::sa1::Sa1Board`: the `$2200–$23FF` register file, Super-MMC ROM banking, BW-RAM with the 2/4 bpp bitmap + linear projections + write-protect, 2 KiB I-RAM, the arithmetic unit, var-len bit unit, H/V timer, and normal + type-1/2 character-conversion DMA) + the **second 65C816** instantiated and stepped in `rustysnes-core` (deterministic master-clock catch-up via the `Board` second-CPU hooks — the crate graph keeps the CPU core out of the cart crate). No chip dump — the SA-1 program is in cart ROM. Validated by `sa1_oncart` (18 commercial SA-1 carts: detection + S-CPU↔SA-1 traffic for all 18, an aggregate "SA-1 CPU executed" liveness floor ≥8 — observed 10 incl. Super Mario RPG / both Kirbys / PGA Tour 96 / Power Rangers Zeo — + deterministic golden) + board unit tests. The main CPU oracle stays 0-diff (SA-1 stepping is gated to SA-1 carts and bounded by the untouched master clock). Honesty gate green (`ORACLE_COPROCESSORS` ∋ Sa1) |
| DSP-2 / DSP-4 | µPD77C25 | BestEffort | µPD77C25 (shared) | **implemented** — `coproc::necdsp_variant::NecDspVariantBoard` reuses the DSP-1 µPD7725 LLE engine, title-detected (`Variant::detect`). DSP-2 uses the generic bit-0 DR/SR split; DSP-4 needed a DSP-1-style half-boundary split instead (found + fixed against a real Top Gear 3000 boot-time 16-bit hardware check). Validated against real Dungeon Master (DSP-2) and Top Gear 3000 (DSP-4) — real title + gameplay content |
| ST010 / ST011 | µPD96050 | BestEffort | µPD96050 (shared) | **implemented** — same `NecDspVariantBoard`, µPD96050 DR/SR bit-0 split + the DP battery data-RAM window. Validated against real F1 ROC II — real title + gameplay content |
| S-DD1 | Nintendo ASIC | BestEffort | — | **implemented** — `coproc::sdd1`: Golomb-code + adaptive-binary-probability decompressor (`Decompressor`, ports ares' constant tables verbatim) streamed during fixed-address DMA via a new `Board::notify_dma_channel` hook (`rustysnes-core` snoops `$43n2-$43n6` writes). No chip dump — decompresses the cart's own compressed ROM. Validated against real Star Ocean and Street Fighter Alpha 2 — real title + gameplay content, after fixing a `u8`-shift-by-8 overflow bug in the codeword reader (ares' `n8` implicitly widens through C++ int promotion; the Rust port needed an explicit `u32` widen) |
| SPC7110 (+RTC-4513) | Hudson ASIC | BestEffort | — (frozen RTC) | **implemented, multiple addressing bugs fixed; the local test ROM turned out to be a fan-translation hack, not the original cartridge — a ROM-sourcing gap, not an open bug** — `coproc::spc7110`: DCU (Hudson adaptive binary range coder over 1/2/4bpp planes), data-port unit, ALU (16×16 multiply, 32/16 divide), memory-control unit, plus a paired `coproc::epsonrtc::EpsonRtc` (RTC-4513, seeded to a fixed epoch — real wall-clock time would break the determinism contract). Wired against the one available ROM (Far East of Eden Zero / Tengai Makyou Zero): header detection fixed (title is "TENGAI MAKYO" not "…MAKYOU"; the `$F`-custom chipset-nibble gate excluded RTC carts' `$F9` byte). **`v0.4.0`:** found + fixed a real addressing bug — `datarom_read`/`mcurom_read`'s PROM/DROM lookups used a plain `offset % len` fold instead of ares' `Bus::mirror` block-mirror algorithm (only equivalent when the buffer size is a power of two), silently returning the wrong byte past the physical chip size. Fixing it (`spc7110::bus_mirror`) pushed the wild-PC excursion from ~20-30 frames into boot to ~90+ frames and it now self-recovers via a BRK/RTI loop instead of crashing outright. **`v0.8.0`:** ported ares' SPC7110 cothread timing exactly (DCU/ALU triggers deferred one master-clock tick, 9/9 unit tests) — watchpoint tracing confirmed it does NOT close the gap (those triggers are never written during this boot's crash path). A new `rustysnes_cpu::disasm` disassembler + branch trace then found the "stall" framing was itself incomplete (the CPU spends most of its time in a real VRAM-upload loop, bank `$4F`) and located two more real bugs, both cross-checked against `ref-proj/ares`'s own board database (`board: SHVC-LDH3C-01`): `$40-$7D` should be unmapped, not a `$C0-FF` mirror (an earlier session's unchecked claim otherwise — fixed); and the DROM buffer was 2 MiB oversized (committed dump is 7 MiB, real chips total 5 MiB — fixed). A fourth, systemic bug found alongside these: the cart layer's open-bus fallback (`Board::read24`'s `MappedAddr::Open` case) returned a hardcoded `0` instead of echoing the Bus's real open-bus latch (ares' `Bus::read(address, data)` pattern) — fixed via `Cart::read24` now taking the caller's open-bus byte and echoing it back for genuinely open addresses (`rustysnes-cart/src/lib.rs`), a fix that benefits every board, not just SPC7110. Re-tracing past the `JSL $4FFB80` dead end with this fix in place now shows a stable, harmless open-bus spin loop instead of the earlier BRK-storm — confirming bank `$4F` is truly inert on real hardware too. **Follow-up session, `v0.9.0`:** a full instruction trace (`disassemble_one` + `System::step_instruction`) showed the path to `JSL $4FFB80` is one unconditional chain of `JSL`s with no branch and no SPC7110 register touched anywhere upstream — ruling out a wrong-branch bug — which raised the real question: is this the right ROM? It is not. Three independent checks (a SHA256 mismatch against `ref-proj/ares`'s database entry for board `SHVC-LDH3C-01`; a header checksum that only validates against this file's non-standard 7 MiB size, not the real cartridge's 5 MiB; and a public nesdev.org thread documenting this exact fan-translation's memory map) confirm the local dump is the English translation patch, which adds a 1 MiB "Expansion ROM" at banks `$40-$4F` that exists only in the patch, never on real hardware — precisely the bank this `JSL` targets. RustySNES's `$40-$7D`-unmapped fix is correct for the real cartridge; **closing this out needs a genuine original-cartridge dump** (sha256 `69d06a3f3a4f3ba769541fe94e92b42142e423e9f0924eab97865b2d826ec82d`), tracked in `docs/rom-test-corpus.md` — full evidence chain: `docs/audit/spc7110-boot-crash-2026-07-08.md` |
| CX4 | Hitachi HG51B169 | BestEffort/Curated | — | **implemented** — clean-room `coproc::hg51b` (HG51B S169 core: sequential mask/value opcode decode transcribed from ares' `pattern(...)` strings, register file, cache, DMA, suspend/wait state machine) + `Cx4Board`. No chip dump — the CX4 program runs from cart ROM; only a small 3 KiB data-ROM constant table (`cx4.rom`) needs external supply. Validated against real Mega Man X2 and Mega Man X3 — real Capcom copyright screens + real gameplay, after fixing a real bug where DMA/cache work triggered while the chip was halted never ran |
| OBC1 | simple ASIC | BestEffort | — (HLE) | **implemented** — `coproc::obc1::Obc1Board`: dedicated 8 KiB RAM, a reprogrammable cursor (`$1FF5`/`$1FF6`) over 4-byte slots + a packed 2-bit-per-slot status byte (`$1FF4`). Validated against real Metal Combat: Falcon's Revenge — real gameplay cinematic |
| ST018 | ARMv3 | BestEffort | — (separate ARM LLE) | **implemented (`v0.4.0`)** — `coproc::armv3`: a full ARMv3 (ARM6-class, pre-Thumb) CPU core (barrel shifter/condition-codes/ALU, mode-banked register file, 3-stage pipeline, the complete instruction set) + `St018Board` (a single combined `0x28000`-byte firmware dump, the `$3800`/`$3802`/`$3804` handshake registers, 16 KiB work RAM), driven by `Board::coprocessor_tick` rather than the SA-1 second-CPU hooks since this ARM core is entirely self-contained in `rustysnes-cart`. Detected via a title match on the confirmed real cart, Hayazashi Nidan Morita Shogi 2 — an earlier investigation wrongly assumed Star Ocean (which uses S-DD1 only). No commercial dump in the local corpus — unit-test-level coverage only, not golden-framebuffer validated |
| S-RTC | Sharp S-RTC | BestEffort | — (frozen time) | **implemented (`v0.4.0`)** — `coproc::sharprtc::SharpRtcBoard`: a standalone Sharp S-RTC (Daikaijuu Monogatari II, ExHiROM), a DIFFERENT chip/protocol from the Epson RTC-4513 paired with SPC7110 above — a 2-register (`$2800`/`$2801`) handshake over a 13-slot decimal clock file (`Ready -> Command -> Read`/`Write` state machine), seeded to a fixed epoch, never wall-clock-advanced. Wraps a base `ExHiRom` board. No commercial dump in the local corpus — unit-test-level coverage only, not golden-framebuffer validated; header detection is a best-effort title match, matching the existing CX4/SPC7110 posture |

One **µPD77C25 / µPD96050 LLE engine** covers DSP-1/2/3/4 + ST010/011 (six chips, one engine).

## Memory-map model support

| Model | Header offset | State |
|---|---|---|
| LoROM ($20) | $007FC0 | **Phase 2 — score-based header detect + ROM/SRAM decode + mirroring** |
| HiROM ($21) | $00FFC0 | **Phase 2 — detect + $C0+ linear + $00-3F:$8000 window + SRAM** |
| ExHiROM ($25) | $40FFC0 | **Phase 2 — detect + A23-inverted extended-bank decode** |
| ExLoROM | $407FC0 (unofficial, no dedicated `$FFD5` value) | **v0.3.0 — detect + A23-inverted LoROM-windowed decode (sourced from bsnes's runtime board database); no real-ROM validation, no ExLoROM dump in the local corpus** |

## Region timing

| Region | Master clock | Lines/frame | State |
|---|---|---|---|
| NTSC | 21.477270 MHz | 262 (+1 interlaced) | **implemented + validated** — the default region; every oracle/golden/commercial ROM in the accuracy battery boots and runs at this timing |
| PAL | 21.281370 MHz | 312 (+1 interlaced) | **implemented + auto-detected** — `Bus::sync_region_from_cart` reads the cart header's destination-code byte at `System::reset()` and reconfigures the PPU's line count/status bit accordingly (`rustysnes-core::scheduler` tests `pal_cart_auto_detects_pal_region_on_reset`/`ntsc_cart_auto_detects_ntsc_region_on_reset` prove this end-to-end, including a full 312-line frame actually completing). No PAL ROM is available in the local test corpus yet, so golden-framebuffer validation against a real PAL cartridge boot (the accuracy-battery-equivalent proof NTSC already has) remains open — tracked, not silently claimed |

Region is **data, not a build fork** (`docs/scheduler.md`). The differing NTSC/PAL master-clock
*rate* (Hz) is a real-world audio/video pacing concern the frontend owns (`docs/adr/0004`); the
core's master-clock counter is a pure tick count, so nothing else in the core depends on which
oscillator frequency a real console would use — only the PPU's line-count/status-bit timeline
is region-dependent here.

## Version policy

- Start at **v0.1.0**. Additive features ship behind **default-off feature flags** so the
  shipped / native / `no_std` / wasm builds stay byte-identical with the flags off. Enforced in
  CI since `v0.8.0` T-81-004: `.github/workflows/ci.yml`'s `lint` job clippys every new
  `rustysnes-frontend` flag individually and combined, plus an explicit flags-off step
  (`--no-default-features --features wasm-winit,help-tui`) as a named regression guard
  independent of whatever `default` becomes.
- The **fractional-timebase refactor** is the one milestone expected to break byte-identity /
  save-state compatibility (`docs/adr/0002`) — and only happens if hard residuals warrant it.
- **Do NOT** import RustyNES engine-lineage "v2.0" anchors as RustySNES releases. The forward
  path is Phase 0 → v1.0.0 → (only then) the fractional-timebase refactor; see
  `to-dos/ROADMAP.md`.
