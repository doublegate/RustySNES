<!-- Managed by Master-Claude. Universal rules come from the imported/inlined core.
     Edit only inside the MC-PROJECT block; mc-sync overwrites everything else. -->
<!-- mc-core: 0.1.0 | mode=import | lang=rust -->
# AGENTS.md — RustySNES

@/home/parobek/.claude/master-core/AGENTS.base.md
@/home/parobek/.claude/master-core/lang/rust.md
@/home/parobek/.claude/master-core/modules/10-commits-and-versioning.md
@/home/parobek/.claude/master-core/modules/20-testing-and-accuracy.md
@/home/parobek/.claude/master-core/modules/30-quality-gates.md
@/home/parobek/.claude/master-core/modules/40-docs-and-adrs.md
@/home/parobek/.claude/master-core/modules/50-architecture-patterns.md
@/home/parobek/.claude/master-core/modules/60-security.md
@/home/parobek/.claude/master-core/modules/70-release-ceremony.md
@/home/parobek/.claude/master-core/modules/80-phase-sprint-workflow.md
@/home/parobek/.claude/master-core/modules/90-multi-language-integration.md
@/home/parobek/.claude/master-core/modules/95-named-pattern-library.md

<<< MC-PROJECT-START >>>
## Project: RustySNES

A cycle-accurate Super Nintendo / Super Famicom emulator in Rust at the Mesen2 / ares / higan bar.
Phases 0–5 complete and **playable**: the CPU (65C816), PPU, APU (SPC700 + S-DSP), base mappers,
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
  `docs/adr/` — ADRs. `ref-docs/` — immutable research. `ref-proj/` — study clones (gitignored; bsnes/ares/Mesen2).
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
REF_PROJ=$PWD/ref-proj bash scripts/accuracysnes/crossval.sh   # battery + PAL image + rendered scenes
```

`accuracysnes-gen` is a **workspace member**, so lint it with the workspace command above — running
`cargo clippy` from inside `tests/roms/AccuracySNES/gen/` picks up different settings and reports
clean while CI fails.

Frontend opt-in features (default-off): wasm-canvas · emu-thread · debug-hooks · hd-pack ·
scripting · retroachievements. Harness features: test-roms · commercial-roms.

## AccuracySNES (the first-party accuracy cartridge)

Two artifacts from one build: `build/accuracysnes.sfc` and `build/accuracysnes-pal.sfc`, which
differ in **one header byte plus the checksum** — so any behavioural difference between them is the
video region and can be nothing else. Region-dependent tests key on the *measured* frame height,
never on the region bit (whose position `$213F` bit 4 was itself settled by diffing the two images).

**Two tiers, deliberately never summed.** The battery is self-scoring: the cart decides pass/fail
on-cart and the host supplies no expected values, so a result means the same thing on any emulator
and on a flash cart. Rendered **scenes** (`docs/adr/0013`) cover the parts of the PPU that only
decide what appears on screen — the cart renders, the host hashes a fixed 256x224 region of
canonical pixels and compares against `tests/golden/accuracysnes-scenes.tsv`. A scene needs a host
holding the golden, so scene results stay in their own column in the coverage report and out of the
pass rate. Per ADR 0013 a golden is blessed **only** from a render the reference emulators agree on.

Working rules that have each already cost a debugging session:

- The generator emits `.a8`/`.a16` from every `sep`/`rep`; ca65 tracks immediate width from the
  directives, not from the instruction. A miss assembles a 2-byte immediate after `sep #$20` and
  desynchronises everything after it. Assembly helpers are **width-neutral** (`php`/`plp`).
- Each scene starts from a rebuilt canvas *and* re-run `init_registers`. Build the canvas once and
  the first scene to touch VRAM silently changes the picture for every scene after it.
- A scene can arrange a state **no picture can show**, and cross-validation cannot catch that class
  at all — an unshowable scene hashes stably and every emulator agrees with it. Two instances, both
  now handled by `scene_low_tiles`: a tile below `$10` covers only ASCII 0-31 (a 4bpp tile spans two
  font glyphs, an 8bpp tile four), which are blank; and a vertical offset that is a multiple of 16
  is invisible against a 16-tile cycle. Check that a scene renders what it claims to arrange.
- `STZ` has no long-addressing form; `cop #$00` is rejected by ca65 2.19 (emit `.byte $02,$00`);
  menu labels are capped at 24 columns.
- **Never hand-write a verdict byte.** Use the assertion helpers even when the condition is not an
  equality — `assert_a16_range` expresses "must not be X" fine. A hand-written `sta V_TEST_RESULT`
  puts a code in the ROM that the generated `ERROR_CODES.md` cannot know about, so the table stops
  being the complete account of failure bytes that it exists to be. Got wrong twice.
- Three emulators failing **identically** usually means a broken test; RustySNES failing **alone**
  means a real bug. Both have happened repeatedly — check which before investigating. But the first
  is a **heuristic, not a proof**: a harness bug upstream of every implementation produces the same
  signature, and one did — it cost a published finding that had to be retracted (see the `$F8`/`$F9`
  correction in the CHANGELOG).

### Group E — the APU, reached through four bytes

The SPC700 is a separate processor with its own RAM; the only channel is `$2140`-`$2143`. The cart
uploads a small SPC700 program through the IPL boot handshake (`apu_upload` in `asm/runtime.s`),
lets it run, and reads its answers back. `gen/src/spc.rs` assembles those programs — `ca65` does not
speak SPC700. Verify any new opcode encoding against
`crates/rustysnes-apu/src/spc700_exec.rs`'s dispatch table.

- **Every program must hand the APU back to the IPL** (`release_to_ipl`), and that path **re-maps
  the IPL ROM first**. `$F1` bit 7 selects whether `$FFC0`+ reads as the boot ROM or as RAM, so a
  test that writes `$F1` for its own reasons (enabling a timer, say) leaves `JMP $FFC0` landing in
  dead RAM — and then *every upload after it silently fails* while the battery still reports 100%.
- **Every handshake wait is bounded**, and a test whose APU never answers reports SKIP. An
  unbounded wait hangs the whole battery and reports nothing about any other test.
- The emitter carries only opcodes a committed test exercises. An unexercised encoding is an
  unverified one, and a wrong byte in it surfaces as an emulator disagreement rather than as an
  assembler bug.

## Conventions

Rust edition 2024, toolchain pinned 1.96. Workspace lints: `pedantic`+`nursery`+`missing_docs`+
`unsafe_code` all `warn`, CI is `-D warnings` (every pub item needs a doc comment); SNES-term
exceptions live in `clippy.toml`. A chip change touches the chip code AND its `docs/<chip>.md`;
hot paths allocation-free; `unsafe` only in the frontend + FFI with `// SAFETY:`; **never commit
commercial ROMs** (only derived screenshots/hashes); never `--all-features`. RustyNES "v2.0 /
engine-lineage" anchors are NOT this project's releases.

<<< MC-PROJECT-END >>>

