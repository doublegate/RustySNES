# Sprint 1 — The egui shell + emu thread + audio ring

**Phase:** Phase 5 — Frontend
**Sprint goal:** a playable native baseline — the always-on egui shell, the dedicated emulation
thread, and the lock-free audio ring + pacing — with the lock discipline correct.
**Estimated duration:** 2 weeks

## Tickets

### T-51-001 — The egui shell (menu / status / Settings) + the `MenuAction` discipline

**Description:** stand up the persistent menu bar (File / Emulation / Tools / View / Debug /
Help) + status bar + tabbed Settings; menu interactions return a `MenuAction` dispatched after
the egui pass; the hidden render branch copies the framebuffer under a brief lock.

**Acceptance criteria:**

- [ ] egui runs every frame; the shell never holds the emu lock inside the egui closure.
- [ ] A ROM loads via File → Open and renders.
- [ ] Settings persist per-setting.

**Dependencies:** T-21-001 (a rendered frame)
**Reference:** `docs/frontend.md` §shell-model
**Estimated complexity:** L

---

### T-51-002 — The dedicated emulation thread + `SharedInput`

**Description:** run the emulator on a dedicated `emu-thread` behind an `Arc<Mutex<EmuCore>>` +
a lock-free `SharedInput`; the winit thread only does UI + present.

**Acceptance criteria:**

- [ ] The emulator advances on its own thread; the UI stays responsive under load.
- [ ] Input is late-latched without breaking determinism.
- [ ] No data races (clean under a thread sanitizer run).

**Dependencies:** T-51-001
**Reference:** `docs/frontend.md` §shell-model
**Estimated complexity:** M

---

### T-51-003 — The lock-free audio ring + dynamic rate control + pacing

**Description:** feed the core's 32 kHz stereo stream into a lock-free ring drained by cpal,
with DRC absorbing jitter, and a display-sync pacing matrix at 60.0988/50.0070 Hz.

**Acceptance criteria:**

- [ ] No audio underruns at steady-state on the target platforms.
- [ ] DRC + run-ahead orchestration live in the frontend, never the core (`docs/adr/0004`).
- [ ] The pacing matrix holds the target refresh.

**Dependencies:** T-51-002; T-31-003 (an audio stream)
**Reference:** `docs/frontend.md` §audio-pacing
**Estimated complexity:** L

---

## Sprint review checklist

- [ ] All tickets checked off or explicitly deferred (with reason).
- [ ] A native build is playable; the determinism path is intact.
- [ ] CHANGELOG.md updated.
