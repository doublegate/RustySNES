# Netplay

`crates/rustysnes-netplay` — GGPO-style rollback netplay. Spec for the crate; the rollback loop's
own mechanics are documented in `session.rs`'s module doc, and this file records the decisions that
span modules.

## Scope

Two players, because the SNES has two physical controller ports (`Bus::joypad: [u16; 2]`). The
sibling RustyNES project supports up to four via the NES Four Score and carries a `Roster` message
plus a mesh transport for it; neither is ported, and without multitap in the core neither would do
anything here.

Deliberately **not** ported from that project either: the signalling/lobby protocol, room directory,
quick-match, STUN, and TURN relay. Those need a hosted signalling server — an operational
commitment, not just code — and the decision was to take the client-side depth without it. Sessions
are established by direct address.

## Determinism

The whole point. `RollbackSession::advance`'s rollback/re-simulate path must reproduce a
hypothetical zero-latency reference run **bit-identically**; `tests/determinism.rs` proves it over
synthetic latency, jitter, and packet loss via `MemoryTransport`.

That constrains where wall-clock may live. `RollbackSession` reads no clock at all — it is a pure
function of its input stream — so anything time-based (timeouts, RTT, liveness) belongs outside it.

## Desync detection (`v1.27.0`)

Peers exchange a state checksum every `checksum_interval` frames (default 30). The message carries
**two** hashes: a combined gameplay digest and a framebuffer-only hash.

### The verdict is graded, not binary

Until `v1.27.0` the **first** mismatch was a fatal `NetplayError::Desync` and the frontend tore the
session down on it. That is too eager. A burst-reordered pair of `Checksum` messages can momentarily
disagree before the deferred `compare_pending_checksums` pass reconciles them, so a transient
network event ended a healthy game.

`diagnostics::DesyncDiagnostics` now records **every** comparison — matching ones included — and
folds them into one `DesyncStatus`:

| status | meaning |
|---|---|
| `InSync` | every comparison so far matched |
| `Suspect { consecutive, first_desync_frame }` | something mismatched, but the run is below the confirm threshold |
| `Desynced { first_desync_frame }` | the run reached the threshold — a real, sustained divergence |

Three consecutive mismatches (~1.5 s at the default interval) confirm. The fatal error now fires
only on `Desynced`, so a transient is survived and a genuine divergence still ends the session —
which it must, because a rollback desync is unrecoverable without a full state resync.

**`Desynced` is sticky.** A later stray match resets the live consecutive run but never downgrades
the verdict, because the underlying condition cannot actually heal. A surface that flapped between
"desynced" and "fine" would train the user to ignore it.

### The run counts consecutive *frames*, not consecutive *records*

The threshold's rationale is stated in time ("~1.5 s at the default interval"), so the
implementation has to agree with that. Counting consecutive *recorded comparisons* would not: on a
lossy link the checksums in between may never arrive to be compared, so three isolated transients
seconds apart would be recorded back-to-back and confirm a desync that never happened.

A run therefore continues only if the mismatch is within `max_run_gap_frames` of the previous
comparison, derived from the session's own `checksum_interval` via
`DesyncDiagnostics::gap_for_interval` (two intervals). That tolerates a single lost checksum inside
a genuine run — a real desync mismatches *every* checksum, so its members are one interval apart —
while refusing to assemble a run out of widely separated transients. Both directions are pinned by
tests, since a gap check that is too tight would let a real desync go unconfirmed on any lossy link.

### The error payload describes one comparison

`NetplayError::Desync` reports the **earliest** diverging comparison, whole. Two things force that.
The confirming pass may contain no mismatch of its own (the run can be built across earlier passes),
and the frame worth reporting is where the divergence *began* — that is where a bisect starts, not
where a counter crossed a threshold.

So `DesyncDiagnostics` retains the first diverging `CrcCompare` rather than just its frame number.
An earlier revision filled the gap by pairing `first_desync_frame` with the hashes of whatever was
compared last, which can be a different — even matching — frame: an error message that looks precise
and is not.

### The framebuffer hash classifies the failure

The remote framebuffer hash used to be discarded at the comparison site. It is now kept, and it is
what makes a mismatch diagnosable rather than merely alarming:

- **same picture, different combined digest** — only the cumulative cycle term diverged, so the bug
  is in **timing**;
- **different picture** — the rendered output itself diverged, so the bug is in **state**.

### Purely observational

`DesyncDiagnostics` only reads values the session already computed and stores copies. It never feeds
back into the rollback algorithm, the checksum exchange, or the emulator, and it holds no clock.
Deleting it would leave every produced frame, checksum, and rollback byte-identical — so it cannot
perturb the determinism contract above.

The history is a fixed-capacity ring (64 entries, ~32 s at the default interval), but the
first-diverging frame and the mismatch counters are **sticky scalars that survive eviction**: a
session running for an hour still reports where it first broke rather than forgetting once 64 newer
comparisons arrive.

### Tests

`diagnostics.rs`'s unit tests cover the verdict arithmetic. `tests/desync_hysteresis.rs` covers what
actually changed, by driving a real session against forged peer checksums:

- a single mismatched checksum no longer ends the session — and the test asserts the forged value
  genuinely reached the comparator (`total == 1`, `mismatches == 1`), so it cannot pass vacuously;
- a sustained divergence still fails, having crossed the threshold rather than firing early;
- a clean session stays `InSync`.

Both were verified by re-injecting the old fail-on-first behaviour and confirming they fail.

## Peer liveness, RTT, and timeouts (`v1.27.0`)

Before this the crate had **no liveness handling at all**: no clock anywhere in it, no handshake
timeout (an absent `Sync` stalled `advance()` forever), and `NetMessage::Quality` — which carries
the peer's ping and frame advantage — was received and explicitly discarded. A peer that unplugged
its cable simply stopped producing frames, with nothing to say why.

### It is a `Transport` decorator, not a session field

Determinism forbids a clock inside `RollbackSession`. So the clock lives outside it:
`LivenessTransport` wraps any `Transport` and timestamps what passes through. The session sees a
plain `Transport`, stays a pure function of its input stream, and `tests/determinism.rs` keeps
proving exactly what it proved before.

That the existing `Transport` trait was already the right seam is why this needed no change to the
session at all.

### The grades

| grade | after |
|---|---|
| `Live` | traffic arriving normally |
| `Interrupted` | silence past `interrupt_after` (default **2 s**) |
| `TimedOut` | silence past `disconnect_after` (default **5 s**) |

`peer_link()` evaluates **both** deadlines off the clock, the handshake one included. It grades the
silence path live, so leaving the handshake path to `tick` alone made the two disagree for up to a
frame — a connection whose handshake window had closed still reading `Live`.

Graded rather than boolean, and the thresholds are deliberately forgiving. Mesen's netplay uses a
roughly 150 ms trigger, which flags ordinary Wi-Fi and LTE jitter as a disconnect; a connection that
reports "lost" every time a packet is late trains the user to ignore it. `interrupt_after` is two
full ping intervals plus slack precisely so **a single lost ping can never move the grade**, and
`disconnect_after` sits in the multi-second range GGPO and Parsec use — long enough to survive a
Wi-Fi roam, short enough not to wait on a dead peer.

`HandshakeTimeout` and `PeerTimeout` are distinct reasons on purpose: a user needs to tell "wrong
address" from "your friend's connection died".

**Any traffic refreshes liveness**, not just pings. Gating on `Quality` alone would let a peer
streaming input perfectly grade as `Interrupted` simply because its pings were the packets that
dropped.

### RTT

`Quality` carries a **probe/echo token pair**, and `PROTOCOL_VERSION` was bumped to 2 for it. A
probe sets `probe` to a nonzero token and `echo` to 0; the receiver answers *immediately, inside
`poll`* with the mirror image — `probe: 0`, `echo: <that token>`. An answer is therefore not itself
a probe, which is the termination argument: two peers cannot ping-pong.

**The first revision had no token and measured nothing.** It matched a reply to a ping positionally
— "whichever probe is outstanding" — and *nothing in the crate ever replied to a `Quality`*:
`RollbackSession` explicitly ignored it, and each peer's own `LivenessTransport` emitted probes on
its own independent 1 s timer. So the packet a peer counted as its echo was simply the other side's
next scheduled probe, and the reported "RTT" was the phase offset between two timers plus one-way
latency. Every test covering it injected a hand-written reply and so agreed with the bug; the
present `two_peers_actually_close_the_round_trip` wires two real decorators together precisely so
the answer has to be one the implementation itself produces.

The token also subsumes two hazards the earlier revision handled with extra state:

- a **duplicated datagram** cannot produce a second sample, because the outstanding probe is
  consumed by the first matching echo;
- a **reply that arrives a generation late** does not match the newer probe's token, so it produces
  no sample rather than a fabricated near-zero RTT at the exact moment the connection is worst —
  the one moment the number has to be trusted.

Because a stale reply is now harmless, probing can stay on a fixed cadence rather than stalling
whenever a reply goes missing. Samples feed an EWMA at weight 0.2, because the number is shown to a
human: unsmoothed, the readout flickers with every packet.

**Only a probe carries a `frame_advantage`.** An answer is emitted from `poll`, which has no access
to the caller's current advantage, so it sends 0 — and reading that 0 as the peer's own value would
zero the readout on every single round trip. A number that is correct only in the gaps between
echoes is worse than no number.

**A torn-down session answers nothing and measures nothing.** Once the disconnect verdict is set it
is sticky and the caller has been told the peer is gone; replying would put traffic on a dead
connection, and a sample taken afterwards describes a link nobody is using.

`ping_smoothing` is read from config, so it is treated as untrusted: NaN falls back to the default
weight and anything else is clamped to `0.0..=1.0`. `f64::clamp` returns NaN for NaN rather than
clamping it, and a single NaN sample would poison the EWMA permanently — every later reading would
be NaN, with no way back.

### The clock is injected

`Clock` is a trait, not a call to `Instant::now()`. Testing timeout behaviour against the wall clock
means `thread::sleep` in tests — slow, and flaky under CI load precisely because the thresholds
being tested are short. (The sibling project's equivalent test does exactly that.) With
`ManualClock` the whole state machine is driven instantly: a 5-second peer timeout is exercised in
microseconds and cannot fail because a runner was busy. Twenty-one tests, no sleeps.

`ManualClock` holds its offset in an `AtomicU64` rather than a `Cell`, which makes it `Sync`. A
non-`Sync` clock would make `LivenessTransport<T, ManualClock>` non-`Sync` too, so a test could not
exercise the decorator on the threaded path the frontend actually uses it on.

`ManualClock` is `pub`, not `#[cfg(test)]`, so a frontend integration test outside the crate can use
it too.

