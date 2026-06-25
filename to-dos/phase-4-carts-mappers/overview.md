# Phase 4 — Carts + coprocessors (Core tier first)

## Goal

Implement the cartridge memory model (LoROM / HiROM / ExHiROM + header auto-detection +
SRAM/battery) and the **Core/Curated** coprocessors — DSP-1 (via the shared µPD77C25 LLE
engine), Super FX/GSU, and SA-1. Tier every board Core/Curated/BestEffort and stand up the
honesty gate from the first board (`docs/adr/0003`). BestEffort breadth is Phase 7.

## Exit criteria

- [ ] LoROM/HiROM/ExHiROM map models + the header score heuristic boot the canonical commercial
      set with the right map mode auto-detected.
- [ ] SRAM / battery save round-trips deterministically.
- [ ] The shared µPD77C25 LLE engine runs DSP-1 (Core/Curated); Super FX/GSU and SA-1 (reusing
      the Phase-1 65C816 core) are cycle-accurate from cart ROM.
- [ ] The honesty gate is live: no BestEffort board backs the oracle.
- [ ] Each implemented board boots a commercial dump locally → committed screenshots / `.snap`
      (never the ROM).
- [ ] All sprints complete; `docs/STATUS.md` coprocessor matrix updated.

## Scope

In-scope:

- The `Cart` trait + the three map models + header detection (`docs/cartridge-format.md`).
- The shared µPD77C25 / µPD96050 LLE engine + DSP-1.
- Super FX/GSU + SA-1 (the Core/Curated coprocessors).
- The honesty-gate CI test.

Out-of-scope (Phase 7):

- BestEffort coprocessors (DSP-2/3/4, S-DD1, SPC7110, CX4, OBC1, ST01x/ST018, S-RTC) — though
  the shared core makes the BestEffort DSP siblings near-free, they stay tiered BestEffort until
  verified.
- Niche peripherals beyond a stub.

## Sprints

- [Sprint 1 — Memory map + header detection + the honesty gate](sprint-1-cart-map.md) — the
  cart foundation.
- Sprint 2 — The shared µPD77C25 core + DSP-1.
  **Status:** stub — refine when Sprint 1 is ~complete.
- Sprint 3 — Super FX/GSU + SA-1.
  **Status:** stub.

## Dependencies

Phases 1–2 (the CPU + scheduler the boards plug into); SA-1 reuses the Phase-1 65C816 core.

## Risks

- **No canonical per-board SRAM / coprocessor bus-window table** — build from the cartridge
  database + ares board definitions. Detect: a board boots but SRAM lands wrong. Mitigate:
  per-board fixtures.
- **Chip-ROM-dump dependence** (DSP family) — gate behind a feature + loud honesty caveat;
  without the dump the board is non-functional, never silently degraded.

## Reference docs

- [docs/cart.md](../../docs/cart.md) — the map models + coprocessor families + the shared core.
- [docs/cartridge-format.md](../../docs/cartridge-format.md) — the header + detection heuristic.
- [docs/adr/0003](../../docs/adr/0003-accuracy-tiering-honesty-gate.md) — the honesty gate.
