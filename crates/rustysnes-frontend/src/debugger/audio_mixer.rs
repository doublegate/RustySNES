//! The 8-voice audio mixer panel (`v1.25.0`, T-FP-F).
//!
//! Per-voice gain, mute, and a VU meter, plus a small set of presets. The `v1.0.1` debugger already
//! had per-voice **mute**; what it could not do is the thing a mixer is actually for — turning one
//! voice down to hear what another is doing, which needs a continuous gain rather than a binary.
//!
//! # Why the meter is computed here
//!
//! `Dsp::voice_taps` reports what each voice contributed to the **most recent sample**. That is the
//! honest primitive: a meter needs a decay constant, and a decay constant baked into the DSP would
//! be a UI concern living in the emulation core. So the smoothing lives here, where the frame rate
//! is known.
//!
//! Gains and mutes are host state: never in a save state, and a gain of exactly `1.0` is a bit-exact
//! no-op in the DSP — the same contract `crate::eq` holds for the frontend's own audio path.

use crate::ui_shell::ShellState;

/// Voices the S-DSP has.
pub const VOICES: usize = 8;

/// How fast a meter falls, as a fraction retained per present.
///
/// Meters rise instantly and fall slowly, which is what makes a transient visible at all — a
/// symmetric response averages a drum hit down to nothing between frames.
const METER_DECAY: f32 = 0.85;

/// One voice's mixer state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceStrip {
    /// Output gain, from zero up to [`rustysnes_core::apu::dsp::MAX_VOICE_GAIN`].
    ///
    /// `1.0` is unity and bit-exact in the DSP.
    pub gain: f32,
    /// Whether the voice is muted.
    pub mute: bool,
    /// Whether the voice is soloed.
    pub solo: bool,
}

impl Default for VoiceStrip {
    fn default() -> Self {
        Self {
            gain: 1.0,
            mute: false,
            solo: false,
        }
    }
}

/// The mixer's whole state.
#[derive(Debug, Clone, PartialEq)]
pub struct MixerState {
    /// Per-voice strips.
    pub strips: [VoiceStrip; VOICES],
    /// Smoothed meter levels, `0.0..=1.0`, one per voice.
    pub meters: [f32; VOICES],
}

impl Default for MixerState {
    fn default() -> Self {
        Self {
            strips: [VoiceStrip::default(); VOICES],
            meters: [0.0; VOICES],
        }
    }
}

impl MixerState {
    /// The mute array to push into the DSP.
    ///
    /// **Solo wins over mute**: with any voice soloed, every un-soloed voice is muted regardless of
    /// its own mute button. That is what solo means, and computing it here rather than in the UI
    /// keeps the two buttons from disagreeing about which one is authoritative.
    #[must_use]
    pub fn mutes(&self) -> [bool; VOICES] {
        let any_solo = self.strips.iter().any(|s| s.solo);
        core::array::from_fn(|i| {
            if any_solo {
                !self.strips[i].solo
            } else {
                self.strips[i].mute
            }
        })
    }

    /// The gain array to push into the DSP, clamped to the DSP's own accepted range.
    #[must_use]
    pub fn gains(&self) -> [f32; VOICES] {
        core::array::from_fn(|i| {
            let g = self.strips[i].gain;
            if g.is_finite() {
                g.clamp(0.0, rustysnes_core::apu::dsp::MAX_VOICE_GAIN)
            } else {
                1.0
            }
        })
    }

    /// Fold this present's taps into the smoothed meters.
    ///
    /// Rise is instant, fall is exponential — see this module's `METER_DECAY`. A plain code span
    /// rather than an intra-doc link, since that constant is private and the doc build rejects a
    /// public item linking to one (`docs/`'s own rule about links that only resolve with
    /// `--document-private-items`).
    pub fn update_meters(&mut self, taps: [(i16, i16); VOICES]) {
        for (meter, (l, r)) in self.meters.iter_mut().zip(taps.iter()) {
            // Clamped: `i16::MIN.saturating_abs()` is `32767`… but the tap arrives as an `i16`
            // whose magnitude can legitimately be `32768` before saturation, and dividing by
            // `i16::MAX` puts a full-scale negative sample fractionally over `1.0`. A meter that
            // reads 100.003% draws outside its own bar.
            #[allow(clippy::cast_precision_loss)]
            let level = ((i32::from(l.saturating_abs()).max(i32::from(r.saturating_abs())) as f32)
                / f32::from(i16::MAX))
            .min(1.0);
            *meter = if level > *meter {
                level
            } else {
                *meter * METER_DECAY
            };
        }
    }

    /// Reset every strip to unity and clear the meters.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whether every strip is at its default — i.e. the mixer is a true bypass.
    #[must_use]
    pub fn is_neutral(&self) -> bool {
        self.strips
            .iter()
            .all(|s| !s.mute && !s.solo && (s.gain - 1.0).abs() < f32::EPSILON)
    }
}

impl ShellState {
    /// The mixer panel.
    pub(crate) fn render_audio_mixer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() {
                self.mixer.reset();
            }
            if ui.button("Solo none").clicked() {
                for strip in &mut self.mixer.strips {
                    strip.solo = false;
                }
            }
            if self.mixer.is_neutral() {
                ui.label(
                    egui::RichText::new("neutral — bit-exact bypass")
                        .small()
                        .weak(),
                );
            }
        });
        ui.separator();

        egui::Grid::new("mixer_grid")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                for h in ["Voice", "Level", "Gain", "M", "S"] {
                    ui.strong(h);
                }
                ui.end_row();
                for i in 0..VOICES {
                    ui.monospace(format!("{i}"));
                    meter_bar(ui, self.mixer.meters[i]);
                    ui.add(
                        egui::Slider::new(
                            &mut self.mixer.strips[i].gain,
                            0.0..=rustysnes_core::apu::dsp::MAX_VOICE_GAIN,
                        )
                        .show_value(true)
                        .fixed_decimals(2),
                    );
                    ui.checkbox(&mut self.mixer.strips[i].mute, "");
                    ui.checkbox(&mut self.mixer.strips[i].solo, "");
                    ui.end_row();
                }
            });
        ui.separator();
        ui.label(
            egui::RichText::new(
                "Gains and mutes are host state: they never enter a save state, and a gain of \
                 exactly 1.00 is bit-exact in the DSP. Soloing any voice mutes every other.",
            )
            .small()
            .weak(),
        );
    }
}

/// A horizontal level bar.
fn meter_bar(ui: &mut egui::Ui, level: f32) {
    let (rect, _r) = ui.allocate_exact_size(egui::vec2(60.0, 12.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 1.0, ui.visuals().extreme_bg_color);
    let filled = level.clamp(0.0, 1.0) * rect.width();
    if filled > 0.5 {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(filled, rect.height()));
        // Green below, amber near the top: a meter that is one colour cannot show "close to
        // clipping", which is the only reading a mixer meter is actually consulted for.
        let color = if level > 0.85 {
            egui::Color32::from_rgb(220, 160, 40)
        } else {
            egui::Color32::from_rgb(60, 180, 90)
        };
        painter.rect_filled(bar, 1.0, color);
    }
}

#[cfg(test)]
mod tests {
    use super::{MixerState, VOICES};

    #[test]
    fn a_fresh_mixer_is_a_bit_exact_bypass() {
        let m = MixerState::default();
        assert!(m.is_neutral());
        assert_eq!(m.mutes(), [false; VOICES]);
        for g in m.gains() {
            assert!((g - 1.0).abs() < f32::EPSILON, "unity, not {g}");
        }
    }

    /// Solo wins over mute: with any voice soloed, every un-soloed voice is muted regardless of
    /// its own mute button. Computing it in one place is what stops the two buttons disagreeing.
    #[test]
    fn solo_overrides_mute() {
        let mut m = MixerState::default();
        m.strips[3].solo = true;
        m.strips[3].mute = true; // even its own mute loses to its solo
        m.strips[5].mute = false;
        let mutes = m.mutes();
        assert!(!mutes[3], "the soloed voice plays");
        for (i, muted) in mutes.iter().enumerate() {
            if i != 3 {
                assert!(muted, "voice {i} must be muted while another is soloed");
            }
        }
        // With no solo, the mute buttons are authoritative again.
        m.strips[3].solo = false;
        let mutes = m.mutes();
        assert!(mutes[3]);
        assert!(!mutes[5]);
    }

    /// A hostile gain must not silence or invert a voice permanently.
    #[test]
    fn gains_are_clamped_to_the_dsp_range() {
        let mut m = MixerState::default();
        m.strips[0].gain = -5.0;
        m.strips[1].gain = 1e9;
        m.strips[2].gain = f32::NAN;
        m.strips[3].gain = f32::INFINITY;
        let g = m.gains();
        assert!((g[0] - 0.0).abs() < f32::EPSILON);
        assert!((g[1] - rustysnes_core::apu::dsp::MAX_VOICE_GAIN).abs() < f32::EPSILON);
        assert!((g[2] - 1.0).abs() < f32::EPSILON, "NaN falls back to unity");
        assert!((g[3] - 1.0).abs() < f32::EPSILON, "inf falls back to unity");
    }

    /// Meters rise instantly and fall slowly — a symmetric response averages a drum hit down to
    /// nothing between frames.
    #[test]
    fn meters_rise_instantly_and_decay_slowly() {
        let mut m = MixerState::default();
        let mut taps = [(0i16, 0i16); VOICES];
        taps[0] = (i16::MAX, 0);
        m.update_meters(taps);
        assert!((m.meters[0] - 1.0).abs() < 1e-3, "{}", m.meters[0]);

        // Silence afterwards: the meter falls but does not snap to zero.
        m.update_meters([(0, 0); VOICES]);
        assert!(m.meters[0] < 1.0 && m.meters[0] > 0.5, "{}", m.meters[0]);
        for _ in 0..50 {
            m.update_meters([(0, 0); VOICES]);
        }
        assert!(m.meters[0] < 0.01, "should decay to ~0: {}", m.meters[0]);
    }

    /// The meter takes the louder channel, so a hard-panned voice still registers.
    #[test]
    fn meters_take_the_louder_channel() {
        let mut m = MixerState::default();
        let mut taps = [(0i16, 0i16); VOICES];
        taps[0] = (0, i16::MAX); // hard right
        m.update_meters(taps);
        assert!(m.meters[0] > 0.9, "{}", m.meters[0]);
    }

    /// `i16::MIN` has no positive counterpart; a naive `abs()` panics in debug.
    #[test]
    fn the_most_negative_sample_does_not_panic() {
        let mut m = MixerState::default();
        let mut taps = [(0i16, 0i16); VOICES];
        taps[0] = (i16::MIN, i16::MIN);
        m.update_meters(taps);
        assert!(m.meters[0] > 0.9);
    }

    #[test]
    fn reset_returns_to_neutral() {
        let mut m = MixerState::default();
        m.strips[0].gain = 0.0;
        m.strips[1].mute = true;
        m.strips[2].solo = true;
        m.meters[0] = 1.0;
        assert!(!m.is_neutral());
        m.reset();
        assert!(m.is_neutral());
        assert!(m.meters.iter().all(|v| *v == 0.0));
    }
}
