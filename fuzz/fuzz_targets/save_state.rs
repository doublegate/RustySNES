//! Fuzz `System::load_state` — the save-state envelope.
//!
//! The format is a magic + version header followed by tagged, length-prefixed sections, one per
//! subsystem, read back through `rustysnes-savestate`'s `SaveReader`. Every section's length is
//! taken from the input, so this is a dense field of "read N bytes where N came from the file".
//!
//! A save-state is untrusted input in practice, not just in theory: states are traded between
//! users, embedded in movie files (`Movie`'s start point can carry one), and handed in by a
//! libretro frontend through `on_unserialize`.
//!
//! Loading requires a cart already present — a state never embeds ROM bytes (`docs/adr/0006`) — so
//! the fixture machine is built first. Without it every input would be rejected on a cart mismatch
//! and the target would fuzz the first ten bytes of the header and nothing else.
//!
//! Existing pinned behaviour this must not regress: `bad_magic_is_rejected_not_panicked_on` and
//! `newer_format_version_is_rejected_not_panicked_on` (`scheduler.rs`).

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut core = common::minimal_core();

    if core.load_state(data).is_ok() {
        // A state that restores must also *run*. A load can succeed with internally inconsistent
        // values (an out-of-range counter that no section-level check rejects) and only fault on
        // the next step — which is the interesting class, because it turns a rejected file into a
        // crash one frame later.
        core.run_frame();

        // Round-trip: whatever was restored must be re-serializable. A state that loads but cannot
        // be saved means a subsystem accepted a value it cannot represent.
        let _ = core.save_state();
    }
});
