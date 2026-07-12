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
  in `docs/rom-test-corpus.md`) and any future mobile/Android target (no appetite assumed by
  default). Movies/scripting/RetroAchievements/rewind-recording on `emu-thread` are reclassified
  as an intentional, permanent architecture boundary rather than a remaining gap — confirmed by
  directly reading RustyNES's own mature `emu_thread.rs`, which doesn't port any of these to its
  thread either. None of the still-open items currently gate a numbered rung — they're an ongoing,
  opportunistic `v1.x.y`-patch cluster.
- **`v1.5.0 "Bedrock"` onward — the RustyNES-parity ladder — IN PROGRESS.** A second, parallel
  ladder theme closing the gap between RustySNES and its sibling NES emulator RustyNES: CI safety
  net (`v1.5.0`, **RELEASED 2026-07-11**), a docs site + accuracy-ledger (`v1.6.0`, **RELEASED
  2026-07-11**), a real debugger module (`v1.7.0`, **RELEASED 2026-07-12**, patched same-day in
  `v1.7.1` for a wasm-demo canvas-sizing bug — a memory panel + the panel-plumbing scaffold;
  `v1.8.0`, **RELEASED 2026-07-12**, added a Memory Compare panel + an in-app glossary), Lua/TAS
  + TAStudio depth (`v1.9.0`), an HD-pack builder (`v1.10.0`), RetroAchievements
  hardcore/leaderboard/rich-presence (`v1.11.0`), a deeper shader/NTSC ladder (`v1.12.0`),
  accessibility/theming + save-state polish (`v1.13.0`), then a full mobile track — Android +
  iOS apps plus dormant monetization scaffolding
  (`v1.14.0`-`v1.18.0`) — and a PGO/BOLT pipeline last (`v1.19.0`). Tracked in lockstep against
  RustyNES's own continuing development via `to-dos/LOCKSTEP-CHECKLIST.md`, not a frozen snapshot
  target. Full detail in `to-dos/VERSION-PLAN.md`'s "RustyNES-parity ladder" section.
- **Further beyond — the fractional-timebase refactor (`docs/adr/0002`).** Assessed in `v1.1.0`
  and found **not currently warranted** — every named accuracy residual is answerable within the
  existing whole-master-clock-tick model (`docs/audit/fractional-timebase-go-no-go-2026-07-11.md`).
  Revisit only if a hard-tier residual surfaces that genuinely needs sub-cycle resolution: the
  one-clock + every-cycle-bus-access collapse (a fractional master clock with a φ1/φ2 split).
  **The one release expected to break byte-identity / save-state compatibility.** Do NOT conflate
  it with "the master clock already exists (the Phase-0 scheduler)" — the RustyNES versioning trap.

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
