# Phase 5 — Frontend

## Goal

The always-on egui shell — persistent menu / status / Settings + toggleable debugger panels,
the dedicated emulation thread, the lock-free audio ring + pacing matrix, gamepads, save-states,
rewind, run-ahead, and the wasm build. Reuse the RustyNES / prior Rusty\* frontend shell; the
SNES-specific work is the second-CPU/APU panels and the Mode-7 / HDMA / coprocessor debug views.

## Exit criteria

- [x] Playable native (winit 0.30 + wgpu 29 + cpal + egui 0.35): real ROMs boot with picture,
      sound, and control. wasm32 target builds (browser frontend is a scaffold).
- [x] The shell never holds the emu lock inside the egui closure (`MenuAction` dispatched after
      the egui pass). NOTE: the synchronous in-`render` drive is the default; the dedicated
      `emu-thread` stays default-off until `Board: Send` lands (a one-word cart change) — deferred.
- [x] The audio ring + dynamic rate control are wired (S-DSP 32 kHz FIFO → DRC-paced resampler →
      lock-free ring → cpal stereo); occupancy-target DRC absorbs pacing jitter.
- [x] Save-states (Sprint 2, `v0.2.0 "Persistence"`) — the core-wide deterministic snapshot
      format (`docs/adr/0006-save-state-format.md`, `Accepted`) is fully implemented and proven
      by a round-trip determinism test.
- [ ] Rewind and run-ahead (Sprint 3, `v0.3.0`) — depends on Sprint 2's save-state primitive.
- [x] The frontend determinism path is intact (rate control + resampling live here, not the core;
      the additive S-DSP FIFO records already-emitted samples and never feeds back into synthesis).
- [ ] All sprints complete (Sprint 1 + 2 done; rewind/run-ahead + the full wasm frontend remain).

## Scope

In-scope:

- The egui shell, audio ring + pacing, gamepads + late-latched input, save-states / rewind /
  run-ahead, the wasm build, the clap-4 `--help` + ratatui TUI (native-only).

Out-of-scope (Phase 8):

- Netplay, RetroAchievements, TAS movie tooling, Lua, shaders — all default-off reach features.

## Sprints

- [Sprint 1 — The egui shell + emu thread + audio ring](sprint-1-shell.md) — the playable
  native baseline. **Status:** complete (`v0.1.0`).
- [Sprint 2 — Save-states](sprint-2-save-states.md) — the versioned core-wide snapshot format
  (`docs/adr/0006`) + `System::save_state()`/`load_state()` + the round-trip determinism proof.
  **Status:** complete. **Release:** `v0.2.0 "Persistence"`.
- Sprint 3 — Rewind, run-ahead, gamepads.
  **Status:** starting — PAL region auto-detection (a `v0.3.0 "Continuum"` line item alongside
  this sprint) has already landed. **Release:** `v0.3.0 "Continuum"`.
- Sprint 4 — The wasm build + `--help` TUI.
  **Status:** stub.

## Dependencies

Phases 2–3 (a rendered frame + an audio stream to present); the determinism contract
(`docs/adr/0004`) is exercised here for save-states / run-ahead.

## Risks

- **Holding the emu lock in the egui closure** would deadlock / stutter — enforce the
  `MenuAction`-after-pass pattern from day one.
- **Determinism leakage** via frontend rate control bleeding into the core — keep DRC + run-ahead
  strictly in the frontend.

## Reference docs

- [docs/frontend.md](../../docs/frontend.md) — the shell model + the determinism boundary.
- [docs/adr/0004](../../docs/adr/0004-determinism-contract.md).
