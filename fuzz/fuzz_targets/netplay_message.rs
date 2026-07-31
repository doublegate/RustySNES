//! Fuzz `NetMessage::decode` — the netplay wire format.
//!
//! The only boundary in this set that takes bytes straight off a socket from an unauthenticated
//! remote peer, so it is the one where a panic is a remotely triggerable denial of service rather
//! than a bad-file bug.
//!
//! Hand-rolled tag-byte-discriminated little-endian framing (no serde), five variants. Three
//! adversarial unit tests already pin truncation, unknown tags, and the empty slice; this target
//! covers the space between them — a valid tag with a partially-valid body.
//!
//! The round-trip is asserted, not merely exercised: `decode` and `encode` are a matched pair
//! written by hand, and a variant whose encoder disagrees with its decoder would corrupt a live
//! session silently rather than reject the packet.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_netplay::NetMessage;

fuzz_target!(|data: &[u8]| {
    if let Ok(msg) = NetMessage::decode(data) {
        let re_encoded = msg.encode();
        let round_tripped =
            NetMessage::decode(&re_encoded).expect("a message this decoder produced must re-decode");
        assert_eq!(
            msg, round_tripped,
            "NetMessage encode/decode disagree — a live session would corrupt silently"
        );
    }
});
