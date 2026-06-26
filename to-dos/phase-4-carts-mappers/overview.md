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
- [~] The shared µPD77C25 LLE engine runs DSP-1 (Core/Curated) — **DONE** (`coproc::upd77c25`
      the full NEC DSP instruction set, revision-parameterized for the six NEC chips;
      `coproc::dsp1` the Lo/HiROM DR/SR windows; boots Super Mario Kart / Pilotwings / Super
      Bases Loaded 2 / Aim for the Ace with user-supplied `dsp1*.rom`; `dsp1_oncart` gate green).
      **Super FX/GSU DONE** (`coproc::gsu` the full Argonaut RISC core + `coproc::superfx` the
      `SuperFxBoard` LoROM Super FX map + bus arbitration; host-synced on the Go flag; boots the
      58 Krom GSU test ROMs via `superfx_oncart` — detection + GSU-executed liveness + FillPoly
      plot-pipeline-into-RAM + deterministic golden). **SA-1 DONE** (`coproc::sa1::Sa1Board` the
      full SA-1 system — Super-MMC banking, BW-RAM 2/4 bpp bitmap, I-RAM, arithmetic/var-len units,
      H/V timer, normal + char-conversion DMA — plus the second 65C816 instantiated + stepped in
      `rustysnes-core` via the new `Board` second-CPU hooks, in deterministic master-clock catch-up
      that leaves the main CPU oracle 0-diff; `sa1_oncart` gate green: 18 commercial SA-1 carts,
      detection + S-CPU↔SA-1 traffic + aggregate SA-1-executed liveness + deterministic golden).
      **Phase 4 Core/Curated coprocessors complete.**
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
  **Status:** **complete.** `coproc::upd77c25` (clean-room µPD7725/µPD96050 LLE engine, full NEC
  DSP instruction set + RQM-handshake host sync) and `coproc::dsp1` (`Dsp1Board`, the Lo/HiROM
  DR/SR windows). Coprocessor detection from the `$FFD6` chipset byte; `board::select` routes
  `Coprocessor::Dsp`; `Cart::install_coprocessor_firmware` loads the user-supplied (gitignored)
  `dsp1*.rom`. Validated by engine unit vectors (synthetic firmware) + the `dsp1_oncart` harness
  gate (4 commercial DSP-1 dumps: detection + RQM-access on both windows + deterministic golden +
  firmware-differential). Honesty gate green. Remaining DSP siblings (DSP-2/3/4, ST010/011) reuse
  this engine in Phase 7.
- Sprint 3 — Super FX/GSU + SA-1.
  **Status:** **complete — Super FX/GSU and SA-1 both done; Phase 4 Core/Curated coprocessors
  finished.** `coproc::gsu` is a clean-room port of ares'
  GSU + SuperFX components (ISC): the full Argonaut RISC instruction set + the ALT-mode machine,
  the `mult`/`fmult`/`lmult` multiplier, the ROM/RAM buffers with their latency, the 256-byte
  opcode cache, the branch-delay pipeline, and the PLOT/RPIX pixel-plot pipeline (pixel cache +
  color/cmode logic + SCBR/SCMR 2/4/8 bpp character addressing). `coproc::superfx::SuperFxBoard`
  owns the cart ROM + Game Pak RAM, decodes the LoROM Super FX CPU map, and arbitrates the shared
  bus (snooze-vector / open-bus). No chip dump — the GSU program is in cart ROM; host-synced on the
  Go flag (`run_until_stopped`, the DSP-1 `run_until_rqm` analogue), so no core-scheduler tick.
  `board::select` routes `Coprocessor::SuperFx`; tier stays Curated, honesty gate green
  (`ORACLE_COPROCESSORS` ∋ SuperFx). Validated by `superfx_oncart` (58 Krom GSU ROMs:
  detection + GSU-executed liveness + FillPoly-into-RAM plot-pipeline + deterministic golden) + the
  per-opcode `GSUTest` suite + engine unit tests.

  **Deferred / honest gaps:** no commercial Super FX dumps are staged
  (`commercial/LoRom/GSU-1`/`GSU-2` are empty), so Star Fox / Yoshi's Island / Doom are not yet
  boot-validated — the Krom homebrew GSU suite is the current liveness/golden oracle. A 4 bpp
  `FillPoly` polygon reaches the framebuffer; full PPU BG-mode display coverage for the 2/8 bpp
  plot demos is a PPU concern, not the GSU's (the GSU correctly plots the bitmap into Game Pak RAM
  in every mode, asserted via `Board::sram`).

  **SA-1 (complete).** `coproc::sa1::Sa1Board` is a clean-room port of ares' `sfc/coprocessor/sa1`
  (ISC): the `$2200–$23FF` register file, the Super-MMC ROM banking (CXB/DXB/EXB/FXB), BW-RAM (the
  shared RAM with the `$2224` S-CPU window, the `$40–$4F` linear image, the SA-1 2/4 bpp bitmap +
  linear projections, and the SWEN/CWEN/BWPA write-protect), 2 KiB I-RAM, the arithmetic unit
  (mul/div/sigma), the variable-length bit unit, the H/V timer, and the normal + type-1/2
  character-conversion DMA. The crate graph forbids `rustysnes-cart` from depending on
  `rustysnes-cpu`, so the **second 65C816** lives in `rustysnes-core`: the scheduler owns an
  optional `sa1_cpu`, exposes the SA-1 memory view + control lines through the new default-no-op
  `Board` second-CPU hooks, wires a `Sa1Bus` adapter, and steps the second CPU in deterministic
  master-clock catch-up — gated to SA-1 carts and bounded by the untouched master clock, so the
  main CPU oracle stays 0-diff. `board::select` routes `Coprocessor::Sa1`; tier stays Curated,
  honesty gate green (`ORACLE_COPROCESSORS` ∋ Sa1). Validated by `sa1_oncart` (18 commercial SA-1
  carts: detection + S-CPU↔SA-1 register traffic for all 18 + an aggregate SA-1-executed liveness
  floor — observed 10, incl. Super Mario RPG / both Kirbys / PGA Tour 96 / Power Rangers Zeo at
  millions of SA-1 cycles — + deterministic golden) + board unit tests.

  **Deferred / honest gaps (SA-1):** the SA-1 timing is deterministic master-clock catch-up, not
  sub-instruction lockstep with the main CPU's individual bus accesses (exact for the register /
  arithmetic / DMA results games observe; the cross-CPU bus-conflict wait-states are not modelled).
  ~7 of the 18 staged carts defer waking the SA-1 (RDYB held, sometimes behind a main-CPU intro)
  past the boot-smoke frame budget, so the liveness gate is an aggregate floor, not per-ROM 100 %.

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
