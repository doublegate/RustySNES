# ADR 0015 — RustySNES is permanently non-commercial (no monetization)

## Status

Accepted (`v1.31.0`).

**Supersedes nothing.** All fourteen prior ADRs were checked; none records a monetization
decision — the scaffold this ADR removes was introduced by a release rung, not by an architecture
decision. **Amends** [ADR 0012](0012-mobile-platform-target.md) only in that its mobile platform
target is now explicitly a **free**-app target.

## Context

`v1.18.0 "Dormant"` (2026-07-14, Mobile Phase 5) added `crates/rustysnes-monetization`: a 179-line
UniFFI crate exposing `check_entitlement`, `default_ad_pacing_policy` and `should_show_ad`, plus
`Entitlement` and `AdPacingPolicy` records. It was ported from the sibling project as a policy-*shape*
template, and it was dormant by design:

- **It was never a dependency of the deterministic core.** No `rustysnes-*` crate ever listed it;
  it reached the apps only as a separately-linked native library plus generated bindings, and it
  had no knowledge of `EmuCore` at all.
- **It gated nothing.** Both shells called it exactly once at startup and wrote the result to a log
  line. `check_entitlement` was a stub that always returned `unlocked: true`.
- **No store SDK was ever wired up.** No Play Billing, StoreKit, RevenueCat, AppLovin, AdMob, ad
  SDK, paywall UI, purchase flow, persisted entitlement, or network call existed anywhere in the
  tree. The only mentions of those vendors were two comments saying they were *not* wired up.
- **Every number in it was an explicit placeholder**, deferred to a later "Mobile Phase 6"
  store-launch decision.

That decision has now been made, and it is the opposite one. The maintainer has decided a paid
layer is the wrong direction for RustySNES.

The reasoning is worth recording, because it is not merely a preference. This project's central
claim is **accuracy that a sceptic can check without trusting the author** — an on-cart battery
that scores itself, three independent reference emulators, a coverage report regenerated with the
artefact it describes, and a changelog that keeps its own retractions. A revenue interest in the
product sits awkwardly against that posture, and it complicates the honesty/provenance stance the
project has just spent a release cleaning up (`docs/originality-and-provenance.md`, ADR 0013's
blessing rules). It is also worth noting that **every reference emulator RustySNES measures itself
against — ares, bsnes, Mesen2, MesenCE, snes9x — is free and non-commercial.** Nothing in the
accuracy programme ever depended on a revenue model, and nothing in it now will.

## Decision

**RustySNES is and will remain open-source and income/profit-free, permanently.**

There is no monetization anywhere in the project: **no ads, no tracking-for-revenue, no freemium,
no demo or time gate, no in-app purchase, and no paid unlock — in any build, on any platform.**

**The Android and iOS apps are kept as free apps**, fully functional, with no ads and no tracking.
Only the paid/ad layer is removed. A store listing remains possible — a **free** app, no ads, no
purchase — but it is unscheduled and has no fixed version.

Concretely, in `v1.31.0`:

- `crates/rustysnes-monetization/` is **deleted** and the workspace member removed. Because no
  emulation-core crate ever depended on it, the deterministic core is untouched **by construction**:
  the AccuracySNES battery, every golden vector, and the save-state format are unchanged.
- The Android paid layer is removed: the `cargoNdkBuild` package, the `jniLibs` include entry, the
  `uniffiBindgenMonetization` task and its generated-sources directory, the `preBuild` dependency,
  and `MainActivity`'s imports and startup call. The 16 KB page-alignment gates and the by-name
  APK presence check are **retained**, now covering two libraries instead of three.
- The iOS paid layer is removed: the startup call in `RustySNESApp.swift`, the crate's bindgen
  step, and its slice of the combined framework. The **umbrella `RustysnesFFI.xcframework` shape is
  deliberately retained** even though one crate now feeds it — it is what prevents a second UniFFI
  crate from reintroducing the `module.modulemap` collision that forced the merge in the first
  place, and it is the artifact name `ios/project.yml` links.
- `v2.0.0` stops meaning "store submission" and is **repurposed for accuracy/fidelity work**. The
  generic SemVer reservation in `to-dos/VERSION-PLAN.md` stands: a MAJOR bump is for a public-API
  or save-state-format break, of which [ADR 0002](0002-fractional-timebase-refactor.md)'s
  fractional-timebase refactor remains the one expected candidate.
- Mobile Phase 6 survives as a purely **technical** free-app go/no-go. Its status stays
  **NOT GREENLIT**.

## Consequences

**Positive.**

- The project's stated nature and its shipped artifacts finally agree. Nothing has to be reconciled
  against a paid product that was never going to exist.
- No code, credential, or CI path carries an ad-SDK or billing dependency — and the App Store §4.7
  audit's Criterion 5 gets *stronger*, changing from "a monetization surface exists but is inert"
  to "no such code exists at all", which is a cleaner §3.1 answer than a dormancy argument.
- The emulation core, save-state format, and every golden vector are unchanged by construction.
  `v1.31.0` is a crate-removal, app-shell and documentation change.

**Negative / carried.**

- Historical `CHANGELOG.md` entries and `docs/releases/RELEASE_NOTES_*.md` that shipped with the
  scaffolding **remain as history**. The scaffolding was real, dormant, and released; deleting that
  record would be the dishonest move. This ADR is where the reversal is recorded.
- `to-dos/LOCKSTEP-CHECKLIST.md` carries a dated row noting this as a deliberate divergence guard,
  so a future parity pass against the sibling cannot helpfully re-propose monetization. (The
  sibling reached the same decision independently in its own ADR 0035; the guard stands regardless.)
- **The crate is deleted, not merely disabled.** Re-introducing monetization would be a new,
  deliberate decision reversing this ADR — which is the intended bar.
