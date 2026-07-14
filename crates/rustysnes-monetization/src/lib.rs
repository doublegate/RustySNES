//! Dormant entitlement/ad-pacing policy scaffold (`v1.18.0 "Dormant"`, Mobile Phase 5) — the
//! monetization crate RustyNES's own mobile track already ships as a dormant module, ported here
//! as a policy-*shape* template only. **Never a dependency of the deterministic core** —
//! `rustysnes-core`/`rustysnes-cpu`/`rustysnes-ppu`/`rustysnes-apu`/`rustysnes-cart` never depend
//! on this crate, and this crate never depends on them; it has no knowledge of `EmuCore` at all.
//! It is wired into both mobile shells (`android/`, `ios/`) as an inert dependency only: compiled
//! in and called once at startup (logged, not gating anything), no real RevenueCat/AppLovin (or
//! equivalent) SDK calls, no purchase/paywall UI shown.
//!
//! # Why "dormant"
//!
//! Unlike RustyNES's own module (which ships a committed pricing figure), every concrete number
//! here is an explicit placeholder default — [`AdPacingPolicy`]'s field values are a template
//! for a *later* maintainer decision, not a real pricing/pacing commitment.
//! `docs/mobile-readiness.md`'s standing "Mobile Phase 6" store-launch gate is where that decision
//! (and actually wiring a real store SDK) would happen — not here, not automatically.
//!
//! # Determinism discipline (same boundary as the deterministic core, extended here on principle)
//!
//! Every function in this crate is pure: no wall-clock reads, no OS RNG, no hidden state. Time
//! enters only as an explicit `now_unix_secs: u64` parameter the caller supplies (matching
//! `docs/adr/0004`'s "host-injected timestamps" convention already used elsewhere in this
//! project) — this crate never feeds anything back into `EmuCore`, so nothing here is subject to
//! the core's own byte-identical-replay contract, but keeping the same discipline anyway means
//! every function here is trivially unit-testable without mocking a clock.

uniffi::setup_scaffolding!();

/// Whether the user currently has the paid unlock.
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entitlement {
    /// `true` if paid features should be unlocked.
    pub unlocked: bool,
}

/// Placeholder ad-pacing knobs — see the module doc for why every field here is an explicit
/// placeholder, not a committed figure.
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdPacingPolicy {
    /// Minimum seconds between two ad presentations.
    pub min_interval_secs: u64,
    /// How many play sessions must complete before the very first ad is eligible at all.
    pub sessions_before_first_ad: u32,
}

impl AdPacingPolicy {
    /// Placeholder values only -- not a committed pacing figure, see the module doc. The single
    /// source of truth both [`Default::default`] and [`default_ad_pacing_policy`] return, so the
    /// two can never drift apart.
    const PLACEHOLDER_DEFAULT: Self = Self {
        min_interval_secs: 300,
        sessions_before_first_ad: 3,
    };
}

impl Default for AdPacingPolicy {
    fn default() -> Self {
        Self::PLACEHOLDER_DEFAULT
    }
}

/// The default, dormant ad-pacing policy.
///
/// Entitlement checks always report unlocked (no paywall active), and ad pacing uses
/// [`AdPacingPolicy::default`]'s placeholder figures. A mobile shell calling this and logging the
/// result (not gating any UI on it) is exactly the "inert dependency" integration this rung wires
/// up — see `android/`/`ios/`'s own call sites.
#[uniffi::export]
#[must_use]
pub const fn default_ad_pacing_policy() -> AdPacingPolicy {
    AdPacingPolicy::PLACEHOLDER_DEFAULT
}

/// Check entitlement for `now_unix_secs`.
///
/// Dormant: always reports unlocked (no active paywall) regardless of the timestamp — a real
/// store-backed check (RevenueCat/StoreKit/Play Billing) is a `docs/mobile-readiness.md`
/// "Mobile Phase 6" store-launch decision, not implemented here. `now_unix_secs` is accepted (and
/// ignored, via `let _ = ...` rather than an underscore-prefixed parameter name -- `#[uniffi::
/// export]`'s generated glue references the parameter by name, which trips
/// `clippy::used_underscore_binding` on a `_`-prefixed one) rather than dropped from the
/// signature, so the real check this eventually becomes doesn't need a breaking signature change
/// across the FFI boundary.
#[uniffi::export]
#[must_use]
pub const fn check_entitlement(now_unix_secs: u64) -> Entitlement {
    let _ = now_unix_secs;
    Entitlement { unlocked: true }
}

/// Pure ad-pacing decision: should an ad be shown right now?
///
/// - `now_unix_secs`: the current time, host-injected (see the module doc).
/// - `last_ad_unix_secs`: when the last ad was shown, or `None` if none has ever been shown this
///   install.
/// - `completed_sessions`: how many play sessions have completed so far.
/// - `policy`: the pacing knobs to evaluate against (typically [`default_ad_pacing_policy`]'s
///   result, but passed explicitly rather than read from hidden state, keeping this function pure
///   and directly unit-testable).
#[uniffi::export]
#[must_use]
pub fn should_show_ad(
    now_unix_secs: u64,
    last_ad_unix_secs: Option<u64>,
    completed_sessions: u32,
    policy: AdPacingPolicy,
) -> bool {
    if completed_sessions < policy.sessions_before_first_ad {
        return false;
    }
    last_ad_unix_secs
        .is_none_or(|last| now_unix_secs.saturating_sub(last) >= policy.min_interval_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entitlement_is_always_unlocked_while_dormant() {
        assert!(check_entitlement(0).unlocked);
        assert!(check_entitlement(u64::MAX).unlocked);
    }

    #[test]
    fn ad_pacing_withholds_the_first_ad_until_enough_sessions_completed() {
        let policy = AdPacingPolicy::default();
        assert!(!should_show_ad(1_000_000, None, 0, policy));
        assert!(!should_show_ad(
            1_000_000,
            None,
            policy.sessions_before_first_ad - 1,
            policy
        ));
    }

    #[test]
    fn ad_pacing_allows_the_first_ad_once_the_session_threshold_is_met() {
        let policy = AdPacingPolicy::default();
        assert!(should_show_ad(
            1_000_000,
            None,
            policy.sessions_before_first_ad,
            policy
        ));
    }

    #[test]
    fn ad_pacing_enforces_the_minimum_interval_between_ads() {
        let policy = AdPacingPolicy::default();
        let last_ad = 1_000_000;
        assert!(!should_show_ad(
            last_ad + policy.min_interval_secs - 1,
            Some(last_ad),
            policy.sessions_before_first_ad,
            policy
        ));
        assert!(should_show_ad(
            last_ad + policy.min_interval_secs,
            Some(last_ad),
            policy.sessions_before_first_ad,
            policy
        ));
    }

    #[test]
    fn ad_pacing_saturates_instead_of_panicking_on_a_time_regression() {
        // `now < last_ad_unix_secs` (a clock rollback) must not panic via unsigned subtraction
        // underflow -- `saturating_sub` clamps to 0, correctly withholding the ad.
        let policy = AdPacingPolicy::default();
        assert!(!should_show_ad(
            0,
            Some(1_000_000),
            policy.sessions_before_first_ad,
            policy
        ));
    }
}
