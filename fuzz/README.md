# RustySNES fuzzing

Continuous adversarial testing of every boundary that ingests untrusted input.
`docs/testing-strategy.md` has named Layer 1 fuzzability as an intended property since it was
written; this is that property actually built out.

## Quick start

```bash
rustup toolchain install nightly     # cargo-fuzz needs `-Z sanitizer=address`
cargo install cargo-fuzz

./fuzz/run.sh                        # 60s per target, all 14
./fuzz/run.sh 600                    # 600s per target
./fuzz/run.sh 600 movie patch        # only these
```

`run.sh` handles corpus seeding, dictionaries, and the sanitizer settings. Read it before running
`cargo fuzz` by hand — three of the things it does are not optional, and each was learned the hard
way (see its header comment).

## Why this is a separate workspace

`fuzz/Cargo.toml` declares an empty `[workspace]` table, the cargo-fuzz convention. That keeps it
out of `cargo build --workspace` at the repo root, which matters because cargo-fuzz requires a
**nightly** toolchain while `rust-toolchain.toml` pins the project to 1.96 stable. Folding these
crates into the main workspace would drag a nightly requirement into every ordinary `cargo check`.

The root manifest therefore needs no `[workspace] exclude` entry, and must not grow one.

## The targets

Fourteen, one per boundary. Every one of these entry points already returns a `Result` (or is
infallible by design), so what is under test is **panic-freedom, unbounded allocation, and
slice-index arithmetic** — not missing error handling.

| Target | Boundary | Why it is untrusted |
|---|---|---|
| `rom_header` | `Header::detect`, `Cart::load` | A downloaded ROM; the chipset byte picks one of ~20 boards, each deriving windows from ROM-supplied size fields |
| `rom_load` | `EmuCore::load_rom` | Same, **plus the zip path** — the only boundary that hands raw bytes to a third-party crate before any of ours sees them |
| `save_state` | `System::load_state` | Traded between users, embedded in movies, supplied by a libretro frontend |
| `movie` | `Movie::deserialize` | A `u32` frame count immediately after the header |
| `netplay_message` | `NetMessage::decode` | **Off a socket, from an unauthenticated peer** — the one place a panic is a remote DoS |
| `patch` | IPS / UPS / BPS | Offset-and-length streams plus varints |
| `cheat_code` | Game Genie / Pro Action Replay | Text pasted from a website |
| `hd_pack_manifest` | `pack.toml` | A downloaded texture pack |
| `slang_preset` | `.slangp` + the GLSL→WGSL bridge | A downloaded shader preset; the bridge's rewriters are infallible `&str -> String`, so a slice-index panic is their only failure mode |
| `config_toml` | `Config` deserialization | Deeply nested `#[serde(default)]` structs; `unwrap_or_default()` at the call site cannot catch a stack overflow |
| `symbols` | `SymbolMap::load` | Returns no `Result` at all — panic-freedom is the entire specification |
| `coproc_firmware` | `install_coprocessor_firmware` | A user-sourced chip-ROM dump; returns a bare `bool`, so there is no error variant to carry a reason |
| `cpu_step` | 65C816 + bus, executing fuzzed WRAM | Reaches the PPU, DMA, CPUIO, and APU register files through the real decode path |
| `apu_port_io` | `$2140-$2143` + SMP execution | The entire S-CPU↔SPC700 channel; drives the IPL handshake off its expected path |

There is deliberately **no** `ppu_reg_io`. `Bus::read24`/`write24` and every `Ppu` field are
private, so such a target could only exist by widening the engine's public API for fuzzing's
benefit. `cpu_step` reaches the same registers by executing code, which is both honest and closer
to how a hostile ROM would get there.

## Findings become tests, not corpus entries

The corpus is gitignored, so it proves nothing to a reviewer and nothing to CI. When a target finds
something:

1. Fix it.
2. Add a committed regression test **next to the code**, named for the input class — the existing
   ones are the pattern: `deserialize_rejects_a_forged_huge_frame_count_without_oom` (`movie.rs`),
   `ips_rejects_an_absurd_offset_rather_than_allocating` (`patch.rs`),
   `load_rejects_a_tile_image_path_that_escapes_the_pack_directory` (`hd_pack.rs`),
   `bad_magic_is_rejected_not_panicked_on` (`scheduler.rs`).
3. **Inject the bug again** and confirm the new test fails at the site the fix names, and that
   nothing else does. This is the same discipline the accuracy cartridge uses, and it applies to
   ordinary code: a test that passes both with and without the fix pins nothing.

### Found so far

- **`rom_header` — unbounded shift on the `$xFD8` RAM-size byte** (fixed in `v1.26.0`).
  `1024 << N` with `N` an arbitrary byte from the image: a debug panic for `N >= 64`, and in
  release a masked shift handing `board::select` a **4 GiB** `vec![0u8; sram_size]`. `wasm32` is
  worse — `usize` is 32 bits, so the panic starts at `N >= 32`. Pinned by
  `a_forged_ram_size_byte_is_clamped_not_shifted_unbounded`, with
  `the_sram_clamp_leaves_every_real_cartridge_size_untouched` as its negative control.

  Note what it took: the bug is unreachable without a **seeded corpus**. Header detection scores
  candidate offsets, so a random image essentially never scores above zero — unseeded, `rom_header`
  plateaus around 29 edges and never reaches this code at all. It surfaced within 20 seconds of
  seeding from the committed permissive ROM corpus.

## CI

- **Compile gate**, every PR, in `ci.yml`'s `lint` job: `cargo build --manifest-path fuzz/Cargo.toml`.
  Cheap, and it is what stops a target silently rotting out of buildability as the API it calls
  moves. (The sibling RustyNES project's `fuzz/README.md` claims this gate; it does not actually
  exist there. Ours does.)
- **Weekly campaign**, in `security.yml`, which already carries a Monday cron and the
  least-privilege posture this belongs under. Per-commit fuzzing is close to worthless — the value
  is in long campaigns — so it is scheduled, not gating.

## Corpus and dictionaries

`corpus/` and `artifacts/` are gitignored; `dictionaries/` is committed.

That split is deliberate. A dictionary is text: reviewable, diffable, and it says *why* a token
matters. A binary corpus is neither, and rots silently. The dictionaries carry the format magics
(`RSNS`, `RSNESMOV`, `PATCH`/`UPS1`/`BPS1`, `PK\x03\x04`, the netplay tag bytes) plus the field
values that sit on a boundary — a magic the mutator has to guess byte by byte is a wall, not a gate.

`Cargo.lock` **is** committed: a campaign that silently changes dependency versions between runs
cannot attribute a new crash to a code change.

## Known environment issue

LeakSanitizer needs `ptrace`, which is unavailable in most containers and sandboxes. Without
`ASAN_OPTIONS=detect_leaks=0` **every** target "crashes" at exit and writes an artifact named
`crash-da39a3ee5e6b4b0d3255bfef95601890afd80709` — that SHA is the empty input, which is the tell:
LSan failed during shutdown and the target found nothing. `run.sh` sets this for you. A campaign
that reports fourteen findings and has actually found none is worse than one that reports nothing.
