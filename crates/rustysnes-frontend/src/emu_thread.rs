//! The dedicated emulation thread (native, behind the default-off `emu-thread` feature).
//!
//! `v1.1.0` feature-parity pass: single-player frame production runs off the winit event-loop
//! thread so UI/render stalls never disturb emulation cadence. The thread owns the
//! `Arc<Mutex<EmuCore>>` handle (shared with the present path), a lock-free [`SharedInput`], an
//! [`EmuControl`] lifecycle block (pause / ROM-loaded gate), a [`crate::present_buffer::PresentBuffer`]
//! lock-free framebuffer handoff, and — closing this build's previously-documented biggest gap —
//! a [`crate::audio::AudioProducer`] so the threaded build finally produces sound. This is the
//! RustyNES `emu_thread` pattern, SNES-adapted (WALLCLOCK regime only — RustySNES has no
//! DISPLAY/VRR present-mode-driven regime infrastructure the way RustyNES does, so that split
//! isn't ported; see the module-level "Known remaining gaps" note below).
//!
//! The thread NEVER does rate control inside the core — it produces frames at the region cadence
//! (scaled by the live speed multiplier) and the present/audio paths absorb the slack (the
//! determinism contract: the core emits the same AV regardless of pacing).
//!
//! ## Known remaining gaps (honestly tracked, not silently claimed as done)
//!
//! This pass closes the audio-output gap and gives the thread a real pause/ROM-loaded lifecycle,
//! but does NOT yet port: cheats/watchpoints/breakpoints/port2-peripheral/voice-mute re-sync,
//! run-ahead, rewind recording, TAS movie apply/record, Lua script pump, netplay-aware pause, or
//! `RetroAchievements` per-frame drive. Each of those needs a genuinely new shared-mutable-state
//! design (the lists/buffers they touch are currently plain `Active` fields the UI edits
//! directly) rather than a mechanical port, and is left as a scoped, documented follow-up rather
//! than rushed. `crates/rustysnes-frontend/Cargo.toml`'s `emu-thread` feature comment tracks the
//! exact same list.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use winit::event_loop::EventLoopProxy;

use crate::app::AppEvent;
use crate::audio::AudioProducer;
use crate::emu::EmuCore;
use crate::pacing::Pacer;
use crate::present_buffer::PresentBuffer;

/// Park interval while the thread is idle (no ROM, or user-paused). Short enough that a resume
/// is near-immediate (also nudged directly via [`EmuThread::unpark`]), long enough not to spin a
/// core.
const IDLE_PARK: Duration = Duration::from_millis(8);

/// Sleep slice between cadence checks while actively producing. 1ms granularity is well within
/// what a 60/50Hz frame period can absorb; RustyNES's own hybrid sleep-then-spin precision
/// (`SPIN_MARGIN`) is a performance refinement not ported this pass (see module doc).
const TICK_SLEEP: Duration = Duration::from_millis(1);

/// Lock-free shared input the winit thread writes (late-latched) and the emu thread reads each
/// frame. Two `AtomicU32` slots hold the packed P1/P2 button bitfields (the low 16 bits used).
#[derive(Debug, Default)]
pub struct SharedInput {
    /// Packed P1 buttons (low 16 bits = the [`crate::input::Buttons`] word).
    pub p1: AtomicU32,
    /// Packed P2 buttons.
    pub p2: AtomicU32,
}

/// Lifecycle control shared between the winit thread (writer) and the emulation thread (reader).
///
/// `v1.1.0`: WALLCLOCK-only (no `regime`/netplay-pause/fast-forward/frame-advance fields — none
/// of those have an existing RustySNES UI hook to drive them yet; see the module doc's "Known
/// remaining gaps").
#[derive(Debug)]
pub struct EmuControl {
    /// Set on exit; the thread observes it and returns.
    stop: AtomicBool,
    /// Set while the user paused emulation from the UX shell (Emulation -> Pause, or the `Space`
    /// hotkey). The thread idles while set.
    user_paused: AtomicBool,
    /// `true` once a ROM is loaded (the thread idles until then).
    has_rom: AtomicBool,
    /// Current speed multiplier (`f32::to_bits()`, `1.0` = normal) — re-synced unconditionally
    /// once per present, same "just re-sync" pattern the synchronous path's cheats/watchpoints
    /// sync already uses. Scales the thread's own cadence `Pacer` the same way
    /// `MenuAction::SetSpeed` already scales the synchronous path's `Active::pacer`.
    speed_bits: AtomicU32,
    /// The base (speed-`1.0`) region frame rate in nanoseconds-per-frame, fixed at spawn (region
    /// doesn't change for a live session — `MenuAction::SetRegion`'s own status message says
    /// "restart to apply").
    base_frame_nanos: AtomicU64,
}

impl EmuControl {
    /// Build the control block in the initial idle (no ROM, not paused) state.
    #[must_use]
    pub fn new(frame_rate: f64) -> Arc<Self> {
        Arc::new(Self {
            stop: AtomicBool::new(false),
            user_paused: AtomicBool::new(false),
            has_rom: AtomicBool::new(false),
            speed_bits: AtomicU32::new(1.0f32.to_bits()),
            base_frame_nanos: AtomicU64::new(dur_nanos(Duration::from_secs_f64(
                1.0 / frame_rate.max(1.0),
            ))),
        })
    }

    /// Mark a ROM loaded (or cleared) so the thread starts (or idles). Re-synced unconditionally
    /// once per present from `EmuCore::rom_loaded()`.
    pub fn set_has_rom(&self, on: bool) {
        self.has_rom.store(on, Ordering::Release);
    }

    /// Pause (or resume) emulation from the UX shell. Re-synced unconditionally once per present
    /// from `ShellState::paused`.
    pub fn set_user_paused(&self, on: bool) {
        self.user_paused.store(on, Ordering::Release);
    }

    /// Update the live speed multiplier (`MenuAction::SetSpeed`'s effect, mirrored onto the
    /// thread). Re-synced unconditionally once per present.
    pub fn set_speed(&self, speed: f32) {
        self.speed_bits.store(speed.to_bits(), Ordering::Release);
    }

    fn speed(&self) -> f32 {
        f32::from_bits(self.speed_bits.load(Ordering::Acquire))
    }

    fn frame_duration(&self) -> Duration {
        let base = self.base_frame_nanos.load(Ordering::Acquire);
        // `base` is a real nanosecond count (always non-negative) and `speed` is clamped to a
        // sane positive range above, so the division result is always non-negative — no sign to
        // lose in practice, but the cast is written out explicitly for clippy's benefit.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let scaled = (base as f64 / f64::from(self.speed().max(1e-3))) as u64;
        Duration::from_nanos(scaled.max(1))
    }
}

/// Handle to the running emulation thread. Dropping it signals the thread to stop and joins it.
pub struct EmuThread {
    handle: Option<JoinHandle<()>>,
    control: Arc<EmuControl>,
}

impl EmuThread {
    /// Spawn the emulation thread. `audio` is the `Send` producer half made from the cpal output
    /// (`AudioOutput::make_producer`); `None` when no audio device was available (the emulator
    /// still runs, silently, exactly like the synchronous path's own `active.audio: None` case).
    /// `present` is the lock-free framebuffer handoff the loop publishes each produced frame into
    /// so the winit present path never blocks on the emu mutex to copy it.
    ///
    /// # Panics
    /// Panics only if the OS refuses to spawn a new thread (`std::thread::Builder::spawn`
    /// failing) — an environment-level resource exhaustion this project has no graceful fallback
    /// for, matching every other `expect`-on-thread-spawn in this codebase.
    #[must_use]
    pub fn spawn(
        core: Arc<Mutex<EmuCore>>,
        input: Arc<SharedInput>,
        audio: Option<AudioProducer>,
        proxy: EventLoopProxy<AppEvent>,
        control: Arc<EmuControl>,
        present: Arc<PresentBuffer>,
    ) -> Self {
        let control_thread = Arc::clone(&control);
        let handle = std::thread::Builder::new()
            .name("emu-thread".into())
            .spawn(move || {
                run_loop(
                    &core,
                    input.as_ref(),
                    audio,
                    &proxy,
                    &control_thread,
                    &present,
                );
            })
            .expect("spawn emu-thread");

        Self {
            handle: Some(handle),
            control,
        }
    }

    /// The control block (pause / ROM / speed writes).
    #[must_use]
    pub const fn control(&self) -> &Arc<EmuControl> {
        &self.control
    }

    /// Wake the emulation thread out of its idle park immediately (called on resume) rather than
    /// waiting for the up-to-`IDLE_PARK` timeout to notice the just-cleared pause flag.
    pub fn unpark(&self) {
        if let Some(h) = self.handle.as_ref() {
            h.thread().unpark();
        }
    }
}

impl Drop for EmuThread {
    fn drop(&mut self) {
        self.control.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            h.thread().unpark();
            let _ = h.join();
        }
    }
}

/// A [`Duration`] as `u64` nanoseconds, saturating (a frame duration is always far below
/// `u64::MAX` ns, so this never clamps in practice).
fn dur_nanos(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

/// The emulation thread's main loop.
fn run_loop(
    core: &Arc<Mutex<EmuCore>>,
    input: &SharedInput,
    mut audio: Option<AudioProducer>,
    proxy: &EventLoopProxy<AppEvent>,
    control: &EmuControl,
    present: &PresentBuffer,
) {
    elevate_thread_priority();
    let mut pacer = Pacer::new(1.0 / control.frame_duration().as_secs_f64());
    loop {
        if control.stop.load(Ordering::Acquire) {
            return;
        }
        let idle =
            !control.has_rom.load(Ordering::Acquire) || control.user_paused.load(Ordering::Acquire);
        if idle {
            pacer.idle(); // avoid a catch-up burst when resuming (Pacer::idle's own purpose).
            std::thread::park_timeout(IDLE_PARK);
            continue;
        }

        pacer.set_rate(1.0 / control.frame_duration().as_secs_f64());
        std::thread::sleep(TICK_SLEEP);
        let frames = pacer.tick();
        let mut produced_any = false;
        for _ in 0..frames {
            if drive_one(core, input, audio.as_mut(), control, present) {
                produced_any = true;
            } else {
                break; // paused/ROM-closed mid-burst (the TOCTOU close) — stop this burst.
            }
        }
        if produced_any && proxy.send_event(AppEvent::EmuFrame).is_err() {
            return; // event loop gone — shutting down.
        }
    }
}

/// Latch input + produce exactly one frame + push its audio + publish its framebuffer. Returns
/// `false` if it bailed because the user paused (or closed the ROM) between the loop-top check
/// and acquiring the lock — the TOCTOU close, mirroring RustyNES's own `drive_one`.
fn drive_one(
    core: &Arc<Mutex<EmuCore>>,
    input: &SharedInput,
    audio: Option<&mut AudioProducer>,
    control: &EmuControl,
    present: &PresentBuffer,
) -> bool {
    #[allow(clippy::cast_possible_truncation)]
    let p1 = input.p1.load(Ordering::Acquire) as u16;
    #[allow(clippy::cast_possible_truncation)]
    let p2 = input.p2.load(Ordering::Acquire) as u16;
    let mut emu = match core.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(), // a poisoned lock shouldn't kill audio/UI
    };
    if control.user_paused.load(Ordering::Acquire) || !emu.rom_loaded() {
        return false;
    }
    emu.set_pad(0, crate::input::Buttons(p1));
    emu.set_pad(1, crate::input::Buttons(p2));
    emu.run_frame();
    if let Some(a) = audio {
        a.push(emu.audio(), control.speed());
    }
    present.publish(emu.framebuffer());
    true
}

/// Best-effort emu-thread priority elevation (Linux). Reduces the occasional OS descheduling
/// that inflates produce-cost/presented-jitter tails. Ported near-verbatim from RustyNES (the
/// same author's own prior art for this exact mechanism) — no SNES-specific content.
///
/// Strategy, in order, all per-THREAD (never the process) and degrading SILENTLY when the
/// privilege/rlimit is absent:
/// 1. `SCHED_RR` at a LOW real-time priority — preempts normal (`SCHED_OTHER`) tasks so the emu
///    thread runs on time, while a low priority keeps it BELOW the audio callback thread. Needs
///    `RLIMIT_RTPRIO`.
/// 2. Fall back to a small negative `nice` — needs `RLIMIT_NICE`.
/// 3. `PR_SET_TIMERSLACK` to 1 microsecond (always permitted for one's own thread).
///
/// When none of the elevations are permitted the thread runs at default priority exactly as
/// before. macOS / Windows keep a documented no-op for now.
///
/// This is the only `unsafe` in `rustysnes-frontend` (workspace `unsafe_code = "warn"`): three
/// libc scheduler syscalls on the calling thread, each with a `// SAFETY:` justification below.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn elevate_thread_priority() {
    // SAFETY: all three are standard libc thread/scheduler syscalls on the CALLING thread
    // (pid/who 0), with valid arguments; they only ever return an error code we inspect, never
    // write through our pointers beyond the `sched_param` we own here.
    let rr = unsafe {
        // Low RR priority: above all SCHED_OTHER, below typical audio RT.
        const EMU_RT_PRIORITY: libc::c_int = 5;
        let param = libc::sched_param {
            sched_priority: EMU_RT_PRIORITY,
        };
        libc::sched_setscheduler(0, libc::SCHED_RR, &raw const param) == 0
    };
    if rr {
        eprintln!("rustysnes: emu thread elevated to SCHED_RR priority 5.");
    } else {
        // SAFETY: see above — `setpriority` on the calling thread, addressed by TID (not `0`,
        // which would target the whole process's nice value under `PRIO_PROCESS`).
        let niced = unsafe {
            // A real TID is always a small positive value (Linux caps `pid_max` far below
            // `u32::MAX`), so this cast never truncates or flips sign in practice.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let tid = libc::syscall(libc::SYS_gettid) as libc::id_t;
            libc::setpriority(libc::PRIO_PROCESS, tid, -10) == 0
        };
        if niced {
            eprintln!("rustysnes: emu thread niced to -10 (no RT rtprio limit).");
        } else {
            eprintln!(
                "rustysnes: emu thread at default priority — for lower-latency scheduling, \
                 grant this process RLIMIT_RTPRIO or RLIMIT_NICE."
            );
        }
    }
    // SAFETY: `prctl(PR_SET_TIMERSLACK, ...)` sets this thread's timer slack (always permitted
    // for one's own thread); extra args are ignored.
    unsafe {
        libc::prctl(libc::PR_SET_TIMERSLACK, 1_000 as libc::c_ulong, 0, 0, 0);
    }
}

/// Non-Linux best-effort priority elevation: a documented no-op for now. Rust's `std` sleeps
/// already use high-resolution timers, so the pacer is precise regardless.
#[cfg(not(target_os = "linux"))]
const fn elevate_thread_priority() {}

#[cfg(test)]
mod tests {
    use super::*;

    // No `EmuThread::spawn` smoke test here: it needs a real `winit::event_loop::EventLoopProxy`,
    // and winit forbids constructing an `EventLoop` off the main thread — `cargo test` runs each
    // test on its own worker thread, so building one here panics unconditionally (a real, tested
    // platform constraint, not a gap in this module). RustyNES's own equivalent test module has
    // the identical scope cut for the identical reason — it tests `SharedInput`/`EmuControl`
    // directly (below), never a full thread spawn. `App`'s own construction path (`on_gfx_ready`)
    // is what actually exercises `EmuThread::spawn` end-to-end, on the real winit main thread.

    #[test]
    fn control_lifecycle_round_trips() {
        let control = EmuControl::new(60.0);
        assert!(!control.has_rom.load(Ordering::Relaxed));
        control.set_has_rom(true);
        assert!(control.has_rom.load(Ordering::Relaxed));
        assert!(!control.user_paused.load(Ordering::Relaxed));
        control.set_user_paused(true);
        assert!(control.user_paused.load(Ordering::Relaxed));
        control.set_speed(2.0);
        assert!((control.speed() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn frame_duration_scales_inversely_with_speed() {
        let control = EmuControl::new(60.0);
        let base = control.frame_duration();
        control.set_speed(2.0);
        let doubled_speed = control.frame_duration();
        // Twice the speed -> half the per-frame duration.
        let ratio = base.as_secs_f64() / doubled_speed.as_secs_f64();
        assert!((ratio - 2.0).abs() < 0.01, "ratio was {ratio}");
    }

    #[test]
    fn shared_input_round_trips() {
        let input = SharedInput::default();
        input.p1.store(0x1234, Ordering::Release);
        input.p2.store(0x5678, Ordering::Release);
        assert_eq!(input.p1.load(Ordering::Acquire), 0x1234);
        assert_eq!(input.p2.load(Ordering::Acquire), 0x5678);
    }
}
