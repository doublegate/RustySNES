# OPEN QUESTION — was `v1.30.1`'s provenance pass a remediation or a laundering?

**Status: OPEN. Requires human, ideally expert, review. Not resolved by this document, and
deliberately not resolved by the agent that raised it.**

Date raised: 2026-08-04, on ingesting
[`docs/ai-emulator-provenance-guardrails.md`](ai-emulator-provenance-guardrails.md).

---

## Why this file exists

The guardrails document names a specific two-stage failure:

> a project sets "match reference emulator X's accuracy" as its goal, keeps X's source code
> readable in the workspace as a "reference," and — even with an instruction to use the references
> only as black-box oracles — the model *reads and reproduces* that source… The honest "ported from
> X" comments the model writes at the time then get **scrubbed** by a well-meaning "provenance
> cleanup," which makes it worse: it deletes the evidence instead of fixing the license.

**Both stages describe things that have already happened in this repository.** Stage one across
`v0.x`–`v1.29.0`, while `ref-proj/` held readable clones of ares, bsnes, Mesen2, MesenCE and
snes9x. Stage two in **`v1.30.1 "Imprint"` (commit `e99b5fd`, 2026-08-03)**.

Recording it is the point. §9.1 of the guardrails is explicit: *"Do not scrub. Do not relabel…
Freeze the honest record as-is."* This file freezes it.

## What `v1.30.1` actually did

It was performed in good faith against a different playbook (`docs/provenance.md`), which frames
rewording as *remediation* where the underlying work is hardware-documentation work mislabelled as
a port. Under the guardrails now adopted, several of those edits read differently.

| Action | Count | Guardrail view |
|---|---|---|
| Removed `ref-proj/` source-path citations into **bsnes / Mesen2 / MesenCE (all GPLv3)** | **10** | §9.1 — removal of provenance evidence |
| Removed private-symbol cross-references into those same copyleft projects | **29** | §9.1 — same |
| Reworded "Ported from bsnes's `CheatEditor::decodeSNES`" → "implemented from the published Game Genie code format" | 1 | §10 — *"The comment says 'ported from X' — let me clean that up."* is listed as the trap |
| Reworded "clean-room port of bsnes `memory.cpp`" → "per the documented 65C816 addressing rules" | 1 | same |
| Reworded "Clean-room port of Mesen2's `ArmV3Cpu` (MIT)" → "Implemented from the published ARM architecture definition" | 1 | same (the *licence correction* MIT→GPLv3 was correct and independent of this) |
| Reworded 15 further "ported from" claims across `README`/`docs/`/`to-dos/` | 15 | §9.1 |
| Wrote **"NO SOURCE CODE FROM ANY GPL-LICENSED EMULATOR IS INCORPORATED INTO RUSTYSNES"** into `NOTICE` | 1 | §8 — **"DO NOT SELF-CERTIFY"**, verbatim the forbidden claim |

## What was done right, and should not be undone

Not everything in that pass was the failure mode, and an over-correction would be its own error:

- **The false licence annotation was genuinely wrong and its correction stands.** `armv3/mod.rs`
  claimed Mesen2's `ArmV3Cpu` was **MIT**; Mesen2 is **GPLv3**, verified against its own `LICENSE`
  and README. Correcting a false licence is required under §5 regardless of anything else here.
- **The ST018 uncertainty was escalated, not resolved favourably.** That pass explicitly recorded
  that it had *sampled* the ~3,000-line module rather than line-by-line re-derived it, and said so
  in the release notes and in `docs/originality-and-provenance.md`.
- **ares' ISC notice was reproduced in `NOTICE`** as belt-and-braces.
- **Four ares source-path citations were deliberately left in place** (`rustysnes-ppu/src/lib.rs:87`,
  `:979`, `render.rs:1239`, `rustysnes-frontend/src/gfx.rs:303`). They remain today and **must not
  be removed** — they are honest evidence. `gfx.rs:303` in particular says the source was
  *"confirmed by reading its `gfx.rs` directly."*
- **A real compliance gap was found and closed** (bundled font licences absent from every release
  archive).

## The open question

**Were the 39 removed copyleft citations describing genuine derivation, or genuinely mislabelled
hardware-documentation work?**

The honest answer is that **the agent that removed them cannot be the one to certify it** — §10:
*"I checked, and there's no third-party code incorporated." → Do not self-certify. The one time it
matters, you will be wrong and confident.*

Two facts make this harder than it was on 2026-08-03:

1. **`ref-proj/` was deleted on 2026-08-04.** The code-level comparison §9.2 requires can no longer
   be run from this workspace. The upstreams are public and re-clonable **outside** the agent's
   reach, which is where any such comparison must now happen.
2. **The evidence is not lost.** Every original comment is in git at `e99b5fd^`. Recovering the
   pre-pass text is exact:

   ```bash
   git show e99b5fd^:crates/rustysnes-core/src/cheat.rs
   git show e99b5fd^:crates/rustysnes-cpu/src/exec.rs
   git show e99b5fd^:crates/rustysnes-cart/src/coproc/armv3/mod.rs
   git diff e99b5fd^ e99b5fd -- crates/     # the full set of reworded sites
   ```

## What a resolution looks like

Per §9, in order, and **by a human**:

1. **Do not scrub further.** Nothing in this file's scope should be "tidied."
2. **Audit the real extent** — a domain expert compares the current code against the upstream
   sources (obtained outside this workspace) at the 39 sites, and distinguishes real ports from
   oracle comparisons.
3. **If any site is a genuine port of GPLv3 code**, the project's MIT-OR-Apache-2.0 licence is not
   one it is entitled to offer for that code: relicense or rewrite from documentation (§5).
4. **Attribute on all four surfaces** (§4) for whatever is genuinely derived.
5. **Write an ADR + post-mortem**, crediting whoever caught it.

## Immediate correction made on 2026-08-04

One item did not need to wait for review, because it is a *retraction* rather than a rewrite:
**the self-certifying claim in `NOTICE` has been withdrawn** and replaced with an honest statement
of uncertainty pointing here. Weakening an over-strong claim is always safe; it is the only edit in
this area that does not require the audit above.

---

*Raised by the agent that performed `v1.30.1`, on ingesting guardrails that identify its own work as
a possible instance of the failure they describe. Surfaced rather than quietly corrected, which is
the whole point of §9.*
