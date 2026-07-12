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
