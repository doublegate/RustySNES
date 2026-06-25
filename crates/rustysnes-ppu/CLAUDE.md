# CLAUDE.md — rustysnes-ppu

PPU1 (5C77) + PPU2 (5C78) video path. Spec: `../../docs/ppu.md` (read before changing rendering
or dot/scanline timing). Advanced lockstep by the master-clock scheduler on its divisor — never
free-run. Determinism is a hard contract: identical seed+ROM+input must yield bit-identical frames.
Touching rendering/timing behavior means updating `../../docs/ppu.md` in the same PR.
