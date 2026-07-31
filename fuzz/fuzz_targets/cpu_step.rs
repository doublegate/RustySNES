//! Fuzz the 65C816 and the bus it drives, by executing fuzzed bytes as code.
//!
//! This is the chip-level target, and it deliberately reaches the chips the way hardware does —
//! through instruction execution — rather than by poking register files directly. `Bus::read24` /
//! `write24` and every `Ppu` field are private, so a direct PPU-register target could only exist by
//! widening the engine's public API for fuzzing's benefit. Executing garbage instead reaches the
//! PPU (`$2100-$213F`), CPUIO (`$4200-$421F`), DMA/HDMA (`$4300-$437F`), and the APU ports
//! (`$2140-$2143`) through the real decode path, which is also how a hostile ROM would get there.
//!
//! What is under test is the whole `step_instruction` surface: every opcode at every addressing
//! mode against an arbitrary bus state, including the ones ordinary ROMs never emit. `STP` halts
//! the CPU until reset, so the step budget bounds the loop rather than any halt check.
//!
//! The fixture cart is present because the reset vector and bank `$00` mapping must be sane for the
//! CPU to run at all; the fuzzed bytes go into WRAM, which is where the CPU is pointed.

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

/// Enough instructions to reach a DMA (which runs to completion inside one step) and to let a
/// write to `$4200` produce an interrupt on a later line, without letting one input dominate the
/// campaign's throughput.
const STEP_BUDGET: usize = 512;

fuzz_target!(|data: &[u8]| {
    // Seven bytes of register seed, the rest is code. Below that there is nothing to execute.
    if data.len() < 8 {
        return;
    }
    let (seed, code) = data.split_at(7);

    let mut core = common::minimal_core();
    let sys = core.system_mut();

    // Fill low WRAM with the fuzzed bytes and point the CPU at it. `poke_wram` is the non-intrusive
    // writer — it does not perturb the open-bus latch or trip watchpoints the way a CPU-side store
    // would, so the machine starts from a state the fuzzer chose rather than one this setup wrote.
    for (i, &byte) in code.iter().enumerate().take(0x2_0000) {
        sys.bus.poke_wram(0x7E_0000 + i as u32, byte);
    }

    sys.cpu.regs.a = u16::from_le_bytes([seed[0], seed[1]]);
    sys.cpu.regs.x = u16::from_le_bytes([seed[2], seed[3]]);
    sys.cpu.regs.y = u16::from_le_bytes([seed[4], seed[5]]);
    // `from_bits_retain`, not `from_bits`: every one of the eight `P` bits is architecturally
    // defined, so there is nothing to reject, and a `from_bits` that returned `None` would quietly
    // drop the fuzzer's chosen mode bits (`M`/`X` widths) — the very thing that decides how the
    // following instructions decode.
    sys.cpu.regs.p = rustysnes_core::cpu::regs::Status::from_bits_retain(seed[6]);
    // WRAM is mirrored into bank `$00`'s low 8 KiB, so this is the fuzzed bytes seen as code.
    sys.cpu.regs.pbr = 0x00;
    sys.cpu.regs.pc = 0x0000;

    for _ in 0..STEP_BUDGET {
        sys.step_instruction();
    }

    // The machine must still be serializable after arbitrary execution: a chip left holding a value
    // its own save path cannot represent is a real defect that only surfaces when a user saves.
    let _ = sys.save_state();
});
