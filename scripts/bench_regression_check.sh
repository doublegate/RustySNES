#!/usr/bin/env bash
# bench_regression_check.sh — headless frame-time regression gate (v1.0.0).
#
# Runs the `rustysnes-core` `headless_frame` Criterion bench and asserts it stays under an
# ABSOLUTE wall-clock ceiling. This is a deliberately non-flaky gate: shared CI runners vary by
# tens of percent run-to-run, so a tight percentage-regression gate would flake. The ceiling
# instead protects the property that actually matters — headless frame production stays
# comfortably under the 16.64 ms NTSC (60.0988 Hz) real-time deadline — and trips only on a gross
# regression. For a tighter ~5% comparison, use Criterion baselines locally instead:
#
#     cargo bench -p rustysnes-core --bench headless_frame -- --save-baseline main
#     # ... make changes ...
#     cargo bench -p rustysnes-core --bench headless_frame -- --baseline main
#
# Ported from RustyNES's own `scripts/bench_regression_check.sh`, adapted for this project's one
# `headless_frame_steady_state` bench id (no per-ROM-tier self-skip needed — the benchmark ROM,
# `tests/roms/undisbeliever/inidisp_hammer_0f00.sfc`, is a committed permissive fixture, always
# present in CI, unlike the gitignored commercial ROMs `save_state_cost.rs` self-skips for).
#
# Ceiling is in nanoseconds and overridable via env (CI can loosen it for a slow runner without
# editing this file). The default gives ~3x margin over the 3.27 ms measured on a 2026-era dev
# machine (`docs/benchmarks.md`'s `v0.4.0` baseline), well under the 16.64 ms NTSC deadline.
set -euo pipefail

cd "$(dirname "$0")/.."

MEASUREMENT_TIME="${BENCH_MEASUREMENT_TIME:-3}"
# 10 ms ceiling = ~60% of the 16.64 ms NTSC frame deadline, ~3x the 3.27 ms measured baseline.
HEADLESS_FRAME_CEILING_NS="${HEADLESS_FRAME_CEILING_NS:-10000000}"

echo "==> Running headless_frame bench (measurement-time=${MEASUREMENT_TIME}s)"
cargo bench -p rustysnes-core --bench headless_frame -- \
    --warm-up-time 1 --measurement-time "${MEASUREMENT_TIME}"

check() {
    local id="$1" ceiling="$2"
    local est="target/criterion/${id}/new/estimates.json"
    if [[ ! -f "${est}" ]]; then
        echo "FAIL: ${id}: estimates file not found (${est})"
        return 1
    fi
    local mean
    mean="$(python3 -c "import json,sys; print(int(json.load(open(sys.argv[1]))['mean']['point_estimate']))" "${est}")"
    local mean_ms
    mean_ms="$(python3 -c "print(f'{${mean}/1e6:.3f}')")"
    local ceiling_ms
    ceiling_ms="$(python3 -c "print(f'{${ceiling}/1e6:.3f}')")"
    if (( mean > ceiling )); then
        echo "FAIL: ${id}: ${mean_ms} ms/frame exceeds ceiling ${ceiling_ms} ms"
        return 1
    fi
    echo "PASS: ${id}: ${mean_ms} ms/frame (ceiling ${ceiling_ms} ms)"
    return 0
}

rc=0
check "headless_frame_steady_state" "${HEADLESS_FRAME_CEILING_NS}" || rc=1

if (( rc != 0 )); then
    echo "==> Frame-time regression gate FAILED — see docs/performance.md."
    exit 1
fi
echo "==> Frame-time regression gate passed."
