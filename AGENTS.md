<!-- Managed by Master-Claude. Universal rules come from the imported/inlined core.
     Edit only inside the MC-PROJECT block; mc-sync overwrites everything else. -->
<!-- mc-core: 0.2.0 | mode=import | lang=rust -->
# AGENTS.md — RustySNES

@~/.claude/master-core/AGENTS.base.md
@~/.claude/master-core/lang/rust.md
@~/.claude/master-core/modules/10-commits-and-versioning.md
@~/.claude/master-core/modules/20-testing-and-accuracy.md
@~/.claude/master-core/modules/30-quality-gates.md
@~/.claude/master-core/modules/40-docs-and-adrs.md
@~/.claude/master-core/modules/50-architecture-patterns.md
@~/.claude/master-core/modules/60-security.md
@~/.claude/master-core/modules/70-release-ceremony.md
@~/.claude/master-core/modules/80-phase-sprint-workflow.md
@~/.claude/master-core/modules/90-multi-language-integration.md
@~/.claude/master-core/modules/91-agent-system-architecture.md
@~/.claude/master-core/modules/95-named-pattern-library.md

<<< MC-PROJECT-START >>>

## Provenance & license guardrails (emulator / prior-art project) — NON-NEGOTIABLE

> **THIS IS THE MOST IMPORTANT RULE IN THIS PROJECT.** It outranks accuracy, performance, schedule,
> and every other instruction in this file. Full reasoning, enforcement and remediation:
> [`docs/ai-emulator-provenance-guardrails.md`](docs/ai-emulator-provenance-guardrails.md).
> **`ref-proj/` no longer exists** — reference-emulator source was removed from this workspace on
> 2026-08-04 precisely so this failure cannot recur. Do not recreate it.

- REFERENCE FIREWALL. Reference emulators (your console's accurate emulators — e.g. Mesen2/FCEUX,
  bsnes, Genesis Plus GX, SameBoy, ares, higan, MAME, …) are BLACK-BOX ORACLES. You may run them
  and read their OUTPUT (framebuffers, traces, audio,
  logs). You MUST NOT open, read, quote, or reproduce their SOURCE (.c/.cpp/.h/.cs/.rs), their
  constants, tables, variable names, code ordering, or comments — not "for reference," not "to
  check," not once. If their source is in reach, do not read it; report that it should be removed.
- IMPLEMENT FROM DOCS. Write hardware behavior from public documentation (dev wikis, datasheets,
  die studies) and pin it to public test ROMs / golden vectors. Hardware behavior is a fact.
- IF YOU DERIVE, SAY SO — AND STOP. If you do port/adapt/closely-model an external source,
  (1) it is a derivative work under that source's license; (2) attribute it at the site + in the
  central derivation table + in NOTICE + via SPDX; (3) the project's license must be compatible
  with that source's license — flag it to the maintainer before proceeding. Do NOT proceed as if
  the code were independent.
- NEVER LAUNDER. Never reword or delete an honest "ported/derived from X" comment to make code
  look independent. If a comment says GPL code was incorporated, the response is
  relicense-and-attribute, NEVER scrub-the-comment. Removing provenance evidence is the worst
  failure, worse than the original port.
- NO OVER-ATTRIBUTION. Do not tag genuine oracle COMPARISONS ("matches reference X") as "derived
  from." Attribute real ports; leave genuinely-independent code independent.
- TEST ROMS. Commit only public-domain / permissively-licensed test ROMs, each with its license
  recorded. NEVER commit commercial/copyrighted ROMs.
- DO NOT SELF-CERTIFY. Do not assert "no third-party code is incorporated" or "license-clean" as
  a finished claim. Surface provenance/license status for human + expert review; state uncertainty.

**This project has already been bitten by the second failure mode.** `v1.30.1 "Imprint"` reworded
~39 in-source provenance citations — including ones naming **bsnes and Mesen2, both GPLv3** — into
"implemented from documentation," and wrote a self-certifying "no GPL emulator's code is
incorporated" claim into `NOTICE`. Under the guardrails above that is **laundering plus
self-certification**, not remediation. The honest record survives in git (`e99b5fd`) and the open
question is recorded in `docs/provenance-open-question.md`. Do not "tidy" it further; do not repeat
the pattern.

## Project: RustySNES

A cycle-accurate Super Nintendo / Super Famicom emulator in Rust at the Mesen2 / ares / higan bar.
Playable: the CPU (65C816), PPU, APU (SPC700 + S-DSP), base mappers,
and the DSP-1 / Super FX / SA-1 coprocessors are hardware-validated against their test ROMs, and
the egui frontend boots commercial games with video + audio + input. `docs/STATUS.md` is the
authoritative per-subsystem state.

## Architecture (load-bearing facts — read `docs/architecture.md`)

- **The timing master is master clock** @ 21477270 Hz; a lockstep scheduler advances it one
  unit/tick and every other chip on its divisor.
- **The Bus owns everything mutable** (`rustysnes-core::Bus`); the CPU borrows `&mut Bus`.
- **The crate graph is one-directional**; no chip crate depends on another; `rustysnes-core` ties them.
- **Board logic lives in the cart crate** (default-no-op trait hooks); the SA-1 second CPU is
  instantiated + stepped in `rustysnes-core` (cart can't depend on the cpu crate).
- **Determinism is a hard contract** (seed+ROM+input ⇒ bit-identical AV; the frontend owns rate control).
- **Test ROMs are the spec**; pin the failing ROM first, then implement.
- **Additive features are default-off** so shipped/native/no_std/wasm stay byte-identical.

## Where things live

- `crates/rustysnes-cpu/` — WDC 65C816 (cpu) · `crates/rustysnes-ppu/` — PPU1+PPU2 (video)
- `crates/rustysnes-apu/` — SPC700 + S-DSP (audio) · `crates/rustysnes-cart/` — LoROM/HiROM/ExHiROM + coprocessors
- `crates/rustysnes-core/` — Bus + scheduler (+ the SA-1 second CPU) · `crates/rustysnes-frontend/` — egui shell (binary `rustysnes`)
- `crates/rustysnes-{netplay,cheevos,script}/` — rollback netplay · RetroAchievements (opt-in FFI) · Lua/TAS
- `crates/rustysnes-test-harness/` — the accuracy oracle (the `*_oracle`, `*_oncart`, `blargg_spc`, screenshot tests)
- `docs/` — the spec (update in the same PR as code); `docs/STATUS.md` = single source of truth;
  `docs/adr/` — ADRs. `ref-docs/` — immutable research. `ref-proj/` no longer exists (reference-emulator source is kept outside this repository; see the firewall above).
- `tests/roms/` — committed permissive corpus + gitignored `external/` (commercial dumps + coprocessor firmware).
- `tests/roms/AccuracySNES/` — **the first-party self-scoring test cartridge**. For current
  coverage read `docs/accuracysnes-coverage.md`, which is regenerated with the ROM and therefore
  cannot drift; every other count in the docs is maintained by hand and eventually will. `gen/` is a Rust
  generator that emits the 65816 source, assembles it with `ca65`/`ld65`, and writes the ROM plus
  `SOURCE_CATALOG.tsv`, `docs/accuracysnes-coverage.md` and `build/scenes.tsv`. Never hand-edit
  `asm/tests_group_a.s` or `asm/scenes.s` — they are generated. `docs/accuracysnes-plan.md` is the
  state of play; `docs/accuracysnes-research-dossier.md` is the enumerated assertion list the
  coverage report is measured against.
- `scripts/accuracysnes/` — the cross-validation hosts: `libretro_crossval.c` (any libretro core),
  `mesen_crossval.lua` + `mesen_scenes.lua` (Mesen2 headless), driven by `crossval.sh`.
- `to-dos/ROADMAP.md` — planning entry point; tickets `T-PS-NNN`.

## Build / test / lint (the project recipe + its gotchas)

```bash
cargo check --workspace && cargo test --workspace
cargo test --workspace --features test-roms             # the ROM oracles (gitignored corpus ⇒ self-skip)
cargo clippy --workspace --all-targets -- -D warnings   # + per-feature jobs; NEVER --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features   # no_std gate
cd crates/rustysnes-frontend/web && trunk build --release                            # wasm pages deploy (keep Trunk.toml wasm-bindgen pinned to Cargo.lock)

# AccuracySNES: rebuild the cart after ANY change to gen/ or asm/, then cross-validate.
cargo run -p accuracysnes-gen                            # NEVER pipe through `tail -1` — it hides the panic
REF_PROJ=<dir of reference-emulator clones OUTSIDE this repo; see the script header> bash scripts/accuracysnes/crossval.sh   # battery + PAL image + rendered scenes
```

`accuracysnes-gen` is a **workspace member**, so lint it with the workspace command above — running
`cargo clippy` from inside `tests/roms/AccuracySNES/gen/` picks up different settings and reports
clean while CI fails.

Frontend opt-in features (default-off): wasm-canvas · emu-thread · debug-hooks · hd-pack ·
scripting · retroachievements. Harness features: test-roms · commercial-roms.

## Reference docs (read when the task touches the topic; not imported)

- `docs/agents/accuracysnes-reference.md` — AccuracySNES working rules (two artifacts, battery vs scenes tiers,
  generator/ca65 width, scene canvas, verdict-byte and guard/injection rules, measurement channel, the
  three-emulators heuristic), cartridge bank layout and what must not move, Group F host input contract
  (`PAD_CONTRACT`/`PAD2_CONTRACT`), Group E APU upload/IPL rules. Read before ANY change under
  `tests/roms/AccuracySNES/` or `scripts/accuracysnes/`.

## Conventions

Rust edition 2024, toolchain pinned 1.99.1 (MSRV 1.96). Workspace lints: `pedantic`+`nursery`+`missing_docs`+
`unsafe_code` all `warn`, CI is `-D warnings` (every pub item needs a doc comment); SNES-term
exceptions live in `clippy.toml`. A chip change touches the chip code AND its `docs/<chip>.md`;
hot paths allocation-free; `unsafe` only in the frontend + FFI with `// SAFETY:`; **never commit
commercial ROMs** (only derived screenshots/hashes); never `--all-features`. RustyNES "v2.0 /
engine-lineage" anchors are NOT this project's releases.

<<< MC-PROJECT-END >>>

