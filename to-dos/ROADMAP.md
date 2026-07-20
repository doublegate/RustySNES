# RustySNES — Roadmap

Entry point for project planning. Each phase below links to its overview; each phase contains
sprints; each sprint contains tickets with stable IDs `T-PS-NNN` (P = phase, S = sprint).
Reference ticket IDs in commit messages. `docs/STATUS.md` is the authoritative current-state
record; this file frames the phase line.

## Status

- **Current phase:** Phases 0–3 **complete** (CPU oracle 0-diff, scheduler + video, audio
  0-diff). Phase 4 (Core/Curated coprocessors: DSP-1, Super FX, SA-1) **complete**. Phase 7
  (BestEffort coprocessors) **complete**: DSP-2/DSP-4/ST010/S-DD1/CX4/
  OBC1/standalone S-RTC implemented + validated (S-RTC unit-tested only, no commercial dump
  available; DSP-3/ST011 explicitly deferred — no verified board/window entry, not a ROM-sourcing
  gap); SPC7110 implemented with multiple confirmed, fixed addressing/open-bus bugs through
  `v0.8.0` that materially improved but did not fully resolve its boot crash (`docs/cart.md`
  §SPC7110, `docs/audit/spc7110-boot-crash-2026-07-08.md`); ST018 is now implemented (unit-tested
  only, no commercial dump available); PAL region auto-detection is implemented and validated
  end-to-end (`Bus::sync_region_from_cart`; no golden-ROM-boot proof yet — no PAL ROM in the local
  corpus, see `docs/rom-test-corpus.md`); ExLoROM is implemented (decode formula sourced from
  bsnes's own runtime board database; no golden-ROM-boot proof — no ExLoROM ROM in the local
  corpus). **Niche peripherals (multitap, mouse, Super Scope) core is now complete, `v0.9.0`** —
  the real serial-shift-register protocol, ported from ares (`rustysnes_core::controller`),
  closing what was Phase 7's one open exit criterion; frontend host-input capture (a real mouse
  pointer, extra gamepads) remains a separate, tracked follow-up outside Phase 7's own scope
  (`docs/frontend.md` §Peripherals). Phase 5 (frontend)
  **desktop UX shell now at RustyNES's maturity bar (`v1.0.0`)**: the
  native+wasm shell is playable (video/audio/input/ROM-load wired), plus a thumbnail Save States
  manager, input rebinding, themes, speed presets, a Performance panel, fullscreen, and a
  first-run welcome modal. **`v1.0.1` closes both items deferred out of `v1.0.0`**: per-channel
  (per-voice) audio mutes (Settings → Audio, gating `Dsp::voice_output`) and global keyboard
  hotkeys (a fixed table, previously menu-bar-only) — see `docs/frontend.md` §Global hotkeys and
  `docs/apu.md` §Per-voice mute. **`v1.1.0`** closes `emu-thread`'s two biggest documented gaps —
  real audio output (a thread-owned `AudioProducer`) and a proper pause/ROM-loaded/speed lifecycle
  (`EmuControl`) plus a `PresentBuffer` lock-free framebuffer handoff — while leaving the rest of
  full parity (cheats/watchpoints/breakpoints/run-ahead/rewind/movies/scripting/netplay-pause/
  RetroAchievements) as a documented follow-up; also fixes a real, independent
  `SuperFxBoard::map` open-bus bug and investigates (without landing code for) the harder
  open-bus-via-DMA-latch bug, DRAM refresh timing, and the fractional-timebase refactor's own
  go/no-go gate — see `to-dos/VERSION-PLAN.md`'s `v1.1.0` section for the full breakdown.
  **`v1.2.0` "Phosphor"** relocates the pure `EmuCore` embedding facade into a new `std`-only
  `rustysnes_core::facade` module, lands `rustysnes-libretro`, a real libretro core wrapping it
  (region-aware NTSC/PAL, cheats, coprocessor firmware auto-resolution, raw memory-map pointers —
  `docs/libretro.md`), and a CRT/HQx presentation post-filter pipeline (scanlines + aperture mask,
  an HQ2x-style edge-directed blend approximation, the default no-filter path kept byte-for-byte
  identical to the pre-filter direct blit — `docs/frontend.md` §Presentation post-filters).
  **`v1.3.0` "Palimpsest"** lands HD texture packs (`hd-pack` feature, off by default): a
  palette-inclusive, allocation-free XXH3-64 tile-identity hash + a write-only `Ppu::tile_tags()`
  side-buffer in `rustysnes-ppu` (proven byte-identical to every prior release when off), a
  frontend `pack.toml` loader + pure-Rust PNG decoder (`crate::hd_pack`), a pure CPU compositor
  fully unit-testable without a GPU adapter (`crate::hd_compositor`), a Settings → Video pack
  selector with `config.toml` persistence, and the compositor wired into the live wgpu present
  path (`Gfx`'s streaming texture now grows on demand to fit a composited frame, the no-pack path
  staying pixel-identical to before) — see `docs/adr/0010`. Save-states are **fully
  implemented** (`v0.2.0 "Persistence"`, `docs/adr/0006` — every subsystem round-trips its exact
  state through one versioned envelope, proven by a round-trip determinism test), and rewind +
  run-ahead (`v0.3.0 "Continuum"`, `crate::rewind` — a bounded ring buffer of full snapshots +
  N-frame peek-and-discard, both config-driven and off by default) are now **fully implemented**
  — the frontend orchestration Phase 8 (netplay, TAS movies) built on this. Phase 6 (accuracy
  push) **dashboard + triage complete, fixes carried forward** (see the Phase 6 section below —
  the accuracy-pass-rate dashboard is done and every named hardware-gotcha item is triaged with
  evidence; the mid-line-raster fix has since landed in `v0.8.0`, see below — the
  accuracy-percentage push itself remains open). **Phase 8 is fully complete**: Sprint 1
  (`v0.8.0 "Instrumentation"`) landed the debugger overlay (T-81-001, live-state panels; a 65C816
  disassembler + PC breakpoints/step controls remain an open follow-up despite an earlier
  sprint-doc checkbox claiming otherwise — no such code exists yet, corrected here rather than
  left stale; read/write watchpoints landed as T-81-001b — a new `debug-hooks` feature on
  `rustysnes-core` itself + a `Bus`-level hook + the debugger's Watch panel, plus a new
  `rustysnes_cpu::disasm` decode-only disassembler), sandboxed Lua scripting + TAS movie
  record/playback (T-81-002, `rustysnes-script` + `rustysnes_core::movie`), Game Genie/Pro Action
  Replay cheat codes (T-81-003, `rustysnes_core::cheat` + a `Bus::read24` intercept), the extended
  byte-identical-with-flags-off CI gate (T-81-004), and the full wasm frontend (T-81-005
  `wasm-canvas`, T-81-006 `wasm-winit` unification). Sprint 2 (`v0.8.0 "Community"`) has also
  **landed**: the netplay save-state cost benchmark (T-82-001, GO on the full-snapshot design),
  GGPO-style rollback netplay (T-82-002, native UDP + wasm32-clippy-verified WebRTC transports,
  its own drive loop independent of `emu-thread`), RetroAchievements native FFI (T-82-003,
  `RustySNES/<ver> rcheevos/<ver>` User-Agent), and the byte-identical CI gate extended again
  (T-82-004). **`v0.9.0` closed out the last residual**: T-81-001's PR B (the 65C816
  disassembly view + PC breakpoints/step-controls in the debugger UI, built on the existing
  `rustysnes_cpu::disasm` engine from T-81-001b plus a new non-intrusive `Bus::peek` read). See
  `docs/STATUS.md` for the authoritative per-subsystem table this line summarizes.
- **Release:** `v0.1.0 "Foundation"`, `v0.2.0 "Persistence"`, `v0.3.0 "Continuum"` (rewind,
  run-ahead, PAL auto-detect, ExLoROM), `v0.4.0 "Completion"` (SPC7110 addressing fix, ST018,
  standalone S-RTC), `v0.5.0 "Fidelity"` (the accuracy-pass-rate dashboard + the full named
  hardware-gotcha regression list — every item fixed, correctly reclassified as an intentional
  non-goal, or honestly researched-and-deferred with a mechanism write-up), `v0.6.0 "Shippable"`
  (release engineering + doc parity — `security.yml`, checksummed release assets, automated
  release-cutting via `release-auto.yml`, the `lint` job's `cargo doc` gate, the documentation
  index, benchmarks, audit trail, and ADR backfill), `v0.7.0 "Resolution"` (true 512-px
  hi-res Modes 5/6 output, a genuine one-pixel-clock-delayed DAC pipeline verified against ares'
  primary source; the save-state `FORMAT_VERSION`'s first real bump, closing the `v1.0.0` gate's
  backward-compat-fixture item early), `v0.8.0 "Community"` (Phase 8 Sprints 1+2 in full:
  debugger overlay + watchpoints/disassembler, Lua scripting/TAS, cheat codes, the real wasm
  frontend, rollback netplay, RetroAchievements — plus, alongside this work, the
  mid-scanline/HDMA-driven register timing fix landing and an SPC7110 open-bus bug fix that
  benefits every board), and `v0.9.0 "Threshold"` (Phase 7's niche peripherals + Phase 8's
  T-81-001 PR B, closing both phases out completely, plus the SPC7110 boot-crash gap resolved as
  a ROM-identity issue), and `v1.0.0 "Zenith"` (the production cut: `Board: Send`, the desktop UX
  shell at RustyNES's maturity bar, a frame-time performance-regression CI gate, an enhanced
  native CLI + `cargo full-build`, the README rewrite, and a GitHub Pages polish pass) are all
  tagged and released on GitHub, establishing the
  real release cadence `to-dos/VERSION-PLAN.md` defines — read it alongside this file; it maps
  the phases above onto a concrete, named `v0.x.0` → `v1.0.0` ladder with release-cut criteria
  per rung, rewritten with `v0.7.0` to front-load Phase 8 breadth into the `v1.0.0` gate rather
  than deferring it post-1.0 (matching what RustyNES actually shipped in its own v1.0.0 — see
  `to-dos/VERSION-PLAN.md`'s "Second reversal"). The open-bus-via-HDMA-latch investigation
  remains open (a genuinely separate bug from the now-landed mid-scanline fix — re-confirmed by
  re-testing its own prototype against the fixed tree), carried forward as an ongoing,
  opportunistic `v0.x.y`-patch cluster alongside the SPC7110 boot gap, DRAM refresh, and
  ROM-sourcing-blocked real-title validation items, rather than gating a numbered rung
  (`to-dos/VERSION-PLAN.md`). `v1.0.1 "Aftertouch"` (per-voice mutes + global hotkeys) followed
  `v1.0.0`, closing the phase spine below out completely. The post-`v1.0` Reach arc that follows
  is **also fully shipped**: `v1.1.0 "Latchkey"` (accuracy research + `emu-thread`'s biggest
  gaps), `v1.2.0 "Phosphor"` (the `rustysnes-libretro` core + the CRT/HQ2x shader pipeline),
  `v1.3.0 "Palimpsest"` (HD texture packs), and `v1.4.0 "Convergence"` (closing out the
  post-`v1.3.0` patch cluster: the open-bus-via-DMA-latch bug and the rest of `emu-thread`
  parity) — see the "Milestones beyond the phases" section below for what's actually still open.

## The phase spine

The order is chosen so each layer rests on a verified one below it (the cycle-accurate-emulator
build spine).

### Phase 0 — Foundation ✅ complete

**Goal:** the Cargo workspace + one-directional crate skeletons compile; CI green on stubs;
`tests/roms/` seeded with the permissive suites; the test-harness skeleton stands up.
**Exit:** `cargo check --workspace` + `cargo test --workspace` (stubs) green in CI.
→ [overview](phase-0-foundation/overview.md)

### Phase 1 — CPU + golden oracle ✅ complete

**Goal:** the 65C816 core passes the SingleStepTests/65816 per-opcode oracle (every opcode ×
addressing mode, 8/16-bit, native + emulation) and the gilyon CPU ROMs.
**Exit:** CPU per-opcode oracle 0-diff (gated on the 65816 license); gilyon CPU tables green.
→ [overview](phase-1-cpu-golden-log/overview.md)

### Phase 2 — Scheduler + video ✅ complete (mid-line raster deferred to Phase 6)

**Goal:** the master-clock lockstep scheduler (the 6/8/12 access map + 1360/1364/1368 lines)
and the PPU to a stable rendered frame; the PPU/DMA/HDMA test ROMs; a deterministic golden
framebuffer.
**Exit:** undisbeliever PPU/DMA/HDMA suite green; a deterministic golden framebuffer for a
known ROM.
→ [overview](phase-2-scheduler-video/overview.md)

### Phase 3 — Audio (SPC700 + S-DSP + the async resync) ✅ complete

**Goal:** the SPC700, S-DSP, ARAM, and the integer-accumulator async resync; the audio oracle.
**Exit:** SingleStepTests/spc700 0-diff; blargg `spc_*` green to the achievable bar;
deterministic golden audio.
→ [overview](phase-3-audio/overview.md)

### Phase 4 — Carts + coprocessors (Core tier first) ✅ complete

**Goal:** the LoROM/HiROM/ExHiROM memory model + header detect, then the Core/Curated
coprocessors (DSP-1 via the shared µPD77C25 core, Super FX, SA-1). Tier + honesty gate from
the first board.
**Exit:** the map models + Core/Curated coprocessors boot + pass their tests; honesty gate
green (`docs/adr/0003`).
→ [overview](phase-4-carts-mappers/overview.md)

### Phase 5 — Frontend 🚧 partial — the shell is playable; save-states/rewind/run-ahead landed; the full wasm frontend (Sprint 4) remains

**Goal:** the always-on egui shell (menu/status/Settings + debugger panels), the audio ring +
pacing, gamepads, save-states, rewind, run-ahead, the wasm build.
**Exit:** playable native + wasm; the frontend determinism path intact.
**Release mapping:** the playable shell shipped inside the retroactive `v0.1.0` tag
(`to-dos/VERSION-PLAN.md`); save-states shipped in `v0.2.0`; rewind/run-ahead shipped in
`v0.3.0`; the full wasm frontend (Sprint 4) remains.
→ [overview](phase-5-frontend/overview.md)

### Phase 6 — Accuracy to target ✅ dashboard + triage complete, fixes carried forward

**Goal:** drive the composed two-layer accuracy battery to ≥90% (100% the goal); identify the
hard-tier residuals and decide which defer to the fractional-timebase refactor. Un-defer the
Phase-2 mid-line-raster gap here.
**Status:** the accuracy-pass-rate dashboard is done (`docs/STATUS.md` §Accuracy dashboard) and
every named hardware-gotcha item has been triaged with evidence — fixed (a real HDMA dot-phase
doc/code drift), correctly reclassified as an intentional non-goal (`$4203`/`$4206`, the
"DMA/HDMA-collision crash quirk"), or honestly researched-and-deferred (DRAM refresh,
open-bus-via-HDMA-latch, hi-res color-math precision). The Phase-2 mid-line-raster gap is
**confirmed real, a fix is designed and prototyped, but NOT landed**: the prototype
(`rustysnes-ppu` compositing each line at the hardware-correct `RENDER_DOT` instead of the
line's end) is independently verified correct for the CPU/HDMA-driven case (SA-1's
`SD F-1 Grand Prix` golden change, pixel-verified as a real improvement), but the same change
breaks all 24 Super FX/GSU golden tests for reasons not yet understood — the identical failure
signature the sibling open-bus-via-HDMA-latch investigation also hit and correctly did not land
(`docs/ppu.md` §Mid-scanline/HDMA-driven register timing has the full mechanism, both
verifications, and what a future investigation needs).
**Exit:** accuracy battery at target; residuals documented + deferred, not point-fixed
(`docs/adr/0002`). **Not yet met** — see Status above.
**Release mapping:** `v0.5.0` (`to-dos/VERSION-PLAN.md`), triage complete; the one bounded
residual (true hi-res Modes 5/6 output) closes in `v0.7.0 "Resolution"`; the rest (mid-scanline/
GSU, open-bus-via-HDMA-latch, SPC7110, DRAM refresh, ROM-dump-gated validation) carries forward
as an ongoing, opportunistic `v0.x.y`-patch cluster, not a gating rung.
→ [overview](phase-6-accuracy-to-100/overview.md)

### Phase 7 — Breadth 🚧 complete (core); frontend host-input capture for peripherals remains

**Goal:** the remaining BestEffort coprocessors + niche peripherals; region timing as data.
**Exit:** the full coprocessor / board matrix in `docs/STATUS.md`.
**Status:** DSP-2/DSP-4/ST010/S-DD1/CX4/OBC1/standalone S-RTC done + validated (S-RTC unit-tested
only — no commercial dump available; DSP-3/ST011 explicitly deferred, no verified board/window
entry — a named residual, not a ROM-sourcing gap); SPC7110 implemented, multiple confirmed
addressing/open-bus bugs fixed through `v0.8.0` — **the remaining boot gap turned out to be a ROM
identity issue, not an emulation bug** (found in a `v0.9.0` follow-up): the local test dump is an
English fan-translation, not the original cartridge (a SHA256 mismatch against `ref-proj/ares`'s
database, a checksum-size inconsistency, and a public forum thread on the patch's own
non-standard memory map all confirm this independently —
`docs/audit/spc7110-boot-crash-2026-07-08.md`), so this is now a ROM-sourcing gap
(`docs/rom-test-corpus.md`), not an open bug; ST018 is now implemented (unit-tested only, no
commercial dump available); PAL region auto-detection and ExLoROM are both implemented (each with
a documented, honest validation gap — no PAL ROM and no ExLoROM ROM exist in the local corpus, so
neither has golden-framebuffer proof; `docs/rom-test-corpus.md` tracks exactly what would close
this). **Niche peripherals (multitap, mouse, Super Scope) core is now complete, `v0.9.0`** — the
real serial-shift-register protocol, ported from ares (`rustysnes_core::controller`); frontend
host-input capture (a real mouse pointer, extra gamepads) remains a separate, tracked follow-up
(`docs/frontend.md` §Peripherals), not part of this phase's own exit criteria.
**Release mapping:** the done work shipped inside `v0.1.0`; PAL auto-detect and ExLoROM landed
inside `v0.3.0 "Continuum"` alongside rewind/run-ahead (all four line items complete); standalone
S-RTC, the SPC7110 addressing fix, and ST018 all land inside `v0.4.0 "Completion"`; the DSP-3/
ST011 residual note lands inside `v0.8.0 "Community"`; the SPC7110 ROM-identity finding and the
niche peripherals core both land inside `v0.9.0 "Threshold"`; the PAL/ExLoROM golden-boot proof
and a genuine original-cartridge SPC7110 dump remain opportunistic if a real ROM ever surfaces.
→ [overview](phase-7-breadth/overview.md)

### Phase 8 — Instrumentation + Community (additive, off-by-default) 🚧 complete

**Goal:** debugger overlay, Lua scripting + TAS movies, cheat-code support, rollback netplay,
and RetroAchievements — each behind a default-off feature, each byte-identical with the feature
off. **This phase gates `v1.0.0`** (reversed from the earlier post-1.0
framing — see "Second reversal" in `to-dos/VERSION-PLAN.md`'s intro): RustyNES front-loaded this
exact breadth into its own v1.0.0 rather than deferring it, and matching that bar means Phase 8
lands before the production cut, not after it. A shader ecosystem/Libretro core remain
post-`v1.0.0` Reach — RustyNES doesn't have HD texture packs either, so `hd-pack` stays
deliberately out of the parity target.
**Status:** Both sprints are done, released together as one tag, `v0.8.0 "Community"`. Sprint 1
landed the debugger overlay,
`rustysnes-script` (Lua scripting + TAS movies), `rustysnes_core::cheat` (Game Genie/Pro Action Replay),
the extended byte-identical-with-flags-off CI gate, the full wasm frontend (T-81-001 through
T-81-006), and, as a follow-up, read/write watchpoints + a `rustysnes_cpu::disasm` disassembler
(T-81-001b). Sprint 2 landed rollback netplay (T-82-002, native UDP + WebRTC transports) and
native RetroAchievements FFI (T-82-003), preceded by the netplay save-state-cost benchmark
(T-82-001, GO) and the byte-identical CI gate extended again (T-82-004).
**`v0.9.0` follow-up:** T-81-001's PR B landed — the 65C816 disassembly view + PC
breakpoints/step-controls, wired into the debugger UI panel on top of T-81-001b's disassembler
engine, plus a new non-intrusive `Bus::peek` read (the debugger's own peeks must never perturb
the open-bus latch or trip watchpoints the way a live CPU access would). No open items remain
anywhere in Phase 8.
**Exit:** features ship; shipped/native/no_std/wasm byte-identical with every new flag off
(the byte-identical-with-all-flags-off CI gate, added starting `v0.8.0` and re-verified through
`v0.9.0`/`v1.0.0`).
**Release mapping:** `v0.8.0 "Community"` (both sprints — debugger/watchpoints/disassembler
engine, scripting/TAS, cheats, the real wasm frontend, netplay, RetroAchievements — plus the
mid-scanline/HDMA fix and the start of the SPC7110 investigation) then `v0.9.0 "Threshold"`
(T-81-001 PR B's disassembly view/breakpoints/step-controls, closing Phase 8 out completely) —
see `to-dos/VERSION-PLAN.md` and `CHANGELOG.md`'s `[0.8.0]`/`[0.9.0]` entries for the full
per-item breakdown, including the `Board: Send`/`emu-thread` prerequisite (still open, tracked
under `v1.0.0`) and the netplay save-state-cost pre-work.
→ [overview](phase-8-reach/overview.md)

## Milestones beyond the phases

- **v0.7.0 "Resolution" — RELEASED 2026-07-09.** True 512-px hi-res (Modes 5/6) output — the one
  bounded item left on Phase 6's residual list; the rest of that list (SPC7110, DRAM refresh,
  ST018/S-RTC/PAL/ExLoROM real-ROM validation) stays an ongoing, opportunistic `v0.x.y`-patch
  cluster, not a gating rung — see `to-dos/VERSION-PLAN.md`. The mid-scanline/GSU and
  open-bus-via-HDMA-latch items were also on this list at the time; the former has since landed
  in `v0.8.0`, the latter remains open.
- **v0.8.0 "Community" — RELEASED 2026-07-10 — Phase 8, gating `v1.0.0`.** See the Phase 8
  section above.
- **v0.9.0 "Threshold" — RELEASED 2026-07-10.** Not an originally-planned rung — closes out
  Phase 7's last exit criterion (niche peripherals) and Phase 8's last ticket half (T-81-001 PR
  B), and resolves the SPC7110 investigation as a ROM-sourcing gap rather than an open bug. The
  last loose ends before the `v1.0.0` production cut.
- **v1.0.0 "Zenith" — RELEASED 2026-07-10 — the production cut.** `Board: Send` (unblocking
  `emu-thread` to compile/test/lint clean, though it stays off-by-default pending full feature
  parity — a real, documented gap, not silently promoted); the desktop UX shell at RustyNES's
  maturity bar (thumbnail Save States manager, input rebinding, themes, speed presets, a
  Performance panel with a frame-time sparkline, fullscreen, a first-run welcome modal); a new
  frame-time performance-regression CI gate; the save-state backward-compat fixture (found
  already landed, from `v0.7.0`); an enhanced native CLI + `cargo full-build`/`full-run`; the
  README rewrite; a GitHub Pages demo-page polish pass; README / CHANGELOG / docs / STATUS in
  sync. The full accuracy battery (27 oracle/golden suites), `no_std`, and both wasm32 frontends
  re-verified with zero regressions. See `to-dos/VERSION-PLAN.md` for the full per-item detail.
- **v1.0.1 "Aftertouch" — RELEASED.** The two items explicitly deferred out of `v1.0.0`:
  per-voice audio mutes (Settings → Audio) and global, non-rebindable keyboard hotkeys.
- **v1.1.0 "Latchkey" — RELEASED.** Reach-phase research + accuracy pass: a real, independent
  `SuperFxBoard::map` open-bus fix, `emu-thread`'s biggest gaps (real audio output +
  pause/ROM-loaded/speed lifecycle + `PresentBuffer`), and three investigated-not-landed items
  (open-bus-via-DMA-latch, DRAM refresh, the fractional-timebase go/no-go — see below).
- **v1.2.0 "Phosphor" — RELEASED.** The `EmuCore` facade relocated into `rustysnes_core::facade`
  (`std`-only), the `rustysnes-libretro` core crate, and the CRT/HQ2x presentation post-filter
  pipeline.
- **v1.3.0 "Palimpsest" — RELEASED.** HD texture packs (`hd-pack` feature, off by default): the
  palette-inclusive `TileTag` hashing hook in `rustysnes-ppu`, the frontend loader + pure CPU
  compositor, Settings UI + config, and the compositor wired into the live wgpu present path.
- **v1.4.0 "Convergence" — RELEASED.** Closed out the post-`v1.3.0` patch cluster: the
  fullscreen-crash-on-wide-monitors bug, RustyNES-parity Window Size presets, `rustysnes-libretro`'s
  peripheral negotiation (Mouse/Super Scope/Multitap via `RETRO_DEVICE_SUBCLASS`), the
  open-bus-via-DMA-latch bug (cross-checked directly against ares'/bsnes' own `CPU::Channel` DMA
  implementation), `emu-thread`'s mechanical cheats/watchpoints/breakpoints/port2-peripheral/
  voice-mute re-sync, and `emu-thread`'s run-ahead + netplay-aware pause (the latter fixing a real
  latent bug: `NetplayState::drive` was previously dead code under `emu-thread`, so netplay was
  silently non-functional in threaded builds). See `to-dos/VERSION-PLAN.md`'s `v1.4.0` section for
  the full per-item detail, including two bot-review-caught fixes (a stale-dims-vs-bytes
  correctness bug, a run-ahead-helper per-frame allocation regression) and two CI gaps closed
  (`emu-thread` was never actually clippy-gated, its own unit tests were never actually executed).
- **Beyond that — Reach, now closed out.** The Libretro core, the CRT/HQx shader/filter pipeline,
  and HD texture packs — the three items originally deferred here — all shipped in `v1.2.0`/
  `v1.3.0` above (see `to-dos/VERSION-PLAN.md`'s "Post-v1.0 — Reach"). What's still genuinely open:
  the SPC7110/PAL/ExLoROM/ST018/S-RTC real-ROM-validation gaps (all ROM-sourcing-blocked, tracked
  in `docs/rom-test-corpus.md`). The mobile/Android + iOS target — previously "no appetite
  assumed by default" here — is now explicitly in scope as of `v1.14.0 "Foundry"`; see the
  RustyNES-parity ladder entry below and `docs/adr/0012-mobile-platform-target.md`.
  Movies/scripting/RetroAchievements/rewind-recording on `emu-thread` are reclassified
  as an intentional, permanent architecture boundary rather than a remaining gap — confirmed by
  directly reading RustyNES's own mature `emu_thread.rs`, which doesn't port any of these to its
  thread either. None of the still-open items currently gate a numbered rung — they're an ongoing,
  opportunistic `v1.x.y`-patch cluster.
- **`v1.5.0 "Bedrock"` onward — the RustyNES-parity ladder — IN PROGRESS.** A second, parallel
  ladder theme closing the gap between RustySNES and its sibling NES emulator RustyNES: CI
  safety net (`v1.5.0`, **RELEASED 2026-07-11**), a docs site + accuracy-ledger (`v1.6.0`,
  **RELEASED 2026-07-11**), a real debugger module (`v1.7.0`, **RELEASED 2026-07-12**, patched
  same-day in `v1.7.1` for a wasm-demo canvas-sizing bug — a memory panel + the panel-plumbing
  scaffold; `v1.8.0`, **RELEASED 2026-07-12**, adds a Memory Compare panel + an in-app
  glossary), Lua scripting bus-widening (`v1.9.0 "Marionette"`, **RELEASED 2026-07-12** —
  `emu.read` now reaches the full 24-bit bus; the wasm `piccolo` backend and TAStudio piano-roll
  editing honestly deferred to a later, explicitly-scoped release), HD-pack `emu-thread` wiring
  (`v1.10.0 "Atelier"`, **RELEASED 2026-07-12** — the threaded build now composites an active
  HD-pack instead of silently rendering native art; the in-app Builder GUI itself and run-ahead
  compositing both honestly deferred), RetroAchievements game-load fix (`v1.11.0 "Podium"`,
  **RELEASED 2026-07-12** — no code path ever called `RaClient::begin_load_game`, so achievements
  could never actually trigger despite login/`do_frame`/toast plumbing all being wired up; the
  originally-planned hardcore mode/leaderboard/rich-presence UI was meaningless without this
  prerequisite fix landing first, so it's deferred to a later rung instead), a third post-filter
  plus a shader-crate extraction (`v1.12.0 "Refraction"`, **RELEASED 2026-07-12** —
  `PostFilter::Xbrz`, an xBRZ-style context-gated corner blend, and the new `rustysnes-gfx-shaders`
  crate housing `BLIT_WGSL`/`CRT_WGSL`/`HQX_WGSL`/`XBRZ_WGSL` for reuse by the mobile track;
  `.slangp`/`.cgp` import and a composite/RF post-pass remain deferred, unrevisited from `v1.2.0`'s
  original scope call), two accessibility theme variants (`v1.13.0 "Vantage"`,
  **RELEASED 2026-07-12** — `AppTheme::HighContrast`/`Colorblind`; the plan's other two items,
  a save-state migration fixture and a keyboard-nav audit, were investigated and found to already
  be a verified-non-issue and a manual-walkthrough task respectively, not code fixes — both
  honestly re-scoped rather than force-fit), then the mobile track's bridge foundations
  (`v1.14.0 "Foundry"`, **RELEASED 2026-07-12** — new `rustysnes-mobile` `UniFFI` crate over the
  same `EmuCore` facade the desktop frontend uses; verified for real with a genuine `cargo ndk`
  ARM64 cross-compile and inspected Kotlin/Swift binding output, not just claimed; the mobile/
  Android+iOS "no appetite" default from `v1.0.0` is formally reversed, `docs/adr/0012`), then a
  real Android alpha (`v1.15.0 "Sideload"`, **RELEASED 2026-07-12** — new `rustysnes-android`
  presentation-only `wgpu`-on-`Surface` crate plus a minimal Kotlin Compose shell, verified for
  real on a live Android emulator including a background/foreground lifecycle exercise; touch UX
  for Mouse/Super Scope/Multitap, save-state UI, and post-filter wiring honestly deferred to
  `v1.15.1+`), then an iOS alpha (`v1.16.0 "Beacon"`, **RELEASED 2026-07-12** — new
  `rustysnes-ios` crate mirroring `rustysnes-android`'s architecture, verified for real via
  genuine iOS-target cross-compiles plus a real, passing `xcodebuild` simulator build on a
  `macos-latest` CI runner — the project's only real Xcode/Swift toolchain; no on-device/
  simulator run yet, only a build), then a hardening rung (`v1.17.0 "Parity"`,
  **RELEASED 2026-07-12** — Save State/Load State on both mobile shells, plus a real,
  pre-existing, already-shipped Android `AudioTrack` crash found and fixed while re-verifying it
  on-device; RetroAchievements/`mlua`/netplay honestly re-scoped to a later rung), then dormant
  monetization scaffolding (`v1.18.0 "Dormant"`, **RELEASED 2026-07-14** — new, standalone
  `rustysnes-monetization` UniFFI crate, never a dependency of the deterministic core, every
  pricing/pacing figure an explicit placeholder; wired into both mobile shells as an inert,
  log-only startup call, verified for real on the Android AVD via `logcat` and compile-verified
  for iOS via a real macOS CI build, which caught and fixed a genuine `xcodebuild`
  xcframework-modulemap collision), and finally an optional PGO/BOLT pipeline for the shipping
  `rustysnes` binary (`v1.19.0 "Afterburner"`, **RELEASED 2026-07-15** — `scripts/pgo/run.sh` +
  `.github/workflows/pgo.yml`, promotion gated on both a `>3%` Criterion speedup and a
  byte-identical `--features test-roms` re-run under the PGO profile, never on speed alone;
  verified for real end-to-end in this development environment, including a genuine BOLT-stage
  structural bug found and fixed in PR review). This closes the RustyNES-parity ladder plan.
  Tracked in lockstep against RustyNES's own continuing development via
  `to-dos/LOCKSTEP-CHECKLIST.md`, not a frozen snapshot target. Full detail in
  `to-dos/VERSION-PLAN.md`'s "RustyNES-parity ladder" section.
- **A new, separate UI/UX-parity ladder, Phase A (`v1.20.0 "Aperture"`, RELEASED 2026-07-15).** A
  systematic audit of RustySNES's menus/settings/debugger against RustyNES's own frontend found
  the wasm demo showing placeholder labels for two features never actually excluded for any real
  reason (`cheats`/`debug-hooks`, now real), a desktop peripheral-input gap (Settings selected the
  Mouse/Super Scope hardware but nothing captured host pointer input, now wired via a new
  `crate::peripherals` module), a missing View → Hide Overscan toggle, and closed the ROM Info
  debugger panel small-catch-up item named above. Full detail in `to-dos/VERSION-PLAN.md`'s
  `v1.20.0 "Aperture"` entry. Phases B (in-app Help docs, deeper debugger panels) and C (wasm Lua
  scripting, browser netplay lobby, browser RetroAchievements, i18n) remain scoped but not
  started.
- **Flagged by the 2026-07-12 lockstep re-check — no rung assigned yet, maintainer go/no-go
  needed.** RustyNES shipped two items since the roadmap's `v2.1.5` baseline that RustySNES's own
  ladder doesn't currently account for: (1) a **GIF/WAV screen+audio capture subsystem**
  (RustyNES `v2.1.9`) — a genuinely new feature category, not incremental growth of anything
  already planned; and (2) **CRT shader-preset depth** — RustyNES ported 3 marquee libretro-slang
  presets (CRT-Royale, guest-advanced, Sony Megatron) against RustySNES's own single
  scanline+aperture-mask `Crt` filter (plus the new `Xbrz` filter, `v1.12.0`). Neither is urgent
  (RustySNES's existing presentation pipeline is fully functional), so neither is silently folded
  into an already-scoped rung — see `to-dos/LOCKSTEP-CHECKLIST.md`'s 2026-07-12 log row for the
  full disposition, including two other RustyNES additions (a SIMD blitter/wasm-size pass, a
  browser RA + Vs. DualSystem libretro pairing) judged small-catch-up or correctly out of scope.
- **Flagged by the 2026-07-15 lockstep re-check — no rung assigned yet, maintainer go/no-go
  needed.** RustyNES cut `v2.2.0 "Capstone"` since the last check (`v2.1.10`), shipping two items
  RustySNES's own roadmap doesn't currently account for: (1) **no fuzzing infrastructure at all**
  — RustySNES has no `fuzz/` directory, while RustyNES's now spans 8 cargo-fuzz targets covering
  PPU/APU register I/O, netplay message parsing, save-state, and movie deserialization; this
  project's own `docs/testing-strategy.md` already names Layer 1 unit testing as "each chip is
  fuzzable in isolation" but that capability was never actually built out — no corpus, no CI
  target, no committed `fuzz/` crate exists for `rustysnes-core`'s own untrusted-input boundaries
  (ROM header parsing, save-state loading, movie deserialization, netplay wire messages); and
  (2) **netplay lobby/matchmaking + spectator/desync/liveness depth** — RustyNES's
  `rustynes-netplay` signaling protocol grew a browse-and-join room directory, server-side
  quick-match, delayed-stream spectators, a graded hysteresis-backed desync verdict, and
  peer-liveness RTT timeouts; `rustysnes-netplay` has none of this (confirmed: no
  `SignalMessage`/`ListRooms`/`QuickMatch`/`delay_frames`/`DesyncStatus`/`PeerLink` equivalents),
  so the original `v1.5.0`-era "netplay already at parity" assessment is now stale against
  RustyNES's deepened baseline. Neither is urgent (RustySNES's existing netplay is functional at
  its own, narrower scope, and no untrusted-input crash has ever been found), so neither is
  silently folded into an already-closed rung — see `to-dos/LOCKSTEP-CHECKLIST.md`'s 2026-07-15
  log row for the full disposition, including three smaller items (a self-contained ROM Info
  debugger panel judged a small catch-up — **closed in `v1.20.0 "Aperture"`**, as part of that
  rung's own separate UI/UX-parity audit against RustyNES's frontend, not this lockstep check
  directly; RustySNES's `movie.rs` deserializer already independently
  hardened against the same OOM-DoS class RustyNES's fuzzing just found, so already covered; and a
  Zapper aperture-hardening technique that doesn't map onto RustySNES's own, architecturally
  different geometric Super Scope hit-detection model, so not directly applicable).
- **Further beyond — the fractional-timebase refactor (`docs/adr/0002`).** Assessed in `v1.1.0`
  and found **not currently warranted** — every named accuracy residual is answerable within the
  existing whole-master-clock-tick model (`docs/audit/fractional-timebase-go-no-go-2026-07-11.md`).
  Revisit only if a hard-tier residual surfaces that genuinely needs sub-cycle resolution: the
  one-clock + every-cycle-bus-access collapse (a fractional master clock with a φ1/φ2 split).
  **The one release expected to break byte-identity / save-state compatibility.** Do NOT conflate
  it with "the master clock already exists (the Phase-0 scheduler)" — the RustyNES versioning trap.

## AccuracySNES follow-on tickets (T-04-*)

AccuracySNES (`tests/roms/AccuracySNES/`) closed ticket **T-04** — the monolithic all-in-one
oracle ROM that no publicly available SNES ROM provided, so we wrote one. The battery currently
stands at **85 tests (80 scoring + 5 golden), 100.00%**, cross-validated against Mesen2 and
snes9x. The tickets below carry the rest of the enumeration in
`docs/accuracysnes-research-dossier.md` §5. Full rationale, blocker analysis, and the ordering
constraints live in **`docs/accuracysnes-plan.md`**; this list is the citable ID index.

| Ticket | Scope | Size | Blocked on |
|---|---|---:|---|
| **T-04-A** | Finish Group A (65C816) — `A5` spot checks, RMW `abs,X`, E-gated branch penalty, the `A6` gaps (`PBR`, `RTI` mode match, B flag, `WAI`) | ~12 | nothing |
| **T-04-B** | Group B — 5A22 bus, clock, timing. **Started:** access speed, `RDNMI` mechanics, IRQ timers, frame geometry, multiply/divide + power-on shipped (14 tests, 2 emulator defects found). Left: `B2` scanline geometry, `B3` DRAM refresh — **unblocked**, write as golden vectors per the `D3` precedent (see the plan §4) — and the rest of `B4` | ~16 | nothing |
| **T-04-C** | The rest of register-observable Group C — `C1.07`/`C1.08`, the 9/10-bit `VMAIN` rotations, CGRAM-during-render, `C7.04`–`C7.09`, `C9.05`, `C11.07`/`C11.08` | ~20 | nothing |
| **T-04-D** | Group D — DMA / HDMA | ~35 | **a research top-up**: the dossier's DMA/HDMA sub-agent never returned, so `D1`/`D2` are under-sourced |
| **T-04-E** | Group E — SPC700 + S-DSP | ~75 | an **on-cart APU harness** (IPL upload, results back through `$2140`–`$2143`) |
| **T-04-F** | Group F — input | ~22 | a decision on the **on-cart / host-driven split**: a cart cannot press its own buttons |
| **T-04-G** | Group G — power-on / reset / cartridge, mostly golden vectors | ~18 | ~~boot-path ordering~~ **UNBLOCKED** — `capture_power_on` samples before `init_registers` into a documented capture block; `B5.05` is the first consumer |
| **T-04-I** | The 256-opcode cycle sweep (`A5.01`–`A5.08`) | 1 mechanism | ~~an external timing table~~ **ORACLE ESTABLISHED** — `docs/accuracysnes-timing-oracle.md`. The 5A22 is a stock WDC core plus a wait-state generator, so WDC Table 5-7 (VDA/VPA cycle classification) + the SNES speed map is emulator-independent and sufficient. Remaining: safe-operand table + sandbox. `STP` excluded — it halts until reset |
| ~~**T-04-J**~~ | ~~Dossier-to-cart ID map~~ **DONE** — `gen/src/dossier.rs` maps every test to its assertion(s), the generator rejects unmapped tests / undeclared double-claims / unjustified blanks, `SOURCE_CATALOG.tsv` carries a `dossier` column, and the harness re-checks the committed artifact. Also converted the dossier's 23 prose sub-groups into per-ID tables: **443** checkable assertions across all 43 sub-groups, up from 232. Coverage lives in `docs/accuracysnes-coverage.md` (**79 / 443**) | — | — |
| **T-04-H** | The renderer-dependent rest of Group C (`C5`, `C6`, `C8`, `C10`, `C12`, most of `C9`, `C13.01`–`C13.06`) | ~42 | ~~a framebuffer oracle~~ **DECISION DRAFTED** — `docs/adr/0013`: host-side oracle, separate tier, never in the on-cart pass rate, goldens blessed only from cross-validated renders. Awaiting ratification |

Suggested order: A → B → C → D → G → E → F, with H taken only if the framebuffer-oracle decision
is taken. Real-hardware validation is the standing ceiling on all of it: every result so far is
three emulators agreeing, and ares/bsnes are one lineage rather than two opinions.

## Cross-phase dependencies

- Phase 2 (scheduler) depends on Phase 1 (the CPU drives the scheduler's access-speed query).
- Phase 3 (audio resync) depends on Phase 2's scheduler (the once-per-scanline forced sync).
- Phase 4's SA-1 reuses the Phase-1 65C816 core (a second instance).
- Phase 6 (accuracy) depends on Phases 1–4 being feature-complete enough to run the full
  battery.
- Phase 8 (netplay / TAS) depends on the determinism contract (`docs/adr/0004`) being
  exercised in Phase 5.

## Open questions (historical — both since resolved)

- ~~The 65816 JSON oracle ships no license~~ — **resolved** (`docs/adr/0005`): the test-harness
  self-generates its own per-opcode JSON oracle (cycle-by-cycle bus-pin trace included),
  bootstrap-validated against the upstream (unlicensed) set as a local cross-check, then
  committed and gated in CI. Phase 1's CPU oracle is 0-diff (`docs/STATUS.md`).
- ~~Per-board SRAM / coprocessor bus windows have no canonical table~~ — built out incrementally
  through Phase 4/7 into a real per-model table (`docs/cart.md`); still genuinely
  board-dependent (no single formula covers every mapper) but no longer an open planning
  question — see `docs/cart.md`'s SRAM-window table.
