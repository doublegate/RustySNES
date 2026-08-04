# RustySNES `v1.27.0` "Tether"

**Released:** 2026-07-31 · **Tag commit:** `5c88457` · **Previous release:** [`v1.26.0` "Bulwark"](https://github.com/doublegate/RustySNES/releases/tag/v1.26.0)

> Netplay client-side depth. Before this release `rustysnes-netplay` had **no clock anywhere in
> it** — no handshake timeout, no peer liveness, no round-trip measurement — and the *first*
> checksum mismatch was a fatal disconnect. Both are now graded, bounded, and diagnosable.
>
> **Deliberately not in scope:** signaling server, STUN, relay, NAT traversal, mesh transport,
> lobby. This rung is client-side only, by decision.

---

## Executive summary

Three modules and a frontend surface, in dependency order:

1. **`diagnostics`** — one transient checksum mismatch no longer ends the session. Desync is now a
   graded, sticky verdict with a 3-consecutive-mismatch threshold, and the remote framebuffer hash
   is kept so a mismatch is *diagnosable* (timing divergence vs state divergence) rather than merely
   alarming.
2. **`LivenessTransport`** — peer grading, RTT over a probe/echo token pair, handshake and peer
   timeouts, with an **injected clock** so determinism is untouched and the tests need no sleeps.
3. **`SpectatorSession`** — read-only spectating that never predicts, never rolls back, and never
   sends. A spectator therefore *cannot* desync.
4. **Frontend** — the connection-quality readout, the graded desync banner, and — the load-bearing
   half — a disconnect that actually fires.

| | |
|---|---|
| Commits | 4 (PRs #278–#281) |
| Diff | 14 files changed, +3,387 / −25 |
| AccuracySNES coverage | **344 of 443** — unchanged; no chip model touched |
| Netplay `PROTOCOL_VERSION` | **1 → 2** (see compatibility) |
| Determinism contract | untouched; `tests/determinism.rs` passes unchanged |

---

## 1. Desync is a verdict, not a tripwire

Until now the **first** checksum mismatch raised a fatal `NetplayError::Desync`, and `app.rs`
disconnected on it. That is too eager: a burst-reordered pair of `Checksum` messages can momentarily
disagree before the deferred `compare_pending_checksums` pass reconciles them — so a transient
network event ended a healthy game.

`rustysnes-netplay::diagnostics` now records **every** comparison, matching ones included, and folds
them into one graded status:

```rust
enum DesyncStatus {
    InSync,
    Suspect  { consecutive: u32, first_desync_frame: u32 },
    Desynced { first_desync_frame: u32 },
}
```

- The threshold is **three consecutive mismatches** — roughly 1.5 s at the default 30-frame
  checksum interval.
- The fatal error fires only on `Desynced`. A transient is survived; a genuine divergence still
  ends the session, which it must, since a rollback desync cannot recover without a full state
  resync.
- `Desynced` is **sticky**: a later stray match resets the live run but never downgrades the
  verdict, because the condition cannot actually heal.

**The diagnostic half.** The remote framebuffer hash was previously discarded at the comparison site
and is now kept. RustySNES already sends both (`Checksum { frame, hash, fb_hash }`), so the
classification comes free:

| observation | meaning |
|---|---|
| same picture, different combined digest | a **timing** divergence |
| different picture | a **state** divergence |

**Determinism is untouched.** The module is purely observational: it reads values the session
already computed, holds no wall clock, and never feeds back into rollback (`docs/adr/0004`).
The history ring is bounded at 64 entries, but the first-diverging frame and the counters are
sticky scalars that survive eviction — so an hour-long session still reports where it first broke.

`tests/desync_hysteresis.rs` drives a real session against forged peer checksums. The transient case
asserts the forged value **actually reached the comparator** (`total == 1`, `mismatches == 1`) so it
cannot pass vacuously, and both it and the sustained-divergence control were verified by re-injecting
the old fail-on-first behaviour and confirming they fail.

---

## 2. Peer liveness, RTT, and timeouts — as a `Transport` decorator

The crate previously had **no liveness handling at all**: no clock anywhere in it, no handshake
timeout (an absent `Sync` stalled `advance()` forever), and `NetMessage::Quality` — which carries
the peer's ping and frame advantage — was received and *explicitly discarded*. A peer that unplugged
its cable simply stopped producing frames, with nothing to say why.

### Why a decorator, not a session field

Determinism (`docs/adr/0004`) forbids a clock inside `RollbackSession`. So the clock lives **outside**
it: `LivenessTransport` wraps the transport, the session still sees a plain `Transport`, and
`RollbackSession` **needed no change at all** — the existing trait was already the right seam.
`tests/determinism.rs` keeps proving what it proved before.

### The grades, and why they are forgiving

```text
Live  --(2 s silence)-->  Interrupted  --(5 s silence)-->  TimedOut
```

with `HandshakeTimeout` (10 s) and `PeerTimeout` as **distinct** `DisconnectReason`s, so a user can
tell "wrong address" from "your friend's connection died".

The thresholds are deliberately generous. Mesen's netplay uses a roughly **150 ms** trigger, which
flags ordinary Wi-Fi and LTE jitter as a disconnect — and a connection that reports "lost" whenever
a packet is late trains the user to ignore the indicator entirely. `interrupt_after` is **two full
ping intervals plus slack**, specifically so a single lost ping can never move the grade.

**Any traffic refreshes liveness, not just pings.** Gating on `Quality` alone would let a peer
streaming input perfectly grade as `Interrupted` because its pings happened to be the packets that
dropped.

### RTT over a probe/echo token pair

`Quality` gained a probe/echo token pair (`PROTOCOL_VERSION` → 2):

- A probe sets a nonzero `probe`; the receiver answers immediately with `probe: 0, echo: <token>`.
  An answer is therefore **not itself a probe**, and two peers cannot ping-pong.
- Matching on the token means a **duplicated datagram produces no second sample**, and a reply that
  arrives a generation late produces **none at all** — rather than a fabricated near-zero RTT at
  exactly the moment the connection is worst.
- Samples feed an **EWMA** (α = 0.2) because the number is shown to a human and an unsmoothed
  readout flickers with every packet.
- **Only a probe carries a `frame_advantage`.** An answer is emitted from `poll`, which has no
  access to the caller's current advantage — taking its `0` would zero the readout on every round
  trip.

`peer_link()` evaluates the handshake deadline off the clock like the silence deadline, and a
torn-down session answers nothing and measures nothing.

### The clock is injected

`Clock`, `SystemClock`, `ManualClock`. Testing timeouts against the wall clock means
`thread::sleep` in tests — slow, and flaky under CI load *precisely because* the thresholds under
test are short. With a manual clock the state machine is driven instantly: a 5-second peer timeout
is exercised in microseconds. **Twenty-one tests, no sleeps**, including one that wires two real
decorators together so the echo has to be one the implementation itself produces.

---

## 3. Read-only spectating

A spectator receives the players' confirmed input stream and replays it into its own `System`.

**It never predicts and never rolls back** — that is the whole difference from `RollbackSession`.
A player must predict to hide latency and must therefore be able to roll back when a prediction was
wrong. A spectator has no input of its own to hide latency for, so it waits until a frame's inputs
are all known and then runs it. No prediction means no misprediction, which means **a spectator
cannot desync**: it either has a frame's inputs or it does not.

**Receive-only.** It never sends an ack, a checksum, or a quality reply, so however many attach, the
match sees no extra traffic. A test feeds one a stream deliberately containing `InputAck`,
`Checksum`, and `Quality` and asserts the send count is still **zero** — because the tempting bug is
to answer them.

**`delay_frames`** (the tournament-broadcast / anti-spoiler hold, clamped to 512) moves **when** a
frame is revealed, never **what** it contains. Both halves are pinned:
`spectator_output_matches_a_reference_run` asserts a spectator's framebuffer sequence is
byte-identical to a direct run of the same inputs with no netplay in the way, and the delayed variant
asserts the frames it *did* show match the same prefix.

**Untrusted-input bounds:**

- An out-of-range player index is dropped — `num_players` is fixed at construction, so it can never
  become valid.
- A frame past `MAX_SPECTATOR_FRAME_LOOKAHEAD` is dropped. Without this, one datagram carrying a
  frame near `u32::MAX` would resize the history buffer unboundedly.
- `delay_frames` / `num_players` are clamped at construction.
- Input is dropped until the handshake is accepted, so a foreign ROM hash means **nothing is
  watchable** — rather than merely that a flag reads `false`. `synced` was set and consulted by
  nothing in the first revision, which a test asserting only that flag would never have caught.

---

## 4. The frontend, and the hang it removes

The preceding three built the machinery; nothing in the frontend read it. The session's transport is
now wrapped in `LivenessTransport` and ticked once per frame from `NetplayState::drive` with this
peer's own frame advantage. The Netplay window shows peer grade, ping, frame advantage, current
frame, a handshake notice, and the graded desync verdict.

**Two deliberate presentation choices:**

- `Interrupted` is **amber, not red**. It is two full ping intervals of silence, which ordinary
  Wi-Fi produces; painting it red would train the user to ignore the colour that matters.
- Ping shows **`—`, not `0 ms`**, until a round trip completes — a zero reads as a *perfect*
  connection at exactly the moment nothing is known yet.

**The load-bearing half is not the readout.** `NetplayError::Disconnected` carries the liveness
verdict and `drive` raises it before advancing. Without that, the liveness work would have been
inert: the session has no clock by design, cannot tell "waiting for the peer's next input" from
"waiting forever", and a peer that never handshakes leaves `advance` spinning with nothing to
report. **That hang is what this set out to remove.**

`RollbackSession` gained `transport()` / `transport_mut()` for it, since the session owns its
transport and a decorated one is otherwise unreachable once a session is built. New
`tests/liveness_session.rs` drives a real session through the decorator and pins both disconnect
reasons plus the transparent case; injecting the raise back out fails two of its three tests.

The terminal reason is reported through the Netplay window's `netplay_error`, **not** through the
per-frame snapshot: `drive` raises the verdict the same frame it appears, so the session is `Idle`
before the next egui pass, and a banner inside the quality readout would have been unreachable code
that looked like a feature.

---

## Compatibility and upgrade notes

- **`PROTOCOL_VERSION` 1 → 2.** The `Quality` message gained probe/echo fields. Peers must run
  matching versions; a mismatched peer is rejected at handshake rather than misparsed.
- **`MAX_PLAYERS` stays 2.** `NetMessage::Roster` and the mesh transport are built for more than two
  peers via the NES Four Score; core multitap exists here but wiring it into netplay is separate
  scope, and without it most of that machinery would be dead weight.
- **Save states:** format version unchanged.
- **Emulation behaviour:** unchanged. No chip model touched; `tests/determinism.rs` passes
  unchanged.
- **Behaviour change users will notice:** a brief network hiccup that previously disconnected you
  now shows an amber `Interrupted` and recovers.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p rustysnes-netplay          # incl. determinism, desync_hysteresis, liveness_session
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

- Desync hysteresis unit-tested through `InSync` → `Suspect` → `Desynced` → sticky, with the
  transient case asserting the forged checksum reached the comparator.
- A synthetic reordered-`Checksum` pair does **not** disconnect.
- Peer timeouts driven against `ManualClock` — 21 tests, zero sleeps.
- `spectator_output_matches_a_reference_run` byte-identical.
- Every new test verified by re-injecting the old behaviour and confirming it fails.

## Included changes

| PR | Commit | Summary |
|---|---|---|
| #278 | `6032e26` | `fix(netplay)`: grade the desync verdict so a transient doesn't end the game |
| #279 | `8f3fb70` | `feat(netplay)`: peer liveness, RTT, and timeouts as a `Transport` decorator |
| #280 | `3e6f9c5` | `feat(netplay)`: read-only spectating with a presentation-only reveal delay |
| #281 | `5c88457` | `feat(frontend)`: surface netplay connection quality and act on a dead peer |

New subsystem doc: `docs/netplay.md`. Full per-entry detail:
[`CHANGELOG.md` → `[1.27.0]`](../../CHANGELOG.md).
