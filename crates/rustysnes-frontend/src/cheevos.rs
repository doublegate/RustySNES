//! Native `RetroAchievements` integration (`v0.8.0 "Community"`, T-82-003).
//!
//! Wraps [`rustysnes_cheevos::RaClient`] with frontend-owned login/session state, a per-frame
//! [`CheevosState::do_frame`] hook (reads WRAM through [`rustysnes_core::Bus::peek_wram`], never
//! mutates it), and unlock-toast buffering drained once per frame by [`CheevosState::poll`].
//! Native-only, mirroring `netplay.rs`: `rustysnes-cheevos` itself is
//! `#![cfg(not(target_arch = "wasm32"))]` (the vendored `rcheevos` C library needs a C toolchain
//! + `std`, and this pass has no browser-side HTTP worker model for RA server calls).
//!
//! Login is asynchronous (`RaClient::begin_login_password`'s completion fires from inside
//! [`RaClient::poll_http_completions`], on whatever thread calls it — here, the render thread).
//! The completion closure can't hold `&mut CheevosState` directly (it must be `'static`), so it
//! writes into a shared `Rc<RefCell<Option<Result<...>>>>` slot instead; [`CheevosState::poll`]
//! (called once per real frame from `app.rs`, same cadence as `NetplayState::drive`) takes the
//! slot's contents and updates `user`/`login_error` from the main thread.
//!
//! [`Self::load_game`]/[`Self::unload_game`] (`v1.11.0 "Podium"`) are called from `app.rs`'s
//! `MenuAction::OpenRom`/`CloseRom` handlers — the two points a ROM's identity actually changes.
//! **Known scope note**: a ROM loaded via the CLI at startup, followed by a *later* login through
//! the Tools window, is not retroactively announced to `rc_client` (login happens after `Active`
//! — and this ROM-change wiring — already ran once at startup). The common path (launch, log in,
//! then open a ROM via the File menu) is unaffected; re-opening the same ROM after logging in
//! works around the CLI case too. Not silently dropped — the fix needs [`Self::poll`] to also
//! reach the currently-loaded ROM's bytes on a successful login, which needs an `EmuCore`
//! reference threaded through a path that doesn't have one today; deferred to a follow-up.

use std::cell::RefCell;
use std::rc::Rc;

use rustysnes_cheevos::{RaClient, RaEvent, RaUser};
use rustysnes_core::System;

/// The shared slot an in-flight login's completion callback writes into.
type LoginResult = Rc<RefCell<Option<Result<(), String>>>>;
/// The shared slot an in-flight [`CheevosState::load_game`] completion callback writes into —
/// same shape/pattern as [`LoginResult`], for the same reason (the completion closure must be
/// `'static`, so it can't hold `&mut CheevosState` directly).
type GameLoadResult = Rc<RefCell<Option<Result<(), String>>>>;

/// Native `RetroAchievements` session state: the `rc_client` handle (created lazily on first login
/// attempt), the logged-in user (if any), and any in-flight login's pending/error state.
#[derive(Default)]
pub struct CheevosState {
    client: Option<RaClient>,
    user: Option<RaUser>,
    login_pending: bool,
    login_error: Option<String>,
    login_result: LoginResult,
    game_load_result: GameLoadResult,
}

impl CheevosState {
    /// Whether a user is currently logged in.
    #[must_use]
    pub const fn is_logged_in(&self) -> bool {
        self.user.is_some()
    }

    /// Whether a login attempt is currently in flight.
    #[must_use]
    pub const fn login_pending(&self) -> bool {
        self.login_pending
    }

    /// The logged-in user's display name, if any.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.display_name.as_str())
    }

    /// The most recent login failure message, if any.
    #[must_use]
    pub fn login_error(&self) -> Option<&str> {
        self.login_error.as_deref()
    }

    /// Begin an asynchronous password login (creates the `rc_client` on first use).
    pub fn login(&mut self, username: &str, password: &str) {
        let client = self.client.get_or_insert_with(RaClient::new);
        self.login_pending = true;
        self.login_error = None;
        let slot = Rc::clone(&self.login_result);
        client.begin_login_password(username, password, move |outcome| {
            *slot.borrow_mut() = Some(outcome);
        });
    }

    /// Log out (a no-op if no client has been created / no user is logged in).
    pub fn logout(&mut self) {
        if let Some(client) = &mut self.client {
            client.logout();
        }
        self.user = None;
    }

    /// Identify and load `rom`'s achievement/leaderboard set (`v1.11.0 "Podium"`).
    ///
    /// A no-op unless a user is logged in — this asynchronously hashes/uploads `rom` and fetches
    /// its achievement set from the RA servers, which is pointless work with nobody to track
    /// progress for. **Before this method existed, no code path ever called
    /// [`RaClient::begin_load_game`] at all** — [`Self::do_frame`] evaluated achievement logic
    /// every frame, but with no game ever identified/loaded into `rc_client`, there was no
    /// achievement set loaded to evaluate against, so achievements could never actually trigger
    /// despite every other piece (login, the per-frame hook, the unlock-toast plumbing) being
    /// wired up. Call once per successful ROM load (`app.rs`'s `MenuAction::OpenRom` / the CLI
    /// startup path); see [`Self::unload_game`] for the matching close.
    pub fn load_game(&mut self, rom: &[u8]) {
        let Some(client) = &mut self.client else {
            return;
        };
        if self.user.is_none() {
            return;
        }
        // Found in review (#92): re-allocate the slot itself (not just clear its contents)
        // before starting a new load. A ROM swapped again before the previous load's completion
        // fires would otherwise let that stale callback write into the SAME slot this new load
        // is about to poll from, surfacing a toast for the wrong game. The stale callback still
        // holds a clone of the OLD `Rc` and writes to it harmlessly; `self.game_load_result` now
        // points to a fresh one, so `poll()` never looks at the stale write again.
        self.game_load_result = Rc::new(RefCell::new(None));
        let slot = Rc::clone(&self.game_load_result);
        client.begin_load_game(rom, move |outcome| {
            *slot.borrow_mut() = Some(outcome);
        });
    }

    /// Unload the current game (a no-op if no client has been created). Call on `MenuAction::CloseRom`
    /// and before loading a new ROM over an already-loaded one, so `rc_client` never evaluates a
    /// stale achievement set against the new cart's memory layout.
    pub fn unload_game(&mut self) {
        if let Some(client) = &mut self.client {
            client.unload_game();
        }
        // Found in review (#92): same stale-slot concern as `load_game` -- discard any in-flight
        // load's eventual completion so it can't surface a toast for a game that was just closed.
        self.game_load_result = Rc::new(RefCell::new(None));
    }

    /// Drain HTTP completions (resolving any in-flight login), and translate newly-fired
    /// [`RaEvent::AchievementTriggered`] events into human-readable toast strings. Call once per
    /// real frame; a no-op if no client has been created yet (no login attempted this session).
    pub fn poll(&mut self) -> Vec<String> {
        let Some(client) = &mut self.client else {
            return Vec::new();
        };
        client.poll_http_completions();

        if let Some(outcome) = self.login_result.borrow_mut().take() {
            self.login_pending = false;
            match outcome {
                Ok(()) => self.user = client.user_info(),
                Err(e) => self.login_error = Some(e),
            }
        }

        let game_load_toast =
            self.game_load_result
                .borrow_mut()
                .take()
                .map(|outcome| match outcome {
                    Ok(()) => {
                        "RetroAchievements: game identified, achievement set loaded".to_string()
                    }
                    Err(e) => format!("RetroAchievements: game identification failed: {e}"),
                });

        game_load_toast
            .into_iter()
            .chain(client.take_events().into_iter().filter_map(|ev| match ev {
                RaEvent::AchievementTriggered { title, points, .. } => {
                    Some(format!("Achievement unlocked: {title} ({points} pts)"))
                }
                _ => None,
            }))
            .collect()
    }

    /// Run one frame of achievement logic against `sys`'s WRAM. A no-op if no client has been
    /// created (login never attempted) — matching rcheevos' own behavior with no game loaded,
    /// this simply evaluates nothing rather than erroring.
    pub fn do_frame(&mut self, sys: &System) {
        let Some(client) = &mut self.client else {
            return;
        };
        client.do_frame(&mut |addr| sys.bus.peek_wram(addr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_game_is_a_no_op_before_any_login_attempt() {
        // No `client` exists yet (matching `do_frame`'s own "login never attempted" no-op
        // posture) -- must not panic, and must not fabricate a pending completion.
        let mut state = CheevosState::default();
        state.load_game(b"not a real rom");
        assert!(state.game_load_result.borrow().is_none());
    }

    #[test]
    fn unload_game_is_a_no_op_before_any_login_attempt() {
        let mut state = CheevosState::default();
        state.unload_game(); // must not panic
    }

    fn fake_user() -> RaUser {
        RaUser {
            display_name: "Tester".into(),
            username: "tester".into(),
            token: "tok".into(),
            score: 0,
            score_softcore: 0,
        }
    }

    /// Found in review (#92): a stale in-flight `load_game` completion arriving after a second
    /// `load_game`/`unload_game` call must not surface a toast for the wrong (already
    /// superseded) game. Proven directly at the mechanism level (the slot is re-allocated, so a
    /// write to the OLD `Rc` clone a stale callback would hold is never observed by `poll`)
    /// rather than by racing `RaClient::begin_load_game`'s real async network call.
    #[test]
    fn load_game_reallocates_the_result_slot_to_discard_a_stale_in_flight_callback() {
        let mut state = CheevosState {
            client: Some(RaClient::new()),
            user: Some(fake_user()),
            ..CheevosState::default()
        };
        let stale_slot = Rc::clone(&state.game_load_result);
        state.load_game(b"a second, superseding rom load");
        assert!(
            !Rc::ptr_eq(&stale_slot, &state.game_load_result),
            "load_game must point game_load_result at a fresh Rc, not reuse the old one"
        );
        // Simulate the first load's completion arriving late, after the second load already ran.
        *stale_slot.borrow_mut() = Some(Err("stale failure for the first ROM".to_string()));
        let toasts = state.poll();
        assert!(
            toasts.iter().all(|t| !t.contains("stale failure")),
            "a write to the discarded slot must not surface as a toast: {toasts:?}"
        );
    }

    #[test]
    fn unload_game_discards_a_stale_in_flight_load_result() {
        let mut state = CheevosState {
            client: Some(RaClient::new()),
            ..CheevosState::default()
        };
        let stale_slot = Rc::clone(&state.game_load_result);
        state.unload_game();
        assert!(
            !Rc::ptr_eq(&stale_slot, &state.game_load_result),
            "unload_game must point game_load_result at a fresh Rc, not reuse the old one"
        );
        *stale_slot.borrow_mut() = Some(Err("stale failure after close".to_string()));
        assert!(
            state.poll().is_empty(),
            "a write to the discarded slot must not surface as a toast after the ROM was closed"
        );
    }

    #[test]
    fn load_game_is_a_no_op_with_a_client_but_no_logged_in_user() {
        // `v1.11.0 "Podium"`: `load_game` must not fire `begin_load_game` (a real network call)
        // for a client that exists (e.g. a login attempt is in flight or previously failed) but
        // has no confirmed logged-in user -- there is nobody to track achievement progress for.
        let mut state = CheevosState {
            client: Some(RaClient::new()),
            ..CheevosState::default()
        };
        state.load_game(b"not a real rom");
        assert!(
            state.game_load_result.borrow().is_none(),
            "load_game must not begin a load without a logged-in user"
        );
    }

    #[test]
    fn poll_surfaces_a_game_load_success_toast() {
        // Exercises `poll`'s drain/format logic directly (not `RaClient::begin_load_game`'s real
        // async network path, which needs a live login/server round trip) by injecting a result
        // into the same shared slot the completion callback would write into.
        let mut state = CheevosState {
            client: Some(RaClient::new()),
            ..CheevosState::default()
        };
        *state.game_load_result.borrow_mut() = Some(Ok(()));
        let toasts = state.poll();
        assert_eq!(
            toasts,
            vec!["RetroAchievements: game identified, achievement set loaded"]
        );
        // Drained -- a second poll must not re-surface the same toast.
        assert!(state.poll().is_empty());
    }

    #[test]
    fn poll_surfaces_a_game_load_failure_toast() {
        let mut state = CheevosState {
            client: Some(RaClient::new()),
            ..CheevosState::default()
        };
        *state.game_load_result.borrow_mut() = Some(Err("bad hash".to_string()));
        let toasts = state.poll();
        assert_eq!(
            toasts,
            vec!["RetroAchievements: game identification failed: bad hash"]
        );
    }
}
