# Phase 7 — Breadth

## Goal

Fill in the remaining **BestEffort** coprocessors and niche peripherals so the full
coprocessor / board matrix in `docs/STATUS.md` is complete — leaning on the shared µPD77C25 /
µPD96050 LLE engine to make the BestEffort DSP siblings near-free. Region timing remains data,
not a build fork. BestEffort boards never back the oracle (`docs/adr/0003`).

## Exit criteria

- [x] DSP-2/4 + ST010/011 ride the shared NEC core (BestEffort, honesty-gated). **DSP-3 has no
      board wired** — no verified board/window entry to pin against (`necdsp_variant.rs`); tracked
      as a named residual (`docs/STATUS.md`), not silently dropped.
- [x] S-DD1, SPC7110 (+ frozen RTC-4513), CX4, OBC1, ST018, S-RTC implemented to BestEffort.
      SPC7110's boot gap turned out to be a ROM-identity issue, not an emulation bug — the local
      test dump is an English fan-translation, not the original cartridge (three independent
      confirmations: a SHA256 mismatch, a checksum-size inconsistency, and a public forum thread
      on the patch's own non-standard memory map — `docs/audit/spc7110-boot-crash-2026-07-08.md`).
      Closing it out for real needs a genuine original-cartridge dump, tracked as a ROM-sourcing
      gap in `docs/rom-test-corpus.md`, not an open bug in `to-dos/VERSION-PLAN.md`'s
      accuracy-debt cluster.
- [x] RTC chips read frozen / seeded time (`docs/adr/0004`); chip-ROM-dump boards carry the
      honesty caveat.
- [x] **Core implemented, `v0.9.0`.** Niche peripherals (multitap, mouse, Super Scope) — the real
      2-bit-per-clock (`data1`/`data2`) serial-shift-register protocol, ported from ares'
      `sfc/controller/{mouse,super-scope,super-multitap}` (`rustysnes_core::controller`), not a
      stub. `Bus::set_port_device`/`set_mouse`/`set_superscope`/`set_multitap_pad`; save-stated as
      real hardware state (`FORMAT_VERSION` 2→3, `docs/adr/0006`); 14 unit tests. `rustysnes-
      netplay`'s 2-player scoping is unrelated (a netplay-session concern, not a controller-port
      one). **Frontend host-input capture (a real mouse pointer, extra gamepads) is NOT yet
      wired** — a Settings control selects the peripheral and it correctly changes emulated
      behavior, but nothing yet feeds it live OS input (`docs/frontend.md` §Peripherals has the
      precise remaining scope) — tracked as a frontend follow-up, not a Phase 7 (core) gap.
- [x] The full board matrix in `docs/STATUS.md` is complete; the honesty gate stays green.
- [x] Coprocessor sprints complete (Sprint 2 folded into the SPC7110/breadth work landed across
      `v0.4.0`-`v0.8.0` rather than as a separate formal sprint — see `docs/STATUS.md`'s
      coprocessor matrix for the per-chip record). Niche peripherals' core is now also complete;
      only frontend host-input capture (out of Phase 7's own scope) remains.

## Scope

In-scope:

- The BestEffort coprocessor family + the shared-core economy.
- Niche peripherals; region-timing-as-data completeness.

Out-of-scope:

- The Satellaview / Sufami Turbo / Super Game Boy pass-through (deferred per
  `ref-docs/research-report.md` "Scope").
- The fractional-timebase refactor (`docs/adr/0002`).

## Sprints

- [Sprint 1 — BestEffort coprocessors via the shared core](sprint-1-besteffort-coprocessors.md)
  — DSP-2/3/4 + ST010/011 + the simpler ASICs. **Complete** (see the sprint doc).
- Sprint 2 — SPC7110 / S-DD1 / CX4 / ST018: **complete** (landed across `v0.4.0`-`v0.8.0`, never
  run as a separate formal sprint — see `docs/STATUS.md`'s coprocessor matrix). **Peripherals
  (multitap/mouse/Super Scope): core complete, `v0.9.0`** — frontend host-input capture is a
  separate, tracked follow-up (`docs/frontend.md` §Peripherals), not part of this phase's own
  exit criteria.

## Dependencies

Phase 4 (the cart foundation + the shared NEC core + the honesty gate).

## Risks

- **Per-board bus windows** (no canonical table) — the long tail of board quirks. Detect: a
  board boots but mis-maps. Mitigate: per-board fixtures + the cartridge database.
- **RTC determinism** — a frozen-time regression. Mitigate: a determinism test per RTC board.

## Reference docs

- [docs/cart.md](../../docs/cart.md) — the coprocessor families + tiers.
- [docs/adr/0003](../../docs/adr/0003-accuracy-tiering-honesty-gate.md),
  [docs/adr/0004](../../docs/adr/0004-determinism-contract.md).
- [docs/STATUS.md](../../docs/STATUS.md) — the matrix this phase completes.
