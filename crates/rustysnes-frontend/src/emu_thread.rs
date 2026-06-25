//! The dedicated emulation thread (native, behind the default-on `emu-thread` feature).
//!
//! Single-player frame production runs off the winit event-loop thread so UI/render stalls never
//! disturb emulation cadence. The thread owns the `Arc<Mutex<EmuCore>>` handle (shared with the
//! present path) + a lock-free `SharedInput`; the winit thread only does UI + present. This is
//! the RustyNES `emu_thread` pattern, ported verbatim in shape.
//!
//! The thread NEVER does rate control inside the core — it produces frames at the region cadence
//! and the present/audio paths absorb the slack (the determinism contract: the core emits the
//! same AV regardless of pacing).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::emu::EmuCore;

/// Lock-free shared input the winit thread writes (late-latched) and the emu thread reads each
/// frame. Two `AtomicU32` slots hold the packed P1/P2 button bitfields (the low 16 bits used).
#[derive(Debug, Default)]
pub struct SharedInput {
    /// Packed P1 buttons (low 16 bits = the [`crate::input::Buttons`] word).
    pub p1: AtomicU32,
    /// Packed P2 buttons.
    pub p2: AtomicU32,
}

/// Handle to the running emulation thread. Dropping it signals the thread to stop and joins it.
pub struct EmuThread {
    handle: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl EmuThread {
    /// Spawn the emulation thread, producing frames at `frame_rate` Hz. `core` is the shared
    /// emulator (locked briefly per frame to step + by the present path to read the
    /// framebuffer); `input` is the lock-free latch.
    #[must_use]
    pub fn spawn(core: Arc<Mutex<EmuCore>>, input: Arc<SharedInput>, frame_rate: f64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let frame_dur = Duration::from_secs_f64(1.0 / frame_rate.max(1.0));

        // Best-effort priority elevation is a TODO (the RustyNES `libc` SCHED_RR nicety); the
        // skeleton just runs at the default priority.
        let handle = std::thread::Builder::new()
            .name("emu-thread".into())
            .spawn(move || {
                let mut next = Instant::now();
                while !stop_thread.load(Ordering::Relaxed) {
                    let p1 = input.p1.load(Ordering::Acquire) as u16;
                    let p2 = input.p2.load(Ordering::Acquire) as u16;
                    {
                        // Brief lock: latch input + step exactly one frame, then drop.
                        let mut emu = match core.lock() {
                            Ok(g) => g,
                            Err(p) => p.into_inner(), // a poisoned lock shouldn't kill audio/UI
                        };
                        emu.set_pad(0, crate::input::Buttons(p1));
                        emu.set_pad(1, crate::input::Buttons(p2));
                        emu.run_frame();
                    }
                    // Pace to the region cadence (the frontend's authoritative wall clock).
                    next += frame_dur;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        // Fell behind (a long lock/UI stall); resync to avoid a burst catch-up.
                        next = now;
                    }
                }
            })
            .expect("spawn emu-thread");

        Self {
            handle: Some(handle),
            stop,
        }
    }
}

impl Drop for EmuThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Region;

    #[test]
    fn thread_spawns_steps_and_joins() {
        let core = Arc::new(Mutex::new(EmuCore::new(0, Region::Ntsc)));
        let input = Arc::new(SharedInput::default());
        // A high frame rate so a few frames run before we drop (and join) the thread.
        let t = EmuThread::spawn(Arc::clone(&core), input, 10_000.0);
        std::thread::sleep(Duration::from_millis(20));
        drop(t); // joins
        // The core is still lockable (no poison) and didn't panic.
        assert!(core.lock().is_ok());
    }
}
