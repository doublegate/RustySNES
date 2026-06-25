//! Criterion bench for `rustysnes-cpu` (each chip is benchmarkable in isolation —
//! that is what the one-directional crate graph buys us).
// reason: a real criterion harness mutates its driver; `const` fits only this empty stub.
#[allow(clippy::missing_const_for_fn)]
fn main() {
    // TODO(T-PS): criterion::criterion_group!/main! once the chip has a tick to measure.
}
