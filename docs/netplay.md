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
