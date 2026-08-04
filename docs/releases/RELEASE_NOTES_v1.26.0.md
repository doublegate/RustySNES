# RustySNES `v1.26.0` "Bulwark"

**Released:** 2026-07-31 · **Tag commit:** `ff9379e` · **Previous release:** [`v1.25.0` "Workbench"](https://github.com/doublegate/RustySNES/releases/tag/v1.25.0)

> The first rung after the frontend-parity program closed. Nothing here changes what the emulator
> computes — every change is to the machinery that proves it, protects it, or measures it.
> **Zero accuracy risk by construction:** no chip model was touched.

---

## Executive summary

`v1.26.0` builds the defences the project had specified but never built. Three of them:

1. **A fuzzing layer that did not exist.** `docs/testing-strategy.md` had named Layer 1
   fuzzability as an intended property since it was written, and no `fuzz/` directory had ever
   existed. Fourteen `cargo-fuzz` targets now cover every untrusted-input boundary in the tree.
   The first campaign found a **real, reachable heap-exhaustion defect in ROM header parsing**.
2. **CI supply-chain hardening.** 15 of 17 `actions/checkout` sites stopped persisting the
   workflow token; all 34 third-party action references are pinned to immutable commit SHAs; and
   the auto-release tag-existence check now fails closed instead of treating a network blip as
   "not yet released".
3. **The golden vectors were reachable from no pull-request job.** 53 rendered-scene goldens plus
   every framebuffer golden ran only on `main`. A PR could move a golden and hear nothing about it
   until after merge.

Plus one measurement that produced a *negative* result and is published as such: run-ahead stays
opt-in, and the reason is now a number rather than an assumption.

| | |
|---|---|
| Commits | 5 (PRs #273–#277) |
| Diff | 46 files changed, +6,319 / −70 |
| AccuracySNES coverage | **344 of 443** (291 on-cart + 53 scene) — unchanged, by design |
| Save-state format version | unchanged |
| Public API | unchanged |

---

## Headline 1 — the fuzzing layer, and the defect it found within twenty seconds

### The infrastructure

`fuzz/` is a **separate cargo workspace** (its own `Cargo.toml` with an empty `[workspace]` table),
so cargo-fuzz's nightly requirement never leaks into an ordinary `cargo check --workspace` and the
targets never appear in the default build graph.

Fourteen targets, one per untrusted-input boundary:

| Target | Entry point under test |
|---|---|
| `rom_header` | `Header::detect` |
| `rom_load` | `EmuCore::load_rom`, including the third-party `zip` container path |
| `save_state` | `System::load_state` via `rustysnes-savestate`'s tagged/length-prefixed reader |
| `movie` | `Movie::deserialize` |
| `netplay_message` | `NetMessage::decode` |
| `patch` | IPS / UPS / BPS application |
| `cheat_code` | Game Genie + Pro Action Replay decoding |
| `hd_pack_manifest` | manifest TOML, tile-path resolution, RGBA conversion |
| `slang_preset` | preset parsing + the GLSL→WGSL bridge and its string rewriters |
| `config_toml` | `Config` serde-derive recursion |
| `symbols` | `SymbolMap::load` — returns no `Result` at all by design, so panic-freedom *is* the contract |
| `coproc_firmware` | user-supplied DSP-1 / CX4 chip-ROM dumps |
| `cpu_step`, `apu_reg_io` | seeded chip state driven by a register-write stream |

Every one of those entry points already returns a `Result`. What is under test is therefore
**panic-freedom, unbounded allocation, and slice-index arithmetic** — not missing error handling.

**CI split, deliberately.** A *compile gate* runs on every PR (`ci.yml`'s `lint` job) so a target
cannot silently rot out of buildability as the API it calls moves. The *campaign* is weekly
(`security.yml`, scheduled only), because per-commit fuzzing mostly re-treads a corpus it already
has.

### The defect: `$xFD8` shifted unbounded

`Header::detect` computed SRAM size as `1024 << N`, with `N` an arbitrary byte lifted straight out
of an untrusted image.

- **Debug builds panicked outright** (`attempt to shift left with overflow`) for `N >= 64`.
- **Release builds masked the shift instead** — so `N = 22` silently handed `board::select` a
  **4 GiB zeroed allocation** at `vec![0u8; header.sram_size]`. Reachable from any downloaded ROM,
  any fan hack, any bad dump.
- **`wasm32` was worse on both counts:** `usize` is 32 bits there, so the panic started at
  `N >= 32` and `N = 21` already exceeded `isize::MAX`.

The fix clamps to `MAX_SRAM_SIZE` (512 KiB) **with the shift amount capped first** — both bounds
are required, since a `min` on the *result* cannot run until the shift has already happened.

512 KiB is the smallest value that cannot refuse a real cartridge: LoROM's SRAM window reaches
448 KiB, HiROM's 256 KiB, and SA-1 BW-RAM tops out at 256 KiB. A header claiming more is describing
memory the console cannot address. **Clamping rather than rejecting is deliberate** — the field is
an allocation hint, every board already wraps its accesses to `sram_size`, and real dumps do carry
garbage there.

Pinned by `a_forged_ram_size_byte_is_clamped_not_shifted_unbounded` (all 256 byte values) with
`the_sram_clamp_leaves_every_real_cartridge_size_untouched` as its negative control, and verified by
re-injecting the bug and confirming *those* tests fail and no others.

**How it was reached matters as much as the fix.** The defect is invisible without a seeded corpus.
Header detection scores candidate offsets, so a random image essentially never scores above zero —
unseeded, `rom_header` plateaus around 29 edges and never reaches this code at all. Seeded from the
committed permissive ROM corpus, it surfaced in under twenty seconds.

---

## Headline 2 — CI supply-chain hardening

Three items. The first two carry over from the sibling project's `v2.2.2` audit; the third was
raised in review of the fuzzing PR.

### `persist-credentials: false` on 15 of 17 checkouts

`actions/checkout` writes the workflow `GITHUB_TOKEN` into `.git/config`, where **anything the job
then executes from the tree** — build scripts, proc macros, test binaries, `scripts/*.sh`, the
MkDocs build — can read it. On a pull request that tree is by definition unreviewed code, and
nearly every job here compiles or runs it.

Highest exposure was `web.yml`'s `build`, which holds `pages: write` + `id-token: write` while
running `trunk`, `cargo doc`, `pip install`, and `mkdocs build`.

**Audited per site rather than applied blanket, and that mattered.** `release-auto.yml`'s `prepare`
job *genuinely needs* the credential: it creates an annotated tag and runs `git push origin "$TAG"`,
which authenticates through exactly that token. The sibling's audit concluded no job needed it;
that conclusion does not transfer, and a blanket sweep would have broken every future release. It
is now the only checkout in the repository that keeps the token, with the reason recorded inline,
and its exposure is bounded by its trigger (`workflow_run` on a completed CI run of `main`, never a
pull request).

### The release-tag existence check now fails closed

It was:

```bash
git ls-remote --exit-code --tags origin "refs/tags/${tag}"
```

which collapses **three** outcomes into two — tag present, tag absent, and *lookup failed* — reading
any non-zero exit as "absent". A transient network blip, auth hiccup, or rate limit therefore sent
an **already-released version** down the `should_release=true` path, re-tagging a release the
ceremony treats as immutable.

It is now `gh api git/matching-refs/tags/<tag>`, chosen over `git/ref/tags/<tag>` because it answers
"absent" with HTTP 200 and an empty array — so a genuine miss can never be confused with an error,
and no error-body parsing is needed. That endpoint matches by **prefix**, so the exact ref is
compared in `jq`. Verified necessary, not theoretical: **`v1.2` prefix-matches 7 real tags while
exact-matching none**, and a naive `length > 0` would have reported it as already released. Every
failure path aborts the job under `set -euo pipefail`.

### All 34 third-party action references pinned to commit SHAs

Across all 8 workflows plus the composite `rust-setup` action. An upstream tag move could
previously change what CI executes without any repository review. Each pin carries its original tag
as a trailing comment so the intent stays readable. One stray `actions/upload-artifact@v4`
(introduced with the fuzz job) was aligned to the `@v7` the rest of the repo already used.

Verified: `actionlint` reports the **same three findings before and after** (a self-hosted runner
label and two pre-existing shellcheck notes) — no new lint surface; all nine files parse.

---

## Headline 3 — the golden vectors ran in no pull-request job

Only `lint`, `test-light`, and `accuracysnes` run on a PR. `test-light` never passes
`--features test-roms`, and the `accuracysnes` job scoped itself to `--test accuracysnes`. So the
**53 rendered-scene goldens and every framebuffer golden** were reached only by `full-test`, which
runs on `main`, on tags, and on the weekly cron.

A PR could shift PPU behaviour, move a golden, and hear nothing about it until merge-to-main.

That is the same structure behind this project's own coprocessor-golden staleness (goldens left
stale by post-bless PPU accuracy fixes): **a golden vector nothing executes only accumulates
drift.** Here it was not that nothing ran them ever, but that nothing ran them while it was still
cheap to fix.

The `accuracysnes` job now also runs the scene, undisbeliever, rainwarrior, coprocessor, and
save-state suites — roughly 5 minutes for the three with a committed corpus. The `*_oncart` suites
need gitignored dumps and self-skip for free; they are listed anyway so the intent is explicit.

**Found while looking for something else.** The search was for RustySNES's analogue of the sibling's
`expansion_level_tripwire`. That mechanism does *not* port: the only `commercial-roms`-gated suite
here (`commercial_screenshots`) is a screenshot generator that asserts nothing, so it holds no
golden that can go stale, and PPU accuracy is not parameterised by a small constant set the way
expansion-audio levels are. Looking for it surfaced a real and larger gap instead.

---

## Measured, and the answer was no: run-ahead stays opt-in

`docs/frontend.md` recorded the per-frame save-state allocation as *the* blocker on making
run-ahead default-on. `v1.25.0` removed that allocation, so the question was re-measured.

Absolute timings are from one development machine and will differ elsewhere. They are recorded
because the **ratios** are what decide this, and a reader can only check a ratio by re-running the
same two benches (`save_state_cost`, `headless_frame`) and comparing. The percentages are the
portable part; the microseconds are the working.

| | cost |
|---|---:|
| `save_state` | ~119 µs |
| `load_state` | ~285 µs |
| **save/load round trip** | **~0.40 ms** |
| one emulated frame (`headless_frame_steady_state`) | **6.39 ms** |
| NTSC frame budget | 16.64 ms |

The round trip is **2.4% of the budget** — it was never the dominant cost, and removing the
allocation did not move the decision. What run-ahead costs is the **extra frame of emulation**,
which is inherent: `frames = 1` needs **79%** of the NTSC budget on a fast development machine,
leaving ~3.5 ms for present, UI, and audio; `frames = 2` needs **118%** and cannot hold 60 fps at
all. Defaulting that on would miss deadlines on ordinary hardware to buy latency nobody asked for.

**One real bug fell out of the measurement.** `RunAheadConfig`'s `Default` is now hand-written so
the throttle is armed. A derived `Default` gave `throttle_ms: 0.0`, which *disables* the throttle —
leaving the safety net off for exactly the user who had just enabled run-ahead from Settings and had
no `throttle_ms` line in their `config.toml`. It now defaults to 14 ms (below the 16.64 ms NTSC
deadline with headroom, and conservative against PAL's 20 ms). An existing config that spells the
field out keeps its value; `#[serde(default)]` fills only absent fields, pinned by a negative-control
test.

---

## Compatibility and upgrade notes

- **Save states:** format version unchanged. States written by `v1.25.0` load unmodified.
- **Config:** `run_ahead.throttle_ms` now defaults to `14.0` when the key is **absent**. A config
  that already spells the field out is untouched. If you had deliberately relied on the absent-key
  behaviour to disable the throttle, set `throttle_ms = 0.0` explicitly.
- **Emulation behaviour:** unchanged. No chip model, timing constant, or golden vector moved.
- **Build:** `fuzz/` is a separate workspace and does not affect `cargo build --workspace`. Running
  the targets yourself needs a nightly toolchain and `cargo-fuzz`.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --features test-roms
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features
cargo +nightly fuzz build --manifest-path fuzz/Cargo.toml     # all 14 targets
```

- All 14 fuzz targets build; each run to a bounded `-max_total_time` locally with zero crashes
  beyond the `rom_header` finding fixed in this release.
- `actionlint` finding count unchanged (3 pre-existing) across all 9 workflow files.
- AccuracySNES battery and all framebuffer goldens unmoved.

## Included changes

| PR | Commit | Summary |
|---|---|---|
| #273 | `55bea3c` | `test(reviewer)`: guard the comment-selection filter that has broken twice |
| #274 | `55f3831` | `docs(readme)`: enumerate the 17 releases the Roadmap had collapsed |
| #275 | `0b18e60` | `test(fuzz)`: build the fuzzing infrastructure and fix the defect it found |
| #276 | `81baef5` | `ci(security)`: harden checkout credentials, pin actions, fail-closed tag check |
| #277 | `ff9379e` | `ci(test)`: gate the goldens on PRs; keep run-ahead opt-in on measurement |

Full per-entry detail: [`CHANGELOG.md` → `[1.26.0]`](../../CHANGELOG.md).
