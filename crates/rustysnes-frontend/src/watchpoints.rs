//! Read/write watchpoint sync (`v0.8.0` T-81-001b).
//!
//! The frontend-owned [`crate::debug_snapshot::WatchpointEntry`] list installed into the `Bus`,
//! mirroring `crate::cheats::sync`'s own "always replace, re-synced once per frame" pattern
//! exactly. Entire module is `debug-hooks`-gated: this frontend's `debug-hooks` feature always
//! also enables `rustysnes-core/debug-hooks` (`Cargo.toml`), so the `Bus` methods this calls
//! always exist here.

use rustysnes_core::Bus;
use rustysnes_core::watchpoint::{WatchKind, Watchpoint};

use crate::debug_snapshot::{WatchpointEntry, WatchpointKind};

impl From<WatchpointKind> for WatchKind {
    fn from(k: WatchpointKind) -> Self {
        match k {
            WatchpointKind::Read => Self::Read,
            WatchpointKind::Write => Self::Write,
            WatchpointKind::ReadWrite => Self::ReadWrite,
        }
    }
}

/// Install every armed watchpoint into `bus` (replacing any previously installed set).
///
/// Called once per real frame from the app's drive loop, same cadence and same "just re-sync
/// unconditionally" reasoning as [`crate::cheats::sync`].
pub fn sync(entries: &[WatchpointEntry], bus: &mut Bus) {
    let points: Vec<Watchpoint> = entries
        .iter()
        .map(|e| Watchpoint {
            address: e.address,
            kind: e.kind.into(),
        })
        .collect();
    bus.set_watchpoints(&points);
}

#[cfg(test)]
mod tests {
    use rustysnes_core::System;
    use rustysnes_core::cpu::Bus as CpuBus;

    use super::*;

    #[test]
    fn sync_installs_the_armed_list() {
        let mut sys = System::new(0);
        sync(
            &[WatchpointEntry {
                address: 0x7E_0000,
                kind: WatchpointKind::ReadWrite,
            }],
            &mut sys.bus,
        );
        sys.bus.write24(0x7E_0000, 0x42);
        let hits = sys.bus.take_watchpoint_hits();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].address, 0x7E_0000);
        assert_eq!(hits[0].value, 0x42);
        assert!(hits[0].is_write);
    }

    #[test]
    fn sync_replaces_the_previously_installed_set() {
        let mut sys = System::new(0);
        sync(
            &[WatchpointEntry {
                address: 0x7E_0000,
                kind: WatchpointKind::ReadWrite,
            }],
            &mut sys.bus,
        );
        sync(&[], &mut sys.bus);
        sys.bus.write24(0x7E_0000, 0x42);
        assert!(sys.bus.take_watchpoint_hits().is_empty());
    }
}
