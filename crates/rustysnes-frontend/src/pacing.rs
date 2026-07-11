//! Wall-clock fixed-timestep pacer + FPS meter, shared between the native winit drive (`app.rs`)
//! and the wasm-canvas `requestAnimationFrame` drive (`wasm.rs`).
//!
//! Both display-sync loops fire at the **display** refresh (once per vsync / once per rAF
//! callback), so stepping exactly one emulated frame per callback runs the emulator at the
//! *monitor's* rate — 2.4x too fast on a 144 Hz panel. [`Pacer`] instead accumulates real elapsed
//! time and reports how many emulated frames a `1 / region-rate` period's worth of that time
//! earns (capped at [`MAX_CATCHUP_FRAMES`]), so the present mode / rAF rate governs only
//! vsync/tearing, never emulation speed. Catch-up after a stall is capped to avoid a spiral of
//! death.
//!
//! Uses [`web_time::Instant`] (not [`std::time::Instant`]) specifically so this module compiles
//! and behaves identically on `wasm32-unknown-unknown` (backed by `Performance.now()`) and native
//! (a transparent `std::time::Instant` passthrough) — the reason `web-time` is a direct dependency
//! of this crate at all.

use web_time::Instant;

/// Cap on emulated frames produced in a single present/rAF callback, so a long stall (debugger
/// break, GC pause, tab backgrounded) is absorbed rather than triggering an unbounded catch-up
/// burst.
pub const MAX_CATCHUP_FRAMES: u32 = 4;
/// FPS display refresh window, in seconds (averages out the per-present batch jitter).
pub const FPS_WINDOW: f64 = 0.5;

/// Wall-clock fixed-timestep pacer + FPS meter for a synchronous (non-threaded) emulation drive.
pub struct Pacer {
    /// Wall-clock instant of the previous `tick`/`idle` (for the elapsed-time delta).
    last: Instant,
    /// Unconsumed real time carried toward the next emulated frame, in seconds.
    accumulator: f64,
    /// Target seconds per emulated frame (`1 / region.frame_rate()`).
    period: f64,
    /// Emulated frames produced since the last FPS-window flush.
    fps_frames: u32,
    /// Wall time accrued since the last FPS-window flush, in seconds.
    fps_time: f64,
    /// The most recently computed smoothed FPS (refreshed twice a second).
    pub fps: f32,
}

impl Pacer {
    pub fn new(rate: f64) -> Self {
        Self {
            last: Instant::now(),
            accumulator: 0.0,
            period: 1.0 / rate,
            fps_frames: 0,
            fps_time: 0.0,
            fps: 0.0,
        }
    }

    /// Live-reconfigure the target rate (`v1.0.0` speed presets: `region.frame_rate() *
    /// speed_multiplier`) without resetting the accumulator/FPS-window state — same
    /// "reconfigure the live X" posture as `Gfx::set_present_mode`. A change takes effect on the
    /// very next `tick`/`advance`; no rebase is needed since the accumulator only ever holds a
    /// fraction of one period's worth of unconsumed real time.
    pub fn set_rate(&mut self, rate: f64) {
        self.period = 1.0 / rate.max(1e-6);
    }

    /// Advance the wall clock and return how many emulated frames to run this present (0..=cap).
    #[cfg_attr(feature = "emu-thread", allow(dead_code))]
    pub fn tick(&mut self) -> u32 {
        let now = Instant::now();
        // Clamp the delta so a hitch can't inject a huge backlog (spiral-of-death guard).
        let dt = (now - self.last).as_secs_f64().min(0.25);
        self.last = now;
        self.advance(dt)
    }

    /// The time-source-free core of [`Self::tick`]: fold `dt` seconds of real time into the
    /// accumulator and return how many whole emulated frames it earns (capped). Split out so the
    /// pacing math is unit-testable without sleeping on the wall clock.
    #[cfg_attr(feature = "emu-thread", allow(dead_code))]
    pub fn advance(&mut self, dt: f64) -> u32 {
        self.accumulator += dt;
        self.fps_time += dt;

        let mut frames = 0;
        while self.accumulator >= self.period && frames < MAX_CATCHUP_FRAMES {
            self.accumulator -= self.period;
            frames += 1;
        }
        if frames == MAX_CATCHUP_FRAMES {
            self.accumulator = 0.0; // drop the backlog rather than chase it forever
        }

        self.fps_frames += frames;
        self.flush_fps();
        frames
    }

    /// Threaded build: the emulation thread produces frames elsewhere, so credit one present here
    /// and let the window average it into a present-rate FPS for the status bar.
    #[cfg_attr(not(feature = "emu-thread"), allow(dead_code))]
    pub fn note_present(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64().min(0.25);
        self.last = now;
        self.fps_time += dt;
        self.fps_frames += 1;
        self.flush_fps();
    }

    /// Reset pacing while paused so resuming doesn't replay accumulated wall time as a burst.
    // Not yet called from the wasm-canvas MVP (no pause menu there yet); native `app.rs` uses it.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn idle(&mut self) {
        self.last = Instant::now();
        self.accumulator = 0.0;
        self.fps_frames = 0;
        self.fps_time = 0.0;
        self.fps = 0.0;
    }

    /// Recompute the smoothed FPS once the averaging window has elapsed.
    // The averaged FPS is a small display value (~50-60); the f64->f32 narrowing is intentional and
    // its precision loss is irrelevant for a status-bar readout.
    #[allow(clippy::cast_possible_truncation)]
    fn flush_fps(&mut self) {
        if self.fps_time >= FPS_WINDOW {
            self.fps = (f64::from(self.fps_frames) / self.fps_time) as f32;
            self.fps_frames = 0;
            self.fps_time = 0.0;
        }
    }
}

#[cfg(test)]
// The test bodies convert small, known-positive `f64` counts (~30..240) to `u32` loop bounds; the
// truncation/sign lints are irrelevant for these literals.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
mod tests {
    use super::{FPS_WINDOW, MAX_CATCHUP_FRAMES, Pacer};
    use crate::FRAME_RATE_NTSC;

    /// The fixed-timestep pacer must run the emulator at the region rate regardless of how often
    /// presents arrive: stepping the accumulator at a 144 Hz display rate and at a 30 Hz display
    /// rate both yield ~60 emulated frames per simulated second. This is the bug-2 guarantee —
    /// emulation speed is decoupled from the monitor refresh.
    #[test]
    fn pacing_tracks_region_rate_not_present_rate() {
        for present_hz in [30.0_f64, 60.0, 75.0, 144.0, 240.0] {
            let mut pacer = Pacer::new(FRAME_RATE_NTSC);
            let dt = 1.0 / present_hz;
            let presents = present_hz.round() as u32; // one simulated second
            let mut frames = 0u32;
            for _ in 0..presents {
                frames += pacer.advance(dt);
            }
            let expected = FRAME_RATE_NTSC.round() as u32; // ~60
            let diff = frames.abs_diff(expected);
            assert!(
                diff <= 2,
                "present_hz={present_hz}: emulated {frames} frames/s, expected ~{expected}"
            );
        }
    }

    /// A long stall must not trigger an unbounded catch-up burst: a single huge delta is clamped
    /// and capped to at most `MAX_CATCHUP_FRAMES` (spiral-of-death guard).
    #[test]
    fn pacing_caps_catchup_after_stall() {
        let mut pacer = Pacer::new(FRAME_RATE_NTSC);
        let frames = pacer.advance(10.0); // a 10-second stall
        assert!(
            frames <= MAX_CATCHUP_FRAMES,
            "catch-up burst {frames} exceeded cap {MAX_CATCHUP_FRAMES}"
        );
    }

    /// The FPS meter reports the measured emulated-frame rate once the averaging window elapses.
    #[test]
    fn fps_meter_reports_region_rate() {
        let mut pacer = Pacer::new(FRAME_RATE_NTSC);
        let dt = 1.0 / 144.0;
        // Run just past the FPS averaging window at a 144 Hz present rate.
        let presents = (FPS_WINDOW * 144.0).ceil() as u32 + 1;
        for _ in 0..presents {
            pacer.advance(dt);
        }
        assert!(
            (pacer.fps - 60.0).abs() < 3.0,
            "fps meter read {}, expected ~60",
            pacer.fps
        );
    }

    /// `set_rate` (`v1.0.0` speed presets) changes the emulated-frame cadence on the very next
    /// `advance` call, without needing to rebuild the `Pacer` (and losing its FPS/accumulator
    /// state) the way a fresh `Pacer::new` would.
    #[test]
    fn set_rate_changes_cadence_immediately() {
        let mut pacer = Pacer::new(FRAME_RATE_NTSC);
        pacer.set_rate(FRAME_RATE_NTSC * 2.0);
        let dt = 1.0 / 60.0; // one present at the ORIGINAL rate's period
        let frames = pacer.advance(dt);
        // At double rate, one 1/60s tick earns ~2 emulated frames, not ~1.
        assert_eq!(frames, 2);
    }
}
