//! Minimal `no_std` `sin`/`cos` for the one-time Gaussian-table build.
//!
//! Only used when the crate is built without `std` (bare-metal). Accuracy must be good enough
//! that `(value * scale + 0.5) as i16` rounds identically to the `std` (libm) path for all 512
//! table entries; range-reduction + an 11-term Taylor series clears that bar comfortably. On the
//! `std` build, `f64::sin`/`cos` are used instead, so the hosted oracle path is unaffected.

// Range reduction legitimately compares floats in a loop; this is correct for the bounded inputs
// (the Gaussian table angles), so allow the float-comparison lint in this tiny helper module.
#![allow(clippy::while_float)]

use core::f64::consts::PI;

const TWO_PI: f64 = 2.0 * PI;

/// Reduce `x` into `[-PI, PI]`.
fn reduce(mut x: f64) -> f64 {
    while x > PI {
        x -= TWO_PI;
    }
    while x < -PI {
        x += TWO_PI;
    }
    x
}

/// `sin(x)` via Taylor series after range reduction.
#[must_use]
pub fn sin(x: f64) -> f64 {
    let x = reduce(x);
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    // term_{k} = -term_{k-1} * x^2 / ((2k)(2k+1))
    let mut k = 1.0_f64;
    while k < 12.0 {
        let d = (2.0 * k) * (2.0 * k + 1.0);
        term = -term * x2 / d;
        sum += term;
        k += 1.0;
    }
    sum
}

/// `cos(x)` via the `sin` identity.
#[must_use]
pub fn cos(x: f64) -> f64 {
    sin(x + PI / 2.0)
}
