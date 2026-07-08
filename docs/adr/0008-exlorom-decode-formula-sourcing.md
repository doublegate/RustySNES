# ADR 0008 — Sourcing the ExLoROM decode formula from bsnes's runtime board database

## Status

Accepted (`v0.3.0 "Continuum"`).

## Context

ExLoROM (the LoROM-style address decode extended to >4 MiB images, mostly homebrew/flashcart
titles) has no dedicated `$FFD5` mapping-mode value — ares and bsnes both document it as
unofficial (`ref-proj/ares/mia/medium/super-famicom.cpp`: "ExLoROM mode is unofficial, and lacks
a mapping mode value"), and real carts that use it typically report plain LoROM's `$20` at that
header offset. This means header-byte detection alone cannot confirm ExLoROM, and — critically —
it means there is no authoritative spec document to read the decode *formula* off of either: no
official Nintendo documentation describes a mapping mode that was never assigned one.

Two candidate sources existed for the actual byte-decode formula: (a) infer it by extrapolating
LoROM's own formula to a larger address space, or (b) find how a real, working reference
emulator's *runtime* board-configuration data models it.

## Decision

Source the decode formula from **bsnes's own runtime board database**
(`ref-proj/bsnes/bsnes/target-bsnes/resource/system/boards.bml`, `board: EXLOROM` /
`EXLOROM-RAM`), not extrapolated from LoROM's formula or the header-detection heuristic. The
`.bml` entries (`map address=00-7d:8000-ffff mask=0x808000 base=0x400000` / `map
address=80-ff:8000-ffff mask=0x808000 base=0x000000`) are decoded against bsnes's own `Bus::reduce`
bit-packing algorithm (`ref-proj/bsnes/bsnes/sfc/memory/memory.cpp`) to derive the concrete
formula: `high | ((bank & 0x7F) << 15) | (addr & 0x7FFF)`, where `high = (bank & 0x80 != 0) ? 0 :
(1<<22)` — the same A23-inverted 4 MiB half-select ExHiROM already uses (`docs/cart.md`
§ExLoROM). This is the pattern already established for CX4/DSP-1/board detection generally:
prefer a reference emulator's own *working, executed* configuration data over reverse-engineering
a formula from partial documentation or extrapolation.

`docs/adr/0003`'s honesty gate applies directly: no real ExLoROM ROM (commercial or homebrew)
exists in this project's local corpus, so the board has formula-level unit-test coverage only,
never claimed as golden-framebuffer-validated.

## Consequences

- (+) The formula is traceable to a concrete, checked-in source file and algorithm rather than an
  unverifiable inference — a future maintainer (or bot reviewer) can re-derive it independently.
- (+) Establishes "read a reference emulator's runtime board database, not just its documentation
  comments" as the go-to sourcing pattern for any future undocumented-mode board this project
  adds (the same technique that resolved ST018's `ARM-LOROM-RAM` board memory map, `docs/
  st018-arm-notes.md`).
- (−) The formula is only as trustworthy as bsnes's own board database being correct for this
  specific unofficial mode — there is no independent second source to cross-check it against
  until a real ExLoROM ROM surfaces to validate end-to-end.
- (−) No golden-ROM-boot proof exists yet; this is carried openly in `docs/STATUS.md`'s accuracy
  dashboard and `docs/cart.md`, not silently presented as hardware-proven.
