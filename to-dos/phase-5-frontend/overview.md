# Phase 5 — Frontend

## Goal

The always-on egui shell — persistent menu / status / Settings + toggleable debugger panels,
the dedicated emulation thread, the lock-free audio ring + pacing matrix, gamepads, save-states,
rewind, run-ahead, and the wasm build. Reuse the RustyNES / prior Rusty\* frontend shell; the
SNES-specific work is the second-CPU/APU panels and the Mode-7 / HDMA / coprocessor debug views.

## Exit criteria

- [ ] Playable native (winit + wgpu + cpal + egui) + wasm.
- [ ] The shell never holds the emu lock inside the egui closure (`MenuAction` dispatched after
      the egui pass); the emulator runs on a dedicated thread.
- [ ] The audio ring + dynamic rate control sustain 60.0988/50.0070 Hz without underruns.
- [ ] Save-states, rewind, and run-ahead work and preserve the determinism contract (incl. the
      SPC accumulator + seeded phase).
- [ ] The frontend determinism path is intact (rate control + run-ahead live here, not the core).
- [ ] All sprints complete.

## Scope

In-scope:

- The egui shell, audio ring + pacing, gamepads + late-latched input, save-states / rewind /
  run-ahead, the wasm build, the clap-4 `--help` + ratatui TUI (native-only).

Out-of-scope (Phase 8):

- Netplay, RetroAchievements, TAS movie tooling, Lua, shaders — all default-off reach features.

## Sprints

- [Sprint 1 — The egui shell + emu thread + audio ring](sprint-1-shell.md) — the playable
  native baseline.
- Sprint 2 — Save-states, rewind, run-ahead, gamepads.
  **Status:** stub — refine when Sprint 1 is ~complete.
- Sprint 3 — The wasm build + `--help` TUI.
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
