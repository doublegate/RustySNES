//! 65C816 CPU-bus read/write watchpoints (`v0.8.0`, T-81-001b).
//!
//! Feature-gated (`debug-hooks`) opt-in debugger observability: an armed watchpoint list checked
//! on every CPU-visible [`crate::Bus::read24`]/[`crate::Bus::write24`] call, recording a bounded
//! ring of hits (address, access kind, value, and the CPU program-bank:counter at the moment of
//! the access) for a frontend debugger to display. This crate compiles the whole module out when
//! `debug-hooks` is off (`lib.rs` gates the `pub mod`), so a default build carries zero extra
//! code on the accuracy-critical Bus read/write path — the same "additive, off-by-default"
//! discipline [`crate::cheat`]'s read intercept already established, just one layer lower (that
//! module stays unconditionally compiled since it's pure computation; this one touches live
//! `Bus` state, so it needs the harder feature gate).
//!
//! Not part of any save state (`docs/adr/0004`): a watchpoint is host debugger tooling, not
//! emulated hardware behavior, so it never perturbs the determinism contract — a build with
//! `debug-hooks` on and zero watchpoints armed costs exactly one `is_empty()` branch per access,
//! identical in shape to [`crate::cheat`]'s own empty-patch-list fast path.

use alloc::vec::Vec;
use core::mem;

/// Ring capacity for recorded hits — generous for one debugger session's worth of activity
/// without unbounded growth if a watchpoint fires every frame and the frontend doesn't drain
/// promptly (the oldest hit is dropped to make room, same policy `rustysnes-netplay`'s pending
/// remote-checksum queue already uses for the same "untrusted/high-frequency producer" reason,
/// even though a watchpoint's producer here is trusted local emulation, not the network).
const MAX_HITS: usize = 256;

/// Which access kind(s) a watchpoint fires on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchKind {
    /// Fire only on a CPU read of the watched address.
    Read,
    /// Fire only on a CPU write to the watched address.
    Write,
    /// Fire on either a read or a write.
    ReadWrite,
}

impl WatchKind {
    /// Whether this watchpoint's armed kind matches an access (`is_write` — `true` for a write,
    /// `false` for a read).
    #[must_use]
    const fn matches(self, is_write: bool) -> bool {
        match self {
            Self::Read => !is_write,
            Self::Write => is_write,
            Self::ReadWrite => true,
        }
    }
}

/// One armed watchpoint: fire when `address` is accessed with a matching `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Watchpoint {
    /// The 24-bit CPU-bus address (`$bank:offset`) this watchpoint targets.
    pub address: u32,
    /// Which access kind(s) trigger it.
    pub kind: WatchKind,
}

/// One recorded watchpoint hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchpointHit {
    /// The 24-bit CPU-bus address that was accessed.
    pub address: u32,
    /// The byte value read or written.
    pub value: u8,
    /// `true` if this hit was a write, `false` if a read.
    pub is_write: bool,
    /// The CPU's `PBR:PC` (24-bit, `$bank:offset`) at the moment of the access — the instruction
    /// that performed it, not the watched address itself. `0` if the scheduler never called
    /// [`crate::Bus::set_debug_pc`] before this access (e.g. a test harness driving `Bus` directly
    /// without a `System`).
    pub pbr_pc: u32,
}

/// The armed watchpoint list + the hit ring, owned by [`crate::Bus`].
#[derive(Debug, Default)]
pub struct WatchpointState {
    watchpoints: Vec<Watchpoint>,
    hits: Vec<WatchpointHit>,
}

impl WatchpointState {
    /// Replace the armed watchpoint list (mirrors [`crate::Bus::set_cheats`]'s "always replace,
    /// re-synced once per frame" contract — see `crates/rustysnes-frontend/src/cheats.rs`'s
    /// `sync` for the frontend-side precedent this debugger UI follows too).
    pub fn set_watchpoints(&mut self, points: &[Watchpoint]) {
        self.watchpoints.clear();
        self.watchpoints.extend_from_slice(points);
    }

    /// Drain every hit recorded since the last call.
    pub fn take_hits(&mut self) -> Vec<WatchpointHit> {
        mem::take(&mut self.hits)
    }

    /// Check `address`/`is_write` against every armed watchpoint, recording a hit on a match.
    /// Costs exactly one `is_empty()` branch when no watchpoints are armed.
    pub fn check(&mut self, address: u32, value: u8, is_write: bool, pbr_pc: u32) {
        if self.watchpoints.is_empty() {
            return;
        }
        let hit = self
            .watchpoints
            .iter()
            .any(|w| w.address == address && w.kind.matches(is_write));
        if !hit {
            return;
        }
        if self.hits.len() >= MAX_HITS {
            self.hits.remove(0);
        }
        self.hits.push(WatchpointHit {
            address,
            value,
            is_write,
            pbr_pc,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_watchpoints_never_record_a_hit() {
        let mut st = WatchpointState::default();
        st.check(0x7E_0000, 0x42, false, 0x00_8000);
        assert!(st.take_hits().is_empty());
    }

    #[test]
    fn read_only_watchpoint_ignores_writes() {
        let mut st = WatchpointState::default();
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_0000,
            kind: WatchKind::Read,
        }]);
        st.check(0x7E_0000, 0x42, true, 0x00_8000);
        assert!(st.take_hits().is_empty());
        st.check(0x7E_0000, 0x42, false, 0x00_8000);
        let hits = st.take_hits();
        assert_eq!(hits.len(), 1);
        assert!(!hits[0].is_write);
    }

    #[test]
    fn write_only_watchpoint_ignores_reads() {
        let mut st = WatchpointState::default();
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_0000,
            kind: WatchKind::Write,
        }]);
        st.check(0x7E_0000, 0x99, false, 0x00_8000);
        assert!(st.take_hits().is_empty());
        st.check(0x7E_0000, 0x99, true, 0x00_8000);
        let hits = st.take_hits();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].is_write);
        assert_eq!(hits[0].value, 0x99);
    }

    #[test]
    fn read_write_watchpoint_fires_on_both() {
        let mut st = WatchpointState::default();
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_1234,
            kind: WatchKind::ReadWrite,
        }]);
        st.check(0x7E_1234, 0x01, false, 0x00_8000);
        st.check(0x7E_1234, 0x02, true, 0x00_8010);
        let hits = st.take_hits();
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn unwatched_address_never_records() {
        let mut st = WatchpointState::default();
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_0000,
            kind: WatchKind::ReadWrite,
        }]);
        st.check(0x7E_0001, 0x42, true, 0x00_8000);
        assert!(st.take_hits().is_empty());
    }

    #[test]
    fn take_hits_drains_and_clears() {
        let mut st = WatchpointState::default();
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_0000,
            kind: WatchKind::ReadWrite,
        }]);
        st.check(0x7E_0000, 0x01, false, 0x00_8000);
        assert_eq!(st.take_hits().len(), 1);
        assert!(st.take_hits().is_empty());
    }

    #[test]
    fn set_watchpoints_replaces_the_previous_list() {
        let mut st = WatchpointState::default();
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_0000,
            kind: WatchKind::ReadWrite,
        }]);
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_1111,
            kind: WatchKind::ReadWrite,
        }]);
        st.check(0x7E_0000, 0x01, false, 0x00_8000);
        assert!(
            st.take_hits().is_empty(),
            "the first list's watchpoint must no longer be armed"
        );
        st.check(0x7E_1111, 0x02, false, 0x00_8000);
        assert_eq!(st.take_hits().len(), 1);
    }

    #[test]
    fn ring_drops_the_oldest_hit_once_full() {
        let mut st = WatchpointState::default();
        st.set_watchpoints(&[Watchpoint {
            address: 0x7E_0000,
            kind: WatchKind::ReadWrite,
        }]);
        for i in 0..(MAX_HITS + 8) {
            #[allow(clippy::cast_possible_truncation)]
            st.check(0x7E_0000, i as u8, false, 0x00_8000);
        }
        let hits = st.take_hits();
        assert_eq!(hits.len(), MAX_HITS);
        // The oldest 8 hits (value 0..8) were dropped to make room; the ring keeps the newest.
        #[allow(clippy::cast_possible_truncation)]
        let expected_first = 8u8;
        assert_eq!(hits[0].value, expected_first);
    }
}
