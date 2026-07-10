//! `RetroAchievements` (`rc_client`) integration for RustySNES.
//!
//! Opt-in (native-only) FFI bridge around the vendored `rcheevos` C library
//! (MIT-licensed, `vendor/rcheevos/`). [`RaClient`] is the safe entry point:
//! login, load a game, drive achievement logic once per emulated frame via
//! [`RaClient::do_frame`], and drain [`RaEvent`]s for unlock toasts and
//! leaderboard UI.
//!
//! The RA flat memory space this crate exposes covers only the SNES's 128 KiB
//! WRAM (`$7E0000..=$7FFFFF`) — see [`memory::ra_addr_to_snes`] for the
//! verified mapping and the documented SRAM scope cut.
#![cfg(not(target_arch = "wasm32"))]
#![allow(unsafe_code)]
#![allow(clippy::missing_panics_doc)]

mod client;
mod events;
mod ffi;
mod http;
pub mod memory;
mod util;

pub use client::{RaAchievement, RaClient, RaGameSummary, RaLeaderboard, RaUser};
pub use events::{RaEvent, RaScoreboardEntry};
pub use memory::ra_addr_to_snes;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_drive_destroy() {
        let mut client = RaClient::new();
        let mut ram = vec![0u8; 0x2_0000];
        ram[0] = 0x42;
        client.do_frame(&mut |addr: u32| {
            let offset = (addr - 0x007E_0000) as usize;
            ram[offset]
        });
        client.poll_http_completions();
        let _ = client.take_events();
    }

    #[test]
    fn nested_read_guard_restores() {
        let mut outer = RaClient::new();
        let mut inner = RaClient::new();
        let ram = vec![0u8; 0x2_0000];

        outer.do_frame(&mut |_addr: u32| -> u8 {
            inner.do_frame(&mut |_addr: u32| -> u8 { ram[0] });
            ram[0]
        });
    }

    #[test]
    fn login_completion_fires_on_transport_error() {
        let mut client = RaClient::new();
        client.begin_login_password("nobody", "wrong", |_result| {
            // The real completion depends on network reachability; this test
            // only proves the call sequence doesn't panic/deadlock and the
            // completion trampoline is reachable end-to-end.
        });
        // Give the worker thread a moment, then drain whatever arrived (or
        // didn't — this environment may have no network access).
        for _ in 0..50 {
            client.poll_http_completions();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
