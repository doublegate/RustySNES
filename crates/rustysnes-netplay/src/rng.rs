//! A seeded, deterministic PRNG for test-only synthetic network conditions.
//!
//! Used by [`crate::transport::MemoryTransport`]'s latency/jitter/drop simulation — never
//! `std::time`, never OS randomness, so a determinism test's "network" behavior replays
//! identically across runs (`docs/adr/0004`). Ported from RustyNES's `rustynes-netplay::rng`
//! (`SplitMix64`, David Blackman & Sebastiano Vigna's public-domain generator).

/// A `SplitMix64` generator: minimal state (one `u64`), well-distributed, fast — exactly what a
/// test harness needs for reproducible synthetic jitter, not cryptographic strength.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Seed a new generator. The same seed always produces the same output sequence.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit output in the sequence.
    pub const fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A pseudo-random `f64` in `[0.0, 1.0)`.
    // The top 53 bits give a uniformly distributed mantissa's worth of precision — the whole
    // point is a lossy u64 -> f64 narrowing into exactly that many significant bits, not a bug.
    #[allow(clippy::cast_precision_loss)]
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// True with probability `p` (`p` clamped to `[0.0, 1.0]`).
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::SplitMix64;

    #[test]
    fn same_seed_is_bit_identical() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn chance_zero_never_fires_chance_one_always_fires() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..100 {
            assert!(!rng.chance(0.0));
        }
        for _ in 0..100 {
            assert!(rng.chance(1.0));
        }
    }
}
