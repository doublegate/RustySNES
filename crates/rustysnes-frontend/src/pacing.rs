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

/// How close the display's refresh must be to the region rate for
/// [`crate::config::PacingMode::Display`] to be chosen automatically, in Hz.
///
/// A 60 Hz panel against NTSC's 60.0988 Hz is a 0.0988 Hz mismatch — well inside this, and exactly
/// the case display-sync exists for. A 59.94 Hz panel is likewise fine. A 75 Hz panel is not: locking
/// to it would need the emulator to run 25% fast, so `Auto` must not pick display-sync there.
pub const DISPLAY_SYNC_TOLERANCE_HZ: f64 = 1.0;

/// The margin reserved for a busy-wait at the end of a paced sleep.
///
/// A thread sleep is only accurate to the OS scheduler's granularity (commonly ~1-2 ms, worse under
/// load), so sleeping right up to the deadline routinely overshoots and drops a frame. Sleeping to
/// `deadline - margin` and then spinning trades a little CPU for hitting the deadline.
pub const SPIN_MARGIN: core::time::Duration = core::time::Duration::from_millis(2);

/// A resolved pacing strategy: what [`crate::config::PacingMode`] actually *means* for this session.
///
/// `v1.25.0` (T-FP-B). Before this, `PacingMode` was declared and serialized in `config.toml` and
/// **read by nothing** — every variant behaved identically. This type is what makes the setting real.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacingPlan {
    /// The concrete mode chosen. Never [`crate::config::PacingMode::Auto`] — `Auto` is a *request*,
    /// and resolving it is this type's whole job.
    pub mode: crate::config::PacingMode,
    /// The wgpu present-mode preference this plan implies (`"fifo"` / `"mailbox"` / `"immediate"`).
    pub present_mode: &'static str,
    /// Whether the caller should sleep/spin to a deadline rather than relying on vsync to block.
    pub self_paced: bool,
    /// Why this plan was chosen, for the Settings/perf panel to display.
    ///
    /// Surfaced because a *silently* downgraded plan is indistinguishable from a broken one: a user
    /// who asked for display-sync on a 75 Hz panel needs to be told it was declined and why.
    pub reason: &'static str,
}

impl core::fmt::Display for PacingPlan {
    /// `"<mode> (<present mode>) — <reason>"`, the exact string the Settings/Performance panels
    /// show. The reason is always included: a downgraded plan the user cannot see is
    /// indistinguishable from a bug.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({}) — {}",
            self.mode.display_name(),
            self.present_mode,
            self.reason
        )
    }
}

/// Resolve a requested [`crate::config::PacingMode`] against what the display and surface support.
///
/// `display_hz` is the monitor's refresh (`None` when the windowing system does not report one);
/// `region_hz` is the emulated console's frame rate. `mailbox`/`immediate` say which wgpu present
/// modes the surface actually offers.
///
/// Emulation always runs at `region_hz` — a pacing mode governs *presentation* (tearing, latency, and
/// whether the frontend blocks on vsync or on its own clock), never emulation speed. That separation
/// is the pre-existing `Pacer` contract and this must not weaken it.
#[must_use]
pub fn resolve(
    requested: crate::config::PacingMode,
    display_hz: Option<f64>,
    region_hz: f64,
    mailbox: bool,
    immediate: bool,
) -> PacingPlan {
    use crate::config::PacingMode as M;
    let display_matches =
        display_hz.is_some_and(|hz| (hz - region_hz).abs() <= DISPLAY_SYNC_TOLERANCE_HZ);

    match requested {
        M::Display if display_matches => PacingPlan {
            mode: M::Display,
            present_mode: "fifo",
            self_paced: false,
            reason: "display refresh matches the region rate; vsync governs pacing",
        },
        // Asked for, but the panel cannot honour it. Do NOT silently pretend.
        M::Display => PacingPlan {
            mode: M::Wallclock,
            present_mode: if immediate { "immediate" } else { "fifo" },
            self_paced: true,
            reason: "display refresh differs from the region rate; fell back to wall-clock",
        },
        M::Vrr if mailbox => PacingPlan {
            mode: M::Vrr,
            present_mode: "mailbox",
            self_paced: true,
            reason: "variable-refresh: present as soon as a frame is ready",
        },
        M::Vrr => PacingPlan {
            mode: M::Wallclock,
            present_mode: if immediate { "immediate" } else { "fifo" },
            self_paced: true,
            reason: "surface offers no mailbox mode; fell back to wall-clock",
        },
        M::Wallclock => PacingPlan {
            mode: M::Wallclock,
            present_mode: if immediate { "immediate" } else { "fifo" },
            self_paced: true,
            reason: "free-running on the wall clock at the region rate",
        },
        // `Auto`: prefer vsync when the panel genuinely matches (lowest jitter, no spin), then
        // mailbox, then wall-clock. Deliberately ordered cheapest-correct first.
        M::Auto if display_matches => PacingPlan {
            mode: M::Display,
            present_mode: "fifo",
            self_paced: false,
            reason: "auto: display refresh matches the region rate",
        },
        M::Auto if mailbox => PacingPlan {
            mode: M::Vrr,
            present_mode: "mailbox",
            self_paced: true,
            reason: "auto: refresh mismatch, using mailbox present",
        },
        M::Auto => PacingPlan {
            mode: M::Wallclock,
            present_mode: if immediate { "immediate" } else { "fifo" },
            self_paced: true,
            reason: "auto: no display-sync or mailbox available",
        },
    }
}

/// Split a wait into a coarse sleep plus a fine spin (`v1.25.0`, T-FP-B).
///
/// Returns `(sleep_for, then_spin)`. A sleep is only as accurate as the OS scheduler, so sleeping to
/// `deadline - SPIN_MARGIN` and busy-waiting the remainder is what actually hits a frame deadline;
/// sleeping the whole way routinely overshoots and drops the frame.
///
/// Pure, so the split is unit-testable without sleeping on the wall clock — the same reason
/// [`Pacer::advance`] is split out of [`Pacer::tick`].
#[must_use]
pub const fn sleep_spin_split(
    remaining: core::time::Duration,
    margin: core::time::Duration,
) -> (core::time::Duration, bool) {
    if remaining.is_zero() {
        return (core::time::Duration::ZERO, false);
    }
    if let Some(coarse) = remaining.checked_sub(margin)
        && !coarse.is_zero()
    {
        (coarse, true)
    } else {
        // Inside the margin there is no point sleeping at all — spin the remainder.
        (core::time::Duration::ZERO, true)
    }
}

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
    /// Whether the window is occluded (`v1.25.0`, T-FP-B) — see [`Self::set_occluded`].
    occluded: bool,
}

impl Pacer {
    /// A pacer targeting `rate` emulated frames per second (the region's frame rate).
    #[must_use]
    pub fn new(rate: f64) -> Self {
        Self {
            last: Instant::now(),
            accumulator: 0.0,
            period: 1.0 / rate,
            fps_frames: 0,
            fps_time: 0.0,
            fps: 0.0,
            occluded: false,
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

    /// The deadline for the next emulated frame, as time remaining from now (`v1.25.0`, T-FP-B).
    ///
    /// Only meaningful for a self-paced plan ([`PacingPlan::self_paced`]); under display-sync the
    /// swapchain blocks instead and this is unused. Returns `ZERO` when a frame is already due.
    #[must_use]
    pub fn time_to_next_frame(&self) -> core::time::Duration {
        let remaining = self.period - self.accumulator;
        if remaining <= 0.0 {
            core::time::Duration::ZERO
        } else {
            core::time::Duration::from_secs_f64(remaining)
        }
    }

    /// Note that the window is occluded/minimised (`v1.25.0`, T-FP-B) — the occlusion watchdog.
    ///
    /// A compositor may stop delivering vsync entirely to a hidden window, so a display-synced loop
    /// can block indefinitely and the emulator silently stops rather than running unseen. The caller
    /// treats an occluded window as self-paced regardless of the plan; this only records it for the
    /// perf panel, and resets the accumulator so becoming visible again does not replay the hidden
    /// interval as a catch-up burst.
    pub fn set_occluded(&mut self, occluded: bool) {
        if occluded != self.occluded {
            self.occluded = occluded;
            if !occluded {
                // Same rationale as `idle`: do not turn invisible time into a burst.
                self.last = Instant::now();
                self.accumulator = 0.0;
            }
        }
    }

    /// Whether the window is currently considered occluded.
    #[must_use]
    pub const fn is_occluded(&self) -> bool {
        self.occluded
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
    use super::{FPS_WINDOW, MAX_CATCHUP_FRAMES, Pacer, SPIN_MARGIN, resolve, sleep_spin_split};
    use crate::FRAME_RATE_NTSC;
    use crate::config::PacingMode as M;
    use core::time::Duration;

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

    /// A 60 Hz panel against NTSC's 60.0988 Hz is what display-sync exists for, so `Display` must
    /// be honoured there and must NOT self-pace (vsync already blocks; spinning on top of it would
    /// burn CPU for nothing).
    #[test]
    fn display_sync_honoured_when_panel_matches() {
        for hz in [59.94_f64, 60.0, 60.0988] {
            let plan = resolve(M::Display, Some(hz), FRAME_RATE_NTSC, true, true);
            assert_eq!(plan.mode, M::Display, "{hz} Hz should honour display-sync");
            assert_eq!(plan.present_mode, "fifo");
            assert!(!plan.self_paced);
        }
    }

    /// Locking to a 75/144 Hz panel would run the emulator 25%/140% fast, so `Display` must be
    /// *declined* and reported as declined — the whole reason [`super::PacingPlan::reason`] exists.
    /// An unreported downgrade is indistinguishable from a bug.
    #[test]
    fn display_sync_declined_when_panel_mismatches() {
        for hz in [50.0_f64, 75.0, 120.0, 144.0] {
            let plan = resolve(M::Display, Some(hz), FRAME_RATE_NTSC, false, true);
            assert_eq!(plan.mode, M::Wallclock, "{hz} Hz must not lock to vsync");
            assert!(plan.self_paced);
            assert!(plan.reason.contains("fell back"));
        }
        // Same when the windowing system reports no refresh rate at all.
        let plan = resolve(M::Display, None, FRAME_RATE_NTSC, false, false);
        assert_eq!(plan.mode, M::Wallclock);
    }

    /// A PAL core on a 50 Hz panel is a match; the tolerance is against the *region* rate, not a
    /// hardcoded 60.
    #[test]
    fn display_sync_is_region_relative() {
        let plan = resolve(M::Display, Some(50.0), crate::FRAME_RATE_PAL, false, false);
        assert_eq!(plan.mode, M::Display);
    }

    /// `Vrr` needs a mailbox present mode; without one it must fall back rather than claim VRR.
    #[test]
    fn vrr_requires_mailbox() {
        let ok = resolve(M::Vrr, Some(144.0), FRAME_RATE_NTSC, true, false);
        assert_eq!(ok.mode, M::Vrr);
        assert_eq!(ok.present_mode, "mailbox");
        assert!(ok.self_paced);

        let no = resolve(M::Vrr, Some(144.0), FRAME_RATE_NTSC, false, true);
        assert_eq!(no.mode, M::Wallclock);
        assert!(no.reason.contains("mailbox"));
    }

    /// `Auto` prefers vsync when the panel genuinely matches (cheapest and lowest-jitter), then
    /// mailbox, then wall-clock — and never resolves to `Auto` itself.
    #[test]
    fn auto_prefers_vsync_then_mailbox_then_wallclock() {
        let matched = resolve(M::Auto, Some(60.0), FRAME_RATE_NTSC, true, true);
        assert_eq!(matched.mode, M::Display);

        let mailbox = resolve(M::Auto, Some(144.0), FRAME_RATE_NTSC, true, true);
        assert_eq!(mailbox.mode, M::Vrr);

        let bare = resolve(M::Auto, Some(144.0), FRAME_RATE_NTSC, false, false);
        assert_eq!(bare.mode, M::Wallclock);
        assert_eq!(bare.present_mode, "fifo");

        for hz in [None, Some(60.0), Some(75.0)] {
            for mailbox in [false, true] {
                let plan = resolve(M::Auto, hz, FRAME_RATE_NTSC, mailbox, false);
                assert_ne!(plan.mode, M::Auto, "Auto must resolve to a concrete mode");
            }
        }
    }

    /// A self-paced plan must only ever ask for a present mode the surface actually offers, so the
    /// caller never has to second-guess it: `immediate` when available, `fifo` otherwise.
    #[test]
    fn wallclock_only_requests_available_modes() {
        assert_eq!(
            resolve(M::Wallclock, None, FRAME_RATE_NTSC, false, true).present_mode,
            "immediate"
        );
        assert_eq!(
            resolve(M::Wallclock, None, FRAME_RATE_NTSC, false, false).present_mode,
            "fifo"
        );
    }

    /// The sleep/spin split must never sleep past the deadline: the coarse sleep always stops at
    /// least `margin` short, which is the entire point (an OS sleep overshoots by ~1-2 ms).
    #[test]
    fn sleep_spin_split_reserves_the_margin() {
        let (sleep, spin) = sleep_spin_split(Duration::from_millis(16), SPIN_MARGIN);
        assert_eq!(
            sleep,
            Duration::from_millis(16)
                .checked_sub(SPIN_MARGIN)
                .expect("16ms exceeds the margin")
        );
        assert!(spin);
        assert!(sleep + SPIN_MARGIN <= Duration::from_millis(16));
    }

    /// Inside the margin there is nothing worth sleeping for — spin the remainder instead of
    /// handing the scheduler a sub-granularity sleep it will overshoot.
    #[test]
    fn sleep_spin_split_spins_inside_the_margin() {
        for us in [1_u64, 500, 1999] {
            let (sleep, spin) = sleep_spin_split(Duration::from_micros(us), SPIN_MARGIN);
            assert_eq!(sleep, Duration::ZERO, "{us}us should not sleep");
            assert!(spin);
        }
    }

    /// A frame already due must neither sleep nor spin.
    #[test]
    fn sleep_spin_split_no_wait_when_due() {
        assert_eq!(
            sleep_spin_split(Duration::ZERO, SPIN_MARGIN),
            (Duration::ZERO, false)
        );
    }

    /// `time_to_next_frame` shrinks as the accumulator fills and reads zero once a frame is owed —
    /// the property the self-paced loop's deadline depends on.
    #[test]
    fn time_to_next_frame_tracks_the_accumulator() {
        let mut pacer = Pacer::new(FRAME_RATE_NTSC);
        let full = pacer.time_to_next_frame();
        assert!(full > Duration::from_millis(15) && full < Duration::from_millis(18));

        // Half a period in, roughly half a period is left.
        pacer.advance(0.5 / FRAME_RATE_NTSC);
        let half = pacer.time_to_next_frame();
        assert!(half < full && half > Duration::from_millis(7));

        // A whole period earns a frame and leaves ~nothing owed.
        let mut pacer = Pacer::new(FRAME_RATE_NTSC);
        assert_eq!(pacer.advance(1.0 / FRAME_RATE_NTSC), 1);
        assert!(pacer.time_to_next_frame() > Duration::from_millis(15));
    }

    /// Becoming visible again must not replay the hidden interval as a catch-up burst — the same
    /// guarantee `idle()` gives around a pause.
    #[test]
    fn unoccluding_does_not_burst() {
        let mut pacer = Pacer::new(FRAME_RATE_NTSC);
        assert!(!pacer.is_occluded());
        pacer.set_occluded(true);
        assert!(pacer.is_occluded());
        // Time passes unseen.
        pacer.advance(2.0);
        pacer.set_occluded(false);
        assert!(!pacer.is_occluded());
        // The accumulator was dropped, so the next real tick earns at most its own frame.
        assert!(pacer.advance(1.0 / FRAME_RATE_NTSC) <= 1);
    }
}
