#!/usr/bin/env bash
# scripts/pgo/run.sh -- profile-guided-optimization recipe for the shipping `rustysnes` binary
# (`v1.19.0 "Afterburner"`): instrument -> train against the committed permissive ROM corpus ->
# optimized rebuild.
#
# Prerequisites (one-time):
#   cargo install cargo-pgo
#   rustup component add llvm-tools-preview
#
# Training corpus: `crates/rustysnes-test-harness/src/bin/pgo_trainer.rs` runs the committed
# MIT/Zlib test ROMs (tests/roms/gilyon + tests/roms/undisbeliever) -- always present on any
# checkout/CI runner, unlike gitignored `external/`/commercial corpora.
#
# Usage:  scripts/pgo/run.sh [frames-per-rom]
#   The optimized binary lands at target/<triple>/release/rustysnes.
#   Compare against the plain release build with
#   `cargo bench -p rustysnes-core --bench headless_frame`, or see
#   `.github/workflows/pgo.yml`'s automated A/B + determinism gate.
set -euo pipefail
cd "$(dirname "$0")/../.."

FRAMES="${1:-3600}" # ~60s of NTSC gameplay per ROM at full native speed

command -v cargo-pgo >/dev/null || {
    echo "error: cargo-pgo not installed (cargo install cargo-pgo)" >&2
    exit 1
}

echo "== 1/3 instrumented build (trainer + shared core crates) =="
cargo pgo build -- -p rustysnes-test-harness --bin pgo_trainer

echo "== 2/3 training run (${FRAMES} frames per ROM, committed permissive corpus) =="
TRIPLE="$(rustc -vV | sed -n 's/host: //p')"
"target/${TRIPLE}/release/pgo_trainer" "${FRAMES}"

echo "== 3/3 optimized build of the shipping frontend =="
cargo pgo optimize build -- -p rustysnes-frontend

echo "done: target/${TRIPLE}/release/rustysnes (PGO-optimized)"
echo "Optional extra: 'cargo pgo bolt build -- -p rustysnes-frontend' chains BOLT"
echo "post-link optimization on Linux (the CI promotion bar is >3%; see"
echo ".github/workflows/pgo.yml + docs/performance.md)."
