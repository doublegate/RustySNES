# CLAUDE.md — rustysnes-cpu

WDC 65C816 core. Spec: `../../docs/cpu.md` (read before changing timing or opcode behavior).
The CPU borrows `&mut Bus`; it owns no chip state of its own. It does not depend on any other
chip crate. Cycle counts are part of the contract — pin the failing test ROM first, then implement.
Touching opcode/timing behavior means updating `../../docs/cpu.md` in the same PR.
