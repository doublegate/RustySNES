# RustySNES ↔ RustyNES Lockstep Checklist

RustySNES's release ladder is being driven toward feature/UX/quality parity with its sibling NES
emulator, RustyNES (`../RustyNES` — both repos are sibling checkouts on this machine). RustyNES
keeps shipping on its own schedule, so rather than freezing a snapshot target, this checklist gets
run once at the start of scoping **each** release in `to-dos/VERSION-PLAN.md`'s ladder, to catch
drift before it accumulates. See `to-dos/VERSION-PLAN.md`'s RustyNES-parity ladder (`v1.5.0`
onward) for the release list this checklist re-validates against.

This is intentionally lightweight — a ~10-minute pass, not a governance process — matching a
solo-maintainer project's actual capacity. It's also written to double as a ready-made prompt: "run
`to-dos/LOCKSTEP-CHECKLIST.md` first" is a complete instruction for whoever (human or agent) scopes
the next release.

## The checklist

1. Read the **Lockstep log** table below for the most recently checked RustyNES ref/date.
2. `git -C ../RustyNES fetch && git -C ../RustyNES log <last-checked-ref>..origin/main --oneline -- CHANGELOG.md`
   — no cloning needed, both repos already live side by side.
3. Skim, since the last check:
   - RustyNES's `CHANGELOG.md` entries.
   - The top status blurb of RustyNES's `docs/STATUS.md` and `to-dos/ROADMAP.md`.
   - Specifically watch for **new oracle/regression-net *categories*** appearing (not just growth
     of existing ones) — Holy Mapperel and the PAL-APU oracle were both added as new categories
     mid-line in RustyNES's v2.1.x, not incremental growth of a suite that already existed.
   - Any CI/release-infrastructure changes (new workflow files, new promotion gates).
   - Any newly logged regressions or known-issues.
4. Classify each finding:
   - **Already covered** — RustySNES's roadmap already has equivalent scope planned or shipped.
     Log it, no further action.
   - **Small catch-up** — fits inside the release currently being scoped without displacing
     already-planned items. Fold it directly into that release's `to-dos/VERSION-PLAN.md` section.
   - **Large catch-up** — a genuinely new theme, multiple PRs' worth, or would displace
     already-planned scope. Do **not** silently cram it in. Add it as a new or deferred entry in
     `to-dos/ROADMAP.md`'s "Milestones beyond the phases" list and flag it for a maintainer
     go/no-go before any detail-scoping happens.
5. Append one row to the Lockstep log below.

**Size threshold, made concrete:** if the RustyNES change maps onto scope RustySNES's roadmap
already names, fold it in. If it would introduce a category with no existing line item, or
wouldn't fit the release being scoped without bumping already-planned items, give it its own
future rung instead. This project has already set this precedent — `v0.9.0 "Threshold"` wasn't
originally planned; it was added when Phase 7/8 leftovers surfaced during scoping, not force-fit
into an adjacent release.

**When to run it:** once, at the start of scoping each new release — never per-PR, never
continuously.

## Lockstep log

| Date | RustyNES ref checked | Findings since last check | Disposition | Notes |
|---|---|---|---|---|
| 2026-07-11 | `v2.1.5` (released) / `main` (v2.1.6 "Expansion Audio" in progress) | Initial baseline for the RustyNES-parity roadmap — see the full gap analysis in the roadmap plan. | Roadmap `v1.5.0`–`v1.19.0` scoped against this baseline. | First entry; establishes the checklist itself. |
| 2026-07-12 | `v2.1.10` / `main` | 5 RustyNES releases since baseline (`v2.1.6`–`v2.1.10`). New: a GIF/WAV screen+audio capture subsystem (`v2.1.9`); 3 marquee libretro-slang CRT presets (CRT-Royale, guest-advanced, Sony Megatron, `v2.1.9`) vs. RustySNES's own single scanline+aperture-mask `Crt` filter; a SIMD software blitter + wasm-size/startup pass (`v2.1.8`); browser RetroAchievements + Vs. DualSystem libretro support (`v2.1.10`). Raw NTSC composite-signal decode (`v2.1.9`) confirmed genuinely NES-specific (8:1 dot-clock subsampling, no SNES analog) — validates `v1.12.0`'s own out-of-scope call. `v2.1.6`/`v2.1.7` (expansion-audio oracle, 2A03/PPU die-revision modeling) are NES-chip-specific, not applicable. | Capture subsystem: **large catch-up**, no RustySNES rung exists — flagged in `to-dos/ROADMAP.md`'s "Milestones beyond the phases" for a maintainer go/no-go, not silently scoped in. CRT-preset depth: **medium catch-up**, noted as a future-rung candidate, not urgent (RustySNES's existing `Crt` filter is functional). SIMD blitter/wasm pass: **small catch-up**, could fold into `v1.19.0 "Afterburner"` alongside PGO/BOLT when that rung is scoped. Browser RA + Vs. DualSystem: correctly out of scope (RustySNES's RA is native-only; no Vs. System equivalent on SNES). Raw-composite: reconfirmed already-covered/correctly-deferred. | First re-run since baseline — overdue (8 RustySNES releases, `v1.5.0`–`v1.12.0`, had landed with no lockstep check in between); run at the start of scoping `v1.13.0`. |
| 2026-07-15 | `v2.2.0 "Capstone"` / `main` | RustyNES cut `v2.2.0` since the last check (`v2.1.10`), plus an `[Unreleased]` Dependabot-consolidation PR. New in `v2.2.0`: netplay lobby/matchmaking (browse-and-join room directory + server-side quick-match), delayed-stream spectators, a graded hysteresis-backed desync verdict, peer-liveness RTT timeouts (all `rustynes-netplay` signaling depth, B5); fuzz-target expansion from 3 → 8 cargo-fuzz targets (PPU/APU reg I/O, netplay message parsing, save-state, movie); a read-only ROM Info debugger panel (CRC32/SHA-256/header decode); 4 new MkDocs handbook pages; FDS medium-model completion (CRC-16/gap/continuous head-seek) + Famicom mic + Zapper 3×3 aperture hardening (peripherals); a fuzz-found movie-deserializer OOM-DoS hardening (untrusted `frame_count`/zero-`bytes_per_frame`). Checked RustySNES's own state directly (not assumed): confirmed no `fuzz/` directory exists at all; confirmed `rustysnes-netplay` has no `SignalMessage`/`ListRooms`/`QuickMatch`/`delay_frames`/`DesyncStatus`/`PeerLink` equivalents; confirmed `rustysnes-frontend/src/debugger/` has no ROM-Info-panel equivalent; confirmed `rustysnes-core::movie`'s deserializer is ALREADY hardened against the identical untrusted-`frame_count`/zero-length-frame OOM class (a `deserialize_rejects_a_forged_huge_frame_count_without_oom` regression test already exists — independently discovered, not copied); confirmed RustySNES's `SuperScopeState` uses a purely geometric (coordinate-clamp) offscreen check, architecturally different from a photodiode-luma-sampling model, so the Zapper aperture technique doesn't map onto it. FDS/Famicom-mic reconfirmed NES/Famicom-specific, not applicable. | **No fuzzing infrastructure at all**: **large catch-up** — genuinely new quality-infra category (not incremental growth of anything RustySNES has), and this project's own `docs/testing-strategy.md` already names fuzzability as an intended Layer 1 property never actually built out; flagged in `to-dos/ROADMAP.md`'s "Milestones beyond the phases" for a maintainer go/no-go. **Netplay lobby/matchmaking + spectator/desync/liveness depth**: **large catch-up** — multiple sub-features, would displace no currently-open rung (the ladder is closed) but is genuinely multiple PRs' worth; the `v1.5.0`-era "netplay already at parity" call is now stale against this deepened baseline; also flagged in `ROADMAP.md`. **ROM Info debugger panel**: **small catch-up**, self-contained, fits cleanly into the existing `debugger/` module architecture — candidate for the next time a small patch/quality release is scoped. **MkDocs handbook pages**: already covered in spirit (RustySNES already maintains its own MkDocs site, updated per-release; no distinct action). **Movie-deserializer OOM-DoS hardening**: already covered — RustySNES independently already has equivalent protection. **Zapper aperture hardening**: not directly applicable — architecture mismatch (RustySNES's Super Scope model doesn't sample framebuffer luma at all). **FDS/Famicom mic**: correctly out of scope (no SNES equivalent hardware). **Dependency-consolidation PR**: routine Dependabot hygiene, not a roadmap-scope item. | First re-run since the roadmap ladder closed at `v1.19.0` — confirms the checklist itself stays a live, standing practice even with no open rung to fold findings into; both large-catch-up items intentionally left unscoped pending a maintainer decision rather than opening a new rung unilaterally. |
