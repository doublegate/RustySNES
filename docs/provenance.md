# Provenance, Licensing & Attribution — Review & Remediation Guide

> ## ⚠ SUPERSEDED IN PART — read `ai-emulator-provenance-guardrails.md` first
>
> **This playbook's Phase 2 "classify, then remediate source comments" step was applied to this
> repository in `v1.30.1`, and under
> [`ai-emulator-provenance-guardrails.md`](ai-emulator-provenance-guardrails.md) — now the
> project's highest-priority rule — several of those edits read as the *laundering* failure rather
> than remediation.**
>
> Where the two documents disagree, **the guardrails win**. Specifically:
>
> - This guide says a category-1 comment mislabelled as a "port of X" should be *reworded*. The
>   guardrails say **never reword or delete an honest "ported from X" comment**, because doing so
>   destroys the evidence needed to judge the licence question — and that the agent doing the
>   rewording is exactly the party that cannot certify the classification (§10, "Do not
>   self-certify").
> - This guide's §0 "prefer the truthful, checkable claim" is right, but a *rewording* by the same
>   automated pass is not checkable; a preserved original plus an expert audit is.
>
> **Safe to apply from this guide:** correcting a factually wrong licence annotation; adding
> attribution; writing `NOTICE`/`LICENSES.md`/originality docs; the count-accuracy and tooling
> appendices. **Do not apply:** Phase 2 rewording of any comment that names a copyleft project,
> without a human expert first establishing whether the site is genuinely derived.
>
> The open question this raised for RustySNES is recorded in
> [`provenance-open-question.md`](provenance-open-question.md).
>
> **What this is.** A reusable playbook for auditing and correcting how a
> `Rusty*` emulator project describes its own provenance — in source comments,
> `NOTICE`, `README.md`, `tests/roms/LICENSES.md`, and the docs — and for
> producing the project's own `docs/originality-and-provenance.md`. It is
> **not** customized to any one console; it tells a Claude Code instance what to
> inspect, how to classify what it finds, how to remediate it honestly, and how
> to verify the result. It was distilled from the RustyNES v2.2.5 "Colophon"
> provenance pass (prompted by public community review) and is meant to bring the
> sibling projects (Rusty2600 / RustySNES / RustyN64, and any other `Rusty*`) to
> the same standard.
>
> **Read this whole guide before editing anything.** Do the audit first; classify
> before you reword; and never "launder" provenance (see [Honesty rules](#0-honesty-rules-non-negotiable)).

---

## 0. Honesty rules (non-negotiable)

These override convenience every time. The entire point of this exercise is that
the written record must match what the code actually is.

- **Do not reword a real copy into an "independent implementation."** If a file
  is genuinely a line-for-line translation of copyleft (GPL/LGPL) source, changing
  the comment to say "independent reimplementation" is falsification, not
  remediation. Either re-implement it cleanly from public documentation, remove
  it, or stop and ask the maintainer. Rewording is only correct when it makes the
  comment *accurately* describe what already happened.
- **Copyright protects expression, not facts or techniques.** Hardware behavior
  (a CPU opcode's effect, a mapper/board register map, a documented signal model)
  is factual — every accurate emulator converges on it, and implementing it from
  public documentation is not infringement. A *visual look* or a *rendering
  technique* is likewise not protectable; the specific *shader/source code* that
  produces it is. Keep this distinction sharp.
- **Prefer the truthful, checkable claim over the flattering one.** "Behavior
  cross-checked against emulator X as an oracle; no code incorporated" is both
  honest and defensible. "Ported from X" (when X is copyleft) is a licensing
  problem even if the code is actually your own.
- **Get counts and facts right.** Use `git ls-files`, not on-disk `find`, for
  "committed" counts (see [Appendix G](#g-count-accuracy-git-ls-files-not-find)).
  Never ship an estimate as a fact.
- **Disclose AI assistance.** If the project is AI-assisted, say so plainly in the
  README and the originality doc. Non-disclosure is what triggers the loss of
  trust; disclosure defuses it.
- **When a finding is a genuine legal judgment call (a real copyleft translation,
  an ambiguous shader port), surface it to the maintainer instead of deciding
  unilaterally.**

---

## 1. The mental model: three categories

Every reference the project touched falls into exactly one of these. Your job is
to make the source, `NOTICE`, and docs say which — accurately.

1. **Implemented from public hardware documentation** (the vast majority).
   Chip/opcode/mapper/board/signal behavior written from console hardware
   references and pinned to public test ROMs. Not a derivative of any emulator.
2. **Incorporated from a permissively-licensed project, with attribution.** A
   real port or vendored source under a compatible license (MIT / BSD / ISC /
   Zlib / Apache-2.0). Must be attributed in-source and in `NOTICE`.
3. **Consulted only as a behavioral oracle.** A reference emulator (often
   copyleft) run to observe/cross-check documented behavior. **No code
   incorporated.** This is legitimate and must be stated as such — not as a
   "port."

The failure mode this guide fixes: category-1 work whose in-source comments were
written as if it were category-2/3 "ports of" a copyleft emulator.

---

## 2. Phase 0 — identify this project's reference set

Reference emulators and their licenses differ per console. Before auditing,
enumerate them for THIS project and record each license (read the actual
`LICENSE`/`COPYING` file — do not assume):

- Look under `ref-proj/` (usually gitignored) and `ref-docs/` for what was
  actually consulted.
- Determine each reference's license class: **copyleft** (GPLv2/GPLv3/LGPL) vs
  **permissive** (MIT/BSD/ISC/Zlib/Apache). Copyleft references may only be
  oracles; permissive ones may be incorporated with attribution.
- Typical per-console references to check for (adapt to the actual project):
  - **NES:** Mesen2/MesenCE (GPLv3), higan (GPLv3), GeraNES (GPLv3), ares (ISC),
    FCEUX/Nestopia/puNES (GPLv2), TriCNES (MIT), emu2413 (MIT).
  - **SNES:** bsnes/higan (GPLv3), Mesen-S (GPLv3), Snes9x (custom
    non-commercial — treat as **do-not-incorporate**), ares (ISC), byuu docs.
  - **N64:** mupen64plus (GPLv2), ares (ISC), Project64 (GPLv2), Angrylion RDP
    (GPLv2/public research), CEN64 (BSD).
  - **Atari 2600:** Stella (GPLv2), z26 (GPLv2), 6502 references.
  - Confirm the real set from the repo; the above is a starting checklist, not
    ground truth.

Record this set — you will reproduce it in `NOTICE` and the originality doc.

---

## 3. Phase 1 — audit (find every provenance claim)

Run these from the repo root. Note the shell may be zsh
([Appendix D](#d-shell-and-tooling-traps)); prefer `grep`/`git grep` with quoted
globs.

### 3.1 Source-comment provenance claims

```bash
# Every "port / verbatim / adapted / derived / based on / translated" tied to a
# reference emulator, across Rust AND shader/other source.
git grep -niE "(ported|port of|verbatim|condensation of|faithful port|adapted from|translated from|derived from|based on)[^.]{0,60}(mesen|higan|ares|geranes|fceux|nestopia|punes|bsnes|snes9x|mupen|project64|angrylion|cen64|stella|z26|tricnes|blip_buf|crt-royale|guest|megatron|bisqwit|troggle)" -- '*.rs' '*.wgsl' '*.glsl' '*.slang'
```

Classify each hit as category 1/2/3 (Section 1). Also grep each reference name
alone (`git grep -ni mesen -- '*.rs'`) to catch differently-worded claims, and
distinguish real problems from harmless **format interop** (reading another
emulator's movie/symbol/HD-pack file format is not copying code) and harmless
**oracle wording** ("emulator X does not pass this test").

### 3.2 High-risk vectors that a naive grep misses

- **CRT / video shaders** (`*.wgsl`, `*.glsl`, `*.slang`, a `*-gfx-shaders`
  crate). Many community shaders are **GPL** (e.g. CRT-Royale is GPLv2+). A
  "single-pass condensation of X's preset" is derivative-work language — see
  [Phase 4](#6-phase-4--creative-expression-shaders-ntsc-filters-palettes).
- **NTSC / composite filters and palette generators.** Check the license of the
  algorithm's origin (e.g. Bisqwit's example code has been published under
  CC BY-SA; EMMIR/LMP88959 NTSC-CRT is permissive-with-credit). Numeric signal
  tables are usually factual (documented on the console's dev wiki) even if a
  comment says "ported verbatim."
- **Audio band-limiting.** A `blip.rs`/BLEP decimator often cites Shay Green's
  `blip_buf` — which is **LGPL-2.1+**, *not* BSD/MIT. Fix the license annotation;
  confirm it is an independent implementation, not copied.
- **Fonts / icons.** Every bundled font needs its license text to travel with it
  in the distribution (OFL 1.1 requires this on **every** platform copy —
  desktop, iOS, Android). Check for unattributed custom fonts.
- **Test ROMs.** Must be public-domain/permissive and catalogued (see
  [Phase 6](#8-phase-6--readme-licensesmd-fonts)). Never commit commercial ROMs.
- **Vendored source trees** (a `vendor/` dir, a `golden/` oracle tree): each must
  carry its upstream `LICENSE` and be attributed in `NOTICE`.
- **Machine-absolute path leaks** in comments (`/home/<user>/...`):
  `git grep -n "/home/" -- '*.rs' '*.wgsl'` — retarget to the upstream URL.
- **Cargo dependency licenses.** Confirm `deny.toml` (if present) allows only
  permissive licenses and admits no GPL/LGPL/AGPL crate. If there is no
  `deny.toml`, spot-check `cargo tree` for copyleft crates.
- **Generated files.** If a committed file (e.g. a `.wgsl`) is generated from a
  Rust `shader_src()` and guarded by a drift test, editing the committed file
  alone breaks the build — you must edit the **generator** too
  ([Appendix A](#a-generated-file-drift)).

---

## 4. Phase 2 — classify, then remediate source comments

For each flagged comment, decide the category and reword to the truth:

- **Category 1 (hardware behavior):** rewrite to cite the public hardware source
  (the console's dev wiki page / board notes / datasheet / documented CPU
  behavior) and, if a reference emulator was consulted, add "cross-checked against
  `<X>` as a behavioral oracle; no third-party emulator code is incorporated."
  Remove copyleft source-file paths and line-number citations, and drop
  copyleft private-symbol/field-name cross-references.
  - Example transform:
    - Before: `// Ported from Mesen2 Foo/Bar.h.`
    - After: `// Register map per the <console> dev-wiki <board> documentation`
      `// (cross-checked against reference emulators as accuracy oracles; no`
      `// third-party emulator code is incorporated).`
- **Category 2 (permissive port):** keep the attribution, make it precise (name,
  license, commit/version), and ensure it is reproduced in `NOTICE`.
- **Category 3 (oracle):** state it is an oracle/cross-check only.

Do **not** change behavior — these are comment edits. Prove byte-identity in
Phase 8.

---

## 5. Phase 3 — rewrite `NOTICE`

Produce a `NOTICE` with these sections (adapt names to the project):

1. **Project copyright + license boilerplate.**
2. **Hardware documentation** — the dev wiki / die studies referenced; "no code
   incorporated; documentation referenced for hardware behavior specification."
3. **Reference emulators (behavioral oracles only — no code incorporated)** —
   list each with its license (e.g. "Mesen2 (GPLv3)"). State plainly that **no
   source code from any GPL-licensed emulator is incorporated.**
4. **Incorporated third-party components (permissively licensed)** — each real
   port/vendored work with copyright holder, license, version/commit, and where
   it lives; reproduce the MIT (or applicable) permission notice text.
5. **Bundled fonts** — each font, author, license, and where its license text
   ships.
6. **Visual influences (independently reimplemented — no code incorporated)** —
   the shaders/filters whose *look* is reproduced, each with author + upstream
   license, explicitly framed as independent reimplementations (only if that is
   true — see Phase 4).
7. **Bundled test ROMs** — pointer to `tests/roms/LICENSES.md`; name the ones
   whose licenses require notice preservation (MIT/zlib).

Verify completeness: every in-tree `LICENSE`/`COPYING` file under `crates/*/vendor/`,
`golden/`, `assets/fonts/`, and `tests/roms/` must be reflected. Every in-source
"reproduced in NOTICE" claim must actually be satisfied.

---

## 6. Phase 4 — creative expression (shaders, NTSC filters, palettes)

Shaders and filters are **creative expression**, so the merger/hardware-facts
reasoning does **not** apply. For each shader/filter that names an upstream:

1. **Determine how it was actually written** — from the *rendered look /
   published algorithm* (defensible as an independent reimplementation), or from
   the *upstream source* (a derivative work; if the upstream is copyleft, it
   cannot ship under a permissive license). A single-pass shader that is
   structurally incompatible with a multi-pass upstream (different pass count,
   different uniform layout, a fraction of the size) is strong evidence of
   independent reimplementation of the *technique* — but confirm, do not assume.
2. **If independent:** reword "port / condensation of X" → "independent
   single-pass reimplementation of the perceptual model of X (author, upstream
   license)"; add per-file attribution; add a "Visual influences" `NOTICE` entry.
3. **If a genuine translation of copyleft source, or you cannot tell:** stop and
   ask the maintainer. Options are re-implement from the description, remove/
   default-disable the feature, or (maintainer decision) relicense. Do **not**
   reword it clean.
4. **Numeric signal/palette tables** that a comment calls "ported verbatim" are
   usually the documented factual signal model (check the console dev wiki). Cite
   the documented model and demote any copyleft-emulator mention to an oracle.

---

## 7. Phase 5 — create `docs/originality-and-provenance.md`

Author the project's own honest account. Structure (keep it honest, not
triumphal):

1. **Thesis** — an independent build with attributed borrowings: the
   *architecture and engineering method* are original; specific algorithms are
   transparently ported from permissive sources; copyleft emulators were oracles
   only. Include an explicit **AI-assistance disclosure** and a **"not a
   superiority claim"** note (comparisons are comparisons, not claims of being
   "better").
2. **Where the project advances / diverges / independently re-derives technique**
   — the genuinely original parts (scheduler/timing model, determinism contract,
   machine-checked accuracy-honesty gates, measured-and-rejected optimizations,
   etc.), with mechanisms and any measured numbers, and honest named comparisons.
3. **How it was built** — research-first, test-as-spec, verify-last; the timeline
   grounded in checkable history (do not overstate — see
   [Appendix H](#h-timeline-claims)).
4. **Independence: oracle vs. port** — the three-category classification applied
   to this project's actual references, including the sharpest honest example.
5. **License compliance** — the project's own license; a reference-emulator
   oracle table (with licenses); the incorporated permissive components; test
   ROMs; vendored-tree integrity protections; and any creative-expression
   (shader/filter) posture.
6. **Conclusion.**

Cross-link `NOTICE`, `docs/STATUS.md` (or equivalent), the CHANGELOG, and the
dev-wiki references.

---

## 8. Phase 6 — README, `LICENSES.md`, fonts

- **README:** add an **AI-assistance disclosure** near the top; **remove any
  comparison chart/graphic with fabricated or AI-hallucinated details**; **fix
  mislabeled screenshots** (e.g. an early-development image captioned as a
  precise-accuracy demonstration); tone down overstated language; ensure the
  **Acknowledgments** section matches `NOTICE`; bump any version badge/citation.
  Any "comparison" must name the reference and disclaim that it is not a
  superiority claim.
- **`tests/roms/LICENSES.md`:** counts must come from `git ls-files`
  ([Appendix G](#g-count-accuracy-git-ls-files-not-find)); fix stale narratives
  (e.g. a test described as an "unmeasured smoke gate" when it now decodes and
  asserts a measured pass rate); remove false exclusion claims; add blanket
  coverage for committed directories not individually tabulated; **no commercial
  ROMs.** Fix stale crate paths in `tests/roms/README.md` too.
- **Fonts:** ensure each bundled font's license text ships alongside it on
  **every** platform (a font present in the Android/iOS asset set without its OFL
  text is a compliance gap even if desktop has it).

---

## 9. Phase 7 — verify (do not skip; run what CI runs)

Comment/doc changes are "byte-identical" only if you prove it. Run the project's
real gates, not a subset:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings` **and every
  feature-combo the project gates** (a `--fix` or a doc-comment edit can trip
  `clippy::doc_markdown` on names like a dev-wiki proper noun, or
  `clippy::too_long_first_doc_paragraph` — backtick code-like tokens, split long
  first paragraphs; see [Appendix C](#c-clippy-doc-lint-traps)).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- the `no_std` cross-compile job if the core is `#![no_std]`
- **the FULL test suite the project's CI runs** — e.g.
  `cargo test --workspace --features <test-roms-equivalent>` — **not** just the
  targeted accuracy oracles. A generated-file **drift test** or a frontend
  `--lib` unit test lives outside the accuracy oracles and will fail on CI if you
  only ran the oracles locally ([Appendix A](#a-generated-file-drift)).
- markdownlint via the project's pinned pre-commit hook on **only the changed
  files** — `pre-commit run markdownlint --files <paths>`. **Never**
  `pre-commit run --all-files` ([Appendix D](#d-shell-and-tooling-traps)).
- the project's accuracy oracle(s) to confirm the core is byte-identical.

Every change in a provenance pass should be comment/doc/attribution only ⇒
accuracy results unchanged. If an accuracy number moves, you changed behavior —
stop and investigate.

---

## 10. Phase 8 — release ceremony (only if cutting a release for this)

If the maintainer wants this shipped as its own release, follow the project's
release process. Key traps ([Appendix](#appendix--specific-lessons--gotchas)):

- Version bump (usually single-sourced in the workspace `Cargo.toml`), regenerate
  the lockfile, update any distribution metadata (e.g. a libretro `.info`), and
  sync STATUS / ROADMAP / CHANGELOG / release notes. Use precise wording — if the
  release changes core-crate *comments*, say "zero emulation-core **behavior**
  changes," not "zero emulation-core changes."
- **The PR-vs-main CI matrix trap:** PRs often run a reduced (Linux-only) matrix;
  the merge to `main` runs the full Windows/macOS matrix, and release automation
  keys off `main` going green. A green PR does **not** prove the release cuts —
  watch main CI after merge.
- **Bot-comment ceremony:** reply to and **Resolve** every automated-reviewer
  thread before merging; adopt valid findings, skip-with-reason the rest; iterate
  green → adopt → push → re-read. Adjudicate any *new* comment on the final
  commit before merging.
- **Never** put a GitHub closing keyword (`Closes`/`Fixes`/`Resolves` + `#`) in a
  commit/PR body for an issue the change doesn't fully finish — GitHub closes it
  on merge regardless of surrounding text, even inside code spans.

---

## Appendix — specific lessons & gotchas

### A. Generated-file drift

If a committed artifact (e.g. `src/<name>.wgsl`) is generated from a Rust
generator (`shader_src()` / `include_str!` pair) and a unit test asserts they
stay byte-identical, editing a **comment in the committed file** breaks that test
unless you make the **same edit in the generator string**. Fix both; the drift
test lives in the frontend/lib unit tests, not the accuracy oracles, so a
targeted-oracle-only local run will miss it.

### B. Don't launder; classify honestly

Rewording "port of GPL X" → "independent reimplementation" is only valid if it is
*true*. For hardware behavior it is (facts aren't copyrightable); for a shader it
requires the code to actually be an independent reimplementation. When unsure,
ask — see [Honesty rules](#0-honesty-rules-non-negotiable).

### C. Clippy doc-lint traps

- `clippy::doc_markdown`: backtick code-like tokens in `///`/`//!` doc comments
  (proper nouns with internal capitalization, crate/board names). `//` line
  comments are exempt. Some crates set `#![allow(clippy::doc_markdown)]` — check.
- `clippy::too_long_first_doc_paragraph`: keep the first doc-comment paragraph
  short; put detail after a blank `///` line.
- After any `clippy --fix`, re-run clippy for **every** feature combo — `--fix`
  compiles only the active feature set and can strip `cfg`-gated code another
  feature needs.

### D. Shell and tooling traps

- The harness shell may be **zsh**: bash-isms like `${!arr[@]}` fail. Use a
  portable `while IFS='|' read -r ...` loop, or `bash -c`.
- **`sed -i` is blocked** by the guard (it has zeroed files). Use the Edit tool.
- **`pre-commit run --all-files` rewrites vendored/immutable trees**
  (trailing-whitespace / end-of-file-fixer). Always scope with
  `--files <changed>` or run a single named hook.
- `ls` may be aliased (adds sizes), breaking `grep '\.nes$'`-style filters — use
  globs or `git ls-files`.
- `gh api graphql`: pass a reply body with `-f body="..."` (or `-F body=@file`),
  **never** `-F body=-` (posts a literal `-`).

### E. Markdown MD004 (`+`-at-line-start)

A prose line that *wraps* so it begins with a plus sign followed by a space (e.g. "…PASS/FAIL" then "+ hex
codes") is parsed as a `+`-style list item and flips MD004's inferred bullet
style, failing every `-` bullet in the file. Reword so no line begins with `+`.

### F. `gh pr merge` local fast-forward glitch

`gh pr merge --squash --delete-branch` can print "not possible to fast-forward,
aborting" from its **local** post-merge step (updating local `main` / deleting
the branch) *after the server-side merge already succeeded*. Verify with
`gh pr view <n> --json state,mergedAt` — if `MERGED`, the merge is done; the
error was only local housekeeping (it may leave your local checkout on a stale
`main`).

### G. Count accuracy: `git ls-files`, not `find`

"Committed" counts must use `git ls-files 'tests/roms/**/*.nes' | wc -l`. On-disk
`find` includes untracked clone contents and gitignored dirs (e.g. an
`external/` ROM dir) and will overcount. Recompute every documented count from
`git ls-files` before writing it down.

### H. Timeline claims

Only assert a development history the public record supports. Distinguish "N years
of involvement with emulation" (defensible from an account's repo trail) from
"N years building this exact codebase" (often much shorter). If the public
timestamps make the project recent, say so and lean on the honest, checkable
framing rather than an unverifiable duration.

---

## Deliverables checklist (per project)

- [ ] Phase 0 reference set enumerated (with licenses).
- [ ] Phase 1 audit run; every hit classified.
- [ ] Source comments reworded to the accurate category (Phase 2).
- [ ] `NOTICE` rewritten with all seven sections (Phase 3).
- [ ] Shaders/filters resolved honestly (Phase 4) — or escalated to the maintainer.
- [ ] `docs/originality-and-provenance.md` authored (Phase 5).
- [ ] README + `tests/roms/LICENSES.md` + fonts corrected (Phase 6).
- [ ] All gates green, incl. the FULL test suite and any drift tests (Phase 7).
- [ ] Accuracy oracle(s) prove the core is byte-identical.
- [ ] (Optional) release ceremony followed (Phase 8).
