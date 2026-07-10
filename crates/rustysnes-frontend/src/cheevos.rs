//! Native `RetroAchievements` integration (`v0.9.0 "Community"`, T-82-003).
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

use std::cell::RefCell;
use std::rc::Rc;

use rustysnes_cheevos::{RaClient, RaEvent, RaUser};
use rustysnes_core::System;

/// The shared slot an in-flight login's completion callback writes into.
type LoginResult = Rc<RefCell<Option<Result<(), String>>>>;

/// Native `RetroAchievements` session state: the `rc_client` handle (created lazily on first login
/// attempt), the logged-in user (if any), and any in-flight login's pending/error state.
#[derive(Default)]
pub struct CheevosState {
    client: Option<RaClient>,
    user: Option<RaUser>,
    login_pending: bool,
    login_error: Option<String>,
    login_result: LoginResult,
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

        client
            .take_events()
            .into_iter()
            .filter_map(|ev| match ev {
                RaEvent::AchievementTriggered { title, points, .. } => {
                    Some(format!("Achievement unlocked: {title} ({points} pts)"))
                }
                _ => None,
            })
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
