# App Store §4.7 — mobile-shell supplement

> **This is NOT the project's §4.7 self-audit.** That audit already existed when this file was
> written: it is "App Store §4.7 self-audit" in `docs/mobile-readiness.md`, completed in `v1.30.0`
> (#291), and **it is the authoritative one**. This file was created in #332 under the false belief
> that no audit existed — the claim is corrected here rather than removed, because the two reach
> **different verdicts on trademark exposure** and a reader needs to know which governs.
>
> | | scope | trademark verdict |
> |---|---|---|
> | `mobile-readiness.md` (#291) — **authoritative** | the whole shipped tree | **findings; maintainer decision needed** (`cli.rs`, `ui_shell.rs`: "Super Nintendo Entertainment System", "Super Famicom", a "Super Scope" picker label) |
> | this file (#332) — supplement | **only** `android/app/src/main` and `ios/RustySNES` | clean *within that scope* |
>
> Both are true of their own scope. This one is narrower, and being narrower is the only reason its
> trademark section reads clean — it never looked at the desktop frontend. Where they differ, the
> wider audit governs.
>
> What this file adds that the authoritative audit does not: a per-criterion capability argument for
> the two mobile shells specifically, with file:line citations, useful when the store submission is
> scoped to those shells.

**Audit date:** 2026-08-02 · **Audited revision:** `main` at the `A6.15` merge (#331) ·
**Result within the mobile-shell scope: PASS on every criterion checked**, with two items flagged
for re-audit before submission — and see the authoritative audit for the trademark findings this
scope cannot see.

## Scope

App Store Review Guideline **§4.7 (Mini apps, mini games, streaming games, chatbots, plug-ins and
game emulators)** permits retro game console emulator apps, subject to the software offered inside
them being lawful and, in practice, user-provided. Google Play's equivalent position is narrower in
form but the same in substance for this app: the emulator itself is not the problem, the content
supply is.

Separately from §4.7, **trademark exposure** is its own App Review and legal concern — an emulator
that names or depicts a console manufacturer's marks invites a rejection that has nothing to do with
§4.7. Both are audited here because both are decided by the same shipped strings.

## Criterion 1 — the app ships no game software · **PASS**

No ROM, BIOS, or firmware image is bundled in either shell.

```text
find android/app/src/main -iname '*.sfc' -o -iname '*.smc' -o -path '*assets*'   -> nothing
find ios -iname '*.sfc' -o -iname '*.smc'                                        -> nothing
```

Android has no `assets/` directory at all. The `jniLibs` the APK does carry are this project's own
`.so` builds (`android/app/build.gradle.kts:46-49`), produced at build time and deliberately not
checked in.

## Criterion 2 — every ROM is user-supplied, through the system picker · **PASS**

There is exactly one way a ROM enters either app, and it is the platform's own document picker. The
user chooses a file they already possess; the app never names, suggests, or reaches for a source.

| shell | mechanism | file:line |
|---|---|---|
| Android | `ActivityResultContracts.OpenDocument()`, read via `contentResolver.openInputStream` | `MainActivity.kt:73`, `:173` |
| iOS | SwiftUI `.fileImporter`, read under `startAccessingSecurityScopedResource()` | `ContentView.swift:52`, `EmulatorViewModel.swift:26` |

The iOS path going through a security-scoped resource is the correct sandbox behaviour for a
user-selected file and is worth noting as evidence the picker is genuine rather than decorative.

## Criterion 3 — no capability to obtain game software · **PASS**

Neither shell can fetch anything.

- **Android declares no permissions at all** — `AndroidManifest.xml` contains no
  `<uses-permission>` element, so not even `INTERNET`. An app without `INTERNET` cannot download a
  ROM by any route.
- **iOS has no networking code and no ATS configuration.** The only `http` string anywhere in the
  bundle is the `DOCTYPE` URL in `Info.plist`'s XML preamble (`Info.plist:2`) — a schema identifier,
  not a request.

This is the strongest single fact in the audit: the "user-provided software" requirement is
enforced by the app's *capabilities*, not merely by its UI.

## Criterion 4 — no third-party trademark exposure **in the mobile shells** · PASS *in scope*

**Read the authoritative audit's §3 first.** It examined the whole tree and found trademark strings
in `cli.rs` and `ui_shell.rs` that require a maintainer decision. Nothing below contradicts that;
this section says only that the *mobile shells* add none of their own.

Every user-visible string in both shells was enumerated. In full:

| string | where |
|---|---|
| `RustySNES` | `AndroidManifest.xml:6` (`android:label`), `Info.plist` (`CFBundleDisplayName`) |
| `Open ROM` | `MainActivity.kt:346`, `ContentView.swift:33` |
| `Save State` | `MainActivity.kt:348`, `ContentView.swift:35` |
| `Load State` | `MainActivity.kt:351`, `ContentView.swift:37` |

That is the complete set. A search of both shells for `nintendo`, `super nintendo`, `famicom`,
`snes`, `super scope`, `multitap`, `game boy`, `mario` and `zelda` returns **nothing** outside this
project's own `RustySNES` / `com.doublegate` identifiers.

Two points worth stating rather than leaving implicit:

- **`RustySNES` is the app name, and `SNES` inside it is the thing to watch.** It is the project's
  own established name, used consistently and not styled to resemble any manufacturer's mark, and
  the app makes no claim of affiliation. It is nonetheless the one string in the audit that touches
  a third-party mark at all, and it is named here so a future reviewer sees it was considered rather
  than missed.
- **The peripheral names are not exposed.** `Super Scope`, `Multitap` and `Mouse` appear in the
  emulator core and in `rustysnes-mobile`'s API, but no mobile shell surfaces them — because the
  peripheral picker UI does not exist yet (`docs/mobile-readiness.md`, "Mouse/Super Scope touch UX").
  **This is the audit's main re-audit trigger:** the moment that UI lands, this criterion has to be
  re-run, and the naming chosen then is a decision this audit does not pre-approve.

## Criterion 5 — no monetization surface, so no §3.1 interaction · **PASS**

`rustysnes-monetization` is compiled into both shells and is **inert**: no Play Billing client, no
StoreKit, no purchase or product call reaches either shell.

```text
grep -rnE 'BillingClient|StoreKit|purchase|SKProduct' android/app/src/main/kotlin ios/RustySNES/Sources
  -> nothing
```

`-E`, and that matters for reproducibility: without it `grep` reads the `|` literally and matches only
the whole string, so the command would report "nothing" for a reason that has nothing to do with the
code. An audit whose command cannot be re-run is an assertion, not an audit.

The crate's own module doc states the dormancy as the design (`crates/rustysnes-monetization/src/lib.rs:6-10`).
Because nothing is sold and no ads are served, §3.1 (In-App Purchase) and the ad-disclosure rules do
not engage at this revision. **Re-audit trigger:** activating it changes that immediately, and the
dormant-vs-live decision is explicitly part of the `v2.0.0` scope.

## Findings

**No blocking findings.** Two items to re-audit before any submission, both flagged above:

1. **The peripheral UI, when it lands** — Super Scope / Mouse / Multitap affordances will need
   user-visible names, and those names are a fresh trademark decision.
2. **`rustysnes-monetization`, if activated** — engages §3.1 and the ad-disclosure rules that this
   revision does not touch.

Neither is a defect. Both are consequences of work that is deliberately not done yet, and recording
them here is what stops a future rung from shipping them past an audit that predates them.

## What this audit does not cover

Out of scope by construction, and each is maintainer-blocked rather than merely undone:

- **Distribution signing and TestFlight** — the `ios.yml` upload step is an explicit no-op pending
  real signing secrets.
- **Google Play's Data Safety form** — a console declaration, not a code property. Note the audit
  above supplies its likely content: no permissions, no network, no data leaves the device.
- **The store-submission readiness assessment itself** — Mobile Phase 6, an explicit maintainer
  go/no-go, scoped to `v2.0.0`.
