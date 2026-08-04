# RustySNES `v1.30.0` "Threshold"

**Released:** 2026-08-03 · **Previous release:** [`v1.29.0` "Triangulate"](https://github.com/doublegate/RustySNES/releases/tag/v1.29.0)

> The last rung before the store gate — and the one that says plainly what is on the far side of it.
> Mobile store-readiness engineering, a **51% headless frame-time regression** bisected and fixed,
> and a fuzzing campaign that turns out never to have run.
>
> **Mobile Phase 6 stays NOT GREENLIT.** This release removes prerequisites from that gate's
> checklist; it does not move the gate. The submission itself is `v2.0.0` and a maintainer decision.

---

## Executive summary

| | |
|---|---|
| Commits | 3 (PRs #332–#333, plus one correction) |
| Diff | 10 files changed, +785 / −25 |
| AccuracySNES coverage | **361 of 443** (304 on-cart + 55 scene + 2 host) — unchanged |
| Headless frame time | **14.34 ms → 7.03 ms** (−51%), against a 16.64 ms NTSC deadline |
| Fuzz targets actually running | **0 → 14** |
| Save-state format version | unchanged |

Three things:

1. **A mobile-shell §4.7 supplement, an unsigned release build path, and an instrumented UniFFI
   smoke test** — plus a correction to four stale claims in the readiness doc, and a correction to a
   false claim this release's own first entry made.
2. **The frame-time gate had been red since 2026-08-01, and it was right.** Headless frame
   production had halved. `git bisect run` named the commit exactly.
3. **The fuzzing infrastructure had never actually run a campaign.** All 14 targets reported a
   FINDING within about a second each, with `fuzz/artifacts/` empty — and that uniformity is the
   tell.

---

## 1. Mobile store-readiness

### The §4.7 supplement — and the correction that had to accompany it

**Read this part carefully, because the first version of this entry was wrong.** It said the §4.7
self-audit was outstanding and that #332 performed it. **That was false.** The audit already
existed, in `docs/mobile-readiness.md`, done in #291 during the `v1.28.0` window, and **it remains
the authoritative one.**

What #332 actually added is a **mobile-shell supplement** scoped to `android/app/src/main` and
`ios/RustySNES` only. The distinction matters, because the two reach **different verdicts on
trademark exposure**:

| | scope | trademark verdict |
|---|---|---|
| `docs/mobile-readiness.md` (#291) — **authoritative** | the whole shipped tree | **findings; maintainer decision needed** — `cli.rs`, `ui_shell.rs` carry "Super Nintendo Entertainment System", "Super Famicom", and a "Super Scope" picker label |
| `docs/app-store-4-7-self-audit.md` (#332) — supplement | **only** the two mobile shells | clean *within that scope* |

Both are true of their own scope. The supplement is narrower, and **being narrower is the only
reason its trademark section reads clean** — it never looked at the desktop frontend. Where they
differ, the wider audit governs. Both files now say so at the top.

The supplement passes on all five criteria it checks, and the strongest evidence is **capability
rather than intent**:

- **Android declares no permissions at all — not even `INTERNET`.** An app without `INTERNET` cannot
  download a ROM by any route.
- **iOS has no networking code and no ATS configuration.** The only `http` string anywhere in the
  bundle is the `DOCTYPE` URL in `Info.plist`'s XML preamble.
- **Exactly one way a ROM enters either app**, and it is the platform's own document picker
  (`ActivityResultContracts.OpenDocument()` on Android; SwiftUI `.fileImporter` under
  `startAccessingSecurityScopedResource()` on iOS).
- **Every user-visible string in both shells was enumerated.** The complete set is `RustySNES`,
  `Open ROM`, `Save State`, `Load State`.

Two re-audit triggers are recorded rather than left implicit: **the peripheral UI when it lands**
(Super Scope / Mouse / Multitap affordances will need user-visible names, and those names are a
fresh trademark decision this audit does not pre-approve), and **`rustysnes-monetization` if it is
ever activated** (which engages §3.1 and the ad-disclosure rules immediately).

### An unsigned `assembleRelease` path

With its own 16 KB alignment gate on the **release** APK, reusing the same `aligned16k()` divisibility
helper the debug gate uses.

Signing material is the maintainer's to provision, so a *signed* release build stays out of reach —
but an unsigned one still runs R8, resource shrinking, and the release manifest merge, which is where
release-only breakage lives. `isMinifyEnabled` is `false` today, so this currently proves the release
variant assembles; **it is wired now because the moment minification is enabled, this is what catches
the fallout.**

### An instrumented UniFFI smoke test

`android/app/src/androidTest`, in its own CI job.

`assembleDebug` already proves the bindings *compile* — `MainActivity` calls `MobileCore` directly.
**What no build can prove** is that `System.loadLibrary` finds the `.so` for the device's ABI, that
JNA's mapping matches its symbols, and that a call marshals across and returns. This project has
already shipped one native Android crash a build could not have caught.

The strongest of the four tests loads a real cartridge (`accuracysnes-hirom.sfc`, copied into
androidTest assets by a Gradle `Copy` task wired into `preBuild`) and requires it to boot and produce
audio. Two earlier drafts of that assertion were **vacuous** — `0 == 0`, then `0 % 2 == 0` — and were
fixed by booting a real cart rather than by tightening the predicate.

It is a **separate job** because an emulator is the flakiest thing in that workflow, and a flaky step
inside `build` would put the 16 KB gates — which are not flaky, and which gate a real Play
requirement — behind an AVD boot.

### Four stale entries in the readiness doc

Marked **DONE rather than deleted**, because a readiness document that silently drops items cannot
be audited backwards: `android.yml` exists and gates alignment twice, the `./gradlew` wrapper is
committed, `ios.yml` boots a simulator and requires the app to survive the launch, and the §4.7 audit
is done.

**What remains genuinely outstanding is stated as such** — distribution signing, TestFlight, and
Play's Data Safety form, all maintainer-blocked.

---

## 2. The frame-time regression: 14.34 ms → 7.03 ms

The frame-time gate had been red since `2026-08-01`, **and it was right.**

`check_hv_irq` runs once per dot — some **89,000 times a frame** — and `v1.29.0`'s
`fix(ppu): derive the H-IRQ dot from the clock` (#300) had it walk the scanline **from dot 0 on every
call** to find the comparator's dot, up to 341 steps. Roughly **30 million iterations a frame.**

Measured on a dev machine: **6.83 ms/frame before that commit, 13.31 ms after**, against a 16.64 ms
NTSC deadline. `git bisect run` over the 22-commit window, using the benchmark itself as the
good/bad oracle, named it exactly.

**Two changes, both value-preserving:**

1. **The walk starts from a lower bound rather than dot 0.** Every dot is at least 4 clocks, so the
   answer cannot be below `ceil((target - 4) / 4)`, and from there it converges in at most two steps.

   ```rust
   let start = target.saturating_sub(4).div_ceil(4);
   if start > DOTS_PER_LINE as u32 { return u16::MAX; }
   let mut dot = start as u16;
   while dot <= DOTS_PER_LINE {
       if clocks_before_dot(dot, short_line) >= target { return dot; }
       dot += 1;
   }
   u16::MAX
   ```

   **The `- 4` matters.** A bound of `ceil(target / 4)` *overshoots* for targets landing just past
   dot 323, where the two 6-clock dots make the prefix exceed `4 * dot`.

2. **The target is computed inside the `irq_enable_h` branch** instead of above it, because the
   V-only arm never reads it — so a ROM that uses no H-IRQ now pays nothing at all.

**Two corrections to an earlier draft of this entry, both caught in review**, and both worth stating
because they change what the number means:

- **47% was Criterion's own change-against-its-saved-baseline**, a different comparison than the one
  the sentence was making. The 14.34 → 7.03 delta is **51%**.
- **7.03 ms is 3% above** the 6.83 ms measured before #300, **not "back to" it.** The remaining gap
  is the other commits that landed in the same window.

**Safety is an exhaustive test.** `the_bounded_walk_matches_an_exhaustive_walk_from_zero` compares
against the **original function verbatim** for every `HTIME` on both line lengths — the change is
compared with what it replaced, not with a belief about what it replaced. A second test pins the
closed-form clock prefix against an accumulating walk.

Battery 56/56, framebuffer goldens unmoved, 68 workspace suites green.

---

## 3. The fuzzing campaign had never run

`Fuzz Campaign` is skipped on every push and pull request and runs only on `security.yml`'s weekly
cron, so the `2026-08-03` scheduled run was **its first real execution** — and all **14 targets
reported a FINDING within about a second each**, with `fuzz/artifacts/` empty.

**That uniformity is the tell**, and `fuzz/run.sh`'s own header already names the failure mode for a
different cause:

> *"A campaign that reports 14 findings and has actually found none is worse than one that reports
> nothing."*

**Root cause:** `cargo fuzz` defaults `--target` to the triple **the cargo-fuzz binary itself was
built for**. CI installs it via `taiki-e/install-action`, which ships a statically linked **musl**
build — so on a gnu runner every target failed with `sanitizer is incompatible with statically linked
libc` and `can't find crate for core`. And `run.sh` counts a non-zero exit as a finding, because a
build failure and a crash look alike.

**It never reproduced locally** because a `cargo install`ed cargo-fuzz is a gnu build whose default is
already correct.

`run.sh` now passes `--target` explicitly, taken from `rustup run nightly rustc -vV` — **not**
`rustc +nightly`, because the `+toolchain` form is rustup's shim, not rustc's, so it fails wherever
`rustc` on `PATH` is a real binary. Both environments now agree.

Verified by running a real campaign: `rom_header` clean at `cov: 759 ft: 978`, where before it exited
in under a second having built nothing.

---

## Compatibility and upgrade notes

- **Save states:** format version unchanged.
- **Emulation behaviour:** unchanged. The frame-time fix is **value-preserving by construction** —
  the exhaustive test compares it against the original function verbatim across the full `HTIME`
  range on both line lengths.
- **Performance:** headless frame production roughly halves in cost for any ROM; a ROM that uses no
  H-IRQ now pays nothing for the comparator at all.
- **Fuzzing:** `fuzz/run.sh` now requires a nightly toolchain reachable through `rustup run nightly`.
  Override the triple with `FUZZ_TARGET_TRIPLE` if you need a non-host target.
- **Android:** an unsigned `assembleRelease` variant now builds in CI. Instrumented tests require an
  emulator; that job is separate and does not gate the alignment checks.

## Known issues and what is deliberately not done

- **Mobile Phase 6 is NOT GREENLIT.** Distribution signing, TestFlight, and the Play Data Safety
  form are maintainer-blocked. The store-submission readiness assessment is `v2.0.0`.
- **Trademark findings remain open** in `cli.rs` and `ui_shell.rs`, per the authoritative audit. The
  recommendation is recorded; the decision is the maintainer's and has not been made.
- **No ROM has run on iOS.** `ios.yml` proves the app boots and survives eight seconds; the app
  bundles no cartridge. The Android smoke test does boot a real cart — that asymmetry is stated
  rather than smoothed over.
- **`rustysnes-monetization` is inert** and compiled into both shells. Activating it engages §3.1
  and is explicitly `v2.0.0` scope.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                                  # 68 suites green
cargo test --workspace --features test-roms
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features
bash fuzz/run.sh                                        # a real campaign, not 14 instant "findings"
```

- AccuracySNES battery **56/56**; framebuffer goldens unmoved.
- Frame-time gate green: 7.03 ms against a 16.64 ms budget.
- All 14 fuzz targets build and run; `rom_header` reaches `cov: 759 ft: 978`.

## Included changes

| PR | Commit | Summary |
|---|---|---|
| #332 | `3d6eeb7` | `feat(mobile)`: add the App Store 4.7 self-audit, a release build path, and a UniFFI smoke test |
| #333 | `ecdb063` | `fix(ppu,fuzz)`: restore headless frame time; make the fuzz campaign run |
| — | `09bac2b` | `docs(mobile)`: correct a false claim — the §4.7 audit already existed |

Full per-entry detail: [`CHANGELOG.md` → `[1.30.0]`](../../CHANGELOG.md).
