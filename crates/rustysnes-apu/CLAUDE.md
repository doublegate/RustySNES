# CLAUDE.md — rustysnes-apu

SPC700 + S-DSP audio path. Spec: `../../docs/apu.md` (read before changing the SPC700 core, DSP
mixing, or APU/main-CPU sync). Runs on its own scheduler divisor off the master clock; the
APU<->CPU port handshake timing is part of the contract. Determinism is hard: identical
seed+ROM+input must yield bit-identical audio. Touching behavior means updating `../../docs/apu.md`
in the same PR.
