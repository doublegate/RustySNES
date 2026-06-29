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
- `to-dos/ROADMAP.md` — planning entry point; tickets `T-PS-NNN`.

## Build / test / lint (the project recipe + its gotchas)

```bash
cargo check --workspace && cargo test --workspace
cargo test --workspace --features test-roms             # the ROM oracles (gitignored corpus ⇒ self-skip)
cargo clippy --workspace --all-targets -- -D warnings   # + per-feature jobs; NEVER --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo build -p rustysnes-core --target thumbv7em-none-eabihf --no-default-features   # no_std gate
cd crates/rustysnes-frontend/web && trunk build --release                            # wasm pages deploy (keep Trunk.toml wasm-bindgen pinned to Cargo.lock)
```

Frontend opt-in features (default-off): wasm-canvas · emu-thread · debug-hooks · hd-pack ·
scripting · retroachievements. Harness features: test-roms · commercial-roms.

## Conventions

Rust edition 2024, toolchain pinned 1.96. Workspace lints: `pedantic`+`nursery`+`missing_docs`+
`unsafe_code` all `warn`, CI is `-D warnings` (every pub item needs a doc comment); SNES-term
exceptions live in `clippy.toml`. A chip change touches the chip code AND its `docs/<chip>.md`;
hot paths allocation-free; `unsafe` only in the frontend + FFI with `// SAFETY:`; **never commit
commercial ROMs** (only derived screenshots/hashes); never `--all-features`. RustyNES "v2.0 /
engine-lineage" anchors are NOT this project's releases.

<<< MC-PROJECT-END >>>

