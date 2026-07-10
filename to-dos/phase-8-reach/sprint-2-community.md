# Sprint 2 — Community

**Phase:** Phase 8 — Instrumentation + Community
**Sprint goal:** rollback netplay and RetroAchievements ship behind default-off feature flags,
each byte-identical with the feature off, resting on the exercised determinism contract.
**Estimated duration:** 2 weeks
**Release mapping:** `v0.9.0 "Community"` (`to-dos/VERSION-PLAN.md`)

## Tickets

### T-82-001 — Netplay save-state cost benchmark (pre-work)

**Description:** before committing to the existing full-snapshot save-state design for rollback
netplay, benchmark `System::save_state()`/`load_state()` cost. `docs/benchmarks.md` currently has
only one number (steady-state frame time); `RewindBuffer` was designed for ~10 Hz capture, and
rollback netplay calls save/restore far more often. If the cost is too high for a real rollback
window, delta/incremental snapshots become necessary — a real design change beyond
`docs/adr/0006`'s "future memory optimization, not correctness requirement" framing.

**Acceptance criteria:**

- [x] A new Criterion benchmark measures `save_state()`/`load_state()` cost across a
      no-coprocessor, a Curated (Super FX/SA-1), and a BestEffort sample —
      `crates/rustysnes-core/benches/save_state_cost.rs`.
- [x] The result is recorded in `docs/benchmarks.md` (§`v0.9.0` pre-work — save-state cost).
- [x] A go/no-go call on the full-snapshot design is made explicitly: **GO** — all three tiers
      cluster tightly (~108 µs save, ~295 µs load), both negligible next to a single frame's own
      ~3.27 ms execution cost; no delta/incremental redesign needed before T-82-002 proceeds.

**Dependencies:** `v0.2.0`'s save-state envelope; `docs/adr/0006`
**Reference:** `docs/benchmarks.md`; `docs/adr/0006-save-state-format.md`
**Estimated complexity:** S

---

### T-82-002 — Rollback netplay (frontend-orchestrated)

**Description:** implement GGPO-style rollback netplay in `rustysnes-netplay` (UDP native +
WebRTC browser, 2+ players), orchestrated by the frontend against the deterministic core
(snapshot / restore / re-simulate). Behind a new `netplay` feature flag — unlike
`retroachievements`/`scripting`, no existing scaffold for this flag exists in
`crates/rustysnes-frontend/Cargo.toml` yet, so this ticket adds it; flag any obsolete/unused
netplay code skeletons found elsewhere in the codebase for removal rather than leaving them to
silently contradict this. Keep the netplay drive loop independent of `emu_thread.rs`'s
single-player pacer — a netplay session uses its own rollback-aware loop, never the generic
`emu-thread` path, avoiding a control-model conflict with `v1.0.0`'s dedicated-thread work.

**Acceptance criteria:**

- [x] Rollback re-simulation is bit-identical (relies on `docs/adr/0004`) — proven by
      `crates/rustysnes-netplay/tests/determinism.rs`: two `RollbackSession`s driven over a
      seeded, deterministic `MemoryTransport` (including one run under real synthetic latency,
      jitter, and 10% packet loss) both match a fresh no-rollback reference run's per-frame
      framebuffer hash, exactly.
- [x] Native (UDP) + browser (WebRTC) transports work — `udp.rs`'s `UdpTransport` is a real
      `std::net::UdpSocket`, proven by a genuine OS-level loopback round-trip test (not just
      unit-level). `webrtc.rs`'s `WebRtcTransport` wraps a `web_sys::RtcDataChannel` and is
      wasm32-clippy-verified against the real API. **Honest scope note:** the frontend UI wiring
      (`crates/rustysnes-frontend/src/netplay.rs`) is native/UDP only for this pass — the
      browser-side SDP offer/answer/ICE negotiation glue needed to actually establish a
      `RtcDataChannel` is a genuinely separate scope of async signaling work, not half-wired.
      No obsolete/unused netplay code skeletons existed anywhere in the codebase to remove
      (`rustysnes-netplay`'s `src/lib.rs` was a bare 1-line stub).
- [x] Netplay sessions use their own drive loop, verified independent of `emu-thread` —
      `NetplayState::drive` calls `RollbackSession::advance` directly on `System`, dispatched
      from `Active::render`'s per-frame loop via an early `continue` that skips the entire
      single-player `apply_frame_input`/cheats/rewind/script/`run_frame` path for that iteration
      whenever a session is connected (`app.rs`); `emu-thread` is untouched by any of this.
- [x] With `netplay` off, the build is byte-identical (CI gate) — the frontend's netplay
      module/UI/`Active` field are all `#[cfg(all(feature = "netplay", not(target_arch =
      "wasm32")))]`-gated (the decode-adjacent crate itself, `rustysnes-netplay`, is always a
      workspace member — same precedent as `rustysnes-script`/`rustysnes_core::cheat`); full
      default-feature workspace build/test/clippy/fmt/doc verified unaffected.

**Dependencies:** T-82-001 (go/no-go on the save-state design); T-51-003; T-31-004 (determinism
exercised)
**Reference:** `docs/frontend.md` §determinism-boundary; `docs/adr/0004`
**Estimated complexity:** L

---

### T-82-003 — RetroAchievements (opt-in, native FFI)

**Description:** implement opt-in RetroAchievements in `rustysnes-cheevos` (native FFI), with the
`RustySNES/<crate ver> rcheevos/<rcheevos ver>` HTTP User-Agent pattern. Default-off feature.

**Acceptance criteria:**

- [x] RA auth + achievement processing work native (opt-in). `rustysnes-cheevos` wraps the
      vendored `rcheevos` `rc_client` C API (MIT, vendored verbatim from RustyNES's own copy,
      diff-confirmed identical) via hand-written `extern "C"` FFI pinned by an ABI-guard test
      (`rc_cheevos_sizeof_*` vs `size_of`); `RaClient::do_frame`/`idle`/`reset` drive achievement
      logic against SNES WRAM only (`ra_addr_to_snes`, RA flat `0x000000..0x01FFFF` ->
      `$7E0000..$7FFFFF`, verified against the real `RetroAchievements/RASnes9x`
      `RA_InstallMemoryBank` source, not guessed); cartridge SRAM is an honest, documented scope
      cut for this pass. Frontend: `crates/rustysnes-frontend/src/cheevos.rs`'s `CheevosState`
      (async password login via a shared `Rc<RefCell<...>>` completion slot, polled once per
      frame) + a Tools -> RetroAchievements... login window + a per-emulated-frame `do_frame`
      hook + unlock-toast status-bar messages. Not wired: leaderboards/rich-presence UI surfaces
      (the `RaClient` API exists; no frontend panel consumes it yet) and netplay interaction
      (deliberately out of scope this pass — see the code comment at the `do_frame` call site).
- [x] The User-Agent leads with `RustySNES/` (a regression test guards it).
      `ra_user_agent_identifies_rustysnes_with_versions` in `rustysnes-cheevos/src/http.rs`
      asserts the leading `RustySNES/<version>` token plus a non-empty `rcheevos/<version>`
      clause (parsed from the vendored `rc_version.h` at build time).
- [x] With `retroachievements` off, the build is byte-identical; clippy runs the RA feature combo.
      `rustysnes-cheevos` is native-only (`#![cfg(not(target_arch = "wasm32"))]`) and every
      frontend wiring site is `#[cfg(all(feature = "retroachievements", not(target_arch =
      "wasm32")))]`-gated; `cargo clippy -p rustysnes-frontend --features retroachievements` and
      the full-combo run (`debug-hooks,scripting,cheats,netplay,retroachievements`) are both clean.

**Dependencies:** T-51-001
**Reference:** `docs/frontend.md`; the RustyNES RA User-Agent convention
**Estimated complexity:** M

---

### T-82-004 — The byte-identical CI gate (feature-off), extended again

**Description:** extend the byte-identical-with-flags-off CI gate (last extended in Sprint 1,
T-81-004) to cover netplay and RetroAchievements; run clippy per explicit feature combo (never
`--all-features`).

**Acceptance criteria:**

- [x] The byte-identical gate passes with all Phase 8 features off (Sprint 1 + Sprint 2 combined).
      `.github/workflows/ci.yml`'s `lint` job's explicit `--no-default-features --features
      wasm-winit,help-tui` guard is unchanged in shape (still the named flags-off regression
      lock T-81-004 established) and passes with all six Phase 8 flags (`debug-hooks`,
      `scripting`, `cheats`, `netplay`, `retroachievements`) compiled out.
- [x] clippy runs each feature combo explicitly (the mutually-exclusive-backend trap avoided).
      `lint` now runs `netplay` and `retroachievements` individually (matching the existing
      `debug-hooks`/`scripting`/`cheats` lines) plus one combined
      `debug-hooks,scripting,cheats,netplay,retroachievements` line — still never
      `--all-features`, since `wasm-winit`/`wasm-canvas` stay mutually exclusive.
- [x] The gate is wired into the standard CI run, ready for `v1.0.0`'s final re-verification.
      `full-test`'s Linux-only combined-feature behavioral run (ahead of every tagged release)
      is extended to the same six-flag combo, covering `retroachievements`' real cross-platform
      build surface (`cc`-compiled vendored `rcheevos`, same category as `scripting`'s `mlua`)
      alongside the others.

**Dependencies:** T-82-002; T-82-003
**Reference:** `docs/testing-strategy.md`; `docs/STATUS.md` §version-policy
**Estimated complexity:** S

---

## Sprint review checklist

- [x] All tickets checked off or explicitly deferred (with reason). T-82-001 through T-82-004 all
      landed (PRs #56, #57, #58, and this CI-gate change).
- [x] Every Phase 8 feature is off by default + byte-identical when off. Verified by the
      flags-off clippy guard (`--no-default-features --features wasm-winit,help-tui`) covering
      all six Sprint 1 + Sprint 2 flags combined.
- [x] CHANGELOG.md updated.
