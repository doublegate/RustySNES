# RustySNES `v1.31.0` "Ledger"

**Released:** 2026-08-04 · **Previous release:** [`v1.30.1` "Imprint"](https://github.com/doublegate/RustySNES/releases/tag/v1.30.1)

> **RustySNES is permanently open-source and income-free.** All monetization is removed — no ads,
> no tracking-for-revenue, no freemium, no demo or time gate, no in-app purchase, no paid unlock,
> in any build on any platform. The Android and iOS apps are **kept as free apps**; only the
> paid/ad layer is gone.
>
> **Zero emulation-core behaviour change.** No `rustysnes-*` crate ever depended on the removed
> code, so the deterministic core is untouched by construction.
>
> A ledger records what a thing actually is. `v1.30.1` made the provenance record honest; this one
> makes the commercial record honest.

---

## Executive summary

| | |
|---|---|
| Decision of record | [ADR 0015 — RustySNES is permanently non-commercial](../adr/0015-rustysnes-is-permanently-non-commercial.md) |
| Emulation behaviour | **unchanged** — AccuracySNES 56/56, scenes 2/2, coverage still **361/443**, no golden moved |
| Crates removed | 1 (`rustysnes-monetization`, 179 lines) |
| Workspace tests | **exactly 5 fewer** — the removed crate's own unit tests, no others |
| Save-state format | unchanged |
| Apps | **still free, still fully functional, still shipped** |

## 1. What was actually there

Stating this plainly is what makes the removal easy to judge, and it is more modest than the word
"monetization" suggests. `crates/rustysnes-monetization` was a 179-line UniFFI crate shipped in
`v1.18.0 "Dormant"` as a policy-*shape* template:

- `check_entitlement` was a **stub that always returned `unlocked: true`**, ignoring its timestamp.
- `should_show_ad` had real pacing math and **zero call sites** outside its own tests.
- Every number in it — a 300-second interval, 3 sessions before a first ad — was an explicit
  placeholder, never a committed figure.
- Both shells called it **once at startup and wrote the result to a log line.** It gated nothing.
- **No Play Billing, StoreKit, RevenueCat, AppLovin, AdMob, ad SDK, paywall UI, purchase flow,
  persisted entitlement, or network call ever existed.** The only mentions of those vendors in the
  entire tree were two comments saying they were *not* wired up.

It was never a dependency of the deterministic core, and had no knowledge of `EmuCore` at all. That
is precisely why removing it cannot move an accuracy number.

## 2. Why it is going

The project's central claim is **accuracy a sceptic can check without trusting the author** — an
on-cart battery that scores itself, three independent reference emulators, a coverage report
regenerated with the artefact it describes, and a changelog that keeps its own retractions. A
revenue interest sits awkwardly against that posture, and it complicates the honesty/provenance
stance `v1.30.1` had just finished cleaning up.

Worth noting too: **every reference emulator RustySNES measures itself against — ares, bsnes,
Mesen2, MesenCE, snes9x — is free and non-commercial.** Nothing in the accuracy programme ever
depended on a revenue model.

## 3. Deleted, not disabled

The distinction is the point. Everything is gone:

| Surface | Removed |
|---|---|
| Crate | `crates/rustysnes-monetization/` (3 files) + the workspace member + its `.gitignore` rule |
| Android Gradle | the `cargoNdkBuild` package, the `jniLibs` include entry, the entire `uniffiBindgenMonetization` task and its generated-sources dir, the `preBuild` dependency |
| Android shell | `MainActivity.kt`'s two imports, the startup call, and `logMonetizationScaffold()` |
| iOS shell | `RustySNESApp.swift`'s `init()` call and `logMonetizationScaffold()` |
| CI | `android.yml`'s two path filters and both `cargo ndk` package lists |
| iOS packaging | the crate's build, bindgen and framework slices in `build-ios-xcframework.sh` |

**Re-introducing monetization would now require a new ADR explicitly reversing ADR 0015**, which is
the intended bar. Disabling a flag is reversible by accident; deleting a crate is not.

## 4. Two things deliberately kept

Both are cases where the obvious cleanup would have quietly removed a safeguard.

**The Android APK presence gate stays by-name, at two libraries.** It would have been easier to
turn `librustysnes_android.so librustysnes_mobile.so librustysnes_monetization.so` into a count.
That gate was hardened in `v1.28.0` precisely so a packaging regression fails loudly — a count
would pass an APK that shipped the *wrong* library. Both 16 KB page-alignment gates are untouched.

**The umbrella `RustysnesFFI.xcframework` shape is retained** even though only one crate now feeds
it. Packaging a second UniFFI crate is what surfaced a genuine `xcodebuild` failure on a real macOS
CI run — `"Multiple commands produce '.../include/module.modulemap'"` — because a
`library`+`-headers` xcframework has its headers copied into **one directory shared across every
such xcframework in the target**, and each crate's modulemap is renamed to the single filename
Clang requires. The `libtool -static` merge that fixed it is kept so a future second FFI crate is a
one-line change rather than a rediscovery, and because `ios/project.yml` links that artifact name.

**That collision is `v1.18.0`'s one durable legacy, and it outlives the rung that caused it.**

## 5. What this changes about the roadmap

- **`v2.0.0` stops meaning "store submission"** and is repurposed for accuracy/fidelity work. A
  free app changes no format and breaks no API, so a listing can land on the `v1.x` line whenever
  the gate opens. The MAJOR bump returns to what the version plan's own rule always said — a
  public-API or save-state-format break, with [ADR 0002](../adr/0002-fractional-timebase-refactor.md)'s
  fractional-timebase refactor the one expected candidate. The freed slot points at the ~82
  uncovered AccuracySNES rows (against the ~422 soft ceiling), the dot-model residuals, and the
  reference-disagreement-blocked hi-res compositor gaps.
- **Mobile Phase 6 becomes a purely technical gate.** **STATUS stays NOT GREENLIT.** A free
  listing — no ads, no purchase — remains possible but unscheduled and tied to no version number.
- **§4.7 Criterion 5 gets *stronger*.** It previously argued *dormancy*: a monetization crate
  existed but was inert, so §3.1 did not engage *at that revision*. It now rests on structure — the
  code is not there — and has **no re-audit trigger**, because it can only change by formally
  reversing an ADR. Finding #2 is **closed, not deferred**.
- **A lockstep divergence guard.** The checklist's job is to flag sibling features RustySNES lacks,
  so without an explicit row a future parity pass would helpfully re-propose monetization. The
  correct disposition is now *decline, citing ADR 0015*. (The sibling reached the same decision
  independently in its own ADR 0035 on the same day; the guard is written for the future case.)

## 6. What is *not* changed

- **Historical records stand.** The released `[1.18.0]`, `[1.28.0]` and `[1.30.0]` CHANGELOG
  sections and every `RELEASE_NOTES_*.md` are untouched; the `v1.18.0` entries in the version plan
  and roadmap gain a removal *pointer* rather than being excised. The scaffolding really did ship
  and really was dormant. The reversal belongs in an ADR, not in a rewrite of the record.
- **The mobile engineering stays.** `rustysnes-mobile`, `rustysnes-android`, both shells, the 16 KB
  alignment gates, the UniFFI smoke test that boots a real cart on a device, the iOS simulator
  launch, and the §4.7 audit all remain — they serve a free app exactly as well as a paid one.
- **No funding surface was added or removed.** There is no `.github/FUNDING.yml` and no
  sponsor/donate link anywhere; there never was.

## Compatibility and upgrade notes

- **Emulation behaviour:** unchanged. No chip model, timing constant, or golden vector moved.
- **Save states:** format version unchanged.
- **Anyone depending on `rustysnes-monetization`:** nothing did — it was never a dependency of any
  crate in this workspace, and it was never published.
- **Android/iOS builds:** two native libraries per ABI instead of three. A build script that
  enumerated three will need updating; the workflow's own gate already does.

## Verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features
cargo test --workspace                                  # exactly 5 fewer tests than v1.30.1
cargo test -p rustysnes-test-harness --features test-roms --test accuracysnes --test accuracysnes_scenes
```

The load-bearing check is structural: **no `rustysnes-*` crate ever depended on the removed crate**,
so the accuracy numbers cannot move. If any of them had, that would have meant the change was wrong
— not the number.

---

Full per-entry detail: [`CHANGELOG.md` → `[1.31.0]`](../../CHANGELOG.md). The decision of record is
[ADR 0015](../adr/0015-rustysnes-is-permanently-non-commercial.md).
