//! Fuzz the APU's host-facing surface — the four ports at `$2140-$2143` plus SMP execution.
//!
//! Those four bytes are the *entire* channel between the S-CPU and the SPC700: the IPL boot
//! handshake, every upload, and every result comes back through them. AccuracySNES's own APU
//! harness talks to the chip this way and nothing else, so this is the real interface, not a
//! synthetic one.
//!
//! The fuzzer plays the role of a hostile S-CPU: arbitrary port writes at arbitrary clock offsets,
//! which drives the IPL boot ROM's handshake state machine off its expected path. The IPL expects a
//! strict kick-byte sequence; what it does when that sequence is violated mid-transfer is exactly
//! the sort of thing no ordinary ROM exercises.
//!
//! Reads are included because `cpu_read_port`, `dsp_read`, `sample`, and `drain_audio` are all part
//! of the same host surface, and the DSP is stepped by `run_cycles` regardless of what the SMP is
//! doing — so this reaches BRR decoding, the gaussian interpolator, and the echo buffer with
//! whatever ARAM state the fuzzed handshake left behind.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_apu::Apu;

fuzz_target!(|data: &[u8]| {
    let mut apu = Apu::new();
    let mut sink = Vec::new();

    // Three bytes per operation: which port, what value, and how long to run afterwards. The clock
    // amount is fuzzed rather than fixed because the handshake is timing-sensitive — a write that
    // lands mid-transfer behaves differently from the same write a thousand cycles later.
    for op in data.chunks_exact(3) {
        apu.cpu_write_port(op[0] & 3, op[1]);

        // Cap the per-step clock so one input cannot spend the whole time budget in `run_cycles`.
        // 4096 clocks is a little over two 32 kHz DSP samples, enough for the mixer to advance.
        apu.run_cycles(u32::from(op[2]) * 16);

        let _ = apu.cpu_read_port(op[0] & 3);
        let _ = apu.dsp_read(op[1]);
        let _ = apu.sample();
    }

    apu.drain_audio(&mut sink);
});
