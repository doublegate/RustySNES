//! A graphic equaliser for the output path (`v1.25.0`).
//!
//! Ported from RustyNES's own `eq.rs`: a cascade of RBJ peaking biquads, one per band, applied to the
//! resampled `f32` stereo stream. Like the DRC servo and every post-filter, this lives strictly in the
//! **frontend** — the deterministic core emits the same samples either way, so the
//! `docs/adr/0004` determinism contract is untouched.
//!
//! # Flat is bit-identical bypass
//!
//! With every band at 0 dB the filters are mathematically an identity, but running them anyway would
//! still perturb the output by float rounding — so "flat" is detected and the samples are returned
//! untouched. That matters beyond purity: it is what lets this be enabled by default-off without any
//! risk of changing existing output, and what makes "is the EQ doing anything?" answerable.

/// The number of bands. Five octave-ish bands is the classic graphic-EQ layout and is enough to fix
/// the two things people actually want (too much bass, harsh treble) without a 20-slider wall.
pub const BANDS: usize = 5;

/// Band centre frequencies in Hz.
///
/// Roughly 2-octave spacing across the SNES's useful range. The top band sits at 10 kHz because the
/// S-DSP's 32 kHz output has a 16 kHz Nyquist limit — a band centred above that would be filtering
/// content that cannot exist.
pub const CENTRES_HZ: [f32; BANDS] = [60.0, 240.0, 1_000.0, 3_500.0, 10_000.0];

/// Shared Q for every band. 0.9 gives adjacent bands a gentle overlap, so moving one slider does not
/// leave an audible notch beside it.
const Q: f32 = 0.9;

/// The gain range each band is clamped to, in dB.
pub const GAIN_LIMIT_DB: f32 = 12.0;

/// One biquad section's coefficients plus its per-channel state.
///
/// Transposed direct form II: two state words per channel, and the two channels are deliberately
/// kept separate — sharing state across L/R is a classic bug that collapses stereo to a smeared mono
/// while still "sounding like EQ".
#[derive(Debug, Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    left: (f32, f32),
    right: (f32, f32),
}

impl Biquad {
    /// RBJ peaking-EQ coefficients for `gain_db` at `centre` Hz, sampled at `rate`.
    fn peaking(centre: f32, gain_db: f32, rate: f32) -> Self {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * centre / rate.max(1.0);
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * Q);
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos_w0) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos_w0) / a0,
            a2: (1.0 - alpha / a) / a0,
            left: (0.0, 0.0),
            right: (0.0, 0.0),
        }
    }

    /// Filter one sample of one channel, advancing that channel's own state.
    ///
    /// Transposed direct form II, the standard two-state biquad recurrence:
    /// `y = b0*x + z1` ; `z1' = b1*x - a1*y + z2` ; `z2' = b2*x - a2*y`.
    fn run(&mut self, x: f32, right_channel: bool) -> f32 {
        let (z1, z2) = if right_channel { self.right } else { self.left };
        let y = self.b0.mul_add(x, z1);
        let next = (
            self.b1.mul_add(x, (-self.a1).mul_add(y, z2)),
            self.b2.mul_add(x, -(self.a2 * y)),
        );
        if right_channel {
            self.right = next;
        } else {
            self.left = next;
        }
        y
    }
}

/// A five-band graphic equaliser.
pub struct Equalizer {
    bands: [Biquad; BANDS],
    gains_db: [f32; BANDS],
    rate: f32,
    /// Whether every band is flat, in which case [`Self::process`] is an exact pass-through.
    flat: bool,
    enabled: bool,
}

impl Equalizer {
    /// Build an equaliser for `rate` Hz output with the given per-band gains in dB.
    #[must_use]
    pub fn new(rate: u32, gains_db: [f32; BANDS], enabled: bool) -> Self {
        let mut me = Self {
            bands: [Biquad::default(); BANDS],
            gains_db: [0.0; BANDS],
            #[allow(clippy::cast_precision_loss)]
            rate: rate.max(1) as f32,
            flat: true,
            enabled,
        };
        me.set_gains(gains_db);
        me
    }

    /// Whether the EQ will actually alter samples (enabled AND not flat).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.enabled && !self.flat
    }

    /// Enable or disable the whole stage.
    pub const fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Update the per-band gains (from the Settings sliders), recomputing coefficients.
    ///
    /// Filter *state* is deliberately preserved across a gain change: zeroing it would produce a
    /// click every time a slider moves.
    pub fn set_gains(&mut self, gains_db: [f32; BANDS]) {
        self.flat = true;
        for i in 0..BANDS {
            let g = gains_db[i].clamp(-GAIN_LIMIT_DB, GAIN_LIMIT_DB);
            self.gains_db[i] = g;
            if g.abs() > 0.01 {
                self.flat = false;
            }
            let state = (self.bands[i].left, self.bands[i].right);
            self.bands[i] = Biquad::peaking(CENTRES_HZ[i], g, self.rate);
            self.bands[i].left = state.0;
            self.bands[i].right = state.1;
        }
    }

    /// The current gains, for the Settings sliders to display.
    #[must_use]
    pub const fn gains_db(&self) -> [f32; BANDS] {
        self.gains_db
    }

    /// Filter one interleaved stereo pair. An inactive EQ returns the input unchanged, bit for bit.
    #[must_use]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.is_active() {
            return (l, r);
        }
        let mut out_l = l;
        let mut out_r = r;
        for band in &mut self.bands {
            out_l = band.run(out_l, false);
            out_r = band.run(out_r, true);
        }
        // A boosted cascade can exceed full scale; clamping here rather than letting it wrap is the
        // difference between "loud" and "harsh digital distortion".
        (out_l.clamp(-1.0, 1.0), out_r.clamp(-1.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_is_an_exact_pass_through() {
        let mut eq = Equalizer::new(48_000, [0.0; BANDS], true);
        assert!(!eq.is_active(), "all-zero gains must be detected as flat");
        // Bit-identical, not merely close: this is what makes the stage safe to leave in the path.
        for (i, x) in [0.0f32, 0.5, -0.5, 1.0, -1.0, 0.123_456]
            .into_iter()
            .enumerate()
        {
            let (l, r) = eq.process(x, -x);
            #[allow(clippy::float_cmp)]
            let exact = l == x && r == -x;
            assert!(exact, "sample {i} was altered by a flat EQ: {l} / {r}");
        }
    }

    #[test]
    fn disabled_is_a_pass_through_even_with_nonzero_gains() {
        let mut eq = Equalizer::new(48_000, [6.0; BANDS], false);
        assert!(!eq.is_active());
        let (l, r) = eq.process(0.25, -0.25);
        #[allow(clippy::float_cmp)]
        let exact = l == 0.25 && r == -0.25;
        assert!(exact);
    }

    #[test]
    fn a_boost_raises_energy_and_a_cut_lowers_it() {
        // Feed a steady tone near a band centre and compare summed energy against flat. A boost must
        // increase it and a cut decrease it; this is the check that the coefficient signs are right,
        // which no amount of "it runs" testing would catch.
        let rate = 48_000.0f32;
        let centre = CENTRES_HZ[2]; // 1 kHz
        let tone = |n: u32| {
            (2.0 * std::f32::consts::PI * centre * f32::from(u16::try_from(n).unwrap_or(u16::MAX))
                / rate)
                .sin()
                * 0.5
        };

        let energy = |gain: f32| -> f32 {
            let mut gains = [0.0; BANDS];
            gains[2] = gain;
            let mut eq = Equalizer::new(48_000, gains, true);
            // Skip the first samples so the filter has settled rather than measuring its attack.
            let mut sum = 0.0;
            for n in 0..4096u32 {
                let (l, _) = eq.process(tone(n), tone(n));
                if n > 512 {
                    sum = l.mul_add(l, sum);
                }
            }
            sum
        };

        let flat = energy(0.0);
        let boosted = energy(9.0);
        let cut = energy(-9.0);
        assert!(
            boosted > flat * 1.5,
            "boost did not raise energy: {boosted} vs {flat}"
        );
        assert!(
            cut < flat * 0.8,
            "cut did not lower energy: {cut} vs {flat}"
        );
    }

    #[test]
    fn channels_are_filtered_independently() {
        // Shared filter state across L/R smears stereo into mono while still "sounding like EQ", so
        // assert the two channels genuinely do not leak into one another: silence on the right must
        // stay silence, however loud the left is.
        let mut gains = [0.0; BANDS];
        gains[1] = 10.0;
        let mut eq = Equalizer::new(48_000, gains, true);
        let mut right_energy = 0.0f32;
        for n in 0..2048u32 {
            let l = (2.0
                * std::f32::consts::PI
                * 240.0
                * f32::from(u16::try_from(n).unwrap_or(u16::MAX))
                / 48_000.0)
                .sin()
                * 0.8;
            let (_, r) = eq.process(l, 0.0);
            right_energy = r.mul_add(r, right_energy);
        }
        assert!(
            right_energy < 1e-6,
            "left channel leaked into the right: energy {right_energy}"
        );
    }

    #[test]
    fn gains_are_clamped_and_output_never_exceeds_full_scale() {
        let mut eq = Equalizer::new(48_000, [1000.0; BANDS], true);
        for g in eq.gains_db() {
            assert!((g - GAIN_LIMIT_DB).abs() < 1e-6, "gain not clamped: {g}");
        }
        // Even a maximal boost on a full-scale input must not leave the valid range.
        for n in 0..1024 {
            let x = if n % 2 == 0 { 1.0 } else { -1.0 };
            let (l, r) = eq.process(x, x);
            assert!(
                l.abs() <= 1.0 + 1e-6 && r.abs() <= 1.0 + 1e-6,
                "clipped past full scale"
            );
        }
    }

    #[test]
    fn setting_gains_to_flat_restores_pass_through() {
        let mut eq = Equalizer::new(48_000, [8.0; BANDS], true);
        assert!(eq.is_active());
        eq.set_gains([0.0; BANDS]);
        assert!(
            !eq.is_active(),
            "returning every slider to 0 must re-enable bypass"
        );
        let (l, _) = eq.process(0.75, 0.75);
        #[allow(clippy::float_cmp)]
        let exact = l == 0.75;
        assert!(exact);
    }
}
