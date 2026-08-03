#!/usr/bin/env bash
#
# run.sh -- seed each target's corpus and run a bounded campaign over all of them.
#
# Why this exists rather than a documented `cargo fuzz run` incantation: three of the settings
# below are not optional, and every one of them was learned by getting it wrong first.
#
#   1. `ASAN_OPTIONS=detect_leaks=0`. LeakSanitizer needs ptrace, which is unavailable in a
#      container and in most sandboxes. Without this, EVERY target "crashes" at exit and writes a
#      `crash-da39a3ee...` artifact -- that SHA is the empty input, which is the tell: LSan failed
#      during shutdown, the target found nothing. A campaign that reports 14 findings and has
#      actually found none is worse than one that reports nothing.
#   2. Corpus seeding. `Header::detect` scores candidate offsets, so a purely random image
#      essentially never scores above zero and `board::select` -- the part worth fuzzing -- is
#      never reached. Unseeded, `rom_header` plateaus around 29 edges; seeded from the committed
#      permissive corpus it reaches the board implementations immediately.
#   3. The dictionaries. Same problem one level down: a magic the mutator has to guess byte by byte
#      is a wall, not a gate.
#
# Usage:
#   ./run.sh                  # 60s per target, all targets
#   ./run.sh 600              # 600s per target
#   ./run.sh 600 movie patch  # 600s each, only these targets
#
# Requires a nightly toolchain (cargo-fuzz needs `-Z sanitizer=address`); the project pins 1.96
# stable in `rust-toolchain.toml`, hence the explicit `+nightly` below.

set -euo pipefail

HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd -- "$HERE/.." && pwd)"

SECONDS_PER_TARGET="${1:-60}"
shift || true

ALL_TARGETS=(
  rom_header rom_load save_state movie netplay_message patch cheat_code
  hd_pack_manifest slang_preset config_toml symbols coproc_firmware
  cpu_step apu_port_io
)
TARGETS=("${@:-}")
if [ -z "${TARGETS[0]:-}" ]; then
  TARGETS=("${ALL_TARGETS[@]}")
fi

# Fail fast with a diagnostic rather than at the first `cargo +nightly fuzz` invocation, which
# reports it as an unrecognized cargo subcommand — accurate, but it does not tell you what to
# install, and by then the seeding below has already run.
if ! cargo fuzz --version >/dev/null 2>&1; then
  echo "error: cargo-fuzz is not installed." >&2
  echo "  rustup toolchain install nightly   # cargo-fuzz needs -Z sanitizer=address" >&2
  echo "  cargo install cargo-fuzz" >&2
  exit 1
fi
if ! rustup toolchain list 2>/dev/null | grep -q '^nightly'; then
  echo "error: no nightly toolchain. cargo-fuzz needs one (this project pins 1.96 stable)." >&2
  echo "  rustup toolchain install nightly" >&2
  exit 1
fi

# See note 1 above. Exported, not passed per-command, because cargo-fuzz re-execs the target.
export ASAN_OPTIONS="${ASAN_OPTIONS:-detect_leaks=0}"

# The target triple, passed EXPLICITLY, and this is note 4 -- learned the same way as the others.
#
# `cargo fuzz` defaults `--target` to the host triple **the cargo-fuzz binary itself was built
# for**, not the one the toolchain targets. CI installs cargo-fuzz through `taiki-e/install-action`,
# which ships a statically linked musl build -- so the default became
# `x86_64-unknown-linux-musl` on a gnu runner, and every target failed to build with
#
#     error: sanitizer is incompatible with statically linked libc, ...
#     error[E0463]: can't find crate for `core` (the musl target may not be installed)
#
# `run.sh` reported all 14 as FINDING, because a build failure is a non-zero exit like a crash is.
# That is the note-1 failure mode wearing a different hat: a campaign claiming fourteen findings
# and holding none. The tell is the same -- every target "finds" something within a second or two,
# and `fuzz/artifacts/` is empty.
#
# It never showed up locally because a `cargo install`ed cargo-fuzz is a gnu build whose default
# is already right. Asking rustc for the host is what makes the two environments agree.
HOST_TRIPLE="${FUZZ_TARGET_TRIPLE:-$(rustc +nightly -vV | awk '/^host:/ { print $2 }')}"
if [ -z "$HOST_TRIPLE" ]; then
  echo "error: could not determine the host triple from 'rustc +nightly -vV'" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Seeding
# ---------------------------------------------------------------------------

seed_dir() {
  mkdir -p "$HERE/corpus/$1"
}

seed_from() {
  # seed_from <target> <find-args...> -- copies matching files into the target's corpus under a
  # content-addressed name, so re-seeding is idempotent and two identical files collapse to one.
  local target="$1"
  shift
  seed_dir "$target"
  local n=0
  while IFS= read -r -d '' f; do
    local sum
    sum="$(sha1sum "$f" | cut -d' ' -f1)"
    if [ ! -e "$HERE/corpus/$target/$sum" ]; then
      cp "$f" "$HERE/corpus/$target/$sum"
      n=$((n + 1))
    fi
  done < <(find "$@" -print0 2>/dev/null)
  echo "  seeded $target: +$n"
}

echo "Seeding corpora..."

# The committed permissive ROM corpus. `tests/roms/external/` is gitignored (commercial dumps and
# coprocessor firmware, which must never be committed), so it is used when present and skipped
# otherwise -- the same self-skipping posture the ROM oracles already take.
for t in rom_header rom_load; do
  seed_from "$t" "$REPO/tests/roms" -name '*.sfc' -not -path '*/external/*' -size -4M
done
if [ -d "$REPO/tests/roms/external" ]; then
  for t in rom_header rom_load; do
    seed_from "$t" "$REPO/tests/roms/external" -name '*.sfc' -size -4M
  done
fi

# Soft-patch files, if any are present locally (the krom set lives under the gitignored tree).
seed_from patch "$REPO/tests/roms" \( -name '*.ips' -o -name '*.bps' -o -name '*.ups' \) -size -4M

# The AccuracySNES cartridge, if it has been built -- a 256 KiB LoROM with a hand-written header,
# structurally unlike anything in the permissive corpus.
if [ -d "$REPO/tests/roms/AccuracySNES/build" ]; then
  for t in rom_header rom_load; do
    seed_from "$t" "$REPO/tests/roms/AccuracySNES/build" -name '*.sfc'
  done
fi

for t in "${ALL_TARGETS[@]}"; do seed_dir "$t"; done

# ---------------------------------------------------------------------------
# Running
# ---------------------------------------------------------------------------

# Which dictionary each target gets. Targets absent from this map have no magic to guess -- they
# take text or a byte stream with no structural gate -- and reach thousands of edges unaided.
dict_for() {
  case "$1" in
  rom_header | rom_load) echo "rom.dict" ;;
  save_state) echo "save_state.dict" ;;
  movie) echo "movie.dict" ;;
  netplay_message) echo "netplay.dict" ;;
  patch) echo "patch.dict" ;;
  *) echo "" ;;
  esac
}

echo
echo "Running ${#TARGETS[@]} target(s) at ${SECONDS_PER_TARGET}s each..."
failed=()
for t in "${TARGETS[@]}"; do
  printf '  %-18s ' "$t"
  args=(-max_total_time="$SECONDS_PER_TARGET" -rss_limit_mb=4096 -print_final_stats=1)
  d="$(dict_for "$t")"
  if [ -n "$d" ]; then
    args+=(-dict="$HERE/dictionaries/$d")
  fi

  log="$HERE/target/$t.campaign.log"
  mkdir -p "$HERE/target"
  if (cd "$REPO" && cargo +nightly fuzz run --target "$HOST_TRIPLE" "$t" "$HERE/corpus/$t" -- "${args[@]}") >"$log" 2>&1; then
    echo "clean   $(grep -oE 'cov: [0-9]+ ft: [0-9]+' "$log" | tail -1)"
  else
    echo "FINDING -- see $log and fuzz/artifacts/$t/"
    failed+=("$t")
  fi
done

echo
if [ ${#failed[@]} -ne 0 ]; then
  echo "${#failed[@]} target(s) found something: ${failed[*]}"
  echo "Reproduce: cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<artifact>"
  echo "Minimize:  cargo +nightly fuzz tmin <target> fuzz/artifacts/<target>/<artifact>"
  echo
  echo "A finding becomes a committed regression test next to the existing ones"
  echo "(e.g. movie.rs's deserialize_rejects_a_forged_huge_frame_count_without_oom),"
  echo "not a corpus entry -- the corpus is gitignored and proves nothing to a reviewer."
  exit 1
fi
echo "All ${#TARGETS[@]} target(s) clean."
